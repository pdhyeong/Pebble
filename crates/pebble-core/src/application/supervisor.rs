use std::sync::{Arc, PoisonError, RwLock};
use std::time::Duration;
use tokio::sync::{mpsc, watch, Mutex};
use tokio::task::JoinHandle;

use super::discovery::spawn_discovery;
use super::engine::{spawn_engine, EngineCommand, EngineDeps, EngineHandle};
use super::ports::{NetworkFactory, PeerDirectory};
use super::server_sync::{spawn_server_sync, ServerDeps, SYNC_BUFFER};
use crate::domain::error::{AppError, AppResult};
use crate::domain::peer::PeerId;

pub const STOP_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
struct Active {
    local_peer: PeerId,
    engine: EngineHandle,
    sync: mpsc::Sender<()>,
}

struct Running {
    engine: EngineHandle,
    engine_task: JoinHandle<()>,
    network_task: JoinHandle<()>,
    stop: watch::Sender<bool>,
    server_task: Option<JoinHandle<()>>,
    discovery_task: Option<JoinHandle<()>>,
}

pub struct P2pSupervisor {
    engine_deps: EngineDeps,
    network: Arc<dyn NetworkFactory>,
    server: Option<ServerDeps>,
    directory: Option<Arc<dyn PeerDirectory>>,
    running: Mutex<Option<Running>>,
    active: RwLock<Option<Active>>,
}

impl P2pSupervisor {
    pub fn new(
        engine_deps: EngineDeps,
        network: Arc<dyn NetworkFactory>,
        server: Option<ServerDeps>,
        directory: Option<Arc<dyn PeerDirectory>>,
    ) -> Self {
        Self {
            engine_deps,
            network,
            server,
            directory,
            running: Mutex::new(None),
            active: RwLock::new(None),
        }
    }

    pub async fn start(&self) -> AppResult<()> {
        let mut running = self.running.lock().await;
        if let Some(previous) = running.take() {
            if previous.engine.is_alive() {
                *running = Some(previous);
                return Err(AppError::already_running());
            }
            self.set_active(None);
            shutdown(previous).await;
        }
        let link = self.network.start()?;
        let (engine, engine_task) =
            spawn_engine(self.engine_deps.clone(), link.commands, link.events);
        let (stop, stop_rx) = watch::channel(false);
        let (sync, sync_rx) = mpsc::channel(SYNC_BUFFER);
        let server_task = self.server.clone().map(|deps| {
            spawn_server_sync(
                deps,
                link.local_peer.clone(),
                engine.clone(),
                stop_rx.clone(),
                sync_rx,
            )
        });
        let discovery_task = self
            .directory
            .clone()
            .map(|directory| spawn_discovery(directory, engine.clone(), stop_rx));
        self.set_active(Some(Active {
            local_peer: link.local_peer,
            engine: engine.clone(),
            sync,
        }));
        *running = Some(Running {
            engine,
            engine_task,
            network_task: link.task,
            stop,
            server_task,
            discovery_task,
        });
        Ok(())
    }

    pub async fn stop(&self) -> AppResult<()> {
        let mut running = self.running.lock().await;
        let current = running.take().ok_or_else(AppError::not_running)?;
        self.set_active(None);
        let was_alive = current.engine.is_alive();
        shutdown(current).await;
        if was_alive {
            Ok(())
        } else {
            Err(AppError::not_running())
        }
    }

    pub async fn engine(&self) -> AppResult<EngineHandle> {
        self.alive()
            .map(|a| a.engine)
            .ok_or_else(AppError::not_running)
    }

    pub async fn local_peer(&self) -> Option<PeerId> {
        self.alive().map(|a| a.local_peer)
    }

    pub async fn is_running(&self) -> bool {
        self.engine().await.is_ok()
    }

    pub async fn folder_changed(&self) {
        if let Some(a) = self.alive() {
            a.engine.notify_folder_changed();
            let _ = a.sync.try_send(());
        }
    }

    fn alive(&self) -> Option<Active> {
        self.active
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
            .filter(|a| a.engine.is_alive())
    }

    fn set_active(&self, next: Option<Active>) {
        *self.active.write().unwrap_or_else(PoisonError::into_inner) = next;
    }
}

async fn shutdown(running: Running) {
    let _ = running.stop.send(true);
    if tokio::time::timeout(STOP_TIMEOUT, running.engine.send(EngineCommand::Shutdown))
        .await
        .is_err()
    {
        eprintln!("[supervisor] 엔진에 종료 신호를 제때 넣지 못했습니다.");
    }
    await_with_timeout("engine", running.engine_task).await;
    await_with_timeout("network", running.network_task).await;
    if let Some(task) = running.server_task {
        await_with_timeout("server", task).await;
    }
    if let Some(task) = running.discovery_task {
        await_with_timeout("discovery", task).await;
    }
}

async fn await_with_timeout(name: &str, task: JoinHandle<()>) {
    let abort = task.abort_handle();
    if tokio::time::timeout(STOP_TIMEOUT, task).await.is_err() {
        eprintln!("[supervisor] {name} 작업이 제때 끝나지 않아 강제로 멈춥니다.");
        abort.abort();
    }
}
