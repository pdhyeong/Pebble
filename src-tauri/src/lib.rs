use std::sync::Arc;

pub mod commands;
pub mod error;
pub mod network;
pub mod state;
pub mod watcher;

use commands::{
    // 전송 관련
    cancel_transfer,
    connect_to_peer,
    download_file,
    // 활동 관련
    get_activity_history,
    get_device_name,
    // 파일 관련
    get_file_list,
    get_local_ip,
    get_local_shared_files,
    get_p2p_status,
    get_shared_folder,
    get_shared_folder_display_name,
    get_shared_folder_stats,
    get_show_folder_name,

    request_file_list,
    request_pairing,
    respond_pairing,

    // 기기 관련
    set_device_name,
    set_shared_folder,

    // 설정 관련
    set_shared_folder_display_name,
    set_show_folder_name,
    // P2P 엔진
    start_p2p,
    stop_p2p,
    upload_file,
};
use state::{P2pState, WatcherState};

/// 인사 테스트 명령어
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// 파일 와처 재시작 명령어
/// 공유 폴더가 변경되었을 때 프론트엔드에서 호출합니다.
///
/// # 인자
/// - `new_path`: 새로 설정된 공유 폴더의 절대 경로 (String)
///
/// # 리턴값
/// - `Ok(())`: 새 경로에 대한 감시가 성공적으로 시작됨
/// - `Err(String)`: 경로가 유효하지 않거나 감시 시작 실패
#[tauri::command]
async fn refresh_watcher(
    new_path: String,
    watcher_state: tauri::State<'_, Arc<tokio::sync::Mutex<WatcherState>>>,
    p2p_state: tauri::State<'_, Arc<P2pState>>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(&new_path);
    let debouncer = watcher::init_file_watcher(path, app, p2p_state.inner().clone())?;
    let mut lock = watcher_state.lock().await;
    // 기존 watcher를 Drop하고 새 watcher로 교체
    lock.debouncer = Some(debouncer);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Arc::new(P2pState::new()))
        .manage(Arc::new(tokio::sync::Mutex::new(WatcherState { debouncer: None })))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            // P2P 엔진
            start_p2p,
            stop_p2p,
            get_p2p_status,
            // 파일 관련
            get_file_list,
            set_shared_folder,
            get_shared_folder,
            get_local_shared_files,
            // 기기 관련
            set_device_name,
            get_device_name,
            get_local_ip,
            // 설정 관련
            set_shared_folder_display_name,
            get_shared_folder_display_name,
            set_show_folder_name,
            get_show_folder_name,
            // 전송 관련
            connect_to_peer,
            request_file_list,
            download_file,
            upload_file,
            cancel_transfer,
            request_pairing,
            respond_pairing,
            // 활동 관련
            get_activity_history,
            get_shared_folder_stats,
            // 파일 와처
            refresh_watcher,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
