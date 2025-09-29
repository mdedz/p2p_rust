use crate::peer::Peer;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Clone)]
pub struct PeerManager {
    pub peers: Arc<Mutex<HashMap<String, Arc<Mutex<Peer>>>>>, 
}

pub struct PeerSummary {
    pub addr: String,
    pub uname: String,
}


impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub async fn add_peer(&self, addr: String, peer: Arc<Mutex<Peer>>) {
        let mut peers = self.peers.lock().await;
        if peers.contains_key(&addr) {
            println!("Peer is already in list: {}", addr);
            return;
        }

        peers.insert(addr, peer);
    }

    pub async fn collect_peers(&self) -> Vec<PeerSummary>{
        let peer_entries: Vec<(String, Arc<Mutex<Peer>>)> = {
            let peers_guard = &self.peers.lock().await;
            peers_guard.iter()
                .map(|(addr, peer_arc)| (addr.clone(), peer_arc.clone()))
                .collect()
        };

        let mut summaries: Vec<(PeerSummary)> = Vec::new();
        for (addr, peer_arc) in peer_entries {
            let peer_guard = peer_arc.lock().await;
            summaries.push(PeerSummary {
                addr: addr,
                uname: peer_guard.uname.clone(),
            });
        }

        summaries        
    }

    pub async fn remove_peer(&self, addr: &str){
        let mut peers = self.peers.lock().await;
        peers.remove(addr);
    }

    pub async fn _update_uname(&self, addr: &str, new_uname: String) {
        let peers = self.peers.lock().await;
        if let Some(peer) = peers.get(addr) {
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

unsafe impl Send for PeerManager {}
unsafe impl Sync for PeerManager {}
