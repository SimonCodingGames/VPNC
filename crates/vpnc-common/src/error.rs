//! Shared error types for VPNC crates.

use thiserror::Error;

/// Common error type used across VPNC libraries.
#[derive(Debug, Error)]
pub enum Error {
    /// Configuration could not be loaded or was invalid.
    #[error("config error: {0}")]
    Config(String),

    /// An I/O operation failed.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Authentication failed or credentials were invalid.
    #[error("authentication failed")]
    Unauthorized,

    /// The caller is authenticated but not allowed to perform the action.
    #[error("forbidden")]
    Forbidden,

    /// A requested resource was not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// The request was invalid.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// An unexpected internal failure.
    #[error("internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Creates an internal error from any displayable value.
    pub fn internal(err: impl ToString) -> Self {
        Self::Internal(err.to_string())
    }
}
