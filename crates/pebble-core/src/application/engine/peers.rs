use std::time::{Duration, Instant};

use super::{Engine, PeerSummary, PendingPairing};
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
pub const PAIRING_EXPIRY: Duration = Duration::from_secs(170);
pub const MAX_DEVICE_NAME_CHARS: usize = 64;

pub(super) fn clip_device_name(name: String) -> String {
    name.chars()
        .filter(|c| !c.is_control())
        .take(MAX_DEVICE_NAME_CHARS)
        .collect()
}

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

    async fn list_shared(
        &self,
        path: &str,
    ) -> Result<Vec<crate::domain::files::FileEntry>, String> {
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
        let pending = PendingPairing {
            inbound,
            pin,
            since: Instant::now(),
        };
        if let Some(replaced) = self.pending_pairings.insert(peer.clone(), pending) {
            self.reject_pairing(replaced.inbound, "새 페어링 요청으로 대체되었습니다.");
        }
        let device_name = clip_device_name(ask.device_name);
        self.device_names.insert(peer.clone(), device_name.clone());
        self.emit(UiEvent::PairingRequest {
            peer_id: peer,
            device_name,
            pin: shown,
        });
    }

    pub(super) async fn respond_pairing(&mut self, peer: PeerId, approved: bool) -> AppResult<()> {
        let pending = self.pending_pairings.remove(&peer).ok_or_else(|| {
            AppError::new(ErrorCode::NotFound, "대기 중인 페어링 요청이 없습니다.")
        })?;
        let inbound = pending.inbound;
        if pending.since.elapsed() >= PAIRING_EXPIRY {
            self.reject_pairing(inbound, "시간이 지나 페어링 요청이 만료되었습니다.");
            return Err(AppError::new(
                ErrorCode::NotFound,
                "시간이 지나 페어링 요청이 만료되었습니다. 상대 기기에서 다시 요청하세요.",
            ));
        }
        if !approved {
            self.reject_pairing(inbound, "사용자가 거절했습니다.");
            return Ok(());
        }
        let settings = self.deps.settings.get();
        if let Err(e) = self.trust(peer.clone()).await {
            self.reject_pairing(inbound, "상대 기기가 페어링을 저장하지 못했습니다.");
            return Err(e);
        }
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

    pub(super) fn require_trusted(&self, peer: &PeerId) -> AppResult<()> {
        if self.trusted.contains(peer) {
            Ok(())
        } else {
            Err(AppError::new(ErrorCode::AccessDenied, NOT_PAIRED))
        }
    }

    pub(super) fn expire_pairings(&mut self) {
        let expired: Vec<PeerId> = self
            .pending_pairings
            .iter()
            .filter(|(_, p)| p.since.elapsed() >= PAIRING_EXPIRY)
            .map(|(peer, _)| peer.clone())
            .collect();
        for peer in expired {
            if let Some(pending) = self.pending_pairings.remove(&peer) {
                self.reject_pairing(pending.inbound, "시간이 지나 페어링 요청이 만료되었습니다.");
            }
        }
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
        if let Err(e) = self.trust(peer.clone()).await {
            self.emit(UiEvent::PairingRejected {
                peer_id: peer,
                error: Some(e.message),
            });
            return;
        }
        let device_name = clip_device_name(reply.device_name);
        self.device_names.insert(peer.clone(), device_name.clone());
        self.emit(UiEvent::PairingApproved {
            peer_id: peer,
            device_name,
            shared_folder_name: reply.shared_folder_name,
        });
    }

    pub(super) async fn load_trusted(&mut self) {
        match self.deps.trust.load().await {
            Ok(peers) => self.trusted.extend(peers),
            Err(e) => {
                self.trust_unreadable = true;
                eprintln!("[engine] 믿는 기기 목록을 읽지 못했습니다. 파일을 덮어쓰지 않도록 새 페어링을 막습니다: {e}");
            }
        }
    }

    async fn trust(&mut self, peer: PeerId) -> AppResult<()> {
        if self.trusted.contains(&peer) {
            return Ok(());
        }
        if self.trust_unreadable {
            return Err(AppError::new(
                ErrorCode::Io,
                "믿는 기기 목록 파일(trusted-peers.json)을 읽지 못해 새 페어링을 저장하지 않습니다. 파일을 고치거나 지운 뒤 다시 시작하세요.",
            ));
        }
        let mut peers: Vec<PeerId> = self.trusted.iter().cloned().collect();
        peers.push(peer.clone());
        peers.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        self.deps.trust.save(&peers).await?;
        self.trusted.insert(peer);
        Ok(())
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
                    .map(|p| p.pin.as_str().to_string()),
            })
            .collect()
    }

    pub(super) async fn on_disconnected(&mut self, peer: PeerId) {
        self.connected.remove(&peer);
        self.pending_pairings.remove(&peer);
        self.emit(UiEvent::ConnectionClosed {
            peer_id: peer.clone(),
        });
        self.fail_transfers_of(&peer, "상대 기기와 연결이 끊어졌습니다.")
            .await;
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
