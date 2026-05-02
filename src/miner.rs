use crate::chain_state::ChainState;
use crate::logger::log_mining;
use crate::mempool::Mempool;
use crate::sync::SharedChain;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// ==========================================
// BLOC MINÉ — doit être AVANT impl Miner
// ==========================================
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

// ==========================================
// MINEUR
// ==========================================
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

    pub fn mine_block(&mut self, _chain: &SharedChain) -> MinedBlock {
        let mut state = ChainState::load();
        let reward = state.current_reward();
        let index = state.block_height as u32 + 1;
        let previous_hash = state.last_block_hash.clone();
        let timestamp = chrono::Utc::now().timestamp_millis() as u128;
        let target = "0".repeat(self.difficulty);

        // Sélectionne TX du mempool par priorité
        let mut mempool = Mempool::load();
        let selected_txs = mempool.select_for_block();
        let fees_collected = selected_txs.iter().map(|tx| tx.fee).sum::<u64>();
        let tx_ids: Vec<String> = selected_txs.iter().map(|tx| tx.tx_id.clone()).collect();

        println!(
            "\n⛏️  Mining bloc {} — Récompense: {} GHST + {} GHST fees",
            index, reward, fees_collected
        );
        println!("   TX incluses : {}", selected_txs.len());
        println!(
            "   Supply      : {} / {} GHST",
            state.minted_supply,
            crate::chain_state::MAX_SUPPLY
        );

        if !selected_txs.is_empty() {
            println!("\n   📋 TX sélectionnées par priorité :");
            for tx in selected_txs.iter().take(3) {
                println!(
                    "      {} {}... — {} GHST fee",
                    tx.priority_label(),
                    &tx.tx_id[..12],
                    tx.fee
                );
            }
            if selected_txs.len() > 3 {
                println!("      ... et {} autres", selected_txs.len() - 3);
            }
        }

        let total_reward = reward + fees_collected;
        let data = format!(
            "coinbase:miner={} reward={} fees={} txs={} height={}",
            &self.address[..16.min(self.address.len())],
            reward,
            fees_collected,
            selected_txs.len(),
            index,
        );

        log_mining(&format!(
            "Mining bloc {} | {} TX | {} GHST reward | {} GHST fees",
            index,
            selected_txs.len(),
            reward,
            fees_collected
        ));

        // Proof of Work
        let mut nonce = 0u64;
        let hash = loop {
            let input = format!("{}{}{}{}{}", index, timestamp, data, previous_hash, nonce);
            let mut h = Sha256::new();
            h.update(input.as_bytes());
            let hash = format!("{:x}", h.finalize());
            if hash.starts_with(&target) {
                println!(
                    "\n✅ Bloc #{} miné ! Nonce: {} Hash: {}...",
                    index,
                    nonce,
                    &hash[..12]
                );
                break hash;
            }
            nonce += 1;
            if nonce.is_multiple_of(500_000) {
                println!("   ... {} essais", nonce);
            }
        };

        // Confirme les TX dans le mempool
        if !tx_ids.is_empty() {
            mempool.confirm_txs(&tx_ids, index as u64);
            println!("✅ {} TX confirmées dans le bloc #{}", tx_ids.len(), index);
        }

        // Met à jour ChainState global
        state.add_block(
            &hash,
            total_reward,
            fees_collected,
            selected_txs.len() as u64,
        );
        self.total_mined += total_reward;

        log_mining(&format!(
            "Bloc #{} confirmé | Supply: {} GHST",
            index, state.minted_supply
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
