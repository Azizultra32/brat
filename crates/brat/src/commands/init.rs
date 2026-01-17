use std::process::Command;

use libbrat_config::BratConfig;
use serde::Serialize;

use crate::cli::{Cli, InitArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Output of the init command.
#[derive(Debug, Serialize)]
pub struct InitOutput {
    /// Path to the repository root.
    pub repo_root: String,

    /// Path to the .brat directory.
    pub brat_dir: String,

    /// Path to the config file (if created).
    pub config_path: Option<String>,

    /// Whether grit was initialized.
    pub grit_initialized: bool,

    /// Actor ID from grit (if available).
    pub grit_actor_id: Option<String>,
}

/// Run the init command.
pub fn run(cli: &Cli, args: &InitArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Check if already initialized
    if ctx.is_initialized() && !args.no_config {
        return Err(BratError::AlreadyInitialized);
    }

    // Initialize grit if needed
    let (grit_initialized, grit_actor_id) = init_grit(&ctx)?;

    // Create .brat/config.toml unless --no-config
    let config_path = if !args.no_config {
        let config = BratConfig::default();
        config.save(&ctx.config_path)?;
        Some(ctx.config_path.display().to_string())
    } else {
        None
    };

    // TODO: Start bratd unless --no-daemon
    // TODO: Create tmux control room unless --no-tmux

    let output = InitOutput {
        repo_root: ctx.repo_root.display().to_string(),
        brat_dir: ctx.brat_dir.display().to_string(),
        config_path,
        grit_initialized,
        grit_actor_id,
    };

    if !cli.json {
        print_human(cli, &format!("Initialized brat in {}", ctx.repo_root.display()));
        if grit_initialized {
            if let Some(ref actor_id) = output.grit_actor_id {
                print_human(cli, &format!("Grit actor: {}", actor_id));
            }
        }
    }

    output_success(cli, output);
    Ok(())
}

/// Initialize grit in the repository.
///
/// Calls `grit init` as a subprocess and parses the output.
fn init_grit(ctx: &BratContext) -> Result<(bool, Option<String>), BratError> {
    // Check if grit is already initialized by looking for .git/grit/
    let grit_dir = ctx.git_dir.join("grit");
    if grit_dir.exists() {
        // Already initialized, try to get the actor ID
        let actor_id = get_grit_actor_id(ctx)?;
        return Ok((false, actor_id));
    }

    // Run grit init
    let output = Command::new("grit")
        .arg("init")
        .arg("--json")
        .current_dir(&ctx.repo_root)
        .output()
        .map_err(|e| BratError::GritInitFailed(format!("failed to run grit: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BratError::GritInitFailed(stderr.to_string()));
    }

    // Parse the JSON output to get the actor ID
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actor_id = parse_grit_init_output(&stdout);

    Ok((true, actor_id))
}

/// Get the current grit actor ID.
fn get_grit_actor_id(ctx: &BratContext) -> Result<Option<String>, BratError> {
    let output = Command::new("grit")
        .args(["actor", "current", "--json"])
        .current_dir(&ctx.repo_root)
        .output()
        .map_err(|e| BratError::GritCommandFailed(format!("failed to run grit: {}", e)))?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_grit_actor_output(&stdout))
}

/// Parse grit init JSON output to extract actor_id.
fn parse_grit_init_output(output: &str) -> Option<String> {
    // Try to parse as JSON and extract actor_id
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(data) = json.get("data") {
            if let Some(actor_id) = data.get("actor_id") {
                return actor_id.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Parse grit actor current JSON output to extract actor_id.
fn parse_grit_actor_output(output: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(data) = json.get("data") {
            if let Some(actor_id) = data.get("actor_id") {
                return actor_id.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}
