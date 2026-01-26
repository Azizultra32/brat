//! Internal event broadcast endpoint.
//!
//! This endpoint allows workflows (running as separate processes) to
//! broadcast events to connected WebSocket clients.

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Deserialize;

use crate::api::state::{BratEvent, DaemonState};

/// Build event routes.
pub fn routes() -> Router<DaemonState> {
    Router::new().route("/internal/broadcast", post(broadcast_event))
}

/// Request body for broadcasting an event.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum BroadcastEventRequest {
    TaskUpdated {
        task_id: String,
        status: String,
        convoy_id: Option<String>,
    },
    SessionStarted {
        session_id: String,
        task_id: String,
        engine: String,
    },
    SessionExited {
        session_id: String,
        task_id: String,
        exit_code: i32,
    },
    MergeCompleted {
        task_id: String,
        commit_sha: String,
        branch: String,
    },
    MergeFailed {
        task_id: String,
        error: String,
        attempt: u32,
    },
    MergeRolledBack {
        task_id: String,
        reset_sha: String,
        reason: String,
    },
    MergeRetryScheduled {
        task_id: String,
        retry_at: String,
        attempt: u32,
    },
}

impl From<BroadcastEventRequest> for BratEvent {
    fn from(req: BroadcastEventRequest) -> Self {
        match req {
            BroadcastEventRequest::TaskUpdated {
                task_id,
                status,
                convoy_id,
            } => BratEvent::TaskUpdated {
                task_id,
                status,
                convoy_id,
            },
            BroadcastEventRequest::SessionStarted {
                session_id,
                task_id,
                engine,
            } => BratEvent::SessionStarted {
                session_id,
                task_id,
                engine,
            },
            BroadcastEventRequest::SessionExited {
                session_id,
                task_id,
                exit_code,
            } => BratEvent::SessionExited {
                session_id,
                task_id,
                exit_code,
            },
            BroadcastEventRequest::MergeCompleted {
                task_id,
                commit_sha,
                branch,
            } => BratEvent::MergeCompleted {
                task_id,
                commit_sha,
                branch,
            },
            BroadcastEventRequest::MergeFailed {
                task_id,
                error,
                attempt,
            } => BratEvent::MergeFailed {
                task_id,
                error,
                attempt,
            },
            BroadcastEventRequest::MergeRolledBack {
                task_id,
                reset_sha,
                reason,
            } => BratEvent::MergeRolledBack {
                task_id,
                reset_sha,
                reason,
            },
            BroadcastEventRequest::MergeRetryScheduled {
                task_id,
                retry_at,
                attempt,
            } => BratEvent::MergeRetryScheduled {
                task_id,
                retry_at,
                attempt,
            },
        }
    }
}

/// Broadcast an event to all connected WebSocket clients.
async fn broadcast_event(
    State(state): State<DaemonState>,
    Json(request): Json<BroadcastEventRequest>,
) -> StatusCode {
    let event: BratEvent = request.into();
    state.broadcast(event);
    StatusCode::ACCEPTED
}
