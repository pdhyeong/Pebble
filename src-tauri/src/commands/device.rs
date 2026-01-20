// 기기 관련 명령어
use std::sync::Arc;
use tauri::State;

use crate::state::P2pState;

/// 기기 이름 설정
#[tauri::command]
pub async fn set_device_name(
    name: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, String> {
    let mut device_name = state.device_name.write().await;
    *device_name = name.clone();
    Ok(format!("기기 이름이 설정되었습니다: {}", name))
}

/// 기기 이름 조회
#[tauri::command]
pub async fn get_device_name(state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    let name = state.device_name.read().await;
    Ok(name.clone())
}

/// 로컬 IP 주소 조회
#[tauri::command]
pub fn get_local_ip() -> Result<String, String> {
    match local_ip_address::local_ip() {
        Ok(ip) => Ok(ip.to_string()),
        Err(e) => Err(format!("IP 주소 조회 실패: {}", e)),
    }
}
