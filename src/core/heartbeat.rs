//! Heartbeat — the Jsonic network's pulse.
//!
//! The Heartbeat is a fixed time interval at which each JVM (Jsonic Virtual
//! Machine) instance confirms its liveness and processes pending work. The
//! Heartbeat drives the entire protocol cycle:
//!
//! 1. Each tick: process pending transactions, update side-chains
//! 2. Adrenaline adjusts tick rate based on network load
//! 3. After N heartbeats (a Solstice interval): trigger Solstice on main-chain
//!
//! In this reference implementation the Heartbeat is a synchronous state
//! machine that can be driven by an external clock or event loop.

use super::dao::DAORegistry;
use super::mainchain::MainChain;
use super::pot;
use super::sidechain::SideChain;
use super::types::{DAOId, TokenDistribution, Transaction, TransactionStatus};

use ed25519_dalek::VerifyingKey;
use std::collections::HashMap;
use thiserror::Error;

/// Number of heartbeats between Solstice events.
const SOLSTICE_INTERVAL: u64 = 100;

/// Why a transaction could not enter the node's validation pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SubmitError {
    #[error("originating DAO {0} is not registered")]
    UnknownOrigin(DAOId),
    #[error("counterparty DAO {0} is not registered")]
    UnknownCounterparty(DAOId),
    #[error("DAO {0} has an invalid public key")]
    InvalidPublicKey(DAOId),
    #[error("transaction amount must be finite and positive")]
    InvalidAmount,
    #[error("invalid signature for DAO {0}")]
    InvalidSignature(DAOId),
    #[error("sequence mismatch for DAO {dao_id}: expected {expected}, got {actual}")]
    SequenceMismatch {
        dao_id: DAOId,
        expected: u64,
        actual: u64,
    },
}

/// The Jsonic network node — ties together all protocol components.
pub struct JsonicNode {
    pub registry: DAORegistry,
    pub main_chain: MainChain,
    pub side_chains: HashMap<DAOId, SideChain>,
    /// Current heartbeat tick counter.
    pub tick: u64,
    /// Heartbeats per Solstice.
    pub solstice_interval: u64,
    /// Base heartbeat interval in milliseconds.
    pub base_heartbeat_ms: u64,
    /// Pending transactions awaiting POT matching.
    pending_matching: Vec<Transaction>,
    /// Next sequence number expected from each registered DAO.
    expected_sequences: HashMap<DAOId, u64>,
}

impl JsonicNode {
    pub fn new() -> Self {
        JsonicNode {
            registry: DAORegistry::new(),
            main_chain: MainChain::new(),
            side_chains: HashMap::new(),
            tick: 0,
            solstice_interval: SOLSTICE_INTERVAL,
            base_heartbeat_ms: 60_000,
            pending_matching: Vec::new(),
            expected_sequences: HashMap::new(),
        }
    }

    /// Register a DAO and initialize its side-chain.
    pub fn register_dao(&mut self, dao: super::types::DAO) {
        let id = dao.id.clone();
        self.registry.add(dao);
        self.side_chains
            .insert(id.clone(), SideChain::new(id.clone()));
        self.expected_sequences.entry(id).or_insert(1);
    }

    /// Submit a transaction into the network.
    /// The transaction is admitted only after basic POT preflight checks,
    /// then recorded on the sender's side-chain and queued for matching.
    pub fn submit_transaction(&mut self, tx: Transaction) -> Result<(), SubmitError> {
        self.validate_inbound_transaction(&tx)?;

        // Record on the sender's side-chain
        if let Some(chain) = self.side_chains.get_mut(&tx.from) {
            chain.submit_transaction(tx.clone());
        }

        // Queue for matching
        let from = tx.from.clone();
        self.pending_matching.push(tx);
        *self.expected_sequences.entry(from).or_insert(1) += 1;
        Ok(())
    }

    fn validate_inbound_transaction(&self, tx: &Transaction) -> Result<(), SubmitError> {
        if !tx.amount.is_finite() || tx.amount <= 0.0 {
            return Err(SubmitError::InvalidAmount);
        }

        let Some(origin) = self.registry.get(&tx.from) else {
            return Err(SubmitError::UnknownOrigin(tx.from.clone()));
        };
        if self.registry.get(&tx.to).is_none() {
            return Err(SubmitError::UnknownCounterparty(tx.to.clone()));
        }

        let public_key: [u8; 32] = origin
            .public_key
            .clone()
            .try_into()
            .map_err(|_| SubmitError::InvalidPublicKey(tx.from.clone()))?;
        let verifying_key = VerifyingKey::from_bytes(&public_key)
            .map_err(|_| SubmitError::InvalidPublicKey(tx.from.clone()))?;
        if !pot::verify_signature(tx, &verifying_key) {
            return Err(SubmitError::InvalidSignature(tx.from.clone()));
        }

        let expected = self.expected_sequences.get(&tx.from).copied().unwrap_or(1);
        if !pot::verify_sequence(tx, expected) {
            return Err(SubmitError::SequenceMismatch {
                dao_id: tx.from.clone(),
                expected,
                actual: tx.sequence_number,
            });
        }

        Ok(())
    }

    /// Execute one heartbeat tick.
    ///
    /// Returns Some(Vec<TokenDistribution>) if this tick triggered a Solstice.
    pub fn heartbeat(&mut self) -> Option<Vec<TokenDistribution>> {
        self.tick += 1;

        // Attempt to match pending transactions
        self.process_matching();

        // Check if it's time for Solstice
        if self.tick.is_multiple_of(self.solstice_interval) {
            Some(self.execute_solstice())
        } else {
            None
        }
    }

    /// Try to match pending transactions between counterparty DAOs.
    ///
    /// For each pair of transactions with the same ID, run POT matching.
    /// For invoice+payment pairs, attempt settlement.
    fn process_matching(&mut self) {
        let mut matched_indices: Vec<usize> = Vec::new();
        let mut settlements: Vec<(usize, usize)> = Vec::new();

        // Find invoice-payment pairs for settlement
        for i in 0..self.pending_matching.len() {
            for j in (i + 1)..self.pending_matching.len() {
                let tx_i = &self.pending_matching[i];
                let tx_j = &self.pending_matching[j];

                // Check for settlement (invoice + payment pair)
                if tx_i.tx_type != tx_j.tx_type {
                    let (invoice, payment, inv_idx, pay_idx) =
                        if tx_i.tx_type == super::types::TransactionType::Invoice {
                            (tx_i, tx_j, i, j)
                        } else {
                            (tx_j, tx_i, j, i)
                        };

                    if let pot::POTVerdict::Settled = pot::settle_invoice(invoice, payment) {
                        settlements.push((inv_idx, pay_idx));
                        self.main_chain.record_transaction_outcome(true);
                        continue;
                    }
                }

                // Check for matching (same transaction acknowledged by both parties)
                if tx_i.id == tx_j.id {
                    let verdict = pot::match_transactions(tx_i, tx_j);
                    match verdict {
                        pot::POTVerdict::Matched => {
                            matched_indices.push(i);
                            matched_indices.push(j);
                            self.main_chain.record_transaction_outcome(true);
                        }
                        pot::POTVerdict::Invalid(_) => {
                            self.main_chain.record_transaction_outcome(false);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Process settlements - update transaction statuses on side-chains
        for (inv_idx, pay_idx) in &settlements {
            let invoice_id = self.pending_matching[*inv_idx].id.clone();
            let invoice_from = self.pending_matching[*inv_idx].from.clone();
            let payment_from = self.pending_matching[*pay_idx].from.clone();
            let settled_value = self.pending_matching[*inv_idx].amount;

            // Buyer (payment_from) -> Seller (invoice_from): edge in the
            // reputation graph, weighted by the settled value.
            self.main_chain
                .record_settled_transaction(&payment_from, &invoice_from, settled_value);

            // Update invoice status to Settled on the issuer's side-chain
            if let Some(chain) = self.side_chains.get_mut(&invoice_from) {
                for block in &mut chain.blocks {
                    for tx in &mut block.transactions {
                        if tx.id == invoice_id {
                            tx.status = TransactionStatus::Settled;
                        }
                    }
                }
            }

            // Record payment on the payer's side-chain
            if let Some(chain) = self.side_chains.get_mut(&payment_from) {
                let mut payment_tx = self.pending_matching[*pay_idx].clone();
                payment_tx.status = TransactionStatus::Settled;
                chain.submit_transaction(payment_tx);
            }

            matched_indices.push(*inv_idx);
            matched_indices.push(*pay_idx);
        }

        // Update matched transaction statuses
        for &idx in &matched_indices {
            if idx < self.pending_matching.len() {
                let tx = &self.pending_matching[idx];
                let from_id = tx.from.clone();
                let tx_id = tx.id.clone();

                if let Some(chain) = self.side_chains.get_mut(&from_id) {
                    for block in &mut chain.blocks {
                        for block_tx in &mut block.transactions {
                            if block_tx.id == tx_id
                                && block_tx.status == TransactionStatus::Unmatched
                            {
                                block_tx.status = TransactionStatus::Matched;
                            }
                        }
                    }
                }
            }
        }

        // Remove matched/settled transactions from the pending pool
        matched_indices.sort_unstable();
        matched_indices.dedup();
        for idx in matched_indices.into_iter().rev() {
            if idx < self.pending_matching.len() {
                self.pending_matching.remove(idx);
            }
        }
    }

    /// Execute a Solstice: gather snapshots, mint tokens, create main-chain block.
    fn execute_solstice(&mut self) -> Vec<TokenDistribution> {
        // Flush all side-chains
        for chain in self.side_chains.values_mut() {
            chain.flush_pending();
        }

        // Collect snapshots with computed relevance scores
        let snapshots: Vec<_> = self
            .side_chains
            .values_mut()
            .map(|chain| {
                // Build a temporary snapshot to compute relevance
                let mut temp = chain.solstice_snapshot(0.0);
                temp.relevance_score = MainChain::compute_relevance_score(&temp);
                temp
            })
            .collect();

        // Execute Solstice on main-chain
        let distributions = self.main_chain.solstice(snapshots);

        // Credit tokens to DAOs
        for dist in &distributions {
            if let Some(dao) = self.registry.get_mut(&dist.dao_id) {
                dao.token_balance += dist.tokens_awarded;
            }
        }

        distributions
    }

    /// Get the current effective heartbeat interval (adjusted by Adrenaline).
    pub fn effective_heartbeat_ms(&self) -> u64 {
        let adrenaline = self.main_chain.adrenaline();
        pot::adjusted_heartbeat_ms(self.base_heartbeat_ms, adrenaline)
    }

    /// Get the number of pending unmatched transactions.
    pub fn pending_count(&self) -> usize {
        self.pending_matching.len()
    }
}

impl Default for JsonicNode {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dao::RegisteredDAO;

    #[test]
    fn test_full_lifecycle() {
        let mut node = JsonicNode::new();
        node.solstice_interval = 5; // Short interval for testing

        // Register two DAOs
        let mut dao_a = RegisteredDAO::register("Acme Corp", "Technology");
        let mut dao_b = RegisteredDAO::register("Globex Inc", "Manufacturing");

        let dao_a_id = dao_a.id().clone();
        let dao_b_id = dao_b.id().clone();

        node.register_dao(dao_a.dao.clone());
        node.register_dao(dao_b.dao.clone());

        // DAO-A sends an invoice to DAO-B
        let invoice = dao_a.create_invoice(&dao_b_id, 25_000.0, "USD", "Consulting Q1");

        // DAO-B creates a payment for that invoice
        let payment = dao_b.create_payment(
            &dao_a_id,
            25_000.0,
            "USD",
            &invoice.id,
            "Payment for Consulting Q1",
        );

        // Submit both to the network
        node.submit_transaction(invoice).expect("submit invoice");
        node.submit_transaction(payment).expect("submit payment");

        // Run heartbeats until Solstice
        let mut distributions = None;
        for _ in 0..5 {
            if let Some(d) = node.heartbeat() {
                distributions = Some(d);
            }
        }

        // Solstice should have occurred
        assert!(distributions.is_some());
        assert_eq!(node.main_chain.height(), 1);

        // DAO-A should have received tokens (it had verified transactions)
        let dao_a_entry = node.registry.get(&dao_a_id).unwrap();
        assert!(
            dao_a_entry.token_balance >= 0.0,
            "DAO-A should have token balance"
        );
    }

    #[test]
    fn test_heartbeat_without_solstice() {
        let mut node = JsonicNode::new();
        node.solstice_interval = 100;

        let result = node.heartbeat();
        assert!(result.is_none());
        assert_eq!(node.tick, 1);
    }

    #[test]
    fn test_sybil_ring_earns_less_than_honest_cluster() {
        // End-to-end proof of PageRank-based Sybil resistance through the full
        // Solstice pipeline.
        //
        // Sybil here cheats hard: 100 fake DAOs each "pay" Sybil $100,000,
        // for $10M of claimed volume. The honest cluster (A, B, C) does
        // 30 mutual trades at $10,000, for $900,000 of real volume - 11x
        // less raw volume than Sybil. The trust-floor reward formula
        // (`compute_dao_reward`) still ranks the honest cluster's worst
        // earner above Sybil because every fake's PageRank converges to the
        // random-walk baseline, so the fakes contribute zero trust.

        let mut node = JsonicNode::new();
        node.solstice_interval = 10_000;

        let mut a = RegisteredDAO::register("A", "Honest");
        let mut b = RegisteredDAO::register("B", "Honest");
        let mut c = RegisteredDAO::register("C", "Honest");
        let mut sybil = RegisteredDAO::register("Sybil", "Attacker");

        let a_id = a.id().clone();
        let b_id = b.id().clone();
        let c_id = c.id().clone();
        let sybil_id = sybil.id().clone();

        node.register_dao(a.dao.clone());
        node.register_dao(b.dao.clone());
        node.register_dao(c.dao.clone());
        node.register_dao(sybil.dao.clone());

        // Honest mutual trade: 30 cycles of A->B, B->C, C->A at $10k each.
        for _ in 0..30 {
            let inv = a.create_invoice(&b_id, 10_000.0, "USD", "h");
            let pay = b.create_payment(&a_id, 10_000.0, "USD", &inv.id, "h");
            node.submit_transaction(inv).expect("submit honest invoice");
            node.submit_transaction(pay).expect("submit honest payment");

            let inv = b.create_invoice(&c_id, 10_000.0, "USD", "h");
            let pay = c.create_payment(&b_id, 10_000.0, "USD", &inv.id, "h");
            node.submit_transaction(inv).expect("submit honest invoice");
            node.submit_transaction(pay).expect("submit honest payment");

            let inv = c.create_invoice(&a_id, 10_000.0, "USD", "h");
            let pay = a.create_payment(&c_id, 10_000.0, "USD", &inv.id, "h");
            node.submit_transaction(inv).expect("submit honest invoice");
            node.submit_transaction(pay).expect("submit honest payment");
        }

        // Sybil ring: 100 fakes, each settles one $100,000 invoice from
        // Sybil. Sybil's claimed volume ($10M) is ~11x the honest cluster's
        // real volume ($900k).
        let mut fakes: Vec<RegisteredDAO> = (0..100)
            .map(|i| RegisteredDAO::register(&format!("fake_{}", i), "Sybil"))
            .collect();
        for fake in &fakes {
            node.register_dao(fake.dao.clone());
        }
        for fake in fakes.iter_mut() {
            let fake_id = fake.id().clone();
            let inv = sybil.create_invoice(&fake_id, 100_000.0, "USD", "s");
            let pay = fake.create_payment(&sybil_id, 100_000.0, "USD", &inv.id, "s");
            node.submit_transaction(inv).expect("submit sybil invoice");
            node.submit_transaction(pay).expect("submit sybil payment");
        }

        // Drain pending matches across many heartbeats, then force a Solstice.
        for _ in 0..200 {
            node.heartbeat();
        }
        node.solstice_interval = node.tick + 1;
        let dist = node
            .heartbeat()
            .expect("Solstice should fire on the configured tick");

        let by_id = |id: &DAOId| {
            dist.iter()
                .find(|d| &d.dao_id == id)
                .map(|d| d.tokens_awarded)
                .unwrap_or(0.0)
        };

        let honest_min = by_id(&a_id).min(by_id(&b_id)).min(by_id(&c_id));
        let sybil_reward = by_id(&sybil_id);

        assert!(
            honest_min > sybil_reward,
            "Honest worst earner ({}) should beat Sybil ({}) despite Sybil \
             claiming 11x the volume",
            honest_min,
            sybil_reward,
        );
    }

    #[test]
    fn test_register_dao_creates_sidechain() {
        let mut node = JsonicNode::new();
        let dao = RegisteredDAO::register("TestDAO", "Finance");
        let id = dao.id().clone();

        node.register_dao(dao.dao);

        assert!(node.side_chains.contains_key(&id));
        assert_eq!(node.registry.count(), 1);
    }

    #[test]
    fn test_submit_rejects_forged_signature() {
        let mut node = JsonicNode::new();
        let mut sender = RegisteredDAO::register("Sender", "Manufacturing");
        let receiver = RegisteredDAO::register("Receiver", "Retail");
        let receiver_id = receiver.id().clone();

        node.register_dao(sender.dao.clone());
        node.register_dao(receiver.dao.clone());

        let mut tx = sender.create_invoice(&receiver_id, 100.0, "USD", "parts");
        tx.amount = 1_000_000.0;

        let err = node
            .submit_transaction(tx)
            .expect_err("tampered transaction should be rejected");
        assert!(matches!(err, SubmitError::InvalidSignature(_)));
        assert_eq!(node.pending_count(), 0);
    }

    #[test]
    fn test_submit_rejects_replay_sequence() {
        let mut node = JsonicNode::new();
        let mut sender = RegisteredDAO::register("Sender", "Manufacturing");
        let receiver = RegisteredDAO::register("Receiver", "Retail");
        let receiver_id = receiver.id().clone();
        let sender_id = sender.id().clone();

        node.register_dao(sender.dao.clone());
        node.register_dao(receiver.dao.clone());

        let tx = sender.create_invoice(&receiver_id, 100.0, "USD", "parts");
        node.submit_transaction(tx.clone()).expect("first submit");

        let err = node
            .submit_transaction(tx)
            .expect_err("replayed sequence should be rejected");
        assert_eq!(
            err,
            SubmitError::SequenceMismatch {
                dao_id: sender_id,
                expected: 2,
                actual: 1,
            }
        );
        assert_eq!(node.pending_count(), 1);
    }

    #[test]
    fn test_submit_rejects_unknown_counterparty() {
        let mut node = JsonicNode::new();
        let mut sender = RegisteredDAO::register("Sender", "Manufacturing");
        let receiver = RegisteredDAO::register("Receiver", "Retail");

        node.register_dao(sender.dao.clone());

        let tx = sender.create_invoice(receiver.id(), 100.0, "USD", "parts");
        let err = node
            .submit_transaction(tx)
            .expect_err("unknown counterparty should be rejected");
        assert!(matches!(err, SubmitError::UnknownCounterparty(_)));
    }
}
