use serde_json::Value;
use tauri::AppHandle;
use tauri_plugin_store::StoreExt;

use pebble_core::application::ports::{CredentialStore, DeviceCredentials};
use pebble_core::domain::error::{AppError, AppResult, ErrorCode};
use pebble_core::domain::peer::PeerId;

const STORE_FILE: &str = "pebble_config.json";
const KEY_SECRET: &str = "device_secret";
const KEY_DB_ID: &str = "device_db_id";
const KEY_PEER: &str = "server_peer_id";

pub struct TauriCredentialStore {
    app: AppHandle,
}

impl TauriCredentialStore {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl CredentialStore for TauriCredentialStore {
    fn load(&self, peer: &PeerId) -> AppResult<Option<DeviceCredentials>> {
        let store = self.app.store(STORE_FILE).map_err(store_error)?;
        let get = |key: &str| store.get(key).and_then(|v| v.as_str().map(String::from));
        let creds = match (get(KEY_SECRET), get(KEY_DB_ID), get(KEY_PEER)) {
            (Some(device_secret), Some(device_db_id), Some(stored_peer))
                if stored_peer == peer.as_str() =>
            {
                Some(DeviceCredentials {
                    peer_id: peer.clone(),
                    device_db_id,
                    device_secret,
                })
            }
            _ => None,
        };
        Ok(creds)
    }

    fn save(&self, creds: &DeviceCredentials) -> AppResult<()> {
        let store = self.app.store(STORE_FILE).map_err(store_error)?;
        store.set(KEY_SECRET, Value::String(creds.device_secret.clone()));
        store.set(KEY_DB_ID, Value::String(creds.device_db_id.clone()));
        store.set(KEY_PEER, Value::String(creds.peer_id.as_str().to_string()));
        store.save().map_err(store_error)
    }
}

fn store_error(e: tauri_plugin_store::Error) -> AppError {
    AppError::new(ErrorCode::Io, format!("설정 저장소 오류: {e}"))
}
