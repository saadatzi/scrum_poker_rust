use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::{get, get_service},
    Router,
};
use dashmap::DashMap;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

#[derive(Debug)]
struct AppState {
    users: DashMap<String, User>,
    revealed: RwLock<bool>,
    tx: broadcast::Sender<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct User {
    id: String,
    name: String,
    vote: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "type")]
enum Action {
    #[serde(rename = "join")]
    Join { name: String },
    #[serde(rename = "vote")]
    Vote { value: String },
    #[serde(rename = "reveal")]
    Reveal,
    #[serde(rename = "clear")]
    Clear,
}

#[derive(Serialize, Clone, Debug)]
struct ServerMessage {
    users: Vec<User>,
    revealed: bool,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let (tx, _) = broadcast::channel(100);
    let state = Arc::new(AppState {
        users: DashMap::new(),
        revealed: RwLock::new(false),
        tx,
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(get_service(ServeDir::new("public")))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.tx.subscribe();
    let user_id = uuid::Uuid::new_v4().to_string();

    // Task to forward broadcast messages to the websocket
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(Message::Text(text))) = receiver.next().await {
        if let Ok(action) = serde_json::from_str::<Action>(&text) {
            match action {
                Action::Join { name } => {
                    state.users.insert(
                        user_id.clone(),
                        User {
                            id: user_id.clone(),
                            name,
                            vote: None,
                        },
                    );
                }
                Action::Vote { value } => {
                    if let Some(mut user) = state.users.get_mut(&user_id) {
                        user.vote = Some(value);
                    }
                }
                Action::Reveal => {
                    if let Ok(mut revealed) = state.revealed.write() {
                        *revealed = !*revealed;
                    }
                }
                Action::Clear => {
                    for mut user in state.users.iter_mut() {
                        user.vote = None;
                    }
                }
            }

            let users: Vec<User> = state.users.iter().map(|u| u.clone()).collect();
            let revealed = *state.revealed.read().unwrap();
            let msg = serde_json::to_string(&ServerMessage { users, revealed }).unwrap();
            let _ = state.tx.send(msg);
        }
    }

    send_task.abort();
    state.users.remove(&user_id);
}
