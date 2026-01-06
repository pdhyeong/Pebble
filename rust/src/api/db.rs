use rusqlite::{params, Connection, Result};
use walkdir::WalkDir;
use std::fs;

pub struct FileMetadata {
    pub path: String,
    pub last_modified: i64,
    pub file_hash: String,
    pub sync_status: String,
}

// DB 연결 및 테이블 초기화
pub fn init_db() -> Result<()> {
    let conn = Connection::open("pebble.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            last_modified INTEGER NOT NULL,
            file_hash TEXT NOT NULL,
            sync_status TEXT NOT NULL
        )",
        [],
    )?;
    Ok(())
}

// 파일 정보 저장 또는 업데이트 (Upsert)
pub fn upsert_file(file: FileMetadata) -> Result<()> {
    let conn = Connection::open("pebble.db")?;
    conn.execute(
        "INSERT INTO files (path, last_modified, file_hash, sync_status)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(path) DO UPDATE SET
            last_modified = excluded.last_modified,
            file_hash = excluded.file_hash,
            sync_status = excluded.sync_status",
        params![file.path, file.last_modified, file.file_hash, file.sync_status],
    )?;
    Ok(())
}

// 동기화가 필요한 파일 목록 가져오기
pub fn get_pending_files() -> Result<Vec<String>> {
    let conn = Connection::open("pebble.db")?;
    let mut stmt = conn.prepare("SELECT path FROM files WHERE sync_status = 'Pending'")?;
    let rows = stmt.query_map([], |row| row.get(0))?;

    let mut paths = Vec::new();
    for path in rows {
        paths.push(path?);
    }
    Ok(paths)
}

pub fn scan_directory(base_path: &str) -> Result<()> {
    for entry in WalkDir::new(base_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        if path.is_file() {
            let metadata = fs::metadata(path).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let last_modified = metadata.modified()
                .unwrap_or(std::time::SystemTime::now())
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            let path_str = path.to_string_lossy().to_string();

            let file_hash = "initial_scan".to_string();

            upsert_file(FileMetadata {
                path: path_str,
                last_modified,
                file_hash,
                sync_status: "Synced".to_string(), // 초기 스캔 시에는 일단 Synced로 간주
            })?;
        }
    }
    Ok(())
}

/// 특정 파일의 sync_status를 업데이트합니다.
///
/// # Arguments
/// * `path` - 업데이트할 파일의 경로
/// * `status` - 새로운 동기화 상태 (예: "Pending", "Synced", "Failed")
///
/// # Security Notes
/// - SQL Injection 방지를 위해 파라미터화된 쿼리 사용
/// - 트랜잭션 없이 단일 업데이트만 수행하여 성능 최적화
pub fn update_sync_status(path: &str, status: &str) -> Result<()> {
    let conn = Connection::open("pebble.db")?;
    let rows_affected = conn.execute(
        "UPDATE files SET sync_status = ?1 WHERE path = ?2",
        params![status, path],
    )?;

    if rows_affected == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }

    Ok(())
}

/// 파일의 해시값과 수정 시간, sync_status를 한 번에 업데이트합니다.
///
/// # Arguments
/// * `path` - 업데이트할 파일의 경로
/// * `last_modified` - Unix timestamp 형식의 수정 시간
/// * `file_hash` - 파일의 blake3 해시값
/// * `sync_status` - 동기화 상태
///
/// # Security Notes
/// - 원자적 업데이트로 데이터 무결성 보장
/// - 파라미터화된 쿼리로 SQL Injection 방지
pub fn update_file_metadata(path: &str, last_modified: i64, file_hash: &str, sync_status: &str) -> Result<()> {
    let conn = Connection::open("pebble.db")?;
    conn.execute(
        "UPDATE files SET last_modified = ?1, file_hash = ?2, sync_status = ?3 WHERE path = ?4",
        params![last_modified, file_hash, sync_status, path],
    )?;
    Ok(())
}

/// 특정 경로의 파일 정보를 가져옵니다.
///
/// # Arguments
/// * `path` - 조회할 파일의 경로
///
/// # Returns
/// * `Option<FileMetadata>` - 파일이 DB에 존재하면 Some, 없으면 None
pub fn get_file_metadata(path: &str) -> Result<Option<FileMetadata>> {
    let conn = Connection::open("pebble.db")?;
    let mut stmt = conn.prepare(
        "SELECT path, last_modified, file_hash, sync_status FROM files WHERE path = ?1"
    )?;

    let mut rows = stmt.query(params![path])?;

    if let Some(row) = rows.next()? {
        Ok(Some(FileMetadata {
            path: row.get(0)?,
            last_modified: row.get(1)?,
            file_hash: row.get(2)?,
            sync_status: row.get(3)?,
        }))
    } else {
        Ok(None)
    }
}