//! Doctor command handler.

use serde::Serialize;

use crate::cli::{Cli, DoctorArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};
use crate::workflows::{ReconcileResult, ReconcileWorkflow};

/// Health check status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warn,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "PASS"),
            CheckStatus::Fail => write!(f, "FAIL"),
            CheckStatus::Warn => write!(f, "WARN"),
        }
    }
}

/// Individual health check result.
#[derive(Debug, Serialize)]
pub struct HealthCheck {
    /// Check name.
    pub name: String,
    /// Check status.
    pub status: CheckStatus,
    /// Status message.
    pub message: String,
    /// Remediation suggestion if not passing.
    pub remediation: Option<String>,
}

/// Output for doctor command.
#[derive(Debug, Serialize)]
pub struct DoctorOutput {
    /// Individual check results.
    pub checks: Vec<HealthCheck>,
    /// Number of passed checks.
    pub passed: usize,
    /// Number of failed checks.
    pub failed: usize,
    /// Number of warnings.
    pub warnings: usize,
    /// Overall status.
    pub overall_status: String,
}

/// Run the doctor command.
pub fn run(cli: &Cli, args: &DoctorArgs) -> Result<(), BratError> {
    // Default to --check if neither flag is provided
    if args.rebuild {
        run_rebuild(cli)
    } else {
        run_check(cli)
    }
}

/// Run health checks (read-only).
fn run_check(cli: &Cli) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;

    let mut checks = Vec::new();

    // Check 1: Git repository
    checks.push(check_git_repo(&ctx));

    // Check 2: Grit initialized
    checks.push(check_grit_initialized(&ctx));

    // Check 3: Brat initialized
    checks.push(check_brat_initialized(&ctx));

    // Check 4: Config valid
    checks.push(check_config_valid(&ctx));

    // Check 5: Worktree root exists (only if brat is initialized)
    if ctx.config.is_some() {
        checks.push(check_worktree_root(&ctx));
    }

    // Check 6: No stale sessions (only if grit is initialized)
    if ctx.is_grit_initialized() {
        checks.push(check_stale_sessions(&ctx));
    }

    // Calculate totals
    let passed = checks.iter().filter(|c| c.status == CheckStatus::Pass).count();
    let failed = checks.iter().filter(|c| c.status == CheckStatus::Fail).count();
    let warnings = checks.iter().filter(|c| c.status == CheckStatus::Warn).count();

    let overall_status = if failed > 0 {
        "unhealthy"
    } else if warnings > 0 {
        "warning"
    } else {
        "healthy"
    }
    .to_string();

    if !cli.json && !cli.quiet {
        println!("Health Check Results:");
        for check in &checks {
            let status_str = match check.status {
                CheckStatus::Pass => "[PASS]",
                CheckStatus::Fail => "[FAIL]",
                CheckStatus::Warn => "[WARN]",
            };
            println!("  {} {} - {}", status_str, check.name, check.message);
            if let Some(ref remediation) = check.remediation {
                println!("         Remediation: {}", remediation);
            }
        }
        println!();
        println!(
            "Overall: {} ({} passed, {} failed, {} warnings)",
            overall_status.to_uppercase(),
            passed,
            failed,
            warnings
        );
    }

    let output = DoctorOutput {
        checks,
        passed,
        failed,
        warnings,
        overall_status,
    };

    output_success(cli, output);
    Ok(())
}

/// Output for rebuild/reconciliation command.
#[derive(Debug, Serialize)]
pub struct RebuildOutput {
    /// Sessions checked.
    pub sessions_checked: usize,
    /// Sessions marked as crashed.
    pub sessions_marked_crashed: usize,
    /// Session IDs that were marked as crashed.
    pub crashed_session_ids: Vec<String>,
    /// Worktrees cleaned up.
    pub worktrees_cleaned: usize,
    /// Worktree session IDs that were cleaned.
    pub cleaned_worktree_ids: Vec<String>,
    /// Errors encountered.
    pub errors: Vec<String>,
    /// Overall status.
    pub overall_status: String,
}

impl From<ReconcileResult> for RebuildOutput {
    fn from(result: ReconcileResult) -> Self {
        let overall_status = if !result.errors.is_empty() {
            "partial".to_string()
        } else if result.had_actions() {
            "reconciled".to_string()
        } else {
            "clean".to_string()
        };

        Self {
            sessions_checked: result.sessions_checked,
            sessions_marked_crashed: result.sessions_marked_crashed,
            crashed_session_ids: result.crashed_session_ids,
            worktrees_cleaned: result.worktrees_cleaned,
            cleaned_worktree_ids: result.cleaned_worktree_ids,
            errors: result.errors,
            overall_status,
        }
    }
}

/// Rebuild harness state by reconciling sessions and cleaning up worktrees.
fn run_rebuild(cli: &Cli) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_grit_initialized()?;

    let grit = ctx.grit_client();
    let worktree_manager = ctx.worktree_manager().ok();
    let config = ctx
        .config
        .as_ref()
        .map(|c| c.interventions.clone())
        .unwrap_or_default();

    let workflow = ReconcileWorkflow::new(grit, worktree_manager, config);
    let result = workflow.run_once()?;

    if !cli.json && !cli.quiet {
        println!("Reconciliation Results:");
        println!("  Sessions checked:        {}", result.sessions_checked);
        println!(
            "  Sessions marked crashed: {}",
            result.sessions_marked_crashed
        );

        for session_id in &result.crashed_session_ids {
            println!("    - {}", session_id);
        }

        println!("  Worktrees cleaned:       {}", result.worktrees_cleaned);

        for worktree_id in &result.cleaned_worktree_ids {
            println!("    - {}", worktree_id);
        }

        if !result.errors.is_empty() {
            println!("\nErrors:");
            for error in &result.errors {
                println!("  - {}", error);
            }
        }

        println!();
        if result.had_actions() {
            print_human(
                cli,
                &format!(
                    "Overall: {} sessions recovered, {} worktrees cleaned",
                    result.sessions_marked_crashed, result.worktrees_cleaned
                ),
            );
        } else {
            print_human(cli, "Overall: No reconciliation needed (state is clean)");
        }
    }

    let output = RebuildOutput::from(result);
    output_success(cli, output);
    Ok(())
}

/// Check if we're in a git repository.
fn check_git_repo(ctx: &BratContext) -> HealthCheck {
    if ctx.git_dir.exists() {
        HealthCheck {
            name: "git_repository".to_string(),
            status: CheckStatus::Pass,
            message: format!("Git repository found at {}", ctx.repo_root.display()),
            remediation: None,
        }
    } else {
        HealthCheck {
            name: "git_repository".to_string(),
            status: CheckStatus::Fail,
            message: "Not in a git repository".to_string(),
            remediation: Some("Run `git init` to create a repository".to_string()),
        }
    }
}

/// Check if Grit is initialized.
fn check_grit_initialized(ctx: &BratContext) -> HealthCheck {
    if ctx.is_grit_initialized() {
        HealthCheck {
            name: "grit_initialized".to_string(),
            status: CheckStatus::Pass,
            message: "Grit ledger found".to_string(),
            remediation: None,
        }
    } else {
        HealthCheck {
            name: "grit_initialized".to_string(),
            status: CheckStatus::Fail,
            message: "Grit not initialized".to_string(),
            remediation: Some("Run `brat init` to initialize".to_string()),
        }
    }
}

/// Check if Brat is initialized.
fn check_brat_initialized(ctx: &BratContext) -> HealthCheck {
    if ctx.is_initialized() {
        HealthCheck {
            name: "brat_initialized".to_string(),
            status: CheckStatus::Pass,
            message: format!("Config found at {}", ctx.config_path.display()),
            remediation: None,
        }
    } else {
        HealthCheck {
            name: "brat_initialized".to_string(),
            status: CheckStatus::Fail,
            message: "Brat not initialized".to_string(),
            remediation: Some("Run `brat init` to initialize".to_string()),
        }
    }
}

/// Check if config is valid.
fn check_config_valid(ctx: &BratContext) -> HealthCheck {
    match &ctx.config {
        Some(config) => match config.validate() {
            Ok(()) => HealthCheck {
                name: "config_valid".to_string(),
                status: CheckStatus::Pass,
                message: "Configuration validated successfully".to_string(),
                remediation: None,
            },
            Err(e) => HealthCheck {
                name: "config_valid".to_string(),
                status: CheckStatus::Fail,
                message: format!("Configuration invalid: {}", e),
                remediation: Some("Fix the configuration in .brat/config.toml".to_string()),
            },
        },
        None => HealthCheck {
            name: "config_valid".to_string(),
            status: CheckStatus::Warn,
            message: "No configuration to validate".to_string(),
            remediation: Some("Run `brat init` to create configuration".to_string()),
        },
    }
}

/// Check if worktree root exists.
fn check_worktree_root(ctx: &BratContext) -> HealthCheck {
    if let Some(config) = &ctx.config {
        let worktree_root = ctx.repo_root.join(&config.swarm.worktree_root);
        if worktree_root.exists() {
            HealthCheck {
                name: "worktree_root_exists".to_string(),
                status: CheckStatus::Pass,
                message: format!("Worktree root exists at {}", worktree_root.display()),
                remediation: None,
            }
        } else {
            HealthCheck {
                name: "worktree_root_exists".to_string(),
                status: CheckStatus::Warn,
                message: format!("Worktree root does not exist: {}", worktree_root.display()),
                remediation: Some(format!(
                    "Create directory: mkdir -p {}",
                    worktree_root.display()
                )),
            }
        }
    } else {
        HealthCheck {
            name: "worktree_root_exists".to_string(),
            status: CheckStatus::Warn,
            message: "Cannot check worktree root without configuration".to_string(),
            remediation: None,
        }
    }
}

/// Check for stale sessions.
fn check_stale_sessions(ctx: &BratContext) -> HealthCheck {
    let client = ctx.grit_client();

    // Get intervention threshold
    let stale_threshold_ms = ctx
        .config
        .as_ref()
        .map(|c| c.interventions.stale_session_ms)
        .unwrap_or(300_000); // 5 minutes default

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    match client.session_list(None) {
        Ok(sessions) => {
            let stale_count = sessions
                .iter()
                .filter(|s| {
                    if let Some(heartbeat_ts) = s.last_heartbeat_ts {
                        let age_ms = now_ms - heartbeat_ts;
                        age_ms > stale_threshold_ms as i64
                    } else {
                        // No heartbeat yet - check if started too long ago
                        let age_ms = now_ms - s.started_ts;
                        age_ms > stale_threshold_ms as i64
                    }
                })
                .count();

            if stale_count == 0 {
                HealthCheck {
                    name: "no_stale_sessions".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("{} active session(s), none stale", sessions.len()),
                    remediation: None,
                }
            } else {
                HealthCheck {
                    name: "no_stale_sessions".to_string(),
                    status: CheckStatus::Warn,
                    message: format!(
                        "{} session(s) have stale heartbeats (>{}m)",
                        stale_count,
                        stale_threshold_ms / 60_000
                    ),
                    remediation: Some(
                        "Run `brat session list` to identify stale sessions".to_string(),
                    ),
                }
            }
        }
        Err(_) => HealthCheck {
            name: "no_stale_sessions".to_string(),
            status: CheckStatus::Warn,
            message: "Could not query sessions".to_string(),
            remediation: None,
        },
    }
}
