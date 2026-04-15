//! Query optimization components
//!
//! This module contains components for optimizing read queries:
//! - ZoneMap: Range-based block pruning
//! - Pruner: Range query pruning logic
//! - Scan: Range scan iterator

pub mod zone_map;
pub mod pruner;
pub mod scan;

// Re-exports for convenience
pub use zone_map::{ZoneMapEntry, ZoneMapBuilder, ZoneMapIndex, ZoneMapResult, ZoneMapError, RangeQueryStats, SequentialDetector};
pub use pruner::{RangeQueryPruner, RangeQueryPrunerConfig, RangeQueryPrunerStats, PrunedBlockIterator};
pub use scan::{RangeScanIterator, RangeScanConfig, RangeScanStats, RangeEntry, QuerySegmentProvider};
