use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::{
    net::{TcpListener},
};
use crate::network;
use crate::{peer::Peer};
use crate::{peer_manager::PeerManager};
use crate::protocol::{send_join};


pub async fn run(port: u16, peer_manager: PeerManager, uname: &String) -> anyhow::Result<()>{
    let listener: TcpListener = TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Server is listening on {}", port);
    loop {
        let (socket, peer_e_addr) = listener.accept().await?;
        println!("New connection: {}", peer_e_addr);
        
        let uname_copy: String = uname.clone(); 
        let pm_copy = peer_manager.clone();
        
        tokio::spawn(async move {
            let e_addr_str: String = peer_e_addr.to_string();
            let peer: Arc<Mutex<Peer>> = Arc::new(Mutex::new(Peer::new(
                "".to_string(),
                socket,
                &"Stranger".to_string(),
            )));
            
            pm_copy.add_peer(e_addr_str.clone(), peer.clone()).await;

            send_join(peer.clone(), pm_copy.clone(), uname_copy).await;
            network::listen(peer.clone(), pm_copy).await;
        });
    }
}