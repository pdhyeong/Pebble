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
    remove_stale_socket(path)?;
    let staging = dir.join(format!(
        ".c-{}",
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    ));
    private_dir(&staging, false)?;
    let claimed = claim_socket(&staging, path);
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

fn remove_stale_socket(path: &Path) -> AppResult<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => std::fs::remove_file(path)
            .map_err(|e| AppError::io("남은 제어 소켓을 지우지 못했습니다", &e)),
        Ok(_) => Err(AppError::new(
            ErrorCode::InvalidPath,
            format!("제어 소켓 자리에 다른 파일이 있습니다: {}", path.display()),
        )),
        Err(_) => Ok(()),
    }
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
            follow_transfer(events, write, &id).await
        }
        ControlRequest::Get { peer, path } => {
            let peer = resolve_peer(services, &peer).await?;
            let id = services.download(peer.as_str(), &path).await?;
            follow_transfer(events, write, &id).await
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
    events: &mut broadcast::Receiver<UiEvent>,
    write: &mut OwnedWriteHalf,
    id: &TransferId,
) -> AppResult<Value> {
    loop {
        let event = next_event(events).await?;
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
}
