// src/api.rs
use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use crate::peer_manager::PeerManagerHandle;
use axum::extract::ws::{Message, WebSocket};
use tracing::debug;

#[derive(Clone)]
pub struct ApiState {
    pub peer_manager: Arc<PeerManagerHandle>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/peers", get(get_peers))
        .route("/send", post(send_message))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn get_peers(State(state): State<ApiState>) -> impl IntoResponse {
    let peers = state.peer_manager.get_peers().await;
    Json(peers)
}

async fn send_message(State(state): State<ApiState>, Json(payload): Json<SendPayload>) -> impl IntoResponse {
    state.peer_manager.broadcast(format!("MSG|{}\n", payload.msg)).await;
    "sent"
}

#[derive(serde::Deserialize)]
struct SendPayload {
    msg: String,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ApiState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: ApiState) {
    debug!("WebSocket connected");

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            if text.starts_with("/peers") {
                let peers = state.peer_manager.get_peers().await;
                let json = serde_json::to_string(&peers).unwrap();
                socket.send(Message::Text(json.into())).await.ok();
            } else {
                state.peer_manager.broadcast(format!("MSG|{}\n", text)).await;
            }
        }
    }
}
