use std::sync::{Arc, Mutex};
use std::fs;
use std::path::Path;
use crate::miner::MinedBlock;
use crate::node::{NodeMessage, send_to_node};

const BLOCKS_FILE: &str = "ghostcoin_blocks.json";

#[derive(Clone)]
pub struct SharedChain {
    pub blocks: Arc<Mutex<Vec<MinedBlock>>>,
}

impl SharedChain {
    pub fn new() -> Self {
        // Charge les blocs depuis le disque au démarrage
        let blocks = Self::load_from_disk();
        println!("📦 {} bloc(s) chargé(s) depuis le disque", blocks.len());

        Self {
            blocks: Arc::new(Mutex::new(blocks)),
        }
    }

    // Charge les blocs depuis le fichier JSON
    fn load_from_disk() -> Vec<MinedBlock> {
        if !Path::new(BLOCKS_FILE).exists() {
            return vec![];
        }
        let json = fs::read_to_string(BLOCKS_FILE).unwrap_or_default();
        serde_json::from_str(&json).unwrap_or_default()
    }

    // Sauvegarde tous les blocs sur le disque
    fn save_to_disk(blocks: &[MinedBlock]) {
        let json = serde_json::to_string_pretty(blocks).unwrap();
        let _    = fs::write(BLOCKS_FILE, json);
    }

    pub fn add_block(&self, block: MinedBlock) {
        let mut chain = self.blocks.lock().unwrap();
        chain.push(block);
        // Sauvegarde immédiatement sur disque
        Self::save_to_disk(&chain);
        println!("💾 Bloc sauvegardé sur disque");
    }

    pub fn length(&self) -> usize {
        self.blocks.lock().unwrap().len()
    }

    pub fn last_hash(&self) -> String {
        let chain = self.blocks.lock().unwrap();
        chain.last()
            .map(|b| b.hash.clone())
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn last_index(&self) -> u32 {
        let chain = self.blocks.lock().unwrap();
        chain.last().map(|b| b.index).unwrap_or(0)
    }
}

pub struct ChainSync {
    pub chain: SharedChain,
    pub peers: Vec<String>,
}

impl ChainSync {
    pub fn new(peers: Vec<String>) -> Self {
        Self {
            chain: SharedChain::new(),
            peers,
        }
    }

    pub async fn broadcast_block(&self, block: &MinedBlock) {
        let msg = NodeMessage::NewBlock {
            block_index: block.index,
            hash:        block.hash.clone(),
        };
        for peer in &self.peers {
            send_to_node(peer, &msg).await;
        }
    }

    pub async fn check_peers(&self) {
        println!("\n🔍 Vérification des pairs :");
        for peer in &self.peers {
            match send_to_node(peer, &NodeMessage::Ping).await {
                Some(_) => println!("   🟢 {} — en ligne", peer),
                None    => println!("   🔴 {} — hors ligne", peer),
            }
        }
    }
}