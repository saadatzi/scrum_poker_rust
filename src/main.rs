use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::{Html, IntoResponse, Redirect},
    routing::{get, get_service},
    Router,
};
use dashmap::DashMap;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::services::ServeDir;

const INDEX_HTML: &str = include_str!("../public/index.html");

#[derive(Debug)]
struct AppState {
    rooms: DashMap<String, Arc<RoomState>>,
}

#[derive(Debug)]
struct RoomState {
    users: DashMap<String, User>,
    revealed: RwLock<bool>,
    tx: broadcast::Sender<String>,
}

impl RoomState {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(100);
        Self {
            users: DashMap::new(),
            revealed: RwLock::new(false),
            tx,
        }
    }
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
    notification: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        rooms: DashMap::new(),
    });

    let app = Router::new()
        .route("/", get(create_room_redirect))
        .route("/room/{room_id}", get(room_page))
        .route("/ws/{room_id}", get(ws_handler))
        .fallback_service(get_service(ServeDir::new("public")))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    println!("Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}

async fn create_room_redirect() -> Redirect {
    Redirect::temporary(&format!("/room/{}", uuid::Uuid::new_v4()))
}

async fn room_page(Path(_room_id): Path<String>) -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn ws_handler(
    Path(room_id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state, room_id))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>, room_id: String) {
    let room = get_or_create_room(&state, &room_id);
    let (mut sender, mut receiver) = socket.split();
    let mut rx = room.tx.subscribe();
    let user_id = uuid::Uuid::new_v4().to_string();

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if sender.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(Message::Text(text))) = receiver.next().await {
        if let Ok(action) = serde_json::from_str::<Action>(&text) {
            let mut notification = None;
            match action {
                Action::Join { name } => {
                    room.users.insert(
                        user_id.clone(),
                        User {
                            id: user_id.clone(),
                            name,
                            vote: None,
                        },
                    );
                }
                Action::Vote { value } => {
                    if let Some(mut user) = room.users.get_mut(&user_id) {
                        user.vote = Some(value);
                    }
                }
                Action::Reveal => {
                    let name = room
                        .users
                        .get(&user_id)
                        .map(|u| u.name.clone())
                        .unwrap_or_else(|| "Someone".to_string());
                    if let Ok(mut revealed) = room.revealed.write() {
                        *revealed = !*revealed;
                        notification = Some(format!("{} toggled reveal", name));
                    }
                }
                Action::Clear => {
                    let name = room
                        .users
                        .get(&user_id)
                        .map(|u| u.name.clone())
                        .unwrap_or_else(|| "Someone".to_string());
                    for mut user in room.users.iter_mut() {
                        user.vote = None;
                    }
                    if let Ok(mut revealed) = room.revealed.write() {
                        *revealed = false;
                    }
                    notification = Some(format!("{} cleared votes", name));
                }
            }

            broadcast_room_state(&room, notification);
        }
    }

    send_task.abort();
    room.users.remove(&user_id);
    broadcast_room_state(&room, None);

    if room.users.is_empty() {
        state.rooms.remove(&room_id);
    }
}

fn get_or_create_room(state: &AppState, room_id: &str) -> Arc<RoomState> {
    state
        .rooms
        .entry(room_id.to_string())
        .or_insert_with(|| Arc::new(RoomState::new()))
        .clone()
}

fn broadcast_room_state(room: &RoomState, notification: Option<String>) {
    let users: Vec<User> = room.users.iter().map(|user| user.clone()).collect();
    let revealed = *room.revealed.read().unwrap();
    let msg = serde_json::to_string(&ServerMessage {
        users,
        revealed,
        notification,
    })
    .unwrap();
    let _ = room.tx.send(msg);
}
