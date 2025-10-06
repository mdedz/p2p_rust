use std::sync::{Arc};
use tokio::{
    io::{AsyncWriteExt, BufReader, AsyncBufReadExt},
    net::{TcpStream},
    sync::{mpsc, Mutex},
};
use std::collections::HashMap;
use crate::peer_manager::PeerSummary;
use tracing::{info, warn, error, debug, trace};

#[derive(Clone)]
pub struct TxRx {
    tx: mpsc::Sender<String>,
    rx: Arc<Mutex<BufReader<tokio::net::tcp::OwnedReadHalf>>>,
}

#[derive(Clone)]
pub struct Peer {
    pub summary: PeerSummary,
    pub tx_rx: Option<TxRx>
}

async fn _uname_is_unique(uname: String, peer_table: Arc<Mutex<HashMap<String, Arc<Mutex<Peer>> >>>) -> bool{
    let peers_snapshot: Vec<Arc<Mutex<Peer>>> = {
        let peers = peer_table.lock().await;
        peers.values().cloned().collect()
    };

    for p in peers_snapshot {
        let peer = p.lock().await;
        if peer.summary.uname == Some(uname.clone()) {
            return false;
        }
    }
    true
}

pub async fn read_message(reader: Arc<Mutex<BufReader<tokio::net::tcp::OwnedReadHalf>>>) -> anyhow::Result<String> {
    let mut line = String::new();
    let mut guard = reader.lock().await;

    let n = guard.read_line(&mut line).await?;
    if n == 0 {
        anyhow::bail!(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Connection is lost"));
    }
    let msg = line.trim_end().to_string();
    Ok(msg)
}

impl Peer {
    pub fn new (summary:PeerSummary, socket: Option<TcpStream>) -> Self{
        if let Some(socket) = socket{
            let (read_half, write_half) = socket.into_split();
            
            let (tx, mut rx) = mpsc::channel::<String>(100);
            let write_half = Arc::new(Mutex::new(write_half));
            
            let write_half_clone = write_half.clone();
            tokio::spawn(async move {
                while let Some(msg) = rx.recv().await {
                    let mut socket = write_half_clone.lock().await;
                    if let Err(e) = socket.write_all(msg.as_bytes()).await {
                        error!("Failed to send message: {}", e);
                        break;
                    }
                }
            });
            let buf_reader = BufReader::new(read_half);

            let rx = Arc::new(Mutex::new(buf_reader));
            let tx_rx = Some(TxRx { tx, rx });

            Self {
                summary,
                tx_rx
            }
        } else {
            Self {
                summary,
                tx_rx: None,
            }
        }


    }
    
    pub fn rx_clone(&self) -> Arc<Mutex<BufReader<tokio::net::tcp::OwnedReadHalf>>> {
        self.tx_rx.as_ref().unwrap().rx.clone()
    }

    pub fn tx_clone(&self) -> mpsc::Sender<String> {
        self.tx_rx.as_ref().unwrap().tx.clone()
    }

}
