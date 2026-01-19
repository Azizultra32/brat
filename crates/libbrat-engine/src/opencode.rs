//! OpenCode engine for spawning and controlling OpenCode CLI sessions.
//!
//! OpenCode is an open-source Claude Code alternative that supports 75+ LLM providers.
//! This engine spawns `opencode run` processes for headless task execution.
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

/// OpenCode engine for spawning OpenCode CLI sessions.
///
/// Uses `opencode run` for headless execution with various LLM providers.
/// Processes are detached with `setsid` so they survive parent exit.
///
/// # Features
///
/// - Supports 75+ LLM providers (OpenAI, Anthropic, Gemini, Bedrock, etc.)
/// - JSON output for structured parsing
/// - File context via `--file` flag
/// - Optional server mode for persistent sessions
///
/// # Example
///
/// ```no_run
/// use libbrat_engine::{Engine, OpenCodeEngine, SpawnSpec};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let engine = OpenCodeEngine::new();
///
///     let spec = SpawnSpec::new("Fix the login bug in auth.rs")
///         .env("OPENCODE_MODEL", "anthropic/claude-3-5-sonnet");
///
///     let result = engine.spawn(spec).await?;
///     println!("Session started: {}", result.session_id);
///     Ok(())
/// }
/// ```
pub struct OpenCodeEngine {
    /// Active sessions.
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
    /// Optional server URL for attach mode.
    server_url: Option<String>,
}

impl OpenCodeEngine {
    /// Create a new OpenCode engine.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            server_url: None,
        }
    }

    /// Create an OpenCode engine that connects to a running `opencode serve` instance.
    ///
    /// Using attach mode avoids cold start overhead for each task.
    ///
    /// # Arguments
    ///
    /// * `server_url` - URL of the running opencode server (e.g., "http://localhost:4096")
    pub fn with_server(server_url: impl Into<String>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            server_url: Some(server_url.into()),
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
        format!("opencode-{}-{:04x}", ts, rand)
    }

    /// Build the opencode command string from the spawn spec.
    fn build_command(&self, spec: &SpawnSpec) -> String {
        let escaped_prompt = spec.command.replace('\'', "'\\''");
        let mut cmd = String::new();

        // Use attach mode if server is configured
        if let Some(ref server_url) = self.server_url {
            cmd.push_str(&format!("opencode run --attach {} ", server_url));
        } else {
            cmd.push_str("opencode run ");
        }

        // Add model if specified in env
        if let Some(model) = spec.env.get("OPENCODE_MODEL") {
            cmd.push_str(&format!("--model {} ", model));
        }

        // Add files if specified in env (comma-separated)
        if let Some(files) = spec.env.get("OPENCODE_FILES") {
            for file in files.split(',') {
                let file = file.trim();
                if !file.is_empty() {
                    cmd.push_str(&format!("--file {} ", file));
                }
            }
        }

        // Add session continuation if specified
        if let Some(session) = spec.env.get("OPENCODE_SESSION") {
            cmd.push_str(&format!("--session {} ", session));
        }

        // JSON output for structured parsing (when not in attach mode)
        if self.server_url.is_none() {
            cmd.push_str("--format json ");
        }

        // The prompt
        cmd.push_str(&format!("'{}'", escaped_prompt));

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
                        debug!(session_id = %session_id, "opencode output: {}", text);
                        if let Ok(mut sessions) = sessions.write() {
                            if let Some(state) = sessions.get_mut(&session_id) {
                                state.output_lines.push(text);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading opencode output: {}", e);
                        break;
                    }
                }
            }
            debug!(session_id = %session_id, "opencode output stream closed");
        });
    }

    /// Collect stderr from process in background thread.
    fn spawn_stderr_collector(session_id: String, child_stderr: std::process::ChildStderr) {
        std::thread::spawn(move || {
            let reader = BufReader::new(child_stderr);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        warn!(session_id = %session_id, "opencode stderr: {}", text);
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading opencode stderr: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

impl Default for OpenCodeEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Engine for OpenCodeEngine {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError> {
        let session_id = Self::generate_session_id();
        info!(session_id = %session_id, working_dir = ?spec.working_dir, "spawning opencode session");

        // Build the opencode command
        let shell_cmd = self.build_command(&spec);
        info!(session_id = %session_id, shell_cmd = %shell_cmd, "opencode command");

        // Use bash login shell to ensure PATH includes opencode
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
            EngineError::SpawnFailed(format!("failed to spawn opencode: {}", e))
        })?;

        let pid = child.id();
        info!(session_id = %session_id, pid = pid, "opencode process spawned");

        // Take stdout for output collection
        let stdout = child.stdout.take().ok_or_else(|| {
            EngineError::SpawnFailed("failed to capture opencode stdout".to_string())
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
    fn test_opencode_engine_creation() {
        let engine = OpenCodeEngine::new();
        assert!(engine.sessions.read().unwrap().is_empty());
        assert!(engine.server_url.is_none());
    }

    #[test]
    fn test_opencode_engine_with_server() {
        let engine = OpenCodeEngine::with_server("http://localhost:4096");
        assert!(engine.sessions.read().unwrap().is_empty());
        assert_eq!(engine.server_url, Some("http://localhost:4096".to_string()));
    }

    #[test]
    fn test_build_command_basic() {
        let engine = OpenCodeEngine::new();
        let spec = SpawnSpec::new("Fix the bug");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("opencode run"));
        assert!(cmd.contains("--format json"));
        assert!(cmd.contains("'Fix the bug'"));
    }

    #[test]
    fn test_build_command_with_model() {
        let engine = OpenCodeEngine::new();
        let spec = SpawnSpec::new("Fix the bug")
            .env("OPENCODE_MODEL", "anthropic/claude-3-5-sonnet");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--model anthropic/claude-3-5-sonnet"));
    }

    #[test]
    fn test_build_command_with_files() {
        let engine = OpenCodeEngine::new();
        let spec = SpawnSpec::new("Refactor these")
            .env("OPENCODE_FILES", "src/main.rs,src/lib.rs");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--file src/main.rs"));
        assert!(cmd.contains("--file src/lib.rs"));
    }

    #[test]
    fn test_build_command_with_server() {
        let engine = OpenCodeEngine::with_server("http://localhost:4096");
        let spec = SpawnSpec::new("Fix the bug");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("--attach http://localhost:4096"));
        // JSON format is not added in attach mode
        assert!(!cmd.contains("--format json"));
    }

    #[test]
    fn test_session_id_format() {
        let id = OpenCodeEngine::generate_session_id();
        assert!(id.starts_with("opencode-"));
        // Should contain timestamp and random hex
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], "opencode");
    }
}
