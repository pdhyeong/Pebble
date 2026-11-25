use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::application::ports::{FolderWatcher, WatchGuard};
use crate::domain::error::{AppError, AppResult, ErrorCode};
use crate::domain::files::is_ignored_name;

pub const DEBOUNCE: Duration = Duration::from_millis(300);

pub struct NotifyFolderWatcher;

impl FolderWatcher for NotifyFolderWatcher {
    fn watch(&self, path: &Path, changes: mpsc::Sender<PathBuf>) -> AppResult<Box<dyn WatchGuard>> {
        let mut debouncer = new_debouncer(DEBOUNCE, move |result: DebounceEventResult| match result {
            Ok(events) => {
                for event in events {
                    if !matches!(event.kind, DebouncedEventKind::Any) {
                        continue;
                    }
                    let ignored = event
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_none_or(is_ignored_name);
                    if !ignored {
                        let _ = changes.try_send(event.path);
                    }
                }
            }
            Err(e) => eprintln!("[watcher] 감시 오류: {e:?}"),
        })
        .map_err(|e| AppError::new(ErrorCode::Io, format!("폴더 감시를 만들지 못했습니다: {e}")))?;
        debouncer
            .watcher()
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| AppError::new(ErrorCode::Io, format!("폴더 감시를 시작하지 못했습니다: {e}")))?;
        Ok(Box::new(debouncer))
    }
}
