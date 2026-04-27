// ==========================================
// SYSTÈME DE FRAIS GHOSTCOIN
// ==========================================

pub struct FeeCalculator;

impl FeeCalculator {
    // Frais minimum par transaction
    pub const MIN_FEE: u64 = 1;

    // Calcule les frais selon priorité
    pub fn calculate(amount: u64, priority: FeePriority) -> u64 {
        let base = match priority {
            FeePriority::Low    => (amount / 1000).max(Self::MIN_FEE),
            FeePriority::Normal => (amount / 500).max(Self::MIN_FEE * 2),
            FeePriority::High   => (amount / 200).max(Self::MIN_FEE * 5),
        };
        base
    }

    // Affiche les frais estimés
    pub fn show_estimate(amount: u64) {
        println!("\n💸 Estimation des frais pour {} GHST :", amount);
        println!("   🐢 Lent   : {} GHST (~10 min)", Self::calculate(amount, FeePriority::Low));
        println!("   🚶 Normal : {} GHST (~2 min)",  Self::calculate(amount, FeePriority::Normal));
        println!("   🚀 Rapide : {} GHST (~30 sec)", Self::calculate(amount, FeePriority::High));
    }
}

#[derive(Debug, Clone)]
pub enum FeePriority {
    Low,
    Normal,
    High,
}

impl FeePriority {
    pub fn from_str(s: &str) -> Self {
        match s {
            "1" => Self::Low,
            "3" => Self::High,
            _   => Self::Normal,
        }
    }
}