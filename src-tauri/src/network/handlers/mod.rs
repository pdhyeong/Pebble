// 네트워크 이벤트 핸들러 모듈
// 도메인별로 분리된 서브모듈에서 EngineContext의 impl 블록을 확장

mod directory;
mod discovery;
mod file_list;
mod file_stream;
mod file_transfer;
mod pairing;

use crate::network::behavior::{MyBehaviour, PairingResponse};
use crate::network::types::{
    IncomingTransferState, OutgoingTransferState, P2pCommand,
};
use libp2p::{request_response, Multiaddr, PeerId, Swarm};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

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
}
