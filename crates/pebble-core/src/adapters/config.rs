use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ConfigPaths {
    root: PathBuf,
}

impl ConfigPaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn user_default() -> Self {
        let base = dirs::config_dir()
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(base.join("pebble"))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn identity(&self) -> PathBuf {
        self.root.join("identity.key")
    }

    pub fn trusted_peers(&self) -> PathBuf {
        self.root.join("trusted-peers.json")
    }

    pub fn control_socket(&self) -> PathBuf {
        self.root.join("control.sock")
    }
}
