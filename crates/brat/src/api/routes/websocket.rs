//! WebSocket handler for real-time event streaming.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use tracing::{debug, warn};

use crate::api::state::DaemonState;

/// Build WebSocket routes.
pub fn routes() -> Router<DaemonState> {
    Router::new().route("/ws", get(ws_handler))
}

/// WebSocket upgrade handler.
async fn ws_handler(
    State(state): State<DaemonState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle an individual WebSocket connection.
async fn handle_socket(socket: WebSocket, state: DaemonState) {
    debug!("WebSocket client connected");

    // Split the socket into sender and receiver
    let (mut sender, mut receiver) = socket.split();

    // Subscribe to event broadcast channel
    let mut rx = state.subscribe_events();

    loop {
        tokio::select! {
            // Forward events to the WebSocket client
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        match serde_json::to_string(&event) {
                            Ok(msg) => {
                                if sender.send(Message::Text(msg.into())).await.is_err() {
                                    debug!("WebSocket client disconnected (send failed)");
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Failed to serialize event: {}", e);
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        warn!("WebSocket client lagged, skipped {} events", n);
                        // Continue receiving - client will get next events
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        debug!("Event channel closed, terminating WebSocket");
                        break;
                    }
                }
            }

            // Handle incoming messages from the client (for future ping/pong or commands)
            result = receiver.next() => {
                match result {
                    Some(Ok(Message::Close(_))) => {
                        debug!("WebSocket client sent close frame");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {
                        // Ignore other messages for now
                    }
                    Some(Err(e)) => {
                        debug!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        debug!("WebSocket client disconnected");
                        break;
                    }
                }
            }
        }
    }

    debug!("WebSocket connection closed");
}
