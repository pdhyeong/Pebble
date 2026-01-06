use anyhow::{Context, Result};
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind},
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::task;

use super::db::{self, FileMetadata};
use super::integrity;

/// 파일 시스템 이벤트 타입
#[derive(Debug, Clone)]
pub enum FileEvent {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
}

/// 파일 감시 핸들러
///
/// 백그라운드에서 실행되며 파일 시스템 변경 사항을 감지하고 DB를 업데이트합니다.
pub struct FileWatcher {
    _watcher: RecommendedWatcher,
    #[allow(dead_code)]
    watch_path: PathBuf,
}

impl FileWatcher {
    /// 새로운 FileWatcher를 생성하고 감시를 시작합니다.
    ///
    /// # Arguments
    /// * `path` - 감시할 디렉토리 경로
    ///
    /// # Returns
    /// * `Result<Self>` - 성공 시 FileWatcher 인스턴스
    ///
    /// # Security Considerations
    /// - 심볼릭 링크 순환 참조 방지를 위해 RecursiveMode 사용
    /// - 파일 시스템 이벤트 필터링으로 불필요한 처리 방지
    pub fn new(path: &str) -> Result<Self> {
        let watch_path = PathBuf::from(path);

        if !watch_path.exists() {
            anyhow::bail!("Watch path does not exist: {}", path);
        }

        if !watch_path.is_dir() {
            anyhow::bail!("Watch path is not a directory: {}", path);
        }

        // 채널 생성: 파일 시스템 이벤트를 받을 채널
        let (tx, rx) = channel();

        // notify 감시자 생성
        let mut watcher = notify::recommended_watcher(tx)
            .context("Failed to create file system watcher")?;

        // 재귀적으로 디렉토리 감시 시작
        watcher
            .watch(&watch_path, RecursiveMode::Recursive)
            .with_context(|| format!("Failed to watch directory: {}", path))?;

        log::info!("Started watching directory: {}", path);

        // 이벤트 처리를 위한 백그라운드 태스크 생성
        Self::spawn_event_handler(rx);

        Ok(Self {
            _watcher: watcher,
            watch_path,
        })
    }

    /// 파일 시스템 이벤트를 처리하는 백그라운드 태스크를 생성합니다.
    ///
    /// # Arguments
    /// * `rx` - 이벤트 수신 채널
    ///
    /// # Architecture
    /// - tokio 런타임에서 비동기로 실행
    /// - 블로킹 작업(파일 I/O, DB 작업)은 별도 스레드에서 처리
    /// - UI 스레드를 방해하지 않도록 설계
    fn spawn_event_handler(rx: Receiver<notify::Result<Event>>) {
        tokio::spawn(async move {
            // Arc<Mutex>로 Receiver를 감싸서 여러 태스크에서 안전하게 사용
            let rx = Arc::new(Mutex::new(rx));

            loop {
                // 이벤트 수신 (블로킹 작업이므로 spawn_blocking 사용)
                let rx_clone = Arc::clone(&rx);
                let event_result = task::spawn_blocking(move || {
                    let rx = rx_clone.lock().unwrap();
                    rx.recv()
                })
                .await;

                match event_result {
                    Ok(Ok(Ok(event))) => {
                        // 이벤트 처리
                        if let Err(e) = Self::handle_event(event).await {
                            log::error!("Error handling file event: {}", e);
                        }
                    }
                    Ok(Ok(Err(e))) => {
                        log::error!("File watcher error: {}", e);
                    }
                    Ok(Err(_)) => {
                        // 채널이 닫힘 (감시 종료)
                        log::info!("File watcher channel closed");
                        break;
                    }
                    Err(e) => {
                        log::error!("Task join error: {}", e);
                        break;
                    }
                }
            }
        });
    }

    /// 파일 시스템 이벤트를 처리합니다.
    ///
    /// # Arguments
    /// * `event` - notify 이벤트
    ///
    /// # Process Flow
    /// 1. 이벤트 타입 분류 (Create/Modify/Remove)
    /// 2. 파일 경로 추출
    /// 3. 해당 작업 수행 (해시 계산 및 DB 업데이트)
    async fn handle_event(event: Event) -> Result<()> {
        let file_event = match event.kind {
            EventKind::Create(CreateKind::File) => {
                if let Some(path) = event.paths.first() {
                    Some(FileEvent::Created(path.clone()))
                } else {
                    None
                }
            }
            EventKind::Modify(ModifyKind::Data(_)) => {
                if let Some(path) = event.paths.first() {
                    Some(FileEvent::Modified(path.clone()))
                } else {
                    None
                }
            }
            EventKind::Remove(RemoveKind::File) => {
                if let Some(path) = event.paths.first() {
                    Some(FileEvent::Removed(path.clone()))
                } else {
                    None
                }
            }
            _ => None, // 다른 이벤트는 무시
        };

        if let Some(file_event) = file_event {
            Self::process_file_event(file_event).await?;
        }

        Ok(())
    }

    /// 파일 이벤트를 처리하고 DB를 업데이트합니다.
    ///
    /// # Arguments
    /// * `event` - 처리할 파일 이벤트
    ///
    /// # Security
    /// - 파일이 실제로 존재하는지 확인
    /// - 디렉토리는 제외하고 파일만 처리
    /// - DB 업데이트 실패 시 에러 로깅
    async fn process_file_event(event: FileEvent) -> Result<()> {
        match event {
            FileEvent::Created(path) | FileEvent::Modified(path) => {
                // 블로킹 작업이므로 spawn_blocking 사용
                task::spawn_blocking(move || -> Result<()> {
                    // 파일이 실제로 존재하고 디렉토리가 아닌지 확인
                    if !path.exists() || !path.is_file() {
                        return Ok(());
                    }

                    let path_str = path.to_string_lossy().to_string();

                    // 파일 해시 계산
                    let file_hash = integrity::calculate_file_hash(&path)
                        .with_context(|| format!("Failed to calculate hash for: {}", path_str))?;

                    // 파일 수정 시간 가져오기
                    let metadata = std::fs::metadata(&path)
                        .with_context(|| format!("Failed to get metadata for: {}", path_str))?;

                    let last_modified = metadata
                        .modified()
                        .unwrap_or(SystemTime::now())
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64;

                    // DB에 파일 정보 업데이트 (Upsert)
                    db::upsert_file(FileMetadata {
                        path: path_str.clone(),
                        last_modified,
                        file_hash,
                        sync_status: "Pending".to_string(),
                    })
                    .with_context(|| format!("Failed to update DB for: {}", path_str))?;

                    log::info!("File change recorded: {} (status: Pending)", path_str);

                    Ok(())
                })
                .await
                .context("Task execution failed")??;
            }
            FileEvent::Removed(path) => {
                let path_str = path.to_string_lossy().to_string();

                // 삭제된 파일은 DB에서 sync_status를 "Deleted"로 업데이트
                // (완전히 삭제하지 않고 상태만 변경하여 동기화 추적 가능)
                task::spawn_blocking(move || -> Result<()> {
                    // 파일이 DB에 존재하는지 확인
                    if let Ok(Some(_)) = db::get_file_metadata(&path_str) {
                        db::update_sync_status(&path_str, "Deleted")
                            .with_context(|| format!("Failed to mark file as deleted: {}", path_str))?;

                        log::info!("File marked as deleted: {}", path_str);
                    }

                    Ok(())
                })
                .await
                .context("Task execution failed")??;
            }
        }

        Ok(())
    }
}

/// 전역 감시자 인스턴스를 저장하기 위한 정적 변수
///
/// Arc<Mutex>로 감싸서 여러 스레드에서 안전하게 접근 가능
static WATCHER_INSTANCE: once_cell::sync::Lazy<Arc<Mutex<Option<FileWatcher>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

/// 파일 감시를 시작합니다.
///
/// # Arguments
/// * `path` - 감시할 디렉토리 경로
///
/// # Returns
/// * `Result<()>` - 성공 또는 에러
///
/// # Notes
/// - 이미 감시 중인 경로가 있으면 중지하고 새로운 경로를 감시합니다
/// - 전역 인스턴스로 관리되어 애플리케이션 생명주기 동안 유지됩니다
pub fn start_watching(path: &str) -> Result<()> {
    let watcher = FileWatcher::new(path)?;

    // 전역 인스턴스에 저장
    let mut instance = WATCHER_INSTANCE
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire watcher lock: {}", e))?;

    *instance = Some(watcher);

    log::info!("File watcher started successfully for: {}", path);

    Ok(())
}

/// 파일 감시를 중지합니다.
///
/// # Notes
/// - 감시자 인스턴스를 제거하면 자동으로 감시가 중지됩니다
pub fn stop_watching() -> Result<()> {
    let mut instance = WATCHER_INSTANCE
        .lock()
        .map_err(|e| anyhow::anyhow!("Failed to acquire watcher lock: {}", e))?;

    if instance.is_some() {
        *instance = None;
        log::info!("File watcher stopped");
    }

    Ok(())
}
