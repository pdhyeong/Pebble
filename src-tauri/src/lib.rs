use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::State;
use tokio::sync::{mpsc, Mutex, RwLock};
use libp2p::PeerId;
use serde::{Deserialize, Serialize};

pub mod network;

use network::{FileListRequestMsg, FileTransferRequestMsg, FileUploadRequestMsg};

// 로컬 파일 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
struct LocalFileInfo {
    name: String,
    path: String,
    is_dir: bool,
    size: u64,
}

struct P2pState {
    is_running: AtomicBool,
    dial_sender: Mutex<Option<mpsc::UnboundedSender<String>>>,
    file_list_sender: Mutex<Option<mpsc::UnboundedSender<FileListRequestMsg>>>,
    file_transfer_sender: Mutex<Option<mpsc::UnboundedSender<FileTransferRequestMsg>>>,
    file_upload_sender: Mutex<Option<mpsc::UnboundedSender<FileUploadRequestMsg>>>,
    shared_folder_sender: Mutex<Option<mpsc::UnboundedSender<PathBuf>>>,
    shutdown_sender: Mutex<Option<mpsc::Sender<()>>>,
    shared_folder_path: RwLock<PathBuf>,
    device_name: RwLock<String>,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn get_file_list(folder_path: &str) -> Result<Vec<String>, String> {
    let path = Path::new(folder_path);
    if !path.is_dir() {
        return Err(format!("The path '{}' is not a valid directory.", folder_path));
    }

    match fs::read_dir(path) {
        Ok(entries) => {
            let file_names: Vec<String> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            Ok(file_names)
        }
        Err(e) => Err(format!("Failed to read directory '{}': {}", folder_path, e)),
    }
}

#[tauri::command]
async fn set_shared_folder(path: String, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);

    // 폴더가 없으면 생성
    if !path_buf.exists() {
        fs::create_dir_all(&path_buf)
            .map_err(|e| format!("폴더 생성 실패: {}", e))?;
    }

    // 내부 상태 업데이트
    {
        let mut shared_path = state.shared_folder_path.write().await;
        *shared_path = path_buf.clone();
    }

    // P2P가 실행 중이면 엔진에도 알림
    if state.is_running.load(Ordering::SeqCst) {
        let sender = state.shared_folder_sender.lock().await;
        if let Some(tx) = sender.as_ref() {
            tx.send(path_buf.clone())
                .map_err(|e| format!("공유 폴더 변경 알림 실패: {}", e))?;
        }
    }

    Ok(format!("공유 폴더가 설정되었습니다: {}", path))
}

// 공유 폴더 경로 조회
#[tauri::command]
async fn get_shared_folder(state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    let path = state.shared_folder_path.read().await;
    Ok(path.display().to_string())
}

// 기기 이름 설정
#[tauri::command]
async fn set_device_name(name: String, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    let mut device_name = state.device_name.write().await;
    *device_name = name.clone();
    Ok(format!("기기 이름이 설정되었습니다: {}", name))
}

// 기기 이름 조회
#[tauri::command]
async fn get_device_name(state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    let name = state.device_name.read().await;
    Ok(name.clone())
}

// 로컬 공유 폴더 파일 목록 조회
#[tauri::command]
async fn get_local_shared_files(relative_path: String, state: State<'_, Arc<P2pState>>) -> Result<Vec<LocalFileInfo>, String> {
    let shared_path = state.shared_folder_path.read().await;

    // 상대 경로를 절대 경로로 변환 (보안: 공유 폴더 외부 접근 방지)
    let clean_path = relative_path.trim_start_matches('/');
    let target_path = if clean_path.is_empty() {
        shared_path.clone()
    } else {
        shared_path.join(clean_path)
    };

    // 보안 검사: 공유 폴더 외부 접근 방지
    let canonical_shared = shared_path.canonicalize()
        .map_err(|e| format!("공유 폴더 경로 확인 실패: {}", e))?;

    // 대상 경로가 존재하지 않으면 에러 (canonicalize가 실패함)
    if !target_path.exists() {
        return Err(format!("경로를 찾을 수 없습니다: {}", relative_path));
    }

    let canonical_target = target_path.canonicalize()
        .map_err(|e| format!("경로 확인 실패: {}", e))?;

    if !canonical_target.starts_with(&canonical_shared) {
        return Err("접근이 거부되었습니다.".to_string());
    }

    // 폴더가 없으면 생성
    if !target_path.exists() {
        fs::create_dir_all(&target_path)
            .map_err(|e| format!("폴더 생성 실패: {}", e))?;
    }

    match fs::read_dir(&target_path) {
        Ok(entries) => {
            let files: Vec<LocalFileInfo> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| {
                    let metadata = entry.metadata().ok();
                    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                    // 상대 경로 생성
                    let file_relative_path = if clean_path.is_empty() {
                        format!("/{}", entry.file_name().to_string_lossy())
                    } else {
                        format!("/{}/{}", clean_path, entry.file_name().to_string_lossy())
                    };

                    LocalFileInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: file_relative_path,
                        is_dir,
                        size,
                    }
                })
                .collect();
            Ok(files)
        }
        Err(e) => Err(format!("폴더 읽기 실패: {}", e)),
    }
}

#[tauri::command]
async fn start_p2p(app_handle: tauri::AppHandle, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    if state.is_running.load(Ordering::SeqCst) {
        return Err("P2P 엔진이 이미 실행 중입니다.".to_string());
    }

    state.is_running.store(true, Ordering::SeqCst);

    // dial 요청을 받을 채널 생성
    let (dial_tx, dial_rx) = mpsc::unbounded_channel::<String>();

    // 파일 목록 요청을 받을 채널 생성
    let (file_list_tx, file_list_rx) = mpsc::unbounded_channel::<FileListRequestMsg>();

    // 파일 전송 요청을 받을 채널 생성
    let (file_transfer_tx, file_transfer_rx) = mpsc::unbounded_channel::<FileTransferRequestMsg>();

    // 파일 업로드 요청을 받을 채널 생성
    let (file_upload_tx, file_upload_rx) = mpsc::unbounded_channel::<FileUploadRequestMsg>();

    // 공유 폴더 경로 변경을 받을 채널 생성
    let (shared_folder_tx, shared_folder_rx) = mpsc::unbounded_channel::<PathBuf>();

    // shutdown 채널 생성
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

    // 현재 공유 폴더 경로와 기기 이름 가져오기
    let shared_folder = state.shared_folder_path.read().await.clone();
    let device_name = state.device_name.read().await.clone();

    // sender들을 state에 저장
    {
        let mut sender = state.dial_sender.lock().await;
        *sender = Some(dial_tx);
    }
    {
        let mut sender = state.file_list_sender.lock().await;
        *sender = Some(file_list_tx);
    }
    {
        let mut sender = state.file_transfer_sender.lock().await;
        *sender = Some(file_transfer_tx);
    }
    {
        let mut sender = state.file_upload_sender.lock().await;
        *sender = Some(file_upload_tx);
    }
    {
        let mut sender = state.shared_folder_sender.lock().await;
        *sender = Some(shared_folder_tx);
    }
    {
        let mut sender = state.shutdown_sender.lock().await;
        *sender = Some(shutdown_tx);
    }

    let state_clone = state.inner().clone();

    tauri::async_runtime::spawn(async move {
        match network::run_p2p_engine(
            app_handle,
            dial_rx,
            file_list_rx,
            file_transfer_rx,
            file_upload_rx,
            shared_folder_rx,
            shutdown_rx,
            shared_folder,
            device_name,
        ).await {
            Ok(_) => println!("P2P 엔진 종료"),
            Err(e) => println!("P2P 엔진 에러: {}", e),
        }
        state_clone.is_running.store(false, Ordering::SeqCst);
        // 엔진 종료 시 sender 제거
        {
            let mut sender = state_clone.dial_sender.lock().await;
            *sender = None;
        }
        {
            let mut sender = state_clone.file_list_sender.lock().await;
            *sender = None;
        }
        {
            let mut sender = state_clone.file_transfer_sender.lock().await;
            *sender = None;
        }
        {
            let mut sender = state_clone.file_upload_sender.lock().await;
            *sender = None;
        }
        {
            let mut sender = state_clone.shared_folder_sender.lock().await;
            *sender = None;
        }
        {
            let mut sender = state_clone.shutdown_sender.lock().await;
            *sender = None;
        }
    });

    Ok("P2P 엔진이 시작되었습니다.".to_string())
}

#[tauri::command]
async fn connect_to_peer(addr: String, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err("P2P 엔진이 실행 중이 아닙니다. 먼저 시작해주세요.".to_string());
    }

    let sender = state.dial_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(addr.clone()).map_err(|e| format!("연결 요청 전송 실패: {}", e))?;
        Ok(format!("연결 요청 전송: {}", addr))
    } else {
        Err("P2P 엔진 채널이 없습니다.".to_string())
    }
}

#[tauri::command]
async fn request_file_list(peer_id: String, path: String, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err("P2P 엔진이 실행 중이 아닙니다.".to_string());
    }

    // peer_id 문자열을 PeerId로 변환
    let peer_id: PeerId = peer_id.parse()
        .map_err(|e| format!("잘못된 PeerId 형식: {}", e))?;

    let sender = state.file_list_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(FileListRequestMsg { peer_id, path: path.clone() })
            .map_err(|e| format!("파일 목록 요청 전송 실패: {}", e))?;
        Ok(format!("파일 목록 요청 전송: {}", path))
    } else {
        Err("P2P 엔진 채널이 없습니다.".to_string())
    }
}

#[tauri::command]
async fn download_file(peer_id: String, path: String, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err("P2P 엔진이 실행 중이 아닙니다.".to_string());
    }

    let peer_id: PeerId = peer_id.parse()
        .map_err(|e| format!("잘못된 PeerId 형식: {}", e))?;

    let sender = state.file_transfer_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(FileTransferRequestMsg { peer_id, path: path.clone() })
            .map_err(|e| format!("파일 다운로드 요청 전송 실패: {}", e))?;
        Ok(format!("파일 다운로드 요청 전송: {}", path))
    } else {
        Err("P2P 엔진 채널이 없습니다.".to_string())
    }
}

#[tauri::command]
async fn upload_file(
    peer_id: String,
    file_path: String,
    remote_path: String,
    state: State<'_, Arc<P2pState>>
) -> Result<String, String> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err("P2P 엔진이 실행 중이 아닙니다.".to_string());
    }

    let peer_id: PeerId = peer_id.parse()
        .map_err(|e| format!("잘못된 PeerId 형식: {}", e))?;

    // 파일 읽기
    let file_path_buf = PathBuf::from(&file_path);
    let file_name = file_path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| "파일 이름을 가져올 수 없습니다.".to_string())?;

    let data = fs::read(&file_path_buf)
        .map_err(|e| format!("파일 읽기 실패: {}", e))?;

    let sender = state.file_upload_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(FileUploadRequestMsg {
            peer_id,
            file_name: file_name.clone(),
            remote_path: remote_path.clone(),
            data,
        }).map_err(|e| format!("파일 업로드 요청 전송 실패: {}", e))?;
        Ok(format!("파일 업로드 요청 전송: {} -> {}", file_name, remote_path))
    } else {
        Err("P2P 엔진 채널이 없습니다.".to_string())
    }
}

#[tauri::command]
fn get_p2p_status(state: State<'_, Arc<P2pState>>) -> bool {
    state.is_running.load(Ordering::SeqCst)
}

#[tauri::command]
async fn stop_p2p(state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    if !state.is_running.load(Ordering::SeqCst) {
        return Err("P2P 엔진이 실행 중이 아닙니다.".to_string());
    }

    let sender = state.shutdown_sender.lock().await;
    if let Some(tx) = sender.as_ref() {
        tx.send(()).await.map_err(|e| format!("종료 신호 전송 실패: {}", e))?;
        Ok("P2P 엔진 종료 요청을 보냈습니다.".to_string())
    } else {
        Err("종료 채널이 없습니다.".to_string())
    }
}

#[tauri::command]
fn get_local_ip() -> Result<String, String> {
    match local_ip_address::local_ip() {
        Ok(ip) => Ok(ip.to_string()),
        Err(e) => Err(format!("IP 주소 조회 실패: {}", e)),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(P2pState {
            is_running: AtomicBool::new(false),
            dial_sender: Mutex::new(None),
            file_list_sender: Mutex::new(None),
            file_transfer_sender: Mutex::new(None),
            file_upload_sender: Mutex::new(None),
            shared_folder_sender: Mutex::new(None),
            shutdown_sender: Mutex::new(None),
            shared_folder_path: RwLock::new(
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join("Pebble")
                    .join("Shared")
            ),
            device_name: RwLock::new("내 기기".to_string()),
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_file_list,
            start_p2p,
            stop_p2p,
            get_p2p_status,
            connect_to_peer,
            request_file_list,
            download_file,
            upload_file,
            set_shared_folder,
            get_shared_folder,
            set_device_name,
            get_device_name,
            get_local_shared_files,
            get_local_ip
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
