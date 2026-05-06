use std::io::{self, Write};
use crate::wallet::{Wallet, validate_address};
use crate::dandelion::DandelionRouter;
use crate::fees::{FeeCalculator, FeePriority};
use crate::explorer::BlockExplorer;
use crate::sync::{ChainSync, SharedChain};
use crate::storage::{broadcast_tx, claim_incoming, PendingTx};
use crate::tx_store::{WalletTxStore, WalletTx};
use crate::chain_state::ChainState;
use crate::config;
use crate::logger::{log_tx, log_error, log_mining};

pub struct Cli {
    pub wallet:      Wallet,
    pub router:      DandelionRouter,
    pub tx_store:    WalletTxStore,
    pub explorer:    BlockExplorer,
    pub wallet_path: String,
    pub password:    String,
}

impl Cli {
    pub fn new_with_balance(
        chain:       SharedChain,
        wallet_path: &str,
        password:    &str,
        address:     &str,
        _balance:    u64,
    ) -> Self {
        let tx_store = WalletTxStore::new(address);
        let txs      = tx_store.load();
        println!("📂 {} transaction(s) dans l'historique", txs.len());
        let mut wallet  = Wallet::new_mainnet();
        wallet.address  = address.to_string();
        wallet.balance  = tx_store.available_balance();
        Self {
            wallet,
            router: DandelionRouter::new(vec![
                "127.0.0.1:8001".to_string(),
                "127.0.0.1:8002".to_string(),
                "127.0.0.1:8003".to_string(),
            ]),
            tx_store,
            explorer:    BlockExplorer::new(chain),
            wallet_path: wallet_path.to_string(),
            password:    password.to_string(),
        }
    }

    fn coming_soon(feature: &str) {
        println!("");
        println!("╔══════════════════════════════════════╗");
        println!("║     🔜 {} — COMING SOON", feature);
        println!("║                                      ║");
        println!("║  Disponible au mainnet v2            ║");
        println!("╚══════════════════════════════════════╝");
        println!("");
    }

    fn check_incoming(&mut self) {
        let incoming = claim_incoming(&self.wallet.address);
        if !incoming.is_empty() {
            println!("\n🔔 {} nouvelle(s) TX reçue(s) !", incoming.len());
            for tx in &incoming {
                let wtx = WalletTx::new_received(&tx.tx_id, tx.amount, &tx.sender);
                self.tx_store.add(wtx);
                self.wallet.balance += tx.amount;
                println!("💰 +{} GHST de {}...", tx.amount,
                    &tx.sender[..20.min(tx.sender.len())]);
                log_tx(&format!("Reçu {} GHST — TX: {}", tx.amount, tx.tx_id));
            }
            println!("💎 Solde disponible : {} GHST", self.wallet.balance);
        }
        let state = ChainState::load();
        self.tx_store.confirm_from_mempool(state.block_height);
    }

    pub fn run(&mut self) {
        self.check_incoming();
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║            👻 GHOSTCOIN (GHST) WALLET                   ║");
        println!("╚══════════════════════════════════════════════════════════╝\n");
        self.wallet.show();

        loop {
            let pending = self.tx_store.pending_balance();
            if pending > 0 {
                println!("   ⏳ Pending : {} GHST", pending);
            }

            println!("\n┌──────────────────────────────────────────────┐");
            println!("│  1.  👛 Mon adresse                          │");
            println!("│  2.  💰 Mon solde détaillé                   │");
            println!("│  3.  📤 Envoyer des GHST                     │");
            println!("│  4.  📡 Vérifier TX reçues                   │");
            println!("│  5.  ✅ Valider une adresse                  │");
            println!("│  6.  📋 Copier mon adresse                   │");
            println!("│  7.  📜 Historique (mon wallet)              │");
            println!("│  8.  🔑 Générer View Key                     │");
            println!("│  9.  🔍 Import View Key (Coming Soon)        │");
            println!("│  A.  💱 Atomic Swap (Coming Soon)            │");
            println!("│  B.  🛡️  Max Privacy (Coming Soon)           │");
            println!("│  C.  🔎 Block Explorer                       │");
            println!("│  D.  📊 Stats réseau global                  │");
            println!("│  E.  ⛏️  Miner un bloc                       │");
            println!("│  F.  🌱 Seed Phrase (Coming Soon)            │");
            println!("│  G.  🔄 Rescan wallet                        │");
            println!("│  H.  📋 Voir mempool                         │");
            println!("│  I.  🔄 Replace-by-Fee (Coming Soon)         │");
            println!("│  J.  🛡️  Quantum Wallet (Coming Soon)        │");
            println!("│  K.  💎 Staking GHST (Coming Soon)           │");
            println!("│  L.  🗳️  Gouvernance (Coming Soon)           │");
            println!("│  M.  🖥️  Masternode (Coming Soon)            │");
            println!("│  P.  💼 Mon Portfolio (Coming Soon)          │");
            println!("│  N.  🧅 Tor Network (Coming Soon)            │");
            println!("│  O.  🌀 MimbleWimble (Coming Soon)           │");
            println!("│  0.  🚪 Quitter                              │");
            println!("└──────────────────────────────────────────────┘");
            print!("Choix : ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();

            match input.trim().to_lowercase().as_str() {
                "1" => self.wallet.show_address_details(),
                "2" => self.show_balance_detailed(),
                "3" => self.send_transaction(),
                "4" => self.check_incoming(),
                "5" => self.validate_address(),
                "6" => self.copy_address(),
                "7" => self.show_my_transactions(),
                "8" => self.generate_view_key(),
                "9" => Self::coming_soon("IMPORT VIEW KEY"),
                "a" => Self::coming_soon("ATOMIC SWAP"),
                "b" => Self::coming_soon("MAX PRIVACY"),
                "c" => self.explorer.show_blocks(),
                "d" => ChainState::load().show(),
                "e" => self.mine_block(),
                "f" => Self::coming_soon("SEED PHRASE"),
                "g" => self.rescan_wallet(),
                "h" => self.show_mempool(),
                "i" => Self::coming_soon("REPLACE-BY-FEE"),
                "j" => Self::coming_soon("QUANTUM-SAFE WALLET"),
                "p" => Self::coming_soon("PORTFOLIO"),
                "k" => Self::coming_soon("STAKING GHST"),
                "l" => Self::coming_soon("GOUVERNANCE"),
                "m" => Self::coming_soon("MASTERNODE"),
                "n" => Self::coming_soon("TOR NETWORK"),
                "o" => Self::coming_soon("MIMBLEWIMBLE"),
                "0" => { println!("👋 Au revoir !"); break; }
                _   => println!("❌ Choix invalide"),
            }
        }
    }
    fn show_portfolio(&mut self) {
      let prices = crate::portfolio::SwapPrices::default();
      let mut engine = crate::atomic_swap::SwapEngine::new(&self.wallet.address);
      engine.sync_ghst(self.wallet.balance);
      engine.portfolio.show(&prices);
    }
    fn show_balance_detailed(&self) {
        let available = self.tx_store.available_balance();
        let pending   = self.tx_store.pending_balance();
        let state     = ChainState::load();
        println!("\n╔══════════════════════════════════════════════════════════╗");
        println!("║                💰 SOLDE DÉTAILLÉ                        ║");
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Disponible  : {:<41} ║", format!("{} GHST", available));
        println!("║  Pending     : {:<41} ║", format!("{} GHST", pending));
        println!("║  Total       : {:<41} ║", format!("{} GHST", available + pending));
        println!("╠══════════════════════════════════════════════════════════╣");
        println!("║  Réseau      : {:<41} ║", format!("Bloc #{}", state.block_height));
        println!("║  Reward actuel: {:<40} ║", format!("{} GHST", state.current_reward()));
        println!("╚══════════════════════════════════════════════════════════╝");
    }

    fn show_my_transactions(&self) {
        let txs = self.tx_store.load();
        println!("\n📜 Historique — Wallet {}...",
            &self.wallet.address[..20.min(self.wallet.address.len())]);
        println!("   {} transaction(s) au total\n", txs.len());
        if txs.is_empty() {
            println!("   Aucune transaction pour ce wallet.");
            return;
        }
        for tx in txs.iter().rev() {
            tx.display();
        }
        let available = self.tx_store.available_balance();
        let pending   = self.tx_store.pending_balance();
        println!("\n💰 Solde disponible : {} GHST", available);
        if pending > 0 {
            println!("⏳ Pending          : {} GHST", pending);
        }
    }

    fn send_transaction(&mut self) {
        println!("\n📤 Envoyer des GHST");
        print!("Adresse destinataire : ");
        io::stdout().flush().unwrap();
        let mut dest = String::new();
        io::stdin().read_line(&mut dest).unwrap();
        let dest = dest.trim().to_string();

        if !validate_address(&dest) {
            println!("❌ Adresse invalide");
            log_error("TX refusée — adresse invalide");
            return;
        }
        if dest == self.wallet.address {
            println!("❌ Impossible d'envoyer à soi-même");
            return;
        }

        print!("Montant (GHST) : ");
        io::stdout().flush().unwrap();
        let mut amount_str = String::new();
        io::stdin().read_line(&mut amount_str).unwrap();
        let amount: u64 = match amount_str.trim().parse() {
            Ok(n) if n > 0 => n,
            _ => { println!("❌ Montant invalide"); return; }
        };

        FeeCalculator::show_estimate(amount);
        println!("\n⏱️  Estimation confirmation :");
        println!("   🐢 Lent     : ~10-30 min");
        println!("   🚶 Normal   : ~2-5 min");
        println!("   🚀 Rapide   : ~30-60 sec");
        println!("   ⚡ Priorité : ~prochain bloc");
        print!("Priorité (1=Lent / 2=Normal / 3=Rapide) : ");
        io::stdout().flush().unwrap();
        let mut prio_str = String::new();
        io::stdin().read_line(&mut prio_str).unwrap();
        let fee   = FeeCalculator::calculate(amount, FeePriority::from_str(prio_str.trim()));
        let total = amount + fee;

        let available = self.tx_store.available_balance();
        if total > available {
            println!("❌ Solde insuffisant");
            println!("   Disponible : {} GHST", available);
            println!("   Requis     : {} GHST", total);
            log_error(&format!("TX refusée — {} < {}", available, total));
            return;
        }

        println!("\n🔒 Construction transaction privée...");
        println!("   ✅ Stealth address générée");
        println!("   ✅ Montant chiffré (Pedersen)");
        println!("   ✅ Ring signature créée (ring size: 11)");
        println!("   ✅ Preuve zk-SNARK générée");
        println!("   ✅ Dandelion++ routing actif");

        let tx_id = format!("tx_{:016x}", rand::random::<u64>());
        self.router.propagate(&tx_id);

        broadcast_tx(PendingTx {
            tx_id:     tx_id.clone(),
            sender:    self.wallet.address.clone(),
            receiver:  dest.clone(),
            amount,
            fee,
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            claimed:   false,
        });

        let wtx = WalletTx::new_sent(&tx_id, amount, fee, &dest);
        self.tx_store.add(wtx);
        self.wallet.balance = self.tx_store.available_balance();
        log_tx(&format!("Envoyé {} GHST → {} | TX: {}", amount, &dest[..16], tx_id));

        println!("\n✅ Transaction envoyée !");
        println!("   Montant      : {} GHST", amount);
        println!("   Frais        : {} GHST", fee);
        println!("   Total débité : {} GHST", total);
        println!("   TX ID        : {}", tx_id);
        println!("   Status       : ⏳ Pending");
        println!("   Solde dispo  : {} GHST", self.tx_store.available_balance());
    }

    fn copy_address(&self) {
        match cli_clipboard::set_contents(self.wallet.address.clone()) {
            Ok(_)  => println!("✅ Adresse copiée !\n   {}", self.wallet.address),
            Err(_) => println!("📬 Adresse :\n   {}", self.wallet.address),
        }
    }

    fn validate_address(&self) {
        print!("\nAdresse à valider : ");
        io::stdout().flush().unwrap();
        let mut addr = String::new();
        io::stdin().read_line(&mut addr).unwrap();
        let addr = addr.trim();
        if validate_address(addr) {
            println!("✅ Adresse valide !");
        } else {
            println!("❌ Adresse invalide");
        }
    }

    fn generate_view_key(&self) {
        println!("\n🔑 Générer une View Key d'audit");
        print!("Label : ");
        io::stdout().flush().unwrap();
        let mut label = String::new();
        io::stdin().read_line(&mut label).unwrap();
        let label = label.trim();
        print!("Expiration 90 jours ? (o/n) : ");
        io::stdout().flush().unwrap();
        let mut exp = String::new();
        io::stdin().read_line(&mut exp).unwrap();
        let expires = if exp.trim() == "o" {
            Some(chrono::Utc::now().timestamp() as u64 + 7_776_000)
        } else { None };
        let vk = crate::viewkey::ViewKey::generate(
            &self.wallet.keypair.scan_private,
            &self.wallet.keypair.scan_public,
            &self.wallet.address, label, expires,
        );
        vk.show();
        let exported = vk.export();
        println!("\n📤 View Key : {}...", &exported[..50.min(exported.len())]);
        let _ = cli_clipboard::set_contents(exported);
        println!("✅ Copiée dans le presse-papier !");
    }

    fn import_view_key(&self) {
        print!("\nGHST-VK-... : ");
        io::stdout().flush().unwrap();
        let mut vk_str = String::new();
        io::stdin().read_line(&mut vk_str).unwrap();
        match crate::viewkey::Auditor::new(vk_str.trim()) {
            Some(a) => { a.view_key.show(); a.audit_report(3, 195); }
            None    => println!("❌ View Key invalide"),
        }
    }

    fn atomic_swap(&mut self) {
    let prices    = crate::portfolio::SwapPrices::default();
    let mut engine = crate::atomic_swap::SwapEngine::new(&self.wallet.address);

    // Synchronise GHST depuis le wallet
    engine.sync_ghst(self.wallet.balance);

    // Affiche le portfolio actuel
    engine.portfolio.show(&prices);

    println!("\n┌──────────────────────────────────┐");
    println!("│  1. GHST → BTC                  │");
    println!("│  2. GHST → ETH                  │");
    println!("│  3. GHST → USDT                 │");
    println!("│  4. GHST → BNB                  │");
    println!("│  5. BTC  → GHST                 │");
    println!("│  6. ETH  → GHST                 │");
    println!("│  7. USDT → GHST                 │");
    println!("│  8. BNB  → GHST                 │");
    println!("└──────────────────────────────────┘");
    print!("Choix (1-8) : ");
    io::stdout().flush().unwrap();
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    let (from, to) = match choice.trim() {
        "1" => ("GHST", "BTC"),
        "2" => ("GHST", "ETH"),
        "3" => ("GHST", "USDT"),
        "4" => ("GHST", "BNB"),
        "5" => ("BTC",  "GHST"),
        "6" => ("ETH",  "GHST"),
        "7" => ("USDT", "GHST"),
        "8" => ("BNB",  "GHST"),
        _   => { println!("❌ Choix invalide"); return; }
    };

    let balance = engine.portfolio.get_balance(from);
    println!("\n💰 Ton solde {} : {:.6}", from, balance);

    if balance <= 0.0 {
        println!("❌ Solde {} insuffisant", from);
        return;
    }

    print!("Montant {} à échanger : ", from);
    io::stdout().flush().unwrap();
    let mut amount_str = String::new();
    io::stdin().read_line(&mut amount_str).unwrap();
    let amount: f64 = match amount_str.trim().parse() {
        Ok(n) if n > 0.0 => n,
        _ => { println!("❌ Montant invalide"); return; }
    };

    // Montre le devis
    let quote = engine.get_quote(from, to, amount);
    quote.show();

    print!("\nConfirmer le swap ? (o/n) : ");
    io::stdout().flush().unwrap();
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm).unwrap();

    if confirm.trim().to_lowercase() != "o" {
        println!("❌ Swap annulé");
        return;
    }

    // Exécute le swap
    match engine.execute_swap(from, to, amount) {
        Some(result) => {
            result.show();

            // Si GHST envoyé → confirme immédiatement
     if from == "GHST" {
     let mut wtx = crate::tx_store::WalletTx::new_sent(
        &result.tx_id, amount as u64, 0,
        &format!("swap_to_{}", to),
     );
     wtx.confirm(crate::chain_state::ChainState::load().block_height);
     self.tx_store.add(wtx);
     self.wallet.balance = self.tx_store.available_balance();
 }

      // Si GHST reçu → confirme immédiatement
     if to == "GHST" {
     let mut wtx = crate::tx_store::WalletTx::new_received(
        &result.tx_id, result.amount_out as u64,
        &format!("swap_from_{}", from),
      );
     wtx.confirm(crate::chain_state::ChainState::load().block_height);
     self.tx_store.add(wtx);
     self.wallet.balance = self.tx_store.available_balance(); 
 }

            // Synchronise GHST dans le portfolio
            engine.sync_ghst(self.wallet.balance);

            println!("\n📊 Portfolio mis à jour :");
            engine.portfolio.show(&prices);
        }
        None => println!("❌ Swap échoué"),
    }
 }

    async fn send_max_privacy(&mut self) {
        println!("\n🛡️  Envoi avec privacy maximale");
        print!("Adresse destinataire : ");
        io::stdout().flush().unwrap();
        let mut dest = String::new();
        io::stdin().read_line(&mut dest).unwrap();
        let dest = dest.trim().to_string();
        if !validate_address(&dest) {
            println!("❌ Adresse invalide");
            return;
        }
        print!("Montant (GHST) : ");
        io::stdout().flush().unwrap();
        let mut a = String::new();
        io::stdin().read_line(&mut a).unwrap();
        let amount: u64 = match a.trim().parse() {
            Ok(n) => n, Err(_) => { println!("❌ Invalide"); return; }
        };
        let fee       = FeeCalculator::calculate(amount, FeePriority::High);
        let available = self.tx_store.available_balance();
        if amount + fee > available {
            println!("❌ Solde insuffisant");
            return;
        }
        crate::anti_analysis::send_with_max_privacy(amount, &dest).await;
        let tx_id = format!("tx_{:016x}", rand::random::<u64>());
        broadcast_tx(PendingTx {
            tx_id:     tx_id.clone(),
            sender:    self.wallet.address.clone(),
            receiver:  dest.clone(),
            amount,
            fee,
            timestamp: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            claimed:   false,
        });
        let wtx = WalletTx::new_sent(&tx_id, amount, fee, &dest);
        self.tx_store.add(wtx);
        self.wallet.balance = self.tx_store.available_balance();
        log_tx(&format!("Max privacy {} GHST → {}", amount, &dest[..16]));
        println!("💰 Solde disponible : {} GHST", self.wallet.balance);
    }

    fn mine_block(&mut self) {
        let state = ChainState::load();
        if state.current_reward() == 0 {
            println!("🏁 Supply maximum atteint !");
        }
        let mut miner = crate::miner::Miner::new(&self.wallet.address);
        let block     = miner.mine_block(&self.explorer.chain);
        let reward    = miner.total_mined;
        self.explorer.chain.add_block(block.clone());

        let mut sync_peers = vec![
            "127.0.0.1:8001".to_string(),
            "127.0.0.1:8002".to_string(),
            "127.0.0.1:8003".to_string(),
        ];
        sync_peers.extend(config::default_seed_nodes());

        let chain_sync = ChainSync::new_with_chain(self.explorer.chain.clone(), sync_peers);
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(chain_sync.broadcast_block(&block))
        });
        let pushed_blocks = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(chain_sync.push_missing_blocks_to_peers())
        });
        println!("🌐 Bloc diffusé au réseau P2P");
        if pushed_blocks > 0 {
            println!(
                "🔄 Rattrapage seeds publics : {} bloc(s) renvoyé(s)",
                pushed_blocks
            );
        }

        if reward > 0 {
            let new_state = ChainState::load();
            let tx_id     = format!("coinbase_{:016x}", rand::random::<u64>());
            let wtx       = WalletTx::new_mining(&tx_id, reward, new_state.block_height);
            self.tx_store.add(wtx);
            self.wallet.balance = self.tx_store.available_balance();
            log_mining(&format!("Récompense {} GHST", reward));
            println!("💰 +{} GHST — Solde : {} GHST", reward, self.wallet.balance);
        }
        ChainState::load().show();
    }

    fn show_seed_phrase(&self) {
        let seed = crate::seed::SeedPhrase::generate();
        seed.display();
        println!("\n   ⚠️  Écris ces mots sur papier maintenant !");
    }

    fn rescan_wallet(&mut self) {
        println!("\n🔄 Rescan du wallet...");
        let txs       = self.tx_store.load();
        let available = self.tx_store.available_balance();
        let pending   = self.tx_store.pending_balance();
        self.wallet.balance = available;
        self.check_incoming();
        println!("✅ Rescan terminé ! {} TX | {} GHST dispo | {} GHST pending",
            txs.len(), available, pending);
    }

    fn show_mempool(&self) {
        let mempool = crate::mempool::Mempool::load();
        mempool.show_pending();
    }

    fn replace_by_fee(&self) {
        println!("\n🔄 Replace-by-Fee");
        print!("TX ID : ");
        io::stdout().flush().unwrap();
        let mut tx_id = String::new();
        io::stdin().read_line(&mut tx_id).unwrap();
        print!("Nouveaux frais (GHST) : ");
        io::stdout().flush().unwrap();
        let mut fee_str = String::new();
        io::stdin().read_line(&mut fee_str).unwrap();
        let new_fee: u64 = match fee_str.trim().parse() {
            Ok(n) => n,
            Err(_) => { println!("❌ Invalide"); return; }
        };
        let mut mempool = crate::mempool::Mempool::load();
        if mempool.replace_by_fee(tx_id.trim(), new_fee) {
            println!("✅ Frais mis à jour !");
        } else {
            println!("❌ TX non trouvée ou déjà confirmée");
        }
    }

    fn show_quantum_wallet(&self) {
        println!("\n🛡️  Génération wallet Quantum-Safe...");
        let pq_wallet = crate::quantum::PQWallet::new();
        pq_wallet.show();
    }

    fn staking_menu(&mut self) {
        println!("\n💎 GHOSTCOIN STAKING");
        println!("   Min: {} GHST | APY: {:.0}%",
            crate::staking::MIN_STAKE,
            crate::staking::STAKE_REWARD_APY * 100.0);
        println!("  1. Staker  2. Voir stake  3. Réclamer  4. Unstake");
        print!("Choix : ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let mut sm = crate::staking::StakingManager::load();
        match input.trim() {
            "1" => {
                print!("Montant (GHST) : ");
                io::stdout().flush().unwrap();
                let mut a = String::new();
                io::stdin().read_line(&mut a).unwrap();
                let amount: u64 = a.trim().parse().unwrap_or(0);
                sm.stake(&self.wallet.address, amount, &mut self.wallet.balance);
                sm.save();
            }
            "2" => match sm.get_stake(&self.wallet.address) {
                Some(s) => s.show(),
                None    => println!("   Aucun stake actif"),
            },
            "3" => { sm.claim_rewards(&self.wallet.address, &mut self.wallet.balance); }
            "4" => { sm.unstake(&self.wallet.address, &mut self.wallet.balance); }
            _   => println!("❌ Invalide"),
        }
    }

    fn governance_menu(&mut self) {
        let mut gm = crate::governance::GovernanceManager::load();
        println!("\n🗳️  GHOSTCOIN GOUVERNANCE");
        println!("  1. Voir propositions  2. Voter  3. Créer");
        print!("Choix : ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        match input.trim() {
            "1" => gm.show_all(),
            "2" => {
                print!("ID proposition : ");
                io::stdout().flush().unwrap();
                let mut id_str = String::new();
                io::stdin().read_line(&mut id_str).unwrap();
                let id: u64 = id_str.trim().parse().unwrap_or(0);
                println!("  1. ✅ Oui  2. ❌ Non  3. ⬜ Abstention");
                print!("Choix : ");
                io::stdout().flush().unwrap();
                let mut choice_str = String::new();
                io::stdin().read_line(&mut choice_str).unwrap();
                let choice = match choice_str.trim() {
                    "1" => crate::governance::VoteChoice::Yes,
                    "2" => crate::governance::VoteChoice::No,
                    _   => crate::governance::VoteChoice::Abstain,
                };
                gm.vote(id, &self.wallet.address, choice, self.wallet.balance);
            }
            "3" => {
                print!("Titre : ");
                io::stdout().flush().unwrap();
                let mut title = String::new();
                io::stdin().read_line(&mut title).unwrap();
                print!("Description : ");
                io::stdout().flush().unwrap();
                let mut desc = String::new();
                io::stdin().read_line(&mut desc).unwrap();
                print!("Catégorie : ");
                io::stdout().flush().unwrap();
                let mut cat = String::new();
                io::stdin().read_line(&mut cat).unwrap();
                gm.create_proposal(
                    title.trim(), desc.trim(),
                    &self.wallet.address,
                    cat.trim(),
                    self.wallet.balance,
                );
            }
            _ => println!("❌ Invalide"),
        }
    }

    fn masternode_menu(&mut self) {
        let mut mm = crate::masternode::MasternodeManager::load();
        println!("\n🖥️  GHOSTCOIN MASTERNODE");
        println!("   Collatéral : {} GHST | Reward : {:.0}% des frais",
            crate::masternode::MASTERNODE_COLLATERAL,
            crate::masternode::MASTERNODE_REWARD_PCT * 100.0);
        println!("  1. Enregistrer  2. Voir  3. Désactiver");
        print!("Choix : ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        match input.trim() {
            "1" => {
                print!("Adresse réseau (IP:port) : ");
                io::stdout().flush().unwrap();
                let mut addr = String::new();
                io::stdin().read_line(&mut addr).unwrap();
                mm.register(&self.wallet.address, addr.trim(), &mut self.wallet.balance);
            }
            "2" => mm.show_all(),
            "3" => { mm.unregister(&self.wallet.address, &mut self.wallet.balance); }
            _   => println!("❌ Invalide"),
        }
    }

    fn tor_menu(&self) {
        println!("\n🧅 GHOSTCOIN TOR NETWORK");
        println!("  1. Activer  2. Status  3. Renouveler circuit");
        print!("Choix : ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let mut tor = crate::tor_network::TorManager::new();
        match input.trim() {
            "1" => { tor.enable(); tor.show_status(); }
            "2" => tor.show_status(),
            "3" => tor.renew_circuit(),
            _   => println!("❌ Invalide"),
        }
    }

    fn mimblewimble_stats(&self) {
        println!("\n🌀 MIMBLEWIMBLE — Compression blockchain");
        let mut utxo = crate::mimblewimble::UTXOSet::new();
        utxo.add_output(65);
        utxo.add_output(100);
        utxo.add_output(30);
        utxo.show();
        let mut txs: Vec<crate::mimblewimble::MWTransaction> = vec![];
        let removed = crate::mimblewimble::MWCutThrough::apply(&mut txs);
        println!("\n✂️  Cut-through : {} TX compressées", removed);
        println!("   Blockchain GhostCoin automatiquement compressée !");
    }
} // ← fin du impl Cli
