// 네트워크 이벤트 핸들러 모듈
// run_p2p_engine의 820줄 God Function에서 분리된 이벤트 핸들러들

use crate::network::behavior::{
    DeviceInfoRequest, DirectoryChangedRequest, DirectoryChangedResponse, FileListRequest,
    FileListResponse, FileTransferRequest, FileTransferResponse, FileTransferStreamMsg,
    FileTransferStreamRequest, FileTransferStreamResponse, PairingRequest, PairingResponse,
};
use crate::network::helpers::{get_quic_address, list_files};
use crate::network::types::{
    CancelTransferMsg, FileListRequestMsg, FileTransferRequestMsg, FileTransferStreamRequestMsg,
    IncomingTransferState, OutgoingTransferState, P2pCommand, PairingApprovalMsg,
    PairingRequestMsg, CHUNK_SIZE,
};
use libp2p::{request_response, Multiaddr, PeerId, Swarm};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::network::behavior::MyBehaviour;

/// P2P 엔진의 공유 상태를 관리하는 컨텍스트
/// 기존에 run_p2p_engine 함수 내 로컬 변수로 흩어져 있던 상태를 구조체로 통합
pub struct EngineContext {
    pub app: AppHandle,
    pub shared_folder: PathBuf,
    pub device_name: String,
    pub state: Arc<crate::state::P2pState>,
    pub outgoing_transfers: HashMap<String, OutgoingTransferState>,
    pub incoming_transfers: HashMap<String, IncomingTransferState>,
    pub req_id_to_transfer_id: HashMap<request_response::RequestId, String>,
    pub peer_addresses: HashMap<PeerId, Vec<Multiaddr>>,
    pub trusted_peers: HashSet<PeerId>,
    pub pending_pairings: HashMap<
        String,
        (
            PeerId,
            request_response::ResponseChannel<PairingResponse>,
            String,
        ),
    >,
}

/// 활동 기록 저장 헬퍼 함수
pub async fn record_activity(
    state: &Arc<crate::state::P2pState>,
    transfer_type: &str,
    file_name: String,
    peer_id: String,
    size: u64,
    status: &str,
    speed: Option<f64>,
) {
    let activity = crate::commands::activity::ActivityRecord {
        id: Uuid::new_v4().to_string(),
        transfer_type: transfer_type.to_string(),
        file_name,
        peer_id,
        device_name: None,
        size,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        status: status.to_string(),
        speed,
    };

    let mut history = state.activity_history.lock().await;
    history.push(activity);

    // 최대 100개까지만 유지
    if history.len() > 100 {
        history.remove(0);
    }
}

impl EngineContext {
    /// 통합 커맨드 처리
    pub fn handle_command(&mut self, cmd: P2pCommand, swarm: &mut Swarm<MyBehaviour>) -> bool {
        match cmd {
            P2pCommand::Shutdown => {
                println!("🛑 P2P 엔진 종료 신호 수신");
                self.app
                    .emit("p2p-stopped", "P2P 엔진이 종료되었습니다.")
                    .ok();
                return true; // 엔진 종료
            }
            P2pCommand::Dial(addr_str) => self.handle_dial(swarm, addr_str),
            P2pCommand::RequestFileList(req) => self.handle_file_list_request(swarm, req),
            P2pCommand::RequestFileTransfer(req) => self.handle_file_transfer_request(swarm, req),
            P2pCommand::RequestFileStream(req) => self.handle_send_file_stream(swarm, req),
            P2pCommand::UpdateSharedFolder(new_path) => {
                println!("📁 공유 폴더 경로 변경: {:?}", new_path);
                self.shared_folder = new_path;
                self.app
                    .emit(
                        "shared-folder-changed",
                        self.shared_folder.display().to_string(),
                    )
                    .ok();
            }
            P2pCommand::RequestPairing(req) => self.handle_pairing_request_send(swarm, req),
            P2pCommand::ApprovePairing(approval) => self.handle_pairing_approval(swarm, approval),
            P2pCommand::CancelTransfer(cancel_req) => {
                self.handle_cancel_transfer(swarm, cancel_req)
            }
            P2pCommand::NotifyDirectoryChanged => {
                self.handle_notify_directory_changed(swarm)
            }
        }
        false // 계속 실행
    }

    // ─── Dial ───

    fn handle_dial(&self, swarm: &mut Swarm<MyBehaviour>, addr_str: String) {
        println!("📞 Dial 요청 수신: {}", addr_str);
        match addr_str.parse::<Multiaddr>() {
            Ok(addr) => match swarm.dial(addr.clone()) {
                Ok(_) => {
                    println!("Dial connecting~: {}", addr);
                    self.app.emit("dial-started", addr.to_string()).ok();
                }
                Err(e) => {
                    println!("Dial failed: {}", e);
                    self.app.emit("dial-failed", format!("{}", e)).ok();
                }
            },
            Err(e) => {
                println!("dial failed: {}", e);
                self.app
                    .emit("dial-failed", format!("주소 파싱 실패: {}", e))
                    .ok();
            }
        }
    }

    // ─── 파일 목록 요청 전송 ───

    fn handle_file_list_request(&self, swarm: &mut Swarm<MyBehaviour>, req: FileListRequestMsg) {
        println!("📂 파일 목록 요청: peer={}, path={}", req.peer_id, req.path);
        let request = FileListRequest {
            path: req.path.clone(),
        };
        swarm
            .behaviour_mut()
            .file_list
            .send_request(&req.peer_id, request);
        self.app
            .emit(
                "file-list-request-sent",
                format!("{}:{}", req.peer_id, req.path),
            )
            .ok();
    }

    // ─── 파일 다운로드 요청 전송 ───

    fn handle_file_transfer_request(
        &self,
        swarm: &mut Swarm<MyBehaviour>,
        req: FileTransferRequestMsg,
    ) {
        println!(
            "📥 파일 다운로드 요청: peer={}, path={}",
            req.peer_id, req.path
        );
        let request = FileTransferRequest {
            path: req.path.clone(),
        };
        swarm
            .behaviour_mut()
            .file_transfer
            .send_request(&req.peer_id, request);
        self.app
            .emit(
                "file-download-started",
                format!("{}:{}", req.peer_id, req.path),
            )
            .ok();
    }

    // ─── 파일 업로드(스트림) 요청 전송 ───

    fn handle_send_file_stream(
        &mut self,
        swarm: &mut Swarm<MyBehaviour>,
        req: FileTransferStreamRequestMsg,
    ) {
        let file_name = req
            .file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        println!(
            "📤 파일 업로드 시작 요청: peer={}, file={}",
            req.peer_id, file_name
        );

        // QUIC 주소로 먼저 연결 시도 (대용량 파일에 효율적)
        if let Some(addrs) = self.peer_addresses.get(&req.peer_id) {
            if let Some(quic_addr) = get_quic_address(addrs) {
                println!("🚀 QUIC 연결 시도: {}", quic_addr);
                if let Err(e) = swarm.dial(quic_addr.clone()) {
                    println!("⚠️ QUIC dial 실패 (기존 연결 사용): {}", e);
                }
            }
        }

        match File::open(&req.file_path) {
            Ok(file) => {
                let metadata = file.metadata().ok();
                let total_size = metadata.map(|m| m.len()).unwrap_or(0);
                let total_chunks = (total_size as f64 / CHUNK_SIZE as f64).ceil() as u32;
                let transfer_id = Uuid::new_v4().to_string();

                let msg = FileTransferStreamMsg::Init {
                    file_name: file_name.clone(),
                    total_size,
                    total_chunks,
                    transfer_id: transfer_id.clone(),
                    remote_path: req.remote_path.clone(),
                };

                let request = FileTransferStreamRequest { msg };
                let req_id = swarm
                    .behaviour_mut()
                    .file_stream
                    .send_request(&req.peer_id, request);

                let state = OutgoingTransferState {
                    file,
                    file_name: file_name.clone(),
                    total_size,
                    total_chunks,
                    current_chunk: 0,
                    peer_id: req.peer_id,
                    transfer_id: transfer_id.clone(),
                    start_time: std::time::Instant::now(),
                };

                self.outgoing_transfers.insert(transfer_id.clone(), state);
                self.req_id_to_transfer_id
                    .insert(req_id, transfer_id.clone());

                self.app
                    .emit(
                        "file-upload-started",
                        serde_json::json!({
                            "peer_id": req.peer_id.to_string(),
                            "file_name": file_name,
                            "transfer_id": transfer_id,
                            "total_chunks": total_chunks,
                            "total_size": total_size,
                        })
                        .to_string(),
                    )
                    .ok();
            }
            Err(e) => {
                println!("❌ 파일 열기 실패: {}", e);
                self.app
                    .emit(
                        "file-upload-error",
                        serde_json::json!({
                            "error": format!("파일 열기 실패: {}", e)
                        })
                        .to_string(),
                    )
                    .ok();
            }
        }
    }

    // ─── 페어링 요청 전송 ───

    fn handle_pairing_request_send(&self, swarm: &mut Swarm<MyBehaviour>, req: PairingRequestMsg) {
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

    fn handle_pairing_approval(
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

    // ─── 전송 취소 ───

    fn handle_cancel_transfer(
        &mut self,
        swarm: &mut Swarm<MyBehaviour>,
        cancel_req: CancelTransferMsg,
    ) {
        let transfer_id = cancel_req.transfer_id.clone();
        println!("🛑 전송 취소 요청: {}", transfer_id);

        // 발신 전송 취소 (Sender가 취소)
        if let Some(state) = self.outgoing_transfers.remove(&transfer_id) {
            let req = FileTransferStreamRequest {
                msg: FileTransferStreamMsg::Cancel {
                    transfer_id: transfer_id.clone(),
                },
            };
            swarm
                .behaviour_mut()
                .file_stream
                .send_request(&state.peer_id, req);

            self.app
                .emit(
                    "transfer-cancelled",
                    serde_json::json!({
                        "transfer_id": transfer_id,
                        "peer_id": state.peer_id.to_string(),
                        "file_name": state.file_name,
                        "reason": "user_cancelled"
                    })
                    .to_string(),
                )
                .ok();
        }

        // 수신 전송 취소 (Receiver가 취소)
        if let Some(state) = self.incoming_transfers.remove(&transfer_id) {
            if let Err(e) = std::fs::remove_file(&state.saved_path) {
                println!("⚠️ 부분 파일 삭제 실패: {}", e);
            }

            let req = FileTransferStreamRequest {
                msg: FileTransferStreamMsg::Cancel {
                    transfer_id: transfer_id.clone(),
                },
            };
            swarm
                .behaviour_mut()
                .file_stream
                .send_request(&state.peer_id, req);

            self.app
                .emit(
                    "transfer-cancelled",
                    serde_json::json!({
                        "transfer_id": transfer_id,
                        "peer_id": state.peer_id.to_string(),
                        "file_name": state.file_name,
                        "reason": "user_cancelled"
                    })
                    .to_string(),
                )
                .ok();
        }
    }

    // ═══════════════════════════════════════════════════
    // Swarm 이벤트 핸들러들
    // ═══════════════════════════════════════════════════

    // ─── mDNS ───

    pub fn handle_mdns_discovered(&mut self, list: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, addr) in list {
            let addr_str = addr.to_string();
            self.peer_addresses
                .entry(peer_id)
                .or_insert_with(Vec::new)
                .push(addr.clone());

            if !addr_str.contains("quic") {
                println!("peer-found ID: {}, 주소: {}", peer_id, addr);
                let payload = format!("{}:{}", peer_id, addr);
                self.app.emit("peer-found", payload).ok();
            } else {
                println!("📡 QUIC 주소 저장 (파일 전송용): {} -> {}", peer_id, addr);
            }
        }
    }

    pub fn handle_mdns_expired(&self, list: Vec<(PeerId, Multiaddr)>) {
        for (peer_id, _addr) in list {
            self.app.emit("peer-expired", peer_id.to_string()).ok();
        }
    }

    // ─── Identify ───

    pub fn handle_identify(&self, info: libp2p::identify::Info) {
        self.app.emit("peer-info", info.agent_version).ok();
    }

    // ─── Ping ───

    pub fn handle_ping(
        &self,
        peer: PeerId,
        result: Result<std::time::Duration, libp2p::ping::Failure>,
    ) {
        if let Ok(rtt) = result {
            let payload = format!("{}:{}", peer, rtt.as_millis());
            self.app.emit("ping-success", payload).ok();
        }
    }

    // ─── 파일 목록 이벤트 ───

    pub fn handle_file_list_event(
        &self,
        swarm: &mut Swarm<MyBehaviour>,
        event: request_response::Event<FileListRequest, FileListResponse>,
    ) {
        match event {
            request_response::Event::Message { peer, message } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    if !self.trusted_peers.contains(&peer) {
                        println!("⛔ 페어링되지 않은 피어의 파일 목록 요청 거부: {}", peer);
                        let response = FileListResponse {
                            path: request.path,
                            files: vec![],
                            error: Some("페어링이 필요합니다.".to_string()),
                        };
                        swarm
                            .behaviour_mut()
                            .file_list
                            .send_response(channel, response)
                            .ok();
                    } else {
                        let response = list_files(&self.shared_folder, &request.path);
                        swarm
                            .behaviour_mut()
                            .file_list
                            .send_response(channel, response)
                            .ok();
                    }
                }
                request_response::Message::Response { response, .. } => {
                    let payload = serde_json::json!({
                        "peer_id": peer.to_string(),
                        "path": response.path,
                        "files": response.files,
                        "error": response.error,
                    });
                    self.app.emit("remote-files", payload.to_string()).ok();
                }
            },
            request_response::Event::OutboundFailure { peer, error, .. } => {
                self.app
                    .emit("file-list-error", format!("{}:{:?}", peer, error))
                    .ok();
            }
            _ => {}
        }
    }

    // ─── 파일 전송(다운로드) 이벤트 ───

    pub fn handle_file_transfer_event(
        &self,
        swarm: &mut Swarm<MyBehaviour>,
        event: request_response::Event<FileTransferRequest, FileTransferResponse>,
    ) {
        match event {
            request_response::Event::Message { peer, message } => match message {
                request_response::Message::Request {
                    request, channel, ..
                } => {
                    if !self.trusted_peers.contains(&peer) {
                        println!("⛔ 페어링되지 않은 피어의 파일 전송 요청 거부: {}", peer);
                        let response = FileTransferResponse {
                            path: request.path,
                            data: None,
                            error: Some("페어링이 필요합니다.".to_string()),
                        };
                        swarm
                            .behaviour_mut()
                            .file_transfer
                            .send_response(channel, response)
                            .ok();
                    } else {
                        let file_path = self
                            .shared_folder
                            .join(request.path.trim_start_matches('/'));
                        let response = match std::fs::read(&file_path) {
                            Ok(data) => FileTransferResponse {
                                path: request.path,
                                data: Some(data),
                                error: None,
                            },
                            Err(e) => FileTransferResponse {
                                path: request.path,
                                data: None,
                                error: Some(format!("{}", e)),
                            },
                        };
                        swarm
                            .behaviour_mut()
                            .file_transfer
                            .send_response(channel, response)
                            .ok();
                    }
                }
                request_response::Message::Response { response, .. } => {
                    println!(
                        "📦 파일 전송 응답 수신: peer={}, path={}",
                        peer, response.path
                    );
                    if let Some(error) = response.error {
                        println!("❌ 파일 다운로드 에러: {}", error);
                        self.app
                            .emit(
                                "file-download-error",
                                serde_json::json!({"peer_id": peer.to_string(), "error": error})
                                    .to_string(),
                            )
                            .ok();
                    } else if let Some(data) = response.data {
                        println!("✅ 파일 데이터 수신: {} bytes", data.len());
                        let download_folder = dirs::download_dir()
                            .unwrap_or_else(|| dirs::home_dir().unwrap_or(PathBuf::from(".")));
                        let file_name = std::path::Path::new(&response.path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or("file".to_string());
                        let save_path = download_folder.join(&file_name);
                        match std::fs::write(&save_path, data) {
                            Ok(_) => {
                                println!("💾 파일 저장 완료: {:?}", save_path);
                                self.app
                                    .emit(
                                        "file-download-complete",
                                        serde_json::json!({"peer_id": peer.to_string(), "saved_path": save_path, "size": 0})
                                            .to_string(),
                                    )
                                    .ok();
                            }
                            Err(e) => {
                                println!("❌ 파일 저장 실패: {}", e);
                                self.app
                                    .emit(
                                        "file-download-error",
                                        serde_json::json!({"peer_id": peer.to_string(), "error": format!("{}", e)})
                                            .to_string(),
                                    )
                                    .ok();
                            }
                        }
                    }
                }
            },
            _ => {}
        }
    }

    // ─── 파일 스트림(청크) 이벤트: Request 수신 (Receiver 측) ───

    pub async fn handle_file_stream_request(
        &mut self,
        swarm: &mut Swarm<MyBehaviour>,
        peer: PeerId,
        request: FileTransferStreamRequest,
        channel: request_response::ResponseChannel<FileTransferStreamResponse>,
    ) {
        // 페어링되지 않은 피어는 차단
        if !self.trusted_peers.contains(&peer) {
            println!("⛔ 페어링되지 않은 피어의 파일 업로드 거부: {}", peer);
            let transfer_id = match &request.msg {
                FileTransferStreamMsg::Init { transfer_id, .. } => transfer_id.clone(),
                FileTransferStreamMsg::Chunk { transfer_id, .. } => transfer_id.clone(),
                FileTransferStreamMsg::Cancel { transfer_id } => transfer_id.clone(),
            };
            let res = FileTransferStreamResponse {
                transfer_id,
                success: false,
                error: Some("페어링이 필요합니다.".to_string()),
            };
            swarm
                .behaviour_mut()
                .file_stream
                .send_response(channel, res)
                .ok();
            return;
        }

        match request.msg {
            FileTransferStreamMsg::Init {
                file_name,
                total_size,
                total_chunks,
                transfer_id,
                remote_path,
            } => {
                println!("📥 파일 업로드 init: {}", file_name);
                let save_dir = if remote_path.is_empty() || remote_path == "/" {
                    self.shared_folder.clone()
                } else {
                    self.shared_folder.join(remote_path.trim_start_matches('/'))
                };

                match std::fs::create_dir_all(&save_dir) {
                    Ok(_) => {
                        let save_path = save_dir.join(&file_name);
                        match File::create(&save_path) {
                            Ok(file) => {
                                let transfer_state = IncomingTransferState {
                                    file,
                                    file_name: file_name.clone(),
                                    saved_path: save_path.clone(),
                                    total_chunks,
                                    received_chunks: 0,
                                    peer_id: peer,
                                    start_time: std::time::Instant::now(),
                                    total_size,
                                };

                                self.app
                                    .emit(
                                        "file-download-started",
                                        serde_json::json!({
                                            "transfer_id": transfer_id,
                                            "peer_id": peer.to_string(),
                                            "file_name": file_name,
                                            "total_size": total_size
                                        })
                                        .to_string(),
                                    )
                                    .ok();

                                self.incoming_transfers
                                    .insert(transfer_id.clone(), transfer_state);

                                let res = FileTransferStreamResponse {
                                    transfer_id,
                                    success: true,
                                    error: None,
                                };
                                swarm
                                    .behaviour_mut()
                                    .file_stream
                                    .send_response(channel, res)
                                    .ok();
                            }
                            Err(e) => {
                                let res = FileTransferStreamResponse {
                                    transfer_id,
                                    success: false,
                                    error: Some(format!("파일 생성 실패: {}", e)),
                                };
                                swarm
                                    .behaviour_mut()
                                    .file_stream
                                    .send_response(channel, res)
                                    .ok();
                            }
                        }
                    }
                    Err(e) => {
                        let res = FileTransferStreamResponse {
                            transfer_id,
                            success: false,
                            error: Some(format!("폴더 생성 실패: {}", e)),
                        };
                        swarm
                            .behaviour_mut()
                            .file_stream
                            .send_response(channel, res)
                            .ok();
                    }
                }
            }
            FileTransferStreamMsg::Chunk {
                transfer_id,
                chunk_index: _,
                data,
            } => {
                if let Some(transfer_state) = self.incoming_transfers.get_mut(&transfer_id) {
                    if let Err(e) = transfer_state.file.write_all(&data) {
                        println!("❌ 파일 쓰기 실패: {}", e);
                        let res = FileTransferStreamResponse {
                            transfer_id,
                            success: false,
                            error: Some(e.to_string()),
                        };
                        swarm
                            .behaviour_mut()
                            .file_stream
                            .send_response(channel, res)
                            .ok();
                        return;
                    }
                    transfer_state.received_chunks += 1;

                    let progress = (transfer_state.received_chunks as f64
                        / transfer_state.total_chunks as f64)
                        * 100.0;
                    self.app
                        .emit(
                            "file-download-progress",
                            serde_json::json!({
                                "transfer_id": transfer_id,
                                "peer_id": peer.to_string(),
                                "file_name": transfer_state.file_name,
                                "progress": progress,
                                "current": transfer_state.received_chunks,
                                "total": transfer_state.total_chunks
                            })
                            .to_string(),
                        )
                        .ok();

                    if transfer_state.received_chunks >= transfer_state.total_chunks {
                        println!("✅ 파일 수신 완료: {:?}", transfer_state.saved_path);
                        
                        let res = FileTransferStreamResponse {
                            transfer_id: transfer_id.clone(),
                            success: true,
                            error: None,
                        };
                        swarm
                            .behaviour_mut()
                            .file_stream
                            .send_response(channel, res)
                            .ok();

                        self.app
                            .emit(
                                "file-download-complete",
                                serde_json::json!({
                                    "transfer_id": transfer_id.clone(),
                                    "peer_id": peer.to_string(),
                                    "saved_path": transfer_state.saved_path.clone(),
                                })
                                .to_string(),
                            )
                            .ok();

                        self.app
                            .emit(
                                "file-received",
                                serde_json::json!({
                                    "peer_id": peer.to_string(),
                                    "file_name": transfer_state.file_name,
                                    "saved_path": transfer_state.saved_path,
                                })
                                .to_string(),
                            )
                            .ok();

                        let duration = transfer_state.start_time.elapsed().as_secs_f64();
                        let speed = if duration > 0.0 {
                            Some(transfer_state.total_size as f64 / duration)
                        } else {
                            None
                        };
                        let file_name = transfer_state.file_name.clone();
                        let total_size = transfer_state.total_size;

                        self.incoming_transfers.remove(&transfer_id);

                        record_activity(
                            &self.state,
                            "download",
                            file_name,
                            peer.to_string(),
                            total_size,
                            "completed",
                            speed,
                        )
                        .await;
                    } else {
                        let res = FileTransferStreamResponse {
                            transfer_id,
                            success: true,
                            error: None,
                        };
                        swarm
                            .behaviour_mut()
                            .file_stream
                            .send_response(channel, res)
                            .ok();
                    }
                } else {
                    let res = FileTransferStreamResponse {
                        transfer_id,
                        success: false,
                        error: Some("Transfer Session Not Found".to_string()),
                    };
                    swarm
                        .behaviour_mut()
                        .file_stream
                        .send_response(channel, res)
                        .ok();
                }
            }
            FileTransferStreamMsg::Cancel { transfer_id } => {
                println!("🛑 전송 취소 수신: {}", transfer_id);

                if let Some(state) = self.incoming_transfers.remove(&transfer_id) {
                    if let Err(e) = std::fs::remove_file(&state.saved_path) {
                        println!("⚠️ 부분 파일 삭제 실패: {}", e);
                    }
                    self.app
                        .emit(
                            "transfer-cancelled",
                            serde_json::json!({
                                "transfer_id": transfer_id,
                                "peer_id": peer.to_string(),
                                "file_name": state.file_name,
                                "reason": "sender_cancelled"
                            })
                            .to_string(),
                        )
                        .ok();
                }

                if self.outgoing_transfers.remove(&transfer_id).is_some() {
                    self.app
                        .emit(
                            "transfer-cancelled",
                            serde_json::json!({
                                "transfer_id": transfer_id,
                                "peer_id": peer.to_string(),
                                "reason": "receiver_cancelled"
                            })
                            .to_string(),
                        )
                        .ok();
                }

                let res = FileTransferStreamResponse {
                    transfer_id,
                    success: true,
                    error: None,
                };
                swarm
                    .behaviour_mut()
                    .file_stream
                    .send_response(channel, res)
                    .ok();
            }
        }
    }

    // ─── 파일 스트림(청크) 이벤트: Response 수신 (Sender 측) ───

    pub async fn handle_file_stream_response(
        &mut self,
        swarm: &mut Swarm<MyBehaviour>,
        response: FileTransferStreamResponse,
        request_id: request_response::RequestId,
    ) {
        if let Some(transfer_id) = self.req_id_to_transfer_id.remove(&request_id) {
            if !response.success {
                println!("❌ 전송 중단 (상대방 오류): {:?}", response.error);
                self.app
                    .emit(
                        "file-upload-error",
                        serde_json::json!({"error": response.error, "transfer_id": transfer_id})
                            .to_string(),
                    )
                    .ok();
                self.outgoing_transfers.remove(&transfer_id);
                return;
            }

            let mut finished = false;
            let mut file_name = String::new();
            let mut total_size = 0u64;
            let mut peer_id_str = String::new();
            let mut speed: Option<f64> = None;

            if let Some(transfer_state) = self.outgoing_transfers.get_mut(&transfer_id) {
                if transfer_state.current_chunk < transfer_state.total_chunks {
                    let mut buffer = vec![0u8; CHUNK_SIZE];
                    match transfer_state.file.read(&mut buffer) {
                        Ok(n) => {
                            if n == 0 {
                                finished = true;
                            } else {
                                buffer.truncate(n);
                                let req = FileTransferStreamRequest {
                                    msg: FileTransferStreamMsg::Chunk {
                                        transfer_id: transfer_id.clone(),
                                        chunk_index: transfer_state.current_chunk,
                                        data: buffer,
                                    },
                                };
                                let new_req_id = swarm
                                    .behaviour_mut()
                                    .file_stream
                                    .send_request(&transfer_state.peer_id, req);
                                self.req_id_to_transfer_id
                                    .insert(new_req_id, transfer_id.clone());

                                transfer_state.current_chunk += 1;
                                let progress = (transfer_state.current_chunk as f64
                                    / transfer_state.total_chunks as f64)
                                    * 100.0;
                                self.app
                                    .emit(
                                        "file-upload-progress",
                                        serde_json::json!({
                                            "transfer_id": transfer_id,
                                            "progress": progress,
                                            "current": transfer_state.current_chunk,
                                            "total": transfer_state.total_chunks
                                        })
                                        .to_string(),
                                    )
                                    .ok();
                            }
                        }
                        Err(e) => {
                            println!("❌ 파일 읽기 에러: {}", e);
                            finished = true;
                        }
                    }
                } else {
                    finished = true;
                }

                if finished {
                    println!("✅ 파일 전송 완료: {}", transfer_state.file_name);
                    self.app
                        .emit(
                            "file-upload-complete",
                            serde_json::json!({
                                "transfer_id": transfer_id,
                                "file_name": transfer_state.file_name
                            })
                            .to_string(),
                        )
                        .ok();

                    let duration = transfer_state.start_time.elapsed().as_secs_f64();
                    speed = if duration > 0.0 {
                        Some(transfer_state.total_size as f64 / duration)
                    } else {
                        None
                    };
                    file_name = transfer_state.file_name.clone();
                    total_size = transfer_state.total_size;
                    peer_id_str = transfer_state.peer_id.to_string();
                }
            }

            if finished {
                self.outgoing_transfers.remove(&transfer_id);
                record_activity(
                    &self.state,
                    "upload",
                    file_name,
                    peer_id_str,
                    total_size,
                    "completed",
                    speed,
                )
                .await;
            }
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

    // ─── 연결 이벤트 ───

    pub fn handle_connection_established(&self, swarm: &mut Swarm<MyBehaviour>, peer_id: PeerId) {
        println!("connection-success PeerId: {}", peer_id);
        let request = DeviceInfoRequest {};
        swarm
            .behaviour_mut()
            .device_info
            .send_request(&peer_id, request);
        self.app
            .emit("connection-success", peer_id.to_string())
            .ok();
    }

    pub fn handle_connection_closed(&self, peer_id: PeerId) {
        self.app.emit("connection-closed", peer_id.to_string()).ok();
    }

    pub fn handle_new_listen_addr(&self, address: &Multiaddr) {
        if !address.to_string().contains("127.0.0.1") {
            println!("listening address: {}", address);
            self.app.emit("listening-on", address.to_string()).ok();
        }
    }

    // ─── 디렉토리 변경 알림 전송 (내 폴더가 변경됐을 때) ───

    fn handle_notify_directory_changed(&self, swarm: &mut Swarm<MyBehaviour>) {
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
