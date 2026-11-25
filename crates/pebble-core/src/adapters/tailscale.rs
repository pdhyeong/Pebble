use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::process::Command;

use crate::application::ports::PeerDirectory;
use crate::domain::error::{AppError, AppResult, ErrorCode};

pub const STATUS_TIMEOUT: Duration = Duration::from_secs(5);

const BINARIES: [&str; 5] = [
    "tailscale",
    "/usr/local/bin/tailscale",
    "/opt/homebrew/bin/tailscale",
    "/usr/bin/tailscale",
    "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
];

pub struct TailscaleDirectory {
    port: u16,
}

impl TailscaleDirectory {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}

#[async_trait]
impl PeerDirectory for TailscaleDirectory {
    async fn addresses(&self) -> AppResult<Vec<String>> {
        let status = read_status().await?;
        let peers = same_owner_peers(&status)?;
        Ok(peers
            .into_iter()
            .map(|ip| format!("/ip4/{ip}/tcp/{}", self.port))
            .collect())
    }
}

async fn read_status() -> AppResult<Vec<u8>> {
    for binary in BINARIES {
        let run = Command::new(binary)
            .args(["status", "--json"])
            .kill_on_drop(true)
            .output();
        if let Ok(Ok(output)) = tokio::time::timeout(STATUS_TIMEOUT, run).await {
            if output.status.success() {
                return Ok(output.stdout);
            }
        }
    }
    Err(AppError::new(
        ErrorCode::NotFound,
        "tailscale 명령을 찾지 못했거나 Tailscale 이 꺼져 있습니다.",
    ))
}

#[derive(Deserialize)]
struct Status {
    #[serde(rename = "Self")]
    me: Node,
    #[serde(rename = "Peer", default)]
    peers: Option<HashMap<String, Node>>,
}

#[derive(Deserialize)]
struct Node {
    #[serde(rename = "UserID", default)]
    user_id: u64,
    #[serde(rename = "Online", default)]
    online: bool,
    #[serde(rename = "TailscaleIPs", default)]
    ips: Vec<String>,
}

pub fn same_owner_peers(raw: &[u8]) -> AppResult<Vec<Ipv4Addr>> {
    let status: Status = serde_json::from_slice(raw).map_err(|e| {
        AppError::new(
            ErrorCode::Internal,
            format!("tailscale 상태를 읽지 못했습니다: {e}"),
        )
    })?;
    let mut found: Vec<Ipv4Addr> = status
        .peers
        .unwrap_or_default()
        .into_values()
        .filter(|node| node.online && node.user_id == status.me.user_id && status.me.user_id != 0)
        .filter_map(|node| node.ips.iter().find_map(|ip| ip.parse::<Ipv4Addr>().ok()))
        .collect();
    found.sort();
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "Self": {"UserID": 7, "Online": true, "TailscaleIPs": ["100.64.0.1", "fd7a::1"]},
        "Peer": {
            "a": {"UserID": 7, "Online": true, "TailscaleIPs": ["100.64.0.3", "fd7a::3"]},
            "b": {"UserID": 7, "Online": false, "TailscaleIPs": ["100.64.0.4"]},
            "c": {"UserID": 9, "Online": true, "TailscaleIPs": ["100.64.0.5"]},
            "d": {"UserID": 7, "Online": true, "TailscaleIPs": ["100.64.0.2"]}
        }
    }"#;

    #[test]
    fn keeps_only_online_devices_of_the_same_owner() {
        let ips = same_owner_peers(SAMPLE.as_bytes()).unwrap();
        assert_eq!(
            ips,
            vec![Ipv4Addr::new(100, 64, 0, 2), Ipv4Addr::new(100, 64, 0, 3)]
        );
    }

    #[test]
    fn no_peers_means_empty_list() {
        let raw = r#"{"Self": {"UserID": 7, "Online": true, "TailscaleIPs": []}, "Peer": null}"#;
        assert!(same_owner_peers(raw.as_bytes()).unwrap().is_empty());
    }

    #[tokio::test]
    #[ignore = "이 기기에 Tailscale 이 켜져 있어야 합니다"]
    async fn reads_real_tailscale_status() {
        let addresses = TailscaleDirectory::new(47123).addresses().await.unwrap();
        println!("{addresses:?}");
        assert!(addresses
            .iter()
            .all(|a| a.starts_with("/ip4/100.") && a.ends_with("/tcp/47123")));
    }

    #[test]
    fn broken_status_is_an_error() {
        assert!(same_owner_peers(b"not json").is_err());
    }
}
