use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const PORTFOLIO_FILE: &str = "ghostcoin_portfolio.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CryptoBalance {
    pub symbol: String,
    pub name: String,
    pub amount: f64,
    pub icon: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Portfolio {
    pub address: String,
    pub balances: Vec<CryptoBalance>,
}

impl Portfolio {
    pub fn load(address: &str) -> Self {
        let path = format!(
            "portfolio_{}.json",
            address
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(16)
                .collect::<String>()
        );

        if !Path::new(&path).exists() {
            return Self::new(address);
        }

        let json = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&json).unwrap_or_else(|_| Self::new(address))
    }

    pub fn new(address: &str) -> Self {
        Self {
            address: address.to_string(),
            balances: vec![
                CryptoBalance {
                    symbol: "GHST".to_string(),
                    name: "GhostCoin".to_string(),
                    amount: 0.0,
                    icon: "👻".to_string(),
                },
                CryptoBalance {
                    symbol: "BTC".to_string(),
                    name: "Bitcoin".to_string(),
                    amount: 0.0,
                    icon: "₿".to_string(),
                },
                CryptoBalance {
                    symbol: "ETH".to_string(),
                    name: "Ethereum".to_string(),
                    amount: 0.0,
                    icon: "Ξ".to_string(),
                },
                CryptoBalance {
                    symbol: "USDT".to_string(),
                    name: "Tether".to_string(),
                    amount: 0.0,
                    icon: "💵".to_string(),
                },
                CryptoBalance {
                    symbol: "BNB".to_string(),
                    name: "BNB".to_string(),
                    amount: 0.0,
                    icon: "🔶".to_string(),
                },
            ],
        }
    }

    pub fn save(&self) {
        let path = format!(
            "portfolio_{}.json",
            self.address
                .chars()
                .filter(|c| c.is_alphanumeric())
                .take(16)
                .collect::<String>()
        );
        let json = serde_json::to_string_pretty(self).unwrap();
        let _ = fs::write(path, json);
    }

    pub fn get_balance(&self, symbol: &str) -> f64 {
        self.balances
            .iter()
            .find(|b| b.symbol == symbol)
            .map(|b| b.amount)
            .unwrap_or(0.0)
    }

    pub fn add_balance(&mut self, symbol: &str, amount: f64) {
        if let Some(b) = self.balances.iter_mut().find(|b| b.symbol == symbol) {
            b.amount += amount;
        }
        self.save();
    }

    pub fn subtract_balance(&mut self, symbol: &str, amount: f64) -> bool {
        if let Some(b) = self.balances.iter_mut().find(|b| b.symbol == symbol) {
            if b.amount >= amount {
                b.amount -= amount;
                self.save();
                return true;
            }
        }
        false
    }

    pub fn total_value_usd(&self, prices: &SwapPrices) -> f64 {
        self.balances
            .iter()
            .map(|b| {
                let price = match b.symbol.as_str() {
                    "GHST" => prices.ghst_usd,
                    "BTC" => prices.btc_usd,
                    "ETH" => prices.eth_usd,
                    "USDT" => 1.0,
                    "BNB" => prices.bnb_usd,
                    _ => 0.0,
                };
                b.amount * price
            })
            .sum()
    }

    pub fn show(&self, prices: &SwapPrices) {
        let total_usd = self.total_value_usd(prices);

        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║              💼 MON PORTFOLIO CRYPTO                    ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║  Valeur totale : {:<40} ║",
            format!("${:.2} USD", total_usd)
        );
        println!("╠══════════════════════════════════════════════════════════╣");
        println!(
            "║  {:<4} {:<10} {:<18} {:<18} ║",
            "Icon", "Crypto", "Solde", "Valeur USD"
        );
        println!("╠══════════════════════════════════════════════════════════╣");

        for b in &self.balances {
            let price = match b.symbol.as_str() {
                "GHST" => prices.ghst_usd,
                "BTC" => prices.btc_usd,
                "ETH" => prices.eth_usd,
                "USDT" => 1.0,
                "BNB" => prices.bnb_usd,
                _ => 0.0,
            };
            let usd_val = b.amount * price;

            let amount_str = if b.symbol == "GHST" {
                format!("{:.0}", b.amount)
            } else {
                format!("{:.6}", b.amount)
            };

            println!(
                "║  {} {:<8} {:<18} {:<18} ║",
                b.icon,
                b.symbol,
                amount_str,
                format!("${:.2}", usd_val),
            );
        }

        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Prix marché :                                           ║");
        println!("║  👻 GHST  = ${:<46} ║", format!("{:.4}", prices.ghst_usd));
        println!("║  ₿  BTC   = ${:<46} ║", format!("{:.0}", prices.btc_usd));
        println!("║  Ξ  ETH   = ${:<46} ║", format!("{:.0}", prices.eth_usd));
        println!("║  🔶 BNB   = ${:<46} ║", format!("{:.0}", prices.bnb_usd));
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}

// ==========================================
// PRIX DU MARCHÉ
// ==========================================
#[derive(Clone, Debug)]
pub struct SwapPrices {
    pub ghst_usd: f64,
    pub btc_usd: f64,
    pub eth_usd: f64,
    pub bnb_usd: f64,
}

impl SwapPrices {
    // Prix par défaut (mis à jour manuellement ou via API future)
    pub fn default() -> Self {
        Self {
            ghst_usd: 0.01, // Prix initial GHST fixé par toi
            btc_usd: 94_000.0,
            eth_usd: 3_200.0,
            bnb_usd: 600.0,
        }
    }

    // Calcule combien de crypto B on obtient pour X crypto A
    pub fn convert(&self, amount: f64, from: &str, to: &str) -> f64 {
        let from_usd = self.to_usd(amount, from);
        self.convert_from_usd(from_usd, to)
    }

    pub fn to_usd(&self, amount: f64, symbol: &str) -> f64 {
        match symbol {
            "GHST" => amount * self.ghst_usd,
            "BTC" => amount * self.btc_usd,
            "ETH" => amount * self.eth_usd,
            "USDT" => amount,
            "BNB" => amount * self.bnb_usd,
            _ => 0.0,
        }
    }

    pub fn convert_from_usd(&self, usd: f64, symbol: &str) -> f64 {
        match symbol {
            "GHST" => usd / self.ghst_usd,
            "BTC" => usd / self.btc_usd,
            "ETH" => usd / self.eth_usd,
            "USDT" => usd,
            "BNB" => usd / self.bnb_usd,
            _ => 0.0,
        }
    }
}
