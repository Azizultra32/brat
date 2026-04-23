use std::path::{Path, PathBuf};
use std::process::Command;

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::error::GriteeError;
use crate::id::{
    generate_convoy_id, generate_session_id, generate_task_id, is_valid_convoy_id,
    is_valid_session_id, is_valid_task_id,
};
use crate::state_machine::StateMachine;
use crate::types::{
    ContextIndexResult, Convoy, ConvoyStatus, DependencyType, FileContext, GriteeIssue,
    GriteeIssueSummary, ProjectContextEntry, Session, SessionRole, SessionStatus, SessionType,
    Symbol, SymbolMatch, Task, TaskDependency, TaskStatus,
};

/// Expected Gritee CLI JSON schema version.
const EXPECTED_GRIT_SCHEMA_VERSION: u32 = 1;

/// JSON envelope from Griteee CLI responses (used by lock commands).
#[derive(Debug, Deserialize)]
struct JsonResponse<T> {
    #[serde(default)]
    schema_version: Option<u32>,
    #[serde(default)]
    #[allow(dead_code)] // Used by Grite but not checked in our code
    ok: bool,
    data: Option<T>,
    error: Option<JsonError>,
}

#[derive(Debug, Deserialize)]
struct JsonError {
    #[serde(default)]
    code: String,
    message: String,
}

/// Convert a byte array to a hex string.
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Issue ID that can be either a hex string or a byte array.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum IssueIdFormat {
    Hex(String),
    Bytes(Vec<u8>),
}

impl IssueIdFormat {
    fn to_hex(&self) -> String {
        match self {
            IssueIdFormat::Hex(s) => s.clone(),
            IssueIdFormat::Bytes(bytes) => bytes_to_hex(bytes),
        }
    }
}

/// Response from issue create command (new format).
#[derive(Debug, Deserialize)]
struct IssueCreateResponse {
    issue_id: IssueIdFormat,
    #[allow(dead_code)]
    event_id: Option<String>,
}

/// Raw issue summary from gritee issue list (new format).
#[derive(Debug, Deserialize)]
struct RawIssueSummary {
    issue_id: IssueIdFormat,
    title: String,
    state: String,
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<String>,
    #[serde(default)]
    updated_ts: i64,
    #[serde(default)]
    comment_count: u32,
}

impl RawIssueSummary {
    fn into_gritee_issue_summary(self) -> GriteeIssueSummary {
        GriteeIssueSummary {
            issue_id: self.issue_id.to_hex(),
            title: self.title,
            state: self.state,
            labels: self.labels,
            updated_ts: self.updated_ts,
        }
    }
}

/// Response from issue list command (new format - no envelope).
#[derive(Debug, Deserialize)]
struct IssueListResponse {
    issues: Vec<RawIssueSummary>,
}

/// Response wrapper for issue show command.
#[derive(Debug, Deserialize)]
struct IssueShowResponse {
    issue: RawIssue,
    #[allow(dead_code)]
    events: Option<Vec<serde_json::Value>>,
}

/// Raw issue from gritee issue show (new format).
#[derive(Debug, Deserialize)]
struct RawIssue {
    issue_id: IssueIdFormat,
    title: String,
    #[serde(default)]
    body: String,
    state: String,
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<String>,
    #[serde(default)]
    comments: Vec<RawComment>,
    #[serde(default)]
    updated_ts: i64,
    #[serde(default)]
    comment_count: u32,
}

/// Raw comment from gritee issue show.
#[derive(Debug, Deserialize)]
struct RawComment {
    #[allow(dead_code)]
    comment_id: Option<IssueIdFormat>,
    body: String,
    #[allow(dead_code)]
    author: Option<String>,
    #[allow(dead_code)]
    created_ts: Option<i64>,
}

impl RawIssue {
    fn into_gritee_issue(self) -> GriteeIssue {
        GriteeIssue {
            issue_id: self.issue_id.to_hex(),
            title: self.title,
            body: self.body,
            comments: self.comments.into_iter().map(|c| c.body).collect(),
            state: self.state,
            labels: self.labels,
            updated_ts: self.updated_ts,
        }
    }
}

/// Response from lock acquire command.
#[derive(Debug, Deserialize)]
struct LockAcquireResponse {
    resource: String,
    owner: String,
    #[serde(default)]
    nonce: Option<String>,
    #[serde(default)]
    expires_unix_ms: Option<i64>,
    #[serde(default)]
    ttl_seconds: Option<i64>,
}

/// Result of a lock acquisition attempt.
#[derive(Debug, Clone)]
pub struct LockResult {
    /// Whether the lock was successfully acquired.
    pub acquired: bool,
    /// The resource that was locked.
    pub resource: String,
    /// The current holder of the lock (if not acquired).
    pub holder: Option<String>,
    /// When the lock expires (Unix timestamp in ms).
    pub expires_unix_ms: Option<i64>,
}

/// Response from gritee dep list command.
#[derive(Debug, Deserialize)]
struct DepListResponse {
    #[allow(dead_code)]
    issue_id: String,
    #[allow(dead_code)]
    direction: String,
    deps: Vec<DepListEntry>,
}

/// Entry in dependency list response.
#[derive(Debug, Deserialize)]
struct DepListEntry {
    issue_id: String,
    dep_type: String,
    title: String,
}

/// Response from gritee dep topo command.
#[derive(Debug, Deserialize)]
struct DepTopoResponse {
    issues: Vec<RawIssueSummary>,
    #[allow(dead_code)]
    order: String,
}

/// Response from gritee context index command.
#[derive(Debug, Deserialize)]
struct ContextIndexResponse {
    indexed: u32,
    skipped: u32,
    total_files: u32,
}

/// Response from gritee context query command.
#[derive(Debug, Deserialize)]
struct ContextQueryResponse {
    #[allow(dead_code)]
    query: String,
    matches: Vec<ContextQueryMatch>,
    #[allow(dead_code)]
    count: u32,
}

/// Match entry from context query.
#[derive(Debug, Deserialize)]
struct ContextQueryMatch {
    symbol: String,
    path: String,
}

/// Response from gritee context show command.
#[derive(Debug, Deserialize)]
struct ContextShowResponse {
    path: String,
    language: String,
    summary: String,
    content_hash: String,
    symbols: Vec<ContextSymbol>,
    #[allow(dead_code)]
    symbol_count: u32,
}

/// Symbol entry from context show.
#[derive(Debug, Deserialize)]
struct ContextSymbol {
    name: String,
    kind: String,
    line_start: u32,
    line_end: u32,
}

/// Response from gritee context project (single key).
#[derive(Debug, Deserialize)]
struct ContextProjectSingleResponse {
    key: String,
    value: String,
}

/// Response from gritee context project (list).
#[derive(Debug, Deserialize)]
struct ContextProjectListResponse {
    entries: Vec<ContextProjectEntry>,
    #[allow(dead_code)]
    count: u32,
}

/// Entry in project context list.
#[derive(Debug, Deserialize)]
struct ContextProjectEntry {
    key: String,
    value: String,
}

/// Client for interacting with the Gritee CLI.
#[derive(Clone)]
pub struct GriteeClient {
    repo_root: PathBuf,
}

impl GriteeClient {
    /// Create a new GriteeClient for the given repository root.
    pub fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
        }
    }

    /// Check if Grite is initialized in the repository.
    pub fn is_initialized(&self, git_dir: &Path) -> bool {
        git_dir.join("grite").exists() || git_dir.join("gritee").exists()
    }

    /// Get the repository root path.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    // -------------------------------------------------------------------------
    // Low-level issue operations
    // -------------------------------------------------------------------------

    /// Create a new issue with the given title, body, and labels.
    pub fn issue_create(
        &self,
        title: &str,
        body: &str,
        labels: &[String],
    ) -> Result<String, GriteeError> {
        let mut args = vec!["issue", "create", "--title", title, "--body", body];
        for label in labels {
            args.push("--label");
            args.push(label);
        }

        let response: IssueCreateResponse = self.run_json_direct(&args)?;
        Ok(response.issue_id.to_hex())
    }

    /// List issues with optional label filters.
    pub fn issue_list(
        &self,
        labels: &[&str],
        state: Option<&str>,
    ) -> Result<Vec<GriteeIssueSummary>, GriteeError> {
        let mut args = vec!["issue", "list"];

        if let Some(state) = state {
            args.push("--state");
            args.push(state);
        }

        for label in labels {
            args.push("--label");
            args.push(label);
        }

        let response: IssueListResponse = self.run_json_direct(&args)?;
        Ok(response
            .issues
            .into_iter()
            .map(|r| r.into_gritee_issue_summary())
            .collect())
    }

    /// Get a single issue by ID.
    pub fn issue_show(&self, issue_id: &str) -> Result<GriteeIssue, GriteeError> {
        let args = vec!["issue", "show", issue_id];
        let response: IssueShowResponse = self.run_json_direct(&args)?;
        let mut issue = response.issue.into_gritee_issue();
        if issue.body.is_empty() {
            if let Some(body) = extract_issue_created_body(response.events.as_deref()) {
                issue.body = body;
            }
        }
        issue
            .comments
            .extend(extract_comment_bodies(response.events.as_deref()));
        Ok(issue)
    }

    /// Add labels to an issue.
    pub fn issue_label_add(&self, issue_id: &str, labels: &[&str]) -> Result<(), GriteeError> {
        for label in labels {
            let args = vec!["issue", "label", "add", "--label", label, issue_id];
            let _: serde_json::Value = self.run_json_direct(&args)?;
        }
        Ok(())
    }

    /// Remove labels from an issue.
    pub fn issue_label_remove(&self, issue_id: &str, labels: &[&str]) -> Result<(), GriteeError> {
        for label in labels {
            let args = vec!["issue", "label", "remove", "--label", label, issue_id];
            // Ignore errors for labels that don't exist
            let _ = self.run_json_direct::<serde_json::Value>(&args);
        }
        Ok(())
    }

    /// Add a comment to an issue.
    pub fn issue_comment(&self, issue_id: &str, body: &str) -> Result<(), GriteeError> {
        let args = vec!["issue", "comment", issue_id, "--body", body];
        let _: serde_json::Value = self.run_json_direct(&args)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Convoy operations
    // -------------------------------------------------------------------------

    /// Create a new convoy.
    pub fn convoy_create(&self, title: &str, body: Option<&str>) -> Result<Convoy, GriteeError> {
        let convoy_id = generate_convoy_id();
        let labels = vec![
            "type:convoy".to_string(),
            format!("convoy:{}", convoy_id),
            ConvoyStatus::Active.as_label().to_string(),
        ];

        let gritee_issue_id = self.issue_create(title, body.unwrap_or(""), &labels)?;

        Ok(Convoy {
            convoy_id,
            gritee_issue_id,
            title: title.to_string(),
            body: body.unwrap_or("").to_string(),
            status: ConvoyStatus::Active,
        })
    }

    /// List all convoys.
    pub fn convoy_list(&self) -> Result<Vec<Convoy>, GriteeError> {
        let issues = self.issue_list(&["type:convoy"], Some("open"))?;
        issues
            .into_iter()
            .filter_map(|issue| parse_convoy_from_summary(&issue).ok())
            .collect::<Vec<_>>()
            .into_iter()
            .map(Ok)
            .collect()
    }

    /// Get a convoy by its Brat convoy ID.
    pub fn convoy_get(&self, convoy_id: &str) -> Result<Convoy, GriteeError> {
        if !is_valid_convoy_id(convoy_id) {
            return Err(GriteeError::InvalidId(convoy_id.to_string()));
        }

        let label = format!("convoy:{}", convoy_id);
        let issues = self.issue_list(&[&label], None)?;

        issues
            .into_iter()
            .find(|issue| issue.labels.contains(&"type:convoy".to_string()))
            .map(|issue| parse_convoy_from_summary(&issue))
            .transpose()?
            .ok_or_else(|| GriteeError::NotFound(format!("convoy {}", convoy_id)))
    }

    // -------------------------------------------------------------------------
    // Task operations
    // -------------------------------------------------------------------------

    /// Create a new task linked to a convoy.
    pub fn task_create(
        &self,
        convoy_id: &str,
        title: &str,
        body: Option<&str>,
    ) -> Result<Task, GriteeError> {
        if !is_valid_convoy_id(convoy_id) {
            return Err(GriteeError::InvalidId(convoy_id.to_string()));
        }

        // Verify convoy exists
        let _ = self.convoy_get(convoy_id)?;

        let task_id = generate_task_id();
        let labels = vec![
            "type:task".to_string(),
            format!("task:{}", task_id),
            format!("convoy:{}", convoy_id),
            TaskStatus::Queued.as_label().to_string(),
        ];

        let gritee_issue_id = self.issue_create(title, body.unwrap_or(""), &labels)?;

        Ok(Task {
            task_id,
            gritee_issue_id,
            convoy_id: convoy_id.to_string(),
            title: title.to_string(),
            body: body.unwrap_or("").to_string(),
            status: TaskStatus::Queued,
        })
    }

    /// List tasks, optionally filtered by convoy.
    pub fn task_list(&self, convoy_id: Option<&str>) -> Result<Vec<Task>, GriteeError> {
        let mut labels: Vec<&str> = vec!["type:task"];

        let convoy_label;
        if let Some(cid) = convoy_id {
            if !is_valid_convoy_id(cid) {
                return Err(GriteeError::InvalidId(cid.to_string()));
            }
            convoy_label = format!("convoy:{}", cid);
            labels.push(&convoy_label);
        }

        let issues = self.issue_list(&labels, Some("open"))?;
        issues
            .into_iter()
            .filter_map(|issue| parse_task_from_summary(&issue).ok())
            .collect::<Vec<_>>()
            .into_iter()
            .map(Ok)
            .collect()
    }

    /// Get a task by its Brat task ID.
    ///
    /// Unlike `task_list`, this returns the full task including body.
    pub fn task_get(&self, task_id: &str) -> Result<Task, GriteeError> {
        if !is_valid_task_id(task_id) {
            return Err(GriteeError::InvalidId(task_id.to_string()));
        }

        let label = format!("task:{}", task_id);
        let issues = self.issue_list(&[&label], None)?;

        // Find the task issue
        let summary = issues
            .into_iter()
            .find(|issue| issue.labels.contains(&"type:task".to_string()))
            .ok_or_else(|| GriteeError::NotFound(format!("task {}", task_id)))?;

        // Fetch full issue to get body
        let full_issue = self.issue_show(&summary.issue_id)?;

        parse_task_from_full_issue(&summary, &full_issue)
    }

    /// Update task status with validation.
    ///
    /// This validates the state transition before updating. Use
    /// `task_update_status_with_options` with `force=true` to bypass validation.
    pub fn task_update_status(
        &self,
        task_id: &str,
        new_status: TaskStatus,
    ) -> Result<(), GriteeError> {
        self.task_update_status_with_options(task_id, new_status, false)
    }

    /// Update task status with options.
    ///
    /// If `force` is true, bypasses state machine validation (admin override).
    pub fn task_update_status_with_options(
        &self,
        task_id: &str,
        new_status: TaskStatus,
        force: bool,
    ) -> Result<(), GriteeError> {
        let task = self.task_get(task_id)?;

        // Validate state transition
        let state_machine = StateMachine::<TaskStatus>::new();
        state_machine
            .validate(task.status, new_status, force)
            .map_err(|e| GriteeError::InvalidStateTransition(e.to_string()))?;

        // Remove old status labels
        let old_labels: Vec<&str> = TaskStatus::all_labels().to_vec();
        self.issue_label_remove(&task.gritee_issue_id, &old_labels)?;

        // Add new status label
        self.issue_label_add(&task.gritee_issue_id, &[new_status.as_label()])?;

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Session operations
    // -------------------------------------------------------------------------

    /// Create a new session for a task.
    ///
    /// This posts a session comment to the task issue and updates labels.
    /// Sessions are stored as comments on task issues, not as separate issues.
    pub fn session_create(
        &self,
        task_id: &str,
        role: SessionRole,
        session_type: SessionType,
        engine: &str,
        worktree: &str,
        pid: Option<u32>,
    ) -> Result<Session, GriteeError> {
        self.session_create_with_id(None, task_id, role, session_type, engine, worktree, pid)
    }

    /// Create a new session with an optional pre-generated ID.
    ///
    /// This is useful when the session ID needs to be known before creation,
    /// such as when creating a worktree that uses the session ID as its name.
    ///
    /// If `session_id` is None, a new ID will be generated.
    pub fn session_create_with_id(
        &self,
        session_id: Option<&str>,
        task_id: &str,
        role: SessionRole,
        session_type: SessionType,
        engine: &str,
        worktree: &str,
        pid: Option<u32>,
    ) -> Result<Session, GriteeError> {
        // Validate task exists and get gritee_issue_id
        let task = self.task_get(task_id)?;

        // Generate session ID if not provided
        let session_id = session_id
            .map(|s| s.to_string())
            .unwrap_or_else(generate_session_id);
        let started_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        // Build session struct
        let session = Session {
            session_id: session_id.clone(),
            task_id: task_id.to_string(),
            gritee_issue_id: task.gritee_issue_id.clone(),
            role,
            session_type,
            engine: engine.to_string(),
            worktree: worktree.to_string(),
            pid,
            status: SessionStatus::Spawned,
            started_ts,
            last_heartbeat_ts: None,
            exit_code: None,
            exit_reason: None,
            last_output_ref: None,
        };

        // Post session comment
        let comment_body = format_session_comment(&session);
        self.issue_comment(&task.gritee_issue_id, &comment_body)?;

        // Update task labels with session info
        self.issue_label_add(
            &task.gritee_issue_id,
            &[
                SessionStatus::Spawned.as_label(),
                session_type.as_label(),
                &format!("engine:{}", engine),
            ],
        )?;

        Ok(session)
    }

    /// List active sessions, optionally filtered by task.
    ///
    /// Only returns sessions that are not in the Exit state.
    pub fn session_list(&self, task_id: Option<&str>) -> Result<Vec<Session>, GriteeError> {
        // Build label filter
        let mut labels: Vec<&str> = vec!["type:task"];
        let task_label;
        if let Some(tid) = task_id {
            if !is_valid_task_id(tid) {
                return Err(GriteeError::InvalidId(tid.to_string()));
            }
            task_label = format!("task:{}", tid);
            labels.push(&task_label);
        }

        // Get tasks with active session labels (not exit)
        let issues = self.issue_list(&labels, Some("open"))?;

        let mut sessions = Vec::new();
        for issue in issues {
            // Check for active session labels
            let has_active_session = issue.labels.iter().any(|l| {
                matches!(
                    l.as_str(),
                    "session:spawned" | "session:ready" | "session:running" | "session:handoff"
                )
            });

            if has_active_session {
                // Fetch full issue to get comments
                if let Ok(full_issue) = self.issue_show(&issue.issue_id) {
                    if let Some(session) = parse_latest_session_from_issue(&full_issue, &issue) {
                        sessions.push(session);
                    }
                }
            }
        }

        Ok(sessions)
    }

    /// Get a session by its Brat session ID.
    pub fn session_get(&self, session_id: &str) -> Result<Session, GriteeError> {
        if !is_valid_session_id(session_id) {
            return Err(GriteeError::InvalidId(session_id.to_string()));
        }

        // Search all task issues for this session
        // TODO: Optimize with session index or session: label
        let tasks = self.issue_list(&["type:task"], None)?;

        for task_summary in tasks {
            if let Ok(issue) = self.issue_show(&task_summary.issue_id) {
                if let Some(session) =
                    parse_session_by_id_from_issue(&issue, &task_summary, session_id)
                {
                    return Ok(session);
                }
            }
        }

        Err(GriteeError::NotFound(format!("session {}", session_id)))
    }

    /// Update session status with validation.
    pub fn session_update_status(
        &self,
        session_id: &str,
        new_status: SessionStatus,
    ) -> Result<(), GriteeError> {
        self.session_update_status_with_options(session_id, new_status, false)
    }

    /// Update session status with options.
    ///
    /// If `force` is true, bypasses state machine validation.
    pub fn session_update_status_with_options(
        &self,
        session_id: &str,
        new_status: SessionStatus,
        force: bool,
    ) -> Result<(), GriteeError> {
        let session = self.session_get(session_id)?;

        // Validate state transition
        let state_machine = StateMachine::<SessionStatus>::new();
        state_machine
            .validate(session.status, new_status, force)
            .map_err(|e| GriteeError::InvalidStateTransition(e.to_string()))?;

        // Update session with new state
        let mut updated_session = session.clone();
        updated_session.status = new_status;
        updated_session.last_heartbeat_ts = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        );

        let comment_body = format_session_comment(&updated_session);
        self.issue_comment(&session.gritee_issue_id, &comment_body)?;

        // Update labels: remove old session status, add new
        let old_labels: Vec<&str> = SessionStatus::all_labels().to_vec();
        self.issue_label_remove(&session.gritee_issue_id, &old_labels)?;
        self.issue_label_add(&session.gritee_issue_id, &[new_status.as_label()])?;

        Ok(())
    }

    /// Record session heartbeat.
    ///
    /// Posts a new session comment with updated heartbeat timestamp.
    pub fn session_heartbeat(&self, session_id: &str) -> Result<(), GriteeError> {
        let session = self.session_get(session_id)?;

        // Update session with new heartbeat
        let mut updated_session = session.clone();
        updated_session.last_heartbeat_ts = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        );

        let comment_body = format_session_comment(&updated_session);
        self.issue_comment(&session.gritee_issue_id, &comment_body)?;

        Ok(())
    }

    /// Record session exit.
    ///
    /// Posts a final session comment with exit information and updates labels.
    pub fn session_exit(
        &self,
        session_id: &str,
        exit_code: i32,
        exit_reason: &str,
        last_output_ref: Option<&str>,
    ) -> Result<(), GriteeError> {
        let session = self.session_get(session_id)?;

        // Build exit comment
        let mut updated_session = session.clone();
        updated_session.status = SessionStatus::Exit;
        updated_session.exit_code = Some(exit_code);
        updated_session.exit_reason = Some(exit_reason.to_string());
        updated_session.last_output_ref = last_output_ref.map(|s| s.to_string());
        updated_session.last_heartbeat_ts = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        );

        let comment_body = format_session_comment(&updated_session);
        self.issue_comment(&session.gritee_issue_id, &comment_body)?;

        // Update labels
        let old_labels: Vec<&str> = SessionStatus::all_labels().to_vec();
        self.issue_label_remove(&session.gritee_issue_id, &old_labels)?;
        self.issue_label_add(&session.gritee_issue_id, &[SessionStatus::Exit.as_label()])?;

        Ok(())
    }

    // -------------------------------------------------------------------------
    // Lock operations
    // -------------------------------------------------------------------------

    /// Acquire a lock on a resource.
    ///
    /// Returns a `LockResult` indicating whether the lock was acquired.
    /// If the lock is already held by another actor, `acquired` will be false
    /// and `holder` will contain the current owner.
    pub fn lock_acquire(&self, resource: &str, ttl_ms: i64) -> Result<LockResult, GriteeError> {
        // Convert milliseconds to seconds (grite now uses --ttl in seconds)
        let ttl_seconds = (ttl_ms / 1000).max(1);
        let ttl_str = ttl_seconds.to_string();
        let args = vec!["lock", "acquire", resource, "--ttl", &ttl_str];

        let response: LockAcquireResponse = self.run_json(&args)?;

        Ok(LockResult {
            acquired: true, // If we get a response without error, the lock was acquired
            resource: response.resource,
            holder: Some(response.owner),
            expires_unix_ms: response.expires_unix_ms,
        })
    }

    /// Release a lock on a resource.
    ///
    /// This is a best-effort operation; errors are logged but not fatal.
    pub fn lock_release(&self, resource: &str) -> Result<(), GriteeError> {
        let args = vec!["lock", "release", resource];
        let _: serde_json::Value = self.run_json(&args)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Dependency operations (gritee issue dep)
    // -------------------------------------------------------------------------

    /// Add a dependency between two tasks.
    ///
    /// The dependency is from `task_issue_id` to `target_issue_id` with the given type.
    /// For example, `DependencyType::DependsOn` means the task depends on the target.
    pub fn task_dep_add(
        &self,
        task_issue_id: &str,
        target_issue_id: &str,
        dep_type: DependencyType,
    ) -> Result<(), GriteeError> {
        let args = vec![
            "issue",
            "dep",
            "add",
            task_issue_id,
            "--target",
            target_issue_id,
            "--type",
            dep_type.as_str(),
        ];
        let _: serde_json::Value = self.run_json_direct(&args)?;
        Ok(())
    }

    /// Remove a dependency between two tasks.
    pub fn task_dep_remove(
        &self,
        task_issue_id: &str,
        target_issue_id: &str,
        dep_type: DependencyType,
    ) -> Result<(), GriteeError> {
        let args = vec![
            "issue",
            "dep",
            "remove",
            task_issue_id,
            "--target",
            target_issue_id,
            "--type",
            dep_type.as_str(),
        ];
        let _: serde_json::Value = self.run_json_direct(&args)?;
        Ok(())
    }

    /// List dependencies for a task.
    ///
    /// If `reverse` is true, returns issues that depend on this task.
    /// If `reverse` is false, returns issues that this task depends on.
    pub fn task_dep_list(
        &self,
        task_issue_id: &str,
        reverse: bool,
    ) -> Result<Vec<TaskDependency>, GriteeError> {
        let mut args = vec!["issue", "dep", "list", task_issue_id];
        if reverse {
            args.push("--reverse");
        }

        let response: DepListResponse = self.run_json_direct(&args)?;
        Ok(response
            .deps
            .into_iter()
            .map(|d| TaskDependency {
                issue_id: d.issue_id,
                dep_type: DependencyType::from_str(&d.dep_type)
                    .unwrap_or(DependencyType::RelatedTo),
                title: d.title,
            })
            .collect())
    }

    /// Get tasks in topological order (ready-to-run first).
    ///
    /// Returns issues sorted so that dependencies come before dependents.
    /// Optionally filter by label (e.g., "convoy:c-xxx" to get tasks for a specific convoy).
    pub fn task_topo_order(
        &self,
        label: Option<&str>,
    ) -> Result<Vec<GriteeIssueSummary>, GriteeError> {
        let mut args = vec!["issue", "dep", "topo", "--state", "open"];
        if let Some(l) = label {
            args.push("--label");
            args.push(l);
        }

        let response: DepTopoResponse = self.run_json_direct(&args)?;
        Ok(response
            .issues
            .into_iter()
            .map(|r| r.into_gritee_issue_summary())
            .collect())
    }

    // -------------------------------------------------------------------------
    // Context operations (gritee context)
    // -------------------------------------------------------------------------

    /// Index files for symbol extraction.
    ///
    /// If `paths` is empty, indexes all tracked files.
    /// If `force` is true, re-indexes even if content hasn't changed.
    /// If `pattern` is provided, only files matching the glob are indexed.
    pub fn context_index(
        &self,
        paths: &[&str],
        force: bool,
        pattern: Option<&str>,
    ) -> Result<ContextIndexResult, GriteeError> {
        let mut args = vec!["context", "index"];
        for path in paths {
            args.push("--path");
            args.push(path);
        }
        if force {
            args.push("--force");
        }
        if let Some(pat) = pattern {
            args.push("--pattern");
            args.push(pat);
        }

        let response: ContextIndexResponse = self.run_json_direct(&args)?;
        Ok(ContextIndexResult {
            indexed: response.indexed,
            skipped: response.skipped,
            total_files: response.total_files,
        })
    }

    /// Query for symbols matching a pattern.
    ///
    /// Returns symbol names and their file paths.
    pub fn context_query(&self, query: &str) -> Result<Vec<SymbolMatch>, GriteeError> {
        let args = vec!["context", "query", query];
        let response: ContextQueryResponse = self.run_json_direct(&args)?;
        Ok(response
            .matches
            .into_iter()
            .map(|m| SymbolMatch {
                symbol: m.symbol,
                path: m.path,
            })
            .collect())
    }

    /// Show context for a specific file.
    ///
    /// Returns file metadata and extracted symbols.
    pub fn context_show(&self, path: &str) -> Result<FileContext, GriteeError> {
        let args = vec!["context", "show", path];
        let response: ContextShowResponse = self.run_json_direct(&args)?;
        Ok(FileContext {
            path: response.path,
            language: response.language,
            summary: response.summary,
            content_hash: response.content_hash,
            symbols: response
                .symbols
                .into_iter()
                .map(|s| Symbol {
                    name: s.name,
                    kind: s.kind,
                    line_start: s.line_start,
                    line_end: s.line_end,
                })
                .collect(),
        })
    }

    /// Get a project context value by key.
    ///
    /// Returns None if the key doesn't exist.
    pub fn context_project_get(&self, key: &str) -> Result<Option<String>, GriteeError> {
        let args = vec!["context", "project", key];
        match self.run_json_direct::<ContextProjectSingleResponse>(&args) {
            Ok(response) => Ok(Some(response.value)),
            Err(GriteeError::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// List all project context entries.
    pub fn context_project_list(&self) -> Result<Vec<ProjectContextEntry>, GriteeError> {
        let args = vec!["context", "project"];
        let response: ContextProjectListResponse = self.run_json_direct(&args)?;
        Ok(response
            .entries
            .into_iter()
            .map(|e| ProjectContextEntry {
                key: e.key,
                value: e.value,
            })
            .collect())
    }

    /// Set a project context value.
    pub fn context_project_set(&self, key: &str, value: &str) -> Result<(), GriteeError> {
        let args = vec!["context", "set", key, value];
        let _: serde_json::Value = self.run_json_direct(&args)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Internal helpers
    // -------------------------------------------------------------------------

    /// Check if grite should run without daemon.
    ///
    /// Returns true if the `GRITE_NO_DAEMON` environment variable is set.
    fn should_skip_daemon() -> bool {
        std::env::var("GRITE_NO_DAEMON").is_ok()
    }

    /// Run a grite command and parse JSON output.
    ///
    /// Retries on db_busy errors up to 3 times with exponential backoff.
    fn run_json<T: DeserializeOwned>(&self, args: &[&str]) -> Result<T, GriteeError> {
        let mut cmd_args = args.to_vec();
        cmd_args.push("--json");
        if Self::should_skip_daemon() {
            cmd_args.push("--no-daemon");
        }

        let max_retries = 5;
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                // Exponential backoff: 100ms, 200ms, 400ms, 800ms
                std::thread::sleep(std::time::Duration::from_millis(100 << attempt));
            }

            let output = Command::new("grite")
                .args(&cmd_args)
                .current_dir(&self.repo_root)
                .output()
                .map_err(|e| GriteeError::CommandFailed(format!("failed to run gritee: {}", e)))?;

            let stdout = String::from_utf8_lossy(&output.stdout);

            // Try to parse as JSON envelope
            let envelope: Result<JsonResponse<T>, _> = serde_json::from_str(&stdout);

            match envelope {
                Ok(env) => {
                    if let Some(error) = env.error {
                        // Check if it's a retryable database lock error
                        let is_retryable = error.code == "db_busy"
                            || error.code == "db_error"
                            || error.message.contains("could not acquire lock")
                            || error.message.contains("WouldBlock")
                            || error.message.contains("temporarily unavailable");
                        if is_retryable && attempt < max_retries - 1 {
                            last_error = Some(GriteeError::CommandFailed(error.message));
                            continue;
                        }
                        return Err(GriteeError::CommandFailed(error.message));
                    }
                    // Check schema version for compatibility
                    if let Some(version) = env.schema_version {
                        if version != EXPECTED_GRIT_SCHEMA_VERSION {
                            eprintln!(
                                "Warning: Grite schema version mismatch (expected {}, got {}). \
                                 Consider updating brat or gritee.",
                                EXPECTED_GRIT_SCHEMA_VERSION, version
                            );
                        }
                    }
                    return env.data.ok_or_else(|| {
                        GriteeError::UnexpectedResponse("missing data in response".into())
                    });
                }
                Err(e) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        // Check if stderr contains retryable lock indication
                        let is_retryable = stderr.contains("db_busy")
                            || stderr.contains("db_error")
                            || stderr.contains("Database locked")
                            || stderr.contains("could not acquire lock")
                            || stderr.contains("WouldBlock")
                            || stderr.contains("temporarily unavailable");
                        if is_retryable && attempt < max_retries - 1 {
                            last_error = Some(GriteeError::CommandFailed(stderr.to_string()));
                            continue;
                        }
                        return Err(GriteeError::CommandFailed(stderr.to_string()));
                    }
                    return Err(GriteeError::ParseError(format!(
                        "failed to parse gritee output: {} - raw: {}",
                        e, stdout
                    )));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GriteeError::CommandFailed("max retries exceeded".into())))
    }

    /// Run a grite command and parse the JSON output directly (no envelope wrapper).
    /// Used for issue commands which now return data directly.
    fn run_json_direct<T: DeserializeOwned>(&self, args: &[&str]) -> Result<T, GriteeError> {
        let mut cmd_args = args.to_vec();
        cmd_args.push("--json");
        if Self::should_skip_daemon() {
            cmd_args.push("--no-daemon");
        }

        let max_retries = 5;
        let mut last_error = None;

        for attempt in 0..max_retries {
            if attempt > 0 {
                // Exponential backoff: 100ms, 200ms, 400ms, 800ms
                std::thread::sleep(std::time::Duration::from_millis(100 << attempt));
            }

            let output = Command::new("grite")
                .args(&cmd_args)
                .current_dir(&self.repo_root)
                .output()
                .map_err(|e| GriteeError::CommandFailed(format!("failed to run gritee: {}", e)))?;

            let stdout = String::from_utf8_lossy(&output.stdout);

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                // Check if stderr contains retryable lock indication
                let is_retryable = stderr.contains("db_busy")
                    || stderr.contains("db_error")
                    || stderr.contains("Database locked")
                    || stderr.contains("could not acquire lock")
                    || stderr.contains("WouldBlock")
                    || stderr.contains("temporarily unavailable");
                if is_retryable && attempt < max_retries - 1 {
                    last_error = Some(GriteeError::CommandFailed(stderr.to_string()));
                    continue;
                }
                return Err(GriteeError::CommandFailed(stderr.to_string()));
            }

            // First try to parse as a JSON envelope with { ok, data, error } structure
            let json_value: serde_json::Value = match serde_json::from_str(&stdout) {
                Ok(v) => v,
                Err(e) => {
                    return Err(GriteeError::ParseError(format!(
                        "failed to parse gritee JSON output: {} - raw: {}",
                        e, stdout
                    )));
                }
            };

            // Check if it's an envelope structure
            if let Some(ok) = json_value.get("ok").and_then(|v| v.as_bool()) {
                if !ok {
                    // Command failed - extract error message
                    let error_msg = json_value
                        .get("error")
                        .and_then(|e| e.get("message"))
                        .and_then(|m| m.as_str())
                        .unwrap_or("unknown error");

                    // Check for retryable errors
                    let is_retryable = error_msg.contains("db_busy")
                        || error_msg.contains("db_error")
                        || error_msg.contains("could not acquire lock");
                    if is_retryable && attempt < max_retries - 1 {
                        last_error = Some(GriteeError::CommandFailed(error_msg.to_string()));
                        continue;
                    }
                    return Err(GriteeError::CommandFailed(error_msg.to_string()));
                }

                // Extract data from envelope
                if let Some(data) = json_value.get("data") {
                    match serde_json::from_value(data.clone()) {
                        Ok(result) => return Ok(result),
                        Err(e) => {
                            return Err(GriteeError::ParseError(format!(
                                "failed to parse gritee data: {} - raw: {}",
                                e, data
                            )));
                        }
                    }
                }
            }

            // Fall back to parsing directly (for backward compatibility)
            match serde_json::from_value(json_value.clone()) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    return Err(GriteeError::ParseError(format!(
                        "failed to parse gritee output: {} - raw: {}",
                        e, stdout
                    )));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GriteeError::CommandFailed("max retries exceeded".into())))
    }
}

/// Parse a convoy from a Grite issue summary.
fn parse_convoy_from_summary(issue: &GriteeIssueSummary) -> Result<Convoy, GriteeError> {
    // Extract convoy ID from labels
    let convoy_id = issue
        .labels
        .iter()
        .find_map(|label| label.strip_prefix("convoy:"))
        .ok_or_else(|| GriteeError::ParseError("missing convoy: label".into()))?
        .to_string();

    // Extract status from labels
    let status = issue
        .labels
        .iter()
        .find_map(|label| ConvoyStatus::from_label(label))
        .unwrap_or_default();

    Ok(Convoy {
        convoy_id,
        gritee_issue_id: issue.issue_id.clone(),
        title: issue.title.clone(),
        body: String::new(), // Summary doesn't include body
        status,
    })
}

/// Parse a task from a Grite issue summary.
fn parse_task_from_summary(issue: &GriteeIssueSummary) -> Result<Task, GriteeError> {
    // Extract task ID from labels
    let task_id = issue
        .labels
        .iter()
        .find_map(|label| label.strip_prefix("task:"))
        .ok_or_else(|| GriteeError::ParseError("missing task: label".into()))?
        .to_string();

    // Extract convoy ID from labels
    let convoy_id = issue
        .labels
        .iter()
        .find_map(|label| label.strip_prefix("convoy:"))
        .ok_or_else(|| GriteeError::ParseError("missing convoy: label".into()))?
        .to_string();

    // Extract status from labels
    let status = issue
        .labels
        .iter()
        .find_map(|label| TaskStatus::from_label(label))
        .unwrap_or_default();

    Ok(Task {
        task_id,
        gritee_issue_id: issue.issue_id.clone(),
        convoy_id,
        title: issue.title.clone(),
        body: String::new(), // Summary doesn't include body
        status,
    })
}

/// Parse a task from a summary and full issue (includes body).
fn parse_task_from_full_issue(
    summary: &GriteeIssueSummary,
    full: &GriteeIssue,
) -> Result<Task, GriteeError> {
    // Extract task ID from labels
    let task_id = summary
        .labels
        .iter()
        .find_map(|label| label.strip_prefix("task:"))
        .ok_or_else(|| GriteeError::ParseError("missing task: label".into()))?
        .to_string();

    // Extract convoy ID from labels
    let convoy_id = summary
        .labels
        .iter()
        .find_map(|label| label.strip_prefix("convoy:"))
        .ok_or_else(|| GriteeError::ParseError("missing convoy: label".into()))?
        .to_string();

    // Extract status from labels
    let status = summary
        .labels
        .iter()
        .find_map(|label| TaskStatus::from_label(label))
        .unwrap_or_default();

    Ok(Task {
        task_id,
        gritee_issue_id: summary.issue_id.clone(),
        convoy_id,
        title: full.title.clone(),
        body: full.body.clone(),
        status,
    })
}

// =============================================================================
// Session Comment Helpers
// =============================================================================

/// Format a session as a comment block per session-event-schema.md.
fn format_session_comment(session: &Session) -> String {
    let mut lines = vec![
        "[session]".to_string(),
        format!("state = \"{}\"", session.status),
        format!("session_id = \"{}\"", session.session_id),
        format!("role = \"{}\"", session.role.as_str()),
        format!("session_type = \"{}\"", session.session_type.as_str()),
        format!("engine = \"{}\"", session.engine),
        format!("worktree = \"{}\"", session.worktree),
    ];

    if let Some(pid) = session.pid {
        lines.push(format!("pid = {}", pid));
    } else {
        lines.push("pid = null".to_string());
    }

    lines.push(format!("started_ts = {}", session.started_ts));

    if let Some(ts) = session.last_heartbeat_ts {
        lines.push(format!("last_heartbeat_ts = {}", ts));
    } else {
        lines.push("last_heartbeat_ts = null".to_string());
    }

    match session.exit_code {
        Some(code) => lines.push(format!("exit_code = {}", code)),
        None => lines.push("exit_code = null".to_string()),
    }

    match &session.exit_reason {
        Some(reason) => lines.push(format!("exit_reason = \"{}\"", reason)),
        None => lines.push("exit_reason = null".to_string()),
    }

    match &session.last_output_ref {
        Some(ref_str) => lines.push(format!("last_output_ref = \"{}\"", ref_str)),
        None => lines.push("last_output_ref = null".to_string()),
    }

    lines.push("[/session]".to_string());
    lines.join("\n")
}

/// Parse session comment block from text.
///
/// Returns None if no valid session block is found.
fn parse_session_comment(text: &str) -> Option<SessionCommentData> {
    // Find [session] ... [/session] block
    let start = text.find("[session]")?;
    let end = text.find("[/session]")?;
    if end <= start {
        return None;
    }
    let block = &text[start + 9..end];

    // Parse key = value pairs (simplified TOML-like)
    let mut session_id = None;
    let mut state = None;
    let mut role = None;
    let mut session_type = None;
    let mut engine = None;
    let mut worktree = String::new();
    let mut pid = None;
    let mut started_ts = None;
    let mut last_heartbeat_ts = None;
    let mut exit_code = None;
    let mut exit_reason = None;
    let mut last_output_ref = None;

    for line in block.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            // Remove surrounding quotes if present
            let value = value.trim_matches('"');

            match key {
                "session_id" => session_id = Some(value.to_string()),
                "state" => state = SessionStatus::from_label(&format!("session:{}", value)),
                "role" => role = SessionRole::from_str(value),
                "session_type" => session_type = SessionType::from_str(value),
                "engine" => engine = Some(value.to_string()),
                "worktree" => worktree = value.to_string(),
                "pid" if value != "null" => pid = value.parse().ok(),
                "started_ts" => started_ts = value.parse().ok(),
                "last_heartbeat_ts" if value != "null" => last_heartbeat_ts = value.parse().ok(),
                "exit_code" if value != "null" => exit_code = value.parse().ok(),
                "exit_reason" if value != "null" => exit_reason = Some(value.to_string()),
                "last_output_ref" if value != "null" => last_output_ref = Some(value.to_string()),
                _ => {}
            }
        }
    }

    Some(SessionCommentData {
        session_id: session_id?,
        status: state?,
        role: role?,
        session_type: session_type?,
        engine: engine?,
        worktree,
        pid,
        started_ts: started_ts?,
        last_heartbeat_ts,
        exit_code,
        exit_reason,
        last_output_ref,
    })
}

/// Intermediate struct for parsed session comment data.
struct SessionCommentData {
    session_id: String,
    status: SessionStatus,
    role: SessionRole,
    session_type: SessionType,
    engine: String,
    worktree: String,
    pid: Option<u32>,
    started_ts: i64,
    last_heartbeat_ts: Option<i64>,
    exit_code: Option<i32>,
    exit_reason: Option<String>,
    last_output_ref: Option<String>,
}

/// Parse the latest session from a Grite issue (body + comments).
fn parse_latest_session_from_issue(
    issue: &GriteeIssue,
    summary: &GriteeIssueSummary,
) -> Option<Session> {
    // Extract task_id from labels
    let task_id = summary
        .labels
        .iter()
        .find_map(|label| label.strip_prefix("task:"))?
        .to_string();

    let data = latest_session_comment(issue)?;

    Some(Session {
        session_id: data.session_id,
        task_id,
        gritee_issue_id: issue.issue_id.clone(),
        role: data.role,
        session_type: data.session_type,
        engine: data.engine,
        worktree: data.worktree,
        pid: data.pid,
        status: data.status,
        started_ts: data.started_ts,
        last_heartbeat_ts: data.last_heartbeat_ts,
        exit_code: data.exit_code,
        exit_reason: data.exit_reason,
        last_output_ref: data.last_output_ref,
    })
}

/// Parse a specific session by ID from a Grite issue.
fn parse_session_by_id_from_issue(
    issue: &GriteeIssue,
    summary: &GriteeIssueSummary,
    session_id: &str,
) -> Option<Session> {
    // Extract task_id from labels
    let task_id = summary
        .labels
        .iter()
        .find_map(|label| label.strip_prefix("task:"))?
        .to_string();

    let data = session_comment_by_id(issue, session_id)?;

    Some(Session {
        session_id: data.session_id,
        task_id,
        gritee_issue_id: issue.issue_id.clone(),
        role: data.role,
        session_type: data.session_type,
        engine: data.engine,
        worktree: data.worktree,
        pid: data.pid,
        status: data.status,
        started_ts: data.started_ts,
        last_heartbeat_ts: data.last_heartbeat_ts,
        exit_code: data.exit_code,
        exit_reason: data.exit_reason,
        last_output_ref: data.last_output_ref,
    })
}

fn extract_issue_created_body(events: Option<&[serde_json::Value]>) -> Option<String> {
    events.into_iter().flatten().find_map(|event| {
        event
            .get("kind")
            .and_then(|kind| kind.get("IssueCreated"))
            .and_then(|issue| issue.get("body"))
            .and_then(|body| body.as_str())
            .map(ToString::to_string)
    })
}

fn extract_comment_bodies(events: Option<&[serde_json::Value]>) -> Vec<String> {
    events
        .into_iter()
        .flatten()
        .filter_map(|event| {
            event
                .get("kind")
                .and_then(|kind| kind.get("CommentAdded"))
                .and_then(|comment| comment.get("body"))
                .and_then(|body| body.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn latest_session_comment(issue: &GriteeIssue) -> Option<SessionCommentData> {
    let mut latest = parse_session_comment(&issue.body);
    for comment in &issue.comments {
        if let Some(data) = parse_session_comment(comment) {
            latest = Some(data);
        }
    }
    latest
}

fn session_comment_by_id(issue: &GriteeIssue, session_id: &str) -> Option<SessionCommentData> {
    let mut latest =
        parse_session_comment(&issue.body).filter(|data| data.session_id == session_id);
    for comment in &issue.comments {
        if let Some(data) = parse_session_comment(comment) {
            if data.session_id == session_id {
                latest = Some(data);
            }
        }
    }
    latest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_parse_session_comment_roundtrip() {
        let session = Session {
            session_id: "s-20250117-a2f9".to_string(),
            task_id: "t-20250117-b3c4".to_string(),
            gritee_issue_id: "issue-123".to_string(),
            role: SessionRole::Witness,
            session_type: SessionType::Polecat,
            engine: "shell".to_string(),
            worktree: ".gritee/worktrees/s-20250117-a2f9".to_string(),
            pid: Some(12345),
            status: SessionStatus::Running,
            started_ts: 1700000000000,
            last_heartbeat_ts: Some(1700000005000),
            exit_code: None,
            exit_reason: None,
            last_output_ref: None,
        };

        let comment = format_session_comment(&session);
        let parsed = parse_session_comment(&comment).expect("should parse");

        assert_eq!(parsed.session_id, session.session_id);
        assert_eq!(parsed.role, session.role);
        assert_eq!(parsed.session_type, session.session_type);
        assert_eq!(parsed.engine, session.engine);
        assert_eq!(parsed.worktree, session.worktree);
        assert_eq!(parsed.pid, session.pid);
        assert_eq!(parsed.status, session.status);
        assert_eq!(parsed.started_ts, session.started_ts);
        assert_eq!(parsed.last_heartbeat_ts, session.last_heartbeat_ts);
        assert_eq!(parsed.exit_code, session.exit_code);
        assert_eq!(parsed.exit_reason, session.exit_reason);
        assert_eq!(parsed.last_output_ref, session.last_output_ref);
    }

    #[test]
    fn test_format_parse_session_with_exit() {
        let session = Session {
            session_id: "s-20250117-dead".to_string(),
            task_id: "t-20250117-beef".to_string(),
            gritee_issue_id: "issue-456".to_string(),
            role: SessionRole::User,
            session_type: SessionType::Crew,
            engine: "claude".to_string(),
            worktree: "".to_string(),
            pid: None,
            status: SessionStatus::Exit,
            started_ts: 1700000000000,
            last_heartbeat_ts: Some(1700000010000),
            exit_code: Some(1),
            exit_reason: Some("timeout".to_string()),
            last_output_ref: Some("sha256:abc123".to_string()),
        };

        let comment = format_session_comment(&session);
        let parsed = parse_session_comment(&comment).expect("should parse");

        assert_eq!(parsed.session_id, session.session_id);
        assert_eq!(parsed.status, SessionStatus::Exit);
        assert_eq!(parsed.exit_code, Some(1));
        assert_eq!(parsed.exit_reason, Some("timeout".to_string()));
        assert_eq!(parsed.last_output_ref, Some("sha256:abc123".to_string()));
    }

    #[test]
    fn test_session_lookup_reads_session_comments() {
        let spawned = Session {
            session_id: "s-20250117-cafe".to_string(),
            task_id: "t-20250117-beef".to_string(),
            gritee_issue_id: "issue-456".to_string(),
            role: SessionRole::Witness,
            session_type: SessionType::Polecat,
            engine: "codex".to_string(),
            worktree: ".gritee/worktrees/s-20250117-cafe".to_string(),
            pid: Some(12345),
            status: SessionStatus::Spawned,
            started_ts: 1700000000000,
            last_heartbeat_ts: None,
            exit_code: None,
            exit_reason: None,
            last_output_ref: None,
        };
        let mut exited = spawned.clone();
        exited.status = SessionStatus::Exit;
        exited.exit_code = Some(0);
        exited.exit_reason = Some("completed successfully".to_string());
        exited.last_output_ref = Some("sha256:abc123".to_string());

        let issue = GriteeIssue {
            issue_id: "issue-456".to_string(),
            title: "task".to_string(),
            body: "task body".to_string(),
            comments: vec![
                format_session_comment(&spawned),
                "not a session".to_string(),
                format_session_comment(&exited),
            ],
            labels: vec!["task:t-20250117-beef".to_string()],
            state: "open".to_string(),
            updated_ts: 1700000010000,
        };
        let summary = GriteeIssueSummary {
            issue_id: "issue-456".to_string(),
            title: "task".to_string(),
            state: "open".to_string(),
            labels: vec!["task:t-20250117-beef".to_string()],
            updated_ts: 1700000010000,
        };

        let parsed =
            parse_session_by_id_from_issue(&issue, &summary, "s-20250117-cafe").expect("session");

        assert_eq!(parsed.status, SessionStatus::Exit);
        assert_eq!(parsed.exit_code, Some(0));
        assert_eq!(parsed.last_output_ref, Some("sha256:abc123".to_string()));
    }

    #[test]
    fn test_extract_comment_bodies_from_events() {
        let events = vec![
            serde_json::json!({
                "kind": {
                    "CommentAdded": {
                        "body": "first"
                    }
                }
            }),
            serde_json::json!({
                "kind": {
                    "LabelAdded": {
                        "label": "status:running"
                    }
                }
            }),
            serde_json::json!({
                "kind": {
                    "CommentAdded": {
                        "body": "second"
                    }
                }
            }),
        ];

        assert_eq!(
            extract_comment_bodies(Some(&events)),
            vec!["first".to_string(), "second".to_string()]
        );
    }

    #[test]
    fn test_extract_issue_created_body_from_events() {
        let events = vec![
            serde_json::json!({
                "kind": {
                    "LabelAdded": {
                        "label": "type:task"
                    }
                }
            }),
            serde_json::json!({
                "kind": {
                    "IssueCreated": {
                        "title": "task",
                        "body": "Allowed paths:\n- notes/probe.txt"
                    }
                }
            }),
        ];

        assert_eq!(
            extract_issue_created_body(Some(&events)),
            Some("Allowed paths:\n- notes/probe.txt".to_string())
        );
    }

    #[test]
    fn test_parse_session_comment_invalid() {
        // No session block
        assert!(parse_session_comment("no session here").is_none());

        // Empty block
        assert!(parse_session_comment("[session][/session]").is_none());

        // Missing required fields
        assert!(parse_session_comment("[session]\nstate = \"running\"\n[/session]").is_none());
    }
}
