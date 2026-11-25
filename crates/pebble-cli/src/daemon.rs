use std::path::PathBuf;
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};

use pebble_core::adapters::config::ConfigPaths;
use pebble_core::adapters::control;
use pebble_core::adapters::event_hub::EventHub;
use pebble_core::adapters::fs::{JsonTrustStore, LocalFileStore, NotifyFolderWatcher};
use pebble_core::adapters::p2p::{Libp2pNetwork, DEFAULT_PORT};
use pebble_core::adapters::tailscale::TailscaleDirectory;
use pebble_core::application::engine::EngineDeps;
use pebble_core::application::folder_watch::FolderWatch;
use pebble_core::application::ports::{FileStore, NetworkFactory, PeerDirectory, UiEvents};
use pebble_core::application::services::AppServices;
use pebble_core::application::state::{ActivityStore, SettingsStore};
use pebble_core::application::supervisor::P2pSupervisor;
use pebble_core::domain::error::AppResult;
use pebble_core::domain::settings::Settings;

use crate::args::DaemonOptions;
use crate::log::LogPrinter;

pub async fn run(config: ConfigPaths, options: DaemonOptions) -> AppResult<()> {
    let home = dirs::home_dir();
    let port = options.port.unwrap_or(DEFAULT_PORT);
    let mut initial = Settings::with_home(home.clone());
    initial.device_name = options.name.unwrap_or_else(host_name);
    let shared = options.shared.unwrap_or_else(|| initial.shared_folder.clone());
    let downloads = options
        .downloads
        .or_else(dirs::download_dir)
        .unwrap_or_else(|| home.unwrap_or_else(|| PathBuf::from(".")).join("Downloads"));

    let hub = Arc::new(EventHub::new(Some(Arc::new(LogPrinter))));
    let ui: Arc<dyn UiEvents> = hub.clone();
    let files: Arc<dyn FileStore> = Arc::new(LocalFileStore::new(downloads.clone()));
    let settings = SettingsStore::new(initial);
    let activity = ActivityStore::default();
    let engine_deps = EngineDeps {
        ui: ui.clone(),
        files: files.clone(),
        settings: settings.clone(),
        activity: activity.clone(),
        trust: Arc::new(JsonTrustStore::new(config.trusted_peers())),
    };
    let network: Arc<dyn NetworkFactory> = Arc::new(Libp2pNetwork::persistent(&config.identity(), port)?);
    let directory = options
        .tailscale
        .then(|| Arc::new(TailscaleDirectory::new(port)) as Arc<dyn PeerDirectory>);
    let supervisor = Arc::new(P2pSupervisor::new(engine_deps, network, None, directory));
    let folder_watch = Arc::new(FolderWatch::spawn(Arc::new(NotifyFolderWatcher), ui.clone(), supervisor.clone()));
    let services = AppServices {
        supervisor,
        folder_watch,
        settings,
        activity,
        files,
        ui,
    };

    let listener = control::bind(&config.control_socket()).await?;
    services.set_shared_folder(shared.clone()).await?;
    services.start_p2p().await?;
    let server = control::serve(listener, services.clone(), hub.sender());

    let peer = services.supervisor.local_peer().await;
    println!("Pebble daemon 시작");
    println!("  이름       {}", services.device_name());
    println!("  PeerId     {}", peer.map(|p| p.to_string()).unwrap_or_default());
    println!("  포트       {port}");
    println!("  공유 폴더  {}", shared.display());
    println!("  받은 파일  {}", downloads.display());
    println!("  Tailscale  {}", if options.tailscale { "같은 계정 기기 자동 연결" } else { "끔" });

    wait_for_stop().await;
    server.abort();
    let _ = services.stop_p2p().await;
    let _ = tokio::fs::remove_file(config.control_socket()).await;
    println!("Pebble daemon 종료");
    Ok(())
}

async fn wait_for_stop() {
    let terminate = async {
        match signal(SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = terminate => {}
    }
}

fn host_name() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|name| name.trim().split('.').next().map(str::to_string))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "pebble".to_string())
}
