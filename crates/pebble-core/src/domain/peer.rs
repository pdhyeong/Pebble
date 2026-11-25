use serde::Serialize;
use std::fmt;

use super::error::{AppError, AppResult, ErrorCode};

const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct PeerId(String);

impl PeerId {
    pub fn parse(raw: &str) -> AppResult<Self> {
        let raw = raw.trim();
        if raw.is_empty() || !raw.chars().all(|c| BASE58_ALPHABET.contains(c)) {
            return Err(AppError::new(
                ErrorCode::InvalidPeerId,
                format!("잘못된 PeerId 형식: {raw}"),
            ));
        }
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_base58_peer_ids() {
        let id = PeerId::parse(" 12D3KooWAbc ").unwrap();
        assert_eq!(id.as_str(), "12D3KooWAbc");
    }

    #[test]
    fn rejects_empty_and_non_base58() {
        for raw in ["", "   ", "12D3Koo0", "abc/def", "peer:addr"] {
            assert_eq!(
                PeerId::parse(raw).unwrap_err().code,
                ErrorCode::InvalidPeerId,
                "{raw}"
            );
        }
    }
}

#[cfg(test)]
mod mutants_d {
    use super::*;

    #[test]
    fn display_shows_the_id() {
        let id = PeerId::parse("12D3KooWA").unwrap();
        assert_eq!(id.to_string(), "12D3KooWA");
    }
}
