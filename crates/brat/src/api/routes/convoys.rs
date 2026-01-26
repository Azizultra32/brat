//! Convoy CRUD endpoints.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::api::state::DaemonState;

use super::status::ErrorResponse;

/// Convoy response.
#[derive(Serialize)]
pub struct ConvoyResponse {
    pub convoy_id: String,
    pub grite_issue_id: String,
    pub title: String,
    pub body: String,
    pub status: String,
}

/// Request to create a convoy.
#[derive(Deserialize)]
pub struct CreateConvoyRequest {
    pub title: String,
    #[serde(default)]
    pub body: String,
}

/// GET /api/v1/repos/:repo_id/convoys
async fn list_convoys(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
) -> Result<Json<Vec<ConvoyResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let convoys = ctx.grite.convoy_list().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list convoys: {}", e),
            }),
        )
    })?;

    let responses: Vec<ConvoyResponse> = convoys
        .into_iter()
        .map(|c| ConvoyResponse {
            convoy_id: c.convoy_id,
            grite_issue_id: c.grite_issue_id,
            title: c.title,
            body: c.body,
            status: format!("{:?}", c.status).to_lowercase(),
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/v1/repos/:repo_id/convoys
async fn create_convoy(
    State(state): State<DaemonState>,
    Path(repo_id): Path<String>,
    Json(req): Json<CreateConvoyRequest>,
) -> Result<(StatusCode, Json<ConvoyResponse>), (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    let body = if req.body.is_empty() {
        None
    } else {
        Some(req.body.as_str())
    };

    let convoy = ctx.grite.convoy_create(&req.title, body).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to create convoy: {}", e),
            }),
        )
    })?;

    Ok((
        StatusCode::CREATED,
        Json(ConvoyResponse {
            convoy_id: convoy.convoy_id,
            grite_issue_id: convoy.grite_issue_id,
            title: convoy.title,
            body: convoy.body,
            status: format!("{:?}", convoy.status).to_lowercase(),
        }),
    ))
}

/// GET /api/v1/repos/:repo_id/convoys/:convoy_id
async fn get_convoy(
    State(state): State<DaemonState>,
    Path((repo_id, convoy_id)): Path<(String, String)>,
) -> Result<Json<ConvoyResponse>, (StatusCode, Json<ErrorResponse>)> {
    let ctx = state.get_repo(&repo_id).await.ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("Repository not found: {}", repo_id),
            }),
        )
    })?;

    // List convoys and find the one with matching ID
    let convoys = ctx.grite.convoy_list().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to list convoys: {}", e),
            }),
        )
    })?;

    let convoy = convoys
        .into_iter()
        .find(|c| c.convoy_id == convoy_id)
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: format!("Convoy not found: {}", convoy_id),
                }),
            )
        })?;

    Ok(Json(ConvoyResponse {
        convoy_id: convoy.convoy_id,
        grite_issue_id: convoy.grite_issue_id,
        title: convoy.title,
        body: convoy.body,
        status: format!("{:?}", convoy.status).to_lowercase(),
    }))
}

/// Build convoy routes.
pub fn routes() -> Router<DaemonState> {
    Router::new()
        .route("/convoys", get(list_convoys).post(create_convoy))
        .route("/convoys/:convoy_id", get(get_convoy))
}
