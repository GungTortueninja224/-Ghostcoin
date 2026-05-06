use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::ristretto::RistrettoPoint;
use curve25519_dalek::scalar::Scalar;
use sha2::{Digest, Sha256};

use crate::seed::SeedPhrase;
use crate::stealth::RecipientKeypair;
use crate::storage::WalletFile;

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
        h.update(round1);
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

fn network_from_address(address: &str) -> Network {
    if address.starts_with("PCT-") {
        Network::Testnet
    } else {
        Network::Mainnet
    }
}

fn scalar_from_seed_phrase(seed_phrase: &str, domain: &[u8]) -> Scalar {
    let mut h = Sha256::new();
    h.update(seed_phrase.as_bytes());
    h.update(domain);
    let result = h.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    Scalar::from_bytes_mod_order(bytes)
}

fn derive_keypair_from_seed_phrase(seed_phrase: &str) -> RecipientKeypair {
    let scan_private = scalar_from_seed_phrase(seed_phrase, b"ghostcoin-scan");
    let spend_private = scalar_from_seed_phrase(seed_phrase, b"ghostcoin-spend");
    RecipientKeypair {
        scan_public: scan_private * RISTRETTO_BASEPOINT_POINT,
        spend_public: spend_private * RISTRETTO_BASEPOINT_POINT,
        scan_private,
        spend_private,
    }
}

fn scalar_from_hex(hex_str: &str) -> Option<Scalar> {
    let bytes = hex::decode(hex_str).ok()?;
    let bytes: [u8; 32] = bytes.try_into().ok()?;
    Some(Scalar::from_bytes_mod_order(bytes))
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

    let prefix = parts[0];
    let payload = parts[1];
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
    pub seed_phrase: Option<String>,
}

impl Wallet {
    fn new_with_network(network: Network) -> Self {
        let seed = SeedPhrase::generate();
        let seed_phrase = seed.words.join(" ");
        let keypair = derive_keypair_from_seed_phrase(&seed_phrase);
        let address = derive_address(&keypair.spend_public, &network);
        let network_name = match network {
            Network::Mainnet => "Mainnet",
            Network::Testnet => "Testnet",
        };
        Self {
            address,
            keypair,
            balance: 0,
            network: network_name.to_string(),
            seed_phrase: Some(seed_phrase),
        }
    }

    pub fn new_mainnet() -> Self {
        Self::new_with_network(Network::Mainnet)
    }

    pub fn new_testnet() -> Self {
        Self::new_with_network(Network::Testnet)
    }

    pub fn from_wallet_file(wallet_file: &WalletFile) -> Option<Self> {
        let scan_private = scalar_from_hex(&wallet_file.scan_private)?;
        let spend_private = scalar_from_hex(&wallet_file.spend_private)?;
        let keypair = RecipientKeypair {
            scan_public: scan_private * RISTRETTO_BASEPOINT_POINT,
            spend_public: spend_private * RISTRETTO_BASEPOINT_POINT,
            scan_private,
            spend_private,
        };
        let network = network_from_address(&wallet_file.address);
        let derived_address = derive_address(&keypair.spend_public, &network);
        if derived_address != wallet_file.address {
            return None;
        }
        let network_name = match network {
            Network::Mainnet => "Mainnet",
            Network::Testnet => "Testnet",
        };
        Some(Self {
            address: wallet_file.address.clone(),
            keypair,
            balance: wallet_file.balance,
            network: network_name.to_string(),
            seed_phrase: wallet_file.seed_phrase.clone(),
        })
    }

    pub fn get_seed_phrase(&self) -> Option<&str> {
        self.seed_phrase.as_deref()
    }

    pub fn show(&self) {
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║               🔒 PRIVACY CHAIN WALLET                   ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║ Réseau  : {:<48} ║", self.network);
        println!(
            "║ Adresse : {:<48} ║",
            &self.address[..self.address.len().min(48)]
        );
        println!(
            "║          {:<48} ║",
            &self.address[self.address.len().min(48)..]
        );
        println!("║ Balance : {:<48} ║", format!("{} coins", self.balance));
        println!("╚══════════════════════════════════════════════════════════╝");
    }

    pub fn show_address_details(&self) {
        let parts: Vec<&str> = self.address.split('-').collect();
        println!("\n📬 Détails de l'adresse :");
        println!("   Adresse complète : {}", self.address);
        println!("   ├── Prefix       : {} ({})", parts[0], self.network);
        println!(
            "   ├── Payload      : {}...{}",
            &parts[1][..8],
            &parts[1][56..]
        );
        println!("   └── Checksum     : {} ✅", parts[2]);
        println!("\n   Valide : {}", validate_address(&self.address));
    }
}
