use axum::{response::Html, routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;

use crate::chain_state::ChainState;
use crate::mempool::Mempool;

struct ViewData {
    state: ChainState,
    mempool: Mempool,
}

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

fn nav_link(active: &str, key: &str, href: &str, label: &str) -> String {
    let class = if active == key {
        "nav-link active"
    } else {
        "nav-link"
    };
    format!("<a class=\"{}\" href=\"{}\">{}</a>", class, href, label)
}

fn render_nav(active: &str) -> String {
    [
        ("overview", "/", "Overview"),
        ("blocks", "/blocks", "Blocks"),
        ("mempool", "/mempool", "Mempool"),
        ("holders", "/holders", "Holders"),
        ("mining", "/mining", "Mining"),
        ("buy", "/buy", "Buy"),
        ("tokenomics", "/tokenomics", "Tokenomics"),
        ("roadmap", "/roadmap", "Roadmap"),
        ("faq", "/faq", "FAQ"),
        ("api", "/api", "API"),
    ]
    .iter()
    .map(|(key, href, label)| nav_link(active, key, href, label))
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
      </aside>
"#
    .to_string()
}

fn render_layout(
    active: &str,
    title: &str,
    kicker: &str,
    lead: &str,
    body: String,
    data: &ViewData,
) -> String {
    let chart_panel = if active == "overview" {
        format!(
            r#"
      <article class="panel content-card wide">
        <div class="section-title">
          <h2>Market context</h2>
          <span>BTC and ETH for macro reference around GHST</span>
        </div>
        <canvas id="priceChart" height="120"></canvas>
      </article>

      <article class="panel content-card narrow">
        <div class="section-title">
          <h2>Supply distribution</h2>
          <span>{:.4}% mined</span>
        </div>
        <canvas id="supplyChart" height="210"></canvas>
        <div class="supply-meta">
          <div class="progress"><div></div></div>
          <div class="progress-legend">
            <span>Mined: {} GHST</span>
            <span>Remaining: {} GHST</span>
          </div>
        </div>
      </article>
"#,
            data.supply_pct(),
            data.state.minted_supply,
            data.remaining_supply()
        )
    } else {
        String::new()
    };

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>GhostCoin Explorer</title>
  <meta name="description" content="GhostCoin public explorer with dedicated pages for blocks, mempool, holders, mining, roadmap, and API.">
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
      width: 48px;
      height: 48px;
      display: grid;
      place-items: center;
      border-radius: 16px;
      border: 1px solid rgba(19,156,117,0.18);
      background: linear-gradient(135deg, var(--accent-soft), #ffffff);
      font-size: 1.5rem;
    }}
    .brand-title {{
      font-size: 1.25rem;
      font-weight: 800;
      letter-spacing: -0.03em;
    }}
    .brand-sub {{
      color: var(--muted);
      font-size: 0.9rem;
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
      font-size: clamp(2.5rem, 4vw, 4.8rem);
      line-height: 0.95;
      letter-spacing: -0.06em;
    }}
    .gradient-text {{
      background: linear-gradient(135deg, #101f25 0%, #1c85a7 50%, #13a07c 100%);
      -webkit-background-clip: text;
      background-clip: text;
      color: transparent;
    }}
    .hero-copy {{
      max-width: 58ch;
      color: var(--muted);
      font-size: 1rem;
      line-height: 1.7;
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
    .wide {{ grid-column: span 8; }}
    .narrow {{ grid-column: span 4; }}
    .content-card,
    .table-card {{
      padding: 24px;
      grid-column: 1 / -1;
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
    .footer {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 16px;
      margin-top: 24px;
      padding: 18px 22px;
      color: var(--muted);
      font-size: 0.9rem;
    }}
    .footer-links {{
      display: flex;
      gap: 16px;
      flex-wrap: wrap;
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
      .footer {{
        flex-direction: column;
        align-items: flex-start;
      }}
    }}
  </style>
</head>
<body>
  <div class="shell">
    <div class="topbar">
      <div class="brand">
        <div class="brand-mark">&#128123;</div>
        <div>
          <div class="brand-title">GhostCoin <span class="live-pill">Live</span></div>
          <div class="brand-sub">Public GHST explorer on Railway</div>
        </div>
      </div>
      <div class="topnav">{}</div>
    </div>

    <section class="hero">
      <div class="panel hero-main">
        <div class="eyebrow">{}</div>
        <h1><span class="gradient-text">{}</span></h1>
        <div class="hero-copy">{}</div>
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
      <div><strong>GhostCoin (GHST)</strong><br>Dedicated explorer pages for every menu section.</div>
      <div class="footer-links">
        <a href="/api/stats">API Stats</a>
        <a href="/api/mempool">Mempool API</a>
      </div>
    </footer>
  </div>

  <script>
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
        render_nav(active),
        kicker,
        title,
        lead,
        render_market_sidebar(),
        data.state.block_height,
        data.state.minted_supply,
        data.max_supply(),
        data.pending_count(),
        data.state.difficulty,
        chart_panel,
        body,
        data.state.minted_supply,
        data.max_supply()
    )
}

fn overview_body(data: &ViewData) -> String {
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
"#,
        data.state.current_reward(),
        data.state.total_fees,
        data.last_hash_short()
    )
}

fn blocks_body(data: &ViewData) -> String {
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
            <div class="eyeline">Total tx</div>
            <div class="big">{}</div>
            <div class="mini-copy">Confirmed transactions across the chain.</div>
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

fn mempool_body(data: &ViewData) -> String {
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

fn holders_body(data: &ViewData) -> String {
    let _ = data;
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

fn mining_body(data: &ViewData) -> String {
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

fn buy_body() -> String {
    r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Buy GHST</h2>
          <span>Current acquisition flow for early network users</span>
        </div>
        <table>
          <tr><th>Method</th><th>Details</th></tr>
          <tr><td>Mine locally</td><td>Run the CLI wallet, choose mining, and secure fresh GHST directly from the chain.</td></tr>
          <tr><td>P2P transfer</td><td>Receive GHST from another wallet once wallet-to-wallet transfers are active in your session.</td></tr>
          <tr><td>Exchange listing</td><td>Planned for a later phase. This page will switch from guide mode to live market routing once listings exist.</td></tr>
        </table>
      </article>
"#
    .to_string()
}

fn tokenomics_body(data: &ViewData) -> String {
    format!(
        r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>Tokenomics</h2>
          <span>Total transactions: {}</span>
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

fn roadmap_body() -> String {
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

fn faq_body() -> String {
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
            <p>Yes. The mempool page reads the current pending queue from the running node and shows real pending transactions when they exist.</p>
          </details>
          <details>
            <summary>Are holders and buy pages fully live?</summary>
            <p>Not yet. Those pages are intentionally marked as demo or guide content until the backing indexer and liquidity sources are ready.</p>
          </details>
        </div>
      </article>
"#
    .to_string()
}

fn api_body() -> String {
    r#"
      <article class="panel table-card">
        <div class="section-title">
          <h2>API and endpoints</h2>
          <span>Use these for wallets, nodes, or dashboards</span>
        </div>
        <div class="endpoint-list">
          <div class="endpoint">TCP -> shuttle.proxy.rlwy.net:48191</div>
          <div class="endpoint">HTTP -> ghostcoin-production.up.railway.app</div>
          <div class="endpoint">API -> ghostcoin-production.up.railway.app/api/stats</div>
          <div class="endpoint">Mempool API -> ghostcoin-production.up.railway.app/api/mempool</div>
        </div>
      </article>
"#
    .to_string()
}

async fn overview_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "overview",
        "GhostCoin Explorer",
        "Privacy chain - GHST mainnet",
        "A brighter public dashboard for GhostCoin. Follow chain growth, inspect mempool activity, review mining economics, and keep the public network endpoints within reach.",
        overview_body(&data),
        &data,
    ))
}

async fn blocks_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "blocks",
        "Blocks",
        "Chain history",
        "Browse the latest block window for the public GhostCoin node without sharing space with the rest of the explorer.",
        blocks_body(&data),
        &data,
    ))
}

async fn mempool_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "mempool",
        "Mempool",
        "Pending transactions",
        "Track the live queue of transactions waiting for confirmation, with fee and priority data in one dedicated page.",
        mempool_body(&data),
        &data,
    ))
}

async fn holders_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "holders",
        "Top Holders",
        "Holder distribution",
        "A dedicated page for holder ranking and concentration visuals, with clear demo labeling until the full index lands.",
        holders_body(&data),
        &data,
    ))
}

async fn mining_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "mining",
        "Mining",
        "Chain economics",
        "Focus on subsidy, difficulty, and halving progress without mixing it into the rest of the explorer flow.",
        mining_body(&data),
        &data,
    ))
}

async fn buy_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "buy",
        "Buy GHST",
        "Acquisition guide",
        "A separate guide page for obtaining GHST today and where exchange links will live once listings arrive.",
        buy_body(),
        &data,
    ))
}

async fn tokenomics_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "tokenomics",
        "Tokenomics",
        "Supply model",
        "GhostCoin supply, reward, issuance, and consensus rules in a dedicated tokenomics page.",
        tokenomics_body(&data),
        &data,
    ))
}

async fn roadmap_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "roadmap",
        "Roadmap",
        "Next milestones",
        "A standalone roadmap page to keep the project direction visible without cluttering the main explorer.",
        roadmap_body(),
        &data,
    ))
}

async fn faq_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "faq",
        "FAQ",
        "Common answers",
        "Quick answers for miners, holders, and node operators, separated into a dedicated FAQ page.",
        faq_body(),
        &data,
    ))
}

async fn api_page() -> Html<String> {
    let data = ViewData::load();
    Html(render_layout(
        "api",
        "API",
        "Integration endpoints",
        "Everything needed to connect dashboards, wallets, and tooling to the public GhostCoin service.",
        api_body(),
        &data,
    ))
}

async fn api_stats() -> Json<Value> {
    let state = ChainState::load();
    let mempool = Mempool::load();
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
        "node_tcp": "shuttle.proxy.rlwy.net:48191",
        "explorer": "ghostcoin-production.up.railway.app",
    }))
}

async fn api_mempool() -> Json<Value> {
    let mempool = Mempool::load();
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

pub async fn start_web_server_on_port(port: u16) {
    let app = Router::new()
        .route("/", get(overview_page))
        .route("/blocks", get(blocks_page))
        .route("/mempool", get(mempool_page))
        .route("/holders", get(holders_page))
        .route("/mining", get(mining_page))
        .route("/buy", get(buy_page))
        .route("/tokenomics", get(tokenomics_page))
        .route("/roadmap", get(roadmap_page))
        .route("/faq", get(faq_page))
        .route("/api", get(api_page))
        .route("/api/stats", get(api_stats))
        .route("/api/mempool", get(api_mempool));

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("Web server demarre sur port {}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

pub async fn start_web_server() {
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8001".to_string())
        .parse::<u16>()
        .unwrap_or(8001);
    start_web_server_on_port(port).await;
}
