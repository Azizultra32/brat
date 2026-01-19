//! Mayor API endpoints.
//!
//! Provides HTTP endpoints for interacting with the AI Mayor orchestrator.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use libbrat_engine::{Engine, MayorEngine, SpawnSpec};
use serde::{Deserialize, Serialize};

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

/// Mayor status response.
#[derive(Serialize)]
pub struct MayorStatusResponse {
    /// Whether the Mayor is currently active.
    pub active: bool,
    /// Session ID if active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Request to start the Mayor.
#[derive(Deserialize, Default)]
pub struct StartMayorRequest {
    /// Optional initial message.
    pub message: Option<String>,
}

/// Response from starting the Mayor.
#[derive(Serialize)]
pub struct StartMayorResponse {
    /// Session ID.
    pub session_id: String,
    /// Initial response lines.
    pub response: Vec<String>,
}

/// Request to ask the Mayor a question.
#[derive(Deserialize)]
pub struct AskMayorRequest {
    /// Message to send.
    pub message: String,
}

/// Response from asking the Mayor.
#[derive(Serialize)]
pub struct AskMayorResponse {
    /// Response lines from the Mayor.
    pub response: Vec<String>,
}

/// Response from stopping the Mayor.
#[derive(Serialize)]
pub struct StopMayorResponse {
    /// Whether the stop was successful.
    pub success: bool,
}

/// Query parameters for getting Mayor history.
#[derive(Deserialize, Default)]
pub struct MayorHistoryQuery {
    /// Number of lines to return (default: 50).
    #[serde(default = "default_history_lines")]
    pub lines: usize,
}

fn default_history_lines() -> usize {
    50
}

/// Response with Mayor conversation history.
#[derive(Serialize)]
pub struct MayorHistoryResponse {
    /// Conversation history lines.
    pub lines: Vec<String>,
}

/// GET /api/v1/repos/:repo_id/mayor/status
async fn get_mayor_status(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<MayorStatusResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MayorEngine::new(ctx.path.clone());
    let active = engine.is_active();
    let session_id = engine.current_session_id();

    Ok(Json(MayorStatusResponse { active, session_id }))
}

/// POST /api/v1/repos/:repo_id/mayor/start
async fn start_mayor(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Json(req): Json<StartMayorRequest>,
) -> Result<(StatusCode, Json<StartMayorResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MayorEngine::new(ctx.path.clone());

    // Check if already active
    if engine.is_active() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Mayor session already active - stop it first".to_string(),
            }),
        ));
    }

    // Create spawn spec
    let spec = SpawnSpec::new(req.message.unwrap_or_default())
        .working_dir(ctx.path.clone());

    // Spawn the Mayor (this is async)
    let result = engine.spawn(spec).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to start Mayor: {}", e),
            }),
        )
    })?;

    // Get initial response from history
    let response = engine.tail(50).unwrap_or_default();

    Ok((
        StatusCode::CREATED,
        Json(StartMayorResponse {
            session_id: result.session_id,
            response,
        }),
    ))
}

/// POST /api/v1/repos/:repo_id/mayor/stop
async fn stop_mayor(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<StopMayorResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MayorEngine::new(ctx.path.clone());

    engine.stop_session().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to stop Mayor: {}", e),
            }),
        )
    })?;

    Ok(Json(StopMayorResponse { success: true }))
}

/// POST /api/v1/repos/:repo_id/mayor/ask
async fn ask_mayor(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Json(req): Json<AskMayorRequest>,
) -> Result<Json<AskMayorResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MayorEngine::new(ctx.path.clone());

    // Check if active
    if !engine.is_active() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Mayor not active - start it first".to_string(),
            }),
        ));
    }

    let response = engine.ask(&req.message).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to send message to Mayor: {}", e),
            }),
        )
    })?;

    Ok(Json(AskMayorResponse { response }))
}

/// GET /api/v1/repos/:repo_id/mayor/history
async fn get_mayor_history(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Query(query): Query<MayorHistoryQuery>,
) -> Result<Json<MayorHistoryResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let engine = MayorEngine::new(ctx.path.clone());

    let lines = engine.tail(query.lines).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to get Mayor history: {}", e),
            }),
        )
    })?;

    Ok(Json(MayorHistoryResponse { lines }))
}

/// Build Mayor routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/mayor/status", get(get_mayor_status))
        .route("/mayor/start", post(start_mayor))
        .route("/mayor/stop", post(stop_mayor))
        .route("/mayor/ask", post(ask_mayor))
        .route("/mayor/history", get(get_mayor_history))
}
