use std::path::PathBuf;
use std::sync::Arc;

use super::engine::{EngineCommand, PeerSummary, TransferState};
use super::events::UiEvent;
use super::folder_watch::FolderWatch;
use super::ports::{FileStore, UiEvents};
use super::state::{ActivityStore, SettingsStore};
use super::supervisor::P2pSupervisor;
use crate::domain::activity::ActivityRecord;
use crate::domain::error::AppResult;
use crate::domain::files::{FileEntry, FolderStats};
use crate::domain::path::RelativePath;
use crate::domain::peer::PeerId;
use crate::domain::transfer::TransferId;

#[derive(Clone)]
pub struct AppServices {
    pub supervisor: Arc<P2pSupervisor>,
    pub folder_watch: Arc<FolderWatch>,
    pub settings: SettingsStore,
    pub activity: ActivityStore,
    pub files: Arc<dyn FileStore>,
    pub ui: Arc<dyn UiEvents>,
}

impl AppServices {
    pub async fn start_p2p(&self) -> AppResult<()> {
        self.supervisor.start().await
    }

    pub async fn stop_p2p(&self) -> AppResult<()> {
        self.supervisor.stop().await
    }

    pub async fn p2p_status(&self) -> bool {
        self.supervisor.is_running().await
    }

    pub async fn dial(&self, address: String) -> AppResult<()> {
        let engine = self.supervisor.engine().await?;
        engine.send(EngineCommand::Dial { address }).await
    }

    pub async fn peers(&self) -> AppResult<Vec<PeerSummary>> {
        let engine = self.supervisor.engine().await?;
        engine.ask(|reply| EngineCommand::Peers { reply }).await
    }

    pub async fn transfer_state(&self, transfer_id: &TransferId) -> AppResult<TransferState> {
        let engine = self.supervisor.engine().await?;
        let transfer_id = transfer_id.clone();
        engine
            .ask(|reply| EngineCommand::TransferState { transfer_id, reply })
            .await
    }

    pub async fn request_file_list(&self, peer: &str, path: String) -> AppResult<()> {
        let peer = PeerId::parse(peer)?;
        let engine = self.supervisor.engine().await?;
        engine
            .send(EngineCommand::RequestFileList { peer, path })
            .await
    }

    pub async fn download(&self, peer: &str, path: &str) -> AppResult<TransferId> {
        let peer = PeerId::parse(peer)?;
        let path = RelativePath::parse(path)?;
        let engine = self.supervisor.engine().await?;
        engine
            .ask(|reply| EngineCommand::Download { peer, path, reply })
            .await
    }

    pub async fn upload(
        &self,
        peer: &str,
        file: PathBuf,
        remote_path: &str,
    ) -> AppResult<TransferId> {
        let peer = PeerId::parse(peer)?;
        let remote_dir = RelativePath::parse(remote_path)?;
        let engine = self.supervisor.engine().await?;
        engine
            .ask(|reply| EngineCommand::Upload {
                peer,
                file,
                remote_dir,
                reply,
            })
            .await
    }

    pub async fn cancel(&self, transfer_id: String) -> AppResult<()> {
        let engine = self.supervisor.engine().await?;
        engine
            .send(EngineCommand::Cancel {
                transfer_id: TransferId::from_wire(transfer_id),
            })
            .await
    }

    pub async fn request_pairing(&self, peer: &str) -> AppResult<String> {
        let peer = PeerId::parse(peer)?;
        let engine = self.supervisor.engine().await?;
        let pin = engine
            .ask(|reply| EngineCommand::RequestPairing { peer, reply })
            .await?;
        Ok(pin.as_str().to_string())
    }

    pub async fn respond_pairing(&self, peer: &str, approved: bool) -> AppResult<()> {
        let peer = PeerId::parse(peer)?;
        let engine = self.supervisor.engine().await?;
        engine
            .ask(|reply| EngineCommand::RespondPairing {
                peer,
                approved,
                reply,
            })
            .await
    }

    pub async fn unpair(&self, peer: &str) -> AppResult<()> {
        let peer = PeerId::parse(peer)?;
        let engine = self.supervisor.engine().await?;
        engine
            .ask(|reply| EngineCommand::Unpair { peer, reply })
            .await
    }

    pub fn set_device_name(&self, name: String) {
        self.settings.update(|s| s.device_name = name);
    }

    pub fn device_name(&self) -> String {
        self.settings.get().device_name
    }

    pub async fn set_shared_folder(&self, path: PathBuf) -> AppResult<()> {
        self.files.ensure_dir(&path).await?;
        self.folder_watch.restart(&path, || {
            self.settings.update(|s| s.shared_folder = path.clone())
        })?;
        self.ui.emit(UiEvent::SharedFolderChanged {
            path: path.display().to_string(),
        });
        Ok(())
    }

    pub fn shared_folder(&self) -> PathBuf {
        self.settings.get().shared_folder
    }

    pub fn set_shared_folder_display_name(&self, name: String) {
        self.settings
            .update(|s| s.shared_folder_display_name = name);
    }

    pub fn shared_folder_display_name(&self) -> String {
        self.settings.get().shared_folder_display_name
    }

    pub fn set_show_folder_name(&self, show: bool) {
        self.settings.update(|s| s.show_folder_name = show);
    }

    pub fn show_folder_name(&self) -> bool {
        self.settings.get().show_folder_name
    }

    pub async fn local_shared_files(&self, relative_path: &str) -> AppResult<Vec<FileEntry>> {
        let rel = RelativePath::parse(relative_path)?;
        self.files.list_dir(&self.shared_folder(), &rel).await
    }

    pub fn activity_history(&self) -> Vec<ActivityRecord> {
        self.activity.snapshot()
    }

    pub async fn shared_folder_stats(&self) -> AppResult<FolderStats> {
        self.files.folder_stats(&self.shared_folder()).await
    }
}
