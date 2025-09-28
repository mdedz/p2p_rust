use std::sync::{Arc};
use tokio::{
    io::{AsyncWriteExt, AsyncReadExt},
    net::{TcpStream},
    sync::{mpsc, Mutex},
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Peer {
    pub addr: String,
    tx: mpsc::Sender<String>,
    read_half: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
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

pub async fn read_message(read_half: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>) -> anyhow::Result<String> {
    let mut buf = vec![0; 1024];

    let n = {
        let mut socket = read_half.lock().await;
        socket.read(&mut buf).await?
    };

    if n == 0 {
        anyhow::bail!("Connection is lost")
    }
    Ok(String::from_utf8_lossy(&buf[..n]).to_string())
}

impl Peer {
    pub fn new (addr: String, socket: TcpStream, uname: &String) -> Self{
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

        Self {
            addr,
            tx,
            read_half: Arc::new(Mutex::new(read_half)),
            uname: uname.clone(),
        }
    }
    
    pub fn read_half_clone(&self) -> Arc<Mutex<tokio::net::tcp::OwnedReadHalf>> {
        self.read_half.clone()
    }

    pub fn addr_clone(&self) -> String {
        self.addr.clone()
    }

    pub fn tx_clone(&self) -> mpsc::Sender<String> {
        self.tx.clone()
    }

    pub fn uname_clone(&self) -> String {
        self.uname.clone()
    }
}

