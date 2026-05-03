use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub fn chain_state_path() -> std::path::PathBuf {
    crate::config::chain_file()
}

fn ensure_chain_state_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
}
pub const MAX_SUPPLY: u64 = 50_000_000;
pub const INITIAL_REWARD: u64 = 65;
pub const HALVING_INTERVAL: u64 = 210_000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChainState {
    pub block_height: u64,
    pub minted_supply: u64,
    pub difficulty: usize,
    pub last_block_hash: String,
    pub last_block_time: i64,
    pub total_tx_count: u64,
    pub total_fees: u64,
}

impl ChainState {
    pub fn new() -> Self {
        Self {
            block_height: 0,
            minted_supply: 0,
            difficulty: 4,
            last_block_hash: "0".to_string(),
            last_block_time: Utc::now().timestamp(),
            total_tx_count: 0,
            total_fees: 0,
        }
    }

    pub fn load() -> Self {
        let path = chain_state_path();
        ensure_chain_state_parent_dir(&path);
        if !path.exists() {
            let state = Self::new();
            state.save();
            return state;
        }
        let json = fs::read_to_string(path).unwrap_or_default();
        serde_json::from_str(&json).unwrap_or_else(|_| Self::new())
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let path = chain_state_path();
            ensure_chain_state_parent_dir(&path);
            let _ = fs::write(path, json);
        }
    }

    pub fn current_reward(&self) -> u64 {
        let halvings = self.block_height / HALVING_INTERVAL;
        if halvings >= 64 {
            return 0;
        }
        let reward = INITIAL_REWARD >> halvings;
        if self.minted_supply.saturating_add(reward) > MAX_SUPPLY {
            MAX_SUPPLY - self.minted_supply
        } else {
            reward
        }
    }

    pub fn remaining_supply(&self) -> u64 {
        MAX_SUPPLY.saturating_sub(self.minted_supply)
    }

    pub fn halving_progress(&self) -> f64 {
        let pos = self.block_height % HALVING_INTERVAL;
        (pos as f64 / HALVING_INTERVAL as f64) * 100.0
    }

    pub fn next_halving_block(&self) -> u64 {
        let current_era = self.block_height / HALVING_INTERVAL;
        (current_era + 1) * HALVING_INTERVAL
    }

    pub fn add_block(&mut self, hash: &str, reward: u64, fees: u64, tx_count: u64) {
        self.block_height = self.block_height.saturating_add(1);
        self.minted_supply = self.minted_supply.saturating_add(reward).min(MAX_SUPPLY);
        self.last_block_hash = hash.to_string();
        self.last_block_time = Utc::now().timestamp();
        self.total_tx_count = self.total_tx_count.saturating_add(tx_count);
        self.total_fees = self.total_fees.saturating_add(fees);
        self.save();
    }

    pub fn show(&self) {
        let reward = self.current_reward();
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║              📊 GHOSTCOIN NETWORK STATS                 ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Block Height   : {:<38} ║", self.block_height);
        println!(
            "║  Minted Supply  : {:<38} ║",
            format!("{} GHST", self.minted_supply)
        );
        println!(
            "║  Max Supply     : {:<38} ║",
            format!("{} GHST", MAX_SUPPLY)
        );
        println!(
            "║  Remaining      : {:<38} ║",
            format!("{} GHST", self.remaining_supply())
        );
        println!("║  Block Reward   : {:<38} ║", format!("{} GHST", reward));
        println!(
            "║  Next Halving   : {:<38} ║",
            format!("bloc #{}", self.next_halving_block())
        );
        println!(
            "║  Halving Prog.  : {:<38} ║",
            format!("{:.2}%", self.halving_progress())
        );
        println!("║  Difficulty     : {:<38} ║", self.difficulty);
        println!("║  Total TX       : {:<38} ║", self.total_tx_count);
        println!(
            "║  Total Fees     : {:<38} ║",
            format!("{} GHST", self.total_fees)
        );
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}
