use std::sync::{Arc};
use anyhow::bail;
use tokio::sync::Mutex;
use tokio::{
    net::{TcpListener},
};
use crate::network;
use crate::peer_manager::PeerSummary;
use crate::{peer::Peer};
use crate::{peer_manager::PeerManager};
use crate::protocol::{send_join};


pub async fn run(server_info: PeerSummary, peer_manager: PeerManager) -> anyhow::Result<()>{
    let splitted_addr: Vec<&str> = server_info.addr.split(":").collect();
    let port_str = splitted_addr
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("No port provided"))?;

    let port: u16 = port_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid port number"))?;

    let listener: TcpListener = TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Server is listening on {}", port);

    loop {
        let (socket, peer_e_addr) = listener.accept().await?;
        println!("New connection: {}", peer_e_addr);
        
        let pm_copy = peer_manager.clone();
        let server_info_c = server_info.clone();
        tokio::spawn(async move {
            let peer: Arc<Mutex<Peer>> = Arc::new(Mutex::new(Peer::new(
                "".to_string(),
                socket,
                &"Stranger".to_string(),
            )));
            
            pm_copy.add_peer(peer_e_addr.to_string().clone(), peer.clone()).await;
            
            send_join(server_info_c, peer.clone(), pm_copy.clone()).await;

            network::listen(peer.clone(), pm_copy).await;
        });
    }
}