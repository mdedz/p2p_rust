use std::sync::Arc;

use tokio::{
    net::{TcpListener},
};
use crate::{peer_manager::{PeerEntry, generate_unique_id}, tls_utils::{make_acceptor, make_server_config}};
use crate::peer_manager::{PeerSummary};
use crate::{peer_manager::PeerManagerHandle};
use crate::protocol::{send_join};
use tracing::{error, debug};

pub async fn run(server_info: PeerSummary, pm: Arc<PeerManagerHandle>) -> anyhow::Result<()>{
    let listen_addr = server_info.listen_addr_or_err(7)?;
    let listener = TcpListener::bind(listen_addr.as_str()).await?;
    debug!("Server listening on {}", listen_addr);


    let maybe_acceptor = pm.tls_cert()
        .as_ref()
        .map(|c| make_acceptor(make_server_config(c).unwrap()));

    loop {
        let (socket, remote_addr) = listener.accept().await?;
        debug!("New connection: {}", remote_addr);
        
        let server_info_c = server_info.clone();
        let peer_manager = pm.clone();
        let acceptor = maybe_acceptor.clone();

        tokio::spawn(async move {
            if let Some(acceptor) = acceptor {
                match acceptor.accept(socket).await {
                    Ok(tls_stream) => {
                        let summary = PeerSummary { 
                            remote_addr: Some(remote_addr.to_string()), 
                            listen_addr: None, 
                            node_id: None, 
                            uname: None 
                        };

                        let conn_id = generate_unique_id();
                        let entry = PeerEntry::new(conn_id.clone(), summary, tls_stream, peer_manager.events_tx());
                        
                        if let Err(e) = peer_manager.add_entry(conn_id.clone(), entry).await{
                            error!("Error during server run {}", e)
                        };
                        
                        if let Err(e) = send_join(server_info_c, conn_id, peer_manager.clone()).await{
                            error!("Send join failed on server side{}", e);
                        };
                    }
                    Err(e) => {
                        error!("TLS accept failed from {}: {}", remote_addr, e)
                    }
                }
            } else {
                let summary = PeerSummary { 
                    remote_addr: Some(remote_addr.to_string()), 
                    listen_addr: None, 
                    node_id: None, 
                    uname: None 
                };
                let conn_id = generate_unique_id();

                if let Err(e) = peer_manager.add_conn(conn_id.clone(), summary, socket).await{
                    error!("Error during server run {}", e)
                };
                
                if let Err(e) = send_join(server_info_c, conn_id, peer_manager.clone()).await{
                    error!("Send join failed on server side{}", e);
                };
            }
        });
    }
}