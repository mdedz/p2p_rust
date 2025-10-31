# P2P Chat (Rust + Tokio + Axum)

**Dark, minimal, peer-to-peer chat built with Rust.**

This repository contains a small, opinionated P2P chat/demo framework implemented in Rust using Tokio for async I/O, `rustls` + `rcgen` for optional TLS, and `axum` for a tiny web API + websocket front-end.

It aims to be a compact, reusable foundation for building peer-to-peer applications (chat, device mesh, telemetry, etc.) with a minimal surface area and clear actor-style `PeerManager` that holds connections and peers.

---

## Features

* Simple TCP-based peer-to-peer protocol (text-based lines)
* Optional TLS support using `rustls` and auto-generated self-signed certs
* Actor-style `PeerManager` to manage connections and nodes
* Web API (Axum) exposing:

  * `/peers` — GET peers list (JSON)
  * `/send` — POST to broadcast (JSON)
  * `/ws` — WebSocket UI for live messages and events
* Small, dark-themed static frontend (in `frontend/index.html`) for local testing
* Clean separation: `server` accepts incoming connections, `client` connects out

---

## Quick Start

### Prerequisites

* Rust toolchain (recommended: stable or nightly via rustup)
* `cargo` available in your PATH
* (Optional) `openssl`/`rcgen` not required — project uses `rcgen` to generate self-signed certs

### Build

```bash
cargo build --release
```

### Run (server)

Run a node listening on a port (e.g. 8000):

```bash
cargo run -- --port 8000 --uname Alice
```

That will start:

* a TCP server listening at `127.0.0.1:8000`
* an HTTP/Axum API on `127.0.0.1:8100` (port + 100)

### Run (client connect-to-peer)

From another terminal you can start a node that connects to an existing peer (client mode):

```bash
cargo run -- --port 9000 --peer 127.0.0.1:8000 --uname Bob
```

If connection succeeds, nodes exchange a brief JSON `JOIN` payload and then forward `PEERS` lists.

### Enable TLS

TLS can be enabled with `--tls`. When enabled the node will look for `tls/cert.der` and `tls/key.der` in the repository root.

* If the files exist, they will be used.
* Otherwise the node auto-generates a self-signed cert and writes `tls/cert.der` and `tls/key.der` for reuse.

Example:

```bash
cargo run -- --port 8000 --tls --uname SecureAlice
cargo run -- --port 9000 --peer 127.0.0.1:8000 --tls --uname SecureBob
```

Notes about TLS:

* This demo trusts the server cert by adding it to the client's root cert store (see `tls_utils.rs`).
* The default self-signed cert contains `localhost` and `127.0.0.1` SANs.
* For production use: replace self-signed cert handling with a proper PKI or mTLS depending on your requirements.

---

## HTTP / WebSocket API

The small web API lets you inspect peers and send messages programmatically.

* `GET /peers` — returns JSON list of `PeerSummary`
* `POST /send` with `{ "msg": "hello" }` — broadcasts message to connected peers
* `GET /ws` — WebSocket that receives `FrontendEvent` JSON messages and can send chat messages (or `/peers` command to request the peer list)

Example using `curl`:

```bash
curl http://127.0.0.1:8100/peers
curl -X POST -H 'Content-Type: application/json' -d '{"msg":"hello from api"}' http://127.0.0.1:8100/send
```

---

## Protocol (short)

This project uses a tiny text protocol over TCP (each message is newline terminated):

* `JOIN|<json>` — register a node and provide `PeerSummary`
* `PEERS|<json1>;<json2>;...` — inform a peer about other peers
* `MSG|<payload>` — chat message forwarded and displayed

`protocol.rs` contains helpers to serialize/deserialize these payloads.

---

## Project layout

```
src/
  client.rs        # client-side connection helper
  server.rs        # server listener and accept logic
  network.rs       # connect_new_peer, handle_peer_list
  peer_manager.rs  # PeerManager + PeerEntry actors and queues
  protocol.rs      # message format helpers (JOIN/PEERS/MSG)
  tls_utils.rs     # rustls/rcgen helpers
  web_api.rs       # axum routes + websocket
  main.rs          # CLI, initialization and orchestration
frontend/
  index.html       # minimal dark UI for demo
tls/               # runtime-generated cert.der & key.der when --tls

Cargo.toml
README.md
```

---

## Developer notes (important implementation details)

* `PeerManagerHandle` is an `Arc`-wrapped actor that accepts `Command` messages through an mpsc channel. This decouples tasks and prevents locking the reactor.
* `PeerEntry` spawns a dedicated reader/writer per connection using Tokio tasks; messages are sent over a channel.
* When a server accepts an incoming socket it calls `add_conn` or `add_entry` depending on TLS presence.
* Outgoing connections use `connect_new_peer` from `network.rs`. If TLS is enabled, it performs the handshake using a client config built from the server certificate.
* `send_join` and `send_peers` implement the basic peer discovery handshake.

---

## Security & production considerations

This repo is intended as a demo / starting point. If you plan to use it beyond testing:

* **Do not** rely on self-signed certs in production. Integrate with a proper CA or mTLS.
* Consider authenticating peers (token-based or mutual TLS) and verifying identity beyond a single `node_id` string.
* Increase channel sizes and add backpressure handling for high throughput scenarios.
* Validate and sanitize all incoming payloads strictly (the demo assumes well-formed JSON in many places).
* Consider reconnect strategies, heartbeat/ping messages, and message sequencing to avoid duplicates.

---

## Logging

Tracing is used (`tracing` crate). The program initialises `tracing_subscriber::fmt()` in `main.rs`. Adjust levels with `RUST_LOG` env var, e.g.: `RUST_LOG=debug cargo run -- ...`.

---

## TODO / Roadmap

* Add integration tests for connection flows and protocol messages
* Add reconnection/backoff strategy for clients
* Add optional mTLS for peer authentication
* Support NAT traversal / hole-punching for non-local networks
* Provide a CLI UI for node management
* Formalize message format and introduce message versioning

---

## License

MIT — see `LICENSE` (or add one if not present).

---
