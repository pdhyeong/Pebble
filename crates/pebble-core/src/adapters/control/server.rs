use serde_json::{json, Value};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast::{self, error::RecvError};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

use super::protocol::{ControlReply, ControlRequest};
use crate::application::engine::TransferState;
use crate::application::events::UiEvent;
use crate::application::services::AppServices;
use crate::domain::error::{AppError, AppResult, ErrorCode};
use crate::domain::peer::PeerId;
use crate::domain::transfer::TransferId;

pub const MAX_REQUEST_BYTES: u64 = 64 * 1024;
pub const LIST_TIMEOUT: Duration = Duration::from_secs(30);
pub const PAIRING_TIMEOUT: Duration = Duration::from_secs(200);
const MIN_PREFIX: usize = 6;
const MIN_FULL_PEER_ID: usize = 46;

const MAX_SOCKET_PATH: usize = 100;
pub const MAX_CLIENTS: usize = 32;
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const ACCEPT_RETRY: Duration = Duration::from_millis(200);
const TRANSFER_CHECK: Duration = Duration::from_secs(30);

pub async fn bind(path: &Path) -> AppResult<UnixListener> {
    if path.as_os_str().len() > MAX_SOCKET_PATH {
        return Err(AppError::new(
            ErrorCode::InvalidPath,
            format!("설정 폴더 경로가 너무 깁니다({}자). {MAX_SOCKET_PATH}자 이하인 --config-dir 를 쓰세요: {}", path.as_os_str().len(), path.display()),
        ));
    }
    let dir = path.parent().ok_or_else(|| {
        AppError::new(
            ErrorCode::InvalidPath,
            "제어 소켓 경로가 올바르지 않습니다.",
        )
    })?;
    private_dir(dir, true)?;
    if UnixStream::connect(path).await.is_ok() {
        return Err(already_running());
    }
    let staging = dir.join(format!(
        ".c-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    ));
    private_dir(&staging, false)?;
    let claimed = remove_stale_socket(path, &staging).and_then(|()| claim_socket(&staging, path));
    let _ = std::fs::remove_dir_all(&staging);
    claimed
}

fn private_dir(dir: &Path, recursive: bool) -> AppResult<()> {
    std::fs::DirBuilder::new()
        .recursive(recursive)
        .mode(0o700)
        .create(dir)
        .map_err(|e| AppError::io("설정 폴더를 만들지 못했습니다", &e))
}

fn remove_stale_socket(path: &Path, staging: &Path) -> AppResult<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => {}
        Ok(_) => {
            return Err(AppError::new(
                ErrorCode::InvalidPath,
                format!("제어 소켓 자리에 다른 파일이 있습니다: {}", path.display()),
            ))
        }
        Err(_) => return Ok(()),
    }
    let moved = staging.join("old");
    match std::fs::rename(path, &moved) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(AppError::io("남은 제어 소켓을 치우지 못했습니다", &e)),
    }
    if std::os::unix::net::UnixStream::connect(&moved).is_ok() {
        let _ = std::fs::hard_link(&moved, path);
        return Err(already_running());
    }
    Ok(())
}

fn claim_socket(staging: &Path, path: &Path) -> AppResult<UnixListener> {
    let temp: PathBuf = staging.join("s");
    let listener =
        UnixListener::bind(&temp).map_err(|e| AppError::io("제어 소켓을 열지 못했습니다", &e))?;
    std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| AppError::io("제어 소켓 권한을 바꾸지 못했습니다", &e))?;
    match std::fs::hard_link(&temp, path) {
        Ok(()) => Ok(listener),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(already_running()),
        Err(e) => Err(AppError::io("제어 소켓을 열지 못했습니다", &e)),
    }
}

fn already_running() -> AppError {
    AppError::new(
        ErrorCode::P2pAlreadyRunning,
        "이 기기에서 Pebble 이 이미 실행 중입니다(앱 또는 daemon).",
    )
}

pub fn serve(
    listener: UnixListener,
    services: AppServices,
    events: broadcast::Sender<UiEvent>,
) -> JoinHandle<()> {
    let slots = Arc::new(Semaphore::new(MAX_CLIENTS));
    tokio::spawn(async move {
        loop {
            let stream = match listener.accept().await {
                Ok((stream, _)) => stream,
                Err(e) => {
                    eprintln!("[control] 연결을 받지 못했습니다: {e}");
                    tokio::time::sleep(ACCEPT_RETRY).await;
                    continue;
                }
            };
            let Ok(slot) = slots.clone().try_acquire_owned() else {
                tokio::spawn(refuse_busy(stream));
                continue;
            };
            let services = services.clone();
            let subscription = events.subscribe();
            tokio::spawn(async move {
                handle(stream, services, subscription).await;
                drop(slot);
            });
        }
    })
}

async fn refuse_busy(stream: UnixStream) {
    let (_, mut write) = stream.into_split();
    let reply: ControlReply = AppError::new(
        ErrorCode::Internal,
        "동시에 연결된 명령이 너무 많습니다. 잠시 뒤 다시 시도하세요.",
    )
    .into();
    let _ = send(&mut write, &reply).await;
}

async fn handle(
    stream: UnixStream,
    services: AppServices,
    mut events: broadcast::Receiver<UiEvent>,
) {
    let (read, mut write) = stream.into_split();
    let mut line = String::new();
    let mut reader = BufReader::new(read.take(MAX_REQUEST_BYTES));
    match tokio::time::timeout(REQUEST_TIMEOUT, reader.read_line(&mut line)).await {
        Ok(Ok(_)) => {}
        _ => return,
    }
    let outcome = match serde_json::from_str::<ControlRequest>(&line) {
        Ok(request) => dispatch(request, &services, &mut events, &mut write).await,
        Err(e) => Err(AppError::new(
            ErrorCode::Internal,
            format!("요청을 읽지 못했습니다: {e}"),
        )),
    };
    let reply = match outcome {
        Ok(data) => ControlReply::Ok { data },
        Err(e) => e.into(),
    };
    let _ = send(&mut write, &reply).await;
}

async fn send(write: &mut OwnedWriteHalf, reply: &ControlReply) -> std::io::Result<()> {
    let mut bytes = serde_json::to_vec(reply).unwrap_or_default();
    bytes.push(b'\n');
    write.write_all(&bytes).await
}

async fn dispatch(
    request: ControlRequest,
    services: &AppServices,
    events: &mut broadcast::Receiver<UiEvent>,
    write: &mut OwnedWriteHalf,
) -> AppResult<Value> {
    match request {
        ControlRequest::Status => {
            let settings = services.settings.get();
            Ok(json!({
                "running": services.p2p_status().await,
                "peer_id": services.supervisor.local_peer().await,
                "device_name": settings.device_name,
                "shared_folder": settings.shared_folder,
            }))
        }
        ControlRequest::Peers => to_json(services.peers().await?),
        ControlRequest::Connect { address } => {
            services.dial(address).await?;
            Ok(Value::Null)
        }
        ControlRequest::Pair { peer } => {
            let peer = resolve_peer(services, &peer).await?;
            let pin = services.request_pairing(peer.as_str()).await?;
            forward(
                write,
                json!({ "event": "pairing-sent", "payload": { "peer_id": peer, "pin": pin } }),
            )
            .await?;
            wait_pairing(events, &peer).await
        }
        ControlRequest::Respond { peer, approve } => {
            let peer = resolve_peer(services, &peer).await?;
            services.respond_pairing(peer.as_str(), approve).await?;
            Ok(Value::Null)
        }
        ControlRequest::Ls { peer, path } => {
            let peer = resolve_peer(services, &peer).await?;
            services
                .request_file_list(peer.as_str(), path.clone())
                .await?;
            wait_listing(events, &peer, &path).await
        }
        ControlRequest::Send { peer, file, dir } => {
            let peer = resolve_peer(services, &peer).await?;
            let id = services.upload(peer.as_str(), file, &dir).await?;
            follow_transfer(services, events, write, &id).await
        }
        ControlRequest::Get { peer, path } => {
            let peer = resolve_peer(services, &peer).await?;
            let id = services.download(peer.as_str(), &path).await?;
            follow_transfer(services, events, write, &id).await
        }
        ControlRequest::Cancel { transfer } => {
            services.cancel(transfer).await?;
            Ok(Value::Null)
        }
    }
}

fn to_json<T: serde::Serialize>(value: T) -> AppResult<Value> {
    serde_json::to_value(value).map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))
}

async fn forward(write: &mut OwnedWriteHalf, event: Value) -> AppResult<()> {
    send(write, &ControlReply::Event { event })
        .await
        .map_err(|e| AppError::io("CLI 연결이 끊겼습니다", &e))
}

async fn resolve_peer(services: &AppServices, query: &str) -> AppResult<PeerId> {
    let peers = services.peers().await?;
    if let Some(found) = peers.iter().find(|p| p.peer_id.as_str() == query) {
        return Ok(found.peer_id.clone());
    }
    let by_name: Vec<&PeerId> = peers
        .iter()
        .filter(|p| {
            p.device_name
                .as_deref()
                .is_some_and(|n| n.eq_ignore_ascii_case(query))
        })
        .map(|p| &p.peer_id)
        .collect();
    let by_prefix: Vec<&PeerId> = peers
        .iter()
        .filter(|p| query.len() >= MIN_PREFIX && p.peer_id.as_str().starts_with(query))
        .map(|p| &p.peer_id)
        .collect();
    match (by_name.as_slice(), by_prefix.as_slice()) {
        ([one], _) | ([], [one]) => Ok((*one).clone()),
        ([], []) if query.len() >= MIN_FULL_PEER_ID => {
            PeerId::parse(query).map_err(|_| peer_not_found(query))
        }
        ([], []) => Err(peer_not_found(query)),
        _ => Err(AppError::new(
            ErrorCode::InvalidPeerId,
            format!("'{query}' 에 해당하는 기기가 여럿입니다. PeerId 로 지정하세요."),
        )),
    }
}

fn peer_not_found(query: &str) -> AppError {
    AppError::new(
        ErrorCode::NotFound,
        format!("기기를 찾지 못했습니다: {query} (`pebble-cli peers` 로 목록을 보세요)"),
    )
}

async fn next_event(events: &mut broadcast::Receiver<UiEvent>) -> AppResult<UiEvent> {
    match events.recv().await {
        Ok(event) => Ok(event),
        Err(RecvError::Lagged(missed)) => Err(AppError::new(
            ErrorCode::Network,
            format!("알림 {missed}개를 놓쳐 결과를 확인하지 못했습니다. `pebble-cli peers` 나 받은 폴더에서 상태를 확인하세요."),
        )),
        Err(RecvError::Closed) => Err(AppError::engine_unavailable()),
    }
}

async fn wait_pairing(
    events: &mut broadcast::Receiver<UiEvent>,
    peer: &PeerId,
) -> AppResult<Value> {
    let wait = async {
        loop {
            match next_event(events).await? {
                UiEvent::PairingApproved {
                    peer_id,
                    device_name,
                    ..
                } if &peer_id == peer => {
                    return Ok(json!({ "peer_id": peer_id, "device_name": device_name }));
                }
                UiEvent::PairingRejected { peer_id, error } if &peer_id == peer => {
                    return Err(AppError::new(
                        ErrorCode::AccessDenied,
                        error.unwrap_or_else(|| "상대가 페어링을 거절했습니다.".to_string()),
                    ));
                }
                _ => {}
            }
        }
    };
    with_timeout(PAIRING_TIMEOUT, "페어링 응답", wait).await
}

async fn wait_listing(
    events: &mut broadcast::Receiver<UiEvent>,
    peer: &PeerId,
    path: &str,
) -> AppResult<Value> {
    let wait = async {
        loop {
            match next_event(events).await? {
                UiEvent::RemoteFiles {
                    peer_id,
                    path: listed,
                    files,
                    error,
                } if &peer_id == peer && listed == path => {
                    return match error {
                        Some(message) => Err(AppError::new(ErrorCode::AccessDenied, message)),
                        None => to_json(files),
                    };
                }
                UiEvent::FileListError { peer_id, error } if &peer_id == peer => {
                    return Err(AppError::new(ErrorCode::Network, error));
                }
                _ => {}
            }
        }
    };
    with_timeout(LIST_TIMEOUT, "파일 목록 응답", wait).await
}

async fn follow_transfer(
    services: &AppServices,
    events: &mut broadcast::Receiver<UiEvent>,
    write: &mut OwnedWriteHalf,
    id: &TransferId,
) -> AppResult<Value> {
    let mut check = tokio::time::interval(TRANSFER_CHECK);
    check.tick().await;
    loop {
        let event = tokio::select! {
            received = events.recv() => match received {
                Ok(event) => event,
                Err(RecvError::Lagged(_)) => match ended_or_wait(services, id).await? {
                    Some(event) => event,
                    None => continue,
                },
                Err(RecvError::Closed) => return Err(AppError::engine_unavailable()),
            },
            _ = check.tick() => match ended_or_wait(services, id).await? {
                Some(event) => event,
                None => continue,
            },
        };
        match &event {
            UiEvent::TransferCompleted { transfer_id, .. } if transfer_id == id => {
                return to_json(&event)
            }
            UiEvent::TransferFailed {
                transfer_id, error, ..
            } if transfer_id == id => {
                return Err(AppError::new(ErrorCode::Network, error.clone()));
            }
            UiEvent::TransferCancelled { transfer_id, .. } if transfer_id == id => {
                return Err(AppError::new(ErrorCode::Network, "전송이 취소되었습니다."));
            }
            UiEvent::TransferStarted { transfer_id, .. }
            | UiEvent::TransferProgress { transfer_id, .. }
                if transfer_id == id =>
            {
                forward(write, to_json(&event)?).await?;
            }
            _ => {}
        }
    }
}

async fn ended_or_wait(services: &AppServices, id: &TransferId) -> AppResult<Option<UiEvent>> {
    match services.transfer_state(id).await? {
        TransferState::Active => Ok(None),
        TransferState::Ended(event) => Ok(Some(event)),
        TransferState::Unknown => Err(AppError::new(
            ErrorCode::NotFound,
            format!("전송 {id} 의 결과를 찾지 못했습니다. 받은 폴더에서 확인하세요."),
        )),
    }
}

async fn with_timeout<F>(limit: Duration, what: &str, wait: F) -> AppResult<Value>
where
    F: std::future::Future<Output = AppResult<Value>>,
{
    tokio::time::timeout(limit, wait).await.unwrap_or_else(|_| {
        Err(AppError::new(
            ErrorCode::Network,
            format!("{what}을 {}초 안에 받지 못했습니다.", limit.as_secs()),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn short_dir() -> PathBuf {
        let dir = PathBuf::from("/tmp").join(format!(
            "pbc-{}",
            &uuid::Uuid::new_v4().simple().to_string()[..8]
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn bind_is_private_and_refuses_second_instance() {
        let dir = short_dir();
        let path = dir.join("cfg").join("control.sock");
        let _listener = bind(&path).await.unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        let dir_mode = std::fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(dir_mode, 0o700);
        assert!(UnixStream::connect(&path).await.is_ok());
        let second = bind(&path).await.unwrap_err();
        assert!(matches!(second.code, ErrorCode::P2pAlreadyRunning));
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|e| e.file_name())
            .collect();
        assert_eq!(leftovers.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn clearing_a_socket_that_turns_out_live_puts_it_back() {
        let dir = short_dir();
        let path = dir.join("control.sock");
        let _live = bind(&path).await.unwrap();
        let staging = dir.join("stage");
        std::fs::create_dir(&staging).unwrap();
        let err = remove_stale_socket(&path, &staging).unwrap_err();
        assert!(matches!(err.code, ErrorCode::P2pAlreadyRunning));
        assert!(UnixStream::connect(&path).await.is_ok());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn stale_socket_is_replaced_but_other_files_are_kept() {
        let dir = short_dir();
        let path = dir.join("control.sock");
        drop(bind(&path).await.unwrap());
        assert!(path.exists());
        let _listener = bind(&path).await.unwrap();
        assert!(UnixStream::connect(&path).await.is_ok());

        let other = dir.join("other.sock");
        std::fs::write(&other, b"keep").unwrap();
        assert!(bind(&other).await.is_err());
        assert_eq!(std::fs::read(&other).unwrap(), b"keep");
        let _ = std::fs::remove_dir_all(dir);
    }

    use crate::adapters::event_hub::EventHub;
    use crate::adapters::fs::{JsonTrustStore, LocalFileStore};
    use crate::application::engine::EngineDeps;
    use crate::application::events::UiEvent;
    use crate::application::net::{NetCommand, NetEvent, NetworkLink, PairingReply};
    use crate::application::ports::{FileStore, NetworkFactory, UiEvents};
    use crate::application::state::{ActivityStore, SettingsStore};
    use crate::application::supervisor::P2pSupervisor;
    use crate::domain::settings::Settings;
    use crate::domain::transfer::{CancelReason, Direction};
    use std::future::Future;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    const P1: &str = "12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const P2: &str = "12D3KooWBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB";
    const P3: &str = "12D3KooWCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC";
    const WAIT: Duration = Duration::from_secs(3);

    async fn within<T>(what: &str, fut: impl Future<Output = T>) -> T {
        tokio::time::timeout(WAIT, fut)
            .await
            .unwrap_or_else(|_| panic!("{what}: 시간 안에 끝나지 않음"))
    }

    struct FakeNet(Mutex<Option<NetworkLink>>);

    impl NetworkFactory for FakeNet {
        fn start(&self) -> AppResult<NetworkLink> {
            Ok(self.0.lock().unwrap().take().expect("한 번만 시작"))
        }
    }

    struct Fixture {
        services: AppServices,
        net: mpsc::Sender<NetEvent>,
        cmds: mpsc::UnboundedReceiver<NetCommand>,
        hub: Arc<EventHub>,
    }

    async fn fixture(started: bool) -> Fixture {
        let root = short_dir();
        let hub = Arc::new(EventHub::new(None));
        let ui: Arc<dyn UiEvents> = hub.clone();
        let files: Arc<dyn FileStore> = Arc::new(LocalFileStore::new(root.join("dl")));
        let mut initial = Settings::with_home(Some(root.clone()));
        initial.device_name = "fixture-device".to_string();
        let settings = SettingsStore::new(initial);
        let activity = ActivityStore::default();
        let deps = EngineDeps {
            ui: ui.clone(),
            files: files.clone(),
            settings: settings.clone(),
            activity: activity.clone(),
            trust: Arc::new(JsonTrustStore::new(root.join("trusted.json"))),
        };
        let (commands, cmds) = mpsc::unbounded_channel();
        let (net, events) = mpsc::channel(64);
        let link = NetworkLink {
            local_peer: PeerId::parse(P3).unwrap(),
            commands,
            events,
            task: tokio::spawn(std::future::pending::<()>()),
        };
        let supervisor = Arc::new(P2pSupervisor::new(
            deps,
            Arc::new(FakeNet(Mutex::new(Some(link)))),
            None,
            None,
        ));
        let folder_watch = Arc::new(crate::application::folder_watch::FolderWatch::spawn(
            Arc::new(crate::adapters::fs::NotifyFolderWatcher),
            ui.clone(),
            supervisor.clone(),
        ));
        let services = AppServices {
            supervisor,
            folder_watch,
            settings,
            activity,
            files,
            ui,
        };
        if started {
            services.start_p2p().await.unwrap();
        }
        Fixture {
            services,
            net,
            cmds,
            hub,
        }
    }

    async fn connect_peer(f: &Fixture, id: &str, name: &str) {
        let peer = PeerId::parse(id).unwrap();
        f.net
            .send(NetEvent::Connected { peer: peer.clone() })
            .await
            .unwrap();
        f.net
            .send(NetEvent::DeviceInfoReplied {
                peer,
                device_name: name.to_string(),
            })
            .await
            .unwrap();
        within("기기 목록", async {
            loop {
                let peers = f.services.peers().await.unwrap();
                if peers
                    .iter()
                    .any(|p| p.peer_id.as_str() == id && p.device_name.as_deref() == Some(name))
                {
                    return;
                }
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    async fn two_peers() -> Fixture {
        let f = fixture(true).await;
        connect_peer(&f, P1, "alpha").await;
        connect_peer(&f, P2, "beta").await;
        f
    }

    #[tokio::test]
    async fn resolve_peer_matches_exact_id_name_and_unique_prefix() {
        let f = two_peers().await;
        let s = &f.services;
        assert_eq!(resolve_peer(s, P2).await.unwrap().as_str(), P2);
        assert_eq!(resolve_peer(s, P1).await.unwrap().as_str(), P1);
        assert_eq!(resolve_peer(s, "BETA").await.unwrap().as_str(), P2);
        assert_eq!(resolve_peer(s, "alpha").await.unwrap().as_str(), P1);
        assert_eq!(resolve_peer(s, "12D3KooWA").await.unwrap().as_str(), P1);
        assert_eq!(resolve_peer(s, "12D3KooWB").await.unwrap().as_str(), P2);
        assert_eq!(
            resolve_peer(s, "12D3Ko").await.unwrap_err().code,
            ErrorCode::InvalidPeerId
        );
    }

    #[tokio::test]
    async fn resolve_peer_rejects_unknown_short_and_ambiguous_queries() {
        let f = two_peers().await;
        let s = &f.services;
        assert_eq!(
            resolve_peer(s, "nobody").await.unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(
            resolve_peer(s, "12D3K").await.unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(
            resolve_peer(s, "12D3KooWZ").await.unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(
            resolve_peer(s, &P3[..45]).await.unwrap_err().code,
            ErrorCode::NotFound
        );
        assert_eq!(
            resolve_peer(s, "12D3KooW").await.unwrap_err().code,
            ErrorCode::InvalidPeerId
        );
    }

    #[tokio::test]
    async fn resolve_peer_accepts_a_full_unknown_peer_id() {
        let f = two_peers().await;
        assert_eq!(resolve_peer(&f.services, P3).await.unwrap().as_str(), P3);
        assert_eq!(
            resolve_peer(&f.services, &P3[..46]).await.unwrap().as_str(),
            &P3[..46]
        );
    }

    #[tokio::test]
    async fn resolve_peer_rejects_duplicate_names() {
        let f = fixture(true).await;
        connect_peer(&f, P1, "same").await;
        connect_peer(&f, P2, "same").await;
        let err = resolve_peer(&f.services, "same").await.unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidPeerId);
    }

    fn pid(id: &str) -> PeerId {
        PeerId::parse(id).unwrap()
    }

    fn approved(id: &str, name: &str) -> UiEvent {
        UiEvent::PairingApproved {
            peer_id: pid(id),
            device_name: name.to_string(),
            shared_folder_name: None,
        }
    }

    fn rejected(id: &str, error: Option<&str>) -> UiEvent {
        UiEvent::PairingRejected {
            peer_id: pid(id),
            error: error.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn wait_pairing_ignores_other_peers_and_returns_ours() {
        let (tx, mut rx) = broadcast::channel(16);
        tx.send(approved(P2, "other")).unwrap();
        tx.send(rejected(P2, Some("other no"))).unwrap();
        tx.send(approved(P1, "mine")).unwrap();
        let ok = within("승인", wait_pairing(&mut rx, &pid(P1)))
            .await
            .unwrap();
        assert_eq!(ok["peer_id"], P1);
        assert_eq!(ok["device_name"], "mine");
    }

    #[tokio::test]
    async fn wait_pairing_reports_our_rejection_only() {
        let (tx, mut rx) = broadcast::channel(16);
        tx.send(rejected(P2, Some("other no"))).unwrap();
        tx.send(rejected(P1, Some("mine no"))).unwrap();
        let err = within("거절", wait_pairing(&mut rx, &pid(P1)))
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AccessDenied);
        assert_eq!(err.message, "mine no");
        tx.send(rejected(P1, None)).unwrap();
        let err = within("기본 거절", wait_pairing(&mut rx, &pid(P1)))
            .await
            .unwrap_err();
        assert_eq!(err.message, "상대가 페어링을 거절했습니다.");
    }

    fn remote_files(id: &str, path: &str, error: Option<&str>) -> UiEvent {
        UiEvent::RemoteFiles {
            peer_id: pid(id),
            path: path.to_string(),
            files: Vec::new(),
            error: error.map(str::to_string),
        }
    }

    #[tokio::test]
    async fn wait_listing_matches_peer_and_path() {
        let (tx, mut rx) = broadcast::channel(16);
        tx.send(remote_files(P2, "/docs", Some("wrong peer")))
            .unwrap();
        tx.send(remote_files(P1, "/other", Some("wrong path")))
            .unwrap();
        tx.send(remote_files(P1, "/docs", None)).unwrap();
        let ok = within("목록", wait_listing(&mut rx, &pid(P1), "/docs"))
            .await
            .unwrap();
        assert_eq!(ok, json!([]));
    }

    #[tokio::test]
    async fn wait_listing_reports_errors_for_our_peer_only() {
        let (tx, mut rx) = broadcast::channel(16);
        tx.send(UiEvent::FileListError {
            peer_id: pid(P2),
            error: "other".into(),
        })
        .unwrap();
        tx.send(remote_files(P1, "/docs", Some("denied"))).unwrap();
        let err = within("거부", wait_listing(&mut rx, &pid(P1), "/docs"))
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::AccessDenied);
        assert_eq!(err.message, "denied");
        tx.send(UiEvent::FileListError {
            peer_id: pid(P2),
            error: "other".into(),
        })
        .unwrap();
        tx.send(UiEvent::FileListError {
            peer_id: pid(P1),
            error: "mine".into(),
        })
        .unwrap();
        let err = within("목록 오류", wait_listing(&mut rx, &pid(P1), "/docs"))
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert_eq!(err.message, "mine");
    }

    fn tid(id: &str) -> TransferId {
        TransferId::from_wire(id.to_string())
    }

    fn started(id: &str) -> UiEvent {
        UiEvent::TransferStarted {
            transfer_id: tid(id),
            direction: Direction::Upload,
            peer_id: pid(P1),
            file_name: format!("{id}.bin"),
            total_size: 10,
        }
    }

    fn progress(id: &str) -> UiEvent {
        UiEvent::TransferProgress {
            transfer_id: tid(id),
            direction: Direction::Upload,
            bytes_done: 5,
            total_size: 10,
            progress: 0.5,
        }
    }

    fn completed(id: &str) -> UiEvent {
        UiEvent::TransferCompleted {
            transfer_id: tid(id),
            direction: Direction::Upload,
            peer_id: pid(P1),
            file_name: format!("{id}.bin"),
            total_size: 10,
            saved_path: None,
            speed_bps: None,
        }
    }

    fn failed(id: &str, error: &str) -> UiEvent {
        UiEvent::TransferFailed {
            transfer_id: tid(id),
            direction: Direction::Upload,
            peer_id: pid(P1),
            file_name: format!("{id}.bin"),
            error: error.to_string(),
        }
    }

    fn cancelled(id: &str) -> UiEvent {
        UiEvent::TransferCancelled {
            transfer_id: tid(id),
            direction: Direction::Upload,
            peer_id: pid(P1),
            file_name: format!("{id}.bin"),
            reason: CancelReason::UserCancelled,
        }
    }

    async fn run_follow(events: Vec<UiEvent>) -> (AppResult<Value>, Vec<Value>) {
        let f = fixture(false).await;
        let (tx, mut rx) = broadcast::channel(32);
        for e in events {
            tx.send(e).unwrap();
        }
        let (client, server) = UnixStream::pair().unwrap();
        let (_server_read, mut write) = server.into_split();
        let outcome = within(
            "전송 따라가기",
            follow_transfer(&f.services, &mut rx, &mut write, &tid("mine")),
        )
        .await;
        drop(write);
        drop(_server_read);
        let mut lines = BufReader::new(client).lines();
        let mut forwarded = Vec::new();
        while let Some(line) = lines.next_line().await.unwrap() {
            let reply: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(reply["type"], "event");
            forwarded.push(reply["event"].clone());
        }
        (outcome, forwarded)
    }

    #[tokio::test]
    async fn follow_transfer_returns_our_completion_and_skips_others() {
        let (outcome, forwarded) = run_follow(vec![
            completed("theirs"),
            started("theirs"),
            progress("theirs"),
            started("mine"),
            progress("mine"),
            completed("mine"),
        ])
        .await;
        let done = outcome.unwrap();
        assert_eq!(done["payload"]["file_name"], "mine.bin");
        assert_eq!(forwarded.len(), 2);
        assert_eq!(forwarded[0]["event"], "transfer-started");
        assert_eq!(forwarded[0]["payload"]["transfer_id"], "mine");
        assert_eq!(forwarded[1]["event"], "transfer-progress");
        assert_eq!(forwarded[1]["payload"]["transfer_id"], "mine");
    }

    async fn trust_and_receive(f: &Fixture, id: &str) {
        let peer = PeerId::parse(P1).unwrap();
        f.net
            .send(NetEvent::Inbound {
                peer: peer.clone(),
                id: crate::application::net::InboundId(1),
                request: crate::application::net::InboundRequest::Pairing(
                    crate::application::net::PairingAsk {
                        device_name: "alpha".into(),
                        pin: "123456".into(),
                    },
                ),
            })
            .await
            .unwrap();
        within("페어링 승인", async {
            while f.services.respond_pairing(P1, true).await.is_err() {
                tokio::task::yield_now().await;
            }
        })
        .await;
        f.net
            .send(NetEvent::Inbound {
                peer,
                id: crate::application::net::InboundId(2),
                request: crate::application::net::InboundRequest::Stream(
                    crate::application::net::StreamMsg::Init {
                        transfer_id: tid(id),
                        file_name: format!("{id}.bin"),
                        total_size: 10,
                        remote_path: "/".into(),
                        for_pull: false,
                    },
                ),
            })
            .await
            .unwrap();
        wait_state(f, id, |s| matches!(s, TransferState::Active)).await;
    }

    async fn cancel_incoming(f: &Fixture, id: &str) {
        f.net
            .send(NetEvent::Inbound {
                peer: PeerId::parse(P1).unwrap(),
                id: crate::application::net::InboundId(3),
                request: crate::application::net::InboundRequest::Stream(
                    crate::application::net::StreamMsg::Cancel {
                        transfer_id: tid(id),
                    },
                ),
            })
            .await
            .unwrap();
        wait_state(f, id, |s| matches!(s, TransferState::Ended(_))).await;
    }

    async fn wait_state(f: &Fixture, id: &str, ready: impl Fn(&TransferState) -> bool) {
        within("전송 상태", async {
            while !ready(&f.services.transfer_state(&tid(id)).await.unwrap()) {
                tokio::task::yield_now().await;
            }
        })
        .await;
    }

    fn lagging(events: Vec<UiEvent>) -> (broadcast::Sender<UiEvent>, broadcast::Receiver<UiEvent>) {
        let (tx, rx) = broadcast::channel(1);
        for e in events {
            tx.send(e).unwrap();
        }
        (tx, rx)
    }

    async fn follow(f: &Fixture, rx: &mut broadcast::Receiver<UiEvent>) -> AppResult<Value> {
        let (_client, server) = UnixStream::pair().unwrap();
        let (_read, mut write) = server.into_split();
        follow_transfer(&f.services, rx, &mut write, &tid("mine")).await
    }

    #[tokio::test]
    async fn follow_transfer_recovers_an_ending_it_missed_while_lagging() {
        let f = fixture(true).await;
        trust_and_receive(&f, "mine").await;
        cancel_incoming(&f, "mine").await;
        let (_tx, mut rx) = lagging(vec![progress("theirs"), progress("theirs")]);
        let outcome = within("놓친 끝 되찾기", follow(&f, &mut rx)).await;
        assert_eq!(outcome.unwrap_err().message, "전송이 취소되었습니다.");
    }

    #[tokio::test]
    async fn follow_transfer_keeps_waiting_after_a_lag_while_the_transfer_runs() {
        let f = fixture(true).await;
        trust_and_receive(&f, "mine").await;
        let (_tx, mut rx) = lagging(vec![
            progress("theirs"),
            progress("theirs"),
            completed("mine"),
        ]);
        let outcome = within("밀린 뒤 계속 기다리기", follow(&f, &mut rx)).await;
        assert_eq!(outcome.unwrap()["payload"]["file_name"], "mine.bin");
    }

    #[tokio::test]
    async fn follow_transfer_gives_up_on_a_transfer_the_engine_never_had() {
        let f = fixture(true).await;
        let (_tx, mut rx) = lagging(vec![progress("theirs"), progress("theirs")]);
        let err = within("모르는 전송", follow(&f, &mut rx))
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
    }

    #[tokio::test]
    async fn follow_transfer_checks_the_engine_on_its_own_when_events_stop() {
        let f = fixture(true).await;
        trust_and_receive(&f, "mine").await;
        cancel_incoming(&f, "mine").await;
        let (_tx, mut rx) = broadcast::channel::<UiEvent>(8);
        tokio::time::pause();
        let started = tokio::time::Instant::now();
        let outcome = tokio::time::timeout(TRANSFER_CHECK * 2, follow(&f, &mut rx))
            .await
            .expect("점검 틱이 오지 않음");
        assert_eq!(outcome.unwrap_err().message, "전송이 취소되었습니다.");
        assert!(started.elapsed() >= TRANSFER_CHECK);
    }

    #[tokio::test]
    async fn follow_transfer_reports_our_failure_only() {
        let (outcome, _) = run_follow(vec![
            failed("theirs", "their error"),
            failed("mine", "my error"),
        ])
        .await;
        let err = outcome.unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert_eq!(err.message, "my error");
    }

    #[tokio::test]
    async fn follow_transfer_reports_our_cancellation_only() {
        let (outcome, _) = run_follow(vec![cancelled("theirs"), cancelled("mine")]).await;
        let err = outcome.unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert_eq!(err.message, "전송이 취소되었습니다.");
        let (outcome, _) = run_follow(vec![cancelled("theirs"), completed("mine")]).await;
        assert_eq!(outcome.unwrap()["payload"]["file_name"], "mine.bin");
    }

    async fn serve_fixture(f: &Fixture) -> (PathBuf, JoinHandle<()>) {
        let dir = short_dir();
        let path = dir.join("c.sock");
        let listener = bind(&path).await.unwrap();
        let task = serve(listener, f.services.clone(), f.hub.sender());
        (path, task)
    }

    async fn call(path: &Path, raw: &str) -> Vec<Value> {
        let stream = UnixStream::connect(path).await.unwrap();
        let (read, mut write) = stream.into_split();
        write.write_all(raw.as_bytes()).await.unwrap();
        write.write_all(b"\n").await.unwrap();
        let mut lines = BufReader::new(read).lines();
        let mut replies = Vec::new();
        within("응답", async {
            while let Some(line) = lines.next_line().await.unwrap() {
                replies.push(serde_json::from_str::<Value>(&line).unwrap());
            }
        })
        .await;
        replies
    }

    #[tokio::test]
    async fn status_request_gets_a_reply() {
        let f = fixture(true).await;
        let (path, task) = serve_fixture(&f).await;
        let replies = call(&path, r#"{"cmd":"status"}"#).await;
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0]["type"], "ok");
        assert_eq!(replies[0]["data"]["running"], true);
        assert_eq!(replies[0]["data"]["peer_id"], P3);
        assert_eq!(replies[0]["data"]["device_name"], "fixture-device");
        task.abort();
    }

    #[tokio::test]
    async fn garbage_request_gets_an_error_reply() {
        let f = fixture(true).await;
        let (path, task) = serve_fixture(&f).await;
        let replies = call(&path, "not json").await;
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0]["type"], "error");
        task.abort();
    }

    #[tokio::test]
    async fn requests_larger_than_a_few_kib_are_read_whole() {
        let mut f = fixture(true).await;
        let (path, task) = serve_fixture(&f).await;
        let address = "a".repeat(5000);
        let raw = format!(r#"{{"cmd":"connect","address":"{address}"}}"#);
        let replies = call(&path, &raw).await;
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0]["type"], "ok");
        let dialed = within("Dial", async {
            loop {
                if let NetCommand::Dial { address } = f.cmds.recv().await.unwrap() {
                    return address;
                }
            }
        })
        .await;
        assert_eq!(dialed.len(), 5000);
        task.abort();
    }

    #[tokio::test]
    async fn pair_request_forwards_the_pin_then_reports_approval() {
        let f = fixture(true).await;
        let (path, task) = serve_fixture(&f).await;
        let stream = UnixStream::connect(&path).await.unwrap();
        let (read, mut write) = stream.into_split();
        let raw = format!("{{\"cmd\":\"pair\",\"peer\":\"{P1}\"}}\n");
        write.write_all(raw.as_bytes()).await.unwrap();
        let mut lines = BufReader::new(read).lines();
        let first = within("pairing-sent", lines.next_line())
            .await
            .unwrap()
            .unwrap();
        let first: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(first["type"], "event");
        assert_eq!(first["event"]["event"], "pairing-sent");
        assert_eq!(first["event"]["payload"]["peer_id"], P1);
        assert_eq!(first["event"]["payload"]["pin"].as_str().unwrap().len(), 6);
        f.net
            .send(NetEvent::PairingReplied {
                peer: pid(P1),
                reply: PairingReply {
                    approved: true,
                    device_name: "alpha".into(),
                    shared_folder_name: None,
                    error: None,
                },
            })
            .await
            .unwrap();
        let last = within("승인 응답", lines.next_line())
            .await
            .unwrap()
            .unwrap();
        let last: Value = serde_json::from_str(&last).unwrap();
        assert_eq!(last["type"], "ok");
        assert_eq!(last["data"]["peer_id"], P1);
        assert_eq!(last["data"]["device_name"], "alpha");
        task.abort();
    }

    #[tokio::test]
    async fn bind_accepts_a_path_of_exactly_the_limit_and_refuses_longer() {
        let dir = short_dir();
        let prefix = dir.as_os_str().len() + 1;
        let exact = dir.join("s".repeat(MAX_SOCKET_PATH - prefix));
        assert_eq!(exact.as_os_str().len(), MAX_SOCKET_PATH);
        let listener = bind(&exact).await.unwrap();
        assert!(UnixStream::connect(&exact).await.is_ok());
        drop(listener);
        let longer = dir.join("s".repeat(MAX_SOCKET_PATH - prefix + 1));
        let err = bind(&longer).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::InvalidPath);
        assert!(!longer.exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn clearing_a_stale_socket_tolerates_a_vanished_path_only() {
        let dir = short_dir();
        let path = dir.join("control.sock");
        drop(bind(&path).await.unwrap());
        let missing_staging = dir.join("no-such-stage");
        assert!(remove_stale_socket(&path, &missing_staging).is_ok());
        let blocker = dir.join("blocker");
        std::fs::write(&blocker, b"x").unwrap();
        let err = remove_stale_socket(&path, &blocker).unwrap_err();
        assert_eq!(err.code, ErrorCode::Io);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn claiming_a_taken_path_reports_already_running_and_other_failures_do_not() {
        let dir = short_dir();
        let staging = dir.join("stage");
        std::fs::create_dir(&staging).unwrap();
        let taken = dir.join("taken.sock");
        std::fs::write(&taken, b"x").unwrap();
        let err = claim_socket(&staging, &taken).unwrap_err();
        assert_eq!(err.code, ErrorCode::P2pAlreadyRunning);
        let staging = dir.join("stage2");
        std::fs::create_dir(&staging).unwrap();
        let unreachable = dir.join("no-such-dir").join("c.sock");
        let err = claim_socket(&staging, &unreachable).unwrap_err();
        assert_eq!(err.code, ErrorCode::NotFound);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn clients_beyond_the_limit_are_told_the_server_is_busy() {
        let f = fixture(true).await;
        let (path, task) = serve_fixture(&f).await;
        let mut idle = Vec::new();
        for _ in 0..MAX_CLIENTS {
            idle.push(UnixStream::connect(&path).await.unwrap());
        }
        let extra = UnixStream::connect(&path).await.unwrap();
        let mut lines = BufReader::new(extra).lines();
        let line = within("바쁨 응답", lines.next_line())
            .await
            .unwrap()
            .unwrap();
        let reply: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(reply["type"], "error");
        assert_eq!(reply["code"], "INTERNAL");
        drop(idle);
        task.abort();
    }
}
