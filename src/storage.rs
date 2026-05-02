use std::fs;
use std::path::Path;

use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::mempool::{Mempool, MempoolTx};

#[derive(Serialize, Deserialize)]
pub struct WalletFile {
    pub address: String,
    pub scan_private: String,
    pub spend_private: String,
    pub balance: u64,
    pub version: String,
}

const WALLET_V2_HEADER: &[u8] = b"GHSTW2";
const WALLET_SALT_LEN: usize = 16;
const WALLET_TAG_LEN: usize = 32;

fn sha256(parts: &[&[u8]]) -> Vec<u8> {
    let mut h = Sha256::new();
    for part in parts {
        h.update(part);
    }
    h.finalize().to_vec()
}

fn stream_xor(data: &[u8], password: &str, salt: &[u8]) -> Vec<u8> {
    let mut key_stream = Vec::with_capacity(data.len());
    let mut counter = 0u64;

    while key_stream.len() < data.len() {
        let counter_bytes = counter.to_le_bytes();
        let block = sha256(&[password.as_bytes(), salt, &counter_bytes]);
        let remaining = data.len() - key_stream.len();
        key_stream.extend(block.into_iter().take(remaining.min(32)));
        counter = counter.saturating_add(1);
    }

    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key_stream[i])
        .collect()
}

fn encrypt_wallet_v2(data: &[u8], password: &str) -> Vec<u8> {
    let mut salt = [0u8; WALLET_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let ciphertext = stream_xor(data, password, &salt);
    let tag = sha256(&[password.as_bytes(), &salt, &ciphertext]);

    let mut out = Vec::with_capacity(
        WALLET_V2_HEADER.len() + WALLET_SALT_LEN + WALLET_TAG_LEN + ciphertext.len(),
    );
    out.extend_from_slice(WALLET_V2_HEADER);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&tag);
    out.extend_from_slice(&ciphertext);
    out
}

fn decrypt_wallet_v2(data: &[u8], password: &str) -> Option<Vec<u8>> {
    let min_len = WALLET_V2_HEADER.len() + WALLET_SALT_LEN + WALLET_TAG_LEN;
    if data.len() < min_len || !data.starts_with(WALLET_V2_HEADER) {
        return None;
    }

    let salt_start = WALLET_V2_HEADER.len();
    let tag_start = salt_start + WALLET_SALT_LEN;
    let cipher_start = tag_start + WALLET_TAG_LEN;
    let salt = &data[salt_start..tag_start];
    let stored_tag = &data[tag_start..cipher_start];
    let ciphertext = &data[cipher_start..];
    let expected_tag = sha256(&[password.as_bytes(), salt, ciphertext]);

    if stored_tag != expected_tag.as_slice() {
        return None;
    }

    Some(stream_xor(ciphertext, password, salt))
}

fn legacy_xor(data: &[u8], password: &str) -> Vec<u8> {
    let key = sha256(&[password.as_bytes()]);
    data.iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % key.len()])
        .collect()
}

pub fn save_wallet(
    address: &str,
    scan_private: &[u8],
    spend_private: &[u8],
    balance: u64,
    password: &str,
    path: &str,
) -> bool {
    let wallet = WalletFile {
        address: address.to_string(),
        scan_private: hex::encode(scan_private),
        spend_private: hex::encode(spend_private),
        balance,
        version: "2.0.0".to_string(),
    };

    match serde_json::to_string(&wallet) {
        Ok(json) => fs::write(path, encrypt_wallet_v2(json.as_bytes(), password)).is_ok(),
        Err(_) => false,
    }
}

pub fn load_wallet(path: &str, password: &str) -> Option<WalletFile> {
    if !Path::new(path).exists() {
        return None;
    }

    let encrypted = fs::read(path).ok()?;
    let decrypted =
        decrypt_wallet_v2(&encrypted, password).unwrap_or_else(|| legacy_xor(&encrypted, password));
    let json = String::from_utf8(decrypted).ok()?;
    serde_json::from_str::<WalletFile>(&json).ok()
}

pub fn wallet_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn update_balance(path: &str, password: &str, new_balance: u64) {
    if let Some(mut w) = load_wallet(path, password) {
        w.balance = new_balance;
        if let Ok(json) = serde_json::to_string(&w) {
            let encrypted = encrypt_wallet_v2(json.as_bytes(), password);
            let _ = fs::write(path, encrypted);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PendingTx {
    pub tx_id: String,
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: String,
    pub claimed: bool,
}

pub fn broadcast_tx(tx: PendingTx) {
    let mut mempool = Mempool::load();
    let mtx = MempoolTx::new(&tx.tx_id, &tx.sender, &tx.receiver, tx.amount, tx.fee);
    mempool.add(mtx);

    crate::logger::log_tx(&format!(
        "Broadcast TX {} - {} GHST fee - {} -> {}",
        &tx.tx_id[..16.min(tx.tx_id.len())],
        tx.fee,
        &tx.sender[..16.min(tx.sender.len())],
        &tx.receiver[..16.min(tx.receiver.len())],
    ));
}

pub fn claim_incoming(my_address: &str) -> Vec<PendingTx> {
    let mut mempool = Mempool::load();
    let mut found = vec![];

    for tx in mempool.txs.iter_mut() {
        if tx.receiver == my_address && !tx.claimed {
            found.push(PendingTx {
                tx_id: tx.tx_id.clone(),
                sender: tx.sender.clone(),
                receiver: tx.receiver.clone(),
                amount: tx.amount,
                fee: tx.fee,
                timestamp: chrono::Utc::now()
                    .format("%Y-%m-%d %H:%M:%S UTC")
                    .to_string(),
                claimed: false,
            });
            tx.claimed = true;
        }
    }

    if !found.is_empty() {
        mempool.save();
    }

    found
}
