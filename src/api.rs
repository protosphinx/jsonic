//! JSON-RPC / REST API surface for a Jsonic node.
//!
//! Exposes the protocol over HTTP so external systems (an ERP, a payments
//! gateway, a block explorer) can submit transactions, drive heartbeats,
//! and query state without linking the protocol crate directly.
//!
//! Routes:
//!   GET  /                        marketing site
//!   GET  /assets/marketing/jsonic-hero-commons.png
//!   GET  /health                  liveness probe
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
            "/assets/marketing/jsonic-hero-commons.png",
            get(hero_commons_asset),
        )
        .route("/health", get(health))
        .route("/daos", post(register_dao))
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
const HERO_COMMONS_IMAGE: &[u8] = include_bytes!("../assets/marketing/jsonic-hero-commons.png");

async fn index() -> Html<String> {
    Html(marketing_page())
}

async fn hero_commons_asset() -> impl IntoResponse {
    (
        [
            ("content-type", "image/png"),
            ("cache-control", "public, max-age=31536000, immutable"),
        ],
        HERO_COMMONS_IMAGE,
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
      --ink: #141821;
      --muted: #5f6878;
      --line: #d9dee7;
      --paper: #f7f8fb;
      --white: #ffffff;
      --blue: #2477b8;
      --green: #169b6b;
      --gold: #d99a21;
      --violet: #6b4ba1;
      --shadow: 0 18px 50px rgba(22, 30, 46, 0.12);
    }

    * { box-sizing: border-box; }

    body {
      margin: 0;
      font-family: "Aptos", "Segoe UI", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
      color: var(--ink);
      background: var(--paper);
      letter-spacing: 0;
    }

    a { color: inherit; text-decoration: none; }

    .site-header {
      position: sticky;
      top: 0;
      z-index: 20;
      border-bottom: 1px solid rgba(217, 222, 231, 0.82);
      background: rgba(255, 255, 255, 0.92);
      backdrop-filter: blur(14px);
    }

    .nav {
      max-width: 1180px;
      margin: 0 auto;
      min-height: 72px;
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 28px;
      padding: 0 24px;
    }

    .brand {
      display: flex;
      align-items: center;
      gap: 12px;
      font-weight: 800;
      font-size: 18px;
    }

    .brand svg {
      width: 42px;
      height: 42px;
      display: block;
    }

    .nav-links {
      display: flex;
      align-items: center;
      gap: 22px;
      color: var(--muted);
      font-size: 14px;
      font-weight: 650;
    }

    .nav-links a:hover { color: var(--ink); }

    .button {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      min-height: 42px;
      padding: 0 18px;
      border-radius: 7px;
      border: 1px solid var(--ink);
      background: var(--ink);
      color: var(--white);
      font-weight: 760;
      font-size: 14px;
      white-space: nowrap;
    }

    .button.secondary {
      background: transparent;
      color: var(--ink);
      border-color: var(--line);
    }

    .hero {
      position: relative;
      min-height: calc(90vh - 72px);
      padding: 0;
      border-top: 0;
      overflow: hidden;
      background:
        linear-gradient(rgba(247, 248, 251, 0.68), rgba(247, 248, 251, 0.9)),
        url("https://images.unsplash.com/photo-1581093458791-9f3c3900df7b?auto=format&fit=crop&w=2200&q=80") center/cover;
    }

    .hero-inner {
      max-width: 1180px;
      margin: 0 auto;
      min-height: calc(90vh - 72px);
      display: grid;
      grid-template-columns: minmax(0, 1.02fr) minmax(340px, 0.78fr);
      align-items: center;
      gap: 56px;
      padding: 56px 24px 110px;
    }

    .eyebrow {
      display: inline-flex;
      align-items: center;
      gap: 10px;
      color: var(--green);
      font-weight: 800;
      font-size: 13px;
      text-transform: uppercase;
    }

    .eyebrow::before {
      content: "";
      width: 36px;
      height: 2px;
      background: var(--green);
    }

    h1 {
      margin: 22px 0 20px;
      max-width: 820px;
      font-family: Georgia, "Times New Roman", serif;
      font-size: 88px;
      line-height: 0.96;
      font-weight: 700;
    }

    .hero-copy {
      max-width: 650px;
      color: #303848;
      font-size: 20px;
      line-height: 1.6;
      margin: 0 0 34px;
    }

    .hero-actions {
      display: flex;
      align-items: center;
      gap: 12px;
      flex-wrap: wrap;
    }

    .network-panel {
      background: rgba(255, 255, 255, 0.88);
      border: 1px solid rgba(217, 222, 231, 0.9);
      border-radius: 8px;
      box-shadow: var(--shadow);
      padding: 24px;
    }

    .panel-title {
      display: flex;
      justify-content: space-between;
      gap: 16px;
      margin-bottom: 22px;
      font-size: 13px;
      color: var(--muted);
      font-weight: 760;
      text-transform: uppercase;
    }

    .flow {
      position: relative;
      min-height: 320px;
      border: 1px solid var(--line);
      border-radius: 8px;
      background:
        linear-gradient(90deg, rgba(36, 119, 184, 0.08) 1px, transparent 1px),
        linear-gradient(rgba(22, 155, 107, 0.08) 1px, transparent 1px),
        #ffffff;
      background-size: 42px 42px;
      overflow: hidden;
    }

    .flow::before,
    .flow::after {
      content: "";
      position: absolute;
      left: 14%;
      right: 14%;
      height: 2px;
      background: var(--line);
      transform-origin: left center;
    }

    .flow::before { top: 40%; transform: rotate(11deg); }
    .flow::after { top: 60%; transform: rotate(-13deg); }

    .node {
      position: absolute;
      width: 112px;
      min-height: 78px;
      padding: 12px;
      border-radius: 8px;
      background: #fff;
      border: 1px solid var(--line);
      box-shadow: 0 10px 24px rgba(22, 30, 46, 0.1);
      font-size: 13px;
      font-weight: 760;
    }

    .node span {
      display: block;
      margin-top: 8px;
      color: var(--muted);
      font-size: 12px;
      font-weight: 600;
      line-height: 1.3;
    }

    .n1 { top: 28px; left: 28px; border-top: 4px solid var(--gold); }
    .n2 { top: 126px; right: 34px; border-top: 4px solid var(--blue); }
    .n3 { bottom: 32px; left: 72px; border-top: 4px solid var(--green); }

    .metric-strip {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 12px;
      margin-top: 14px;
    }

    .metric {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fff;
      padding: 14px;
    }

    .metric strong {
      display: block;
      font-size: 24px;
    }

    .metric span {
      display: block;
      color: var(--muted);
      font-size: 12px;
      margin-top: 4px;
    }

    section {
      padding: 86px 24px;
      border-top: 1px solid var(--line);
      background: var(--white);
    }

    section.alt { background: #f0f4f8; }

    .section-inner {
      max-width: 1180px;
      margin: 0 auto;
    }

    .section-heading {
      max-width: 780px;
      margin-bottom: 38px;
    }

    h2 {
      margin: 0 0 14px;
      font-family: Georgia, "Times New Roman", serif;
      font-size: 50px;
      line-height: 1.05;
      font-weight: 700;
    }

    .section-heading p {
      margin: 0;
      color: var(--muted);
      font-size: 18px;
      line-height: 1.65;
    }

    .grid {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 18px;
    }

    .card {
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fff;
      padding: 24px;
      min-height: 210px;
    }

    .card .kicker {
      color: var(--blue);
      font-size: 13px;
      font-weight: 820;
      text-transform: uppercase;
      margin-bottom: 18px;
    }

    .card h3 {
      margin: 0 0 12px;
      font-size: 22px;
      line-height: 1.2;
    }

    .card p {
      color: var(--muted);
      margin: 0;
      line-height: 1.62;
    }

    .architecture {
      display: grid;
      grid-template-columns: 0.9fr 1.1fr;
      gap: 32px;
      align-items: start;
    }

    .steps {
      display: grid;
      gap: 12px;
    }

    .step {
      display: grid;
      grid-template-columns: 42px 1fr;
      gap: 16px;
      padding: 18px;
      border: 1px solid var(--line);
      border-radius: 8px;
      background: #fff;
    }

    .step-number {
      width: 42px;
      height: 42px;
      border-radius: 50%;
      background: var(--ink);
      color: #fff;
      display: grid;
      place-items: center;
      font-weight: 820;
    }

    .step h3 {
      margin: 0 0 6px;
      font-size: 18px;
    }

    .step p {
      margin: 0;
      color: var(--muted);
      line-height: 1.55;
    }

    .terminal {
      border-radius: 8px;
      overflow: hidden;
      border: 1px solid #202737;
      background: #10141d;
      color: #d7e0ef;
      box-shadow: var(--shadow);
    }

    .terminal-head {
      height: 42px;
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 0 16px;
      border-bottom: 1px solid #252d3d;
      color: #8d99aa;
      font-size: 13px;
    }

    .dot {
      width: 10px;
      height: 10px;
      border-radius: 50%;
      background: var(--gold);
    }

    .dot:nth-child(2) { background: var(--green); }
    .dot:nth-child(3) { background: var(--blue); }

    pre {
      margin: 0;
      padding: 22px;
      overflow-x: auto;
      font: 14px/1.7 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    }

    .footer {
      padding: 34px 24px;
      background: var(--ink);
      color: #cfd6e2;
    }

    .footer-inner {
      max-width: 1180px;
      margin: 0 auto;
      display: flex;
      justify-content: space-between;
      gap: 24px;
      flex-wrap: wrap;
      font-size: 14px;
    }

    @media (max-width: 860px) {
      .nav { min-height: 64px; }
      .nav-links { display: none; }
      .hero-inner {
        grid-template-columns: 1fr;
        min-height: auto;
        padding: 48px 20px 74px;
      }
      h1 { font-size: 54px; }
      h2 { font-size: 38px; }
      .hero-copy { font-size: 18px; }
      .network-panel { padding: 16px; }
      .grid,
      .architecture,
      .metric-strip {
        grid-template-columns: 1fr;
      }
      section { padding: 64px 20px; }
      .flow { min-height: 280px; }
      .node { width: 104px; }
    }

    @media (max-width: 520px) {
      .hero-inner {
        max-width: 390px;
        margin: 0;
      }
      h1 {
        max-width: 310px;
        font-size: 40px;
      }
      h2 { font-size: 32px; }
      .hero-copy {
        max-width: 350px;
        font-size: 17px;
      }
      .button { width: 100%; }
      .hero-actions { align-items: stretch; }
      .network-panel {
        width: 100%;
        max-width: 350px;
        overflow: hidden;
      }
      .flow { min-height: 238px; }
      .n1 { top: 28px; left: 28px; }
      .n2 { top: 96px; right: 16px; }
      .n3 { bottom: 20px; left: 48px; }
      .metric-strip {
        grid-template-columns: repeat(3, minmax(0, 1fr));
      }
      .metric { padding: 10px; }
      .metric strong { font-size: 20px; }
      .panel-title {
        flex-direction: column;
        gap: 4px;
      }
    }

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

    html,
    body {
      max-width: 100%;
      overflow-x: hidden;
    }

    body {
      background: #fff;
      color: #111;
      font-family: "Aptos", "Segoe UI", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
    }

    .site-header {
      position: relative;
      border-top: 1px solid #222;
      border-bottom: 1px solid #d7d7d7;
      background: #fff;
      backdrop-filter: none;
    }

    .nav {
      max-width: 1800px;
      min-height: 88px;
      padding: 0 36px;
      gap: 32px;
    }

    .brand {
      width: 42px;
      min-width: 42px;
      gap: 0;
    }

    .brand svg {
      width: 34px;
      height: 34px;
    }

    .nav-links {
      flex: 1;
      gap: 40px;
      color: #111;
      font-size: 22px;
      font-weight: 500;
    }

    .nav-actions {
      display: flex;
      align-items: center;
      gap: 28px;
      color: #111;
      font-size: 20px;
      font-weight: 520;
    }

    .search-pill {
      display: flex;
      align-items: center;
      gap: 10px;
      min-width: 205px;
      min-height: 54px;
      padding: 0 16px;
      border: 1px solid #8d8d8d;
      border-radius: 4px;
      color: #111;
      background: #fff;
    }

    .search-pill .key {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 28px;
      height: 28px;
      border: 1px solid #a7a7a7;
      border-radius: 4px;
      color: #777;
      font-size: 15px;
      line-height: 1;
    }

    .nav-icon {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      white-space: nowrap;
    }

    .hero {
      min-height: auto;
      border-top: 0;
      background: #fff;
      overflow: visible;
    }

    .hero-art {
      max-width: 1888px;
      margin: 0 auto;
      padding: 0 28px;
      background: #fff;
    }

    .hero-art img {
      display: block;
      width: 100%;
      height: 590px;
      object-fit: cover;
      object-position: center;
      border: 0;
    }

    .hero-copy-block {
      max-width: 960px;
      margin: 88px auto 30px;
      padding: 0 24px;
      text-align: center;
    }

    .eyebrow {
      display: block;
      color: #5f2eea;
      font-size: 18px;
      font-weight: 760;
      line-height: 1.2;
      text-transform: uppercase;
    }

    .eyebrow::before { content: none; }

    h1 {
      max-width: 860px;
      margin: 34px auto 28px;
      font-family: "Aptos", "Segoe UI", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
      font-size: 76px;
      line-height: 0.98;
      font-weight: 860;
      letter-spacing: 0;
    }

    .hero-copy {
      max-width: 830px;
      margin: 0 auto 34px;
      color: #5d5d5d;
      font-size: 29px;
      line-height: 1.45;
    }

    .hero-actions {
      justify-content: center;
      gap: 14px;
    }

    .button {
      min-height: 48px;
      padding: 0 24px;
      border-radius: 999px;
      border-color: #111;
      background: #111;
      color: #fff;
      font-size: 16px;
      font-weight: 760;
    }

    .button.secondary {
      background: #fff;
      color: #111;
      border-color: #bcbcbc;
    }

    section {
      padding: 96px 44px;
      border-top: 0;
      background: #fff;
    }

    .section-inner {
      max-width: 1800px;
    }

    .editorial {
      display: grid;
      grid-template-columns: minmax(0, 1fr) 470px;
      gap: 88px;
      align-items: start;
      min-height: 560px;
    }

    .editorial-copy {
      max-width: 840px;
    }

    h2 {
      margin: 26px 0 46px;
      font-family: "Aptos", "Segoe UI", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
      font-size: 72px;
      line-height: 1.06;
      font-weight: 860;
      letter-spacing: 0;
    }

    .editorial-copy p,
    .split-copy p {
      margin: 0;
      color: #616161;
      font-size: 30px;
      line-height: 1.55;
    }

    .stats {
      display: grid;
      gap: 44px;
      border-left: 1px solid #d3d3d3;
      padding: 60px 0 0 90px;
    }

    .stat {
      display: grid;
      grid-template-columns: 40px 1fr;
      gap: 20px;
      align-items: start;
    }

    .stat-icon {
      color: #6c6c6c;
      font-size: 32px;
      line-height: 1;
    }

    .stat strong {
      display: block;
      color: #111;
      font-size: 46px;
      line-height: 1;
      font-weight: 840;
    }

    .stat span:last-child {
      display: block;
      margin-top: 12px;
      color: #606060;
      font-size: 20px;
    }

    .split {
      display: grid;
      grid-template-columns: minmax(0, 0.95fr) minmax(420px, 1fr);
      gap: 72px;
      align-items: center;
    }

    .split-copy {
      max-width: 780px;
    }

    .split-image {
      align-self: end;
      overflow: hidden;
      border-radius: 48px 48px 0 0;
      border: 1px solid #d9d9f2;
      background: #eff7ff;
    }

    .split-image img {
      display: block;
      width: 100%;
      height: 420px;
      object-fit: cover;
      object-position: center;
    }

    .path-grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 18px;
      margin-top: 36px;
    }

    .path-card {
      min-height: 260px;
      padding: 28px;
      border: 1px solid #d8d8d8;
      border-radius: 6px;
      background: #fff;
    }

    .path-card h3 {
      margin: 0 0 16px;
      font-size: 28px;
      line-height: 1.15;
    }

    .path-card p {
      margin: 0;
      color: #5f5f5f;
      font-size: 18px;
      line-height: 1.55;
    }

    .node-section {
      background: #f7f7fb;
    }

    .node-panel {
      display: grid;
      grid-template-columns: minmax(0, 0.9fr) minmax(430px, 1fr);
      gap: 52px;
      align-items: center;
    }

    .terminal {
      border-radius: 8px;
      border: 1px solid #1f1f1f;
      background: #111;
      box-shadow: 0 24px 60px rgba(0, 0, 0, 0.16);
    }

    .footer {
      background: #111;
    }

    @media (max-width: 1100px) {
      .nav {
        min-height: 78px;
      }

      .nav-links {
        gap: 22px;
        font-size: 18px;
      }

      .nav-actions {
        display: none;
      }

      .hero-art img {
        height: 420px;
      }

      h1 {
        font-size: 58px;
      }

      h2 {
        font-size: 54px;
      }

      .hero-copy,
      .editorial-copy p,
      .split-copy p {
        font-size: 23px;
      }

      .editorial,
      .split,
      .node-panel {
        grid-template-columns: 1fr;
      }

      .stats {
        grid-template-columns: repeat(3, minmax(0, 1fr));
        border-left: 0;
        border-top: 1px solid #d3d3d3;
        padding: 38px 0 0;
      }

      .path-grid {
        grid-template-columns: repeat(2, minmax(0, 1fr));
      }
    }

    @media (max-width: 720px) {
      .nav {
        padding: 0 20px;
      }

      .nav-links {
        display: none;
      }

      .hero-art {
        padding: 0;
      }

      .hero-art img {
        height: 270px;
      }

      .hero-copy-block {
        max-width: 390px;
        margin: 54px 0 30px;
        padding: 0 20px;
      }

      h1 {
        max-width: 350px;
        font-size: 38px;
      }

      h2 {
        font-size: 42px;
      }

      .hero-copy,
      .editorial-copy p,
      .split-copy p {
        max-width: 350px;
        font-size: 20px;
      }

      .hero-actions {
        max-width: 350px;
        margin: 0;
      }

      section {
        padding: 72px 20px;
      }

      .editorial {
        min-height: auto;
        gap: 48px;
      }

      .stats,
      .path-grid {
        grid-template-columns: 1fr;
      }

      .split {
        gap: 40px;
      }

      .split-image img {
        height: 280px;
      }

      .node-panel {
        gap: 32px;
      }
    }
  </style>
</head>
<body>
  <header class="site-header">
    <nav class="nav" aria-label="Main navigation">
      <a class="brand" href="/" aria-label="Jsonic home">
"##;

const MARKETING_TAIL: &str = r##"        <span class="sr-only">Jsonic</span>
      </a>
      <div class="nav-links">
        <a href="#learn">Learn</a>
        <a href="#use">Use</a>
        <a href="#build">Build</a>
        <a href="#participate">Participate</a>
        <a href="#research">Research</a>
      </div>
      <div class="nav-actions" aria-label="Site tools">
        <a class="search-pill" href="https://github.com/protosphinx/jsonic" aria-label="Search Jsonic on GitHub">
          <span>search</span>
          <span class="key">cmd</span>
          <span class="key">k</span>
        </a>
        <a class="nav-icon" href="/health">Node</a>
        <span class="nav-icon">Languages EN</span>
      </div>
    </nav>
  </header>

  <main>
    <section class="hero" aria-labelledby="hero-heading">
      <div class="hero-art">
        <img src="/assets/marketing/jsonic-hero-commons.png" alt="Illustrated Jsonic manufacturing commons with makers, buyers, ledgers, and robotics.">
      </div>
      <div class="hero-copy-block">
        <div class="eyebrow">Jsonic L1</div>
        <h1 id="hero-heading">The manufacturing network that belongs to its makers</h1>
        <p class="hero-copy">
          Jsonic is a Layer 1 blockchain where production, invoices, settlement,
          and counterparty reputation become public economic infrastructure.
        </p>
        <div class="hero-actions">
          <a class="button" href="#learn">Start learning</a>
          <a class="button secondary" href="/health">Check the live node</a>
        </div>
      </div>
    </section>

    <section id="learn">
      <div class="section-inner editorial">
        <div class="editorial-copy">
          <div class="eyebrow">The production-owned ledger</div>
          <h2>Jsonic gives verified commerce a native chain</h2>
          <p>
            A bank record says money moved. An ERP record says goods moved.
            Jsonic connects both sides on-chain, so manufacturers and buyers can
            prove useful economic work without trusting a private database.
          </p>
        </div>
        <aside class="stats" aria-label="Jsonic network facts">
          <div class="stat"><span class="stat-icon">[]</span><div><strong>69</strong><span>core tests passing</span></div></div>
          <div class="stat"><span class="stat-icon">POT</span><div><strong>2-party</strong><span>transaction proof</span></div></div>
          <div class="stat"><span class="stat-icon">L1</span><div><strong>live</strong><span>reference RPC node</span></div></div>
        </aside>
      </div>
    </section>

    <section id="use">
      <div class="section-inner split">
        <div class="split-copy">
          <div class="eyebrow">Your business is yours</div>
          <h2>Use commerce without making fake volume valuable</h2>
          <p>
            Jsonic rewards signed activity between real counterparties. Reputation
            flows through the transaction graph, so trusted buyers, suppliers,
            and retailers make each other more credible over time.
          </p>
        </div>
        <div class="split-image">
          <img src="/assets/marketing/jsonic-hero-commons.png" alt="Jsonic manufacturing network illustration crop.">
        </div>
      </div>
    </section>

    <section id="build">
      <div class="section-inner">
        <div class="editorial-copy">
          <div class="eyebrow">Build on real work</div>
          <h2>Choose your path into Jsonic</h2>
        </div>
        <div class="path-grid">
          <article class="path-card">
            <h3>Learn the protocol</h3>
            <p>Understand DAOs, side-chains, Proof of Transaction, PageRank reputation, and Solstice minting.</p>
          </article>
          <article class="path-card">
            <h3>Use the network</h3>
            <p>Register business identities, submit signed invoices and payments, and inspect balances.</p>
          </article>
          <article class="path-card">
            <h3>Build apps</h3>
            <p>Connect ERP systems, payment workflows, agents, and explorers through the JSON-RPC API.</p>
          </article>
          <article class="path-card">
            <h3>Run a node</h3>
            <p>Operate the reference server with persistent state, replay protection, and live health checks.</p>
          </article>
        </div>
      </div>
    </section>

    <section id="participate">
      <div class="section-inner editorial">
        <div class="editorial-copy">
          <div class="eyebrow">Participate</div>
          <h2>Every honest counterparty strengthens the graph</h2>
          <p>
            The network gets better when real buyers confirm real production.
            Honest supply chains compound reputation; isolated self-dealing
            rings drift toward the trust floor.
          </p>
        </div>
        <aside class="stats" aria-label="Jsonic participation model">
          <div class="stat"><span class="stat-icon">01</span><div><strong>register</strong><span>a business DAO</span></div></div>
          <div class="stat"><span class="stat-icon">02</span><div><strong>record</strong><span>signed commerce</span></div></div>
          <div class="stat"><span class="stat-icon">03</span><div><strong>settle</strong><span>at Solstice</span></div></div>
        </aside>
      </div>
    </section>

    <section class="node-section" id="research">
      <div class="section-inner node-panel">
        <div class="split-copy">
          <div class="eyebrow">Research and node</div>
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
        assert!(html.contains("The manufacturing network that belongs to its makers"));
        assert!(html.contains("Jsonic gives verified commerce a native chain"));
        assert!(html.contains("Proof of Transaction"));
        assert!(html.contains("/assets/marketing/jsonic-hero-commons.png"));
        assert!(html.contains("GET  /health"));
    }

    #[tokio::test]
    async fn marketing_image_serves_png() {
        let node = fresh_node();
        let app = build_router(node);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/assets/marketing/jsonic-hero-commons.png")
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
