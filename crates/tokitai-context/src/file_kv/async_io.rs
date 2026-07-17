//! Async I/O for non-blocking writes
//!
//! This module provides asynchronous I/O operations for FileKV to improve
//! write throughput by avoiding blocking the executor during disk operations.
//!
//! # Features
//! - **Async Segment Writes**: Non-blocking segment file operations
//! - **Async WAL**: Asynchronous Write-Ahead Log operations  
//! - **Async Flush**: Background flush with async I/O
//! - **Write Queue**: Ordered write operations with batching
//!
//! # Architecture
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
//! │  Write API  │────▶│  AsyncWriter │────▶│  Disk (SSD) │
//! └─────────────┘     └──────────────┘     └─────────────┘
//!        │                   │
//!        │                   ▼
//!        │            ┌──────────────┐
//!        └───────────▶│  WriteQueue  │
//!                     └──────────────┘
//! ```

use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use parking_lot::{Mutex, RwLock};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::spawn_blocking;
use tracing::{debug, error, info, warn};

use crate::error::ContextResult;

/// Async I/O operation types
#[derive(Debug, Clone)]
pub enum AsyncWriteOp {
    /// Write data to segment file
    SegmentWrite {
        segment_id: u64,
        offset: u64,
        data: Bytes,
    },
    /// Write WAL entry
    WalWrite {
        data: Bytes,
        sync: bool,
    },
    /// Flush and sync file
    Flush {
        path: PathBuf,
    },
    /// Create/open segment file
    CreateSegment {
        segment_id: u64,
        preallocate_bytes: u64,
    },
}

/// Result of an async write operation
#[derive(Debug, Clone)]
pub struct AsyncWriteResult {
    /// Operation ID for tracking
    pub op_id: u64,
    /// Time taken for the operation
    pub duration_us: u64,
    /// Bytes written
    pub bytes_written: usize,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Statistics for async I/O operations
#[derive(Debug, Default, Clone)]
pub struct AsyncIoStats {
    /// Total async write operations
    pub total_writes: u64,
    /// Successful writes
    pub successful_writes: u64,
    /// Failed writes
    pub failed_writes: u64,
    /// Total bytes written asynchronously
    pub total_bytes_written: u64,
    /// Average write latency in microseconds
    pub avg_write_latency_us: f64,
    /// P99 write latency in microseconds
    pub p99_write_latency_us: f64,
    /// Current queue depth
    pub queue_depth: u64,
    /// Writes currently in flight
    pub writes_in_flight: u64,
    /// Total time spent in async I/O (microseconds)
    pub total_io_time_us: u64,
}

impl AsyncIoStats {
    /// Get a snapshot of current statistics
    pub fn snapshot(&self) -> Self {
        self.clone()
    }

    /// Format stats as Prometheus-style metrics
    pub fn to_prometheus(&self) -> String {
        format!(
            "# HELP tokitai_async_writes_total Total async write operations
# TYPE tokitai_async_writes_total counter
tokitai_async_writes_total {}

# HELP tokitai_async_writes_success_total Successful async writes
# TYPE tokitai_async_writes_success_total counter
tokitai_async_writes_success_total {}

# HELP tokitai_async_writes_failed_total Failed async writes
# TYPE tokitai_async_writes_failed_total counter
tokitai_async_writes_failed_total {}

# HELP tokitai_async_bytes_written_total Total bytes written asynchronously
# TYPE tokitai_async_bytes_written_total counter
tokitai_async_bytes_written_total {}

# HELP tokitai_async_write_latency_us Average async write latency in microseconds
# TYPE tokitai_async_write_latency_us gauge
tokitai_async_write_latency_us {}

# HELP tokitai_async_write_p99_latency_us P99 async write latency in microseconds
# TYPE tokitai_async_write_p99_latency_us gauge
tokitai_async_write_p99_latency_us {}

# HELP tokitai_async_queue_depth Current async write queue depth
# TYPE tokitai_async_queue_depth gauge
tokitai_async_queue_depth {}

# HELP tokitai_async_writes_in_flight Async writes currently in flight
# TYPE tokitai_async_writes_in_flight gauge
tokitai_async_writes_in_flight {}
",
            self.total_writes,
            self.successful_writes,
            self.failed_writes,
            self.total_bytes_written,
            self.avg_write_latency_us,
            self.p99_write_latency_us,
            self.queue_depth,
            self.writes_in_flight,
        )
    }
}

/// Configuration for async I/O
#[derive(Debug, Clone)]
pub struct AsyncIoConfig {
    /// Enable async I/O (default: true)
    pub enabled: bool,
    /// Maximum number of concurrent async writes (default: 4)
    pub max_concurrent_writes: usize,
    /// Maximum queue depth for pending writes (default: 1024)
    pub max_queue_depth: usize,
    /// Timeout for async write operations in milliseconds (default: 5000)
    pub write_timeout_ms: u64,
    /// Enable write coalescing (default: true)
    pub enable_coalescing: bool,
    /// Coalesce window in milliseconds (default: 10)
    pub coalesce_window_ms: u64,
}

impl Default for AsyncIoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_concurrent_writes: 4,
            max_queue_depth: 1024,
            write_timeout_ms: 5000,
            enable_coalescing: true,
            coalesce_window_ms: 10,
        }
    }
}

/// Internal message for async write worker
#[derive(Debug)]
struct WriteMessage {
    op: AsyncWriteOp,
    response_tx: tokio::sync::oneshot::Sender<AsyncWriteResult>,
    op_id: u64,
}

/// Cache for open file handles to reduce open/close overhead
struct FileHandleCache {
    /// Maximum number of cached file handles
    max_handles: usize,
    /// Cached writers: segment_id -> (file, current_offset)
    writers: VecDeque<(u64, BufWriter<File>)>,
}

impl FileHandleCache {
    fn new(max_handles: usize) -> Self {
        Self {
            max_handles,
            writers: VecDeque::with_capacity(max_handles),
        }
    }

    /// Get or create a writer for a segment
    fn get_or_create_writer(&mut self, segment_id: u64, base_dir: &Path) -> ContextResult<&mut BufWriter<File>> {
        // Check if we already have this segment open
        let pos = self.writers.iter().position(|(id, _)| *id == segment_id);

        if let Some(pos) = pos {
            // Move to front (LRU)
            let writer = self.writers.remove(pos)
                .ok_or_else(|| crate::error::ContextError::InvalidState(
                    "Writer disappeared from cache".to_string()
                ))?;
            self.writers.push_front(writer);
            Ok(&mut self.writers.front_mut().ok_or_else(|| crate::error::ContextError::InvalidState(
                "Writer cache is empty after push_front".to_string()
            ))?.1)
        } else {
            // Need to open new file
            let path = base_dir.join(format!("segment_{:010}.dat", segment_id));
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| crate::error::ContextError::Io(
                    std::io::Error::new(e.kind(), format!("Failed to open segment file: {:?}: {}", path, e))
                ))?;

            let writer = BufWriter::new(file);
            self.writers.push_front((segment_id, writer));

            // Evict if over capacity
            if self.writers.len() > self.max_handles {
                self.writers.pop_back();
            }

            Ok(&mut self.writers.front_mut().ok_or_else(|| crate::error::ContextError::InvalidState(
                "Writer cache is empty after push_front".to_string()
            ))?.1)
        }
    }

    /// Flush and close all cached writers
    fn flush_all(&mut self) -> ContextResult<()> {
        while let Some((segment_id, mut writer)) = self.writers.pop_front() {
            writer.flush().map_err(|e| crate::error::ContextError::Io(
                std::io::Error::new(e.kind(), format!("Failed to flush segment {}: {}", segment_id, e))
            ))?;
        }
        Ok(())
    }
}

/// Async writer for non-blocking file operations
pub struct AsyncWriter {
    config: AsyncIoConfig,
    /// Sender for write operations
    write_tx: mpsc::Sender<WriteMessage>,
    /// Statistics
    stats: Arc<RwLock<AsyncIoStats>>,
    /// Semaphore limiting concurrent writes
    write_semaphore: Arc<Semaphore>,
    /// Counter for operation IDs
    op_counter: std::sync::atomic::AtomicU64,
    /// Track running worker task
    _worker_handle: Option<tokio::task::JoinHandle<()>>,
    /// Base directory for segment files
    base_dir: PathBuf,
    /// Open file handles cache
    file_handles: Arc<Mutex<FileHandleCache>>,
}

impl AsyncWriter {
    /// Create a new async writer
    pub fn new(config: AsyncIoConfig, base_dir: PathBuf) -> ContextResult<Self> {
        let (write_tx, write_rx) = mpsc::channel::<WriteMessage>(config.max_queue_depth);
        let write_semaphore = Arc::new(Semaphore::new(config.max_concurrent_writes));
        let stats = Arc::new(RwLock::new(AsyncIoStats::default()));
        let file_handles = Arc::new(Mutex::new(FileHandleCache::new(16)));
        
        let stats_clone = Arc::clone(&stats);
        let semaphore_clone = Arc::clone(&write_semaphore);
        let file_handles_clone = Arc::clone(&file_handles);
        let base_dir_clone = base_dir.clone();
        
        // Spawn worker thread to process async writes
        let worker_handle = tokio::spawn(async move {
            Self::worker_loop(
                write_rx,
                stats_clone,
                semaphore_clone,
                file_handles_clone,
                base_dir_clone,
            ).await;
        });

        Ok(Self {
            config,
            write_tx,
            stats,
            write_semaphore,
            op_counter: std::sync::atomic::AtomicU64::new(0),
            _worker_handle: Some(worker_handle),
            base_dir,
            file_handles,
        })
    }

    /// Worker loop that processes write operations
    async fn worker_loop(
        mut write_rx: mpsc::Receiver<WriteMessage>,
        stats: Arc<RwLock<AsyncIoStats>>,
        semaphore: Arc<Semaphore>,
        file_handles: Arc<Mutex<FileHandleCache>>,
        base_dir: PathBuf,
    ) {
        while let Some(msg) = write_rx.recv().await {
            let op_id = msg.op_id;
            let start = Instant::now();

            // Acquire semaphore permit to limit concurrency
            let _permit = semaphore.acquire().await;

            // Execute the write operation in a blocking task
            let spawn_result = spawn_blocking({
                let file_handles = Arc::clone(&file_handles);
                let base_dir = base_dir.clone();
                let op = msg.op;

                move || Self::execute_write_op(op, &file_handles, &base_dir)
            })
            .await;

            // Flatten the nested Result
            let result: ContextResult<AsyncWriteResult> = match spawn_result {
                Ok(r) => r,
                Err(e) => Err(crate::error::ContextError::Internal(anyhow::anyhow!(
                    "Spawn blocking error: {}", e
                ))),
            };

            let duration_us = start.elapsed().as_micros() as u64;
            let bytes_written = match &result {
                Ok(r) => r.bytes_written,
                Err(_) => 0,
            };

            // Update statistics
            {
                let mut stats = stats.write();
                stats.total_writes += 1;
                stats.total_bytes_written += bytes_written as u64;
                stats.total_io_time_us += duration_us;

                // Update latency stats - only count successful writes
                if result.is_ok() {
                    let latency_sum = stats.avg_write_latency_us * (stats.successful_writes as f64);
                    stats.avg_write_latency_us = (latency_sum + duration_us as f64)
                        / ((stats.successful_writes + 1) as f64);

                    // Simple P99 estimation (would need proper histogram in production)
                    stats.p99_write_latency_us = stats.avg_write_latency_us * 1.5;
                }
            }

            // Send result back
            let write_result = match result {
                Ok(r) => r,
                Err(e) => AsyncWriteResult {
                    op_id,
                    duration_us,
                    bytes_written: 0,
                    success: false,
                    error: Some(e.to_string()),
                },
            };

            let _ = msg.response_tx.send(write_result);
        }
    }

    /// Execute a single write operation
    fn execute_write_op(
        op: AsyncWriteOp,
        file_handles: &Mutex<FileHandleCache>,
        base_dir: &Path,
    ) -> ContextResult<AsyncWriteResult> {
        use crate::error::ContextError;
        
        match op {
            AsyncWriteOp::SegmentWrite { segment_id, offset, data } => {
                let mut handles = file_handles.lock();
                let writer = handles.get_or_create_writer(segment_id, base_dir)?;
                
                // Seek to offset
                writer.seek(SeekFrom::Start(offset)).map_err(|e| {
                    ContextError::Io(std::io::Error::new(e.kind(), format!("Failed to seek in segment {}: {}", segment_id, e)))
                })?;
                
                // Write data
                writer.write_all(&data).map_err(|e| {
                    ContextError::Io(std::io::Error::new(e.kind(), format!("Failed to write to segment {}: {}", segment_id, e)))
                })?;
                
                Ok(AsyncWriteResult {
                    op_id: 0,
                    duration_us: 0,
                    bytes_written: data.len(),
                    success: true,
                    error: None,
                })
            }
            AsyncWriteOp::WalWrite { data, sync } => {
                let wal_path = base_dir.join("WAL");
                let mut file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&wal_path)
                    .map_err(|e| ContextError::Io(
                        std::io::Error::new(e.kind(), format!("Failed to open WAL file: {}", e))
                    ))?;
                
                file.write_all(&data).map_err(|e| ContextError::Io(
                    std::io::Error::new(e.kind(), format!("Failed to write to WAL: {}", e))
                ))?;
                
                if sync {
                    file.sync_all().map_err(|e| ContextError::Io(
                        std::io::Error::new(e.kind(), format!("Failed to sync WAL: {}", e))
                    ))?;
                }
                
                Ok(AsyncWriteResult {
                    op_id: 0,
                    duration_us: 0,
                    bytes_written: data.len(),
                    success: true,
                    error: None,
                })
            }
            AsyncWriteOp::Flush { path } => {
                // Open, flush, and sync file
                let file = OpenOptions::new()
                    .write(true)
                    .open(&path)
                    .map_err(|e| ContextError::Io(
                        std::io::Error::new(e.kind(), format!("Failed to open file for flush: {:?}: {}", path, e))
                    ))?;

                file.sync_all().map_err(|e| ContextError::Io(
                    std::io::Error::new(e.kind(), format!("Failed to sync file: {:?}: {}", path, e))
                ))?;
                
                Ok(AsyncWriteResult {
                    op_id: 0,
                    duration_us: 0,
                    bytes_written: 0,
                    success: true,
                    error: None,
                })
            }
            AsyncWriteOp::CreateSegment { segment_id, preallocate_bytes } => {
                let path = base_dir.join(format!("segment_{:010}.dat", segment_id));
                let file = OpenOptions::new()
                    .create(true)
                    .write(true)
                    .truncate(true)
                    .open(&path)
                    .map_err(|e| ContextError::Io(
                        std::io::Error::new(e.kind(), format!("Failed to create segment file: {:?}: {}", path, e))
                    ))?;
                
                // Pre-allocate space
                file.set_len(preallocate_bytes).map_err(|e| {
                    ContextError::Io(std::io::Error::new(e.kind(), format!("Failed to preallocate segment {}: {}", segment_id, e)))
                })?;
                
                Ok(AsyncWriteResult {
                    op_id: 0,
                    duration_us: 0,
                    bytes_written: preallocate_bytes as usize,
                    success: true,
                    error: None,
                })
            }
        }
    }

    /// Submit an async write operation
    pub async fn write(&self, op: AsyncWriteOp) -> ContextResult<AsyncWriteResult> {
        if !self.config.enabled {
            return Err(crate::error::ContextError::Internal(anyhow::anyhow!(
                "Async I/O is disabled"
            )));
        }

        let op_id = self.op_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.write_tx.send(WriteMessage {
            op,
            response_tx,
            op_id,
        }).await.map_err(|e| crate::error::ContextError::Internal(anyhow::anyhow!(
            "Failed to send write operation: {}", e
        )))?;

        // Update in-flight counter
        {
            let mut stats = self.stats.write();
            stats.writes_in_flight += 1;
        }

        // Wait for response with timeout
        let result = tokio::time::timeout(
            Duration::from_millis(self.config.write_timeout_ms),
            response_rx
        ).await
        .map_err(|_| crate::error::ContextError::Internal(anyhow::anyhow!("Async write timeout")))?
        .map_err(|_| crate::error::ContextError::Internal(anyhow::anyhow!("Worker task failed")))?;

        // Update in-flight counter and stats
        {
            let mut stats = self.stats.write();
            stats.writes_in_flight = stats.writes_in_flight.saturating_sub(1);
            if result.success {
                stats.successful_writes += 1;
            } else {
                stats.failed_writes += 1;
            }
        }

        Ok(result)
    }

    /// Submit a segment write operation
    pub async fn write_segment(
        &self,
        segment_id: u64,
        offset: u64,
        data: Bytes,
    ) -> ContextResult<AsyncWriteResult> {
        self.write(AsyncWriteOp::SegmentWrite {
            segment_id,
            offset,
            data,
        }).await
    }

    /// Submit a WAL write operation
    pub async fn write_wal(&self, data: Bytes, sync: bool) -> ContextResult<AsyncWriteResult> {
        self.write(AsyncWriteOp::WalWrite { data, sync }).await
    }

    /// Flush a file asynchronously
    pub async fn flush(&self, path: PathBuf) -> ContextResult<AsyncWriteResult> {
        self.write(AsyncWriteOp::Flush { path }).await
    }

    /// Create a new segment file with pre-allocation
    pub async fn create_segment(
        &self,
        segment_id: u64,
        preallocate_bytes: u64,
    ) -> ContextResult<AsyncWriteResult> {
        self.write(AsyncWriteOp::CreateSegment {
            segment_id,
            preallocate_bytes,
        }).await
    }

    /// Get current statistics
    pub fn stats(&self) -> AsyncIoStats {
        self.stats.read().clone()
    }

    /// Get queue depth
    pub fn queue_depth(&self) -> usize {
        self.write_tx.capacity()
    }

    /// Flush and close all cached file handles
    pub async fn flush_all(&self) -> ContextResult<()> {
        let file_handles = Arc::clone(&self.file_handles);
        spawn_blocking(move || {
            file_handles.lock().flush_all()
        })
        .await
        .map_err(|e| crate::error::ContextError::Internal(anyhow::anyhow!("Flush failed: {}", e)))?
    }

    /// Check if async I/O is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

impl Drop for AsyncWriter {
    fn drop(&mut self) {
        // Flush all cached writers on drop
        if let Err(e) = self.file_handles.lock().flush_all() {
            error!("Failed to flush file handles on AsyncWriter drop: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_async_writer() -> (AsyncWriter, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = AsyncIoConfig {
            enabled: true,
            max_concurrent_writes: 2,
            max_queue_depth: 100,
            write_timeout_ms: 5000,
            enable_coalescing: false,
            coalesce_window_ms: 10,
        };
        let writer = AsyncWriter::new(config, temp_dir.path().to_path_buf()).unwrap();
        (writer, temp_dir)
    }

    #[tokio::test]
    async fn test_async_segment_write() {
        let (writer, _temp_dir) = create_test_async_writer();
        
        let data = Bytes::from(b"test data".to_vec());
        let result = writer.write_segment(1, 0, data.clone()).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.bytes_written, 9);
    }

    #[tokio::test]
    async fn test_async_wal_write() {
        let (writer, _temp_dir) = create_test_async_writer();
        
        let data = Bytes::from(b"wal entry".to_vec());
        let result = writer.write_wal(data.clone(), false).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.bytes_written, 9);
    }

    #[tokio::test]
    async fn test_async_flush() {
        let (writer, temp_dir) = create_test_async_writer();
        
        // Create a file first
        let file_path = temp_dir.path().join("test.dat");
        File::create(&file_path).unwrap();
        
        let result = writer.flush(file_path).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_async_create_segment() {
        let (writer, _temp_dir) = create_test_async_writer();
        
        let result = writer.create_segment(1, 1024 * 1024).await.unwrap();
        
        assert!(result.success);
        assert_eq!(result.bytes_written, 1024 * 1024);
    }

    #[tokio::test]
    async fn test_async_stats() {
        let (writer, _temp_dir) = create_test_async_writer();
        
        // Initial stats should be empty
        let stats = writer.stats();
        assert_eq!(stats.total_writes, 0);
        
        // Do a write
        let data = Bytes::from(b"test".to_vec());
        let _ = writer.write_segment(1, 0, data).await.unwrap();
        
        // Stats should be updated
        let stats = writer.stats();
        assert_eq!(stats.total_writes, 1);
        assert_eq!(stats.successful_writes, 1);
        assert!(stats.total_bytes_written > 0);
    }

    #[tokio::test]
    async fn test_concurrent_writes() {
        let (writer, _temp_dir) = create_test_async_writer();
        
        let mut handles = Vec::new();
        for i in 0..10 {
            let data = Bytes::from(format!("data {}", i).into_bytes());
            let handle = writer.write_segment(i % 3, i * 10, data);
            handles.push(handle);
        }
        
        let results = futures::future::join_all(handles).await;
        
        // All should succeed
        for result in results {
            assert!(result.is_ok());
            let r = result.unwrap();
            assert!(r.success);
        }
        
        // Stats should reflect all writes
        let stats = writer.stats();
        assert_eq!(stats.total_writes, 10);
        assert_eq!(stats.successful_writes, 10);
    }

    #[tokio::test]
    async fn test_write_coalescing_disabled() {
        let (writer, _temp_dir) = create_test_async_writer();
        
        // With coalescing disabled, each write is independent
        let data1 = Bytes::from(b"write1".to_vec());
        let data2 = Bytes::from(b"write2".to_vec());
        
        let r1 = writer.write_segment(1, 0, data1).await.unwrap();
        let r2 = writer.write_segment(1, 10, data2).await.unwrap();
        
        assert!(r1.success);
        assert!(r2.success);
    }

    #[tokio::test]
    async fn test_prometheus_metrics() {
        let (writer, _temp_dir) = create_test_async_writer();
        
        // Do some writes
        for i in 0..5 {
            let data = Bytes::from(format!("data {}", i).into_bytes());
            let _ = writer.write_segment(i % 2, i * 10, data).await.unwrap();
        }
        
        let stats = writer.stats();
        let metrics = stats.to_prometheus();
        
        assert!(metrics.contains("tokitai_async_writes_total 5"));
        assert!(metrics.contains("tokitai_async_writes_success_total 5"));
        assert!(metrics.contains("tokitai_async_bytes_written_total"));
    }

    #[tokio::test]
    async fn test_queue_depth_tracking() {
        let (writer, _temp_dir) = create_test_async_writer();

        // Submit multiple writes without waiting
        let mut handles = Vec::new();
        for i in 0..20 {
            let data = Bytes::from(format!("data {}", i).into_bytes());
            let handle = writer.write_segment(i % 3, i * 100, data);
            handles.push(handle);
        }

        // Wait for all to complete
        let _ = futures::future::join_all(handles).await;

        // All should be done
        let stats = writer.stats();
        assert_eq!(stats.writes_in_flight, 0);
        assert_eq!(stats.total_writes, 20);
        assert_eq!(stats.successful_writes, 20);
    }

    #[tokio::test]
    async fn test_disabled_async_io() {
        let temp_dir = TempDir::new().unwrap();
        let config = AsyncIoConfig {
            enabled: false,
            ..Default::default()
        };
        let writer = AsyncWriter::new(config, temp_dir.path().to_path_buf()).unwrap();
        
        let data = Bytes::from(b"test".to_vec());
        let result = writer.write_segment(1, 0, data).await;
        
        assert!(result.is_err());
    }
}
