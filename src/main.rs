
use clap::{Parser};
use tokio::io::{self, AsyncBufReadExt};
use crate::peer_manager::{create_node_id, summary_to_peer, PeerManager, PeerSummary};
use tracing::{info, warn, error, debug, trace};
use tracing_subscriber;

mod peer;
mod client;
mod server;
mod protocol;
mod ui;
mod peer_manager;
mod network;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long)]
    port: u16,
    #[arg(long)]
    peer: Option<String>,
    #[arg(long)]
    uname: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    tracing_subscriber::fmt::init();
    
    let args = Args::parse();
    let s_listen_addr = format!("127.0.0.1:{}", args.port);

    //server side
    let s_info = PeerSummary { 
        listen_addr: Some(s_listen_addr),
        remote_addr:None, 
        node_id: Some(create_node_id()),
        uname: args.uname
    };

    let server_peer = summary_to_peer(s_info.clone(), None);
    let peer_manager = PeerManager::new(server_peer);
    let server_pm = peer_manager.clone();
    
    let s_info_copy = s_info.clone();
    tokio::spawn(async move {
        if let Err(e) = server::run(s_info_copy, server_pm).await{
            error!("Error on server side: {}", e)
        }
    });
    
    //client side
    if let Some(peer_addr) = args.peer{
        let client_pm = peer_manager.clone();
        let new_peer_info = PeerSummary { 
            listen_addr: Some(peer_addr),
            remote_addr: None, 
            node_id: None,
            uname: None
        };
        tokio::spawn(async move {
            if let Err(e) = client::connect(s_info, new_peer_info, client_pm).await{
                error!("Error on client side: {}", e)
            }
        });
    }

    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line_trim = line.trim();
        
        if line_trim.is_empty() { continue; }
        if line_trim.starts_with("/"){
                let ui_pm = peer_manager.clone();
                ui::parse_command(
                    &line_trim[1..], ui_pm)
                    .await;
                continue; 
        } else{
            peer_manager.broadcast_message(format!("{}\n", line)).await;
        }
        
    }
    Ok(())
}