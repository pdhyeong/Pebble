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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_paths_live_under_the_root() {
        let paths = ConfigPaths::new(PathBuf::from("/cfg/pebble"));
        assert_eq!(paths.root(), Path::new("/cfg/pebble"));
        assert_eq!(paths.identity(), PathBuf::from("/cfg/pebble/identity.key"));
        assert_eq!(
            paths.trusted_peers(),
            PathBuf::from("/cfg/pebble/trusted-peers.json")
        );
        assert_eq!(
            paths.control_socket(),
            PathBuf::from("/cfg/pebble/control.sock")
        );
    }
}
