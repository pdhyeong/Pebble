use serde_json::Value;
use tauri::{AppHandle, Emitter};

use pebble_core::application::events::UiEvent;
use pebble_core::application::ports::UiEvents;

pub struct TauriUiEvents {
    app: AppHandle,
}

impl TauriUiEvents {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl UiEvents for TauriUiEvents {
    fn emit(&self, event: UiEvent) {
        let value = match serde_json::to_value(&event) {
            Ok(value) => value,
            Err(e) => {
                eprintln!("[ui] 이벤트를 직렬화하지 못했습니다: {e}");
                return;
            }
        };
        let (Some(name), Some(payload)) = (
            value.get("event").and_then(Value::as_str),
            value.get("payload"),
        ) else {
            eprintln!("[ui] 이벤트 모양이 올바르지 않습니다: {value}");
            return;
        };
        if let Err(e) = self.app.emit(name, payload) {
            eprintln!("[ui] {name} 이벤트를 보내지 못했습니다: {e}");
        }
    }
}
