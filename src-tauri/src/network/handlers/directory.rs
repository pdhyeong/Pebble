use crate::network::behavior::{DirectoryChangedRequest, DirectoryChangedResponse};
use crate::network::behavior::MyBehaviour;
use libp2p::{request_response, Swarm};
use tauri::Emitter;

use super::EngineContext;

impl EngineContext {
    // ─── 디렉토리 변경 알림 전송 (내 폴더가 변경됐을 때) ───

    pub(crate) fn handle_notify_directory_changed(&self, swarm: &mut Swarm<MyBehaviour>) {
        for peer_id in &self.trusted_peers {
            swarm
                .behaviour_mut()
                .directory_changed
                .send_request(peer_id, DirectoryChangedRequest {});
        }
        if !self.trusted_peers.is_empty() {
            println!(
                "[P2P] 디렉토리 변경 알림 전송: {}개 피어",
                self.trusted_peers.len()
            );
        }
    }

    // ─── 디렉토리 변경 알림 수신 (상대방 폴더가 변경됐을 때) ───

    pub fn handle_directory_changed_event(
        &self,
        swarm: &mut Swarm<MyBehaviour>,
        event: request_response::Event<DirectoryChangedRequest, DirectoryChangedResponse>,
    ) {
        if let request_response::Event::Message { peer, message } = event {
            if let request_response::Message::Request { channel, .. } = message {
                if self.trusted_peers.contains(&peer) {
                    swarm
                        .behaviour_mut()
                        .directory_changed
                        .send_response(channel, DirectoryChangedResponse {})
                        .ok();

                    self.app
                        .emit(
                            "remote-directory-changed",
                            serde_json::json!({ "peer_id": peer.to_string() }).to_string(),
                        )
                        .ok();

                    println!("[P2P] {} 에서 디렉토리 변경 알림 수신", peer);
                }
            }
        }
    }
}
