use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::network::{connect_new_peer};
use crate::{peer_manager::PeerManager};
use crate::peer::{Peer};
use crate::protocol::send_join;

pub async fn connect(addr: String, peer_manager: PeerManager, uname: String) -> anyhow::Result<()> {
    let peer: Arc<Mutex<Peer>> = connect_new_peer(&addr, peer_manager.clone()).await?;
    send_join(peer, peer_manager, uname).await;

    Ok(())
}