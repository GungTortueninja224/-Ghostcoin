use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::Utc;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TxStatus {
    Pending,
    Confirmed { confirmations: u64 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum TxDirection {
    Sent,
    Received,
    Mining,
    Change,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WalletTx {
    pub tx_id:        String,
    pub direction:    TxDirection,
    pub amount:       u64,
    pub fee:          u64,
    pub address:      String,
    pub timestamp:    String,
    pub block_height: Option<u64>,
    pub status:       TxStatus,
    pub note:         String,
}

impl WalletTx {
    pub fn new_sent(tx_id: &str, amount: u64, fee: u64, to: &str) -> Self {
        Self {
            tx_id:        tx_id.to_string(),
            direction:    TxDirection::Sent,
            amount,
            fee,
            address:      to.to_string(),
            timestamp:    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            block_height: None,
            status:       TxStatus::Pending,
            note:         String::new(),
        }
    }

    pub fn new_received(tx_id: &str, amount: u64, from: &str) -> Self {
        Self {
            tx_id:        tx_id.to_string(),
            direction:    TxDirection::Received,
            amount,
            fee:          0,
            address:      from.to_string(),
            timestamp:    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            block_height: None,
            status:       TxStatus::Pending,
            note:         String::new(),
        }
    }

    pub fn new_mining(tx_id: &str, amount: u64, block_height: u64) -> Self {
        Self {
            tx_id:        tx_id.to_string(),
            direction:    TxDirection::Mining,
            amount,
            fee:          0,
            address:      "coinbase".to_string(),
            timestamp:    Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            block_height: Some(block_height),
            status:       TxStatus::Confirmed { confirmations: 1 },
            note:         String::new(),
        }
    }

    pub fn confirm(&mut self, block_height: u64) {
        self.block_height = Some(block_height);
        self.status       = TxStatus::Confirmed { confirmations: 1 };
    }

    pub fn display(&self) {
        let (icon, label) = match &self.direction {
            TxDirection::Sent     => ("📤", "Envoyé"),
            TxDirection::Received => ("📥", "Reçu"),
            TxDirection::Mining   => ("⛏️ ", "Minage"),
            TxDirection::Change   => ("🔄", "Change"),
        };

        let status = match &self.status {
            TxStatus::Pending => "⏳ Pending".to_string(),
            TxStatus::Confirmed { confirmations } =>
                format!("✅ Confirmé ({} conf.)", confirmations),
        };

        println!("┌─────────────────────────────────────────────────────────┐");
        println!("│ {} {:<52} │", icon, label);
        println!("├─────────────────────────────────────────────────────────┤");
        println!("│ Montant  : {:<47} │", format!("{} GHST", self.amount));
        if self.fee > 0 {
            println!("│ Frais    : {:<47} │", format!("{} GHST", self.fee));
            println!("│ Total    : {:<47} │", format!("{} GHST", self.amount + self.fee));
        }
        println!("│ Adresse  : {:<47} │", &self.address[..self.address.len().min(47)]);
        println!("│ TX ID    : {:<47} │", &self.tx_id[..self.tx_id.len().min(47)]);
        println!("│ Date     : {:<47} │", self.timestamp);
        println!("│ Status   : {:<47} │", status);
        if let Some(h) = self.block_height {
            println!("│ Bloc     : {:<47} │", format!("#{}", h));
        }
        println!("└─────────────────────────────────────────────────────────┘");
    }
}

// ==========================================
// STOCKAGE PAR WALLET
// ==========================================
pub struct WalletTxStore {
    pub address: String,
    pub path:    String,
}

impl WalletTxStore {
    pub fn new(address: &str) -> Self {
        let safe_addr = address
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(20)
            .collect::<String>();
        let path = format!("txhistory_{}.json", safe_addr);
        Self { address: address.to_string(), path }
    }

    pub fn load(&self) -> Vec<WalletTx> {
        if !Path::new(&self.path).exists() { return vec![]; }
        let json = fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&json).unwrap_or_default()
    }

    pub fn save(&self, txs: &[WalletTx]) {
        let json = serde_json::to_string_pretty(txs).unwrap();
        let _    = fs::write(&self.path, json);
    }

    pub fn add(&self, tx: WalletTx) {
        let mut txs = self.load();
        if !txs.iter().any(|t| t.tx_id == tx.tx_id) {
            txs.push(tx);
            self.save(&txs);
        }
    }

    pub fn confirm_tx(&self, tx_id: &str, block_height: u64) {
        let mut txs = self.load();
        for tx in txs.iter_mut() {
            if tx.tx_id == tx_id {
                tx.confirm(block_height);
            }
        }
        self.save(&txs);
    }

    pub fn confirm_from_mempool(&self, block_height: u64) {
        let mempool  = crate::mempool::Mempool::load();
        let mut txs  = self.load();
        let mut changed = false;

        for tx in txs.iter_mut() {
            if tx.status == TxStatus::Pending {
                let confirmed = mempool.txs.iter().any(|mtx|
                    mtx.tx_id == tx.tx_id && mtx.claimed
                );
                if confirmed {
                    tx.confirm(block_height);
                    changed = true;
                    println!("✅ TX confirmée bloc #{} : {}...",
                        block_height, &tx.tx_id[..16.min(tx.tx_id.len())]);
                }
            }
        }

        if changed {
            self.save(&txs);
        }
    }

    pub fn available_balance(&self) -> u64 {
        self.load().iter()
            .filter(|tx| {
                tx.status != TxStatus::Pending
                || matches!(tx.direction, TxDirection::Mining)
            })
            .fold(0i64, |acc, tx| {
                match tx.direction {
                    TxDirection::Received
                    | TxDirection::Mining
                    | TxDirection::Change  => acc + tx.amount as i64,
                    TxDirection::Sent      => acc - (tx.amount + tx.fee) as i64,
                }
            })
            .max(0) as u64
    }

    pub fn pending_balance(&self) -> u64 {
        self.load().iter()
            .filter(|tx| tx.status == TxStatus::Pending)
            .fold(0i64, |acc, tx| {
                match tx.direction {
                    TxDirection::Received => acc + tx.amount as i64,
                    TxDirection::Sent     => acc - (tx.amount + tx.fee) as i64,
                    _ => acc,
                }
            })
            .max(0) as u64
    }
}