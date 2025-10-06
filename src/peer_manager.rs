use crate::peer::Peer;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::{net::TcpStream, sync::Mutex};
use std::collections::HashMap;
use uuid::Uuid;

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
    pub fn uname_or_err(&self) -> anyhow::Result<String> {
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

    pub fn uname(&self) -> Option<String> {
        self.uname.clone()
    }

    pub fn uname_or_default(&self) -> String {
        self.uname.clone().unwrap_or("Stranger".to_string())
    }

    pub fn listen_addr(&self) -> Option<String> {
        self.listen_addr.clone()
    }

    pub fn node_id(&self) -> Option<String> {
        self.node_id.clone()
    }

    pub fn remote_addr(&self) -> Option<String> {
        self.remote_addr.clone()
    }
}

#[derive(Clone)]
pub struct PeerManager {
    pub peers: Arc<Mutex<HashMap<String, Arc<Mutex<Peer>>>>>, 
    pub self_peer: Arc<Mutex<Peer>>,
}

impl PeerManager {
    pub fn new(self_peer: Arc<Mutex<Peer>>) -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            self_peer
        }
    }

    pub async fn add_peer(&self, node_id: String, peer: Arc<Mutex<Peer>>) {
        let mut peers = self.peers.lock().await;
        if peers.contains_key(&node_id) {
            println!("Peer is already in list: {}", node_id);
            return;
        }

        peers.insert(node_id, peer);
    }

    pub async fn collect_peers(&self) -> Vec<PeerSummary>{
        let peer_entries: Vec<(String, Arc<Mutex<Peer>>)> = {
            let peers_guard = &self.peers.lock().await;
            peers_guard.iter()
                .map(|(node_id, peer_arc)| (node_id.clone(), peer_arc.clone()))
                .collect()
        };

        let mut summaries: Vec<PeerSummary> = Vec::new();
        for (_, peer_arc) in peer_entries {
            let peer_guard = peer_arc.lock().await;
            summaries.push(peer_guard.summary.clone());
        }

        summaries        
    }

    pub async fn contains_listen_addr(&self, listen_addr: String) -> bool {
        let peers = self.collect_peers().await;
        let s_listen_addr = Some(listen_addr);
        peers.iter().any(|peer| peer.listen_addr == s_listen_addr)
    }


    pub async fn remove_peer(&self, node_id: &str){
        let mut peers = self.peers.lock().await;
        peers.remove(node_id);
    }

    pub async fn _update_uname(&self, node_id: &str, new_uname: String) {
        let peers = self.peers.lock().await;
        if let Some(peer) = peers.get(node_id) {
            let mut peer_guard = peer.lock().await;
            peer_guard.summary.uname = Some(new_uname);
        }
    }

    pub async fn broadcast_message(&self, msg: String) {
        let peers_snapshot: Vec<Arc<Mutex<Peer>>> = {
            let peers = self.peers.lock().await;
            peers.values().cloned().collect()
        };

        for peer in peers_snapshot {
            let tx = {
            let peer_guard = peer.lock().await;
            peer_guard.tx_clone()
        };

        if let Err(e) = tx.send(msg.clone()).await {
            eprintln!("Failed to broadcast to peer: {}", e);
            }
        }
    }
    
    pub async fn list_users(&self) {
        let peers = self.peers.lock().await;
        println!("Users List:");
        for peer in peers.values() {
            let peer = peer.lock().await;
            println!("{}", peer.summary.uname_or_default());
        }
    }
}

