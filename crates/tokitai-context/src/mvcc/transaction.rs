//! Transaction Module
//!
//! This module implements read-write transactions with write buffering.
//! Writes are buffered locally until commit, providing isolation.
//!
//! # Transaction Lifecycle
//!
//! ```text
//! ┌─────────┐                      ┌───────────┐
//! │  Start  │─────────────────────▶│  Active   │
//! └─────────┘                      └─────┬─────┘
//!                                         │
//!                    ┌────────────────────┼────────────────────┐
//!                    │                    │                    │
//!                    ▼                    ▼                    │
//!             ┌──────────┐         ┌──────────┐               │
//!             │ Committed│         │ Aborted  │               │
//!             └──────────┘         └──────────┘               │
//!                    │                    │                    │
//!                    └────────────────────┴────────────────────┘
//! ```
//!
//! # Write Buffering
//!
//! All writes (put/delete) are buffered in the transaction until commit.
//! This provides:
//! - **Read Your Own Writes**: Transaction can see its own uncommitted writes
//! - **Isolation**: Other transactions cannot see uncommitted writes
//! - **Atomic Commit**: All writes become visible atomically on commit

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Transaction ID type
pub type TransactionId = u64;

/// Transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionState {
    /// Transaction is active and can perform operations
    Active,
    /// Transaction has been committed (read-only after this)
    Committed,
    /// Transaction has been aborted (read-only after this)
    Aborted,
}

/// Write operation in a transaction
#[derive(Debug, Clone)]
pub enum WriteOperation {
    /// Put operation with value
    Put(Vec<u8>),
    /// Delete operation (tombstone)
    Delete,
}

/// Transaction - represents a read-write transaction
pub struct Transaction {
    /// Unique transaction ID
    id: TransactionId,
    /// Current state
    state: TransactionState,
    /// Buffered writes (key → operation)
    writes: HashMap<String, WriteOperation>,
    /// Read set (keys read by this transaction)
    read_set: HashMap<String, Vec<u8>>,
    /// Start timestamp (milliseconds since epoch)
    start_time: u64,
    /// Commit timestamp (milliseconds since epoch, set on commit)
    commit_time: Option<u64>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(id: TransactionId, state: TransactionState) -> Self {
        Self {
            id,
            state,
            writes: HashMap::new(),
            read_set: HashMap::new(),
            start_time: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            commit_time: None,
        }
    }

    /// Get transaction ID
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Get transaction state
    pub fn state(&self) -> TransactionState {
        self.state
    }

    /// Set transaction state (internal use)
    pub fn set_state(&mut self, state: TransactionState) {
        self.state = state;
    }

    /// Check if transaction is active
    pub fn is_active(&self) -> bool {
        self.state == TransactionState::Active
    }

    /// Check if transaction is committed
    pub fn is_committed(&self) -> bool {
        self.state == TransactionState::Committed
    }

    /// Check if transaction is aborted
    pub fn is_aborted(&self) -> bool {
        self.state == TransactionState::Aborted
    }

    /// Put a key-value pair (buffered until commit)
    pub fn put(&mut self, key: String, value: Vec<u8>) {
        self.writes.insert(key, WriteOperation::Put(value));
    }

    /// Delete a key (buffered until commit)
    pub fn delete(&mut self, key: String) {
        self.writes.insert(key, WriteOperation::Delete);
    }

    /// Get a buffered write operation
    pub fn get_write(&self, key: &str) -> Option<&WriteOperation> {
        self.writes.get(key)
    }

    /// Get all buffered writes
    pub fn get_writes(&self) -> &HashMap<String, WriteOperation> {
        &self.writes
    }

    /// Get the number of buffered writes
    pub fn write_count(&self) -> usize {
        self.writes.len()
    }

    /// Record a read (for conflict detection)
    pub fn record_read(&mut self, key: String, value: Vec<u8>) {
        self.read_set.insert(key, value);
    }

    /// Get the read set
    pub fn get_read_set(&self) -> &HashMap<String, Vec<u8>> {
        &self.read_set
    }

    /// Get start timestamp
    pub fn start_time(&self) -> u64 {
        self.start_time
    }

    /// Get commit timestamp (if committed)
    pub fn commit_time(&self) -> Option<u64> {
        self.commit_time
    }

    /// Set commit timestamp
    fn set_commit_time(&mut self, timestamp: u64) {
        self.commit_time = Some(timestamp);
    }

    /// Clear buffered data (for cleanup)
    pub fn clear(&mut self) {
        self.writes.clear();
        self.read_set.clear();
    }
}

/// Transaction manager - tracks transaction lifecycle
pub struct TransactionManager {
    /// Active transactions
    active: HashMap<TransactionId, Arc<AtomicU64>>,
    /// Committed transactions (for debugging/audit)
    committed: Vec<TransactionId>,
    /// Aborted transactions (for debugging/audit)
    aborted: Vec<TransactionId>,
    /// Maximum history size
    max_history: usize,
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new() -> Self {
        Self {
            active: HashMap::new(),
            committed: Vec::new(),
            aborted: Vec::new(),
            max_history: 1000,
        }
    }

    /// Register a new transaction
    pub fn register_transaction(&mut self, txn_id: TransactionId) {
        self.active.insert(txn_id, Arc::new(AtomicU64::new(0)));
    }

    /// Commit a transaction
    pub fn commit_transaction(&mut self, txn_id: TransactionId) {
        self.active.remove(&txn_id);
        self.committed.push(txn_id);
        
        // Trim history if needed
        if self.committed.len() > self.max_history {
            self.committed.remove(0);
        }
    }

    /// Abort a transaction
    pub fn abort_transaction(&mut self, txn_id: TransactionId) {
        self.active.remove(&txn_id);
        self.aborted.push(txn_id);
        
        // Trim history if needed
        if self.aborted.len() > self.max_history {
            self.aborted.remove(0);
        }
    }

    /// Check if a transaction is active
    pub fn is_active(&self, txn_id: TransactionId) -> bool {
        self.active.contains_key(&txn_id)
    }

    /// Get active transaction IDs
    pub fn get_active_transactions(&self) -> Vec<TransactionId> {
        self.active.keys().copied().collect()
    }

    /// Get committed transaction IDs
    pub fn get_committed_transactions(&self) -> &[TransactionId] {
        &self.committed
    }

    /// Get aborted transaction IDs
    pub fn get_aborted_transactions(&self) -> &[TransactionId] {
        &self.aborted
    }

    /// Get the number of active transactions
    pub fn active_count(&self) -> usize {
        self.active.len()
    }

    /// Clear history (for testing)
    pub fn clear_history(&mut self) {
        self.committed.clear();
        self.aborted.clear();
    }
}

impl Default for TransactionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let txn = Transaction::new(1, TransactionState::Active);
        
        assert_eq!(txn.id(), 1);
        assert_eq!(txn.state(), TransactionState::Active);
        assert!(txn.is_active());
        assert!(!txn.is_committed());
        assert!(!txn.is_aborted());
        assert_eq!(txn.write_count(), 0);
    }

    #[test]
    fn test_transaction_writes() {
        let mut txn = Transaction::new(2, TransactionState::Active);
        
        txn.put("key1".to_string(), b"value1".to_vec());
        txn.put("key2".to_string(), b"value2".to_vec());
        txn.delete("key3".to_string());
        
        assert_eq!(txn.write_count(), 3);
        
        // Check buffered writes
        assert!(matches!(txn.get_write("key1"), Some(WriteOperation::Put(_))));
        assert!(matches!(txn.get_write("key2"), Some(WriteOperation::Put(_))));
        assert!(matches!(txn.get_write("key3"), Some(WriteOperation::Delete)));
        assert!(txn.get_write("key4").is_none());
    }

    #[test]
    fn test_transaction_state_transitions() {
        let mut txn = Transaction::new(3, TransactionState::Active);
        
        // Active → Committed
        txn.set_state(TransactionState::Committed);
        assert!(txn.is_committed());
        assert!(!txn.is_active());
        
        // Can't transition from committed (enforced by manager)
    }

    #[test]
    fn test_transaction_read_set() {
        let mut txn = Transaction::new(4, TransactionState::Active);
        
        txn.record_read("key1".to_string(), b"value1".to_vec());
        txn.record_read("key2".to_string(), b"value2".to_vec());
        
        assert_eq!(txn.get_read_set().len(), 2);
        assert_eq!(txn.get_read_set().get("key1"), Some(&b"value1".to_vec()));
    }

    #[test]
    fn test_transaction_clear() {
        let mut txn = Transaction::new(5, TransactionState::Active);
        
        txn.put("key1".to_string(), b"value1".to_vec());
        txn.record_read("key2".to_string(), b"value2".to_vec());
        
        txn.clear();
        
        assert_eq!(txn.write_count(), 0);
        assert!(txn.get_read_set().is_empty());
    }

    #[test]
    fn test_transaction_manager_lifecycle() {
        let mut manager = TransactionManager::new();
        
        // Register
        manager.register_transaction(1);
        manager.register_transaction(2);
        manager.register_transaction(3);
        
        assert_eq!(manager.active_count(), 3);
        assert!(manager.is_active(1));
        assert!(manager.is_active(2));
        assert!(manager.is_active(3));
        
        // Commit one
        manager.commit_transaction(1);
        assert!(!manager.is_active(1));
        assert!(manager.is_active(2));
        assert!(manager.is_active(3));
        assert_eq!(manager.active_count(), 2);
        
        // Abort one
        manager.abort_transaction(2);
        assert!(!manager.is_active(2));
        assert!(manager.is_active(3));
        assert_eq!(manager.active_count(), 1);
        
        // Check history
        assert_eq!(manager.get_committed_transactions(), &[1]);
        assert_eq!(manager.get_aborted_transactions(), &[2]);
    }

    #[test]
    fn test_transaction_manager_history_limit() {
        let mut manager = TransactionManager::new();
        manager.max_history = 5;
        
        // Register and commit 10 transactions
        for i in 1..=10 {
            manager.register_transaction(i);
            manager.commit_transaction(i);
        }
        
        // Should only keep last 5
        assert_eq!(manager.get_committed_transactions().len(), 5);
        assert_eq!(manager.get_committed_transactions(), &[6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_concurrent_transaction_ids() {
        use std::thread;
        
        let manager = Arc::new(parking_lot::RwLock::new(TransactionManager::new()));
        let mut handles = vec![];
        
        // Spawn 10 threads, each registering 100 transactions
        for t in 0..10 {
            let manager = Arc::clone(&manager);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let txn_id = t * 100 + i;
                    let mut mgr = manager.write();
                    mgr.register_transaction(txn_id as u64);
                    if i % 2 == 0 {
                        mgr.commit_transaction(txn_id as u64);
                    } else {
                        mgr.abort_transaction(txn_id as u64);
                    }
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        let mgr = manager.read();
        assert_eq!(mgr.active_count(), 0); // All committed or aborted
        assert_eq!(mgr.get_committed_transactions().len(), 500);
        assert_eq!(mgr.get_aborted_transactions().len(), 500);
    }
}
