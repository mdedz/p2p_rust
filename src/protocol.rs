use crate::peer::Peer;


pub async fn handle_message(mut peer_guard: tokio::sync::MutexGuard<'_, Peer>) -> anyhow::Result<()> {
    let (uname, msg) = peer_guard.read_message().await?;
    println!("handle message {}: {}", uname, msg);

    if msg.starts_with("JOIN"){
        let parts: Vec<&str> = msg.split("|").collect();
        if parts.len() == 3{
            let addr = parts[1].to_string();
            let uname = parts[2].to_string();

            println!("Retrieved data from new peer addr: {}; uname: {}", addr, uname);
            
            peer_guard.uname = uname;
        }
    }

    Ok(())
}

pub async fn send_join(peer_guard: tokio::sync::MutexGuard<'_, Peer>, uname: String) {
    peer_guard
        .send_message(format!("JOIN|{}|{}", peer_guard.addr.clone(), uname))
        .await
        .unwrap();
}