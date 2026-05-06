use axum::{
    extract::State,
    http::HeaderMap,
    response::Html,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::chain_state::ChainState;
use crate::mempool::Mempool;
use crate::storage::{broadcast_tx, PendingTx};

struct ViewData {
    state: ChainState,
    mempool: Mempool,
}

struct FaucetState {
    claims: Mutex<HashMap<String, u64>>,
}

impl FaucetState {
    fn new() -> Self {
        Self {
            claims: Mutex::new(HashMap::new()),
        }
    }

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn can_claim(&self, key: &str) -> bool {
        let claims = self
            .claims
            .lock()
            .expect("faucet claims mutex poisoned");
        match claims.get(key) {
            Some(&ts) => Self::now_secs().saturating_sub(ts) >= 86_400,
            None => true,
        }
    }

    fn record(&self, key: &str) {
        self.claims
            .lock()
            .expect("faucet claims mutex poisoned")
            .insert(key.to_string(), Self::now_secs());
    }
}

#[derive(Deserialize)]
struct FaucetRequest {
    address: String,
}

#[derive(Serialize)]
struct FaucetResponse {
    success: bool,
    message: String,
    tx_id: Option<String>,
    amount: Option<u64>,
}

const FAUCET_AMOUNT: u64 = 25;
const FAUCET_FEE: u64 = 1;
const FAUCET_ADDRESS: &str = "faucet";

impl ViewData {
    fn load() -> Self {
        Self {
            state: ChainState::load(),
            mempool: Mempool::load(),
        }
    }

    fn max_supply(&self) -> u64 {
        50_000_000
    }

    fn remaining_supply(&self) -> u64 {
        self.max_supply().saturating_sub(self.state.minted_supply)
    }

    fn supply_pct(&self) -> f64 {
        (self.state.minted_supply as f64 / self.max_supply() as f64) * 100.0
    }

    fn pending_count(&self) -> usize {
        self.mempool.pending_count()
    }

    fn total_mempool_fees(&self) -> u64 {
        self.mempool.total_fees()
    }

    fn avg_fee(&self) -> f64 {
        let pending = self.pending_count();
        if pending == 0 {
            0.0
        } else {
            self.total_mempool_fees() as f64 / pending as f64
        }
    }

    fn last_hash_short(&self) -> String {
        if self.state.last_block_hash.len() > 18 {
            format!("{}...", &self.state.last_block_hash[..18])
        } else {
            self.state.last_block_hash.clone()
        }
    }

    fn blocks_rows(&self) -> String {
        if self.state.block_height == 0 {
            return "<tr><td>#0</td><td><code>0</code></td><td>Waiting for first block</td><td>Fresh node</td></tr>"
                .to_string();
        }

        (0..6)
            .map(|offset| {
                let height = self.state.block_height.saturating_sub(offset);
                let hash = if offset == 0 {
                    self.last_hash_short()
                } else {
                    format!("Snapshot #{}", height)
                };
                let status = if offset == 0 { "Current tip" } else { "Recent window" };
                format!(
                    "<tr><td>#{}</td><td><code>{}</code></td><td>{} GHST</td><td>{}</td></tr>",
                    height,
                    hash,
                    self.state.current_reward(),
                    status
                )
            })
            .collect::<Vec<_>>()
            .join("")
    }

    fn mempool_rows(&self) -> String {
        let rows: Vec<String> = self
            .mempool
            .sorted_by_priority()
            .iter()
            .take(12)
            .map(|tx| {
                let tx_id = if tx.tx_id.len() > 22 {
                    format!("{}...", &tx.tx_id[..22])
                } else {
                    tx.tx_id.clone()
                };
                format!(
                    "<tr><td><code>{}</code></td><td>{} GHST</td><td>{} GHST</td><td>{}/byte</td><td>{}</td></tr>",
                    tx_id,
                    tx.amount,
                    tx.fee,
                    tx.fee_rate,
                    tx.priority_label()
                )
            })
            .collect();

        if rows.is_empty() {
            "<tr><td colspan=\"5\">No pending transactions right now.</td></tr>".to_string()
        } else {
            rows.join("")
        }
    }

    fn holders_rows(&self) -> String {
        [
            ("ghost100000000_37c4d8", "Genesis", "103 046.88 GHST", "12.348%", "2 760"),
            ("ghost100000000_c81e40", "Miner Pool", "55 267.70 GHST", "6.622%", "4 548"),
            ("ghost100000000_6aad1f", "Miner Pool", "41 945.06 GHST", "5.026%", "2 380"),
            ("ghost100000000_9bd193", "Treasury Demo", "29 110.24 GHST", "3.487%", "1 122"),
            ("ghost100000000_12a770", "Cold Wallet", "21 440.00 GHST", "2.566%", "814"),
        ]
        .iter()
        .enumerate()
        .map(|(idx, (address, badge, balance, share, txs))| {
            format!(
                "<tr><td>{}</td><td><strong>{}</strong> <span class=\"badge ghost\">{}</span></td><td>{}</td><td>{}</td><td>{}</td></tr>",
                idx + 1,
                address,
                badge,
                balance,
                share,
                txs
            )
        })
        .collect::<Vec<_>>()
        .join("")
    }
}

fn nav_link(active: &str, key: &str, label: &str) -> String {
    let class = if active == key { "nav-link active" } else { "nav-link" };
    format!(
        "<a class=\"{}\" href=\"#\" data-tab=\"{}\">{}</a>",
        class, key, label
    )
}

fn render_nav(active: &str) -> String {
    [
        ("overview", "Overview"),
        ("blocks", "Blocks"),
        ("mempool", "Mempool"),
        ("holders", "Holders"),
        ("mining", "Mining"),
        ("buy", "Buy"),
        ("tokenomics", "Tokenomics"),
        ("roadmap", "Roadmap"),
        ("faq", "FAQ"),
        ("api", "API"),
    ]
    .iter()
    .map(|(key, label)| nav_link(active, key, label))
    .collect::<Vec<_>>()
    .join("")
}

fn render_market_sidebar() -> String {
    r#"
      <aside class="panel hero-side">
        <div class="section-kicker">Market snapshot</div>
        <div class="price-card">
          <div><strong>GHST</strong><span>Mainnet reference</span></div>
          <div class="price-value"><div id="ghstPrice">$0.0100</div><div class="chip up">Mainnet</div></div>
        </div>
        <div class="price-card">
          <div><strong>BTC</strong><span>Bitcoin live feed</span></div>
          <div class="price-value"><div id="btcPrice">Loading...</div><div class="chip up" id="btcChange">...</div></div>
        </div>
        <div class="price-card">
          <div><strong>ETH</strong><span>Ethereum live feed</span></div>
          <div class="price-value"><div id="ethPrice">Loading...</div><div class="chip up" id="ethChange">...</div></div>
        </div>
        <div class="price-card">
          <div><strong>BNB</strong><span>BNB live feed</span></div>
          <div class="price-value"><div id="bnbPrice">Loading...</div><div class="chip up" id="bnbChange">...</div></div>
        </div>
        <div>
          <strong>Last update</strong><br>
          <span style="color:var(--muted)" id="lastUpdate">--</span>
        </div>
        <div class="faucet-box">
          <div class="section-kicker">Testnet faucet</div>
          <strong>Claim 25 GHST</strong>
          <p>One claim per address and per IP every 24 hours.</p>
          <form id="faucetForm" class="faucet-form">
            <input id="faucetAddress" type="text" placeholder="PC1-... address" autocomplete="off" />
            <button type="submit" class="hero-btn primary faucet-btn">Claim test coins</button>
          </form>
          <div id="faucetMessage" class="faucet-message">Use a valid GhostCoin address to request testnet funds.</div>
        </div>
      </aside>
"#
    .to_string()
}

fn render_wallet_strip() -> String {
    r##"
      <article class="panel wallet-strip">
        <div class="section-title">
          <h2>Download wallet</h2>
          <span>GhostCoin CLI · v1.0</span>
        </div>
        <div class="wallet-grid">
          <a class="wallet-card" href="https://github.com/GungTortueninja224/-Ghostcoin/releases">
            <div class="wallet-meta">
              <div class="wallet-icon">⌂</div>
              <div><strong>Windows</strong><span>~24 MB</span></div>
            </div>
            <div class="wallet-arrow">↗</div>
          </a>
          <a class="wallet-card" href="https://github.com/GungTortueninja224/-Ghostcoin/releases">
            <div class="wallet-meta">
              <div class="wallet-icon">⌘</div>
              <div><strong>macOS</strong><span>~24 MB</span></div>
            </div>
            <div class="wallet-arrow">↗</div>
          </a>
          <a class="wallet-card" href="https://github.com/GungTortueninja224/-Ghostcoin/releases">
            <div class="wallet-meta">
              <div class="wallet-icon">›</div>
              <div><strong>Linux</strong><span>~24 MB</span></div>
            </div>
            <div class="wallet-arrow">↗</div>
          </a>
          <a class="wallet-card" href="https://github.com/GungTortueninja224/-Ghostcoin">
            <div class="wallet-meta">
              <div class="wallet-icon">⌗</div>
              <div><strong>View source</strong><span>GitHub repository</span></div>
            </div>
            <div class="wallet-arrow">↗</div>
          </a>
        </div>
      </article>
"##
    .to_string()
}

fn render_panel(key: &str, active: bool, content: String) -> String {
    let class = if active { "tab-panel active" } else { "tab-panel" };
    format!(r#"<section class="{class}" data-panel="{key}">{content}</section>"#)
}

fn overview_panel(data: &ViewData) -> String {
    format!(
        r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Overview</h2>
          <span>Fast health check for the public GhostCoin node</span>
        </div>
        <div class="substats">
          <div class="substat">
            <div class="eyeline">Current reward</div>
            <div class="big">{} GHST</div>
            <div class="mini-copy">Block subsidy in the current era.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Total fees</div>
            <div class="big">{} GHST</div>
            <div class="mini-copy">Fees already secured by confirmed blocks.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Last hash</div>
            <div class="big"><code>{}</code></div>
            <div class="mini-copy">Current chain head hash on this node.</div>
          </div>
        </div>
      </article>

      <article class="panel content-card wide">
        <div class="section-title">
          <h2>Market context</h2>
          <span>BTC and ETH for macro reference around GHST</span>
        </div>
        <div class="chart-shell">
          <canvas id="priceChart" height="120"></canvas>
        </div>
      </article>

      <article class="panel content-card narrow">
        <div class="section-title">
          <h2>Supply distribution</h2>
          <span>{:.4}% mined</span>
        </div>
        <div class="donut-shell">
          <canvas id="supplyChart" height="210"></canvas>
        </div>
        <div class="supply-meta">
          <div class="progress"><div></div></div>
          <div class="progress-legend">
            <span>Mined: {} GHST</span>
            <span>Remaining: {} GHST</span>
          </div>
        </div>
      </article>
"#,
        data.state.current_reward(),
        data.state.total_fees,
        data.last_hash_short(),
        data.supply_pct(),
        data.state.minted_supply,
        data.remaining_supply()
    )
}

fn blocks_panel(data: &ViewData) -> String {
    format!(
        r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Blocks</h2>
          <span>Latest chain snapshot</span>
        </div>
        <div class="substats">
          <div class="substat">
            <div class="eyeline">Current tip</div>
            <div class="big">#{}</div>
            <div class="mini-copy">Latest public height on the node.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Last hash</div>
            <div class="big"><code>{}</code></div>
            <div class="mini-copy">Head hash for the active chain.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Confirmed tx</div>
            <div class="big">{}</div>
            <div class="mini-copy">Transactions already mined into blocks.</div>
          </div>
        </div>
        <table>
          <tr><th>Height</th><th>Hash / Snapshot</th><th>Reward</th><th>Status</th></tr>
          {}
        </table>
      </article>
"#,
        data.state.block_height,
        data.last_hash_short(),
        data.state.total_tx_count,
        data.blocks_rows()
    )
}

fn mempool_panel(data: &ViewData) -> String {
    format!(
        r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Mempool</h2>
          <span>Live pending transaction queue</span>
        </div>
        <div class="substats">
          <div class="substat">
            <div class="eyeline">Pending tx</div>
            <div class="big">{}</div>
            <div class="mini-copy">Transactions waiting for inclusion.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Total fees</div>
            <div class="big">{} GHST</div>
            <div class="mini-copy">Accumulated fees in the current mempool.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Average fee</div>
            <div class="big">{:.2} GHST</div>
            <div class="mini-copy">Average fee across pending entries.</div>
          </div>
        </div>
        <table>
          <tr><th>TX ID</th><th>Amount</th><th>Fee</th><th>Fee rate</th><th>Priority</th></tr>
          {}
        </table>
      </article>
"#,
        data.pending_count(),
        data.total_mempool_fees(),
        data.avg_fee(),
        data.mempool_rows()
    )
}

fn holders_panel(data: &ViewData) -> String {
    format!(
        r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Top holders</h2>
          <span><span class="badge demo">Demo ranking until full holder indexer lands</span></span>
        </div>
        <div class="substats">
          <div class="substat">
            <div class="eyeline">Total holders</div>
            <div class="big">14,217</div>
            <div class="mini-copy">Estimated public holder set.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Top 10 share</div>
            <div class="big">39.60%</div>
            <div class="mini-copy">Demo concentration view for the explorer layout.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Average balance</div>
            <div class="big">59 GHST</div>
            <div class="mini-copy">Display-only until the address index ships.</div>
          </div>
        </div>
        <table>
          <tr><th>#</th><th>Address</th><th>Balance</th><th>Share</th><th>TXs</th></tr>
          {}
        </table>
      </article>
"#,
        data.holders_rows()
    )
}

fn mining_panel(data: &ViewData) -> String {
    format!(
        r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Mining</h2>
          <span>Current chain economics and halving progress</span>
        </div>
        <div class="substats">
          <div class="substat">
            <div class="eyeline">Reward</div>
            <div class="big">{} GHST</div>
            <div class="mini-copy">Current subsidy for each mined block.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Next halving</div>
            <div class="big">#{}</div>
            <div class="mini-copy">Planned halving height based on protocol rules.</div>
          </div>
          <div class="substat">
            <div class="eyeline">Era progress</div>
            <div class="big">{:.2}%</div>
            <div class="mini-copy">Progress inside the active halving window.</div>
          </div>
        </div>
        <table>
          <tr><th>Metric</th><th>Value</th></tr>
          <tr><td>Consensus</td><td>Proof of Work, SHA-256</td></tr>
          <tr><td>Current reward</td><td>{} GHST / block</td></tr>
          <tr><td>Difficulty</td><td>{}</td></tr>
          <tr><td>Halving interval</td><td>210,000 blocks</td></tr>
        </table>
      </article>
"#,
        data.state.current_reward(),
        data.state.next_halving_block(),
        data.state.halving_progress(),
        data.state.current_reward(),
        data.state.difficulty
    )
}

fn buy_panel() -> String {
    r##"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Buy GHST</h2>
          <span>Current acquisition flow for early network users</span>
        </div>
        <div class="buy-grid">
          <div class="buy-card">
            <div class="buy-top">
              <div class="buy-icon">⛏</div>
              <span class="badge ghost">Live</span>
            </div>
            <h3>Mine GHST</h3>
            <p>Run the CLI wallet, start mining locally, and secure fresh GHST directly from the active chain.</p>
          </div>
          <div class="buy-card soft">
            <div class="buy-top">
              <div class="buy-icon">⇄</div>
              <span class="badge demo">Soon</span>
            </div>
            <h3>Decentralized exchange</h3>
            <p>Privacy-friendly swap routes can plug in here once live liquidity and routing are ready.</p>
          </div>
          <div class="buy-card soft">
            <div class="buy-top">
              <div class="buy-icon">◎</div>
              <span class="badge demo">Soon</span>
            </div>
            <h3>OTC trading</h3>
            <p>Large size community transfers will fit here later with clearer counterparties and settlement flow.</p>
          </div>
        </div>
      </article>
"##
    .to_string()
}

fn tokenomics_panel(data: &ViewData) -> String {
    format!(
        r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Tokenomics</h2>
          <span>Confirmed transactions: {}</span>
        </div>
        <table>
          <tr><th>Field</th><th>Value</th></tr>
          <tr><td>Name</td><td>GhostCoin</td></tr>
          <tr><td>Symbol</td><td>GHST</td></tr>
          <tr><td>Max supply</td><td>{} GHST</td></tr>
          <tr><td>Current reward</td><td>{} GHST per block</td></tr>
          <tr><td>Consensus</td><td>Proof of Work, SHA-256</td></tr>
          <tr><td>Halving interval</td><td>210,000 blocks</td></tr>
          <tr><td>Infrastructure</td><td>Rust + Railway</td></tr>
          <tr><td>Status</td><td>Public explorer online</td></tr>
        </table>
      </article>
"#,
        data.state.total_tx_count,
        data.max_supply(),
        data.state.current_reward()
    )
}

fn roadmap_panel() -> String {
    r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Roadmap</h2>
          <span>Short-term product and protocol priorities</span>
        </div>
        <div class="roadmap">
          <div class="roadmap-item"><strong>Phase 1 - Stable explorer</strong><p>Keep the public dashboard clean, readable, and wired to live node stats plus mempool visibility.</p></div>
          <div class="roadmap-item"><strong>Phase 2 - Wallet transfers</strong><p>Expose the first public GHST transactions between wallets with clearer transaction detail pages and live confirmations.</p></div>
          <div class="roadmap-item"><strong>Phase 3 - Public seed</strong><p>Move to a persistent seed or VPS so chain history survives restarts and bootstrap becomes automatic.</p></div>
        </div>
      </article>
"#
    .to_string()
}

fn faq_panel() -> String {
    r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>FAQ</h2>
          <span>Quick answers for miners, holders, and node operators</span>
        </div>
        <div class="faq">
          <details open>
            <summary>Why can Railway show height #0 after a restart?</summary>
            <p>Because the free Railway setup has no persistent volume. After a restart, the node needs either a public bootstrap peer or a persistent disk to recover its chain state automatically.</p>
          </details>
          <details>
            <summary>Is the mempool live?</summary>
            <p>Yes. The mempool section reads the current pending queue from the running node and shows real pending transactions when they exist.</p>
          </details>
          <details>
            <summary>Are holders and buy sections fully live?</summary>
            <p>Not yet. Those sections are intentionally marked as demo or guide content until the backing indexer and liquidity sources are ready.</p>
          </details>
        </div>
      </article>
"#
    .to_string()
}

fn api_panel() -> String {
    r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>API and endpoints</h2>
          <span>Use these for wallets, nodes, or dashboards</span>
        </div>
        <div class="endpoint-list">
          <div class="endpoint">TCP -> 168.220.83.3:8001</div>
          <div class="endpoint">HTTP -> ghostcoin-production.up.railway.app</div>
          <div class="endpoint">API -> ghostcoin-production.up.railway.app/api/stats</div>
          <div class="endpoint">Mempool API -> ghostcoin-production.up.railway.app/api/mempool</div>
        </div>
      </article>
"#
    .to_string()
}

fn render_join_page() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Join GhostCoin Testnet</title>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body {
    background: #0a0a0e;
    color: #e0e0e0;
    font-family: "Courier New", monospace;
    padding: 2rem 1rem;
  }
  .wrap { max-width: 760px; margin: 0 auto; }
  h1 { color: #00ff88; font-size: 1.9rem; margin-bottom: 0.35rem; }
  .sub { color: #6b7280; font-size: 0.9rem; margin-bottom: 2.2rem; }
  h2 {
    color: #59f2b0;
    font-size: 1rem;
    letter-spacing: 2px;
    text-transform: uppercase;
    margin: 2rem 0 0.8rem;
    border-bottom: 1px solid rgba(0,255,136,0.14);
    padding-bottom: 0.45rem;
  }
  p { color: #a8afb8; line-height: 1.75; margin-bottom: 1rem; }
  pre {
    background: #111118;
    border: 1px solid rgba(0,255,136,0.14);
    border-radius: 6px;
    padding: 1rem 1.2rem;
    color: #00ff88;
    font-size: 0.85rem;
    overflow-x: auto;
    margin-bottom: 1rem;
    white-space: pre-wrap;
  }
  .badge {
    display: inline-block;
    background: rgba(0,255,136,0.08);
    border: 1px solid rgba(0,255,136,0.2);
    border-radius: 4px;
    padding: 3px 9px;
    font-size: 0.75rem;
    color: #8bf8c7;
    margin-bottom: 1rem;
  }
  a { color: #00ff88; }
  .nav {
    display: flex;
    gap: 1.2rem;
    margin-bottom: 2rem;
    font-size: 0.88rem;
    flex-wrap: wrap;
  }
  .nav a {
    color: rgba(0,255,136,0.72);
    text-decoration: none;
  }
  .nav a:hover { color: #00ff88; }
  table { width: 100%; border-collapse: collapse; margin-bottom: 1rem; }
  td, th {
    border: 1px solid rgba(0,255,136,0.14);
    padding: 0.6rem 0.8rem;
    font-size: 0.85rem;
    text-align: left;
  }
  th { color: #8bf8c7; background: #111118; }
  td { color: #b4bcc6; }
  code { color: #9ef8d0; }
</style>
</head>
<body>
<div class="wrap">

  <h1>Join GhostCoin Testnet</h1>
  <p class="sub">GhostCoin Testnet v1 — privacy-first L1 blockchain in Rust</p>

  <nav class="nav">
    <a href="/">Explorer</a>
    <a href="/faucet">Faucet</a>
    <a href="/join">How to join</a>
    <a href="https://github.com/GungTortueninja224/-Ghostcoin" target="_blank" rel="noreferrer">GitHub</a>
  </nav>

  <div class="badge">TESTNET v1 — faucet active · public bootstrap active</div>

  <h2>1. Requirements</h2>
  <p>You need Git and Rust installed on your machine. Linux, macOS, and Windows PowerShell are all fine.</p>
  <pre>curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env</pre>

  <h2>2. Clone and build</h2>
  <pre>git clone https://github.com/GungTortueninja224/-Ghostcoin.git
cd -Ghostcoin
cargo build --release</pre>
  <p>First build usually takes a few minutes.</p>

  <h2>3. Start the wallet and sync from the testnet seeds</h2>
  <p>On Linux or macOS:</p>
  <pre>GHOSTCOIN_BOOTSTRAP_PEERS="168.220.83.3:8001,137.66.10.29:8001" \
cargo run --release</pre>
  <p>On Windows PowerShell:</p>
  <pre>$env:GHOSTCOIN_BOOTSTRAP_PEERS="168.220.83.3:8001,137.66.10.29:8001"
cargo run --release</pre>
  <p>The interactive wallet starts local nodes automatically and syncs from the public seeds.</p>

  <h2>4. Check network status</h2>
  <pre>cargo run --release -- status 168.220.83.3:8001</pre>
  <p>You should see the current public block height and mempool state.</p>

  <h2>5. Claim test GHST</h2>
  <p>Use the faucet to receive 25 GHST on a valid GhostCoin address:</p>
  <pre>curl -X POST https://ghostcoin-production.up.railway.app/api/faucet \
  -H "Content-Type: application/json" \
  -d '{"address":"YOUR_GHST_ADDRESS"}'</pre>
  <p>Or use the faucet form directly on the <a href="/faucet">main explorer page</a>. Limit: 1 claim per address and per IP every 24 hours.</p>

  <h2>6. Mine a block</h2>
  <p>Open the interactive wallet and choose <code>E</code> to mine a block.</p>
  <pre>Wallet menu -> E -> Mine a block</pre>
  <p>Successful blocks earn 65 GHST in the current era. Reward halves every 210,000 blocks.</p>

  <h2>7. Send a transaction</h2>
  <p>Open the interactive wallet and choose <code>3</code> to send GHST.</p>
  <pre>Wallet menu -> 3 -> Enter recipient -> Enter amount</pre>

  <h2>Network info</h2>
  <table>
    <tr><th>Parameter</th><th>Value</th></tr>
    <tr><td>Network</td><td>GhostCoin Testnet v1</td></tr>
    <tr><td>Token</td><td>GHST</td></tr>
    <tr><td>Consensus</td><td>Proof of Work (SHA-256)</td></tr>
    <tr><td>Max supply</td><td>50,000,000 GHST</td></tr>
    <tr><td>Block reward</td><td>65 GHST</td></tr>
    <tr><td>Halving</td><td>Every 210,000 blocks</td></tr>
    <tr><td>P2P port</td><td>8001</td></tr>
    <tr><td>Seed 1</td><td>168.220.83.3:8001</td></tr>
    <tr><td>Seed 2</td><td>137.66.10.29:8001</td></tr>
    <tr><td>Explorer</td><td><a href="/">ghostcoin-production.up.railway.app</a></td></tr>
  </table>

  <h2>Privacy features (v1)</h2>
  <table>
    <tr><th>Feature</th><th>Status</th></tr>
    <tr><td>Stealth addresses</td><td>Active</td></tr>
    <tr><td>Ring signatures</td><td>Active</td></tr>
    <tr><td>Dandelion++ propagation</td><td>Active</td></tr>
    <tr><td>zk-SNARKs</td><td>Roadmap v2</td></tr>
  </table>

  <h2>Resources</h2>
  <p>
    <a href="https://github.com/GungTortueninja224/-Ghostcoin/blob/main/docs/whitepaper/GhostCoin_Whitepaper_v0.1.pdf" target="_blank" rel="noreferrer">Whitepaper v0.1 (PDF)</a>
    · <a href="https://github.com/GungTortueninja224/-Ghostcoin" target="_blank" rel="noreferrer">GitHub</a>
    · <a href="/api/stats">API stats</a>
  </p>

</div>
</body>
</html>"#
    .to_string()
}

fn render_single_page(data: &ViewData) -> String {
    let panels = [
        render_panel("overview", true, overview_panel(data)),
        render_panel("blocks", false, blocks_panel(data)),
        render_panel("mempool", false, mempool_panel(data)),
        render_panel("holders", false, holders_panel(data)),
        render_panel("mining", false, mining_panel(data)),
        render_panel("buy", false, buy_panel()),
        render_panel("tokenomics", false, tokenomics_panel(data)),
        render_panel("roadmap", false, roadmap_panel()),
        render_panel("faq", false, faq_panel()),
        render_panel("api", false, api_panel()),
    ]
    .join("");

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>GhostCoin Explorer</title>
  <meta name="description" content="GhostCoin public explorer with one clean page and switchable sections for blocks, mempool, holders, mining, roadmap, and API.">
  <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
  <style>
    :root {{
      --bg: #f6f5f0;
      --panel: rgba(255,255,255,0.88);
      --line: rgba(19,35,31,0.09);
      --text: #16251f;
      --muted: #62706d;
      --accent: #139c75;
      --accent-deep: #0d6b57;
      --accent-soft: #dff4ec;
      --ink: #1d3440;
      --shadow: 0 22px 60px rgba(22,36,28,0.10);
      --radius: 24px;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      color: var(--text);
      background:
        radial-gradient(circle at top left, rgba(19,156,117,0.12), transparent 28%),
        radial-gradient(circle at top right, rgba(28,52,64,0.08), transparent 24%),
        linear-gradient(180deg, #fffef9 0%, var(--bg) 100%);
      font-family: "Trebuchet MS", "Gill Sans", sans-serif;
    }}
    a {{ color: inherit; text-decoration: none; }}
    code {{
      font-family: "Consolas", "Courier New", monospace;
      color: var(--accent-deep);
      font-size: 0.92em;
    }}
    .shell {{
      width: min(1280px, calc(100vw - 28px));
      margin: 0 auto;
      padding: 18px 0 46px;
    }}
    .topbar {{
      position: sticky;
      top: 12px;
      z-index: 30;
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 18px;
      padding: 16px 20px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: rgba(255,255,255,0.78);
      backdrop-filter: blur(16px);
      box-shadow: 0 18px 40px rgba(20,35,31,0.08);
    }}
    .brand {{
      display: flex;
      align-items: center;
      gap: 14px;
    }}
    .brand-mark {{
      width: 58px;
      height: 58px;
      display: grid;
      place-items: center;
      border-radius: 50%;
      border: 1px solid rgba(19,156,117,0.22);
      background: radial-gradient(circle at 30% 30%, #ffffff, #edf8f4 60%, #d6efe6 100%);
      box-shadow: inset 0 1px 0 rgba(255,255,255,0.8), 0 12px 24px rgba(19,156,117,0.14);
      font-size: 1.02rem;
      font-weight: 900;
      letter-spacing: -0.06em;
      color: #111;
    }}
    .brand-title {{
      font-size: 1.32rem;
      font-weight: 800;
      letter-spacing: -0.03em;
    }}
    .brand-sub {{
      color: var(--muted);
      font-size: 0.78rem;
      font-weight: 700;
      letter-spacing: 0.12em;
      text-transform: uppercase;
    }}
    .live-pill {{
      display: inline-flex;
      align-items: center;
      gap: 8px;
      margin-left: 10px;
      padding: 6px 11px;
      border-radius: 999px;
      background: #effaf5;
      color: var(--accent-deep);
      font-size: 0.76rem;
      font-weight: 700;
    }}
    .live-pill::before {{
      content: "";
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--accent);
      box-shadow: 0 0 0 7px rgba(19,156,117,0.10);
    }}
    .topnav {{
      display: flex;
      align-items: center;
      gap: 10px;
      flex-wrap: nowrap;
      overflow-x: auto;
      max-width: 100%;
      padding-bottom: 2px;
      scrollbar-width: none;
    }}
    .topnav::-webkit-scrollbar {{ display: none; }}
    .topnav a {{
      flex: 0 0 auto;
      padding: 10px 14px;
      border-radius: 999px;
      color: var(--ink);
      font-size: 0.92rem;
      font-weight: 700;
      border: 1px solid transparent;
      transition: 160ms ease;
    }}
    .topnav a:hover,
    .topnav a:focus-visible {{
      background: rgba(19,156,117,0.09);
      border-color: rgba(19,156,117,0.16);
      outline: none;
    }}
    .topnav a.active {{
      background: linear-gradient(135deg, var(--accent), #64d1ae);
      color: #ffffff;
      box-shadow: 0 14px 30px rgba(19,156,117,0.22);
    }}
    .hero {{
      display: grid;
      grid-template-columns: 1.3fr 0.9fr;
      gap: 22px;
      margin-top: 28px;
      margin-bottom: 24px;
    }}
    .panel {{
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: var(--radius);
      box-shadow: var(--shadow);
      backdrop-filter: blur(12px);
    }}
    .hero-main {{
      padding: 34px;
      position: relative;
      overflow: hidden;
    }}
    .hero-main::after {{
      content: "";
      position: absolute;
      right: -44px;
      bottom: -66px;
      width: 220px;
      height: 220px;
      background: radial-gradient(circle, rgba(19,156,117,0.18), transparent 66%);
    }}
    .eyebrow {{
      display: inline-flex;
      align-items: center;
      gap: 8px;
      padding: 8px 12px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent-deep);
      font-size: 0.78rem;
      font-weight: 700;
      letter-spacing: 0.06em;
      text-transform: uppercase;
    }}
    h1 {{
      margin: 18px 0 12px;
      font-size: clamp(2.8rem, 4vw, 5.1rem);
      line-height: 0.92;
      letter-spacing: -0.06em;
      max-width: 10.5ch;
    }}
    .gradient-text {{
      background: linear-gradient(135deg, #101f25 0%, #1c85a7 50%, #13a07c 100%);
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
    }}
    .hero-copy {{
      max-width: 46ch;
      color: var(--muted);
      font-size: 1.02rem;
      line-height: 1.75;
      margin-bottom: 18px;
    }}
    .hero-actions {{
      display: flex;
      flex-wrap: wrap;
      gap: 12px;
      margin: 0 0 18px;
    }}
    .hero-btn {{
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      min-height: 46px;
      padding: 0 16px;
      border-radius: 999px;
      font-weight: 800;
      font-size: 0.92rem;
      border: 1px solid rgba(20,35,31,0.08);
      background: rgba(255,255,255,0.88);
      color: var(--ink);
      box-shadow: 0 10px 24px rgba(20,35,31,0.06);
    }}
    .hero-btn.primary {{
      background: linear-gradient(135deg, var(--accent), #64d1ae);
      color: #fff;
      border-color: transparent;
    }}
    .hero-tags {{
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
    }}
    .hero-tag {{
      display: inline-flex;
      align-items: center;
      padding: 8px 12px;
      border-radius: 999px;
      background: rgba(255,255,255,0.8);
      border: 1px solid rgba(20,35,31,0.08);
      color: var(--ink);
      font-size: 0.78rem;
      font-weight: 700;
    }}
    .hero-side {{
      padding: 28px;
      display: grid;
      gap: 16px;
      align-content: start;
    }}
    .section-kicker {{
      color: var(--muted);
      font-size: 0.8rem;
      letter-spacing: 0.08em;
      text-transform: uppercase;
    }}
    .price-card {{
      display: flex;
      justify-content: space-between;
      gap: 16px;
      padding: 14px 16px;
      border-radius: 18px;
      background: rgba(255,255,255,0.84);
      border: 1px solid rgba(29,52,64,0.08);
    }}
    .price-card strong {{ display: block; font-size: 0.98rem; }}
    .price-card span {{ color: var(--muted); font-size: 0.8rem; }}
    .price-value {{ text-align: right; font-weight: 800; }}
    .chip {{
      display: inline-flex;
      margin-top: 5px;
      padding: 4px 9px;
      border-radius: 999px;
      font-size: 0.72rem;
      font-weight: 700;
    }}
    .chip.up {{ background: #ecfaf4; color: var(--accent-deep); }}
    .chip.down {{ background: #fff1e8; color: #b96b1f; }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(12, 1fr);
      gap: 18px;
    }}
    .stats {{
      grid-column: 1 / -1;
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 18px;
    }}
    .stat {{
      padding: 22px;
      border-radius: 22px;
      background: rgba(255,255,255,0.78);
      border: 1px solid var(--line);
      box-shadow: 0 16px 40px rgba(20,35,31,0.06);
    }}
    .stat-top {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 12px;
    }}
    .stat-name {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }}
    .stat-icon {{
      width: 40px;
      height: 40px;
      display: grid;
      place-items: center;
      border-radius: 14px;
      background: var(--accent-soft);
      color: var(--accent-deep);
      font-size: 1rem;
    }}
    .stat-number {{
      font-size: clamp(1.55rem, 2vw, 2rem);
      font-weight: 800;
      letter-spacing: -0.04em;
    }}
    .stat-copy {{
      margin-top: 8px;
      color: var(--muted);
      font-size: 0.9rem;
      line-height: 1.55;
    }}
    .content-card,
    .table-card {{
      padding: 24px;
      grid-column: 1 / -1;
    }}
    .chart-shell {{
      position: relative;
      height: 280px;
      padding-top: 6px;
    }}
    .donut-shell {{
      position: relative;
      height: 250px;
      max-width: 260px;
      margin: 0 auto 8px;
    }}
    canvas {{
      max-width: 100%;
    }}
    .wide {{ grid-column: span 8; }}
    .narrow {{ grid-column: span 4; }}
    .tab-panel {{
      display: none;
      grid-column: 1 / -1;
    }}
    .tab-panel.active {{
      display: contents;
    }}
    .section-title {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 12px;
      margin-bottom: 18px;
    }}
    .section-title h2 {{
      margin: 0;
      font-size: 1.15rem;
      letter-spacing: -0.03em;
    }}
    .section-title span {{
      color: var(--muted);
      font-size: 0.88rem;
    }}
    .substats {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px;
      margin-bottom: 20px;
    }}
    .substat {{
      padding: 18px 20px;
      border-radius: 20px;
      background: rgba(255,255,255,0.84);
      border: 1px solid rgba(20,35,31,0.08);
    }}
    .eyeline {{
      color: var(--muted);
      font-size: 0.76rem;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }}
    .big {{
      margin-top: 10px;
      font-size: 1.7rem;
      font-weight: 800;
      letter-spacing: -0.04em;
    }}
    .mini-copy {{
      margin-top: 6px;
      color: var(--muted);
      font-size: 0.88rem;
    }}
    .progress {{
      height: 14px;
      border-radius: 999px;
      background: #edf1ef;
      overflow: hidden;
    }}
    .progress > div {{
      height: 100%;
      width: {:.4}%;
      background: linear-gradient(90deg, var(--accent), #46c9a0);
    }}
    .progress-legend {{
      display: flex;
      justify-content: space-between;
      gap: 12px;
      color: var(--muted);
      font-size: 0.86rem;
    }}
    .supply-meta {{
      display: grid;
      gap: 16px;
    }}
    .badge {{
      display: inline-flex;
      align-items: center;
      gap: 6px;
      padding: 5px 10px;
      border-radius: 999px;
      font-size: 0.74rem;
      font-weight: 700;
    }}
    .badge.ghost {{
      margin-left: 8px;
      background: rgba(19,156,117,0.12);
      color: var(--accent-deep);
    }}
    .badge.demo {{
      background: rgba(29,52,64,0.08);
      color: var(--ink);
    }}
    .roadmap {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px;
    }}
    .roadmap-item {{
      padding: 20px;
      border-radius: 22px;
      background: linear-gradient(180deg, rgba(255,255,255,0.94), rgba(247,250,249,0.96));
      border: 1px solid rgba(20,35,31,0.08);
    }}
    .roadmap-item strong {{
      display: block;
      margin-bottom: 8px;
      font-size: 1rem;
    }}
    .roadmap-item p {{
      margin: 0;
      color: var(--muted);
      font-size: 0.92rem;
      line-height: 1.6;
    }}
    .buy-grid {{
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px;
    }}
    .buy-card {{
      padding: 20px;
      border-radius: 22px;
      background: linear-gradient(180deg, rgba(255,255,255,0.96), rgba(240,251,246,0.9));
      border: 1px solid rgba(19,156,117,0.14);
    }}
    .buy-card.soft {{
      background: rgba(255,255,255,0.82);
      border-color: rgba(20,35,31,0.08);
    }}
    .buy-top {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 14px;
    }}
    .buy-icon {{
      width: 44px;
      height: 44px;
      display: grid;
      place-items: center;
      border-radius: 14px;
      background: #fff;
      border: 1px solid rgba(20,35,31,0.08);
      font-size: 1.15rem;
      box-shadow: 0 10px 20px rgba(20,35,31,0.06);
    }}
    .buy-card h3 {{
      margin: 0 0 8px;
      font-size: 1.02rem;
      letter-spacing: -0.02em;
    }}
    .buy-card p {{
      margin: 0;
      color: var(--muted);
      line-height: 1.65;
      font-size: 0.92rem;
    }}
    .wallet-strip {{
      grid-column: 1 / -1;
      padding: 24px;
    }}
    .wallet-grid {{
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 14px;
    }}
    .wallet-card {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 12px;
      padding: 18px;
      border-radius: 20px;
      background: rgba(255,255,255,0.88);
      border: 1px solid rgba(20,35,31,0.08);
      box-shadow: 0 12px 24px rgba(20,35,31,0.05);
    }}
    .wallet-meta {{
      display: flex;
      align-items: center;
      gap: 12px;
      min-width: 0;
    }}
    .wallet-icon {{
      width: 40px;
      height: 40px;
      display: grid;
      place-items: center;
      border-radius: 12px;
      background: #fff;
      border: 1px solid rgba(20,35,31,0.08);
      color: var(--ink);
      font-weight: 900;
      box-shadow: 0 8px 18px rgba(20,35,31,0.06);
    }}
    .wallet-meta strong {{
      display: block;
      font-size: 0.98rem;
      margin-bottom: 3px;
    }}
    .wallet-meta span {{
      display: block;
      color: var(--muted);
      font-size: 0.8rem;
    }}
    .wallet-arrow {{
      color: var(--accent-deep);
      font-weight: 900;
      font-size: 1rem;
    }}
    .faq {{
      display: grid;
      gap: 14px;
    }}
    .faq details {{
      padding: 16px 18px;
      border-radius: 18px;
      border: 1px solid rgba(20,35,31,0.08);
      background: rgba(255,255,255,0.82);
    }}
    .faq summary {{
      cursor: pointer;
      font-weight: 700;
    }}
    .faq p {{
      margin: 12px 0 0;
      color: var(--muted);
      line-height: 1.65;
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
    }}
    th, td {{
      padding: 14px 10px;
      text-align: left;
      border-bottom: 1px solid rgba(20,35,31,0.08);
    }}
    th {{
      color: var(--muted);
      font-size: 0.78rem;
      text-transform: uppercase;
      letter-spacing: 0.08em;
    }}
    tr:last-child td {{ border-bottom: none; }}
    .endpoint-list {{
      display: grid;
      gap: 12px;
    }}
    .endpoint {{
      padding: 14px 16px;
      border-radius: 18px;
      background: #f8fcfb;
      border: 1px solid rgba(19,156,117,0.14);
      font-family: "Consolas", "Courier New", monospace;
      color: var(--accent-deep);
      font-size: 0.9rem;
    }}
    .faucet-box {{
      margin-top: 18px;
      padding-top: 18px;
      border-top: 1px solid rgba(20,35,31,0.08);
    }}
    .faucet-box p {{
      margin: 8px 0 14px;
      color: var(--muted);
      line-height: 1.55;
      font-size: 0.9rem;
    }}
    .faucet-form {{
      display: grid;
      gap: 10px;
    }}
    .faucet-form input {{
      width: 100%;
      min-height: 46px;
      padding: 0 14px;
      border-radius: 14px;
      border: 1px solid rgba(20,35,31,0.12);
      background: rgba(255,255,255,0.92);
      color: var(--text);
      font-size: 0.95rem;
      outline: none;
    }}
    .faucet-form input:focus {{
      border-color: rgba(19,156,117,0.48);
      box-shadow: 0 0 0 4px rgba(19,156,117,0.08);
    }}
    .faucet-btn {{
      width: 100%;
    }}
    .faucet-message {{
      margin-top: 10px;
      min-height: 22px;
      color: var(--muted);
      font-size: 0.85rem;
      line-height: 1.5;
    }}
    .faucet-message.success {{
      color: var(--accent-deep);
    }}
    .faucet-message.error {{
      color: #b42318;
    }}
    .footer {{
      display: grid;
      grid-template-columns: 1.2fr 0.8fr;
      gap: 16px;
      margin-top: 24px;
      padding: 24px 22px;
      border-top: 1px solid rgba(20,35,31,0.08);
      color: var(--muted);
      font-size: 0.9rem;
      background: rgba(255,255,255,0.48);
      border-radius: 22px;
    }}
    .footer strong {{
      display: block;
      color: var(--text);
      font-size: 1rem;
      margin-bottom: 6px;
    }}
    .footer-links {{
      display: flex;
      gap: 16px;
      flex-wrap: wrap;
      align-items: flex-start;
      justify-content: flex-end;
    }}
    .footer-links a {{
      color: var(--accent-deep);
      font-weight: 700;
    }}
    @media (max-width: 1120px) {{
      .hero,
      .stats,
      .substats,
      .roadmap {{
        grid-template-columns: 1fr 1fr;
      }}
      .wide,
      .narrow {{
        grid-column: 1 / -1;
      }}
    }}
    @media (max-width: 760px) {{
      .shell {{ width: min(100vw - 18px, 100%); }}
      .topbar {{ flex-direction: column; align-items: flex-start; border-radius: 28px; }}
      .hero,
      .stats,
      .substats,
      .roadmap {{
        grid-template-columns: 1fr;
      }}
      .hero-main,
      .hero-side,
      .content-card,
      .table-card {{
        padding: 20px;
      }}
      .wallet-grid {{
        grid-template-columns: 1fr 1fr;
      }}
      .buy-grid {{
        grid-template-columns: 1fr;
      }}
      .chart-shell {{
        height: 220px;
      }}
      .donut-shell {{
        height: 210px;
      }}
      .footer {{
        grid-template-columns: 1fr;
      }}
      .footer-links {{
        justify-content: flex-start;
      }}
      .wallet-grid {{
        grid-template-columns: 1fr;
      }}
    }}
  </style>
</head>
<body>
  <div class="shell">
    <div class="topbar">
      <div class="brand">
        <div class="brand-mark">GC</div>
        <div>
          <div class="brand-title">GhostCoin <span class="live-pill">Live</span></div>
          <div class="brand-sub">Privacy blockchain</div>
        </div>
      </div>
      <div class="topnav">{}</div>
    </div>

    <section class="hero">
      <div class="panel hero-main">
        <div class="eyebrow">Privacy chain - GHST mainnet</div>
        <h1>The <span class="gradient-text">GhostCoin</span> Explorer</h1>
        <div class="hero-copy">Live stats for the GHST network.</div>
        <div class="hero-actions">
          <a class="hero-btn primary" href="https://github.com/GungTortueninja224/-Ghostcoin/releases">Download wallet</a>
          <a class="hero-btn" href="https://ghostcoin-production.up.railway.app/api/stats">Open API</a>
          <a class="hero-btn" href="https://github.com/GungTortueninja224/-Ghostcoin">GitHub</a>
        </div>
        <div class="hero-tags">
          <span class="hero-tag">Stealth addresses</span>
          <span class="hero-tag">Ring signatures</span>
          <span class="hero-tag">zk-SNARKs</span>
        </div>
      </div>
      {}
    </section>

    <section class="grid">
      <div class="stats">
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Block height</div><div class="stat-icon">B</div></div>
          <div class="stat-number">#{}</div>
          <div class="stat-copy">Latest known public height on this node.</div>
        </article>
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Circulating supply</div><div class="stat-icon">S</div></div>
          <div class="stat-number">{} GHST</div>
          <div class="stat-copy">Issued on-chain so far out of a fixed {} GHST cap.</div>
        </article>
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Mempool</div><div class="stat-icon">M</div></div>
          <div class="stat-number">{} tx</div>
          <div class="stat-copy">Transactions currently waiting for confirmation.</div>
        </article>
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Difficulty</div><div class="stat-icon">D</div></div>
          <div class="stat-number">{}</div>
          <div class="stat-copy">Current SHA-256 mining target difficulty.</div>
        </article>
      </div>

      {}

      {}
    </section>

    <footer class="footer">
      <div><strong>GhostCoin (GHST)</strong>One explorer page with cleaner sections, live stats, and a calmer layout inspired by your newer references.</div>
      <div class="footer-links">
        <a href="/api/stats">API Stats</a>
        <a href="/api/mempool">Mempool API</a>
        <a href="https://github.com/GungTortueninja224/-Ghostcoin">GitHub</a>
      </div>
    </footer>
  </div>

  <script>
    const navLinks = Array.from(document.querySelectorAll('.nav-link'));
    const tabPanels = Array.from(document.querySelectorAll('.tab-panel'));

    function activateTab(key) {{
      navLinks.forEach(link => {{
        link.classList.toggle('active', link.dataset.tab === key);
      }});
      tabPanels.forEach(panel => {{
        panel.classList.toggle('active', panel.dataset.panel === key);
      }});
    }}

    navLinks.forEach(link => {{
      link.addEventListener('click', (event) => {{
        event.preventDefault();
        activateTab(link.dataset.tab);
      }});
    }});

    const faucetForm = document.getElementById('faucetForm');
    const faucetAddress = document.getElementById('faucetAddress');
    const faucetMessage = document.getElementById('faucetMessage');

    if (faucetForm && faucetAddress && faucetMessage) {{
      faucetForm.addEventListener('submit', async (event) => {{
        event.preventDefault();
        const address = faucetAddress.value.trim();
        if (!address) {{
          faucetMessage.textContent = 'Enter a GhostCoin address first.';
          faucetMessage.className = 'faucet-message error';
          return;
        }}

        faucetMessage.textContent = 'Submitting faucet request...';
        faucetMessage.className = 'faucet-message';

        try {{
          const res = await fetch('/api/faucet', {{
            method: 'POST',
            headers: {{ 'Content-Type': 'application/json' }},
            body: JSON.stringify({{ address }})
          }});
          const data = await res.json();
          faucetMessage.textContent = data.message || 'Unknown faucet response.';
          faucetMessage.className = 'faucet-message ' + (data.success ? 'success' : 'error');
          if (data.success) {{
            faucetAddress.value = '';
          }}
        }} catch (error) {{
          faucetMessage.textContent = 'Faucet request failed. Please try again.';
          faucetMessage.className = 'faucet-message error';
        }}
      }});
    }}

    async function fetchPrices() {{
      try {{
        const res = await fetch('https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,binancecoin&vs_currencies=usd&include_24hr_change=true');
        const data = await res.json();

        function paint(key, priceId, changeId) {{
          const item = data[key];
          const change = item.usd_24h_change.toFixed(2);
          const priceNode = document.getElementById(priceId);
          const chip = document.getElementById(changeId);
          if (!priceNode || !chip) return;
          priceNode.textContent = '$' + item.usd.toLocaleString();
          chip.textContent = (change > 0 ? '+' : '') + change + '%';
          chip.className = 'chip ' + (change > 0 ? 'up' : 'down');
        }}

        paint('bitcoin', 'btcPrice', 'btcChange');
        paint('ethereum', 'ethPrice', 'ethChange');
        paint('binancecoin', 'bnbPrice', 'bnbChange');
        const lastUpdate = document.getElementById('lastUpdate');
        if (lastUpdate) lastUpdate.textContent = new Date().toLocaleTimeString();
      }} catch (e) {{
        console.log('Price fetch error:', e);
      }}
    }}

    async function fetchPriceHistory() {{
      const canvas = document.getElementById('priceChart');
      if (!canvas) return;
      try {{
        const [btcRes, ethRes] = await Promise.all([
          fetch('https://api.coingecko.com/api/v3/coins/bitcoin/market_chart?vs_currency=usd&days=7&interval=daily'),
          fetch('https://api.coingecko.com/api/v3/coins/ethereum/market_chart?vs_currency=usd&days=7&interval=daily')
        ]);
        const btcData = await btcRes.json();
        const ethData = await ethRes.json();
        const labels = btcData.prices.map(point => {{
          const date = new Date(point[0]);
          return date.toLocaleDateString('en-US', {{ month: 'short', day: 'numeric' }});
        }});

        new Chart(canvas.getContext('2d'), {{
          type: 'line',
          data: {{
            labels,
            datasets: [
              {{
                label: 'BTC',
                data: btcData.prices.map(point => point[1]),
                borderColor: '#1d3440',
                backgroundColor: 'rgba(29,52,64,0.08)',
                tension: 0.35,
                fill: true,
                borderWidth: 2
              }},
              {{
                label: 'ETH',
                data: ethData.prices.map(point => point[1]),
                borderColor: '#139c75',
                backgroundColor: 'rgba(19,156,117,0.10)',
                tension: 0.35,
                fill: true,
                borderWidth: 2
              }}
            ]
          }},
          options: {{
            responsive: true,
            maintainAspectRatio: false,
            interaction: {{ mode: 'index', intersect: false }},
            plugins: {{ legend: {{ labels: {{ color: '#16251f' }} }} }},
            scales: {{
              x: {{ ticks: {{ color: '#62706d' }}, grid: {{ color: 'rgba(20,35,31,0.08)' }} }},
              y: {{
                ticks: {{ color: '#62706d', callback: value => '$' + Number(value).toLocaleString() }},
                grid: {{ color: 'rgba(20,35,31,0.08)' }}
              }}
            }}
          }}
        }});
      }} catch (e) {{
        console.log('Chart error:', e);
      }}
    }}

    function initSupplyChart() {{
      const canvas = document.getElementById('supplyChart');
      if (!canvas) return;
      const minted = {};
      const remaining = {} - minted;
      new Chart(canvas.getContext('2d'), {{
        type: 'doughnut',
        data: {{
          labels: ['Mined', 'Remaining'],
          datasets: [{{
            data: [minted, remaining],
            backgroundColor: ['#139c75', '#dfe8e4'],
            borderColor: ['#0d6b57', '#c9d7d1'],
            borderWidth: 1
          }}]
        }},
        options: {{
          responsive: true,
          plugins: {{ legend: {{ position: 'bottom', labels: {{ color: '#16251f' }} }} }},
          cutout: '72%'
        }}
      }});
    }}

    fetchPrices();
    fetchPriceHistory();
    initSupplyChart();
    setInterval(fetchPrices, 60000);
  </script>
</body>
</html>"##,
        data.supply_pct(),
        render_nav("overview"),
        render_market_sidebar(),
        data.state.block_height,
        data.state.minted_supply,
        data.max_supply(),
        data.pending_count(),
        data.state.difficulty,
        render_wallet_strip(),
        panels,
        data.state.minted_supply,
        data.max_supply()
    )
}

async fn home() -> Html<String> {
    let data = match tokio::task::spawn_blocking(ViewData::load).await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("failed to load explorer view data: {}", e);
            ViewData::load()
        }
    };
    Html(render_single_page(&data))
}

async fn join() -> Html<String> {
    Html(render_join_page())
}

async fn api_stats() -> Json<Value> {
    let (state, mempool) = match tokio::task::spawn_blocking(|| (ChainState::load(), Mempool::load())).await {
        Ok(tuple) => tuple,
        Err(e) => {
            eprintln!("failed to load api stats data: {}", e);
            (ChainState::load(), Mempool::load())
        }
    };
    Json(json!({
        "name": "GhostCoin",
        "symbol": "GHST",
        "block_height": state.block_height,
        "minted_supply": state.minted_supply,
        "max_supply": 50_000_000,
        "block_reward": state.current_reward(),
        "difficulty": state.difficulty,
        "total_tx": state.total_tx_count,
        "total_fees": state.total_fees,
        "mempool_count": mempool.pending_count(),
        "last_hash": state.last_block_hash,
        "status": "online",
        "node_tcp": "168.220.83.3:8001",
        "explorer": "ghostcoin-production.up.railway.app",
    }))
}

async fn api_mempool() -> Json<Value> {
    let mempool = match tokio::task::spawn_blocking(Mempool::load).await {
        Ok(mempool) => mempool,
        Err(e) => {
            eprintln!("failed to load mempool data: {}", e);
            Mempool::load()
        }
    };
    let txs: Vec<Value> = mempool
        .sorted_by_priority()
        .iter()
        .take(20)
        .map(|tx| {
            json!({
                "tx_id": tx.tx_id,
                "amount": tx.amount,
                "fee": tx.fee,
                "fee_rate": tx.fee_rate,
                "priority": tx.priority_label(),
            })
        })
        .collect();

    Json(json!({
        "count": mempool.pending_count(),
        "total_fees": mempool.total_fees(),
        "transactions": txs,
    }))
}

fn claim_ip(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|v| !v.is_empty())
        })
        .unwrap_or("unknown")
        .to_string()
}

async fn api_faucet(
    State(faucet): State<Arc<FaucetState>>,
    headers: HeaderMap,
    Json(body): Json<FaucetRequest>,
) -> Json<FaucetResponse> {
    let address = body.address.trim().to_string();
    let ip = claim_ip(&headers);

    if !crate::wallet::validate_address(&address) {
        return Json(FaucetResponse {
            success: false,
            message: "Invalid address.".to_string(),
            tx_id: None,
            amount: None,
        });
    }

    if !faucet.can_claim(&address) {
        return Json(FaucetResponse {
            success: false,
            message: "Address already claimed in the last 24h.".to_string(),
            tx_id: None,
            amount: None,
        });
    }

    if !faucet.can_claim(&ip) {
        return Json(FaucetResponse {
            success: false,
            message: "IP already claimed in the last 24h.".to_string(),
            tx_id: None,
            amount: None,
        });
    }

    let tx_id = format!("faucet_{:016x}", rand::random::<u64>());
    let tx = PendingTx {
        tx_id: tx_id.clone(),
        sender: FAUCET_ADDRESS.to_string(),
        receiver: address.clone(),
        amount: FAUCET_AMOUNT,
        fee: FAUCET_FEE,
        timestamp: chrono::Utc::now().to_rfc3339(),
        claimed: false,
    };

    if let Err(e) = tokio::task::spawn_blocking(move || broadcast_tx(tx)).await {
        return Json(FaucetResponse {
            success: false,
            message: format!("Node error: {}", e),
            tx_id: None,
            amount: None,
        });
    }
    faucet.record(&address);
    faucet.record(&ip);

    Json(FaucetResponse {
        success: true,
        message: format!("{} GHST sent to {}", FAUCET_AMOUNT, address),
        tx_id: Some(tx_id),
        amount: Some(FAUCET_AMOUNT),
    })
}
async fn mobile_app_handler() -> HttpResponse {
    let html = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/static/app.html"));
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(html)
}
async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "ghostcoin-web",
    }))
}

pub async fn start_web_server_on_port(port: u16) {
    let faucet_state = Arc::new(FaucetState::new());
    let app = Router::new()
        .route("/", get(home))
        .route("/join", get(join))
        .route("/faucet", get(home))
        .route("/health", get(health))
        .route("/blocks", get(home))
        .route("/mempool", get(home))
        .route("/holders", get(home))
        .route("/mining", get(home))
        .route("/buy", get(home))
        .route("/tokenomics", get(home))
        .route("/roadmap", get(home))
        .route("/faq", get(home))
        .route("/api", get(home))
        .route("/api/stats", get(api_stats))
        .route("/api/mempool", get(api_mempool))
        .route("/api/faucet", post(api_faucet))
        .with_state(faucet_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Web server demarre sur port {}", port);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("failed to bind web server on {}: {}", addr, e);
            return;
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("web server stopped with error on {}: {}", addr, e);
    }
}

pub async fn start_web_server() {
    let port = crate::config::web_port();
    start_web_server_on_port(port).await;
}
