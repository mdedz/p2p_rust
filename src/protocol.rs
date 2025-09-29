use crate::{network::handle_peer_list, peer::{self, Peer}, peer_manager::{PeerManager, PeerSummary}};
use std::{sync::Arc};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};


pub async fn handle_message(peer_manager: PeerManager, peer: Arc<Mutex<Peer>>) -> anyhow::Result<()> {
    let (read_half, uname) = {
        let peer_guard = peer.lock().await;
        (peer_guard.read_half_clone(), peer_guard.uname_clone())
    };

    let msg = {
        peer::read_message(read_half).await?
    };

    if msg.starts_with("JOIN") {
        // let parts: Vec<&str> = msg.splitn(2, '|').collect();
        let parts: Vec<&str> = msg.split("|").collect();
        if parts.len() == 3 {
            let peer_info: PeerSummary = serde_json::from_str(parts[1]).unwrap();
            let mut peer_guard = peer.lock().await;
            peer_guard.uname = peer_info.uname; 
            peer_guard.addr = peer_info.addr; 
        }
        

        // let parts: Vec<&str> = msg.split("|").collect();
        // if parts.len() == 3 {
        //     let _addr = parts[1].to_string();
        //     uname = parts[2].to_string();
            
        //     let mut peer_guard = peer.lock().await;
        //     peer_guard.uname = uname.clone();
        // }
    }
    
    if msg.starts_with("PEERS|") {
        let peers_str = &msg["PEERS|".len()..];
        let mut addrs: Vec<String> = Vec::new();

        for entry in peers_str.split(';') {
            if entry.trim().is_empty() {
                continue;
            }

            if let Some((addr, _uname)) = entry.split_once('|') { 
                addrs.push(addr.to_string());
            }
        }

        handle_peer_list(peer_manager, addrs).await?;
    }

    println!("{}: {}", uname, msg);
    Ok(())
}

pub async fn send_join(peer: Arc<Mutex<Peer>>, peer_manager: PeerManager, uname: String) {
    let (addr, tx) = {
        let peer_guard = peer.lock().await;
        (peer_guard.addr_clone(), peer_guard.tx_clone())
    };

    let peer_info = PeerSummary { addr, uname };
    let serialized = serde_json::to_string(&peer_info).unwrap();

    if let Err(e) = tx.send(format!("JOIN|{}\n", serialized)).await {
        eprintln!("send_join failed: {}", e);
    }
    
    let summaries = peer_manager.collect_peers().await;
    let mut payload = String::from("PEERS|");

    payload.push_str(
        &summaries
        .iter()
        .map(|p| serde_json::to_string(&p).unwrap()) // Используем "|" вместо ":"
        .collect::<Vec<_>>()
        .join(";")
    );

    let peers = {
        let guard = peer_manager.peers.lock().await;
        guard.values().cloned().collect::<Vec<_>>()
    };

    for peer_arc in peers {
        let msg = payload.clone();
        let tx = {
            let p = peer_arc.lock().await;
            p.tx_clone()
        };
        if let Err(e) = tx.send(format!("{}\n", msg)).await {
            eprintln!("send_join failed: {}", e);
        }
    }
}

