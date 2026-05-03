//! Proof of Transaction (POT) validation engine.
//!
//! POT is Jsonic's consensus mechanism, inspired by real-world financial
//! auditing. It validates B2B transactions by:
//!
//! 1. **Signature verification** — the originating DAO signed the transaction.
//! 2. **Matching** — both counterparties (sender & receiver) acknowledge the
//!    transaction on their respective side-chains.
//! 3. **Sequential ID monitoring** — transaction sequence numbers are
//!    contiguous within each DAO's chain.
//! 4. **Settlement** — a Payment references a valid Invoice and the amounts
//!    match, completing the transaction lifecycle.
//!
//! Transactions that pass all checks become "Matched" or "Settled" and count
//! toward the DAO's relevance score and token rewards.

use ed25519_dalek::VerifyingKey;

use super::crypto;
use super::types::{Transaction, TransactionType};

/// Result of a POT validation check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum POTVerdict {
    /// Transaction is valid and matched between both DAOs.
    Matched,
    /// Invoice + Payment pair is fully settled.
    Settled,
    /// Validation failed with a reason.
    Invalid(String),
}

/// Verify the digital signature on a transaction.
pub fn verify_signature(tx: &Transaction, verifying_key: &VerifyingKey) -> bool {
    let payload = format!(
        "{}:{}:{}:{}:{}:{}:{}",
        tx.id,
        tx.from,
        tx.to,
        tx.amount,
        tx.currency,
        tx.sequence_number,
        tx.timestamp.to_rfc3339(),
    );
    crypto::verify(payload.as_bytes(), &tx.signature, verifying_key)
}

/// Validate that a transaction's sequence number follows the expected order.
pub fn verify_sequence(tx: &Transaction, expected_next: u64) -> bool {
    tx.sequence_number == expected_next
}

/// Attempt to match two transactions between counterparty DAOs.
///
/// For an invoice from DAO-A to DAO-B to be "matched", DAO-B must have a
/// corresponding acknowledgment. In practice this means both DAOs recorded
/// the same transaction (same ID, same amount, matching from/to).
pub fn match_transactions(sender_tx: &Transaction, receiver_tx: &Transaction) -> POTVerdict {
    // Must reference the same transaction ID
    if sender_tx.id != receiver_tx.id {
        return POTVerdict::Invalid("Transaction IDs do not match".to_string());
    }

    // Counterparty fields must mirror
    if sender_tx.from != receiver_tx.to || sender_tx.to != receiver_tx.from {
        // For matching, the receiver records the same from/to (acknowledging the sender's tx)
        // OR the fields are properly mirrored. We accept either convention.
        if sender_tx.from != receiver_tx.from || sender_tx.to != receiver_tx.to {
            return POTVerdict::Invalid("Counterparty DAO IDs do not match".to_string());
        }
    }

    // Amounts must agree
    if (sender_tx.amount - receiver_tx.amount).abs() > f64::EPSILON {
        return POTVerdict::Invalid(format!(
            "Amount mismatch: sender={}, receiver={}",
            sender_tx.amount, receiver_tx.amount
        ));
    }

    // Currency must agree
    if sender_tx.currency != receiver_tx.currency {
        return POTVerdict::Invalid("Currency mismatch".to_string());
    }

    POTVerdict::Matched
}

/// Attempt to settle an invoice with a payment.
///
/// A settlement occurs when:
/// - The payment references the invoice by ID
/// - The payment amount matches the invoice amount
/// - The payment is from the invoice's `to` DAO (the debtor pays)
/// - The payment is to the invoice's `from` DAO (the creditor receives)
pub fn settle_invoice(invoice: &Transaction, payment: &Transaction) -> POTVerdict {
    if invoice.tx_type != TransactionType::Invoice {
        return POTVerdict::Invalid("First transaction is not an Invoice".to_string());
    }
    if payment.tx_type != TransactionType::Payment {
        return POTVerdict::Invalid("Second transaction is not a Payment".to_string());
    }

    // Payment must reference this invoice
    match &payment.invoice_ref {
        Some(ref_id) if ref_id == &invoice.id => {}
        Some(ref_id) => {
            return POTVerdict::Invalid(format!(
                "Payment references invoice '{}', expected '{}'",
                ref_id, invoice.id
            ));
        }
        None => {
            return POTVerdict::Invalid("Payment does not reference any invoice".to_string());
        }
    }

    // The payer (payment.from) should be the invoice's recipient (invoice.to)
    if payment.from != invoice.to {
        return POTVerdict::Invalid("Payment sender does not match invoice recipient".to_string());
    }

    // The payee (payment.to) should be the invoice issuer (invoice.from)
    if payment.to != invoice.from {
        return POTVerdict::Invalid("Payment recipient does not match invoice issuer".to_string());
    }

    // Amounts must match
    if (invoice.amount - payment.amount).abs() > f64::EPSILON {
        return POTVerdict::Invalid(format!(
            "Payment amount ({}) does not match invoice amount ({})",
            payment.amount, invoice.amount
        ));
    }

    POTVerdict::Settled
}

/// Compute the Anxiety metric: ratio of invalid transactions to total.
/// Returns a value in (0, 1). Lower is healthier.
pub fn compute_anxiety(total_transactions: u64, invalid_transactions: u64) -> f64 {
    if total_transactions == 0 {
        return 0.0;
    }
    invalid_transactions as f64 / total_transactions as f64
}

/// Compute the Adrenaline factor based on transaction throughput.
/// As transactions per heartbeat increase, adrenaline rises, which
/// reduces the heartbeat interval to keep throughput consistent.
pub fn compute_adrenaline(
    transactions_per_heartbeat: u64,
    target_transactions_per_heartbeat: u64,
) -> f64 {
    if target_transactions_per_heartbeat == 0 {
        return 1.0;
    }
    transactions_per_heartbeat as f64 / target_transactions_per_heartbeat as f64
}

/// Adjust heartbeat interval based on adrenaline.
/// Higher adrenaline = shorter heartbeat (faster processing).
pub fn adjusted_heartbeat_ms(base_heartbeat_ms: u64, adrenaline: f64) -> u64 {
    if adrenaline <= 0.0 {
        return base_heartbeat_ms;
    }
    let adjusted = base_heartbeat_ms as f64 / adrenaline;
    // Floor at 100ms to prevent runaway
    (adjusted as u64).max(100)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dao::RegisteredDAO;

    #[test]
    fn test_verify_signature_valid() {
        let mut dao = RegisteredDAO::register("TestDAO", "Tech");
        let other_id = "recipient_dao".to_string();
        let invoice = dao.create_invoice(&other_id, 1000.0, "USD", "Test invoice");
        assert!(verify_signature(&invoice, &dao.keypair.verifying_key));
    }

    #[test]
    fn test_verify_signature_tampered() {
        let mut dao = RegisteredDAO::register("TestDAO", "Tech");
        let other_id = "recipient_dao".to_string();
        let mut invoice = dao.create_invoice(&other_id, 1000.0, "USD", "Test invoice");
        invoice.amount = 9999.0; // tamper
        assert!(!verify_signature(&invoice, &dao.keypair.verifying_key));
    }

    #[test]
    fn test_match_transactions_success() {
        let mut dao_a = RegisteredDAO::register("DAO-A", "Tech");
        let dao_b = RegisteredDAO::register("DAO-B", "Finance");

        let invoice = dao_a.create_invoice(dao_b.id(), 5000.0, "USD", "Services");

        // DAO-B acknowledges by recording the same transaction
        let mut ack = invoice.clone();
        ack.signature = crate::core::crypto::sign(
            format!(
                "{}:{}:{}:{}:{}:{}:{}",
                ack.id,
                ack.from,
                ack.to,
                ack.amount,
                ack.currency,
                ack.sequence_number,
                ack.timestamp.to_rfc3339()
            )
            .as_bytes(),
            &dao_b.keypair.signing_key,
        );

        let verdict = match_transactions(&invoice, &ack);
        assert_eq!(verdict, POTVerdict::Matched);
    }

    #[test]
    fn test_match_transactions_amount_mismatch() {
        let mut dao_a = RegisteredDAO::register("DAO-A", "Tech");
        let dao_b = RegisteredDAO::register("DAO-B", "Finance");

        let invoice = dao_a.create_invoice(dao_b.id(), 5000.0, "USD", "Services");
        let mut tampered = invoice.clone();
        tampered.amount = 4000.0;

        let verdict = match_transactions(&invoice, &tampered);
        assert!(matches!(verdict, POTVerdict::Invalid(_)));
    }

    #[test]
    fn test_settle_invoice_success() {
        let mut dao_a = RegisteredDAO::register("DAO-A", "Tech");
        let mut dao_b = RegisteredDAO::register("DAO-B", "Finance");

        let invoice = dao_a.create_invoice(dao_b.id(), 10_000.0, "EUR", "Q1 services");
        let payment = dao_b.create_payment(
            dao_a.id(),
            10_000.0,
            "EUR",
            &invoice.id,
            "Paying Q1 services",
        );

        let verdict = settle_invoice(&invoice, &payment);
        assert_eq!(verdict, POTVerdict::Settled);
    }

    #[test]
    fn test_settle_invoice_wrong_payer() {
        let mut dao_a = RegisteredDAO::register("DAO-A", "Tech");
        let dao_b = RegisteredDAO::register("DAO-B", "Finance");
        let mut dao_c = RegisteredDAO::register("DAO-C", "Retail");

        let invoice = dao_a.create_invoice(dao_b.id(), 10_000.0, "EUR", "Services");
        // DAO-C tries to pay an invoice meant for DAO-B
        let payment = dao_c.create_payment(dao_a.id(), 10_000.0, "EUR", &invoice.id, "Wrong payer");

        let verdict = settle_invoice(&invoice, &payment);
        assert!(matches!(verdict, POTVerdict::Invalid(_)));
    }

    #[test]
    fn test_settle_invoice_amount_mismatch() {
        let mut dao_a = RegisteredDAO::register("DAO-A", "Tech");
        let mut dao_b = RegisteredDAO::register("DAO-B", "Finance");

        let invoice = dao_a.create_invoice(dao_b.id(), 10_000.0, "EUR", "Services");
        let payment = dao_b.create_payment(
            dao_a.id(),
            5_000.0, // partial payment — not allowed for settlement
            "EUR",
            &invoice.id,
            "Partial payment",
        );

        let verdict = settle_invoice(&invoice, &payment);
        assert!(matches!(verdict, POTVerdict::Invalid(_)));
    }

    #[test]
    fn test_anxiety_computation() {
        assert_eq!(compute_anxiety(0, 0), 0.0);
        assert_eq!(compute_anxiety(100, 0), 0.0);
        assert_eq!(compute_anxiety(100, 10), 0.1);
        assert_eq!(compute_anxiety(100, 100), 1.0);
    }

    #[test]
    fn test_adrenaline_computation() {
        assert_eq!(compute_adrenaline(100, 100), 1.0);
        assert_eq!(compute_adrenaline(200, 100), 2.0);
        assert_eq!(compute_adrenaline(50, 100), 0.5);
    }

    #[test]
    fn test_adjusted_heartbeat() {
        // Normal load
        assert_eq!(adjusted_heartbeat_ms(1000, 1.0), 1000);
        // Double load → halve the interval
        assert_eq!(adjusted_heartbeat_ms(1000, 2.0), 500);
        // Floor at 100ms
        assert_eq!(adjusted_heartbeat_ms(1000, 100.0), 100);
    }
}
