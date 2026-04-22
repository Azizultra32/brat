//! Witness command handler.

use std::time::Duration;

use libbrat_engine::{
    AiderEngine, ClaudeCodeEngine, CodexEngine, ContinueEngine, CopilotEngine, Engine,
    GeminiEngine, OpenCodeEngine, ShellEngine,
};
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

    // Require both brat and gritee to be initialized
    let config = ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    // Check if witness role is enabled
    if !config.roles.witness_enabled {
        return Err(BratError::RoleDisabled("witness".to_string()));
    }

    // Reconcile stale sessions before starting (unless skipped)
    if !args.skip_reconcile {
        let gritee = ctx.gritee_client();
        let worktree_manager = ctx.worktree_manager().ok();
        let interventions_config = config.interventions.clone();

        let reconcile = ReconcileWorkflow::new(gritee, worktree_manager, interventions_config);
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
    let base_witness_config = WitnessConfig::from_brat_config(config);

    // Create GriteeClient and WorktreeManager
    let gritee = ctx.gritee_client();
    let worktree_manager = ctx.worktree_manager().ok();

    // Determine engine: CLI flag takes precedence over config
    let engine_name = args
        .engine
        .as_ref()
        .unwrap_or(&config.swarm.engine)
        .to_lowercase();

    // Create engine and run workflow
    match engine_name.as_str() {
        "codex" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using Codex engine");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "codex");
            let engine = CodexEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        "claude" | "claude-code" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using Claude Code engine");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "claude");
            let engine = ClaudeCodeEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        "opencode" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using OpenCode engine (open source Claude Code alternative)");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "opencode");
            let engine = OpenCodeEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        "aider" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using Aider engine");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "aider");
            let engine = AiderEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        "gemini" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using Gemini engine (Google's Gemini CLI)");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "gemini");
            let engine = GeminiEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        "copilot" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using GitHub Copilot CLI engine");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "copilot");
            let engine = CopilotEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        "continue" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using Continue.dev engine");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "continue");
            let engine = ContinueEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        "shell" => {
            if !cli.quiet && !cli.json {
                print_human(cli, "Using Shell engine");
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "shell");
            let engine = ShellEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
        _ => {
            if !cli.quiet && !cli.json {
                print_human(
                    cli,
                    &format!(
                        "Unknown engine '{}', falling back to Claude Code. \
                        Available: claude-code, codex, opencode, aider, gemini, copilot, continue, shell",
                        engine_name
                    ),
                );
            }
            let witness_config =
                resolved_witness_config(&base_witness_config, &config.swarm.engine, "claude");
            let engine = ClaudeCodeEngine::new();
            let workflow = WitnessWorkflow::new(witness_config, gritee, engine, worktree_manager);
            run_witness_loop(cli, args, workflow).await
        }
    }
}

fn resolved_witness_config(
    base: &WitnessConfig,
    configured_engine: &str,
    resolved_engine: &str,
) -> WitnessConfig {
    let mut config = base.clone();
    config.engine_command = resolved_engine.to_string();

    if canonical_engine_name(configured_engine) != canonical_engine_name(resolved_engine) {
        config.engine_args.clear();
    }

    config
}

fn canonical_engine_name(engine: &str) -> String {
    match engine.to_lowercase().as_str() {
        "claude-code" => "claude".to_string(),
        other => other.to_string(),
    }
}

/// Run the witness workflow loop (shared implementation for any engine).
async fn run_witness_loop<E: Engine + 'static>(
    cli: &Cli,
    args: &WitnessRunArgs,
    mut workflow: WitnessWorkflow<E>,
) -> Result<(), BratError> {
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
