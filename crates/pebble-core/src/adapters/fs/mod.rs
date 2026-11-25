mod trust;
mod watcher;

pub use trust::JsonTrustStore;
pub use watcher::NotifyFolderWatcher;

use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::application::ports::{ChunkSink, ChunkSource, FileStore, IncomingTarget, OpenedFile};
use crate::domain::error::{AppError, AppResult, ErrorCode};
use crate::domain::files::{is_ignored_name, FileEntry, FileSnapshot, FolderStats};
use crate::domain::path::{FileName, RelativePath};
use crate::domain::transfer::CHUNK_SIZE;

pub struct LocalFileStore {
    downloads: PathBuf,
}

impl LocalFileStore {
    pub fn new(downloads: PathBuf) -> Self {
        Self { downloads }
    }
}

#[async_trait]
impl FileStore for LocalFileStore {
    async fn ensure_dir(&self, dir: &Path) -> AppResult<()> {
        fs::create_dir_all(dir)
            .await
            .map_err(|e| AppError::io("폴더를 만들지 못했습니다", &e))
    }

    async fn list_dir(&self, root: &Path, rel: &RelativePath) -> AppResult<Vec<FileEntry>> {
        self.ensure_dir(root).await?;
        let dir = resolve_inside(root, rel).await?;
        let mut entries = fs::read_dir(&dir)
            .await
            .map_err(|e| AppError::io("폴더를 읽지 못했습니다", &e))?;
        let mut files = Vec::new();
        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| AppError::io("폴더를 읽지 못했습니다", &e))?
        {
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_ignored_name(&name) {
                continue;
            }
            let Ok(file_name) = FileName::parse(&name) else {
                continue;
            };
            let metadata = entry.metadata().await.ok();
            files.push(FileEntry {
                path: rel.child(&file_name).to_display(),
                name,
                is_dir: metadata.as_ref().is_some_and(|m| m.is_dir()),
                size: metadata.as_ref().map_or(0, |m| m.len()),
            });
        }
        Ok(files)
    }

    async fn folder_stats(&self, root: &Path) -> AppResult<FolderStats> {
        let root = root.to_path_buf();
        tokio::task::spawn_blocking(move || walk_stats(&root))
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
    }

    async fn snapshot_top_level(&self, root: &Path) -> AppResult<Vec<FileSnapshot>> {
        let root = root.to_path_buf();
        tokio::task::spawn_blocking(move || snapshot(&root))
            .await
            .map_err(|e| AppError::new(ErrorCode::Internal, e.to_string()))?
    }

    async fn open_shared(&self, root: &Path, rel: &RelativePath) -> AppResult<OpenedFile> {
        let path = resolve_inside(root, rel).await?;
        open_file(&path).await
    }

    async fn open_local(&self, path: &Path) -> AppResult<OpenedFile> {
        open_file(path).await
    }

    async fn create_incoming(&self, target: IncomingTarget<'_>, name: &FileName) -> AppResult<Box<dyn ChunkSink>> {
        let dir = match target {
            IncomingTarget::Shared { root, dir } => {
                self.ensure_dir(root).await?;
                create_inside(root, dir).await?
            }
            IncomingTarget::Downloads => {
                self.ensure_dir(&self.downloads).await?;
                self.downloads.clone()
            }
        };
        let tag = uuid::Uuid::new_v4().simple().to_string();
        let part_path = dir.join(name.partial(&tag[..12]));
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&part_path)
            .await
            .map_err(|e| AppError::io("받을 파일을 만들지 못했습니다", &e))?;
        Ok(Box::new(PartFile {
            file,
            part_path,
            dir,
            name: name.clone(),
        }))
    }
}

async fn resolve_inside(root: &Path, rel: &RelativePath) -> AppResult<PathBuf> {
    let canonical_root = fs::canonicalize(root)
        .await
        .map_err(|e| AppError::io("공유 폴더를 확인하지 못했습니다", &e))?;
    let target = fs::canonicalize(rel.resolve_under(root))
        .await
        .map_err(|e| AppError::io("경로를 찾지 못했습니다", &e))?;
    if !target.starts_with(&canonical_root) {
        return Err(AppError::access_denied());
    }
    Ok(target)
}

async fn create_inside(root: &Path, rel: &RelativePath) -> AppResult<PathBuf> {
    let canonical_root = fs::canonicalize(root)
        .await
        .map_err(|e| AppError::io("공유 폴더를 확인하지 못했습니다", &e))?;
    let mut current = canonical_root.clone();
    for segment in rel.segments() {
        let next = current.join(segment);
        match fs::create_dir(&next).await {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(AppError::io("폴더를 만들지 못했습니다", &e)),
        }
        let resolved = fs::canonicalize(&next)
            .await
            .map_err(|e| AppError::io("경로를 찾지 못했습니다", &e))?;
        if !resolved.starts_with(&canonical_root) {
            return Err(AppError::access_denied());
        }
        current = resolved;
    }
    Ok(current)
}

async fn open_file(path: &Path) -> AppResult<OpenedFile> {
    let file = fs::File::open(path)
        .await
        .map_err(|e| AppError::io("파일을 열지 못했습니다", &e))?;
    let metadata = file
        .metadata()
        .await
        .map_err(|e| AppError::io("파일 정보를 읽지 못했습니다", &e))?;
    if !metadata.is_file() {
        return Err(AppError::new(ErrorCode::InvalidPath, "파일이 아닙니다."));
    }
    let size = metadata.len();
    Ok(OpenedFile {
        name: FileName::from_path(path)?,
        size,
        source: Box::new(FileSource {
            reader: file.take(size),
        }),
    })
}

struct FileSource {
    reader: tokio::io::Take<fs::File>,
}

#[async_trait]
impl ChunkSource for FileSource {
    async fn next_chunk(&mut self) -> AppResult<Option<Vec<u8>>> {
        let mut buf = Vec::with_capacity(CHUNK_SIZE);
        (&mut self.reader)
            .take(CHUNK_SIZE as u64)
            .read_to_end(&mut buf)
            .await
            .map_err(|e| AppError::io("파일을 읽지 못했습니다", &e))?;
        Ok((!buf.is_empty()).then_some(buf))
    }
}

const MAX_NAME_ATTEMPTS: u32 = 1000;

struct PartFile {
    file: fs::File,
    part_path: PathBuf,
    dir: PathBuf,
    name: FileName,
}

#[async_trait]
impl ChunkSink for PartFile {
    async fn write(&mut self, data: &[u8]) -> AppResult<()> {
        self.file
            .write_all(data)
            .await
            .map_err(|e| AppError::io("파일에 쓰지 못했습니다", &e))
    }

    async fn commit(self: Box<Self>) -> AppResult<PathBuf> {
        let PartFile {
            mut file,
            part_path,
            dir,
            name,
        } = *self;
        file.flush()
            .await
            .map_err(|e| AppError::io("파일을 마무리하지 못했습니다", &e))?;
        file.sync_all()
            .await
            .map_err(|e| AppError::io("파일을 마무리하지 못했습니다", &e))?;
        drop(file);
        let final_path = claim_free_name(&part_path, &dir, &name).await?;
        if let Err(e) = fs::remove_file(&part_path).await {
            eprintln!("[fs] 임시 파일 삭제 실패 {}: {e}", part_path.display());
        }
        Ok(final_path)
    }

    async fn abort(self: Box<Self>) {
        let PartFile { file, part_path, .. } = *self;
        drop(file);
        if let Err(e) = fs::remove_file(&part_path).await {
            eprintln!("[fs] 임시 파일 삭제 실패 {}: {e}", part_path.display());
        }
    }
}

async fn claim_free_name(part_path: &Path, dir: &Path, name: &FileName) -> AppResult<PathBuf> {
    for n in 0..MAX_NAME_ATTEMPTS {
        let candidate = if n == 0 { name.clone() } else { name.numbered(n) };
        let final_path = dir.join(candidate.as_str());
        match fs::hard_link(part_path, &final_path).await {
            Ok(()) => return Ok(final_path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(AppError::io("받은 파일을 옮기지 못했습니다", &e)),
        }
    }
    Err(AppError::new(ErrorCode::Io, "같은 이름의 파일이 너무 많습니다."))
}

fn walk_stats(root: &Path) -> AppResult<FolderStats> {
    let mut stats = FolderStats::default();
    if !root.is_dir() {
        return Ok(stats);
    }
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir).map_err(|e| AppError::io("폴더를 읽지 못했습니다", &e))?;
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                pending.push(entry.path());
            } else if file_type.is_file() {
                stats.total_size += entry.metadata().map_or(0, |m| m.len());
                stats.file_count += 1;
            }
        }
    }
    Ok(stats)
}

fn snapshot(root: &Path) -> AppResult<Vec<FileSnapshot>> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Ok(Vec::new());
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        if !metadata.is_file() || is_ignored_name(&name) {
            continue;
        }
        let modified_at = metadata
            .modified()
            .ok()
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
        files.push(FileSnapshot {
            name,
            path: entry.path().display().to_string(),
            size: metadata.len(),
            modified_at,
        });
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> LocalFileStore {
        LocalFileStore::new(std::env::temp_dir())
    }

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pebble-fs-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn symlink_outside_root_is_denied() {
        let root = temp_root("root");
        let outside = temp_root("outside");
        std::fs::write(outside.join("secret.txt"), b"x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        #[cfg(unix)]
        {
            let rel = RelativePath::parse("/link/secret.txt").unwrap();
            let err = store().open_shared(&root, &rel).await.err().unwrap();
            assert_eq!(err.code, ErrorCode::AccessDenied);
        }
    }

    #[tokio::test]
    async fn incoming_file_appears_only_after_commit() {
        let root = temp_root("incoming");
        let dir = RelativePath::parse("/sub").unwrap();
        let name = FileName::parse("a.bin").unwrap();
        let mut sink = store()
            .create_incoming(IncomingTarget::Shared { root: &root, dir: &dir }, &name)
            .await
            .unwrap();
        sink.write(b"hello").await.unwrap();
        assert!(!root.join("sub/a.bin").exists());
        let names = || -> Vec<String> {
            std::fs::read_dir(root.join("sub"))
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect()
        };
        let during = names();
        assert_eq!(during.len(), 1);
        assert!(during[0].starts_with(".a.bin.") && during[0].ends_with(".part"));
        let saved = sink.commit().await.unwrap();
        assert_eq!(std::fs::read(&saved).unwrap(), b"hello");
        assert_eq!(names(), vec!["a.bin".to_string()]);
    }

    #[tokio::test]
    async fn same_name_transfers_never_mix_or_overwrite() {
        let root = temp_root("same-name");
        let name = FileName::parse("s.txt").unwrap();
        let top = RelativePath::root();
        let target = || IncomingTarget::Shared { root: &root, dir: &top };
        std::fs::write(root.join("s.txt"), b"original").unwrap();
        let mut first = store().create_incoming(target(), &name).await.unwrap();
        let mut second = store().create_incoming(target(), &name).await.unwrap();
        first.write(b"AAAAAAAAAA").await.unwrap();
        second.write(b"BB").await.unwrap();
        let first_saved = first.commit().await.unwrap();
        let second_saved = second.commit().await.unwrap();
        assert_eq!(std::fs::read(root.join("s.txt")).unwrap(), b"original");
        assert_eq!(first_saved.file_name().unwrap(), "s (1).txt");
        assert_eq!(second_saved.file_name().unwrap(), "s (2).txt");
        assert_eq!(std::fs::read(first_saved).unwrap(), b"AAAAAAAAAA");
        assert_eq!(std::fs::read(second_saved).unwrap(), b"BB");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn upload_through_symlink_creates_nothing_outside() {
        let root = temp_root("link-root");
        let outside = temp_root("link-outside");
        std::os::unix::fs::symlink(&outside, root.join("link")).unwrap();
        let dir = RelativePath::parse("/link/made_by_peer").unwrap();
        let name = FileName::parse("x.txt").unwrap();
        let err = store()
            .create_incoming(IncomingTarget::Shared { root: &root, dir: &dir }, &name)
            .await
            .err()
            .unwrap();
        assert_eq!(err.code, ErrorCode::AccessDenied);
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn aborted_incoming_file_is_removed() {
        let root = temp_root("abort");
        let name = FileName::parse("a.bin").unwrap();
        let sink = store()
            .create_incoming(
                IncomingTarget::Shared {
                    root: &root,
                    dir: &RelativePath::root(),
                },
                &name,
            )
            .await
            .unwrap();
        sink.abort().await;
        assert!(std::fs::read_dir(&root).unwrap().next().is_none());
    }

    #[tokio::test]
    async fn chunks_cover_whole_file_and_stop_at_declared_size() {
        let root = temp_root("chunks");
        let data: Vec<u8> = (0..CHUNK_SIZE + 10).map(|i| (i % 251) as u8).collect();
        std::fs::write(root.join("f.bin"), &data).unwrap();
        let mut opened = store().open_local(&root.join("f.bin")).await.unwrap();
        assert_eq!(opened.size, data.len() as u64);
        let first = opened.source.next_chunk().await.unwrap().unwrap();
        let second = opened.source.next_chunk().await.unwrap().unwrap();
        assert_eq!(first.len(), CHUNK_SIZE);
        assert_eq!(second.len(), 10);
        assert!(opened.source.next_chunk().await.unwrap().is_none());
        assert_eq!([first, second].concat(), data);
    }

    #[tokio::test]
    async fn list_dir_reports_relative_paths() {
        let root = temp_root("list");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs/a.txt"), b"abc").unwrap();
        std::fs::write(root.join("docs/.a.txt.0123.part"), b"half").unwrap();
        let rel = RelativePath::parse("/docs").unwrap();
        let files = store().list_dir(&root, &rel).await.unwrap();
        assert_eq!(
            files,
            vec![FileEntry {
                name: "a.txt".into(),
                path: "/docs/a.txt".into(),
                is_dir: false,
                size: 3
            }]
        );
    }

    #[tokio::test]
    async fn folder_stats_counts_nested_files() {
        let root = temp_root("stats");
        std::fs::create_dir_all(root.join("a/b")).unwrap();
        std::fs::write(root.join("x"), b"12").unwrap();
        std::fs::write(root.join("a/b/y"), b"345").unwrap();
        let stats = store().folder_stats(&root).await.unwrap();
        assert_eq!(stats, FolderStats { total_size: 5, file_count: 2 });
    }
}
