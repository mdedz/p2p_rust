# Rust P2P Chat — `main` branch

**Important:** The repository contains two branches: `legacy` (older implementation) and `main` (current refactored version). In short:

* `legacy` — the classic implementation: `Peer` uses `Arc<Mutex<...>>`, each peer owns its own reader/writer, and `PeerManager` is a shared `Arc<Mutex<HashMap<...>>>`. Suitable for learning and simple network topologies.
* `main` — a reworked, actor-based implementation: connections are identified by a temporary `conn_id` before registration (`JOIN`), the central `PeerManagerHandle` operates as an actor (mpsc commands + oneshot responses), socket I/O is handled within `PeerEntry`, which sends events to the manager. Main improvements include explicit access serialization, better separation of concerns, and production readiness.

---

# Overview

`main` is a modern, stable, and scalable version of the Rust P2P chat built on top of `tokio`. The goal is to simplify connection management, remove overreliance on mutexes, and transition to an explicit actor model.

## Core Concepts

* Connections are created and assigned a `conn_id` — until a remote node sends a `JOIN`, it remains a temporary connection.
* When `JOIN|<PeerSummary>` arrives, the manager registers that connection under a `node_id`, allowing messages to be addressed by `node_id`.
* All operations on connection/peer collections go through the `PeerManagerHandle` (mpsc commands), which eliminates race conditions with HashMaps.
* I/O (reader/writer) runs in independent async tasks rather than global mutexes.

---

## Quick Start

### Build

```bash
cargo build --release
```

### Run a node (server + client in one process)

```bash
# start a server on port 8080
cargo run -- --port 8080 --uname Alice

# connect a client to another node
cargo run -- --port 8081 --peer 127.0.0.1:8080 --uname Bob
```

After startup, type into stdin — each line will be broadcast to all known peers.

> Note: CLI command parsing (`/l`, `/p`) is currently commented out in `main`.

---

## Message Protocol

All messages are newline-terminated strings (`\n`).

* `JOIN|<json>` — connection registration. JSON matches `PeerSummary`.
* `PEERS|<entry1>;<entry2>;...` — list of known peers, each `entry` is a JSON `PeerSummary`, separated by semicolons.
* Any other line is treated as a chat message.

`PeerSummary` example:

```json
{
  "remote_addr": "127.0.0.1:1234",
  "listen_addr": "127.0.0.1:8080",
  "node_id": "...",
  "uname": "Alice"
}
```

---

## Project Structure (Main Files)

* `main.rs` — initialization, creates `PeerManagerHandle`, starts the server listener, optionally connects to a peer, reads stdin, and broadcasts messages.
* `server.rs` — accepts incoming TCP connections, creates a `conn_id`, calls `peer_manager.add_conn(...)`, and sends a `JOIN` message with local info.
* `client.rs` — initiates outgoing connections via `connect_new_peer(...)`.
* `network.rs` — manages connection logic, prevents self-connections and duplicates, handles `connect_new_peer` and `handle_peer_list`.
* `peer_manager.rs` — the core actor (`PeerManagerHandle`): receives commands (`AddConn`, `RegisterNode`, `Broadcast`, `SendTo`, `GetPeers`, etc.), maintains separate `HashMap`s for `conns` and `peers`, and processes `PeerEvent`s.
* `protocol.rs` — parses `JOIN`/`PEERS`, builds payloads, and sends messages through the manager API.
* `ui.rs` — command-line interface (temporarily disabled in `main`).

---

## PeerManagerHandle — Public API (Summary)

`PeerManagerHandle::new(self_peer_info) -> Arc<PeerManagerHandle>` — creates the actor and starts its loop.

Async methods:

* `add_conn(conn_id, summary, socket) -> anyhow::Result<()>` — add a new temporary connection.
* `register_node(conn_id, summary) -> anyhow::Result<()>` — register a node ID for a previously added connection.
* `remove_conn(conn_id)` — remove a connection.
* `remove_node(node_id)` — remove a registered node.
* `broadcast(msg)` — send `msg` to all registered nodes.
* `send_to(node_id: Option<String>, conn_id: Option<String>, msg: String) -> anyhow::Result<()>` — send message to a node or connection.
* `get_peers() -> Vec<PeerSummary>` — get all registered peers.
* `get_peer(node_id) -> Option<PeerSummary>` — get a peer summary by node ID.
* `get_conn(conn_id) -> Option<PeerSummary>` — get a summary by connection ID.
* `contains_listen_addr(addr) -> bool` — check if a peer with this `listen_addr` already exists.

---

## Connection Behavior

* On `AddConn`, a `PeerEntry` is spawned, creating reader/writer tasks:

  * The writer consumes from an mpsc channel and writes messages to the socket (each ending with `\n`).
  * The reader consumes socket lines, creates `PeerEvent`s, and forwards them to `events_tx`.
* On `PeerEvent::Join`, `handle_join_json` parses the `PeerSummary`, calls `register_node`, and triggers `send_peers`.
* On `PeerEvent::Peers`, `handle_peers_json` parses the list and calls `handle_peer_list` to establish new outbound connections.

---

## Practical Notes and Limits

* `conn_id` is generated via UUID. The local `node_id` is generated at startup (`main`), but for remote nodes, it comes via `JOIN`.
* `PeerEntry` uses a buffered mpsc channel (capacity 60) for outbound messages.
* The manager’s event channel has a capacity of 1000.
* There’s no authentication/encryption yet — the protocol is plain text. TLS, Noise, or Signal will be added later.

---

## Examples

* Self-connections are rejected (`Cannot connect to itself`).
* If a `PEERS|` message arrives, the manager attempts to connect to all listed addresses via `connect_new_peer`.
* After a successful `JOIN`, the remote node is moved from `conns` to `peers` and assigned a persistent `node_id`.

---

## Future Improvements

* Re-enable CLI commands (`/l`, `/p`, rename, etc.).
* Add optional authentication and transport encryption.
* Implement persistence of known peers and reconnection policy.
* Add unit and integration tests for core components.

---

## License

MIT

---
