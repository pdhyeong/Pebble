use crate::network::behavior::{FileTransferRequest, FileTransferResponse};
use crate::network::behavior::MyBehaviour;
use crate::network::types::FileTransferRequestMsg;
use libp2p::{request_response, Swarm};
use std::path::PathBuf;
use tauri::Emitter;

use super::EngineContext;

impl EngineContext {
    // ─── 파일 다운로드 요청 전송 ───

    pub(crate) fn handle_file_transfer_request(
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
}
