//! Cryptographic utilities for the Jsonic protocol.
//!
//! - SHA-256 for hashing (block hashes, Merkle roots)
//! - Ed25519 for DAO identity key pairs and transaction signing

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use super::types::Hash;

// ---------------------------------------------------------------------------
// Hashing
// ---------------------------------------------------------------------------

/// SHA-256 hash of arbitrary bytes, returned as a hex string.
pub fn sha256(data: &[u8]) -> Hash {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// SHA-256 hash of a string.
pub fn sha256_str(data: &str) -> Hash {
    sha256(data.as_bytes())
}

/// Compute a Merkle root from an array of hashes.
/// Returns the hash of an empty string for an empty slice.
pub fn merkle_root(hashes: &[Hash]) -> Hash {
    if hashes.is_empty() {
        return sha256_str("");
    }
    if hashes.len() == 1 {
        return hashes[0].clone();
    }

    let mut next_level = Vec::new();
    for i in (0..hashes.len()).step_by(2) {
        let left = &hashes[i];
        let right = if i + 1 < hashes.len() {
            &hashes[i + 1]
        } else {
            left
        };
        next_level.push(sha256_str(&format!("{}{}", left, right)));
    }
    merkle_root(&next_level)
}

// ---------------------------------------------------------------------------
// Key Pairs & Signing (Ed25519)
// ---------------------------------------------------------------------------

/// An Ed25519 key pair for DAO identity.
pub struct KeyPair {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

/// Generate a new Ed25519 key pair.
pub fn generate_keypair() -> KeyPair {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    KeyPair {
        signing_key,
        verifying_key,
    }
}

/// Sign data with a signing key, returning the signature as bytes.
pub fn sign(data: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    let signature = signing_key.sign(data);
    signature.to_bytes().to_vec()
}

/// Verify a signature against data and a verifying (public) key.
pub fn verify(data: &[u8], signature: &[u8], verifying_key: &VerifyingKey) -> bool {
    let Ok(sig) = ed25519_dalek::Signature::from_slice(signature) else {
        return false;
    };
    verifying_key.verify(data, &sig).is_ok()
}

// ---------------------------------------------------------------------------
// ID Derivation
// ---------------------------------------------------------------------------

/// Derive a DAO ID from its public key.
/// The ID is the first 40 hex characters of SHA-256(public_key_bytes).
pub fn derive_dao_id(verifying_key: &VerifyingKey) -> String {
    let hash = sha256(verifying_key.as_bytes());
    hash[..40].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_deterministic() {
        let h1 = sha256_str("hello jsonic");
        let h2 = sha256_str("hello jsonic");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // 256 bits = 64 hex chars
    }

    #[test]
    fn test_sha256_different_inputs() {
        let h1 = sha256_str("hello");
        let h2 = sha256_str("world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_merkle_root_empty() {
        let root = merkle_root(&[]);
        assert_eq!(root, sha256_str(""));
    }

    #[test]
    fn test_merkle_root_single() {
        let h = sha256_str("tx1");
        let root = merkle_root(std::slice::from_ref(&h));
        assert_eq!(root, h);
    }

    #[test]
    fn test_merkle_root_two() {
        let h1 = sha256_str("tx1");
        let h2 = sha256_str("tx2");
        let root = merkle_root(&[h1.clone(), h2.clone()]);
        let expected = sha256_str(&format!("{}{}", h1, h2));
        assert_eq!(root, expected);
    }

    #[test]
    fn test_sign_and_verify() {
        let kp = generate_keypair();
        let data = b"invoice from DAO1 to DAO2: 10000 USD";
        let sig = sign(data, &kp.signing_key);
        assert!(verify(data, &sig, &kp.verifying_key));
    }

    #[test]
    fn test_verify_rejects_tampered_data() {
        let kp = generate_keypair();
        let sig = sign(b"original data", &kp.signing_key);
        assert!(!verify(b"tampered data", &sig, &kp.verifying_key));
    }

    #[test]
    fn test_verify_rejects_wrong_key() {
        let kp1 = generate_keypair();
        let kp2 = generate_keypair();
        let data = b"some data";
        let sig = sign(data, &kp1.signing_key);
        assert!(!verify(data, &sig, &kp2.verifying_key));
    }

    #[test]
    fn test_derive_dao_id() {
        let kp = generate_keypair();
        let id = derive_dao_id(&kp.verifying_key);
        assert_eq!(id.len(), 40);
        // Deterministic for the same key
        assert_eq!(id, derive_dao_id(&kp.verifying_key));
    }
}
