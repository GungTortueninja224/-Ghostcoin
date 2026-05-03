use crate::chain_state::ChainState;
use crate::config::{self, GhostCoinConfig};
use crate::node::{run_node, NodeState};
use crate::sync::{ChainSync, SharedChain};
use crate::web_server::start_web_server_on_port;
use std::fs;
use std::path::Path;

fn reset_chain_files() -> [std::path::PathBuf; 2] {
    [config::blocks_file(), config::chain_file()]
}

fn env_port(key: &str) -> Option<u16> {
    std::env::var(key).ok()?.parse::<u16>().ok()
}

fn env_flag(key: &str) -> bool {
    matches!(
        std::env::var(key),
        Ok(ref v) if v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
    )
}

fn reset_chain_if_requested() {
    if !env_flag("RESET_CHAIN") {
        return;
    }

    println!("RESET_CHAIN=true detecte: reinitialisation des fichiers chain...");
    for file in reset_chain_files() {
        if Path::new(&file).exists() {
            match fs::remove_file(&file) {
                Ok(_) => println!("  - supprime {}", file.display()),
                Err(e) => println!("  - echec suppression {}: {}", file.display(), e),
            }
        } else {
            println!("  - absent {}", file.display());
        }
    }
}

pub async fn run_server_mode() {
    let config = GhostCoinConfig::new();
    reset_chain_if_requested();

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
    let p2p_port = env_port("RAILWAY_TCP_APPLICATION_PORT").unwrap_or_else(config::p2p_port);
    let shared_chain = SharedChain::new();
    let node = NodeState::new(p2p_port, shared_chain);
    let n = node.clone();
    tokio::spawn(async move {
        run_node(n).await;
    });
    println!("ðŸŒ Noeud P2P TCP demarre sur port {}", p2p_port);

    let bootstrap_peers = config::bootstrap_peers();
    if bootstrap_peers.is_empty() {
        println!(
            "Bootstrap desactive: aucun peer configure. Definis GHOSTCOIN_BOOTSTRAP_PEERS=host:port[,host:port] pour recuperer la chaine au demarrage."
        );
        println!(
            "Sans volume persistant ni peer bootstrap public, Railway redemarrera a #0 apres restart."
        );
    } else {
        println!("Bootstrap peers configures: {}", bootstrap_peers.join(", "));
        let bootstrap_chain = node.chain.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let sync = ChainSync::new_with_chain(bootstrap_chain, bootstrap_peers);
            let added = sync.sync_from_peers().await;
            if added > 0 {
                println!("Bootstrap: {} bloc(s) recuperes depuis les peers", added);
            } else {
                println!("Bootstrap: aucun bloc recupere depuis les peers configures");
            }
        });
    }

    // Keep explorer available. If PORT conflicts with P2P, move web server.
    let railway_port = env_port("PORT").unwrap_or(p2p_port);
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
