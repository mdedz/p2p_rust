
use clap::{Parser};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::{io::{self, AsyncBufReadExt}, sync::mpsc};
use std::{fs, net::SocketAddr, sync::{Arc, Mutex}};
use crate::{peer_manager::{FrontendEvent, PeerManagerHandle, PeerSummary, generate_unique_id}, tls_utils::{TlsCert, generate_self_signed_cert}, web_api::ApiState};
use tracing::{error, debug};
use tracing_subscriber;

mod client;
mod server;
mod protocol;
mod web_api;
mod peer_manager;
mod network;
mod tls_utils;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(long)]
    port: u16,
    #[arg(long)]
    peer: Option<String>,
    #[arg(long)]
    uname: Option<String>,
    #[arg(long, default_value_t = false)]
    tls: bool,
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
    let tls_enabled = args.tls;
    
    let tls_cert: Option<Arc<TlsCert>> = if tls_enabled { 
        if fs::exists("tls/cert.der").unwrap_or(false) && fs::exists("tls/key.der").unwrap_or(false) {
            let cert_bytes = fs::read("tls/cert.der")?;
            let key_bytes = fs::read("tls/key.der")?;
            let private_key = PrivateKeyDer::try_from(key_bytes)
                                                            .map_err(|_| anyhow::anyhow!("Failed to parse private key"))?;
            Some(Arc::new(TlsCert {
                certs: vec![CertificateDer::from(cert_bytes)],
                key: private_key,
            }))
        } else {
            let tls = generate_self_signed_cert()?;
            fs::write("tls/cert.der", tls.certs[0].as_ref())?;
            fs::write("tls/key.der", tls.key.secret_der())?;
            Some(Arc::new(tls))
        }
    } else {None};

    let peer_manager = PeerManagerHandle::new(s_info.clone(), web_api_tx, tls_enabled, tls_cert);
    
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
        let s_info_copy = s_info.clone();

        tokio::spawn(async move {
            if let Err(e) = client::connect(s_info_copy, new_peer_info, client_pm).await{
                error!("Error on client side: {}", e)
            }
        });
    }

    let api_state = ApiState {
        peer_manager: peer_manager.clone(),
        clients: Arc::new(Mutex::new(Vec::new())),
    };

    let api_router = web_api::router(api_state, web_api_rx);

    let api_addr: SocketAddr = format!("127.0.0.1:{}", args.port + 100)
        .parse()
        .expect("Invalid API address");

    tokio::spawn(async move {
        debug!("Web API listening on {}", api_addr);

        let listener = tokio::net::TcpListener::bind(api_addr)
            .await
            .expect("Failed to bind API port");

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
        peer_manager.broadcast(format!("MSG|{}", line)).await;
        
    }
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
    }

    // Ok(())
}