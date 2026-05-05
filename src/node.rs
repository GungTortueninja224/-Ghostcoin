use crate::miner::MinedBlock;
use crate::mempool::{Mempool, MempoolTx};
use crate::sync::{ChainSync, SharedChain};
use crate::config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

#[derive(Clone)]
pub struct NodeState {
    pub port: u16,
    pub peers: Arc<Mutex<Vec<String>>>,
    pub peer_sessions: Arc<Mutex<HashMap<String, String>>>,
    pub block_count: Arc<Mutex<u32>>,
    pub chain: SharedChain,
}

impl NodeState {
    pub fn new(port: u16, chain: SharedChain) -> Self {
        let initial_count = chain.last_index();
        Self {
            port,
            peers: Arc::new(Mutex::new(vec![])),
            peer_sessions: Arc::new(Mutex::new(HashMap::new())),
            block_count: Arc::new(Mutex::new(initial_count)),
            chain,
        }
    }

    pub fn add_peer(&self, peer: &str) {
        let mut peers = self.peers.lock().expect("node peers mutex poisoned");
        if !peers.contains(&peer.to_string()) && peers.len() < 50 {
            peers.push(peer.to_string());
        }
    }

    pub fn remove_peer(&self, peer: &str) {
        let mut peers = self.peers.lock().expect("node peers mutex poisoned");
        peers.retain(|p| p != peer);
    }

    pub fn register_peer_session(&self, session_addr: &str, canonical_peer: &str) {
        let mut sessions = self
            .peer_sessions
            .lock()
            .expect("peer session registry mutex poisoned");
        sessions.insert(session_addr.to_string(), canonical_peer.to_string());
    }

    pub fn remove_peer_session(&self, session_addr: &str) {
        self.remove_peer(session_addr);

        let peer_to_remove = {
            let mut sessions = self
                .peer_sessions
                .lock()
                .expect("peer session registry mutex poisoned");
            sessions.remove(session_addr)
        };

        if let Some(peer) = peer_to_remove {
            self.remove_peer(&peer);
        }
    }

    pub fn get_peers(&self) -> Vec<String> {
        self.peers
            .lock()
            .expect("node peers mutex poisoned")
            .iter()
            .take(20)
            .cloned()
            .collect()
    }

    pub fn knows_peer(&self, peer: &str) -> bool {
        self.peers
            .lock()
            .expect("node peers mutex poisoned")
            .iter()
            .any(|p| p == peer)
    }

    pub fn add_to_mempool(&self, tx: MempoolTx) -> bool {
        Mempool::insert_persisted(tx)
    }

    pub fn peer_count(&self) -> usize {
        self.peers.lock().expect("node peers mutex poisoned").len()
    }

    pub fn mempool_size(&self) -> usize {
        Mempool::load().pending_count()
    }

    pub fn block_count(&self) -> u32 {
        let in_memory = *self
            .block_count
            .lock()
            .expect("node block_count mutex poisoned");
        in_memory.max(self.chain.last_index())
    }

    pub fn set_block_count(&self, count: u32) {
        let mut current = self
            .block_count
            .lock()
            .expect("node block_count mutex poisoned");
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
    NewTx { tx: MempoolTx },
    TxAck {
        tx_id: String,
        accepted: bool,
        mempool: usize,
    },
    GetMempool { limit: usize },
    MempoolSnapshot { txs: Vec<MempoolTx> },
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
    let bind_host = if config::is_server() {
        "0.0.0.0"
    } else {
        "127.0.0.1"
    };
    let addr = format!("{}:{}", bind_host, state.port);
    let listener = match TcpListener::bind(&addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("failed to bind node on {}: {}", addr, e);
            return;
        }
    };

    println!("Node {} listening on {}", state.port, addr);

    loop {
        let (socket, peer_addr) = match listener.accept().await {
            Ok(conn) => conn,
            Err(e) => {
                eprintln!("accept error on {}: {}", addr, e);
                continue;
            }
        };
        let state_clone = state.clone();
        tokio::spawn(async move {
            handle_peer(socket, peer_addr.to_string(), state_clone).await;
        });
    }
}

async fn handle_peer(mut socket: TcpStream, peer_addr: String, state: NodeState) {
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

    state.remove_peer_session(&peer_addr);
}

async fn process_message(msg: NodeMessage, state: &NodeState, peer_addr: &str) -> Option<NodeMessage> {
    match msg {
        NodeMessage::Hello {
            from_port,
            version: _,
            height,
        } => {
            let canonical = canonical_peer_addr(peer_addr, from_port);
            if let Some(canonical) = canonical.clone() {
                state.add_peer(&canonical);
                state.register_peer_session(peer_addr, &canonical);
            }
            println!("Node {}: hello received", state.port);

            if height > state.chain.last_index() {
                let sync_peer = canonical.unwrap_or_else(|| peer_addr.to_string());
                let sync = ChainSync::new_with_chain(state.chain.clone(), vec![sync_peer]);
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
        NodeMessage::GetMempool { limit } => {
            let snapshot_limit = limit.clamp(1, crate::mempool::MAX_MEMPOOL_TXS);
            let txs = match tokio::task::spawn_blocking(move || {
                Mempool::snapshot_pending(snapshot_limit)
            })
            .await
            {
                Ok(txs) => txs,
                Err(e) => {
                    println!("Node {}: mempool snapshot task failed: {}", state.port, e);
                    vec![]
                }
            };

            Some(NodeMessage::MempoolSnapshot { txs })
        }
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
        NodeMessage::NewTx { tx } => {
            let tx_id = tx.tx_id.clone();
            let state_for_store = state.clone();
            let tx_for_store = tx.clone();
            let added = match tokio::task::spawn_blocking(move || {
                state_for_store.add_to_mempool(tx_for_store)
            })
            .await
            {
                Ok(added) => added,
                Err(e) => {
                    println!("Node {}: mempool store task failed: {}", state.port, e);
                    false
                }
            };
            let state_for_size = state.clone();
            let mempool_size = match tokio::task::spawn_blocking(move || state_for_size.mempool_size()).await {
                Ok(size) => size,
                Err(e) => {
                    println!("Node {}: mempool size task failed: {}", state.port, e);
                    0
                }
            };
            println!(
                "Node {}: tx {} {} (mempool {})",
                state.port,
                &tx_id[..16.min(tx_id.len())],
                if added { "stored" } else { "already known" },
                mempool_size
            );

            if added && tx.relay_count < crate::mempool::MAX_RELAY_COUNT {
                let mut forwarded_tx = tx.clone();
                forwarded_tx.relay_count = forwarded_tx
                    .relay_count
                    .saturating_add(1)
                    .min(crate::mempool::MAX_RELAY_COUNT);
                let peers = state.get_peers();
                for peer in peers {
                    if peer != peer_addr {
                        send_to_node_fire_and_forget(
                            &peer,
                            &NodeMessage::NewTx {
                                tx: forwarded_tx.clone(),
                            },
                        )
                        .await;
                    }
                }
            }
            Some(NodeMessage::TxAck {
                tx_id,
                accepted: true,
                mempool: mempool_size,
            })
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
            let chain = state.chain.clone();
            let block_for_merge = block.clone();
            let added = match tokio::task::spawn_blocking(move || {
                chain.merge_blocks_from_network(vec![block_for_merge])
            })
            .await
            {
                Ok(added) => added,
                Err(e) => {
                    println!("Node {}: block merge task failed: {}", state.port, e);
                    0
                }
            };

            if added > 0 {
                let tip = state.chain.last_index();
                state.set_block_count(tip);
                println!("Node {}: full block integrated (tip #{})", state.port, tip);
            }

            if !already_known && added > 0 {
                let peers = state
                    .peers
                    .lock()
                    .expect("node peers mutex poisoned")
                    .clone();
                for peer in peers {
                    if peer == peer_addr {
                        continue;
                    }
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
            let chain = state.chain.clone();
            let added = match tokio::task::spawn_blocking(move || chain.merge_blocks_from_network(blocks)).await
            {
                Ok(added) => added,
                Err(e) => {
                    println!("Node {}: batch merge task failed: {}", state.port, e);
                    0
                }
            };
            if added > 0 {
                let tip = state.chain.last_index();
                state.set_block_count(tip);
                println!("Node {}: batch sync integrated (tip #{})", state.port, tip);
            }
            None
        }
        NodeMessage::Ping => Some(NodeMessage::Pong),
        NodeMessage::MempoolSnapshot { .. } => None,
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
