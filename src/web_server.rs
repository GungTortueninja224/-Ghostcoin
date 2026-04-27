use axum::{routing::get, Router, Json, response::Html};
use serde_json::{json, Value};
use std::net::SocketAddr;
use crate::chain_state::ChainState;
use crate::mempool::Mempool;

async fn home() -> Html<String> {
    let state   = ChainState::load();
    let mempool = Mempool::load();

    Html(format!(r#"<!DOCTYPE html>
<html>
<head>
    <title>👻 GhostCoin Explorer</title>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        * {{ margin:0; padding:0; box-sizing:border-box; }}
        body {{ background:#0a0a0a; color:#e0e0e0; font-family:'Courier New',monospace; }}
        .header {{ background:linear-gradient(135deg,#1a1a2e,#16213e); padding:30px; text-align:center; border-bottom:2px solid #7c3aed; }}
        .logo {{ font-size:3em; }}
        .title {{ font-size:2em; color:#7c3aed; font-weight:bold; }}
        .subtitle {{ color:#888; margin-top:5px; }}
        .container {{ max-width:1200px; margin:30px auto; padding:0 20px; }}
        .grid {{ display:grid; grid-template-columns:repeat(auto-fit,minmax(200px,1fr)); gap:20px; margin-bottom:30px; }}
        .card {{ background:#1a1a2e; border:1px solid #7c3aed; border-radius:12px; padding:25px; text-align:center; }}
        .card-icon {{ font-size:2em; margin-bottom:10px; }}
        .card-label {{ color:#888; font-size:0.9em; }}
        .card-value {{ color:#7c3aed; font-size:1.5em; font-weight:bold; }}
        .section {{ background:#1a1a2e; border:1px solid #333; border-radius:12px; padding:25px; margin-bottom:20px; }}
        .section-title {{ color:#7c3aed; font-size:1.2em; font-weight:bold; margin-bottom:15px; padding-bottom:10px; border-bottom:1px solid #333; }}
        .stat-row {{ display:flex; justify-content:space-between; padding:8px 0; border-bottom:1px solid #222; }}
        .stat-label {{ color:#888; }}
        .badge {{ background:#7c3aed; color:white; padding:3px 10px; border-radius:20px; font-size:0.8em; }}
        .online {{ color:#22c55e; }}
        .footer {{ text-align:center; padding:30px; color:#444; border-top:1px solid #222; margin-top:40px; }}
    </style>
    <script>setTimeout(()=>location.reload(),30000);</script>
</head>
<body>
<div class="header">
    <div class="logo">👻</div>
    <div class="title">GhostCoin (GHST)</div>
    <div class="subtitle">Privacy Blockchain — Monero/Zcash Level</div>
</div>
<div class="container">
    <div class="grid">
        <div class="card"><div class="card-icon">📦</div><div class="card-label">Block Height</div><div class="card-value">#{}</div></div>
        <div class="card"><div class="card-icon">👻</div><div class="card-label">Supply Miné</div><div class="card-value">{} GHST</div></div>
        <div class="card"><div class="card-icon">💰</div><div class="card-label">Supply Max</div><div class="card-value">50,000,000</div></div>
        <div class="card"><div class="card-icon">⛏️</div><div class="card-label">Block Reward</div><div class="card-value">{} GHST</div></div>
        <div class="card"><div class="card-icon">📝</div><div class="card-label">Mempool TX</div><div class="card-value">{}</div></div>
        <div class="card"><div class="card-icon">🌐</div><div class="card-label">Réseau</div><div class="card-value online">🟢 Online</div></div>
    </div>
    <div class="section">
        <div class="section-title">⛓️ Blockchain Details</div>
        <div class="stat-row"><span class="stat-label">Algorithme</span><span>SHA-256 + zk-SNARKs</span></div>
        <div class="stat-row"><span class="stat-label">Privacy</span><span>Ring Sig + Stealth + CT</span></div>
        <div class="stat-row"><span class="stat-label">Difficulté</span><span>{}</span></div>
        <div class="stat-row"><span class="stat-label">Total TX</span><span>{}</span></div>
        <div class="stat-row"><span class="stat-label">Total Fees</span><span>{} GHST</span></div>
        <div class="stat-row"><span class="stat-label">Dernier Hash</span><span style="color:#7c3aed">{}...</span></div>
    </div>
    <div class="section">
        <div class="section-title">🔌 API</div>
        <div class="stat-row"><span class="stat-label">Stats</span><span style="color:#7c3aed">/api/stats</span></div>
        <div class="stat-row"><span class="stat-label">Mempool</span><span style="color:#7c3aed">/api/mempool</span></div>
        <div class="stat-row"><span class="stat-label">Noeud TCP</span><span style="color:#22c55e">shuttle.proxy.rlwy.net:48191</span></div>
    </div>
</div>
<div class="footer">👻 GhostCoin (GHST) — Built with Rust 🦀 | Powered by Railway 🚂</div>
</body>
</html>"#,
        state.block_height,
        state.minted_supply,
        state.current_reward(),
        mempool.pending_count(),
        state.difficulty,
        state.total_tx_count,
        state.total_fees,
        &state.last_block_hash[..16.min(state.last_block_hash.len())],
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