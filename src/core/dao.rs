//! DAO (Decentralized Autonomous Organization) registration and management.
//!
//! In the Jsonic world, a DAO is the on-chain counterpart of a real-world
//! business. Businesses register on the network to become DAOs. Individuals
//! cannot become DAOs — the network is exclusively B2B.

use chrono::Utc;

use super::crypto::{derive_dao_id, generate_keypair, sign, KeyPair};
use super::types::{
    Transaction, TransactionStatus, TransactionType, DAOId, DAO, DAOProfile,
};

/// A registered DAO with its private signing key.
/// The signing key never leaves the DAO's node in a real deployment.
pub struct RegisteredDAO {
    pub dao: DAO,
    pub keypair: KeyPair,
    /// Next sequence number for transactions originating from this DAO.
    next_sequence: u64,
}

impl RegisteredDAO {
    /// Register a new DAO on the Jsonic network.
    pub fn register(name: &str, sector: &str) -> Self {
        let keypair = generate_keypair();
        let id = derive_dao_id(&keypair.verifying_key);
        let public_key = keypair.verifying_key.to_bytes().to_vec();

        let dao = DAO {
            id,
            public_key,
            profile: DAOProfile {
                name: name.to_string(),
                sector: sector.to_string(),
                registered_at: Utc::now(),
            },
            token_balance: 0.0,
        };

        RegisteredDAO {
            dao,
            keypair,
            next_sequence: 1,
        }
    }

    /// Get the DAO's unique identifier.
    pub fn id(&self) -> &DAOId {
        &self.dao.id
    }

    /// Create and sign an invoice to another DAO.
    pub fn create_invoice(
        &mut self,
        to: &DAOId,
        amount: f64,
        currency: &str,
        description: &str,
    ) -> Transaction {
        let tx_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let seq = self.next_sequence;
        self.next_sequence += 1;

        let signing_payload = Self::transaction_signing_payload(
            &tx_id,
            &self.dao.id,
            to,
            amount,
            currency,
            seq,
            &timestamp.to_rfc3339(),
        );

        let signature = sign(signing_payload.as_bytes(), &self.keypair.signing_key);

        Transaction {
            id: tx_id,
            tx_type: TransactionType::Invoice,
            from: self.dao.id.clone(),
            to: to.clone(),
            amount,
            currency: currency.to_string(),
            description: description.to_string(),
            timestamp,
            status: TransactionStatus::Unmatched,
            signature,
            invoice_ref: None,
            sequence_number: seq,
        }
    }

    /// Create and sign a payment against an existing invoice.
    pub fn create_payment(
        &mut self,
        to: &DAOId,
        amount: f64,
        currency: &str,
        invoice_id: &str,
        description: &str,
    ) -> Transaction {
        let tx_id = uuid::Uuid::new_v4().to_string();
        let timestamp = Utc::now();
        let seq = self.next_sequence;
        self.next_sequence += 1;

        let signing_payload = Self::transaction_signing_payload(
            &tx_id,
            &self.dao.id,
            to,
            amount,
            currency,
            seq,
            &timestamp.to_rfc3339(),
        );

        let signature = sign(signing_payload.as_bytes(), &self.keypair.signing_key);

        Transaction {
            id: tx_id,
            tx_type: TransactionType::Payment,
            from: self.dao.id.clone(),
            to: to.clone(),
            amount,
            currency: currency.to_string(),
            description: description.to_string(),
            timestamp,
            status: TransactionStatus::Unmatched,
            signature,
            invoice_ref: Some(invoice_id.to_string()),
            sequence_number: seq,
        }
    }

    /// Build the canonical byte string that is signed for a transaction.
    fn transaction_signing_payload(
        tx_id: &str,
        from: &str,
        to: &str,
        amount: f64,
        currency: &str,
        seq: u64,
        timestamp: &str,
    ) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}:{}",
            tx_id, from, to, amount, currency, seq, timestamp
        )
    }

    /// Verify that a transaction's signature is valid for this DAO.
    pub fn verify_transaction(&self, tx: &Transaction) -> bool {
        let payload = Self::transaction_signing_payload(
            &tx.id,
            &tx.from,
            &tx.to,
            tx.amount,
            &tx.currency,
            tx.sequence_number,
            &tx.timestamp.to_rfc3339(),
        );
        super::crypto::verify(
            payload.as_bytes(),
            &tx.signature,
            &self.keypair.verifying_key,
        )
    }
}

/// Registry that tracks all DAOs on the network.
pub struct DAORegistry {
    daos: Vec<DAO>,
}

impl DAORegistry {
    pub fn new() -> Self {
        DAORegistry { daos: Vec::new() }
    }

    /// Register a DAO in the network registry.
    pub fn add(&mut self, dao: DAO) {
        self.daos.push(dao);
    }

    /// Look up a DAO by its ID.
    pub fn get(&self, id: &DAOId) -> Option<&DAO> {
        self.daos.iter().find(|d| d.id == *id)
    }

    /// Get a mutable reference to a DAO by its ID.
    pub fn get_mut(&mut self, id: &DAOId) -> Option<&mut DAO> {
        self.daos.iter_mut().find(|d| d.id == *id)
    }

    /// Total number of registered DAOs.
    pub fn count(&self) -> u64 {
        self.daos.len() as u64
    }

    /// Iterate over all registered DAOs.
    pub fn iter(&self) -> impl Iterator<Item = &DAO> {
        self.daos.iter()
    }
}

impl Default for DAORegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_dao() {
        let dao = RegisteredDAO::register("Acme Corp", "Technology");
        assert_eq!(dao.dao.profile.name, "Acme Corp");
        assert_eq!(dao.dao.profile.sector, "Technology");
        assert_eq!(dao.dao.id.len(), 40);
        assert_eq!(dao.dao.token_balance, 0.0);
    }

    #[test]
    fn test_create_and_verify_invoice() {
        let mut sender = RegisteredDAO::register("Acme Corp", "Technology");
        let receiver = RegisteredDAO::register("Globex Inc", "Manufacturing");

        let invoice = sender.create_invoice(
            receiver.id(),
            50_000.0,
            "USD",
            "Q1 consulting services",
        );

        assert_eq!(invoice.tx_type, TransactionType::Invoice);
        assert_eq!(invoice.from, *sender.id());
        assert_eq!(invoice.to, *receiver.id());
        assert_eq!(invoice.amount, 50_000.0);
        assert_eq!(invoice.status, TransactionStatus::Unmatched);
        assert_eq!(invoice.sequence_number, 1);
        assert!(sender.verify_transaction(&invoice));
    }

    #[test]
    fn test_create_payment_references_invoice() {
        let mut sender = RegisteredDAO::register("Acme Corp", "Technology");
        let mut receiver = RegisteredDAO::register("Globex Inc", "Manufacturing");

        let invoice = sender.create_invoice(receiver.id(), 10_000.0, "EUR", "Invoice #001");
        let payment = receiver.create_payment(
            sender.id(),
            10_000.0,
            "EUR",
            &invoice.id,
            "Payment for Invoice #001",
        );

        assert_eq!(payment.tx_type, TransactionType::Payment);
        assert_eq!(payment.invoice_ref.as_deref(), Some(invoice.id.as_str()));
        assert!(receiver.verify_transaction(&payment));
    }

    #[test]
    fn test_sequence_numbers_increment() {
        let mut dao = RegisteredDAO::register("TestDAO", "Finance");
        let other_id = "deadbeef".to_string();

        let tx1 = dao.create_invoice(&other_id, 100.0, "USD", "First");
        let tx2 = dao.create_invoice(&other_id, 200.0, "USD", "Second");
        let tx3 = dao.create_invoice(&other_id, 300.0, "USD", "Third");

        assert_eq!(tx1.sequence_number, 1);
        assert_eq!(tx2.sequence_number, 2);
        assert_eq!(tx3.sequence_number, 3);
    }

    #[test]
    fn test_dao_registry() {
        let mut registry = DAORegistry::new();
        let dao1 = RegisteredDAO::register("DAO1", "Tech");
        let dao2 = RegisteredDAO::register("DAO2", "Finance");

        let id1 = dao1.id().clone();
        let id2 = dao2.id().clone();

        registry.add(dao1.dao);
        registry.add(dao2.dao);

        assert_eq!(registry.count(), 2);
        assert!(registry.get(&id1).is_some());
        assert!(registry.get(&id2).is_some());
        assert!(registry.get(&"nonexistent".to_string()).is_none());
    }
}
