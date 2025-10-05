use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::{peer::Peer};
use crate::{peer_manager::PeerManager};
use crate::protocol::{handle_message};

use tokio::net::TcpStream;
use crate::{network};


pub async fn listen(peer: Arc<Mutex<Peer>>, pm: PeerManager){
    loop {
        if let Err(e) = handle_message(pm.clone(), peer.clone()).await {
            if let Some(_) = e.downcast_ref::<std::io::Error>() {
                if let Some(id) = peer.lock().await.node_id.clone() {
                    pm.remove_peer(&id).await;
                }
                break;
            } else {
                eprintln!("protocol error: {}", e);
                break;
            }
        }
    }
}

pub async fn connect_new_peer(listen_addr: String, pm:PeerManager) -> anyhow::Result<Arc<Mutex<Peer>>> {
    let l_addr_copy = listen_addr.clone();
    let pm_listen_addr = pm.listen_addr.clone().ok_or_else(|| anyhow::anyhow!("No listen address was specified"))?;
    
    if  pm_listen_addr == listen_addr {
        anyhow::bail!("Cannot connect to itself {}", listen_addr)
    }

    if pm.contains_listen_addr(listen_addr.clone()).await {
        anyhow::bail!("Peer is already in list")
    }

    match TcpStream::connect(listen_addr.clone()).await {
        Ok(socket) => {
            let peer: Arc<Mutex<Peer>> = Arc::new(Mutex::new(Peer::new(
                None, 
                Some(l_addr_copy), 
                socket, 
                &"Stranger".to_string(),
                None
            )));

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
        // println!("Connecting new peer {}", listen_addr);
        if let Err(e) = connect_new_peer(listen_addr, pm.clone()).await {
            // eprintln!("Failed to connect to peer: {}", e);
        }
    }

    Ok(())
}

