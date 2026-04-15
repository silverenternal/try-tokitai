//! Compression algorithms
//!
//! This module contains compression implementations:
//! - Dictionary: Dictionary-based compression with zstd
//! - Strategy: Unified compression strategy trait with Zstd, Snappy, LZ4 implementations
//! - Factory: Factory functions to create compressors based on BlockCompressionMode

pub mod dictionary;
pub mod factory;
pub mod strategy;

// Re-exports for convenience
pub use dictionary::{DictionaryCompressor, DictionaryCompressionConfig, DictionaryStats};
pub use factory::{create_compressor, NoCompression};
pub use strategy::{
    CompressionAlgorithmId, CompressionStrategy, Lz4Compressor, SnappyCompressor, ZstdCompressor,
};
