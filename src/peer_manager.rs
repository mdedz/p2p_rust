use crate::peer::Peer;
use std::sync::Arc;
use anyhow::Ok;
use serde::{Deserialize, Serialize};
use tokio::{net::TcpStream, sync::{mpsc, oneshot, Mutex}};
use std::collections::HashMap;
use uuid::Uuid;
use tracing::{info, warn, error, debug, trace};

pub fn summary_to_peer(summary: PeerSummary, socket: Option<TcpStream> ) -> Arc<Mutex<Peer>>{
    Arc::new(Mutex::new(Peer::new(
        summary,
        socket,
    )))
}

pub fn create_node_id() -> String{
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
    node_id: Option<String>,         
    summary: PeerSummary,            
    tx: mpsc::Sender<String>,
}

enum Command {
    AddConn {
        conn_id: String,
        summary: PeerSummary,
        tx: mpsc::Sender<String>,
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
        node_id: String,
        msg: String,
        resp: oneshot::Sender<anyhow::Result<()>>,
    },

    GetPeers {
        resp: oneshot::Sender<Vec<PeerSummary>>,
    },
    
    ContainsListenAddr {
        listen_addr: String,
        resp: oneshot::Sender<bool>,
    },

    // ListUsers,
}

#[derive(Clone)]
pub struct PeerManagerHandle {
    tx: mpsc::Sender<Command>,
}

impl PeerManagerHandle {
    pub fn start() -> Self {
        let (tx, mut rx) = mpsc::channel::<Command>(256);

        tokio::spawn(async move {
            let mut conns: HashMap<String, PeerEntry> = HashMap::new();
            let mut peers: HashMap<String, PeerEntry> = HashMap::new();

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Command::AddConn { conn_id, summary, tx: peer_tx, resp } => {
                        let res = (|| {
                            if conns.contains_key(&conn_id) {
                                anyhow::bail!("conn already exists");
                            }
                            let entry = PeerEntry {conn_id: conn_id.clone(), node_id: None, summary, tx: peer_tx };
                            conns.insert(conn_id, entry);
                            
                            Ok(())
                        })();
                        let _ = resp.send(res);
                    }
                    
                    Command::RegisterNode { conn_id, node_id, summary, resp } => {
                        let res = (|| {
                            if let Some(mut entry) = conns.remove(&conn_id) {
                                entry.node_id = Some(node_id.clone());
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
                            tokio::spawn(async move{
                                if let Err(e) = send.send(m).await {
                                    warn!("Failed to send a message to peer {} message {}", node_id, m);
                                }
                            });
                        };
                    }
                    Command::SendTo { node_id, msg, resp } => {
                        let res = (|| {
                            if let Some(entry) = peers.get(&node_id){
                                let send = entry.tx.clone();
                                let m = msg.clone();
                                tokio::spawn(async move{
                                    if let Err(e) = send.send(m).await {
                                        warn!("Failed to send a message to peer {} message {}", node_id, m);
                                    }
                                });

                                Ok(())
                            } else {
                                anyhow::bail!("Could not find node to send a message node = {} and msg = {}", node_id, msg)
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
                            if e.summary.listen_addr == Some(listen_addr).clone() {
                                found = true; break
                            }
                        };

                        for e in conns.values() {
                            if e.summary.listen_addr == Some(listen_addr).clone() {
                                found = true; break
                            }
                        };

                        let _ = resp.send(found);
                    }
                }
            }
        });

        Self { tx }
    }

    pub async fn add_conn (&self, conn_id: String, node_id: String, summary: PeerSummary) -> anyhow::Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let cmd = Command::RegisterNode { conn_id, node_id, summary, resp: resp_tx };
        self.tx.send(cmd).await.map_err(|e| anyhow::anyhow!("actor stopped: {}", e))?;
        resp_rx.await.map_err(|e| anyhow::anyhow!(e))?
    }

}
