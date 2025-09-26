use crate::peer::Peer;
use std::sync::{Arc};
use tokio::sync::Mutex;
use std::collections::HashMap;

pub async fn handle_message(peer: &mut Peer, peer_table: &Arc<Mutex<HashMap<String, Peer>>>) -> anyhow::Result<()> {
    
    let msg = peer.read_message().await?;
    println!("Received: {}", msg);

    Ok(())
}

pub async fn send_join(peer: &mut Peer) {
    peer.send_message(format!("JOIN|{}", peer.addr.clone())).await.unwrap();
}