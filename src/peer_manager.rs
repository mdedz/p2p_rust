use crate::peer::Peer;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct PeerSummary {
    pub node_id:  Option<String>,
    pub remote_addr: Option<String>,
    pub listen_addr: Option<String>,
    pub uname: Option<String>,
}

impl PeerSummary {
    pub fn uname(&self) -> anyhow::Result<String> {
        self.uname.clone()
            .ok_or_else(|| anyhow::anyhow!("uname is missing"))
    }

    pub fn listen_addr(&self) -> anyhow::Result<String> {
        self.listen_addr.clone()
            .ok_or_else(|| anyhow::anyhow!("listen_addr is missing"))
    }

    pub fn node_id(&self) -> anyhow::Result<String> {
        self.node_id.clone()
            .ok_or_else(|| anyhow::anyhow!("node_id is missing"))
    }

    pub fn remote_addr(&self) -> anyhow::Result<String> {
        self.remote_addr.clone()
            .ok_or_else(|| anyhow::anyhow!("remote_addr is missing"))
    }
}

#[derive(Clone)]
pub struct PeerManager {
    pub peers: Arc<Mutex<HashMap<String, Arc<Mutex<Peer>>>>>, 
    pub node_id: String,
    pub listen_addr: Option<String>,
}


impl PeerManager {
    pub fn new(listen_addr: Option<String>) -> Self {
        let node_id = Uuid::new_v4().to_string();
        Self {
            peers: Arc::new(Mutex::new(HashMap::new())),
            node_id,
            listen_addr
        }
    }

    pub async fn add_peer(&self, node_id: String, peer: Arc<Mutex<Peer>>) {
        let mut peers = self.peers.lock().await;
        if peers.contains_key(&node_id) {
            // println!("Peer is already in list: {}", node_id);
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
        for (node_id, peer_arc) in peer_entries {
            let peer_guard = peer_arc.lock().await;
            summaries.push(PeerSummary {
                remote_addr: peer_guard.remote_addr.clone(),
                listen_addr: peer_guard.listen_addr.clone(),
                uname: Some(peer_guard.uname.clone()),
                node_id: peer_guard.node_id.clone().or(Some(node_id)),
            });
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
            peer_guard.uname = new_uname;
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
            println!("{}", peer.uname);
        }
    }
}

