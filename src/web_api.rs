use axum::{
    extract::{State, WebSocketUpgrade},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::sync::{Arc, Mutex};
use axum::extract::ws::{Message, WebSocket};
use futures::{stream::StreamExt, SinkExt};
use tokio::{task, sync::mpsc};
use tracing::debug;
use tower_http::services::fs::ServeDir;
use crate::peer_manager::{FrontendEvent, PeerManagerHandle};

#[derive(Clone)]
pub struct ApiState {
    pub peer_manager: Arc<PeerManagerHandle>,
    pub clients: Arc<Mutex<Vec<mpsc::UnboundedSender<Message>>>>,
}

pub fn router(state: ApiState, web_api_rx: mpsc::Receiver<FrontendEvent>) -> Router {
    start_event_forwarder(state.clone(), web_api_rx);

    Router::new()
        .route("/peers", get(get_peers))
        .route("/send", post(send_message))
        .route("/ws", get(ws_handler))
        .with_state(state)
        .fallback_service(ServeDir::new("frontend"))
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
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();
    let tx_c = tx.clone();
    state.clients.lock().unwrap().push(tx.clone());

    let send_task = task::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let recv_state = state.clone();
    let recv_task = task::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                if text.starts_with("/peers") {
                    let peers = recv_state.peer_manager.get_peers().await;
                    let json = serde_json::to_string(&peers).unwrap();
                    let _ = tx.send(Message::Text(json.into())); 
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

    let mut clients = state.clients.lock().unwrap();
    clients.retain(|c| !c.same_channel(&tx_c));
    debug!("WebSocket disconnected");
}

fn start_event_forwarder(state: ApiState, mut web_api_rx: mpsc::Receiver<FrontendEvent>) {
    tokio::spawn(async move {
        while let Some(event) = web_api_rx.recv().await {
            println!("{:?}", serde_json::to_string(&event));
            let msg = match serde_json::to_string(&event) {
                Ok(json) => Message::Text(json.into()),
                Err(_) => continue,
            };
            let mut clients = state.clients.lock().unwrap();
            clients.retain(|client| client.send(msg.clone()).is_ok());
        }
    });
}

