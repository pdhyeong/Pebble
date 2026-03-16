// 상태 관리 모듈
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::network::P2pCommand;

/// 로컬 파일 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalFileInfo {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

/// P2P 상태 관리
pub struct P2pState {
    pub is_running: AtomicBool,
    /// 통합 커맨드 채널 (9개 → 1개)
    pub command_sender: Mutex<Option<mpsc::UnboundedSender<P2pCommand>>>,
    pub shared_folder_path: RwLock<PathBuf>,
    pub device_name: RwLock<String>,
    pub shared_folder_display_name: RwLock<String>,
    pub show_folder_name: AtomicBool,
    pub activity_history: Mutex<Vec<crate::commands::activity::ActivityRecord>>,
}

impl P2pState {
    pub fn new() -> Self {
        let shared_folder_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Pebble")
            .join("Shared");

        // 폴더 basename을 기본 표시 이름으로 사용
        let display_name = shared_folder_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Shared")
            .to_string();

        Self {
            is_running: AtomicBool::new(false),
            command_sender: Mutex::new(None),
            shared_folder_path: RwLock::new(shared_folder_path),
            device_name: RwLock::new("내 기기".to_string()),
            shared_folder_display_name: RwLock::new(display_name),
            show_folder_name: AtomicBool::new(true),
            activity_history: Mutex::new(Vec::new()),
        }
    }

    /// 통합 커맨드 전송 헬퍼
    pub async fn send_command(&self, cmd: P2pCommand) -> Result<(), String> {
        let sender = self.command_sender.lock().await;
        if let Some(tx) = sender.as_ref() {
            tx.send(cmd).map_err(|e| format!("커맨드 전송 실패: {}", e))
        } else {
            Err("P2P 엔진 채널이 없습니다.".to_string())
        }
    }
}

impl Default for P2pState {
    fn default() -> Self {
        Self::new()
    }
}
