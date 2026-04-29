//! Doctor command handler.

use std::process::Output;

use serde::{Deserialize, Serialize};

use crate::cli::{Cli, DoctorArgs};
use crate::context::BratContext;
use crate::error::BratError;
use crate::grite_cli::new_grite_command;
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

    // Check 2: Grite initialized
    checks.push(check_gritee_initialized(&ctx));

    // Check 3: Brat initialized
    checks.push(check_brat_initialized(&ctx));

    // Check 4: Grite local projection is accessible through CLI-only fallback.
    if ctx.is_gritee_initialized() {
        let projection_check = check_gritee_store_accessible(&ctx);
        let projection_accessible = projection_check.status == CheckStatus::Pass;
        checks.push(projection_check);

        // Check 5: Grite local projection maintenance signal.
        if projection_accessible {
            checks.push(check_gritee_db_maintenance(&ctx));
        }
    }

    // Check 6: Config valid
    checks.push(check_config_valid(&ctx));

    // Check 7: Worktree root exists (only if brat is initialized)
    if ctx.config.is_some() {
        checks.push(check_worktree_root(&ctx));
    }

    // Check 8: No stale sessions (only if gritee is initialized)
    if ctx.is_gritee_initialized() {
        checks.push(check_stale_sessions(&ctx));
    }

    // Calculate totals
    let passed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Pass)
        .count();
    let failed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .count();
    let warnings = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warn)
        .count();

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
    ctx.require_gritee_initialized()?;

    let gritee = ctx.gritee_client();
    let worktree_manager = ctx.worktree_manager().ok();
    let config = ctx
        .config
        .as_ref()
        .map(|c| c.interventions.clone())
        .unwrap_or_default();

    let workflow = ReconcileWorkflow::new(gritee, worktree_manager, config);
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

/// Check if Grite is initialized.
fn check_gritee_initialized(ctx: &BratContext) -> HealthCheck {
    if ctx.is_gritee_initialized() {
        HealthCheck {
            name: "gritee_initialized".to_string(),
            status: CheckStatus::Pass,
            message: "Grite ledger found".to_string(),
            remediation: None,
        }
    } else {
        HealthCheck {
            name: "gritee_initialized".to_string(),
            status: CheckStatus::Fail,
            message: "Grite not initialized".to_string(),
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

/// Check whether the local Grite projection can be read without relying on a daemon.
fn check_gritee_store_accessible(ctx: &BratContext) -> HealthCheck {
    match new_grite_command()
        .arg("--no-daemon")
        .arg("issue")
        .arg("list")
        .arg("--json")
        .env("GRITE_NO_DAEMON", "1")
        .current_dir(&ctx.repo_root)
        .output()
    {
        Ok(output) if output.status.success() => HealthCheck {
            name: "gritee_projection_accessible".to_string(),
            status: CheckStatus::Pass,
            message: "Grite projection is readable via CLI-only mode".to_string(),
            remediation: None,
        },
        Ok(output) => build_gritee_store_failure_check(command_output_excerpt(&output)),
        Err(e) => HealthCheck {
            name: "gritee_projection_accessible".to_string(),
            status: CheckStatus::Fail,
            message: format!("Could not run Grite CLI: {}", e),
            remediation: Some(
                "Install Grite or set BRAT_GRITE_BIN to the Grite executable, then rerun `brat doctor --check --json`."
                    .to_string(),
            ),
        },
    }
}

fn build_gritee_store_failure_check(detail: String) -> HealthCheck {
    let problem = classify_gritee_store_error(&detail);
    let status = match problem {
        GriteStoreProblem::Busy => CheckStatus::Warn,
        GriteStoreProblem::Corrupt => CheckStatus::Fail,
        GriteStoreProblem::Other => CheckStatus::Fail,
    };

    let message = match problem {
        GriteStoreProblem::Busy => format!("Grite projection is locked or busy: {}", detail),
        GriteStoreProblem::Corrupt => format!("Grite projection may be corrupt: {}", detail),
        GriteStoreProblem::Other => format!("Grite projection probe failed: {}", detail),
    };

    HealthCheck {
        name: "gritee_projection_accessible".to_string(),
        status,
        message,
        remediation: Some(gritee_recovery_ladder(problem).to_string()),
    }
}

/// Check whether Grite recommends local projection maintenance.
fn check_gritee_db_maintenance(ctx: &BratContext) -> HealthCheck {
    match new_grite_command()
        .arg("--no-daemon")
        .arg("db")
        .arg("stats")
        .arg("--json")
        .env("GRITE_NO_DAEMON", "1")
        .current_dir(&ctx.repo_root)
        .output()
    {
        Ok(output) if output.status.success() => parse_gritee_db_stats(&output.stdout)
            .map(build_gritee_db_maintenance_check)
            .unwrap_or_else(|e| HealthCheck {
                name: "gritee_db_maintenance".to_string(),
                status: CheckStatus::Warn,
                message: format!("Could not parse Grite DB stats: {}", e),
                remediation: Some(
                    "Run `grite --no-daemon db stats --json` and `grite doctor --json` directly."
                        .to_string(),
                ),
            }),
        Ok(output) => HealthCheck {
            name: "gritee_db_maintenance".to_string(),
            status: CheckStatus::Warn,
            message: format!(
                "Could not read Grite DB maintenance stats: {}",
                command_output_excerpt(&output)
            ),
            remediation: Some(
                "Run `grite --no-daemon db stats --json`; if it stays unavailable, follow the `gritee_projection_accessible` recovery ladder."
                    .to_string(),
            ),
        },
        Err(e) => HealthCheck {
            name: "gritee_db_maintenance".to_string(),
            status: CheckStatus::Warn,
            message: format!("Could not run Grite DB stats command: {}", e),
            remediation: Some(
                "Install Grite or set BRAT_GRITE_BIN to the Grite executable.".to_string(),
            ),
        },
    }
}

fn parse_gritee_db_stats(stdout: &[u8]) -> Result<GriteDbStats, String> {
    let response: GriteJsonResponse<GriteDbStats> =
        serde_json::from_slice(stdout).map_err(|e| e.to_string())?;
    if response.ok {
        response
            .data
            .ok_or_else(|| "missing data field in successful Grite response".to_string())
    } else {
        let detail = response
            .error
            .map(|e| {
                if let Some(code) = e.code {
                    format!("{}: {}", code, e.message)
                } else {
                    e.message
                }
            })
            .unwrap_or_else(|| "Grite returned ok=false without an error message".to_string());
        Err(detail)
    }
}

fn build_gritee_db_maintenance_check(stats: GriteDbStats) -> HealthCheck {
    let last_rebuild = stats
        .days_since_rebuild
        .map(|days| format!("last rebuild {}d ago", days))
        .unwrap_or_else(|| "last rebuild not recorded".to_string());
    let message = format!(
        "Grite DB stats: {}, {} events, {} issues, {} events since rebuild, {}",
        format_size_mib(stats.size_bytes),
        stats.event_count,
        stats.issue_count,
        stats.events_since_rebuild,
        last_rebuild
    );

    if stats.rebuild_recommended {
        HealthCheck {
            name: "gritee_db_maintenance".to_string(),
            status: CheckStatus::Warn,
            message,
            remediation: Some(
                "Run `grite doctor --fix --json` or `grite rebuild`, then rerun `brat --no-daemon doctor --check --json`."
                    .to_string(),
            ),
        }
    } else {
        HealthCheck {
            name: "gritee_db_maintenance".to_string(),
            status: CheckStatus::Pass,
            message,
            remediation: None,
        }
    }
}

fn format_size_mib(bytes: u64) -> String {
    format!("{:.1} MiB", bytes as f64 / 1024.0 / 1024.0)
}

#[derive(Debug, Deserialize)]
struct GriteJsonResponse<T> {
    #[serde(default)]
    ok: bool,
    data: Option<T>,
    error: Option<GriteJsonError>,
}

#[derive(Debug, Deserialize)]
struct GriteJsonError {
    code: Option<String>,
    message: String,
}

#[derive(Debug, Deserialize)]
struct GriteDbStats {
    #[serde(default)]
    size_bytes: u64,
    #[serde(default)]
    event_count: u64,
    #[serde(default)]
    issue_count: u64,
    #[serde(default)]
    events_since_rebuild: u64,
    #[serde(default)]
    days_since_rebuild: Option<u64>,
    #[serde(default)]
    rebuild_recommended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GriteStoreProblem {
    Busy,
    Corrupt,
    Other,
}

fn classify_gritee_store_error(detail: &str) -> GriteStoreProblem {
    let lower = detail.to_ascii_lowercase();
    if lower.contains("db_busy")
        || lower.contains("database busy")
        || lower.contains("database locked")
        || lower.contains("resource temporarily unavailable")
        || lower.contains("could not acquire lock")
        || lower.contains("wouldblock")
    {
        GriteStoreProblem::Busy
    } else if lower.contains("corrupt")
        || lower.contains("checksum")
        || lower.contains("invalid wal")
        || lower.contains("malformed")
    {
        GriteStoreProblem::Corrupt
    } else {
        GriteStoreProblem::Other
    }
}

fn gritee_recovery_ladder(problem: GriteStoreProblem) -> &'static str {
    match problem {
        GriteStoreProblem::Busy => {
            "Non-destructive recovery ladder: wait and retry; run `brat daemon status --json` and `grite daemon status --json`; stop stale daemons with `brat daemon stop` or `grite daemon stop`; retry with `brat --no-daemon doctor --check --json`; if the projection remains unusable, run `grite doctor --fix --json` or `grite rebuild`. Do not delete `.git/grite` or rewrite `refs/grite/*`."
        }
        GriteStoreProblem::Corrupt => {
            "Non-destructive recovery ladder: run `grite doctor --json` to inspect substrate state; run `grite doctor --fix --json` or `grite rebuild` for local projection repair; then run `brat --no-daemon doctor --check --json`. Preserve `refs/grite/*`; projection rebuilds must be local and monotonic."
        }
        GriteStoreProblem::Other => {
            "Recovery ladder: rerun with `brat --no-daemon doctor --check --json`; inspect `grite doctor --json`; use `grite doctor --fix --json` only for safe local repairs; then rerun Brat status. Preserve tracked files and `refs/grite/*`."
        }
    }
}

fn command_output_excerpt(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else if stdout.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        format!("{} {}", stdout.trim(), stderr.trim())
    };
    single_line_excerpt(&combined, 500)
}

fn single_line_excerpt(input: &str, max_len: usize) -> String {
    let excerpt = input.split_whitespace().collect::<Vec<_>>().join(" ");
    if excerpt.is_empty() {
        "(no output)".to_string()
    } else if excerpt.chars().count() <= max_len {
        excerpt
    } else {
        let suffix = "...";
        if max_len <= suffix.len() {
            suffix.chars().take(max_len).collect()
        } else {
            let prefix: String = excerpt.chars().take(max_len - suffix.len()).collect();
            format!("{}{}", prefix, suffix)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_db_busy_errors() {
        let detail = r#"{"error":{"code":"db_busy","message":"database busy: Database locked by another process: Resource temporarily unavailable"}}"#;

        assert_eq!(classify_gritee_store_error(detail), GriteStoreProblem::Busy);
    }

    #[test]
    fn classifies_projection_corruption_errors() {
        let detail = "sled checksum mismatch: corrupt local projection";

        assert_eq!(
            classify_gritee_store_error(detail),
            GriteStoreProblem::Corrupt
        );
    }

    #[test]
    fn unclassified_projection_failures_are_failures() {
        let check = build_gritee_store_failure_check("permission denied".to_string());

        assert_eq!(check.status, CheckStatus::Fail);
        assert!(check.message.contains("probe failed"));
    }

    #[test]
    fn excerpts_are_single_line_and_bounded() {
        let excerpt = single_line_excerpt("one\n\ntwo\tthree", 20);
        let bounded = single_line_excerpt("abcdefghijklmnopqrstuvwxyz", 10);
        let unicode = single_line_excerpt("alpha βeta gamma", 12);

        assert_eq!(excerpt, "one two three");
        assert_eq!(bounded, "abcdefg...");
        assert_eq!(unicode, "alpha βet...");
    }

    #[test]
    fn parses_db_stats_and_builds_pass_check() {
        let stdout = br#"{
            "schema_version": 1,
            "ok": true,
            "data": {
                "path": ".git/grite/sled",
                "size_bytes": 1048576,
                "event_count": 42,
                "issue_count": 7,
                "last_rebuild_ts": null,
                "events_since_rebuild": 42,
                "days_since_rebuild": null,
                "rebuild_recommended": false
            }
        }"#;

        let stats = parse_gritee_db_stats(stdout).expect("stats should parse");
        let check = build_gritee_db_maintenance_check(stats);

        assert_eq!(check.name, "gritee_db_maintenance");
        assert_eq!(check.status, CheckStatus::Pass);
        assert!(check.message.contains("1.0 MiB"));
        assert!(check.remediation.is_none());
    }

    #[test]
    fn db_stats_recommendation_warns() {
        let stats = GriteDbStats {
            size_bytes: 1024,
            event_count: 12_000,
            issue_count: 10,
            events_since_rebuild: 12_000,
            days_since_rebuild: Some(8),
            rebuild_recommended: true,
        };

        let check = build_gritee_db_maintenance_check(stats);

        assert_eq!(check.status, CheckStatus::Warn);
        assert!(check.remediation.unwrap().contains("grite rebuild"));
    }
}

/// Check for stale sessions.
fn check_stale_sessions(ctx: &BratContext) -> HealthCheck {
    let client = ctx.gritee_client();

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
