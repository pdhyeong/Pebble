use serde::Serialize;
use std::fmt;
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    P2pNotRunning,
    P2pAlreadyRunning,
    InvalidPeerId,
    InvalidPath,
    AccessDenied,
    NotFound,
    Io,
    Network,
    EngineUnavailable,
    Internal,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn io(context: &str, err: &io::Error) -> Self {
        let code = match err.kind() {
            io::ErrorKind::NotFound => ErrorCode::NotFound,
            io::ErrorKind::PermissionDenied => ErrorCode::AccessDenied,
            _ => ErrorCode::Io,
        };
        Self::new(code, format!("{context}: {err}"))
    }

    pub fn not_running() -> Self {
        Self::new(ErrorCode::P2pNotRunning, "P2P 엔진이 실행 중이 아닙니다.")
    }

    pub fn already_running() -> Self {
        Self::new(
            ErrorCode::P2pAlreadyRunning,
            "P2P 엔진이 이미 실행 중입니다.",
        )
    }

    pub fn engine_unavailable() -> Self {
        Self::new(
            ErrorCode::EngineUnavailable,
            "P2P 엔진이 응답하지 않습니다.",
        )
    }

    pub fn access_denied() -> Self {
        Self::new(
            ErrorCode::AccessDenied,
            "공유 폴더 밖에는 접근할 수 없습니다.",
        )
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for AppError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_serializes_with_screaming_code() {
        let json = serde_json::to_value(AppError::not_running()).unwrap();
        assert_eq!(json["code"], "P2P_NOT_RUNNING");
        assert!(json["message"].is_string());
    }

    #[test]
    fn io_error_maps_kind_to_code() {
        let err = io::Error::new(io::ErrorKind::NotFound, "x");
        assert_eq!(AppError::io("열기", &err).code, ErrorCode::NotFound);
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "x");
        assert_eq!(AppError::io("열기", &err).code, ErrorCode::AccessDenied);
    }
}

#[cfg(test)]
mod mutants_d {
    use super::*;

    #[test]
    fn display_shows_the_message() {
        let err = AppError::new(ErrorCode::Io, "디스크가 가득 찼습니다");
        assert_eq!(err.to_string(), "디스크가 가득 찼습니다");
    }
}
