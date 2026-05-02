use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};

// ==========================================
// ÉTAT PARTAGÉ DU NOEUD
// ==========================================
#[derive(Clone)]
pub struct NodeState {
    pub port:        u16,
    pub peers:       Arc<Mutex<Vec<String>>>,
    pub mempool:     Arc<Mutex<Vec<String>>>,
    pub block_count: Arc<Mutex<u32>>,
}

impl NodeState {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            peers:       Arc::new(Mutex::new(vec![])),
            mempool:     Arc::new(Mutex::new(vec![])),
            block_count: Arc::new(Mutex::new(0)),
        }
    }

    pub fn add_peer(&self, peer: &str) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.contains(&peer.to_string()) {
            peers.push(peer.to_string());
        }
    }

    pub fn add_to_mempool(&self, tx: &str) {
        let mut mempool = self.mempool.lock().unwrap();
        if !mempool.contains(&tx.to_string()) {
            mempool.push(tx.to_string());
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.lock().unwrap().len()
    }

    pub fn mempool_size(&self) -> usize {
        self.mempool.lock().unwrap().len()
    }

    pub fn block_count(&self) -> u32 {
        *self.block_count.lock().unwrap()
    }

    pub fn increment_blocks(&self) {
        let mut count = self.block_count.lock().unwrap();
        *count += 1;
    }
}

// ==========================================
// MESSAGES RÉSEAU
// ==========================================
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeMessage {
    Hello      { from_port: u16 },
    NewTx      { tx_data: String },
    NewBlock   { block_index: u32, hash: String },
    GetStatus,
    Status     { port: u16, peers: usize, mempool: usize, blocks: u32 },
    Ping,
    Pong,
}

// ==========================================
// SERVEUR DU NOEUD
// ==========================================
pub async fn run_node(state: NodeState) {
    let addr = format!("127.0.0.1:{}", state.port);
    let listener = TcpListener::bind(&addr).await
        .expect("Impossible de démarrer");

    println!("🟢 Noeud {} démarré sur {}", state.port, addr);

    loop {
        let (socket, peer_addr) = listener.accept().await.unwrap();
        let state_clone = state.clone();

        tokio::spawn(async move {
            handle_peer(socket, peer_addr.to_string(), state_clone).await;
        });
    }
}

async fn handle_peer(mut socket: TcpStream, peer_addr: String, state: NodeState) {
    let mut buf = vec![0u8; 4096];

    match socket.read(&mut buf).await {
        Ok(n) if n > 0 => {
            println!("📨 Message reçu de {}", peer_addr);
            let raw = String::from_utf8_lossy(&buf[..n]);

            match serde_json::from_str::<NodeMessage>(&raw) {
                Ok(msg) => {
                    let response = process_message(msg, &state).await;

                    if let Some(resp) = response {
                        let json = serde_json::to_string(&resp).unwrap();
                        let _ = socket.write_all(json.as_bytes()).await;
                    }
                }
                Err(_) => {}
            }
        }
        _ => {}
    }
}

async fn process_message(msg: NodeMessage, state: &NodeState) -> Option<NodeMessage> {
    match msg {
        NodeMessage::Hello { from_port } => {
            let peer = format!("127.0.0.1:{}", from_port);
            state.add_peer(&peer);
            println!("👋 Noeud {} : Hello de {}", state.port, from_port);
            Some(NodeMessage::Pong)
        }

        NodeMessage::NewTx { tx_data } => {
            state.add_to_mempool(&tx_data);
            println!("💸 Noeud {} : TX reçue — mempool: {}",
                state.port, state.mempool_size());
            None
        }

        NodeMessage::NewBlock { block_index, hash } => {
            state.increment_blocks();
            println!("📦 Noeud {} : Bloc {} reçu — {}...",
                state.port, block_index, &hash[..8]);
            None
        }

        NodeMessage::GetStatus => {
            Some(NodeMessage::Status {
                port:    state.port,
                peers:   state.peer_count(),
                mempool: state.mempool_size(),
                blocks:  state.block_count(),
            })
        }

        NodeMessage::Ping => Some(NodeMessage::Pong),

        _ => None,
    }
}

// ==========================================
// CLIENT — envoie un message à un noeud
// ==========================================
pub async fn send_to_node(addr: &str, msg: &NodeMessage) -> Option<NodeMessage> {
    match TcpStream::connect(addr).await {
        Ok(mut stream) => {
            let json = serde_json::to_string(msg).unwrap();
            let _ = stream.write_all(json.as_bytes()).await;

            let mut buf = vec![0u8; 4096];
            match stream.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    let raw = String::from_utf8_lossy(&buf[..n]);
                    serde_json::from_str::<NodeMessage>(&raw).ok()
                }
                _ => None,
            }
        }
        Err(_) => {
            println!("❌ Impossible de joindre {}", addr);
            None
        }
    }
}
