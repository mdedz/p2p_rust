use serde::{Deserialize, Serialize};
use tokio::{net::TcpStream, sync::{mpsc::{Receiver}, oneshot}};
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{warn, debug, error, info};
use tokio::{
    io::{AsyncWriteExt, BufReader, AsyncBufReadExt},
    net::{tcp::{OwnedReadHalf, OwnedWriteHalf}},
    sync::{mpsc},
};


pub fn generate_unique_id() -> String{
    Uuid::new_v4().to_string()
}


#[derive(Clone, Serialize, Deserialize)]
pub struct PeerSummary {
    pub remote_addr: Option<String>,
    pub listen_addr: Option<String>,
    pub node_id:  Option<String>,
    pub uname: Option<String>,
}

impl PeerSummary {
    pub fn _uname_or_err(&self) -> anyhow::Result<String> {
        self.uname.clone()
            .ok_or_else(|| anyhow::anyhow!("uname is missing"))
    }

    pub fn listen_addr_or_err(&self) -> anyhow::Result<String> {
        self.listen_addr.clone()
            .ok_or_else(|| anyhow::anyhow!("listen_addr is missing"))
    }

    pub fn node_id_or_err(&self) -> anyhow::Result<String> {
        self.node_id.clone()
            .ok_or_else(|| anyhow::anyhow!("node_id is missing"))
    }

    pub fn remote_addr_or_err(&self) -> anyhow::Result<String> {
        self.remote_addr.clone()
            .ok_or_else(|| anyhow::anyhow!("remote_addr is missing"))
    }

    // pub fn uname(&self) -> Option<String> {
    //     self.uname.clone()
    // }

    pub fn uname_or_default(&self) -> String {
        self.uname.clone().unwrap_or("Stranger".to_string())
    }

    // pub fn listen_addr(&self) -> Option<String> {
    //     self.listen_addr.clone()
    // }

    // pub fn node_id(&self) -> Option<String> {
    //     self.node_id.clone()
    // }

    // pub fn remote_addr(&self) -> Option<String> {
    //     self.remote_addr.clone()
    // }
}

#[derive(Clone)]
struct PeerEntry {
    conn_id: String,                 
    summary: PeerSummary,            
    tx: mpsc::Sender<String>,
}

impl PeerEntry {
    pub fn new (conn_id: String, summary:PeerSummary, socket: TcpStream, events_tx: mpsc::Sender<PeerEvent>) -> Self{
        let (reader, writer) = socket.into_split();
        let (tx, rx) = mpsc::channel::<String>(60);

        Self::spawn_reader(reader, summary.node_id.clone().unwrap_or_default(), events_tx.clone());
        Self::spawn_writer(writer, rx);

        Self { conn_id, summary, tx }
    }

    pub async fn send(&self, msg: String) -> anyhow::Result<()>{
        self.tx.send(msg).await?;
        Ok(())
    }

    pub fn spawn_writer(mut writer: OwnedWriteHalf, mut rx: mpsc::Receiver<String>) {
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = writer.write_all(msg.as_bytes()).await {
                    error!("Peer writer error: {}", e);
                    break;
                }
                if let Err(e) = writer.write_all(b"\n").await {
                    error!("Peer writer newline error: {}", e);
                    break;
                }
            }
        });
    }

    pub fn spawn_reader(mut reader: OwnedReadHalf, node_id: String, events_tx: mpsc::Sender<PeerEvent>) {
        tokio::spawn(async move {
            let mut buf = BufReader::new(&mut reader);
            let mut line = String::new();

            loop {
                line.clear();
                match buf.read_line(&mut line).await {
                    Ok(0) => {
                        let _ = events_tx
                            .send(PeerEvent::Disconnected { node_id: node_id.clone() });
                    }

                    Ok(_) => {
                        let msg = line.trim().to_string();
                        if let Err(_) = events_tx
                            .send(PeerEvent::Message { node_id: node_id.clone(), msg: msg })
                            .await
                        {
                            warn!("PeerEvent channel closed");
                            break;
                        } 
                    }

                    Err(e) => {
                        let _ = events_tx
                            .send(PeerEvent::Error { node_id: node_id.clone(), error: e.to_string() })
                            .await;
                        break;
                    }
                }
            }

        });
    }

}

pub enum PeerEvent {
    Message { node_id: String, msg: String },
    Join { conn_id: String, msg: String },
    Connected { node_id: String },
    Disconnected { node_id: String },
    Error { node_id: String, error: String },
}

enum Command {
    AddConn {
        conn_id: String,
        summary: PeerSummary,
        socket: TcpStream,
        resp: oneshot::Sender<anyhow::Result<()>>,
    },

    RegisterNode {
        conn_id: String,
        node_id: String,
        summary: PeerSummary,
        resp: oneshot::Sender<anyhow::Result<()>>,
    },

    RemoveConn {
        conn_id: String,
    },

    RemoveNode {
        node_id: String,
    },

    Broadcast {
        msg: String,
    },

    SendTo {
        node_id: Option<String>,
        conn_id: Option<String>,
        msg: String,
        resp: oneshot::Sender<anyhow::Result<()>>,
    },

    GetPeers {
        resp: oneshot::Sender<Vec<PeerSummary>>,
    },
    
    GetPeer {
        node_id: String,
        resp: oneshot::Sender<anyhow::Result<PeerSummary>>,
    },

    GetConn {
        conn_id: String,
        resp: oneshot::Sender<anyhow::Result<PeerSummary>>,
    },

    ContainsListenAddr {
        listen_addr: String,
        resp: oneshot::Sender<bool>,
    },

}

#[derive(Clone)]
pub struct PeerManagerHandle {
    tx: mpsc::Sender<Command>,
    pub events_tx: mpsc::Sender<PeerEvent>,
}

impl PeerManagerHandle {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::channel::<Command>(256);
        let (events_tx, mut events_rx) = mpsc::channel::<PeerEvent>(1000);
        let events_tx_c = events_tx.clone();
        Self::spawn_peer_event_handler(events_rx);
                
        tokio::spawn(async move {
            let mut conns: HashMap<String, PeerEntry> = HashMap::new();
            let mut peers: HashMap<String, PeerEntry> = HashMap::new();

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Command::AddConn { conn_id, summary, socket, resp } => {
                        let res = (|| {
                            if conns.contains_key(&conn_id) {
                                anyhow::bail!("conn already exists");
                            }
                            let entry = PeerEntry::new(conn_id.clone(), summary, socket, events_tx_c.clone());
                            conns.insert(conn_id, entry);
                            Ok(())
                        })();
                        let _ = resp.send(res);
                    }
                    
                    Command::RegisterNode { conn_id, node_id, summary, resp } => {
                        let res = (|| {
                            if let Some(mut entry) = conns.remove(&conn_id) {
                                entry.summary.node_id = Some(node_id.clone());
                                entry.summary = summary.clone();
                                if let Some(old) = peers.remove(&node_id) {
                                    warn!("Replacing existing peer with the same node_id {} ", node_id);
                                    drop(old);
                                }
                                peers.insert(node_id.clone(), entry);
                                Ok(())
                            } else {
                                if let Some(entry) = peers.get_mut(&node_id){
                                    entry.summary = summary.clone();
                                    Ok(())
                                } else {
                                    anyhow::bail!("conn_id not found and node_id not present");
                                }
                            }
                        })();
                        let _ = resp.send(res);
                    }
                
                    Command::RemoveConn { conn_id } => {
                        if let Some(entry) = conns.remove(&conn_id) {
                            debug!("Dropping connection for {}", conn_id);
                            drop(entry);
                        } else {
                            let maybe_key = peers.iter()
                                .find_map(|(k, v)| if v.conn_id == conn_id { Some(k.clone()) } else { None });
                            if let Some(node_id) = maybe_key {
                                peers.remove(&node_id);
                                debug!("Dropped connection from peers {}", node_id)
                            } 
                        };
                    }
                    Command::RemoveNode { node_id } => {
                        if peers.remove(&node_id).is_some() {
                            debug!("Removed node {}", node_id)
                        }
                    }
                    Command::Broadcast { msg } => {
                        for (node_id, entry) in peers.iter() {
                            let send = entry.tx.clone();
                            let m = msg.clone();
                            let node_id = node_id.clone();
                            tokio::spawn(async move{
                                if let Err(e) = send.send(m.clone()).await {
                                    warn!("Failed to send a message to peer {} message {} and e {}", node_id, m, e);
                                }
                            });
                        };
                    }
                    Command::SendTo { node_id, conn_id, msg, resp } => {
                        let res = (|| {
                            if let Some(unique_id) = node_id.or(conn_id){
                                if let Some(entry) = peers.get(&unique_id){
                                    let send = entry.tx.clone();
                                    let m = msg.clone();
                                    let unique_id = unique_id.clone();
                                    tokio::spawn(async move{
                                        if let Err(e) = send.send(m.clone()).await {
                                            warn!("Failed to send a message to peer {} message {} and e {}", unique_id, m, e);
                                        }
                                    });
    
                                    Ok(())
                                } else {
                                    anyhow::bail!("Could not find node to send a message node = {} and msg = {}", unique_id, msg)
                                }
                            } else {
                                anyhow::bail!("None conn_id or node_id were passed");
                            }
                        })();
                        let _ = resp.send(res);
                    }

                    Command::GetPeers { resp } => {
                        let mut v = Vec::with_capacity(peers.len());
                        for entry in peers.values() {
                            v.push(entry.summary.clone())
                        }
                        let _ = resp.send(v);
                    }

                    Command::ContainsListenAddr { listen_addr, resp } => {
                        let mut found = false;
                        for e in peers.values() {
                            if e.summary.listen_addr.as_ref().map(|s| s.as_str()) == Some(listen_addr.as_str()) {
                                found = true; break
                            }
                        };

                        for e in conns.values() {
                            if e.summary.listen_addr.as_ref().map(|s| s.as_str()) == Some(listen_addr.as_str()) {
                                found = true; break
                            }
                        };

                        let _ = resp.send(found);
                    }

                    Command::GetConn { conn_id, resp } => {
                        let res = (|| {
                            if let Some(conn) = conns.get(&conn_id){
                                Ok(conn.summary.clone())
                            } else {
                                anyhow::bail!("No connection found by id: {}", conn_id)
                            }
                        })();
                        let _ = resp.send(res);
                    }

                    Command::GetPeer { node_id, resp } => {
                        let res = (|| {
                            if let Some(conn) = peers.get(&node_id){
                                Ok(conn.summary.clone())
                            } else {
                                anyhow::bail!("No connection found by id: {}", node_id)
                            }
                        })();
                        let _ = resp.send(res);
                    }
                }
            }
        });

        Self { tx, events_tx }
    }

    fn spawn_peer_event_handler(mut events_rx: Receiver<PeerEvent>) {
        tokio::spawn(async move {
            while let Some(event) = events_rx.recv().await {
                match event {
                    PeerEvent::Message { node_id, msg } => {
                        debug!("Received from {}: {}", node_id, msg);
                    }
                    PeerEvent::Join { conn_id, msg } => {
                        debug!("Received Join from {}: {}", conn_id, msg);
                    }
                    PeerEvent::Disconnected { node_id } => {
                        info!("Peer {} disconnected", node_id);
                    }
                    PeerEvent::Connected { node_id } => {
                        info!("Peer {} connected", node_id);
                    }
                    PeerEvent::Error { node_id, error } => {
                        error!(error);
                    }
                }
            }
        });
    }

    pub async fn add_conn(&self, conn_id: String, summary: PeerSummary, socket: TcpStream) -> anyhow::Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::AddConn { conn_id, summary, socket, resp: resp_tx };
        self.tx.send(cmd).await.map_err(|e| anyhow::anyhow!("actor stopped: {}", e))?;
        resp_rx.await.map_err(|e| anyhow::anyhow!(e))?
    }

    pub async fn register_node(&self, conn_id: String, node_id: String, summary: PeerSummary) -> anyhow::Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::RegisterNode { conn_id, node_id, summary, resp: resp_tx };
        self.tx.send(cmd).await.map_err(|e| anyhow::anyhow!("actor stopped: {}", e))?;
        resp_rx.await.map_err(|e| anyhow::anyhow!(e))?
    }

    pub async fn remove_conn(&self, conn_id: String) {
        let _ = self.tx.send(Command::RemoveConn { conn_id }).await;
    }

    pub async fn remove_node(&self, node_id: String) {
        let _ = self.tx.send(Command::RemoveNode { node_id }).await;
    }

    pub async fn broadcast(&self, msg: String) {
        let _ = self.tx.send(Command::Broadcast { msg }).await;
    }

    pub async fn send_to(&self, node_id: Option<String>, conn_id: Option<String>, msg: String) -> anyhow::Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::SendTo { node_id, conn_id, msg, resp: resp_tx };
        self.tx.send(cmd).await.map_err(|e| anyhow::anyhow!("actor stopped: {}", e))?;
        resp_rx.await.map_err(|e| anyhow::anyhow!(e))?
    }

    pub async fn get_peers(&self) -> Vec<PeerSummary> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::GetPeers { resp: resp_tx };
        if self.tx.send(cmd).await.is_err() {
            return vec![];
        }
        resp_rx.await.unwrap_or_default()
    }

    pub async fn get_peer(&self, node_id: String) -> Option<PeerSummary> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::GetPeer { node_id, resp: resp_tx };
        if self.tx.send(cmd).await.is_err() {
            return None
        }
        resp_rx.await.ok()?    
            .ok() 
    }

    pub async fn get_conn(&self, conn_id: String) -> Option<PeerSummary> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::GetConn { conn_id, resp: resp_tx };
        if self.tx.send(cmd).await.is_err() {
            return None
        }
        resp_rx.await.ok()?    
            .ok() 
    }

    pub async fn contains_listen_addr(&self, addr: String) -> bool {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::ContainsListenAddr { listen_addr: addr, resp: resp_tx };
        let _ = self.tx.send(cmd).await;
        resp_rx.await.unwrap_or(false)
    }
}
