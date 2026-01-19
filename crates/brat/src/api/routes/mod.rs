//! API route definitions.

mod convoys;
mod health;
mod mayor;
mod repos;
mod sessions;
mod status;
mod tasks;

use axum::Router;

use crate::api::state::DaemonState;

/// Build all API routes.
pub fn api_routes() -> Router<DaemonState> {
    Router::new()
        .merge(health::routes())
        .merge(repos::routes())
}
