use crate::chain_state::{ChainState, MAX_SUPPLY};
use crate::config;
use crate::logger::log_mining;
use crate::mempool::Mempool;
use crate::sync::{ChainSync, SharedChain};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinedBlock {
    pub index: u32,
    pub timestamp: u128,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub tx_count: usize,
    pub fees_collected: u64,
}

pub struct Miner {
    pub address: String,
    pub difficulty: usize,
    pub total_mined: u64,
}

impl Miner {
    pub fn new(address: &str) -> Self {
        let state = ChainState::load();
        Self {
            address: address.to_string(),
            difficulty: state.difficulty,
            total_mined: 0,
        }
    }

    pub fn mine_block(&mut self, chain: &SharedChain) -> MinedBlock {
        let mut sync_peers = config::default_seed_nodes();
        sync_peers.extend(config::bootstrap_peers());
        if !sync_peers.is_empty() {
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                let sync = ChainSync::new_with_chain(chain.clone(), sync_peers.clone());
                let sync_handle = handle.clone();
                let imported_blocks = tokio::task::block_in_place(move || {
                    sync_handle.block_on(sync.sync_from_peers())
                });
                if imported_blocks > 0 {
                    println!(
                        "Pre-mining sync imported {} block(s)",
                        imported_blocks
                    );
                }

                let mempool_sync = ChainSync::new_with_chain(chain.clone(), sync_peers);
                let mempool_handle = handle.clone();
                let imported_mempool = tokio::task::block_in_place(move || {
                    mempool_handle.block_on(mempool_sync.sync_mempool_from_peers())
                });
                if imported_mempool > 0 {
                    println!(
                        "Pre-mining mempool sync imported {} pending tx",
                        imported_mempool
                    );
                }
            }
        }

        let state = ChainState::load();
        let reward = state.current_reward();
        let index = chain.last_index().saturating_add(1);
        let previous_hash = chain.last_hash();
        let timestamp = chrono::Utc::now().timestamp_millis() as u128;
        let target = "0".repeat(self.difficulty);

        let mut mempool = Mempool::load();
        let selected_txs = mempool.select_for_block();
        let fees_collected = selected_txs.iter().map(|tx| tx.fee).sum::<u64>();
        let tx_ids: Vec<String> = selected_txs.iter().map(|tx| tx.tx_id.clone()).collect();

        println!(
            "\nMining bloc {} - Reward: {} GHST + {} GHST fees",
            index, reward, fees_collected
        );
        println!("   Included TX: {}", selected_txs.len());
        println!("   Supply      : {} / {} GHST", state.minted_supply, MAX_SUPPLY);

        if !selected_txs.is_empty() {
            println!("\n   Selected TX by priority:");
            for tx in selected_txs.iter().take(3) {
                println!(
                    "      {} {}... - {} GHST fee",
                    tx.priority_label(),
                    &tx.tx_id[..12],
                    tx.fee
                );
            }
            if selected_txs.len() > 3 {
                println!("      ... and {} more", selected_txs.len() - 3);
            }
        }

        let total_reward = reward.saturating_add(fees_collected);
        let tx_ids_serialized = if tx_ids.is_empty() {
            "-".to_string()
        } else {
            tx_ids.join("|")
        };
        let data = format!(
            "coinbase:miner={} reward={} fees={} txs={} height={} txids={}",
            &self.address[..16.min(self.address.len())],
            reward,
            fees_collected,
            selected_txs.len(),
            index,
            tx_ids_serialized,
        );

        log_mining(&format!(
            "Mining bloc {} | {} TX | {} GHST reward | {} GHST fees",
            index,
            selected_txs.len(),
            reward,
            fees_collected
        ));

        let mut nonce = 0u64;
        let hash = loop {
            let input = format!("{}{}{}{}{}", index, timestamp, data, previous_hash, nonce);
            let mut h = Sha256::new();
            h.update(input.as_bytes());
            let hash = format!("{:x}", h.finalize());
            if hash.starts_with(&target) {
                println!(
                    "\nBloc #{} mined! Nonce: {} Hash: {}...",
                    index,
                    nonce,
                    &hash[..12]
                );
                break hash;
            }
            nonce = nonce.saturating_add(1);
            if nonce.is_multiple_of(500_000) {
                println!("   ... {} attempts", nonce);
            }
        };

        if !tx_ids.is_empty() {
            mempool.confirm_txs(&tx_ids, index as u64);
            println!("{} TX confirmed in block #{}", tx_ids.len(), index);
        }

        self.total_mined = self.total_mined.saturating_add(total_reward);

        log_mining(&format!(
            "Bloc #{} confirmed | Supply: {} GHST",
            index,
            state.minted_supply.saturating_add(total_reward)
        ));

        MinedBlock {
            index,
            timestamp,
            data,
            previous_hash,
            hash,
            nonce,
            tx_count: selected_txs.len(),
            fees_collected,
        }
    }
}
