//! Witness workflow implementation.
//!
//! The Witness role spawns and manages polecat sessions for queued tasks.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use libbrat_config::BratConfig;
use libbrat_engine::{Engine, SpawnSpec};
use libbrat_grit::{GritClient, SessionRole, SessionStatus, SessionType, Task, TaskStatus};
use libbrat_session::{MonitorConfig, MonitorHandle, SessionMonitor};
use libbrat_worktree::WorktreeManager;
use serde::Serialize;

use super::error::WorkflowError;
use super::locks::LockHelper;

/// Configuration for the Witness workflow.
#[derive(Debug, Clone)]
pub struct WitnessConfig {
    /// Maximum concurrent polecat sessions.
    pub max_polecats: u32,
    /// Engine command to spawn.
    pub engine_command: String,
    /// Arguments to pass to engine.
    pub engine_args: Vec<String>,
    /// Monitor configuration.
    pub monitor_config: MonitorConfig,
    /// Lock policy string ("off", "warn", "require").
    pub lock_policy: String,
    /// Session timeout in minutes (for lock TTL calculation).
    pub session_timeout_minutes: u32,
}

impl WitnessConfig {
    /// Create config from BratConfig.
    pub fn from_brat_config(config: &BratConfig) -> Self {
        Self {
            max_polecats: config.swarm.max_polecats,
            engine_command: config.swarm.engine.clone(),
            engine_args: Vec::new(),
            monitor_config: MonitorConfig::default()
                .heartbeat_interval(Duration::from_millis(
                    config.interventions.heartbeat_interval_ms,
                )),
            lock_policy: config.locks.policy.clone(),
            session_timeout_minutes: 60, // Default 1 hour
        }
    }
}

/// Result of a single witness control loop iteration.
#[derive(Debug, Default, Serialize)]
pub struct WitnessLoopResult {
    /// Number of tasks found (queued + running).
    pub tasks_found: usize,
    /// Number of sessions already active.
    pub sessions_active: usize,
    /// Number of sessions spawned this iteration.
    pub sessions_spawned: usize,
    /// Errors encountered during this iteration.
    pub errors: Vec<String>,
}

/// The Witness workflow controller.
///
/// Manages polecat sessions for queued tasks.
pub struct WitnessWorkflow<E: Engine + 'static> {
    /// Configuration.
    config: WitnessConfig,
    /// Grit client for task/session queries.
    grit: Arc<GritClient>,
    /// Session monitor for spawning and tracking sessions.
    monitor: SessionMonitor<E>,
    /// Track active sessions by task_id -> session_id.
    active_sessions: HashMap<String, String>,
    /// Lock helper for policy-aware lock management.
    lock_helper: LockHelper,
    /// Track acquired locks by session_id -> list of lock resources.
    session_locks: HashMap<String, Vec<String>>,
}

impl<E: Engine + 'static> WitnessWorkflow<E> {
    /// Create a new WitnessWorkflow.
    pub fn new(
        config: WitnessConfig,
        grit: GritClient,
        engine: E,
        worktree_manager: Option<WorktreeManager>,
    ) -> Self {
        let grit = Arc::new(grit);
        let engine_name = config.engine_command.clone();
        let monitor = SessionMonitor::new(
            engine,
            engine_name,
            (*grit).clone(),
            worktree_manager,
            config.monitor_config.clone(),
        );
        let lock_helper = LockHelper::from_config(Arc::clone(&grit), &config.lock_policy);

        Self {
            config,
            grit,
            monitor,
            active_sessions: HashMap::new(),
            lock_helper,
            session_locks: HashMap::new(),
        }
    }

    /// Run a single iteration of the witness control loop.
    pub async fn run_once(&mut self) -> Result<WitnessLoopResult, WorkflowError> {
        let mut result = WitnessLoopResult::default();

        // Step 0: Clean up locks for exited sessions
        self.cleanup_exited_session_locks().await;

        // Step 1: Query Grit for actionable tasks
        let tasks = self.query_actionable_tasks()?;
        result.tasks_found = tasks.len();

        // Step 2: Get current active session count
        let active_sessions = self.monitor.list_sessions().await;
        result.sessions_active = active_sessions.len();

        // Step 3: Calculate spawn budget
        let spawn_budget = self
            .config
            .max_polecats
            .saturating_sub(active_sessions.len() as u32);

        if spawn_budget == 0 {
            return Ok(result);
        }

        // Step 4: Spawn sessions for tasks without active sessions
        for task in tasks.iter().take(spawn_budget as usize) {
            // Skip if already has an active session
            if self.has_active_session(&task.task_id).await {
                continue;
            }

            match self.spawn_session_for_task(task).await {
                Ok(session_id) => {
                    result.sessions_spawned += 1;
                    self.active_sessions
                        .insert(task.task_id.clone(), session_id);
                }
                Err(e) => {
                    result.errors.push(format!(
                        "Failed to spawn session for {}: {}",
                        task.task_id, e
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Query tasks with status:queued or status:running.
    fn query_actionable_tasks(&self) -> Result<Vec<Task>, WorkflowError> {
        let mut tasks = Vec::new();

        // Query queued tasks
        if let Ok(queued) = self.grit.task_list(None) {
            for task in queued {
                if task.status == TaskStatus::Queued || task.status == TaskStatus::Running {
                    tasks.push(task);
                }
            }
        }

        Ok(tasks)
    }

    /// Check if a task already has an active session.
    async fn has_active_session(&self, task_id: &str) -> bool {
        // Check in-memory cache first
        if self.active_sessions.contains_key(task_id) {
            return true;
        }

        // Query Grit for active sessions on this task
        if let Ok(sessions) = self.grit.session_list(Some(task_id)) {
            return sessions.iter().any(|s| s.status != SessionStatus::Exit);
        }

        false
    }

    /// Spawn a new polecat session for a task.
    async fn spawn_session_for_task(&mut self, task: &Task) -> Result<String, WorkflowError> {
        // For AI engines, fetch full task to get body (task_list doesn't include body)
        let is_ai_engine = matches!(self.config.engine_command.as_str(), "codex" | "claude");
        let full_task = if is_ai_engine && task.body.is_empty() {
            self.grit.task_get(&task.task_id)?
        } else {
            task.clone()
        };

        // Parse paths from task body for lock acquisition
        let paths = full_task.parse_paths();
        let lock_resources: Vec<String> = paths
            .iter()
            .map(|p| format!("path:{}", p))
            .collect();

        // Acquire locks for task paths (TTL = session timeout + 5 min buffer)
        let ttl_ms = (self.config.session_timeout_minutes as i64 + 5) * 60 * 1000;
        let acquired_locks = self.lock_helper.acquire_locks(&lock_resources, ttl_ms)?;

        // Build spawn spec - for AI engines (codex, claude), use task body as prompt
        let command = if is_ai_engine {
            // Construct prompt from task title and body
            format!(
                "Task: {}\n\n{}",
                full_task.title,
                &full_task.body
            )
        } else {
            self.config.engine_command.clone()
        };

        let spec = SpawnSpec::new(command)
            .args(self.config.engine_args.clone())
            .arg("--task")
            .arg(&task.task_id);

        // Spawn via SessionMonitor (handles worktree, Grit record, etc.)
        let handle = match self
            .monitor
            .spawn_session(
                &task.task_id,
                SessionRole::Witness,
                SessionType::Polecat,
                spec,
            )
            .await
        {
            Ok(h) => h,
            Err(e) => {
                // Release acquired locks on spawn failure
                self.lock_helper.release_locks(&acquired_locks);
                return Err(e.into());
            }
        };

        let session_id = handle.session_id().to_string();

        // Store acquired locks for later release
        if !acquired_locks.is_empty() {
            self.session_locks.insert(session_id.clone(), acquired_locks.clone());
        }

        // Post spawn comment (include lock info if any)
        let lock_info = if acquired_locks.is_empty() {
            String::new()
        } else {
            format!(" Acquired locks: {}", acquired_locks.join(", "))
        };
        let comment = format!(
            "Witness spawned polecat session `{}` for this task.{}",
            session_id, lock_info
        );
        self.grit.issue_comment(&task.grit_issue_id, &comment)?;

        // Update task status to Running if it was Queued
        if task.status == TaskStatus::Queued {
            self.grit
                .task_update_status(&task.task_id, TaskStatus::Running)?;
        }

        Ok(session_id)
    }

    /// Clean up locks for sessions that have exited.
    async fn cleanup_exited_session_locks(&mut self) {
        // Get list of session IDs we're tracking
        let tracked_sessions: Vec<String> = self.session_locks.keys().cloned().collect();

        for session_id in tracked_sessions {
            // Check if session still exists and is active
            let is_active = match self.grit.session_get(&session_id) {
                Ok(session) => session.status != SessionStatus::Exit,
                Err(_) => false, // Session not found, treat as exited
            };

            if !is_active {
                // Session has exited, release its locks
                if let Some(locks) = self.session_locks.remove(&session_id) {
                    self.lock_helper.release_locks(&locks);
                }
                // Also clean up active_sessions mapping
                self.active_sessions.retain(|_, sid| sid != &session_id);
            }
        }
    }

    /// Get a handle to an active session.
    pub async fn get_session_handle(&self, session_id: &str) -> Option<MonitorHandle> {
        self.monitor.get_handle(session_id).await
    }

    /// Graceful shutdown of all sessions.
    pub async fn shutdown(&self) -> Result<(), WorkflowError> {
        self.monitor.shutdown().await?;
        Ok(())
    }
}
