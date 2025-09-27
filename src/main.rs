
use clap::{Parser};
use tokio::io::{self, AsyncBufReadExt};
use std::sync::{Arc};
use tokio::sync::Mutex;
use std::collections::HashMap;

mod peer;
mod client;
mod server;
mod protocol;
mod ui;

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
    let uname_clone = uname.clone();

    let peer_table: Arc<Mutex<HashMap<String, peer::Peer>>>  = Arc::new(Mutex::new(HashMap::new()));

    let server_peers = peer_table.clone();
    tokio::spawn(async move {
        server::run(args.port, server_peers, &uname).await.unwrap();
    });

    if let Some(peer_addr) = args.peer{
        let client_peers = peer_table.clone();
        tokio::spawn(async move {
            client::connect(peer_addr, client_peers.clone(), uname_clone).await;
        });
    }

    let stdin = io::BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line_trim = line.trim();
        
        if line_trim.is_empty() { continue; }
        if line_trim.starts_with("/"){ ui::parse_command(&line_trim[1..], peer_table.clone()).await; continue; }
        
        let peers_snapshot = {
            let peers = peer_table.lock().await;
            peers.clone()
        };
        for peer in peers_snapshot.values(){
            let _ = peer.send_message(line.clone()).await;
        }
    }
    Ok(())
}