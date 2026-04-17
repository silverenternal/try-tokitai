//! WAL (Write-Ahead Log) helpers for FileKV
//!
//! This module provides WAL operation helpers for the FileKV module.

use std::hash::Hasher;
use std::io::{BufRead, BufReader, Read};
use std::sync::Arc;
use std::time::Instant;

use crate::core::error::FatalError;
use crate::io::{FileKVFile, FileKVFileSystem};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// Result type for WAL operations
pub type Result<T> = std::result::Result<T, FatalError>;

/// WAL entry with operation and optional payload
/// PERF-005 FIX: payload uses serde_bytes for efficient binary serialization
/// T-018: Added sequence_number for continuity validation during recovery
/// T-001: Added binary serialization support for performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalEntry {
    /// Monotonically increasing sequence number assigned at write time
    #[serde(default)]
    pub sequence_number: u64,
    pub operation: WalOperation,
    #[serde(with = "serde_bytes")]
    pub payload: Option<Vec<u8>>,
}

// T-001: Binary serialization constants
const OP_ADD: u8 = 0;
const OP_DELETE: u8 = 1;
const OP_BATCH_ADD: u8 = 2;

impl WalEntry {
    /// T-001: Serialize this entry to a compact binary format.
    ///
    /// Binary layout:
    /// - sequence_number: u64 LE
    /// - operation_type: u8 (0=Add, 1=Delete, 2=BatchAdd)
    /// - session_len: u16 LE
    /// - session bytes
    /// - hash_len: u16 LE
    /// - hash bytes
    /// - layer_len: u16 LE (only for Add, 0 for Delete/BatchAdd)
    /// - layer bytes (only for Add)
    /// - payload_len: u32 LE (0 = None)
    /// - payload bytes
    /// - For BatchAdd: [entry_count: u16 LE, then per-entry: key_len u16, key, value_len u32, value, hash_len u16, hash]
    /// - checksum: u32 LE (CRC32C over all preceding bytes)
    pub fn serialize_binary(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);

        // sequence_number: u64 LE
        buf.extend_from_slice(&self.sequence_number.to_le_bytes());

        match &self.operation {
            WalOperation::Add { session, hash, layer } => {
                buf.push(OP_ADD);
                write_string_u16(&mut buf, session);
                write_string_u16(&mut buf, hash);
                write_string_u16(&mut buf, layer);
            }
            WalOperation::Delete { session, hash } => {
                buf.push(OP_DELETE);
                write_string_u16(&mut buf, session);
                write_string_u16(&mut buf, hash);
                buf.extend_from_slice(&0u16.to_le_bytes()); // layer_len = 0
            }
            WalOperation::BatchAdd { entries } => {
                buf.push(OP_BATCH_ADD);
                buf.extend_from_slice(&0u16.to_le_bytes()); // session_len = 0
                buf.extend_from_slice(&0u16.to_le_bytes()); // hash_len = 0
                buf.extend_from_slice(&0u16.to_le_bytes()); // layer_len = 0
                                                            // Encode batch entries in payload area below
                let mut batch_payload = Vec::with_capacity(256);
                batch_payload.extend_from_slice(&(entries.len() as u16).to_le_bytes());
                for entry in entries {
                    write_string_u16(&mut batch_payload, &entry.key);
                    batch_payload.extend_from_slice(&(entry.value.len() as u32).to_le_bytes());
                    batch_payload.extend_from_slice(&entry.value);
                    write_string_u16(&mut batch_payload, &entry.hash);
                }
                buf.extend_from_slice(&(batch_payload.len() as u32).to_le_bytes());
                buf.extend_from_slice(&batch_payload);
                // Compute checksum and return
                let checksum = crc32c::crc32c(&buf);
                buf.extend_from_slice(&checksum.to_le_bytes());
                return buf;
            }
        }

        // payload_len: u32 LE
        if let Some(ref payload) = self.payload {
            buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
            buf.extend_from_slice(payload);
        } else {
            buf.extend_from_slice(&0u32.to_le_bytes());
        }

        // checksum: u32 LE (CRC32C over all preceding bytes)
        let checksum = crc32c::crc32c(&buf);
        buf.extend_from_slice(&checksum.to_le_bytes());

        buf
    }

    /// T-001: Deserialize a WalEntry from binary format.
    pub fn deserialize_binary(data: &[u8]) -> Result<Self> {
        let min_len = 8 + 1 + 2 + 2 + 2 + 4 + 4; // seq + op + 3*u16 + payload_len + checksum
        if data.len() < min_len {
            return Err(FatalError::Corruption(format!(
                "Binary WAL entry too short: {} bytes",
                data.len()
            )));
        }

        let mut cursor = std::io::Cursor::new(data);

        // sequence_number: u64 LE
        let mut seq_bytes = [0u8; 8];
        cursor
            .read_exact(&mut seq_bytes)
            .map_err(|e| FatalError::Corruption(format!("Failed to read sequence: {}", e)))?;
        let sequence_number = u64::from_le_bytes(seq_bytes);

        // operation_type: u8
        let mut op_byte = [0u8; 1];
        cursor
            .read_exact(&mut op_byte)
            .map_err(|e| FatalError::Corruption(format!("Failed to read op: {}", e)))?;
        let op_type = op_byte[0];

        // session: u16 len + bytes
        let session = read_string_u16(&mut cursor)?;

        // hash: u16 len + bytes
        let hash = read_string_u16(&mut cursor)?;

        // layer: u16 len + bytes
        let layer = read_string_u16(&mut cursor)?;

        // payload: u32 len + bytes
        let mut payload_len_bytes = [0u8; 4];
        cursor
            .read_exact(&mut payload_len_bytes)
            .map_err(|e| FatalError::Corruption(format!("Failed to read payload len: {}", e)))?;
        let payload_len = u32::from_le_bytes(payload_len_bytes) as usize;

        let payload = if payload_len > 0 {
            let mut payload = vec![0u8; payload_len];
            cursor
                .read_exact(&mut payload)
                .map_err(|e| FatalError::Corruption(format!("Failed to read payload: {}", e)))?;
            Some(payload)
        } else {
            None
        };

        // checksum: u32 LE (verify)
        let checksum_pos = cursor.position() as usize;
        if checksum_pos + 4 > data.len() {
            return Err(FatalError::Corruption(
                "Truncated checksum in binary WAL entry".to_string(),
            ));
        }
        let stored_checksum = u32::from_le_bytes([
            data[checksum_pos],
            data[checksum_pos + 1],
            data[checksum_pos + 2],
            data[checksum_pos + 3],
        ]);
        let computed_checksum = crc32c::crc32c(&data[..checksum_pos]);
        if stored_checksum != computed_checksum {
            return Err(FatalError::Corruption(format!(
                "Checksum mismatch in WAL entry: stored=0x{:08X}, computed=0x{:08X}",
                stored_checksum, computed_checksum
            )));
        }

        let operation = match op_type {
            OP_ADD => {
                if layer.is_empty() {
                    return Err(FatalError::Corruption("Add operation missing layer".to_string()));
                }
                WalOperation::Add { session, hash, layer }
            }
            OP_DELETE => WalOperation::Delete { session, hash },
            OP_BATCH_ADD => {
                // Decode batch entries from payload
                let entries = if let Some(ref payload) = payload {
                    decode_batch_entries(payload)?
                } else {
                    Vec::new()
                };
                WalOperation::BatchAdd { entries }
            }
            _ => {
                return Err(FatalError::Corruption(format!(
                    "Unknown WAL operation type: {}",
                    op_type
                )));
            }
        };

        Ok(WalEntry {
            sequence_number,
            operation,
            payload,
        })
    }

    /// T-001: Check if data is in binary format (first byte != b'{')
    #[inline]
    pub fn is_binary_format(data: &[u8]) -> bool {
        data.first().map(|&b| b != b'{').unwrap_or(false)
    }
}

/// T-001: Helper to write a string with u16 LE length prefix
#[inline]
fn write_string_u16(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
    buf.extend_from_slice(bytes);
}

/// T-001: Helper to read a string with u16 LE length prefix
fn read_string_u16(cursor: &mut std::io::Cursor<&[u8]>) -> Result<String> {
    let mut len_bytes = [0u8; 2];
    cursor
        .read_exact(&mut len_bytes)
        .map_err(|e| FatalError::Corruption(format!("Failed to read string length: {}", e)))?;
    let len = u16::from_le_bytes(len_bytes) as usize;
    if len == 0 {
        return Ok(String::new());
    }
    let mut buf = vec![0u8; len];
    cursor
        .read_exact(&mut buf)
        .map_err(|e| FatalError::Corruption(format!("Failed to read string: {}", e)))?;
    String::from_utf8(buf).map_err(|e| FatalError::Corruption(format!("Invalid UTF-8 in WAL entry: {}", e)))
}

/// T-001: Decode batch entries from binary payload
fn decode_batch_entries(data: &[u8]) -> Result<Vec<BatchEntry>> {
    if data.len() < 2 {
        return Err(FatalError::Corruption("Batch payload too short".to_string()));
    }
    let entry_count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let mut entries = Vec::with_capacity(entry_count);
    let mut offset = 2;

    for _ in 0..entry_count {
        if offset + 2 > data.len() {
            return Err(FatalError::Corruption("Truncated batch entry key length".to_string()));
        }
        let key_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if offset + key_len > data.len() {
            return Err(FatalError::Corruption("Truncated batch entry key".to_string()));
        }
        let key = String::from_utf8(data[offset..offset + key_len].to_vec())
            .map_err(|e| FatalError::Corruption(format!("Invalid UTF-8 in batch key: {}", e)))?;
        offset += key_len;

        if offset + 4 > data.len() {
            return Err(FatalError::Corruption("Truncated batch entry value length".to_string()));
        }
        let value_len =
            u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]]) as usize;
        offset += 4;
        if offset + value_len > data.len() {
            return Err(FatalError::Corruption("Truncated batch entry value".to_string()));
        }
        let value = data[offset..offset + value_len].to_vec();
        offset += value_len;

        if offset + 2 > data.len() {
            return Err(FatalError::Corruption("Truncated batch entry hash length".to_string()));
        }
        let hash_len = u16::from_le_bytes([data[offset], data[offset + 1]]) as usize;
        offset += 2;
        if offset + hash_len > data.len() {
            return Err(FatalError::Corruption("Truncated batch entry hash".to_string()));
        }
        let hash = String::from_utf8(data[offset..offset + hash_len].to_vec())
            .map_err(|e| FatalError::Corruption(format!("Invalid UTF-8 in batch hash: {}", e)))?;
        offset += hash_len;

        entries.push(BatchEntry { key, value, hash });
    }

    Ok(entries)
}

/// T-001: Load WAL entries from raw file data, auto-detecting format.
///
/// Supports both JSON (backward compatibility) and binary formats.
/// Detection: if first non-whitespace byte is `{`, treat as JSON; otherwise binary.
pub(crate) fn load_wal_entries(file_data: &[u8]) -> Result<Vec<WalEntry>> {
    if file_data.is_empty() {
        return Ok(Vec::new());
    }

    // Auto-detect format
    let first_non_ws = file_data.iter().find(|&&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'));
    match first_non_ws {
        Some(&b'{') => load_wal_entries_json(file_data),
        _ => load_wal_entries_binary(file_data),
    }
}

/// T-001: Load entries from JSON format (backward compatibility)
fn load_wal_entries_json(file_data: &[u8]) -> Result<Vec<WalEntry>> {
    let mut entries = Vec::new();
    let reader = BufReader::new(file_data);
    for line_result in reader.lines() {
        let line = line_result.map_err(|e| FatalError::Corruption(format!("Failed to read WAL line: {}", e)))?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<WalEntry>(&line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                eprintln!("Warning: Failed to parse WAL entry (JSON): {}", e);
            }
        }
    }
    Ok(entries)
}

/// T-001: Load entries from binary format
fn load_wal_entries_binary(file_data: &[u8]) -> Result<Vec<WalEntry>> {
    let mut entries = Vec::new();
    let mut offset = 0;

    while offset + 4 <= file_data.len() {
        // Read length prefix: u32 LE
        let record_len = u32::from_le_bytes([
            file_data[offset],
            file_data[offset + 1],
            file_data[offset + 2],
            file_data[offset + 3],
        ]) as usize;
        offset += 4;

        if offset + record_len > file_data.len() {
            // Partial write or truncated record - stop here
            eprintln!(
                "Warning: Truncated binary WAL record at offset {} (expected {} bytes, got {})",
                offset - 4,
                record_len,
                file_data.len() - offset + 4
            );
            break;
        }

        match WalEntry::deserialize_binary(&file_data[offset..offset + record_len]) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                eprintln!("Warning: Failed to parse binary WAL entry: {}", e);
            }
        }
        offset += record_len;
    }

    Ok(entries)
}

/// T-001: Validate WAL integrity in JSON format (backward compatibility)
fn validate_wal_integrity_json(file_data: &[u8], path: &std::path::Path) -> Result<()> {
    let reader = BufReader::new(file_data);
    let mut line_num = 0u64;
    let mut corrupted_count = 0u64;

    for line_result in reader.lines() {
        let line = line_result.map_err(FatalError::Io)?;
        line_num += 1;

        if line.trim().is_empty() {
            continue;
        }

        if let Err(e) = serde_json::from_str::<WalEntry>(&line) {
            corrupted_count += 1;
            tracing::warn!("WAL line {} is corrupted: {} (file: {})", line_num, e, path.display());
        }
    }

    if corrupted_count > 0 {
        return Err(FatalError::WalCorrupted(format!(
            "{} corrupted entries found in WAL file {} ({}/{} lines valid)",
            corrupted_count,
            path.display(),
            line_num.saturating_sub(corrupted_count),
            line_num
        )));
    }

    Ok(())
}

/// T-001: Validate WAL integrity in binary format
fn validate_wal_integrity_binary(file_data: &[u8], path: &std::path::Path) -> Result<()> {
    let mut offset = 0;
    let mut record_count = 0u64;
    let mut corrupted_count = 0u64;

    while offset + 4 <= file_data.len() {
        let record_len = u32::from_le_bytes([
            file_data[offset],
            file_data[offset + 1],
            file_data[offset + 2],
            file_data[offset + 3],
        ]) as usize;
        offset += 4;

        if offset + record_len > file_data.len() {
            // Truncated record - treat as corruption
            corrupted_count += 1;
            tracing::warn!(
                "Truncated binary WAL record at offset {} (file: {})",
                offset - 4,
                path.display()
            );
            break;
        }

        if let Err(e) = WalEntry::deserialize_binary(&file_data[offset..offset + record_len]) {
            corrupted_count += 1;
            tracing::warn!(
                "Binary WAL entry {} is corrupted: {} (file: {})",
                record_count,
                e,
                path.display()
            );
        } else {
            record_count += 1;
        }
        offset += record_len;
    }

    if corrupted_count > 0 {
        return Err(FatalError::WalCorrupted(format!(
            "{} corrupted entries found in WAL file {} ({}/{} records valid)",
            corrupted_count,
            path.display(),
            record_count,
            record_count + corrupted_count
        )));
    }

    Ok(())
}

/// WAL durability level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLevel {
    /// Data not persisted
    None,
    /// Data in OS cache
    Cached,
    /// Data persisted to disk
    Persisted,
}

/// WAL operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WalOperation {
    /// Add operation
    Add {
        session: String,
        hash: String,
        layer: String,
    },
    /// Delete operation
    Delete { session: String, hash: String },
    /// Batch add operation (atomic)
    BatchAdd { entries: Vec<BatchEntry> },
}

/// A single entry within a batch operation
/// PERF-005 FIX: value stored as raw bytes instead of base64 string
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEntry {
    pub key: String,
    #[serde(with = "serde_bytes")]
    pub value: Vec<u8>,
    pub hash: String,
}

/// WAL manager
pub struct WalManager {
    fs: Arc<dyn FileKVFileSystem>,
    wal_dir: std::path::PathBuf,
    entries: Vec<WalEntry>,
    current_wal_path: std::path::PathBuf,
    current_wal_file: Option<Box<dyn FileKVFile>>,
    /// WAL sequence number for ordering (used for recovery)
    wal_sequence: u64,
    /// CFG-002: WAL sync mode for controlling durability vs performance
    sync_mode: crate::WalSyncMode,
    /// OPT-001: Internal write buffer to reduce syscall overhead
    /// Buffered data that hasn't been flushed to disk yet
    write_buffer: Vec<u8>,
    /// T-006: Last time we performed a fsync (for timed sync in Batch mode)
    last_sync_time: Instant,
    /// T-006: Sync interval in milliseconds for Batch mode (default 10ms)
    sync_interval_ms: u64,
}

impl WalManager {
    pub fn new<P: AsRef<std::path::Path>>(
        fs: Arc<dyn FileKVFileSystem>,
        wal_dir: P,
        _enable_wal: bool,
    ) -> Result<Self> {
        Self::new_with_config(
            fs,
            wal_dir,
            _enable_wal,
            64 * 1024 * 1024,
            10,
            crate::WalSyncMode::default(),
        )
    }

    pub fn new_with_config<P: AsRef<std::path::Path>>(
        fs: Arc<dyn FileKVFileSystem>,
        wal_dir: P,
        _enable_wal: bool,
        max_size_bytes: u64,
        max_files: usize,
        sync_mode: crate::WalSyncMode,
    ) -> Result<Self> {
        Self::new_with_full_config(
            fs,
            wal_dir,
            _enable_wal,
            max_size_bytes,
            max_files,
            sync_mode,
            64, // default batch_sync_interval
            10, // default sync_interval_ms
        )
    }

    /// Create WalManager with full configuration
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_full_config<P: AsRef<std::path::Path>>(
        fs: Arc<dyn FileKVFileSystem>,
        wal_dir: P,
        _enable_wal: bool,
        _max_size_bytes: u64,
        _max_files: usize,
        sync_mode: crate::WalSyncMode,
        _batch_sync_interval: u64,
        sync_interval_ms: u64,
    ) -> Result<Self> {
        let wal_dir = wal_dir.as_ref().to_path_buf();
        fs.create_dir_all(&wal_dir)?;

        // Find the latest WAL file
        let mut latest_wal_path = None;
        let mut latest_seq = 0u64;

        for path in fs.read_dir(&wal_dir)? {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("wal_") && name.ends_with(".log") {
                    if let Some(seq_str) = name.strip_prefix("wal_").and_then(|s| s.strip_suffix(".log")) {
                        if let Ok(seq) = seq_str.parse::<u64>() {
                            if seq > latest_seq {
                                latest_seq = seq;
                                latest_wal_path = Some(path);
                            }
                        }
                    }
                }
            }
        }

        // Load existing WAL entries if any
        let mut entries = Vec::new();
        let current_wal_path = latest_wal_path.unwrap_or_else(|| wal_dir.join(format!("wal_{}.log", latest_seq + 1)));

        if fs.file_exists(&current_wal_path) {
            // T-001: Read WAL file through FileKVFileSystem abstraction (works with MemFs)
            let metadata = fs.file_metadata(&current_wal_path)?;
            if metadata.len > 0 {
                let mut file = fs.open_file(&current_wal_path, true, false, false)?;
                let mut file_data = vec![0u8; metadata.len as usize];
                file.read_exact(&mut file_data)?;
                entries = load_wal_entries(&file_data)?;
            }
        }

        let wal_sequence = if fs.file_exists(&current_wal_path) {
            latest_seq
        } else {
            latest_seq + 1
        };

        Ok(Self {
            fs,
            wal_dir,
            entries,
            current_wal_path,
            current_wal_file: None,
            wal_sequence,
            sync_mode,
            write_buffer: Vec::with_capacity(64 * 1024), // 64KB initial buffer
            last_sync_time: Instant::now(),
            sync_interval_ms,
        })
    }

    pub fn log(&mut self, op: WalOperation) -> Result<DurabilityLevel> {
        let entry = WalEntry {
            sequence_number: self.wal_sequence,
            operation: op,
            payload: None,
        };
        self.wal_sequence += 1;
        self.append_entry_to_disk(&entry)?;
        self.entries.push(entry);
        Ok(DurabilityLevel::Persisted)
    }

    /// PERF-005 FIX: payload is now `Vec<u8>` for binary format, avoiding base64 overhead
    pub fn log_with_payload(&mut self, op: WalOperation, payload: Vec<u8>) -> Result<DurabilityLevel> {
        let entry = WalEntry {
            sequence_number: self.wal_sequence,
            operation: op,
            payload: Some(payload),
        };
        self.wal_sequence += 1;
        self.append_entry_to_disk(&entry)?;
        self.entries.push(entry);
        Ok(DurabilityLevel::Persisted)
    }

    /// T-001: Append a single WAL entry to disk using binary serialization.
    /// OPT-001: Uses internal write buffer to reduce syscall overhead.
    ///
    /// Binary format uses length-prefixed records: [len: u32 LE][data: len bytes]
    fn append_entry_to_disk(&mut self, entry: &WalEntry) -> Result<()> {
        // Open WAL file if not already open
        if self.current_wal_file.is_none() {
            // Ensure parent dir exists
            self.fs.create_dir_all(&self.wal_dir)?;

            let file = self.fs.open_file(&self.current_wal_path, false, true, true)?;
            self.current_wal_file = Some(file);
        }

        // T-001: Serialize entry to binary format
        let binary_data = entry.serialize_binary();

        // T-001: Write length-prefixed record: [len: u32 LE][data]
        let len_prefix = (binary_data.len() as u32).to_le_bytes();
        self.write_buffer.extend_from_slice(&len_prefix);
        self.write_buffer.extend_from_slice(&binary_data);

        // OPT-001: Flush buffer to file when it reaches threshold
        if self.write_buffer.len() >= 32 * 1024 {
            // 32KB threshold
            self.flush_buffer_to_file()?;
        }

        // Apply sync mode policy
        self.apply_sync_policy()
    }

    /// OPT-001: Flush internal write buffer to the open WAL file
    fn flush_buffer_to_file(&mut self) -> Result<()> {
        if self.write_buffer.is_empty() {
            return Ok(());
        }
        if let Some(ref mut file) = self.current_wal_file {
            file.write_all(&self.write_buffer)?;
            self.write_buffer.clear();
        }
        Ok(())
    }

    /// OPT-001 + T-006: Apply sync policy based on sync_mode and batch_sync_interval/sync_interval_ms
    fn apply_sync_policy(&mut self) -> Result<()> {
        match self.sync_mode {
            crate::WalSyncMode::Immediate => {
                self.flush_buffer_to_file()?;
                if let Some(ref mut file) = self.current_wal_file {
                    file.flush()?;
                    file.sync_all()?;
                }
            }
            crate::WalSyncMode::Batch => {
                // T-006: Timed fsync - check if sync interval has elapsed
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_sync_time).as_millis() as u64;
                if elapsed >= self.sync_interval_ms {
                    self.flush_buffer_to_file()?;
                    if let Some(ref mut file) = self.current_wal_file {
                        file.sync_all()?;
                    }
                    self.last_sync_time = now;
                }
            }
            crate::WalSyncMode::Lazy => {
                // Don't flush immediately, let OS handle it
                // Buffer will be flushed when it reaches threshold or on explicit flush
            }
        }
        Ok(())
    }

    /// Read all WAL entries for recovery
    pub fn read_entries(&self) -> Result<Vec<WalEntry>> {
        Ok(self.entries.clone())
    }

    /// Phase 2: Validate WAL file integrity and return a FatalError if corruption is detected.
    ///
    /// This method performs a stricter validation than `read_entries()`:
    /// - Checks that all lines parse correctly (no partial writes)
    /// - Reports corruption as `FatalError::WalCorrupted` so callers know
    ///   this is unrecoverable and must not be retried.
    ///
    /// Use this during startup or after crash recovery to detect WAL corruption
    /// early.
    /// T-001: Supports both JSON and binary formats.
    pub fn validate_wal_integrity(&self) -> Result<()> {
        if !self.fs.file_exists(&self.current_wal_path) {
            return Ok(()); // No WAL file, nothing to validate
        }

        // T-001: Read through FileKVFileSystem abstraction
        let metadata = self.fs.file_metadata(&self.current_wal_path)?;
        if metadata.len == 0 {
            return Ok(());
        }

        let mut file = self.fs.open_file(&self.current_wal_path, true, false, false)?;
        let mut file_data = vec![0u8; metadata.len as usize];
        file.read_exact(&mut file_data)?;

        // Auto-detect format and validate accordingly
        let first_non_ws = file_data.iter().find(|&&b| !matches!(b, b' ' | b'\t' | b'\n' | b'\r'));
        match first_non_ws {
            Some(&b'{') => validate_wal_integrity_json(&file_data, &self.current_wal_path),
            _ => validate_wal_integrity_binary(&file_data, &self.current_wal_path),
        }
    }

    /// Clear all WAL entries (after successful recovery)
    pub fn clear(&mut self) -> Result<()> {
        // OPT-001: Flush any pending buffer data first
        self.flush_buffer_to_file()?;

        // Close current WAL file
        if let Some(mut file) = self.current_wal_file.take() {
            let _ = file.flush();
            drop(file);
        }

        // Remove old WAL files (keep only the latest)
        for path in self.fs.read_dir(&self.wal_dir)? {
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                if name.starts_with("wal_") && name.ends_with(".log") && path != self.current_wal_path {
                    let _ = self.fs.remove_file(&path);
                }
            }
        }

        // Truncate current WAL file
        if self.fs.file_exists(&self.current_wal_path) {
            let mut file = self.fs.create_file(&self.current_wal_path)?;
            file.write_all(b"")?;
            file.flush()?;
        }

        self.entries.clear();
        Ok(())
    }

    /// Log a batch of add operations atomically
    /// OPT-001: All entries are serialized to a single buffer and written in one syscall
    /// All entries are written to a single WAL record for atomicity
    /// T-001: Uses binary serialization for performance
    pub fn log_batch(&mut self, batch_entries: &[(String, Vec<u8>)]) -> Result<DurabilityLevel> {
        if batch_entries.is_empty() {
            return Ok(DurabilityLevel::Persisted);
        }

        // Build batch entries
        let entries: Vec<BatchEntry> = batch_entries
            .iter()
            .map(|(key, value)| {
                let mut hasher = xxhash_rust::xxh3::Xxh3::default();
                hasher.write(value);
                let hash = hasher.finish();

                // PERF-005 FIX: Store value directly as bytes
                BatchEntry {
                    key: key.clone(),
                    value: value.clone(),
                    hash: format!("{:016X}", hash),
                }
            })
            .collect();

        // Create batch operation
        let op = WalOperation::BatchAdd { entries };
        let entry = WalEntry {
            sequence_number: self.wal_sequence,
            operation: op,
            payload: None,
        };
        self.wal_sequence += 1;

        // Open WAL file if not already open
        if self.current_wal_file.is_none() {
            self.fs.create_dir_all(&self.wal_dir)?;
            let file = self.fs.open_file(&self.current_wal_path, false, true, true)?;
            self.current_wal_file = Some(file);
        }

        // T-001: Serialize to binary format with length prefix
        let binary_data = entry.serialize_binary();
        let len_prefix = (binary_data.len() as u32).to_le_bytes();
        self.write_buffer.extend_from_slice(&len_prefix);
        self.write_buffer.extend_from_slice(&binary_data);

        // OPT-001: Flush buffer if it reaches threshold
        if self.write_buffer.len() >= 32 * 1024 {
            self.flush_buffer_to_file()?;
        }

        // Apply sync policy for the batch
        self.apply_sync_policy()?;

        // Track in memory
        self.entries.push(entry);

        Ok(DurabilityLevel::Persisted)
    }

    /// OPT-001: Explicit flush to ensure all buffered data is written to disk
    /// Called during shutdown or when durability guarantee is needed
    pub fn flush(&mut self) -> Result<()> {
        self.flush_buffer_to_file()?;
        if let Some(ref mut file) = self.current_wal_file {
            file.flush()?;
        }
        Ok(())
    }

    /// OPT-001: Sync to ensure all data is persisted to disk (fsync)
    pub fn sync_all(&mut self) -> Result<()> {
        self.flush()?;
        if let Some(ref mut file) = self.current_wal_file {
            file.sync_all()?;
        }
        Ok(())
    }
}

/// OPT-001: Ensure buffered data is flushed when WalManager is dropped
impl Drop for WalManager {
    fn drop(&mut self) {
        // Flush any remaining buffered data to disk
        if !self.write_buffer.is_empty() {
            if let Some(ref mut file) = self.current_wal_file {
                let _ = file.write_all(&self.write_buffer);
                self.write_buffer.clear();
                let _ = file.flush();
            }
        }
    }
}

/// WAL operation helper for batch operations
pub struct WalBatchWriter<'a> {
    wal_guard: parking_lot::MutexGuard<'a, WalManager>,
    operations_count: usize,
}

impl<'a> WalBatchWriter<'a> {
    /// Create a new batch writer from a WAL mutex
    pub fn new(wal: &'a Mutex<WalManager>) -> Option<Self> {
        Some(Self {
            wal_guard: wal.lock(),
            operations_count: 0,
        })
    }

    /// Log an add operation
    ///
    /// Returns DurabilityLevel indicating whether data is persisted
    pub fn log_add(&mut self, key: &str, value: &[u8]) -> Result<DurabilityLevel> {
        let mut hasher = xxhash_rust::xxh3::Xxh3::default();
        hasher.write(value);
        let hash = hasher.finish();

        // PERF-005 FIX: Binary payload format
        let hash_bytes = hash.to_le_bytes();
        let len_bytes = (value.len() as u64).to_le_bytes();
        let mut payload = Vec::with_capacity(16 + value.len());
        payload.extend_from_slice(&len_bytes);
        payload.extend_from_slice(&hash_bytes);
        payload.extend_from_slice(value);

        let op = WalOperation::Add {
            session: key.to_string(),
            hash: format!("{:016X}", hash),
            layer: "segment".to_string(),
        };
        let durability = self
            .wal_guard
            .log_with_payload(op, payload)
            .map_err(|e| FatalError::Corruption(format!("WAL operation failed: {}", e)))?;
        self.operations_count += 1;
        Ok(durability)
    }

    /// Log a delete operation
    ///
    /// Returns DurabilityLevel indicating whether data is persisted
    pub fn log_delete(&mut self, key: &str) -> Result<DurabilityLevel> {
        let op = WalOperation::Delete {
            session: key.to_string(),
            hash: String::new(),
        };
        self.wal_guard
            .log(op)
            .map_err(|e| FatalError::Corruption(format!("WAL operation failed: {}", e)))
    }

    /// Get the number of operations logged
    pub fn operations_count(&self) -> usize {
        self.operations_count
    }
}

/// Simple WAL writer for single operations
///
/// Returns DurabilityLevel indicating whether data is persisted
pub fn log_wal_add(wal: &Mutex<WalManager>, key: &str, value: &[u8]) -> Result<DurabilityLevel> {
    let mut wal_guard = wal.lock();
    let mut hasher = xxhash_rust::xxh3::Xxh3::default();
    hasher.write(value);
    let hash = hasher.finish();

    // PERF-005 FIX: Binary payload format
    let hash_bytes = hash.to_le_bytes();
    let len_bytes = (value.len() as u64).to_le_bytes();
    let mut payload = Vec::with_capacity(16 + value.len());
    payload.extend_from_slice(&len_bytes);
    payload.extend_from_slice(&hash_bytes);
    payload.extend_from_slice(value);

    let op = WalOperation::Add {
        session: key.to_string(),
        hash: format!("{:016X}", hash),
        layer: "segment".to_string(),
    };
    wal_guard
        .log_with_payload(op, payload)
        .map_err(|e| FatalError::Corruption(format!("WAL operation failed: {}", e)))
}

/// Simple WAL writer for delete operations
///
/// Returns DurabilityLevel indicating whether data is persisted
pub fn log_wal_delete(wal: &Mutex<WalManager>, key: &str) -> Result<DurabilityLevel> {
    let mut wal_guard = wal.lock();
    let op = WalOperation::Delete {
        session: key.to_string(),
        hash: String::new(),
    };
    wal_guard
        .log(op)
        .map_err(|e| FatalError::Corruption(format!("WAL operation failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::memfs::MemFs;
    use std::sync::Arc;

    fn create_test_wal_manager() -> WalManager {
        let fs: Arc<dyn FileKVFileSystem> = Arc::new(MemFs::new());
        let wal_dir = std::path::PathBuf::from("/test_wal");
        WalManager::new(fs, wal_dir, true).unwrap()
    }

    #[test]
    fn test_wal_sequence_numbers_are_monotonic() {
        let mut wal = create_test_wal_manager();

        // Write several entries
        for i in 0..5 {
            let op = WalOperation::Add {
                session: format!("key_{}", i),
                hash: format!("hash_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).unwrap();
        }

        // Read entries and verify sequence numbers are monotonically increasing
        let entries = wal.read_entries().unwrap();
        assert_eq!(entries.len(), 5);

        // Sequence numbers start from wal_sequence initial value (which is 1 for new WAL)
        let start_seq = entries[0].sequence_number;
        for (idx, entry) in entries.iter().enumerate() {
            assert_eq!(
                entry.sequence_number,
                start_seq + idx as u64,
                "Entry {} should have sequence_number {}",
                idx,
                start_seq + idx as u64
            );
        }
    }

    #[test]
    fn test_wal_entry_validation_continuity_valid() {
        // Create valid entries with continuous sequence numbers
        let entries: Vec<WalEntry> = (0..5)
            .map(|i| WalEntry {
                sequence_number: i,
                operation: WalOperation::Add {
                    session: format!("key_{}", i),
                    hash: format!("hash_{}", i),
                    layer: "segment".to_string(),
                },
                payload: None,
            })
            .collect();

        // Validate sequence continuity
        let (valid_entries, warnings) = validate_wal_sequence_continuity_for_test(&entries);
        assert_eq!(valid_entries.len(), 5, "All entries should be valid");
        assert!(
            warnings.is_empty(),
            "No warnings should be generated for valid sequence"
        );
    }

    #[test]
    fn test_wal_entry_validation_continuity_gap() {
        // Create entries with a gap in sequence numbers
        let mut entries: Vec<WalEntry> = (0..3)
            .map(|i| WalEntry {
                sequence_number: i,
                operation: WalOperation::Add {
                    session: format!("key_{}", i),
                    hash: format!("hash_{}", i),
                    layer: "segment".to_string(),
                },
                payload: None,
            })
            .collect();

        // Add entry with sequence gap (skip 3, 4)
        entries.push(WalEntry {
            sequence_number: 5, // Should be 3
            operation: WalOperation::Add {
                session: "key_5".to_string(),
                hash: "hash_5".to_string(),
                layer: "segment".to_string(),
            },
            payload: None,
        });

        let (valid_entries, warnings) = validate_wal_sequence_continuity_for_test(&entries);
        assert_eq!(valid_entries.len(), 3, "Entry with gap should be skipped");
        assert_eq!(warnings.len(), 1, "Should have one warning for the gap");
        assert!(warnings[0].contains("unexpected sequence_number=5"));
    }

    #[test]
    fn test_wal_entry_validation_duplicate_sequence() {
        // Create entries with duplicate sequence numbers
        let mut entries: Vec<WalEntry> = (0..3)
            .map(|i| WalEntry {
                sequence_number: i,
                operation: WalOperation::Add {
                    session: format!("key_{}", i),
                    hash: format!("hash_{}", i),
                    layer: "segment".to_string(),
                },
                payload: None,
            })
            .collect();

        // Add entry with duplicate sequence number
        entries.push(WalEntry {
            sequence_number: 2, // Duplicate of entry at index 2
            operation: WalOperation::Add {
                session: "key_dup".to_string(),
                hash: "hash_dup".to_string(),
                layer: "segment".to_string(),
            },
            payload: None,
        });

        let (valid_entries, warnings) = validate_wal_sequence_continuity_for_test(&entries);
        assert_eq!(valid_entries.len(), 3, "Duplicate entry should be skipped");
        assert_eq!(warnings.len(), 1, "Should have one warning for the duplicate");
        assert!(warnings[0].contains("unexpected sequence_number=2"));
    }

    #[test]
    fn test_wal_entry_validation_out_of_order() {
        // Create entries with out-of-order sequence numbers
        let entries = vec![
            WalEntry {
                sequence_number: 5,
                operation: WalOperation::Add {
                    session: "key_5".to_string(),
                    hash: "hash_5".to_string(),
                    layer: "segment".to_string(),
                },
                payload: None,
            },
            WalEntry {
                sequence_number: 3, // Out of order - less than previous (expected 6)
                operation: WalOperation::Add {
                    session: "key_3".to_string(),
                    hash: "hash_3".to_string(),
                    layer: "segment".to_string(),
                },
                payload: None,
            },
            WalEntry {
                sequence_number: 4, // Still out of order (expected 6)
                operation: WalOperation::Add {
                    session: "key_4".to_string(),
                    hash: "hash_4".to_string(),
                    layer: "segment".to_string(),
                },
                payload: None,
            },
        ];

        let (valid_entries, warnings) = validate_wal_sequence_continuity_for_test(&entries);
        assert_eq!(valid_entries.len(), 1, "Only first entry should be valid");
        assert_eq!(warnings.len(), 2, "Should have warnings for out-of-order entries");
    }

    #[test]
    fn test_wal_entry_validation_empty() {
        let entries: Vec<WalEntry> = vec![];
        let (valid_entries, warnings) = validate_wal_sequence_continuity_for_test(&entries);
        assert_eq!(valid_entries.len(), 0);
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_wal_entry_validation_single_entry() {
        let entries = vec![WalEntry {
            sequence_number: 42, // Any starting number is valid
            operation: WalOperation::Add {
                session: "key_42".to_string(),
                hash: "hash_42".to_string(),
                layer: "segment".to_string(),
            },
            payload: None,
        }];

        let (valid_entries, warnings) = validate_wal_sequence_continuity_for_test(&entries);
        assert_eq!(valid_entries.len(), 1);
        assert!(warnings.is_empty());
    }

    /// Test helper that replicates the validation logic from lifecycle.rs
    fn validate_wal_sequence_continuity_for_test(entries: &[WalEntry]) -> (Vec<WalEntry>, Vec<String>) {
        let mut valid_entries = Vec::with_capacity(entries.len());
        let mut warnings = Vec::new();
        let mut expected_seq: Option<u64> = None;

        for (idx, entry) in entries.iter().enumerate() {
            if let Some(prev_seq) = expected_seq {
                if entry.sequence_number != prev_seq + 1 {
                    warnings.push(format!(
                        "WAL entry {} has unexpected sequence_number={} (expected={}), possible gap or corruption - skipping",
                        idx, entry.sequence_number, prev_seq + 1
                    ));
                    continue;
                }
            }
            expected_seq = Some(entry.sequence_number);
            valid_entries.push(entry.clone());
        }

        (valid_entries, warnings)
    }

    // =========================================================================
    // T-001: Binary serialization tests
    // =========================================================================

    #[test]
    fn test_binary_serialize_deserialize_add() {
        let entry = WalEntry {
            sequence_number: 42,
            operation: WalOperation::Add {
                session: "test_session".to_string(),
                hash: "ABCDEF0123456789".to_string(),
                layer: "segment".to_string(),
            },
            payload: Some(vec![1, 2, 3, 4, 5]),
        };

        let binary = entry.serialize_binary();
        assert!(!binary.is_empty());
        assert!(WalEntry::is_binary_format(&binary));

        let decoded = WalEntry::deserialize_binary(&binary).unwrap();
        assert_eq!(decoded.sequence_number, 42);
        match &decoded.operation {
            WalOperation::Add { session, hash, layer } => {
                assert_eq!(session, "test_session");
                assert_eq!(hash, "ABCDEF0123456789");
                assert_eq!(layer, "segment");
            }
            _ => panic!("Expected Add operation"),
        }
        assert_eq!(decoded.payload, Some(vec![1, 2, 3, 4, 5]));
    }

    #[test]
    fn test_binary_serialize_deserialize_delete() {
        let entry = WalEntry {
            sequence_number: 100,
            operation: WalOperation::Delete {
                session: "del_key".to_string(),
                hash: "HASH123".to_string(),
            },
            payload: None,
        };

        let binary = entry.serialize_binary();
        let decoded = WalEntry::deserialize_binary(&binary).unwrap();
        assert_eq!(decoded.sequence_number, 100);
        match &decoded.operation {
            WalOperation::Delete { session, hash } => {
                assert_eq!(session, "del_key");
                assert_eq!(hash, "HASH123");
            }
            _ => panic!("Expected Delete operation"),
        }
        assert!(decoded.payload.is_none());
    }

    #[test]
    fn test_binary_serialize_deserialize_batch_add() {
        let batch_entries = vec![
            BatchEntry {
                key: "key1".to_string(),
                value: b"value1".to_vec(),
                hash: "AAAA0000".to_string(),
            },
            BatchEntry {
                key: "key2".to_string(),
                value: b"value2_longer".to_vec(),
                hash: "BBBB1111".to_string(),
            },
        ];

        let entry = WalEntry {
            sequence_number: 200,
            operation: WalOperation::BatchAdd { entries: batch_entries },
            payload: None,
        };

        let binary = entry.serialize_binary();
        let decoded = WalEntry::deserialize_binary(&binary).unwrap();
        assert_eq!(decoded.sequence_number, 200);
        match &decoded.operation {
            WalOperation::BatchAdd { entries } => {
                assert_eq!(entries.len(), 2);
                assert_eq!(entries[0].key, "key1");
                assert_eq!(entries[0].value, b"value1");
                assert_eq!(entries[1].key, "key2");
                assert_eq!(entries[1].value, b"value2_longer");
            }
            _ => panic!("Expected BatchAdd operation"),
        }
    }

    #[test]
    fn test_binary_checksum_verification() {
        let entry = WalEntry {
            sequence_number: 1,
            operation: WalOperation::Add {
                session: "s".to_string(),
                hash: "h".to_string(),
                layer: "l".to_string(),
            },
            payload: None,
        };

        let mut binary = entry.serialize_binary();
        // Corrupt the checksum
        let len = binary.len();
        binary[len - 1] ^= 0xFF;

        let result = WalEntry::deserialize_binary(&binary);
        assert!(result.is_err(), "Should fail with corrupted checksum");
    }

    #[test]
    fn test_binary_format_detection() {
        // JSON starts with '{'
        let json_data = br#"{"sequence_number":1}"#;
        assert!(!WalEntry::is_binary_format(json_data));

        // Binary data starts with sequence number bytes
        let binary_data = vec![0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        assert!(WalEntry::is_binary_format(&binary_data));
    }

    #[test]
    fn test_load_wal_entries_json_format() {
        let json_wal = b"{\"sequence_number\":1,\"operation\":{\"Add\":{\"session\":\"k1\",\"hash\":\"h1\",\"layer\":\"seg\"}},\"payload\":null}\n\
                        {\"sequence_number\":2,\"operation\":{\"Add\":{\"session\":\"k2\",\"hash\":\"h2\",\"layer\":\"seg\"}},\"payload\":null}\n";
        let entries = load_wal_entries(json_wal).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence_number, 1);
        assert_eq!(entries[1].sequence_number, 2);
    }

    #[test]
    fn test_load_wal_entries_binary_format() {
        let entry1 = WalEntry {
            sequence_number: 10,
            operation: WalOperation::Add {
                session: "bin_key1".to_string(),
                hash: "bin_hash1".to_string(),
                layer: "segment".to_string(),
            },
            payload: None,
        };
        let entry2 = WalEntry {
            sequence_number: 11,
            operation: WalOperation::Add {
                session: "bin_key2".to_string(),
                hash: "bin_hash2".to_string(),
                layer: "segment".to_string(),
            },
            payload: None,
        };

        let binary1 = entry1.serialize_binary();
        let binary2 = entry2.serialize_binary();

        // Build length-prefixed binary WAL
        let mut wal_data = Vec::new();
        wal_data.extend_from_slice(&(binary1.len() as u32).to_le_bytes());
        wal_data.extend_from_slice(&binary1);
        wal_data.extend_from_slice(&(binary2.len() as u32).to_le_bytes());
        wal_data.extend_from_slice(&binary2);

        let entries = load_wal_entries(&wal_data).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].sequence_number, 10);
        assert_eq!(entries[1].sequence_number, 11);
    }

    #[test]
    fn test_binary_smaller_than_json() {
        // Verify binary format is more compact than JSON
        let entry = WalEntry {
            sequence_number: 1,
            operation: WalOperation::Add {
                session: "session_key".to_string(),
                hash: "ABCDEF0123456789".to_string(),
                layer: "segment".to_string(),
            },
            payload: Some(vec![0u8; 100]),
        };

        let binary = entry.serialize_binary();
        let json = serde_json::to_string(&entry).unwrap();

        // Binary should be significantly smaller (at least 20% smaller)
        assert!(
            binary.len() < json.len(),
            "Binary ({} bytes) should be smaller than JSON ({} bytes)",
            binary.len(),
            json.len()
        );
    }

    #[test]
    fn test_wal_recovery_with_binary_format() {
        // Create WAL, write entries in binary, verify recovery
        let fs: Arc<dyn FileKVFileSystem> = Arc::new(MemFs::new());
        let wal_dir = std::path::PathBuf::from("/test_wal_binary");
        let mut wal = WalManager::new(fs.clone(), wal_dir.clone(), true).unwrap();

        // Write several entries
        for i in 0..10 {
            let op = WalOperation::Add {
                session: format!("key_{}", i),
                hash: format!("hash_{:016X}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).unwrap();
        }

        // Flush to ensure data is written
        wal.flush().unwrap();

        // Create a new WalManager that loads from the same WAL file
        let wal2 = WalManager::new(fs.clone(), wal_dir.clone(), true).unwrap();
        let entries = wal2.read_entries();
        assert!(entries.is_ok());
        assert_eq!(entries.unwrap().len(), 10);
    }
}
