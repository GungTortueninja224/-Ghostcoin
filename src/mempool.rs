use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::Utc;

pub const MEMPOOL_FILE:   &str = "ghostcoin_mempool.json";
pub const MAX_BLOCK_TXS:  usize = 100;   // max TX par bloc
pub const TX_EXPIRY_SECS: i64   = 86400; // 24h avant expiration

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MempoolTx {
    pub tx_id:       String,
    pub sender:      String,
    pub receiver:    String,
    pub amount:      u64,
    pub fee:         u64,
    pub fee_rate:    u64,   // fee par byte (priorité)
    pub size_bytes:  u64,
    pub timestamp:   i64,
    pub claimed:     bool,
    pub relay_count: u32,   // combien de fois broadcasté
}

impl MempoolTx {
    pub fn new(
        tx_id: &str, sender: &str, receiver: &str,
        amount: u64, fee: u64,
    ) -> Self {
        // Taille estimée d'une TX GhostCoin
        let size_bytes = 250u64;
        let fee_rate   = fee / size_bytes.max(1);

        Self {
            tx_id:       tx_id.to_string(),
            sender:      sender.to_string(),
            receiver:    receiver.to_string(),
            amount,
            fee,
            fee_rate,
            size_bytes,
            timestamp:   Utc::now().timestamp(),
            claimed:     false,
            relay_count: 0,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now().timestamp() - self.timestamp > TX_EXPIRY_SECS
    }

    pub fn priority_label(&self) -> &str {
        match self.fee_rate {
            0..=1  => "🐢 Lent",
            2..=5  => "🚶 Normal",
            6..=10 => "🚀 Rapide",
            _      => "⚡ Prioritaire",
        }
    }
}

// ==========================================
// MEMPOOL MANAGER
// ==========================================
pub struct Mempool {
    pub txs: Vec<MempoolTx>,
}

impl Mempool {
    pub fn load() -> Self {
        if !Path::new(MEMPOOL_FILE).exists() {
            return Self { txs: vec![] };
        }
        let json = fs::read_to_string(MEMPOOL_FILE).unwrap_or_default();
        let txs  = serde_json::from_str(&json).unwrap_or_default();
        Self { txs }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(&self.txs).unwrap();
        let _    = fs::write(MEMPOOL_FILE, json);
    }

    // Ajoute une TX au mempool
    pub fn add(&mut self, tx: MempoolTx) -> bool {
        // Vérifie doublon
        if self.txs.iter().any(|t| t.tx_id == tx.tx_id) {
            return false;
        }
        println!("📝 Mempool: TX ajoutée {} — {} GHST fee — {}",
            &tx.tx_id[..16], tx.fee, tx.priority_label());
        self.txs.push(tx);
        self.save();
        true
    }

    // Trie par fee_rate décroissant (meilleurs fees en premier)
    pub fn sorted_by_priority(&self) -> Vec<&MempoolTx> {
        let mut sorted: Vec<&MempoolTx> = self.txs.iter()
            .filter(|tx| !tx.claimed && !tx.is_expired())
            .collect();
        sorted.sort_by(|a, b| b.fee_rate.cmp(&a.fee_rate));
        sorted
    }

    // Sélectionne les meilleures TX pour un bloc
    pub fn select_for_block(&self) -> Vec<MempoolTx> {
        self.sorted_by_priority()
            .into_iter()
            .take(MAX_BLOCK_TXS)
            .cloned()
            .collect()
    }

    // Confirme les TX incluses dans un bloc
    pub fn confirm_txs(&mut self, tx_ids: &[String], block_height: u64) {
        for tx in self.txs.iter_mut() {
            if tx_ids.contains(&tx.tx_id) {
                tx.claimed = true;
                println!("✅ TX confirmée bloc #{} : {}...",
                    block_height, &tx.tx_id[..16]);
            }
        }
        self.save();
    }

    // Supprime les TX expirées
    pub fn purge_expired(&mut self) -> usize {
        let before = self.txs.len();
        self.txs.retain(|tx| !tx.is_expired() || !tx.claimed);
        let removed = before - self.txs.len();
        if removed > 0 {
            println!("🗑️  {} TX expirées supprimées du mempool", removed);
            self.save();
        }
        removed
    }

    // Replace-by-Fee : augmente les frais d'une TX pending
    pub fn replace_by_fee(&mut self, tx_id: &str, new_fee: u64) -> bool {
        for tx in self.txs.iter_mut() {
            if tx.tx_id == tx_id && !tx.claimed {
                let old_fee  = tx.fee;
                tx.fee       = new_fee;
                tx.fee_rate  = new_fee / tx.size_bytes.max(1);
                tx.timestamp = Utc::now().timestamp();
                println!("🔄 RBF: TX {}... fee {} → {} GHST",
                    &tx_id[..16], old_fee, new_fee);
                self.save();
                return true;
            }
        }
        false
    }

    pub fn pending_count(&self) -> usize {
        self.txs.iter().filter(|tx| !tx.claimed).count()
    }

    pub fn total_fees(&self) -> u64 {
        self.txs.iter()
            .filter(|tx| !tx.claimed)
            .map(|tx| tx.fee)
            .sum()
    }

    pub fn show_pending(&self) {
        let pending: Vec<&MempoolTx> = self.sorted_by_priority();

        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                 📋 MEMPOOL GHOSTCOIN                    ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  TX pending : {:<42} ║", self.pending_count());
        println!("║  Fees total : {:<42} ║", format!("{} GHST", self.total_fees()));
        println!("╚══════════════════════════════════════════════════════════╝");

        for tx in pending.iter().take(5) {
            println!("\n  {} TX: {}...", tx.priority_label(), &tx.tx_id[..16]);
            println!("     Fee: {} GHST | Rate: {}/byte | {} GHST",
                tx.fee, tx.fee_rate, tx.amount);
            println!("     De: {}...", &tx.sender[..20.min(tx.sender.len())]);
        }

        if pending.len() > 5 {
            println!("\n  ... et {} autres TX", pending.len() - 5);
        }
    }
}