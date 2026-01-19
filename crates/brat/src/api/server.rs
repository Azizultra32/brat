//! Axum HTTP server setup.

use std::net::SocketAddr;

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing::info;

use super::routes;
use super::state::DaemonState;

/// Server configuration.
pub struct ServerConfig {
    /// Host to bind to.
    pub host: String,
    /// Port to listen on.
    pub port: u16,
    /// CORS allowed origin (None = allow all).
    pub cors_origin: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            cors_origin: None,
        }
    }
}

/// Build the Axum router with all routes.
pub fn build_router(state: DaemonState) -> Router {
    // Build CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .nest("/api/v1", routes::api_routes())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Run the HTTP server.
pub async fn run_server(config: ServerConfig) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "brat=info,tower_http=info".into()),
        )
        .init();

    // Create daemon state
    let state = DaemonState::new();

    // Build router
    let app = build_router(state);

    // Parse address
    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;

    info!("Starting bratd on http://{}", addr);

    // Run server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
