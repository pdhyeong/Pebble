use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::application::ports::{DeviceCredentials, ServerApi};
use crate::domain::error::{AppError, AppResult, ErrorCode};
use crate::domain::files::FileSnapshot;
use crate::domain::peer::PeerId;

pub const SERVER_BASE_URL: &str = "https://pebbletrans.duckdns.org";
pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

pub struct HttpServerApi {
    http: reqwest::Client,
    base_url: String,
    registration_key: String,
}

impl HttpServerApi {
    pub fn new(base_url: impl Into<String>, registration_key: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            http,
            base_url: base_url.into(),
            registration_key: registration_key.into(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }
}

#[derive(Serialize)]
struct RegisterBody<'a> {
    #[serde(rename = "peerId")]
    peer_id: &'a str,
    name: &'a str,
}

#[derive(Deserialize)]
struct RegisterResponse {
    device: RegisteredDevice,
    #[serde(rename = "deviceSecret")]
    device_secret: String,
}

#[derive(Deserialize)]
struct RegisteredDevice {
    id: String,
}

#[derive(Serialize)]
struct HeartbeatBody<'a> {
    #[serde(rename = "peerId")]
    peer_id: &'a str,
    #[serde(rename = "lastIp", skip_serializing_if = "Option::is_none")]
    last_ip: Option<String>,
}

#[derive(Serialize)]
struct ServerFileItem {
    name: String,
    path: String,
    size: u64,
    #[serde(rename = "modifiedAt", skip_serializing_if = "Option::is_none")]
    modified_at: Option<String>,
}

#[derive(Serialize)]
struct FileSyncBody<'a> {
    #[serde(rename = "deviceId")]
    device_id: &'a str,
    files: Vec<ServerFileItem>,
}

#[async_trait]
impl ServerApi for HttpServerApi {
    async fn register(&self, peer: &PeerId, device_name: &str) -> AppResult<DeviceCredentials> {
        let response = self
            .http
            .post(self.url("/devices/register"))
            .header("x-registration-key", &self.registration_key)
            .json(&RegisterBody {
                peer_id: peer.as_str(),
                name: device_name,
            })
            .send()
            .await
            .map_err(|e| network_error("기기 등록", &e))?;
        let parsed: RegisterResponse = ensure_success(response, "기기 등록")
            .await?
            .json()
            .await
            .map_err(|e| network_error("기기 등록 응답 해석", &e))?;
        Ok(DeviceCredentials {
            peer_id: peer.clone(),
            device_db_id: parsed.device.id,
            device_secret: parsed.device_secret,
        })
    }

    async fn heartbeat(&self, creds: &DeviceCredentials) -> AppResult<()> {
        let last_ip = local_ip_address::local_ip().ok().map(|ip| ip.to_string());
        let response = self
            .http
            .patch(self.url("/devices/heartbeat"))
            .header("x-device-id", creds.peer_id.as_str())
            .header("x-device-secret", &creds.device_secret)
            .json(&HeartbeatBody {
                peer_id: creds.peer_id.as_str(),
                last_ip,
            })
            .send()
            .await
            .map_err(|e| network_error("heartbeat", &e))?;
        ensure_success(response, "heartbeat").await.map(drop)
    }

    async fn sync_files(
        &self,
        creds: &DeviceCredentials,
        files: Vec<FileSnapshot>,
    ) -> AppResult<()> {
        let files = files
            .into_iter()
            .map(|f| ServerFileItem {
                name: f.name,
                path: f.path,
                size: f.size,
                modified_at: f.modified_at,
            })
            .collect();
        let response = self
            .http
            .post(self.url("/files/sync"))
            .header("x-device-id", creds.peer_id.as_str())
            .header("x-device-secret", &creds.device_secret)
            .json(&FileSyncBody {
                device_id: &creds.device_db_id,
                files,
            })
            .send()
            .await
            .map_err(|e| network_error("파일 동기화", &e))?;
        ensure_success(response, "파일 동기화").await.map(drop)
    }
}

async fn ensure_success(response: reqwest::Response, what: &str) -> AppResult<reqwest::Response> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }
    let body = response.text().await.unwrap_or_default();
    Err(AppError::new(
        ErrorCode::Network,
        format!("{what} 실패 HTTP {status}: {body}"),
    ))
}

fn network_error(what: &str, err: &reqwest::Error) -> AppError {
    AppError::new(ErrorCode::Network, format!("{what} 실패: {err}"))
}
