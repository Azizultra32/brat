//! Worktree manager for polecat sessions.
//!
//! This module provides the main `WorktreeManager` struct for creating,
//! tracking, and cleaning up git worktrees for polecat sessions.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::WorktreeError;
use crate::git::{GitWorktree, WorktreeEntry};

/// Information about a managed worktree.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// HEAD commit hash.
    pub head: String,
    /// Whether this is the main worktree (not linked).
    pub is_main: bool,
    /// Whether the worktree is locked.
    pub locked: bool,
    /// Session ID if this worktree is managed by Brat.
    pub session_id: Option<String>,
}

impl From<WorktreeEntry> for WorktreeInfo {
    fn from(entry: WorktreeEntry) -> Self {
        let session_id = extract_session_id_from_path(&entry.path);
        Self {
            path: entry.path,
            head: entry.head,
            is_main: entry.branch.is_some() && !entry.detached,
            locked: entry.locked,
            session_id,
        }
    }
}

/// Report from cleanup operation.
#[derive(Debug, Default)]
pub struct CleanupReport {
    /// Session IDs that were cleaned up.
    pub cleaned: Vec<String>,
    /// Errors encountered during cleanup.
    pub errors: Vec<(String, WorktreeError)>,
}

/// Manages git worktrees for polecat sessions.
///
/// Each polecat session runs in an isolated git worktree to enable
/// parallel work without conflicts. The manager handles creation,
/// tracking, and cleanup of these worktrees.
pub struct WorktreeManager {
    /// Path to the repository root.
    #[allow(dead_code)]
    repo_root: PathBuf,
    /// Root directory for worktrees (e.g., `.grite/worktrees`).
    worktree_root: PathBuf,
    /// Maximum number of polecat worktrees allowed.
    max_polecats: u32,
    /// Git worktree command wrapper.
    git: GitWorktree,
}

impl WorktreeManager {
    /// Create a new WorktreeManager.
    ///
    /// # Arguments
    ///
    /// * `repo_root` - Path to the repository root.
    /// * `worktree_root` - Relative path for worktrees (e.g., `.grite/worktrees`).
    /// * `max_polecats` - Maximum number of concurrent polecat worktrees.
    pub fn new(
        repo_root: impl Into<PathBuf>,
        worktree_root: impl AsRef<str>,
        max_polecats: u32,
    ) -> Self {
        let repo_root = repo_root.into();
        let worktree_root = repo_root.join(worktree_root.as_ref());
        let git = GitWorktree::new(&repo_root);

        Self {
            repo_root,
            worktree_root,
            max_polecats,
            git,
        }
    }

    /// Get the worktree root path.
    pub fn worktree_root(&self) -> &Path {
        &self.worktree_root
    }

    /// Create a new worktree for a session.
    ///
    /// The worktree is named after the session ID and created with a
    /// detached HEAD at the current commit.
    ///
    /// # Arguments
    ///
    /// * `session_id` - Session ID (e.g., `s-20250117-a2f9`).
    ///
    /// # Returns
    ///
    /// The absolute path to the created worktree.
    pub fn create(&self, session_id: &str) -> Result<PathBuf, WorktreeError> {
        // Validate session ID format
        if !is_valid_session_id(session_id) {
            return Err(WorktreeError::InvalidSessionId(session_id.to_string()));
        }

        // Build worktree path
        let worktree_path = self.worktree_root.join(session_id);

        // Check if already exists
        if worktree_path.exists() {
            return Err(WorktreeError::AlreadyExists(session_id.to_string()));
        }

        // Check max polecats limit
        let current_count = self.count_managed_worktrees()?;
        if current_count >= self.max_polecats {
            return Err(WorktreeError::MaxReached {
                current: current_count,
                max: self.max_polecats,
            });
        }

        // Ensure worktree root exists
        fs::create_dir_all(&self.worktree_root)?;

        // Create the worktree with detached HEAD
        self.git.add(&worktree_path, "HEAD", true)?;

        Ok(worktree_path)
    }

    /// List all managed worktrees.
    ///
    /// Returns worktrees in the worktree_root directory, excluding the main worktree.
    pub fn list(&self) -> Result<Vec<WorktreeInfo>, WorktreeError> {
        let entries = self.git.list()?;

        Ok(entries
            .into_iter()
            .map(WorktreeInfo::from)
            .filter(|w| !w.is_main && w.path.starts_with(&self.worktree_root))
            .collect())
    }

    /// Get a worktree by session ID.
    pub fn get(&self, session_id: &str) -> Result<WorktreeInfo, WorktreeError> {
        let worktree_path = self.worktree_root.join(session_id);

        for entry in self.git.list()? {
            if entry.path == worktree_path {
                return Ok(WorktreeInfo::from(entry));
            }
        }

        Err(WorktreeError::NotFound(session_id.to_string()))
    }

    /// Remove a worktree by session ID.
    ///
    /// Uses `--force` to remove even if the worktree has uncommitted changes.
    pub fn remove(&self, session_id: &str) -> Result<(), WorktreeError> {
        let worktree_path = self.worktree_root.join(session_id);

        if !worktree_path.exists() {
            // Already removed, just prune git metadata
            self.git.prune()?;
            return Ok(());
        }

        self.git.remove(&worktree_path, true)?;
        Ok(())
    }

    /// Prune stale worktree administrative files.
    pub fn prune(&self) -> Result<(), WorktreeError> {
        self.git.prune()?;
        Ok(())
    }

    /// Clean up worktrees for sessions that are no longer active.
    ///
    /// # Arguments
    ///
    /// * `active_session_ids` - Set of session IDs that are still active.
    ///
    /// # Returns
    ///
    /// A report of cleaned up worktrees and any errors.
    pub fn cleanup_stale(
        &self,
        active_session_ids: &HashSet<String>,
    ) -> Result<CleanupReport, WorktreeError> {
        let mut report = CleanupReport::default();

        for worktree in self.list()? {
            let Some(session_id) = worktree.session_id else {
                continue; // Not a session worktree
            };

            // Skip if session is active
            if active_session_ids.contains(&session_id) {
                continue;
            }

            // Clean it up
            match self.remove(&session_id) {
                Ok(()) => report.cleaned.push(session_id),
                Err(e) => report.errors.push((session_id, e)),
            }
        }

        // Prune git metadata
        self.prune()?;

        Ok(report)
    }

    /// Count managed worktrees (excluding main).
    fn count_managed_worktrees(&self) -> Result<u32, WorktreeError> {
        Ok(self.list()?.len() as u32)
    }
}

/// Check if a string is a valid session ID.
///
/// Session IDs have the format: `s-YYYYMMDD-XXXX` where XXXX is 4 hex characters.
fn is_valid_session_id(s: &str) -> bool {
    // Format: s-YYYYMMDD-XXXX (e.g., s-20250117-a2f9)
    if s.len() != 15 {
        return false;
    }
    if !s.starts_with("s-") {
        return false;
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 {
        return false;
    }
    // parts[0] = "s"
    // parts[1] = YYYYMMDD (8 chars)
    // parts[2] = XXXX (4 hex chars)
    if parts[1].len() != 8 || !parts[1].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    if parts[2].len() != 4 || !parts[2].chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    true
}

/// Extract session ID from a worktree path.
///
/// Returns the session ID if the path's filename is a valid session ID.
fn extract_session_id_from_path(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    if is_valid_session_id(name) {
        Some(name.to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_session_id() {
        assert!(is_valid_session_id("s-20250117-a2f9"));
        assert!(is_valid_session_id("s-20240101-0000"));
        assert!(is_valid_session_id("s-99991231-ffff"));

        assert!(!is_valid_session_id(""));
        assert!(!is_valid_session_id("s-2025011-a2f9")); // Date too short
        assert!(!is_valid_session_id("s-20250117-a2f")); // Hex too short
        assert!(!is_valid_session_id("t-20250117-a2f9")); // Wrong prefix
        assert!(!is_valid_session_id("s-20250117-a2f9-extra")); // Too long
        assert!(!is_valid_session_id("s-2025011a-a2f9")); // Non-digit in date
        assert!(!is_valid_session_id("s-20250117-ghij")); // Non-hex in suffix
    }

    #[test]
    fn test_extract_session_id_from_path() {
        assert_eq!(
            extract_session_id_from_path(Path::new("/repo/.grite/worktrees/s-20250117-a2f9")),
            Some("s-20250117-a2f9".to_string())
        );
        assert_eq!(
            extract_session_id_from_path(Path::new("s-20250117-a2f9")),
            Some("s-20250117-a2f9".to_string())
        );
        assert_eq!(
            extract_session_id_from_path(Path::new("/repo/.grite/worktrees/not-a-session")),
            None
        );
        assert_eq!(extract_session_id_from_path(Path::new("/repo")), None);
    }
}
