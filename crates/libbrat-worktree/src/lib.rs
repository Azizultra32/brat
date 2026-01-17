//! Git worktree management for Brat polecat sessions.
//!
//! This crate provides a `WorktreeManager` for creating, tracking, and
//! cleaning up git worktrees. Each polecat session runs in an isolated
//! worktree to enable parallel work without conflicts.
//!
//! # Example
//!
//! ```ignore
//! use libbrat_worktree::WorktreeManager;
//!
//! let manager = WorktreeManager::new("/path/to/repo", ".grit/worktrees", 6);
//!
//! // Create a worktree for a session
//! let path = manager.create("s-20250117-a2f9")?;
//!
//! // List managed worktrees
//! let worktrees = manager.list()?;
//!
//! // Remove a worktree
//! manager.remove("s-20250117-a2f9")?;
//! ```

mod error;
mod git;
mod manager;

pub use error::WorktreeError;
pub use git::WorktreeEntry;
pub use manager::{CleanupReport, WorktreeInfo, WorktreeManager};
