//! Gemini engine for spawning and controlling Gemini CLI sessions.
//!
//! Gemini CLI provides access to Google's Gemini models with a free tier.
//! This engine spawns `gemini` processes for headless task execution.
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

/// Gemini engine for spawning Gemini CLI sessions.
///
/// Uses `gemini` CLI for headless execution with Google's models.
/// Processes are detached with `setsid` so they survive parent exit.
///
/// # Features
///
/// - Free tier available from Google
/// - Simple prompt-based interface
/// - Streaming output support
///
/// # Environment Variables
///
/// - `GEMINI_MODEL` - Model to use (e.g., "gemini-pro", "gemini-ultra")
/// - `GOOGLE_API_KEY` - Google API key for authentication
///
/// # Example
///
/// ```no_run
/// use libbrat_engine::{Engine, GeminiEngine, SpawnSpec};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let engine = GeminiEngine::new();
///
///     let spec = SpawnSpec::new("Explain async/await in JavaScript")
///         .env("GEMINI_MODEL", "gemini-pro");
///
///     let result = engine.spawn(spec).await?;
///     println!("Session started: {}", result.session_id);
///     Ok(())
/// }
/// ```
pub struct GeminiEngine {
    /// Active sessions.
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
}

impl GeminiEngine {
    /// Create a new Gemini engine.
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
        format!("gemini-{}-{:04x}", ts, rand)
    }

    /// Build the gemini command string from the spawn spec.
    fn build_command(&self, spec: &SpawnSpec) -> String {
        let escaped_prompt = spec.command.replace('\'', "'\\''");
        let mut cmd = String::from("gemini ");

        // Add model if specified in env
        if let Some(model) = spec.env.get("GEMINI_MODEL") {
            cmd.push_str(&format!("-m {} ", model));
        }

        // Enable streaming for real-time output
        cmd.push_str("-s ");

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
                        debug!(session_id = %session_id, "gemini output: {}", text);
                        if let Ok(mut sessions) = sessions.write() {
                            if let Some(state) = sessions.get_mut(&session_id) {
                                state.output_lines.push(text);
                            }
                        }
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading gemini output: {}", e);
                        break;
                    }
                }
            }
            debug!(session_id = %session_id, "gemini output stream closed");
        });
    }

    /// Collect stderr from process in background thread.
    fn spawn_stderr_collector(session_id: String, child_stderr: std::process::ChildStderr) {
        std::thread::spawn(move || {
            let reader = BufReader::new(child_stderr);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        warn!(session_id = %session_id, "gemini stderr: {}", text);
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading gemini stderr: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

impl Default for GeminiEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Engine for GeminiEngine {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError> {
        let session_id = Self::generate_session_id();
        info!(session_id = %session_id, working_dir = ?spec.working_dir, "spawning gemini session");

        // Build the gemini command
        let shell_cmd = self.build_command(&spec);
        info!(session_id = %session_id, shell_cmd = %shell_cmd, "gemini command");

        // Use bash login shell to ensure PATH includes gemini
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
            EngineError::SpawnFailed(format!("failed to spawn gemini: {}", e))
        })?;

        let pid = child.id();
        info!(session_id = %session_id, pid = pid, "gemini process spawned");

        // Take stdout for output collection
        let stdout = child.stdout.take().ok_or_else(|| {
            EngineError::SpawnFailed("failed to capture gemini stdout".to_string())
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

                warn!(session_id = %session.session_id, "graceful stop timed out, killing");
                let _ = state.child.kill();
            }
            StopMode::Kill => {
                state.child.kill().map_err(|e| {
                    EngineError::StopFailed(format!("failed to kill process: {}", e))
                })?;
            }
        }

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
            Ok(None) => Ok(EngineHealth::alive(state.pid)),
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
    fn test_gemini_engine_creation() {
        let engine = GeminiEngine::new();
        assert!(engine.sessions.read().unwrap().is_empty());
    }

    #[test]
    fn test_build_command_basic() {
        let engine = GeminiEngine::new();
        let spec = SpawnSpec::new("Explain async");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("gemini"));
        assert!(cmd.contains("-s"));
        assert!(cmd.contains("'Explain async'"));
    }

    #[test]
    fn test_build_command_with_model() {
        let engine = GeminiEngine::new();
        let spec = SpawnSpec::new("Question").env("GEMINI_MODEL", "gemini-ultra");
        let cmd = engine.build_command(&spec);
        assert!(cmd.contains("-m gemini-ultra"));
    }

    #[test]
    fn test_session_id_format() {
        let id = GeminiEngine::generate_session_id();
        assert!(id.starts_with("gemini-"));
    }
}
