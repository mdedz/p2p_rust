use tokio::net::TcpStream;
use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::network;
use crate::{peer_manager::PeerManager};
use crate::peer::{Peer};
use crate::protocol::send_join;

pub async fn connect(addr: String, peer_manager: PeerManager, uname: String) {
    match TcpStream::connect(&addr).await {
        Ok(socket) => {
            println!("Connected to {}", addr);
            let peer = Arc::new(Mutex::new(Peer::new(addr.clone(), socket, &"Stranger".to_string())));

            peer_manager.add_peer(addr.clone(), peer.clone()).await;

            let peer_clone = peer.clone();
            send_join(peer_clone, uname).await;

            network::listen(peer.clone(), peer_manager, &addr).await;
            
        }
        Err(e) => println!("Failed to connect to {}: {}", addr, e)
    }
    send_join(peer_clone, uname).await;
}