use tokio::{
    net::{TcpListener},
};
use crate::network;
use crate::peer_manager::{summary_to_peer, PeerSummary};
use crate::{peer_manager::PeerManager};
use crate::protocol::{send_join, send_peers};
use tracing::{error, debug};

pub async fn run(server_info: PeerSummary, peer_manager: PeerManager) -> anyhow::Result<()>{
    let server_info_copy = server_info.clone();
    let listen_addr = server_info_copy.listen_addr_or_err()?;
    
    let listener = TcpListener::bind(listen_addr.as_str()).await?;
    debug!("Server listening on {}", listen_addr);

    loop {
        let (socket, remote_addr) = listener.accept().await?;
        debug!("New connection: {}", remote_addr);
        
        let pm_copy = peer_manager.clone();
        let server_info_c = server_info.clone();
        tokio::spawn(async move {
            let summary = PeerSummary { 
                remote_addr: Some(remote_addr.to_string()), 
                listen_addr: None, 
                node_id: None, 
                uname: None 
            };
            let peer = summary_to_peer(summary, Some(socket));
            
            if let Err(e) = send_join(server_info_c, peer.clone(), pm_copy.clone()).await{
                error!("Send join failed on server side{}", e);
            };

            send_peers(pm_copy.clone()).await;
            network::listen(peer.clone(), pm_copy).await;
        });
    }
}