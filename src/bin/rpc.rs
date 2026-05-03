//! Jsonic JSON-RPC server.
//!
//! Boots a node, optionally restores prior main-chain state from a sled
//! database, mounts the routes from `jsonic_protocol::api`, and serves on
//! a TCP listener. On a clean shutdown (Ctrl+C), persists the latest
//! main-chain state back to the store.
//!
//! Environment:
//!   JSONIC_RPC_ADDR        bind address (default 127.0.0.1:8080)
//!   JSONIC_RPC_DATA_DIR    sled directory (default ./jsonic-data)

use std::env;
use std::sync::Arc;

use jsonic_protocol::api::{SharedNode, build_router};
use jsonic_protocol::core::heartbeat::JsonicNode;
use jsonic_protocol::core::store::{ChainStore, SledStore};
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::var("JSONIC_RPC_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
    let data_dir = env::var("JSONIC_RPC_DATA_DIR").unwrap_or_else(|_| "./jsonic-data".to_string());

    let store = SledStore::open(&data_dir)?;
    let mut node = JsonicNode::new();
    if let Some(restored) = store.load_main_chain()? {
        eprintln!(
            "[jsonic-rpc] restored main-chain at height {} from {}",
            restored.height(),
            data_dir
        );
        node.main_chain = restored;
    } else {
        eprintln!(
            "[jsonic-rpc] no prior chain at {}, starting fresh",
            data_dir
        );
    }

    let shared: SharedNode = Arc::new(RwLock::new(node));
    let app = build_router(shared.clone());

    let listener = TcpListener::bind(&addr).await?;
    eprintln!("[jsonic-rpc] listening on http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    let final_chain = shared.read().await.main_chain.clone();
    store.save_main_chain(&final_chain)?;
    eprintln!(
        "[jsonic-rpc] persisted main-chain at height {} to {}",
        final_chain.height(),
        data_dir
    );
    Ok(())
}

async fn shutdown_signal() {
    let _ = signal::ctrl_c().await;
    eprintln!("[jsonic-rpc] ctrl+c received, shutting down");
}
