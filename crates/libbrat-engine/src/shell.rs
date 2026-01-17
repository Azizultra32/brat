use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};
use uuid::Uuid;

use crate::engine::{
    Engine, EngineHealth, EngineInput, SessionHandle, SpawnResult, SpawnSpec, StopMode,
    DEFAULT_STOP_TIMEOUT_MS,
};
use crate::error::EngineError;

/// Internal state for a shell session.
struct ShellSession {
    /// The child process.
    child: Child,

    /// Process ID.
    pid: u32,

    /// Captured stdout lines.
    stdout_lines: Vec<String>,

    /// Captured stderr lines (for future use).
    #[allow(dead_code)]
    stderr_lines: Vec<String>,

    /// Whether the session has been stopped.
    stopped: bool,

    /// Exit code if the process has exited.
    exit_code: Option<i32>,
}

/// Shell engine for spawning and controlling shell processes.
///
/// This is primarily used for testing and simulation. It spawns real
/// processes and tracks their lifecycle.
pub struct ShellEngine {
    /// Active sessions indexed by session ID.
    sessions: Arc<Mutex<HashMap<String, ShellSession>>>,
}

impl ShellEngine {
    /// Create a new shell engine.
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Generate a unique session ID.
    fn generate_session_id() -> String {
        format!("shell-{}", Uuid::new_v4().to_string().split('-').next().unwrap())
    }
}

impl Default for ShellEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Engine for ShellEngine {
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError> {
        let spawn_future = async {
            let mut cmd = Command::new(&spec.command);
            cmd.args(&spec.args)
                .current_dir(&spec.working_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            for (key, value) in &spec.env {
                cmd.env(key, value);
            }

            let child = cmd.spawn().map_err(|e| {
                EngineError::SpawnFailed(format!("failed to spawn {}: {}", spec.command, e))
            })?;

            let pid = child.id().ok_or_else(|| {
                EngineError::SpawnFailed("process exited immediately".to_string())
            })?;

            let session_id = Self::generate_session_id();

            let session = ShellSession {
                child,
                pid,
                stdout_lines: Vec::new(),
                stderr_lines: Vec::new(),
                stopped: false,
                exit_code: None,
            };

            let mut sessions = self.sessions.lock().await;
            sessions.insert(session_id.clone(), session);

            Ok(SpawnResult { session_id, pid })
        };

        timeout(Duration::from_millis(spec.timeout_ms), spawn_future)
            .await
            .map_err(|_| EngineError::Timeout(spec.timeout_ms))?
    }

    async fn send(&self, session: &SessionHandle, input: EngineInput) -> Result<(), EngineError> {
        let mut sessions = self.sessions.lock().await;
        let sess = sessions
            .get_mut(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

        if sess.stopped || sess.exit_code.is_some() {
            return Err(EngineError::SessionExited(session.session_id.clone()));
        }

        match input {
            EngineInput::Text(text) => {
                if let Some(stdin) = sess.child.stdin.as_mut() {
                    // Use std::io::Write for the synchronous write
                    let stdin_ref = stdin;
                    // We need to write synchronously, so we'll use try_write
                    use tokio::io::AsyncWriteExt;
                    stdin_ref.write_all(text.as_bytes()).await.map_err(|e| {
                        EngineError::SendFailed(format!("failed to write to stdin: {}", e))
                    })?;
                    stdin_ref.flush().await.map_err(|e| {
                        EngineError::SendFailed(format!("failed to flush stdin: {}", e))
                    })?;
                } else {
                    return Err(EngineError::SendFailed("stdin not available".to_string()));
                }
            }
            EngineInput::Signal(sig) => {
                // Send signal using libc
                #[cfg(unix)]
                {
                    let pid = sess.pid as i32;
                    unsafe {
                        libc::kill(pid, sig);
                    }
                }
                #[cfg(not(unix))]
                {
                    let _ = sig; // Suppress unused warning
                    return Err(EngineError::SendFailed(
                        "signals not supported on this platform".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }

    async fn tail(&self, session: &SessionHandle, n: usize) -> Result<Vec<String>, EngineError> {
        let mut sessions = self.sessions.lock().await;
        let sess = sessions
            .get_mut(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

        // Try to read any available output
        if let Some(stdout) = sess.child.stdout.take() {
            let mut reader = BufReader::new(stdout).lines();
            let mut count = 0;
            while let Ok(Some(line)) = reader.next_line().await {
                sess.stdout_lines.push(line);
                count += 1;
                if count >= 1000 {
                    break;
                }
            }
        }

        // Return the last n lines
        let lines = &sess.stdout_lines;
        let start = lines.len().saturating_sub(n);
        Ok(lines[start..].to_vec())
    }

    async fn stop(&self, session: &SessionHandle, how: StopMode) -> Result<(), EngineError> {
        let mut sessions = self.sessions.lock().await;
        let sess = sessions
            .get_mut(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

        if sess.stopped {
            return Ok(());
        }

        match how {
            StopMode::Graceful => {
                // Send SIGTERM first
                #[cfg(unix)]
                {
                    let pid = sess.pid as i32;
                    unsafe {
                        libc::kill(pid, libc::SIGTERM);
                    }
                }

                // Wait for process to exit with timeout
                let wait_result = timeout(
                    Duration::from_millis(DEFAULT_STOP_TIMEOUT_MS),
                    sess.child.wait(),
                )
                .await;

                match wait_result {
                    Ok(Ok(status)) => {
                        sess.exit_code = status.code();
                        sess.stopped = true;
                    }
                    Ok(Err(e)) => {
                        return Err(EngineError::Io(e));
                    }
                    Err(_) => {
                        // Timeout - force kill
                        sess.child.kill().await.ok();
                        sess.stopped = true;
                    }
                }
            }
            StopMode::Kill => {
                sess.child.kill().await.map_err(|e| EngineError::Io(e))?;
                sess.stopped = true;
            }
        }

        Ok(())
    }

    async fn health(&self, session: &SessionHandle) -> Result<EngineHealth, EngineError> {
        let mut sessions = self.sessions.lock().await;
        let sess = sessions
            .get_mut(&session.session_id)
            .ok_or_else(|| EngineError::SessionNotFound(session.session_id.clone()))?;

        // Check if process has exited
        match sess.child.try_wait() {
            Ok(Some(status)) => {
                let exit_code = status.code().unwrap_or(-1);
                sess.exit_code = Some(exit_code);
                sess.stopped = true;

                let reason = if status.success() {
                    "completed successfully".to_string()
                } else {
                    format!("exited with code {}", exit_code)
                };

                Ok(EngineHealth::exited(exit_code, reason))
            }
            Ok(None) => {
                // Still running
                Ok(EngineHealth::alive(sess.pid))
            }
            Err(e) => Err(EngineError::Io(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_and_health() {
        let engine = ShellEngine::new();

        // Spawn a simple echo command
        let spec = SpawnSpec::new("/bin/sh").args(["-c", "echo hello && sleep 0.1"]);

        let result = engine.spawn(spec).await.unwrap();
        assert!(!result.session_id.is_empty());
        assert!(result.pid > 0);

        let handle = SessionHandle::from(&result);

        // Check health - should be alive initially
        let _health = engine.health(&handle).await.unwrap();
        // Note: The process might exit quickly, so we just check it doesn't error

        // Wait a bit and check again
        tokio::time::sleep(Duration::from_millis(200)).await;
        let health = engine.health(&handle).await.unwrap();
        assert!(!health.alive);
        assert_eq!(health.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_spawn_and_stop() {
        let engine = ShellEngine::new();

        // Spawn a long-running command
        let spec = SpawnSpec::new("/bin/sh").args(["-c", "sleep 10"]);

        let result = engine.spawn(spec).await.unwrap();
        let handle = SessionHandle::from(&result);

        // Should be alive
        let health = engine.health(&handle).await.unwrap();
        assert!(health.alive);

        // Stop it
        engine.stop(&handle, StopMode::Graceful).await.unwrap();

        // Should be dead now
        let health = engine.health(&handle).await.unwrap();
        assert!(!health.alive);
    }

    #[tokio::test]
    async fn test_session_not_found() {
        let engine = ShellEngine::new();
        let handle = SessionHandle::new("nonexistent");

        let result = engine.health(&handle).await;
        assert!(matches!(result, Err(EngineError::SessionNotFound(_))));
    }
}
