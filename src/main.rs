
use clap::{Parser};
use tokio::io::{self, AsyncBufReadExt};
use crate::peer_manager::PeerManager;

mod peer;
mod client;
mod server;
mod protocol;
mod ui;
mod peer_manager;

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

    let peer_manager = PeerManager::new();

    let server_pm = peer_manager.clone();
    tokio::spawn(async move {
        server::run(args.port, server_pm, &uname).await.unwrap();
    });
    
    if let Some(peer_addr) = args.peer{
        let client_pm = peer_manager.clone();
        tokio::spawn(async move {
            client::connect(peer_addr, client_pm, uname_clone).await;
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
        }
        peer_manager.broadcast_message(line);
    }
    Ok(())
}