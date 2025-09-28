use crate::peer::{self, Peer};
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn handle_message(peer: Arc<Mutex<Peer>>) -> anyhow::Result<()> {
    let (read_half, mut uname) = {
        let peer_guard = peer.lock().await;
        (peer_guard.read_half_clone(), peer_guard.uname_clone())
    };

    let msg = {
        peer::read_message(read_half).await?
    };

    if msg.starts_with("JOIN") {
        let parts: Vec<&str> = msg.split("|").collect();
        if parts.len() == 3 {
            let _addr = parts[1].to_string();
            uname = parts[2].to_string();
            
            let mut peer_guard = peer.lock().await;
            peer_guard.uname = uname.clone();
        }
    }
    
    println!("{}: {}", uname, msg);
    Ok(())
}

pub async fn send_join(peer: Arc<Mutex<Peer>>, uname: String) {
    let (addr, tx) = {
        let peer_guard = peer.lock().await;
        (peer_guard.addr_clone(), peer_guard.tx_clone())
    };

    if let Err(e) = tx.send(format!("JOIN|{}|{}", addr, uname)).await {
        eprintln!("send_join failed: {}", e);
    }
}