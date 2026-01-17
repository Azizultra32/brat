use libbrat_grit::TaskStatus;
use serde::Serialize;

use crate::cli::{Cli, TaskCommand, TaskCreateArgs, TaskUpdateArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Output of the task create command.
#[derive(Debug, Serialize)]
pub struct TaskCreateOutput {
    /// Brat task ID.
    pub task_id: String,

    /// Grit's internal issue ID.
    pub grit_issue_id: String,

    /// Parent convoy ID.
    pub convoy_id: String,

    /// Task title.
    pub title: String,

    /// Task status.
    pub status: String,
}

/// Output of the task update command.
#[derive(Debug, Serialize)]
pub struct TaskUpdateOutput {
    /// Task ID that was updated.
    pub task_id: String,

    /// New status.
    pub status: String,
}

/// Run the task command.
pub fn run(cli: &Cli, cmd: &TaskCommand) -> Result<(), BratError> {
    match cmd {
        TaskCommand::Create(args) => run_create(cli, args),
        TaskCommand::Update(args) => run_update(cli, args),
    }
}

/// Run the task create command.
fn run_create(cli: &Cli, args: &TaskCreateArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require both brat and grit to be initialized
    ctx.require_initialized()?;
    ctx.require_grit_initialized()?;

    let client = ctx.grit_client();
    let task = client.task_create(&args.convoy, &args.title, args.body.as_deref())?;

    let output = TaskCreateOutput {
        task_id: task.task_id.clone(),
        grit_issue_id: task.grit_issue_id,
        convoy_id: task.convoy_id,
        title: task.title,
        status: format!("{:?}", task.status).to_lowercase(),
    };

    if !cli.json {
        print_human(cli, &format!("Created task {}", task.task_id));
    }

    output_success(cli, output);
    Ok(())
}

/// Run the task update command.
fn run_update(cli: &Cli, args: &TaskUpdateArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    // Require both brat and grit to be initialized
    ctx.require_initialized()?;
    ctx.require_grit_initialized()?;

    // Parse the status argument
    let new_status = parse_task_status(&args.status)?;

    let client = ctx.grit_client();
    client.task_update_status_with_options(&args.task_id, new_status, args.force)?;

    let output = TaskUpdateOutput {
        task_id: args.task_id.clone(),
        status: format!("{:?}", new_status).to_lowercase(),
    };

    if !cli.json {
        let msg = if args.force {
            format!("Force-updated task {} to {}", args.task_id, args.status)
        } else {
            format!("Updated task {} to {}", args.task_id, args.status)
        };
        print_human(cli, &msg);
    }

    output_success(cli, output);
    Ok(())
}

/// Parse a status string into a TaskStatus.
fn parse_task_status(s: &str) -> Result<TaskStatus, BratError> {
    match s.to_lowercase().as_str() {
        "queued" => Ok(TaskStatus::Queued),
        "running" => Ok(TaskStatus::Running),
        "blocked" => Ok(TaskStatus::Blocked),
        "needs-review" | "needs_review" | "needsreview" => Ok(TaskStatus::NeedsReview),
        "merged" => Ok(TaskStatus::Merged),
        "dropped" => Ok(TaskStatus::Dropped),
        _ => Err(BratError::GritCommandFailed(format!(
            "invalid status '{}': expected one of queued, running, blocked, needs-review, merged, dropped",
            s
        ))),
    }
}
