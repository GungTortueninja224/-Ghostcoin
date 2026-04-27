use rand::Rng;
use rand::rngs::OsRng;

// ==========================================
// DANDELION++ — Protection IP / Timing
// ==========================================

// Une transaction passe par 2 phases :
// 1. "Tige"     : passe par N noeuds aléatoires en secret
// 2. "Floraison": broadcast à tout le réseau

pub struct DandelionRouter {
    pub stem_length: usize,   // nombre de sauts avant floraison
    pub peers: Vec<String>,   // noeuds connus
}

#[derive(Debug, Clone)]
pub enum DandelionPhase {
    Stem { hops_remaining: usize, next_peer: String },
    Fluff,  // broadcast complet
}

impl DandelionRouter {
    pub fn new(peers: Vec<String>) -> Self {
        let mut rng = OsRng;
        // Longueur de tige aléatoire entre 2 et 4 sauts
        let stem_length = rng.gen_range(2..=4);
        Self { stem_length, peers }
    }

    // Décide comment propager une transaction
    pub fn route(&self, tx_id: &str) -> DandelionPhase {
        let mut rng = OsRng;

        if self.peers.is_empty() {
            // Pas de pairs → floraison directe
            return DandelionPhase::Fluff;
        }

        // Probabilité de floraison : 10% à chaque saut
        let fluff_chance: f64 = rng.gen();
        if fluff_chance < 0.10 {
            println!("🌸 TX {} → Floraison (broadcast)", &tx_id[..8]);
            return DandelionPhase::Fluff;
        }

        // Sinon : choisit un pair aléatoire pour la tige
        let idx       = rng.gen_range(0..self.peers.len());
        let next_peer = self.peers[idx].clone();

        println!("🌱 TX {} → Tige vers {}", &tx_id[..8], next_peer);

        DandelionPhase::Stem {
            hops_remaining: self.stem_length,
            next_peer,
        }
    }

    // Simule la propagation complète d'une transaction
    pub fn propagate(&self, tx_id: &str) {
        println!("\n📡 Propagation Dandelion++ pour TX: {}", tx_id);
        println!("   Longueur tige max : {} sauts", self.stem_length);

        let mut hops = 0;

        loop {
            let phase = self.route(tx_id);

            match phase {
                DandelionPhase::Stem { hops_remaining: _, next_peer } => {
                    hops += 1;
                    println!("   Saut {} → {} (origine masquée)", hops, next_peer);

                    if hops >= self.stem_length {
                        // Forcer floraison après N sauts max
                        println!("🌸 Floraison forcée après {} sauts", hops);
                        println!("✅ TX broadcastée — origine intraçable !");
                        break;
                    }
                }
                DandelionPhase::Fluff => {
                    println!("🌸 Floraison après {} sauts", hops);
                    println!("✅ TX broadcastée — origine intraçable !");
                    break;
                }
            }
        }
    }
}
