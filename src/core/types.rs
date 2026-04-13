//! Core types for the Jsonic B2B blockchain protocol.
//!
//! The type hierarchy mirrors the whitepaper architecture:
//!   DAO → Side-chain (per-DAO ledger) → Main-chain (global state)
//!   Transactions flow between DAOs and are validated via POT.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Identity
// ---------------------------------------------------------------------------

/// Hex-encoded identifier that uniquely identifies a DAO on-chain.
/// Derived from the first 40 hex chars of SHA-256(public_key).
pub type DAOId = String;

/// Hex-encoded SHA-256 hash.
pub type Hash = String;

// ---------------------------------------------------------------------------
// DAO
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAOProfile {
    /// Human-readable business name (pseudonymous).
    pub name: String,
    /// Industry or sector classification.
    pub sector: String,
    /// Registration timestamp.
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAO {
    pub id: DAOId,
    pub public_key: Vec<u8>,
    pub profile: DAOProfile,
    /// Current token balance held by this DAO.
    pub token_balance: f64,
}

// ---------------------------------------------------------------------------
// Transactions
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    Invoice,
    Payment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionStatus {
    /// Recorded by sender but not yet acknowledged by counterparty.
    Unmatched,
    /// Both DAOs acknowledge the transaction.
    Matched,
    /// Payment received against an invoice — transaction is complete.
    Settled,
    /// Flagged as invalid during POT validation.
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub tx_type: TransactionType,
    /// DAO that initiates (e.g. sends the invoice).
    pub from: DAOId,
    /// Counterparty DAO.
    pub to: DAOId,
    /// Monetary value in protocol-neutral units.
    pub amount: f64,
    /// ISO-4217 currency code for the original fiat denomination.
    pub currency: String,
    /// Free-text description / memo.
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub status: TransactionStatus,
    /// Ed25519 signature by the originating DAO.
    pub signature: Vec<u8>,
    /// If this is a Payment, the invoice transaction ID it settles.
    pub invoice_ref: Option<String>,
    /// Sequential ID within the originating DAO's side-chain.
    pub sequence_number: u64,
}

// ---------------------------------------------------------------------------
// Blocks
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    pub index: u64,
    pub previous_hash: Hash,
    pub timestamp: DateTime<Utc>,
    pub merkle_root: Hash,
    pub hash: Hash,
}

/// Side-chain block — lives on an individual DAO's chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SideChainBlock {
    pub header: BlockHeader,
    pub dao_id: DAOId,
    pub transactions: Vec<Transaction>,
    /// Running balance after applying this block's transactions.
    pub closing_balance: BalanceSheet,
}

/// Main-chain block — created at Solstice from all side-chain snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MainChainBlock {
    pub header: BlockHeader,
    /// Snapshots from every DAO side-chain included in this Solstice.
    pub dao_snapshots: Vec<DAOSnapshot>,
    /// Tokens minted and distributed in this Solstice.
    pub token_distribution: Vec<TokenDistribution>,
    /// Network health metrics at time of block creation.
    pub network_metrics: NetworkMetrics,
}

// ---------------------------------------------------------------------------
// Balances & Snapshots
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BalanceSheet {
    /// Total value of receivables (invoices sent, not yet paid).
    pub accounts_receivable: f64,
    /// Total value of payables (invoices received, not yet paid).
    pub accounts_payable: f64,
    /// Total revenue from settled transactions.
    pub revenue: f64,
    /// Total expenses from settled payments.
    pub expenses: f64,
}

impl BalanceSheet {
    /// Net position = revenue − expenses.
    pub fn net_position(&self) -> f64 {
        self.revenue - self.expenses
    }
}

/// Compact summary of a DAO's side-chain state at Solstice.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAOSnapshot {
    pub dao_id: DAOId,
    pub side_chain_height: u64,
    pub latest_block_hash: Hash,
    pub closing_balance: BalanceSheet,
    /// Count of matched (verified) transactions since last Solstice.
    pub matched_tx_count: u64,
    /// Total value of matched transactions since last Solstice.
    pub matched_tx_value: f64,
    /// DAO's relevance score used for token distribution.
    pub relevance_score: f64,
}

// ---------------------------------------------------------------------------
// Tokenomics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDistribution {
    pub dao_id: DAOId,
    pub tokens_awarded: f64,
    /// Breakdown of how the award was calculated.
    pub reason: String,
}

// ---------------------------------------------------------------------------
// Network Metrics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Total registered DAOs.
    pub total_daos: u64,
    /// Ratio of invalid/incomplete transactions — lower is healthier.
    /// Value in the open interval (0, 1).
    pub anxiety: f64,
    /// Current heartbeat interval in milliseconds.
    pub heartbeat_ms: u64,
    /// Adrenaline factor applied to heartbeat.
    pub adrenaline: f64,
    /// Total token supply after this block.
    pub total_token_supply: f64,
}
