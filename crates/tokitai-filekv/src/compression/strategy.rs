//! Compression strategy trait and implementations
//!
//! This module defines a unified `CompressionStrategy` trait and provides
//! implementations for Zstd, Snappy, and LZ4 compression algorithms.

use std::error::Error;

/// Compression strategy trait
///
/// All compression algorithms must implement this trait to be usable
/// by the compression factory and block compression functions.
pub trait CompressionStrategy: Send + Sync {
    /// Returns the name of this compression algorithm
    fn name(&self) -> &str;

    /// Compress data
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>;

    /// Decompress data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>>;
}

// ============================================================
// ZstdCompressor
// ============================================================

/// Zstandard compression implementation
pub struct ZstdCompressor {
    level: i32,
}

impl ZstdCompressor {
    /// Create a new Zstd compressor with the given compression level (1-22)
    pub fn new(level: i32) -> Self {
        Self {
            level: level.clamp(1, 22),
        }
    }
}

impl CompressionStrategy for ZstdCompressor {
    fn name(&self) -> &str {
        "zstd"
    }

    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let compressed = zstd::encode_all(data, self.level)
            .map_err(|e| format!("zstd compression failed: {}", e))?;
        Ok(compressed)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let decompressed = zstd::decode_all(data)
            .map_err(|e| format!("zstd decompression failed: {}", e))?;
        Ok(decompressed)
    }
}

// ============================================================
// SnappyCompressor
// ============================================================

/// Snappy compression implementation (low latency)
pub struct SnappyCompressor;

impl SnappyCompressor {
    /// Create a new Snappy compressor
    pub fn new(_level: i32) -> Self {
        // Snappy does not support compression levels
        Self
    }
}

impl CompressionStrategy for SnappyCompressor {
    fn name(&self) -> &str {
        "snappy"
    }

    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let compressed = snap::raw::max_compress_len(data.len());
        let mut output = vec![0u8; compressed];
        let written = snap::raw::Encoder::new()
            .compress(data, &mut output)
            .map_err(|e| format!("snappy compression failed: {}", e))?;
        output.truncate(written);
        Ok(output)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let len = snap::raw::decompress_len(data)
            .map_err(|e| format!("snappy decompress_len failed: {}", e))?;
        let mut output = vec![0u8; len];
        snap::raw::Decoder::new()
            .decompress(data, &mut output)
            .map_err(|e| format!("snappy decompression failed: {}", e))?;
        Ok(output)
    }
}

// ============================================================
// Lz4Compressor
// ============================================================

/// LZ4 compression implementation (high throughput)
pub struct Lz4Compressor {
    level: i32,
}

impl Lz4Compressor {
    /// Create a new LZ4 compressor with the given compression level (0-16, 0=fast)
    pub fn new(level: i32) -> Self {
        Self {
            level: level.clamp(0, 16),
        }
    }
}

impl CompressionStrategy for Lz4Compressor {
    fn name(&self) -> &str {
        "lz4"
    }

    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        // lz4 crate provides block-level compression
        // Use prepend_size=true to store original size in output for decompression
        let mode = if self.level <= 0 {
            lz4::block::CompressionMode::FAST(0)
        } else {
            lz4::block::CompressionMode::HIGHCOMPRESSION(self.level.min(12))
        };
        let compressed = lz4::block::compress(data, Some(mode), true)
            .map_err(|e| format!("lz4 compression failed: {}", e))?;
        Ok(compressed)
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        // With prepend_size=true during compression, decompress can use None
        let decompressed = lz4::block::decompress(data, None)
            .map_err(|e| format!("lz4 decompression failed: {}", e))?;
        Ok(decompressed)
    }
}

// ============================================================
// Compression algorithm ID for persistence
// ============================================================

/// Compression algorithm identifier stored in block header
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionAlgorithmId {
    /// No compression
    None = 0,
    /// Zstandard
    Zstd = 1,
    /// Snappy
    Snappy = 2,
    /// LZ4
    Lz4 = 3,
}

impl CompressionAlgorithmId {
    /// Create from u8 identifier
    pub fn from_u8(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::None),
            1 => Some(Self::Zstd),
            2 => Some(Self::Snappy),
            3 => Some(Self::Lz4),
            _ => None,
        }
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to test roundtrip for a given compressor
    fn test_roundtrip<C: CompressionStrategy>(compressor: &C, data: &[u8]) {
        let compressed = compressor.compress(data).expect("compress failed");
        let decompressed = compressor.decompress(&compressed).expect("decompress failed");
        assert_eq!(data, decompressed.as_slice(), "roundtrip failed for {}", compressor.name());
    }

    // ─── Basic roundtrip tests ───

    #[test]
    fn test_zstd_roundtrip_basic() {
        let compressor = ZstdCompressor::new(3);
        test_roundtrip(&compressor, b"Hello, World! This is a test message.");
    }

    #[test]
    fn test_snappy_roundtrip_basic() {
        let compressor = SnappyCompressor;
        test_roundtrip(&compressor, b"Hello, World! This is a test message.");
    }

    #[test]
    fn test_lz4_roundtrip_basic() {
        let compressor = Lz4Compressor::new(0);
        test_roundtrip(&compressor, b"Hello, World! This is a test message.");
    }

    // ─── Empty data tests ───

    #[test]
    fn test_zstd_empty() {
        let compressor = ZstdCompressor::new(3);
        test_roundtrip(&compressor, b"");
    }

    #[test]
    fn test_snappy_empty() {
        let compressor = SnappyCompressor;
        test_roundtrip(&compressor, b"");
    }

    #[test]
    fn test_lz4_empty() {
        let compressor = Lz4Compressor::new(0);
        test_roundtrip(&compressor, b"");
    }

    // ─── Large data tests ───

    #[test]
    fn test_zstd_large_data() {
        let compressor = ZstdCompressor::new(3);
        let mut data = Vec::with_capacity(100_000);
        for i in 0..10_000 {
            data.extend_from_slice(format!("item_{:04} ", i).as_bytes());
        }
        test_roundtrip(&compressor, &data);
    }

    #[test]
    fn test_snappy_large_data() {
        let compressor = SnappyCompressor;
        let mut data = Vec::with_capacity(100_000);
        for i in 0..10_000 {
            data.extend_from_slice(format!("item_{:04} ", i).as_bytes());
        }
        test_roundtrip(&compressor, &data);
    }

    #[test]
    fn test_lz4_large_data() {
        let compressor = Lz4Compressor::new(0);
        let mut data = Vec::with_capacity(100_000);
        for i in 0..10_000 {
            data.extend_from_slice(format!("item_{:04} ", i).as_bytes());
        }
        test_roundtrip(&compressor, &data);
    }

    // ─── Repetitive data tests (high compression ratio) ───

    #[test]
    fn test_zstd_repetitive() {
        let compressor = ZstdCompressor::new(3);
        let data = "The quick brown fox jumps over the lazy dog. ".repeat(1000);
        let compressed = compressor.compress(data.as_bytes()).unwrap();
        let ratio = compressed.len() as f64 / data.len() as f64;
        assert!(ratio < 0.1, "Zstd ratio should be < 0.1 for repetitive data, got {:.3}", ratio);
        test_roundtrip(&compressor, data.as_bytes());
    }

    #[test]
    fn test_snappy_repetitive() {
        let compressor = SnappyCompressor;
        let data = "The quick brown fox jumps over the lazy dog. ".repeat(1000);
        let compressed = compressor.compress(data.as_bytes()).unwrap();
        let ratio = compressed.len() as f64 / data.len() as f64;
        assert!(ratio < 0.15, "Snappy ratio should be < 0.15 for repetitive data, got {:.3}", ratio);
        test_roundtrip(&compressor, data.as_bytes());
    }

    #[test]
    fn test_lz4_repetitive() {
        let compressor = Lz4Compressor::new(0);
        let data = "The quick brown fox jumps over the lazy dog. ".repeat(1000);
        let compressed = compressor.compress(data.as_bytes()).unwrap();
        let ratio = compressed.len() as f64 / data.len() as f64;
        assert!(ratio < 0.15, "LZ4 ratio should be < 0.15 for repetitive data, got {:.3}", ratio);
        test_roundtrip(&compressor, data.as_bytes());
    }

    // ─── Compression ratio comparison ───

    #[test]
    fn test_compression_ratio_comparison() {
        let data = "The quick brown fox jumps over the lazy dog. ".repeat(500);

        let zstd = ZstdCompressor::new(3);
        let snappy = SnappyCompressor;
        let lz4 = Lz4Compressor::new(0);

        let zstd_size = zstd.compress(data.as_bytes()).unwrap().len();
        let snappy_size = snappy.compress(data.as_bytes()).unwrap().len();
        let lz4_size = lz4.compress(data.as_bytes()).unwrap().len();

        // Zstd should have the best compression ratio
        assert!(
            zstd_size < snappy_size && zstd_size < lz4_size,
            "Zstd should have better compression ratio: zstd={}, snappy={}, lz4={}",
            zstd_size,
            snappy_size,
            lz4_size
        );

        println!("Compression ratio comparison (original={} bytes):", data.len());
        println!("  Zstd:  {} bytes ({:.2}%)", zstd_size, zstd_size as f64 / data.len() as f64 * 100.0);
        println!("  Snappy: {} bytes ({:.2}%)", snappy_size, snappy_size as f64 / data.len() as f64 * 100.0);
        println!("  LZ4:   {} bytes ({:.2}%)", lz4_size, lz4_size as f64 / data.len() as f64 * 100.0);
    }

    // ─── Algorithm ID tests ───

    #[test]
    fn test_algorithm_id_roundtrip() {
        for id in [CompressionAlgorithmId::None, CompressionAlgorithmId::Zstd, CompressionAlgorithmId::Snappy, CompressionAlgorithmId::Lz4] {
            let val = id as u8;
            let recovered = CompressionAlgorithmId::from_u8(val).expect("should recover id");
            assert_eq!(id, recovered);
        }
    }

    #[test]
    fn test_algorithm_id_invalid() {
        assert!(CompressionAlgorithmId::from_u8(4).is_none());
        assert!(CompressionAlgorithmId::from_u8(255).is_none());
    }

    // ─── Name tests ───

    #[test]
    fn test_compressor_names() {
        let zstd = ZstdCompressor::new(3);
        let snappy = SnappyCompressor;
        let lz4 = Lz4Compressor::new(0);

        assert_eq!(zstd.name(), "zstd");
        assert_eq!(snappy.name(), "snappy");
        assert_eq!(lz4.name(), "lz4");
    }
}
