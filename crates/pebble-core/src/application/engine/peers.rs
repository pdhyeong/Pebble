use super::{Engine, PeerSummary};
use crate::application::events::UiEvent;
use crate::application::net::{
    FileListReply, InboundId, NetCommand, PairingAsk, PairingReply, RequestKind,
};
use crate::domain::error::{AppError, AppResult, ErrorCode};
use crate::domain::pairing::Pin;
use crate::domain::path::RelativePath;
use crate::domain::peer::PeerId;
use crate::domain::transfer::TransferId;

pub(super) const NOT_PAIRED: &str = "페어링이 필요합니다.";

impl Engine {
    pub(super) fn request_file_list(&self, peer: PeerId, path: String) {
        self.send_net(NetCommand::RequestFileList {
            peer: peer.clone(),
            path: path.clone(),
        });
        self.emit(UiEvent::FileListRequestSent {
            peer_id: peer,
            path,
        });
    }

    pub(super) async fn serve_file_list(&self, peer: PeerId, inbound: InboundId, path: String) {
        let listed = if self.trusted.contains(&peer) {
            self.list_shared(&path).await
        } else {
            Err(NOT_PAIRED.to_string())
        };
        let reply = match listed {
            Ok(files) => FileListReply {
                path,
                files,
                error: None,
            },
            Err(error) => FileListReply {
                path,
                files: Vec::new(),
                error: Some(error),
            },
        };
        self.send_net(NetCommand::RespondFileList { inbound, reply });
    }

    async fn list_shared(&self, path: &str) -> Result<Vec<crate::domain::files::FileEntry>, String> {
        let rel = RelativePath::parse(path).map_err(|e| e.message)?;
        let root = self.deps.settings.get().shared_folder;
        self.deps
            .files
            .list_dir(&root, &rel)
            .await
            .map_err(|e| e.message)
    }

    pub(super) fn on_directory_changed(&self, peer: PeerId, inbound: InboundId) {
        self.send_net(NetCommand::RespondDirectoryChanged { inbound });
        if self.trusted.contains(&peer) {
            self.emit(UiEvent::RemoteDirectoryChanged { peer_id: peer });
        }
    }

    pub(super) fn notify_folder_changed(&self) {
        if self.trusted.is_empty() {
            return;
        }
        self.send_net(NetCommand::NotifyDirectoryChanged {
            peers: self.trusted.iter().cloned().collect(),
        });
    }

    pub(super) fn request_pairing(&self, peer: PeerId) -> Pin {
        let pin = Pin::generate(&mut rand::rng());
        self.send_net(NetCommand::RequestPairing {
            peer: peer.clone(),
            ask: PairingAsk {
                device_name: self.deps.settings.get().device_name,
                pin: pin.as_str().to_string(),
            },
        });
        self.emit(UiEvent::PairingSent {
            peer_id: peer,
            pin: pin.as_str().to_string(),
        });
        pin
    }

    pub(super) fn on_pairing_request(&mut self, peer: PeerId, inbound: InboundId, ask: PairingAsk) {
        let Some(pin) = Pin::from_wire(&ask.pin) else {
            self.reject_pairing(inbound, "PIN 형식이 올바르지 않습니다.");
            return;
        };
        let shown = pin.as_str().to_string();
        if let Some((replaced, _)) = self.pending_pairings.insert(peer.clone(), (inbound, pin)) {
            self.reject_pairing(replaced, "새 페어링 요청으로 대체되었습니다.");
        }
        self.device_names.insert(peer.clone(), ask.device_name.clone());
        self.emit(UiEvent::PairingRequest {
            peer_id: peer,
            device_name: ask.device_name,
            pin: shown,
        });
    }

    pub(super) async fn respond_pairing(&mut self, peer: PeerId, approved: bool) -> AppResult<()> {
        let (inbound, _) = self.pending_pairings.remove(&peer).ok_or_else(|| {
            AppError::new(ErrorCode::NotFound, "대기 중인 페어링 요청이 없습니다.")
        })?;
        if !approved {
            self.reject_pairing(inbound, "사용자가 거절했습니다.");
            return Ok(());
        }
        let settings = self.deps.settings.get();
        self.trust(peer.clone()).await;
        self.emit(UiEvent::PairingApproved {
            device_name: self.device_names.get(&peer).cloned().unwrap_or_default(),
            peer_id: peer,
            shared_folder_name: None,
        });
        self.send_net(NetCommand::RespondPairing {
            inbound,
            reply: PairingReply {
                approved: true,
                shared_folder_name: settings.advertised_folder_name(),
                device_name: settings.device_name,
                error: None,
            },
        });
        Ok(())
    }

    fn reject_pairing(&self, inbound: InboundId, reason: &str) {
        self.send_net(NetCommand::RespondPairing {
            inbound,
            reply: PairingReply {
                approved: false,
                device_name: self.deps.settings.get().device_name,
                shared_folder_name: None,
                error: Some(reason.to_string()),
            },
        });
    }

    pub(super) async fn on_pairing_reply(&mut self, peer: PeerId, reply: PairingReply) {
        if !reply.approved {
            self.emit(UiEvent::PairingRejected {
                peer_id: peer,
                error: reply.error,
            });
            return;
        }
        self.trust(peer.clone()).await;
        self.device_names.insert(peer.clone(), reply.device_name.clone());
        self.emit(UiEvent::PairingApproved {
            peer_id: peer,
            device_name: reply.device_name,
            shared_folder_name: reply.shared_folder_name,
        });
    }

    pub(super) async fn load_trusted(&mut self) {
        match self.deps.trust.load().await {
            Ok(peers) => self.trusted.extend(peers),
            Err(e) => eprintln!("[engine] 믿는 기기 목록을 읽지 못했습니다: {e}"),
        }
    }

    async fn trust(&mut self, peer: PeerId) {
        if !self.trusted.insert(peer) {
            return;
        }
        let mut peers: Vec<PeerId> = self.trusted.iter().cloned().collect();
        peers.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        if let Err(e) = self.deps.trust.save(&peers).await {
            eprintln!("[engine] 믿는 기기 목록을 저장하지 못했습니다: {e}");
        }
    }

    pub(super) fn peer_summaries(&self) -> Vec<PeerSummary> {
        let mut ids: Vec<&PeerId> = self
            .connected
            .iter()
            .chain(self.trusted.iter())
            .chain(self.pending_pairings.keys())
            .collect();
        ids.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        ids.dedup();
        ids.into_iter()
            .map(|peer| PeerSummary {
                peer_id: peer.clone(),
                device_name: self.device_names.get(peer).cloned(),
                connected: self.connected.contains(peer),
                trusted: self.trusted.contains(peer),
                pending_pin: self
                    .pending_pairings
                    .get(peer)
                    .map(|(_, pin)| pin.as_str().to_string()),
            })
            .collect()
    }

    pub(super) async fn on_disconnected(&mut self, peer: PeerId) {
        self.connected.remove(&peer);
        self.pending_pairings.remove(&peer);
        self.emit(UiEvent::ConnectionClosed {
            peer_id: peer.clone(),
        });
        self.fail_transfers_of(&peer, "상대 기기와 연결이 끊어졌습니다.").await;
    }

    pub(super) async fn on_request_failed(
        &mut self,
        peer: PeerId,
        kind: RequestKind,
        transfer_id: Option<TransferId>,
        error: String,
    ) {
        match kind {
            RequestKind::FileList => self.emit(UiEvent::FileListError {
                peer_id: peer,
                error,
            }),
            RequestKind::Pairing => self.emit(UiEvent::PairingRejected {
                peer_id: peer,
                error: Some(error),
            }),
            RequestKind::Stream => {
                if let Some(id) = transfer_id {
                    self.fail_transfer(&id, error).await;
                }
            }
            RequestKind::DeviceInfo | RequestKind::DirectoryChanged => {}
        }
    }
}
