use crate::network::{connect_new_peer};
use crate::peer_manager::PeerSummary;
use crate::{peer_manager::PeerManager};
use crate::protocol::send_join;

pub async fn connect(client_peer_info: PeerSummary, server_info: PeerSummary, peer_manager: PeerManager) -> anyhow::Result<()> {
    let listen_addr = server_info.listen_addr()?;

    let new_peer= connect_new_peer(listen_addr, peer_manager.clone()).await;
    if let Err(e) = new_peer {
        eprintln!("Failed to connect to peer: {}", e);
    } else{
        send_join(client_peer_info, new_peer?, peer_manager).await;
    }

    Ok(())
}