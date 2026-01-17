//! Brat Configuration Library
//!
//! This crate provides configuration types for the Brat harness.
//! Configuration is stored in `.brat/config.toml`.

mod config;

pub use config::{
    BratConfig, BratdConfig, ConfigError, EngineConfig, InterventionsConfig, LocksConfig,
    LogsConfig, RefineryConfig, ReposConfig, RolesConfig, SwarmConfig, TmuxConfig,
};
