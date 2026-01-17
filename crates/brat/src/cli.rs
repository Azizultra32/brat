use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Brat - Multi-agent coding harness backed by Grit
#[derive(Parser, Debug)]
#[command(name = "brat", version, about, long_about = None)]
pub struct Cli {
    /// Output in JSON format
    #[arg(long, global = true)]
    pub json: bool,

    /// Suppress human-readable output
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Target a specific repository
    #[arg(long, global = true)]
    pub repo: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize Brat in the current repository
    Init(InitArgs),

    /// Show harness status
    Status(StatusArgs),

    /// Convoy management
    #[command(subcommand)]
    Convoy(ConvoyCommand),

    /// Task management
    #[command(subcommand)]
    Task(TaskCommand),

    /// Witness workflow (polecat session management)
    #[command(subcommand)]
    Witness(WitnessCommand),

    /// Refinery workflow (merge queue management)
    #[command(subcommand)]
    Refinery(RefineryCommand),

    /// Session management
    #[command(subcommand)]
    Session(SessionCommand),

    /// Lock status and management
    #[command(subcommand)]
    Lock(LockCommand),

    /// Health check and diagnostics
    Doctor(DoctorArgs),
}

/// Arguments for the init command
#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Don't start the bratd daemon
    #[arg(long)]
    pub no_daemon: bool,

    /// Don't create the tmux control room
    #[arg(long)]
    pub no_tmux: bool,

    /// Don't create .brat/config.toml
    #[arg(long)]
    pub no_config: bool,
}

/// Arguments for the status command
#[derive(Parser, Debug)]
pub struct StatusArgs {
    /// Aggregate status across all configured repos
    #[arg(long)]
    pub all_repos: bool,

    /// Filter by convoy ID
    #[arg(long)]
    pub convoy: Option<String>,

    /// Watch for changes (streaming mode)
    #[arg(long)]
    pub watch: bool,

    /// Poll interval in seconds for watch mode
    #[arg(long, default_value = "2")]
    pub poll_interval: u64,
}

/// Convoy subcommands
#[derive(Subcommand, Debug)]
pub enum ConvoyCommand {
    /// Create a new convoy
    Create(ConvoyCreateArgs),
}

/// Arguments for convoy create
#[derive(Parser, Debug)]
pub struct ConvoyCreateArgs {
    /// Convoy title
    #[arg(long)]
    pub title: String,

    /// Convoy body/description
    #[arg(long)]
    pub body: Option<String>,
}

/// Task subcommands
#[derive(Subcommand, Debug)]
pub enum TaskCommand {
    /// Create a new task
    Create(TaskCreateArgs),

    /// Update task status
    Update(TaskUpdateArgs),
}

/// Arguments for task create
#[derive(Parser, Debug)]
pub struct TaskCreateArgs {
    /// Convoy ID to link the task to
    #[arg(long)]
    pub convoy: String,

    /// Task title
    #[arg(long)]
    pub title: String,

    /// Task body/description
    #[arg(long)]
    pub body: Option<String>,
}

/// Arguments for task update
#[derive(Parser, Debug)]
pub struct TaskUpdateArgs {
    /// Task ID to update
    pub task_id: String,

    /// New status (queued, running, blocked, needs-review, merged, dropped)
    #[arg(long)]
    pub status: String,

    /// Force the transition (bypass state machine validation)
    #[arg(long)]
    pub force: bool,
}

/// Witness subcommands
#[derive(Subcommand, Debug)]
pub enum WitnessCommand {
    /// Run the witness workflow
    Run(WitnessRunArgs),
}

/// Arguments for witness run
#[derive(Parser, Debug)]
pub struct WitnessRunArgs {
    /// Run once and exit (default: run as daemon)
    #[arg(long)]
    pub once: bool,

    /// Poll interval in seconds for daemon mode
    #[arg(long, default_value = "10")]
    pub poll_interval: u64,

    /// Skip session reconciliation on startup
    #[arg(long)]
    pub skip_reconcile: bool,
}

/// Refinery subcommands
#[derive(Subcommand, Debug)]
pub enum RefineryCommand {
    /// Run the refinery workflow
    Run(RefineryRunArgs),
}

/// Arguments for refinery run
#[derive(Parser, Debug)]
pub struct RefineryRunArgs {
    /// Run once and exit (default: run as daemon)
    #[arg(long)]
    pub once: bool,

    /// Poll interval in seconds for daemon mode
    #[arg(long, default_value = "10")]
    pub poll_interval: u64,
}

/// Session subcommands
#[derive(Subcommand, Debug)]
pub enum SessionCommand {
    /// List active sessions
    List(SessionListArgs),
    /// Show session details
    Show(SessionShowArgs),
    /// Stop a session gracefully
    Stop(SessionStopArgs),
    /// Tail session logs
    Tail(SessionTailArgs),
}

/// Arguments for session list
#[derive(Parser, Debug)]
pub struct SessionListArgs {
    /// Filter by task ID
    #[arg(long)]
    pub task: Option<String>,
}

/// Arguments for session show
#[derive(Parser, Debug)]
pub struct SessionShowArgs {
    /// Session ID to show
    pub session_id: String,
}

/// Arguments for session stop
#[derive(Parser, Debug)]
pub struct SessionStopArgs {
    /// Session ID to stop
    pub session_id: String,

    /// Reason for stopping
    #[arg(long, default_value = "user-stop")]
    pub reason: String,
}

/// Arguments for session tail
#[derive(Parser, Debug)]
pub struct SessionTailArgs {
    /// Session ID to tail
    pub session_id: String,

    /// Number of lines to show
    #[arg(long, short = 'n', default_value = "50")]
    pub lines: usize,

    /// Follow log output (stream new lines)
    #[arg(long, short = 'f')]
    pub follow: bool,
}

/// Lock subcommands
#[derive(Subcommand, Debug)]
pub enum LockCommand {
    /// Show lock status
    Status(LockStatusArgs),
}

/// Arguments for lock status
#[derive(Parser, Debug)]
pub struct LockStatusArgs {
    /// Show only conflicting locks
    #[arg(long)]
    pub conflicts_only: bool,
}

/// Arguments for doctor command
#[derive(Parser, Debug)]
pub struct DoctorArgs {
    /// Check mode (read-only health validation)
    #[arg(long, conflicts_with = "rebuild")]
    pub check: bool,

    /// Rebuild mode (rebuilds harness state)
    #[arg(long, conflicts_with = "check")]
    pub rebuild: bool,
}
