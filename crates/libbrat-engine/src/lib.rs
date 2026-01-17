//! Brat Engine Library
//!
//! This crate provides the `Engine` trait and implementations for spawning
//! and controlling coding agent sessions. Engines encapsulate how sessions
//! are spawned and controlled (Claude Code, Codex CLI, shell, etc.).
//!
//! # Example
//!
//! ```no_run
//! use libbrat_engine::{Engine, ShellEngine, SpawnSpec, SessionHandle, StopMode};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let engine = ShellEngine::new();
//!
//!     // Spawn a session
//!     let spec = SpawnSpec::new("/bin/sh").args(["-c", "echo hello"]);
//!     let result = engine.spawn(spec).await?;
//!
//!     // Get a handle to the session
//!     let handle = SessionHandle::from(&result);
//!
//!     // Check health
//!     let health = engine.health(&handle).await?;
//!     println!("Session alive: {}", health.alive);
//!
//!     // Stop the session
//!     engine.stop(&handle, StopMode::Graceful).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod engine;
pub mod error;
pub mod shell;

// Re-export public API
pub use engine::{
    Engine, EngineHealth, EngineInput, SessionHandle, SpawnResult, SpawnSpec, StopMode,
    DEFAULT_HEALTH_TIMEOUT_MS, DEFAULT_SEND_TIMEOUT_MS, DEFAULT_SPAWN_RETRY,
    DEFAULT_SPAWN_TIMEOUT_MS, DEFAULT_STOP_TIMEOUT_MS, DEFAULT_TAIL_TIMEOUT_MS,
};
pub use error::EngineError;
pub use shell::ShellEngine;
