//! Aider engine for spawning and controlling Aider CLI sessions.
//!
//! Aider is a mature AI coding CLI tool with excellent scripting support.
//! This engine spawns `aider --message --yes` processes for headless task execution.
//!
//! Processes are detached using `setsid` so they survive parent exit.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use async_trait::async_trait;
use tracing::{debug, info, warn};

use crate::engine::{
    Engine, EngineHealth, EngineInput, SessionHandle, SpawnResult, SpawnSpec, StopMode,
};
use crate::error::EngineError;

/// Session state tracked by the engine.
struct SessionState {
    /// Process handle.
    child: Child,
    /// PID of the process.
    pid: u32,
    /// Captured output lines.
    output_lines: Vec<String>,
    /// Whether the session has exited.
    exited: bool,
    /// Exit code if exited.
    exit_code: Option<i32>,
}

/// Aider engine for spawning Aider CLI sessions.
///
/// Uses `aider --message --yes` for headless execution.
/// Processes are detached with `setsid` so they survive parent exit.
///
/// # Features
///
/// - Multi-model support (GPT-4, Claude, Gemini, local models)
/// - Built-in git integration (disabled by default for brat control)
/// - File context via `--file` and `--read` flags
/// - Session history restoration
///
/// # Environment Variables
///
/// - `AIDER_MODEL` - Model to use (e.g., "gpt-4-turbo", "claude-3-opus")
/// - `AIDER_FILES` - Comma-separated list of files to edit
/// - `AIDER_READ_FILES` - Comma-separated list of read-only context files
///
/// # Example
///
/// ```no_run
/// use libbrat_engine::{Engine, AiderEngine, SpawnSpec};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let engine = AiderEngine::new();
///
///     let spec = SpawnSpec::new("Fix the login bug in auth.rs")
///         .env("AIDER_MODEL", "gpt-4-turbo")
///         .env("AIDER_FILES", "src/auth.rs");
///
///     let result = engine.spawn(spec).await?;
///     println!("Session started: {}", result.session_id);
///     Ok(())
/// }
/// ```
pub struct AiderEngine {
    /// Active sessions.
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
}

impl AiderEngine {
    /// Create a new Aider engine.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate a unique session ID.
    fn generate_session_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let rand: u16 = rand::random();
        format!("aider-{}-{:04x}", ts, rand)
    }

    /// Build the aider command string from the spawn spec.
    fn build_command(&self, spec: &SpawnSpec) -> String {
        let escaped_prompt = spec.command.replace('\'', "'\\''");
        let mut cmd = format!(
            "aider --message '{}' --yes --no-auto-commits --no-pretty",
            escaped_prompt
        );

        // Add model if specified in env
        if let Some(model) = spec.env.get("AIDER_MODEL") {
            cmd.push_str(&format!(" --model {}", model));
        }

        // Add files to edit if specified in env (comma-separated)
        if let Some(files) = spec.env.get("AIDER_FILES") {
            for file in files.split(',') {
                let file = file.trim();
                if !file.is_empty() {
                    cmd.push_str(&format!(" --file {}", file));
                }
            }
        }

        // Add read-only context files if specified (comma-separated)
        if let Some(files) = spec.env.get("AIDER_READ_FILES") {
            for file in files.split(',') {
                let file = file.trim();
                if !file.is_empty() {
                    cmd.push_str(&format!(" --read {}", file));
                }
            }
        }

        // Restore chat history if requested
        if spec.env.get("AIDER_RESTORE_HISTORY").is_some() {
            cmd.push_str(" --restore-chat-history");
        }

        // Disable git if requested
        if spec.env.get("AIDER_NO_GIT").is_some() {
            cmd.push_str(" --no-git");
        }

        cmd
    }

    /// Collect output from process in background thread.
    fn spawn_output_collector(
        sessions: Arc<RwLock<HashMap<String, SessionState>>>,
        session_id: String,
        child_stdout: std::process::ChildStdout,
    ) {
        std::thread::spawn(move || {
            let reader = BufReader::new(child_stdout);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        debug!(session_id = %session_id, "aider output: {}", text);
                        if let Ok(mut sessions) = sessions.write() {
                            if let Some(state) = sessions.get_mut(&session_id) {
                                state.output_lines.push(text);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading aider output: {}", e);
                        break;
                    }
                }
            }
            debug!(session_id = %session_id, "aider output stream closed");
        });
    }

    /// Collect stderr from process in background thread.
    fn spawn_stderr_collector(session_id: String, child_stderr: std::process::ChildStderr) {
        std::thread::spawn(move || {
            let reader = BufReader::new(child_stderr);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        warn!(session_id = %session_id, "aider stderr: {}", text);
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading aider stderr: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

impl Default for AiderEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Engine for AiderEngine {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError> {
        let session_id = Self::generate_session_id();
        info!(session_id = %session_id, working_dir = ?spec.working_dir, "spawning aider session");

        // Build the aider command
        let shell_cmd = self.build_command(&spec);
        info!(session_id = %session_id, shell_cmd = %shell_cmd, "aider command");

        // Use bash login shell to ensure PATH includes aider (via pip/pipx)
        let mut cmd = Command::new("bash");
        cmd.arg("-l").arg("-c").arg(&shell_cmd);

        // Set working directory
        cmd.current_dir(&spec.working_dir);

        // Set environment variables
        for (key, value) in &spec.env {
            cmd.env(key, value);
        }

        // Configure stdio
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Process detachment so process survives parent exit
        crate::platform::configure_detached_process(&mut cmd);

        // Spawn the process
        let mut child = cmd.spawn().map_err(|e| {
            EngineError::SpawnFailed(format!("failed to spawn aider: {}", e))
        })?;

        let pid = child.id();
        info!(session_id = %session_id, pid = pid, "aider process spawned");

        // Take stdout for output collection
        let stdout = child.stdout.take().ok_or_else(|| {
            EngineError::SpawnFailed("failed to capture aider stdout".to_string())
        })?;

        // Take stderr for error logging
        let stderr = child.stderr.take();

        // Store session state
        let state = SessionState {
            child,
            pid,
            output_lines: Vec::new(),
            exited: false,
            exit_code: None,
        };

        {
            let mut sessions = self.sessions.write().map_err(|_| {
                EngineError::SpawnFailed("failed to acquire session lock".to_string())
            })?;
            sessions.insert(session_id.clone(), state);
        }

        // Start background output collector
        Self::spawn_output_collector(Arc::clone(&self.sessions), session_id.clone(), stdout);

        // Start background stderr collector if available
        if let Some(stderr) = stderr {
            Self::spawn_stderr_collector(session_id.clone(), stderr);
        }

        Ok(SpawnResult { session_id, pid })
    }

    async fn send(&self, session: &SessionHandle, input: EngineInput) -> Result<(), EngineError> {
        let mut sessions = self.sessions.write().map_err(|_| {
            EngineError::SendFailed("failed to acquire session lock".to_string())
        })?;

        let state = sessions.get_mut(&session.session_id).ok_or_else(|| {
            EngineError::SessionNotFound(session.session_id.clone())
        })?;

        match input {
            EngineInput::Text(text) => {
                if let Some(ref mut stdin) = state.child.stdin {
                    stdin.write_all(text.as_bytes()).map_err(|e| {
                        EngineError::SendFailed(format!("failed to write to stdin: {}", e))
                    })?;
                    stdin.write_all(b"\n").map_err(|e| {
                        EngineError::SendFailed(format!("failed to write newline: {}", e))
                    })?;
                    stdin.flush().map_err(|e| {
                        EngineError::SendFailed(format!("failed to flush stdin: {}", e))
                    })?;
                } else {
                    return Err(EngineError::SendFailed("stdin not available".to_string()));
                }
            }
            EngineInput::Signal(sig) => {
                crate::platform::send_signal(state.pid, sig)
                    .map_err(EngineError::SendFailed)?;
            }
        }

        Ok(())
    }

    async fn tail(&self, session: &SessionHandle, n: usize) -> Result<Vec<String>, EngineError> {
        let sessions = self.sessions.read().map_err(|_| {
            EngineError::TailFailed("failed to acquire session lock".to_string())
        })?;

        let state = sessions.get(&session.session_id).ok_or_else(|| {
            EngineError::SessionNotFound(session.session_id.clone())
        })?;

        let lines = &state.output_lines;
        let start = lines.len().saturating_sub(n);
        Ok(lines[start..].to_vec())
    }

    async fn stop(&self, session: &SessionHandle, how: StopMode) -> Result<(), EngineError> {
        let mut sessions = self.sessions.write().map_err(|_| {
            EngineError::StopFailed("failed to acquire session lock".to_string())
        })?;

        let state = sessions.get_mut(&session.session_id).ok_or_else(|| {
            EngineError::SessionNotFound(session.session_id.clone())
        })?;

        match how {
            StopMode::Graceful => {
                // Try SIGTERM first
                let _ = crate::platform::send_term_signal(state.pid);

                // Wait up to 5 seconds for graceful exit
                for _ in 0..50 {
                    match state.child.try_wait() {
                        Ok(Some(status)) => {
                            state.exited = true;
                            state.exit_code = status.code();
                            return Ok(());
                        }
                        Ok(None) => {
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            return Err(EngineError::StopFailed(format!(
                                "failed to check process status: {}",
                                e
                            )));
                        }
                    }
                }

                // Fall through to kill
                warn!(session_id = %session.session_id, "graceful stop timed out, killing");
                let _ = state.child.kill();
            }
            StopMode::Kill => {
                state.child.kill().map_err(|e| {
                    EngineError::StopFailed(format!("failed to kill process: {}", e))
                })?;
            }
        }

        // Wait for process to exit
        match state.child.wait() {
            Ok(status) => {
                state.exited = true;
                state.exit_code = status.code();
            }
            Err(e) => {
                return Err(EngineError::StopFailed(format!(
                    "failed to wait for process: {}",
                    e
                )));
            }
        }

        Ok(())
    }

    async fn health(&self, session: &SessionHandle) -> Result<EngineHealth, EngineError> {
        let mut sessions = self.sessions.write().map_err(|_| {
            EngineError::HealthCheckFailed("failed to acquire session lock".to_string())
        })?;

        let state = sessions.get_mut(&session.session_id).ok_or_else(|| {
            EngineError::SessionNotFound(session.session_id.clone())
        })?;

        // Check if process has exited
        match state.child.try_wait() {
            Ok(Some(status)) => {
                state.exited = true;
                state.exit_code = status.code();
                let reason = if status.success() {
                    "completed successfully".to_string()
                } else {
                    format!("exited with code {:?}", status.code())
                };
                Ok(EngineHealth::exited(status.code().unwrap_or(-1), reason))
            }
            Ok(None) => {
                // Still running
                Ok(EngineHealth::alive(state.pid))
            }
            Err(e) => Err(EngineError::HealthCheckFailed(format!(
                "failed to check process status: {}",
                e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aider_engine_creation() {
        let engine = AiderEngine::new();
        assert!(engine.sessions.read().unwrap().is_empty());
    }

    #[test]
    fn test_build_command_basic() {
        let engine = AiderEngine::new();
        let spec = SpawnSpec::new("Fix the bug");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("aider --message"));
        assert!(cmd.contains("--yes"));
        assert!(cmd.contains("--no-auto-commits"));
        assert!(cmd.contains("--no-pretty"));
        assert!(cmd.contains("'Fix the bug'"));
    }

    #[test]
    fn test_build_command_with_model() {
        let engine = AiderEngine::new();
        let spec = SpawnSpec::new("Fix the bug")
            .env("AIDER_MODEL", "gpt-4-turbo");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--model gpt-4-turbo"));
    }

    #[test]
    fn test_build_command_with_files() {
        let engine = AiderEngine::new();
        let spec = SpawnSpec::new("Refactor these")
            .env("AIDER_FILES", "src/main.rs,src/lib.rs");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--file src/main.rs"));
        assert!(cmd.contains("--file src/lib.rs"));
    }

    #[test]
    fn test_build_command_with_read_files() {
        let engine = AiderEngine::new();
        let spec = SpawnSpec::new("Review this")
            .env("AIDER_READ_FILES", "README.md,DESIGN.md");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--read README.md"));
        assert!(cmd.contains("--read DESIGN.md"));
    }

    #[test]
    fn test_build_command_with_restore_history() {
        let engine = AiderEngine::new();
        let spec = SpawnSpec::new("Continue work")
            .env("AIDER_RESTORE_HISTORY", "1");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--restore-chat-history"));
    }

    #[test]
    fn test_build_command_with_no_git() {
        let engine = AiderEngine::new();
        let spec = SpawnSpec::new("Do something")
            .env("AIDER_NO_GIT", "1");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--no-git"));
    }

    #[test]
    fn test_session_id_format() {
        let id = AiderEngine::generate_session_id();
        assert!(id.starts_with("aider-"));
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "aider");
    }
}
