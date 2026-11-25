use std::path::{Path, PathBuf};

use super::error::{AppError, AppResult, ErrorCode};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RelativePath(Vec<String>);

impl RelativePath {
    pub fn root() -> Self {
        Self::default()
    }

    pub fn parse(raw: &str) -> AppResult<Self> {
        let mut segments = Vec::new();
        for segment in raw.split(['/', '\\']) {
            match segment {
                "" | "." => continue,
                ".." => return Err(invalid_path(raw)),
                s if s.contains('\0') || is_drive_prefix(s) => return Err(invalid_path(raw)),
                s => segments.push(s.to_string()),
            }
        }
        Ok(Self(segments))
    }

    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(String::as_str)
    }

    pub fn file_name(&self) -> Option<FileName> {
        self.0.last().map(|s| FileName(s.clone()))
    }

    pub fn child(&self, name: &FileName) -> Self {
        let mut segments = self.0.clone();
        segments.push(name.0.clone());
        Self(segments)
    }

    pub fn resolve_under(&self, root: &Path) -> PathBuf {
        let mut path = root.to_path_buf();
        path.extend(&self.0);
        path
    }

    pub fn to_display(&self) -> String {
        format!("/{}", self.0.join("/"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileName(String);

impl FileName {
    pub fn parse(raw: &str) -> AppResult<Self> {
        let forbidden = raw.is_empty()
            || raw == "."
            || raw == ".."
            || raw.contains(['/', '\\', '\0'])
            || is_drive_prefix(raw);
        if forbidden {
            return Err(AppError::new(
                ErrorCode::InvalidPath,
                format!("사용할 수 없는 파일 이름입니다: {raw}"),
            ));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn from_path(path: &Path) -> AppResult<Self> {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self::parse(&name)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn partial(&self, tag: &str) -> String {
        let room = MAX_FILE_NAME_BYTES.saturating_sub(tag.len() + PARTIAL_OVERHEAD);
        let mut cut = self.0.len().min(room);
        while !self.0.is_char_boundary(cut) {
            cut -= 1;
        }
        format!(".{}.{tag}.part", &self.0[..cut])
    }

    pub fn numbered(&self, n: u32) -> Self {
        match self.0.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() => Self(format!("{stem} ({n}).{ext}")),
            _ => Self(format!("{} ({n})", self.0)),
        }
    }
}

const MAX_FILE_NAME_BYTES: usize = 255;
const PARTIAL_OVERHEAD: usize = ".".len() + ".".len() + ".part".len();

fn is_drive_prefix(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    let drive = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    drive || (cfg!(windows) && segment.contains(':'))
}

fn invalid_path(raw: &str) -> AppError {
    AppError::new(
        ErrorCode::InvalidPath,
        format!("공유 폴더 밖을 가리키는 경로입니다: {raw}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_normalizes_slashes_and_dots() {
        let rel = RelativePath::parse("/a//./b\\c/").unwrap();
        assert_eq!(rel.to_display(), "/a/b/c");
        assert_eq!(RelativePath::parse("/").unwrap().segments().count(), 0);
        assert_eq!(
            RelativePath::parse("a//b")
                .unwrap()
                .segments()
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
    }

    #[test]
    fn parse_rejects_escape_attempts() {
        for raw in ["..", "/a/../../etc", "a/..", "C:/Windows", "a/\0b", "..\\x"] {
            let err = RelativePath::parse(raw).unwrap_err();
            assert_eq!(err.code, ErrorCode::InvalidPath, "{raw}");
        }
    }

    #[test]
    fn resolve_stays_under_root() {
        let rel = RelativePath::parse("/docs/a.txt").unwrap();
        let root = Path::new("/shared");
        let resolved = rel.resolve_under(root);
        assert!(resolved.starts_with(root));
        assert_eq!(resolved, Path::new("/shared/docs/a.txt"));
    }

    #[test]
    fn file_name_rejects_separators_and_dots() {
        for raw in ["", ".", "..", "a/b", "a\\b", "../x", "C:"] {
            assert!(FileName::parse(raw).is_err(), "{raw}");
        }
        assert_eq!(FileName::parse(".env").unwrap().as_str(), ".env");
        assert_eq!(
            FileName::parse("보고서.pdf").unwrap().partial("ab12"),
            ".보고서.pdf.ab12.part"
        );
        assert_eq!(
            FileName::parse("보고서.pdf").unwrap().numbered(2).as_str(),
            "보고서 (2).pdf"
        );
        assert_eq!(
            FileName::parse("README").unwrap().numbered(1).as_str(),
            "README (1)"
        );
        assert_eq!(
            FileName::parse(".env").unwrap().numbered(1).as_str(),
            ".env (1)"
        );
    }

    #[test]
    fn child_and_file_name_roundtrip() {
        let dir = RelativePath::parse("/docs").unwrap();
        let name = FileName::parse("a.txt").unwrap();
        let child = dir.child(&name);
        assert_eq!(child.to_display(), "/docs/a.txt");
        assert_eq!(child.file_name(), Some(name));
        assert_eq!(RelativePath::root().file_name(), None);
    }
}
