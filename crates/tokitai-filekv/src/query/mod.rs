//! Query optimization components
//!
//! This module contains components for optimizing read queries:
//! - ZoneMap: Range-based block pruning
//! - Pruner: Range query pruning logic
//! - Scan: Range scan iterator

pub mod pruner;
pub mod scan;
pub mod zone_map;

// Re-exports for convenience
pub use pruner::{PrunedBlockIterator, RangeQueryPruner, RangeQueryPrunerConfig, RangeQueryPrunerStats};
pub use scan::{QuerySegmentProvider, RangeEntry, RangeScanConfig, RangeScanIterator, RangeScanStats};
pub use zone_map::{
    RangeQueryStats, SequentialDetector, ZoneMapBuilder, ZoneMapEntry, ZoneMapError, ZoneMapIndex, ZoneMapResult,
};
