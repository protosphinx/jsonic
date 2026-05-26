//! JSON-RPC / REST API surface for a Jsonic node.
//!
//! Exposes the protocol over HTTP so external systems (an ERP, a payments
//! gateway, a block explorer) can submit transactions, drive heartbeats,
//! and query state without linking the protocol crate directly.
//!
//! Routes:
//!   GET  /                        service index
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
    response::{IntoResponse, Json},
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
struct IndexResponse {
    service: &'static str,
    status: &'static str,
    version: &'static str,
    endpoints: &'static [&'static str],
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

async fn index() -> Json<IndexResponse> {
    Json(IndexResponse {
        service: "Jsonic RPC",
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        endpoints: &[
            "GET /health",
            "POST /daos",
            "POST /transactions",
            "POST /heartbeats",
            "GET /blocks/:height",
            "GET /metrics",
            "GET /balance/:dao_id",
            "GET /reputation/:dao_id",
        ],
    })
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
    async fn index_reports_service_surface() {
        let node = fresh_node();
        let app = build_router(node);
        let resp = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let v = body_json(resp).await;
        assert_eq!(v["service"], "Jsonic RPC");
        assert_eq!(v["status"], "ok");
        assert!(
            v["endpoints"]
                .as_array()
                .unwrap()
                .contains(&json!("GET /health"))
        );
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
