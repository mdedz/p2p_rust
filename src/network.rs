use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::{peer::Peer};
use crate::{peer_manager::PeerManager};
use crate::protocol::{handle_message};
use std::collections::HashMap;

use tokio::net::TcpStream;
use crate::network;

struct PeerInfo {
    addr: String,
    uname: String,
    last_seen: std::time::SystemTime,
}

type PeerMap = Arc<Mutex<HashMap<String, PeerInfo>>>;

pub async fn listen(peer: Arc<Mutex<Peer>>, pm: PeerManager, addr: &String){
    loop {
        if let Err(_) = handle_message(peer.clone()).await {
            pm.remove_peer(addr).await;
            break;
        }
    }
}

pub async fn connect_new_peer(addr: &String, pm:PeerManager) -> anyhow::Result<Arc<Mutex<Peer>>> {
    match TcpStream::connect(addr).await {
        Ok(socket) => {
            println!("Connected to {}", addr);
            let peer: Arc<Mutex<Peer>> = Arc::new(Mutex::new(Peer::new(addr.clone(), socket, &"Stranger".to_string())));

            pm.add_peer(addr.clone(), peer.clone()).await;
            network::listen(peer.clone(), pm, &addr).await;
            
            Ok(peer)
        }
        Err(e) => { 
            let err_text = format!("Failed to connect to {}: {}", addr, e);
            println!("{}", err_text);
            anyhow::bail!(err_text);
        }
    }
}

pub async fn handle_peer_list(pm:PeerManager, peer_map: PeerMap, peer_list: Vec<String>) -> anyhow::Result<()>{
    for addr in peer_list {
        let addr_copy = addr.clone();
        let peer = {
            connect_new_peer(&addr, pm.clone()).await?
        };
        
        let uname = {
            let peer_guard = peer.lock().await;
            peer_guard.uname.clone()
        };

        let peer_info = PeerInfo {
            addr: addr,
            uname: uname,
            last_seen: std::time::SystemTime::now()
        };

        let mut peer_map_guard = peer_map.lock().await;
        peer_map_guard.insert(addr_copy, peer_info);
    }
    Ok(())
}