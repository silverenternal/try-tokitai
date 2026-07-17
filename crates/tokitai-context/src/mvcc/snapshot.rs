//! Snapshot Module
//!
//! This module implements read-only snapshots for consistent point-in-time reads.
//! A snapshot captures the set of active transactions at creation time, which
//! is used to determine version visibility during reads.
//!
//! # Snapshot Isolation
//!
//! Snapshot isolation guarantees that:
//! 1. All reads see a consistent snapshot of the database
//! 2. All writes made before the snapshot are visible
//! 3. Writes made after the snapshot are not visible
//! 4. Writes by transactions active at snapshot time are not visible
//!
//! # Usage
//!
//! ```rust
//! use tokitai_context::mvcc::{MvccManager, MvccConfig};
//!
//! let manager = MvccManager::new(MvccConfig::default());
//!
//! // Create a snapshot
//! let mut snapshot = manager.begin_snapshot();
//!
//! // Read data using the snapshot
//! // let value = snapshot.get(&manager, "key")?;
//!
//! // Release when done
//! // manager.release_snapshot(&mut snapshot)?;
//! ```

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use parking_lot::RwLock;

use super::TransactionId;

/// Snapshot ID type
pub type SnapshotId = u64;

/// A read-only snapshot
///
/// A snapshot captures:
/// - A unique snapshot ID (monotonically increasing)
/// - The set of active transactions at creation time
///
/// When reading a version, it's visible if:
/// 1. version.txn_id < snapshot.id
/// 2. version.txn_id is not in the active set
pub struct Snapshot {
    /// Unique snapshot ID
    id: SnapshotId,
    /// Active transactions at snapshot creation time
    active_transactions: Vec<TransactionId>,
    /// Active transactions as a HashSet for O(1) lookup
    active_set: HashSet<TransactionId>,
    /// Creation timestamp (milliseconds since epoch)
    creation_time: u64,
    /// Number of reads performed with this snapshot
    read_count: AtomicUsize,
}

impl Clone for Snapshot {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            active_transactions: self.active_transactions.clone(),
            active_set: self.active_set.clone(),
            creation_time: self.creation_time,
            read_count: AtomicUsize::new(self.read_count.load(Ordering::Relaxed)),
        }
    }
}

impl Snapshot {
    /// Create a new snapshot
    pub fn new(id: SnapshotId, active_transactions: Vec<TransactionId>) -> Self {
        let active_set: HashSet<TransactionId> = active_transactions.iter().copied().collect();
        
        Self {
            id,
            active_transactions,
            active_set,
            creation_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            read_count: AtomicUsize::new(0),
        }
    }

    /// Get snapshot ID
    pub fn id(&self) -> SnapshotId {
        self.id
    }

    /// Get active transactions list
    pub fn active_transactions(&self) -> &[TransactionId] {
        &self.active_transactions
    }

    /// Check if a transaction ID is in the active set
    pub fn is_active(&self, txn_id: TransactionId) -> bool {
        self.active_set.contains(&txn_id)
    }

    /// Get creation timestamp
    pub fn creation_time(&self) -> u64 {
        self.creation_time
    }

    /// Increment read count
    pub fn record_read(&self) {
        self.read_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get read count
    pub fn read_count(&self) -> usize {
        self.read_count.load(Ordering::Relaxed)
    }

    /// Check if a version is visible to this snapshot
    pub fn is_visible(&self, txn_id: TransactionId) -> bool {
        txn_id < self.id && !self.is_active(txn_id)
    }
}

/// Snapshot manager - tracks active snapshots
pub struct SnapshotManager {
    /// Active snapshots
    snapshots: RwLock<HashMap<SnapshotId, Arc<SnapshotInfo>>>,
    /// Statistics
    stats: SnapshotManagerStats,
}

/// Information about a snapshot
pub struct SnapshotInfo {
    /// Snapshot ID
    pub id: SnapshotId,
    /// Creation time
    pub creation_time: u64,
    /// Number of reads
    pub read_count: AtomicUsize,
}

use std::collections::HashMap;

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new() -> Self {
        Self {
            snapshots: RwLock::new(HashMap::new()),
            stats: SnapshotManagerStats::default(),
        }
    }

    /// Create a new snapshot
    pub fn create_snapshot(
        &self,
        id: SnapshotId,
        _active_transactions: &[TransactionId],
    ) -> Arc<SnapshotInfo> {
        let info = Arc::new(SnapshotInfo {
            id,
            creation_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            read_count: AtomicUsize::new(0),
        });
        
        let mut snapshots = self.snapshots.write();
        snapshots.insert(id, info.clone());
        
        self.stats.snapshots_created.fetch_add(1, Ordering::Relaxed);
        self.stats.active_count.fetch_add(1, Ordering::Relaxed);
        
        info
    }

    /// Release a snapshot
    pub fn release_snapshot(&self, id: SnapshotId) -> bool {
        let mut snapshots = self.snapshots.write();
        if snapshots.remove(&id).is_some() {
            self.stats.snapshots_released.fetch_add(1, Ordering::Relaxed);
            self.stats.active_count.fetch_sub(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get a snapshot by ID
    pub fn get_snapshot(&self, id: SnapshotId) -> Option<Arc<SnapshotInfo>> {
        self.snapshots.read().get(&id).cloned()
    }

    /// Get all active snapshot IDs
    pub fn get_active_ids(&self) -> Vec<SnapshotId> {
        self.snapshots.read().keys().copied().collect()
    }

    /// Get the number of active snapshots
    pub fn active_count(&self) -> usize {
        self.stats.active_count.load(Ordering::Relaxed)
    }

    /// Get statistics
    pub fn stats(&self) -> &SnapshotManagerStats {
        &self.stats
    }

    /// Find the minimum visible transaction ID across all snapshots
    ///
    /// This is used for garbage collection - versions older than this
    /// cannot be visible to any active snapshot.
    pub fn min_visible_txn_id(&self) -> TransactionId {
        let snapshots = self.snapshots.read();
        
        if snapshots.is_empty() {
            // No active snapshots, all committed versions are visible
            return TransactionId::MAX;
        }
        
        // Find the minimum snapshot ID
        // Any version with txn_id < min_snapshot_id and not in active set is visible
        snapshots.values()
            .map(|s| s.id)
            .min()
            .unwrap_or(TransactionId::MAX)
    }

    /// Record a read for a snapshot
    pub fn record_read(&self, id: SnapshotId) {
        if let Some(info) = self.snapshots.read().get(&id) {
            info.read_count.fetch_add(1, Ordering::Relaxed);
        }
    }
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot manager statistics
#[derive(Debug, Default)]
pub struct SnapshotManagerStats {
    /// Total snapshots created
    pub snapshots_created: AtomicU64,
    /// Total snapshots released
    pub snapshots_released: AtomicU64,
    /// Current active snapshots
    pub active_count: AtomicUsize,
}

impl SnapshotManagerStats {
    /// Get a snapshot of statistics
    pub fn snapshot(&self) -> SnapshotManagerStatsSnapshot {
        SnapshotManagerStatsSnapshot {
            snapshots_created: self.snapshots_created.load(Ordering::Relaxed),
            snapshots_released: self.snapshots_released.load(Ordering::Relaxed),
            active_count: self.active_count.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of snapshot manager statistics
#[derive(Debug, Clone, Default)]
pub struct SnapshotManagerStatsSnapshot {
    pub snapshots_created: u64,
    pub snapshots_released: u64,
    pub active_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_creation() {
        let snapshot = Snapshot::new(1, vec![1, 2, 3]);
        
        assert_eq!(snapshot.id(), 1);
        assert_eq!(snapshot.active_transactions(), &[1, 2, 3]);
        assert!(snapshot.is_active(1));
        assert!(snapshot.is_active(2));
        assert!(snapshot.is_active(3));
        assert!(!snapshot.is_active(4));
    }

    #[test]
    fn test_snapshot_visibility() {
        let snapshot = Snapshot::new(10, vec![5, 7, 9]);
        
        // txn_id < 10 and not in {5, 7, 9} → visible
        assert!(snapshot.is_visible(1));
        assert!(snapshot.is_visible(3));
        assert!(snapshot.is_visible(8));
        
        // txn_id >= 10 → not visible
        assert!(!snapshot.is_visible(10));
        assert!(!snapshot.is_visible(11));
        
        // txn_id in active set → not visible
        assert!(!snapshot.is_visible(5));
        assert!(!snapshot.is_visible(7));
        assert!(!snapshot.is_visible(9));
    }

    #[test]
    fn test_snapshot_read_counting() {
        let snapshot = Snapshot::new(1, vec![]);
        
        assert_eq!(snapshot.read_count(), 0);
        
        snapshot.record_read();
        assert_eq!(snapshot.read_count(), 1);
        
        snapshot.record_read();
        snapshot.record_read();
        assert_eq!(snapshot.read_count(), 3);
    }

    #[test]
    fn test_snapshot_manager_lifecycle() {
        let manager = SnapshotManager::new();
        
        // Create snapshots
        let _s1 = manager.create_snapshot(1, &[1, 2]);
        let _s2 = manager.create_snapshot(2, &[1, 2, 3]);
        
        assert_eq!(manager.active_count(), 2);
        assert!(manager.get_snapshot(1).is_some());
        assert!(manager.get_snapshot(2).is_some());
        
        // Release one
        assert!(manager.release_snapshot(1));
        assert_eq!(manager.active_count(), 1);
        assert!(manager.get_snapshot(1).is_none());
        assert!(manager.get_snapshot(2).is_some());
        
        // Release non-existent
        assert!(!manager.release_snapshot(999));
    }

    #[test]
    fn test_snapshot_manager_min_visible_txn_id() {
        let manager = SnapshotManager::new();
        
        // No snapshots → MAX
        assert_eq!(manager.min_visible_txn_id(), TransactionId::MAX);
        
        // Create snapshots
        manager.create_snapshot(10, &[1, 2, 3]);
        manager.create_snapshot(5, &[1, 2]);
        manager.create_snapshot(15, &[1, 2, 3, 4, 5]);
        
        // Min is 5
        assert_eq!(manager.min_visible_txn_id(), 5);
        
        // Release min
        manager.release_snapshot(5);
        
        // New min is 10
        assert_eq!(manager.min_visible_txn_id(), 10);
    }

    #[test]
    fn test_snapshot_manager_stats() {
        let manager = SnapshotManager::new();
        
        manager.create_snapshot(1, &[]);
        manager.create_snapshot(2, &[]);
        manager.release_snapshot(1);
        
        let stats = manager.stats().snapshot();
        assert_eq!(stats.snapshots_created, 2);
        assert_eq!(stats.snapshots_released, 1);
        assert_eq!(stats.active_count, 1);
    }

    #[test]
    fn test_snapshot_empty_active_set() {
        let snapshot = Snapshot::new(5, vec![]);
        
        // All txn_id < 5 are visible
        assert!(snapshot.is_visible(1));
        assert!(snapshot.is_visible(4));
        
        // txn_id >= 5 are not visible
        assert!(!snapshot.is_visible(5));
        assert!(!snapshot.is_visible(6));
    }

    #[test]
    fn test_concurrent_snapshots() {
        use std::thread;
        
        let manager = Arc::new(SnapshotManager::new());
        let mut handles = vec![];
        
        // Spawn 10 threads, each creating 10 snapshots
        for t in 0..10 {
            let manager = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for i in 0..10 {
                    let id = t * 10 + i as u64;
                    manager.create_snapshot(id, &[]);
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        assert_eq!(manager.active_count(), 100);
    }
}
