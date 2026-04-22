//! Session management endpoints.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use libbrat_engine::platform::{process_exists, send_term_signal, wait_for_process_exit};
use libbrat_gritee::SessionStatus;
use libbrat_session::read_session_logs;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

/// Session response.
#[derive(Serialize)]
pub struct SessionResponse {
    pub session_id: String,
    pub task_id: String,
    pub gritee_issue_id: String,
    pub engine: String,
    pub status: String,
    pub pid: Option<u32>,
    pub worktree: Option<String>,
    pub started_ts: i64,
    pub exit_code: Option<i32>,
    pub exit_reason: Option<String>,
}

/// Query parameters for listing sessions.
#[derive(Deserialize, Default)]
pub struct ListSessionsQuery {
    /// Filter by task ID.
    pub task: Option<String>,
}

/// Request to stop a session.
#[derive(Deserialize)]
pub struct StopSessionRequest {
    #[serde(default = "default_stop_reason")]
    pub reason: String,
}

fn default_stop_reason() -> String {
    "api-stop".to_string()
}

fn session_status_to_string(status: SessionStatus) -> String {
    match status {
        SessionStatus::Spawned => "spawned".to_string(),
        SessionStatus::Ready => "ready".to_string(),
        SessionStatus::Running => "running".to_string(),
        SessionStatus::Handoff => "handoff".to_string(),
        SessionStatus::Exit => "exit".to_string(),
    }
}

/// GET /api/v1/repos/:repo_id/sessions
async fn list_sessions(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Query(query): Query<ListSessionsQuery>,
) -> Result<Json<Vec<SessionResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let sessions = ctx
        .gritee
        .session_list(query.task.as_deref())
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to list sessions: {}", e),
                }),
            )
        })?;

    let responses: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            session_id: s.session_id,
            task_id: s.task_id,
            gritee_issue_id: s.gritee_issue_id,
            engine: s.engine,
            status: session_status_to_string(s.status),
            pid: s.pid,
            worktree: if s.worktree.is_empty() {
                None
            } else {
                Some(s.worktree)
            },
            started_ts: s.started_ts,
            exit_code: s.exit_code,
            exit_reason: s.exit_reason,
        })
        .collect();

    Ok(Json(responses))
}

/// GET /api/v1/repos/:repo_id/sessions/:session_id
async fn get_session(
    State(state): State<DaemonState>,
    Path((repo_id, session_id)): Path<(String, String)>,
) -> Result<Json<SessionResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // List sessions and find the one with matching ID
    let sessions = ctx.gritee.session_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list sessions: {}", e),
            }),
        )
    })?;

    let session = sessions
        .into_iter()
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", session_id),
                }),
            )
        })?;

    Ok(Json(SessionResponse {
        session_id: session.session_id,
        task_id: session.task_id,
        gritee_issue_id: session.gritee_issue_id,
        engine: session.engine,
        status: session_status_to_string(session.status),
        pid: session.pid,
        worktree: if session.worktree.is_empty() {
            None
        } else {
            Some(session.worktree)
        },
        started_ts: session.started_ts,
        exit_code: session.exit_code,
        exit_reason: session.exit_reason,
    }))
}

/// POST /api/v1/repos/:repo_id/sessions/:session_id/stop
async fn stop_session(
    State(state): State<DaemonState>,
    Path((repo_id, session_id)): Path<(String, String)>,
    Json(req): Json<StopSessionRequest>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let session = ctx.gritee.session_get(&session_id).map_err(|e| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Session not found: {}", e),
            }),
        )
    })?;

    if session.status == SessionStatus::Exit {
        return Ok(StatusCode::NO_CONTENT);
    }

    let mut exit_posted = false;
    let mut signal_sent = false;

    if let Some(pid) = session.pid {
        if process_exists(pid) {
            if let Err(e) = send_term_signal(pid) {
                if !process_exists(pid) {
                    exit_posted = true;
                } else {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ErrorResponse {
                            error: format!("Failed to signal session process: {}", e),
                        }),
                    ));
                }
            } else if wait_for_process_exit(pid, Duration::from_millis(ctx.config.engine.stop_timeout_ms)) {
                exit_posted = true;
            } else {
                ctx.gritee
                    .issue_comment(
                        &session.gritee_issue_id,
                        &format!("Stop requested for session `{}` (reason: {}).", session_id, req.reason),
                    )
                    .map_err(|e| {
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ErrorResponse {
                                error: format!("Failed to record stop request: {}", e),
                            }),
                        )
                    })?;
                signal_sent = true;
            }
        } else {
            exit_posted = true;
        }
    } else {
        exit_posted = true;
    }

    if exit_posted {
        // Reconcile sessions that are already dead or have no live process to
        // wait on. Live sessions will be marked exited by the monitor path.
        ctx.gritee
            .session_exit(&session_id, -1, &req.reason, session.last_output_ref.as_deref())
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse {
                        error: format!("Failed to stop session: {}", e),
                    }),
                )
            })?;
    }

    if signal_sent {
        Ok(StatusCode::ACCEPTED)
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

/// Query parameters for getting session logs.
#[derive(Deserialize, Default)]
pub struct SessionLogsQuery {
    /// Number of lines to return (default: 100).
    #[serde(default = "default_log_lines")]
    pub lines: usize,
}

fn default_log_lines() -> usize {
    100
}

/// Response with session logs.
#[derive(Serialize)]
pub struct SessionLogsResponse {
    /// Log lines.
    pub lines: Vec<String>,
    /// Whether there are more lines available.
    pub has_more: bool,
}

/// GET /api/v1/repos/:repo_id/sessions/:session_id/logs
async fn get_session_logs(
    State(state): State<DaemonState>,
    Path((repo_id, session_id)): Path<(String, String)>,
    Query(query): Query<SessionLogsQuery>,
) -> Result<Json<SessionLogsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // Find the session
    let sessions = ctx.gritee.session_list(None).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list sessions: {}", e),
            }),
        )
    })?;

    let session = sessions
        .into_iter()
        .find(|s| s.session_id == session_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Session not found: {}", session_id),
                }),
            )
        })?;

    // Check if there's log output
    let output_ref = session.last_output_ref.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "No logs available for this session".to_string(),
            }),
        )
    })?;

    // Read the logs using the canonical sha256/file contract, with raw blob refs
    // accepted as a compatibility path.
    let content = read_session_logs(&ctx.path, &session_id, &output_ref).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to read logs: {}", e),
            }),
        )
    })?;

    // Split into lines and take last N
    let all_lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let total_lines = all_lines.len();
    let start_idx = total_lines.saturating_sub(query.lines);
    let lines: Vec<String> = all_lines[start_idx..].to_vec();
    let has_more = start_idx > 0;

    Ok(Json(SessionLogsResponse { lines, has_more }))
}

/// Build session routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/:session_id", get(get_session))
        .route("/sessions/:session_id/stop", post(stop_session))
        .route("/sessions/:session_id/logs", get(get_session_logs))
}
