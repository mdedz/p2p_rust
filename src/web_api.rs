use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, mpsc::Receiver},
};
use axum::extract::ws::{Message, WebSocket};
use futures::{stream::StreamExt, SinkExt};
use tokio::task;
use tracing::debug;

use crate::peer_manager::{FrontendEvent, PeerManagerHandle};

#[derive(Clone)]
pub struct ApiState {
    pub peer_manager: Arc<PeerManagerHandle>,
    pub clients: Arc<Mutex<HashSet<tokio::sync::mpsc::UnboundedSender<Message>>>>,
}

pub fn router(state: ApiState, web_api_rx: Receiver<FrontendEvent>) -> Router {
    // Spawn background task to forward events to all connected clients
    start_event_forwarder(state.clone(), web_api_rx);

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

#[derive(serde::Deserialize)]
struct SendPayload {
    msg: String,
}

async fn send_message(
    State(state): State<ApiState>,
    Json(payload): Json<SendPayload>,
) -> impl IntoResponse {
    state
        .peer_manager
        .broadcast(format!("MSG|{}\n", payload.msg))
        .await;
    "sent"
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<ApiState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: ApiState) {
    debug!("WebSocket connected");

    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

    // Add this client to the global client list
    state.clients.lock().unwrap().insert(tx.clone());

    // Task for sending messages to this client
    let send_task = task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Task for receiving messages from this client
    let recv_state = state.clone();
    let recv_task = task::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if text.starts_with("/peers") {
                    let peers = recv_state.peer_manager.get_peers().await;
                    let json = serde_json::to_string(&peers).unwrap();
                    let _ = tx.send(Message::Text(json));
                } else {
                    recv_state
                        .peer_manager
                        .broadcast(format!("MSG|{}\n", text))
                        .await;
                }
            }
        }
    });

    tokio::select! {
        _ = send_task => (),
        _ = recv_task => (),
    }

    // Remove the client when disconnected
    state.clients.lock().unwrap().remove(&tx);
    debug!("WebSocket disconnected");
}

fn start_event_forwarder(state: ApiState, web_api_rx: Receiver<FrontendEvent>) {
    task::spawn_blocking(move || {
        for event in web_api_rx {
            let msg = match serde_json::to_string(&event) {
                Ok(json) => Message::Text(json),
                Err(_) => continue,
            };
            let clients = state.clients.lock().unwrap();
            for client in clients.iter() {
                let _ = client.send(msg.clone());
            }
        }
    });
}
