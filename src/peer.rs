
#[derive(Clone)]
pub struct Peer {
    pub summary: PeerSummary,
    tx: mpsc::Sender<String>
}

