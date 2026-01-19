//! Mayor engine for the AI-driven orchestrator.
//!
//! The Mayor is a special Claude Code session with elevated context that can:
//! - Analyze work requests and break them into tasks
//! - Create convoys and tasks via brat CLI
//! - Monitor progress and coordinate agents
//! - Respond to user queries about status
//!
//! The Mayor uses Claude Code's `--resume` flag to maintain conversation context
//! across multiple invocations. State is persisted to `.brat/mayor_state.json`.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::engine::{
    Engine, EngineHealth, EngineInput, SessionHandle, SpawnResult, SpawnSpec, StopMode,
};
use crate::error::EngineError;

/// Claude CLI JSON response format.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ClaudeResponse {
    #[serde(rename = "type")]
    response_type: String,
    result: Option<String>,
    session_id: String,
    is_error: Option<bool>,
}

/// Persisted mayor state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MayorState {
    /// Claude session ID (persists conversation).
    pub session_id: String,
    /// Working directory.
    pub working_dir: PathBuf,
    /// Accumulated output lines from all calls.
    pub output_lines: Vec<String>,
    /// Whether the session is logically active.
    pub active: bool,
}

impl MayorState {
    /// Path to the state file within a repo.
    pub fn state_file_path(repo_root: &PathBuf) -> PathBuf {
        repo_root.join(".brat").join("mayor_state.json")
    }

    /// Load state from disk.
    pub fn load(repo_root: &PathBuf) -> Option<Self> {
        let path = Self::state_file_path(repo_root);
        if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str(&content) {
                    Ok(state) => Some(state),
                    Err(e) => {
                        warn!("failed to parse mayor state: {}", e);
                        None
                    }
                },
                Err(e) => {
                    warn!("failed to read mayor state: {}", e);
                    None
                }
            }
        } else {
            None
        }
    }

    /// Save state to disk.
    pub fn save(&self, repo_root: &PathBuf) -> Result<(), EngineError> {
        let path = Self::state_file_path(repo_root);
        let content = serde_json::to_string_pretty(self).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to serialize mayor state: {}", e))
        })?;
        fs::write(&path, content).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to write mayor state: {}", e))
        })?;
        Ok(())
    }

    /// Delete state file.
    pub fn delete(repo_root: &PathBuf) {
        let path = Self::state_file_path(repo_root);
        let _ = fs::remove_file(&path);
    }
}

/// Mayor engine for AI-driven orchestration.
///
/// The Mayor uses Claude Code's session feature to maintain conversation context.
/// Each `ask` makes a new Claude `--print` call that continues the conversation.
/// State is persisted to disk to survive across CLI invocations.
pub struct MayorEngine {
    /// Repository root (for state persistence).
    repo_root: PathBuf,
}

impl MayorEngine {
    /// Create a new Mayor engine for the given repo.
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    /// Check if a mayor session is currently active.
    pub fn is_active(&self) -> bool {
        MayorState::load(&self.repo_root)
            .map(|s| s.active)
            .unwrap_or(false)
    }

    /// Get the current session ID if active.
    pub fn current_session_id(&self) -> Option<String> {
        MayorState::load(&self.repo_root)
            .filter(|s| s.active)
            .map(|s| s.session_id)
    }

    /// Get the current state if active.
    pub fn current_state(&self) -> Option<MayorState> {
        MayorState::load(&self.repo_root).filter(|s| s.active)
    }

    /// Write mayor context file to the workspace.
    fn write_mayor_context(working_dir: &PathBuf, workflows: &[String]) -> Result<(), EngineError> {
        let context_dir = working_dir.join(".claude");
        fs::create_dir_all(&context_dir).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to create .claude directory: {}", e))
        })?;

        let workflows_list = if workflows.is_empty() {
            "No workflows defined. You can create workflows in .brat/workflows/".to_string()
        } else {
            workflows
                .iter()
                .map(|w| format!("- {}", w))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let context = format!(
            r#"# Mayor Context

You are the **Mayor** - the primary AI coordinator for this workspace. Your role is to:
1. Analyze user requests and break them into discrete, parallelizable tasks
2. Create convoys (groups of related tasks) and individual tasks
3. Monitor progress and report status
4. Coordinate work across multiple agents

## Your Capabilities

You have access to the `brat` CLI. Always use `--json` for machine-readable output.

### Convoy Management
```bash
# Create a new convoy (group of related tasks)
brat convoy create --title "Convoy Title" --body "Description" --json

# Check status
brat status --json
```

### Task Management
```bash
# Create a task within a convoy
brat task create --convoy <convoy_id> --title "Task Title" --body "Detailed instructions" --json

# Update task status
brat task update <task_id> --status <queued|running|blocked|needs-review|merged|dropped>
```

### Workflow Execution
```bash
# List available workflow templates
brat workflow list --json

# Show workflow details
brat workflow show <name> --json

# Run a workflow (creates convoy + tasks from template)
brat workflow run <name> --var key=value --json
```

### Session Monitoring
```bash
# List active agent sessions
brat session list --json

# Show session details
brat session show <session_id> --json

# View session output
brat session tail <session_id> -n 50
```

## Available Workflows

{workflows_list}

## Guidelines

1. **Task Decomposition**: Break large requests into small, focused tasks that can run in parallel
2. **Clear Instructions**: Each task body should have complete context - agents can't see other tasks
3. **Use Workflows**: When a request matches an available workflow, use `brat workflow run`
4. **Monitor Progress**: Check `brat status` to see what's happening
5. **Report Back**: Summarize results and status to the user

## Important Notes

- Tasks are picked up by the Witness workflow and assigned to coding agents (Claude Code, Codex)
- Each task runs in its own git worktree for isolation
- Use convoy titles that describe the overall goal
- Use task titles that describe specific deliverables
"#,
            workflows_list = workflows_list
        );

        let context_file = context_dir.join("mayor_context.md");
        fs::write(&context_file, &context).map_err(|e| {
            EngineError::SpawnFailed(format!("failed to write mayor context: {}", e))
        })?;

        info!(context_file = ?context_file, "wrote mayor context");
        Ok(())
    }

    /// Execute a Claude call with the given message and return the response.
    /// If session_id is provided, resumes that session; otherwise starts a new one.
    fn execute_claude_call(
        session_id: Option<&str>,
        working_dir: &PathBuf,
        message: &str,
    ) -> Result<(String, String), EngineError> {
        // Build claude command with JSON output to get session_id
        // Use --permission-mode bypassPermissions to allow Mayor to run brat commands
        let escaped_message = message.replace("'", "'\\''");
        let shell_cmd = if let Some(sid) = session_id {
            format!(
                "claude --output-format json --print --permission-mode bypassPermissions --resume {} -p '{}'",
                sid, escaped_message
            )
        } else {
            format!(
                "claude --output-format json --print --permission-mode bypassPermissions -p '{}'",
                escaped_message
            )
        };

        info!(session_id = ?session_id, "executing claude call");
        debug!(shell_cmd = %shell_cmd, "claude command");

        let mut cmd = Command::new("bash");
        cmd.arg("-l").arg("-c").arg(&shell_cmd);
        cmd.current_dir(working_dir);
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let output = cmd.output().map_err(|e| {
            EngineError::SpawnFailed(format!("failed to execute claude: {}", e))
        })?;

        // Log stderr if any
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            for line in stderr.lines() {
                warn!("claude stderr: {}", line);
            }
        }

        if !output.status.success() {
            warn!(exit_code = ?output.status.code(), "claude exited with error");
        }

        // Parse JSON response
        let stdout = String::from_utf8_lossy(&output.stdout);
        let response: ClaudeResponse = serde_json::from_str(&stdout).map_err(|e| {
            EngineError::SpawnFailed(format!(
                "failed to parse claude response: {} (output: {})",
                e,
                stdout.chars().take(200).collect::<String>()
            ))
        })?;

        if response.is_error.unwrap_or(false) {
            return Err(EngineError::SpawnFailed(format!(
                "claude returned error: {}",
                response.result.unwrap_or_default()
            )));
        }

        let result_text = response.result.unwrap_or_default();
        Ok((response.session_id, result_text))
    }

    /// Send a message to the mayor and return the response.
    pub fn ask(&self, message: &str) -> Result<Vec<String>, EngineError> {
        let mut state = MayorState::load(&self.repo_root).ok_or_else(|| {
            EngineError::SessionNotFound("no active mayor session".to_string())
        })?;

        if !state.active {
            return Err(EngineError::SessionNotFound("mayor session not active".to_string()));
        }

        // Execute Claude call with session resumption
        let (new_session_id, result) = Self::execute_claude_call(
            Some(&state.session_id),
            &state.working_dir,
            message,
        )?;

        // Update session_id in case it changed
        state.session_id = new_session_id;

        // Append to output history
        state.output_lines.push(format!(">>> {}", message));
        let response_lines: Vec<String> = result.lines().map(|s| s.to_string()).collect();
        state.output_lines.extend(response_lines.clone());
        state.output_lines.push(String::new()); // Blank line separator

        // Save updated state
        state.save(&self.repo_root)?;

        Ok(response_lines)
    }

    /// Get the last N lines of output.
    pub fn tail(&self, n: usize) -> Result<Vec<String>, EngineError> {
        let state = MayorState::load(&self.repo_root).ok_or_else(|| {
            EngineError::SessionNotFound("no active mayor session".to_string())
        })?;

        let lines = &state.output_lines;
        let start = lines.len().saturating_sub(n);
        Ok(lines[start..].to_vec())
    }

    /// Stop the mayor session.
    pub fn stop_session(&self) -> Result<(), EngineError> {
        if !self.is_active() {
            return Err(EngineError::SessionNotFound("no active mayor session".to_string()));
        }

        MayorState::delete(&self.repo_root);
        info!("mayor session stopped");
        Ok(())
    }
}

impl Default for MayorEngine {
    fn default() -> Self {
        Self::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

#[async_trait]
impl Engine for MayorEngine {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError> {
        // Check if a session is already active
        if self.is_active() {
            return Err(EngineError::SpawnFailed(
                "mayor session already active - stop it first".to_string(),
            ));
        }

        info!(working_dir = ?spec.working_dir, "starting mayor session");

        // Load available workflows
        let workflows_dir = spec.working_dir.join(".brat/workflows");
        let workflows: Vec<String> = if workflows_dir.exists() {
            fs::read_dir(&workflows_dir)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == "yaml" || ext == "yml")
                                .unwrap_or(false)
                        })
                        .filter_map(|e| {
                            e.path()
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .map(|s| s.to_string())
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        // Write mayor context
        Self::write_mayor_context(&spec.working_dir, &workflows)?;

        // Build initial message
        let initial_message = if spec.command.is_empty() {
            "You are the Mayor. Read your context from .claude/mayor_context.md and confirm you understand your role. Briefly list your main capabilities.".to_string()
        } else {
            spec.command.clone()
        };

        // Execute initial Claude call (no session_id for first call)
        let (session_id, result) = Self::execute_claude_call(
            None,
            &spec.working_dir,
            &initial_message,
        )?;

        // Store state
        let mut output_lines = vec![format!(">>> {}", initial_message)];
        output_lines.extend(result.lines().map(|s| s.to_string()));
        output_lines.push(String::new());

        let state = MayorState {
            session_id: session_id.clone(),
            working_dir: spec.working_dir.clone(),
            output_lines,
            active: true,
        };

        state.save(&self.repo_root)?;

        info!(session_id = %session_id, "mayor session started");

        // Return a pseudo-PID (we don't have a persistent process)
        Ok(SpawnResult {
            session_id,
            pid: std::process::id(), // Use current process ID as placeholder
        })
    }

    async fn send(&self, _session: &SessionHandle, input: EngineInput) -> Result<(), EngineError> {
        match input {
            EngineInput::Text(text) => {
                // For text input, use ask()
                self.ask(&text)?;
                Ok(())
            }
            EngineInput::Signal(_) => {
                // Signals don't make sense for this session-based approach
                Err(EngineError::SendFailed("signals not supported for mayor".to_string()))
            }
        }
    }

    async fn tail(&self, _session: &SessionHandle, n: usize) -> Result<Vec<String>, EngineError> {
        self.tail(n)
    }

    async fn stop(&self, _session: &SessionHandle, _how: StopMode) -> Result<(), EngineError> {
        self.stop_session()
    }

    async fn health(&self, _session: &SessionHandle) -> Result<EngineHealth, EngineError> {
        if self.is_active() {
            Ok(EngineHealth::alive(std::process::id()))
        } else {
            Ok(EngineHealth::exited(0, "session not active".to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mayor_engine_creation() {
        let dir = tempdir().unwrap();
        let engine = MayorEngine::new(dir.path().to_path_buf());
        assert!(!engine.is_active());
        assert!(engine.current_session_id().is_none());
    }

    #[test]
    fn test_mayor_state_persistence() {
        let dir = tempdir().unwrap();
        let repo_root = dir.path().to_path_buf();

        // Create .brat directory
        fs::create_dir_all(repo_root.join(".brat")).unwrap();

        let state = MayorState {
            session_id: "test-123".to_string(),
            working_dir: repo_root.clone(),
            output_lines: vec!["line1".to_string(), "line2".to_string()],
            active: true,
        };

        // Save and load
        state.save(&repo_root).unwrap();
        let loaded = MayorState::load(&repo_root).unwrap();

        assert_eq!(loaded.session_id, "test-123");
        assert_eq!(loaded.output_lines.len(), 2);
        assert!(loaded.active);

        // Delete
        MayorState::delete(&repo_root);
        assert!(MayorState::load(&repo_root).is_none());
    }
}
