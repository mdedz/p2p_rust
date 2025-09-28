use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::{
    net::{TcpListener},
};
use crate::{peer::Peer};
use crate::{peer_manager::PeerManager};
use crate::protocol::{handle_message, send_join};


pub async fn run(port: u16, peer_manager: PeerManager, uname: &String) -> anyhow::Result<()>{
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    println!("Server is listening on {}", port);
    loop {
        let (socket, peer_addr) = listener.accept().await?;
        println!("New connection: {}", peer_addr);
        
        let uname_copy = uname.clone(); 
        let server_pm = peer_manager.clone();

        tokio::spawn(async move {
            let addr_str = peer_addr.to_string();
            let peer = Arc::new(Mutex::new(Peer::new(
                addr_str.clone(),
                socket,
                &"Stranger".to_string(),
            )));
            
            server_pm.add_peer(addr_str.clone(), peer.clone()).await;
            
            let peer_guard = peer.lock().await;
            send_join(peer_guard, uname_copy).await;

            loop {
                let peer_guard = peer.lock().await;
                if let Err(_) = handle_message(peer_guard).await{
                    server_pm.remove_peer(&addr_str).await;
                    break;
                }
            }
        });
    }

}