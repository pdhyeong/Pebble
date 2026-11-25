use std::sync::Arc;
use tauri::{AppHandle, Manager};

use crate::adapters::tauri::commands;
use crate::adapters::tauri::credentials::TauriCredentialStore;
use crate::adapters::tauri::events::TauriUiEvents;
use pebble_core::adapters::assembly::{assemble, NodeOptions};
use pebble_core::adapters::config::ConfigPaths;
#[cfg(unix)]
use pebble_core::adapters::control;
use pebble_core::adapters::event_hub::EventHub;
use pebble_core::adapters::p2p::DEFAULT_PORT;
use pebble_core::adapters::server::{HttpServerApi, SERVER_BASE_URL};
use pebble_core::application::server_sync::ServerDeps;
use pebble_core::application::services::AppServices;
use pebble_core::domain::settings::Settings;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            let services = build_services(app.handle())?;
            app.manage(services);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::start_p2p,
            commands::stop_p2p,
            commands::get_p2p_status,
            commands::connect_to_peer,
            commands::request_file_list,
            commands::download_file,
            commands::upload_file,
            commands::cancel_transfer,
            commands::request_pairing,
            commands::respond_pairing,
            commands::set_device_name,
            commands::get_device_name,
            commands::set_shared_folder,
            commands::get_shared_folder,
            commands::set_shared_folder_display_name,
            commands::get_shared_folder_display_name,
            commands::set_show_folder_name,
            commands::get_show_folder_name,
            commands::get_local_shared_files,
            commands::get_local_ip,
            commands::get_activity_history,
            commands::get_shared_folder_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn build_services(app: &AppHandle) -> Result<AppServices, Box<dyn std::error::Error>> {
    let runtime = tauri::async_runtime::handle();
    let _context = runtime.inner().enter();

    let config = std::env::var_os("PEBBLE_CONFIG_DIR")
        .map(|dir| ConfigPaths::new(dir.into()))
        .unwrap_or_else(ConfigPaths::user_default);
    let port = std::env::var("PEBBLE_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let hub = Arc::new(EventHub::new(Some(Arc::new(TauriUiEvents::new(
        app.clone(),
    )))));
    let downloads = dirs::download_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let credentials = Arc::new(TauriCredentialStore::new(app.clone()));
    let services = assemble(
        NodeOptions {
            config: config.clone(),
            port,
            ui: hub.clone(),
            downloads,
            settings: Settings::with_home(dirs::home_dir()),
            tailscale: true,
        },
        |files, settings| {
            Ok(Some(ServerDeps {
                api: Arc::new(HttpServerApi::new(
                    SERVER_BASE_URL,
                    env!("REGISTRATION_KEY"),
                )?),
                credentials,
                files,
                settings,
            }))
        },
    )?;
    spawn_control(&config, &services, &hub);
    Ok(services)
}

#[cfg(unix)]
fn spawn_control(config: &ConfigPaths, services: &AppServices, hub: &EventHub) {
    let socket = config.control_socket();
    let services = services.clone();
    let events = hub.sender();
    tauri::async_runtime::spawn(async move {
        match control::bind(&socket).await {
            Ok(listener) => {
                control::serve(listener, services, events);
            }
            Err(e) => eprintln!("[control] CLI 연결 통로를 열지 못했습니다: {e}"),
        }
    });
}

#[cfg(not(unix))]
fn spawn_control(_config: &ConfigPaths, _services: &AppServices, _hub: &EventHub) {}
