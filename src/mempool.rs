use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

pub const MAX_BLOCK_TXS: usize = 100;
pub const TX_EXPIRY_SECS: i64 = 86_400;
pub const CLAIM_RETENTION_SECS: i64 = 604_800;
pub const MAX_MEMPOOL_TXS: usize = 10_000;

fn mempool_file() -> PathBuf {
    crate::config::data_dir().join("ghostcoin_mempool.json")
}

fn ensure_mempool_parent_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = fs::create_dir_all(parent);
        }
    }
}

fn backup_corrupt_mempool_file(path: &Path) {
    if !path.exists() {
        return;
    }

    let backup_name = format!(
        "{}.corrupt.{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("ghostcoin_mempool.json"),
        Utc::now().timestamp()
    );
    let backup_path = path.with_file_name(backup_name);
    let _ = fs::rename(path, backup_path);
}

fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    ensure_mempool_parent_dir(path);
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, contents)?;

    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp_path, path)?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MempoolTx {
    pub tx_id: String,
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub fee: u64,
    pub fee_rate: u64,
    pub size_bytes: u64,
    pub timestamp: i64,
    #[serde(default)]
    pub claimed: bool,
    #[serde(default)]
    pub receiver_claimed: bool,
    pub relay_count: u32,
}

impl MempoolTx {
    pub fn new(tx_id: &str, sender: &str, receiver: &str, amount: u64, fee: u64) -> Self {
        let size_bytes = 250u64;
        let fee_rate = fee / size_bytes.max(1);

        Self {
            tx_id: tx_id.to_string(),
            sender: sender.to_string(),
            receiver: receiver.to_string(),
            amount,
            fee,
            fee_rate,
            size_bytes,
            timestamp: Utc::now().timestamp(),
            claimed: false,
            receiver_claimed: false,
            relay_count: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp().saturating_sub(self.timestamp) > TX_EXPIRY_SECS
    }

    pub fn should_prune(&self) -> bool {
        let age = Utc::now().timestamp().saturating_sub(self.timestamp);
        if self.claimed {
            age > CLAIM_RETENTION_SECS
        } else {
            age > TX_EXPIRY_SECS
        }
    }

    pub fn priority_label(&self) -> &str {
        match self.fee_rate {
            0..=1 => "Lent",
            2..=5 => "Normal",
            6..=10 => "Rapide",
            _ => "Prioritaire",
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.tx_id.trim().is_empty() {
            return Err("TX invalide: id vide".to_string());
        }
        if self.sender.trim().is_empty() || self.receiver.trim().is_empty() {
            return Err("TX invalide: adresse source ou destination vide".to_string());
        }
        if self.sender == self.receiver {
            return Err("TX invalide: source et destination identiques".to_string());
        }
        if self.amount == 0 {
            return Err("TX invalide: montant nul".to_string());
        }
        if !self.claimed && self.is_expired() {
            return Err("TX invalide: transaction expirée".to_string());
        }
        Ok(())
    }
}

pub struct Mempool {
    pub txs: Vec<MempoolTx>,
}

impl Mempool {
    pub fn load() -> Self {
        let path = mempool_file();
        ensure_mempool_parent_dir(&path);
        if !path.exists() {
            return Self { txs: vec![] };
        }
        let json = match fs::read_to_string(&path) {
            Ok(json) => json,
            Err(e) => {
                println!(
                    "Unable to read mempool file {}: {}. Starting with empty mempool.",
                    path.display(),
                    e
                );
                return Self { txs: vec![] };
            }
        };

        let parsed: Vec<MempoolTx> = match serde_json::from_str(&json) {
            Ok(txs) => txs,
            Err(e) => {
                println!(
                    "Corrupted mempool file {}: {}. Backing it up and resetting mempool.",
                    path.display(),
                    e
                );
                backup_corrupt_mempool_file(&path);
                return Self { txs: vec![] };
            }
        };

        let mut seen = HashSet::new();
        let mut txs = Vec::new();
        let mut repaired = false;

        for tx in parsed {
            if !seen.insert(tx.tx_id.clone()) || tx.validate().is_err() || tx.should_prune() {
                repaired = true;
                continue;
            }
            txs.push(tx);
        }

        txs.sort_by_key(|tx| std::cmp::Reverse((tx.fee_rate, tx.timestamp)));
        if txs.len() > MAX_MEMPOOL_TXS {
            txs.truncate(MAX_MEMPOOL_TXS);
            repaired = true;
        }

        let mempool = Self { txs };
        if repaired {
            mempool.save();
        }
        mempool
    }

    pub fn save(&self) {
        if let Ok(json) = serde_json::to_string_pretty(&self.txs) {
            let path = mempool_file();
            if let Err(e) = write_atomic(&path, &json) {
                println!("Failed to save mempool {}: {}", path.display(), e);
            }
        }
    }

    pub fn add(&mut self, tx: MempoolTx) -> bool {
        self.purge_expired();
        if let Err(reason) = tx.validate() {
            println!("Mempool: {}", reason);
            return false;
        }
        if self.txs.iter().any(|t| t.tx_id == tx.tx_id) {
            return false;
        }
        if self.txs.len() >= MAX_MEMPOOL_TXS {
            println!("Mempool: capacity reached ({} tx max)", MAX_MEMPOOL_TXS);
            return false;
        }

        println!(
            "Mempool: TX ajoutée {} - {} GHST fee - {}",
            &tx.tx_id[..16.min(tx.tx_id.len())],
            tx.fee,
            tx.priority_label()
        );
        self.txs.push(tx);
        self.save();
        true
    }

    pub fn sorted_by_priority(&self) -> Vec<&MempoolTx> {
        let mut sorted: Vec<&MempoolTx> = self
            .txs
            .iter()
            .filter(|tx| !tx.claimed && !tx.is_expired())
            .collect();
        sorted.sort_by_key(|tx| std::cmp::Reverse(tx.fee_rate));
        sorted
    }

    pub fn select_for_block(&self) -> Vec<MempoolTx> {
        self.sorted_by_priority()
            .into_iter()
            .take(MAX_BLOCK_TXS)
            .cloned()
            .collect()
    }

    pub fn confirm_txs(&mut self, tx_ids: &[String], block_height: u64) {
        for tx in self.txs.iter_mut() {
            if tx_ids.contains(&tx.tx_id) {
                tx.claimed = true;
                println!(
                    "TX confirmée bloc #{} : {}...",
                    block_height,
                    &tx.tx_id[..16.min(tx.tx_id.len())]
                );
            }
        }
        self.save();
    }

    pub fn purge_expired(&mut self) -> usize {
        let before = self.txs.len();
        self.txs.retain(|tx| !tx.should_prune());
        let removed = before - self.txs.len();
        if removed > 0 {
            println!("{} TX supprimées du mempool", removed);
            self.save();
        }
        removed
    }

    pub fn replace_by_fee(&mut self, tx_id: &str, new_fee: u64) -> bool {
        for tx in self.txs.iter_mut() {
            if tx.tx_id == tx_id && !tx.claimed {
                if new_fee <= tx.fee {
                    return false;
                }
                let old_fee = tx.fee;
                tx.fee = new_fee;
                tx.fee_rate = new_fee / tx.size_bytes.max(1);
                tx.timestamp = Utc::now().timestamp();
                println!(
                    "RBF: TX {}... fee {} -> {} GHST",
                    &tx_id[..16.min(tx_id.len())],
                    old_fee,
                    new_fee
                );
                self.save();
                return true;
            }
        }
        false
    }

    pub fn pending_count(&self) -> usize {
        self.txs
            .iter()
            .filter(|tx| !tx.claimed && !tx.is_expired())
            .count()
    }

    pub fn total_fees(&self) -> u64 {
        self.txs
            .iter()
            .filter(|tx| !tx.claimed && !tx.is_expired())
            .map(|tx| tx.fee)
            .sum()
    }

    pub fn show_pending(&self) {
        let pending = self.sorted_by_priority();

        println!("\nMEMPOOL GHOSTCOIN");
        println!("  TX pending : {}", self.pending_count());
        println!("  Fees total : {} GHST", self.total_fees());

        for tx in pending.iter().take(5) {
            println!(
                "\n  {} TX: {}...",
                tx.priority_label(),
                &tx.tx_id[..16.min(tx.tx_id.len())]
            );
            println!(
                "     Fee: {} GHST | Rate: {}/byte | {} GHST",
                tx.fee, tx.fee_rate, tx.amount
            );
            println!("     De: {}...", &tx.sender[..20.min(tx.sender.len())]);
        }

        if pending.len() > 5 {
            println!("\n  ... et {} autres TX", pending.len() - 5);
        }
    }
}
