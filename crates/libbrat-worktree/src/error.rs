//! Error types for worktree operations.

use thiserror::Error;

/// Error type for worktree operations.
#[derive(Debug, Error)]
pub enum WorktreeError {
    /// Git command failed.
    #[error("git command failed: {0}")]
    GitFailed(String),

    /// Worktree not found.
    #[error("worktree not found: {0}")]
    NotFound(String),

    /// Worktree already exists.
    #[error("worktree already exists: {0}")]
    AlreadyExists(String),

    /// Maximum number of worktrees reached.
    #[error("max worktrees reached: {current}/{max}")]
    MaxReached {
        /// Current number of worktrees.
        current: u32,
        /// Maximum allowed.
        max: u32,
    },

    /// Invalid worktree path.
    #[error("invalid worktree path: {0}")]
    InvalidPath(String),

    /// Invalid session ID.
    #[error("invalid session id: {0}")]
    InvalidSessionId(String),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
