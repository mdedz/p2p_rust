use crate::{network::handle_peer_list, peer_manager::{PeerManagerHandle, PeerSummary}};
use std::{sync::Arc};
use tracing::{error, debug};

pub async fn handle_join_json(peer_manager: Arc<PeerManagerHandle>, msg: String, conn_id: String) -> anyhow::Result<()> {
        let parts: Vec<&str> = msg.splitn(2, '|').collect();
        if parts.len() < 2 {
            error!("Invalid JOIN message format");
            return Ok(());
        }

        let peer_info: PeerSummary = match serde_json::from_str(parts[1]) {
            Ok(info) => info,
            Err(e) => {
                anyhow::bail!("Failed to parse JOIN payload: {}", e);
            }
        };
        peer_manager.register_node(conn_id, peer_info).await?;
        send_peers(&peer_manager).await;

        Ok(())
}

pub async fn handle_peers_json(peer_manager: Arc<PeerManagerHandle>, msg: String) -> anyhow::Result<()> {
    let peers_str = &msg["PEERS|".len()..];
    let mut addrs: Vec<String> = Vec::new();

    for entry in peers_str.split(';') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }

        match serde_json::from_str::<PeerSummary>(entry) {
            Ok(peer_summary) => {
                let listen_addr = peer_summary.listen_addr_or_err(10)?;
                addrs.push(listen_addr);
            }
            Err(e) => {
                error!("Failed to parse peer entry '{}': {}", entry, e);
            }
        }
    }

    handle_peer_list(peer_manager, addrs).await?;
    
    Ok(())
}


pub async fn send_join(client_info:PeerSummary, server_conn_id: String, peer_manager: Arc<PeerManagerHandle>) -> anyhow::Result<()>{
    debug!("Sending join from {} to {}", client_info.clone().node_id.unwrap_or("none".to_string()), server_conn_id);
    let msg = join_payload(client_info).await;
    peer_manager.send_to(None, Some(server_conn_id), msg).await
}

pub async fn send_peers(peer_manager: &PeerManagerHandle) {
    let summaries = peer_manager.get_peers().await;
    let payload = peers_payload(&summaries).await;
    
    for peer_info in summaries {
        debug!("Sending peers to {}, peers: {}", peer_info.node_id.clone().unwrap_or_default(), payload.clone());
        let msg = payload.clone();
        if let Err(e) = peer_manager.send_to(peer_info.node_id, None, msg).await {
            error!("send_peers failed: {}", e);
        }
    }
}

async fn join_payload(client_info:PeerSummary) -> String{
    let client_info_s = serde_json::to_string(&client_info).unwrap();
    format!("JOIN|{}\n", client_info_s)
}

pub async fn peers_payload(summaries: &Vec<PeerSummary>) -> String{
    let mut payload = String::from("PEERS|");
    
    payload.push_str(
        &summaries
        .iter()
        .map(|p| serde_json::to_string(&p).unwrap()) 
        .collect::<Vec<_>>()
        .join(";")
    );
    
    payload
}