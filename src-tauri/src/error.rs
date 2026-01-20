// Application error types for structured error handling
use serde::Serialize;

/// Application error codes for frontend categorization
#[derive(Debug, Serialize, Clone)]
pub enum ErrorCode {
    /// P2P engine not running
    P2pNotRunning,
    /// P2P engine already running
    P2pAlreadyRunning,
    /// Invalid peer ID format
    InvalidPeerId,
    /// Channel communication failed
    ChannelError,
    /// File not found
    FileNotFound,
    /// IO operation failed
    IoError,
    /// Parse/conversion error
    ParseError,
    /// Unknown error
    Unknown,
}

/// Structured application error
#[derive(Debug, Serialize, Clone)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn p2p_not_running() -> Self {
        Self::new(ErrorCode::P2pNotRunning, "P2P 엔진이 실행 중이 아닙니다.")
    }

    pub fn p2p_already_running() -> Self {
        Self::new(
            ErrorCode::P2pAlreadyRunning,
            "P2P 엔진이 이미 실행 중입니다.",
        )
    }

    pub fn invalid_peer_id(details: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::InvalidPeerId,
            format!("잘못된 PeerId 형식: {}", details.into()),
        )
    }

    pub fn channel_error(details: impl Into<String>) -> Self {
        Self::new(ErrorCode::ChannelError, details)
    }

    pub fn file_not_found(path: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::FileNotFound,
            format!("파일을 찾을 수 없습니다: {}", path.into()),
        )
    }

    pub fn io_error(details: impl Into<String>) -> Self {
        Self::new(ErrorCode::IoError, details)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}
