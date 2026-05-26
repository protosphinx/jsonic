//! Persistence layer for Jsonic node state.
//!
//! Defines a `ChainStore` trait so callers can plug in different backends
//! (in-memory for tests, sled for production). The reference implementation
//! persists full `JsonicNode` state, including the main-chain, registered DAOs,
//! side-chains, pending transactions, heartbeat tick, and sequence counters.
//!
//! All serialization goes through `bincode` for compactness and speed.

use std::path::Path;
use std::sync::Mutex;

use thiserror::Error;

use super::heartbeat::JsonicNode;
use super::mainchain::MainChain;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("backend error: {0}")]
    Backend(String),
    #[error("serialization error: {0}")]
    Encode(#[from] bincode::Error),
}

pub type StoreResult<T> = Result<T, StoreError>;

const KEY_MAIN_CHAIN: &[u8] = b"main_chain";
const KEY_NODE: &[u8] = b"node";

/// A persistent store for node and main-chain state. Implementations must be
/// thread-safe; concurrent saves and loads are expected from an RPC server.
pub trait ChainStore: Send + Sync {
    /// Persist the full node state. Overwrites any previous snapshot.
    fn save_node(&self, node: &JsonicNode) -> StoreResult<()>;

    /// Load the full node state if any has been saved. Returns `None`
    /// on a fresh store.
    fn load_node(&self) -> StoreResult<Option<JsonicNode>>;

    /// Persist the full main-chain state. Overwrites any previous snapshot.
    ///
    /// Kept for compatibility with older stores and tests. New nodes should
    /// prefer `save_node` so registry and side-chain state survive restart.
    fn save_main_chain(&self, chain: &MainChain) -> StoreResult<()>;

    /// Load the main-chain state if any has been saved. Returns `None`
    /// on a fresh store. Used as a migration fallback for pre-node snapshots.
    fn load_main_chain(&self) -> StoreResult<Option<MainChain>>;
}

// ---------------------------------------------------------------------------
// In-memory backend
// ---------------------------------------------------------------------------

/// In-memory store. Useful for tests, ephemeral nodes, and as a default.
#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<Option<Vec<u8>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ChainStore for MemoryStore {
    fn save_node(&self, node: &JsonicNode) -> StoreResult<()> {
        let bytes = bincode::serialize(node)?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| StoreError::Backend(format!("mutex poisoned: {e}")))?;
        *guard = Some(bytes);
        Ok(())
    }

    fn load_node(&self) -> StoreResult<Option<JsonicNode>> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| StoreError::Backend(format!("mutex poisoned: {e}")))?;
        match guard.as_ref() {
            Some(bytes) => Ok(Some(bincode::deserialize(bytes)?)),
            None => Ok(None),
        }
    }

    fn save_main_chain(&self, chain: &MainChain) -> StoreResult<()> {
        let bytes = bincode::serialize(chain)?;
        let mut guard = self
            .inner
            .lock()
            .map_err(|e| StoreError::Backend(format!("mutex poisoned: {e}")))?;
        *guard = Some(bytes);
        Ok(())
    }

    fn load_main_chain(&self) -> StoreResult<Option<MainChain>> {
        let guard = self
            .inner
            .lock()
            .map_err(|e| StoreError::Backend(format!("mutex poisoned: {e}")))?;
        match guard.as_ref() {
            Some(bytes) => Ok(Some(bincode::deserialize(bytes)?)),
            None => Ok(None),
        }
    }
}

// ---------------------------------------------------------------------------
// sled backend
// ---------------------------------------------------------------------------

/// On-disk store backed by sled. Survives node restarts.
pub struct SledStore {
    db: sled::Db,
}

impl SledStore {
    /// Open or create a sled database at the given path.
    pub fn open<P: AsRef<Path>>(path: P) -> StoreResult<Self> {
        let db = sled::open(path).map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(SledStore { db })
    }
}

impl ChainStore for SledStore {
    fn save_node(&self, node: &JsonicNode) -> StoreResult<()> {
        let bytes = bincode::serialize(node)?;
        self.db
            .insert(KEY_NODE, bytes)
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        self.db
            .flush()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }

    fn load_node(&self) -> StoreResult<Option<JsonicNode>> {
        let raw = self
            .db
            .get(KEY_NODE)
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        match raw {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    fn save_main_chain(&self, chain: &MainChain) -> StoreResult<()> {
        let bytes = bincode::serialize(chain)?;
        self.db
            .insert(KEY_MAIN_CHAIN, bytes)
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        self.db
            .flush()
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        Ok(())
    }

    fn load_main_chain(&self) -> StoreResult<Option<MainChain>> {
        let raw = self
            .db
            .get(KEY_MAIN_CHAIN)
            .map_err(|e| StoreError::Backend(e.to_string()))?;
        match raw {
            Some(bytes) => Ok(Some(bincode::deserialize(&bytes)?)),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dao::RegisteredDAO;
    use crate::core::heartbeat::JsonicNode;
    use crate::core::types::BalanceSheet;
    use crate::core::types::DAOSnapshot;

    fn fixture_chain() -> MainChain {
        // Drive the chain through one Solstice so it carries non-trivial
        // state: blocks, supply, reputation graph edges, the lot.
        let mut chain = MainChain::new();
        chain.record_settled_transaction(&"buyer".to_string(), &"seller".to_string(), 12_345.0);
        chain.record_transaction_outcome(true);
        chain.record_transaction_outcome(false);
        let snap = DAOSnapshot {
            dao_id: "seller".to_string(),
            side_chain_height: 1,
            latest_block_hash: "abc".to_string(),
            closing_balance: BalanceSheet::default(),
            matched_tx_count: 1,
            matched_tx_value: 12_345.0,
            relevance_score: 1.0,
        };
        chain.solstice(vec![snap]);
        chain
    }

    fn assert_round_trip_equal(original: &MainChain, restored: &MainChain) {
        assert_eq!(original.blocks.len(), restored.blocks.len());
        assert_eq!(original.total_token_supply, restored.total_token_supply);
        assert_eq!(
            original.reputation_graph.node_count(),
            restored.reputation_graph.node_count(),
        );
        if let (Some(a), Some(b)) = (original.blocks.first(), restored.blocks.first()) {
            assert_eq!(a.header.hash, b.header.hash);
        }
    }

    fn fixture_node() -> JsonicNode {
        let mut node = JsonicNode::new();
        node.solstice_interval = 5;

        let mut seller = RegisteredDAO::register("Seller", "Manufacturing");
        let mut buyer = RegisteredDAO::register("Buyer", "Retail");
        let seller_id = seller.id().clone();
        let buyer_id = buyer.id().clone();

        node.register_dao(seller.dao.clone());
        node.register_dao(buyer.dao.clone());

        let invoice = seller.create_invoice(&buyer_id, 250.0, "USD", "widgets");
        let payment = buyer.create_payment(&seller_id, 250.0, "USD", &invoice.id, "paid");
        node.submit_transaction(invoice).expect("invoice");
        node.submit_transaction(payment).expect("payment");
        node.heartbeat();
        node
    }

    #[test]
    fn memory_store_roundtrip() {
        let chain = fixture_chain();
        let store = MemoryStore::new();
        store.save_main_chain(&chain).expect("save");
        let restored = store.load_main_chain().expect("load").expect("present");
        assert_round_trip_equal(&chain, &restored);
    }

    #[test]
    fn memory_store_empty_returns_none() {
        let store = MemoryStore::new();
        assert!(store.load_main_chain().expect("load").is_none());
    }

    #[test]
    fn memory_store_node_roundtrip() {
        let node = fixture_node();
        let store = MemoryStore::new();
        store.save_node(&node).expect("save");
        let restored = store.load_node().expect("load").expect("present");

        assert_eq!(restored.tick, node.tick);
        assert_eq!(restored.registry.count(), node.registry.count());
        assert_eq!(restored.side_chains.len(), node.side_chains.len());
        assert_eq!(restored.pending_count(), node.pending_count());
        assert_eq!(restored.main_chain.height(), node.main_chain.height());
    }

    #[test]
    fn sled_store_roundtrip_survives_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("chain.db");

        let chain = fixture_chain();
        {
            let store = SledStore::open(&path).expect("open");
            store.save_main_chain(&chain).expect("save");
        }

        // Drop and reopen: simulates a node restart.
        let store = SledStore::open(&path).expect("reopen");
        let restored = store.load_main_chain().expect("load").expect("present");
        assert_round_trip_equal(&chain, &restored);
    }

    #[test]
    fn sled_store_node_roundtrip_survives_reopen() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("node.db");

        let node = fixture_node();
        {
            let store = SledStore::open(&path).expect("open");
            store.save_node(&node).expect("save");
        }

        let store = SledStore::open(&path).expect("reopen");
        let restored = store.load_node().expect("load").expect("present");
        assert_eq!(restored.tick, node.tick);
        assert_eq!(restored.registry.count(), node.registry.count());
        assert_eq!(restored.side_chains.len(), node.side_chains.len());
        assert_eq!(restored.pending_count(), node.pending_count());
    }

    #[test]
    fn sled_store_restored_node_can_continue_processing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("live-node.db");

        let mut node = JsonicNode::new();
        node.solstice_interval = 3;

        let mut seller = RegisteredDAO::register("Seller", "Manufacturing");
        let mut buyer = RegisteredDAO::register("Buyer", "Retail");
        let seller_id = seller.id().clone();
        let buyer_id = buyer.id().clone();

        node.register_dao(seller.dao.clone());
        node.register_dao(buyer.dao.clone());

        let invoice = seller.create_invoice(&buyer_id, 500.0, "USD", "widgets");
        node.submit_transaction(invoice.clone()).expect("invoice");
        node.heartbeat();

        {
            let store = SledStore::open(&path).expect("open");
            store.save_node(&node).expect("save");
        }

        let store = SledStore::open(&path).expect("reopen");
        let mut restored = store.load_node().expect("load").expect("present");

        let payment = buyer.create_payment(&seller_id, 500.0, "USD", &invoice.id, "paid");
        restored.submit_transaction(payment).expect("payment");

        assert!(restored.heartbeat().is_none());
        let dist = restored
            .heartbeat()
            .expect("third heartbeat should trigger Solstice");

        assert_eq!(restored.main_chain.height(), 1);
        assert_eq!(restored.pending_count(), 0);
        assert!(dist.iter().any(|d| d.dao_id == seller_id));
    }

    #[test]
    fn sled_store_overwrites_previous_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("chain.db");

        let store = SledStore::open(&path).expect("open");
        let v1 = MainChain::new();
        store.save_main_chain(&v1).expect("save v1");

        let v2 = fixture_chain();
        store.save_main_chain(&v2).expect("save v2");

        let restored = store.load_main_chain().expect("load").expect("present");
        assert_eq!(restored.blocks.len(), v2.blocks.len());
        assert_eq!(restored.total_token_supply, v2.total_token_supply);
    }
}
