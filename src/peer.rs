use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::{
    io::{AsyncWriteExt, AsyncReadExt},
    net::{TcpStream},
};
#[derive(Clone)]
pub struct Peer {
    pub addr: String,
    write_half: Arc<Mutex<tokio::net::tcp::OwnedWriteHalf>>,
    read_half: Arc<Mutex<tokio::net::tcp::OwnedReadHalf>>,
}

impl Peer {
    pub fn new (addr: String, socket: TcpStream) -> Self {
        let (read_half, write_half) = socket.into_split();
        Self {
            addr,
            write_half: Arc::new(Mutex::new(write_half)), 
            read_half: Arc::new(Mutex::new(read_half)),
        }
    }

    pub async fn send_message(&self, msg: String) -> anyhow::Result<()> {
        let mut socket = self.write_half.lock().await;
        socket.write_all(msg.as_bytes()).await?;
        Ok(())
    }

    pub async fn read_message(&mut self) -> anyhow::Result<String> {
        let mut buf = vec![0; 1024];
        let mut socket = self.read_half.lock().await;
        let n = socket.read(&mut buf).await?;

        Ok(String::from_utf8_lossy(&buf[..n]).to_string())
    }
}

