use crate::{peer_manager::PeerManager};

pub async fn parse_command(command: &str, peer_manager:PeerManager){
    match command {
        "l" => {get_all_users(peer_manager).await;},
        _ => println!("All commands:\nl:List all users")
    }
}

pub async fn get_all_users(peer_manager:PeerManager){
    peer_manager.list_users().await;
}