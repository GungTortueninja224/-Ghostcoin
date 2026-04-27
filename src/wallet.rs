use curve25519_dalek::ristretto::RistrettoPoint;
use sha2::{Sha256, Digest};

use crate::stealth::RecipientKeypair;

// ==========================================
// VERSIONS DU RÉSEAU
// ==========================================
pub enum Network {
    Mainnet,
    Testnet,
}

impl Network {
    fn prefix(&self) -> &str {
        match self {
            Network::Mainnet => "PC1",
            Network::Testnet => "PCT",
        }
    }
}

// ==========================================
// GÉNÉRATION D'ADRESSE AMÉLIORÉE
// ==========================================
fn derive_address(spend_public: &RistrettoPoint, network: &Network) -> String {
    let bytes = spend_public.compress();
    let bytes = bytes.as_bytes();

    // 1. Double SHA256 (comme Bitcoin)
    let round1 = {
        let mut h = Sha256::new();
        h.update(bytes);
        h.finalize()
    };

    let round2 = {
        let mut h = Sha256::new();
        h.update(&round1);
        h.finalize()
    };

    // 2. Payload = 32 bytes complets
    let payload = hex::encode(&round2[..32]);

    // 3. Checksum = 4 premiers bytes du hash du payload
    let checksum = {
        let mut h = Sha256::new();
        h.update(payload.as_bytes());
        let result = h.finalize();
        hex::encode(&result[..2]).to_uppercase()
    };

    // 4. Format final : PREFIX-PAYLOAD-CHECKSUM
    format!("{}-{}-{}", network.prefix(), payload, checksum)
}

// ==========================================
// VALIDATION D'ADRESSE
// ==========================================
pub fn validate_address(address: &str) -> bool {
    let parts: Vec<&str> = address.split('-').collect();

    // Doit avoir 3 parties : prefix, payload, checksum
    if parts.len() != 3 {
        return false;
    }

    let prefix   = parts[0];
    let payload  = parts[1];
    let checksum = parts[2];

    // Vérifie le prefix
    if prefix != "PC1" && prefix != "PCT" {
        return false;
    }

    // Vérifie la longueur du payload (32 bytes = 64 chars hex)
    if payload.len() != 64 {
        return false;
    }

    // Recalcule le checksum
    let expected = {
        let mut h = Sha256::new();
        h.update(payload.as_bytes());
        let result = h.finalize();
        hex::encode(&result[..2]).to_uppercase()
    };

    // Compare les checksums
    checksum == expected
}

// ==========================================
// WALLET COMPLET
// ==========================================
pub struct Wallet {
    pub address: String,
    pub keypair: RecipientKeypair,
    pub balance: u64,
    pub network: String,
}

impl Wallet {
    pub fn new_mainnet() -> Self {
        let keypair = RecipientKeypair::generate();
        let address = derive_address(&keypair.spend_public, &Network::Mainnet);
        Self {
            address,
            keypair,
            balance: 0,
            network: "Mainnet".to_string(),
        }
    }

    pub fn new_testnet() -> Self {
        let keypair = RecipientKeypair::generate();
        let address = derive_address(&keypair.spend_public, &Network::Testnet);
        Self {
            address,
            keypair,
            balance: 0,
            network: "Testnet".to_string(),
        }
    }

    pub fn show(&self) {
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║               🔒 PRIVACY CHAIN WALLET                   ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Réseau  : {:<48} ║", self.network);
        println!("║ Adresse : {:<48} ║", &self.address[..self.address.len().min(48)]);
        println!("║          {:<48} ║", &self.address[self.address.len().min(48)..]);
        println!("║ Balance : {:<48} ║", format!("{} coins", self.balance));
        println!("╚══════════════════════════════════════════════════════════╝");
    }

    pub fn show_address_details(&self) {
        let parts: Vec<&str> = self.address.split('-').collect();
        println!("\n📬 Détails de l'adresse :");
        println!("   Adresse complète : {}", self.address);
        println!("   ├── Prefix       : {} ({})", parts[0], self.network);
        println!("   ├── Payload      : {}...{}", &parts[1][..8], &parts[1][56..]);
        println!("   └── Checksum     : {} ✅", parts[2]);
        println!("\n   Valide : {}", validate_address(&self.address));
    }
}
