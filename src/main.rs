mod block;
mod blockchain;
mod stealth;
mod confidential;
mod ring_signature;
mod zkproof;
mod network;
mod mempool;
mod consensus;
mod wallet;
mod dandelion;
mod cli;
mod node;
mod config;
mod storage;
mod sync;
mod miner;
mod viewkey;
mod atomic_swap;
mod anti_analysis;
mod fees;
mod explorer;
mod chain_state;
mod tx_store;
mod seed;
mod logger;
mod tests;
mod quantum;
mod mimblewimble;
mod tor_network;
mod staking;
mod governance;
mod masternode;
mod portfolio;
mod server_mode;

use tokio::time::{sleep, Duration};
use node::{NodeState, NodeMessage, run_node, send_to_node};
use config::{print_logo, print_tokenomics, GhostCoinConfig};
use sync::SharedChain;
use storage::{save_wallet, load_wallet, wallet_exists};
use std::io::{self, Write};
use std::env;

fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn get_wallet_path(address: &str) -> String {
    format!("wallet_{}.ghst",
        &address.chars().filter(|c| c.is_alphanumeric()).take(20).collect::<String>())
}

#[tokio::main]
async fn main() {
    // ── MODE SERVEUR (Railway/VPS) ───────────────
    // Si la variable d'environnement GHOSTCOIN_SERVER est définie
    // → tourne en mode noeud sans interface CLI
    if env::var("GHOSTCOIN_SERVER").is_ok() {
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

            let w          = wallet::Wallet::new_mainnet();
            wallet_addr    = w.address.clone();
            wallet_path    = get_wallet_path(&wallet_addr);
            password_used  = pwd1.clone();
            wallet_balance = 0u64;

            save_wallet(
                &wallet_addr,
                w.keypair.scan_private.as_bytes(),
                w.keypair.spend_private.as_bytes(),
                0, &password_used, &wallet_path,
            );

            println!("\n╔══════════════════════════════════════════════════════════╗");
            println!("║           ✅ WALLET CRÉÉ AVEC SUCCÈS                    ║");
            println!("╠══════════════════════════════════════════════════════════╣");
            println!("║ Adresse : {:<48} ║", &wallet_addr[..wallet_addr.len().min(48)]);
            println!("║ Fichier : {:<48} ║", &wallet_path[..wallet_path.len().min(48)]);
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
                _ => read_input("Nom du fichier (.ghst) : ")
            };

            if !wallet_exists(&path_to_load) {
                println!("❌ Wallet non trouvé : {}", path_to_load);
                return;
            }

            let pwd = read_input("🔑 Clé privée : ");

            match load_wallet(&path_to_load, &pwd) {
                Some(w) => {
                    wallet_addr    = w.address.clone();
                    wallet_path    = path_to_load;
                    password_used  = pwd;
                    wallet_balance = w.balance;
                    println!("✅ Wallet chargé !");
                    println!("   Adresse : {}...", &wallet_addr[..20.min(wallet_addr.len())]);
                    println!("   Solde   : {} GHST", wallet_balance);
                }
                None => {
                    println!("❌ Clé privée incorrecte");
                    return;
                }
            }
        }

        _ => { println!("❌ Choix invalide"); return; }
    }

    // ── NOEUDS P2P ───────────────────────────────
    println!("\n🌐 Démarrage du réseau...");

    let node1 = NodeState::new(8001);
    let node2 = NodeState::new(8002);
    let node3 = NodeState::new(8003);

    node1.add_peer("127.0.0.1:8002");
    node1.add_peer("127.0.0.1:8003");
    node2.add_peer("127.0.0.1:8001");
    node2.add_peer("127.0.0.1:8003");
    node3.add_peer("127.0.0.1:8001");
    node3.add_peer("127.0.0.1:8002");

    let n1 = node1.clone();
    let n2 = node2.clone();
    let n3 = node3.clone();

    tokio::spawn(async move { run_node(n1).await });
    tokio::spawn(async move { run_node(n2).await });
    tokio::spawn(async move { run_node(n3).await });

    sleep(Duration::from_millis(200)).await;

    send_to_node("127.0.0.1:8002",
        &NodeMessage::Hello { from_port: 8001 }).await;
    send_to_node("127.0.0.1:8003",
        &NodeMessage::Hello { from_port: 8001 }).await;

    sleep(Duration::from_millis(100)).await;

    println!("\n📊 Réseau {} :", config.name);
    for port in [8001u16, 8002, 8003] {
        let addr = format!("127.0.0.1:{}", port);
        if let Some(NodeMessage::Status { port, peers, mempool, blocks }) =
            send_to_node(&addr, &NodeMessage::GetStatus).await
        {
            println!("   🟢 Noeud {} | Pairs: {} | Mempool: {} | Blocs: {}",
                port, peers, mempool, blocks);
        }
    }

    let shared_chain = SharedChain::new();
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