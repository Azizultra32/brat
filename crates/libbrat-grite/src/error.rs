use thiserror::Error;

/// Errors that can occur when interacting with Grite.
#[derive(Debug, Error)]
pub enum GriteError {
    /// Grit command failed to execute or returned an error.
    #[error("grite command failed: {0}")]
    CommandFailed(String),

    /// Entity not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Failed to parse response from Grite.
    #[error("parse error: {0}")]
    ParseError(String),

    /// Unexpected response from Grite.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),

    /// Invalid ID format.
    #[error("invalid ID format: {0}")]
    InvalidId(String),

    /// Invalid state transition.
    #[error("invalid state transition: {0}")]
    InvalidStateTransition(String),

    /// JSON serialization/deserialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
