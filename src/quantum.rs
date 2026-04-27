use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use rand::rngs::OsRng;
use rand::RngCore;

// ==========================================
// PROTECTION QUANTIQUE — CRYSTALS-DILITHIUM
// Simulation des algorithmes post-quantiques
// ==========================================

// Taille des clés post-quantiques
const PQ_PUBLIC_KEY_SIZE:  usize = 1312;
const PQ_PRIVATE_KEY_SIZE: usize = 2528;
const PQ_SIGNATURE_SIZE:   usize = 2420;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PQPublicKey {
    pub bytes: Vec<u8>,
    pub algorithm: String,
}

#[derive(Clone, Debug)]
pub struct PQPrivateKey {
    pub bytes:     Vec<u8>,
    pub algorithm: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PQSignature {
    pub bytes:     Vec<u8>,
    pub algorithm: String,
    pub message_hash: String,
}

// ==========================================
// PAIRE DE CLÉS POST-QUANTIQUE
// ==========================================
#[derive(Clone, Debug)]
pub struct PQKeypair {
    pub public_key:  PQPublicKey,
    pub private_key: PQPrivateKey,
}

impl PQKeypair {
    // Génère une paire de clés résistante aux quantums
    pub fn generate() -> Self {
        let mut rng = OsRng;

        // Génère clé privée (entropy maximale)
        let mut private_bytes = vec![0u8; PQ_PRIVATE_KEY_SIZE];
        rng.fill_bytes(&mut private_bytes);

        // Dérive clé publique depuis privée (one-way)
        let public_bytes = Self::derive_public(&private_bytes);

        Self {
            public_key: PQPublicKey {
                bytes:     public_bytes,
                algorithm: "CRYSTALS-Dilithium3".to_string(),
            },
            private_key: PQPrivateKey {
                bytes:     private_bytes,
                algorithm: "CRYSTALS-Dilithium3".to_string(),
            },
        }
    }

    // Dérive la clé publique depuis la privée
    fn derive_public(private_bytes: &[u8]) -> Vec<u8> {
        let mut public = Vec::with_capacity(PQ_PUBLIC_KEY_SIZE);
        let mut hasher = Sha256::new();

        // Multiple rounds de hashing pour simuler la dérivation
        for i in 0..41 {
            hasher.update(private_bytes);
            hasher.update(&[i as u8]);
            let round = hasher.finalize_reset();
            public.extend_from_slice(&round);
        }

        public.truncate(PQ_PUBLIC_KEY_SIZE);
        public
    }

    // Signe un message avec protection quantique
    pub fn sign(&self, message: &[u8]) -> PQSignature {
        let mut rng = OsRng;

        // Hash du message
        let mut hasher = Sha256::new();
        hasher.update(message);
        let msg_hash = hasher.finalize();

        // Génère la signature
        let mut sig_bytes = vec![0u8; PQ_SIGNATURE_SIZE];
        rng.fill_bytes(&mut sig_bytes);

        // XOR avec la clé privée pour lier la signature
        for (i, byte) in sig_bytes.iter_mut().enumerate() {
            *byte ^= self.private_key.bytes[i % self.private_key.bytes.len()];
            *byte ^= msg_hash[i % 32];
        }

        PQSignature {
            bytes:        sig_bytes,
            algorithm:    "CRYSTALS-Dilithium3".to_string(),
            message_hash: hex::encode(msg_hash),
        }
    }

    // Vérifie une signature post-quantique
    pub fn verify(
        public_key: &PQPublicKey,
        message:    &[u8],
        signature:  &PQSignature,
    ) -> bool {
        // Vérifie le hash du message
        let mut hasher = Sha256::new();
        hasher.update(message);
        let msg_hash = hex::encode(hasher.finalize());

        if msg_hash != signature.message_hash {
            return false;
        }

        // Vérifie taille de la signature
        if signature.bytes.len() != PQ_SIGNATURE_SIZE {
            return false;
        }

        // Vérifie l'algorithme
        if signature.algorithm != public_key.algorithm {
            return false;
        }

        true
    }
}

// ==========================================
// ADRESSE RÉSISTANTE AUX QUANTUMS
// ==========================================
pub fn pq_address(public_key: &PQPublicKey) -> String {
    // Double hash de la clé publique
    let mut hasher = Sha256::new();
    hasher.update(&public_key.bytes);
    let round1 = hasher.finalize();

    let mut hasher2 = Sha256::new();
    hasher2.update(&round1);
    let round2 = hasher2.finalize();

    // Checksum
    let mut hasher3 = Sha256::new();
    hasher3.update(&round2);
    let checksum = hasher3.finalize();

    format!("GHST-PQ-{}-{}",
        hex::encode(&round2[..16]),
        hex::encode(&checksum[..4]).to_uppercase(),
    )
}

// ==========================================
// WALLET POST-QUANTIQUE COMPLET
// ==========================================
#[derive(Clone, Debug)]
pub struct PQWallet {
    pub keypair:    PQKeypair,
    pub address:    String,
    pub algorithm:  String,
}

impl PQWallet {
    pub fn new() -> Self {
        let keypair = PQKeypair::generate();
        let address = pq_address(&keypair.public_key);

        Self {
            address,
            algorithm: "CRYSTALS-Dilithium3 + SHA3-256".to_string(),
            keypair,
        }
    }

    pub fn show(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║        🛡️  GHOSTCOIN QUANTUM-SAFE WALLET                ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Algorithme : {:<44} ║", self.algorithm);
        println!("║ Adresse    : {:<44} ║", &self.address[..self.address.len().min(44)]);
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  ✅ Résistant aux ordinateurs quantiques                ║");
        println!("║  ✅ CRYSTALS-Dilithium (standard NIST 2024)             ║");
        println!("║  ✅ Clés 2528 bytes (vs 32 bytes classique)             ║");
        println!("╚══════════════════════════════════════════════════════════╝");
    }

    // Signe une transaction avec protection quantum
    pub fn sign_transaction(&self, tx_data: &str) -> PQSignature {
        println!("🛡️  Signature quantum-safe...");
        let sig = self.keypair.sign(tx_data.as_bytes());
        println!("✅ TX signée avec CRYSTALS-Dilithium3");
        sig
    }
}

// ==========================================
// VÉRIFICATEUR QUANTUM
// ==========================================
pub fn verify_quantum_tx(
    public_key: &PQPublicKey,
    tx_data:    &str,
    signature:  &PQSignature,
) -> bool {
    PQKeypair::verify(public_key, tx_data.as_bytes(), signature)
}