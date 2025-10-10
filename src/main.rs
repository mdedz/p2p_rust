
use clap::{Parser};
use tokio::{io::{self, AsyncBufReadExt}, sync::mpsc};
use std::net::SocketAddr;
use crate::{peer_manager::{generate_unique_id, AppState, FrontendEvent, PeerEvent, PeerManagerHandle, PeerSummary}, web_api::ApiState};
use tracing::{error, debug};
use tracing_subscriber;

mod client;
mod server;
mod protocol;
mod web_api;
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
        node_id: Some(generate_unique_id()),
        uname: args.uname
    };
    let (web_api_tx, web_api_rx) = mpsc::channel::<FrontendEvent>(1000);
    let peer_manager = PeerManagerHandle::new(s_info.clone(), web_api_tx);
    
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

    let api_state = ApiState { peer_manager: peer_manager.clone(), web_api_rx };
    let api_router = web_api::router(api_state);

    let api_addr: SocketAddr = format!("127.0.0.1:{}", args.port + 100).parse().unwrap();

    tokio::spawn(async move {
        debug!("Web API listening on {}", api_addr);

        let listener = tokio::net::TcpListener::bind(api_addr).await.unwrap();

        if let Err(e) = axum::serve(listener, api_router).await {
            error!("API server error: {}", e);
        }
    });

    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line_trim = line.trim();
        
        if line_trim.is_empty() { continue; }
        // if line_trim.starts_with("/"){
        //         let ui_pm = peer_manager.clone();
        //         ui::parse_command(
        //             &line_trim[1..], ui_pm)
        //             .await;
        //         continue; 
        // } else{
        // }
        peer_manager.broadcast(format!("MSG|{}\n", line)).await;
        
    }
    Ok(())
}