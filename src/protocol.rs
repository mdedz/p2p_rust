use crate::{network::handle_peer_list, peer::{self, Peer}, peer_manager::{PeerManager, PeerSummary}};
use std::{sync::Arc};
use tokio::sync::Mutex;
use serde::{Serialize, Deserialize};
use tokio::sync::mpsc::Sender;

pub async fn handle_message(peer_manager: PeerManager, peer: Arc<Mutex<Peer>>) -> anyhow::Result<()> {
    let (read_half, uname) = {
        let peer_guard = peer.lock().await;
        (peer_guard.read_half_clone(), peer_guard.uname_clone())
    };

    let msg = {
        peer::read_message(read_half).await?
    };

    if msg.starts_with("JOIN") {
        let parts: Vec<&str> = msg.split("|").collect();
        let peer_info: PeerSummary = serde_json::from_str(parts[1]).unwrap();
        let mut peer_guard = peer.lock().await;
        peer_guard.uname = peer_info.uname; 
        peer_guard.addr = peer_info.addr; 
        
    } else if msg.starts_with("PEERS|") {
        let peers_str = &msg["PEERS|".len()..];
        let mut addrs: Vec<String> = Vec::new();

        for entry in peers_str.split(';') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }

            match serde_json::from_str::<PeerSummary>(entry) {
                Ok(peer_summary) => {
                    addrs.push(peer_summary.addr);
                }
                Err(e) => {
                    eprintln!("Failed to parse peer entry '{}': {}", entry, e);
                }
            }
        }

        handle_peer_list(peer_manager, addrs).await?;
    }
    else {
        println!("{}: {}", uname, msg);
    }

    Ok(())
}

pub async fn send_join(client_info:PeerSummary, server_peer: Arc<Mutex<Peer>>, peer_manager: PeerManager) {
    let (tx, joint_payload_msg) = join_payload(server_peer, client_info).await;
    if let Err(e) = tx.send(joint_payload_msg).await {
        eprintln!("send_join failed: {}", e);
    }
    
    let (payload, peers) = peers_payload(peer_manager).await;
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

async fn join_payload(server_peer: Arc<Mutex<Peer>>, client_info:PeerSummary) -> (Sender<String>, String){
    let tx: Sender<String> = {
        let server_pg = server_peer.lock().await;
        server_pg.tx_clone()
    };
    
    let client_info_s = serde_json::to_string(&client_info).unwrap();
    ( tx, format!("JOIN|{}\n", client_info_s) )
}

pub async fn peers_payload(peer_manager: PeerManager, ) -> (String, Vec<Arc<Mutex<Peer>>>){
    let summaries = peer_manager.collect_peers().await;
    let mut payload = String::from("PEERS|");
    
    payload.push_str(
        &summaries
        .iter()
        .map(|p| serde_json::to_string(&p).unwrap()) // Используем "|" вместо ":"
        .collect::<Vec<_>>()
        .join(";")
    );
    
    let peers: Vec<Arc<Mutex<Peer>>> = {
        let guard = peer_manager.peers.lock().await;
        guard.values().cloned().collect::<Vec<_>>()
    };
    println!("{}", payload);
    (payload, peers)
}