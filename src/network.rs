use std::sync::Arc;

use crate::peer_manager::{generate_unique_id, PeerSummary};
use crate::{peer_manager::PeerManagerHandle};
use crate::protocol::{send_join};
use tracing::{warn, error, debug};
use tokio::net::TcpStream;


pub async fn connect_new_peer(self_peer: &PeerSummary, listen_addr: String, pm: Arc<PeerManagerHandle>) -> anyhow::Result<String> {
    let l_addr_copy = listen_addr.clone();
    let pm_listen_addr = self_peer.listen_addr_or_err()?;
    
    if  pm_listen_addr == listen_addr {
        anyhow::bail!("Cannot connect to itself {}", listen_addr)
    }

    if pm.contains_listen_addr(listen_addr.clone()).await {
        anyhow::bail!("Peer is already in list")
    }

    match TcpStream::connect(listen_addr.clone()).await {
        Ok(socket) => {
            let summary = PeerSummary {
                remote_addr: None,
                listen_addr: Some(l_addr_copy),
                node_id: None,
                uname: None,
            };

            let conn_id = generate_unique_id();
            pm.add_conn(conn_id.clone(), summary, socket).await?;

            let client_info = self_peer.clone();
            send_join(client_info, conn_id.clone(), &pm).await?;

            Ok(conn_id)
        }
        Err(e) => { 
            let err_text = format!("Failed to connect to {}: {}", listen_addr, e);
            error!("{}", err_text);
            anyhow::bail!(err_text);
        }
    }
}


pub async fn handle_peer_list(pm: Arc<PeerManagerHandle>, peer_list: Vec<String>, self_peer:PeerSummary) -> anyhow::Result<()>{
    for listen_addr in peer_list {
        debug!("Connecting new peer {}", listen_addr);
        if let Err(e) = connect_new_peer(&self_peer, listen_addr, pm.clone()).await {
            warn!("Failed to connect to peer: {}", e);
        }
    }

    Ok(())
}

