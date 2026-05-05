use crate::chain_state::ChainState;
use crate::config;
use crate::mempool::Mempool;
use crate::miner::MinedBlock;
use crate::node::{send_to_node, send_to_node_fire_and_forget, NodeMessage};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn blocks_file() -> std::path::PathBuf {
    config::blocks_file()
}

fn ensure_blocks_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
}

fn backup_corrupt_blocks_file(path: &Path) {
    if !path.exists() {
        return;
    }

    let backup_name = format!(
        "{}.corrupt.{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("ghostcoin_blocks.json"),
        chrono::Utc::now().timestamp()
    );
    let backup_path = path.with_file_name(backup_name);
    let _ = fs::rename(path, backup_path);
}

fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    ensure_blocks_parent_dir(path);
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, contents)?;

    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp_path, path)?;
    Ok(())
}

const SYNC_CHUNK_SIZE: usize = 128;
const SYNC_CHUNK_MAX: usize = 512;
const MEMPOOL_SYNC_LIMIT: usize = 2_048;

#[derive(Clone)]
pub struct SharedChain {
    pub blocks: Arc<Mutex<Vec<MinedBlock>>>,
}

impl SharedChain {
    pub fn new() -> Self {
        let blocks = Self::load_from_disk();
        println!("Loaded {} block(s) from disk", blocks.len());

        Self {
            blocks: Arc::new(Mutex::new(blocks)),
        }
    }

    fn load_from_disk() -> Vec<MinedBlock> {
        let path = blocks_file();
        ensure_blocks_parent_dir(&path);
        if !path.exists() {
            Self::rebuild_chain_state(&[]);
            return vec![];
        }

        let json = match fs::read_to_string(&path) {
            Ok(json) => json,
            Err(e) => {
                println!(
                    "Unable to read block store {}: {}. Starting from an empty chain.",
                    path.display(),
                    e
                );
                Self::rebuild_chain_state(&[]);
                return vec![];
            }
        };

        let blocks: Vec<MinedBlock> = match serde_json::from_str(&json) {
            Ok(blocks) => blocks,
            Err(e) => {
                println!(
                    "Corrupted block store {}: {}. Backing it up and rebuilding from empty chain.",
                    path.display(),
                    e
                );
                backup_corrupt_blocks_file(&path);
                Self::rebuild_chain_state(&[]);
                return vec![];
            }
        };
        let normalized = Self::normalize_blocks(blocks);
        Self::save_to_disk(&normalized);
        Self::rebuild_chain_state(&normalized);
        Self::reconcile_mempool_with_blocks(&normalized);
        normalized
    }

    fn normalize_blocks(mut blocks: Vec<MinedBlock>) -> Vec<MinedBlock> {
        if blocks.is_empty() {
            return blocks;
        }

        let original_len = blocks.len();
        blocks.sort_by_key(|b| b.index);
        let mut normalized: Vec<MinedBlock> = Vec::new();
        let mut seen_hashes = HashSet::new();
        let mut state = ChainState::new();
        state.difficulty = ChainState::load().difficulty;

        for block in blocks {
            if !seen_hashes.insert(block.hash.clone()) {
                continue;
            }

            if let Err(reason) = Self::validate_block_candidate(&block, &normalized, &state) {
                println!(
                    "Discarding invalid local block #{} during normalization: {}",
                    block.index, reason
                );
                continue;
            }

            Self::apply_block_to_state(&mut state, &block);
            normalized.push(block);
        }

        if normalized.len() < original_len {
            println!(
                "Local chain repaired: kept {} contiguous block(s) out of {}",
                normalized.len(),
                original_len
            );
        }

        normalized
    }

    fn save_to_disk(blocks: &[MinedBlock]) {
        let path = blocks_file();
        ensure_blocks_parent_dir(&path);
        match serde_json::to_string_pretty(blocks) {
            Ok(json) => {
                if let Err(e) = write_atomic(&path, &json) {
                    println!("Failed to save blocks {}: {}", path.display(), e);
                }
            }
            Err(e) => println!("Failed to serialize blocks for {}: {}", path.display(), e),
        }
    }

    fn calculate_block_hash(block: &MinedBlock) -> String {
        let input = format!(
            "{}{}{}{}{}",
            block.index, block.timestamp, block.data, block.previous_hash, block.nonce
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn meets_difficulty(hash: &str, difficulty: usize) -> bool {
        hash.starts_with(&"0".repeat(difficulty.min(64)))
    }

    fn parse_coinbase_field(data: &str, field: &str) -> Option<u64> {
        let prefix = format!("{}=", field);
        data.split_whitespace()
            .find_map(|part| part.strip_prefix(&prefix))
            .and_then(|raw| raw.parse::<u64>().ok())
    }

    fn parse_coinbase_text_field(data: &str, field: &str) -> Option<String> {
        let prefix = format!("{}=", field);
        data.split_whitespace()
            .find_map(|part| part.strip_prefix(&prefix))
            .map(str::to_string)
    }

    fn claimed_reward(block: &MinedBlock) -> Option<u64> {
        Self::parse_coinbase_field(&block.data, "reward")
    }

    fn included_tx_ids(block: &MinedBlock, mempool: &Mempool) -> Vec<String> {
        if let Some(raw_ids) = Self::parse_coinbase_text_field(&block.data, "txids") {
            if raw_ids.is_empty() || raw_ids == "-" {
                return vec![];
            }
            return raw_ids
                .split('|')
                .filter(|tx_id| !tx_id.is_empty())
                .map(str::to_string)
                .collect();
        }

        if block.tx_count == 0 {
            return vec![];
        }

        let candidates = mempool.select_for_block();
        if candidates.len() < block.tx_count {
            return vec![];
        }

        let selected = &candidates[..block.tx_count];
        let total_fees = selected.iter().map(|tx| tx.fee).sum::<u64>();
        if total_fees != block.fees_collected {
            return vec![];
        }

        selected.iter().map(|tx| tx.tx_id.clone()).collect()
    }

    fn reconcile_mempool_with_blocks(blocks: &[MinedBlock]) {
        if !blocks.iter().any(|block| block.tx_count > 0) {
            return;
        }

        let mut mempool = Mempool::load();
        for block in blocks {
            if block.tx_count == 0 {
                continue;
            }

            let tx_ids = Self::included_tx_ids(block, &mempool);
            if !tx_ids.is_empty() {
                mempool.confirm_txs(&tx_ids, block.index as u64);
            }
        }
    }

    fn validate_block_candidate(
        block: &MinedBlock,
        chain: &[MinedBlock],
        state: &ChainState,
    ) -> Result<(), String> {
        let expected_index = chain.last().map(|b| b.index.saturating_add(1)).unwrap_or(1);
        if block.index != expected_index {
            return Err(format!(
                "non-contiguous index: expected {}, got {}",
                expected_index, block.index
            ));
        }

        let expected_prev_hash = chain
            .last()
            .map(|b| b.hash.as_str())
            .unwrap_or("0");
        if block.previous_hash != expected_prev_hash {
            return Err(format!(
                "invalid prev_hash: expected {}, got {}",
                expected_prev_hash, block.previous_hash
            ));
        }

        let computed_hash = Self::calculate_block_hash(block);
        if computed_hash != block.hash {
            return Err(format!(
                "hash mismatch: computed {}, claimed {}",
                computed_hash, block.hash
            ));
        }

        if !Self::meets_difficulty(&block.hash, state.difficulty) {
            return Err(format!("insufficient PoW for difficulty {}", state.difficulty));
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        if block.timestamp > now_ms.saturating_add(120_000) {
            return Err("timestamp too far in the future".to_string());
        }

        if let Some(last) = chain.last() {
            if block.timestamp <= last.timestamp {
                return Err("timestamp not strictly increasing".to_string());
            }
        }

        let claimed_reward = Self::claimed_reward(block)
            .ok_or_else(|| "missing reward field in coinbase data".to_string())?;
        let expected_reward = state.current_reward();
        if claimed_reward > expected_reward {
            return Err(format!(
                "reward {} exceeds expected max {}",
                claimed_reward, expected_reward
            ));
        }
        if state.minted_supply.saturating_add(claimed_reward) > crate::chain_state::MAX_SUPPLY {
            return Err("reward would exceed max supply".to_string());
        }

        if let Some(claimed_height) = Self::parse_coinbase_field(&block.data, "height") {
            if claimed_height != block.index as u64 {
                return Err(format!(
                    "coinbase height mismatch: claimed {}, block index {}",
                    claimed_height, block.index
                ));
            }
        }

        Ok(())
    }

    fn apply_block_to_state(state: &mut ChainState, block: &MinedBlock) {
        let claimed_reward = Self::claimed_reward(block).unwrap_or_else(|| state.current_reward());
        state.block_height = state.block_height.saturating_add(1);
        state.minted_supply = state
            .minted_supply
            .saturating_add(claimed_reward)
            .min(crate::chain_state::MAX_SUPPLY);
        state.last_block_hash = block.hash.clone();
        state.last_block_time = (block.timestamp / 1000).min(i64::MAX as u128) as i64;
        state.total_tx_count = state.total_tx_count.saturating_add(block.tx_count as u64);
        state.total_fees = state.total_fees.saturating_add(block.fees_collected);
    }

    fn build_chain_state(blocks: &[MinedBlock], difficulty: usize) -> Result<ChainState, String> {
        let mut state = ChainState::new();
        state.difficulty = difficulty;
        let mut validated: Vec<MinedBlock> = Vec::with_capacity(blocks.len());

        for block in blocks {
            Self::validate_block_candidate(block, &validated, &state)?;
            Self::apply_block_to_state(&mut state, block);
            validated.push(block.clone());
        }

        Ok(state)
    }

    fn rebuild_chain_state(blocks: &[MinedBlock]) {
        let current = ChainState::load();
        let mut rebuilt = ChainState::new();
        rebuilt.difficulty = current.difficulty;

        for block in blocks {
            Self::apply_block_to_state(&mut rebuilt, block);
        }

        rebuilt.save();
    }

    pub fn add_block(&self, block: MinedBlock) {
        let mut chain = self
            .blocks
            .lock()
            .expect("shared chain mutex poisoned");
        let mut state = ChainState::load();

        if chain
            .iter()
            .any(|existing| existing.hash == block.hash || existing.index == block.index)
        {
            return;
        }

        if let Err(reason) = Self::validate_block_candidate(&block, &chain, &state) {
            println!("Rejected inconsistent local block #{}: {}", block.index, reason);
            return;
        }

        chain.push(block.clone());
        chain.sort_by_key(|b| b.index);
        Self::apply_block_to_state(&mut state, &block);
        Self::save_to_disk(&chain);
        state.save();
        println!("Block saved to disk");
    }

    pub fn merge_blocks_from_network(&self, mut incoming: Vec<MinedBlock>) -> usize {
        if incoming.is_empty() {
            return 0;
        }

        incoming.sort_by_key(|b| b.index);
        let mut chain = self
            .blocks
            .lock()
            .expect("shared chain mutex poisoned");
        let difficulty = ChainState::load().difficulty;
        let current_len = chain.len();
        let current_tip_hash = chain.last().map(|b| b.hash.clone());

        let Some(first_block) = incoming.first() else {
            return 0;
        };
        let first_index = first_block.index;
        let first_prev_hash = first_block.previous_hash.clone();

        let prefix_len = if first_index == 1 && first_prev_hash == "0" {
            0
        } else if let Some(anchor) = chain
            .iter()
            .position(|existing| existing.hash == first_prev_hash)
        {
            anchor.saturating_add(1)
        } else {
            println!(
                "[sync] rejected incoming segment starting at #{}: unknown fork parent {}",
                first_index, first_prev_hash
            );
            return 0;
        };

        let mut candidate = chain[..prefix_len].to_vec();
        candidate.extend(incoming);

        let candidate_state = match Self::build_chain_state(&candidate, difficulty) {
            Ok(state) => state,
            Err(reason) => {
                let rejected_index = candidate
                    .get(prefix_len)
                    .map(|block| block.index)
                    .unwrap_or(first_index);
                println!("[sync] rejected block #{}: {}", rejected_index, reason);
                return 0;
            }
        };

        let candidate_tip_hash = candidate.last().map(|b| b.hash.clone());
        if candidate.len() < current_len {
            return 0;
        }
        if candidate.len() == current_len && candidate_tip_hash == current_tip_hash {
            return 0;
        }
        if candidate.len() == current_len && prefix_len == current_len {
            return 0;
        }
        if candidate.len() == current_len && prefix_len < current_len {
            println!(
                "[sync] ignored competing fork of equal length at height {}",
                current_len
            );
            return 0;
        }
        if candidate.len() <= current_len && prefix_len < current_len {
            println!(
                "[sync] ignored shorter fork (candidate {}, current {})",
                candidate.len(),
                current_len
            );
            return 0;
        }

        let common_prefix = chain
            .iter()
            .zip(candidate.iter())
            .take_while(|(left, right)| left.hash == right.hash)
            .count();
        let integrated = candidate.len().saturating_sub(common_prefix);

        *chain = candidate;
        Self::save_to_disk(&chain);
        candidate_state.save();
        Self::reconcile_mempool_with_blocks(&chain[common_prefix..]);

        if prefix_len < current_len {
            println!(
                "[sync] adopted longer fork: replaced {} block(s), tip now #{}",
                current_len.saturating_sub(prefix_len),
                chain.last().map(|b| b.index).unwrap_or(0)
            );
        } else {
            println!("P2P sync integrated {} block(s)", integrated);
        }

        integrated
    }

    pub fn get_blocks_since(&self, from_index: u32, limit: usize) -> Vec<MinedBlock> {
        let chain = self
            .blocks
            .lock()
            .expect("shared chain mutex poisoned");
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
            .expect("shared chain mutex poisoned")
            .iter()
            .any(|block| block.hash == hash)
    }

    pub fn length(&self) -> usize {
        self.blocks
            .lock()
            .expect("shared chain mutex poisoned")
            .len()
    }

    pub fn last_hash(&self) -> String {
        let chain = self
            .blocks
            .lock()
            .expect("shared chain mutex poisoned");
        chain
            .last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn last_index(&self) -> u32 {
        let chain = self
            .blocks
            .lock()
            .expect("shared chain mutex poisoned");
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
        if crate::config::debug_enabled() {
            println!("DEBUG push_missing: requesting tip from {}", peer);
        }
        let Some(NodeMessage::ChainTip { last_index, .. }) =
            send_to_node(peer, &NodeMessage::GetChainTip).await
        else {
            if crate::config::debug_enabled() {
                println!("DEBUG push_missing: no ChainTip response from {}", peer);
            }
            return 0;
        };

        let local_tip = self.chain.last_index();
        if crate::config::debug_enabled() {
            println!(
                "DEBUG push_missing: peer={} remote_tip={} local_tip={}",
                peer, last_index, local_tip
            );
        }
        if last_index >= local_tip {
            if crate::config::debug_enabled() {
                println!("DEBUG push_missing: peer {} already up to date", peer);
            }
            return 0;
        }

        let blocks = self.chain.get_blocks_since(last_index, SYNC_CHUNK_MAX);
        let count = blocks.len();
        if count == 0 {
            if crate::config::debug_enabled() {
                println!("DEBUG push_missing: no local blocks to send to {}", peer);
            }
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

    pub async fn sync_mempool_from_peer(&self, peer: &str) -> usize {
        let Some(NodeMessage::MempoolSnapshot { txs }) = send_to_node(
            peer,
            &NodeMessage::GetMempool {
                limit: MEMPOOL_SYNC_LIMIT,
            },
        )
        .await
        else {
            return 0;
        };

        if txs.is_empty() {
            return 0;
        }

        let added = match tokio::task::spawn_blocking(move || Mempool::merge_persisted(txs)).await
        {
            Ok(added) => added,
            Err(e) => {
                println!("[sync] mempool merge task failed for {}: {}", peer, e);
                0
            }
        };

        if added > 0 {
            println!("Mempool sync imported {} tx from {}", added, peer);
        }

        added
    }

    pub async fn sync_mempool_from_peers(&self) -> usize {
        let mut total_added = 0usize;
        for peer in &self.peers {
            total_added = total_added.saturating_add(self.sync_mempool_from_peer(peer).await);
        }
        total_added
    }

    pub async fn check_peers(&self) {
        println!("\nVerification des pairs :");
        for peer in &self.peers {
            match send_to_node(peer, &NodeMessage::Ping).await {
                Some(_) => println!("   {} - online", peer),
                None => println!("   {} - offline", peer),
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

            let chain = self.chain.clone();
            let added = match tokio::task::spawn_blocking(move || {
                chain.merge_blocks_from_network(blocks)
            })
            .await
            {
                Ok(added) => added,
                Err(e) => {
                    println!("[sync] merge task failed: {}", e);
                    break;
                }
            };
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
