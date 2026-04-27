use std::fs;
use std::path::Path;
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use crate::mempool::{Mempool, MempoolTx};

// ==========================================
// WALLET SUR DISQUE
// ==========================================
#[derive(Serialize, Deserialize)]
pub struct WalletFile {
    pub address:       String,
    pub scan_private:  String,
    pub spend_private: String,
    pub balance:       u64,
    pub version:       String,
}

fn xor_encrypt(data: &[u8], password: &str) -> Vec<u8> {
    let key: Vec<u8> = {
        let mut h = Sha256::new();
        h.update(password.as_bytes());
        h.finalize().to_vec()
    };
    data.iter().enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect()
}

pub fn save_wallet(
    address: &str, scan_private: &[u8],
    spend_private: &[u8], balance: u64,
    password: &str, path: &str,
) -> bool {
    let wallet = WalletFile {
        address:       address.to_string(),
        scan_private:  hex::encode(scan_private),
        spend_private: hex::encode(spend_private),
        balance,
        version:       "1.0.0".to_string(),
    };
    let json      = serde_json::to_string(&wallet).unwrap();
    let encrypted = xor_encrypt(json.as_bytes(), password);
    fs::write(path, &encrypted).is_ok()
}

pub fn load_wallet(path: &str, password: &str) -> Option<WalletFile> {
    if !Path::new(path).exists() { return None; }
    let encrypted = fs::read(path).ok()?;
    let decrypted = xor_encrypt(&encrypted, password);
    let json      = String::from_utf8(decrypted).ok()?;
    serde_json::from_str::<WalletFile>(&json).ok()
}

pub fn wallet_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn update_balance(path: &str, password: &str, new_balance: u64) {
    if let Some(mut w) = load_wallet(path, password) {
        w.balance = new_balance;
        let json      = serde_json::to_string(&w).unwrap();
        let encrypted = xor_encrypt(json.as_bytes(), password);
        let _         = fs::write(path, encrypted);
    }
}

// ==========================================
// TX PENDANTES — utilise le Mempool unifié
// ==========================================
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingTx {
    pub tx_id:     String,
    pub sender:    String,
    pub receiver:  String,
    pub amount:    u64,
    pub fee:       u64,
    pub timestamp: String,
    pub claimed:   bool,
}

pub fn broadcast_tx(tx: PendingTx) {
    let mut mempool = Mempool::load();
    let mtx = MempoolTx::new(
        &tx.tx_id, &tx.sender, &tx.receiver,
        tx.amount, tx.fee,
    );
    mempool.add(mtx);
    crate::logger::log_tx(&format!(
        "Broadcast TX {} — {} GHST fee — {} → {}",
        &tx.tx_id[..16], tx.fee,
        &tx.sender[..16.min(tx.sender.len())],
        &tx.receiver[..16.min(tx.receiver.len())],
    ));
}

pub fn claim_incoming(my_address: &str) -> Vec<PendingTx> {
    let mut mempool = Mempool::load();
    let mut found   = vec![];

    for tx in mempool.txs.iter_mut() {
        if tx.receiver == my_address && !tx.claimed {
            found.push(PendingTx {
                tx_id:     tx.tx_id.clone(),
                sender:    tx.sender.clone(),
                receiver:  tx.receiver.clone(),
                amount:    tx.amount,
                fee:       tx.fee,
                timestamp: chrono::Utc::now()
                    .format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                claimed:   false,
            });
            tx.claimed = true;
        }
    }

    if !found.is_empty() {
        mempool.save();
    }

    found
}