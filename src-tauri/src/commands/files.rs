// 파일 관련 명령어
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::State;

use crate::network::P2pCommand;
use crate::state::{LocalFileInfo, P2pState};

/// 공유 폴더 경로 설정
#[tauri::command]
pub async fn set_shared_folder(
    path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<String, String> {
    let path_buf = PathBuf::from(&path);

    // 폴더가 없으면 생성
    if !path_buf.exists() {
        fs::create_dir_all(&path_buf).map_err(|e| format!("폴더 생성 실패: {}", e))?;
    }

    // 내부 상태 업데이트
    {
        let mut shared_path = state.shared_folder_path.write().await;
        *shared_path = path_buf.clone();
    }

    // P2P가 실행 중이면 엔진에도 알림
    if state.is_running.load(Ordering::SeqCst) {
        state
            .send_command(P2pCommand::UpdateSharedFolder(path_buf))
            .await
            .map_err(|e| format!("공유 폴더 변경 알림 실패: {}", e))?;
    }

    Ok(format!("공유 폴더가 설정되었습니다: {}", path))
}

/// 공유 폴더 경로 조회
#[tauri::command]
pub async fn get_shared_folder(state: State<'_, Arc<P2pState>>) -> Result<String, String> {
    let path = state.shared_folder_path.read().await;
    Ok(path.display().to_string())
}

/// 로컬 공유 폴더 파일 목록 조회
#[tauri::command]
pub async fn get_local_shared_files(
    relative_path: String,
    state: State<'_, Arc<P2pState>>,
) -> Result<Vec<LocalFileInfo>, String> {
    let shared_path = state.shared_folder_path.read().await;

    // 상대 경로를 절대 경로로 변환 (보안: 공유 폴더 외부 접근 방지)
    let clean_path = relative_path.trim_start_matches('/');
    let target_path = if clean_path.is_empty() {
        shared_path.clone()
    } else {
        shared_path.join(clean_path)
    };

    // 보안 검사: 공유 폴더 외부 접근 방지
    let canonical_shared = shared_path
        .canonicalize()
        .map_err(|e| format!("공유 폴더 경로 확인 실패: {}", e))?;

    // 대상 경로가 존재하지 않으면 에러
    if !target_path.exists() {
        return Err(format!("경로를 찾을 수 없습니다: {}", relative_path));
    }

    let canonical_target = target_path
        .canonicalize()
        .map_err(|e| format!("경로 확인 실패: {}", e))?;

    if !canonical_target.starts_with(&canonical_shared) {
        return Err("접근이 거부되었습니다.".to_string());
    }

    // 폴더가 없으면 생성
    if !target_path.exists() {
        fs::create_dir_all(&target_path).map_err(|e| format!("폴더 생성 실패: {}", e))?;
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

/// 폴더 내 파일 목록 조회 (일반)
#[tauri::command]
pub fn get_file_list(folder_path: &str) -> Result<Vec<String>, String> {
    let path = Path::new(folder_path);
    if !path.is_dir() {
        return Err(format!(
            "The path '{}' is not a valid directory.",
            folder_path
        ));
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
