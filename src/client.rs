use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::network::{connect_new_peer};
use crate::peer_manager::PeerSummary;
use crate::{peer_manager::PeerManager};
use crate::peer::{Peer};
use crate::protocol::send_join;

pub async fn connect(client_peer_info: PeerSummary, server_info: PeerSummary, peer_manager: PeerManager) -> anyhow::Result<()> {
    let new_peer: Arc<Mutex<Peer>> = connect_new_peer(&server_info.addr, peer_manager.clone()).await?;
    send_join(peer, peer_manager, uname).await;

    Ok(())
}