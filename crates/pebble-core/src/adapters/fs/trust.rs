use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

use crate::application::ports::TrustStore;
use crate::domain::error::{AppError, AppResult};
use crate::domain::peer::PeerId;

pub struct JsonTrustStore {
    path: PathBuf,
}

impl JsonTrustStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

#[async_trait]
impl TrustStore for JsonTrustStore {
    async fn load(&self) -> AppResult<Vec<PeerId>> {
        let bytes = match fs::read(&self.path).await {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(AppError::io("믿는 기기 목록을 읽지 못했습니다", &e)),
        };
        let raw: Vec<String> = serde_json::from_slice(&bytes)
            .map_err(|e| AppError::new(crate::domain::error::ErrorCode::Internal, format!("믿는 기기 목록이 손상되었습니다: {e}")))?;
        Ok(raw.iter().filter_map(|id| PeerId::parse(id).ok()).collect())
    }

    async fn save(&self, peers: &[PeerId]) -> AppResult<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)
                .await
                .map_err(|e| AppError::io("설정 폴더를 만들지 못했습니다", &e))?;
        }
        let raw: Vec<&str> = peers.iter().map(PeerId::as_str).collect();
        let bytes = serde_json::to_vec_pretty(&raw)
            .map_err(|e| AppError::new(crate::domain::error::ErrorCode::Internal, e.to_string()))?;
        let temp = self.path.with_extension("json.tmp");
        fs::write(&temp, bytes)
            .await
            .map_err(|e| AppError::io("믿는 기기 목록을 저장하지 못했습니다", &e))?;
        fs::rename(&temp, &self.path)
            .await
            .map_err(|e| AppError::io("믿는 기기 목록을 저장하지 못했습니다", &e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn saved_peers_survive_reload_and_missing_file_is_empty() {
        let dir = std::env::temp_dir().join(format!("pebble-trust-{}", uuid::Uuid::new_v4()));
        let store = JsonTrustStore::new(dir.join("nested/trusted.json"));
        assert!(store.load().await.unwrap().is_empty());
        let peers = vec![PeerId::parse("12D3KooWA").unwrap(), PeerId::parse("12D3KooWB").unwrap()];
        store.save(&peers).await.unwrap();
        assert_eq!(store.load().await.unwrap(), peers);
    }
}
