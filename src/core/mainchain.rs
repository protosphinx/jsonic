//! Main-chain — the Jsonic blockchain backbone.
//!
//! The main-chain connects all DAO side-chains. At each Solstice (periodic
//! sync point, analogous to end of financial year), the main-chain:
//!
//! 1. Collects DAOSnapshots from every side-chain
//! 2. Computes relevance scores and token distributions
//! 3. Mints new tokens and awards them to DAOs
//! 4. Records network health metrics (Anxiety, Adrenaline, Heartbeat)
//! 5. Creates a new main-chain block

use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::crypto::{merkle_root, sha256_str};
use super::pot;
use super::reputation::{
    compute_dao_reward, compute_pagerank, NodeId, PageRankConfig, ReputationGraph,
};
use super::types::{
    BlockHeader, DAOId, DAOSnapshot, Hash, MainChainBlock, NetworkMetrics, TokenDistribution,
};

/// Base tokens minted per Solstice epoch, distributed among DAOs.
const BASE_MINT_PER_SOLSTICE: f64 = 10_000.0;

/// Base heartbeat interval in milliseconds.
const BASE_HEARTBEAT_MS: u64 = 60_000; // 1 minute

/// Target transactions per heartbeat for Adrenaline calculation.
const TARGET_TX_PER_HEARTBEAT: u64 = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainChain {
    pub blocks: Vec<MainChainBlock>,
    pub total_token_supply: f64,
    /// Cumulative DAO-to-DAO transaction graph used for PageRank reputation.
    /// Edges are added when invoices settle (buyer to seller, weighted by value).
    pub reputation_graph: ReputationGraph,
    /// Running count of total transactions across all heartbeats.
    pub(crate) total_transactions: u64,
    /// Running count of invalid transactions for Anxiety.
    pub(crate) invalid_transactions: u64,
    /// Transactions observed in the current heartbeat window.
    pub(crate) current_heartbeat_tx_count: u64,
}

impl MainChain {
    pub fn new() -> Self {
        MainChain {
            blocks: Vec::new(),
            total_token_supply: 0.0,
            reputation_graph: ReputationGraph::new(),
            total_transactions: 0,
            invalid_transactions: 0,
            current_heartbeat_tx_count: 0,
        }
    }

    /// Record transaction outcomes for network metrics.
    pub fn record_transaction_outcome(&mut self, is_valid: bool) {
        self.total_transactions += 1;
        self.current_heartbeat_tx_count += 1;
        if !is_valid {
            self.invalid_transactions += 1;
        }
    }

    /// Record a settled invoice as an edge in the reputation graph.
    /// `buyer` paid `seller` `value` units. PageRank propagates trust along
    /// these edges, so a sale to a high-reputation buyer carries more weight.
    pub fn record_settled_transaction(&mut self, buyer: &DAOId, seller: &DAOId, value: f64) {
        self.reputation_graph.add_transaction(
            NodeId::DAO(buyer.clone()),
            NodeId::DAO(seller.clone()),
            1,
            value,
        );
    }

    /// Perform a Solstice: collect snapshots, mint tokens, create a block.
    ///
    /// Returns the list of token distributions so callers can credit DAOs.
    ///
    /// Relevance scores in the incoming snapshots are overridden with a
    /// PageRank-weighted reward computed against the cumulative reputation
    /// graph. A DAO whose only buyers are Sybil nodes thus receives near-zero
    /// reward regardless of raw transaction count.
    pub fn solstice(&mut self, mut snapshots: Vec<DAOSnapshot>) -> Vec<TokenDistribution> {
        let pr_scores = compute_pagerank(&self.reputation_graph, &PageRankConfig::default());
        for snap in &mut snapshots {
            let node = NodeId::DAO(snap.dao_id.clone());
            let reward = compute_dao_reward(&node, &self.reputation_graph, &pr_scores);
            // Fall back to the simple count*ln(value) metric only when the
            // reputation graph has no inbound edges for this DAO yet (e.g.
            // first Solstice before any invoices have settled).
            if reward > 0.0 {
                snap.relevance_score = reward;
            }
        }

        let distributions = self.compute_token_distribution(&snapshots);

        // Update total supply
        let minted: f64 = distributions.iter().map(|d| d.tokens_awarded).sum();
        self.total_token_supply += minted;

        // Compute network metrics
        let anxiety = pot::compute_anxiety(self.total_transactions, self.invalid_transactions);
        let adrenaline = pot::compute_adrenaline(
            self.current_heartbeat_tx_count,
            TARGET_TX_PER_HEARTBEAT,
        );
        let heartbeat_ms = pot::adjusted_heartbeat_ms(BASE_HEARTBEAT_MS, adrenaline);

        let network_metrics = NetworkMetrics {
            total_daos: snapshots.len() as u64,
            anxiety,
            heartbeat_ms,
            adrenaline,
            total_token_supply: self.total_token_supply,
        };

        // Build the block
        let block = self.create_block(snapshots, distributions.clone(), network_metrics);
        self.blocks.push(block);

        // Reset per-heartbeat counter
        self.current_heartbeat_tx_count = 0;

        distributions
    }

    /// Compute token distribution for each DAO based on the whitepaper's
    /// DAO Valuation Method.
    ///
    /// Tokens are distributed proportionally to each DAO's relevance score,
    /// which factors in:
    /// - Volume of matched transactions
    /// - Value of matched transactions
    /// - Ratio of matched vs unmatched (low Anxiety contribution)
    fn compute_token_distribution(
        &self,
        snapshots: &[DAOSnapshot],
    ) -> Vec<TokenDistribution> {
        let total_relevance: f64 = snapshots.iter().map(|s| s.relevance_score).sum();

        if total_relevance == 0.0 {
            // No relevant activity — no tokens minted
            return snapshots
                .iter()
                .map(|s| TokenDistribution {
                    dao_id: s.dao_id.clone(),
                    tokens_awarded: 0.0,
                    reason: "No matched transactions in this Solstice".to_string(),
                })
                .collect();
        }

        snapshots
            .iter()
            .map(|s| {
                let share = s.relevance_score / total_relevance;
                let tokens = BASE_MINT_PER_SOLSTICE * share;
                TokenDistribution {
                    dao_id: s.dao_id.clone(),
                    tokens_awarded: tokens,
                    reason: format!(
                        "Reputation-weighted relevance {:.4} (period: {} matched txs worth {:.2}), share {:.2}%",
                        s.relevance_score,
                        s.matched_tx_count,
                        s.matched_tx_value,
                        share * 100.0,
                    ),
                }
            })
            .collect()
    }

    /// Compute a DAO's relevance score from its snapshot.
    ///
    /// Score = matched_tx_count * ln(1 + matched_tx_value)
    ///
    /// This rewards both volume and value of legitimate transactions
    /// while using a logarithmic dampener on value to prevent gaming.
    pub fn compute_relevance_score(snapshot: &DAOSnapshot) -> f64 {
        if snapshot.matched_tx_count == 0 {
            return 0.0;
        }
        snapshot.matched_tx_count as f64 * (1.0 + snapshot.matched_tx_value).ln()
    }

    /// Create a main-chain block.
    fn create_block(
        &self,
        snapshots: Vec<DAOSnapshot>,
        distributions: Vec<TokenDistribution>,
        network_metrics: NetworkMetrics,
    ) -> MainChainBlock {
        let previous_hash = self
            .blocks
            .last()
            .map(|b| b.header.hash.clone())
            .unwrap_or_else(|| sha256_str("genesis_main"));

        // Merkle root from DAO snapshot hashes
        let snapshot_hashes: Vec<Hash> = snapshots
            .iter()
            .map(|s| sha256_str(&serde_json::to_string(s).unwrap_or_default()))
            .collect();
        let merkle = merkle_root(&snapshot_hashes);

        let index = self.blocks.len() as u64;
        let timestamp = Utc::now();

        let header_data = format!(
            "{}:{}:{}:{}",
            index,
            previous_hash,
            timestamp.to_rfc3339(),
            merkle,
        );
        let hash = sha256_str(&header_data);

        MainChainBlock {
            header: BlockHeader {
                index,
                previous_hash,
                timestamp,
                merkle_root: merkle,
                hash,
            },
            dao_snapshots: snapshots,
            token_distribution: distributions,
            network_metrics,
        }
    }

    /// Current chain height.
    pub fn height(&self) -> u64 {
        self.blocks.len() as u64
    }

    /// Current network anxiety level.
    pub fn anxiety(&self) -> f64 {
        pot::compute_anxiety(self.total_transactions, self.invalid_transactions)
    }

    /// Current adrenaline factor.
    pub fn adrenaline(&self) -> f64 {
        pot::compute_adrenaline(self.current_heartbeat_tx_count, TARGET_TX_PER_HEARTBEAT)
    }
}

impl Default for MainChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::BalanceSheet;

    fn make_snapshot(dao_id: &str, matched_count: u64, matched_value: f64) -> DAOSnapshot {
        let mut snapshot = DAOSnapshot {
            dao_id: dao_id.to_string(),
            side_chain_height: 1,
            latest_block_hash: "abc".to_string(),
            closing_balance: BalanceSheet::default(),
            matched_tx_count: matched_count,
            matched_tx_value: matched_value,
            relevance_score: 0.0,
        };
        snapshot.relevance_score = MainChain::compute_relevance_score(&snapshot);
        snapshot
    }

    #[test]
    fn test_solstice_creates_block() {
        let mut chain = MainChain::new();
        let snapshots = vec![
            make_snapshot("dao1", 10, 50_000.0),
            make_snapshot("dao2", 5, 20_000.0),
        ];

        chain.solstice(snapshots);
        assert_eq!(chain.height(), 1);
    }

    #[test]
    fn test_token_distribution_proportional() {
        let mut chain = MainChain::new();

        // dao1 has much higher relevance than dao2
        let snapshots = vec![
            make_snapshot("dao1", 100, 1_000_000.0),
            make_snapshot("dao2", 1, 100.0),
        ];

        let distributions = chain.solstice(snapshots);

        assert_eq!(distributions.len(), 2);
        // dao1 should get significantly more tokens
        assert!(distributions[0].tokens_awarded > distributions[1].tokens_awarded);
        // Total should be close to BASE_MINT_PER_SOLSTICE
        let total: f64 = distributions.iter().map(|d| d.tokens_awarded).sum();
        assert!((total - BASE_MINT_PER_SOLSTICE).abs() < 0.01);
    }

    #[test]
    fn test_no_tokens_when_no_activity() {
        let mut chain = MainChain::new();
        let snapshots = vec![
            make_snapshot("dao1", 0, 0.0),
            make_snapshot("dao2", 0, 0.0),
        ];

        let distributions = chain.solstice(snapshots);

        for d in &distributions {
            assert_eq!(d.tokens_awarded, 0.0);
        }
        // No tokens minted
        assert_eq!(chain.total_token_supply, 0.0);
    }

    #[test]
    fn test_relevance_score() {
        let s1 = make_snapshot("dao1", 10, 50_000.0);
        let s2 = make_snapshot("dao2", 0, 0.0);

        assert!(s1.relevance_score > 0.0);
        assert_eq!(s2.relevance_score, 0.0);
    }

    #[test]
    fn test_multiple_solstices() {
        let mut chain = MainChain::new();

        chain.solstice(vec![make_snapshot("dao1", 10, 50_000.0)]);
        chain.solstice(vec![make_snapshot("dao1", 20, 100_000.0)]);

        assert_eq!(chain.height(), 2);
        // Two full mints
        assert!((chain.total_token_supply - 2.0 * BASE_MINT_PER_SOLSTICE).abs() < 0.01);
        // Block hashes chain
        assert_eq!(
            chain.blocks[1].header.previous_hash,
            chain.blocks[0].header.hash
        );
    }

    #[test]
    fn test_network_metrics_recorded() {
        let mut chain = MainChain::new();
        chain.record_transaction_outcome(true);
        chain.record_transaction_outcome(true);
        chain.record_transaction_outcome(false); // 1 invalid out of 3

        chain.solstice(vec![make_snapshot("dao1", 2, 10_000.0)]);

        let metrics = &chain.blocks[0].network_metrics;
        assert_eq!(metrics.total_daos, 1);
        assert!((metrics.anxiety - 1.0 / 3.0).abs() < 0.01);
        assert!(metrics.total_token_supply > 0.0);
    }
}
