use crate::miner::MinedBlock;
use crate::sync::SharedChain;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Clone)]
pub struct NodeState {
    pub port: u16,
    pub peers: Arc<Mutex<Vec<String>>>,
    pub mempool: Arc<Mutex<Vec<String>>>,
    pub block_count: Arc<Mutex<u32>>,
    pub chain: SharedChain,
}

impl NodeState {
    pub fn new(port: u16, chain: SharedChain) -> Self {
        let initial_count = chain.length() as u32;
        Self {
            port,
            peers: Arc::new(Mutex::new(vec![])),
            mempool: Arc::new(Mutex::new(vec![])),
            block_count: Arc::new(Mutex::new(initial_count)),
            chain,
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
        let in_memory = *self.block_count.lock().unwrap();
        in_memory.max(self.chain.length() as u32)
    }

    pub fn set_block_count(&self, count: u32) {
        let mut current = self.block_count.lock().unwrap();
        if count > *current {
            *current = count;
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum NodeMessage {
    Hello {
        from_port: u16,
    },
    NewTx {
        tx_data: String,
    },
    NewBlock {
        block_index: u32,
        hash: String,
    },
    NewBlockFull {
        block: MinedBlock,
    },
    GetStatus,
    Status {
        port: u16,
        peers: usize,
        mempool: usize,
        blocks: u32,
    },
    GetChainTip,
    ChainTip {
        last_index: u32,
        last_hash: String,
    },
    GetBlocksSince {
        from_index: u32,
        limit: usize,
    },
    Blocks {
        blocks: Vec<MinedBlock>,
    },
    Ping,
    Pong,
}

pub async fn run_node(state: NodeState) {
    let addr = format!("127.0.0.1:{}", state.port);
    let listener = TcpListener::bind(&addr)
        .await
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

            if let Ok(msg) = serde_json::from_str::<NodeMessage>(&raw) {
                let response = process_message(msg, &state).await;

                if let Some(resp) = response {
                    let json = serde_json::to_string(&resp).unwrap();
                    let _ = socket.write_all(json.as_bytes()).await;
                }
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
            println!(
                "💸 Noeud {} : TX reçue — mempool: {}",
                state.port,
                state.mempool_size()
            );
            None
        }

        NodeMessage::NewBlock { block_index, hash } => {
            state.set_block_count(block_index);
            println!(
                "📦 Noeud {} : Bloc {} reçu — {}...",
                state.port,
                block_index,
                &hash[..8.min(hash.len())]
            );
            None
        }

        NodeMessage::NewBlockFull { block } => {
            let block_hash = block.hash.clone();
            let already_known = state.chain.has_block_hash(&block_hash);
            let added = state.chain.merge_blocks_from_network(vec![block.clone()]);

            if added > 0 {
                let tip = state.chain.last_index();
                state.set_block_count(tip);
                println!(
                    "🔗 Noeud {} : bloc complet intégré (hauteur locale: #{})",
                    state.port, tip
                );
            }

            if !already_known && added > 0 {
                let peers = state.peers.lock().unwrap().clone();
                for peer in peers {
                    let _ = send_to_node(&peer, &NodeMessage::NewBlockFull { block: block.clone() }).await;
                }
            }

            None
        }

        NodeMessage::GetStatus => Some(NodeMessage::Status {
            port: state.port,
            peers: state.peer_count(),
            mempool: state.mempool_size(),
            blocks: state.block_count(),
        }),

        NodeMessage::GetChainTip => {
            let (last_index, last_hash) = state.chain.tip();
            Some(NodeMessage::ChainTip {
                last_index,
                last_hash,
            })
        }

        NodeMessage::GetBlocksSince { from_index, limit } => {
            let blocks = state.chain.get_blocks_since(from_index, limit);
            Some(NodeMessage::Blocks { blocks })
        }

        NodeMessage::Ping => Some(NodeMessage::Pong),

        _ => None,
    }
}

pub async fn send_to_node(addr: &str, msg: &NodeMessage) -> Option<NodeMessage> {
    match TcpStream::connect(addr).await {
        Ok(mut stream) => {
            let json = serde_json::to_string(msg).unwrap();
            if stream.write_all(json.as_bytes()).await.is_err() {
                return None;
            }
            let _ = stream.shutdown().await;

            let mut buf = Vec::new();
            match stream.read_to_end(&mut buf).await {
                Ok(n) if n > 0 => serde_json::from_slice::<NodeMessage>(&buf[..n]).ok(),
                _ => None,
            }
        }
        Err(_) => {
            println!("❌ Impossible de joindre {}", addr);
            None
        }
    }
}
