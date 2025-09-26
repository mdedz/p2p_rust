use std::sync::{Arc};
use anyhow::anyhow;
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncWriteExt, AsyncReadExt},
    net::{TcpStream},
};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Peer {
    pub addr: String,
    write_half: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    read_half: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
    pub uname: String //unique
}

async fn _uname_is_unique(uname: String, peer_table: Arc<Mutex<HashMap<String, Peer>>>) -> bool{
    let peers = peer_table.lock().await;
    !peers.values().any(|p| p.uname == uname)
}

impl Peer {
    pub fn new (addr: String, socket: TcpStream, uname: &String, peer_table: Arc<Mutex<HashMap<String, Peer>>>) -> anyhow::Result<Self> {
        let (read_half, write_half) = socket.into_split();
        if !_uname_is_unique(*uname, peer_table.clone()) {
            return Err(anyhow!("Username is already taken: {}", uname));
        }

        let (read_half, write_half) = socket.into_split();

        Ok(Self {
            addr,
            write_half: Arc::new(Mutex::new(write_half)),
            read_half: Arc::new(Mutex::new(read_half)),
            uname: uname.clone(),
        })
    }
    

    pub async fn send_message(&self, msg: String) -> anyhow::Result<()> {
        let mut socket = self.write_half.lock().await;
        socket.write_all(msg.as_bytes()).await?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> anyhow::Result<(&String, String)> {
        let mut buf = vec![0; 1024];
        let mut socket = self.read_half.lock().await;
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            anyhow::bail!("Connection is lost")
        }
        Ok((&self.uname, String::from_utf8_lossy(&buf[..n]).to_string()))
    }
}

