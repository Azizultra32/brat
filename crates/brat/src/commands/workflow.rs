//! Workflow command handlers.

use std::collections::HashMap;

use libbrat_workflow::{WorkflowExecutor, WorkflowParser};

use crate::cli::{Cli, WorkflowCommand, WorkflowListArgs, WorkflowRunArgs, WorkflowShowArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Run a workflow subcommand.
pub fn run(cli: &Cli, cmd: &WorkflowCommand) -> Result<(), BratError> {
    match cmd {
        WorkflowCommand::List(args) => run_list(cli, args),
        WorkflowCommand::Show(args) => run_show(cli, args),
        WorkflowCommand::Run(args) => run_run(cli, args),
    }
}

/// List available workflows.
fn run_list(cli: &Cli, _args: &WorkflowListArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    let parser = WorkflowParser::from_repo_root(&ctx.repo_root);

    let workflows = parser
        .list_workflows()
        .map_err(|e| BratError::Other(format!("failed to list workflows: {}", e)))?;

    if cli.json {
        #[derive(serde::Serialize)]
        struct Output {
            workflows: Vec<String>,
            workflows_dir: String,
        }
        output_success(
            cli,
            Output {
                workflows,
                workflows_dir: parser.workflows_dir().to_string_lossy().to_string(),
            },
        );
    } else {
        if workflows.is_empty() {
            print_human(cli, "No workflows found.");
            print_human(
                cli,
                &format!(
                    "Create workflow files in: {}",
                    parser.workflows_dir().display()
                ),
            );
        } else {
            print_human(cli, "Available workflows:");
            for name in &workflows {
                print_human(cli, &format!("  - {}", name));
            }
        }
    }

    Ok(())
}

/// Show workflow details.
fn run_show(cli: &Cli, args: &WorkflowShowArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    let parser = WorkflowParser::from_repo_root(&ctx.repo_root);

    let template = parser
        .load(&args.name)
        .map_err(|e| BratError::Other(format!("failed to load workflow: {}", e)))?;

    if cli.json {
        output_success(cli, &template);
    } else {
        print_human(cli, &format!("Workflow: {}", template.name));
        print_human(cli, &format!("Version: {}", template.version));
        print_human(
            cli,
            &format!("Type: {:?}", template.workflow_type).to_lowercase(),
        );
        if let Some(ref desc) = template.description {
            print_human(cli, &format!("Description: {}", desc));
        }

        if !template.inputs.is_empty() {
            print_human(cli, "\nInputs:");
            for (name, spec) in &template.inputs {
                let required = if spec.required { " (required)" } else { "" };
                let default = spec
                    .default
                    .as_ref()
                    .map(|d| format!(" [default: {}]", d))
                    .unwrap_or_default();
                let desc = spec
                    .description
                    .as_ref()
                    .map(|d| format!(" - {}", d))
                    .unwrap_or_default();
                print_human(cli, &format!("  {}{}{}{}", name, required, default, desc));
            }
        }

        match template.workflow_type {
            libbrat_workflow::WorkflowType::Workflow => {
                if !template.steps.is_empty() {
                    print_human(cli, "\nSteps:");
                    for step in &template.steps {
                        let deps = if step.needs.is_empty() {
                            String::new()
                        } else {
                            format!(" (needs: {})", step.needs.join(", "))
                        };
                        print_human(cli, &format!("  {} - {}{}", step.id, step.title, deps));
                    }
                }
            }
            libbrat_workflow::WorkflowType::Convoy => {
                if !template.legs.is_empty() {
                    print_human(cli, "\nLegs (parallel):");
                    for leg in &template.legs {
                        print_human(cli, &format!("  {} - {}", leg.id, leg.title));
                    }
                }
                if let Some(ref synthesis) = template.synthesis {
                    print_human(cli, &format!("\nSynthesis: {}", synthesis.title));
                }
            }
        }
    }

    Ok(())
}

/// Run a workflow.
fn run_run(cli: &Cli, args: &WorkflowRunArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let parser = WorkflowParser::from_repo_root(&ctx.repo_root);
    let gritee = ctx.gritee_client();
    let executor = WorkflowExecutor::new(gritee);

    // Load the workflow template
    let template = parser
        .load(&args.name)
        .map_err(|e| BratError::Other(format!("failed to load workflow: {}", e)))?;

    // Build variables map from CLI args
    let vars: HashMap<String, String> = args.vars.iter().cloned().collect();

    // Execute the workflow
    let instance = executor
        .execute(&template, vars)
        .map_err(|e| BratError::Other(format!("failed to execute workflow: {}", e)))?;

    if cli.json {
        output_success(cli, &instance);
    } else {
        print_human(cli, &format!("Workflow '{}' executed successfully!", args.name));
        print_human(cli, &format!("Instance ID: {}", instance.instance_id));
        print_human(cli, &format!("Convoy ID: {}", instance.convoy_id));
        print_human(cli, &format!("Tasks created: {}", instance.task_ids.len()));
        for task_id in &instance.task_ids {
            print_human(cli, &format!("  - {}", task_id));
        }
    }

    Ok(())
}
