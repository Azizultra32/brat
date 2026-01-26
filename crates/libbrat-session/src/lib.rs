//! Session lifecycle management for Brat.
//!
//! This crate bridges libbrat-engine, libbrat-gritee, and libbrat-worktree to
//! provide coordinated session lifecycle management. It enables:
//!
//! - Atomic spawn (worktree + process + Grite session)
//! - Background health polling with heartbeat updates
//! - State transition management
//! - Exit detection and cleanup
//!
//! # Example
//!
//! ```ignore
//! use libbrat_session::{SessionMonitor, MonitorConfig};
//! use libbrat_engine::ShellEngine;
//!
//! let engine = ShellEngine::new();
//! let gritee = GriteeClient::new("/path/to/repo");
//! let monitor = SessionMonitor::new(engine, "shell", gritee, None, MonitorConfig::default());
//!
//! // Spawn a session
//! let handle = monitor.spawn_session(
//!     "t-20250117-abcd",
//!     SessionRole::Witness,
//!     SessionType::Polecat,
//!     SpawnSpec::new("claude").arg("--session").arg("task-123"),
//! ).await?;
//!
//! // Subscribe to events
//! let mut events = monitor.subscribe();
//!
//! // Wait for ready
//! while let Ok(event) = events.recv().await {
//!     if matches!(event, MonitorEvent::Ready { .. }) {
//!         break;
//!     }
//! }
//! ```

mod config;
mod error;
mod event;
mod handle;
pub mod logs;
mod monitor;

pub use config::MonitorConfig;
pub use error::SessionMonitorError;
pub use event::MonitorEvent;
pub use handle::MonitorHandle;
pub use logs::write_session_logs;
pub use monitor::SessionMonitor;
