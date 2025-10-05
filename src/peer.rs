use std::sync::{Arc};
use tokio::{
    io::{AsyncWriteExt, BufReader, AsyncBufReadExt},
    net::{TcpStream},
    sync::{mpsc, Mutex},
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Peer {
    pub node_id: Option<String>,
    pub remote_addr: Option<String>,
    pub listen_addr: Option<String>,
    tx: mpsc::Sender<String>,
    read_half: Arc<Mutex<BufReader<tokio::net::tcp::OwnedReadHalf>>>,
    pub uname: String 
}

async fn _uname_is_unique(uname: String, peer_table: Arc<Mutex<HashMap<String, Arc<Mutex<Peer>> >>>) -> bool{
    let peers_snapshot: Vec<Arc<Mutex<Peer>>> = {
        let peers = peer_table.lock().await;
        peers.values().cloned().collect()
    };

    for p in peers_snapshot {
        let peer = p.lock().await;
        if peer.uname == uname {
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
    pub fn new (remote_addr:Option<String>, listen_addr: Option<String>, socket: TcpStream, uname: &String, node_id:Option<String> ) -> Self{
        let (read_half, write_half) = socket.into_split();
        
        let (tx, mut rx) = mpsc::channel::<String>(100);
        let write_half = Arc::new(Mutex::new(write_half));
        
        let write_half_clone = write_half.clone();
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let mut socket = write_half_clone.lock().await;
                if let Err(e) = socket.write_all(msg.as_bytes()).await {
                    eprintln!("Failed to send message: {}", e);
                    break;
                }
            }
        });
        let buf_reader = BufReader::new(read_half);
        Self {
            node_id,
            remote_addr,
            listen_addr,
            tx,
            read_half: Arc::new(Mutex::new(buf_reader)),
            uname: uname.clone(),
        }
    }
    
    pub fn read_half_clone(&self) -> Arc<Mutex<BufReader<tokio::net::tcp::OwnedReadHalf>>> {
        self.read_half.clone()
    }

    pub fn tx_clone(&self) -> mpsc::Sender<String> {
        self.tx.clone()
    }

    pub fn uname_clone(&self) -> String {
        self.uname.clone()
    }
}
