use std::io;

use libbrat_config::ConfigError;
use libbrat_grit::GritError;

use crate::workflows::WorkflowError;

/// Brat CLI errors.
#[derive(Debug, thiserror::Error)]
pub enum BratError {
    /// Not in a git repository.
    #[error("not a git repository (or any parent up to mount point)")]
    NotAGitRepo,

    /// Brat is not initialized in this repository.
    #[error("brat not initialized in this repository (run 'brat init' first)")]
    NotInitialized,

    /// Grit is not initialized in this repository.
    #[error("grit not initialized in this repository (run 'brat init' first)")]
    GritNotInitialized,

    /// Brat is already initialized.
    #[error("brat already initialized in this repository")]
    AlreadyInitialized,

    /// Grit initialization failed.
    #[error("grit init failed: {0}")]
    GritInitFailed(String),

    /// Grit command failed.
    #[error("grit command failed: {0}")]
    GritCommandFailed(String),

    /// Grit error.
    #[error("grit error: {0}")]
    Grit(#[from] GritError),

    /// Configuration error.
    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    /// IO error.
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    /// JSON serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Role is disabled in configuration.
    #[error("role disabled: {0}")]
    RoleDisabled(String),

    /// Workflow error.
    #[error("workflow error: {0}")]
    Workflow(#[from] WorkflowError),
}

impl BratError {
    /// Returns an error code for JSON output.
    pub fn error_code(&self) -> &'static str {
        match self {
            BratError::NotAGitRepo => "not_git_repo",
            BratError::NotInitialized => "not_initialized",
            BratError::GritNotInitialized => "grit_not_initialized",
            BratError::AlreadyInitialized => "already_initialized",
            BratError::GritInitFailed(_) => "grit_init_failed",
            BratError::GritCommandFailed(_) => "grit_command_failed",
            BratError::Grit(_) => "grit_error",
            BratError::Config(_) => "config_error",
            BratError::Io(_) => "io_error",
            BratError::Json(_) => "json_error",
            BratError::RoleDisabled(_) => "role_disabled",
            BratError::Workflow(_) => "workflow_error",
        }
    }

    /// Returns an exit code for the CLI.
    pub fn exit_code(&self) -> i32 {
        match self {
            BratError::NotAGitRepo => 2,
            BratError::NotInitialized => 3,
            BratError::GritNotInitialized => 4,
            BratError::AlreadyInitialized => 5,
            BratError::GritInitFailed(_) => 6,
            BratError::GritCommandFailed(_) => 7,
            BratError::Grit(_) => 8,
            BratError::Config(_) => 9,
            BratError::Io(_) => 10,
            BratError::Json(_) => 11,
            BratError::RoleDisabled(_) => 12,
            BratError::Workflow(_) => 13,
        }
    }
}
