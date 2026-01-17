use std::collections::HashMap;
use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::EngineError;

// Default timeouts from docs/engine.md
pub const DEFAULT_SPAWN_TIMEOUT_MS: u64 = 60_000;
pub const DEFAULT_SEND_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_TAIL_TIMEOUT_MS: u64 = 10_000;
pub const DEFAULT_STOP_TIMEOUT_MS: u64 = 10_000;
pub const DEFAULT_HEALTH_TIMEOUT_MS: u64 = 5_000;
pub const DEFAULT_SPAWN_RETRY: u32 = 1;

/// Specification for spawning a new engine session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSpec {
    /// Working directory for the session.
    pub working_dir: PathBuf,

    /// Command to execute.
    pub command: String,

    /// Arguments to pass to the command.
    pub args: Vec<String>,

    /// Environment variables to set.
    pub env: HashMap<String, String>,

    /// Timeout for the spawn operation in milliseconds.
    #[serde(default = "default_spawn_timeout")]
    pub timeout_ms: u64,
}

fn default_spawn_timeout() -> u64 {
    DEFAULT_SPAWN_TIMEOUT_MS
}

impl SpawnSpec {
    /// Create a new spawn spec with the given command.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            command: command.into(),
            args: Vec::new(),
            env: HashMap::new(),
            timeout_ms: DEFAULT_SPAWN_TIMEOUT_MS,
        }
    }

    /// Set the working directory.
    pub fn working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = dir.into();
        self
    }

    /// Add an argument.
    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add multiple arguments.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(|a| a.into()));
        self
    }

    /// Set an environment variable.
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set the spawn timeout.
    pub fn timeout_ms(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
}

/// Result of successfully spawning a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnResult {
    /// Unique identifier for this session.
    pub session_id: String,

    /// Process ID of the spawned session.
    pub pid: u32,
}

/// Handle to an existing session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SessionHandle {
    /// Unique identifier for this session.
    pub session_id: String,
}

impl SessionHandle {
    /// Create a new session handle.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
        }
    }
}

impl From<&SpawnResult> for SessionHandle {
    fn from(result: &SpawnResult) -> Self {
        Self {
            session_id: result.session_id.clone(),
        }
    }
}

impl From<SpawnResult> for SessionHandle {
    fn from(result: SpawnResult) -> Self {
        Self {
            session_id: result.session_id,
        }
    }
}

/// Input to send to a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineInput {
    /// Text input (written to stdin).
    Text(String),

    /// Signal to send to the process.
    Signal(i32),
}

/// How to stop a session.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StopMode {
    /// Graceful shutdown (SIGTERM, then wait).
    Graceful,

    /// Immediate kill (SIGKILL).
    Kill,
}

/// Health status of a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineHealth {
    /// Whether the session is still alive.
    pub alive: bool,

    /// Process ID (if known).
    pub pid: Option<u32>,

    /// Exit code (if the session has exited).
    pub exit_code: Option<i32>,

    /// Exit reason description (if the session has exited).
    pub exit_reason: Option<String>,
}

impl EngineHealth {
    /// Create a health status for a running session.
    pub fn alive(pid: u32) -> Self {
        Self {
            alive: true,
            pid: Some(pid),
            exit_code: None,
            exit_reason: None,
        }
    }

    /// Create a health status for an exited session.
    pub fn exited(exit_code: i32, reason: impl Into<String>) -> Self {
        Self {
            alive: false,
            pid: None,
            exit_code: Some(exit_code),
            exit_reason: Some(reason.into()),
        }
    }
}

/// Engine trait for spawning and controlling sessions.
///
/// Engines encapsulate how sessions are spawned and controlled. Different
/// implementations handle different backends (Claude Code, Codex CLI, shell, etc.).
#[async_trait]
pub trait Engine: Send + Sync {
    /// Spawn a new session with the given specification.
    async fn spawn(&self, spec: SpawnSpec) -> Result<SpawnResult, EngineError>;

    /// Send input to an existing session.
    async fn send(&self, session: &SessionHandle, input: EngineInput) -> Result<(), EngineError>;

    /// Get the last N lines of output from a session.
    async fn tail(&self, session: &SessionHandle, n: usize) -> Result<Vec<String>, EngineError>;

    /// Stop a session.
    async fn stop(&self, session: &SessionHandle, how: StopMode) -> Result<(), EngineError>;

    /// Check the health of a session.
    async fn health(&self, session: &SessionHandle) -> Result<EngineHealth, EngineError>;
}
