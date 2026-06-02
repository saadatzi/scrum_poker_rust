# Scrum Poker (Rust)

Lightweight Scrum Poker web app written in Rust using Axum and WebSockets. The app creates a new room for each visitor and encodes the room id in the URL so you can share a link and let others join the same room. No authentication required.

## Features

- Per-URL rooms: every visit to `/` redirects to `/room/<uuid>` and a shared URL joins the same room.
- WebSocket-based realtime updates scoped to each room (`/ws/<room_id>`).
- Simple client UI in `public/` (no build step for frontend).
- Server automatically removes empty rooms.

## Prerequisites

- Rust and Cargo (install via https://rustup.rs/). A recent stable toolchain is recommended.

## Build & run

Development (fast, rebuilds automatically each run):

```sh
cargo run
```

This starts the server on port 3000. You should see:

```
Server running on http://localhost:3000
```

Open `http://localhost:3000` in your browser — it will redirect you to a new room URL like `/room/<uuid>`.

Build a release binary:

```sh
cargo build --release
```

Run the release binary:

- On Windows:

```powershell
target\release\scrum_poker_rust.exe
```

- On Linux / macOS:

```sh
./target/release/scrum_poker_rust
```

## Usage

- Visit `http://localhost:3000`. The server redirects to a room URL: `/room/<uuid>`.
- Copy and share the full `/room/<uuid>` URL with others to let them join the same room.
- The frontend connects to the WebSocket endpoint `ws://<host>/ws/<room_id>`.

Client message examples (JSON):

- Join: `{ "type": "join", "name": "Alice" }`
- Vote: `{ "type": "vote", "value": "8" }`
- Toggle reveal: `{ "type": "reveal" }`
- Clear votes: `{ "type": "clear" }`

Server broadcasts a JSON message with the current room state (users, revealed flag, and optional notification).

## Project layout

- `src/main.rs` – server code (Axum + WebSocket handling)
- `public/` – frontend static files (`index.html`, `client.js`, `style.css`)
- `Cargo.toml` – Rust dependencies

## Notes / Troubleshooting

- The server listens on `0.0.0.0:3000`. If port 3000 is already in use, change the bind address in `src/main.rs`.
- No persistent storage; rooms and state are kept in memory and removed when empty.

---

If you want, I can also add a small README badge, a CONTRIBUTING section, or a button in the UI to copy the room link to clipboard.