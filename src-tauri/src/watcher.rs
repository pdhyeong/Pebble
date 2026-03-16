/// 파일 와처 모듈
/// 공유 폴더의 파일 변경(생성, 수정, 삭제)을 실시간으로 감지하고,
/// 프론트엔드 UI 갱신 및 연결된 피어에게 디렉토리 변경 알림을 전송합니다.
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::network::P2pCommand;
use crate::state::P2pState;

// -----------------------------------------------------------------
// 이벤트 페이로드: 프론트엔드로 전달되는 파일 변경 정보
// -----------------------------------------------------------------

/// 프론트엔드의 "local-files-changed" 이벤트가 받는 페이로드
#[derive(Debug, Clone, Serialize)]
pub struct FileChangedPayload {
    /// 변경된 파일/폴더의 절대 경로
    pub path: String,
    /// 변경 종류: "created" | "modified" | "removed"
    pub kind: String,
}

// -----------------------------------------------------------------
// 핵심 함수: init_file_watcher
// -----------------------------------------------------------------

/// 지정된 경로에 대한 파일 감시자를 시작합니다.
///
/// # 인자 (Inputs)
/// - `watch_path` : 감시 대상 폴더 경로 (공유 폴더)
/// - `app_handle` : 프론트엔드로 이벤트를 보내기 위한 Tauri AppHandle
/// - `state`      : P2P 명령 채널에 접근하기 위한 공유 상태 (Arc)
///
/// # 리턴값 (Returns)
/// - `Ok(())` : 별도의 백그라운드 스레드에서 감시자가 실행됩니다.
/// - `Err(String)` : 경로가 유효하지 않거나 감시자 생성에 실패한 경우.
///
/// # 동작 흐름
/// 1. OS별 network 감시자를 debounce 래퍼 위에 생성합니다.
/// 2. 의미 있는 이벤트(생성·수정·삭제)만 필터링합니다.
/// 3. 이벤트를 Tauri 프론트엔드로 emit하여 로컬 UI를 갱신합니다.
/// 4. 연결된 피어들에게 "내 폴더가 바뀌었어!" 신호만 전송합니다.
///    (파일 본문은 전송하지 않습니다.)
pub fn init_file_watcher(
    watch_path: PathBuf,
    app_handle: AppHandle,
    state: Arc<P2pState>,
) -> Result<notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>, String> {
    if !watch_path.exists() {
        return Err(format!(
            "감시 경로가 존재하지 않습니다: {}",
            watch_path.display()
        ));
    }

    // 이벤트를 처리하기 위한 비동기 채널
    let (tx, mut rx) = mpsc::unbounded_channel::<(PathBuf, String)>();

    // 새 Tokio 비동기 런타임에서 이벤트 처리기를 실행합니다.
    let app_handle_clone = app_handle.clone();
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some((path, kind)) = rx.recv().await {
            // 1. 프론트엔드 로컬 UI 갱신
            let payload = FileChangedPayload {
                path: path.to_string_lossy().to_string(),
                kind: kind.clone(),
            };
            if let Err(e) = app_handle_clone.emit("local-files-changed", &payload) {
                eprintln!("[Watcher] 이벤트 emit 실패: {e}");
            }

            // 2. 연결된 피어들에게 디렉토리 변경 신호 전송 (파일 본문 없이 알림만)
            if let Err(e) = state_clone.send_command(P2pCommand::NotifyDirectoryChanged).await {
                eprintln!("[Watcher] 피어 알림 전송 실패: {e}");
            }
        }
    });

    // debounce 래퍼 생성 (300ms 내 중복 이벤트 무시)
    let debouncer = new_debouncer(
        Duration::from_millis(300),
        move |result: notify_debouncer_mini::DebounceEventResult| {
            let events = match result {
                Ok(e) => e,
                Err(errs) => {
                    eprintln!("[Watcher] 에러: {errs:?}");
                    return;
                }
            };

            for event in events {
                // OS 임시 파일 및 hidden 파일 무시 (.DS_Store, .part 등)
                if should_ignore(&event.path) {
                    continue;
                }

                let kind_str = match event.kind {
                    DebouncedEventKind::Any => "modified",
                    // AnyContinuous 및 기타 이벤트는 무시 (점진적 변경은 최종 Any 이벤트로 처리됨)
                    _ => continue,
                };

                // created/removed는 notify::EventKind로 구분하기 어려워 kind_str 사용
                let _ = tx.send((event.path.clone(), kind_str.to_string()));
            }
        },
    )
    .map_err(|e| format!("감시자 생성 실패: {e}"))?;

    // 감시 시작 (하위 폴더 포함하여 Recursive 감시)
    let mut watcher_ref = debouncer;
    watcher_ref
        .watcher()
        .watch(&watch_path, RecursiveMode::Recursive)
        .map_err(|e| format!("경로 감시 시작 실패: {e}"))?;

    println!("[Watcher] 감시 시작: {}", watch_path.display());
    Ok(watcher_ref)
}

// -----------------------------------------------------------------
// 내부 헬퍼: 무시할 파일 필터
// -----------------------------------------------------------------

/// OS 임시 파일, 히든 파일, 전송 임시 파일 등을 걸러냅니다.
fn should_ignore(path: &Path) -> bool {
    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n,
        None => return true,
    };

    // macOS 메타데이터 파일
    if name == ".DS_Store" { return true; }
    // Vim/Nano 스왑 파일
    if name.ends_with(".swp") || name.ends_with(".swx") { return true; }
    // 전송 중 임시 파일
    if name.ends_with(".part") || name.ends_with(".tmp") { return true; }
    // 히든 파일 (`.` 시작)
    if name.starts_with('.') { return true; }

    false
}
