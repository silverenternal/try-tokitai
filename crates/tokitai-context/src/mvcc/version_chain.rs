//! Version Chain Module
//!
//! This module implements the core MVCC data structure - a linked list of versions
//! for each key, allowing multiple concurrent readers to see different versions.
//!
//! # Data Structure
//!
//! ```text
//! Key: "user:123"
//! │
//! ▼
//! ┌─────────────┐    ┌─────────────┐    ┌─────────────┐
//! │  Version 3  │───▶│  Version 2  │───▶│  Version 1  │
//! │  txn_id: 5  │    │  txn_id: 3  │    │  txn_id: 1  │
//! │  value: C   │    │  value: B   │    │  value: A   │
//! │  deleted: F │    │  deleted: F │    │  deleted: T │
//! └─────────────┘    └─────────────┘    └─────────────┘
//!      (latest)                              (oldest)
//! ```
//!
//! Each version contains:
//! - `txn_id`: Transaction ID that created this version
//! - `value`: The actual data (None if deleted)
//! - `deleted`: Flag indicating if this is a tombstone
//! - `next`: Pointer to the next (older) version
//! - `timestamp`: Creation timestamp for GC ordering

use std::sync::Arc;
use parking_lot::RwLock;
use bytes::Bytes;
use dashmap::DashMap;

use super::{TransactionId, MvccStats};

/// A single version in the version chain
#[derive(Debug, Clone)]
pub struct Version {
    /// Transaction ID that created this version
    pub txn_id: TransactionId,
    /// The value data (None if this is a delete/tombstone)
    pub value: Option<Bytes>,
    /// Whether this version is a tombstone (delete marker)
    pub deleted: bool,
    /// Timestamp for GC ordering (milliseconds since epoch)
    pub timestamp: u64,
    /// Reference to the next (older) version
    pub next: Option<Arc<Version>>,
}

impl Version {
    /// Create a new version
    pub fn new(txn_id: TransactionId, value: Option<Vec<u8>>, deleted: bool) -> Self {
        Self {
            txn_id,
            value: value.map(Bytes::from),
            deleted,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            next: None,
        }
    }

    /// Create a new version with timestamp
    pub fn with_timestamp(txn_id: TransactionId, value: Option<Vec<u8>>, deleted: bool, timestamp: u64) -> Self {
        Self {
            txn_id,
            value: value.map(Bytes::from),
            deleted,
            timestamp,
            next: None,
        }
    }

    /// Check if this version has a value (not a tombstone)
    pub fn has_value(&self) -> bool {
        !self.deleted && self.value.is_some()
    }

    /// Get the value as bytes
    pub fn value_bytes(&self) -> Option<&[u8]> {
        self.value.as_ref().map(|b| b.as_ref())
    }
}

/// Reference to a version (lightweight handle)
#[derive(Debug, Clone)]
pub struct VersionRef {
    /// Transaction ID
    pub txn_id: TransactionId,
    /// Whether this is a tombstone
    pub deleted: bool,
    /// Value size (0 if deleted or None)
    pub value_size: usize,
    /// Timestamp
    pub timestamp: u64,
}

impl From<&Version> for VersionRef {
    fn from(version: &Version) -> Self {
        Self {
            txn_id: version.txn_id,
            deleted: version.deleted,
            value_size: version.value.as_ref().map(|b| b.len()).unwrap_or(0),
            timestamp: version.timestamp,
        }
    }
}

/// Version chain for a single key
///
/// This is the core MVCC data structure. It maintains a linked list of versions,
/// with the newest version at the head. Readers traverse the chain to find the
/// appropriate version visible to their snapshot.
pub struct VersionChain {
    /// The key this chain is for
    key: String,
    /// Head of the version chain (newest version)
    head: RwLock<Option<Arc<Version>>>,
    /// Number of versions in the chain
    version_count: std::sync::atomic::AtomicUsize,
    /// Optional statistics tracker
    stats: Option<Arc<MvccStats>>,
}

impl VersionChain {
    /// Create a new version chain
    pub fn new(key: String, stats: Option<Arc<MvccStats>>) -> Self {
        Self {
            key,
            head: RwLock::new(None),
            version_count: std::sync::atomic::AtomicUsize::new(0),
            stats,
        }
    }

    /// Add a new version to the chain (at the head)
    ///
    /// This is thread-safe and can be called concurrently.
    pub fn append(&self, version: Version) {
        let mut version = version;
        
        let mut head = self.head.write();
        // Set the next pointer before wrapping in Arc
        version.next = head.clone();
        let version_arc = Arc::new(version);
        *head = Some(version_arc);
        drop(head);
        
        self.version_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        
        if let Some(ref stats) = self.stats {
            stats.versions_created.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    /// Create and append a new value version
    pub fn put(&self, txn_id: TransactionId, value: Vec<u8>) {
        let version = Version::new(txn_id, Some(value), false);
        self.append(version);
    }

    /// Create and append a tombstone (delete marker)
    pub fn delete(&self, txn_id: TransactionId) {
        let version = Version::new(txn_id, None, true);
        self.append(version);
    }

    /// Get the latest version visible to a transaction
    ///
    /// Traverses the version chain to find the first version that is:
    /// 1. Created by a transaction that committed before the reader's snapshot
    /// 2. Not in the reader's active transaction set
    pub fn get_visible(&self, is_visible: impl Fn(TransactionId) -> bool) -> Option<Arc<Version>> {
        let head = self.head.read();
        let mut current = head.clone();
        
        while let Some(version) = current {
            if is_visible(version.txn_id) {
                return Some(version);
            }
            current = version.next.clone();
        }
        
        None
    }

    /// Get the latest value (for testing, ignores visibility)
    pub fn get_latest(&self) -> Option<Arc<Version>> {
        self.head.read().clone()
    }

    /// Get all versions for debugging/testing
    pub fn get_all_versions(&self) -> Vec<Arc<Version>> {
        let mut versions = Vec::new();
        let head = self.head.read();
        let mut current = head.clone();
        
        while let Some(version) = current {
            versions.push(version.clone());
            current = version.next.clone();
        }
        
        versions
    }

    /// Get the number of versions in the chain
    pub fn version_count(&self) -> usize {
        self.version_count.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get version references (for GC)
    pub fn get_version_refs(&self) -> Vec<VersionRef> {
        let mut refs = Vec::new();
        let head = self.head.read();
        let mut current = head.clone();
        
        while let Some(version) = current {
            refs.push(VersionRef::from(version.as_ref()));
            current = version.next.clone();
        }
        
        refs
    }

    /// Garbage collect old versions
    ///
    /// Removes versions that are:
    /// 1. Older than `max_versions` count
    /// 2. Not visible to any active snapshot
    /// 3. Not the latest version
    ///
    /// Returns the number of versions collected
    pub fn garbage_collect(
        &self,
        min_visible_txn_id: TransactionId,
        max_versions: usize,
    ) -> usize {
        let mut head = self.head.write();

        // Collect all versions first (newest to oldest)
        let mut all_versions: Vec<Arc<Version>> = Vec::new();
        let mut current = head.clone();
        while let Some(version) = current {
            all_versions.push(version.clone());
            current = version.next.clone();
        }

        let original_count = all_versions.len();

        // Filter to keep only versions that satisfy the conditions
        // Keep newest versions first, up to max_versions
        let mut kept: Vec<Arc<Version>> = Vec::new();
        for version in &all_versions {
            if kept.len() < max_versions && version.txn_id >= min_visible_txn_id {
                kept.push(version.clone());
            }
        }

        // Rebuild chain from kept versions (kept is already newest-first)
        let mut new_chain: Option<Arc<Version>> = None;
        for version in kept.into_iter().rev() {
            let mut new_version = (*version.as_ref()).clone();
            new_version.next = new_chain.clone();
            new_chain = Some(Arc::new(new_version));
        }

        let kept_count = new_chain.as_ref().map_or(0, |_| {
            let mut count = 0;
            let mut curr = new_chain.clone();
            while curr.is_some() {
                count += 1;
                curr = curr.unwrap().next.clone();
            }
            count
        });
        let collected_count = original_count - kept_count;

        // Update the head
        *head = new_chain;

        // Update version count
        self.version_count.store(kept_count, std::sync::atomic::Ordering::Relaxed);

        // Update stats
        if let Some(ref stats) = self.stats {
            stats.versions_gc_collected.fetch_add(
                collected_count as u64,
                std::sync::atomic::Ordering::Relaxed
            );
        }

        collected_count
    }

    /// Get the key for this chain
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Check if the chain is empty
    pub fn is_empty(&self) -> bool {
        self.head.read().is_none()
    }

    /// Clear all versions (for testing)
    pub fn clear(&self) {
        let mut head = self.head.write();
        *head = None;
        self.version_count.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Global version chain registry
///
/// Maps keys to their version chains. This is the main data structure
/// that would be integrated into MemTable or FileKV.
pub struct VersionChainRegistry {
    /// Map from key to version chain
    chains: DashMap<String, Arc<VersionChain>>,
    /// Statistics
    stats: Option<Arc<MvccStats>>,
}

impl VersionChainRegistry {
    /// Create a new registry
    pub fn new(stats: Option<Arc<MvccStats>>) -> Self {
        Self {
            chains: DashMap::new(),
            stats,
        }
    }

    /// Get or create a version chain for a key
    pub fn get_or_create(&self, key: &str) -> Arc<VersionChain> {
        self.chains
            .entry(key.to_string())
            .or_insert_with(|| {
                Arc::new(VersionChain::new(key.to_string(), self.stats.clone()))
            })
            .clone()
    }

    /// Get a version chain if it exists
    pub fn get(&self, key: &str) -> Option<Arc<VersionChain>> {
        self.chains.get(key).map(|r| r.value().clone())
    }

    /// Get all keys
    pub fn get_all_keys(&self) -> Vec<String> {
        self.chains.iter().map(|r| r.key().clone()).collect()
    }

    /// Get the number of keys
    pub fn key_count(&self) -> usize {
        self.chains.len()
    }

    /// Get total version count across all chains
    pub fn total_version_count(&self) -> usize {
        self.chains.iter()
            .map(|r| r.version_count())
            .sum()
    }

    /// Garbage collect all chains
    pub fn garbage_collect_all(
        &self,
        min_visible_txn_id: TransactionId,
        max_versions: usize,
    ) -> usize {
        self.chains.iter()
            .map(|r| r.garbage_collect(min_visible_txn_id, max_versions))
            .sum()
    }

    /// Clear all chains (for testing)
    pub fn clear(&self) {
        self.chains.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let version = Version::new(1, Some(b"hello".to_vec()), false);
        
        assert_eq!(version.txn_id, 1);
        assert_eq!(version.value_bytes(), Some(b"hello".as_slice()));
        assert!(!version.deleted);
        assert!(version.has_value());
    }

    #[test]
    fn test_version_tombstone() {
        let version = Version::new(2, None, true);
        
        assert_eq!(version.txn_id, 2);
        assert!(version.deleted);
        assert!(!version.has_value());
        assert_eq!(version.value_bytes(), None);
    }

    #[test]
    fn test_version_chain_append() {
        let chain = VersionChain::new("key1".to_string(), None);
        
        chain.put(1, b"value1".to_vec());
        chain.put(2, b"value2".to_vec());
        chain.put(3, b"value3".to_vec());
        
        assert_eq!(chain.version_count(), 3);
        
        let latest = chain.get_latest().unwrap();
        assert_eq!(latest.txn_id, 3);
        assert_eq!(latest.value_bytes(), Some(b"value3".as_slice()));
    }

    #[test]
    fn test_version_chain_delete() {
        let chain = VersionChain::new("key2".to_string(), None);
        
        chain.put(1, b"value".to_vec());
        chain.delete(2);
        
        assert_eq!(chain.version_count(), 2);
        
        let latest = chain.get_latest().unwrap();
        assert_eq!(latest.txn_id, 2);
        assert!(latest.deleted);
    }

    #[test]
    fn test_version_chain_get_visible() {
        let chain = VersionChain::new("key3".to_string(), None);
        
        chain.put(1, b"v1".to_vec());
        chain.put(2, b"v2".to_vec());
        chain.put(3, b"v3".to_vec());
        chain.put(5, b"v5".to_vec());
        
        // Visible: txn_id < 4 (so versions 1, 2, 3 are visible, latest is 3)
        let visible = chain.get_visible(|txn_id| txn_id < 4);
        assert!(visible.is_some());
        assert_eq!(visible.unwrap().txn_id, 3);
        
        // Visible: txn_id < 2 (only version 1 is visible)
        let visible = chain.get_visible(|txn_id| txn_id < 2);
        assert!(visible.is_some());
        assert_eq!(visible.unwrap().txn_id, 1);
        
        // Visible: txn_id < 1 (nothing is visible)
        let visible = chain.get_visible(|txn_id| txn_id < 1);
        assert!(visible.is_none());
    }

    #[test]
    fn test_version_chain_with_active_set() {
        let chain = VersionChain::new("key4".to_string(), None);
        
        chain.put(1, b"v1".to_vec());
        chain.put(3, b"v3".to_vec());
        chain.put(5, b"v5".to_vec());

        let active_set = [3, 5];

        // Visible: txn_id < 6 AND txn_id not in {3, 5}
        // So version 1 is the latest visible
        let visible = chain.get_visible(|txn_id| txn_id < 6 && !active_set.contains(&txn_id));
        assert!(visible.is_some());
        assert_eq!(visible.unwrap().txn_id, 1);
    }

    #[test]
    fn test_version_chain_garbage_collection() {
        let stats = Arc::new(MvccStats::default());
        let chain = VersionChain::new("key5".to_string(), Some(stats.clone()));
        
        // Add 10 versions
        for i in 1..=10 {
            chain.put(i, format!("v{}", i).into_bytes());
        }
        
        assert_eq!(chain.version_count(), 10);
        
        // GC: keep versions >= 5, max 5 versions
        // Iterating from newest (10) to oldest (1):
        // - Keep 10,9,8,7,6 (5 versions, all >= 5)
        // - Collect 5 (max reached)
        // - Collect 4,3,2,1 (< 5)
        // Total: keep 5, collect 5
        let collected = chain.garbage_collect(5, 5);
        
        assert_eq!(collected, 5);
        assert_eq!(chain.version_count(), 5);
        
        // Latest should be version 10
        let latest = chain.get_latest().unwrap();
        assert_eq!(latest.txn_id, 10);
        
        // Verify we have exactly 5 versions with txn_ids 10,9,8,7,6
        let all = chain.get_all_versions();
        let txn_ids: Vec<_> = all.iter().map(|v| v.txn_id).collect();
        assert_eq!(txn_ids, vec![10, 9, 8, 7, 6]);
    }

    #[test]
    fn test_version_registry() {
        let registry = VersionChainRegistry::new(None);
        
        let chain1 = registry.get_or_create("key1");
        let chain2 = registry.get_or_create("key2");
        
        chain1.put(1, b"v1".to_vec());
        chain2.put(1, b"v2".to_vec());
        
        assert_eq!(registry.key_count(), 2);
        assert_eq!(registry.total_version_count(), 2);
        
        let retrieved = registry.get("key1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().version_count(), 1);
    }

    #[test]
    fn test_version_chain_all_versions() {
        let chain = VersionChain::new("key".to_string(), None);
        
        chain.put(1, b"v1".to_vec());
        chain.put(2, b"v2".to_vec());
        chain.put(3, b"v3".to_vec());
        
        let all = chain.get_all_versions();
        assert_eq!(all.len(), 3);
        
        // Should be in reverse order (newest first)
        assert_eq!(all[0].txn_id, 3);
        assert_eq!(all[1].txn_id, 2);
        assert_eq!(all[2].txn_id, 1);
    }

    #[test]
    fn test_version_stats() {
        let stats = Arc::new(MvccStats::default());
        let chain = VersionChain::new("key".to_string(), Some(stats.clone()));
        
        chain.put(1, b"v1".to_vec());
        chain.put(2, b"v2".to_vec());
        chain.delete(3);
        
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.versions_created, 3);
        
        // GC some versions
        chain.garbage_collect(2, 2);
        
        let snapshot = stats.snapshot();
        assert_eq!(snapshot.versions_gc_collected, 1);
    }
}
