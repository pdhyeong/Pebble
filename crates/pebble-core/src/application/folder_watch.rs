use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use tokio::sync::mpsc;

use super::events::UiEvent;
use super::ports::{FolderWatcher, UiEvents, WatchGuard};
use super::supervisor::P2pSupervisor;
use crate::domain::error::AppResult;

pub const CHANGE_BUFFER: usize = 256;

pub struct FolderWatch {
    watcher: Arc<dyn FolderWatcher>,
    changes: mpsc::Sender<PathBuf>,
    guard: Mutex<Option<Box<dyn WatchGuard>>>,
}

impl FolderWatch {
    pub fn spawn(watcher: Arc<dyn FolderWatcher>, ui: Arc<dyn UiEvents>, supervisor: Arc<P2pSupervisor>) -> Self {
        let (changes, inbox) = mpsc::channel(CHANGE_BUFFER);
        tokio::spawn(forward_changes(inbox, ui, supervisor));
        Self {
            watcher,
            changes,
            guard: Mutex::new(None),
        }
    }

    pub fn restart(&self, path: &Path) -> AppResult<()> {
        let next = self.watcher.watch(path, self.changes.clone())?;
        *self.guard.lock().unwrap_or_else(PoisonError::into_inner) = Some(next);
        Ok(())
    }
}

async fn forward_changes(mut inbox: mpsc::Receiver<PathBuf>, ui: Arc<dyn UiEvents>, supervisor: Arc<P2pSupervisor>) {
    while let Some(path) = inbox.recv().await {
        ui.emit(UiEvent::LocalFilesChanged {
            path: path.display().to_string(),
            kind: "modified",
        });
        supervisor.folder_changed().await;
    }
}
