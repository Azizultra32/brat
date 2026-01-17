//! Session monitor implementation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use libbrat_engine::{Engine, SessionHandle, SpawnSpec, StopMode};
use libbrat_grit::{
    generate_session_id, GritClient, SessionRole, SessionStatus, SessionType, StateMachine,
};
use libbrat_worktree::WorktreeManager;
use tokio::sync::{broadcast, mpsc, watch, RwLock};
use tokio::task::JoinHandle;
use tokio::time::interval;

use crate::config::MonitorConfig;
use crate::error::SessionMonitorError;
use crate::event::MonitorEvent;
use crate::handle::{MonitorCommand, MonitorHandle};

/// Internal state for a monitored session.
#[allow(dead_code)]
struct SessionState {
    /// Session identifier.
    session_id: String,
    /// Associated task identifier.
    task_id: String,
    /// Engine session handle.
    engine_handle: SessionHandle,
    /// Current status.
    status: SessionStatus,
    /// Worktree path (if polecat session).
    worktree_path: Option<PathBuf>,
    /// Command sender for this session's monitoring task.
    command_tx: mpsc::Sender<MonitorCommand>,
    /// Handle to the monitoring task.
    task_handle: JoinHandle<()>,
    /// Number of consecutive health check failures.
    consecutive_failures: u32,
    /// Last heartbeat time.
    last_heartbeat: Instant,
}

/// Session lifecycle monitor.
///
/// Bridges Engine, Grit, and Worktree for coordinated session management.
/// Spawns background tasks for health polling and heartbeat updates.
pub struct SessionMonitor<E: Engine + 'static> {
    /// Engine for spawning and controlling sessions.
    engine: Arc<E>,
    /// Grit client for session persistence.
    grit: Arc<GritClient>,
    /// Worktree manager for polecat sessions.
    worktree_manager: Option<Arc<WorktreeManager>>,
    /// Configuration.
    config: MonitorConfig,
    /// Active session monitors.
    sessions: Arc<RwLock<HashMap<String, SessionState>>>,
    /// Event broadcaster.
    event_tx: broadcast::Sender<MonitorEvent>,
    /// Shutdown signal.
    shutdown_tx: watch::Sender<bool>,
    /// Shutdown receiver (clone for new tasks).
    shutdown_rx: watch::Receiver<bool>,
}

impl<E: Engine + 'static> SessionMonitor<E> {
    /// Create a new SessionMonitor.
    ///
    /// # Arguments
    ///
    /// * `engine` - Engine for spawning and controlling sessions.
    /// * `grit` - Grit client for session persistence.
    /// * `worktree_manager` - Optional worktree manager for polecat sessions.
    /// * `config` - Monitor configuration.
    pub fn new(
        engine: E,
        grit: GritClient,
        worktree_manager: Option<WorktreeManager>,
        config: MonitorConfig,
    ) -> Self {
        let (event_tx, _) = broadcast::channel(256);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        Self {
            engine: Arc::new(engine),
            grit: Arc::new(grit),
            worktree_manager: worktree_manager.map(Arc::new),
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            shutdown_tx,
            shutdown_rx,
        }
    }

    /// Subscribe to monitor events.
    pub fn subscribe(&self) -> broadcast::Receiver<MonitorEvent> {
        self.event_tx.subscribe()
    }

    /// Spawn a new session with full coordination.
    ///
    /// For polecat sessions:
    /// 1. Generate session ID
    /// 2. Create worktree
    /// 3. Spawn process in worktree
    /// 4. Create session record in Grit
    /// 5. Start monitoring task
    ///
    /// This is atomic: if any step fails, previous steps are rolled back.
    ///
    /// # Arguments
    ///
    /// * `task_id` - Task to associate with this session.
    /// * `role` - Session role (Witness, Refinery, etc.).
    /// * `session_type` - Session type (Polecat or Crew).
    /// * `spec` - Spawn specification for the engine.
    pub async fn spawn_session(
        &self,
        task_id: &str,
        role: SessionRole,
        session_type: SessionType,
        spec: SpawnSpec,
    ) -> Result<MonitorHandle, SessionMonitorError> {
        // 1. Generate session ID
        let session_id = generate_session_id();

        // 2. Create worktree if polecat session
        let (worktree_path, spawn_spec) = if session_type == SessionType::Polecat {
            if let Some(wm) = &self.worktree_manager {
                let path = wm.create(&session_id)?;
                let new_spec = SpawnSpec::new(&spec.command)
                    .working_dir(&path)
                    .args(spec.args.clone())
                    .timeout_ms(spec.timeout_ms);
                // Copy env vars
                let mut new_spec = new_spec;
                for (k, v) in &spec.env {
                    new_spec = new_spec.env(k, v);
                }
                (Some(path), new_spec)
            } else {
                return Err(SessionMonitorError::SpawnFailed(
                    "polecat session requires worktree manager".to_string(),
                ));
            }
        } else {
            (None, spec)
        };

        // 3. Spawn process (rollback worktree on failure)
        let spawn_result = match self.engine.spawn(spawn_spec).await {
            Ok(result) => result,
            Err(e) => {
                // Rollback worktree
                if let Some(ref path) = worktree_path {
                    if let Some(wm) = &self.worktree_manager {
                        let _ = wm.remove(&session_id);
                    }
                    let _ = path; // silence unused warning
                }
                return Err(e.into());
            }
        };

        // 4. Create Grit session (rollback engine on failure)
        let worktree_str = worktree_path
            .as_ref()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let _grit_session = match self.grit.session_create_with_id(
            Some(&session_id),
            task_id,
            role,
            session_type,
            "shell", // TODO: Get engine name from Engine trait
            &worktree_str,
            Some(spawn_result.pid),
        ) {
            Ok(session) => session,
            Err(e) => {
                // Rollback: stop engine and remove worktree
                let handle = SessionHandle::from(&spawn_result);
                let _ = self.engine.stop(&handle, StopMode::Kill).await;
                if let Some(wm) = &self.worktree_manager {
                    let _ = wm.remove(&session_id);
                }
                return Err(e.into());
            }
        };

        // 5. Start monitoring task
        let engine_handle = SessionHandle::from(&spawn_result);
        let (command_tx, command_rx) = mpsc::channel(16);

        let task_handle = self.spawn_monitor_task(
            session_id.clone(),
            task_id.to_string(),
            engine_handle.clone(),
            worktree_path.clone(),
            command_rx,
        );

        // Store session state
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(
                session_id.clone(),
                SessionState {
                    session_id: session_id.clone(),
                    task_id: task_id.to_string(),
                    engine_handle,
                    status: SessionStatus::Spawned,
                    worktree_path: worktree_path.clone(),
                    command_tx: command_tx.clone(),
                    task_handle,
                    consecutive_failures: 0,
                    last_heartbeat: Instant::now(),
                },
            );
        }

        // Emit spawned event
        let _ = self.event_tx.send(MonitorEvent::Spawned {
            session_id: session_id.clone(),
            task_id: task_id.to_string(),
            pid: spawn_result.pid,
            worktree_path: worktree_path.map(|p| p.to_string_lossy().to_string()),
        });

        Ok(MonitorHandle::new(session_id, command_tx))
    }

    /// List all monitored sessions.
    pub async fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.read().await;
        sessions.keys().cloned().collect()
    }

    /// Get handle to a monitored session.
    pub async fn get_handle(&self, session_id: &str) -> Option<MonitorHandle> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|state| {
            MonitorHandle::new(state.session_id.clone(), state.command_tx.clone())
        })
    }

    /// Graceful shutdown of all sessions.
    pub async fn shutdown(&self) -> Result<(), SessionMonitorError> {
        // Signal shutdown
        let _ = self.shutdown_tx.send(true);

        // Stop all sessions
        let sessions = self.sessions.read().await;
        for state in sessions.values() {
            let _ = state.command_tx.send(MonitorCommand::Shutdown).await;
        }

        Ok(())
    }

    /// Spawn the monitoring task for a session.
    fn spawn_monitor_task(
        &self,
        session_id: String,
        task_id: String,
        engine_handle: SessionHandle,
        worktree_path: Option<PathBuf>,
        command_rx: mpsc::Receiver<MonitorCommand>,
    ) -> JoinHandle<()> {
        let engine = Arc::clone(&self.engine);
        let grit = Arc::clone(&self.grit);
        let worktree_manager = self.worktree_manager.clone();
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let shutdown_rx = self.shutdown_rx.clone();
        let sessions = Arc::clone(&self.sessions);

        tokio::spawn(async move {
            Self::monitor_loop(
                engine,
                grit,
                worktree_manager,
                config,
                session_id,
                task_id,
                engine_handle,
                worktree_path,
                command_rx,
                event_tx,
                shutdown_rx,
                sessions,
            )
            .await
        })
    }

    /// The actual monitoring loop for a session.
    async fn monitor_loop(
        engine: Arc<E>,
        grit: Arc<GritClient>,
        worktree_manager: Option<Arc<WorktreeManager>>,
        config: MonitorConfig,
        session_id: String,
        _task_id: String,
        engine_handle: SessionHandle,
        worktree_path: Option<PathBuf>,
        mut command_rx: mpsc::Receiver<MonitorCommand>,
        event_tx: broadcast::Sender<MonitorEvent>,
        mut shutdown_rx: watch::Receiver<bool>,
        sessions: Arc<RwLock<HashMap<String, SessionState>>>,
    ) {
        let mut health_interval = interval(config.health_poll_interval);
        let mut heartbeat_interval = interval(config.heartbeat_interval);
        let mut consecutive_failures = 0u32;
        let mut current_status = SessionStatus::Spawned;

        loop {
            tokio::select! {
                // Health check timer
                _ = health_interval.tick() => {
                    match engine.health(&engine_handle).await {
                        Ok(health) if health.alive => {
                            consecutive_failures = 0;

                            let _ = event_tx.send(MonitorEvent::HealthCheck {
                                session_id: session_id.clone(),
                                alive: true,
                                consecutive_failures: 0,
                            });

                            // Auto-transition Spawned -> Ready on first successful health
                            if current_status == SessionStatus::Spawned {
                                if let Ok(()) = grit.session_update_status(&session_id, SessionStatus::Ready) {
                                    let _ = event_tx.send(MonitorEvent::StateChanged {
                                        session_id: session_id.clone(),
                                        from: SessionStatus::Spawned,
                                        to: SessionStatus::Ready,
                                    });
                                    let _ = event_tx.send(MonitorEvent::Ready {
                                        session_id: session_id.clone(),
                                    });
                                    current_status = SessionStatus::Ready;

                                    // Update session state
                                    let mut sessions = sessions.write().await;
                                    if let Some(state) = sessions.get_mut(&session_id) {
                                        state.status = SessionStatus::Ready;
                                    }
                                }
                            }
                        }
                        Ok(health) if !health.alive => {
                            // Process exited
                            let exit_code = health.exit_code.unwrap_or(-1);
                            let exit_reason = health.exit_reason.unwrap_or_else(|| "unknown".to_string());

                            Self::handle_exit(
                                &grit,
                                &engine,
                                &engine_handle,
                                &worktree_manager,
                                &config,
                                &session_id,
                                exit_code,
                                &exit_reason,
                                &event_tx,
                            ).await;
                            break;
                        }
                        Ok(_) => {}
                        Err(e) => {
                            consecutive_failures += 1;

                            let _ = event_tx.send(MonitorEvent::HealthCheck {
                                session_id: session_id.clone(),
                                alive: false,
                                consecutive_failures,
                            });

                            if consecutive_failures >= config.max_health_failures {
                                // Assume dead after too many failures
                                Self::handle_exit(
                                    &grit,
                                    &engine,
                                    &engine_handle,
                                    &worktree_manager,
                                    &config,
                                    &session_id,
                                    -1,
                                    &format!("health check timeout: {}", e),
                                    &event_tx,
                                ).await;
                                break;
                            }
                        }
                    }
                }

                // Heartbeat timer
                _ = heartbeat_interval.tick() => {
                    if let Err(e) = grit.session_heartbeat(&session_id) {
                        let _ = event_tx.send(MonitorEvent::Error {
                            session_id: Some(session_id.clone()),
                            error: format!("heartbeat failed: {}", e),
                        });
                    } else {
                        let _ = event_tx.send(MonitorEvent::Heartbeat {
                            session_id: session_id.clone(),
                        });
                    }
                }

                // Command from MonitorHandle
                Some(cmd) = command_rx.recv() => {
                    match cmd {
                        MonitorCommand::Transition { new_status, reply } => {
                            let result = Self::do_transition(
                                &grit,
                                &session_id,
                                current_status,
                                new_status,
                                &event_tx,
                            ).await;

                            if result.is_ok() {
                                // Update session state
                                let mut sessions = sessions.write().await;
                                if let Some(state) = sessions.get_mut(&session_id) {
                                    state.status = new_status;
                                }
                                current_status = new_status;
                            }

                            let _ = reply.send(result);
                        }
                        MonitorCommand::Stop { mode, reply } => {
                            let result = engine.stop(&engine_handle, mode).await
                                .map_err(SessionMonitorError::from);
                            let _ = reply.send(result);
                            // Don't break - wait for health check to detect exit
                        }
                        MonitorCommand::Shutdown => {
                            let _ = engine.stop(&engine_handle, StopMode::Graceful).await;
                            Self::handle_exit(
                                &grit,
                                &engine,
                                &engine_handle,
                                &worktree_manager,
                                &config,
                                &session_id,
                                0,
                                "shutdown",
                                &event_tx,
                            ).await;
                            break;
                        }
                    }
                }

                // Global shutdown signal
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        let _ = engine.stop(&engine_handle, StopMode::Graceful).await;
                        Self::handle_exit(
                            &grit,
                            &engine,
                            &engine_handle,
                            &worktree_manager,
                            &config,
                            &session_id,
                            0,
                            "monitor shutdown",
                            &event_tx,
                        ).await;
                        break;
                    }
                }
            }
        }

        // Cleanup: remove from sessions map
        {
            let mut sessions = sessions.write().await;
            sessions.remove(&session_id);
        }

        // Cleanup worktree if configured
        if config.cleanup_worktrees {
            if let Some(wm) = worktree_manager {
                if let Some(ref _path) = worktree_path {
                    if wm.remove(&session_id).is_ok() {
                        let _ = event_tx.send(MonitorEvent::WorktreeCleaned {
                            session_id: session_id.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Handle session exit.
    ///
    /// Captures the last N lines of output and writes them to `.grit/logs/`.
    async fn handle_exit(
        grit: &Arc<GritClient>,
        engine: &Arc<E>,
        engine_handle: &SessionHandle,
        _worktree_manager: &Option<Arc<WorktreeManager>>,
        config: &MonitorConfig,
        session_id: &str,
        exit_code: i32,
        exit_reason: &str,
        event_tx: &broadcast::Sender<MonitorEvent>,
    ) {
        // Capture last N lines of output for observability
        let last_output_ref = match engine.tail(engine_handle, config.exit_output_lines).await {
            Ok(lines) if !lines.is_empty() => {
                match crate::logs::write_session_logs(grit.repo_root(), session_id, &lines) {
                    Ok(hash_ref) => Some(hash_ref),
                    Err(e) => {
                        eprintln!("Warning: Failed to write session logs: {}", e);
                        None
                    }
                }
            }
            Ok(_) => None, // Empty output
            Err(e) => {
                eprintln!("Warning: Failed to capture session output: {}", e);
                None
            }
        };

        // Update Grit with log reference
        let _ = grit.session_exit(session_id, exit_code, exit_reason, last_output_ref.as_deref());

        // Emit event
        let _ = event_tx.send(MonitorEvent::Exited {
            session_id: session_id.to_string(),
            exit_code,
            exit_reason: exit_reason.to_string(),
        });
    }

    /// Perform a state transition.
    async fn do_transition(
        grit: &Arc<GritClient>,
        session_id: &str,
        current_status: SessionStatus,
        new_status: SessionStatus,
        event_tx: &broadcast::Sender<MonitorEvent>,
    ) -> Result<(), SessionMonitorError> {
        // Validate transition using state machine
        let machine = StateMachine::<SessionStatus>::new();
        if let Err(e) = machine.validate(current_status, new_status, false) {
            return Err(SessionMonitorError::InvalidTransition(e.to_string()));
        }

        // Update in Grit
        grit.session_update_status(session_id, new_status)?;

        // Emit event
        let _ = event_tx.send(MonitorEvent::StateChanged {
            session_id: session_id.to_string(),
            from: current_status,
            to: new_status,
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Note: Full integration tests require a mock engine implementation.
    // These tests verify basic structure and compilation.

    #[test]
    fn test_monitor_config_default() {
        let config = MonitorConfig::default();
        assert_eq!(config.health_poll_interval, Duration::from_secs(10));
    }
}
