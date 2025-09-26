use std::sync::{Arc};
use tokio::sync::Mutex;
use std::collections::HashMap;
use tokio::{
    net::{TcpListener},
};
use crate::peer::Peer;
use crate::protocol::handle_message;


pub async fn run(port: u16, peer_table: Arc<Mutex<HashMap<String, Peer>>> ) -> anyhow::Result<()>{
    let listener = TcpListener::bind(("0.0.0.0", port)).await?;

    println!("Server is listening on {}", port);

    loop {
        let (socket, peer_addr) = listener.accept().await?;
        let peer_table = peer_table.clone();
        println!("New connection: {}", peer_addr);

        tokio::spawn(async move {
            let addr_str = peer_addr.to_string();
            let mut peer = Peer::new(addr_str.clone(), socket);
            
            peer_table.lock().await.insert(addr_str.clone(), peer.clone());

            loop {
                if let Err(_) = handle_message(&mut peer, &peer_table).await{
                    peer_table.lock().await.remove(&addr_str);
                    break;
                }
            }
        });
    }

}