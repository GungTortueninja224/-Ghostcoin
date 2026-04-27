use curve25519_dalek::scalar::Scalar;
use curve25519_dalek::ristretto::RistrettoPoint;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ViewKey {
    pub scan_public:  String,
    pub scan_private: String,
    pub owner:        String,
    pub expires:      Option<u64>,
    pub label:        String,
}

impl ViewKey {
    pub fn generate(
        scan_private:  &Scalar,
        scan_public:   &RistrettoPoint,
        owner_address: &str,
        label:         &str,
        expires:       Option<u64>,
    ) -> Self {
        Self {
            scan_private: hex::encode(scan_private.as_bytes()),
            scan_public:  hex::encode(scan_public.compress().as_bytes()),
            owner:        owner_address.to_string(),
            expires,
            label:        label.to_string(),
        }
    }

    pub fn export(&self) -> String {
        let json = serde_json::to_string(self).unwrap();
        format!("GHST-VK-{}", base64_encode(json.as_bytes()))
    }

    pub fn import(vk_string: &str) -> Option<Self> {
        if !vk_string.starts_with("GHST-VK-") {
            println!("❌ Format invalide");
            return None;
        }
        let b64  = &vk_string["GHST-VK-".len()..];
        let json = base64_decode(b64)?;
        let json = String::from_utf8(json).ok()?;
        serde_json::from_str::<ViewKey>(&json).ok()
    }

    pub fn is_valid(&self) -> bool {
        match self.expires {
            None      => true,
            Some(exp) => (chrono::Utc::now().timestamp() as u64) < exp,
        }
    }

    pub fn show(&self) {
        println!("\n╔══════════════════════════════════════════════════╗");
        println!("║            🔑 GHOSTCOIN VIEW KEY                ║");
        println!("╠══════════════════════════════════════════════════╣");
        println!("║ Label   : {:<38} ║", self.label);
        println!("║ Owner   : {}... ║", &self.owner[..20.min(self.owner.len())]);
        println!("║ Valide  : {:<38} ║", if self.is_valid() { "✅ Oui" } else { "❌ Expirée" });
        println!("║ Expire  : {:<38} ║", self.expires.map_or("Jamais".to_string(), |e| format!("ts {}", e)));
        println!("╚══════════════════════════════════════════════════╝");
    }
}

pub struct Auditor {
    pub view_key: ViewKey,
}

impl Auditor {
    pub fn new(vk_string: &str) -> Option<Self> {
        ViewKey::import(vk_string).map(|vk| Self { view_key: vk })
    }

    pub fn audit_report(&self, tx_count: usize, total_received: u64) {
        println!("\n╔══════════════════════════════════════════════════╗");
        println!("║           📋 RAPPORT D'AUDIT GHOSTCOIN          ║");
        println!("╠══════════════════════════════════════════════════╣");
        println!("║ Wallet  : {}... ║", &self.view_key.owner[..20.min(self.view_key.owner.len())]);
        println!("║ Label   : {:<32} ║", self.view_key.label);
        println!("║ TX      : {:<32} ║", tx_count);
        println!("║ Total   : {:<32} ║", format!("{} GHST", total_received));
        println!("║ Status  : {:<32} ║", if self.view_key.is_valid() { "✅ Valide" } else { "❌ Expirée" });
        println!("╠══════════════════════════════════════════════════╣");
        println!("║ ⚠️  Lecture seule — impossible de dépenser      ║");
        println!("╚══════════════════════════════════════════════════╝");
    }
}

fn base64_encode(data: &[u8]) -> String {
    const C: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut r = String::new();
    let mut i = 0;
    while i < data.len() {
        let b0 = data[i] as u32;
        let b1 = if i+1 < data.len() { data[i+1] as u32 } else { 0 };
        let b2 = if i+2 < data.len() { data[i+2] as u32 } else { 0 };
        r.push(C[((b0>>2)&0x3F) as usize] as char);
        r.push(C[(((b0<<4)|(b1>>4))&0x3F) as usize] as char);
        r.push(if i+1<data.len() { C[(((b1<<2)|(b2>>6))&0x3F) as usize] as char } else { '=' });
        r.push(if i+2<data.len() { C[(b2&0x3F) as usize] as char } else { '=' });
        i += 3;
    }
    r
}

fn base64_decode(data: &str) -> Option<Vec<u8>> {
    let data: Vec<u8> = data.bytes().filter(|&b| b != b'=').map(|b| match b {
        b'A'..=b'Z' => b-b'A', b'a'..=b'z' => b-b'a'+26,
        b'0'..=b'9' => b-b'0'+52, b'+' => 62, b'/' => 63, _ => 255,
    }).collect();
    let mut r = Vec::new();
    let mut i = 0;
    while i+1 < data.len() {
        r.push((data[i]<<2)|(data[i+1]>>4));
        if i+2 < data.len() { r.push((data[i+1]<<4)|(data[i+2]>>2)); }
        if i+3 < data.len() { r.push((data[i+2]<<6)|data[i+3]); }
        i += 4;
    }
    Some(r)
}