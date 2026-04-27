use sha2::{Sha256, Digest};

pub struct Block {
    pub index: u32,
    pub timestamp: u128,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
}

impl Block {
    pub fn new(index: u32, data: String, previous_hash: String) -> Self {
        let timestamp = chrono::Utc::now().timestamp_millis() as u128;
        let hash = Self::calculate_hash(index, &data, &previous_hash, timestamp);
        Block { index, timestamp, data, previous_hash, hash }
    }

    fn calculate_hash(index: u32, data: &str, previous_hash: &str, timestamp: u128) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}{}", index, data, previous_hash, timestamp));
        format!("{:x}", hasher.finalize())
    }
}