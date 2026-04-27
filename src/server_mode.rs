use crate::chain_state::ChainState;
use crate::config::GhostCoinConfig;
use crate::web_server::start_web_server;

pub async fn run_server_mode() {
    let config = GhostCoinConfig::new();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         👻 GHOSTCOIN NODE — SERVER MODE                 ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  {} v{}                                     ║", config.name, config.version);
    println!("╚══════════════════════════════════════════════════════════╝");

    let state = ChainState::load();
    println!("📦 Blockchain height : #{}", state.block_height);
    println!("💰 Supply actuel     : {} GHST", state.minted_supply);
    println!("🌐 Block Explorer    : ghostcoin-production.up.railway.app");

    // Démarre le serveur web en arrière-plan
    tokio::spawn(async {
        start_web_server().await;
    });

    println!("\n✅ GhostCoin Network en ligne !");
    println!("   Block Explorer accessible publiquement\n");

    // Boucle infinie
    let mut tick = 0u64;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        tick += 1;
        let state = ChainState::load();
        println!("⏱️  [{} min] Height: #{} | Supply: {} GHST",
            tick, state.block_height, state.minted_supply);
    }
}