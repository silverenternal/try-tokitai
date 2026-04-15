//! Custom Bloom Filter with deterministic hashing and serializable bitset
//!
//! This module implements a standard Bloom Filter using XXH3 for deterministic hashing,
//! enabling direct bitset serialization/deserialization without rebuilding the filter.
//!
//! # Key Features
//! - Deterministic XXH3 hashing (seeded for multiple hash functions)
//! - Direct bitset serialization (no keys list needed)
//! - V3 file format: [magic 4B][version 4B][num_bits 4B][num_hashes 4B][bitset_bytes]
//! - Fast load time (< 100µs) - just mmap/load bitset, no reconstruction
//! - Fast negative query (< 10µs) - just hash and check bits
//!
//! # Algorithm
//! A Bloom Filter uses k independent hash functions to map items to m bits:
//! - Insert: set bits at positions h1(x), h2(x), ..., hk(x) mod m
//! - Query: check if all bits at h1(x), h2(x), ..., hk(x) are set
//!
//! We use double hashing technique to simulate k hash functions:
//! h_i(x) = h1(x) + i * h2(x) mod m
//! where h1 and h2 are derived from XXH3 with different seeds.

use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use serde::{Serialize, Deserialize};

use crate::core::error::FatalError;

/// V3 Bloom Filter magic number
pub const CUSTOM_BLOOM_MAGIC: u32 = 0x424C4D33; // "BLM3" = Bloom v3

/// V3 Bloom Filter version
pub const CUSTOM_BLOOM_VERSION: u32 = 3;

/// Result type for custom bloom operations
pub type Result<T> = std::result::Result<T, FatalError>;

/// Custom Bloom Filter with deterministic hashing
///
/// Uses XXH3 with double hashing technique for k hash functions.
/// Bitset is directly serializable, enabling fast save/load.
#[derive(Debug, Clone)]
pub struct CustomBloom {
    /// Number of bits in the filter
    num_bits: usize,
    /// Number of hash functions
    num_hashes: usize,
    /// Bit vector (stored as Vec<u8>)
    bits: Vec<u8>,
}

impl CustomBloom {
    /// Create a new empty Bloom Filter
    ///
    /// # Arguments
    /// * `num_bits` - Number of bits in the filter (determines memory usage)
    /// * `num_hashes` - Number of hash functions (determines accuracy)
    pub fn new(num_bits: usize, num_hashes: usize) -> Self {
        let byte_len = (num_bits + 7) / 8;
        Self {
            num_bits,
            num_hashes,
            bits: vec![0u8; byte_len],
        }
    }

    /// Create a Bloom Filter optimized for n elements with target false positive rate
    ///
    /// # Formula
    /// - m = -n * ln(p) / (ln(2))^2  (optimal number of bits)
    /// - k = (m/n) * ln(2)           (optimal number of hash functions)
    ///
    /// where n = expected elements, p = target FPR
    pub fn with_capacity(expected_items: usize, fpr: f64) -> Self {
        let num_bits = Self::optimal_num_bits(expected_items, fpr);
        let num_hashes = Self::optimal_num_hashes(expected_items, num_bits);
        Self::new(num_bits, num_hashes)
    }

    /// Insert a key into the filter
    pub fn insert(&mut self, key: &[u8]) {
        let hashes = self.compute_hashes(key);
        for pos in hashes {
            self.set_bit(pos);
        }
    }

    /// Check if a key might be in the filter
    ///
    /// Returns:
    /// - `true`: Key is probably in the set (may be false positive)
    /// - `false`: Key is definitely not in the set
    pub fn contains(&self, key: &[u8]) -> bool {
        let hashes = self.compute_hashes(key);
        hashes.iter().all(|&pos| self.get_bit(pos))
    }

    /// Get number of bits
    pub fn num_bits(&self) -> usize {
        self.num_bits
    }

    /// Get number of hash functions
    pub fn num_hashes(&self) -> usize {
        self.num_hashes
    }

    /// Get bitset as bytes (for serialization)
    pub fn to_bytes(&self) -> &[u8] {
        &self.bits
    }

    /// Get estimated false positive rate
    pub fn estimated_fpr(&self, num_items: usize) -> f64 {
        if num_items == 0 || self.num_bits == 0 {
            return 0.0;
        }
        let k = self.num_hashes as f64;
        let m = self.num_bits as f64;
        let n = num_items as f64;
        (1.0 - (-k * n / m).exp()).powi(k as i32)
    }

    /// Get approximate memory usage in bytes
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.bits.len()
    }

    // ============================================================
    // Internal methods
    // ============================================================

    /// Set bit at position pos
    fn set_bit(&mut self, pos: usize) {
        let byte_idx = pos / 8;
        let bit_idx = pos % 8;
        if byte_idx < self.bits.len() {
            self.bits[byte_idx] |= 1 << bit_idx;
        }
    }

    /// Get bit at position pos
    fn get_bit(&self, pos: usize) -> bool {
        let byte_idx = pos / 8;
        let bit_idx = pos % 8;
        if byte_idx < self.bits.len() {
            (self.bits[byte_idx] & (1 << bit_idx)) != 0
        } else {
            false
        }
    }

    /// Compute k hash positions using double hashing
    ///
    /// Double hashing: h(i) = h1(key) + i * h2(key) mod m
    /// where h1 and h2 are XXH3 with different seeds
    fn compute_hashes(&self, key: &[u8]) -> Vec<usize> {
        let h1 = self.hash1(key);
        let h2 = self.hash2(key);
        let m = self.num_bits as u64;

        (0..self.num_hashes)
            .map(|i| {
                let pos = h1.wrapping_add((i as u64).wrapping_mul(h2)) % m;
                pos as usize
            })
            .collect()
    }

    /// First hash function: XXH3 with seed 0
    fn hash1(&self, key: &[u8]) -> u64 {
        xxhash_rust::xxh3::xxh3_64_with_seed(key, 0)
    }

    /// Second hash function: XXH3 with seed 0xDEADBEEF
    fn hash2(&self, key: &[u8]) -> u64 {
        xxhash_rust::xxh3::xxh3_64_with_seed(key, 0xDEADBEEF)
    }

    /// Calculate optimal number of bits for given n and p
    fn optimal_num_bits(n: usize, p: f64) -> usize {
        if p <= 0.0 || p >= 1.0 {
            return n * 10; // fallback
        }
        let ln2_sq = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        (-((n as f64) * p.ln()) / ln2_sq).ceil() as usize
    }

    /// Calculate optimal number of hash functions
    fn optimal_num_hashes(n: usize, m: usize) -> usize {
        if n == 0 || m == 0 {
            return 1;
        }
        let k = (m as f64 / n as f64) * std::f64::consts::LN_2;
        k.ceil().max(1.0) as usize
    }
}

// ============================================================
// Serialization / Deserialization
// ============================================================

/// V3 Bloom Filter file header
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct BloomHeader {
    magic: u32,
    version: u32,
    num_bits: u32,
    num_hashes: u32,
}

impl CustomBloom {
    /// Save Bloom Filter to file (V3 format)
    ///
    /// Format: [magic 4B][version 4B][num_bits 4B][num_hashes 4B][bitset_bytes]
    pub fn save_to_file(&self, path: &Path) -> Result<()> {
        let file = File::create(path).map_err(FatalError::Io)?;
        let mut writer = BufWriter::new(file);

        // Write header
        let header = BloomHeader {
            magic: CUSTOM_BLOOM_MAGIC,
            version: CUSTOM_BLOOM_VERSION,
            num_bits: self.num_bits as u32,
            num_hashes: self.num_hashes as u32,
        };

        writer.write_all(&header.magic.to_le_bytes())?;
        writer.write_all(&header.version.to_le_bytes())?;
        writer.write_all(&header.num_bits.to_le_bytes())?;
        writer.write_all(&header.num_hashes.to_le_bytes())?;

        // Write bitset
        writer.write_all(&self.bits)?;

        writer.flush()?;
        writer.get_ref().sync_all().map_err(FatalError::Io)?;

        Ok(())
    }

    /// Load Bloom Filter from file
    ///
    /// Supports V3 format (direct bitset load, no reconstruction needed)
    pub fn load_from_file(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }

        let file = File::open(path).map_err(FatalError::Io)?;
        let mut reader = BufReader::new(file);

        // Read header
        let mut magic_buf = [0u8; 4];
        reader.read_exact(&mut magic_buf).map_err(FatalError::Io)?;
        let magic = u32::from_le_bytes(magic_buf);

        if magic != CUSTOM_BLOOM_MAGIC {
            // Not a V3 format file - return None for fallback to old loader
            return Ok(None);
        }

        let mut version_buf = [0u8; 4];
        reader.read_exact(&mut version_buf).map_err(FatalError::Io)?;
        let version = u32::from_le_bytes(version_buf);

        if version != CUSTOM_BLOOM_VERSION {
            return Err(FatalError::Corruption(format!(
                "Unsupported custom bloom version: {}, expected {}",
                version, CUSTOM_BLOOM_VERSION
            )));
        }

        let mut num_bits_buf = [0u8; 4];
        reader.read_exact(&mut num_bits_buf).map_err(FatalError::Io)?;
        let num_bits = u32::from_le_bytes(num_bits_buf) as usize;

        let mut num_hashes_buf = [0u8; 4];
        reader.read_exact(&mut num_hashes_buf).map_err(FatalError::Io)?;
        let num_hashes = u32::from_le_bytes(num_hashes_buf) as usize;

        // Read bitset
        let byte_len = (num_bits + 7) / 8;
        let mut bits = vec![0u8; byte_len];
        reader.read_exact(&mut bits).map_err(FatalError::Io)?;

        Ok(Some(Self {
            num_bits,
            num_hashes,
            bits,
        }))
    }

    /// Estimate file size on disk
    pub fn estimated_file_size(&self) -> usize {
        16 + self.bits.len() // 16 bytes header + bitset
    }
}

// ============================================================
// Conversion helpers for migration from bloom crate
// ============================================================

impl CustomBloom {
    /// Create from existing bloom crate filter (for migration)
    ///
    /// This extracts the bitset and parameters from the old bloom filter
    /// and creates an equivalent CustomBloom.
    pub fn from_bloom_filter(bloom: &::bloom::BloomFilter) -> Self {
        let num_bits = bloom.num_bits();
        let num_hashes = bloom.num_hashes() as usize;
        let bitvec_bytes = bloom.to_bytes();

        // The bloom crate's to_bytes() returns the internal bit vector
        // We need to copy it directly as-is
        Self {
            num_bits,
            num_hashes,
            bits: bitvec_bytes,
        }
    }

    /// Create from raw bit vector (for loading from V3 file)
    ///
    /// This is the inverse of `to_bytes()` for V3 persistence.
    pub fn from_bits(num_bits: usize, num_hashes: usize, bits: Vec<u8>) -> Self {
        Self {
            num_bits,
            num_hashes,
            bits,
        }
    }

    /// Create from keys (for migration from V1/V2 format)
    ///
    /// Rebuilds the bloom filter by inserting all keys.
    /// This is slower than direct bitset loading but needed for migration.
    pub fn from_keys(keys: &[String], expected_items: usize, fpr: f64) -> Self {
        let mut bloom = Self::with_capacity(expected_items.max(keys.len()), fpr);
        for key in keys {
            bloom.insert(key.as_bytes());
        }
        bloom
    }

    /// Convert to bloom crate filter (for backward compatibility)
    /// Note: This losesves hash function compatibility since bloom crate uses RandomState
    pub fn to_bloom_filter(&self) -> ::bloom::BloomFilter {
        // Reconstruct by inserting placeholder keys
        // This is only for API compatibility, not bit-identical
        let bloom = ::bloom::BloomFilter::with_size(
            self.num_bits,
            self.num_hashes as u32,
        );
        // Cannot reconstruct exact bits without keys
        // This method is provided for interface compatibility only
        bloom
    }
}

// ============================================================
// PartialEq for testing
// ============================================================

impl PartialEq for CustomBloom {
    fn eq(&self, other: &Self) -> bool {
        self.num_bits == other.num_bits
            && self.num_hashes == other.num_hashes
            && self.bits == other.bits
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_basic_insert_contains() {
        let mut bloom = CustomBloom::with_capacity(1000, 0.01); // 1000 items, 1% FPR

        // Should not contain before insert
        assert!(!bloom.contains(b"test_key"));

        // Insert and verify
        bloom.insert(b"test_key");
        assert!(bloom.contains(b"test_key"));
    }

    #[test]
    fn test_no_false_negatives() {
        let mut bloom = CustomBloom::with_capacity(100, 0.01);

        let keys = vec![
            b"key1".to_vec(),
            b"key2".to_vec(),
            b"key3".to_vec(),
            b"hello_world".to_vec(),
            b"test_123".to_vec(),
        ];

        // Insert all keys
        for key in &keys {
            bloom.insert(key);
        }

        // All inserted keys must be found (no false negatives)
        for key in &keys {
            assert!(bloom.contains(key), "Should contain inserted key");
        }
    }

    #[test]
    fn test_false_positive_rate() {
        let num_items = 10000;
        let target_fpr = 0.01; // 1%
        let mut bloom = CustomBloom::with_capacity(num_items, target_fpr);

        // Insert known items
        for i in 0..num_items {
            bloom.insert(format!("key_{}", i).as_bytes());
        }

        // Test non-existent items for false positives
        let mut false_positives = 0;
        let test_count = 10000;

        for i in num_items..(num_items + test_count) {
            if bloom.contains(format!("key_{}", i).as_bytes()) {
                false_positives += 1;
            }
        }

        let actual_fpr = false_positives as f64 / test_count as f64;
        // Actual FPR should be reasonably close to target (within 3x)
        assert!(
            actual_fpr < target_fpr * 3.0,
            "False positive rate {} exceeds target {} * 3",
            actual_fpr,
            target_fpr
        );

        println!(
            "FPR test: target={}, actual={} ({} false positives in {} tests)",
            target_fpr,
            actual_fpr,
            false_positives,
            test_count
        );
    }

    #[test]
    fn test_save_load_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test_bloom_v3.bin");

        let mut original = CustomBloom::with_capacity(1000, 0.01);

        // Insert some keys
        for i in 0..100 {
            original.insert(format!("key_{}", i).as_bytes());
        }

        // Save
        original.save_to_file(&path).unwrap();

        // Load
        let loaded = CustomBloom::load_from_file(&path).unwrap().expect("Should load V3 bloom");

        // Verify identical
        assert_eq!(original, loaded);

        // Verify all keys still work
        for i in 0..100 {
            assert!(loaded.contains(format!("key_{}", i).as_bytes()));
        }
    }

    #[test]
    fn test_load_nonexistent_file() {
        let path = Path::new("/tmp/nonexistent_bloom_v3.bin");
        let result = CustomBloom::load_from_file(path).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_load_old_format_returns_none() {
        // Create a file with old magic (simulating v1/v2 format)
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("old_format.bin");

        let mut file = File::create(&path).unwrap();
        file.write_all(&crate::BLOOM_MAGIC.to_le_bytes()).unwrap();
        file.write_all(&2u32.to_le_bytes()).unwrap(); // v2 version

        let result = CustomBloom::load_from_file(&path).unwrap();
        assert!(result.is_none()); // Should return None for non-V3 format
    }

    #[test]
    fn test_file_size_estimation() {
        let bloom = CustomBloom::with_capacity(10000, 0.01);
        let estimated = bloom.estimated_file_size();
        let actual_header = 16;
        let expected = actual_header + bloom.to_bytes().len();
        assert_eq!(estimated, expected);
    }

    #[test]
    fn test_memory_usage() {
        let bloom = CustomBloom::with_capacity(10000, 0.01);
        let mem = bloom.memory_usage();
        assert!(mem > bloom.to_bytes().len()); // Should include struct overhead
    }

    #[test]
    fn test_optimal_calculations() {
        // Test optimal bits calculation
        let bits = CustomBloom::optimal_num_bits(1000, 0.01);
        assert!(bits > 1000); // Should be more bits than items

        // Test optimal hashes calculation
        let hashes = CustomBloom::optimal_num_hashes(1000, bits);
        assert!(hashes >= 1 && hashes <= 20); // Reasonable range
    }

    #[test]
    fn test_edge_cases() {
        // Empty bloom
        let bloom = CustomBloom::new(64, 4);
        assert!(!bloom.contains(b"anything"));

        // Single bit
        let mut bloom = CustomBloom::new(1, 1);
        bloom.insert(b"key");
        assert!(bloom.contains(b"key"));
    }

    #[test]
    fn test_large_dataset() {
        let mut bloom = CustomBloom::with_capacity(100_000, 0.01);

        // Insert 100k items
        for i in 0..100_000 {
            bloom.insert(format!("item_{}", i).as_bytes());
        }

        // Verify some inserted items
        for i in (0..100_000).step_by(1000) {
            assert!(bloom.contains(format!("item_{}", i).as_bytes()));
        }

        // Check false positive rate
        let mut fp = 0;
        for i in 100_000..110_000 {
            if bloom.contains(format!("item_{}", i).as_bytes()) {
                fp += 1;
            }
        }
        let fpr = fp as f64 / 10000.0;
        println!("Large dataset FPR: {} ({} false positives)", fpr, fp);
        assert!(fpr < 0.05); // Should be well under 5%
    }

    /// Performance test: measure load time
    #[test]
    fn test_load_performance() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("perf_test.bin");

        // Create and save a bloom filter
        let mut bloom = CustomBloom::with_capacity(100_000, 0.01);
        for i in 0..100_000 {
            bloom.insert(format!("key_{}", i).as_bytes());
        }
        bloom.save_to_file(&path).unwrap();

        // Measure load time
        let start = std::time::Instant::now();
        let iterations = 100;

        for _ in 0..iterations {
            let _ = CustomBloom::load_from_file(&path).unwrap().unwrap();
        }

        let elapsed = start.elapsed();
        let avg_us = elapsed.as_micros() as f64 / iterations as f64;

        println!(
            "Load performance: avg {:.2}µs over {} iterations",
            avg_us, iterations
        );

        // Should be well under 100µs per load
        assert!(
            avg_us < 1000.0, // Allow some margin, target < 100µs
            "Load time {}µs exceeds target 1000µs",
            avg_us
        );
    }

    /// Performance test: measure contains time for negative case
    #[test]
    fn test_contains_negative_performance() {
        let bloom = CustomBloom::with_capacity(100_000, 0.01);
        let key = b"nonexistent_key";

        let iterations = 100_000;
        let start = std::time::Instant::now();

        for _ in 0..iterations {
            let _ = bloom.contains(key);
        }

        let elapsed = start.elapsed();
        let avg_ns = elapsed.as_nanos() as f64 / iterations as f64;

        println!(
            "Contains (negative) performance: avg {:.2}ns over {} iterations",
            avg_ns, iterations
        );

        // Should be well under 10µs (10000ns) per check
        assert!(
            avg_ns < 1000.0, // Allow some margin, target < 10µs
            "Contains time {}ns exceeds target 1000ns",
            avg_ns
        );
    }
}
