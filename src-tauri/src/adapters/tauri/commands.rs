use std::path::PathBuf;
use tauri::State;

use pebble_core::application::services::AppServices;
use pebble_core::domain::activity::ActivityRecord;
use pebble_core::domain::error::{AppError, AppResult, ErrorCode};
use pebble_core::domain::files::{FileEntry, FolderStats};

type Services<'a> = State<'a, AppServices>;

#[tauri::command]
pub async fn start_p2p(services: Services<'_>) -> AppResult<()> {
    services.start_p2p().await
}

#[tauri::command]
pub async fn stop_p2p(services: Services<'_>) -> AppResult<()> {
    services.stop_p2p().await
}

#[tauri::command]
pub async fn get_p2p_status(services: Services<'_>) -> AppResult<bool> {
    Ok(services.p2p_status().await)
}

#[tauri::command]
pub async fn connect_to_peer(addr: String, services: Services<'_>) -> AppResult<()> {
    services.dial(addr).await
}

#[tauri::command]
pub async fn request_file_list(
    peer_id: String,
    path: String,
    services: Services<'_>,
) -> AppResult<()> {
    services.request_file_list(&peer_id, path).await
}

#[tauri::command]
pub async fn download_file(
    peer_id: String,
    path: String,
    services: Services<'_>,
) -> AppResult<String> {
    Ok(services.download(&peer_id, &path).await?.to_string())
}

#[tauri::command]
pub async fn upload_file(
    peer_id: String,
    file_path: String,
    remote_path: String,
    services: Services<'_>,
) -> AppResult<String> {
    Ok(services
        .upload(&peer_id, PathBuf::from(file_path), &remote_path)
        .await?
        .to_string())
}

#[tauri::command]
pub async fn cancel_transfer(transfer_id: String, services: Services<'_>) -> AppResult<()> {
    services.cancel(transfer_id).await
}

#[tauri::command]
pub async fn request_pairing(peer_id: String, services: Services<'_>) -> AppResult<String> {
    services.request_pairing(&peer_id).await
}

#[tauri::command]
pub async fn respond_pairing(
    peer_id: String,
    approved: bool,
    services: Services<'_>,
) -> AppResult<()> {
    services.respond_pairing(&peer_id, approved).await
}

#[tauri::command]
pub fn set_device_name(name: String, services: Services<'_>) {
    services.set_device_name(name);
}

#[tauri::command]
pub fn get_device_name(services: Services<'_>) -> String {
    services.device_name()
}

#[tauri::command]
pub async fn set_shared_folder(path: String, services: Services<'_>) -> AppResult<()> {
    services.set_shared_folder(PathBuf::from(path)).await
}

#[tauri::command]
pub fn get_shared_folder(services: Services<'_>) -> String {
    services.shared_folder().display().to_string()
}

#[tauri::command]
pub fn set_shared_folder_display_name(name: String, services: Services<'_>) {
    services.set_shared_folder_display_name(name);
}

#[tauri::command]
pub fn get_shared_folder_display_name(services: Services<'_>) -> String {
    services.shared_folder_display_name()
}

#[tauri::command]
pub fn set_show_folder_name(show: bool, services: Services<'_>) {
    services.set_show_folder_name(show);
}

#[tauri::command]
pub fn get_show_folder_name(services: Services<'_>) -> bool {
    services.show_folder_name()
}

#[tauri::command]
pub async fn get_local_shared_files(
    relative_path: String,
    services: Services<'_>,
) -> AppResult<Vec<FileEntry>> {
    services.local_shared_files(&relative_path).await
}

#[tauri::command]
pub fn get_local_ip() -> AppResult<String> {
    local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .map_err(|e| AppError::new(ErrorCode::Network, format!("IP 주소 조회 실패: {e}")))
}

#[tauri::command]
pub fn get_activity_history(services: Services<'_>) -> Vec<ActivityRecord> {
    services.activity_history()
}

#[tauri::command]
pub async fn get_shared_folder_stats(services: Services<'_>) -> AppResult<FolderStats> {
    services.shared_folder_stats().await
}
