use crate::peer::Peer;
use std::sync::{Arc};
use tokio::sync::Mutex;
use std::collections::HashMap;

pub async fn parse_command(command: &str, peer_table: Arc<Mutex<HashMap<String, Peer>>>){
    match command {
        "l" => {get_all_users(peer_table).await;},
        _ => println!("All commands:\nl:List all users")
    }
}

pub async fn get_all_users(peer_table: Arc<Mutex<HashMap<String, Peer>>>){
    let peers = peer_table.lock().await;
    println!("Users List");
    for peer in peers.values(){
        println!("{}", peer.uname)
    }
}