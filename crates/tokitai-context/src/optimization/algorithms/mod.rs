//! Optimization algorithms
//!
//! Efficient algorithms for LCS, LSH, and other optimizations.

pub mod lcs;
pub mod lsh;

pub use lcs::{HirschbergLCS, OptimizedLcsResult};
pub use lsh::{
    MinHashGenerator, MinHashSignature, LSHConfig, LSHIndex,
    LSHIndexStats, MinHashLSHIndex, DocumentMetadata,
};
