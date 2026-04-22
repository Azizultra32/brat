//! Session command handler.

use std::process::{Command as ProcessCommand, Stdio};
use std::time::Duration;

use libbrat_engine::platform::{
    configure_detached_process, process_exists, send_term_signal, wait_for_process_exit,
};
use libbrat_gritee::SessionStatus;
use libbrat_session::read_session_logs;
use serde::Serialize;

use crate::cli::{
    Cli, SessionCommand, SessionFinalizeStopArgs, SessionListArgs, SessionShowArgs,
    SessionStopArgs, SessionTailArgs,
};
use crate::context::BratContext;
use crate::error::BratError;
use crate::output::{output_success, print_human};

/// Session info for list/show output.
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    /// Session ID.
    pub session_id: String,
    /// Associated task ID.
    pub task_id: String,
    /// Role executing the session.
    pub role: String,
    /// Session type (polecat/crew).
    pub session_type: String,
    /// Engine name.
    pub engine: String,
    /// Session state.
    pub state: String,
    /// Timestamp when session started (millis since epoch).
    pub started_ts: i64,
    /// Last heartbeat timestamp (millis since epoch).
    pub last_heartbeat_ts: Option<i64>,
    /// Heartbeat age in milliseconds (computed).
    pub heartbeat_age_ms: Option<i64>,
    /// Path to worktree.
    pub worktree: String,
    /// Process ID.
    pub pid: Option<u32>,
}

/// Output for session list command.
#[derive(Debug, Serialize)]
pub struct SessionListOutput {
    /// List of active sessions.
    pub sessions: Vec<SessionInfo>,
    /// Total count.
    pub total: usize,
}

/// Output for session show command.
#[derive(Debug, Serialize)]
pub struct SessionShowOutput {
    /// Session details.
    pub session: SessionInfo,
}

/// Output for session stop command.
#[derive(Debug, Serialize)]
pub struct SessionStopOutput {
    /// Session ID that was stopped.
    pub session_id: String,
    /// Reason for stopping.
    pub reason: String,
    /// Whether exit was posted to Gritee.
    pub exit_posted: bool,
}

/// Output for session tail command.
#[derive(Debug, Serialize)]
pub struct SessionTailOutput {
    /// Session ID.
    pub session_id: String,
    /// Number of lines returned.
    pub lines_count: usize,
    /// The log lines.
    pub lines: Vec<String>,
    /// Whether there were more lines available.
    pub truncated: bool,
}

/// Run the session command.
pub fn run(cli: &Cli, cmd: &SessionCommand) -> Result<(), BratError> {
    match cmd {
        SessionCommand::List(args) => run_list(cli, args),
        SessionCommand::Show(args) => run_show(cli, args),
        SessionCommand::Stop(args) => run_stop(cli, args),
        SessionCommand::Tail(args) => run_tail(cli, args),
        SessionCommand::FinalizeStop(args) => run_finalize_stop(cli, args),
    }
}

/// Run the session list command.
fn run_list(cli: &Cli, args: &SessionListArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    // Get sessions, optionally filtered by task
    let sessions = client.session_list(args.task.as_deref())?;

    let session_infos: Vec<SessionInfo> = sessions
        .into_iter()
        .map(|s| {
            let heartbeat_age_ms = s.last_heartbeat_ts.map(|ts| now_ms - ts);
            SessionInfo {
                session_id: s.session_id,
                task_id: s.task_id,
                role: s.role.as_str().to_string(),
                session_type: s.session_type.as_str().to_string(),
                engine: s.engine,
                state: format!("{:?}", s.status).to_lowercase(),
                started_ts: s.started_ts,
                last_heartbeat_ts: s.last_heartbeat_ts,
                heartbeat_age_ms,
                worktree: s.worktree,
                pid: s.pid,
            }
        })
        .collect();

    let total = session_infos.len();

    if !cli.json && !cli.quiet {
        if session_infos.is_empty() {
            print_human(cli, "No active sessions");
        } else {
            println!("Active Sessions ({}):", total);
            for s in &session_infos {
                let heartbeat_str = match s.heartbeat_age_ms {
                    Some(age_ms) if age_ms < 60_000 => format!("{}s ago", age_ms / 1000),
                    Some(age_ms) => format!("{}m ago", age_ms / 60_000),
                    None => "never".to_string(),
                };
                println!(
                    "  {}  {}  {}/{}  {}  heartbeat {}",
                    s.session_id, s.task_id, s.role, s.session_type, s.state, heartbeat_str
                );
            }
        }
    }

    let output = SessionListOutput {
        sessions: session_infos,
        total,
    };

    output_success(cli, output);
    Ok(())
}

/// Run the session show command.
fn run_show(cli: &Cli, args: &SessionShowArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let session = client.session_get(&args.session_id)?;

    let heartbeat_age_ms = session.last_heartbeat_ts.map(|ts| now_ms - ts);
    let session_info = SessionInfo {
        session_id: session.session_id.clone(),
        task_id: session.task_id.clone(),
        role: session.role.as_str().to_string(),
        session_type: session.session_type.as_str().to_string(),
        engine: session.engine.clone(),
        state: format!("{:?}", session.status).to_lowercase(),
        started_ts: session.started_ts,
        last_heartbeat_ts: session.last_heartbeat_ts,
        heartbeat_age_ms,
        worktree: session.worktree.clone(),
        pid: session.pid,
    };

    if !cli.json && !cli.quiet {
        println!("Session: {}", session_info.session_id);
        println!("  Task:       {}", session_info.task_id);
        println!("  Role:       {}", session_info.role);
        println!("  Type:       {}", session_info.session_type);
        println!("  Engine:     {}", session_info.engine);
        println!("  State:      {}", session_info.state);
        println!("  Started:    {}", session_info.started_ts);
        if let Some(ts) = session_info.last_heartbeat_ts {
            let age_str = match session_info.heartbeat_age_ms {
                Some(age_ms) if age_ms < 60_000 => format!("{}s ago", age_ms / 1000),
                Some(age_ms) => format!("{}m ago", age_ms / 60_000),
                None => "unknown".to_string(),
            };
            println!("  Heartbeat:  {} ({})", ts, age_str);
        }
        if !session_info.worktree.is_empty() {
            println!("  Worktree:   {}", session_info.worktree);
        }
        if let Some(pid) = session_info.pid {
            println!("  PID:        {}", pid);
        }
    }

    let output = SessionShowOutput {
        session: session_info,
    };

    output_success(cli, output);
    Ok(())
}

/// Run the session stop command.
fn run_stop(cli: &Cli, args: &SessionStopArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;
    let config = ctx.require_initialized()?;
    let stop_timeout = Duration::from_millis(config.engine.stop_timeout_ms);
    let finalize_timeout_ms = normalize_finalize_timeout_ms(config.interventions.stale_session_ms);

    let client = ctx.gritee_client();
    let session = client.session_get(&args.session_id)?;
    let mut exit_posted = false;
    let mut signal_sent = false;

    if session.status != libbrat_gritee::SessionStatus::Exit {
        if let Some(pid) = session.pid {
            if process_exists(pid) {
                if let Err(e) = send_term_signal(pid) {
                    if !process_exists(pid) {
                        exit_posted = true;
                    } else {
                        return Err(BratError::GriteeCommandFailed(format!(
                            "failed to signal session process: {}",
                            e
                        )));
                    }
                } else {
                    signal_sent = true;
                    if wait_for_process_exit(pid, stop_timeout) {
                        exit_posted = true;
                        signal_sent = false;
                    } else {
                        client.issue_comment(
                            &session.gritee_issue_id,
                            &format!(
                                "Stop requested for session `{}` (reason: {}).",
                                args.session_id, args.reason
                            ),
                        )?;
                        spawn_stop_finalizer(
                            &ctx,
                            &args.session_id,
                            pid,
                            &args.reason,
                            finalize_timeout_ms,
                        )?;
                    }
                }
            } else {
                exit_posted = true;
            }
        } else {
            exit_posted = true;
        }

        if exit_posted {
            // Reconcile sessions that are already dead or have no live process
            // to wait on. Live sessions will be marked exited by the monitor.
            client.session_exit(
                &args.session_id,
                -1,
                &args.reason,
                session.last_output_ref.as_deref(),
            )?;
        }
    }

    if !cli.json && !cli.quiet {
        let message = if exit_posted {
            format!("Stopped session {} (reason: {})", args.session_id, args.reason)
        } else if signal_sent {
            format!(
                "Sent stop signal to session {} (reason: {})",
                args.session_id, args.reason
            )
        } else {
            format!("Session {} already exited", args.session_id)
        };
        print_human(cli, &message);
    }

    let output = SessionStopOutput {
        session_id: args.session_id.clone(),
        reason: args.reason.clone(),
        exit_posted,
    };

    output_success(cli, output);
    Ok(())
}

fn run_finalize_stop(cli: &Cli, args: &SessionFinalizeStopArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;
    let wait_timeout_ms = normalize_finalize_timeout_ms(args.wait_timeout_ms);

    if !wait_for_process_exit(args.pid, Duration::from_millis(wait_timeout_ms)) {
        spawn_stop_finalizer(
            &ctx,
            &args.session_id,
            args.pid,
            &args.reason,
            wait_timeout_ms,
        )?;
        return Ok(());
    }

    let client = ctx.gritee_client();
    let session = client.session_get(&args.session_id)?;
    if session.status == SessionStatus::Exit {
        return Ok(());
    }

    client.session_exit(
        &args.session_id,
        -1,
        &args.reason,
        session.last_output_ref.as_deref(),
    )?;

    Ok(())
}

fn spawn_stop_finalizer(
    ctx: &BratContext,
    session_id: &str,
    pid: u32,
    reason: &str,
    wait_timeout_ms: u64,
) -> Result<(), BratError> {
    let brat_bin = std::env::current_exe()
        .map_err(|e| BratError::Other(format!("failed to get current exe: {}", e)))?;

    let mut cmd = ProcessCommand::new(brat_bin);
    cmd.current_dir(&ctx.repo_root)
        .arg("--quiet")
        .arg("session")
        .arg("finalize-stop")
        .arg(session_id)
        .arg("--pid")
        .arg(pid.to_string())
        .arg("--reason")
        .arg(reason)
        .arg("--wait-timeout-ms")
        .arg(wait_timeout_ms.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    configure_detached_process(&mut cmd);

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| BratError::Other(format!("failed to spawn stop finalizer: {}", e)))
}

fn normalize_finalize_timeout_ms(timeout_ms: u64) -> u64 {
    timeout_ms.max(1_000)
}

/// Run the session tail command.
fn run_tail(cli: &Cli, args: &SessionTailArgs) -> Result<(), BratError> {
    let ctx = BratContext::resolve(cli)?;
    ctx.require_initialized()?;
    ctx.require_gritee_initialized()?;

    let client = ctx.gritee_client();

    // Get session to find log blob ref
    let session = client.session_get(&args.session_id)?;

    // Check if session has logs available
    let last_output_ref = match &session.last_output_ref {
        Some(ref_str) => ref_str.clone(),
        None => {
            print_human(cli, "No logs available for this session");
            let output = SessionTailOutput {
                session_id: args.session_id.clone(),
                lines_count: 0,
                lines: Vec::new(),
                truncated: false,
            };
            output_success(cli, output);
            return Ok(());
        }
    };

    // Read the logs using the canonical sha256/file contract, with raw blob refs
    // accepted as a compatibility path.
    let log_content = read_session_logs(&ctx.repo_root, &args.session_id, &last_output_ref)
        .map_err(BratError::GriteeCommandFailed)?;

    // Split into lines and get the last N
    let all_lines: Vec<&str> = log_content.lines().collect();
    let total_lines = all_lines.len();
    let truncated = total_lines > args.lines;
    let start = total_lines.saturating_sub(args.lines);
    let lines: Vec<String> = all_lines[start..].iter().map(|s| s.to_string()).collect();

    // Output lines
    if !cli.json && !cli.quiet {
        for line in &lines {
            println!("{}", line);
        }
    }

    // Follow mode
    if args.follow {
        run_tail_follow(cli, &ctx.repo_root, &args.session_id, &client)?;
    } else {
        let output = SessionTailOutput {
            session_id: args.session_id.clone(),
            lines_count: lines.len(),
            lines,
            truncated,
        };
        output_success(cli, output);
    }

    Ok(())
}

/// Follow mode for session tail - polls for new log content.
fn run_tail_follow(
    cli: &Cli,
    repo_root: &std::path::Path,
    session_id: &str,
    client: &libbrat_gritee::GriteeClient,
) -> Result<(), BratError> {
    let poll_interval = Duration::from_secs(1);
    let mut last_ref: Option<String> = None;
    let mut last_line_count: usize = 0;

    loop {
        // Get current session state
        let session = match client.session_get(session_id) {
            Ok(s) => s,
            Err(_) => {
                // Session might be gone, stop following
                if !cli.json && !cli.quiet {
                    println!("\n[session exited]");
                }
                break;
            }
        };

        // Check for new logs
        if let Some(ref ref_str) = session.last_output_ref {
            // If ref changed or this is first check, read new content
            if last_ref.as_ref() != Some(ref_str) || last_ref.is_none() {
                if let Ok(log_content) = read_session_logs(repo_root, session_id, ref_str) {
                    let lines: Vec<&str> = log_content.lines().collect();

                    // Output only new lines
                    for line in lines.iter().skip(last_line_count) {
                        if !cli.json {
                            println!("{}", line);
                        }
                    }

                    last_line_count = lines.len();
                    last_ref = Some(ref_str.clone());
                }
            }
        }

        // Check if session has exited
        if session.status == libbrat_gritee::SessionStatus::Exit {
            if !cli.json && !cli.quiet {
                println!("\n[session exited]");
            }
            break;
        }

        std::thread::sleep(poll_interval);
    }

    Ok(())
}
