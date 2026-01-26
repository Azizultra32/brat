//! API route definitions.

mod convoys;
mod events;
mod health;
mod mayor;
mod repos;
mod sessions;
mod status;
mod tasks;
mod websocket;

use axum::Router;

use crate::api::state::DaemonState;

/// Build all API routes.
pub fn api_routes() -> Router<DaemonState> {
    Router::new()
        .merge(health::routes())
        .merge(repos::routes())
        .merge(websocket::routes())
        .merge(events::routes())
}
