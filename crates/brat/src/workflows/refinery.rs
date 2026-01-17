//! Refinery workflow implementation.
//!
//! The Refinery role manages the merge queue for completed tasks.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use libbrat_config::BratConfig;
use libbrat_grit::{GritClient, Task, TaskStatus};
use serde::Serialize;

use super::error::WorkflowError;
use super::locks::LockHelper;

/// Merge status labels.
pub mod merge_labels {
    pub const QUEUED: &str = "merge:queued";
    pub const RUNNING: &str = "merge:running";
    pub const SUCCEEDED: &str = "merge:succeeded";
    pub const FAILED: &str = "merge:failed";

    pub fn all() -> &'static [&'static str] {
        &[QUEUED, RUNNING, SUCCEEDED, FAILED]
    }
}

/// Status of a CI/CD check.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckStatus {
    /// Check passed successfully.
    Success,
    /// Check is still running or queued.
    Pending,
    /// Check failed.
    Failure,
    /// Check not found (not configured or not in GitHub repo).
    NotFound,
}

/// Configuration for the Refinery workflow.
#[derive(Debug, Clone)]
pub struct RefineryConfig {
    /// Maximum concurrent merge attempts.
    pub max_parallel_merges: u32,
    /// Merge strategy: "rebase", "merge", or "squash".
    pub rebase_strategy: String,
    /// Required status checks before merge.
    pub required_checks: Vec<String>,
    /// Maximum merge retry attempts.
    pub merge_retry_limit: u32,
    /// Lock policy string ("off", "warn", "require").
    pub lock_policy: String,
}

impl RefineryConfig {
    /// Create config from BratConfig.
    pub fn from_brat_config(config: &BratConfig) -> Self {
        Self {
            max_parallel_merges: config.refinery.max_parallel_merges,
            rebase_strategy: config.refinery.rebase_strategy.clone(),
            required_checks: config.refinery.required_checks.clone(),
            merge_retry_limit: config.refinery.merge_retry_limit,
            lock_policy: config.locks.policy.clone(),
        }
    }
}

/// Result of a single refinery control loop iteration.
#[derive(Debug, Default, Serialize)]
pub struct RefineryLoopResult {
    /// Number of tasks in merge queue.
    pub queued: usize,
    /// Number of merge attempts this iteration.
    pub attempted: usize,
    /// Number of successful merges.
    pub succeeded: usize,
    /// Number of failed merges.
    pub failed: usize,
    /// Errors encountered during this iteration.
    pub errors: Vec<String>,
}

/// The Refinery workflow controller.
///
/// Manages the merge queue for completed tasks.
pub struct RefineryWorkflow {
    /// Configuration.
    config: RefineryConfig,
    /// Grit client for task/session queries.
    grit: Arc<GritClient>,
    /// Repository root path.
    repo_root: PathBuf,
    /// Track merge attempts by task_id.
    merge_attempts: HashMap<String, u32>,
    /// Currently merging task IDs.
    merging: Vec<String>,
    /// Lock helper for policy-aware lock management.
    lock_helper: LockHelper,
}

impl RefineryWorkflow {
    /// Create a new RefineryWorkflow.
    pub fn new(config: RefineryConfig, grit: GritClient, repo_root: PathBuf) -> Self {
        let grit = Arc::new(grit);
        let lock_helper = LockHelper::from_config(Arc::clone(&grit), &config.lock_policy);

        Self {
            config,
            grit,
            repo_root,
            merge_attempts: HashMap::new(),
            merging: Vec::new(),
            lock_helper,
        }
    }

    /// Run a single iteration of the refinery control loop.
    pub async fn run_once(&mut self) -> Result<RefineryLoopResult, WorkflowError> {
        let mut result = RefineryLoopResult::default();

        // Step 1: Query merge queue
        let queued_tasks = self.query_merge_queue()?;
        result.queued = queued_tasks.len();

        if queued_tasks.is_empty() {
            return Ok(result);
        }

        // Step 2: Calculate merge budget
        let merge_budget = self
            .config
            .max_parallel_merges
            .saturating_sub(self.merging.len() as u32);

        if merge_budget == 0 {
            return Ok(result);
        }

        // Step 3: Attempt merges for queued tasks
        for task in queued_tasks.iter().take(merge_budget as usize) {
            // Skip if already merging
            if self.merging.contains(&task.task_id) {
                continue;
            }

            // Check retry limit
            let attempts = self.merge_attempts.get(&task.task_id).copied().unwrap_or(0);
            if attempts >= self.config.merge_retry_limit {
                result.errors.push(format!(
                    "Task {} exceeded merge retry limit ({})",
                    task.task_id, self.config.merge_retry_limit
                ));
                continue;
            }

            result.attempted += 1;
            self.merge_attempts
                .insert(task.task_id.clone(), attempts + 1);
            self.merging.push(task.task_id.clone());

            match self.attempt_merge(task).await {
                Ok(()) => {
                    result.succeeded += 1;
                    self.merging.retain(|id| id != &task.task_id);
                    self.merge_attempts.remove(&task.task_id);
                }
                Err(e) => {
                    result.failed += 1;
                    result.errors.push(format!(
                        "Merge failed for {}: {}",
                        task.task_id, e
                    ));
                    self.merging.retain(|id| id != &task.task_id);
                }
            }
        }

        Ok(result)
    }

    /// Query tasks with merge:queued label.
    fn query_merge_queue(&self) -> Result<Vec<Task>, WorkflowError> {
        // Query tasks that are needs-review status with merge:queued label
        let tasks = self.grit.task_list(None)?;

        Ok(tasks
            .into_iter()
            .filter(|task| task.status == TaskStatus::NeedsReview)
            .collect())
    }

    /// Attempt to merge a task's branch.
    async fn attempt_merge(&self, task: &Task) -> Result<(), WorkflowError> {
        // Set merge:running label
        self.set_merge_label(&task.grit_issue_id, merge_labels::RUNNING)?;

        // Post merge attempt comment
        let attempt = self.merge_attempts.get(&task.task_id).copied().unwrap_or(1);
        let comment = format!(
            "[merge]\nattempt = {}\nstrategy = \"{}\"\nresult = \"running\"\n[/merge]",
            attempt, self.config.rebase_strategy
        );
        self.grit.issue_comment(&task.grit_issue_id, &comment)?;

        // Check required checks
        match self.check_required_checks(task) {
            Ok(true) => {
                // All checks passed - proceed with merge
            }
            Ok(false) => {
                // Checks pending - skip this task for now, will retry next cycle
                // Remove running label since we're not actually merging yet
                self.set_merge_label(&task.grit_issue_id, merge_labels::QUEUED)?;
                return Ok(());
            }
            Err(e) => {
                // Check failed - mark as failed
                self.set_merge_label(&task.grit_issue_id, merge_labels::FAILED)?;
                self.grit.issue_comment(
                    &task.grit_issue_id,
                    &format!("Merge blocked: {}", e),
                )?;
                return Err(e);
            }
        }

        // Acquire repo-wide lock before git operations (TTL = 10 minutes)
        let lock_resource = "repo:global".to_string();
        let ttl_ms = 10 * 60 * 1000;
        let acquired_locks = match self.lock_helper.acquire_locks(&[lock_resource.clone()], ttl_ms) {
            Ok(locks) => locks,
            Err(e) => {
                // Lock acquisition failed - skip this task for now, will retry next cycle
                self.set_merge_label(&task.grit_issue_id, merge_labels::QUEUED)?;
                self.grit.issue_comment(
                    &task.grit_issue_id,
                    &format!("Merge deferred: {}", e),
                )?;
                return Err(e);
            }
        };

        // Get task branch name (convention: task-{task_id})
        let branch = format!("task-{}", task.task_id);

        // Execute merge based on strategy
        let merge_result = match self.config.rebase_strategy.as_str() {
            "rebase" => self.git_rebase_merge(&branch).await,
            "merge" => self.git_merge(&branch).await,
            "squash" => self.git_squash_merge(&branch).await,
            _ => self.git_rebase_merge(&branch).await, // Default to rebase
        };

        // Release repo lock (always, regardless of success/failure)
        self.lock_helper.release_locks(&acquired_locks);

        match merge_result {
            Ok(commit_sha) => {
                // Success: update labels and status
                self.set_merge_label(&task.grit_issue_id, merge_labels::SUCCEEDED)?;

                // Post success comment
                let comment = format!(
                    "[merge]\nattempt = {}\nstrategy = \"{}\"\nresult = \"succeeded\"\nmerge_commit = \"{}\"\n[/merge]",
                    attempt, self.config.rebase_strategy, commit_sha
                );
                self.grit.issue_comment(&task.grit_issue_id, &comment)?;

                // Update task status to Merged
                self.grit.task_update_status(&task.task_id, TaskStatus::Merged)?;

                Ok(())
            }
            Err(e) => {
                // Failure: update labels
                self.set_merge_label(&task.grit_issue_id, merge_labels::FAILED)?;

                // Post failure comment
                let comment = format!(
                    "[merge]\nattempt = {}\nstrategy = \"{}\"\nresult = \"failed\"\nerror = \"{}\"\n[/merge]",
                    attempt, self.config.rebase_strategy, e
                );
                self.grit.issue_comment(&task.grit_issue_id, &comment)?;

                Err(e)
            }
        }
    }

    /// Check if required status checks are passing.
    ///
    /// Returns:
    /// - `Ok(true)` if all checks passed
    /// - `Ok(false)` if checks are pending or not found (will retry)
    /// - `Err(...)` if a check explicitly failed
    fn check_required_checks(&self, task: &Task) -> Result<bool, WorkflowError> {
        // Skip check verification if no checks configured
        if self.config.required_checks.is_empty() {
            return Ok(true);
        }

        // Get branch head commit
        let branch = format!("task-{}", task.task_id);
        let commit_sha = match self.run_git(&["rev-parse", &branch]) {
            Ok(sha) => sha,
            Err(_) => {
                // Branch doesn't exist yet - treat as pending
                return Ok(false);
            }
        };

        // Check each required check
        for check_name in &self.config.required_checks {
            let status = self.query_check_status(&commit_sha, check_name)?;

            match status {
                CheckStatus::Success => continue,
                CheckStatus::Pending => {
                    // Check still running - don't merge yet
                    return Ok(false);
                }
                CheckStatus::Failure => {
                    // Check failed - block merge
                    return Err(WorkflowError::GitFailed(format!(
                        "Required check '{}' failed",
                        check_name
                    )));
                }
                CheckStatus::NotFound => {
                    // Check not found - treat as pending for MVP
                    // This handles non-GitHub repos and missing checks gracefully
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Query the status of a specific check for a commit.
    ///
    /// Uses the `gh` CLI to query GitHub check runs. If `gh` is not available
    /// or the repo is not a GitHub repo, returns `NotFound`.
    fn query_check_status(
        &self,
        commit_sha: &str,
        check_name: &str,
    ) -> Result<CheckStatus, WorkflowError> {
        // Use gh CLI to query check runs
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/:owner/:repo/commits/{}/check-runs", commit_sha),
                "--jq",
                &format!(
                    ".check_runs[] | select(.name == \"{}\") | .conclusion // .status",
                    check_name
                ),
            ])
            .current_dir(&self.repo_root)
            .output();

        let output = match output {
            Ok(o) => o,
            Err(_) => {
                // gh CLI not available
                return Ok(CheckStatus::NotFound);
            }
        };

        if !output.status.success() {
            // gh command failed - might not be in a GitHub repo or not authenticated
            return Ok(CheckStatus::NotFound);
        }

        let status = String::from_utf8_lossy(&output.stdout).trim().to_string();

        match status.as_str() {
            "success" => Ok(CheckStatus::Success),
            "in_progress" | "queued" | "pending" => Ok(CheckStatus::Pending),
            "failure" | "cancelled" | "timed_out" | "action_required" => Ok(CheckStatus::Failure),
            "" => Ok(CheckStatus::NotFound),
            _ => Ok(CheckStatus::Pending), // Unknown status treated as pending
        }
    }

    /// Perform a rebase merge.
    async fn git_rebase_merge(&self, branch: &str) -> Result<String, WorkflowError> {
        // Checkout main, rebase branch, fast-forward merge
        self.run_git(&["checkout", "main"])?;
        self.run_git(&["pull", "--rebase", "origin", "main"])?;

        // Rebase the branch onto main
        let rebase_result = self.run_git(&["rebase", "main", branch]);
        if rebase_result.is_err() {
            // Abort rebase on conflict
            let _ = self.run_git(&["rebase", "--abort"]);
            return Err(WorkflowError::GitFailed(
                "rebase conflict".to_string(),
            ));
        }

        // Fast-forward merge
        self.run_git(&["checkout", "main"])?;
        self.run_git(&["merge", "--ff-only", branch])?;

        // Get merge commit SHA
        let sha = self.get_head_sha()?;

        // Push to origin
        self.run_git(&["push", "origin", "main"])?;

        Ok(sha)
    }

    /// Perform a regular merge.
    async fn git_merge(&self, branch: &str) -> Result<String, WorkflowError> {
        self.run_git(&["checkout", "main"])?;
        self.run_git(&["pull", "--rebase", "origin", "main"])?;

        // Merge with merge commit
        let message = format!("Merge branch '{}'", branch);
        self.run_git(&["merge", "--no-ff", "-m", &message, branch])?;

        let sha = self.get_head_sha()?;
        self.run_git(&["push", "origin", "main"])?;

        Ok(sha)
    }

    /// Perform a squash merge.
    async fn git_squash_merge(&self, branch: &str) -> Result<String, WorkflowError> {
        self.run_git(&["checkout", "main"])?;
        self.run_git(&["pull", "--rebase", "origin", "main"])?;

        // Squash merge
        self.run_git(&["merge", "--squash", branch])?;

        // Commit the squash
        let message = format!("Squash merge branch '{}'", branch);
        self.run_git(&["commit", "-m", &message])?;

        let sha = self.get_head_sha()?;
        self.run_git(&["push", "origin", "main"])?;

        Ok(sha)
    }

    /// Run a git command.
    fn run_git(&self, args: &[&str]) -> Result<String, WorkflowError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| WorkflowError::GitFailed(format!("failed to run git: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorkflowError::GitFailed(stderr.to_string()));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get the current HEAD SHA.
    fn get_head_sha(&self) -> Result<String, WorkflowError> {
        self.run_git(&["rev-parse", "HEAD"])
    }

    /// Set merge label on a task issue.
    fn set_merge_label(&self, issue_id: &str, label: &str) -> Result<(), WorkflowError> {
        // Remove all merge labels first
        for old_label in merge_labels::all() {
            let _ = self.grit.issue_label_remove(issue_id, &[old_label]);
        }

        // Add new label
        self.grit.issue_label_add(issue_id, &[label])?;
        Ok(())
    }

    /// Get number of currently merging tasks.
    pub fn active_merges(&self) -> usize {
        self.merging.len()
    }

    /// Graceful shutdown.
    pub async fn shutdown(&self) -> Result<(), WorkflowError> {
        // Nothing to shutdown for refinery
        Ok(())
    }
}
