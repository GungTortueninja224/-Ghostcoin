use crate::chain_state::ChainState;
use crate::config::{self, GhostCoinConfig};
use crate::node::{run_node, NodeState};
use crate::sync::{ChainSync, SharedChain};
use crate::web_server::start_web_server_on_port;
use std::fs;
use std::path::{Path, PathBuf};

fn reset_chain_files() -> [PathBuf; 2] {
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

    println!("RESET_CHAIN=true detected: resetting chain files...");
    for file in reset_chain_files() {
        if Path::new(&file).exists() {
            match fs::remove_file(&file) {
                Ok(_) => println!("  - removed {}", file.display()),
                Err(e) => println!("  - failed to remove {}: {}", file.display(), e),
            }
        } else {
            println!("  - missing {}", file.display());
        }
    }
}

fn resolve_p2p_port() -> u16 {
    env_port("RAILWAY_TCP_APPLICATION_PORT").unwrap_or_else(config::p2p_port)
}

fn resolve_web_port(p2p_port: u16) -> u16 {
    let port_from_public_binding = env_port("PORT");
    if let Some(port) = port_from_public_binding {
        if port != p2p_port {
            return port;
        }
    }

    env_port("GHOSTCOIN_WEB_PORT")
        .filter(|port| *port != p2p_port)
        .unwrap_or(8080)
}

pub async fn run_server_mode() {
    let app_config = GhostCoinConfig::new();
    reset_chain_if_requested();

    println!("============================================================");
    println!("               GHOSTCOIN NODE - SERVER MODE");
    println!("============================================================");
    println!("{} v{}", app_config.name, app_config.version);

    let state = ChainState::load();
    println!("Blockchain height : #{}", state.block_height);
    println!("Current supply    : {} GHST", state.minted_supply);
    println!("Block Explorer    : ghostcoin-production.up.railway.app");

    let p2p_port = resolve_p2p_port();
    let web_port = resolve_web_port(p2p_port);

    if web_port == p2p_port {
        eprintln!(
            "FATAL: web port {} conflicts with P2P port {}. Set PORT or GHOSTCOIN_WEB_PORT to a different value.",
            web_port, p2p_port
        );
        std::process::exit(1);
    }

    let shared_chain = SharedChain::new();
    let node = NodeState::new(p2p_port, shared_chain);
    let node_task = node.clone();
    tokio::spawn(async move {
        run_node(node_task).await;
    });
    println!("P2P node started on port {}", p2p_port);

    let bootstrap_peers = config::bootstrap_peers();
    if bootstrap_peers.is_empty() {
        println!(
            "Bootstrap disabled: set GHOSTCOIN_BOOTSTRAP_PEERS=host:port[,host:port] to recover chain state automatically."
        );
        println!(
            "Without a persistent volume or public bootstrap peer, Railway can restart at #0 after a reboot."
        );
    } else {
        println!("Bootstrap peers: {}", bootstrap_peers.join(", "));
        let bootstrap_chain = node.chain.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            let sync = ChainSync::new_with_chain(bootstrap_chain, bootstrap_peers);
            let added = sync.sync_from_peers().await;
            if added > 0 {
                println!("Bootstrap recovered {} block(s)", added);
            } else {
                println!("Bootstrap found no new blocks");
            }

            let pushed = sync.push_missing_blocks_to_peers().await;
            if pushed > 0 {
                println!(
                    "Bootstrap backfilled {} block(s) to lagging peer(s)",
                    pushed
                );
            }
        });
    }

    tokio::spawn(async move {
        let mut tick = 0u64;
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            tick += 1;
            let state = ChainState::load();
            println!(
                "[{} min] Height: #{} | Supply: {} GHST",
                tick, state.block_height, state.minted_supply
            );
        }
    });

    println!("Web server starting on port {}", web_port);
    println!("\nGhostCoin network online.");
    println!("Public explorer is available.\n");

    // Keep the web server on the main task so bind failures are visible and
    // Railway healthchecks reflect the real application state immediately.
    start_web_server_on_port(web_port).await;
}
