use crate::block::Block;
use crate::consensus::ConsensusConfig;

#[derive(Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: usize,
}

impl Blockchain {
    pub fn new() -> Self {
        let config = ConsensusConfig::new();
        Self {
            chain: vec![Block::genesis()],
            difficulty: config.difficulty,
        }
    }

    pub fn add_block(&mut self, data: String) -> Result<&Block, String> {
        let previous = self
            .chain
            .last()
            .ok_or_else(|| "Blockchain vide: bloc précédent introuvable".to_string())?;
        let block = Block::new(
            previous.index + 1,
            data,
            previous.hash.clone(),
            self.difficulty,
        );

        if !block.is_valid_successor(previous) {
            return Err("Bloc rejeté: validation du chaînage ou du hash échouée".to_string());
        }

        self.chain.push(block);
        self.chain
            .last()
            .ok_or_else(|| "Bloc ajouté mais introuvable".to_string())
    }

    pub fn is_valid(&self) -> bool {
        match self.chain.first() {
            Some(genesis) if genesis.index == 0 && genesis.previous_hash == "0" => {}
            _ => return false,
        }

        self.chain.iter().all(Block::is_valid_hash)
            && self
                .chain
                .windows(2)
                .all(|pair| pair[1].is_valid_successor(&pair[0]))
    }

    pub fn height(&self) -> u64 {
        self.chain.last().map(|block| block.index).unwrap_or(0)
    }

    pub fn last_hash(&self) -> &str {
        self.chain
            .last()
            .map(|block| block.hash.as_str())
            .unwrap_or("0")
    }
}
