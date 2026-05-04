#![allow(dead_code, unused)]

mod anti_analysis;
mod atomic_swap;
mod block;
mod blockchain;
mod chain_state;
mod cli;
mod confidential;
mod config;
mod consensus;
mod dandelion;
mod explorer;
mod fees;
mod governance;
mod instance_lock;
mod logger;
mod masternode;
mod mempool;
mod mimblewimble;
mod miner;
mod network;
mod node;
mod portfolio;
mod quantum;
mod ring_signature;
mod seed;
mod server_mode;
mod staking;
mod stealth;
mod storage;
mod sync;
mod tests;
mod tor_network;
mod tx_store;
mod viewkey;
mod wallet;
mod web_server;
mod zkproof;

use config::{print_logo, print_tokenomics, GhostCoinConfig};
use node::{run_node, send_to_node, NodeMessage, NodeState};
use std::env;
use std::io::{self, Write};
use storage::{load_wallet, save_wallet, wallet_exists};
use sync::{ChainSync, SharedChain};
use tokio::time::{sleep, Duration};

const DEFAULT_SEED_NODE: &str = "ghostcoin-seed-1.fly.dev:8001";
const DEFAULT_STATUS_NODE: &str = "127.0.0.1:8001";

fn default_seed_nodes() -> Vec<String> {
    config::default_seed_nodes()
}

fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn get_wallet_path(address: &str) -> String {
    format!(
        "wallet_{}.ghst",
        &address
            .chars()
            .filter(|c| c.is_alphanumeric())
            .take(20)
            .collect::<String>()
    )
}

fn print_cli_help(binary_name: &str) {
    println!("Usage:");
    println!("  {}                  # Lance le mode wallet interactif", binary_name);
    println!(
        "  {} connect [ADDR]   # Teste la connectivite d'un noeud (Ping/Pong)",
        binary_name
    );
    println!(
        "  {} status [ADDR]    # Recupere le statut d'un noeud",
        binary_name
    );
    println!("  {} help             # Affiche cette aide", binary_name);
    println!();
    println!("Par defaut:");
    println!("  connect -> {}", DEFAULT_SEED_NODE);
    println!("  status  -> {}", DEFAULT_STATUS_NODE);
}

async fn run_cli_command(args: &[String]) -> bool {
    if args.len() <= 1 {
        return false;
    }

    let binary_name = args.first().map(String::as_str).unwrap_or("ghostcoin");
    let command = args[1].as_str();

    match command {
        "help" | "--help" | "-h" => {
            print_cli_help(binary_name);
            true
        }
        "connect" => {
            let addr = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_SEED_NODE);

            println!("Test de connexion vers {}...", addr);
            match send_to_node(addr, &NodeMessage::Ping).await {
                Some(NodeMessage::Pong) => println!("Connexion OK (Pong recu)."),
                Some(other) => println!("Reponse inattendue: {:?}", other),
                None => {}
            }
            true
        }
        "status" => {
            let addr = args
                .get(2)
                .map(String::as_str)
                .unwrap_or(DEFAULT_STATUS_NODE);

            println!("Demande de statut a {}...", addr);
            match send_to_node(addr, &NodeMessage::GetStatus).await {
                Some(NodeMessage::Status {
                    port,
                    peers,
                    mempool,
                    blocks,
                }) => {
                    println!("Statut du noeud:");
                    println!("  Port    : {}", port);
                    println!("  Peers   : {}", peers);
                    println!("  Mempool : {}", mempool);
                    println!("  Blocks  : {}", blocks);
                }
                Some(other) => println!("Reponse inattendue: {:?}", other),
                None => {}
            }
            true
        }
        _ => {
            println!("Commande inconnue: {}", command);
            print_cli_help(binary_name);
            true
        }
    }
}

#[tokio::main]
async fn main() {
    config::ensure_data_dir().expect("Failed to create data directory");

    let args: Vec<String> = env::args().collect();
    if run_cli_command(&args).await {
        return;
    }

    let _instance_lock = if config::is_server() {
        None
    } else {
        match instance_lock::InstanceLock::acquire() {
            Ok(lock) => Some(lock),
            Err(message) => {
                eprintln!("{}", message);
                return;
            }
        }
    };

    // ── MODE SERVEUR (Railway/VPS) ───────────────
    // Si la variable d'environnement GHOSTCOIN_SERVER est définie
    // → tourne en mode noeud sans interface CLI
    if config::is_server() {
        server_mode::run_server_mode().await;
        return;
    }

    // ── MODE WALLET (local) ──────────────────────
    print_logo();
    print_tokenomics();

    let config = GhostCoinConfig::new();
    println!("\n⚙️  {} v{} démarrage...\n", config.name, config.version);

    crate::logger::init_logger(false);

    // ── MENU CONNEXION ───────────────────────────
    println!("╔══════════════════════════════════════╗");
    println!("║      🔐 CONNEXION GHOSTCOIN          ║");
    println!("╠══════════════════════════════════════╣");
    println!("║  1. Créer un nouveau wallet          ║");
    println!("║  2. Charger un wallet existant       ║");
    println!("╚══════════════════════════════════════╝");

    let choice = read_input("Choix : ");

    let wallet_path;
    let password_used;
    let wallet_addr;
    let wallet_balance;

    match choice.as_str() {
        "1" => {
            println!("\n🆕 Création d'un nouveau wallet GhostCoin...");
            let pwd1 = read_input("🔑 Choisir une clé privée : ");
            let pwd2 = read_input("🔑 Confirmer la clé privée : ");

            if pwd1 != pwd2 {
                println!("❌ Les clés ne correspondent pas");
                return;
            }
            if pwd1.len() < 8 {
                println!("❌ Clé trop courte — minimum 8 caractères");
                return;
            }

            let w = wallet::Wallet::new_mainnet();
            wallet_addr = w.address.clone();
            wallet_path = get_wallet_path(&wallet_addr);
            password_used = pwd1.clone();
            wallet_balance = 0u64;

            save_wallet(
                &wallet_addr,
                w.keypair.scan_private.as_bytes(),
                w.keypair.spend_private.as_bytes(),
                0,
                &password_used,
                &wallet_path,
            );

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║           ✅ WALLET CRÉÉ AVEC SUCCÈS                    ║");
            println!("╠══════════════════════════════════════════════════════════╣");
            println!(
                "║ Adresse : {:<48} ║",
                &wallet_addr[..wallet_addr.len().min(48)]
            );
            println!(
                "║ Fichier : {:<48} ║",
                &wallet_path[..wallet_path.len().min(48)]
            );
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║ ⚠️  Garde ton fichier .ghst et ta clé privée en sécurité ║");
            println!("╚══════════════════════════════════════════════════════════╝");
        }

        "2" => {
            println!("\n📂 Charger un wallet");
            println!("  A. Entrer l'adresse du wallet");
            println!("  B. Entrer le nom du fichier .ghst");

            let load_choice = read_input("Choix (A/B) : ");
            let path_to_load = match load_choice.to_lowercase().as_str() {
                "a" => {
                    let addr = read_input("Adresse (PC1-...) : ");
                    get_wallet_path(&addr)
                }
                _ => read_input("Nom du fichier (.ghst) : "),
            };

            if !wallet_exists(&path_to_load) {
                println!("❌ Wallet non trouvé : {}", path_to_load);
                return;
            }

            let pwd = read_input("🔑 Clé privée : ");

            match load_wallet(&path_to_load, &pwd) {
                Some(w) => {
                    wallet_addr = w.address.clone();
                    wallet_path = path_to_load;
                    password_used = pwd;
                    wallet_balance = w.balance;
                    println!("✅ Wallet chargé !");
                    println!(
                        "   Adresse : {}...",
                        &wallet_addr[..20.min(wallet_addr.len())]
                    );
                    println!("   Solde   : {} GHST", wallet_balance);
                }
                None => {
                    println!("❌ Clé privée incorrecte");
                    return;
                }
            }
        }

        _ => {
            println!("❌ Choix invalide");
            return;
        }
    }

    // ── NOEUDS P2P ───────────────────────────────
    println!("\n🌐 Démarrage du réseau...");


    let shared_chain = SharedChain::new();
    let node1 = NodeState::new(8001, shared_chain.clone());
    let node2 = NodeState::new(8002, shared_chain.clone());
    let node3 = NodeState::new(8003, shared_chain.clone());

    node1.add_peer("127.0.0.1:8002");
    node1.add_peer("127.0.0.1:8003");
    node2.add_peer("127.0.0.1:8001");
    node2.add_peer("127.0.0.1:8003");
    node3.add_peer("127.0.0.1:8001");
    node3.add_peer("127.0.0.1:8002");
    for seed in default_seed_nodes() {
        node1.add_peer(&seed);
        node2.add_peer(&seed);
        node3.add_peer(&seed);
    }

    let n1 = node1.clone();
    let n2 = node2.clone();
    let n3 = node3.clone();

    tokio::spawn(async move { run_node(n1).await });
    tokio::spawn(async move { run_node(n2).await });
    tokio::spawn(async move { run_node(n3).await });

    sleep(Duration::from_millis(200)).await;

    send_to_node(
        "127.0.0.1:8002",
        &NodeMessage::Hello {
            from_port: 8001,
            version: 1,
            height: shared_chain.last_index(),
        },
    )
    .await;
    send_to_node(
        "127.0.0.1:8003",
        &NodeMessage::Hello {
            from_port: 8001,
            version: 1,
            height: shared_chain.last_index(),
        },
    )
    .await;

    sleep(Duration::from_millis(100)).await;

    println!("\n📊 Réseau {} :", config.name);
    let mut bootstrap_peers = vec![
        "127.0.0.1:8001".to_string(),
        "127.0.0.1:8002".to_string(),
        "127.0.0.1:8003".to_string(),
    ];
    bootstrap_peers.extend(default_seed_nodes());

    let bootstrap_sync = ChainSync::new_with_chain(shared_chain.clone(), bootstrap_peers);

    let synced_blocks = bootstrap_sync.sync_from_peers().await;
    if synced_blocks > 0 {
        println!("Sync P2P terminee : {} bloc(s) importe(s)", synced_blocks);
    } else {
        println!("Sync P2P : aucun nouveau bloc importe");
    }

    let pushed_blocks = bootstrap_sync.push_missing_blocks_to_peers().await;
    if pushed_blocks > 0 {
        println!(
            "Push P2P vers les seeds : {} bloc(s) envoye(s) pour rattrapage",
            pushed_blocks
        );
    }

    for port in [8001u16, 8002, 8003] {
        let addr = format!("127.0.0.1:{}", port);
        if let Some(NodeMessage::Status {
            port,
            peers,
            mempool,
            blocks,
        }) = send_to_node(&addr, &NodeMessage::GetStatus).await
        {
            println!(
                "   🟢 Noeud {} | Pairs: {} | Mempool: {} | Blocs: {}",
                port, peers, mempool, blocks
            );
        }
    }
    println!("\n✅ {} opérationnel !\n", config.name);

    let mut cli = cli::Cli::new_with_balance(
        shared_chain,
        &wallet_path,
        &password_used,
        &wallet_addr,
        wallet_balance,
    );
    cli.run();
}
