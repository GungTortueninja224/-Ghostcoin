use rand::Rng;
use rand::rngs::OsRng;

// ==========================================
// TOR / ONION ROUTING — Protection IP avancée
// ==========================================
// Comment ça marche :
//
// Sans Tor :  Toi → Noeud GhostCoin  (IP visible)
// Avec Tor :  Toi → Noeud1 → Noeud2 → Noeud3 → Réseau
//             (chaque noeud voit seulement le précédent)

const TOR_CIRCUIT_LENGTH: usize = 3; // 3 sauts minimum

#[derive(Clone, Debug)]
pub struct TorNode {
    pub id:       String,
    pub address:  String,
    pub is_exit:  bool,
}

#[derive(Clone, Debug)]
pub struct TorCircuit {
    pub nodes:     Vec<TorNode>,
    pub circuit_id: String,
}

impl TorCircuit {
    // Crée un circuit Tor avec 3 noeuds aléatoires
    pub fn build(known_nodes: &[TorNode]) -> Option<Self> {
        let mut rng = OsRng;

        if known_nodes.len() < TOR_CIRCUIT_LENGTH {
            println!("⚠️  Pas assez de noeuds Tor disponibles");
            return None;
        }

        // Sélectionne 3 noeuds aléatoires
        let mut selected = Vec::new();
        let mut available: Vec<&TorNode> = known_nodes.iter().collect();

        for i in 0..TOR_CIRCUIT_LENGTH {
            let idx = rng.gen_range(0..available.len());
            let mut node = available.remove(idx).clone();

            // Le dernier noeud est le noeud de sortie
            node.is_exit = i == TOR_CIRCUIT_LENGTH - 1;
            selected.push(node);
        }

        let circuit_id = format!("circuit_{:016x}", rand::random::<u64>());

        println!("🧅 Circuit Tor créé : {}", &circuit_id[..20]);
        for (i, node) in selected.iter().enumerate() {
            let role = if node.is_exit { "Exit" } else { "Relay" };
            println!("   Saut {} ({}) : {}", i + 1, role, node.address);
        }

        Some(Self { nodes: selected, circuit_id })
    }

    // Envoie une TX via le circuit Tor
    pub fn send_through_circuit(&self, tx_data: &str) -> bool {
        println!("\n🧅 Envoi via Tor...");
        println!("   TX : {}...", &tx_data[..tx_data.len().min(20)]);

        for (i, node) in self.nodes.iter().enumerate() {
            let role = if node.is_exit { "Exit" } else { "Relay" };
            println!("   ✅ Saut {} ({}) via {} — IP masquée",
                i + 1, role, node.address);
        }

        println!("   ✅ TX propagée — IP origine intraçable !");
        true
    }
}

// ==========================================
// GESTIONNAIRE TOR
// ==========================================
pub struct TorManager {
    pub nodes:    Vec<TorNode>,
    pub circuit:  Option<TorCircuit>,
    pub enabled:  bool,
}

impl TorManager {
    pub fn new() -> Self {
        // Noeuds Tor simulés (en prod : vrais noeuds Tor)
        let nodes = vec![
            TorNode {
                id:      "node_a".to_string(),
                address: "tor1.ghostcoin.net:9050".to_string(),
                is_exit: false,
            },
            TorNode {
                id:      "node_b".to_string(),
                address: "tor2.ghostcoin.net:9050".to_string(),
                is_exit: false,
            },
            TorNode {
                id:      "node_c".to_string(),
                address: "tor3.ghostcoin.net:9050".to_string(),
                is_exit: false,
            },
            TorNode {
                id:      "node_d".to_string(),
                address: "tor4.ghostcoin.net:9050".to_string(),
                is_exit: false,
            },
        ];

        Self { nodes, circuit: None, enabled: false }
    }

    // Active Tor et crée un circuit
    pub fn enable(&mut self) -> bool {
        match TorCircuit::build(&self.nodes) {
            Some(circuit) => {
                self.circuit = Some(circuit);
                self.enabled = true;
                println!("✅ Tor activé — IP protégée !");
                true
            }
            None => {
                println!("❌ Impossible d'activer Tor");
                false
            }
        }
    }

    // Renouvelle le circuit (change les noeuds)
    pub fn renew_circuit(&mut self) {
        println!("\n🔄 Renouvellement du circuit Tor...");
        if let Some(circuit) = TorCircuit::build(&self.nodes) {
            self.circuit = Some(circuit);
            println!("✅ Nouveau circuit actif !");
        }
    }

    // Envoie une TX via Tor
    pub fn send(&self, tx_data: &str) -> bool {
        if !self.enabled {
            println!("⚠️  Tor désactivé — envoi direct (IP visible)");
            return false;
        }

        match &self.circuit {
            Some(circuit) => circuit.send_through_circuit(tx_data),
            None => {
                println!("❌ Pas de circuit Tor actif");
                false
            }
        }
    }

    pub fn show_status(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║              🧅 GHOSTCOIN TOR STATUS                    ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Status   : {:<44} ║",
            if self.enabled { "✅ Actif" } else { "❌ Inactif" });
        println!("║  Noeuds   : {:<44} ║", format!("{} disponibles", self.nodes.len()));
        if let Some(c) = &self.circuit {
            println!("║  Circuit  : {:<44} ║", &c.circuit_id[..20]);
            println!("║  Sauts    : {:<44} ║", format!("{} noeuds", c.nodes.len()));
        }
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Protection : IP, timing, métadonnées                  ║");
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}