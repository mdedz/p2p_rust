use std::sync::Arc;

use crate::network::{connect_new_peer};
use crate::peer_manager::PeerSummary;
use crate::{peer_manager::PeerManagerHandle};
// use crate::protocol::{send_join, send_peers};
use tracing::{warn};

pub async fn connect(client_peer_info: PeerSummary, server_info: PeerSummary, peer_manager: Arc<PeerManagerHandle>) -> anyhow::Result<()> {
    let listen_addr = server_info.listen_addr_or_err()?;

    let new_peer= connect_new_peer(&client_peer_info, listen_addr, peer_manager.clone()).await;
    if let Err(e) = new_peer {
        warn!("Failed to connect to peer: {}", e);
    } else{
        // send_join(client_peer_info, new_peer?, &peer_manager).await?;
        // send_peers(&peer_manager).await;
    }

    Ok(())
}