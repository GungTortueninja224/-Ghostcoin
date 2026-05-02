use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub index: u64,
    pub timestamp: i64,
    pub data: String,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub difficulty: usize,
}

impl Block {
    pub fn genesis() -> Self {
        Self::new(0, "Genesis".to_string(), "0".to_string(), 2)
    }

    pub fn new(index: u64, data: String, previous_hash: String, difficulty: usize) -> Self {
        let timestamp = Utc::now().timestamp_millis();
        let mut block = Self {
            index,
            timestamp,
            data,
            previous_hash,
            hash: String::new(),
            nonce: 0,
            difficulty,
        };
        block.mine();
        block
    }

    pub fn calculate_hash(&self) -> String {
        let input = format!(
            "{}|{}|{}|{}|{}|{}",
            self.index, self.timestamp, self.data, self.previous_hash, self.nonce, self.difficulty
        );
        let mut hasher = Sha256::new();
        hasher.update(input.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn mine(&mut self) {
        let target = Self::target_prefix(self.difficulty);
        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
                break;
            }
            self.nonce = self.nonce.saturating_add(1);
        }
    }

    pub fn is_valid_successor(&self, previous: &Block) -> bool {
        self.index == previous.index + 1
            && self.previous_hash == previous.hash
            && self.is_valid_hash()
            && self.timestamp >= previous.timestamp
    }

    pub fn is_valid_hash(&self) -> bool {
        !self.hash.is_empty()
            && self.hash == self.calculate_hash()
            && self.hash.starts_with(&Self::target_prefix(self.difficulty))
    }

    fn target_prefix(difficulty: usize) -> String {
        "0".repeat(difficulty.min(64))
    }
}
