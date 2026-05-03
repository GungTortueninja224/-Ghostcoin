use axum::{response::Html, routing::get, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;

use crate::chain_state::ChainState;
use crate::mempool::Mempool;

async fn home() -> Html<String> {
    let state = ChainState::load();
    let mempool = Mempool::load();
    let max_supply = 50_000_000u64;
    let remaining_supply = max_supply.saturating_sub(state.minted_supply);
    let supply_pct = (state.minted_supply as f64 / max_supply as f64) * 100.0;

    Html(format!(
        r##"<!DOCTYPE html>
<html lang="fr">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>GhostCoin Explorer</title>
  <meta name="description" content="GhostCoin public block explorer. GHST supply, mempool, reward, endpoints and live market context.">
  <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
  <style>
    :root {{
      --bg: #f7f6f1;
      --panel: rgba(255,255,255,0.84);
      --panel-strong: #ffffff;
      --line: rgba(18, 38, 31, 0.10);
      --text: #15251f;
      --muted: #60716d;
      --accent: #0f9d74;
      --accent-deep: #0b6b55;
      --accent-soft: #dff5ee;
      --ink: #173540;
      --warn: #c46e1d;
      --shadow: 0 24px 60px rgba(22, 35, 28, 0.10);
      --radius: 24px;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      color: var(--text);
      background:
        radial-gradient(circle at top left, rgba(15,157,116,0.12), transparent 30%),
        radial-gradient(circle at top right, rgba(23,53,64,0.07), transparent 25%),
        linear-gradient(180deg, #fffef9 0%, var(--bg) 100%);
      font-family: "Trebuchet MS", "Gill Sans", sans-serif;
    }}
    a {{ color: inherit; text-decoration: none; }}
    .shell {{
      width: min(1280px, calc(100vw - 28px));
      margin: 0 auto;
      padding: 18px 0 44px;
    }}
    .topbar {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      gap: 16px;
      padding: 18px 22px;
      border-radius: 999px;
      border: 1px solid var(--line);
      background: rgba(255,255,255,0.72);
      backdrop-filter: blur(14px);
      box-shadow: 0 18px 40px rgba(21,37,31,0.06);
      position: sticky;
      top: 12px;
      z-index: 10;
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
      border: 1px solid rgba(15,157,116,0.16);
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
      font-size: 0.92rem;
    }}
    .live-pill {{
      display: inline-flex;
      align-items: center;
      gap: 8px;
      margin-left: 10px;
      padding: 7px 12px;
      border-radius: 999px;
      background: #effaf6;
      color: var(--accent-deep);
      font-size: 0.78rem;
      font-weight: 700;
    }}
    .live-pill::before {{
      content: "";
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--accent);
      box-shadow: 0 0 0 8px rgba(15,157,116,0.12);
    }}
    .topnav {{
      display: flex;
      gap: 10px;
      flex-wrap: wrap;
    }}
    .topnav a {{
      padding: 10px 14px;
      border-radius: 999px;
      color: var(--ink);
      font-size: 0.92rem;
      font-weight: 600;
    }}
    .topnav a:hover {{ background: rgba(15,157,116,0.08); }}
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
      right: -50px;
      bottom: -70px;
      width: 220px;
      height: 220px;
      background: radial-gradient(circle, rgba(15,157,116,0.18), transparent 65%);
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
      font-size: clamp(2.4rem, 4vw, 4.7rem);
      line-height: 0.95;
      letter-spacing: -0.06em;
    }}
    .hero-copy {{
      max-width: 58ch;
      color: var(--muted);
      font-size: 1rem;
      line-height: 1.7;
    }}
    .mini-grid {{
      display: flex;
      flex-wrap: wrap;
      gap: 14px;
      margin-top: 28px;
    }}
    .mini {{
      min-width: 150px;
      padding: 16px 18px;
      border-radius: 18px;
      background: rgba(255,255,255,0.85);
      border: 1px solid rgba(15,157,116,0.12);
    }}
    .mini .label {{
      color: var(--muted);
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.06em;
    }}
    .mini .value {{
      margin-top: 8px;
      font-size: 1.35rem;
      font-weight: 800;
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
      background: rgba(255,255,255,0.82);
      border: 1px solid rgba(23,53,64,0.08);
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
    .chip.down {{ background: #fff0e7; color: var(--warn); }}
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
      background: rgba(255,255,255,0.76);
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
      font-size: 1.05rem;
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
    .content-card {{
      padding: 24px;
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
    .supply-meta {{
      display: grid;
      gap: 16px;
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
    .features {{
      grid-column: 1 / -1;
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px;
    }}
    .feature {{
      padding: 20px;
      border-radius: 22px;
      background: linear-gradient(180deg, rgba(255,255,255,0.92), rgba(239,250,246,0.95));
      border: 1px solid rgba(15,157,116,0.14);
    }}
    .feature strong {{
      display: block;
      margin: 10px 0 6px;
      font-size: 1rem;
    }}
    .feature p {{
      margin: 0;
      color: var(--muted);
      font-size: 0.9rem;
      line-height: 1.6;
    }}
    .table-card {{
      grid-column: 1 / -1;
      padding: 24px;
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
      border: 1px solid rgba(15,157,116,0.14);
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
    .footer-links a {{ color: var(--accent-deep); font-weight: 600; }}
    @media (max-width: 1120px) {{
      .hero, .stats, .features {{ grid-template-columns: 1fr 1fr; }}
      .wide, .narrow {{ grid-column: 1 / -1; }}
    }}
    @media (max-width: 760px) {{
      .shell {{ width: min(100vw - 18px, 100%); }}
      .topbar {{ flex-direction: column; align-items: flex-start; border-radius: 28px; }}
      .hero, .stats, .features {{ grid-template-columns: 1fr; }}
      .hero-main, .hero-side, .content-card, .table-card {{ padding: 20px; }}
      .footer {{ flex-direction: column; align-items: flex-start; }}
    }}
  </style>
</head>
<body>
  <div class="shell">
    <div class="topbar">
      <div class="brand">
        <div class="brand-mark">👻</div>
        <div>
          <div class="brand-title">GhostCoin <span class="live-pill">Live Network</span></div>
          <div class="brand-sub">Public GHST block explorer on Railway</div>
        </div>
      </div>
      <div class="topnav">
        <a href="/api/stats">API Stats</a>
        <a href="/api/mempool">Mempool</a>
        <a href="#tokenomics">Tokenomics</a>
        <a href="#network">Network</a>
      </div>
    </div>

    <section class="hero">
      <div class="panel hero-main">
        <div class="eyebrow">Privacy chain · GHST mainnet</div>
        <h1>GhostCoin Block Explorer</h1>
        <div class="hero-copy">
          A lighter, cleaner public dashboard for GhostCoin. Track the chain, monitor live supply,
          inspect mempool activity, and connect wallets or tooling directly to the public endpoints.
        </div>
        <div class="mini-grid">
          <div class="mini">
            <div class="label">Current height</div>
            <div class="value">#{}</div>
          </div>
          <div class="mini">
            <div class="label">Network status</div>
            <div class="value" style="color:var(--accent-deep)">Online</div>
          </div>
          <div class="mini">
            <div class="label">Block reward</div>
            <div class="value">{} GHST</div>
          </div>
        </div>
      </div>

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
    </section>

    <section class="grid">
      <div class="stats">
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Circulating supply</div><div class="stat-icon">👻</div></div>
          <div class="stat-number">{} GHST</div>
          <div class="stat-copy">Issued on-chain so far out of a fixed {} GHST cap.</div>
        </article>
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Mempool</div><div class="stat-icon">🧾</div></div>
          <div class="stat-number">{} tx</div>
          <div class="stat-copy">Transactions waiting for confirmation in the next block.</div>
        </article>
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Difficulty</div><div class="stat-icon">⚒️</div></div>
          <div class="stat-number">{}</div>
          <div class="stat-copy">Current SHA-256 mining target difficulty.</div>
        </article>
        <article class="stat">
          <div class="stat-top"><div class="stat-name">Total fees</div><div class="stat-icon">💸</div></div>
          <div class="stat-number">{} GHST</div>
          <div class="stat-copy">Fees already secured by accepted blocks.</div>
        </article>
      </div>

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

      <div class="features">
        <article class="feature"><div>👤</div><strong>Stealth addresses</strong><p>Each payment can target a one-time destination, reducing recipient linkability.</p></article>
        <article class="feature"><div>💍</div><strong>Ring signatures</strong><p>Transactions can hide the real signer inside a larger crowd.</p></article>
        <article class="feature"><div>🔮</div><strong>zk-SNARKs</strong><p>Proof systems validate rules while revealing far less raw transaction data.</p></article>
        <article class="feature"><div>🌿</div><strong>Dandelion++</strong><p>Broadcast propagation is shaped to make transaction origin harder to trace.</p></article>
        <article class="feature"><div>🛡️</div><strong>Quantum-safe track</strong><p>The roadmap includes post-quantum ideas for longer-term signature resilience.</p></article>
        <article class="feature"><div>🌊</div><strong>MimbleWimble ideas</strong><p>Compression and minimal on-chain data remain part of the broader design story.</p></article>
      </div>

      <article class="panel table-card" id="tokenomics">
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

      <article class="panel table-card" id="network">
        <div class="section-title">
          <h2>Network endpoints</h2>
          <span>Use these for wallets, nodes, or dashboards</span>
        </div>
        <div class="endpoint-list">
          <div class="endpoint">TCP → shuttle.proxy.rlwy.net:48191</div>
          <div class="endpoint">HTTP → ghostcoin-production.up.railway.app</div>
          <div class="endpoint">API → ghostcoin-production.up.railway.app/api/stats</div>
          <div class="endpoint">Mempool API → ghostcoin-production.up.railway.app/api/mempool</div>
        </div>
      </article>
    </section>

    <footer class="footer">
      <div><strong>GhostCoin (GHST)</strong><br>Public privacy-chain explorer with automatic refresh every 30 seconds.</div>
      <div class="footer-links">
        <a href="/api/stats">API Stats</a>
        <a href="/api/mempool">Mempool</a>
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
          document.getElementById(priceId).textContent = '$' + item.usd.toLocaleString();
          const chip = document.getElementById(changeId);
          chip.textContent = (change > 0 ? '+' : '') + change + '%';
          chip.className = 'chip ' + (change > 0 ? 'up' : 'down');
        }}

        paint('bitcoin', 'btcPrice', 'btcChange');
        paint('ethereum', 'ethPrice', 'ethChange');
        paint('binancecoin', 'bnbPrice', 'bnbChange');
        document.getElementById('lastUpdate').textContent = new Date().toLocaleTimeString();
      }} catch (e) {{
        console.log('Price fetch error:', e);
      }}
    }}

    async function fetchPriceHistory() {{
      try {{
        const [btcRes, ethRes] = await Promise.all([
          fetch('https://api.coingecko.com/api/v3/coins/bitcoin/market_chart?vs_currency=usd&days=7&interval=daily'),
          fetch('https://api.coingecko.com/api/v3/coins/ethereum/market_chart?vs_currency=usd&days=7&interval=daily')
        ]);
        const btcData = await btcRes.json();
        const ethData = await ethRes.json();
        const labels = btcData.prices.map(point => {{
          const date = new Date(point[0]);
          return date.toLocaleDateString('fr-FR', {{ month: 'short', day: 'numeric' }});
        }});

        const ctx = document.getElementById('priceChart').getContext('2d');
        new Chart(ctx, {{
          type: 'line',
          data: {{
            labels,
            datasets: [
              {{
                label: 'BTC',
                data: btcData.prices.map(point => point[1]),
                borderColor: '#173540',
                backgroundColor: 'rgba(23,53,64,0.08)',
                tension: 0.35,
                fill: true,
                borderWidth: 2
              }},
              {{
                label: 'ETH',
                data: ethData.prices.map(point => point[1]),
                borderColor: '#0f9d74',
                backgroundColor: 'rgba(15,157,116,0.10)',
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
            plugins: {{
              legend: {{ labels: {{ color: '#15251f' }} }}
            }},
            scales: {{
              x: {{ ticks: {{ color: '#60716d' }}, grid: {{ color: 'rgba(20,35,31,0.08)' }} }},
              y: {{
                ticks: {{ color: '#60716d', callback: value => '$' + Number(value).toLocaleString() }},
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
      const minted = {};
      const remaining = {} - minted;
      const ctx = document.getElementById('supplyChart').getContext('2d');
      new Chart(ctx, {{
        type: 'doughnut',
        data: {{
          labels: ['Mined', 'Remaining'],
          datasets: [{{
            data: [minted, remaining],
            backgroundColor: ['#0f9d74', '#dfe8e4'],
            borderColor: ['#0b6b55', '#c9d7d1'],
            borderWidth: 1
          }}]
        }},
        options: {{
          responsive: true,
          plugins: {{
            legend: {{ position: 'bottom', labels: {{ color: '#15251f' }} }}
          }},
          cutout: '72%'
        }}
      }});
    }}

    fetchPrices();
    fetchPriceHistory();
    initSupplyChart();
    setInterval(fetchPrices, 60000);
    setTimeout(() => location.reload(), 30000);
  </script>
</body>
</html>"##,
        supply_pct,
        state.block_height,
        state.current_reward(),
        state.minted_supply,
        max_supply,
        mempool.pending_count(),
        state.difficulty,
        state.total_fees,
        supply_pct,
        state.minted_supply,
        remaining_supply,
        state.total_tx_count,
        max_supply,
        state.current_reward(),
        state.minted_supply,
        max_supply
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
        .route("/", get(home))
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
