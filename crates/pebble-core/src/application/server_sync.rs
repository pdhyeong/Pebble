use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;

use super::engine::EngineHandle;
use super::ports::{CredentialStore, DeviceCredentials, FileStore, ServerApi};
use super::state::SettingsStore;
use crate::domain::error::AppResult;
use crate::domain::peer::PeerId;

pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);
pub const SYNC_BUFFER: usize = 1;

#[derive(Clone)]
pub struct ServerDeps {
    pub api: Arc<dyn ServerApi>,
    pub credentials: Arc<dyn CredentialStore>,
    pub files: Arc<dyn FileStore>,
    pub settings: SettingsStore,
}

pub fn spawn_server_sync(
    deps: ServerDeps,
    peer: PeerId,
    engine: EngineHandle,
    stop: watch::Receiver<bool>,
    sync_requests: mpsc::Receiver<()>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        tokio::select! {
            _ = run(deps, peer, stop, sync_requests) => {}
            _ = engine.closed() => {}
        }
    })
}

async fn run(
    deps: ServerDeps,
    peer: PeerId,
    mut stop: watch::Receiver<bool>,
    mut sync_requests: mpsc::Receiver<()>,
) {
    let creds = tokio::select! {
        creds = load_or_register(&deps, &peer) => creds,
        _ = stop.changed() => return,
    };
    let creds = match creds {
        Ok(creds) => creds,
        Err(e) => {
            eprintln!("[server] 기기 등록 실패: {e}");
            return;
        }
    };
    let mut heartbeat = tokio::time::interval(HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            _ = stop.changed() => break,
            _ = heartbeat.tick() => {
                if let Err(e) = deps.api.heartbeat(&creds).await {
                    eprintln!("[server] heartbeat 실패: {e}");
                }
            }
            request = sync_requests.recv() => {
                if request.is_none() {
                    break;
                }
                sync_files(&deps, &creds).await;
            }
        }
    }
}

async fn load_or_register(deps: &ServerDeps, peer: &PeerId) -> AppResult<DeviceCredentials> {
    if let Some(creds) = deps.credentials.load(peer)? {
        return Ok(creds);
    }
    let device_name = deps.settings.get().device_name;
    let creds = deps.api.register(peer, &device_name).await?;
    deps.credentials.save(&creds)?;
    Ok(creds)
}

async fn sync_files(deps: &ServerDeps, creds: &DeviceCredentials) {
    let root = deps.settings.get().shared_folder;
    let result = match deps.files.snapshot_top_level(&root).await {
        Ok(files) => deps.api.sync_files(creds, files).await,
        Err(e) => Err(e),
    };
    if let Err(e) = result {
        eprintln!("[server] 파일 동기화 실패: {e}");
    }
}
