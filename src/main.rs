
use clap::{Parser};
use tokio::io::{self, AsyncBufReadExt};
use crate::peer_manager::{PeerManager, PeerSummary};

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
    let args = Args::parse();
    let uname = args.uname.unwrap_or_else(|| "stranger".to_string());

    let s_listen_addr = format!("127.0.0.1:{}", args.port);
    let peer_manager = PeerManager::new(Some(s_listen_addr.clone()));
    let server_pm = peer_manager.clone();
    
    //server side
    let s_info = PeerSummary { 
        listen_addr: Some(s_listen_addr),
        remote_addr:None, 
        node_id: None,
        uname: Some(uname)
    };
    
    let s_info_copy = s_info.clone();
    tokio::spawn(async move {
        if let Err(e) = server::run(s_info_copy, server_pm).await{
            eprintln!("Error on server side: {}", e)
        }
    });
    
    //client side
    if let Some(peer_addr) = args.peer{
        let client_pm = peer_manager.clone();
        let new_peer_info = PeerSummary { 
            listen_addr: Some(peer_addr),
            remote_addr: None, 
            node_id: None,
            uname: Some("Stranger".to_string()) 
        };
        tokio::spawn(async move {
            if let Err(e) = client::connect(s_info, new_peer_info, client_pm).await{
                eprintln!("Error on client side: {}", e)
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