use tokio::net::TcpStream;
use std::sync::{Arc};
use tokio::sync::Mutex;
use std::collections::HashMap;
use crate::peer::{self, Peer};
use crate::protocol::send_join;
use crate::protocol::handle_message;

pub async fn connect(addr: String, peer_table: Arc<Mutex<HashMap<String, peer::Peer>>> ) {
    match TcpStream::connect(&addr).await {
        Ok(socket) => {
            println!("Connected to {}", addr);
            let mut peer = Peer::new(addr.clone(), socket);
            
            peer_table.lock().await.insert(addr.clone(), peer.clone());
            send_join(&mut peer).await;

            loop {
                if let Err(_) = handle_message(&mut peer, &peer_table).await{
                    peer_table.lock().await.remove(&addr.clone());
                    break;
                }
            }
        }
        Err(e) => println!("Failed to connect to {}: {}", addr, e)
    }
}