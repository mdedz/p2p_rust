use crate::{network::handle_peer_list, peer::{self, Peer}, peer_manager::{PeerManager, PeerSummary}};
use std::{sync::Arc};
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;
use tracing::{info, warn, error, debug, trace};

pub async fn handle_message(peer_manager: PeerManager, peer: Arc<Mutex<Peer>>) -> anyhow::Result<()> {
    let (read_half, uname) = {
        let peer_guard = peer.lock().await;
        (peer_guard.rx_clone(), peer_guard.summary.uname_or_default())
    };

    let msg = {
        peer::read_message(read_half).await?
    };

    if msg.starts_with("JOIN") {
        let parts: Vec<&str> = msg.splitn(2, '|').collect();
        if parts.len() < 2 {
            error!("Invalid JOIN message format");
            return Ok(());
        }

        let peer_info: PeerSummary = match serde_json::from_str(parts[1]) {
            Ok(info) => info,
            Err(e) => {
                error!("Failed to parse JOIN payload: {}", e);
                return Ok(());
            }
        };

        let node_id = {
            let mut peer_guard = peer.lock().await;
            
            let node_id = peer_info.node_id_or_err()?;
            peer_guard.summary = peer_info;

            node_id
        };

        peer_manager.add_peer(node_id, peer.clone()).await;
        send_peers(peer_manager.clone()).await;

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
                    let listen_addr = peer_summary.listen_addr_or_err()?;
                    addrs.push(listen_addr);
                }
                Err(e) => {
                    error!("Failed to parse peer entry '{}': {}", entry, e);
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

pub async fn send_join(client_info:PeerSummary, server_peer: Arc<Mutex<Peer>>, peer_manager: PeerManager) -> anyhow::Result<()>{
    let node_id = peer_manager.self_peer.lock().await.summary.node_id_or_err()?;

    let tx: Sender<String> = {
        let server_pg = server_peer.lock().await;
        server_pg.tx_clone()
    };

    let msg = join_payload(client_info, node_id).await;
    
    if let Err(e) = tx.send(msg).await {
        error!("send_join failed: {}", e);
    }

    Ok(())
}

pub async fn send_peers(peer_manager: PeerManager) {
    let (payload, peers) = peers_payload(peer_manager).await;
    for peer_arc in peers {
        let msg = payload.clone();
        let tx = {
            let p = peer_arc.lock().await;
            debug!("Sending peers to {:?} with {}", p.summary.listen_addr(), payload);
            p.tx_clone()
        };
        if let Err(e) = tx.send(format!("{}\n", msg)).await {
            error!("send_join failed: {}", e);
        }
    }
}

async fn join_payload(mut client_info:PeerSummary, node_id: String) -> String{
    client_info.node_id = Some(node_id);

    let client_info_s = serde_json::to_string(&client_info).unwrap();
    format!("JOIN|{}\n", client_info_s)
}

pub async fn peers_payload(peer_manager: PeerManager, ) -> (String, Vec<Arc<Mutex<Peer>>>){
    let summaries = peer_manager.collect_peers().await;
    let mut payload = String::from("PEERS|");
    
    payload.push_str(
        &summaries
        .iter()
        .map(|p| serde_json::to_string(&p).unwrap()) 
        .collect::<Vec<_>>()
        .join(";")
    );
    
    let peers: Vec<Arc<Mutex<Peer>>> = {
        let guard = peer_manager.peers.lock().await;
        guard.values().cloned().collect::<Vec<_>>()
    };
    (payload, peers)
}