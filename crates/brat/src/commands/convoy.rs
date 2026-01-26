use serde::Serialize;

use crate::cli::{Cli, ConvoyCommand, ConvoyCreateArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Output of the convoy create command.
#[derive(Debug, Serialize)]
pub struct ConvoyCreateOutput {
    /// Brat convoy ID.
    pub convoy_id: String,

    /// Grite's internal issue ID.
    pub gritee_issue_id: String,

    /// Convoy title.
    pub title: String,

    /// Convoy status.
    pub status: String,
}

/// Run the convoy command.
pub fn run(cli: &Cli, cmd: &ConvoyCommand) -> Result<(), BratError> {
    match cmd {
        ConvoyCommand::Create(args) => run_create(cli, args),
    }
}

/// Run the convoy create command.
fn run_create(cli: &Cli, args: &ConvoyCreateArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require both brat and gritee to be initialized
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let convoy = client.convoy_create(&args.title, args.body.as_deref())?;

    let output = ConvoyCreateOutput {
        convoy_id: convoy.convoy_id.clone(),
        gritee_issue_id: convoy.gritee_issue_id,
        title: convoy.title,
        status: format!("{:?}", convoy.status).to_lowercase(),
    };

    if !cli.json {
        print_human(cli, &format!("Created convoy {}", convoy.convoy_id));
    }

    output_success(cli, output);
    Ok(())
}
