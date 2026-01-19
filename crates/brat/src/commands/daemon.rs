//! Daemon management commands.

use crate::api;
use crate::cli::{
    Cli, DaemonCommand, DaemonLogsArgs, DaemonRestartArgs, DaemonStartArgs, DaemonStatusArgs,
    DaemonStopArgs,
};
use crate::daemon::DaemonManager;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Run a daemon subcommand.
pub async fn run(cli: &Cli, cmd: &DaemonCommand) -> Result<(), BratError> {
    match cmd {
        DaemonCommand::Start(args) => run_start(cli, args).await,
        DaemonCommand::Stop(args) => run_stop(cli, args),
        DaemonCommand::Status(args) => run_status(cli, args),
        DaemonCommand::Restart(args) => run_restart(cli, args).await,
        DaemonCommand::Logs(args) => run_logs(cli, args),
    }
}

/// Start the daemon.
async fn run_start(cli: &Cli, args: &DaemonStartArgs) -> Result<(), BratError> {
    if args.foreground {
        // Run in foreground (same as `brat api`)
        let config = api::server::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: args.port,
            cors_origin: None,
            idle_timeout_secs: if args.idle_timeout == 0 {
                None
            } else {
                Some(args.idle_timeout)
            },
        };

        api::run_server(config)
            .await
            .map_err(|e| BratError::Other(format!("Daemon error: {}", e)))
    } else {
        // Start in background
        let manager = DaemonManager::with_config(args.port, args.idle_timeout);

        if manager.is_running() {
            let status = manager.status();
            if cli.json {
                output_success(
                    cli,
                    &serde_json::json!({
                        "status": "already_running",
                        "pid": status.pid,
                        "url": status.url,
                    }),
                );
            } else {
                print_human(
                    cli,
                    &format!(
                        "Daemon is already running (PID: {}, URL: {})",
                        status.pid.unwrap_or(0),
                        status.url
                    ),
                );
            }
            return Ok(());
        }

        manager.start()?;

        let status = manager.status();
        if cli.json {
            output_success(
                cli,
                &serde_json::json!({
                    "status": "started",
                    "pid": status.pid,
                    "url": status.url,
                }),
            );
        } else {
            print_human(
                cli,
                &format!(
                    "Daemon started (PID: {}, URL: {})",
                    status.pid.unwrap_or(0),
                    status.url
                ),
            );
        }

        Ok(())
    }
}

/// Stop the daemon.
fn run_stop(cli: &Cli, _args: &DaemonStopArgs) -> Result<(), BratError> {
    let manager = DaemonManager::new();

    if !manager.is_running() {
        if cli.json {
            output_success(
                cli,
                &serde_json::json!({
                    "status": "not_running",
                }),
            );
        } else {
            print_human(cli, "Daemon is not running");
        }
        return Ok(());
    }

    manager.stop()?;

    if cli.json {
        output_success(
            cli,
            &serde_json::json!({
                "status": "stopped",
            }),
        );
    } else {
        print_human(cli, "Daemon stopped");
    }

    Ok(())
}

/// Show daemon status.
fn run_status(cli: &Cli, _args: &DaemonStatusArgs) -> Result<(), BratError> {
    let manager = DaemonManager::new();
    let status = manager.status();

    if cli.json {
        output_success(
            cli,
            &serde_json::json!({
                "running": status.running,
                "pid": status.pid,
                "port": status.port,
                "url": status.url,
            }),
        );
    } else if status.running {
        print_human(
            cli,
            &format!(
                "Daemon is running\n  PID:  {}\n  URL:  {}",
                status.pid.unwrap_or(0),
                status.url
            ),
        );
    } else {
        print_human(cli, "Daemon is not running");
    }

    Ok(())
}

/// Restart the daemon.
async fn run_restart(cli: &Cli, args: &DaemonRestartArgs) -> Result<(), BratError> {
    let manager = DaemonManager::with_config(args.port, args.idle_timeout);

    // Stop if running
    if manager.is_running() {
        manager.stop()?;
        if !cli.quiet {
            print_human(cli, "Stopped existing daemon");
        }
    }

    // Start
    manager.start()?;

    let status = manager.status();
    if cli.json {
        output_success(
            cli,
            &serde_json::json!({
                "status": "restarted",
                "pid": status.pid,
                "url": status.url,
            }),
        );
    } else {
        print_human(
            cli,
            &format!(
                "Daemon restarted (PID: {}, URL: {})",
                status.pid.unwrap_or(0),
                status.url
            ),
        );
    }

    Ok(())
}

/// Show daemon logs.
fn run_logs(cli: &Cli, args: &DaemonLogsArgs) -> Result<(), BratError> {
    let manager = DaemonManager::new();
    let logs = manager.tail_logs(args.lines)?;

    if cli.json {
        output_success(
            cli,
            &serde_json::json!({
                "lines": logs,
            }),
        );
    } else if logs.is_empty() {
        print_human(cli, "No logs available");
    } else {
        for line in logs {
            println!("{}", line);
        }
    }

    Ok(())
}
