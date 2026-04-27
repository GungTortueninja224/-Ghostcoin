use crate::portfolio::{Portfolio, SwapPrices};

pub const SWAP_FEE_PCT: f64 = 0.003; // 0.3% de frais comme Uniswap

pub struct SwapEngine {
    pub prices:    SwapPrices,
    pub portfolio: Portfolio,
}

impl SwapEngine {
    pub fn new(address: &str) -> Self {
        Self {
            prices:    SwapPrices::default(),
            portfolio: Portfolio::load(address),
        }
    }

    pub fn sync_ghst(&mut self, ghst_balance: u64) {
        // Synchronise le solde GHST depuis le wallet
        if let Some(b) = self.portfolio.balances.iter_mut()
            .find(|b| b.symbol == "GHST")
        {
            b.amount = ghst_balance as f64;
        }
        self.portfolio.save();
    }

    pub fn execute_swap(
        &mut self,
        from:   &str,
        to:     &str,
        amount: f64,
    ) -> Option<SwapResult> {
        // Vérifie solde
        let balance = self.portfolio.get_balance(from);
        if amount > balance {
            println!("❌ Solde insuffisant");
            println!("   {} disponible : {:.6}", from, balance);
            return None;
        }

        // Calcule conversion
        let usd_value    = self.prices.to_usd(amount, from);
        let fee_usd      = usd_value * SWAP_FEE_PCT;
        let net_usd      = usd_value - fee_usd;
        let received     = self.prices.from_usd(net_usd, to);

        if received <= 0.0 {
            println!("❌ Montant trop faible");
            return None;
        }

        // Exécute
        self.portfolio.subtract_balance(from, amount);
        self.portfolio.add_balance(to, received);

        Some(SwapResult {
            from:        from.to_string(),
            to:          to.to_string(),
            amount_in:   amount,
            amount_out:  received,
            fee_usd,
            usd_value,
            tx_id:       format!("swap_{:016x}", rand::random::<u64>()),
        })
    }

    pub fn get_quote(&self, from: &str, to: &str, amount: f64) -> SwapQuote {
        let usd_value = self.prices.to_usd(amount, from);
        let fee_usd   = usd_value * SWAP_FEE_PCT;
        let net_usd   = usd_value - fee_usd;
        let received  = self.prices.from_usd(net_usd, to);
        let rate      = if amount > 0.0 { received / amount } else { 0.0 };

        SwapQuote {
            from:       from.to_string(),
            to:         to.to_string(),
            amount_in:  amount,
            amount_out: received,
            rate,
            fee_usd,
            usd_value,
        }
    }
}

pub struct SwapQuote {
    pub from:       String,
    pub to:         String,
    pub amount_in:  f64,
    pub amount_out: f64,
    pub rate:       f64,
    pub fee_usd:    f64,
    pub usd_value:  f64,
}

impl SwapQuote {
    pub fn show(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                  📊 DEVIS SWAP                          ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Tu donnes  : {:<43} ║",
            format!("{:.6} {}", self.amount_in, self.from));
        println!("║  Tu reçois  : {:<43} ║",
            format!("{:.6} {}", self.amount_out, self.to));
        println!("║  Valeur USD : {:<43} ║",
            format!("${:.4}", self.usd_value));
        println!("║  Frais      : {:<43} ║",
            format!("${:.4} (0.3%)", self.fee_usd));
        println!("║  Taux       : {:<43} ║",
            format!("1 {} = {:.6} {}", self.from, self.rate, self.to));
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}

pub struct SwapResult {
    pub from:       String,
    pub to:         String,
    pub amount_in:  f64,
    pub amount_out: f64,
    pub fee_usd:    f64,
    pub usd_value:  f64,
    pub tx_id:      String,
}

impl SwapResult {
    pub fn show(&self) {
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                  ✅ SWAP RÉUSSI                         ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Échangé    : {:<43} ║",
            format!("{:.6} {}", self.amount_in, self.from));
        println!("║  Reçu       : {:<43} ║",
            format!("{:.6} {}", self.amount_out, self.to));
        println!("║  Valeur USD : {:<43} ║",
            format!("${:.4}", self.usd_value));
        println!("║  Frais      : {:<43} ║",
            format!("${:.4}", self.fee_usd));
        println!("║  TX ID      : {:<43} ║",
            &self.tx_id[..self.tx_id.len().min(43)]);
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  ✅ Solde mis à jour dans ton portfolio                 ║");
        println!("╚══════════════════════════════════════════════════════════╝");
    }
}