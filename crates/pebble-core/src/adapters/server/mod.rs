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
    pub fn new(
        base_url: impl Into<String>,
        registration_key: impl Into<String>,
    ) -> AppResult<Self> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| {
                AppError::new(
                    ErrorCode::Internal,
                    format!("서버 연결 도구를 만들지 못했습니다: {e}"),
                )
            })?;
        Ok(Self {
            http,
            base_url: base_url.into(),
            registration_key: registration_key.into(),
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    const PEER: &str = "12D3KooWAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    struct Captured {
        head: String,
        body: serde_json::Value,
    }

    impl Captured {
        fn header(&self, name: &str) -> Option<String> {
            self.head.lines().skip(1).find_map(|line| {
                let (key, value) = line.split_once(':')?;
                key.trim()
                    .eq_ignore_ascii_case(name)
                    .then(|| value.trim().to_string())
            })
        }

        fn request_line(&self) -> &str {
            self.head.lines().next().unwrap()
        }
    }

    async fn serve_once(status: &str, body: &str) -> (String, JoinHandle<Captured>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let response = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let task = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut raw = Vec::new();
            let mut chunk = [0u8; 4096];
            let (head_end, length) = loop {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0, "연결이 요청 도중 닫혔습니다");
                raw.extend_from_slice(&chunk[..n]);
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&raw[..pos]).to_string();
                    let length = head
                        .lines()
                        .find_map(|l| {
                            let (k, v) = l.split_once(':')?;
                            k.trim()
                                .eq_ignore_ascii_case("content-length")
                                .then(|| v.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    break (pos + 4, length);
                }
            };
            while raw.len() < head_end + length {
                let n = socket.read(&mut chunk).await.unwrap();
                assert!(n > 0, "연결이 요청 도중 닫혔습니다");
                raw.extend_from_slice(&chunk[..n]);
            }
            socket.write_all(response.as_bytes()).await.unwrap();
            socket.shutdown().await.unwrap();
            Captured {
                head: String::from_utf8_lossy(&raw[..head_end - 4]).to_string(),
                body: serde_json::from_slice(&raw[head_end..head_end + length])
                    .unwrap_or(serde_json::Value::Null),
            }
        });
        (base, task)
    }

    async fn closed_port_url() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);
        url
    }

    fn creds() -> DeviceCredentials {
        DeviceCredentials {
            peer_id: PeerId::parse(PEER).unwrap(),
            device_db_id: "db-7".into(),
            device_secret: "shh".into(),
        }
    }

    fn api(base: &str) -> HttpServerApi {
        HttpServerApi::new(base, "reg-key").unwrap()
    }

    #[test]
    fn url_joins_base_and_path() {
        assert_eq!(
            api("http://example.test:9").url("/devices/register"),
            "http://example.test:9/devices/register"
        );
    }

    #[tokio::test]
    async fn register_posts_identity_and_returns_credentials() {
        let (base, request) = serve_once(
            "200 OK",
            r#"{"device":{"id":"db-42"},"deviceSecret":"top-secret"}"#,
        )
        .await;
        let peer = PeerId::parse(PEER).unwrap();
        let got = api(&base).register(&peer, "내 노트북").await.unwrap();
        assert_eq!(got.peer_id, peer);
        assert_eq!(got.device_db_id, "db-42");
        assert_eq!(got.device_secret, "top-secret");

        let request = request.await.unwrap();
        assert_eq!(request.request_line(), "POST /devices/register HTTP/1.1");
        assert_eq!(
            request.header("x-registration-key").as_deref(),
            Some("reg-key")
        );
        assert_eq!(request.body["peerId"], PEER);
        assert_eq!(request.body["name"], "내 노트북");
    }

    #[tokio::test]
    async fn register_reports_http_failure_with_status_and_body() {
        let (base, _request) = serve_once("403 Forbidden", "bad key").await;
        let err = api(&base)
            .register(&PeerId::parse(PEER).unwrap(), "x")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert_eq!(err.message, "기기 등록 실패 HTTP 403 Forbidden: bad key");
    }

    #[tokio::test]
    async fn register_reports_unreadable_success_body() {
        let (base, _request) = serve_once("200 OK", "not json").await;
        let err = api(&base)
            .register(&PeerId::parse(PEER).unwrap(), "x")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert!(err.message.starts_with("기기 등록 응답 해석 실패:"));
    }

    #[tokio::test]
    async fn register_reports_connection_failure() {
        let base = closed_port_url().await;
        let err = api(&base)
            .register(&PeerId::parse(PEER).unwrap(), "x")
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert!(err.message.starts_with("기기 등록 실패:"));
    }

    #[tokio::test]
    async fn heartbeat_patches_with_device_headers() {
        let (base, request) = serve_once("200 OK", "{}").await;
        api(&base).heartbeat(&creds()).await.unwrap();
        let request = request.await.unwrap();
        assert_eq!(request.request_line(), "PATCH /devices/heartbeat HTTP/1.1");
        assert_eq!(request.header("x-device-id").as_deref(), Some(PEER));
        assert_eq!(request.header("x-device-secret").as_deref(), Some("shh"));
        assert_eq!(request.body["peerId"], PEER);
    }

    #[tokio::test]
    async fn heartbeat_fails_on_server_error_and_connection_failure() {
        let (base, _request) = serve_once("500 Internal Server Error", "boom").await;
        let err = api(&base).heartbeat(&creds()).await.unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert_eq!(
            err.message,
            "heartbeat 실패 HTTP 500 Internal Server Error: boom"
        );

        let base = closed_port_url().await;
        let err = api(&base).heartbeat(&creds()).await.unwrap_err();
        assert!(err.message.starts_with("heartbeat 실패:"));
    }

    #[tokio::test]
    async fn sync_files_posts_every_file_with_its_metadata() {
        let (base, request) = serve_once("200 OK", "{}").await;
        let files = vec![
            FileSnapshot {
                name: "a.txt".into(),
                path: "/a.txt".into(),
                size: 5,
                modified_at: Some("2026-01-02T03:04:05Z".into()),
            },
            FileSnapshot {
                name: "b.bin".into(),
                path: "/b.bin".into(),
                size: 9,
                modified_at: None,
            },
        ];
        api(&base).sync_files(&creds(), files).await.unwrap();
        let request = request.await.unwrap();
        assert_eq!(request.request_line(), "POST /files/sync HTTP/1.1");
        assert_eq!(request.header("x-device-id").as_deref(), Some(PEER));
        assert_eq!(request.header("x-device-secret").as_deref(), Some("shh"));
        assert_eq!(request.body["deviceId"], "db-7");
        let sent = request.body["files"].as_array().unwrap();
        assert_eq!(sent.len(), 2);
        assert_eq!(sent[0]["name"], "a.txt");
        assert_eq!(sent[0]["path"], "/a.txt");
        assert_eq!(sent[0]["size"], 5);
        assert_eq!(sent[0]["modifiedAt"], "2026-01-02T03:04:05Z");
        assert_eq!(sent[1]["name"], "b.bin");
        assert_eq!(sent[1]["size"], 9);
        assert!(sent[1].get("modifiedAt").is_none());
    }

    #[tokio::test]
    async fn sync_files_fails_on_server_error_and_connection_failure() {
        let (base, _request) = serve_once("502 Bad Gateway", "").await;
        let err = api(&base)
            .sync_files(&creds(), Vec::new())
            .await
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::Network);
        assert!(err.message.starts_with("파일 동기화 실패 HTTP 502"));

        let base = closed_port_url().await;
        let err = api(&base)
            .sync_files(&creds(), Vec::new())
            .await
            .unwrap_err();
        assert!(err.message.starts_with("파일 동기화 실패:"));
    }
}
