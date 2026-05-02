use crate::chain_state::ChainState;
use crate::config::GhostCoinConfig;
use crate::node::{run_node, NodeState};
use crate::sync::SharedChain;
use crate::web_server::start_web_server_on_port;

fn env_port(key: &str) -> Option<u16> {
    std::env::var(key).ok()?.parse::<u16>().ok()
}

pub async fn run_server_mode() {
    let config = GhostCoinConfig::new();

    println!("â•”â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•—");
    println!("â•‘         ðŸ‘» GHOSTCOIN NODE â€” SERVER MODE                 â•‘");
    println!("â• â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•£");
    println!("â•‘  {} v{}                                     â•‘", config.name, config.version);
    println!("â•šâ•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•");

    let state = ChainState::load();
    println!("ðŸ“¦ Blockchain height : #{}", state.block_height);
    println!("ðŸ’° Supply actuel     : {} GHST", state.minted_supply);
    println!("ðŸŒ Block Explorer    : ghostcoin-production.up.railway.app");

    // Railway TCP proxy forwards to this internal port.
    let p2p_port = env_port("RAILWAY_TCP_APPLICATION_PORT")
        .or_else(|| env_port("GHOSTCOIN_P2P_PORT"))
        .unwrap_or(8001);
    let shared_chain = SharedChain::new();
    let node = NodeState::new(p2p_port, shared_chain);
    let n = node.clone();
    tokio::spawn(async move {
        run_node(n).await;
    });
    println!("ðŸŒ Noeud P2P TCP demarre sur port {}", p2p_port);

    // Keep explorer available. If PORT conflicts with P2P, move web server.
    let railway_port = env_port("PORT").unwrap_or(8001);
    let web_port = if railway_port == p2p_port {
        env_port("GHOSTCOIN_WEB_PORT").unwrap_or(8080)
    } else {
        railway_port
    };

    if web_port == p2p_port {
        println!("âš ï¸  Web server desactive (conflit port {})", web_port);
    } else {
        tokio::spawn(async move {
            start_web_server_on_port(web_port).await;
        });
        println!("ðŸŒ Web server demarre sur port {}", web_port);
    }

    println!("\nâœ… GhostCoin Network en ligne !");
    println!("   Block Explorer accessible publiquement\n");

    let mut tick = 0u64;
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        tick += 1;
        let state = ChainState::load();
        println!(
            "â±ï¸  [{} min] Height: #{} | Supply: {} GHST",
            tick, state.block_height, state.minted_supply
        );
    }
}
