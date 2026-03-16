use crate::network::behavior::{
    FileTransferStreamMsg, FileTransferStreamRequest, FileTransferStreamResponse, MyBehaviour,
};
use crate::network::helpers::get_quic_address;
use crate::network::types::{
    CancelTransferMsg, FileTransferStreamRequestMsg, IncomingTransferState, OutgoingTransferState,
    CHUNK_SIZE,
};
use libp2p::{request_response, PeerId, Swarm};
use std::fs::File;
use std::io::{Read, Write};
use tauri::Emitter;
use uuid::Uuid;

use super::{record_activity, EngineContext};

impl EngineContext {
    // ─── 파일 업로드(스트림) 요청 전송 ───

    pub(crate) fn handle_send_file_stream(
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

    // ─── 전송 취소 ───

    pub(crate) fn handle_cancel_transfer(
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
}
