// 설정 관련 명령어
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

use crate::state::P2pState;

/// 공유 폴더 표시 이름 설정
#[tauri::command]
pub async fn set_shared_folder_display_name(
    name: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, String> {
    let mut display_name = state.shared_folder_display_name.write().await;
    *display_name = name.clone();
    Ok(format!("공유 폴더 표시 이름 설정: {}", name))
}

/// 공유 폴더 표시 이름 조회
#[tauri::command]
pub async fn get_shared_folder_display_name(
    state: State<'_, Arc<P2pState>>,
) -> Result<String, String> {
    let name = state.shared_folder_display_name.read().await;
    Ok(name.clone())
}

/// 폴더 이름 공개 여부 설정
#[tauri::command]
pub fn set_show_folder_name(show: bool, state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    state.show_folder_name.store(show, Ordering::SeqCst);
    Ok(format!("폴더 이름 공개 설정: {}", show))
}

/// 폴더 이름 공개 여부 조회
#[tauri::command]
pub fn get_show_folder_name(state: State<'_, Arc<P2pState>>) -> Result<bool, String> {
    Ok(state.show_folder_name.load(Ordering::SeqCst))
}
