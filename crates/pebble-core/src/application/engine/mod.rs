mod peers;
mod transfers;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use serde::Serialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use super::events::UiEvent;
use super::net::{InboundId, InboundRequest, NetCommand, NetEvent};
use super::ports::{FileStore, TrustStore, UiEvents};
use super::state::{ActivityStore, SettingsStore};
use crate::domain::activity::ActivityRecord;
use crate::domain::error::{AppError, AppResult};
use crate::domain::pairing::Pin;
use crate::domain::path::RelativePath;
use crate::domain::peer::PeerId;
use crate::domain::transfer::{Direction, TransferId, TransferStatus};
use transfers::{Incoming, Outgoing, PendingPull, STALL_TIMEOUT};

pub const COMMAND_BUFFER: usize = 64;

type Reply<T> = oneshot::Sender<AppResult<T>>;

pub enum EngineCommand {
    Dial { address: String },
    Probe { address: String },
    Peers { reply: Reply<Vec<PeerSummary>> },
    RequestFileList { peer: PeerId, path: String },
    Download { peer: PeerId, path: RelativePath, reply: Reply<TransferId> },
    Upload { peer: PeerId, file: PathBuf, remote_dir: RelativePath, reply: Reply<TransferId> },
    Cancel { transfer_id: TransferId },
    RequestPairing { peer: PeerId, reply: Reply<Pin> },
    RespondPairing { peer: PeerId, approved: bool, reply: Reply<()> },
    FolderChanged,
    Shutdown,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerSummary {
    pub peer_id: PeerId,
    pub device_name: Option<String>,
    pub connected: bool,
    pub trusted: bool,
    pub pending_pin: Option<String>,
}

#[derive(Clone)]
pub struct EngineHandle {
    commands: mpsc::Sender<EngineCommand>,
}

impl EngineHandle {
    pub async fn send(&self, command: EngineCommand) -> AppResult<()> {
        self.commands
            .send(command)
            .await
            .map_err(|_| AppError::engine_unavailable())
    }

    pub async fn ask<T>(&self, make: impl FnOnce(Reply<T>) -> EngineCommand) -> AppResult<T> {
        let (reply, answer) = oneshot::channel();
        self.send(make(reply)).await?;
        answer.await.map_err(|_| AppError::engine_unavailable())?
    }

    pub fn notify_folder_changed(&self) {
        let _ = self.commands.try_send(EngineCommand::FolderChanged);
    }

    pub fn is_alive(&self) -> bool {
        !self.commands.is_closed()
    }

    pub async fn closed(&self) {
        self.commands.closed().await;
    }
}

#[derive(Clone)]
pub struct EngineDeps {
    pub ui: Arc<dyn UiEvents>,
    pub files: Arc<dyn FileStore>,
    pub settings: SettingsStore,
    pub activity: ActivityStore,
    pub trust: Arc<dyn TrustStore>,
}

pub fn spawn_engine(
    deps: EngineDeps,
    net: mpsc::UnboundedSender<NetCommand>,
    net_events: mpsc::Receiver<NetEvent>,
) -> (EngineHandle, JoinHandle<()>) {
    let (commands, inbox) = mpsc::channel(COMMAND_BUFFER);
    let engine = Engine {
        deps,
        net,
        trusted: HashSet::new(),
        connected: HashSet::new(),
        device_names: HashMap::new(),
        pending_pairings: HashMap::new(),
        outgoing: HashMap::new(),
        incoming: HashMap::new(),
        pulls: HashMap::new(),
    };
    let task = tokio::spawn(engine.run(inbox, net_events));
    (EngineHandle { commands }, task)
}

struct Engine {
    deps: EngineDeps,
    net: mpsc::UnboundedSender<NetCommand>,
    trusted: HashSet<PeerId>,
    connected: HashSet<PeerId>,
    device_names: HashMap<PeerId, String>,
    pending_pairings: HashMap<PeerId, (InboundId, Pin)>,
    outgoing: HashMap<TransferId, Outgoing>,
    incoming: HashMap<TransferId, Incoming>,
    pulls: HashMap<TransferId, PendingPull>,
}

impl Engine {
    async fn run(
        mut self,
        mut inbox: mpsc::Receiver<EngineCommand>,
        mut net_events: mpsc::Receiver<NetEvent>,
    ) {
        self.load_trusted().await;
        let mut sweep = tokio::time::interval(STALL_TIMEOUT / 4);
        let reason = loop {
            tokio::select! {
                _ = sweep.tick() => self.sweep_stalled().await,
                command = inbox.recv() => match command {
                    None | Some(EngineCommand::Shutdown) => break "P2P 엔진이 종료되었습니다.",
                    Some(command) => self.on_command(command).await,
                },
                event = net_events.recv() => match event {
                    None => break "네트워크가 멈춰 P2P 엔진이 종료되었습니다.",
                    Some(event) => self.on_net_event(event).await,
                },
            }
        };
        self.abort_all_transfers().await;
        self.send_net(NetCommand::Shutdown);
        self.emit(UiEvent::P2pStopped {
            message: reason.to_string(),
        });
    }

    async fn on_command(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::Dial { address } => self.send_net(NetCommand::Dial { address }),
            EngineCommand::Probe { address } => self.send_net(NetCommand::Probe { address }),
            EngineCommand::Peers { reply } => {
                let _ = reply.send(Ok(self.peer_summaries()));
            }
            EngineCommand::RequestFileList { peer, path } => self.request_file_list(peer, path),
            EngineCommand::Download { peer, path, reply } => {
                let _ = reply.send(self.start_download(peer, &path));
            }
            EngineCommand::Upload {
                peer,
                file,
                remote_dir,
                reply,
            } => {
                let _ = reply.send(self.start_upload(peer, &file, &remote_dir).await);
            }
            EngineCommand::Cancel { transfer_id } => self.cancel_by_user(&transfer_id).await,
            EngineCommand::RequestPairing { peer, reply } => {
                let _ = reply.send(Ok(self.request_pairing(peer)));
            }
            EngineCommand::RespondPairing {
                peer,
                approved,
                reply,
            } => {
                let _ = reply.send(self.respond_pairing(peer, approved).await);
            }
            EngineCommand::FolderChanged => self.notify_folder_changed(),
            EngineCommand::Shutdown => {}
        }
    }

    async fn on_net_event(&mut self, event: NetEvent) {
        match event {
            NetEvent::ListeningOn { address } => {
                if !address.contains("/127.0.0.1/") {
                    self.emit(UiEvent::ListeningOn { address });
                }
            }
            NetEvent::PeerDiscovered { peer, address } => self.emit(UiEvent::PeerFound {
                peer_id: peer,
                address,
            }),
            NetEvent::PeerExpired { peer } => self.emit(UiEvent::PeerExpired { peer_id: peer }),
            NetEvent::Identified {
                peer,
                agent_version,
            } => self.emit(UiEvent::PeerInfo {
                peer_id: peer,
                agent_version,
            }),
            NetEvent::Pinged { peer, rtt } => self.emit(UiEvent::PingSuccess {
                peer_id: peer,
                rtt_ms: u64::try_from(rtt.as_millis()).unwrap_or(u64::MAX),
            }),
            NetEvent::DialStarted { address } => self.emit(UiEvent::DialStarted { address }),
            NetEvent::DialFailed { address, error } => {
                self.emit(UiEvent::DialFailed { address, error })
            }
            NetEvent::Connected { peer } => {
                self.connected.insert(peer.clone());
                self.send_net(NetCommand::RequestDeviceInfo { peer: peer.clone() });
                self.emit(UiEvent::ConnectionSuccess { peer_id: peer });
            }
            NetEvent::Disconnected { peer } => self.on_disconnected(peer).await,
            NetEvent::Inbound { peer, id, request } => self.on_inbound(peer, id, request).await,
            NetEvent::FileListReplied { peer, reply } => self.emit(UiEvent::RemoteFiles {
                peer_id: peer,
                path: reply.path,
                files: reply.files,
                error: reply.error,
            }),
            NetEvent::StreamAcked { peer, ack } => self.on_stream_ack(peer, ack).await,
            NetEvent::PairingReplied { peer, reply } => self.on_pairing_reply(peer, reply).await,
            NetEvent::DeviceInfoReplied { peer, device_name } => {
                self.device_names.insert(peer.clone(), device_name.clone());
                self.emit(UiEvent::DeviceInfoReceived {
                    peer_id: peer.clone(),
                    device_name: device_name.clone(),
                });
                if self.trusted.contains(&peer) {
                    self.emit(UiEvent::PairingApproved {
                        peer_id: peer,
                        device_name,
                        shared_folder_name: None,
                    });
                }
            }
            NetEvent::RequestFailed {
                peer,
                kind,
                transfer_id,
                error,
            } => self.on_request_failed(peer, kind, transfer_id, error).await,
        }
    }

    async fn on_inbound(&mut self, peer: PeerId, id: InboundId, request: InboundRequest) {
        match request {
            InboundRequest::Pairing(ask) => self.on_pairing_request(peer, id, ask),
            InboundRequest::DeviceInfo => self.send_net(NetCommand::RespondDeviceInfo {
                inbound: id,
                device_name: self.deps.settings.get().device_name,
            }),
            InboundRequest::DirectoryChanged => self.on_directory_changed(peer, id),
            InboundRequest::FileList { path } => self.serve_file_list(peer, id, path).await,
            InboundRequest::Stream(msg) => self.on_stream_request(peer, id, msg).await,
        }
    }

    fn send_net(&self, command: NetCommand) {
        if self.net.send(command).is_err() {
            eprintln!("[engine] 네트워크 액터가 이미 종료되었습니다.");
        }
    }

    fn emit(&self, event: UiEvent) {
        self.deps.ui.emit(event);
    }

    fn record(
        &self,
        direction: Direction,
        peer: &PeerId,
        file_name: &str,
        size: u64,
        status: TransferStatus,
        speed: Option<f64>,
    ) {
        self.deps.activity.push(ActivityRecord {
            id: uuid::Uuid::new_v4().to_string(),
            transfer_type: direction,
            file_name: file_name.to_string(),
            peer_id: peer.clone(),
            device_name: self.device_names.get(peer).cloned(),
            size,
            timestamp: unix_now(),
            status,
            speed,
        });
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or_default()
}
