use serde::Serialize;
use std::fmt;
use std::time::Duration;

pub const CHUNK_SIZE: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct TransferId(String);

impl TransferId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn from_wire(raw: String) -> Self {
        Self(raw)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TransferId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferStatus {
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CancelReason {
    UserCancelled,
    PeerCancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkError {
    OutOfOrder { expected: u32, got: u32 },
    TooManyChunks,
    WrongLength { expected: u64, got: u64 },
}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfOrder { expected, got } => {
                write!(f, "청크 순서가 어긋났습니다 (기대 {expected}, 받음 {got})")
            }
            Self::TooManyChunks => f.write_str("선언한 청크 수보다 많이 받았습니다"),
            Self::WrongLength { expected, got } => {
                write!(f, "청크 크기가 맞지 않습니다 (기대 {expected}, 받음 {got})")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChunkCounter {
    total_size: u64,
    total_chunks: u32,
    done_chunks: u32,
    done_bytes: u64,
}

impl ChunkCounter {
    pub fn new(total_size: u64) -> Self {
        let chunks = total_size.div_ceil(CHUNK_SIZE as u64);
        Self {
            total_size,
            total_chunks: u32::try_from(chunks).unwrap_or(u32::MAX),
            done_chunks: 0,
            done_bytes: 0,
        }
    }

    pub fn accept(&mut self, index: u32, len: usize) -> Result<(), ChunkError> {
        if self.done_chunks >= self.total_chunks {
            return Err(ChunkError::TooManyChunks);
        }
        if index != self.done_chunks {
            return Err(ChunkError::OutOfOrder {
                expected: self.done_chunks,
                got: index,
            });
        }
        let expected = (self.total_size - self.done_bytes).min(CHUNK_SIZE as u64);
        let got = len as u64;
        if got != expected {
            return Err(ChunkError::WrongLength { expected, got });
        }
        self.done_chunks += 1;
        self.done_bytes += got;
        Ok(())
    }

    pub fn next_index(&self) -> u32 {
        self.done_chunks
    }

    pub fn is_complete(&self) -> bool {
        self.done_chunks >= self.total_chunks
    }

    pub fn total_size(&self) -> u64 {
        self.total_size
    }

    pub fn done_bytes(&self) -> u64 {
        self.done_bytes
    }

    pub fn percent(&self) -> f64 {
        if self.total_chunks == 0 {
            return 100.0;
        }
        f64::from(self.done_chunks) * 100.0 / f64::from(self.total_chunks)
    }
}

pub fn speed_bps(total_size: u64, elapsed: Duration) -> Option<f64> {
    let secs = elapsed.as_secs_f64();
    (secs > 0.0).then(|| total_size as f64 / secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_byte_file_is_complete_immediately() {
        let counter = ChunkCounter::new(0);
        assert!(counter.is_complete());
        assert_eq!(counter.percent(), 100.0);
    }

    #[test]
    fn counts_chunks_with_ceiling() {
        assert!(!ChunkCounter::new(1).is_complete());
        let mut counter = ChunkCounter::new(CHUNK_SIZE as u64 + 1);
        counter.accept(0, CHUNK_SIZE).unwrap();
        assert!(!counter.is_complete());
        counter.accept(1, 1).unwrap();
        assert!(counter.is_complete());
        assert_eq!(counter.done_bytes(), CHUNK_SIZE as u64 + 1);
    }

    #[test]
    fn rejects_out_of_order_and_overflow() {
        let mut counter = ChunkCounter::new(10);
        assert_eq!(
            counter.accept(1, 5),
            Err(ChunkError::OutOfOrder { expected: 0, got: 1 })
        );
        assert_eq!(counter.accept(0, 11), Err(ChunkError::WrongLength { expected: 10, got: 11 }));
        counter.accept(0, 10).unwrap();
        assert_eq!(counter.accept(1, 1), Err(ChunkError::TooManyChunks));
    }

    #[test]
    fn rejects_short_chunks_so_truncated_files_never_complete() {
        let mut counter = ChunkCounter::new(2 * CHUNK_SIZE as u64);
        assert_eq!(
            counter.accept(0, 1),
            Err(ChunkError::WrongLength { expected: CHUNK_SIZE as u64, got: 1 })
        );
        counter.accept(0, CHUNK_SIZE).unwrap();
        assert_eq!(
            counter.accept(1, CHUNK_SIZE - 1),
            Err(ChunkError::WrongLength { expected: CHUNK_SIZE as u64, got: CHUNK_SIZE as u64 - 1 })
        );
        assert!(!counter.is_complete());
    }

    #[test]
    fn percent_follows_chunks() {
        let mut counter = ChunkCounter::new(4 * CHUNK_SIZE as u64);
        counter.accept(0, CHUNK_SIZE).unwrap();
        assert_eq!(counter.percent(), 25.0);
    }

    #[test]
    fn speed_is_none_without_elapsed_time() {
        assert_eq!(speed_bps(100, Duration::ZERO), None);
        assert_eq!(speed_bps(100, Duration::from_secs(2)), Some(50.0));
    }

    #[test]
    fn enums_serialize_to_contract_strings() {
        assert_eq!(serde_json::to_value(Direction::Upload).unwrap(), "upload");
        assert_eq!(serde_json::to_value(TransferStatus::Cancelled).unwrap(), "cancelled");
        assert_eq!(serde_json::to_value(CancelReason::PeerCancelled).unwrap(), "peer_cancelled");
    }
}
