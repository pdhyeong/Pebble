use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct FolderStats {
    pub total_size: u64,
    pub file_count: usize,
}

#[derive(Debug, Clone)]
pub struct FileSnapshot {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub modified_at: Option<String>,
}

pub fn is_ignored_name(name: &str) -> bool {
    const IGNORED_SUFFIXES: [&str; 4] = [".swp", ".swx", ".part", ".tmp"];
    name.starts_with('.') || IGNORED_SUFFIXES.iter().any(|s| name.ends_with(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_hidden_and_temporary_files() {
        for name in [
            ".DS_Store",
            ".hidden",
            "a.swp",
            "a.swx",
            "movie.mp4.part",
            "x.tmp",
        ] {
            assert!(is_ignored_name(name), "{name}");
        }
        for name in ["a.txt", "report.partial", "tmp"] {
            assert!(!is_ignored_name(name), "{name}");
        }
    }
}
