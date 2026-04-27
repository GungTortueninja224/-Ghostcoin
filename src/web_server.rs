use axum::{routing::get, Router, Json, response::Html};
use serde_json::{json, Value};
use std::net::SocketAddr;
use crate::chain_state::ChainState;
use crate::mempool::Mempool;

async fn home() -> Html<String> {
    let state   = ChainState::load();
    let mempool = Mempool::load();

    Html(format!(r#"<!DOCTYPE html>
<html lang="fr">
<head>
    <title>👻 GhostCoin (GHST) — Explorer</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        :root {{
            --purple: #7c3aed;
            --purple-light: #a855f7;
            --green: #22c55e;
            --red: #ef4444;
            --bg: #050510;
            --card: #0d0d1a;
            --border: #1a1a3e;
            --text: #e2e8f0;
            --muted: #64748b;
        }}
        * {{ margin:0; padding:0; box-sizing:border-box; }}
        body {{
            background: var(--bg);
            color: var(--text);
            font-family: 'Inter', -apple-system, sans-serif;
            min-height: 100vh;
        }}

        /* HEADER */
        .header {{
            background: linear-gradient(180deg, #0d0d2e 0%, #050510 100%);
            border-bottom: 1px solid var(--border);
            padding: 0 40px;
            display: flex;
            align-items: center;
            justify-content: space-between;
            height: 70px;
            position: sticky;
            top: 0;
            z-index: 100;
            backdrop-filter: blur(10px);
        }}
        .logo-section {{
            display: flex;
            align-items: center;
            gap: 12px;
        }}
        .logo-icon {{ font-size: 2em; }}
        .logo-text {{
            font-size: 1.3em;
            font-weight: 700;
            background: linear-gradient(135deg, var(--purple-light), #60a5fa);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .badge-live {{
            background: var(--green);
            color: white;
            padding: 3px 10px;
            border-radius: 20px;
            font-size: 0.75em;
            font-weight: 600;
            animation: pulse 2s infinite;
        }}
        @keyframes pulse {{
            0%, 100% {{ opacity: 1; }}
            50% {{ opacity: 0.6; }}
        }}
        .nav-links {{ display: flex; gap: 20px; }}
        .nav-link {{
            color: var(--muted);
            text-decoration: none;
            font-size: 0.9em;
            transition: color 0.2s;
        }}
        .nav-link:hover {{ color: var(--purple-light); }}

        /* PRICE BANNER */
        .price-banner {{
            background: linear-gradient(135deg, #0d0d2e, #1a0a3e);
            border-bottom: 1px solid var(--border);
            padding: 12px 40px;
            display: flex;
            align-items: center;
            gap: 40px;
            overflow-x: auto;
        }}
        .price-item {{ display: flex; align-items: center; gap: 10px; white-space: nowrap; }}
        .price-symbol {{ color: var(--muted); font-size: 0.85em; }}
        .price-value {{ font-weight: 700; font-size: 1em; }}
        .price-change {{ font-size: 0.8em; padding: 2px 8px; border-radius: 10px; }}
        .price-change.up {{ background: rgba(34,197,94,0.15); color: var(--green); }}
        .price-change.down {{ background: rgba(239,68,68,0.15); color: var(--red); }}

        /* MAIN */
        .container {{ max-width: 1400px; margin: 0 auto; padding: 30px 40px; }}

        /* HERO STATS */
        .hero {{ margin-bottom: 30px; }}
        .hero-title {{
            font-size: 2em;
            font-weight: 800;
            margin-bottom: 5px;
            background: linear-gradient(135deg, white, var(--purple-light));
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
        }}
        .hero-subtitle {{ color: var(--muted); margin-bottom: 25px; }}

        /* STATS GRID */
        .stats-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
            gap: 15px;
            margin-bottom: 30px;
        }}
        .stat-card {{
            background: var(--card);
            border: 1px solid var(--border);
            border-radius: 16px;
            padding: 20px;
            transition: border-color 0.2s, transform 0.2s;
        }}
        .stat-card:hover {{
            border-color: var(--purple);
            transform: translateY(-2px);
        }}
        .stat-icon {{ font-size: 1.5em; margin-bottom: 8px; }}
        .stat-label {{ color: var(--muted); font-size: 0.8em; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 5px; }}
        .stat-value {{ font-size: 1.4em; font-weight: 700; color: white; }}
        .stat-sub {{ color: var(--muted); font-size: 0.8em; margin-top: 3px; }}

        /* CHARTS */
        .charts-grid {{
            display: grid;
            grid-template-columns: 2fr 1fr;
            gap: 20px;
            margin-bottom: 30px;
        }}
        .chart-card {{
            background: var(--card);
            border: 1px solid var(--border);
            border-radius: 16px;
            padding: 25px;
        }}
        .chart-title {{
            font-size: 1em;
            font-weight: 600;
            margin-bottom: 20px;
            color: var(--purple-light);
            display: flex;
            align-items: center;
            gap: 8px;
        }}

        /* SUPPLY BAR */
        .supply-bar-container {{ margin: 10px 0; }}
        .supply-bar-bg {{
            background: var(--border);
            border-radius: 10px;
            height: 12px;
            overflow: hidden;
            margin: 10px 0;
        }}
        .supply-bar-fill {{
            background: linear-gradient(90deg, var(--purple), var(--purple-light));
            height: 100%;
            border-radius: 10px;
            transition: width 1s ease;
        }}
        .supply-labels {{
            display: flex;
            justify-content: space-between;
            font-size: 0.8em;
            color: var(--muted);
        }}

        /* PRIVACY BADGES */
        .privacy-section {{
            background: var(--card);
            border: 1px solid var(--border);
            border-radius: 16px;
            padding: 25px;
            margin-bottom: 20px;
        }}
        .privacy-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 15px;
            margin-top: 15px;
        }}
        .privacy-item {{
            background: rgba(124,58,237,0.1);
            border: 1px solid rgba(124,58,237,0.3);
            border-radius: 12px;
            padding: 15px;
        }}
        .privacy-item-icon {{ font-size: 1.5em; margin-bottom: 8px; }}
        .privacy-item-title {{ font-weight: 600; font-size: 0.9em; margin-bottom: 4px; }}
        .privacy-item-desc {{ color: var(--muted); font-size: 0.8em; }}

        /* TABLE */
        .table-card {{
            background: var(--card);
            border: 1px solid var(--border);
            border-radius: 16px;
            padding: 25px;
            margin-bottom: 20px;
        }}
        .table-title {{
            font-size: 1em;
            font-weight: 600;
            margin-bottom: 20px;
            color: var(--purple-light);
        }}
        table {{ width: 100%; border-collapse: collapse; }}
        th {{ color: var(--muted); font-size: 0.8em; text-transform: uppercase; padding: 10px; text-align: left; border-bottom: 1px solid var(--border); }}
        td {{ padding: 12px 10px; border-bottom: 1px solid rgba(255,255,255,0.05); font-size: 0.9em; }}
        tr:hover td {{ background: rgba(124,58,237,0.05); }}

        /* NODE INFO */
        .node-card {{
            background: linear-gradient(135deg, rgba(124,58,237,0.1), rgba(96,165,250,0.1));
            border: 1px solid var(--purple);
            border-radius: 16px;
            padding: 25px;
            margin-bottom: 20px;
        }}
        .node-address {{
            font-family: monospace;
            background: rgba(0,0,0,0.3);
            padding: 10px 15px;
            border-radius: 8px;
            color: var(--green);
            font-size: 0.9em;
            margin-top: 10px;
        }}

        /* FOOTER */
        .footer {{
            text-align: center;
            padding: 40px;
            color: var(--muted);
            border-top: 1px solid var(--border);
            font-size: 0.85em;
        }}
        .footer-links {{ display: flex; justify-content: center; gap: 20px; margin-top: 10px; }}
        .footer-link {{ color: var(--purple-light); text-decoration: none; }}

        /* RESPONSIVE */
        @media (max-width: 768px) {{
            .charts-grid {{ grid-template-columns: 1fr; }}
            .header {{ padding: 0 15px; }}
            .container {{ padding: 20px 15px; }}
            .price-banner {{ padding: 10px 15px; }}
        }}
    </style>
</head>
<body>

<!-- HEADER -->
<div class="header">
    <div class="logo-section">
        <span class="logo-icon">👻</span>
        <span class="logo-text">GhostCoin</span>
        <span class="badge-live">● LIVE</span>
    </div>
    <div class="nav-links">
        <a href="/api/stats" class="nav-link">API Stats</a>
        <a href="/api/mempool" class="nav-link">Mempool</a>
    </div>
</div>

<!-- PRICE BANNER (live via JS) -->
<div class="price-banner" id="priceBanner">
    <div class="price-item">
        <span class="price-symbol">👻 GHST</span>
        <span class="price-value" id="ghstPrice">$0.0100</span>
        <span class="price-change up">Mainnet</span>
    </div>
    <div class="price-item">
        <span class="price-symbol">₿ BTC</span>
        <span class="price-value" id="btcPrice">Loading...</span>
        <span class="price-change up" id="btcChange">...</span>
    </div>
    <div class="price-item">
        <span class="price-symbol">Ξ ETH</span>
        <span class="price-value" id="ethPrice">Loading...</span>
        <span class="price-change up" id="ethChange">...</span>
    </div>
    <div class="price-item">
        <span class="price-symbol">🔶 BNB</span>
        <span class="price-value" id="bnbPrice">Loading...</span>
        <span class="price-change up" id="bnbChange">...</span>
    </div>
    <div class="price-item" style="margin-left:auto">
        <span class="price-symbol">🕐 Last update:</span>
        <span class="price-value" id="lastUpdate" style="font-size:0.85em">--</span>
    </div>
</div>

<div class="container">

    <div class="hero">
        <div class="hero-title">👻 GhostCoin Block Explorer</div>
        <div class="hero-subtitle">Privacy Blockchain — Monero/Zcash Level • Rust 🦀 • SHA-256 + zk-SNARKs</div>
    </div>

    <!-- STATS GRID -->
    <div class="stats-grid">
        <div class="stat-card">
            <div class="stat-icon">📦</div>
            <div class="stat-label">Block Height</div>
            <div class="stat-value">#{}</div>
            <div class="stat-sub">Blocs minés</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">👻</div>
            <div class="stat-label">Supply Circulant</div>
            <div class="stat-value">{} GHST</div>
            <div class="stat-sub">sur 50,000,000 max</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">⛏️</div>
            <div class="stat-label">Block Reward</div>
            <div class="stat-value">{} GHST</div>
            <div class="stat-sub">par bloc miné</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">📝</div>
            <div class="stat-label">Mempool</div>
            <div class="stat-value">{} TX</div>
            <div class="stat-sub">en attente</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">⚡</div>
            <div class="stat-label">Difficulté</div>
            <div class="stat-value">{}</div>
            <div class="stat-sub">ajustable</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">💸</div>
            <div class="stat-label">Total Fees</div>
            <div class="stat-value">{} GHST</div>
            <div class="stat-sub">collectés</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">🔄</div>
            <div class="stat-label">Total TX</div>
            <div class="stat-value">{}</div>
            <div class="stat-sub">transactions</div>
        </div>
        <div class="stat-card">
            <div class="stat-icon">🌐</div>
            <div class="stat-label">Réseau</div>
            <div class="stat-value" style="color:var(--green)">🟢 Online</div>
            <div class="stat-sub">24h/24 Railway</div>
        </div>
    </div>

    <!-- CHARTS -->
    <div class="charts-grid">
        <div class="chart-card">
            <div class="chart-title">📈 Prix BTC/ETH en temps réel (7 jours)</div>
            <canvas id="priceChart" height="120"></canvas>
        </div>
        <div class="chart-card">
            <div class="chart-title">🟣 Supply Distribution</div>
            <canvas id="supplyChart" height="200"></canvas>
            <div class="supply-bar-container" style="margin-top:20px">
                <div class="supply-labels">
                    <span>Miné : {} GHST</span>
                    <span>Restant : {} GHST</span>
                </div>
                <div class="supply-bar-bg">
                    <div class="supply-bar-fill" style="width:{:.2}%"></div>
                </div>
                <div class="supply-labels">
                    <span>0</span>
                    <span>{:.4}% complété</span>
                    <span>50,000,000</span>
                </div>
            </div>
        </div>
    </div>

    <!-- PRIVACY FEATURES -->
    <div class="privacy-section">
        <div class="chart-title">🔒 Privacy Features</div>
        <div class="privacy-grid">
            <div class="privacy-item">
                <div class="privacy-item-icon">👤</div>
                <div class="privacy-item-title">Stealth Addresses</div>
                <div class="privacy-item-desc">Adresse unique par transaction — destinataire intraçable</div>
            </div>
            <div class="privacy-item">
                <div class="privacy-item-icon">💍</div>
                <div class="privacy-item-title">Ring Signatures</div>
                <div class="privacy-item-desc">Signature parmi un groupe — expéditeur anonyme</div>
            </div>
            <div class="privacy-item">
                <div class="privacy-item-icon">🔮</div>
                <div class="privacy-item-title">zk-SNARKs (Groth16)</div>
                <div class="privacy-item-desc">Preuve sans révélation — montants cachés</div>
            </div>
            <div class="privacy-item">
                <div class="privacy-item-icon">🧅</div>
                <div class="privacy-item-title">Dandelion++</div>
                <div class="privacy-item-desc">Protection IP — origine de TX intraçable</div>
            </div>
            <div class="privacy-item">
                <div class="privacy-item-icon">🛡️</div>
                <div class="privacy-item-title">Quantum-Safe</div>
                <div class="privacy-item-desc">CRYSTALS-Dilithium — résistant aux quantums</div>
            </div>
            <div class="privacy-item">
                <div class="privacy-item-icon">🌀</div>
                <div class="privacy-item-title">MimbleWimble</div>
                <div class="privacy-item-desc">Compression blockchain — données minimales</div>
            </div>
        </div>
    </div>

    <!-- TOKENOMICS TABLE -->
    <div class="table-card">
        <div class="table-title">💎 Tokenomics</div>
        <table>
            <tr><th>Paramètre</th><th>Valeur</th></tr>
            <tr><td>Nom</td><td>GhostCoin</td></tr>
            <tr><td>Symbole</td><td><span style="background:var(--purple);padding:2px 8px;border-radius:10px;font-size:0.85em">GHST</span></td></tr>
            <tr><td>Supply Maximum</td><td>50,000,000 GHST</td></tr>
            <tr><td>Distribution</td><td>100% Minage (0% premine)</td></tr>
            <tr><td>Récompense initiale</td><td>65 GHST / bloc</td></tr>
            <tr><td>Halving</td><td>Tous les 210,000 blocs</td></tr>
            <tr><td>Algorithme PoW</td><td>SHA-256</td></tr>
            <tr><td>Courbe cryptographique</td><td>Ristretto255 + BLS12-381</td></tr>
            <tr><td>Langage</td><td>Rust 🦀</td></tr>
            <tr><td>Infrastructure</td><td>Railway ☁️</td></tr>
        </table>
    </div>

    <!-- NODE INFO -->
    <div class="node-card">
        <div class="chart-title">🌐 Connexion au Réseau GhostCoin</div>
        <p style="color:var(--muted);margin:10px 0">Connecte ton wallet ou ton noeud :</p>
        <div class="node-address">TCP → shuttle.proxy.rlwy.net:48191</div>
        <div class="node-address" style="margin-top:8px">HTTP → ghostcoin-production.up.railway.app</div>
        <div class="node-address" style="margin-top:8px">API → ghostcoin-production.up.railway.app/api/stats</div>
    </div>

</div>

<div class="footer">
    <div>👻 <strong>GhostCoin (GHST)</strong> — Privacy Blockchain</div>
    <div style="margin-top:5px">Built with Rust 🦀 | Powered by Railway 🚂 | © 2026 GhostCoin</div>
    <div class="footer-links">
        <a href="/api/stats" class="footer-link">API Stats</a>
        <a href="/api/mempool" class="footer-link">Mempool</a>
    </div>
    <div style="margin-top:10px;font-size:0.8em">Auto-refresh toutes les 30 secondes</div>
</div>

<script>
// ==========================================
// PRIX EN TEMPS RÉEL VIA COINGECKO
// ==========================================
async function fetchPrices() {{
    try {{
        const res = await fetch(
            'https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,binancecoin&vs_currencies=usd&include_24hr_change=true'
        );
        const data = await res.json();

        // BTC
        const btc = data.bitcoin;
        document.getElementById('btcPrice').textContent = '$' + btc.usd.toLocaleString();
        const btcChg = btc.usd_24h_change.toFixed(2);
        const btcEl = document.getElementById('btcChange');
        btcEl.textContent = (btcChg > 0 ? '+' : '') + btcChg + '%';
        btcEl.className = 'price-change ' + (btcChg > 0 ? 'up' : 'down');

        // ETH
        const eth = data.ethereum;
        document.getElementById('ethPrice').textContent = '$' + eth.usd.toLocaleString();
        const ethChg = eth.usd_24h_change.toFixed(2);
        const ethEl = document.getElementById('ethChange');
        ethEl.textContent = (ethChg > 0 ? '+' : '') + ethChg + '%';
        ethEl.className = 'price-change ' + (ethChg > 0 ? 'up' : 'down');

        // BNB
        const bnb = data.binancecoin;
        document.getElementById('bnbPrice').textContent = '$' + bnb.usd.toLocaleString();
        const bnbChg = bnb.usd_24h_change.toFixed(2);
        const bnbEl = document.getElementById('bnbChange');
        bnbEl.textContent = (bnbChg > 0 ? '+' : '') + bnbChg + '%';
        bnbEl.className = 'price-change ' + (bnbChg > 0 ? 'up' : 'down');

        document.getElementById('lastUpdate').textContent =
            new Date().toLocaleTimeString();

        return {{ btc: btc.usd, eth: eth.usd }};
    }} catch(e) {{
        console.log('Price fetch error:', e);
        return null;
    }}
}}

// ==========================================
// GRAPHIQUE PRIX 7 JOURS
// ==========================================
async function fetchPriceHistory() {{
    try {{
        const [btcRes, ethRes] = await Promise.all([
            fetch('https://api.coingecko.com/api/v3/coins/bitcoin/market_chart?vs_currency=usd&days=7&interval=daily'),
            fetch('https://api.coingecko.com/api/v3/coins/ethereum/market_chart?vs_currency=usd&days=7&interval=daily'),
        ]);
        const btcData = await btcRes.json();
        const ethData = await ethRes.json();

        const labels = btcData.prices.map(p => {{
            const d = new Date(p[0]);
            return d.toLocaleDateString('fr-FR', {{month:'short', day:'numeric'}});
        }});

        const ctx = document.getElementById('priceChart').getContext('2d');
        new Chart(ctx, {{
            type: 'line',
            data: {{
                labels,
                datasets: [
                    {{
                        label: 'BTC ($)',
                        data: btcData.prices.map(p => p[1]),
                        borderColor: '#f59e0b',
                        backgroundColor: 'rgba(245,158,11,0.1)',
                        yAxisID: 'y',
                        tension: 0.4,
                        fill: true,
                    }},
                    {{
                        label: 'ETH ($)',
                        data: ethData.prices.map(p => p[1]),
                        borderColor: '#60a5fa',
                        backgroundColor: 'rgba(96,165,250,0.1)',
                        yAxisID: 'y1',
                        tension: 0.4,
                        fill: true,
                    }},
                ]
            }},
            options: {{
                responsive: true,
                interaction: {{ mode: 'index', intersect: false }},
                plugins: {{
                    legend: {{ labels: {{ color: '#e2e8f0' }} }},
                }},
                scales: {{
                    x: {{ ticks: {{ color: '#64748b' }}, grid: {{ color: '#1a1a3e' }} }},
                    y: {{
                        type: 'linear', position: 'left',
                        ticks: {{ color: '#f59e0b', callback: v => '$' + v.toLocaleString() }},
                        grid: {{ color: '#1a1a3e' }},
                    }},
                    y1: {{
                        type: 'linear', position: 'right',
                        ticks: {{ color: '#60a5fa', callback: v => '$' + v.toLocaleString() }},
                        grid: {{ drawOnChartArea: false }},
                    }},
                }},
            }}
        }});
    }} catch(e) {{
        console.log('Chart error:', e);
    }}
}}

// ==========================================
// GRAPHIQUE SUPPLY DONUT
// ==========================================
function initSupplyChart() {{
    const minted  = {};
    const remaining = 50000000 - minted;
    const ctx = document.getElementById('supplyChart').getContext('2d');
    new Chart(ctx, {{
        type: 'doughnut',
        data: {{
            labels: ['Miné', 'Restant'],
            datasets: [{{
                data: [minted, remaining],
                backgroundColor: ['#7c3aed', '#1a1a3e'],
                borderColor: ['#a855f7', '#2d2d5e'],
                borderWidth: 2,
            }}]
        }},
        options: {{
            responsive: true,
            plugins: {{
                legend: {{
                    labels: {{ color: '#e2e8f0' }},
                    position: 'bottom',
                }},
            }},
            cutout: '70%',
        }}
    }});
}}

// ==========================================
// INIT
// ==========================================
fetchPrices();
fetchPriceHistory();
initSupplyChart();

// Refresh prix toutes les 60 secondes
setInterval(fetchPrices, 60000);

// Refresh page toutes les 30 secondes
setTimeout(() => location.reload(), 30000);
</script>

</body>
</html>"#,
        state.block_height,
        state.minted_supply,
        state.current_reward(),
        mempool.pending_count(),
        state.difficulty,
        state.total_fees,
        state.total_tx_count,
        state.minted_supply,
        50_000_000u64.saturating_sub(state.minted_supply),
        state.minted_supply as f64 / 500_000.0,
        state.minted_supply as f64 / 500_000.0,
        state.minted_supply,
    ))
}

async fn api_stats() -> Json<Value> {
    let state   = ChainState::load();
    let mempool = Mempool::load();
    Json(json!({
        "name":          "GhostCoin",
        "symbol":        "GHST",
        "block_height":  state.block_height,
        "minted_supply": state.minted_supply,
        "max_supply":    50_000_000,
        "block_reward":  state.current_reward(),
        "difficulty":    state.difficulty,
        "total_tx":      state.total_tx_count,
        "total_fees":    state.total_fees,
        "mempool_count": mempool.pending_count(),
        "last_hash":     state.last_block_hash,
        "status":        "online",
        "node_tcp":      "shuttle.proxy.rlwy.net:48191",
        "explorer":      "ghostcoin-production.up.railway.app",
    }))
}

async fn api_mempool() -> Json<Value> {
    let mempool = Mempool::load();
    let txs: Vec<Value> = mempool.sorted_by_priority()
        .iter().take(20)
        .map(|tx| json!({
            "tx_id":    tx.tx_id,
            "amount":   tx.amount,
            "fee":      tx.fee,
            "fee_rate": tx.fee_rate,
            "priority": tx.priority_label(),
        }))
        .collect();
    Json(json!({
        "count":        mempool.pending_count(),
        "total_fees":   mempool.total_fees(),
        "transactions": txs,
    }))
}

pub async fn start_web_server() {
    let app = Router::new()
        .route("/",            get(home))
        .route("/api/stats",   get(api_stats))
        .route("/api/mempool", get(api_mempool));

    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8001".to_string())
        .parse::<u16>()
        .unwrap_or(8001);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("🌐 Web server démarré sur port {}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}