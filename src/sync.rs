use crate::chain_state::ChainState;
use crate::miner::MinedBlock;
use crate::node::{send_to_node, send_to_node_fire_and_forget, NodeMessage};
use chrono::Utc;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};

fn blocks_file() -> String {
    if std::env::var("GHOSTCOIN_SERVER").is_ok() {
        "/app/data/ghostcoin_blocks.json".to_string()
    } else {
        "ghostcoin_blocks.json".to_string()
    }
}

fn ensure_blocks_parent_dir(path: &str) {
    if let Some(parent) = Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
}
const SYNC_CHUNK_SIZE: usize = 128;
const SYNC_CHUNK_MAX: usize = 512;

#[derive(Clone)]
pub struct SharedChain {
    pub blocks: Arc<Mutex<Vec<MinedBlock>>>,
}

impl SharedChain {
    pub fn new() -> Self {
        let blocks = Self::load_from_disk();
        println!("📦 {} bloc(s) chargé(s) depuis le disque", blocks.len());

        Self {
            blocks: Arc::new(Mutex::new(blocks)),
        }
    }

    fn load_from_disk() -> Vec<MinedBlock> {
        let path = blocks_file();
        ensure_blocks_parent_dir(&path);
        if !Path::new(&path).exists() {
            return vec![];
        }
        let json = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&json).unwrap_or_default()
    }

    fn save_to_disk(blocks: &[MinedBlock]) {
        let path = blocks_file();
        ensure_blocks_parent_dir(&path);
        let json = serde_json::to_string_pretty(blocks).unwrap();
        let _ = fs::write(path, json);
    }

    fn rebuild_chain_state(blocks: &[MinedBlock]) {
        let current = ChainState::load();
        let mut rebuilt = ChainState::new();
        rebuilt.difficulty = current.difficulty;
        let mut use_current_as_base = false;

        if let Some(first) = blocks.first() {
            if first.index > 1
                && current.block_height.saturating_add(1) == first.index as u64
                && current.last_block_hash == first.previous_hash
            {
                rebuilt.block_height = current.block_height;
                rebuilt.minted_supply = current.minted_supply;
                rebuilt.total_tx_count = current.total_tx_count;
                rebuilt.total_fees = current.total_fees;
                rebuilt.last_block_hash = current.last_block_hash.clone();
                rebuilt.last_block_time = current.last_block_time;
                use_current_as_base = true;
            }
        }

        for block in blocks {
            if use_current_as_base && block.index <= rebuilt.block_height as u32 {
                continue;
            }
            let reward = rebuilt.current_reward();
            let minted_this_block = reward.saturating_add(block.fees_collected);

            rebuilt.block_height = rebuilt.block_height.saturating_add(1);
            rebuilt.minted_supply = rebuilt
                .minted_supply
                .saturating_add(minted_this_block)
                .min(crate::chain_state::MAX_SUPPLY);
            rebuilt.last_block_hash = block.hash.clone();
            rebuilt.last_block_time = Utc::now().timestamp();
            rebuilt.total_tx_count = rebuilt.total_tx_count.saturating_add(block.tx_count as u64);
            rebuilt.total_fees = rebuilt.total_fees.saturating_add(block.fees_collected);
        }

        rebuilt.save();
    }

    pub fn add_block(&self, block: MinedBlock) {
        let mut chain = self.blocks.lock().unwrap();
        if chain
            .iter()
            .any(|existing| existing.hash == block.hash || existing.index == block.index)
        {
            return;
        }
        chain.push(block);
        chain.sort_by_key(|b| b.index);
        Self::save_to_disk(&chain);
        Self::rebuild_chain_state(&chain);
        println!("💾 Bloc sauvegardé sur disque");
    }

    pub fn merge_blocks_from_network(&self, mut incoming: Vec<MinedBlock>) -> usize {
        if incoming.is_empty() {
            return 0;
        }

        incoming.sort_by_key(|b| b.index);
        let mut chain = self.blocks.lock().unwrap();
        let mut known_hashes: std::collections::HashSet<String> =
            chain.iter().map(|b| b.hash.clone()).collect();
        let mut added = 0usize;

        for block in incoming {
            if known_hashes.contains(&block.hash) {
                continue;
            }
            if chain.iter().any(|existing| existing.index == block.index) {
                continue;
            }

            let parent_known = if block.index == 1 {
                block.previous_hash == "0" && chain.is_empty()
            } else if let Some(parent) = chain
                .iter()
                .find(|existing| existing.hash == block.previous_hash)
            {
                parent.index.saturating_add(1) == block.index
            } else if chain.is_empty() {
                let state = ChainState::load();
                state.block_height.saturating_add(1) == block.index as u64
                    && state.last_block_hash == block.previous_hash
            } else {
                false
            };

            if !parent_known {
                continue;
            }

            known_hashes.insert(block.hash.clone());
            chain.push(block);
            added = added.saturating_add(1);
        }

        if added > 0 {
            chain.sort_by_key(|b| b.index);
            Self::save_to_disk(&chain);
            Self::rebuild_chain_state(&chain);
            println!("🔄 Sync P2P: {} bloc(s) intégré(s)", added);
        }

        added
    }

    pub fn get_blocks_since(&self, from_index: u32, limit: usize) -> Vec<MinedBlock> {
        let chain = self.blocks.lock().unwrap();
        let capped = limit.clamp(1, SYNC_CHUNK_MAX);
        chain
            .iter()
            .filter(|b| b.index > from_index)
            .take(capped)
            .cloned()
            .collect()
    }

    pub fn has_block_hash(&self, hash: &str) -> bool {
        self.blocks
            .lock()
            .unwrap()
            .iter()
            .any(|block| block.hash == hash)
    }

    pub fn length(&self) -> usize {
        self.blocks.lock().unwrap().len()
    }

    pub fn last_hash(&self) -> String {
        let chain = self.blocks.lock().unwrap();
        chain
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn last_index(&self) -> u32 {
        let chain = self.blocks.lock().unwrap();
        chain.last().map(|b| b.index).unwrap_or(0)
    }

    pub fn tip(&self) -> (u32, String) {
        (self.last_index(), self.last_hash())
    }
}

pub struct ChainSync {
    pub chain: SharedChain,
    pub peers: Vec<String>,
}

impl ChainSync {
    pub fn new(peers: Vec<String>) -> Self {
        Self {
            chain: SharedChain::new(),
            peers,
        }
    }

    pub fn new_with_chain(chain: SharedChain, peers: Vec<String>) -> Self {
        Self { chain, peers }
    }

    pub async fn broadcast_block(&self, block: &MinedBlock) {
        let msg = NodeMessage::NewBlockFull {
            block: block.clone(),
        };
        for peer in &self.peers {
            send_to_node_fire_and_forget(peer, &msg).await;
        }
    }

    pub async fn push_missing_blocks_to_peer(&self, peer: &str) -> usize {
        println!("DEBUG push_missing: requesting tip from {}", peer);
        let Some(NodeMessage::ChainTip { last_index, .. }) =
            send_to_node(peer, &NodeMessage::GetChainTip).await
        else {
            println!("DEBUG push_missing: no ChainTip response from {}", peer);
            return 0;
        };

        let local_tip = self.chain.last_index();
        println!(
            "DEBUG push_missing: peer={} remote_tip={} local_tip={}",
            peer, last_index, local_tip
        );
        if last_index >= local_tip {
            println!("DEBUG push_missing: peer {} already up to date", peer);
            return 0;
        }

        // Send the full ordered suffix in one payload so the receiver can
        // integrate it sequentially after a restart.
        let blocks = self.chain.get_blocks_since(last_index, SYNC_CHUNK_MAX);
        let count = blocks.len();
        if count == 0 {
            println!("DEBUG push_missing: no local blocks to send to {}", peer);
            return 0;
        }

        send_to_node_fire_and_forget(peer, &NodeMessage::Blocks { blocks }).await;
        println!("Rattrapage: {} bloc(s) envoyes a {}", count, peer);
        count
    }

    pub async fn push_missing_blocks_to_peers(&self) -> usize {
        let mut total_sent = 0usize;
        for peer in &self.peers {
            total_sent = total_sent.saturating_add(self.push_missing_blocks_to_peer(peer).await);
        }
        total_sent
    }

    pub async fn check_peers(&self) {
        println!("\n🔍 Vérification des pairs :");
        for peer in &self.peers {
            match send_to_node(peer, &NodeMessage::Ping).await {
                Some(_) => println!("   🟢 {} — en ligne", peer),
                None => println!("   🔴 {} — hors ligne", peer),
            }
        }
    }

    pub async fn sync_from_peers(&self) -> usize {
        let mut total_added = 0usize;
        let mut local_tip = self.chain.last_index();

        loop {
            let mut best_peer: Option<String> = None;
            let mut best_height = local_tip;

            for peer in &self.peers {
                if let Some(NodeMessage::ChainTip { last_index, .. }) =
                    send_to_node(peer, &NodeMessage::GetChainTip).await
                {
                    if last_index > best_height {
                        best_height = last_index;
                        best_peer = Some(peer.clone());
                    }
                }
            }

            let Some(peer) = best_peer else {
                break;
            };
            if best_height <= local_tip {
                break;
            }

            let response = send_to_node(
                &peer,
                &NodeMessage::GetBlocksSince {
                    from_index: local_tip,
                    limit: SYNC_CHUNK_SIZE,
                },
            )
            .await;

            let Some(NodeMessage::Blocks { blocks }) = response else {
                break;
            };
            if blocks.is_empty() {
                break;
            }

            let added = self.chain.merge_blocks_from_network(blocks);
            if added == 0 {
                break;
            }

            total_added = total_added.saturating_add(added);
            local_tip = self.chain.last_index();

            if local_tip >= best_height {
                break;
            }
        }

        total_added
    }
}
