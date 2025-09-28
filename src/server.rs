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
        let (socket, peer_addr) = listener.accept().await?;
        println!("New connection: {}", peer_addr);
        
        let uname_copy: String = uname.clone(); 

        tokio::spawn(async move {
            let addr_str: String = peer_addr.to_string();
            let peer: Arc<Mutex<Peer>> = Arc::new(Mutex::new(Peer::new(
                addr_str.clone(),
                socket,
                &"Stranger".to_string(),
            )));
            
            peer_manager.add_peer(addr_str.clone(), peer.clone()).await;

            send_join(peer.clone(), uname).await;
            network::listen(peer.clone(), peer_manager, &addr_str).await;
        });
    }
}