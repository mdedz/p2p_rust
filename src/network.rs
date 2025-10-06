use std::sync::{Arc};
use tokio::sync::Mutex;
use crate::peer_manager::{summary_to_peer, PeerSummary};
use crate::{peer::Peer};
use crate::{peer_manager::PeerManager};
use crate::protocol::{handle_message, send_join};

use tokio::net::TcpStream;
use crate::{network};


pub async fn listen(peer: Arc<Mutex<Peer>>, pm: PeerManager){
    loop {
        if let Err(e) = handle_message(pm.clone(), peer.clone()).await {
            if let Some(_) = e.downcast_ref::<std::io::Error>() {
                if let Some(id) = peer.lock().await.summary.node_id() {
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
    let pm_listen_addr = pm.self_peer.lock().await.summary.listen_addr_or_err()?;
    
    if  pm_listen_addr == listen_addr {
        anyhow::bail!("Cannot connect to itself {}", listen_addr)
    }

    if pm.contains_listen_addr(listen_addr.clone()).await {
        anyhow::bail!("Peer is already in list")
    }

    match TcpStream::connect(listen_addr.clone()).await {
        Ok(socket) => {
            let summary = PeerSummary {
                remote_addr: None,
                listen_addr: Some(l_addr_copy),
                node_id: None,
                uname: None,
            };

            let peer = summary_to_peer(summary, Some(socket));

            let peer_copy = peer.clone();
            spawn_listen(peer_copy, pm.clone());

            let client_info = pm.self_peer.lock().await.summary.clone();
            send_join(client_info, peer.clone(), pm).await?;

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
        println!("Connecting new peer {}", listen_addr);
        if let Err(e) = connect_new_peer(listen_addr, pm.clone()).await {
            eprintln!("Failed to connect to peer: {}", e);
        }
    }

    Ok(())
}

