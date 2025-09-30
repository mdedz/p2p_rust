use std::sync::{Arc};
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
    let server_info_copy = server_info.clone();
    let listen_addr = server_info_copy.listen_addr.unwrap();
    
    let splitted_addr: Vec<&str> = listen_addr.split(":").collect();
    let port_str = splitted_addr
        .get(1)
        .ok_or_else(|| anyhow::anyhow!("No port provided"))?;

    let port: u16 = port_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid port number"))?;

    let listener: TcpListener = TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Server is listening on {}", port);

    loop {
        let (socket, remote_addr) = listener.accept().await?;
        println!("New connection: {}", remote_addr);
        
        let pm_copy = peer_manager.clone();
        let server_info_c = server_info.clone();
        tokio::spawn(async move {
            println!("{}", remote_addr.to_string());

            let peer: Arc<Mutex<Peer>> = Arc::new(Mutex::new(Peer::new(
                Some(remote_addr.to_string()),
                None,
                socket,
                &"Stranger".to_string(),
                None
            )));
            
            //pm_copy.add_peer(peer_e_addr.to_string().clone(), peer.clone()).await;
            
            send_join(server_info_c, peer.clone(), pm_copy.clone()).await;

            network::listen(peer.clone(), pm_copy).await;
        });
    }
}