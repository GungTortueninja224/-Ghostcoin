use std::fs;
use std::path::Path;
use serde::{Serialize, Deserialize};
use chrono::Utc;

pub const MIN_STAKE:        u64 = 100;   // minimum 100 GHST pour staker
pub const STAKE_REWARD_APY: f64 = 0.12; // 12% APY
pub const STAKE_FILE:       &str = "ghostcoin_stakes.json";

// ==========================================
// STAKE D'UN WALLET
// ==========================================
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Stake {
    pub address:      String,
    pub amount:       u64,
    pub locked_since: i64,    // timestamp
    pub unlock_time:  i64,    // timestamp (30 jours minimum)
    pub rewards:      u64,
    pub active:       bool,
}

impl Stake {
    pub fn new(address: &str, amount: u64) -> Option<Self> {
        if amount < MIN_STAKE {
            println!("❌ Minimum {} GHST requis pour staker", MIN_STAKE);
            return None;
        }

        let now         = Utc::now().timestamp();
        let unlock_time = now + (30 * 24 * 3600); // 30 jours

        Some(Self {
            address:      address.to_string(),
            amount,
            locked_since: now,
            unlock_time,
            rewards:      0,
            active:       true,
        })
    }

    // Calcule les récompenses accumulées
    pub fn calculate_rewards(&self) -> u64 {
        if !self.active { return self.rewards; }

        let now        = Utc::now().timestamp();
        let days_staked = (now - self.locked_since) as f64 / 86400.0;
        let daily_rate  = STAKE_REWARD_APY / 365.0;
        let reward      = (self.amount as f64 * daily_rate * days_staked) as u64;

        reward
    }

    pub fn can_unstake(&self) -> bool {
        Utc::now().timestamp() >= self.unlock_time
    }

    pub fn days_locked(&self) -> i64 {
        (Utc::now().timestamp() - self.locked_since) / 86400
    }

    pub fn days_until_unlock(&self) -> i64 {
        let remaining = self.unlock_time - Utc::now().timestamp();
        (remaining / 86400).max(0)
    }

    pub fn show(&self) {
        let rewards = self.calculate_rewards();
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║              💎 GHOSTCOIN STAKING INFO                  ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Staké        : {:<40} ║", format!("{} GHST", self.amount));
        println!("║  APY          : {:<40} ║", format!("{:.0}%", STAKE_REWARD_APY * 100.0));
        println!("║  Récompenses  : {:<40} ║", format!("{} GHST", rewards));
        println!("║  Staké depuis : {:<40} ║", format!("{} jours", self.days_locked()));
        println!("║  Unlock dans  : {:<40} ║",
            if self.can_unstake() {
                "✅ Disponible maintenant !".to_string()
            } else {
                format!("{} jours", self.days_until_unlock())
            });
        println!("║  Status       : {:<40} ║",
            if self.active { "✅ Actif" } else { "❌ Inactif" });
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}

// ==========================================
// GESTIONNAIRE DE STAKING
// ==========================================
pub struct StakingManager {
    pub stakes: Vec<Stake>,
}

impl StakingManager {
    pub fn load() -> Self {
        if !Path::new(STAKE_FILE).exists() {
            return Self { stakes: vec![] };
        }
        let json   = fs::read_to_string(STAKE_FILE).unwrap_or_default();
        let stakes = serde_json::from_str(&json).unwrap_or_default();
        Self { stakes }
    }

    pub fn save(&self) {
        let json = serde_json::to_string_pretty(&self.stakes).unwrap();
        let _    = fs::write(STAKE_FILE, json);
    }

    // Stake des GHST
    pub fn stake(&mut self, address: &str, amount: u64, balance: &mut u64) -> bool {
        if amount > *balance {
            println!("❌ Solde insuffisant ({} GHST disponible)", balance);
            return false;
        }

        match Stake::new(address, amount) {
            Some(stake) => {
                *balance -= amount;
                self.stakes.push(stake);
                self.save();
                println!("✅ {} GHST stakés avec succès !", amount);
                println!("   APY        : {:.0}%", STAKE_REWARD_APY * 100.0);
                println!("   Unlock dans : 30 jours");
                println!("   Récompenses : calculées quotidiennement");
                true
            }
            None => false,
        }
    }

    pub fn unstake(&mut self, address: &str, balance: &mut u64) -> bool {
    let pos = self.stakes.iter().position(|s|
        s.address == address && s.active
    );

    match pos {
        Some(i) => {
            if !self.stakes[i].can_unstake() {
                println!("❌ Stake verrouillé encore {} jours",
                    self.stakes[i].days_until_unlock());
                return false;
            }

            let rewards    = self.stakes[i].calculate_rewards();
            let amount     = self.stakes[i].amount;
            let total_back = amount + rewards;

            self.stakes[i].active  = false;
            self.stakes[i].rewards = rewards;

            *balance += total_back;
            self.save();

            println!("✅ Unstake réussi !");
            println!("   Capital récupéré : {} GHST", amount);
            println!("   Récompenses      : {} GHST", rewards);
            println!("   Total reçu       : {} GHST", total_back);
            true
        }
        None => {
            println!("❌ Aucun stake actif trouvé");
            false
        }
    }
}

    // Réclame seulement les récompenses
    pub fn claim_rewards(&mut self, address: &str, balance: &mut u64) -> u64 {
        let rewards: u64 = self.stakes.iter()
            .filter(|s| s.address == address && s.active)
            .map(|s| s.calculate_rewards())
            .sum();

        if rewards == 0 {
            println!("   Aucune récompense disponible");
            return 0;
        }

        *balance += rewards;
        self.save();
        println!("✅ {} GHST de récompenses réclamées !", rewards);
        rewards
    }

    pub fn get_stake(&self, address: &str) -> Option<&Stake> {
        self.stakes.iter().find(|s| s.address == address && s.active)
    }

    pub fn total_staked(&self) -> u64 {
        self.stakes.iter().filter(|s| s.active).map(|s| s.amount).sum()
    }

    pub fn show_all(&self) {
        println!("\n📊 Staking global : {} GHST stakés", self.total_staked());
        println!("   {} stakers actifs",
            self.stakes.iter().filter(|s| s.active).count());
    }
}