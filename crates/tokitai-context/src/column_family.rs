//! P3-005: Column Family Support
//!
//! This module provides column family isolation for tokitai-context,
//! allowing multiple logical key-value stores within a single physical storage.
//!
//! # Features
//! - Isolated column families with independent configurations
//! - Batch operations within and across column families
//! - Column family statistics and metrics
//! - Prometheus metrics export
//! - Async/await API
//!
//! # Example
//! ```rust,no_run
//! use tokitai_context::column_family::{ColumnFamilyManager, ColumnFamilyConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let mut manager = ColumnFamilyManager::new();
//!     
//!     // Create column families
//!     manager.create_family("default", ColumnFamilyConfig::default())?;
//!     manager.create_family("users", ColumnFamilyConfig::default())?;
//!     manager.create_family("sessions", ColumnFamilyConfig::default())?;
//!     
//!     // Put data in different families
//!     let users = manager.get_family("users")?;
//!     users.put(b"user:1", b"alice".to_vec()).await?;
//!     
//!     // Get data from specific family
//!     let value = users.get(b"user:1").await?;
//!     println!("User: {:?}", value);
//!     
//!     Ok(())
//! }
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use dashmap::DashMap;
use parking_lot::RwLock;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// Result type for column family operations
pub type ColumnFamilyResult<T> = Result<T, ColumnFamilyError>;

/// Error types for column family operations
#[derive(Debug, thiserror::Error)]
pub enum ColumnFamilyError {
    #[error("Column family not found: {0}")]
    NotFound(String),

    #[error("Column family already exists: {0}")]
    AlreadyExists(String),

    #[error("Invalid column family name: {0}")]
    InvalidName(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Batch operation failed: {0}")]
    BatchFailed(String),
}

/// Configuration for a column family
#[derive(Clone, Debug)]
pub struct ColumnFamilyConfig {
    /// Maximum size in bytes before triggering compaction
    pub max_size: u64,
    /// Block cache size in bytes
    pub block_cache_size: u64,
    /// Enable bloom filter
    pub enable_bloom_filter: bool,
    /// Compression algorithm
    pub compression: CompressionType,
    /// Write buffer size
    pub write_buffer_size: u64,
    /// Number of levels in LSM tree
    pub num_levels: usize,
}

impl Default for ColumnFamilyConfig {
    fn default() -> Self {
        Self {
            max_size: 1024 * 1024 * 1024, // 1GB
            block_cache_size: 64 * 1024 * 1024, // 64MB
            enable_bloom_filter: true,
            compression: CompressionType::None,
            write_buffer_size: 64 * 1024 * 1024, // 64MB
            num_levels: 7,
        }
    }
}

impl ColumnFamilyConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum size
    pub fn with_max_size(mut self, size: u64) -> Self {
        self.max_size = size;
        self
    }

    /// Set block cache size
    pub fn with_block_cache_size(mut self, size: u64) -> Self {
        self.block_cache_size = size;
        self
    }

    /// Enable/disable bloom filter
    pub fn with_bloom_filter(mut self, enable: bool) -> Self {
        self.enable_bloom_filter = enable;
        self
    }

    /// Set compression type
    pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.compression = compression;
        self
    }
}

/// Compression algorithm options
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CompressionType {
    #[default]
    None,
    Snappy,
    Zlib,
    Bz2,
    Lz4,
    Zstd,
}

impl CompressionType {
    /// Get compression name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            CompressionType::None => "none",
            CompressionType::Snappy => "snappy",
            CompressionType::Zlib => "zlib",
            CompressionType::Bz2 => "bz2",
            CompressionType::Lz4 => "lz4",
            CompressionType::Zstd => "zstd",
        }
    }
}

/// Statistics for a column family
#[derive(Debug, Default)]
pub struct ColumnFamilyStats {
    /// Total number of puts
    total_puts: AtomicU64,
    /// Total number of gets
    total_gets: AtomicU64,
    /// Total number of deletes
    total_deletes: AtomicU64,
    /// Total number of bytes written
    total_bytes_written: AtomicU64,
    /// Total number of bytes read
    total_bytes_read: AtomicU64,
    /// Current estimated size
    total_estimated_size: AtomicU64,
    /// Number of files
    num_files: AtomicU64,
    /// Cache hits
    cache_hits: AtomicU64,
    /// Cache misses
    cache_misses: AtomicU64,
}

#[cfg(test)]
impl Clone for ColumnFamilyStats {
    fn clone(&self) -> Self {
        Self {
            total_puts: AtomicU64::new(self.total_puts.load(Ordering::Relaxed)),
            total_gets: AtomicU64::new(self.total_gets.load(Ordering::Relaxed)),
            total_deletes: AtomicU64::new(self.total_deletes.load(Ordering::Relaxed)),
            total_bytes_written: AtomicU64::new(self.total_bytes_written.load(Ordering::Relaxed)),
            total_bytes_read: AtomicU64::new(self.total_bytes_read.load(Ordering::Relaxed)),
            total_estimated_size: AtomicU64::new(self.total_estimated_size.load(Ordering::Relaxed)),
            num_files: AtomicU64::new(self.num_files.load(Ordering::Relaxed)),
            cache_hits: AtomicU64::new(self.cache_hits.load(Ordering::Relaxed)),
            cache_misses: AtomicU64::new(self.cache_misses.load(Ordering::Relaxed)),
        }
    }
}

impl ColumnFamilyStats {
    /// Create new stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Increment puts
    pub fn inc_puts(&self, bytes: u64) {
        self.total_puts.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_written.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment gets
    pub fn inc_gets(&self, bytes: u64) {
        self.total_gets.fetch_add(1, Ordering::Relaxed);
        self.total_bytes_read.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Increment deletes
    pub fn inc_deletes(&self) {
        self.total_deletes.fetch_add(1, Ordering::Relaxed);
    }

    /// Update estimated size
    pub fn set_estimated_size(&self, size: u64) {
        self.total_estimated_size.store(size, Ordering::Relaxed);
    }

    /// Increment file count
    pub fn inc_num_files(&self) {
        self.num_files.fetch_add(1, Ordering::Relaxed);
    }

    /// Record cache hit
    pub fn record_cache_hit(&self) {
        self.cache_hits.fetch_add(1, Ordering::Relaxed);
    }

    /// Record cache miss
    pub fn record_cache_miss(&self) {
        self.cache_misses.fetch_add(1, Ordering::Relaxed);
    }

    /// Get cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        let hits = self.cache_hits.load(Ordering::Relaxed) as f64;
        let misses = self.cache_misses.load(Ordering::Relaxed) as f64;
        let total = hits + misses;
        if total == 0.0 {
            0.0
        } else {
            hits / total
        }
    }

    /// Export to Prometheus format
    pub fn to_prometheus(&self, prefix: &str, family_name: &str) -> String {
        let mut metrics = String::new();
        
        metrics.push_str(&format!(
            "{}_puts_total{{family=\"{}\"}} {}\n",
            prefix, family_name, self.total_puts.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_gets_total{{family=\"{}\"}} {}\n",
            prefix, family_name, self.total_gets.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_deletes_total{{family=\"{}\"}} {}\n",
            prefix, family_name, self.total_deletes.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_bytes_written_total{{family=\"{}\"}} {}\n",
            prefix, family_name, self.total_bytes_written.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_bytes_read_total{{family=\"{}\"}} {}\n",
            prefix, family_name, self.total_bytes_read.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_estimated_size_bytes{{family=\"{}\"}} {}\n",
            prefix, family_name, self.total_estimated_size.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_num_files{{family=\"{}\"}} {}\n",
            prefix, family_name, self.num_files.load(Ordering::Relaxed)
        ));
        metrics.push_str(&format!(
            "{}_cache_hit_rate{{family=\"{}\"}} {:.4}\n",
            prefix, family_name, self.cache_hit_rate()
        ));

        metrics
    }
}

/// A single column family with isolated storage
pub struct ColumnFamily {
    name: String,
    config: ColumnFamilyConfig,
    storage: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
    stats: Arc<ColumnFamilyStats>,
    path: PathBuf,
}

impl ColumnFamily {
    /// Create a new column family
    pub fn new(name: String, config: ColumnFamilyConfig, path: PathBuf) -> Self {
        Self {
            name,
            config,
            storage: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(ColumnFamilyStats::new()),
            path,
        }
    }

    /// Get column family name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get column family configuration
    pub fn config(&self) -> &ColumnFamilyConfig {
        &self.config
    }

    /// Get statistics reference
    pub fn stats(&self) -> Arc<ColumnFamilyStats> {
        self.stats.clone()
    }

    /// Get storage path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Put a key-value pair
    pub async fn put(&self, key: &[u8], value: Vec<u8>) -> ColumnFamilyResult<()> {
        let bytes = value.len() as u64;
        let mut storage = self.storage.lock().await;
        storage.insert(key.to_vec(), value);
        self.stats.inc_puts(bytes);
        debug!("Put key {:?} in family {}", hex::encode(key), self.name);
        Ok(())
    }

    /// Get a value by key
    pub async fn get(&self, key: &[u8]) -> ColumnFamilyResult<Option<Vec<u8>>> {
        let storage = self.storage.lock().await;
        let result = storage.get(key).cloned();

        if let Some(value) = &result {
            self.stats.inc_gets(value.len() as u64);
            self.stats.record_cache_hit();
        } else {
            self.stats.record_cache_miss();
        }

        debug!("Get key {:?} in family {}: {:?}", hex::encode(key), self.name, result.is_some());
        Ok(result)
    }

    /// Delete a key
    pub async fn delete(&self, key: &[u8]) -> ColumnFamilyResult<()> {
        let mut storage = self.storage.lock().await;
        let existed = storage.remove(key);
        self.stats.inc_deletes();
        debug!("Delete key {:?} in family {}: {:?}", hex::encode(key), self.name, existed.is_some());
        Ok(())
    }

    /// Check if key exists
    pub async fn exists(&self, key: &[u8]) -> ColumnFamilyResult<bool> {
        let storage = self.storage.lock().await;
        Ok(storage.contains_key(key))
    }

    /// Get all keys
    pub async fn keys(&self) -> ColumnFamilyResult<Vec<Vec<u8>>> {
        let storage = self.storage.lock().await;
        Ok(storage.keys().cloned().collect())
    }

    /// Get all key-value pairs
    pub async fn iter(&self) -> ColumnFamilyResult<Vec<(Vec<u8>, Vec<u8>)>> {
        let storage = self.storage.lock().await;
        Ok(storage.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
    }

    /// Batch put multiple key-value pairs
    pub async fn batch_put(&self, entries: Vec<(Vec<u8>, Vec<u8>)>) -> ColumnFamilyResult<()> {
        let mut storage = self.storage.lock().await;
        let mut total_bytes = 0u64;
        
        for (key, value) in &entries {
            total_bytes += value.len() as u64;
            storage.insert(key.clone(), value.clone());
        }
        
        self.stats.inc_puts(total_bytes);
        debug!("Batch put {} entries in family {}", entries.len(), self.name);
        Ok(())
    }

    /// Batch delete multiple keys
    pub async fn batch_delete(&self, keys: Vec<Vec<u8>>) -> ColumnFamilyResult<()> {
        let mut storage = self.storage.lock().await;
        
        for key in &keys {
            storage.remove(key);
        }
        
        self.stats.inc_deletes();
        debug!("Batch delete {} keys in family {}", keys.len(), self.name);
        Ok(())
    }

    /// Get estimated size
    pub async fn estimated_size(&self) -> u64 {
        let storage = self.storage.lock().await;
        let size: u64 = storage.iter()
            .map(|(k, v)| (k.len() + v.len()) as u64)
            .sum();
        self.stats.set_estimated_size(size);
        size
    }

    /// Clear all data
    pub async fn clear(&self) -> ColumnFamilyResult<()> {
        let mut storage = self.storage.lock().await;
        storage.clear();
        debug!("Cleared all data in family {}", self.name);
        Ok(())
    }

    /// Export metrics to Prometheus format
    pub fn to_prometheus(&self) -> String {
        self.stats.to_prometheus("tokitai_column_family", &self.name)
    }
}

/// Batch operation for column families
#[derive(Clone, Debug)]
pub struct BatchOperation {
    family_name: String,
    operations: Vec<BatchOp>,
}

impl BatchOperation {
    /// Create a new batch operation for a specific family
    pub fn new(family_name: String) -> Self {
        Self {
            family_name,
            operations: Vec::new(),
        }
    }

    /// Add a put operation
    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.operations.push(BatchOp::Put { key, value });
    }

    /// Add a delete operation
    pub fn delete(&mut self, key: Vec<u8>) {
        self.operations.push(BatchOp::Delete { key });
    }

    /// Get number of operations
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

/// Single operation in a batch
#[derive(Clone, Debug)]
pub enum BatchOp {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

/// Manager for multiple column families
pub struct ColumnFamilyManager {
    families: DashMap<String, Arc<ColumnFamily>>,
    root_path: PathBuf,
    default_family_created: AtomicU64,
}

impl ColumnFamilyManager {
    /// Create a new column family manager
    pub fn new() -> Self {
        Self {
            families: DashMap::new(),
            root_path: PathBuf::from("./.tokitai/column_families"),
            default_family_created: AtomicU64::new(0),
        }
    }

    /// Create with custom root path
    pub fn with_root_path<P: AsRef<Path>>(root_path: P) -> Self {
        Self {
            families: DashMap::new(),
            root_path: root_path.as_ref().to_path_buf(),
            default_family_created: AtomicU64::new(0),
        }
    }

    /// Initialize the manager and create default family
    pub async fn init(&self) -> ColumnFamilyResult<()> {
        // Create root directory
        std::fs::create_dir_all(&self.root_path)?;

        // Create default column family if not exists
        if !self.families.contains_key("default") {
            self.create_family("default", ColumnFamilyConfig::default())?;
            self.default_family_created.store(1, Ordering::Relaxed);
        }

        info!("Column family manager initialized at {:?}", self.root_path);
        Ok(())
    }

    /// Create a new column family
    pub fn create_family(&self, name: &str, config: ColumnFamilyConfig) -> ColumnFamilyResult<()> {
        // Validate name
        if name.is_empty() {
            return Err(ColumnFamilyError::InvalidName("Name cannot be empty".to_string()));
        }
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(ColumnFamilyError::InvalidName(
                format!("Invalid characters in name: {}", name)
            ));
        }

        // Check if already exists
        if self.families.contains_key(name) {
            return Err(ColumnFamilyError::AlreadyExists(name.to_string()));
        }

        // Create family directory
        let family_path = self.root_path.join(name);
        std::fs::create_dir_all(&family_path)?;

        // Create column family
        let family = ColumnFamily::new(name.to_string(), config, family_path);
        self.families.insert(name.to_string(), Arc::new(family));

        info!("Created column family: {}", name);
        Ok(())
    }

    /// Get a column family by name
    pub fn get_family(&self, name: &str) -> ColumnFamilyResult<Arc<ColumnFamily>> {
        self.families
            .get(name)
            .map(|entry| entry.clone())
            .ok_or_else(|| ColumnFamilyError::NotFound(name.to_string()))
    }

    /// Get the default column family
    pub fn default_family(&self) -> ColumnFamilyResult<Arc<ColumnFamily>> {
        self.get_family("default")
    }

    /// List all column family names
    pub fn list_families(&self) -> Vec<String> {
        self.families.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get number of column families
    pub fn num_families(&self) -> usize {
        self.families.len()
    }

    /// Check if a column family exists
    pub fn has_family(&self, name: &str) -> bool {
        self.families.contains_key(name)
    }

    /// Drop a column family
    pub fn drop_family(&self, name: &str) -> ColumnFamilyResult<()> {
        if name == "default" {
            return Err(ColumnFamilyError::InvalidName(
                "Cannot drop default family".to_string()
            ));
        }

        let removed = self.families.remove(name);
        if removed.is_none() {
            return Err(ColumnFamilyError::NotFound(name.to_string()));
        }

        // Remove directory (best effort)
        let family_path = self.root_path.join(name);
        if family_path.exists() {
            let _ = std::fs::remove_dir_all(&family_path);
        }

        info!("Dropped column family: {}", name);
        Ok(())
    }

    /// Execute a batch operation
    pub async fn batch(&self, batch: BatchOperation) -> ColumnFamilyResult<()> {
        let family = self.get_family(&batch.family_name)?;
        
        let puts: Vec<_> = batch.operations.iter()
            .filter_map(|op| match op {
                BatchOp::Put { key, value } => Some((key.clone(), value.clone())),
                _ => None,
            })
            .collect();

        let deletes: Vec<_> = batch.operations.iter()
            .filter_map(|op| match op {
                BatchOp::Delete { key } => Some(key.clone()),
                _ => None,
            })
            .collect();

        if !puts.is_empty() {
            family.batch_put(puts).await?;
        }

        if !deletes.is_empty() {
            family.batch_delete(deletes).await?;
        }

        Ok(())
    }

    /// Execute batch operations across multiple families
    pub async fn batch_multi(&self, batches: Vec<BatchOperation>) -> ColumnFamilyResult<()> {
        for batch in batches {
            self.batch(batch).await?;
        }
        Ok(())
    }

    /// Get total statistics across all families
    pub fn total_stats(&self) -> ColumnFamilyStats {
        let total = ColumnFamilyStats::new();
        
        for entry in self.families.iter() {
            let stats = entry.value().stats();
            total.total_puts.fetch_add(stats.total_puts.load(Ordering::Relaxed), Ordering::Relaxed);
            total.total_gets.fetch_add(stats.total_gets.load(Ordering::Relaxed), Ordering::Relaxed);
            total.total_deletes.fetch_add(stats.total_deletes.load(Ordering::Relaxed), Ordering::Relaxed);
            total.total_bytes_written.fetch_add(stats.total_bytes_written.load(Ordering::Relaxed), Ordering::Relaxed);
            total.total_bytes_read.fetch_add(stats.total_bytes_read.load(Ordering::Relaxed), Ordering::Relaxed);
            total.cache_hits.fetch_add(stats.cache_hits.load(Ordering::Relaxed), Ordering::Relaxed);
            total.cache_misses.fetch_add(stats.cache_misses.load(Ordering::Relaxed), Ordering::Relaxed);
        }

        total
    }

    /// Export all metrics to Prometheus format
    pub fn to_prometheus(&self) -> String {
        let mut metrics = String::new();
        
        for entry in self.families.iter() {
            metrics.push_str(&entry.value().to_prometheus());
        }

        metrics
    }

    /// Get root path
    pub fn root_path(&self) -> &Path {
        &self.root_path
    }
}

impl Default for ColumnFamilyManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_column_family_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        manager.create_family("test", ColumnFamilyConfig::default()).unwrap();
        assert!(manager.has_family("test"));
        assert!(manager.has_family("default"));
    }

    #[tokio::test]
    async fn test_column_family_put_get() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();

        let value = family.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
    }

    #[tokio::test]
    async fn test_column_family_delete() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();
        family.delete(b"key1").await.unwrap();

        let value = family.get(b"key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_column_family_exists() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();

        assert!(family.exists(b"key1").await.unwrap());
        assert!(!family.exists(b"nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_column_family_batch_put() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        let entries = vec![
            (b"key1".to_vec(), b"value1".to_vec()),
            (b"key2".to_vec(), b"value2".to_vec()),
            (b"key3".to_vec(), b"value3".to_vec()),
        ];
        family.batch_put(entries).await.unwrap();

        assert_eq!(family.get(b"key1").await.unwrap(), Some(b"value1".to_vec()));
        assert_eq!(family.get(b"key2").await.unwrap(), Some(b"value2".to_vec()));
        assert_eq!(family.get(b"key3").await.unwrap(), Some(b"value3".to_vec()));
    }

    #[tokio::test]
    async fn test_column_family_batch_operation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let mut batch = BatchOperation::new("default".to_string());
        batch.put(b"key1".to_vec(), b"value1".to_vec());
        batch.put(b"key2".to_vec(), b"value2".to_vec());
        batch.delete(b"key3".to_vec());

        manager.batch(batch).await.unwrap();

        let family = manager.get_family("default").unwrap();
        assert_eq!(family.get(b"key1").await.unwrap(), Some(b"value1".to_vec()));
        assert_eq!(family.get(b"key2").await.unwrap(), Some(b"value2".to_vec()));
    }

    #[tokio::test]
    async fn test_column_family_iter() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();
        family.put(b"key2", b"value2".to_vec()).await.unwrap();

        let entries = family.iter().await.unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_column_family_stats() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();
        family.get(b"key1").await.unwrap();
        family.delete(b"key1").await.unwrap();

        let stats = family.stats();
        assert_eq!(stats.total_puts.load(Ordering::Relaxed), 1);
        assert_eq!(stats.total_gets.load(Ordering::Relaxed), 1);
        assert_eq!(stats.total_deletes.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn test_column_family_prometheus() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();

        let metrics = family.to_prometheus();
        assert!(metrics.contains("tokitai_column_family_puts_total"));
        assert!(metrics.contains("family=\"default\""));
    }

    #[tokio::test]
    async fn test_column_family_manager_prometheus() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        manager.create_family("users", ColumnFamilyConfig::default()).unwrap();

        let metrics = manager.to_prometheus();
        assert!(metrics.contains("family=\"default\""));
        assert!(metrics.contains("family=\"users\""));
    }

    #[tokio::test]
    async fn test_column_family_drop() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        manager.create_family("temp", ColumnFamilyConfig::default()).unwrap();
        assert!(manager.has_family("temp"));

        manager.drop_family("temp").unwrap();
        assert!(!manager.has_family("temp"));
    }

    #[tokio::test]
    async fn test_column_family_cannot_drop_default() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let result = manager.drop_family("default");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_column_family_invalid_name() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let result = manager.create_family("", ColumnFamilyConfig::default());
        assert!(result.is_err());

        let result = manager.create_family("invalid@name", ColumnFamilyConfig::default());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_column_family_already_exists() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        manager.create_family("test", ColumnFamilyConfig::default()).unwrap();
        let result = manager.create_family("test", ColumnFamilyConfig::default());
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_column_family_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let result = manager.get_family("nonexistent");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_column_family_clear() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();
        family.put(b"key2", b"value2".to_vec()).await.unwrap();
        family.clear().await.unwrap();

        let keys = family.keys().await.unwrap();
        assert_eq!(keys.len(), 0);
    }

    #[tokio::test]
    async fn test_column_family_estimated_size() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        let family = manager.get_family("default").unwrap();
        family.put(b"key1", b"value1".to_vec()).await.unwrap();
        family.put(b"key2", b"value2".to_vec()).await.unwrap();

        let size = family.estimated_size().await;
        assert!(size > 0);
    }

    #[tokio::test]
    async fn test_column_family_multi_family_isolation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        manager.create_family("users", ColumnFamilyConfig::default()).unwrap();
        manager.create_family("sessions", ColumnFamilyConfig::default()).unwrap();

        let users = manager.get_family("users").unwrap();
        let sessions = manager.get_family("sessions").unwrap();

        users.put(b"user:1", b"alice".to_vec()).await.unwrap();
        sessions.put(b"session:1", b"session_data".to_vec()).await.unwrap();

        // Data should be isolated
        assert_eq!(users.get(b"user:1").await.unwrap(), Some(b"alice".to_vec()));
        assert_eq!(users.get(b"session:1").await.unwrap(), None);
        assert_eq!(sessions.get(b"session:1").await.unwrap(), Some(b"session_data".to_vec()));
        assert_eq!(sessions.get(b"user:1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_column_family_total_stats() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ColumnFamilyManager::with_root_path(temp_dir.path());
        manager.init().await.unwrap();

        manager.create_family("users", ColumnFamilyConfig::default()).unwrap();

        let default = manager.get_family("default").unwrap();
        let users = manager.get_family("users").unwrap();

        default.put(b"key1", b"value1".to_vec()).await.unwrap();
        users.put(b"key2", b"value2".to_vec()).await.unwrap();

        let total = manager.total_stats();
        assert_eq!(total.total_puts.load(Ordering::Relaxed), 2);
    }
}
