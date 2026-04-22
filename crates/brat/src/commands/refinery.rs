//! Refinery command handler.

use std::time::Duration;

use serde::Serialize;

use crate::cli::{Cli, RefineryCommand, RefineryRunArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};
use crate::workflows::{RefineryConfig, RefineryLoopResult, RefineryWorkflow};

/// Output of the refinery run command.
#[derive(Debug, Serialize)]
pub struct RefineryRunOutput {
    /// Number of iterations completed.
    pub iterations: usize,
    /// Total tasks queued for merge.
    pub total_queued: usize,
    /// Total merge attempts.
    pub total_attempted: usize,
    /// Total successful merges.
    pub total_succeeded: usize,
    /// Total failed merges.
    pub total_failed: usize,
    /// Total errors encountered.
    pub total_errors: usize,
}

/// Run the refinery command.
pub async fn run(cli: &Cli, cmd: &RefineryCommand) -> Result<(), BratError> {
    match cmd {
        RefineryCommand::Run(args) => run_refinery(cli, args).await,
    }
}

/// Run the refinery workflow.
async fn run_refinery(cli: &Cli, args: &RefineryRunArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require both brat and gritee to be initialized
    let config = ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    // Check if refinery role is enabled
    if !config.roles.refinery_enabled {
        return Err(BratError::RoleDisabled("refinery".to_string()));
    }

    // Build workflow config
    let refinery_config = RefineryConfig::from_brat_config(config);

    // Create GriteeClient
    let gritee = ctx.gritee_client();

    // Create workflow
    let mut workflow = RefineryWorkflow::new(refinery_config, gritee, ctx.repo_root.clone())?;

    if args.once {
        // Single iteration mode
        let result = workflow.run_once().await?;
        output_refinery_result(cli, &result);

        let output = RefineryRunOutput {
            iterations: 1,
            total_queued: result.queued,
            total_attempted: result.attempted,
            total_succeeded: result.succeeded,
            total_failed: result.failed,
            total_errors: result.errors.len(),
        };

        output_success(cli, output);
    } else {
        // Daemon mode
        if !cli.quiet && !cli.json {
            print_human(
                cli,
                &format!(
                    "Starting refinery daemon (poll interval: {}s)...",
                    args.poll_interval
                ),
            );
        }

        let poll_duration = Duration::from_secs(args.poll_interval);

        loop {
            match workflow.run_once().await {
                Ok(result) => {
                    if !cli.quiet && !cli.json {
                        output_refinery_result(cli, &result);
                    }
                }
                Err(e) => {
                    if !cli.quiet {
                        eprintln!("Refinery error: {}", e);
                    }
                }
            }

            tokio::time::sleep(poll_duration).await;
        }
    }

    Ok(())
}

/// Output refinery iteration result.
fn output_refinery_result(cli: &Cli, result: &RefineryLoopResult) {
    if cli.json {
        if let Ok(json) = serde_json::to_string(result) {
            println!("{}", json);
        }
    } else if !cli.quiet {
        println!(
            "Refinery: queued={}, attempted={}, succeeded={}, failed={}, errors={}",
            result.queued,
            result.attempted,
            result.succeeded,
            result.failed,
            result.errors.len()
        );

        for error in &result.errors {
            eprintln!("  Error: {}", error);
        }
    }
}
