use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast::{self, error::RecvError};
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

pub async fn bind(path: &Path) -> AppResult<UnixListener> {
    if path.as_os_str().len() > MAX_SOCKET_PATH {
        return Err(AppError::new(
            ErrorCode::InvalidPath,
            format!("설정 폴더 경로가 너무 깁니다({}자). {MAX_SOCKET_PATH}자 이하인 --config-dir 를 쓰세요: {}", path.as_os_str().len(), path.display()),
        ));
    }
    if let Some(dir) = path.parent() {
        tokio::fs::create_dir_all(dir)
            .await
            .map_err(|e| AppError::io("설정 폴더를 만들지 못했습니다", &e))?;
    }
    if UnixStream::connect(path).await.is_ok() {
        return Err(AppError::new(
            ErrorCode::P2pAlreadyRunning,
            "이 기기에서 Pebble 이 이미 실행 중입니다(앱 또는 daemon).",
        ));
    }
    let _ = tokio::fs::remove_file(path).await;
    let listener = UnixListener::bind(path).map_err(|e| AppError::io("제어 소켓을 열지 못했습니다", &e))?;
    restrict_to_owner(path).await?;
    Ok(listener)
}

async fn restrict_to_owner(path: &Path) -> AppResult<()> {
    use std::os::unix::fs::PermissionsExt;
    tokio::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(|e| AppError::io("제어 소켓 권한을 바꾸지 못했습니다", &e))
}

pub fn serve(listener: UnixListener, services: AppServices, events: broadcast::Sender<UiEvent>) -> JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                continue;
            };
            let services = services.clone();
            let subscription = events.subscribe();
            tokio::spawn(handle(stream, services, subscription));
        }
    })
}

async fn handle(stream: UnixStream, services: AppServices, mut events: broadcast::Receiver<UiEvent>) {
    let (read, mut write) = stream.into_split();
    let mut line = String::new();
    let mut reader = BufReader::new(read.take(MAX_REQUEST_BYTES));
    if reader.read_line(&mut line).await.is_err() {
        return;
    }
    let outcome = match serde_json::from_str::<ControlRequest>(&line) {
        Ok(request) => dispatch(request, &services, &mut events, &mut write).await,
        Err(e) => Err(AppError::new(ErrorCode::Internal, format!("요청을 읽지 못했습니다: {e}"))),
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
            forward(write, json!({ "event": "pairing-sent", "payload": { "peer_id": peer, "pin": pin } })).await;
            wait_pairing(events, &peer).await
        }
        ControlRequest::Respond { peer, approve } => {
            let peer = resolve_peer(services, &peer).await?;
            services.respond_pairing(peer.as_str(), approve).await?;
            Ok(Value::Null)
        }
        ControlRequest::Ls { peer, path } => {
            let peer = resolve_peer(services, &peer).await?;
            services.request_file_list(peer.as_str(), path.clone()).await?;
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

async fn forward(write: &mut OwnedWriteHalf, event: Value) {
    let _ = send(write, &ControlReply::Event { event }).await;
}

async fn resolve_peer(services: &AppServices, query: &str) -> AppResult<PeerId> {
    let peers = services.peers().await?;
    if let Some(found) = peers.iter().find(|p| p.peer_id.as_str() == query) {
        return Ok(found.peer_id.clone());
    }
    let by_name: Vec<&PeerId> = peers
        .iter()
        .filter(|p| p.device_name.as_deref().is_some_and(|n| n.eq_ignore_ascii_case(query)))
        .map(|p| &p.peer_id)
        .collect();
    let by_prefix: Vec<&PeerId> = peers
        .iter()
        .filter(|p| query.len() >= MIN_PREFIX && p.peer_id.as_str().starts_with(query))
        .map(|p| &p.peer_id)
        .collect();
    match (by_name.as_slice(), by_prefix.as_slice()) {
        ([one], _) | ([], [one]) => Ok((*one).clone()),
        ([], []) if query.len() >= MIN_FULL_PEER_ID => PeerId::parse(query).map_err(|_| {
            AppError::new(
                ErrorCode::NotFound,
                format!("기기를 찾지 못했습니다: {query} (`pebble-cli peers` 로 목록을 보세요)"),
            )
        }),
        ([], []) => Err(AppError::new(
            ErrorCode::NotFound,
            format!("기기를 찾지 못했습니다: {query} (`pebble-cli peers` 로 목록을 보세요)"),
        )),
        _ => Err(AppError::new(
            ErrorCode::InvalidPeerId,
            format!("'{query}' 에 해당하는 기기가 여럿입니다. PeerId 로 지정하세요."),
        )),
    }
}

async fn next_event(events: &mut broadcast::Receiver<UiEvent>) -> AppResult<UiEvent> {
    loop {
        match events.recv().await {
            Ok(event) => return Ok(event),
            Err(RecvError::Lagged(_)) => continue,
            Err(RecvError::Closed) => return Err(AppError::engine_unavailable()),
        }
    }
}

async fn wait_pairing(events: &mut broadcast::Receiver<UiEvent>, peer: &PeerId) -> AppResult<Value> {
    let wait = async {
        loop {
            match next_event(events).await? {
                UiEvent::PairingApproved { peer_id, device_name, .. } if &peer_id == peer => {
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

async fn wait_listing(events: &mut broadcast::Receiver<UiEvent>, peer: &PeerId, path: &str) -> AppResult<Value> {
    let wait = async {
        loop {
            match next_event(events).await? {
                UiEvent::RemoteFiles { peer_id, path: listed, files, error } if &peer_id == peer && listed == path => {
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
            UiEvent::TransferCompleted { transfer_id, .. } if transfer_id == id => return to_json(&event),
            UiEvent::TransferFailed { transfer_id, error, .. } if transfer_id == id => {
                return Err(AppError::new(ErrorCode::Network, error.clone()));
            }
            UiEvent::TransferCancelled { transfer_id, .. } if transfer_id == id => {
                return Err(AppError::new(ErrorCode::Network, "전송이 취소되었습니다."));
            }
            UiEvent::TransferStarted { transfer_id, .. } | UiEvent::TransferProgress { transfer_id, .. }
                if transfer_id == id =>
            {
                forward(write, to_json(&event)?).await;
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
