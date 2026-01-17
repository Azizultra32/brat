//! Witness command handler.

use std::time::Duration;

use libbrat_engine::ShellEngine;
use serde::Serialize;

use crate::cli::{Cli, WitnessCommand, WitnessRunArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};
use crate::workflows::{ReconcileWorkflow, WitnessConfig, WitnessLoopResult, WitnessWorkflow};

/// Output of the witness run command.
#[derive(Debug, Serialize)]
pub struct WitnessRunOutput {
    /// Number of iterations completed.
    pub iterations: usize,
    /// Total tasks found.
    pub total_tasks_found: usize,
    /// Total sessions spawned.
    pub total_sessions_spawned: usize,
    /// Total errors encountered.
    pub total_errors: usize,
}

/// Run the witness command.
pub async fn run(cli: &Cli, cmd: &WitnessCommand) -> Result<(), BratError> {
    match cmd {
        WitnessCommand::Run(args) => run_witness(cli, args).await,
    }
}

/// Run the witness workflow.
async fn run_witness(cli: &Cli, args: &WitnessRunArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require both brat and grit to be initialized
    let config = ctx.require_initialized()?;
    ctx.require_grit_initialized()?;

    // Check if witness role is enabled
    if !config.roles.witness_enabled {
        return Err(BratError::RoleDisabled("witness".to_string()));
    }

    // Reconcile stale sessions before starting (unless skipped)
    if !args.skip_reconcile {
        let grit = ctx.grit_client();
        let worktree_manager = ctx.worktree_manager().ok();
        let interventions_config = config.interventions.clone();

        let reconcile = ReconcileWorkflow::new(grit, worktree_manager, interventions_config);
        match reconcile.run_once() {
            Ok(result) => {
                if result.had_actions() && !cli.quiet && !cli.json {
                    print_human(
                        cli,
                        &format!(
                            "Reconciled {} crashed session(s), cleaned {} worktree(s)",
                            result.sessions_marked_crashed, result.worktrees_cleaned
                        ),
                    );
                }
            }
            Err(e) => {
                if !cli.quiet {
                    eprintln!("Warning: Reconciliation failed: {}", e);
                }
            }
        }
    }

    // Build workflow config
    let witness_config = WitnessConfig::from_brat_config(config);

    // Create engine (MVP: always use ShellEngine)
    let engine = ShellEngine::new();

    // Create GritClient and WorktreeManager
    let grit = ctx.grit_client();
    let worktree_manager = ctx.worktree_manager().ok();

    // Create workflow
    let mut workflow = WitnessWorkflow::new(witness_config, grit, engine, worktree_manager);

    if args.once {
        // Single iteration mode
        let result = workflow.run_once().await?;
        output_witness_result(cli, &result);

        let output = WitnessRunOutput {
            iterations: 1,
            total_tasks_found: result.tasks_found,
            total_sessions_spawned: result.sessions_spawned,
            total_errors: result.errors.len(),
        };

        output_success(cli, output);
    } else {
        // Daemon mode
        if !cli.quiet && !cli.json {
            print_human(
                cli,
                &format!(
                    "Starting witness daemon (poll interval: {}s)...",
                    args.poll_interval
                ),
            );
        }

        let poll_duration = Duration::from_secs(args.poll_interval);

        loop {
            match workflow.run_once().await {
                Ok(result) => {
                    if !cli.quiet && !cli.json {
                        output_witness_result(cli, &result);
                    }
                }
                Err(e) => {
                    if !cli.quiet {
                        eprintln!("Witness error: {}", e);
                    }
                }
            }

            tokio::time::sleep(poll_duration).await;
        }
    }

    Ok(())
}

/// Output witness iteration result.
fn output_witness_result(cli: &Cli, result: &WitnessLoopResult) {
    if cli.json {
        if let Ok(json) = serde_json::to_string(result) {
            println!("{}", json);
        }
    } else if !cli.quiet {
        println!(
            "Witness: tasks={}, active={}, spawned={}, errors={}",
            result.tasks_found,
            result.sessions_active,
            result.sessions_spawned,
            result.errors.len()
        );

        for error in &result.errors {
            eprintln!("  Error: {}", error);
        }
    }
}
