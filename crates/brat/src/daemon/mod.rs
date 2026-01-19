//! Daemon management for bratd.
//!
//! This module provides utilities for managing the bratd daemon:
//! - Starting/stopping the daemon
//! - Checking if daemon is running
//! - Auto-starting daemon when needed

mod manager;

pub use manager::{DaemonManager, DaemonStatus};
