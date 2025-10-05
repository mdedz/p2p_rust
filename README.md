# Rust P2P Chat

A simple peer-to-peer (P2P) chat application implemented in Rust using **Tokio** for async networking.  
This project demonstrates building a fully asynchronous P2P network with peer management, message broadcasting, and a simple CLI interface.

---

## Features

- **P2P Network**: Each node can act as both client and server.
- **Peer Management**: Keeps track of connected peers with unique node IDs.
- **Message Broadcasting**: Messages sent by one peer are broadcasted to all connected peers.
- **Async Networking**: Uses `tokio::net::TcpStream` and async channels for high-performance networking.
- **CLI Commands**: Supports listing all connected users and inspecting the network state.

---

## Modules Overview

### `main.rs`
- Parses command-line arguments for port, peer address, and username.
- Starts server listener and optionally connects to a peer.
- Handles stdin input for messages and commands.

### `peer.rs`
- Represents a connected peer.
- Wraps the TCP socket and provides async reading/writing of messages.
- Maintains peer info: username, node ID, remote and listen addresses.

### `peer_manager.rs`
- Manages all connected peers.
- Provides adding, removing, broadcasting messages, and listing users.
- Uses `Arc<Mutex<HashMap<...>>>` for concurrent access.

### `network.rs`
- Handles connection logic for new peers.
- Spawns listening tasks for each peer.
- Connects to peers, avoiding duplicates or self-connections.

### `protocol.rs`
- Defines the message protocol (`JOIN`, `PEERS|`).
- Handles incoming messages and updates peer manager.
- Sends peers info and JOIN requests.

### `server.rs`
- TCP listener accepting new connections.
- Sends JOIN and PEERS messages to new peers.
- Spawns a listener task for each connection.

### `client.rs`
- Connects to an existing peer.
- Sends JOIN message and requests current peer list.

### `ui.rs`
- Parses CLI commands starting with `/`.
- Supports listing users and peer info.

---

## Commands

- `/l` - List all connected users.
- `/p` - Show peers payload (network info).
- Any other text is sent as a broadcast message to all peers.

---

## Usage

### Running a server:
```bash
cargo run -- --port 8080 --uname Alice

### Connecting to a peer:

```bash
cargo run -- --port 8081 --peer 127.0.0.1:8080 --uname Bob
```

### Sending messages:

* Type your message and press Enter to broadcast to all peers.
* Use `/l` to list all connected users.

---

## Dependencies

* [`tokio`](https://docs.rs/tokio) — Async runtime for networking and concurrency.
* [`clap`](https://docs.rs/clap) — Command-line argument parser.
* [`serde`](https://docs.rs/serde) — Serialization/deserialization for peer info.
* [`anyhow`](https://docs.rs/anyhow) — Error handling.

---

## Architecture Notes

1. **PeerManager** maintains all connected peers and handles broadcast logic.
2. **Peer** wraps a TCP connection with read/write halves and async channels.
3. **Protocol** defines a simple text-based protocol:

   * `JOIN|<peer_info>` — informs network about a new peer.
   * `PEERS|<peer_list>` — propagates list of known peers.
4. **Async Execution**: Each peer connection runs in its own task via `tokio::spawn`.
5. **Thread Safety**: Shared structures are wrapped in `Arc<Mutex<...>>`.

---

## Future Improvements

* Replace `Mutex` with `RwLock` for better read performance on large networks.
* Implement private messaging between peers.
* Add persistent peer storage to reconnect after restarts.
* Enhance CLI with more commands (kick, rename, etc.).
* Improve error handling and reconnection logic.

---

## License

MIT License. Free to use and modify.

---

```
