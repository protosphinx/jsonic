//! Reputation engine — PageRank-based trust propagation for the Jsonic network.
//!
//! Applies the same mathematics as Google's PageRank to compute reputation
//! scores for DAOs and consumers in the transaction graph.
//!
//! # PageRank Formula
//!
//! ```text
//! PR(A) = (1 - d) / N + d * Σ [ PR(Ti) / C(Ti) * W(Ti → A) ]
//! ```
//!
//! Where:
//! - `d`    = damping factor (0.85) — probability of following a transaction edge
//!   vs. random jump
//! - `N`    = total number of nodes in the graph
//! - `Ti`   = nodes that transact with A (in-edges: entities that bought from
//!   or sold to A)
//! - `C(Ti)`= out-degree of Ti (number of unique trading partners)
//! - `W(Ti → A)` = normalized edge weight from Ti to A, based on transaction
//!   value and volume
//!
//! # Jsonic-Specific Extensions
//!
//! On top of raw PageRank, Jsonic applies:
//!
//! - **Diversity factor**: rewards breadth of unique counterparties.
//!   `diversity(A) = unique_counterparties(A) / max_unique_counterparties`
//!
//! - **Volume weighting**: edges are weighted by both the number of transactions
//!   AND their total value, so 1M sales to 1M buyers carries more weight than
//!   1 sale of 1M to a single buyer.
//!
//! - **Sybil resistance**: new/empty nodes start with minimal rank (1-d)/N,
//!   so creating fake DAOs to self-transact yields near-zero reputation since
//!   those fake DAOs have no inbound reputation themselves.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// A node in the transaction graph: either a DAO or a verified consumer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeId {
    DAO(String),
    /// Consumer identified by a tokenized payment credential
    /// (hashed credit card, UPI ID, wallet address, etc.)
    Consumer(String),
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeId::DAO(id) => write!(f, "DAO:{}", &id[..id.len().min(12)]),
            NodeId::Consumer(id) => write!(f, "Consumer:{}", &id[..id.len().min(12)]),
        }
    }
}

/// A directed, weighted edge in the transaction graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionEdge {
    pub from: NodeId,
    pub to: NodeId,
    /// Number of individual transactions along this edge.
    pub tx_count: u64,
    /// Total monetary value of transactions along this edge.
    pub tx_value: f64,
}

impl TransactionEdge {
    /// Combined weight of this edge.
    /// Uses log-dampened value to prevent a single huge transaction from
    /// dominating, while still rewarding higher volume.
    ///
    /// `weight = tx_count * ln(1 + tx_value)`
    pub fn weight(&self) -> f64 {
        self.tx_count as f64 * (1.0 + self.tx_value).ln()
    }
}

/// The transaction graph on which PageRank is computed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationGraph {
    /// All nodes in the graph.
    nodes: HashSet<NodeId>,
    /// Adjacency list: from -> [(to, edge)].
    outbound: HashMap<NodeId, Vec<TransactionEdge>>,
    /// Reverse adjacency: to -> [(from, edge)].
    inbound: HashMap<NodeId, Vec<TransactionEdge>>,
}

impl ReputationGraph {
    pub fn new() -> Self {
        ReputationGraph {
            nodes: HashSet::new(),
            outbound: HashMap::new(),
            inbound: HashMap::new(),
        }
    }

    /// Add or update a transaction edge between two nodes.
    /// If an edge already exists between (from, to), the counts and values
    /// are accumulated.
    pub fn add_transaction(&mut self, from: NodeId, to: NodeId, tx_count: u64, tx_value: f64) {
        self.nodes.insert(from.clone());
        self.nodes.insert(to.clone());

        // Update outbound edge
        let out_edges = self.outbound.entry(from.clone()).or_default();
        if let Some(edge) = out_edges.iter_mut().find(|e| e.to == to) {
            edge.tx_count += tx_count;
            edge.tx_value += tx_value;
        } else {
            out_edges.push(TransactionEdge {
                from: from.clone(),
                to: to.clone(),
                tx_count,
                tx_value,
            });
        }

        // Update inbound edge
        let in_edges = self.inbound.entry(to.clone()).or_default();
        if let Some(edge) = in_edges.iter_mut().find(|e| e.from == from) {
            edge.tx_count += tx_count;
            edge.tx_value += tx_value;
        } else {
            in_edges.push(TransactionEdge {
                from: from.clone(),
                to: to.clone(),
                tx_count,
                tx_value,
            });
        }
    }

    /// Total number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of unique trading partners for a node (out-degree).
    pub fn out_degree(&self, node: &NodeId) -> usize {
        self.outbound.get(node).map(|e| e.len()).unwrap_or(0)
    }

    /// Total outbound weight from a node (sum of all outbound edge weights).
    fn total_outbound_weight(&self, node: &NodeId) -> f64 {
        self.outbound
            .get(node)
            .map(|edges| edges.iter().map(|e| e.weight()).sum())
            .unwrap_or(0.0)
    }

    /// Number of unique counterparties (both inbound and outbound) for a node.
    pub fn unique_counterparties(&self, node: &NodeId) -> usize {
        let mut partners = HashSet::new();
        if let Some(edges) = self.outbound.get(node) {
            for e in edges {
                partners.insert(&e.to);
            }
        }
        if let Some(edges) = self.inbound.get(node) {
            for e in edges {
                partners.insert(&e.from);
            }
        }
        partners.len()
    }
}

impl Default for ReputationGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// PageRank computation
// ---------------------------------------------------------------------------

/// Configuration for PageRank computation.
pub struct PageRankConfig {
    /// Damping factor (probability of following an edge vs. random jump).
    /// Standard value: 0.85.
    pub damping: f64,
    /// Maximum number of iterations.
    pub max_iterations: u32,
    /// Convergence threshold — stop when the L1 norm of the rank vector
    /// changes by less than this between iterations.
    pub tolerance: f64,
}

impl Default for PageRankConfig {
    fn default() -> Self {
        PageRankConfig {
            damping: 0.85,
            max_iterations: 100,
            tolerance: 1e-8,
        }
    }
}

/// Result of a PageRank computation.
#[derive(Debug, Clone)]
pub struct ReputationScores {
    /// Raw PageRank scores per node.
    pub pagerank: HashMap<NodeId, f64>,
    /// Diversity factor per node (0.0 - 1.0). Count-based; kept for
    /// inspection and the composite score. Not used by `compute_dao_reward`,
    /// which uses a reputation-weighted notion of diversity instead.
    pub diversity: HashMap<NodeId, f64>,
    /// Final composite score: pagerank * (1 + diversity).
    pub composite: HashMap<NodeId, f64>,
    /// Number of iterations until convergence.
    pub iterations: u32,
    /// The random-walk floor `(1 - d) / N`. Every node's PageRank is at
    /// least this value. Used by `compute_dao_reward` to filter out the
    /// signal that a node has only because it exists.
    pub baseline_rank: f64,
}

/// Compute PageRank on the transaction graph.
///
/// This implements the iterative power method:
///
/// ```text
/// PR(i)_new = (1 - d) / N  +  d * Σ_j [ PR(j)_old * W(j→i) / W_out(j) ]
/// ```
///
/// Where:
/// - The sum is over all nodes j that have an edge to i
/// - `W(j→i)` is the weight of the edge from j to i
/// - `W_out(j)` is the total outbound weight from j
///
/// Dangling nodes (nodes with no outbound edges) distribute their rank
/// uniformly across all nodes, just like in the original PageRank paper.
pub fn compute_pagerank(graph: &ReputationGraph, config: &PageRankConfig) -> ReputationScores {
    let n = graph.node_count();
    if n == 0 {
        return ReputationScores {
            pagerank: HashMap::new(),
            diversity: HashMap::new(),
            composite: HashMap::new(),
            iterations: 0,
            baseline_rank: 0.0,
        };
    }

    let n_f64 = n as f64;
    let d = config.damping;
    let baseline_rank = (1.0 - d) / n_f64;

    // Initialize: uniform distribution
    let initial_rank = 1.0 / n_f64;
    let mut ranks: HashMap<NodeId, f64> = graph
        .nodes
        .iter()
        .map(|node| (node.clone(), initial_rank))
        .collect();

    let mut iterations = 0;

    for _ in 0..config.max_iterations {
        iterations += 1;
        let mut new_ranks: HashMap<NodeId, f64> = HashMap::new();

        // Compute dangling node contribution.
        // Dangling nodes have no outbound edges — their rank is distributed
        // uniformly to all nodes.
        let dangling_sum: f64 = graph
            .nodes
            .iter()
            .filter(|node| graph.out_degree(node) == 0)
            .map(|node| ranks.get(node).copied().unwrap_or(0.0))
            .sum();

        // Base rank: random jump + dangling redistribution
        let base_rank = (1.0 - d) / n_f64 + d * dangling_sum / n_f64;

        for node in &graph.nodes {
            let mut rank = base_rank;

            // Sum contributions from all inbound edges
            if let Some(in_edges) = graph.inbound.get(node) {
                for edge in in_edges {
                    let source_rank = ranks.get(&edge.from).copied().unwrap_or(0.0);
                    let source_total_weight = graph.total_outbound_weight(&edge.from);

                    if source_total_weight > 0.0 {
                        // Weighted contribution: source's rank * (this edge's weight / source's total outbound weight)
                        rank += d * source_rank * (edge.weight() / source_total_weight);
                    }
                }
            }

            new_ranks.insert(node.clone(), rank);
        }

        // Check convergence: L1 norm of difference
        let delta: f64 = graph
            .nodes
            .iter()
            .map(|node| {
                let old = ranks.get(node).copied().unwrap_or(0.0);
                let new = new_ranks.get(node).copied().unwrap_or(0.0);
                (old - new).abs()
            })
            .sum();

        ranks = new_ranks;

        if delta < config.tolerance {
            break;
        }
    }

    // Compute diversity factors
    let max_counterparties = graph
        .nodes
        .iter()
        .map(|n| graph.unique_counterparties(n))
        .max()
        .unwrap_or(1)
        .max(1); // avoid division by zero

    let diversity: HashMap<NodeId, f64> = graph
        .nodes
        .iter()
        .map(|node| {
            let cp = graph.unique_counterparties(node) as f64;
            let factor = cp / max_counterparties as f64;
            (node.clone(), factor)
        })
        .collect();

    // Composite score: pagerank * (1 + diversity_boost)
    // Diversity boost scales from 0 to 1, so composite ranges from PR to 2*PR
    let composite: HashMap<NodeId, f64> = graph
        .nodes
        .iter()
        .map(|node| {
            let pr = ranks.get(node).copied().unwrap_or(0.0);
            let div = diversity.get(node).copied().unwrap_or(0.0);
            (node.clone(), pr * (1.0 + div))
        })
        .collect();

    ReputationScores {
        pagerank: ranks,
        diversity,
        composite,
        iterations,
        baseline_rank,
    }
}

/// Compute the token reward for a DAO based on its reputation-weighted sales.
///
/// ```text
/// trust(buyer) = max(0, PR(buyer) - baseline_rank)
/// reward(DAO)  = Σ_sale [ ln(1 + sale_value) * tx_count * trust(buyer) ]
/// ```
///
/// The `baseline_rank` is `(1 - d) / N`: the PageRank a node has purely from
/// existing in the graph. Subtracting it before weighting filters out the
/// "everyone-gets-some-rank" signal so that nodes which have not earned trust
/// from other earned-trust nodes contribute nothing to a seller's reward.
///
/// Why no diversity multiplier? An earlier formulation multiplied by
/// `(1 + unique_counterparty_count / max_count)`. That made a DAO with N fake
/// unique buyers earn a 2x multiplier even though those buyers were
/// reputation-less, which let a Sybil ring beat an honest cluster as long as
/// the ring had enough fake unique nodes. The trust-floor formulation captures
/// real diversity automatically: each unique high-trust buyer contributes
/// independently, while fake buyers (whose PR sits at baseline) contribute
/// zero regardless of count.
///
/// Properties:
/// - Selling to high-reputation buyers yields more tokens (linear in trust).
/// - Selling to many independently-trusted buyers yields more tokens (each
///   contributes its own trust to the sum).
/// - Higher sale values yield more tokens, log-dampened so a single huge
///   transaction cannot dominate.
/// - A Sybil ring of N fake buyers contributes ~0 because every fake's PR
///   converges to the random-walk baseline.
pub fn compute_dao_reward(dao: &NodeId, graph: &ReputationGraph, scores: &ReputationScores) -> f64 {
    let inbound_edges = match graph.inbound.get(dao) {
        Some(edges) => edges,
        None => return 0.0,
    };

    let baseline = scores.baseline_rank;
    let mut reward = 0.0;
    for edge in inbound_edges {
        let buyer_rank = scores.pagerank.get(&edge.from).copied().unwrap_or(0.0);
        let trust = (buyer_rank - baseline).max(0.0);
        reward += (1.0 + edge.tx_value).ln() * edge.tx_count as f64 * trust;
    }
    reward
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dao(name: &str) -> NodeId {
        NodeId::DAO(name.to_string())
    }

    fn consumer(name: &str) -> NodeId {
        NodeId::Consumer(name.to_string())
    }

    // -----------------------------------------------------------------------
    // Graph construction
    // -----------------------------------------------------------------------

    #[test]
    fn test_empty_graph() {
        let graph = ReputationGraph::new();
        let scores = compute_pagerank(&graph, &PageRankConfig::default());
        assert_eq!(scores.pagerank.len(), 0);
        assert_eq!(scores.iterations, 0);
    }

    #[test]
    fn test_single_transaction() {
        let mut graph = ReputationGraph::new();
        graph.add_transaction(consumer("alice"), dao("factory1"), 1, 100.0);

        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.out_degree(&consumer("alice")), 1);
        assert_eq!(graph.out_degree(&dao("factory1")), 0);
    }

    #[test]
    fn test_accumulated_edges() {
        let mut graph = ReputationGraph::new();
        graph.add_transaction(consumer("alice"), dao("factory1"), 5, 500.0);
        graph.add_transaction(consumer("alice"), dao("factory1"), 3, 300.0);

        // Should accumulate, not duplicate
        assert_eq!(graph.outbound[&consumer("alice")].len(), 1);
        assert_eq!(graph.outbound[&consumer("alice")][0].tx_count, 8);
        assert_eq!(graph.outbound[&consumer("alice")][0].tx_value, 800.0);
    }

    // -----------------------------------------------------------------------
    // PageRank fundamentals
    // -----------------------------------------------------------------------

    #[test]
    fn test_pagerank_sums_to_one() {
        let mut graph = ReputationGraph::new();
        graph.add_transaction(dao("A"), dao("B"), 10, 1000.0);
        graph.add_transaction(dao("B"), dao("C"), 5, 500.0);
        graph.add_transaction(dao("C"), dao("A"), 3, 300.0);

        let scores = compute_pagerank(&graph, &PageRankConfig::default());
        let total: f64 = scores.pagerank.values().sum();
        assert!(
            (total - 1.0).abs() < 1e-6,
            "PageRank should sum to 1.0, got {}",
            total
        );
    }

    #[test]
    fn test_pagerank_converges() {
        let mut graph = ReputationGraph::new();
        graph.add_transaction(dao("A"), dao("B"), 10, 1000.0);
        graph.add_transaction(dao("B"), dao("A"), 10, 1000.0);

        let scores = compute_pagerank(&graph, &PageRankConfig::default());
        // Symmetric graph → equal ranks
        let pr_a = scores.pagerank[&dao("A")];
        let pr_b = scores.pagerank[&dao("B")];
        assert!(
            (pr_a - pr_b).abs() < 1e-6,
            "Symmetric graph should have equal ranks: A={}, B={}",
            pr_a,
            pr_b
        );
    }

    // -----------------------------------------------------------------------
    // Sybil resistance: fake DAOs don't help
    // -----------------------------------------------------------------------

    #[test]
    fn test_sybil_resistance() {
        // Scenario: DAO "legit" has real transactions with high-rep partners.
        // DAO "sybil" creates 100 fake DAOs that only transact with it.
        //
        // Raw PageRank may give sybil a high score due to sheer inbound count
        // (just like a website with 100 spam backlinks gets some raw PageRank).
        // But the REWARD function — which weights each sale by the buyer's
        // reputation — is what actually determines token minting. Sybil's
        // buyers all have near-zero reputation, so its reward is low.
        let mut graph = ReputationGraph::new();

        // Legitimate ecosystem: A ↔ B ↔ C ↔ A, all with real volume
        graph.add_transaction(dao("A"), dao("B"), 100, 500_000.0);
        graph.add_transaction(dao("B"), dao("C"), 80, 400_000.0);
        graph.add_transaction(dao("C"), dao("A"), 60, 300_000.0);
        graph.add_transaction(dao("B"), dao("A"), 50, 250_000.0);
        graph.add_transaction(dao("C"), dao("B"), 40, 200_000.0);
        graph.add_transaction(dao("A"), dao("C"), 30, 150_000.0);

        // Sybil attack: "sybil" creates 100 fake DAOs that buy from it
        for i in 0..100 {
            let fake = dao(&format!("fake_{}", i));
            graph.add_transaction(fake, dao("sybil"), 1, 1000.0);
        }

        let scores = compute_pagerank(&graph, &PageRankConfig::default());

        // The real test: compute_dao_reward uses buyer reputation as weight.
        // Sybil's 100 fake buyers each have minimal reputation (only (1-d)/N),
        // so the reward is low despite high transaction count.
        let reward_a = compute_dao_reward(&dao("A"), &graph, &scores);
        let reward_sybil = compute_dao_reward(&dao("sybil"), &graph, &scores);

        assert!(
            reward_a > reward_sybil,
            "Legit DAO A reward ({}) should exceed sybil reward ({}) because sybil's buyers have no reputation",
            reward_a,
            reward_sybil
        );
    }

    // -----------------------------------------------------------------------
    // Volume vs diversity
    // -----------------------------------------------------------------------

    #[test]
    fn test_diversity_rewards_breadth() {
        let mut graph = ReputationGraph::new();

        // DAO "diverse" sells to 5 different consumers
        for i in 0..5 {
            graph.add_transaction(consumer(&format!("c{}", i)), dao("diverse"), 10, 1000.0);
        }

        // DAO "concentrated" sells 50 items to 1 consumer (same total volume)
        graph.add_transaction(consumer("whale"), dao("concentrated"), 50, 5000.0);

        let scores = compute_pagerank(&graph, &PageRankConfig::default());

        let div_diverse = scores.diversity[&dao("diverse")];
        let div_concentrated = scores.diversity[&dao("concentrated")];

        assert!(
            div_diverse > div_concentrated,
            "Diverse DAO ({}) should have higher diversity than concentrated ({})",
            div_diverse,
            div_concentrated
        );
    }

    #[test]
    fn test_composite_score_combines_pagerank_and_diversity() {
        let mut graph = ReputationGraph::new();

        // Build a small ecosystem
        graph.add_transaction(dao("A"), dao("B"), 10, 10_000.0);
        graph.add_transaction(dao("B"), dao("A"), 10, 10_000.0);
        graph.add_transaction(consumer("c1"), dao("A"), 5, 5_000.0);

        let scores = compute_pagerank(&graph, &PageRankConfig::default());

        for node in graph.nodes.iter() {
            let pr = scores.pagerank[node];
            let div = scores.diversity[node];
            let comp = scores.composite[node];
            let expected = pr * (1.0 + div);
            assert!(
                (comp - expected).abs() < 1e-10,
                "Composite should be PR * (1 + diversity) for {}",
                node
            );
        }
    }

    // -----------------------------------------------------------------------
    // Reward computation
    // -----------------------------------------------------------------------

    #[test]
    fn test_reward_higher_for_reputable_buyers() {
        let mut graph = ReputationGraph::new();

        // Create a reputation ecosystem: A ↔ B with heavy traffic (high rank)
        graph.add_transaction(dao("A"), dao("B"), 100, 1_000_000.0);
        graph.add_transaction(dao("B"), dao("A"), 100, 1_000_000.0);

        // "newbie" consumer with no history
        graph.add_transaction(consumer("newbie"), dao("seller"), 1, 1000.0);
        // High-rep DAO A also buys from "seller"
        graph.add_transaction(dao("A"), dao("seller"), 1, 1000.0);

        let scores = compute_pagerank(&graph, &PageRankConfig::default());
        let reward = compute_dao_reward(&dao("seller"), &graph, &scores);

        // Reward should be positive (seller has buyers)
        assert!(reward > 0.0, "Seller should earn positive reward");

        // Compare: a seller with ONLY newbie buyer should earn less
        let mut graph2 = ReputationGraph::new();
        graph2.add_transaction(consumer("newbie"), dao("seller2"), 1, 1000.0);
        let scores2 = compute_pagerank(&graph2, &PageRankConfig::default());
        let reward2 = compute_dao_reward(&dao("seller2"), &graph2, &scores2);

        assert!(
            reward > reward2,
            "Seller with high-rep buyer ({}) should earn more than seller with only newbie ({})",
            reward,
            reward2
        );
    }

    #[test]
    fn test_no_reward_for_no_sales() {
        let mut graph = ReputationGraph::new();
        graph.add_transaction(dao("buyer"), dao("other"), 10, 10_000.0);

        let scores = compute_pagerank(&graph, &PageRankConfig::default());
        let reward = compute_dao_reward(&dao("buyer"), &graph, &scores);

        // "buyer" only buys, never sells — no inbound edges as seller
        assert_eq!(reward, 0.0);
    }

    // -----------------------------------------------------------------------
    // Edge weight formula
    // -----------------------------------------------------------------------

    #[test]
    fn test_edge_weight_log_dampened() {
        let small = TransactionEdge {
            from: dao("A"),
            to: dao("B"),
            tx_count: 1,
            tx_value: 100.0,
        };
        let large = TransactionEdge {
            from: dao("A"),
            to: dao("B"),
            tx_count: 1,
            tx_value: 1_000_000.0,
        };

        // Large value should have higher weight, but not 10,000x higher
        // (because of log dampening)
        let ratio = large.weight() / small.weight();
        assert!(ratio > 1.0, "Larger value should have higher weight");
        assert!(
            ratio < 100.0,
            "Log dampening should prevent linear scaling, ratio={}",
            ratio
        );
    }

    #[test]
    fn test_edge_weight_rewards_volume() {
        let one_sale = TransactionEdge {
            from: dao("A"),
            to: dao("B"),
            tx_count: 1,
            tx_value: 1_000_000.0,
        };
        let many_sales = TransactionEdge {
            from: dao("A"),
            to: dao("B"),
            tx_count: 1000,
            tx_value: 1_000_000.0,
        };

        // 1000 sales should be 1000x the weight of 1 sale (same value)
        let ratio = many_sales.weight() / one_sale.weight();
        assert!(
            (ratio - 1000.0).abs() < 0.01,
            "Volume should scale linearly, ratio={}",
            ratio
        );
    }
}
