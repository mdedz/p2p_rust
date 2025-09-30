use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::{peer::Peer};
use crate::{peer_manager::PeerManager};
use crate::protocol::{handle_message};

use tokio::net::TcpStream;
use crate::{network};


pub async fn listen(peer: Arc<Mutex<Peer>>, pm: PeerManager){
    loop {
        if let Err(_) = handle_message(pm.clone(), peer.clone()).await {
            let node_id = {
                let peer_guard = peer.lock().await;
                peer_guard.node_id.clone()
            };

            if let Some(id) = node_id {
                pm.remove_peer(&id).await;
            }
            
            break;
        }
    }
}

pub async fn connect_new_peer(listen_addr: String, pm:PeerManager) -> anyhow::Result<Arc<Mutex<Peer>>> {
    let l_addr_copy = listen_addr.clone();
    match TcpStream::connect(listen_addr.clone()).await {
        Ok(socket) => {
            println!("Connected to {}", l_addr_copy);

            let peer: Arc<Mutex<Peer>> = Arc::new(Mutex::new(Peer::new(
                None, 
                Some(l_addr_copy), 
                socket, 
                &"Stranger".to_string(),
                None
            )));
            // pm.add_peer(addr.clone(), peer.clone()).await;

            let peer_copy = peer.clone();
            spawn_listen(peer_copy, pm);
            
            Ok(peer)
        }
        Err(e) => { 
            let err_text = format!("Failed to connect to {}: {}", listen_addr, e);
            println!("{}", err_text);
            anyhow::bail!(err_text);
        }
    }
}

fn spawn_listen(peer: Arc<Mutex<Peer>>, pm: PeerManager) {
    tokio::spawn(async move {
        network::listen(peer, pm.clone()).await;
    });
}

pub async fn handle_peer_list(pm:PeerManager, peer_list: Vec<String>) -> anyhow::Result<()>{
    for listen_addr in peer_list {
        if let Err(e) = connect_new_peer(listen_addr, pm.clone()).await {
            eprintln!("Failed to connect to peer: {}", e);
        }
    }
        // let uname = {
        //     let peer_guard = peer.lock().await;
        //     peer_guard.uname.clone()
        // };

        // let peer_info = PeerInfo {
        //     addr: addr,
        //     uname: uname,
        //     last_seen: std::time::SystemTime::now()
        // };

        // let mut peer_map_guard = peer_map.lock().await;
        // peer_map_guard.insert(addr_copy, peer_info);
    Ok(())
}

