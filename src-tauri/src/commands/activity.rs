// 활동 기록 관련 명령어
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

use crate::state::P2pState;

/// 활동 기록 항목
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRecord {
    pub id: String,
    pub transfer_type: String, // "upload" or "download"
    pub file_name: String,
    pub peer_id: String,
    pub device_name: Option<String>,
    pub size: u64,
    pub timestamp: i64,     // Unix timestamp
    pub status: String,     // "completed", "failed", "cancelled"
    pub speed: Option<f64>, // bytes per second
}

/// 공유 폴더 통계
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedFolderStats {
    pub total_size: u64,
    pub file_count: usize,
}

/// 활동 기록 조회
#[tauri::command]
pub async fn get_activity_history(
    state: State<'_, Arc<P2pState>>,
) -> Result<Vec<ActivityRecord>, String> {
    let history = state.activity_history.lock().await;
    Ok(history.clone())
}

/// 공유 폴더 통계 조회
#[tauri::command]
pub async fn get_shared_folder_stats(
    state: State<'_, Arc<P2pState>>,
) -> Result<SharedFolderStats, String> {
    // shared_folder_path를 사용 (실제 사용자가 설정한 경로)
    let shared_folder = state.shared_folder_path.read().await;

    calculate_folder_stats(&shared_folder).map_err(|e| format!("폴더 통계 계산 실패: {}", e))
}

/// 폴더 통계 계산 (재귀적)
fn calculate_folder_stats(path: &PathBuf) -> Result<SharedFolderStats, std::io::Error> {
    let mut total_size = 0u64;
    let mut file_count = 0usize;

    fn visit_dir(
        path: &PathBuf,
        total_size: &mut u64,
        file_count: &mut usize,
    ) -> Result<(), std::io::Error> {
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dir(&path, total_size, file_count)?;
                } else {
                    let metadata = entry.metadata()?;
                    *total_size += metadata.len();
                    *file_count += 1;
                }
            }
        }
        Ok(())
    }

    visit_dir(path, &mut total_size, &mut file_count)?;

    Ok(SharedFolderStats {
        total_size,
        file_count,
    })
}
