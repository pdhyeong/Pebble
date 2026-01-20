// 네트워크 헬퍼 함수들
use crate::network::behavior::{FileInfo, FileListResponse};
use std::path::PathBuf;

/// 폴더 내 파일 목록 조회
pub fn list_files(base_path: &PathBuf, relative_path: &str) -> FileListResponse {
    let full_path = if relative_path.is_empty() || relative_path == "/" {
        base_path.clone()
    } else {
        base_path.join(relative_path.trim_start_matches('/'))
    };

    // 공유 폴더가 없으면 생성
    if !base_path.exists() {
        if let Err(e) = std::fs::create_dir_all(base_path) {
            return FileListResponse {
                path: relative_path.to_string(),
                files: vec![],
                error: Some(format!("공유 폴더 생성 실패: {}", e)),
            };
        }
    }

    match std::fs::read_dir(&full_path) {
        Ok(entries) => {
            let files: Vec<FileInfo> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| {
                    let metadata = entry.metadata().ok();
                    let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                    let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

                    FileInfo {
                        name: entry.file_name().to_string_lossy().to_string(),
                        path: entry
                            .path()
                            .strip_prefix(base_path)
                            .map(|p| format!("/{}", p.display()))
                            .unwrap_or_else(|_| entry.path().display().to_string()),
                        is_dir,
                        size,
                    }
                })
                .collect();

            FileListResponse {
                path: relative_path.to_string(),
                files,
                error: None,
            }
        }
        Err(e) => FileListResponse {
            path: relative_path.to_string(),
            files: vec![],
            error: Some(format!("폴더 읽기 실패: {}", e)),
        },
    }
}

/// QUIC 주소 선택 헬퍼
pub fn get_quic_address(addrs: &[libp2p::Multiaddr]) -> Option<&libp2p::Multiaddr> {
    addrs.iter().find(|a| a.to_string().contains("quic"))
}
