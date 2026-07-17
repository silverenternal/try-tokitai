//! Compression algorithms
//!
//! Dictionary-based compression for improved storage efficiency.

pub mod dictionary;

pub use dictionary::{
    DictionaryCompressor, DictionaryCompressionConfig, DictionaryStats,
    DictionaryMetadata, DictionaryContentAddressableStorage,
};
