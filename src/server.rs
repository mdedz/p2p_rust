use tokio::{
    net::{TcpListener},
};
use crate::{network, peer_manager::generate_unique_id};
use crate::peer_manager::{PeerSummary};
use crate::{peer_manager::PeerManagerHandle};
use crate::protocol::{send_join, send_peers};
use tracing::{error, debug};

pub async fn run(server_info: PeerSummary, peer_manager: PeerManagerHandle) -> anyhow::Result<()>{
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
            let conn_id = generate_unique_id();
            peer_manager.add_conn(conn_id, summary, socket);
            
            if let Err(e) = send_join(server_info_c, peer.clone(), pm_copy.clone()).await{
                error!("Send join failed on server side{}", e);
            };

            send_peers(pm_copy.clone()).await;
            network::listen(peer.clone(), pm_copy).await;
        });
    }
}