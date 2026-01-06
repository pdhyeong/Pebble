use anyhow::{Context, Result};
use blake3::Hasher;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// blake3를 사용하여 파일의 해시값을 계산합니다.
///
/// # Arguments
/// * `file_path` - 해시를 계산할 파일의 경로
///
/// # Returns
/// * `Result<String>` - 성공 시 16진수 문자열 형태의 해시값, 실패 시 에러
///
/// # Security
/// - blake3는 암호학적으로 안전한 해시 함수로, 파일 무결성 검증에 적합합니다
/// - 충돌 공격에 강하며, SHA-256보다 빠른 성능을 제공합니다
pub fn calculate_file_hash<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let path = file_path.as_ref();

    // 파일 존재 여부 확인
    if !path.exists() {
        anyhow::bail!("File does not exist: {}", path.display());
    }

    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", path.display());
    }

    // 파일 열기
    let file = File::open(path)
        .with_context(|| format!("Failed to open file: {}", path.display()))?;

    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();

    // 버퍼 크기는 64KB로 설정 (성능과 메모리 사용량의 균형)
    let mut buffer = vec![0u8; 65536];

    // 파일을 청크 단위로 읽어 해시 계산
    loop {
        let bytes_read = reader.read(&mut buffer)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
    }

    // 해시 값을 16진수 문자열로 변환
    let hash = hasher.finalize();
    Ok(hash.to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_calculate_hash_empty_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let hash = calculate_file_hash(temp_file.path()).unwrap();

        // blake3의 빈 파일 해시값
        assert_eq!(hash, "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262e00f03e7b69af26b7faaf09fcd333050338ddfe085b8cc869ca98b206c08243a");
    }

    #[test]
    fn test_calculate_hash_with_content() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Hello, Pebble!").unwrap();
        temp_file.flush().unwrap();

        let hash = calculate_file_hash(temp_file.path()).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 128); // blake3는 512비트 (128 hex chars)
    }

    #[test]
    fn test_hash_consistency() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"Test data").unwrap();
        temp_file.flush().unwrap();

        let hash1 = calculate_file_hash(temp_file.path()).unwrap();
        let hash2 = calculate_file_hash(temp_file.path()).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_nonexistent_file() {
        let result = calculate_file_hash("/nonexistent/path/to/file.txt");
        assert!(result.is_err());
    }
}
