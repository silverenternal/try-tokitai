//! Sparse Index module for FileKV
//!
//! Implements sparse indexing for efficient segment lookups.
//! Uses hybrid approach: HashMap for O(1) point lookups + sorted Vec for range queries.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use ahash::AHashMap;
use serde::{Deserialize, Serialize};

use crate::query::zone_map::{ZoneMapEntry, ZoneMapIndex};

/// Index error types
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    #[error("Index not found: {0}")]
    IndexNotFound(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, IndexError>;

/// Sparse index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparseIndexEntry {
    pub key: String,
    pub offset: u64,
    pub seq_num: u64,
}

/// Sparse index for a segment
///
/// Hybrid design for optimal performance in both point and range queries:
/// - `key_map`: HashMap for O(1) point lookups (replaces O(n) linear scan)
/// - `entries`: Sorted Vec for range queries and zone map support
/// - `zone_map`: Zone map entries for range query pruning (shared via Arc)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SparseIndex {
    #[serde(skip)]  // Don't serialize, rebuild on load
    key_map: AHashMap<String, u64>,  // key -> offset for O(1) lookup

    pub entries: Vec<SparseIndexEntry>,
    pub segment_id: u64,
    /// Zone map entries for range query pruning (shared via Arc for O(1) clone)
    #[serde(default, skip)]
    pub zone_map: Arc<Vec<ZoneMapEntry>>,
}

impl SparseIndex {
    pub fn new(segment_id: u64) -> Self {
        Self {
            key_map: AHashMap::new(),
            entries: Vec::new(),
            segment_id,
            zone_map: Arc::new(Vec::new()),
        }
    }

    pub fn add(&mut self, key: String, offset: u64, seq_num: u64) {
        // Insert into both HashMap (for O(1) lookup) and Vec (for range queries)
        self.key_map.insert(key.clone(), offset);
        self.entries.push(SparseIndexEntry { key, offset, seq_num });
    }
    
    /// Build key_map from entries (called after deserialization)
    pub fn build_key_map(&mut self) {
        if self.key_map.is_empty() && !self.entries.is_empty() {
            self.key_map = self.entries.iter()
                .map(|e| (e.key.clone(), e.offset))
                .collect();
        }
    }

    /// O(1) point lookup using HashMap
    pub fn find(&self, key: &str) -> Option<u64> {
        self.key_map.get(key).copied()
    }

    /// 1.2 OPTIMIZATION: Check if a key might exist in this segment based on zone map
    /// Returns true if the key falls within the segment's key range, false if definitely not present
    pub fn key_might_exist(&self, key: &str) -> bool {
        if self.zone_map.is_empty() {
            return true; // No zone map, must check
        }

        // Get overall min/max from first and last zone map entries
        if let (Some(first), Some(last)) = (self.zone_map.first(), self.zone_map.last()) {
            key >= first.min_key.as_str() && key <= last.max_key.as_str()
        } else {
            true
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let mut index: Self = serde_json::from_str(&json)?;
        // Rebuild key_map after loading (since it's skipped during serialization)
        index.build_key_map();
        Ok(index)
    }
}

/// Dense index for detailed lookups
///
/// GAP-C4: Added block_size configuration for sequential prefetch
/// PERF-005 P2: Uses AHashMap for O(1) lookups with faster hashing than BTreeMap
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DenseIndex {
    pub entries: AHashMap<String, DenseIndexEntry>,
    /// GAP-C4: Block size used for calculating block_id (bytes)
    #[serde(default)]
    pub block_size: u64,
}

impl DenseIndex {
    /// Create a new dense index with block size configuration
    pub fn with_block_size(block_size: u64) -> Self {
        Self {
            entries: AHashMap::new(),
            block_size,
        }
    }

    /// Get the configured block size
    pub fn block_size(&self) -> u64 {
        self.block_size
    }

    /// Calculate block_id from an entry offset
    pub fn offset_to_block_id(&self, offset: u64) -> u64 {
        if self.block_size == 0 {
            0
        } else {
            offset / self.block_size
        }
    }
}

/// Dense index entry for detailed lookups
/// 
/// GAP-C4: Added block_id field for sequential prefetch tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DenseIndexEntry {
    pub offset: u64,
    pub key_len: u32,  // CFG-003: Added for fast lookups
    pub value_len: u32,
    pub checksum: u32,
    pub seq_num: u64,
    /// GAP-C4: Block ID for sequential prefetch tracking
    #[serde(default)]
    pub block_id: u64,
}

/// Index manager for all segments
#[derive(Clone)]
pub struct IndexManager {
    index_dir: PathBuf,
    indexes: BTreeMap<u64, Arc<SparseIndex>>,
    dense_indexes: BTreeMap<u64, DenseIndex>,
}

impl IndexManager {
    pub fn new<P: AsRef<Path>>(index_dir: P) -> Result<Self> {
        std::fs::create_dir_all(index_dir.as_ref())?;
        Ok(Self {
            index_dir: index_dir.as_ref().to_path_buf(),
            indexes: BTreeMap::new(),
            dense_indexes: BTreeMap::new(),
        })
    }

    pub fn add_index(&mut self, segment_id: u64, index: Arc<SparseIndex>) {
        self.indexes.insert(segment_id, index);
    }

    pub fn add_dense_index(&mut self, segment_id: u64, index: DenseIndex) {
        self.dense_indexes.insert(segment_id, index);
    }

    pub fn get_index(&self, segment_id: u64) -> Option<Arc<SparseIndex>> {
        self.indexes.get(&segment_id).map(Arc::clone)
    }

    pub fn load_all_indexes(&mut self) -> Result<()> {
        for entry in std::fs::read_dir(&self.index_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("idx") {
                let index = SparseIndex::load(&path)?;
                self.indexes.insert(index.segment_id, Arc::new(index));
            }
        }
        Ok(())
    }

    pub fn save_index(&self, segment_id: u64) -> Result<()> {
        if let Some(index) = self.indexes.get(&segment_id) {
            let path = self.index_dir.join(format!("segment_{}.idx", segment_id));
            index.save(&path)?;
        }
        Ok(())
    }

    /// Get all sparse indexes
    pub fn all_indexes(&self) -> &BTreeMap<u64, Arc<SparseIndex>> {
        &self.indexes
    }

    /// Get all dense indexes
    pub fn all_dense_indexes(&self) -> &BTreeMap<u64, DenseIndex> {
        &self.dense_indexes
    }

    /// Get zone map index for a segment
    /// Returns a ZoneMapIndex that shares the entries via Arc (no clone of entries)
    pub fn get_zone_map(&self, segment_id: u64) -> Option<ZoneMapIndex> {
        self.indexes.get(&segment_id).map(|idx| {
            ZoneMapIndex::from_shared(segment_id, Arc::clone(&idx.zone_map))
        })
    }

    /// Update zone map for a segment
    pub fn update_zone_map(&mut self, segment_id: u64, zone_map: Arc<Vec<ZoneMapEntry>>) -> Result<()> {
        if let Some(index) = self.indexes.get_mut(&segment_id) {
            let index_mut = Arc::make_mut(index);
            index_mut.zone_map = zone_map;
            // Save the updated index to disk
            self.save_index(segment_id)?;
        }
        Ok(())
    }
}

/// Index configuration
#[derive(Debug, Clone)]
pub struct SparseIndexConfig {
    pub sparse_index_interval: usize,
}

impl Default for SparseIndexConfig {
    fn default() -> Self {
        Self {
            sparse_index_interval: 100,
        }
    }
}
