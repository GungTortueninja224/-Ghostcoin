// Consensus gardé minimal — le vrai mining est dans miner.rs
pub struct ProofOfWork;

impl ProofOfWork {
    pub fn new() -> Self { Self }
}

pub struct ConsensusConfig {
    pub difficulty: usize,
    pub block_time: u64,
    pub reward:     u64,
}

impl ConsensusConfig {
    pub fn new() -> Self {
        Self {
            difficulty: 4,
            block_time: 120,
            reward:     65,
        }
    }
}