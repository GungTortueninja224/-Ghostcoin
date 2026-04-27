use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};

// ==========================================
// MESSAGES QUI CIRCULENT SUR LE RÉSEAU
// ==========================================
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Message {
    // Un noeud annonce qu'il existe
    Hello { address: String },

    // Un noeud partage sa blockchain
    ShareChain { blocks: Vec<BlockData> },

    // Un noeud annonce une nouvelle transaction
    NewTransaction { tx_data: String },

    // Un noeud demande la blockchain
    RequestChain,

    // Confirmation reçue
    Ok,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BlockData {
    pub index:         u32,
    pub timestamp:     u128,
    pub data:          String,
    pub previous_hash: String,
    pub hash:          String,
}

// ==========================================
// NOEUD DU RÉSEAU
// ==========================================
pub struct Node {
    pub address:  String,
    pub peers:    Arc<Mutex<Vec<String>>>,
    pub mempool:  Arc<Mutex<Vec<String>>>,
}

impl Node {
    pub fn new(address: &str) -> Self {
        Self {
            address:  address.to_string(),
            peers:    Arc::new(Mutex::new(vec![])),
            mempool:  Arc::new(Mutex::new(vec![])),
        }
    }

    // Ajoute un pair connu
    pub fn add_peer(&self, peer: &str) {
        let mut peers = self.peers.lock().unwrap();
        if !peers.contains(&peer.to_string()) {
            peers.push(peer.to_string());
            println!("🔗 Pair ajouté : {}", peer);
        }
    }

    // Démarre le serveur — écoute les connexions entrantes
    pub async fn start_server(&self) {
        let address  = self.address.clone();
        let peers    = Arc::clone(&self.peers);
        let mempool  = Arc::clone(&self.mempool);

        println!("🌐 Noeud démarré sur {}", address);

        let listener = TcpListener::bind(&address).await
            .expect("Impossible de démarrer le serveur");

        loop {
            let (socket, addr) = listener.accept().await.unwrap();
            println!("📥 Connexion entrante de : {}", addr);

            let peers_clone   = Arc::clone(&peers);
            let mempool_clone = Arc::clone(&mempool);

            // Chaque connexion dans son propre thread
            tokio::spawn(async move {
                handle_connection(socket, peers_clone, mempool_clone).await;
            });
        }
    }

    // Envoie un message à un pair
    pub async fn send_message(&self, peer: &str, message: &Message) -> bool {
        match TcpStream::connect(peer).await {
            Ok(mut stream) => {
                let json = serde_json::to_string(message).unwrap();
                let _ = stream.write_all(json.as_bytes()).await;
                println!("📤 Message envoyé à {} : {:?}", peer, message);
                true
            }
            Err(_) => {
                println!("❌ Impossible de joindre {}", peer);
                false
            }
        }
    }

    // Broadcast : envoie à tous les pairs
    pub async fn broadcast(&self, message: &Message) {
        let peers = self.peers.lock().unwrap().clone();
        for peer in peers {
            self.send_message(&peer, message).await;
        }
    }

    // Annonce une nouvelle transaction au réseau
    pub async fn announce_transaction(&self, tx_data: &str) {
        // Ajoute au mempool local
        {
            let mut mempool = self.mempool.lock().unwrap();
            mempool.push(tx_data.to_string());
            println!("📝 TX ajoutée au mempool local");
        }

        // Broadcast aux pairs
        let msg = Message::NewTransaction {
            tx_data: tx_data.to_string()
        };
        self.broadcast(&msg).await;
    }
}

// ==========================================
// GESTION D'UNE CONNEXION ENTRANTE
// ==========================================
async fn handle_connection(
    mut socket: TcpStream,
    peers:      Arc<Mutex<Vec<String>>>,
    mempool:    Arc<Mutex<Vec<String>>>,
) {
    let mut buf = vec![0u8; 4096];

    match socket.read(&mut buf).await {
        Ok(n) if n > 0 => {
            let raw = String::from_utf8_lossy(&buf[..n]);

            match serde_json::from_str::<Message>(&raw) {
                Ok(msg) => handle_message(msg, peers, mempool).await,
                Err(e)  => println!("❌ Message invalide : {}", e),
            }
        }
        _ => println!("⚠️  Connexion fermée"),
    }
}

// ==========================================
// TRAITEMENT DES MESSAGES
// ==========================================
async fn handle_message(
    message: Message,
    peers:   Arc<Mutex<Vec<String>>>,
    mempool: Arc<Mutex<Vec<String>>>,
) {
    match message {
        Message::Hello { address } => {
            println!("👋 Hello reçu de : {}", address);
            let mut peers = peers.lock().unwrap();
            if !peers.contains(&address) {
                peers.push(address.clone());
                println!("🔗 Nouveau pair : {}", address);
            }
        }

        Message::NewTransaction { tx_data } => {
            println!("💸 Nouvelle TX reçue : {}", tx_data);
            let mut mempool = mempool.lock().unwrap();
            if !mempool.contains(&tx_data) {
                mempool.push(tx_data);
                println!("📝 TX ajoutée au mempool");
            }
        }

        Message::RequestChain => {
            println!("📋 Demande de blockchain reçue");
            // En production : envoyer la vraie blockchain ici
        }

        Message::ShareChain { blocks } => {
            println!("📦 Blockchain reçue : {} blocs", blocks.len());
        }

        Message::Ok => {
            println!("✅ OK reçu");
        }
    }
}
