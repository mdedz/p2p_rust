use serde::{Deserialize, Serialize};
use tokio::{net::TcpStream, sync::{mpsc::{Receiver, Sender}, oneshot, RwLock}};
use std::{collections::HashMap, sync::{Arc}};
use uuid::Uuid;
use tracing::{warn, debug, error, info};
use tokio::{
    io::{AsyncRead, AsyncWrite, split, AsyncWriteExt, BufReader, AsyncBufReadExt},
    sync::{mpsc},
};

use crate::{protocol::{handle_join_json, handle_peers_json}, tls_utils};

#[derive(Clone, serde::Serialize)]
pub enum FrontendEvent {
    PeerJoined(String),
    PeerDisconnected(String),
    MessageReceived { from: String, content: String },
}

pub fn generate_unique_id() -> String{
    Uuid::new_v4().to_string()
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PeerSummary {
    pub remote_addr: Option<String>,
    pub listen_addr: Option<String>,
    pub node_id: Option<String>,
    pub uname: Option<String>,
}

impl PeerSummary {
    pub fn _uname_or_err(&self) -> anyhow::Result<String> {
        self.uname.clone()
            .ok_or_else(|| anyhow::anyhow!("uname is missing"))
    }

    pub fn listen_addr_or_err(&self, test: u16) -> anyhow::Result<String> {
        self.listen_addr.clone()
            .ok_or_else(|| anyhow::anyhow!("listen_addr is missing {}", test))
    }

}

#[derive(Clone)]
pub struct PeerEntry {
    conn_id: String,                 
    summary: Arc<RwLock<PeerSummary>>,            
    tx: mpsc::Sender<String>,
}

impl PeerEntry {
    pub fn new<S>(conn_id: String, summary:PeerSummary, socket: S, events_tx: mpsc::Sender<PeerEvent>) -> Arc<Self>
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (reader, writer) = split(socket);
        let (tx, rx) = mpsc::channel::<String>(60);
        
        let entry = Arc::new(Self { conn_id, summary: Arc::new(RwLock::new(summary)), tx });
        let entry_clone = entry.clone();

        Self::spawn_reader(entry_clone, reader, events_tx.clone());
        Self::spawn_writer(writer, rx);

        entry
    }

    pub fn spawn_writer<W>(mut writer: W, mut rx: mpsc::Receiver<String>) 
    where 
        W: AsyncWrite + Unpin + Send + 'static
    {
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

    pub fn spawn_reader<R>(self: Arc<Self>, mut reader: R, events_tx: mpsc::Sender<PeerEvent>)
    where 
        R: AsyncRead + Unpin + Send + 'static
    {
        tokio::spawn(async move {
            let mut buf = BufReader::new(&mut reader);
            let mut line = String::new();

            loop {
                line.clear();
                
                let node_id = {
                    let summary = self.summary.read().await;
                    summary.node_id.clone().unwrap_or_default()
                };

                let conn_id = self.conn_id.clone();
                
                match buf.read_line(&mut line).await {
                    Ok(0) => {
                        let _ = events_tx
                            .send(PeerEvent::Disconnected { node_id: node_id.clone() }).await;
                        break;
                    }

                    Ok(_) => {
                        let msg = line.trim().to_string();
                        if msg.is_empty() { continue; }
                        let res = (async || { 
                            if msg.starts_with("JOIN|"){
                                events_tx
                                    .send(PeerEvent::Join { conn_id: conn_id.clone(), msg: msg }).await
                            } else if msg.starts_with("PEERS|") {
                                events_tx
                                    .send(PeerEvent::Peers { msg: msg }).await
                            } else if msg.starts_with("MSG") {
                                events_tx
                                    .send(PeerEvent::Message { node_id: node_id.clone(), msg: msg }).await
                            }
                            else {
                                warn!("Could not process msg {}", msg);
                                Ok(())
                            }
                        })().await;

                        if let Err(e) = res{
                            warn!("PeerEvent channel closed {}", e);
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
    Peers { msg: String },
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
    AddEntry {
        conn_id: String,
        entry: Arc<PeerEntry>,
        resp: oneshot::Sender<anyhow::Result<()>>,
    },
    RegisterNode {
        conn_id: String,
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
    events_tx: mpsc::Sender<PeerEvent>,
    tls_enabled: bool,
    tls_cert: Option<Arc<tls_utils::TlsCert>>,
    pub self_peer_info: PeerSummary 
}

impl PeerManagerHandle {
    pub fn new(self_peer_info: PeerSummary, web_api_tx: Sender<FrontendEvent>, tls_enabled: bool, tls_cert: Option<Arc<tls_utils::TlsCert>>) -> Arc<Self>  {
        let (tx, rx) = mpsc::channel::<Command>(256);
        let (events_tx, events_rx) = mpsc::channel::<PeerEvent>(1000);

        let events_tx_c = events_tx.clone();
        let handle = Arc::new(Self { tx, events_tx: events_tx_c, tls_enabled, tls_cert, self_peer_info });
        let handle_clone = Arc::clone(&handle);

        Self::spawn_peer_event_handler(handle_clone, events_rx, web_api_tx);
        tokio::spawn(Self::command_loop(handle.clone(), rx, events_tx));

        handle
    }
    
    pub fn events_tx(&self) -> mpsc::Sender<PeerEvent> {
        self.events_tx.clone()
    }

    pub fn tls_enabled(&self) -> bool {
        self.tls_enabled.clone()
    }

    pub fn tls_cert(&self) -> Option<Arc<tls_utils::TlsCert>> {
        self.tls_cert.clone()
    }

    async fn command_loop(handle: Arc<Self>, mut rx: mpsc::Receiver<Command>, events_tx: Sender<PeerEvent>) {
        let mut conns: HashMap<String, Arc<PeerEntry>> = HashMap::new();
        let mut peers: HashMap<String, Arc<PeerEntry>> = HashMap::new();

        while let Some(cmd) = rx.recv().await {
            handle.handle_command(cmd, &mut conns, &mut peers, events_tx.clone()).await;
        }
    }

    async fn handle_command(
        &self,
        cmd: Command,
        conns: &mut HashMap<String, Arc<PeerEntry>>,
        peers: &mut HashMap<String, Arc<PeerEntry>>,
        events_tx: Sender<PeerEvent>
    ) {
        match cmd {
            Command::AddConn { conn_id, summary, socket, resp } => {
                let res = (|| {
                    if conns.contains_key(&conn_id) {
                        anyhow::bail!("conn already exists");
                    }
                    let entry = PeerEntry::new(conn_id.clone(), summary, socket, events_tx);
                    conns.insert(conn_id, entry);
                    
                    Ok(())
                })();
                let _ = resp.send(res);
            }

            Command::AddEntry { conn_id, entry, resp } => {
                let res = (|| {
                    if conns.contains_key(&conn_id) {
                        anyhow::bail!("conn already exists");
                    }
                    conns.insert(conn_id, entry);
                    Ok(())
                })();
                let _ = resp.send(res);
            }

            Command::RegisterNode { conn_id, summary, resp } => {
                let res: Result<(), anyhow::Error> = (|| async {
                    let summary_entry = summary.clone();
                    let node_id = &summary_entry
                        .node_id
                        .ok_or_else(|| anyhow::anyhow!("Node id was not specified in summary"))?
                        .clone();
                   
                    if let Some(entry) = conns.remove(&conn_id) {
                        let old_entry = Arc::clone(&entry);

                        {
                            let mut s = old_entry.summary.write().await;
                            *s = summary.clone();
                        };

                        if let Some(old) = peers.remove(node_id) {
                            warn!("Replacing existing peer with the same node_id {} ", node_id);
                            drop(old);
                        }
                        
                        peers.insert(node_id.clone(), entry);
                        Ok(())
                    } else {
                        if let Some(entry) = peers.get_mut(node_id){
                            let entry = Arc::clone(entry);

                            tokio::spawn(async move {
                                let mut s = entry.summary.write().await;
                                *s = summary.clone();
                            });
                            Ok(())
                        } else {
                            anyhow::bail!("conn_id not found and node_id not present");
                        }
                    }
                })().await;
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
                    let entry = (|| -> anyhow::Result<Option<&Arc<PeerEntry>>> {
                        if let Some(node_id) = node_id {
                            Ok(peers.get(&node_id))
                        } else if let Some(conn_id) = conn_id {
                            Ok(conns.get(&conn_id))
                        } else {
                            anyhow::bail!("No id to send_to was passed {}", msg)
                        }
                    })()?;
                    if let Some(entry) = entry{
                        let send = entry.tx.clone();
                        let m = msg.clone();
                        tokio::spawn(async move{
                            if let Err(e) = send.send(m.clone()).await {
                                warn!("Failed to send a message to peer message {} and e {}", m, e);
                            }
                        });

                        Ok(())
                    } else {
                        anyhow::bail!("Could not find node to send a messageand msg = {}", msg)
                    }
                })();
                let _ = resp.send(res);
            }

            Command::GetPeers { resp } => {
                let mut v = Vec::with_capacity(peers.len());
                for entry in peers.values() {
                    let summary = entry.summary.read().await.clone();
                    v.push(summary);
                }

                let _ = resp.send(v);
            }

            Command::ContainsListenAddr { listen_addr, resp } => {
                let mut found = false;
                for e in peers.values() {
                    let summary = e.summary.read().await;
                    if summary.listen_addr.as_ref().map(|s| s.as_str()) == Some(listen_addr.as_str()) {
                        found = true; break
                    }
                };

                for e in conns.values() {
                    let summary = e.summary.read().await;
                    if summary.listen_addr.as_ref().map(|s| s.as_str()) == Some(listen_addr.as_str()) {
                        found = true; break
                    }
                };

                let _ = resp.send(found);
            }

            Command::GetConn { conn_id, resp } => {
                let res = (|| async {
                    if let Some(entry) = conns.get(&conn_id) {
                        let summary = entry.summary.read().await.clone();
                        Ok(summary)
                    } else {
                        anyhow::bail!("No connection found by id: {}", conn_id)
                    }
                })().await;

                let _ = resp.send(res);
            }

            Command::GetPeer { node_id, resp } => {
                let res = (|| async {
                    if let Some(entry) = peers.get(&node_id){
                        let summary = entry.summary.read().await.clone();
                        Ok(summary)
                    } else {
                        anyhow::bail!("No connection found by id: {}", node_id)
                    }
                })().await;

                let _ = resp.send(res);
            }
        }
    }

    fn spawn_peer_event_handler(self: Arc<Self>, mut events_rx: Receiver<PeerEvent>, web_api_tx: Sender<FrontendEvent>) {
        tokio::spawn(async move {
            while let Some(event) = events_rx.recv().await {
                match event {
                    PeerEvent::Message { node_id, msg } => {
                        debug!("Received from {}: {}", node_id, msg);
                        let peer = self.get_peer(node_id).await.clone();
                        if let Some(peer) = peer {
                            if let Some((_, msg)) = msg.split_once("|") {
                                let uname = peer.uname.clone().unwrap_or("Stranger".to_string());
                                println!("{}: {}", uname, msg);
                                
                                let fe = FrontendEvent::MessageReceived { from: uname, content: msg.to_string() };
                                let _ = web_api_tx.send(fe).await; 
                            }
                        };
                    }
                    PeerEvent::Join { conn_id, msg } => {
                        debug!("Received Join from {}: {}", conn_id, msg);
                        let fe = FrontendEvent::PeerJoined(msg.clone());
                        let _ = web_api_tx.send(fe).await;
                        if let Err(e) = handle_join_json(self.clone(), msg.clone(), conn_id.clone()).await {
                            error!("Error during handling join {}", e)                            
                        };
                    }
                    PeerEvent::Peers { msg } => {
                        debug!("Received Peers from {}", msg);
                        if let Err(e) = handle_peers_json(self.clone(), msg.clone()).await {
                            error!("Error during handling peers {}", e)                            
                        };
                    }
                    PeerEvent::Disconnected { node_id } => {
                        self.remove_node(node_id.clone()).await;
                        info!("Peer {} disconnected", node_id);
                        let fe = FrontendEvent::PeerDisconnected(format!("{}", node_id));
                        let _ = web_api_tx.send(fe).await;
                    }
                    PeerEvent::Connected { node_id } => {
                        info!("Peer {} connected", node_id);
                    }
                    PeerEvent::Error { node_id, error } => {
                        error!("{}: {}",node_id, error);
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

    pub async fn add_entry(&self, conn_id: String, entry: Arc<PeerEntry>) -> anyhow::Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::AddEntry { conn_id, entry, resp: resp_tx };
        self.tx.send(cmd).await.map_err(|e| anyhow::anyhow!("actor stopped: {}", e))?;
        resp_rx.await.map_err(|e| anyhow::anyhow!(e))?
    }

    pub async fn register_node(&self, conn_id: String, summary: PeerSummary) -> anyhow::Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::RegisterNode { conn_id, summary, resp: resp_tx };
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
