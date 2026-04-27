// ==========================================
// GHOSTCOIN — CONFIGURATION OFFICIELLE
// ==========================================

pub struct GhostCoinConfig {
    pub name:             &'static str,
    pub symbol:           &'static str,
    pub max_supply:       u64,
    pub block_reward:     u64,
    pub halving_interval: u64,
    pub block_time:       u64,
    pub difficulty:       usize,
    pub version:          &'static str,
    pub port:             u16,
}

impl GhostCoinConfig {
    pub fn new() -> Self {
        Self {
            name:             "GhostCoin",
            symbol:           "GHST",
            max_supply:       50_000_000,
            block_reward:     65,
            halving_interval: 210_000,  // récompense divisée par 2 tous les 210k blocs
            block_time:       120,      // 2 minutes par bloc
            difficulty:       4,
            version:          "1.0.0",
            port:             8001,
        }
    }

    // Calcule la récompense actuelle selon le bloc
    pub fn current_reward(&self, block_height: u64) -> u64 {
        let halvings = block_height / self.halving_interval;
        if halvings >= 64 {
            return 0; // Plus de récompense après 64 halvings
        }
        self.block_reward >> halvings // divise par 2 à chaque halving
    }

    // Vérifie si le supply max est atteint
    pub fn is_supply_maxed(&self, current_supply: u64) -> bool {
        current_supply >= self.max_supply
    }
}

// ==========================================
// LOGO ASCII GHOSTCOIN
// ==========================================
pub fn print_logo() {
    println!(r#"
  ██████╗ ██╗  ██╗ ██████╗ ███████╗████████╗
 ██╔════╝ ██║  ██║██╔═══██╗██╔════╝╚══██╔══╝
 ██║  ███╗███████║██║   ██║███████╗   ██║   
 ██║   ██║██╔══██║██║   ██║╚════██║   ██║   
 ╚██████╔╝██║  ██║╚██████╔╝███████║   ██║   
  ╚═════╝ ╚═╝  ╚═╝ ╚═════╝ ╚══════╝   ╚═╝   
                                             
  ██████╗ ██████╗ ██╗███╗   ██╗             
 ██╔════╝██╔═══██╗██║████╗  ██║             
 ██║     ██║   ██║██║██╔██╗ ██║             
 ██║     ██║   ██║██║██║╚██╗██║             
 ╚██████╗╚██████╔╝██║██║ ╚████║             
  ╚═════╝ ╚═════╝ ╚═╝╚═╝  ╚═══╝             

  🔒 Privacy • Speed • Freedom
  Symbol : GHST  |  Supply : 50,000,000
  Version: 1.0.0 |  Network: Mainnet
    "#);
}

// ==========================================
// TOKENOMICS
// ==========================================
pub fn print_tokenomics() {
    let config = GhostCoinConfig::new();

    println!("╔══════════════════════════════════════════════╗");
    println!("║         💎 GHOSTCOIN TOKENOMICS             ║");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  Nom          : {:<28} ║", config.name);
    println!("║  Symbole      : {:<28} ║", config.symbol);
    println!("║  Supply max   : {:<28} ║", "50,000,000 GHST");
    println!("║  Récompense   : {:<28} ║", "65 GHST / bloc");
    println!("║  Halving      : {:<28} ║", "tous les 210,000 blocs");
    println!("║  Temps/bloc   : {:<28} ║", "~2 minutes");
    println!("║  Difficulté   : {:<28} ║", "Ajustable");
    println!("║  Algorithme   : {:<28} ║", "SHA256 + zk-SNARKs");
    println!("║  Privacy      : {:<28} ║", "Ring Sig + Stealth + CT");
    println!("║  Réseau       : {:<28} ║", "P2P + Dandelion++");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  📊 DISTRIBUTION                            ║");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  Mineurs      : {:<28} ║", "100% (aucune premine)");
    println!("║  Fondateurs   : {:<28} ║", "0% (équitable)");
    println!("║  ICO/Vente    : {:<28} ║", "0% (minable seulement)");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  🔒 PRIVACY FEATURES                        ║");
    println!("╠══════════════════════════════════════════════╣");
    println!("║  • Stealth Addresses  (destinataire caché)  ║");
    println!("║  • Ring Signatures    (expéditeur caché)    ║");
    println!("║  • Confidential TX    (montants cachés)     ║");
    println!("║  • zk-SNARKs          (preuve sans révéler) ║");
    println!("║  • Dandelion++        (IP cachée)           ║");
    println!("╚══════════════════════════════════════════════╝");
}

// ==========================================
// HALVING SCHEDULE
// ==========================================
pub fn print_halving_schedule() {
    let config = GhostCoinConfig::new();

    println!("\n📅 GHOSTCOIN — Calendrier Halving");
    println!("─────────────────────────────────────────────");
    println!("{:<10} {:<15} {:<15} {:<15}",
        "Halving", "Bloc", "Récompense", "Supply miné");

    let mut total = 0u64;
    for i in 0..6 {
        let bloc   = i * config.halving_interval;
        let reward = config.current_reward(bloc);
        let mined  = i * config.halving_interval * config.current_reward(
            if i == 0 { 0 } else { (i-1) * config.halving_interval }
        );
        total += mined;
        println!("{:<10} {:<15} {:<15} {:<15}",
            format!("#{}", i),
            format!("{}", bloc),
            format!("{} GHST", reward),
            format!("{} GHST", total.min(config.max_supply)),
        );
    }
    println!("─────────────────────────────────────────────");
}