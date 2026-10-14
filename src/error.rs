//! Error types.

use std::fmt;

/// Base58 decoding error code, mirrors `Base58.ErrorCode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Base58ErrorCode {
    /// Checksum mismatch.
    Checksum,
    /// Invalid input.
    Invalid,
}

impl fmt::Display for Base58ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Base58ErrorCode::Checksum => f.write_str("E_CHECKSUM"),
            Base58ErrorCode::Invalid => f.write_str("E_INVALID"),
        }
    }
}

/// Detailed Base58 decoding error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Base58Error {
    pub message: String,
    pub code: Base58ErrorCode,
    /// Checksum computed from the decoded data (checksum errors only).
    pub actual: Vec<u8>,
    /// Checksum embedded in the encoded string (checksum errors only).
    pub expected: Vec<u8>,
    /// The decoded payload without checksum (checksum errors only).
    pub data: Vec<u8>,
    /// Hash used for the checksum, `double_sha256` or `ripemd160` (checksum errors only).
    pub hash: &'static str,
}

impl Base58Error {
    pub(crate) fn invalid(message: impl Into<String>) -> Self {
        Base58Error {
            message: message.into(),
            code: Base58ErrorCode::Invalid,
            actual: vec![],
            expected: vec![],
            data: vec![],
            hash: "",
        }
    }
}

impl fmt::Display for Base58Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

/// Library error type.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Base58(Base58Error),
    /// Generic error with a message.
    #[error("{0}")]
    Msg(String),
    /// Error while ABI encoding, with the coding path.
    #[error("Encoding error at {path}: {message}")]
    Encoding { path: String, message: String },
    /// Error while ABI decoding, with the coding path.
    #[error("Decoding error at {path}: {message}")]
    Decoding { path: String, message: String },
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[cfg(feature = "api")]
    #[error(transparent)]
    Api(Box<crate::api::APIError>),
    #[cfg(feature = "api")]
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

impl Error {
    pub(crate) fn msg(message: impl Into<String>) -> Self {
        Error::Msg(message.into())
    }

    /// Returns the base58 error details if this is a base58 decoding error.
    pub fn as_base58(&self) -> Option<&Base58Error> {
        match self {
            Error::Base58(e) => Some(e),
            _ => None,
        }
    }
}

impl From<Base58Error> for Error {
    fn from(e: Base58Error) -> Self {
        Error::Base58(e)
    }
}

/// Library result type.
pub type Result<T, E = Error> = std::result::Result<T, E>;
