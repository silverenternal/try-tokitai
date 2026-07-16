//! Shared engine types
//!
//! This module contains type definitions that are shared across engine modules
//! to avoid circular dependencies and duplication.

/// Cache lookup result for accurate cache hit/miss statistics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheLookupResult {
    MemTableHit,
    BlockCacheHit,
    DiskHit,
    CacheMiss,
}
