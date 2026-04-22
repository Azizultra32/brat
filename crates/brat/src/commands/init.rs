use std::path::Path;
use libbrat_config::BratConfig;
use serde::Serialize;

use crate::agents_md::BRAT_AGENTS_SECTION;
use crate::cli::{Cli, InitArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::grite_cli::new_grite_command;
use crate::output::{output_success, print_human};

/// Action taken for AGENTS.md
#[derive(Clone, Copy)]
enum AgentsMdAction {
    Created,
    Updated,
    Skipped,
    Disabled,
}

impl AgentsMdAction {
    fn as_str(&self) -> &'static str {
        match self {
            AgentsMdAction::Created => "created",
            AgentsMdAction::Updated => "updated",
            AgentsMdAction::Skipped => "skipped",
            AgentsMdAction::Disabled => "disabled",
        }
    }
}

/// Output of the init command.
#[derive(Debug, Serialize)]
pub struct InitOutput {
    /// Path to the repository root.
    pub repo_root: String,

    /// Path to the .brat directory.
    pub brat_dir: String,

    /// Path to the config file (if created).
    pub config_path: Option<String>,

    /// Whether gritee was initialized.
    pub gritee_initialized: bool,

    /// Actor ID from gritee (if available).
    pub gritee_actor_id: Option<String>,

    /// Action taken for AGENTS.md (created, updated, skipped, disabled).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_md_action: Option<String>,
}

/// Run the init command.
pub fn run(cli: &Cli, args: &InitArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Check if already initialized
    if ctx.is_initialized() && !args.no_config {
        return Err(BratError::AlreadyInitialized);
    }

    // Initialize gritee if needed (pass --no-agents-md if set)
    let (gritee_initialized, gritee_actor_id) = init_gritee(&ctx, args.no_agents_md)?;

    // Handle AGENTS.md (add brat section)
    let agents_md_action = if args.no_agents_md {
        AgentsMdAction::Disabled
    } else {
        handle_agents_md(&ctx.repo_root)?
    };

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
        gritee_initialized,
        gritee_actor_id,
        agents_md_action: Some(agents_md_action.as_str().to_string()),
    };

    if !cli.json {
        print_human(cli, &format!("Initialized brat in {}", ctx.repo_root.display()));
        if gritee_initialized {
            if let Some(ref actor_id) = output.gritee_actor_id {
                print_human(cli, &format!("Grite actor: {}", actor_id));
            }
        }
        // Print AGENTS.md status
        match agents_md_action {
            AgentsMdAction::Created => {
                print_human(cli, "Created AGENTS.md with brat instructions");
            }
            AgentsMdAction::Updated => {
                print_human(cli, "Updated AGENTS.md with brat section");
            }
            AgentsMdAction::Skipped => {
                print_human(cli, "AGENTS.md already contains brat section");
            }
            AgentsMdAction::Disabled => {}
        }
    }

    output_success(cli, output);
    Ok(())
}

/// Initialize gritee in the repository.
///
/// Calls `gritee init` as a subprocess and parses the output.
fn init_gritee(ctx: &BratContext, no_agents_md: bool) -> Result<(bool, Option<String>), BratError> {
    // Accept both current (.git/grite/) and legacy (.git/gritee/) layouts.
    if ctx.is_gritee_initialized() {
        // Already initialized, try to get the actor ID
        let actor_id = get_gritee_actor_id(ctx)?;
        return Ok((false, actor_id));
    }

    // Run gritee init
    let mut cmd = new_grite_command();
    cmd.arg("init").arg("--json");
    if no_agents_md {
        cmd.arg("--no-agents-md");
    }
    cmd.current_dir(&ctx.repo_root);

    let output = cmd
        .output()
        .map_err(|e| BratError::GriteeInitFailed(format!("failed to run gritee: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(BratError::GriteeInitFailed(stderr.to_string()));
    }

    // Parse the JSON output to get the actor ID
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actor_id = parse_gritee_init_output(&stdout);

    Ok((true, actor_id))
}

/// Get the current gritee actor ID.
fn get_gritee_actor_id(ctx: &BratContext) -> Result<Option<String>, BratError> {
    let output = new_grite_command()
        .args(["actor", "current", "--json"])
        .current_dir(&ctx.repo_root)
        .output()
        .map_err(|e| BratError::GriteeCommandFailed(format!("failed to run gritee: {}", e)))?;

    if !output.status.success() {
        return Ok(None);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_gritee_actor_output(&stdout))
}

/// Parse gritee init JSON output to extract actor_id.
fn parse_gritee_init_output(output: &str) -> Option<String> {
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

/// Parse gritee actor current JSON output to extract actor_id.
fn parse_gritee_actor_output(output: &str) -> Option<String> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(data) = json.get("data") {
            if let Some(actor_id) = data.get("actor_id") {
                return actor_id.as_str().map(|s| s.to_string());
            }
        }
    }
    None
}

/// Handle AGENTS.md - add brat section if not already present.
fn handle_agents_md(repo_root: &Path) -> Result<AgentsMdAction, BratError> {
    let agents_md_path = repo_root.join("AGENTS.md");

    if agents_md_path.exists() {
        let content = std::fs::read_to_string(&agents_md_path)
            .map_err(|e| BratError::Other(format!("failed to read AGENTS.md: {}", e)))?;

        if content.contains("## Brat") {
            return Ok(AgentsMdAction::Skipped);
        }

        // Append Brat section
        let updated = format!("{}\n\n{}", content, BRAT_AGENTS_SECTION);
        std::fs::write(&agents_md_path, updated)
            .map_err(|e| BratError::Other(format!("failed to update AGENTS.md: {}", e)))?;
        Ok(AgentsMdAction::Updated)
    } else {
        // Create new with Brat section
        std::fs::write(&agents_md_path, BRAT_AGENTS_SECTION)
            .map_err(|e| BratError::Other(format!("failed to create AGENTS.md: {}", e)))?;
        Ok(AgentsMdAction::Created)
    }
}
