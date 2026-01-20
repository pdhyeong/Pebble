// 상태 관리 모듈
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use tokio::sync::{mpsc, Mutex, RwLock};

use crate::network::{
    CancelTransferMsg, FileListRequestMsg, FileTransferRequestMsg, FileTransferStreamRequestMsg,
    PairingApprovalMsg, PairingRequestMsg,
};

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
    pub dial_sender: Mutex<Option<mpsc::UnboundedSender<String>>>,
    pub file_list_sender: Mutex<Option<mpsc::UnboundedSender<FileListRequestMsg>>>,
    pub file_transfer_sender: Mutex<Option<mpsc::UnboundedSender<FileTransferRequestMsg>>>,
    pub file_stream_sender: Mutex<Option<mpsc::UnboundedSender<FileTransferStreamRequestMsg>>>,
    pub shared_folder_sender: Mutex<Option<mpsc::UnboundedSender<PathBuf>>>,
    pub pairing_request_sender: Mutex<Option<mpsc::UnboundedSender<PairingRequestMsg>>>,
    pub pairing_approval_sender: Mutex<Option<mpsc::UnboundedSender<PairingApprovalMsg>>>,
    pub cancel_transfer_sender: Mutex<Option<mpsc::UnboundedSender<CancelTransferMsg>>>,
    pub shutdown_sender: Mutex<Option<mpsc::Sender<()>>>,
    pub shared_folder_path: RwLock<PathBuf>,
    pub device_name: RwLock<String>,
    pub shared_folder_display_name: RwLock<String>,
    pub show_folder_name: AtomicBool,
    pub activity_history: Mutex<Vec<crate::commands::activity::ActivityRecord>>,
    pub shared_folder: Mutex<PathBuf>,
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
            dial_sender: Mutex::new(None),
            file_list_sender: Mutex::new(None),
            file_transfer_sender: Mutex::new(None),
            file_stream_sender: Mutex::new(None),
            shared_folder_sender: Mutex::new(None),
            pairing_request_sender: Mutex::new(None),
            pairing_approval_sender: Mutex::new(None),
            cancel_transfer_sender: Mutex::new(None),
            shutdown_sender: Mutex::new(None),
            shared_folder_path: RwLock::new(shared_folder_path.clone()),
            device_name: RwLock::new("내 기기".to_string()),
            shared_folder_display_name: RwLock::new(display_name),
            show_folder_name: AtomicBool::new(true),
            activity_history: Mutex::new(Vec::new()),
            shared_folder: Mutex::new(shared_folder_path),
        }
    }
}

impl Default for P2pState {
    fn default() -> Self {
        Self::new()
    }
}
