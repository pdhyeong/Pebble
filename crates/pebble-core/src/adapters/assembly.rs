use std::path::PathBuf;
use std::sync::Arc;

use super::config::ConfigPaths;
use super::fs::{JsonTrustStore, LocalFileStore, NotifyFolderWatcher};
use super::p2p::Libp2pNetwork;
use super::tailscale::TailscaleDirectory;
use crate::application::engine::EngineDeps;
use crate::application::folder_watch::FolderWatch;
use crate::application::ports::{FileStore, NetworkFactory, PeerDirectory, UiEvents};
use crate::application::server_sync::ServerDeps;
use crate::application::services::AppServices;
use crate::application::state::{ActivityStore, SettingsStore};
use crate::application::supervisor::P2pSupervisor;
use crate::domain::error::AppResult;
use crate::domain::settings::Settings;

pub struct NodeOptions {
    pub config: ConfigPaths,
    pub port: u16,
    pub ui: Arc<dyn UiEvents>,
    pub downloads: PathBuf,
    pub settings: Settings,
    pub tailscale: bool,
}

pub fn assemble(
    options: NodeOptions,
    server: impl FnOnce(Arc<dyn FileStore>, SettingsStore) -> AppResult<Option<ServerDeps>>,
) -> AppResult<AppServices> {
    let NodeOptions {
        config,
        port,
        ui,
        downloads,
        settings,
        tailscale,
    } = options;
    let files: Arc<dyn FileStore> = Arc::new(LocalFileStore::new(downloads));
    let settings = SettingsStore::new(settings);
    let activity = ActivityStore::default();
    let server = server(files.clone(), settings.clone())?;
    let engine_deps = EngineDeps {
        ui: ui.clone(),
        files: files.clone(),
        settings: settings.clone(),
        activity: activity.clone(),
        trust: Arc::new(JsonTrustStore::new(config.trusted_peers())),
    };
    let network: Arc<dyn NetworkFactory> =
        Arc::new(Libp2pNetwork::persistent(&config.identity(), port)?);
    let directory =
        tailscale.then(|| Arc::new(TailscaleDirectory::new(port)) as Arc<dyn PeerDirectory>);
    let supervisor = Arc::new(P2pSupervisor::new(engine_deps, network, server, directory));
    let folder_watch = Arc::new(FolderWatch::spawn(
        Arc::new(NotifyFolderWatcher),
        ui.clone(),
        supervisor.clone(),
    ));
    Ok(AppServices {
        supervisor,
        folder_watch,
        settings,
        activity,
        files,
        ui,
    })
}
