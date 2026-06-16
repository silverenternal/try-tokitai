//! Privacy & Encryption Utilities
//!
//! - BLAKE3: content-addressed chunk IDs (deduplication) — 64 hex chars
//! - SHA256: integrity verification — 64 hex chars
//! - AES256: data encryption at rest (stub)

/// Compute BLAKE3 hash of data — 64-character hex output.
/// Used for content-addressed chunk deduplication.
pub fn blake3_hash(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Verify that data matches a previously computed BLAKE3 hash.
pub fn blake3_verify(data: &[u8], expected_hash: &str) -> bool {
    blake3_hash(data) == expected_hash
}

/// Compute SHA-256 digest of data — 64-character hex output.
/// Used for data integrity verification.
pub fn sha256_digest(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Verify data integrity against a SHA-256 hash.
pub fn sha256_verify(data: &[u8], expected: &str) -> bool {
    sha256_digest(data) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_output_length() {
        let hash = blake3_hash(b"hello");
        assert_eq!(hash.len(), 64, "BLAKE3 hex output must be 64 chars");
    }

    #[test]
    fn test_sha256_output_length() {
        let hash = sha256_digest(b"hello");
        assert_eq!(hash.len(), 64, "SHA-256 hex output must be 64 chars");
    }

    #[test]
    fn test_blake3_and_sha256_differ() {
        let data = b"hello world";
        let b3 = blake3_hash(data);
        let s2 = sha256_digest(data);
        assert_ne!(b3, s2, "BLAKE3 and SHA256 must produce different outputs for the same input");
    }

    #[test]
    fn test_blake3_deterministic() {
        let h1 = blake3_hash(b"test data");
        let h2 = blake3_hash(b"test data");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_blake3_different_inputs() {
        let h1 = blake3_hash(b"hello");
        let h2 = blake3_hash(b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_blake3_verify() {
        let data = b"verification test";
        let hash = blake3_hash(data);
        assert!(blake3_verify(data, &hash));
        assert!(!blake3_verify(b"wrong", &hash));
    }

    #[test]
    fn test_sha256_verify() {
        let data = b"integrity check";
        let hash = sha256_digest(data);
        assert!(sha256_verify(data, &hash));
        assert!(!sha256_verify(b"tampered", &hash));
    }

    #[test]
    fn test_known_sha256() {
        // SHA-256 of empty string is well-known
        let hash = sha256_digest(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
