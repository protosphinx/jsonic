//! JSON-RPC / REST API surface for a Jsonic node.
//!
//! Exposes the protocol over HTTP so external systems (an ERP, a payments
//! gateway, a block explorer) can submit transactions, drive heartbeats,
//! and query state without linking the protocol crate directly.
//!
//! Routes:
//!   GET  /                        marketing site
//!   GET  /assets/marketing/jsonic-hero-factory-ledger.png
//!   GET  /health                  liveness probe
//!   GET  /daos                    list registered DAOs
//!   POST /daos                    register a DAO (body: DAO JSON)
//!   POST /transactions            submit a signed transaction (body: Transaction JSON)
//!   POST /heartbeats              tick the heartbeat n times (body: {ticks: u64})
//!   GET  /blocks/:height          fetch a main-chain block by height
//!   GET  /metrics                 network metrics from the latest block
//!   GET  /balance/:dao_id         token balance and current side-chain balance sheet
//!   GET  /reputation/:dao_id      PageRank score and trust for a DAO
//!
//! All bodies are JSON. The handler set is stateful behind an
//! `Arc<RwLock<JsonicNode>>` so a single node services concurrent requests.

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Json},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::core::heartbeat::JsonicNode;
use crate::core::reputation::{NodeId, PageRankConfig, compute_pagerank};
use crate::core::types::{BalanceSheet, DAO, DAOId, MainChainBlock, NetworkMetrics, Transaction};

/// Shared, thread-safe handle to a Jsonic node.
pub type SharedNode = Arc<RwLock<JsonicNode>>;

pub fn build_router(node: SharedNode) -> Router {
    Router::new()
        .route("/", get(index))
        .route(
            "/assets/marketing/jsonic-hero-factory-ledger.png",
            get(hero_factory_asset),
        )
        .route("/health", get(health))
        .route("/daos", get(list_daos).post(register_dao))
        .route("/transactions", post(submit_transaction))
        .route("/heartbeats", post(heartbeat))
        .route("/blocks/:height", get(get_block))
        .route("/metrics", get(get_metrics))
        .route("/balance/:dao_id", get(get_balance))
        .route("/reputation/:dao_id", get(get_reputation))
        .with_state(node)
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    height: u64,
    pending: usize,
    tick: u64,
    registered_daos: u64,
    heartbeat_ms: u64,
    total_token_supply: f64,
}

#[derive(Serialize)]
struct RegisterDaoResponse {
    dao_id: DAOId,
}

#[derive(Deserialize)]
pub struct HeartbeatRequest {
    pub ticks: u64,
}

#[derive(Serialize)]
struct HeartbeatResponse {
    ticks_run: u64,
    solstices_fired: u64,
    final_tick: u64,
    pending: usize,
}

#[derive(Serialize)]
struct BalanceResponse {
    dao_id: DAOId,
    token_balance: f64,
    side_chain_height: u64,
    closing_balance: BalanceSheet,
}

#[derive(Serialize)]
struct ReputationResponse {
    dao_id: DAOId,
    pagerank: f64,
    baseline: f64,
    trust: f64,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

const LOGO_MARK: &str = include_str!("../assets/brand/logo-jsconic-mark-color.svg");
const HERO_FACTORY_IMAGE: &[u8] =
    include_bytes!("../assets/marketing/jsonic-hero-factory-ledger.png");

async fn index() -> Html<String> {
    Html(marketing_page())
}

async fn hero_factory_asset() -> impl IntoResponse {
    (
        [
            ("content-type", "image/png"),
            ("cache-control", "public, max-age=31536000, immutable"),
        ],
        HERO_FACTORY_IMAGE,
    )
        .into_response()
}

fn marketing_page() -> String {
    let mut html =
        String::with_capacity(MARKETING_HEAD.len() + LOGO_MARK.len() + MARKETING_TAIL.len());
    html.push_str(MARKETING_HEAD);
    html.push_str(LOGO_MARK);
    html.push_str(MARKETING_TAIL);
    html
}

async fn health(State(node): State<SharedNode>) -> Json<HealthResponse> {
    let guard = node.read().await;
    Json(HealthResponse {
        status: "ok",
        height: guard.main_chain.height(),
        pending: guard.pending_count(),
        tick: guard.tick,
        registered_daos: guard.registry.count(),
        heartbeat_ms: guard.effective_heartbeat_ms(),
        total_token_supply: guard.main_chain.total_token_supply,
    })
}

const MARKETING_HEAD: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Jsonic L1 - Proof of Transaction for Manufacturing</title>
  <meta name="description" content="Jsonic is a Layer 1 protocol that rewards verified manufacturing, sales, and B2B commerce with PageRank-weighted Proof of Transaction.">
  <style>
    :root {
      color-scheme: light;
      --ink: #111418;
      --muted: #5d6467;
      --paper: #fbfaf4;
      --panel: #ffffff;
      --line: #d9d3c3;
      --teal: #0f8d74;
      --amber: #c88d24;
      --violet: #6d55c8;
      --bluewash: #e7f7ff;
      --shadow: 0 22px 70px rgba(36, 42, 39, 0.14);
    }

    * { box-sizing: border-box; }

    html,
    body {
      max-width: 100%;
      overflow-x: hidden;
    }

    body {
      margin: 0;
      color: var(--ink);
      background: var(--paper);
      font-family: "Aptos", "Segoe UI", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
      letter-spacing: 0;
    }

    a { color: inherit; text-decoration: none; }

    .sr-only {
      position: absolute;
      width: 1px;
      height: 1px;
      padding: 0;
      margin: -1px;
      overflow: hidden;
      clip: rect(0, 0, 0, 0);
      white-space: nowrap;
      border: 0;
    }

    .site-header {
      position: sticky;
      top: 0;
      z-index: 20;
      border-bottom: 1px solid rgba(17, 20, 24, 0.12);
      background: rgba(251, 250, 244, 0.94);
      backdrop-filter: blur(18px);
    }

    .nav {
      max-width: 1280px;
      min-height: 74px;
      margin: 0 auto;
      padding: 0 28px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 28px;
    }

    .brand {
      display: inline-flex;
      align-items: center;
      gap: 12px;
      font-size: 19px;
      font-weight: 820;
    }

    .brand svg {
      width: 38px;
      height: 38px;
      display: block;
    }

    .nav-links {
      display: flex;
      align-items: center;
      gap: 28px;
      color: #2a2f32;
      font-size: 15px;
      font-weight: 720;
    }

    .nav-links a:hover { color: var(--teal); }

    .nav-actions {
      display: flex;
      align-items: center;
      gap: 14px;
    }

    .status-pill {
      display: inline-flex;
      align-items: center;
      gap: 9px;
      min-height: 40px;
      padding: 0 14px;
      border: 1px solid var(--line);
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.78);
      font-size: 14px;
      font-weight: 760;
    }

    .status-dot {
      width: 9px;
      height: 9px;
      border-radius: 50%;
      background: var(--teal);
      box-shadow: 0 0 0 5px rgba(15, 141, 116, 0.13);
    }

    .button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 46px;
      padding: 0 20px;
      border: 1px solid var(--ink);
      border-radius: 7px;
      background: var(--ink);
      color: #fff;
      font-size: 15px;
      font-weight: 780;
      white-space: nowrap;
    }

    .button.secondary {
      background: rgba(255, 255, 255, 0.74);
      color: var(--ink);
      border-color: rgba(17, 20, 24, 0.22);
    }

    .hero {
      position: relative;
      min-height: 560px;
      display: grid;
      align-items: stretch;
      background:
        linear-gradient(90deg, rgba(251, 250, 244, 0.98) 0%, rgba(251, 250, 244, 0.88) 44%, rgba(251, 250, 244, 0.16) 80%),
        url("/assets/marketing/jsonic-hero-factory-ledger.png") center/cover;
      border-bottom: 1px solid var(--line);
      overflow: hidden;
    }

    .hero::after {
      content: "";
      position: absolute;
      left: 0;
      right: 0;
      bottom: 0;
      height: 54px;
      background:
        linear-gradient(90deg, rgba(15, 141, 116, 0.18), rgba(200, 141, 36, 0.18), rgba(109, 85, 200, 0.14)),
        linear-gradient(var(--line), var(--line)) top/100% 1px no-repeat;
    }

    .hero-inner {
      position: relative;
      z-index: 1;
      width: 100%;
      max-width: 1280px;
      margin: 0 auto;
      padding: 52px 28px 66px;
      display: grid;
      align-content: center;
      gap: 24px;
    }

    .eyebrow {
      display: inline-flex;
      align-items: center;
      gap: 12px;
      color: var(--teal);
      font-size: 13px;
      font-weight: 860;
      letter-spacing: 0;
      text-transform: uppercase;
    }

    .eyebrow::before {
      content: "";
      width: 42px;
      height: 2px;
      background: var(--teal);
    }

    h1,
    h2,
    h3,
    p {
      overflow-wrap: anywhere;
    }

    h1 {
      max-width: 760px;
      margin: 0;
      font-size: 60px;
      line-height: 0.96;
      font-weight: 880;
    }

    .hero-copy {
      max-width: 660px;
      margin: 0;
      color: #31383b;
      font-size: 20px;
      line-height: 1.55;
    }

    .hero-actions {
      display: flex;
      gap: 12px;
      flex-wrap: wrap;
      align-items: center;
    }

    .signal-strip {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      max-width: 820px;
      margin-top: 8px;
      border: 1px solid rgba(17, 20, 24, 0.16);
      background: rgba(255, 255, 255, 0.72);
      backdrop-filter: blur(10px);
    }

    .signal {
      padding: 14px 18px;
      border-right: 1px solid rgba(17, 20, 24, 0.14);
    }

    .signal:last-child { border-right: 0; }

    .signal strong {
      display: block;
      font-size: 22px;
      line-height: 1.1;
    }

    .signal span {
      display: block;
      margin-top: 7px;
      color: var(--muted);
      font-size: 13px;
      line-height: 1.3;
    }

    section {
      padding: 92px 28px;
      background: var(--paper);
    }

    .section-inner {
      max-width: 1280px;
      margin: 0 auto;
    }

    .section-heading {
      max-width: 780px;
      margin-bottom: 42px;
    }

    h2 {
      margin: 16px 0 0;
      font-size: 56px;
      line-height: 1.04;
      font-weight: 860;
    }

    .section-heading p {
      margin: 22px 0 0;
      color: var(--muted);
      font-size: 21px;
      line-height: 1.62;
    }

    .proof-layout {
      display: grid;
      grid-template-columns: minmax(0, 0.95fr) minmax(420px, 1fr);
      gap: 48px;
      align-items: start;
    }

    .proof-list {
      display: grid;
      gap: 12px;
    }

    .proof-step {
      display: grid;
      grid-template-columns: 64px 1fr;
      gap: 18px;
      padding: 22px;
      border: 1px solid var(--line);
      background: rgba(255, 255, 255, 0.72);
    }

    .proof-step span {
      color: var(--amber);
      font-size: 26px;
      font-weight: 860;
    }

    .proof-step h3 {
      margin: 0 0 8px;
      font-size: 23px;
      line-height: 1.15;
    }

    .proof-step p {
      margin: 0;
      color: var(--muted);
      line-height: 1.55;
    }

    .commerce-section {
      background: #eef8f3;
      border-top: 1px solid var(--line);
      border-bottom: 1px solid var(--line);
    }

    .commerce-grid,
    .path-grid {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 16px;
    }

    .tile {
      min-height: 260px;
      padding: 26px;
      border: 1px solid rgba(17, 20, 24, 0.14);
      border-radius: 8px;
      background: rgba(255, 255, 255, 0.78);
      box-shadow: 0 14px 36px rgba(36, 42, 39, 0.08);
    }

    .tile h3 {
      margin: 0 0 16px;
      font-size: 28px;
      line-height: 1.15;
    }

    .tile p {
      margin: 0;
      color: var(--muted);
      font-size: 17px;
      line-height: 1.58;
    }

    .builders-section {
      background:
        linear-gradient(120deg, rgba(231, 247, 255, 0.8), rgba(251, 250, 244, 0.84)),
        var(--paper);
    }

    .node-section {
      background: #151816;
      color: #f6f0df;
    }

    .node-section .eyebrow {
      color: #7ee0c9;
    }

    .node-section .eyebrow::before {
      background: #7ee0c9;
    }

    .node-section p {
      color: #c7c2b4;
    }

    .node-section .button {
      background: #f6f0df;
      color: #111418;
      border-color: #f6f0df;
    }

    .node-section .button.secondary {
      background: transparent;
      color: #f6f0df;
      border-color: rgba(246, 240, 223, 0.34);
    }

    .node-panel {
      display: grid;
      grid-template-columns: minmax(0, 0.9fr) minmax(430px, 1fr);
      gap: 52px;
      align-items: center;
    }

    .terminal {
      border: 1px solid rgba(246, 240, 223, 0.22);
      border-radius: 8px;
      background: #0b0d0c;
      color: #dff8ee;
      overflow: hidden;
      box-shadow: var(--shadow);
    }

    .terminal-head {
      min-height: 42px;
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 0 16px;
      border-bottom: 1px solid rgba(246, 240, 223, 0.18);
      color: #9eb9ad;
      font-size: 13px;
    }

    .dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: var(--amber);
    }

    .dot:nth-child(2) { background: var(--teal); }
    .dot:nth-child(3) { background: var(--violet); }

    pre {
      margin: 0;
      padding: 24px;
      overflow-x: auto;
      font: 14px/1.8 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    }

    .footer {
      padding: 34px 28px;
      background: #0b0d0c;
      color: #d6d1c3;
    }

    .footer-inner {
      max-width: 1280px;
      margin: 0 auto;
      display: flex;
      justify-content: space-between;
      gap: 24px;
      flex-wrap: wrap;
      font-size: 14px;
    }

    @media (max-width: 980px) {
      .nav-links {
        gap: 18px;
        font-size: 14px;
      }

      .nav-actions {
        display: none;
      }

      .hero {
        min-height: 560px;
        background:
          linear-gradient(rgba(251, 250, 244, 0.9), rgba(251, 250, 244, 0.72)),
          url("/assets/marketing/jsonic-hero-factory-ledger.png") center/cover;
      }

      .hero-inner {
        padding-top: 56px;
      }

      h1 {
        max-width: 680px;
        font-size: 50px;
      }

      h2 {
        font-size: 44px;
      }

      .proof-layout,
      .node-panel {
        grid-template-columns: 1fr;
      }

      .commerce-grid,
      .path-grid,
      .signal-strip {
        grid-template-columns: 1fr;
      }

      .signal {
        border-right: 0;
        border-bottom: 1px solid rgba(17, 20, 24, 0.14);
      }

      .signal:last-child {
        border-bottom: 0;
      }
    }

    @media (max-width: 640px) {
      .nav {
        min-height: 66px;
        padding: 0 18px;
      }

      .nav-links {
        display: none;
      }

      .brand svg {
        width: 34px;
        height: 34px;
      }

      .hero {
        min-height: 560px;
        background:
          linear-gradient(rgba(251, 250, 244, 0.96), rgba(251, 250, 244, 0.74)),
          url("/assets/marketing/jsonic-hero-factory-ledger.png") center top/auto 230px no-repeat,
          var(--paper);
      }

      .hero::after {
        height: 46px;
      }

      .hero-inner {
        padding: 248px 20px 64px;
        gap: 24px;
      }

      h1 {
        max-width: 350px;
        font-size: 42px;
      }

      .hero-copy {
        max-width: 350px;
        font-size: 18px;
      }

      .hero-actions {
        align-items: stretch;
      }

      .button {
        width: 100%;
      }

      .signal-strip {
        display: none;
      }

      section {
        padding: 68px 20px;
      }

      h2 {
        font-size: 34px;
      }

      .section-heading p {
        font-size: 18px;
      }

      .proof-step {
        grid-template-columns: 1fr;
      }

      .tile {
        min-height: auto;
      }
    }
  </style>
</head>
<body>
  <header class="site-header">
    <nav class="nav" aria-label="Main navigation">
      <a class="brand" href="/" aria-label="Jsonic home">
"##;

const MARKETING_TAIL: &str = r##"        <span>Jsonic</span>
      </a>
      <div class="nav-links">
        <a href="#protocol">Protocol</a>
        <a href="#commerce">Commerce</a>
        <a href="#builders">Builders</a>
        <a href="#node">Node</a>
      </div>
      <div class="nav-actions">
        <a class="status-pill" href="/health"><span class="status-dot"></span>live node</a>
      </div>
    </nav>
  </header>

  <main>
    <section class="hero" id="top" aria-labelledby="hero-heading">
      <div class="hero-inner">
        <div class="eyebrow">Jsonic L1</div>
        <h1 id="hero-heading">Manufacturing needs its own settlement layer</h1>
        <p class="hero-copy">
          Jsonic turns signed invoices, payments, and production events into a
          public chain for real commerce. Proof of Transaction rewards the
          businesses that create value, not idle capital or fake volume.
        </p>
        <div class="hero-actions">
          <a class="button" href="#protocol">Read the protocol</a>
          <a class="button secondary" href="/health">Check live node</a>
        </div>
        <div class="signal-strip" aria-label="Jsonic network facts">
          <div class="signal"><strong>75 tests</strong><span>Core engine, API, and asset route covered</span></div>
          <div class="signal"><strong>2-party proof</strong><span>Invoices only count when counterparties match</span></div>
          <div class="signal"><strong>PageRank trust</strong><span>Reputation flows through verified trade</span></div>
        </div>
      </div>
    </section>

    <section id="protocol">
      <div class="section-inner proof-layout">
        <div class="section-heading">
          <div class="eyebrow">Protocol</div>
          <h2>A ledger for signed economic activity</h2>
          <p>
            Jsonic records the facts manufacturers already depend on:
            production, invoices, settlement, and counterparty trust. The chain
            mints around verified relationships instead of isolated assertions.
          </p>
        </div>
        <div class="proof-list">
          <article class="proof-step">
            <span>01</span>
            <div><h3>Register a business DAO</h3><p>Each participant gets a ledger identity, signing key, and side-chain for local state.</p></div>
          </article>
          <article class="proof-step">
            <span>02</span>
            <div><h3>Match both sides of trade</h3><p>Invoices and payments become useful only when counterparties independently sign matching activity.</p></div>
          </article>
          <article class="proof-step">
            <span>03</span>
            <div><h3>Settle by reputation</h3><p>At Solstice, PageRank-weighted commerce determines who earned network rewards.</p></div>
          </article>
        </div>
      </div>
    </section>

    <section class="commerce-section" id="commerce">
      <div class="section-inner">
        <div class="section-heading">
          <div class="eyebrow">Commerce</div>
          <h2>Built for supply chains, not speculation theater</h2>
          <p>
            The most valuable work on Jsonic is useful trade with reputable
            counterparties. Empty circular activity can exist, but it cannot
            build much trust.
          </p>
        </div>
        <div class="commerce-grid">
          <article class="tile">
            <h3>Manufacturers</h3>
            <p>Turn production and sales into verifiable network history without surrendering your operating data to a private marketplace.</p>
          </article>
          <article class="tile">
            <h3>Buyers</h3>
            <p>Carry reputation across suppliers by settling invoices and confirming real trade on a shared ledger.</p>
          </article>
          <article class="tile">
            <h3>Operators</h3>
            <p>Run the reference node, process signed activity, and inspect chain state through a simple RPC surface.</p>
          </article>
        </div>
      </div>
    </section>

    <section class="builders-section" id="builders">
      <div class="section-inner">
        <div class="section-heading">
          <div class="eyebrow">Builders</div>
          <h2>Choose your path into Jsonic</h2>
          <p>
            Start with protocol primitives, integrate the API, or connect agents
            to the live node through the MCP server.
          </p>
        </div>
        <div class="path-grid">
          <article class="tile">
            <h3>Learn the protocol</h3>
            <p>Understand DAOs, side-chains, Proof of Transaction, PageRank reputation, and Solstice minting.</p>
          </article>
          <article class="tile">
            <h3>Build apps</h3>
            <p>Connect ERP systems, payment workflows, agents, and explorers through the JSON-RPC API.</p>
          </article>
          <article class="tile">
            <h3>Run a node</h3>
            <p>Operate the reference server with persistent state, replay protection, and live health checks.</p>
          </article>
        </div>
      </div>
    </section>

    <section class="node-section" id="node">
      <div class="section-inner node-panel">
        <div class="section-heading">
          <div class="eyebrow">Node</div>
          <h2>Proof of Transaction is running now</h2>
          <p>
            Jsonic is early, but the core engine is real: signed transaction
            admission, sequence replay protection, PageRank scoring, side-chain
            snapshots, persistent storage, TypeScript SDKs, and MCP tools.
          </p>
          <div class="hero-actions">
            <a class="button" href="/health">Node health</a>
            <a class="button secondary" href="https://github.com/protosphinx/jsonic">GitHub</a>
          </div>
        </div>
        <div class="terminal" aria-label="Jsonic RPC endpoints">
          <div class="terminal-head"><span class="dot"></span><span class="dot"></span><span class="dot"></span><span>jsonic-rpc</span></div>
          <pre>GET  /health
GET  /daos
POST /daos
POST /transactions
POST /heartbeats
GET  /blocks/:height
GET  /metrics
GET  /balance/:dao_id
GET  /reputation/:dao_id</pre>
        </div>
      </div>
    </section>
  </main>

  <footer class="footer">
    <div class="footer-inner">
      <span>Jsonic L1 - Proof of Transaction for manufacturing economies</span>
      <span>Production, settlement, and reputation on-chain.</span>
    </div>
  </footer>
</body>
</html>
"##;

async fn list_daos(State(node): State<SharedNode>) -> Json<Vec<DAO>> {
    let guard = node.read().await;
    Json(guard.registry.iter().cloned().collect())
}

async fn register_dao(State(node): State<SharedNode>, Json(dao): Json<DAO>) -> impl IntoResponse {
    let id = dao.id.clone();
    let mut guard = node.write().await;
    guard.register_dao(dao);
    (
        StatusCode::CREATED,
        Json(RegisterDaoResponse { dao_id: id }),
    )
}

async fn submit_transaction(
    State(node): State<SharedNode>,
    Json(tx): Json<Transaction>,
) -> impl IntoResponse {
    let mut guard = node.write().await;
    match guard.submit_transaction(tx) {
        Ok(()) => StatusCode::ACCEPTED.into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: err.to_string(),
            }),
        )
            .into_response(),
    }
}

async fn heartbeat(
    State(node): State<SharedNode>,
    Json(req): Json<HeartbeatRequest>,
) -> Json<HeartbeatResponse> {
    let mut guard = node.write().await;
    let mut solstices = 0u64;
    for _ in 0..req.ticks {
        if guard.heartbeat().is_some() {
            solstices += 1;
        }
    }
    Json(HeartbeatResponse {
        ticks_run: req.ticks,
        solstices_fired: solstices,
        final_tick: guard.tick,
        pending: guard.pending_count(),
    })
}

async fn get_block(
    State(node): State<SharedNode>,
    Path(height): Path<u64>,
) -> Result<Json<MainChainBlock>, (StatusCode, Json<ErrorResponse>)> {
    let guard = node.read().await;
    match guard.main_chain.blocks.get(height as usize) {
        Some(block) => Ok(Json(block.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("no block at height {height}"),
            }),
        )),
    }
}

async fn get_metrics(
    State(node): State<SharedNode>,
) -> Result<Json<NetworkMetrics>, (StatusCode, Json<ErrorResponse>)> {
    let guard = node.read().await;
    match guard.main_chain.blocks.last() {
        Some(block) => Ok(Json(block.network_metrics.clone())),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "no main-chain blocks yet; run a heartbeat past a Solstice".to_string(),
            }),
        )),
    }
}

async fn get_balance(
    State(node): State<SharedNode>,
    Path(dao_id): Path<DAOId>,
) -> Result<Json<BalanceResponse>, (StatusCode, Json<ErrorResponse>)> {
    let guard = node.read().await;
    let dao = guard.registry.get(&dao_id).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("DAO {dao_id} is not registered"),
            }),
        )
    })?;
    let chain = guard.side_chains.get(&dao_id).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("side-chain for {dao_id} missing"),
            }),
        )
    })?;
    Ok(Json(BalanceResponse {
        dao_id: dao_id.clone(),
        token_balance: dao.token_balance,
        side_chain_height: chain.height(),
        closing_balance: chain.current_balance(),
    }))
}

async fn get_reputation(
    State(node): State<SharedNode>,
    Path(dao_id): Path<DAOId>,
) -> Json<ReputationResponse> {
    let guard = node.read().await;
    let scores = compute_pagerank(
        &guard.main_chain.reputation_graph,
        &PageRankConfig::default(),
    );
    let key = NodeId::DAO(dao_id.clone());
    let pr = scores.pagerank.get(&key).copied().unwrap_or(0.0);
    let trust = (pr - scores.baseline_rank).max(0.0);
    Json(ReputationResponse {
        dao_id,
        pagerank: pr,
        baseline: scores.baseline_rank,
        trust,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dao::RegisteredDAO;
    use axum::body::{Body, to_bytes};
    use axum::http::{Request, StatusCode};
    use serde_json::{Value, json};
    use tower::ServiceExt;

    fn fresh_node() -> SharedNode {
        let mut node = JsonicNode::new();
        node.solstice_interval = 5;
        Arc::new(RwLock::new(node))
    }

    async fn body_json(resp: axum::response::Response) -> Value {
        let bytes = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&bytes).expect("parse json")
    }

    #[tokio::test]
    async fn health_reports_state() {
        let node = fresh_node();
        let app = build_router(node);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["status"], "ok");
        assert_eq!(v["height"], 0);
        assert_eq!(v["tick"], 0);
        assert_eq!(v["registered_daos"], 0);
        assert_eq!(v["heartbeat_ms"], 60_000);
        assert_eq!(v["total_token_supply"], 0.0);
    }

    #[tokio::test]
    async fn index_serves_marketing_site() {
        let node = fresh_node();
        let app = build_router(node);
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert!(content_type.starts_with("text/html"));
        let bytes = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read body");
        let html = String::from_utf8(bytes.to_vec()).expect("utf8 html");
        assert!(html.contains("Manufacturing needs its own settlement layer"));
        assert!(html.contains("A ledger for signed economic activity"));
        assert!(html.contains("Proof of Transaction"));
        assert!(html.contains("/assets/marketing/jsonic-hero-factory-ledger.png"));
        assert!(html.contains("75 tests"));
        assert!(html.contains("GET  /health"));
        assert!(html.contains("GET  /daos"));
    }

    #[tokio::test]
    async fn marketing_image_serves_png() {
        let node = fresh_node();
        let app = build_router(node);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/marketing/jsonic-hero-factory-ledger.png")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        assert_eq!(content_type, "image/png");
        let bytes = to_bytes(resp.into_body(), usize::MAX)
            .await
            .expect("read png body");
        assert!(bytes.starts_with(b"\x89PNG"));
    }

    #[tokio::test]
    async fn list_daos_reports_registered_identities() {
        let node = fresh_node();
        let app = build_router(node);
        let alice = RegisteredDAO::register("Alice Co", "Tech");
        let bob = RegisteredDAO::register("Bob Inc", "Mfg");

        for dao in [alice.dao.clone(), bob.dao.clone()] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/daos")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&dao).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::CREATED);
        }

        let resp = app
            .oneshot(Request::builder().uri("/daos").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let daos = body_json(resp).await;
        assert_eq!(daos.as_array().unwrap().len(), 2);
        assert!(daos.to_string().contains(&alice.id()[..12]));
        assert!(daos.to_string().contains(&bob.id()[..12]));
    }

    #[tokio::test]
    async fn full_lifecycle_through_rpc() {
        // End-to-end: register two DAOs, submit invoice + payment, run
        // heartbeats past a Solstice, then verify the seller's balance and
        // the freshly-minted block via the RPC surface.
        let node = fresh_node();
        let app = build_router(node);

        let mut alice = RegisteredDAO::register("Alice Co", "Tech");
        let mut bob = RegisteredDAO::register("Bob Inc", "Mfg");
        let alice_id = alice.id().clone();
        let bob_id = bob.id().clone();

        for dao in [alice.dao.clone(), bob.dao.clone()] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/daos")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&dao).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::CREATED);
        }

        let invoice = alice.create_invoice(&bob_id, 25_000.0, "USD", "consulting");
        let invoice_id = invoice.id.clone();
        let payment = bob.create_payment(&alice_id, 25_000.0, "USD", &invoice_id, "paid");

        for tx in [invoice, payment] {
            let resp = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/transactions")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_vec(&tx).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::ACCEPTED);
        }

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/heartbeats")
                    .header("content-type", "application/json")
                    .body(Body::from(json!({"ticks": 5}).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["solstices_fired"], 1);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/blocks/0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let block = body_json(resp).await;
        assert_eq!(block["header"]["index"], 0);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/balance/{}", alice_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bal = body_json(resp).await;
        assert!(bal["token_balance"].as_f64().unwrap() >= 0.0);
        assert_eq!(bal["closing_balance"]["revenue"], 25_000.0);
        assert_eq!(bal["closing_balance"]["accounts_receivable"], 0.0);

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/balance/{}", bob_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bal = body_json(resp).await;
        assert_eq!(bal["closing_balance"]["expenses"], 25_000.0);
        assert_eq!(bal["closing_balance"]["accounts_payable"], 0.0);
    }

    #[tokio::test]
    async fn unknown_dao_balance_is_404() {
        let node = fresh_node();
        let app = build_router(node);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/balance/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn submit_rejects_unknown_dao() {
        let node = fresh_node();
        let app = build_router(node);
        let mut ghost = RegisteredDAO::register("Ghost", "Phantom");
        let other = RegisteredDAO::register("Other", "Real");
        let tx = ghost.create_invoice(other.id(), 100.0, "USD", "haunting fees");
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/transactions")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&tx).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
