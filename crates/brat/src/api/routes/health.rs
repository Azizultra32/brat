//! Health check endpoint.

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::api::state::DaemonState;

/// Health check response.
#[derive(Serialize)]
pub struct HealthResponse {
    /// Whether the daemon is healthy.
    pub ok: bool,
    /// Daemon version.
    pub version: String,
    /// Uptime in seconds.
    pub uptime_secs: u64,
    /// Number of registered repositories.
    pub repos_count: usize,
}

/// GET /api/v1/health
async fn health(State(state): State<DaemonState>) -> Json<HealthResponse> {
    let repos = state.repos.read().await;
    Json(HealthResponse {
        ok: true,
        version: state.version.clone(),
        uptime_secs: state.uptime_secs(),
        repos_count: repos.len(),
    })
}

/// Build health routes.
pub fn routes() -> Router<DaemonState> {
    Router::new().route("/health", get(health))
}
