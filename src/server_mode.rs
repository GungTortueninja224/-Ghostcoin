use crate::node::{NodeState, NodeMessage, run_node, send_to_node};
use crate::chain_state::ChainState;
use crate::sync::SharedChain;
use crate::config::GhostCoinConfig;
use tokio::time::{sleep, Duration};

pub async fn run_server_mode() {
    let config = GhostCoinConfig::new();

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║         👻 GHOSTCOIN NODE — SERVER MODE                 ║");
    println!("╠══════════════════════════════════════════════════════════╣");
    println!("║  {} v{}                                     ║", config.name, config.version);
    println!("╚══════════════════════════════════════════════════════════╝");

    // Charge l'état de la blockchain
    let state = ChainState::load();
    println!("📦 Blockchain height : #{}", state.block_height);
    println!("💰 Supply actuel     : {} GHST", state.minted_supply);

    // Démarre les noeuds P2P
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

    sleep(Duration::from_millis(500)).await;

    // Connexions initiales
    send_to_node("127.0.0.1:8002",
        &NodeMessage::Hello { from_port: 8001 }).await;
    send_to_node("127.0.0.1:8003",
        &NodeMessage::Hello { from_port: 8001 }).await;

    println!("\n🟢 Noeuds démarrés :");
    println!("   🟢 Port 8001 — actif");
    println!("   🟢 Port 8002 — actif");
    println!("   🟢 Port 8003 — actif");

    println!("\n✅ GhostCoin Network en ligne !");
    println!("   Le noeud tourne 24h/24...\n");

    // Boucle infinie — garde le noeud actif
    let mut tick = 0u64;
    loop {
        sleep(Duration::from_secs(60)).await;
        tick += 1;

        let state = ChainState::load();
        println!("⏱️  [{} min] Height: #{} | Supply: {} GHST | Peers connectés",
            tick, state.block_height, state.minted_supply);

        // Vérifie les pairs toutes les 5 minutes
        if tick % 5 == 0 {
            for port in [8001u16, 8002, 8003] {
                let addr = format!("127.0.0.1:{}", port);
                match send_to_node(&addr, &NodeMessage::Ping).await {
                    Some(_) => {}
                    None    => println!("⚠️  Noeud {} non répondant", port),
                }
            }
        }
    }
}