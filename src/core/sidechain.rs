//! Side-chain implementation — one per DAO.
//!
//! Each DAO maintains its own side-chain which serves as a decentralized
//! ledger / balance sheet. Transactions accumulate in a pending pool until
//! their combined value reaches the Materiality threshold, at which point
//! a new block is mined and appended to the chain.
//!
//! At Solstice the side-chain produces a DAOSnapshot that is transferred
//! to the main-chain.

use chrono::Utc;

use super::crypto::{merkle_root, sha256_str};
use super::types::{
    BalanceSheet, BlockHeader, DAOId, DAOSnapshot, Hash, SideChainBlock, Transaction,
    TransactionStatus, TransactionType,
};

/// Default Materiality percentage (5%).
/// A new block is created when the pending transaction value exceeds
/// this fraction of the total existing on-chain value.
const DEFAULT_MATERIALITY: f64 = 0.05;

/// Minimum absolute value to trigger a block when the chain is empty.
const MINIMUM_BLOCK_VALUE: f64 = 100.0;

pub struct SideChain {
    pub dao_id: DAOId,
    pub blocks: Vec<SideChainBlock>,
    /// Transactions waiting to be included in the next block.
    pub pending: Vec<Transaction>,
    /// Materiality threshold as a fraction (0.0 – 1.0).
    pub materiality: f64,
    /// Running count of matched transactions since the last Solstice.
    matched_tx_count_since_solstice: u64,
    /// Running value of matched transactions since the last Solstice.
    matched_tx_value_since_solstice: f64,
}

impl SideChain {
    pub fn new(dao_id: DAOId) -> Self {
        SideChain {
            dao_id,
            blocks: Vec::new(),
            pending: Vec::new(),
            materiality: DEFAULT_MATERIALITY,
            matched_tx_count_since_solstice: 0,
            matched_tx_value_since_solstice: 0.0,
        }
    }

    /// Submit a transaction to the pending pool.
    /// Automatically generates a new block if Materiality is reached.
    pub fn submit_transaction(&mut self, tx: Transaction) -> Option<&SideChainBlock> {
        // Track matched/settled transactions for relevance scoring
        if tx.status == TransactionStatus::Matched || tx.status == TransactionStatus::Settled {
            self.matched_tx_count_since_solstice += 1;
            self.matched_tx_value_since_solstice += tx.amount;
        }

        self.pending.push(tx);

        if self.should_create_block() {
            self.create_block();
            self.blocks.last()
        } else {
            None
        }
    }

    /// Check whether the pending transactions have reached the Materiality
    /// threshold relative to the total on-chain value.
    fn should_create_block(&self) -> bool {
        let pending_value: f64 = self.pending.iter().map(|tx| tx.amount).sum();

        let total_on_chain_value = self.total_on_chain_value();

        if total_on_chain_value == 0.0 {
            // Chain is empty — use an absolute minimum
            return pending_value >= MINIMUM_BLOCK_VALUE;
        }

        pending_value >= total_on_chain_value * self.materiality
    }

    /// Sum of all transaction amounts across all existing blocks.
    fn total_on_chain_value(&self) -> f64 {
        self.blocks
            .iter()
            .flat_map(|b| &b.transactions)
            .map(|tx| tx.amount)
            .sum()
    }

    /// Create a new block from pending transactions and append it.
    fn create_block(&mut self) {
        let transactions: Vec<Transaction> = self.pending.drain(..).collect();
        let previous_hash = self
            .blocks
            .last()
            .map(|b| b.header.hash.clone())
            .unwrap_or_else(|| sha256_str("genesis"));

        let opening_balance = self
            .blocks
            .last()
            .map(|b| b.closing_balance.clone())
            .unwrap_or_default();

        let closing_balance = Self::apply_transactions(&opening_balance, &transactions);

        let tx_hashes: Vec<Hash> = transactions
            .iter()
            .map(|tx| sha256_str(&serde_json::to_string(tx).unwrap_or_default()))
            .collect();
        let merkle = merkle_root(&tx_hashes);

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

        let block = SideChainBlock {
            header: BlockHeader {
                index,
                previous_hash,
                timestamp,
                merkle_root: merkle,
                hash,
            },
            dao_id: self.dao_id.clone(),
            transactions,
            closing_balance,
        };

        self.blocks.push(block);
    }

    /// Apply a set of transactions to a balance sheet, producing updated balances.
    fn apply_transactions(
        opening: &BalanceSheet,
        transactions: &[Transaction],
    ) -> BalanceSheet {
        let mut balance = opening.clone();

        for tx in transactions {
            match (tx.tx_type, tx.status) {
                // An invoice we sent = accounts receivable
                (TransactionType::Invoice, TransactionStatus::Unmatched)
                | (TransactionType::Invoice, TransactionStatus::Matched) => {
                    balance.accounts_receivable += tx.amount;
                }
                // A settled invoice = revenue realized
                (TransactionType::Invoice, TransactionStatus::Settled) => {
                    balance.accounts_receivable -= tx.amount;
                    balance.revenue += tx.amount;
                }
                // A payment we made = expense
                (TransactionType::Payment, TransactionStatus::Matched)
                | (TransactionType::Payment, TransactionStatus::Settled) => {
                    balance.accounts_payable -= tx.amount;
                    balance.expenses += tx.amount;
                }
                // An unmatched payment = accounts payable
                (TransactionType::Payment, TransactionStatus::Unmatched) => {
                    balance.accounts_payable += tx.amount;
                }
                // Invalid transactions don't affect balances
                (_, TransactionStatus::Invalid) => {}
            }
        }

        balance
    }

    /// Current chain height (number of blocks).
    pub fn height(&self) -> u64 {
        self.blocks.len() as u64
    }

    /// Latest block hash, or the genesis hash if empty.
    pub fn latest_hash(&self) -> Hash {
        self.blocks
            .last()
            .map(|b| b.header.hash.clone())
            .unwrap_or_else(|| sha256_str("genesis"))
    }

    /// Current closing balance (from the latest block, or default).
    pub fn current_balance(&self) -> BalanceSheet {
        self.blocks
            .last()
            .map(|b| b.closing_balance.clone())
            .unwrap_or_default()
    }

    /// Produce a snapshot for the main-chain at Solstice.
    /// This also resets the per-Solstice counters.
    pub fn solstice_snapshot(&mut self, relevance_score: f64) -> DAOSnapshot {
        let snapshot = DAOSnapshot {
            dao_id: self.dao_id.clone(),
            side_chain_height: self.height(),
            latest_block_hash: self.latest_hash(),
            closing_balance: self.current_balance(),
            matched_tx_count: self.matched_tx_count_since_solstice,
            matched_tx_value: self.matched_tx_value_since_solstice,
            relevance_score,
        };

        // Reset per-Solstice counters
        self.matched_tx_count_since_solstice = 0;
        self.matched_tx_value_since_solstice = 0.0;

        snapshot
    }

    /// Force-flush any pending transactions into a block,
    /// regardless of Materiality. Used before Solstice.
    pub fn flush_pending(&mut self) {
        if !self.pending.is_empty() {
            self.create_block();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dao::RegisteredDAO;

    fn make_test_invoice(from: &str, to: &str, amount: f64) -> Transaction {
        let mut dao = RegisteredDAO::register("Test", "Test");
        let mut tx = dao.create_invoice(&to.to_string(), amount, "USD", "test");
        tx.from = from.to_string();
        tx.to = to.to_string();
        tx.status = TransactionStatus::Matched;
        tx
    }

    #[test]
    fn test_new_sidechain() {
        let chain = SideChain::new("dao1".to_string());
        assert_eq!(chain.height(), 0);
        assert!(chain.pending.is_empty());
        assert!(chain.blocks.is_empty());
    }

    #[test]
    fn test_block_creation_on_materiality() {
        let mut chain = SideChain::new("dao1".to_string());

        // Submit a transaction worth >= MINIMUM_BLOCK_VALUE on empty chain
        let tx = make_test_invoice("dao1", "dao2", 200.0);
        let result = chain.submit_transaction(tx);

        assert!(result.is_some());
        assert_eq!(chain.height(), 1);
        assert!(chain.pending.is_empty());
    }

    #[test]
    fn test_pending_below_threshold() {
        let mut chain = SideChain::new("dao1".to_string());

        // Submit below the minimum threshold for an empty chain
        let tx = make_test_invoice("dao1", "dao2", 10.0);
        let result = chain.submit_transaction(tx);

        assert!(result.is_none());
        assert_eq!(chain.height(), 0);
        assert_eq!(chain.pending.len(), 1);
    }

    #[test]
    fn test_balance_sheet_tracking() {
        let mut chain = SideChain::new("dao1".to_string());

        let tx = make_test_invoice("dao1", "dao2", 500.0);
        chain.submit_transaction(tx);

        let balance = chain.current_balance();
        assert_eq!(balance.accounts_receivable, 500.0);
    }

    #[test]
    fn test_flush_pending() {
        let mut chain = SideChain::new("dao1".to_string());

        let tx = make_test_invoice("dao1", "dao2", 10.0);
        chain.submit_transaction(tx);
        assert_eq!(chain.height(), 0);

        chain.flush_pending();
        assert_eq!(chain.height(), 1);
        assert!(chain.pending.is_empty());
    }

    #[test]
    fn test_block_hashes_chain() {
        let mut chain = SideChain::new("dao1".to_string());

        let tx1 = make_test_invoice("dao1", "dao2", 200.0);
        chain.submit_transaction(tx1);

        let tx2 = make_test_invoice("dao1", "dao2", 200.0);
        chain.submit_transaction(tx2);

        assert_eq!(chain.height(), 2);
        assert_eq!(
            chain.blocks[1].header.previous_hash,
            chain.blocks[0].header.hash
        );
    }

    #[test]
    fn test_solstice_snapshot() {
        let mut chain = SideChain::new("dao1".to_string());

        let tx = make_test_invoice("dao1", "dao2", 500.0);
        chain.submit_transaction(tx);

        let snapshot = chain.solstice_snapshot(0.85);
        assert_eq!(snapshot.dao_id, "dao1");
        assert_eq!(snapshot.side_chain_height, 1);
        assert_eq!(snapshot.matched_tx_count, 1);
        assert_eq!(snapshot.matched_tx_value, 500.0);
        assert_eq!(snapshot.relevance_score, 0.85);

        // Counters should be reset
        let snapshot2 = chain.solstice_snapshot(0.5);
        assert_eq!(snapshot2.matched_tx_count, 0);
        assert_eq!(snapshot2.matched_tx_value, 0.0);
    }
}
