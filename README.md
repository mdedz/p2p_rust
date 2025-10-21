# Rust P2P Chat

A simple **peer-to-peer (P2P) chat application** implemented in Rust using **Tokio** for asynchronous networking.  
This project demonstrates how to build a decentralized network with an actor-based peer manager, broadcast messaging, and a small HTTP/WebSocket API.

---

## 🧭 Overview

Each node acts as both:
- a **server** (accepting incoming TCP connections), and
- a **client** (initiating connections to other peers).

The network is formed dynamically: peers exchange `JOIN` and `PEERS` messages encoded as simple text frames to discover and register each other.

A lightweight **actor system** (implemented with `tokio::mpsc`) handles peer state management, message broadcasting, and coordination with the web API.

---

## ✨ Features

- ⚡ Fully **asynchronous I/O** with Tokio
- 🔁 **P2P architecture** — every node is both server and client
- 👥 **Peer discovery** via `JOIN` and `PEERS` messages
- 📢 **Message broadcasting** across peers
- 🧠 **Actor-based peer manager** (no global locks!)
- 🌐 **Web API & WebSocket** for frontend or external clients
- 💬 **CLI chat** — type into stdin to broadcast messages
- 🧩 Modular design: `server`, `client`, `network`, `protocol`, `peer_manager`, `web_api`

---

## 📂 Module Overview

| Module | Description |
|--------|--------------|
| **`main.rs`** | CLI entrypoint. Parses args (`--port`, `--peer`, `--uname`), starts listener, connects to peers, and spawns the web API. |
| **`server.rs`** | Listens for incoming TCP peers, creates connection entries, and sends initial `JOIN` messages. |
| **`client.rs`** | Initiates outbound connections to peers when `--peer` is provided. |
| **`network.rs`** | Contains `connect_new_peer` (for outgoing connections) and `handle_peer_list` (for connecting to new peers discovered via `PEERS`). |
| **`protocol.rs`** | Defines message types and parsing logic: handles `JOIN`, `PEERS`, and `MSG` frames. |
| **`peer_manager.rs`** | Core actor managing peers and connections. Exposes async API for adding/removing peers, broadcasting, and sending. |
| **`web_api.rs`** | Minimal Axum web server providing REST endpoints and a WebSocket interface for frontend integration. |

---

## 🧱 Core Data Structures

### `PeerSummary`
```rust
pub struct PeerSummary {
    pub remote_addr: Option<String>,
    pub listen_addr: Option<String>,
    pub node_id: Option<String>,
    pub uname: Option<String>,
}
````

Fields are optional because nodes gradually exchange info as they connect and register.

---

## 💬 Message Protocol

Simple text-based framing (`\n` delimited):

| Type      | Format | Description        |                                         |
| --------- | ------ | ------------------ | --------------------------------------- |
| **JOIN**  | `JOIN  | <json>`            | Introduce a node and share its metadata |
| **PEERS** | `PEERS | <json>;<json>;...` | Share known peer summaries              |
| **MSG**   | `MSG   | <payload>`         | Broadcast a chat message to all peers   |

**Example:**

```
JOIN|{"listen_addr":"127.0.0.1:8081","node_id":"abc123","uname":"Bob"}
PEERS|{"listen_addr":"127.0.0.1:8082"};{"listen_addr":"127.0.0.1:8083"}
MSG|Hello, world!
```

---

## ⚙️ How It Works

1. Each node starts a TCP listener and a small HTTP API (on `port + 100`).
2. When a connection is made, both peers exchange `JOIN` messages.
3. Each node sends its current peer list as `PEERS|...`.
4. When a new peer joins, others can connect automatically using `handle_peer_list`.
5. All incoming messages (`MSG|text`) are broadcast to all connected peers.
6. The **peer manager actor** handles concurrency safely by serializing commands over a channel.

---

## 🚀 Running

### Start a node

```bash
cargo run -- --port 8080 --uname Alice
```

* Listens on TCP port **8080**
* Starts web API on **8180** (`port + 100`)
* Username is **Alice**

### Connect another node

```bash
cargo run -- --port 8081 --peer 127.0.0.1:8080 --uname Bob
```

* Connects to peer at `127.0.0.1:8080`
* Exchanges `JOIN` and `PEERS` messages

### Chat from CLI

Type into the terminal and press Enter — your message will be broadcast to all connected peers as:

```
MSG|<your text>\n
```

---

## 🌐 Web API

Each node also runs an HTTP API on `127.0.0.1:(port + 100)`.

| Endpoint | Method | Description                                     |
| -------- | ------ | ----------------------------------------------- |
| `/peers` | `GET`  | Returns all connected peers as JSON             |
| `/send`  | `POST` | Broadcast a message: `{"msg":"Hello"}`          |
| `/ws`    | `GET`  | WebSocket stream for real-time frontend updates |

### Example `curl` usage

List peers:

```bash
curl http://127.0.0.1:8180/peers
```

Send a message:

```bash
curl -X POST http://127.0.0.1:8180/send -H "Content-Type: application/json" -d '{"msg":"Hi peers!"}'
```

---

## 🧠 Architecture

### Actor pattern

* `PeerManagerHandle` exposes async methods.
* Internally, it sends `Command` enums over a `tokio::mpsc` channel.
* The background `command_loop` processes all mutations serially.

### Peer lifecycle

1. **Connection accepted/initiated** → `add_conn`
2. **JOIN received** → `register_node`
3. **PEERS received** → `handle_peer_list`
4. **Disconnected** → cleanup from peer maps

---

## 🧩 Dependencies

* [`tokio`](https://docs.rs/tokio) — async runtime
* [`axum`](https://docs.rs/axum) — web API & WebSocket
* [`serde` / `serde_json`](https://serde.rs) — serialization
* [`uuid`](https://docs.rs/uuid) — unique IDs for nodes/connections
* [`clap`](https://docs.rs/clap) — CLI argument parsing
* [`anyhow`](https://docs.rs/anyhow) — error handling
* [`tracing`](https://docs.rs/tracing) — logging
* [`tower-http`](https://docs.rs/tower-http) — for serving static frontend (optional)

---

## 🧪 Example Network

```
Alice (127.0.0.1:8080)
   ↑         ↓
Bob (127.0.0.1:8081)
   ↑         ↓
Carol (127.0.0.1:8082)
```

* Alice starts first
* Bob connects to Alice
* Alice sends Bob its known peers
* Carol connects to Alice or Bob and learns about the other nodes
* Now all three can exchange chat messages in real time!

---

## 🔧 Future Improvements

* Persistent peer discovery cache
* Reconnection & retry logic
* Encrypted connections (TLS)
* Authenticated JOIN messages
* Message history storage
* Frontend UI using WebSocket API

---

## 🪪 License

MIT License — free to use, modify, and distribute.

---
