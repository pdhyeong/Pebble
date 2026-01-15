use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::State;
use tokio::sync::{mpsc, Mutex};
use libp2p::PeerId;

pub mod network;

use network::FileListRequestMsg;

struct P2pState {
    is_running: AtomicBool,
    dial_sender: Mutex<Option<mpsc::UnboundedSender<String>>>,
    file_list_sender: Mutex<Option<mpsc::UnboundedSender<FileListRequestMsg>>>,
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
async fn start_p2p(app_handle: tauri::AppHandle, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    if state.is_running.load(Ordering::SeqCst) {
        return Err("P2P 엔진이 이미 실행 중입니다.".to_string());
    }

    state.is_running.store(true, Ordering::SeqCst);

    // dial 요청을 받을 채널 생성
    let (dial_tx, dial_rx) = mpsc::unbounded_channel::<String>();

    // 파일 목록 요청을 받을 채널 생성
    let (file_list_tx, file_list_rx) = mpsc::unbounded_channel::<FileListRequestMsg>();

    // sender들을 state에 저장
    {
        let mut sender = state.dial_sender.lock().await;
        *sender = Some(dial_tx);
    }
    {
        let mut sender = state.file_list_sender.lock().await;
        *sender = Some(file_list_tx);
    }

    let state_clone = state.inner().clone();

    tauri::async_runtime::spawn(async move {
        match network::run_p2p_engine(app_handle, dial_rx, file_list_rx).await {
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
fn get_p2p_status(state: State<'_, Arc<P2pState>>) -> bool {
    state.is_running.load(Ordering::SeqCst)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(P2pState {
            is_running: AtomicBool::new(false),
            dial_sender: Mutex::new(None),
            file_list_sender: Mutex::new(None),
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_file_list,
            start_p2p,
            get_p2p_status,
            connect_to_peer,
            request_file_list
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
