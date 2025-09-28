use crate::peer::Peer;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Clone)]
pub struct PeerManager {
    peers: Arc<Mutex<HashMap<String, Arc<Mutex<Peer>>>>>, 
}

impl PeerManager {
    pub fn new() -> Self {
        Self {
            peers: Arc::new(Mutex::new(HashMap::new()))
        }
    }

    pub async fn add_peer(&self, addr: String, peer: Arc<Mutex<Peer>>) {
        let mut peers = self.peers.lock().await;
        peers.insert(addr, peer);
    }

    pub async fn remove_peer(&self, addr: &str){
        let mut peers = self.peers.lock().await;
        peers.remove(addr);
    }

    pub async fn update_uname(&self, addr: &str, new_uname: String) {
        let peers = self.peers.lock().await;
        if let Some(peer) = peers.get(addr) {
            let mut peer_guard = peer.lock().await;
            peer_guard.uname = new_uname;
        }
    }

    pub async fn broadcast_message(&self, msg: String) {
        let peers_snapshot = {
            let peers = self.peers.lock().await;
            peers.values().cloned().collect::<Vec<_>>()
        };

        for peer in peers_snapshot {
            let peer = peer.lock().await;
            let _ = peer.send_message(msg.clone()).await;
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