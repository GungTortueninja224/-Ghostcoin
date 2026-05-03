use crate::miner::MinedBlock;
use crate::sync::{ChainSync, SharedChain};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

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
        let initial_count = chain.last_index();
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
        if !peers.contains(&peer.to_string()) && peers.len() < 50 {
            peers.push(peer.to_string());
        }
    }

    pub fn remove_peer(&self, peer: &str) {
        let mut peers = self.peers.lock().unwrap();
        peers.retain(|p| p != peer);
    }

    pub fn get_peers(&self) -> Vec<String> {
        self.peers
            .lock()
            .unwrap()
            .iter()
            .take(20)
            .cloned()
            .collect()
    }

    pub fn knows_peer(&self, peer: &str) -> bool {
        self.peers.lock().unwrap().iter().any(|p| p == peer)
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
        in_memory.max(self.chain.last_index())
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
        version: u32,
        height: u32,
    },
    PeerList { peers: Vec<String> },
    GetPeers,
    NewTx { tx_data: String },
    NewBlock { block_index: u32, hash: String },
    NewBlockFull { block: MinedBlock },
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
    GetBlocksSince { from_index: u32, limit: usize },
    Blocks { blocks: Vec<MinedBlock> },
    Ping,
    Pong,
}

fn canonical_peer_addr(peer_addr: &str, announced_port: u16) -> Option<String> {
    peer_addr.parse::<SocketAddr>().ok().map(|addr| {
        let canonical = SocketAddr::new(addr.ip(), announced_port);
        canonical.to_string()
    })
}

async fn read_message(socket: &mut TcpStream) -> Result<NodeMessage, String> {
    let mut len_buf = [0u8; 4];
    socket
        .read_exact(&mut len_buf)
        .await
        .map_err(|e| format!("failed to read length prefix: {}", e))?;

    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 {
        return Err("empty message".to_string());
    }
    if len > 10_000_000 {
        return Err(format!("message too large: {} bytes", len));
    }

    let mut buf = vec![0u8; len];
    socket
        .read_exact(&mut buf)
        .await
        .map_err(|e| format!("failed to read message body: {}", e))?;

    serde_json::from_slice::<NodeMessage>(&buf)
        .map_err(|e| format!("failed to deserialize message: {}", e))
}

async fn write_message(socket: &mut TcpStream, msg: &NodeMessage) -> Result<(), String> {
    let data =
        serde_json::to_vec(msg).map_err(|e| format!("failed to serialize message: {}", e))?;
    let len = (data.len() as u32).to_be_bytes();

    socket
        .write_all(&len)
        .await
        .map_err(|e| format!("failed to write length prefix: {}", e))?;
    socket
        .write_all(&data)
        .await
        .map_err(|e| format!("failed to write message body: {}", e))?;
    socket
        .flush()
        .await
        .map_err(|e| format!("failed to flush socket: {}", e))?;

    Ok(())
}

pub async fn run_node(state: NodeState) {
    let bind_host = if std::env::var("GHOSTCOIN_SERVER").is_ok() {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };
    let addr = format!("{}:{}", bind_host, state.port);
    let listener = TcpListener::bind(&addr).await.expect("failed to bind node");

    println!("Node {} listening on {}", state.port, addr);

    loop {
        let (socket, peer_addr) = listener.accept().await.unwrap();
        let state_clone = state.clone();
        tokio::spawn(async move {
            handle_peer(socket, peer_addr.to_string(), state_clone).await;
        });
    }
}

async fn handle_peer(mut socket: TcpStream, peer_addr: String, state: NodeState) {
    state.add_peer(&peer_addr);

    loop {
        match timeout(Duration::from_secs(8), read_message(&mut socket)).await {
            Ok(Ok(msg)) => {
                println!("Message received from {}", peer_addr);
                let response = process_message(msg, &state, &peer_addr).await;
                if let Some(resp) = response {
                    if let Err(e) = write_message(&mut socket, &resp).await {
                        println!("Write error to {}: {}", peer_addr, e);
                        break;
                    }
                }
            }
            Ok(Err(e)) => {
                if !e.contains("early eof") {
                    println!("Read error from {}: {}", peer_addr, e);
                }
                break;
            }
            Err(_) => {
                println!("Timeout reading from {}", peer_addr);
                break;
            }
        }
    }
}

async fn process_message(msg: NodeMessage, state: &NodeState, peer_addr: &str) -> Option<NodeMessage> {
    match msg {
        NodeMessage::Hello {
            from_port,
            version: _,
            height,
        } => {
            if let Some(canonical) = canonical_peer_addr(peer_addr, from_port) {
                state.add_peer(&canonical);
            }
            println!("Node {}: hello received", state.port);

            if height > state.chain.last_index() {
                let sync = ChainSync::new_with_chain(state.chain.clone(), vec![peer_addr.to_string()]);
                tokio::spawn(async move {
                    let _ = sync.sync_from_peers().await;
                });
            }

            Some(NodeMessage::PeerList {
                peers: state.get_peers(),
            })
        }
        NodeMessage::GetPeers => Some(NodeMessage::PeerList {
            peers: state.get_peers(),
        }),
        NodeMessage::PeerList { peers } => {
            for peer in peers {
                if peer != peer_addr && !state.knows_peer(&peer) {
                    let state_clone = state.clone();
                    tokio::spawn(async move {
                        connect_with_backoff(peer, state_clone).await;
                    });
                }
            }
            None
        }
        NodeMessage::NewTx { tx_data } => {
            state.add_to_mempool(&tx_data);
            println!("Node {}: tx received (mempool {})", state.port, state.mempool_size());
            None
        }
        NodeMessage::NewBlock { block_index, hash } => {
            state.set_block_count(block_index);
            println!(
                "Node {}: block {} received {}...",
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
                println!("Node {}: full block integrated (tip #{})", state.port, tip);
            }

            if !already_known && added > 0 {
                let peers = state.peers.lock().unwrap().clone();
                for peer in peers {
                    send_to_node_fire_and_forget(
                        &peer,
                        &NodeMessage::NewBlockFull {
                            block: block.clone(),
                        },
                    )
                    .await;
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
        NodeMessage::Blocks { blocks } => {
            println!("DEBUG Blocks recus: {} blocs", blocks.len());
            let added = state.chain.merge_blocks_from_network(blocks);
            if added > 0 {
                let tip = state.chain.last_index();
                state.set_block_count(tip);
                println!("Node {}: batch sync integrated (tip #{})", state.port, tip);
            }
            None
        }
        NodeMessage::Ping => Some(NodeMessage::Pong),
        _ => None,
    }
}

async fn connect_with_backoff(addr: String, state: NodeState) {
    let mut delay = Duration::from_secs(2);
    let max_delay = Duration::from_secs(120);
    let max_attempts = 6;

    for attempt in 1..=max_attempts {
        let hello = NodeMessage::Hello {
            from_port: state.port,
            version: 1,
            height: state.chain.last_index(),
        };

        match send_to_node(&addr, &hello).await {
            Some(NodeMessage::PeerList { peers }) => {
                println!("[peer] connected to {} after {} attempt(s)", addr, attempt);
                state.add_peer(&addr);

                for peer in peers {
                    if peer != addr && !state.knows_peer(&peer) {
                        state.add_peer(&peer);
                    }
                }

                if let Some(NodeMessage::ChainTip { last_index, .. }) =
                    send_to_node(&addr, &NodeMessage::GetChainTip).await
                {
                    if last_index > state.chain.last_index() {
                        let sync = ChainSync::new_with_chain(state.chain.clone(), vec![addr.clone()]);
                        let _ = sync.sync_from_peers().await;
                    }
                }
                return;
            }
            Some(NodeMessage::Pong) => {
                println!("[peer] connected to {} after {} attempt(s)", addr, attempt);
                state.add_peer(&addr);
                return;
            }
            _ => {
                println!(
                    "[peer {}] attempt {}/{} failed - retry in {:?}",
                    addr, attempt, max_attempts, delay
                );
                tokio::time::sleep(delay).await;
                delay = (delay * 2).min(max_delay);
            }
        }
    }

    println!("[peer {}] unreachable after {} attempts", addr, max_attempts);
    state.remove_peer(&addr);
}

pub async fn send_to_node(addr: &str, msg: &NodeMessage) -> Option<NodeMessage> {
    match TcpStream::connect(addr).await {
        Ok(mut stream) => {
            if write_message(&mut stream, msg).await.is_err() {
                return None;
            }
            match timeout(Duration::from_secs(8), read_message(&mut stream)).await {
                Ok(Ok(resp)) => Some(resp),
                _ => None,
            }
        }
        Err(_) => {
            println!("Unable to reach {}", addr);
            None
        }
    }
}

pub async fn send_to_node_fire_and_forget(addr: &str, msg: &NodeMessage) {
    match TcpStream::connect(addr).await {
        Ok(mut stream) => {
            let _ = write_message(&mut stream, msg).await;
            let _ = stream.shutdown().await;
        }
        Err(_) => {
            println!("Unable to reach {}", addr);
        }
    }
}
