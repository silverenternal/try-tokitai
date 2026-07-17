//! Multi-Version Concurrency Control (MVCC) Module
//!
//! This module provides snapshot isolation for concurrent transactions,
//! allowing readers to see a consistent view of data at a point in time
//! without blocking writers.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      Transaction Manager                     │
//! │  - Assigns transaction IDs (monotonically increasing)       │
//! │  - Tracks active transactions                                │
//! │  - Manages snapshot creation                                 │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                        Version Chain                         │
//! │  key → [Version1] → [Version2] → [Version3] → ...           │
//! │         │            │            │                          │
//! │         ▼            ▼            ▼                          │
//! │      (txn1,v1)    (txn2,v2)    (txn3,v3)                    │
//! │      (deleted)    (value)      (value)                      │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                         Snapshot                             │
//! │  - Captures active transaction set at creation time         │
//! │  - Used for visibility checks during reads                   │
//! │  - Released when transaction completes                       │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Visibility Rules
//!
//! A version is visible to a transaction if:
//! 1. The version's transaction ID < transaction's snapshot ID
//! 2. The version's transaction ID is not in the active set
//! 3. The version is the latest visible version for the key
//!
//! # Example
//!
//! ```rust,no_run
//! use tokitai_context::mvcc::{MvccManager, MvccConfig, Transaction};
//! use tokitai_context::error::ContextResult;
//!
//! # fn main() -> ContextResult<()> {
//! let config = MvccConfig::default();
//! let manager = MvccManager::new(config);
//!
//! // Start a read-write transaction
//! let mut txn = manager.begin_rw_transaction();
//!
//! // Write some data
//! txn.put("key1".to_string(), b"value1".to_vec());
//! txn.put("key2".to_string(), b"value2".to_vec());
//!
//! // Commit the transaction
//! manager.commit_transaction(&mut txn)?;
//!
//! // Start a read-only transaction (snapshot)
//! let mut snapshot = manager.begin_snapshot();
//!
//! // Read data - sees committed data as of snapshot creation
//! let value = snapshot.get(&manager, "key1")?;
//! assert_eq!(value, Some(b"value1".to_vec()));
//!
//! // Release the snapshot
//! manager.release_snapshot(&mut snapshot);
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - **Snapshot Isolation**: Readers see consistent point-in-time views
//! - **Non-blocking Reads**: Readers never block writers
//! - **Non-blocking Writes**: Writers never block readers
//! - **Automatic Garbage Collection**: Old versions cleaned up
//! - **Transaction ID Management**: Monotonically increasing IDs

pub mod snapshot;
pub mod transaction;
pub mod version_chain;

pub use snapshot::{Snapshot, SnapshotId, SnapshotManager};
pub use transaction::{Transaction, TransactionId, TransactionManager, TransactionState};
pub use version_chain::{Version, VersionChain, VersionRef};

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use tracing::{debug, info};

use crate::error::{ContextResult, ContextError};

/// MVCC configuration
#[derive(Debug, Clone)]
pub struct MvccConfig {
    /// Maximum number of versions to keep per key (for GC)
    pub max_versions_per_key: usize,
    /// Enable automatic garbage collection
    pub enable_auto_gc: bool,
    /// GC threshold (number of versions before triggering GC)
    pub gc_threshold: usize,
    /// Snapshot timeout in milliseconds (0 = no timeout)
    pub snapshot_timeout_ms: u64,
}

impl Default for MvccConfig {
    fn default() -> Self {
        Self {
            max_versions_per_key: 10,
            enable_auto_gc: true,
            gc_threshold: 5,
            snapshot_timeout_ms: 60_000, // 1 minute
        }
    }
}

/// MVCC manager - coordinates transactions and snapshots
pub struct MvccManager {
    /// Configuration
    config: MvccConfig,
    /// Transaction ID generator (monotonically increasing)
    txn_id_generator: AtomicU64,
    /// Snapshot ID generator (monotonically increasing)
    snapshot_id_generator: AtomicU64,
    /// Transaction manager
    txn_manager: RwLock<TransactionManager>,
    /// Snapshot manager
    snapshot_manager: RwLock<SnapshotManager>,
    /// Active transaction set (for visibility checks)
    active_transactions: RwLock<Vec<TransactionId>>,
    /// Statistics
    stats: Arc<MvccStats>,
}

impl MvccManager {
    /// Create a new MVCC manager
    pub fn new(config: MvccConfig) -> Self {
        Self {
            config,
            txn_id_generator: AtomicU64::new(1),
            snapshot_id_generator: AtomicU64::new(1),
            txn_manager: RwLock::new(TransactionManager::new()),
            snapshot_manager: RwLock::new(SnapshotManager::new()),
            active_transactions: RwLock::new(Vec::new()),
            stats: Arc::new(MvccStats::default()),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &MvccConfig {
        &self.config
    }

    /// Begin a read-write transaction
    pub fn begin_rw_transaction(&self) -> Transaction {
        let txn_id = self.txn_id_generator.fetch_add(1, Ordering::SeqCst);
        
        let mut txn_manager = self.txn_manager.write();
        txn_manager.register_transaction(txn_id);
        
        let mut active = self.active_transactions.write();
        active.push(txn_id);
        
        self.stats.transactions_started.fetch_add(1, Ordering::Relaxed);
        self.stats.active_transactions.fetch_add(1, Ordering::Relaxed);
        
        debug!(txn_id = %txn_id, "Started read-write transaction");
        
        Transaction::new(txn_id, TransactionState::Active)
    }

    /// Begin a read-only transaction (snapshot)
    pub fn begin_snapshot(&self) -> Snapshot {
        let snapshot_id = self.snapshot_id_generator.fetch_add(1, Ordering::SeqCst);
        let active_txns = self.active_transactions.read().clone();
        
        let snapshot_manager = self.snapshot_manager.write();
        snapshot_manager.create_snapshot(snapshot_id, &active_txns);
        
        self.stats.snapshots_created.fetch_add(1, Ordering::Relaxed);
        self.stats.active_snapshots.fetch_add(1, Ordering::Relaxed);
        
        debug!(snapshot_id = %snapshot_id, "Created snapshot");
        
        Snapshot::new(snapshot_id, active_txns)
    }

    /// Commit a transaction
    pub fn commit_transaction(&self, txn: &mut Transaction) -> ContextResult<()> {
        if txn.state() != TransactionState::Active {
            return Err(ContextError::InvalidState(
                format!("Transaction {:?} is not active, cannot commit", txn.id())
            ));
        }
        
        // Mark transaction as committed
        txn.set_state(TransactionState::Committed);
        
        // Remove from active transactions
        {
            let mut active = self.active_transactions.write();
            active.retain(|&id| id != txn.id());
        }
        
        // Update transaction manager
        {
            let mut txn_manager = self.txn_manager.write();
            txn_manager.commit_transaction(txn.id());
        }
        
        self.stats.transactions_committed.fetch_add(1, Ordering::Relaxed);
        self.stats.active_transactions.fetch_sub(1, Ordering::Relaxed);
        
        debug!(txn_id = %txn.id(), "Transaction committed");
        
        Ok(())
    }

    /// Abort a transaction
    pub fn abort_transaction(&self, txn: &mut Transaction) -> ContextResult<()> {
        if txn.state() != TransactionState::Active {
            return Err(ContextError::InvalidState(
                format!("Transaction {:?} is not active, cannot abort", txn.id())
            ));
        }
        
        // Mark transaction as aborted
        txn.set_state(TransactionState::Aborted);
        
        // Remove from active transactions
        {
            let mut active = self.active_transactions.write();
            active.retain(|&id| id != txn.id());
        }
        
        // Update transaction manager
        {
            let mut txn_manager = self.txn_manager.write();
            txn_manager.abort_transaction(txn.id());
        }
        
        self.stats.transactions_aborted.fetch_add(1, Ordering::Relaxed);
        self.stats.active_transactions.fetch_sub(1, Ordering::Relaxed);
        
        debug!(txn_id = %txn.id(), "Transaction aborted");
        
        Ok(())
    }

    /// Release a snapshot
    pub fn release_snapshot(&self, snapshot: &mut Snapshot) -> ContextResult<()> {
        let snapshot_manager = self.snapshot_manager.write();
        snapshot_manager.release_snapshot(snapshot.id());
        
        self.stats.snapshots_released.fetch_add(1, Ordering::Relaxed);
        self.stats.active_snapshots.fetch_sub(1, Ordering::Relaxed);
        
        debug!(snapshot_id = %snapshot.id(), "Snapshot released");
        
        Ok(())
    }

    /// Check if a transaction ID is visible to a snapshot
    pub fn is_visible(&self, txn_id: TransactionId, snapshot: &Snapshot) -> bool {
        // Transaction ID must be less than snapshot ID
        if txn_id >= snapshot.id() {
            return false;
        }
        
        // Transaction ID must not be in the active set at snapshot time
        if snapshot.active_transactions().contains(&txn_id) {
            return false;
        }
        
        true
    }

    /// Get statistics
    pub fn stats(&self) -> Arc<MvccStats> {
        self.stats.clone()
    }

    /// Get the next transaction ID (for testing)
    pub fn next_transaction_id(&self) -> TransactionId {
        self.txn_id_generator.load(Ordering::SeqCst)
    }

    /// Get active transaction count
    pub fn active_transaction_count(&self) -> usize {
        self.active_transactions.read().len()
    }

    /// Get active snapshot count
    pub fn active_snapshot_count(&self) -> usize {
        self.snapshot_manager.read().active_count()
    }
}

/// MVCC statistics
#[derive(Debug, Default)]
pub struct MvccStats {
    /// Total transactions started
    pub transactions_started: std::sync::atomic::AtomicU64,
    /// Total transactions committed
    pub transactions_committed: std::sync::atomic::AtomicU64,
    /// Total transactions aborted
    pub transactions_aborted: std::sync::atomic::AtomicU64,
    /// Total snapshots created
    pub snapshots_created: std::sync::atomic::AtomicU64,
    /// Total snapshots released
    pub snapshots_released: std::sync::atomic::AtomicU64,
    /// Current active transactions
    pub active_transactions: std::sync::atomic::AtomicU64,
    /// Current active snapshots
    pub active_snapshots: std::sync::atomic::AtomicU64,
    /// Total versions created
    pub versions_created: std::sync::atomic::AtomicU64,
    /// Total versions garbage collected
    pub versions_gc_collected: std::sync::atomic::AtomicU64,
}

impl MvccStats {
    /// Get a snapshot of statistics
    pub fn snapshot(&self) -> MvccStatsSnapshot {
        MvccStatsSnapshot {
            transactions_started: self.transactions_started.load(Ordering::Relaxed),
            transactions_committed: self.transactions_committed.load(Ordering::Relaxed),
            transactions_aborted: self.transactions_aborted.load(Ordering::Relaxed),
            snapshots_created: self.snapshots_created.load(Ordering::Relaxed),
            snapshots_released: self.snapshots_released.load(Ordering::Relaxed),
            active_transactions: self.active_transactions.load(Ordering::Relaxed),
            active_snapshots: self.active_snapshots.load(Ordering::Relaxed),
            versions_created: self.versions_created.load(Ordering::Relaxed),
            versions_gc_collected: self.versions_gc_collected.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of MVCC statistics
#[derive(Debug, Clone, Default)]
pub struct MvccStatsSnapshot {
    pub transactions_started: u64,
    pub transactions_committed: u64,
    pub transactions_aborted: u64,
    pub snapshots_created: u64,
    pub snapshots_released: u64,
    pub active_transactions: u64,
    pub active_snapshots: u64,
    pub versions_created: u64,
    pub versions_gc_collected: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvcc_manager_creation() {
        let config = MvccConfig::default();
        let manager = MvccManager::new(config);
        
        assert_eq!(manager.next_transaction_id(), 1);
        assert_eq!(manager.active_transaction_count(), 0);
        assert_eq!(manager.active_snapshot_count(), 0);
    }

    #[test]
    fn test_transaction_lifecycle() {
        let config = MvccConfig::default();
        let manager = MvccManager::new(config);
        
        let mut txn = manager.begin_rw_transaction();
        assert_eq!(txn.id(), 1);
        assert_eq!(txn.state(), TransactionState::Active);
        assert_eq!(manager.active_transaction_count(), 1);
        
        manager.commit_transaction(&mut txn).unwrap();
        assert_eq!(txn.state(), TransactionState::Committed);
        assert_eq!(manager.active_transaction_count(), 0);
    }

    #[test]
    fn test_snapshot_lifecycle() {
        let config = MvccConfig::default();
        let manager = MvccManager::new(config);
        
        let mut snapshot = manager.begin_snapshot();
        assert_eq!(snapshot.id(), 1);
        assert_eq!(manager.active_snapshot_count(), 1);
        
        manager.release_snapshot(&mut snapshot).unwrap();
        assert_eq!(manager.active_snapshot_count(), 0);
    }

    #[test]
    fn test_visibility_rules() {
        let config = MvccConfig::default();
        let manager = MvccManager::new(config);
        
        // Start a transaction and create a snapshot
        let mut txn1 = manager.begin_rw_transaction();
        let txn1_id = txn1.id();
        
        let mut snapshot = manager.begin_snapshot();
        let snapshot_id = snapshot.id();
        
        // txn1 started before snapshot, so it should be in active set
        assert!(snapshot.active_transactions().contains(&txn1_id));
        
        // txn1 is not visible to snapshot (it's in active set)
        assert!(!manager.is_visible(txn1_id, &snapshot));

        // A hypothetical txn2 that committed before snapshot would be visible
        let hypothetical_txn_id = snapshot_id - 1;
        // But we need to ensure it's not in active set
        let test_snapshot = Snapshot::new(snapshot_id, vec![]);
        assert!(manager.is_visible(hypothetical_txn_id, &test_snapshot));

        manager.release_snapshot(&mut snapshot).unwrap();
        manager.abort_transaction(&mut txn1).unwrap();
    }

    #[test]
    fn test_transaction_abort() {
        let config = MvccConfig::default();
        let manager = MvccManager::new(config);
        
        let mut txn = manager.begin_rw_transaction();
        assert_eq!(manager.active_transaction_count(), 1);
        
        manager.abort_transaction(&mut txn).unwrap();
        assert_eq!(txn.state(), TransactionState::Aborted);
        assert_eq!(manager.active_transaction_count(), 0);
    }

    #[test]
    fn test_stats_tracking() {
        let config = MvccConfig::default();
        let manager = MvccManager::new(config);
        
        let mut txn = manager.begin_rw_transaction();
        manager.commit_transaction(&mut txn).unwrap();
        
        let mut snapshot = manager.begin_snapshot();
        manager.release_snapshot(&mut snapshot).unwrap();
        
        let stats = manager.stats().snapshot();
        assert_eq!(stats.transactions_started, 1);
        assert_eq!(stats.transactions_committed, 1);
        assert_eq!(stats.snapshots_created, 1);
        assert_eq!(stats.snapshots_released, 1);
    }

    #[test]
    fn test_concurrent_transactions() {
        let config = MvccConfig::default();
        let manager = MvccManager::new(config);
        
        let mut txn1 = manager.begin_rw_transaction();
        let mut txn2 = manager.begin_rw_transaction();
        let mut txn3 = manager.begin_rw_transaction();
        
        assert_eq!(manager.active_transaction_count(), 3);
        
        manager.commit_transaction(&mut txn1).unwrap();
        assert_eq!(manager.active_transaction_count(), 2);
        
        manager.abort_transaction(&mut txn2).unwrap();
        assert_eq!(manager.active_transaction_count(), 1);
        
        manager.commit_transaction(&mut txn3).unwrap();
        assert_eq!(manager.active_transaction_count(), 0);
    }
}
