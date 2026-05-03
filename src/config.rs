use std::path::PathBuf;

// ==========================================
// GHOSTCOIN - CONFIGURATION OFFICIELLE
// ==========================================

pub fn is_server() -> bool {
    matches!(
        std::env::var("GHOSTCOIN_SERVER"),
        Ok(ref v) if v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
    )
}

pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("GHOSTCOIN_DATA_DIR") {
        return PathBuf::from(dir);
    }

    if is_server() {
        PathBuf::from("/app/data")
    } else {
        PathBuf::from(".")
    }
}

pub fn blocks_file() -> PathBuf {
    data_dir().join("ghostcoin_blocks.json")
}

pub fn chain_file() -> PathBuf {
    data_dir().join("ghostcoin_chain.json")
}

pub fn p2p_port() -> u16 {
    std::env::var("GHOSTCOIN_P2P_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8001)
}

pub fn web_port() -> u16 {
    std::env::var("PORT")
        .or_else(|_| std::env::var("GHOSTCOIN_WEB_PORT"))
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8080)
}

pub fn bootstrap_peers() -> Vec<String> {
    std::env::var("GHOSTCOIN_BOOTSTRAP_PEERS")
        .map(|value| {
            value
                .split(',')
                .map(str::trim)
                .filter(|peer| !peer.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn ensure_data_dir() -> std::io::Result<()> {
    let dir = data_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
        println!("[config] created data dir: {}", dir.display());
    }
    Ok(())
}

pub struct GhostCoinConfig {
    pub name: &'static str,
    pub symbol: &'static str,
    pub max_supply: u64,
    pub block_reward: u64,
    pub halving_interval: u64,
    pub block_time: u64,
    pub difficulty: usize,
    pub version: &'static str,
    pub port: u16,
}

impl GhostCoinConfig {
    pub fn new() -> Self {
        Self {
            name: "GhostCoin",
            symbol: "GHST",
            max_supply: 50_000_000,
            block_reward: 65,
            halving_interval: 210_000,
            block_time: 120,
            difficulty: 4,
            version: "1.0.0",
            port: p2p_port(),
        }
    }

    pub fn current_reward(&self, block_height: u64) -> u64 {
        let halvings = block_height / self.halving_interval;
        if halvings >= 64 {
            return 0;
        }
        self.block_reward >> halvings
    }

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

  Privacy • Speed • Freedom
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

    println!("\n📅 GHOSTCOIN - Calendrier Halving");
    println!("─────────────────────────────────────────────");
    println!(
        "{:<10} {:<15} {:<15} {:<15}",
        "Halving", "Bloc", "Récompense", "Supply miné"
    );

    let mut total = 0u64;
    for i in 0..6 {
        let bloc = i * config.halving_interval;
        let reward = config.current_reward(bloc);
        let mined = i
            * config.halving_interval
            * config.current_reward(if i == 0 {
                0
            } else {
                (i - 1) * config.halving_interval
            });
        total += mined;
        println!(
            "{:<10} {:<15} {:<15} {:<15}",
            format!("#{}", i),
            format!("{}", bloc),
            format!("{} GHST", reward),
            format!("{} GHST", total.min(config.max_supply)),
        );
    }
    println!("─────────────────────────────────────────────");
}
