use std::sync::Arc;

use tokio::{
    net::{TcpListener},
};
use crate::{peer_manager::{generate_unique_id}};
use crate::peer_manager::{PeerSummary};
use crate::{peer_manager::PeerManagerHandle};
use crate::protocol::{send_join, send_peers};
use tracing::{error, debug};

pub async fn run(server_info: PeerSummary, peer_manager: Arc<PeerManagerHandle>) -> anyhow::Result<()>{
    let server_info_copy = server_info.clone();
    let listen_addr = server_info_copy.listen_addr_or_err()?;
    
    let listener = TcpListener::bind(listen_addr.as_str()).await?;
    debug!("Server listening on {}", listen_addr);

    loop {
        let (socket, remote_addr) = listener.accept().await?;
        debug!("New connection: {}", remote_addr);
        
        let server_info_c = server_info.clone();
        let peer_manager = peer_manager.clone();
        tokio::spawn(async move {
            let summary = PeerSummary { 
                remote_addr: Some(remote_addr.to_string()), 
                listen_addr: None, 
                node_id: None, 
                uname: None 
            };

            let conn_id = generate_unique_id();
            if let Err(e) = peer_manager.add_conn(conn_id.clone(), summary, socket).await{
                error!("Error during server run {}", e)
            };
            
            if let Err(e) = send_join(server_info_c, conn_id, &peer_manager).await{
                error!("Send join failed on server side{}", e);
            };

            // send_peers(&peer_manager).await;
            // network::listen(peer.clone(), pm_copy).await;
        });
    }
}