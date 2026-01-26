//! Event emitter for broadcasting events to WebSocket clients via the daemon.
//!
//! When the daemon is running, workflows can emit events that are broadcast
//! to all connected WebSocket clients for real-time UI updates.

use std::time::Duration;

use serde::Serialize;
use tracing::{debug, warn};

/// Default daemon port.
const DEFAULT_DAEMON_PORT: u16 = 3000;

/// Timeout for event broadcast requests.
const BROADCAST_TIMEOUT: Duration = Duration::from_secs(2);

/// Event emitter that sends events to the daemon for broadcasting.
#[derive(Clone)]
pub struct EventEmitter {
    /// Daemon URL for broadcasting events.
    daemon_url: String,
    /// Whether event emission is enabled.
    enabled: bool,
}

impl EventEmitter {
    /// Create a new event emitter.
    ///
    /// If the daemon is not running, event emission will be disabled.
    pub fn new() -> Self {
        let port = std::env::var("BRAT_DAEMON_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(DEFAULT_DAEMON_PORT);

        let daemon_url = format!("http://127.0.0.1:{}/api/v1/internal/broadcast", port);

        // Check if daemon is running by trying to reach health endpoint
        let enabled = Self::check_daemon_health(port);

        if enabled {
            debug!("Event emitter enabled, daemon at port {}", port);
        } else {
            debug!("Event emitter disabled, daemon not running");
        }

        Self { daemon_url, enabled }
    }

    /// Create a disabled event emitter (for testing or when daemon is not needed).
    pub fn disabled() -> Self {
        Self {
            daemon_url: String::new(),
            enabled: false,
        }
    }

    /// Check if the daemon is running.
    fn check_daemon_health(port: u16) -> bool {
        let url = format!("http://127.0.0.1:{}/api/v1/health", port);

        // Use a blocking client with short timeout
        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(1))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        client.get(&url).send().map(|r| r.status().is_success()).unwrap_or(false)
    }

    /// Emit a task updated event.
    pub fn task_updated(&self, task_id: &str, status: &str, convoy_id: Option<&str>) {
        self.emit(&TaskUpdatedEvent {
            r#type: "TaskUpdated",
            data: TaskUpdatedData {
                task_id: task_id.to_string(),
                status: status.to_string(),
                convoy_id: convoy_id.map(|s| s.to_string()),
            },
        });
    }

    /// Emit a session started event.
    pub fn session_started(&self, session_id: &str, task_id: &str, engine: &str) {
        self.emit(&SessionStartedEvent {
            r#type: "SessionStarted",
            data: SessionStartedData {
                session_id: session_id.to_string(),
                task_id: task_id.to_string(),
                engine: engine.to_string(),
            },
        });
    }

    /// Emit a session exited event.
    pub fn session_exited(&self, session_id: &str, task_id: &str, exit_code: i32) {
        self.emit(&SessionExitedEvent {
            r#type: "SessionExited",
            data: SessionExitedData {
                session_id: session_id.to_string(),
                task_id: task_id.to_string(),
                exit_code,
            },
        });
    }

    /// Emit a merge completed event.
    pub fn merge_completed(&self, task_id: &str, commit_sha: &str, branch: &str) {
        self.emit(&MergeCompletedEvent {
            r#type: "MergeCompleted",
            data: MergeCompletedData {
                task_id: task_id.to_string(),
                commit_sha: commit_sha.to_string(),
                branch: branch.to_string(),
            },
        });
    }

    /// Emit a merge failed event.
    pub fn merge_failed(&self, task_id: &str, error: &str, attempt: u32) {
        self.emit(&MergeFailedEvent {
            r#type: "MergeFailed",
            data: MergeFailedData {
                task_id: task_id.to_string(),
                error: error.to_string(),
                attempt,
            },
        });
    }

    /// Emit a merge rolled back event.
    pub fn merge_rolled_back(&self, task_id: &str, reset_sha: &str, reason: &str) {
        self.emit(&MergeRolledBackEvent {
            r#type: "MergeRolledBack",
            data: MergeRolledBackData {
                task_id: task_id.to_string(),
                reset_sha: reset_sha.to_string(),
                reason: reason.to_string(),
            },
        });
    }

    /// Emit a merge retry scheduled event.
    pub fn merge_retry_scheduled(&self, task_id: &str, retry_at: &str, attempt: u32) {
        self.emit(&MergeRetryScheduledEvent {
            r#type: "MergeRetryScheduled",
            data: MergeRetryScheduledData {
                task_id: task_id.to_string(),
                retry_at: retry_at.to_string(),
                attempt,
            },
        });
    }

    /// Emit an event to the daemon.
    fn emit<T: Serialize>(&self, event: &T) {
        if !self.enabled {
            return;
        }

        // Use a blocking client since we're in sync context
        let client = match reqwest::blocking::Client::builder()
            .timeout(BROADCAST_TIMEOUT)
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to create HTTP client for event broadcast: {}", e);
                return;
            }
        };

        match client
            .post(&self.daemon_url)
            .json(event)
            .send()
        {
            Ok(response) => {
                if !response.status().is_success() {
                    warn!("Event broadcast failed with status: {}", response.status());
                }
            }
            Err(e) => {
                // Don't warn on connection errors - daemon might have stopped
                debug!("Event broadcast failed: {}", e);
            }
        }
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// Event structures for serialization

#[derive(Serialize)]
struct TaskUpdatedEvent {
    r#type: &'static str,
    data: TaskUpdatedData,
}

#[derive(Serialize)]
struct TaskUpdatedData {
    task_id: String,
    status: String,
    convoy_id: Option<String>,
}

#[derive(Serialize)]
struct SessionStartedEvent {
    r#type: &'static str,
    data: SessionStartedData,
}

#[derive(Serialize)]
struct SessionStartedData {
    session_id: String,
    task_id: String,
    engine: String,
}

#[derive(Serialize)]
struct SessionExitedEvent {
    r#type: &'static str,
    data: SessionExitedData,
}

#[derive(Serialize)]
struct SessionExitedData {
    session_id: String,
    task_id: String,
    exit_code: i32,
}

#[derive(Serialize)]
struct MergeCompletedEvent {
    r#type: &'static str,
    data: MergeCompletedData,
}

#[derive(Serialize)]
struct MergeCompletedData {
    task_id: String,
    commit_sha: String,
    branch: String,
}

#[derive(Serialize)]
struct MergeFailedEvent {
    r#type: &'static str,
    data: MergeFailedData,
}

#[derive(Serialize)]
struct MergeFailedData {
    task_id: String,
    error: String,
    attempt: u32,
}

#[derive(Serialize)]
struct MergeRolledBackEvent {
    r#type: &'static str,
    data: MergeRolledBackData,
}

#[derive(Serialize)]
struct MergeRolledBackData {
    task_id: String,
    reset_sha: String,
    reason: String,
}

#[derive(Serialize)]
struct MergeRetryScheduledEvent {
    r#type: &'static str,
    data: MergeRetryScheduledData,
}

#[derive(Serialize)]
struct MergeRetryScheduledData {
    task_id: String,
    retry_at: String,
    attempt: u32,
}
