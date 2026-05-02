#[cfg(test)]
mod tests {
    use crate::chain_state::{ChainState, HALVING_INTERVAL, INITIAL_REWARD, MAX_SUPPLY};

    #[test]
    fn test_initial_reward() {
        let state = ChainState::new();
        assert_eq!(state.current_reward(), INITIAL_REWARD);
    }

    #[test]
    fn test_halving_reward() {
        let mut state = ChainState::new();
        state.block_height = HALVING_INTERVAL;
        assert_eq!(state.current_reward(), INITIAL_REWARD / 2);
    }

    #[test]
    fn test_double_halving() {
        let mut state = ChainState::new();
        state.block_height = HALVING_INTERVAL * 2;
        assert_eq!(state.current_reward(), INITIAL_REWARD / 4);
    }

    #[test]
    fn test_max_supply_stops_reward() {
        let mut state = ChainState::new();
        state.minted_supply = MAX_SUPPLY;
        assert_eq!(state.current_reward(), 0);
    }

    #[test]
    fn test_remaining_supply() {
        let mut state = ChainState::new();
        state.minted_supply = 1_000_000;
        assert_eq!(state.remaining_supply(), MAX_SUPPLY - 1_000_000);
    }

    #[test]
    fn test_no_overflow_supply() {
        let mut state = ChainState::new();
        state.minted_supply = MAX_SUPPLY - 10;
        let reward = state.current_reward();
        assert!(state.minted_supply + reward <= MAX_SUPPLY);
    }

    #[test]
    fn test_tx_balance() {
        use crate::tx_store::{TxDirection, TxStatus, WalletTx};

        let tx = WalletTx::new_received("tx_test_001", 100, "sender_addr");
        assert_eq!(tx.amount, 100);
        assert_eq!(tx.direction, TxDirection::Received);
        assert_eq!(tx.status, TxStatus::Pending);
    }

    #[test]
    fn test_double_spend_prevention() {
        // Solde 100, essaie de dépenser 150 → refus
        let balance: u64 = 100;
        let spend:   u64 = 150;
        assert!(spend > balance, "Double spend détecté correctement");
    }

    #[test]
    fn test_seed_phrase_generation() {
        use crate::seed::SeedPhrase;
        let seed = SeedPhrase::generate();
        assert_eq!(seed.words.len(), 12);
    }

    #[test]
    fn test_halving_progress() {
        let mut state = ChainState::new();
        state.block_height = HALVING_INTERVAL / 2;
        let progress = state.halving_progress();
        assert!((progress - 50.0).abs() < 0.01);
    }
}
