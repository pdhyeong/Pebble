use crate::network::behavior::{FileListRequest, FileListResponse};
use crate::network::behavior::MyBehaviour;
use crate::network::helpers::list_files;
use crate::network::types::FileListRequestMsg;
use libp2p::{request_response, Swarm};
use tauri::Emitter;

use super::EngineContext;

impl EngineContext {
    // ─── 파일 목록 요청 전송 ───

    pub(crate) fn handle_file_list_request(&self, swarm: &mut Swarm<MyBehaviour>, req: FileListRequestMsg) {
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
}
