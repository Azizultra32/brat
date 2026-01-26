use serde::{Deserialize, Serialize};

// =============================================================================
// Dependency Types (for grite issue dep commands)
// =============================================================================

/// Type of dependency relationship between issues/tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// This issue blocks the target issue.
    Blocks,
    /// This issue depends on the target issue.
    DependsOn,
    /// This issue is related to the target issue (non-directional).
    RelatedTo,
}

impl DependencyType {
    /// Convert to string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            DependencyType::Blocks => "blocks",
            DependencyType::DependsOn => "depends_on",
            DependencyType::RelatedTo => "related_to",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "blocks" => Some(DependencyType::Blocks),
            "depends_on" => Some(DependencyType::DependsOn),
            "related_to" => Some(DependencyType::RelatedTo),
            _ => None,
        }
    }
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A dependency relationship between tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDependency {
    /// The target task's grite issue ID.
    pub issue_id: String,
    /// The type of dependency.
    pub dep_type: DependencyType,
    /// The target task's title.
    pub title: String,
}

// =============================================================================
// Context Types (for grite context commands)
// =============================================================================

/// Result of context indexing operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextIndexResult {
    /// Number of files successfully indexed.
    pub indexed: u32,
    /// Number of files skipped (binary, unchanged, etc.).
    pub skipped: u32,
    /// Total number of files processed.
    pub total_files: u32,
}

/// A symbol match from context query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolMatch {
    /// The symbol name.
    pub symbol: String,
    /// The file path containing the symbol.
    pub path: String,
}

/// A symbol extracted from a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name.
    pub name: String,
    /// Symbol kind (function, class, struct, etc.).
    pub kind: String,
    /// Starting line number.
    pub line_start: u32,
    /// Ending line number.
    pub line_end: u32,
}

/// File context information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContext {
    /// File path.
    pub path: String,
    /// Detected programming language.
    pub language: String,
    /// AI-generated summary of the file.
    pub summary: String,
    /// Content hash (SHA256 hex).
    pub content_hash: String,
    /// Extracted symbols.
    pub symbols: Vec<Symbol>,
}

/// Project context key-value entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectContextEntry {
    /// The key.
    pub key: String,
    /// The value.
    pub value: String,
}

// =============================================================================
// Grit Issue Types
// =============================================================================

/// A Grit issue as returned by `grite issue show --json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GriteIssue {
    pub issue_id: String,
    pub title: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub updated_ts: i64,
}

/// Summary of a Grit issue from list command.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GriteIssueSummary {
    pub issue_id: String,
    pub title: String,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub updated_ts: i64,
}

/// Convoy status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConvoyStatus {
    Active,
    Paused,
    Complete,
    Failed,
}

impl ConvoyStatus {
    /// Convert to label string.
    pub fn as_label(&self) -> &'static str {
        match self {
            ConvoyStatus::Active => "status:active",
            ConvoyStatus::Paused => "status:paused",
            ConvoyStatus::Complete => "status:complete",
            ConvoyStatus::Failed => "status:failed",
        }
    }

    /// Parse from label string.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "status:active" => Some(ConvoyStatus::Active),
            "status:paused" => Some(ConvoyStatus::Paused),
            "status:complete" => Some(ConvoyStatus::Complete),
            "status:failed" => Some(ConvoyStatus::Failed),
            _ => None,
        }
    }

    /// All possible status labels.
    pub fn all_labels() -> &'static [&'static str] {
        &[
            "status:active",
            "status:paused",
            "status:complete",
            "status:failed",
        ]
    }
}

impl Default for ConvoyStatus {
    fn default() -> Self {
        ConvoyStatus::Active
    }
}

/// Task status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Queued,
    Running,
    Blocked,
    NeedsReview,
    Merged,
    Dropped,
}

impl TaskStatus {
    /// Convert to label string.
    pub fn as_label(&self) -> &'static str {
        match self {
            TaskStatus::Queued => "status:queued",
            TaskStatus::Running => "status:running",
            TaskStatus::Blocked => "status:blocked",
            TaskStatus::NeedsReview => "status:needs-review",
            TaskStatus::Merged => "status:merged",
            TaskStatus::Dropped => "status:dropped",
        }
    }

    /// Parse from label string.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "status:queued" => Some(TaskStatus::Queued),
            "status:running" => Some(TaskStatus::Running),
            "status:blocked" => Some(TaskStatus::Blocked),
            "status:needs-review" => Some(TaskStatus::NeedsReview),
            "status:merged" => Some(TaskStatus::Merged),
            "status:dropped" => Some(TaskStatus::Dropped),
            _ => None,
        }
    }

    /// All possible status labels for tasks.
    pub fn all_labels() -> &'static [&'static str] {
        &[
            "status:queued",
            "status:running",
            "status:blocked",
            "status:needs-review",
            "status:merged",
            "status:dropped",
        ]
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        TaskStatus::Queued
    }
}

/// Session lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session created, not yet ready.
    Spawned,
    /// Engine healthy, initial prompt delivered.
    Ready,
    /// Actively executing task work.
    Running,
    /// Waiting for review or merge.
    Handoff,
    /// Session terminated (success or failure).
    Exit,
}

impl SessionStatus {
    /// Convert to label string.
    pub fn as_label(&self) -> &'static str {
        match self {
            SessionStatus::Spawned => "session:spawned",
            SessionStatus::Ready => "session:ready",
            SessionStatus::Running => "session:running",
            SessionStatus::Handoff => "session:handoff",
            SessionStatus::Exit => "session:exit",
        }
    }

    /// Parse from label string.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "session:spawned" => Some(SessionStatus::Spawned),
            "session:ready" => Some(SessionStatus::Ready),
            "session:running" => Some(SessionStatus::Running),
            "session:handoff" => Some(SessionStatus::Handoff),
            "session:exit" => Some(SessionStatus::Exit),
            _ => None,
        }
    }

    /// All possible session status labels.
    pub fn all_labels() -> &'static [&'static str] {
        &[
            "session:spawned",
            "session:ready",
            "session:running",
            "session:handoff",
            "session:exit",
        ]
    }
}

impl Default for SessionStatus {
    fn default() -> Self {
        SessionStatus::Spawned
    }
}

/// Session type: polecat (isolated worktree) or crew (shared).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionType {
    /// Isolated worktree session managed by the Witness role.
    Polecat,
    /// Shared session for user-driven work.
    Crew,
}

impl SessionType {
    /// Convert to label string.
    pub fn as_label(&self) -> &'static str {
        match self {
            SessionType::Polecat => "session:polecat",
            SessionType::Crew => "session:crew",
        }
    }

    /// Parse from label string.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "session:polecat" => Some(SessionType::Polecat),
            "session:crew" => Some(SessionType::Crew),
            _ => None,
        }
    }

    /// Convert to string for comment format.
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionType::Polecat => "polecat",
            SessionType::Crew => "crew",
        }
    }

    /// Parse from string (for comment parsing).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "polecat" => Some(SessionType::Polecat),
            "crew" => Some(SessionType::Crew),
            _ => None,
        }
    }
}

impl Default for SessionType {
    fn default() -> Self {
        SessionType::Polecat
    }
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Actor role for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionRole {
    /// Strategic planner (readonly).
    Mayor,
    /// Worker controller, spawns polecat sessions.
    Witness,
    /// Post-merge cleanup and polish.
    Refinery,
    /// Background janitor for reconciliation and cleanup.
    Deacon,
    /// Human user.
    User,
}

impl SessionRole {
    /// Convert to string.
    pub fn as_str(&self) -> &'static str {
        match self {
            SessionRole::Mayor => "mayor",
            SessionRole::Witness => "witness",
            SessionRole::Refinery => "refinery",
            SessionRole::Deacon => "deacon",
            SessionRole::User => "user",
        }
    }

    /// Parse from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "mayor" => Some(SessionRole::Mayor),
            "witness" => Some(SessionRole::Witness),
            "refinery" => Some(SessionRole::Refinery),
            "deacon" => Some(SessionRole::Deacon),
            "user" => Some(SessionRole::User),
            _ => None,
        }
    }
}

impl Default for SessionRole {
    fn default() -> Self {
        SessionRole::User
    }
}

impl std::fmt::Display for SessionRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A parsed session from a task issue comment.
///
/// Sessions are stored as comments on task issues, not as separate issues.
/// This struct represents the parsed session state from a `[session]...[/session]` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Brat session ID (e.g., "s-20250117-7b3d").
    pub session_id: String,

    /// Associated task ID.
    pub task_id: String,

    /// Grit issue ID of the parent task.
    pub grite_issue_id: String,

    /// Role executing the session.
    pub role: SessionRole,

    /// Session type (polecat/crew).
    pub session_type: SessionType,

    /// Engine name (e.g., "codex", "claude", "shell").
    pub engine: String,

    /// Path to worktree (for polecat sessions).
    #[serde(default)]
    pub worktree: String,

    /// Process ID (if running).
    pub pid: Option<u32>,

    /// Session status.
    pub status: SessionStatus,

    /// Timestamp when session started (millis since epoch).
    pub started_ts: i64,

    /// Last heartbeat timestamp (millis since epoch).
    pub last_heartbeat_ts: Option<i64>,

    /// Exit code (if exited).
    pub exit_code: Option<i32>,

    /// Exit reason (signal, timeout, crash, user-stop, completed).
    pub exit_reason: Option<String>,

    /// Reference to last output (sha256:<hex>).
    pub last_output_ref: Option<String>,
}

/// A parsed convoy from a Grit issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Convoy {
    /// Brat convoy ID (e.g., "c-20250116-a2f9").
    pub convoy_id: String,

    /// Grit's internal issue ID.
    pub grite_issue_id: String,

    /// Convoy title.
    pub title: String,

    /// Convoy description/body.
    #[serde(default)]
    pub body: String,

    /// Convoy status.
    pub status: ConvoyStatus,
}

/// A parsed task from a Grit issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Brat task ID (e.g., "t-20250116-3a2c").
    pub task_id: String,

    /// Grit's internal issue ID.
    pub grite_issue_id: String,

    /// Parent convoy ID.
    pub convoy_id: String,

    /// Task title.
    pub title: String,

    /// Task description/body.
    #[serde(default)]
    pub body: String,

    /// Task status.
    pub status: TaskStatus,
}

impl Task {
    /// Parse paths from task body.
    ///
    /// Looks for a "Paths:" line in the body and extracts comma-separated paths.
    /// Example body line: `Paths: src/main.rs, src/lib.rs, tests/`
    ///
    /// Returns an empty Vec if no Paths line is found.
    pub fn parse_paths(&self) -> Vec<String> {
        for line in self.body.lines() {
            let trimmed = line.trim();
            if let Some(paths_str) = trimmed.strip_prefix("Paths:") {
                return paths_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_label_roundtrip() {
        for status in [
            TaskStatus::Queued,
            TaskStatus::Running,
            TaskStatus::Blocked,
            TaskStatus::NeedsReview,
            TaskStatus::Merged,
            TaskStatus::Dropped,
        ] {
            let label = status.as_label();
            let parsed = TaskStatus::from_label(label);
            assert_eq!(parsed, Some(status));
        }
    }

    #[test]
    fn test_convoy_status_label_roundtrip() {
        for status in [
            ConvoyStatus::Active,
            ConvoyStatus::Paused,
            ConvoyStatus::Complete,
            ConvoyStatus::Failed,
        ] {
            let label = status.as_label();
            let parsed = ConvoyStatus::from_label(label);
            assert_eq!(parsed, Some(status));
        }
    }

    #[test]
    fn test_invalid_label() {
        assert_eq!(TaskStatus::from_label("invalid"), None);
        assert_eq!(ConvoyStatus::from_label("status:unknown"), None);
    }

    #[test]
    fn test_session_type_label_roundtrip() {
        for session_type in [SessionType::Polecat, SessionType::Crew] {
            let label = session_type.as_label();
            let parsed = SessionType::from_label(label);
            assert_eq!(parsed, Some(session_type));
        }
    }

    #[test]
    fn test_session_type_str_roundtrip() {
        for session_type in [SessionType::Polecat, SessionType::Crew] {
            let s = session_type.as_str();
            let parsed = SessionType::from_str(s);
            assert_eq!(parsed, Some(session_type));
        }
    }

    #[test]
    fn test_session_role_roundtrip() {
        for role in [
            SessionRole::Mayor,
            SessionRole::Witness,
            SessionRole::Refinery,
            SessionRole::Deacon,
            SessionRole::User,
        ] {
            let s = role.as_str();
            let parsed = SessionRole::from_str(s);
            assert_eq!(parsed, Some(role));
        }
    }

    #[test]
    fn test_session_status_label_roundtrip() {
        for status in [
            SessionStatus::Spawned,
            SessionStatus::Ready,
            SessionStatus::Running,
            SessionStatus::Handoff,
            SessionStatus::Exit,
        ] {
            let label = status.as_label();
            let parsed = SessionStatus::from_label(label);
            assert_eq!(parsed, Some(status));
        }
    }

    #[test]
    fn test_task_parse_paths() {
        let task = Task {
            task_id: "t-20250117-test".to_string(),
            grite_issue_id: "issue-123".to_string(),
            convoy_id: "c-20250117-test".to_string(),
            title: "Test task".to_string(),
            body: "Some description\n\nPaths: src/main.rs, src/lib.rs, tests/\n\nMore text".to_string(),
            status: TaskStatus::Queued,
        };

        let paths = task.parse_paths();
        assert_eq!(paths, vec!["src/main.rs", "src/lib.rs", "tests/"]);
    }

    #[test]
    fn test_task_parse_paths_empty() {
        let task = Task {
            task_id: "t-20250117-test".to_string(),
            grite_issue_id: "issue-123".to_string(),
            convoy_id: "c-20250117-test".to_string(),
            title: "Test task".to_string(),
            body: "Some description without paths".to_string(),
            status: TaskStatus::Queued,
        };

        let paths = task.parse_paths();
        assert!(paths.is_empty());
    }

    #[test]
    fn test_task_parse_paths_single() {
        let task = Task {
            task_id: "t-20250117-test".to_string(),
            grite_issue_id: "issue-123".to_string(),
            convoy_id: "c-20250117-test".to_string(),
            title: "Test task".to_string(),
            body: "Paths: src/single.rs".to_string(),
            status: TaskStatus::Queued,
        };

        let paths = task.parse_paths();
        assert_eq!(paths, vec!["src/single.rs"]);
    }

    #[test]
    fn test_dependency_type_roundtrip() {
        for dep_type in [
            DependencyType::Blocks,
            DependencyType::DependsOn,
            DependencyType::RelatedTo,
        ] {
            let s = dep_type.as_str();
            let parsed = DependencyType::from_str(s);
            assert_eq!(parsed, Some(dep_type));
        }
    }

    #[test]
    fn test_dependency_type_invalid() {
        assert_eq!(DependencyType::from_str("invalid"), None);
        assert_eq!(DependencyType::from_str(""), None);
    }
}
