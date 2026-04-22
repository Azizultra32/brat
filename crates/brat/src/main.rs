mod agents_md;
mod api;
mod cli;
mod commands;
mod context;
mod daemon;
mod error;
mod grite_cli;
mod output;
mod workflows;

use clap::Parser;

use cli::{Cli, Command};
use daemon::DaemonManager;
use error::BratError;
use output::output_error;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = run_command(&cli).await;

    if let Err(err) = result {
        output_error(&cli, &err);
        std::process::exit(err.exit_code());
    }
}

/// Check if the command benefits from having the daemon running.
fn should_auto_start_daemon(cmd: &Command) -> bool {
    matches!(
        cmd,
        Command::Status(_)
            | Command::Convoy(_)
            | Command::Task(_)
            | Command::Context(_)
            | Command::Session(_)
            | Command::Mayor(_)
            | Command::Witness(_)
            | Command::Refinery(_)
    )
}

async fn run_command(cli: &Cli) -> Result<(), BratError> {
    // Auto-start daemon if needed and not disabled
    if !cli.no_daemon && should_auto_start_daemon(&cli.command) {
        let manager = DaemonManager::new();
        if let Err(e) = manager.ensure_running() {
            // Log warning but don't fail - commands work without daemon
            if !cli.quiet && !cli.json {
                eprintln!("Warning: Could not start daemon: {}", e);
            }
        }
    }

    match &cli.command {
        Command::Init(args) => commands::init::run(cli, args),
        Command::Status(args) => commands::status::run(cli, args),
        Command::Convoy(cmd) => commands::convoy::run(cli, cmd),
        Command::Task(cmd) => commands::task::run(cli, cmd),
        Command::Context(cmd) => commands::context::run(cli, cmd),
        Command::Witness(cmd) => commands::witness::run(cli, cmd).await,
        Command::Refinery(cmd) => commands::refinery::run(cli, cmd).await,
        Command::Session(cmd) => commands::session::run(cli, cmd),
        Command::Lock(cmd) => commands::lock::run(cli, cmd),
        Command::Doctor(args) => commands::doctor::run(cli, args),
        Command::Api(args) => run_api_server(args).await,
        Command::Workflow(cmd) => commands::workflow::run(cli, cmd),
        Command::Mayor(cmd) => commands::mayor::run(cli, cmd).await,
        Command::Daemon(cmd) => commands::daemon::run(cli, cmd).await,
    }
}

async fn run_api_server(args: &cli::ApiArgs) -> Result<(), BratError> {
    let config = api::server::ServerConfig {
        host: args.host.clone(),
        port: args.port,
        cors_origin: args.cors_origin.clone(),
        idle_timeout_secs: if args.idle_timeout == 0 {
            None
        } else {
            Some(args.idle_timeout)
        },
    };

    api::run_server(config).await.map_err(|e| {
        BratError::Other(format!("API server error: {}", e))
    })
}
