use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::Utc;

pub const MASTERNODE_COLLATERAL: u64  = 1000;  // 1000 GHST requis
pub const MASTERNODE_REWARD_PCT: f64  = 0.60;  // 60% des frais du bloc
pub const MASTERNODE_FILE:       &str = "ghostcoin_masternodes.json";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MasternodeStatus {
    Active,
    Inactive,
    Waiting, // en attente de confirmation
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Masternode {
    pub id:          String,
    pub owner:       String,
    pub address:     String,  // adresse réseau
    pub collateral:  u64,
    pub started_at:  i64,
    pub last_reward: i64,
    pub total_earned: u64,
    pub status:      MasternodeStatus,
}

impl Masternode {
    pub fn new(owner: &str, address: &str) -> Self {
        let id = format!("mn_{:016x}", rand::random::<u64>());
        Self {
            id,
            owner:        owner.to_string(),
            address:      address.to_string(),
            collateral:   MASTERNODE_COLLATERAL,
            started_at:   Utc::now().timestamp(),
            last_reward:  Utc::now().timestamp(),
            total_earned: 0,
            status:       MasternodeStatus::Waiting,
        }
    }

    pub fn days_active(&self) -> i64 {
        (Utc::now().timestamp() - self.started_at) / 86400
    }

    pub fn activate(&mut self) {
        self.status = MasternodeStatus::Active;
        println!("✅ Masternode activé !");
    }

    pub fn earn_reward(&mut self, amount: u64) {
        self.total_earned += amount;
        self.last_reward   = Utc::now().timestamp();
    }

    pub fn show(&self) {
        let status_str = match &self.status {
            MasternodeStatus::Active   => "✅ Actif",
            MasternodeStatus::Inactive => "❌ Inactif",
            MasternodeStatus::Waiting  => "⏳ En attente",
        };

        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║              🖥️  GHOSTCOIN MASTERNODE                   ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  ID         : {:<42} ║", &self.id[..20]);
        println!("║  Owner      : {:<42} ║", &self.owner[..self.owner.len().min(42)]);
        println!("║  Collatéral : {:<42} ║", format!("{} GHST", self.collateral));
        println!("║  Status     : {:<42} ║", status_str);
        println!("║  Actif dep. : {:<42} ║", format!("{} jours", self.days_active()));
        println!("║  Total gagné: {:<42} ║", format!("{} GHST", self.total_earned));
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Services   : InstantSend, PrivateSend, Gouvernance     ║");
        println!("║  Reward     : {:.0}% des frais de chaque bloc           ║",
            MASTERNODE_REWARD_PCT * 100.0);
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}

// ==========================================
// GESTIONNAIRE MASTERNODES
// ==========================================
pub struct MasternodeManager {
    pub nodes: Vec<Masternode>,
}

impl MasternodeManager {
    pub fn load() -> Self {
        if !Path::new(MASTERNODE_FILE).exists() {
            return Self { nodes: vec![] };
        }
        let json  = fs::read_to_string(MASTERNODE_FILE).unwrap_or_default();
        let nodes = serde_json::from_str(&json).unwrap_or_default();
        Self { nodes }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(&self.nodes).unwrap();
        let _    = fs::write(MASTERNODE_FILE, json);
    }

    // Enregistre un nouveau masternode
    pub fn register(
        &mut self,
        owner:   &str,
        address: &str,
        balance: &mut u64,
    ) -> bool {
        if *balance < MASTERNODE_COLLATERAL {
            println!("❌ {} GHST requis comme collatéral", MASTERNODE_COLLATERAL);
            println!("   Ton solde : {} GHST", balance);
            return false;
        }

        // Vérifie pas de doublon
        if self.nodes.iter().any(|n| n.owner == owner && n.status == MasternodeStatus::Active) {
            println!("❌ Tu as déjà un masternode actif");
            return false;
        }

        let mut mn = Masternode::new(owner, address);
        mn.activate();
        *balance -= MASTERNODE_COLLATERAL;

        println!("✅ Masternode enregistré !");
        println!("   Collatéral verrouillé : {} GHST", MASTERNODE_COLLATERAL);
        println!("   Reward : {:.0}% des frais de chaque bloc", MASTERNODE_REWARD_PCT * 100.0);

        self.nodes.push(mn);
        self.save();
        true
    }

    // Distribue les récompenses aux masternodes actifs
    pub fn distribute_rewards(&mut self, block_fees: u64) -> u64 {
        let active_count = self.nodes.iter()
            .filter(|n| n.status == MasternodeStatus::Active)
            .count();

        if active_count == 0 { return 0; }

        let total_reward   = (block_fees as f64 * MASTERNODE_REWARD_PCT) as u64;
        let reward_per_mn  = total_reward / active_count as u64;

        for node in self.nodes.iter_mut() {
            if node.status == MasternodeStatus::Active {
                node.earn_reward(reward_per_mn);
            }
        }

        self.save();

        if reward_per_mn > 0 {
            println!("💎 Masternodes : {} GHST distribués à {} noeuds",
                total_reward, active_count);
        }

        total_reward
    }

    // Récupère le collatéral (désactive le masternode)
    pub fn unregister(&mut self, owner: &str, balance: &mut u64) -> bool {
        match self.nodes.iter_mut()
            .find(|n| n.owner == owner && n.status == MasternodeStatus::Active)
        {
            Some(mn) => {
                mn.status  = MasternodeStatus::Inactive;
                *balance  += MASTERNODE_COLLATERAL;
                self.save();
                println!("✅ Masternode désactivé");
                println!("   {} GHST collatéral récupéré", MASTERNODE_COLLATERAL);
                true
            }
            None => {
                println!("❌ Aucun masternode actif trouvé");
                false
            }
        }
    }

    pub fn active_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.status == MasternodeStatus::Active).count()
    }

    pub fn show_all(&self) {
        println!("\n🖥️  GHOSTCOIN MASTERNODES — {} actif(s)", self.active_count());
        for mn in &self.nodes {
            mn.show();
        }
    }
}