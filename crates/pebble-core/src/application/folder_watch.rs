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
    pub fn spawn(
        watcher: Arc<dyn FolderWatcher>,
        ui: Arc<dyn UiEvents>,
        supervisor: Arc<P2pSupervisor>,
    ) -> Self {
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

async fn forward_changes(
    mut inbox: mpsc::Receiver<PathBuf>,
    ui: Arc<dyn UiEvents>,
    supervisor: Arc<P2pSupervisor>,
) {
    while let Some(first) = inbox.recv().await {
        let mut next = Some(first);
        while let Some(path) = next {
            ui.emit(UiEvent::LocalFilesChanged {
                path: path.display().to_string(),
                kind: "modified",
            });
            next = inbox.try_recv().ok();
        }
        supervisor.folder_changed().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::engine::EngineDeps;
    use crate::application::ports::NetworkFactory;
    use crate::application::state::{ActivityStore, SettingsStore};
    use crate::domain::settings::Settings;
    use std::time::Duration;

    #[derive(Default)]
    struct Recorder(Mutex<Vec<String>>);

    impl UiEvents for Recorder {
        fn emit(&self, event: UiEvent) {
            if let UiEvent::LocalFilesChanged { path, .. } = event {
                self.0.lock().unwrap().push(path);
            }
        }
    }

    struct NoNetwork;

    impl NetworkFactory for NoNetwork {
        fn start(&self) -> AppResult<crate::application::net::NetworkLink> {
            Err(crate::domain::error::AppError::new(
                crate::domain::error::ErrorCode::Network,
                "unused",
            ))
        }
    }

    #[tokio::test]
    async fn burst_of_changes_is_forwarded_in_order() {
        let ui = Arc::new(Recorder::default());
        let deps = EngineDeps {
            ui: ui.clone(),
            files: Arc::new(crate::adapters::fs::LocalFileStore::new(
                std::env::temp_dir(),
            )),
            settings: SettingsStore::new(Settings::with_home(None)),
            activity: ActivityStore::default(),
            trust: Arc::new(crate::adapters::fs::JsonTrustStore::new(
                std::env::temp_dir().join("unused-trust.json"),
            )),
        };
        let supervisor = Arc::new(P2pSupervisor::new(deps, Arc::new(NoNetwork), None, None));
        let (changes, inbox) = mpsc::channel(CHANGE_BUFFER);
        let task = tokio::spawn(forward_changes(inbox, ui.clone(), supervisor));
        for i in 0..CHANGE_BUFFER {
            changes.try_send(PathBuf::from(format!("/f{i}"))).unwrap();
        }
        drop(changes);
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap();
        let seen = ui.0.lock().unwrap().clone();
        assert_eq!(seen.len(), CHANGE_BUFFER);
        assert_eq!(seen.first().map(String::as_str), Some("/f0"));
        assert_eq!(seen.last().map(String::as_str), Some("/f255"));
    }
}
