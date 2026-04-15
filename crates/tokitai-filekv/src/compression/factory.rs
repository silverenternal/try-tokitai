//! Compression factory
//!
//! Provides factory functions to create compression strategies based on
//! the configured `BlockCompressionMode`.

use crate::compression::strategy::{
    CompressionStrategy, Lz4Compressor, SnappyCompressor, ZstdCompressor,
};
use crate::core::types::BlockCompressionMode;

/// Create a compressor based on the configured compression mode
pub fn create_compressor(mode: &BlockCompressionMode, level: i32) -> Box<dyn CompressionStrategy> {
    match mode {
        BlockCompressionMode::None => Box::new(NoCompression),
        BlockCompressionMode::Zstd => Box::new(ZstdCompressor::new(level)),
        BlockCompressionMode::Snappy => Box::new(SnappyCompressor::new(level)),
        BlockCompressionMode::Lz4 => Box::new(Lz4Compressor::new(level)),
    }
}

/// No compression passthrough
pub struct NoCompression;

impl CompressionStrategy for NoCompression {
    fn name(&self) -> &str {
        "none"
    }

    fn compress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(data.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_zstd_compressor() {
        let compressor = create_compressor(&BlockCompressionMode::Zstd, 3);
        assert_eq!(compressor.name(), "zstd");
        let data = b"test data";
        let compressed = compressor.compress(data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_create_snappy_compressor() {
        let compressor = create_compressor(&BlockCompressionMode::Snappy, 0);
        assert_eq!(compressor.name(), "snappy");
        let data = b"test data";
        let compressed = compressor.compress(data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_create_lz4_compressor() {
        let compressor = create_compressor(&BlockCompressionMode::Lz4, 0);
        assert_eq!(compressor.name(), "lz4");
        let data = b"test data";
        let compressed = compressor.compress(data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_create_none_compressor() {
        let compressor = create_compressor(&BlockCompressionMode::None, 0);
        assert_eq!(compressor.name(), "none");
        let data = b"test data";
        let compressed = compressor.compress(data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(data.as_slice(), decompressed.as_slice());
        assert_eq!(compressed.len(), data.len());
    }
}
