use crate::network::behavior::{PairingRequest, PairingResponse, MyBehaviour};
use crate::network::types::PairingRequestMsg;
use crate::network::types::PairingApprovalMsg;
use libp2p::{request_response, Swarm};
use tauri::Emitter;

use super::EngineContext;

impl EngineContext {
    // ─── 페어링 요청 전송 ───

    pub(crate) fn handle_pairing_request_send(&self, swarm: &mut Swarm<MyBehaviour>, req: PairingRequestMsg) {
        let pin: String = (0..6)
            .map(|_| rand::random::<u8>() % 10)
            .map(|n| char::from_digit(n as u32, 10).unwrap())
            .collect();

        println!("🔐 페어링 요청 전송: peer={}, PIN={}", req.peer_id, pin);

        let request = PairingRequest {
            device_name: self.device_name.clone(),
            pin: pin.clone(),
        };
        swarm
            .behaviour_mut()
            .pairing
            .send_request(&req.peer_id, request);

        self.app
            .emit(
                "pairing-sent",
                serde_json::json!({
                    "peer_id": req.peer_id.to_string(),
                    "pin": pin,
                })
                .to_string(),
            )
            .ok();
    }

    // ─── 페어링 승인/거절 ───

    pub(crate) fn handle_pairing_approval(
        &mut self,
        swarm: &mut Swarm<MyBehaviour>,
        approval: PairingApprovalMsg,
    ) {
        let peer_key = approval.peer_id.to_string();
        if let Some((peer_id, channel, _pin)) = self.pending_pairings.remove(&peer_key) {
            // Note: shared_folder_display_name 접근은 비동기지만, 여기서는 동기 컨텍스트
            // 이 핸들러는 handle_pairing_approval_async에서 호출됨
            let response = PairingResponse {
                approved: approval.approved,
                device_name: self.device_name.clone(),
                shared_folder_name: None, // async 버전에서 채워짐
                error: if approval.approved {
                    None
                } else {
                    Some("사용자가 거절했습니다.".to_string())
                },
            };

            if approval.approved {
                self.trusted_peers.insert(peer_id);
                println!("✅ 페어링 승인: {}", peer_id);
            } else {
                println!("❌ 페어링 거절: {}", peer_id);
            }

            swarm
                .behaviour_mut()
                .pairing
                .send_response(channel, response)
                .ok();
        }
    }

    /// 페어링 승인/거절 (비동기 버전 - shared_folder_display_name 접근 필요)
    pub async fn handle_pairing_approval_async(
        &mut self,
        swarm: &mut Swarm<MyBehaviour>,
        approval: PairingApprovalMsg,
    ) {
        let peer_key = approval.peer_id.to_string();
        if let Some((peer_id, channel, _pin)) = self.pending_pairings.remove(&peer_key) {
            let shared_folder_name = if approval.approved
                && self
                    .state
                    .show_folder_name
                    .load(std::sync::atomic::Ordering::SeqCst)
            {
                Some(self.state.shared_folder_display_name.read().await.clone())
            } else {
                None
            };

            let response = PairingResponse {
                approved: approval.approved,
                device_name: self.device_name.clone(),
                shared_folder_name,
                error: if approval.approved {
                    None
                } else {
                    Some("사용자가 거절했습니다.".to_string())
                },
            };

            if approval.approved {
                self.trusted_peers.insert(peer_id);
                println!("✅ 페어링 승인: {}", peer_id);
            } else {
                println!("❌ 페어링 거절: {}", peer_id);
            }

            swarm
                .behaviour_mut()
                .pairing
                .send_response(channel, response)
                .ok();
        }
    }

    // ─── 페어링 Swarm 이벤트 ───

    pub fn handle_pairing_event(
        &mut self,
        _swarm: &mut Swarm<MyBehaviour>,
        event: request_response::Event<PairingRequest, PairingResponse>,
    ) {
        match event {
            request_response::Event::Message { peer, message } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    println!(
                        "🔐 페어링 요청 수신: {} (PIN: {})",
                        request.device_name, request.pin
                    );
                    let peer_key = peer.to_string();
                    self.pending_pairings
                        .insert(peer_key, (peer, channel, request.pin.clone()));

                    self.app
                        .emit(
                            "pairing-request",
                            serde_json::json!({
                                "peer_id": peer.to_string(),
                                "device_name": request.device_name,
                                "pin": request.pin,
                            })
                            .to_string(),
                        )
                        .ok();
                }
                request_response::Message::Response { response, .. } => {
                    if response.approved {
                        self.trusted_peers.insert(peer);
                        println!("✅ 페어링 승인됨: {}", response.device_name);
                        self.app
                            .emit(
                                "pairing-approved",
                                serde_json::json!({
                                    "peer_id": peer.to_string(),
                                    "device_name": response.device_name,
                                    "shared_folder_name": response.shared_folder_name,
                                })
                                .to_string(),
                            )
                            .ok();
                    } else {
                        println!("❌ 페어링 거절됨: {:?}", response.error);
                        self.app
                            .emit(
                                "pairing-rejected",
                                serde_json::json!({
                                    "peer_id": peer.to_string(),
                                    "error": response.error,
                                })
                                .to_string(),
                            )
                            .ok();
                    }
                }
            },
            _ => {}
        }
    }
}
