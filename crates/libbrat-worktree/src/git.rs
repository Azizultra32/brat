//! Git worktree command wrapper.
//!
//! This module provides a low-level wrapper around git worktree commands.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::WorktreeError;

/// Raw entry from `git worktree list --porcelain`.
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    /// Path to the worktree.
    pub path: PathBuf,
    /// HEAD commit hash.
    pub head: String,
    /// Branch name (if not detached).
    pub branch: Option<String>,
    /// Whether this is a detached HEAD.
    pub detached: bool,
    /// Whether the worktree is locked.
    pub locked: bool,
    /// Whether the worktree is prunable (directory missing).
    pub prunable: bool,
}

/// Git worktree command wrapper.
pub struct GitWorktree {
    repo_root: PathBuf,
}

impl GitWorktree {
    /// Create a new GitWorktree wrapper for the given repository.
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
        }
    }

    /// Add a new worktree.
    ///
    /// # Arguments
    ///
    /// * `path` - Path where the worktree will be created.
    /// * `commit` - Commit, branch, or ref to checkout.
    /// * `detach` - If true, create a detached HEAD worktree.
    pub fn add(&self, path: &Path, commit: &str, detach: bool) -> Result<(), WorktreeError> {
        let mut args = vec!["worktree", "add"];
        if detach {
            args.push("--detach");
        }
        let path_str = path
            .to_str()
            .ok_or_else(|| WorktreeError::InvalidPath(path.display().to_string()))?;
        args.push(path_str);
        args.push(commit);

        self.run(&args)
    }

    /// List all worktrees in porcelain format.
    pub fn list(&self) -> Result<Vec<WorktreeEntry>, WorktreeError> {
        let output = self.run_output(&["worktree", "list", "--porcelain"])?;
        parse_porcelain_output(&output)
    }

    /// Remove a worktree.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the worktree to remove.
    /// * `force` - If true, remove even if worktree has uncommitted changes.
    pub fn remove(&self, path: &Path, force: bool) -> Result<(), WorktreeError> {
        let mut args = vec!["worktree", "remove"];
        if force {
            args.push("--force");
        }
        let path_str = path
            .to_str()
            .ok_or_else(|| WorktreeError::InvalidPath(path.display().to_string()))?;
        args.push(path_str);

        self.run(&args)
    }

    /// Prune stale worktree administrative files.
    pub fn prune(&self) -> Result<String, WorktreeError> {
        self.run_output(&["worktree", "prune", "--verbose"])
    }

    /// Run a git command.
    fn run(&self, args: &[&str]) -> Result<(), WorktreeError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitFailed(stderr.trim().to_string()));
        }
        Ok(())
    }

    /// Run a git command and return stdout.
    fn run_output(&self, args: &[&str]) -> Result<String, WorktreeError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_root)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitFailed(stderr.trim().to_string()));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Parse `git worktree list --porcelain` output.
///
/// Format:
/// ```text
/// worktree /path/to/main
/// HEAD abc123def456
/// branch refs/heads/main
///
/// worktree /path/to/linked
/// HEAD def456abc123
/// detached
/// ```
fn parse_porcelain_output(output: &str) -> Result<Vec<WorktreeEntry>, WorktreeError> {
    let mut entries = Vec::new();
    let mut current: Option<WorktreeEntry> = None;

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            // Start a new entry
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(WorktreeEntry {
                path: PathBuf::from(path),
                head: String::new(),
                branch: None,
                detached: false,
                locked: false,
                prunable: false,
            });
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            if let Some(ref mut entry) = current {
                entry.head = head.to_string();
            }
        } else if let Some(branch) = line.strip_prefix("branch ") {
            if let Some(ref mut entry) = current {
                entry.branch = Some(branch.to_string());
            }
        } else if line == "detached" {
            if let Some(ref mut entry) = current {
                entry.detached = true;
            }
        } else if line == "locked" || line.starts_with("locked ") {
            if let Some(ref mut entry) = current {
                entry.locked = true;
            }
        } else if line == "prunable" || line.starts_with("prunable ") {
            if let Some(ref mut entry) = current {
                entry.prunable = true;
            }
        }
    }

    // Don't forget the last entry
    if let Some(entry) = current {
        entries.push(entry);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_porcelain_single_main() {
        let output = r#"worktree /home/user/repo
HEAD abc123def456
branch refs/heads/main
"#;

        let entries = parse_porcelain_output(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, PathBuf::from("/home/user/repo"));
        assert_eq!(entries[0].head, "abc123def456");
        assert_eq!(entries[0].branch, Some("refs/heads/main".to_string()));
        assert!(!entries[0].detached);
        assert!(!entries[0].locked);
    }

    #[test]
    fn test_parse_porcelain_with_detached() {
        let output = r#"worktree /home/user/repo
HEAD abc123def456
branch refs/heads/main

worktree /home/user/repo/.grit/worktrees/s-20250117-a2f9
HEAD def456abc123
detached
"#;

        let entries = parse_porcelain_output(output).unwrap();
        assert_eq!(entries.len(), 2);

        assert_eq!(entries[0].path, PathBuf::from("/home/user/repo"));
        assert!(!entries[0].detached);

        assert_eq!(
            entries[1].path,
            PathBuf::from("/home/user/repo/.grit/worktrees/s-20250117-a2f9")
        );
        assert!(entries[1].detached);
        assert!(entries[1].branch.is_none());
    }

    #[test]
    fn test_parse_porcelain_with_locked() {
        let output = r#"worktree /home/user/repo/.grit/worktrees/test
HEAD abc123
detached
locked
"#;

        let entries = parse_porcelain_output(output).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].locked);
    }

    #[test]
    fn test_parse_porcelain_empty() {
        let output = "";
        let entries = parse_porcelain_output(output).unwrap();
        assert!(entries.is_empty());
    }
}
