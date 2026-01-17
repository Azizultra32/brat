use std::io;

/// Errors that can occur during engine operations.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// Failed to spawn a new session.
    #[error("spawn failed: {0}")]
    SpawnFailed(String),

    /// The requested session was not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Operation timed out.
    #[error("timeout after {0}ms")]
    Timeout(u64),

    /// Session has already exited.
    #[error("session already exited: {0}")]
    SessionExited(String),

    /// Failed to send input to session.
    #[error("send failed: {0}")]
    SendFailed(String),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

impl EngineError {
    /// Returns an exit code suitable for CLI usage.
    pub fn exit_code(&self) -> i32 {
        match self {
            EngineError::SpawnFailed(_) => 1,
            EngineError::SessionNotFound(_) => 3,
            EngineError::Timeout(_) => 4,
            EngineError::SessionExited(_) => 5,
            EngineError::SendFailed(_) => 6,
            EngineError::Io(_) => 7,
        }
    }
}
