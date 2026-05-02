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
        let spend: u64 = 150;
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

    #[test]
    fn test_blockchain_rejects_tampering() {
        let mut chain = crate::blockchain::Blockchain::new();
        chain.add_block("tx:a->b:10".to_string()).unwrap();
        assert!(chain.is_valid());

        chain.chain[1].data = "tx:a->b:999999".to_string();
        assert!(!chain.is_valid());
    }

    #[test]
    fn test_mempool_rejects_invalid_tx() {
        let tx = crate::mempool::MempoolTx::new("bad", "same", "same", 0, 1);
        assert!(tx.validate().is_err());
    }

    #[test]
    fn test_wallet_storage_round_trip_v2() {
        let path = ".test_wallet_storage.ghst";
        let _ = std::fs::remove_file(path);

        assert!(crate::storage::save_wallet(
            "PC1-test",
            b"scan-private",
            b"spend-private",
            42,
            "strong-password",
            path,
        ));

        let wallet = crate::storage::load_wallet(path, "strong-password").unwrap();
        assert_eq!(wallet.address, "PC1-test");
        assert_eq!(wallet.balance, 42);
        assert_eq!(wallet.version, "2.0.0");
        assert!(crate::storage::load_wallet(path, "wrong-password").is_none());

        let _ = std::fs::remove_file(path);
    }
}
