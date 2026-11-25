use async_trait::async_trait;
use std::fmt;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;

use super::events::UiEvent;
use super::net::NetworkLink;
use crate::domain::error::AppResult;
use crate::domain::files::{FileEntry, FileSnapshot, FolderStats};
use crate::domain::path::{FileName, RelativePath};
use crate::domain::peer::PeerId;

pub trait UiEvents: Send + Sync + 'static {
    fn emit(&self, event: UiEvent);
}

pub trait NetworkFactory: Send + Sync + 'static {
    fn start(&self) -> AppResult<NetworkLink>;
}

pub struct OpenedFile {
    pub name: FileName,
    pub size: u64,
    pub source: Box<dyn ChunkSource>,
}

pub enum IncomingTarget<'a> {
    Shared {
        root: &'a Path,
        dir: &'a RelativePath,
    },
    Downloads,
}

#[async_trait]
pub trait ChunkSource: Send + Sync {
    async fn next_chunk(&mut self) -> AppResult<Option<Vec<u8>>>;
}

#[async_trait]
pub trait ChunkSink: Send + Sync {
    async fn write(&mut self, data: &[u8]) -> AppResult<()>;
    async fn commit(self: Box<Self>) -> AppResult<PathBuf>;
    async fn abort(self: Box<Self>);
}

#[async_trait]
pub trait FileStore: Send + Sync + 'static {
    async fn ensure_dir(&self, dir: &Path) -> AppResult<()>;
    async fn list_dir(&self, root: &Path, rel: &RelativePath) -> AppResult<Vec<FileEntry>>;
    async fn folder_stats(&self, root: &Path) -> AppResult<FolderStats>;
    async fn snapshot_top_level(&self, root: &Path) -> AppResult<Vec<FileSnapshot>>;
    async fn open_shared(&self, root: &Path, rel: &RelativePath) -> AppResult<OpenedFile>;
    async fn open_local(&self, path: &Path) -> AppResult<OpenedFile>;
    async fn create_incoming(
        &self,
        target: IncomingTarget<'_>,
        name: &FileName,
    ) -> AppResult<Box<dyn ChunkSink>>;
}

pub trait WatchGuard: Send {}

impl<T: Send> WatchGuard for T {}

pub trait FolderWatcher: Send + Sync + 'static {
    fn watch(&self, path: &Path, changes: mpsc::Sender<PathBuf>) -> AppResult<Box<dyn WatchGuard>>;
}

#[derive(Clone)]
pub struct DeviceCredentials {
    pub peer_id: PeerId,
    pub device_db_id: String,
    pub device_secret: String,
}

impl fmt::Debug for DeviceCredentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceCredentials")
            .field("peer_id", &self.peer_id)
            .field("device_db_id", &self.device_db_id)
            .field("device_secret", &"<hidden>")
            .finish()
    }
}

#[async_trait]
pub trait ServerApi: Send + Sync + 'static {
    async fn register(&self, peer: &PeerId, device_name: &str) -> AppResult<DeviceCredentials>;
    async fn heartbeat(&self, creds: &DeviceCredentials) -> AppResult<()>;
    async fn sync_files(
        &self,
        creds: &DeviceCredentials,
        files: Vec<FileSnapshot>,
    ) -> AppResult<()>;
}

#[async_trait]
pub trait TrustStore: Send + Sync + 'static {
    async fn load(&self) -> AppResult<Vec<PeerId>>;
    async fn save(&self, peers: &[PeerId]) -> AppResult<()>;
}

#[async_trait]
pub trait PeerDirectory: Send + Sync + 'static {
    async fn addresses(&self) -> AppResult<Vec<String>>;
}

pub trait CredentialStore: Send + Sync + 'static {
    fn load(&self, peer: &PeerId) -> AppResult<Option<DeviceCredentials>>;
    fn save(&self, creds: &DeviceCredentials) -> AppResult<()>;
}
