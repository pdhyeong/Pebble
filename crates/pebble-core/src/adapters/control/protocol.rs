use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

use crate::domain::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
pub enum ControlRequest {
    Status,
    Peers,
    Connect { address: String },
    Pair { peer: String },
    Respond { peer: String, approve: bool },
    Ls { peer: String, path: String },
    Send { peer: String, file: PathBuf, dir: String },
    Get { peer: String, path: String },
    Cancel { transfer: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ControlReply {
    Ok { data: Value },
    Event { event: Value },
    Error { code: Value, message: String },
}

impl From<AppError> for ControlReply {
    fn from(e: AppError) -> Self {
        Self::Error {
            code: serde_json::to_value(e.code).unwrap_or(Value::Null),
            message: e.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_use_kebab_case_commands() {
        let raw = r#"{"cmd":"send","peer":"done","file":"/tmp/a.txt","dir":"/"}"#;
        let parsed: ControlRequest = serde_json::from_str(raw).unwrap();
        assert!(matches!(parsed, ControlRequest::Send { ref peer, .. } if peer == "done"));
        let status = serde_json::to_value(ControlRequest::Status).unwrap();
        assert_eq!(status["cmd"], "status");
    }

    #[test]
    fn errors_carry_contract_codes() {
        let reply: ControlReply = AppError::not_running().into();
        let json = serde_json::to_value(reply).unwrap();
        assert_eq!(json["type"], "error");
        assert_eq!(json["code"], "P2P_NOT_RUNNING");
    }
}
