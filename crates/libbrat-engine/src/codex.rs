//! Codex engine for spawning and controlling Codex CLI sessions.
//!
//! This engine spawns `codex exec --dangerously-bypass-approvals-and-sandbox
//! --json` processes for headless task execution. Output is captured and
//! streamed as JSONL events.
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

/// Codex engine for spawning Codex CLI sessions.
///
/// Uses `codex exec --dangerously-bypass-approvals-and-sandbox --json` for
/// headless execution with JSONL output.
/// Processes are detached with `setsid` so they survive parent exit.
pub struct CodexEngine {
    /// Active sessions.
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
}

impl CodexEngine {
    /// Create a new Codex engine.
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
        format!("codex-{}-{:04x}", ts, rand)
    }

    /// Filter Brat-internal spawn arguments before forwarding to Codex.
    fn filtered_codex_args(args: &[String]) -> Vec<String> {
        let mut filtered = Vec::new();
        let mut skip_next = false;

        for arg in args {
            if skip_next {
                skip_next = false;
                continue;
            }

            if arg == "--task" {
                skip_next = true;
                continue;
            }

            filtered.push(arg.clone());
        }

        filtered
    }

    /// Build argv for the current Codex CLI.
    fn build_codex_args(spec: &SpawnSpec) -> Vec<String> {
        let mut args = vec![
            "exec".to_string(),
            "--dangerously-bypass-approvals-and-sandbox".to_string(),
            "--json".to_string(),
            "--cd".to_string(),
            spec.working_dir.to_string_lossy().to_string(),
        ];

        args.extend(Self::filtered_codex_args(&spec.args));
        args.push(spec.command.clone());
        args
    }

    fn push_output_line(
        sessions: &Arc<RwLock<HashMap<String, SessionState>>>,
        session_id: &str,
        text: String,
    ) {
        if let Ok(mut sessions) = sessions.write() {
            if let Some(state) = sessions.get_mut(session_id) {
                state.output_lines.push(text);
            }
        }
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
                        debug!(session_id = %session_id, "codex output: {}", text);
                        Self::push_output_line(&sessions, &session_id, text);
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading codex output: {}", e);
                        break;
                    }
                }
            }
            // Mark session as potentially exited when stdout closes
            debug!(session_id = %session_id, "codex output stream closed");
        });
    }

    /// Collect stderr from process in background thread.
    fn spawn_stderr_collector(
        sessions: Arc<RwLock<HashMap<String, SessionState>>>,
        session_id: String,
        child_stderr: std::process::ChildStderr,
    ) {
        std::thread::spawn(move || {
            let reader = BufReader::new(child_stderr);
            for line in reader.lines() {
                match line {
                    Ok(text) => {
                        warn!(session_id = %session_id, "codex stderr: {}", text);
                        Self::push_output_line(
                            &sessions,
                            &session_id,
                            format!("[stderr] {}", text),
                        );
                    }
                    Err(e) => {
                        warn!(session_id = %session_id, "error reading codex stderr: {}", e);
                        break;
                    }
                }
            }
        });
    }
}

impl Default for CodexEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Engine for CodexEngine {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError> {
        let session_id = Self::generate_session_id();
        info!(session_id = %session_id, working_dir = ?spec.working_dir, "spawning codex session");

        let codex_args = Self::build_codex_args(&spec);
        info!(session_id = %session_id, args = ?codex_args, "codex command");

        let mut cmd = Command::new("codex");
        cmd.args(&codex_args);

        // Set working directory
        cmd.current_dir(&spec.working_dir);

        // Set environment
        for (key, value) in &spec.env {
            cmd.env(key, value);
        }

        // Configure stdio - null stdin since codex exec doesn't need input
        cmd.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // Process detachment so process survives parent exit
        crate::platform::configure_detached_process(&mut cmd);

        // Spawn the process
        let mut child = cmd
            .spawn()
            .map_err(|e| EngineError::SpawnFailed(format!("failed to spawn codex: {}", e)))?;

        let pid = child.id();
        info!(session_id = %session_id, pid = pid, "codex process spawned");

        // Take stdout for output collection
        let stdout = child.stdout.take().ok_or_else(|| {
            EngineError::SpawnFailed("failed to capture codex stdout".to_string())
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
            Self::spawn_stderr_collector(Arc::clone(&self.sessions), session_id.clone(), stderr);
        }

        Ok(SpawnResult { session_id, pid })
    }

    async fn send(&self, session: &SessionHandle, input: EngineInput) -> Result<(), EngineError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| EngineError::SendFailed("failed to acquire session lock".to_string()))?;

        let state = sessions
            .get_mut(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

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
                crate::platform::send_signal(state.pid, sig).map_err(EngineError::SendFailed)?;
            }
        }

        Ok(())
    }

    async fn tail(&self, session: &SessionHandle, n: usize) -> Result<Vec<String>, EngineError> {
        let sessions = self
            .sessions
            .read()
            .map_err(|_| EngineError::TailFailed("failed to acquire session lock".to_string()))?;

        let state = sessions
            .get(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

        let lines = &state.output_lines;
        let start = lines.len().saturating_sub(n);
        Ok(lines[start..].to_vec())
    }

    async fn stop(&self, session: &SessionHandle, how: StopMode) -> Result<(), EngineError> {
        let mut sessions = self
            .sessions
            .write()
            .map_err(|_| EngineError::StopFailed("failed to acquire session lock".to_string()))?;

        let state = sessions
            .get_mut(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

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

        let state = sessions
            .get_mut(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

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

    #[tokio::test]
    async fn test_codex_engine_creation() {
        let engine = CodexEngine::new();
        assert!(engine.sessions.read().unwrap().is_empty());
    }

    #[test]
    fn test_codex_args_use_current_cli_contract() {
        let spec = SpawnSpec::new("implement the task")
            .working_dir("/tmp/brat-task")
            .args(["--task", "t-20260423-e704", "--model", "gpt-5.4"]);

        let args = CodexEngine::build_codex_args(&spec);

        assert_eq!(args[0], "exec");
        assert!(args.contains(&"--dangerously-bypass-approvals-and-sandbox".to_string()));
        assert!(args.contains(&"--json".to_string()));
        assert!(!args.contains(&"--yolo".to_string()));
        assert!(!args.contains(&"--task".to_string()));
        assert!(!args.contains(&"t-20260423-e704".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"gpt-5.4".to_string()));

        let cd_index = args.iter().position(|arg| arg == "--cd").unwrap();
        assert_eq!(args[cd_index + 1], "/tmp/brat-task");
        assert_eq!(args.last().unwrap(), "implement the task");
    }
}
