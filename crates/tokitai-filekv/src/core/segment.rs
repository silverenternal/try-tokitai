//! Segment 文件模块
//!
//! 顺序写入的数据段文件，格式：
//! ┌─────────────────────────────────────┐
//! │ Entry 1                             │
//! │ ├─ Key Length (u32)                 │
//! │ ├─ Key Bytes                        │
//! │ ├─ Value Length (u32)               │
//! │ ├─ Value Bytes                      │
//! │ ├─ Checksum (u32, CRC32C)           │
//! ├─────────────────────────────────────┤
//! │ Entry 2                             │
//! │ ...                                 │
//! └─────────────────────────────────────┘

use arc_swap::ArcSwapOption;
use parking_lot::Mutex;
use std::hash::Hasher;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc; // RES-001: Lock-free Arc<Mmap> management

use crate::core::error::FatalError;
use crate::io::{FileKVFile, FileKVFileSystem, MmapView};

pub const SEGMENT_MAGIC: u32 = 0x54435347; // "TCSG" = Tokitai Context SeGment
pub const SEGMENT_VERSION: u32 = 1;

// ============================================================
// OPT-009: Segment V2 Format - Block-level metadata
// ============================================================

/// OPT-009 Block header format for V2 segments
///
/// Each block in V2 format has a header containing key range metadata:
/// ┌──────────────────────────────────────────────────┐
/// │ OPT-009 Block Header (variable size)             │
/// │ ├─ magic: u32 (0x424C4B48 = "BLKH")              │
/// │ ├─ min_key_len: u16                              │
/// │ ├─ min_key: bytes                                │
/// │ ├─ max_key_len: u16                              │
/// │ ├─ max_key: bytes                                │
/// │ ├─ entry_count: u16                              │
/// │ ├─ block_offset: u64                             │
/// │ ├─ bloom_size: u32 (0 if bloom disabled)         │
/// │ ├─ bloom_filter: bytes (variable, if bloom_size>0)│
/// ├──────────────────────────────────────────────────┤
/// │ Existing BlockHeader (compression, if enabled)   │
/// ├──────────────────────────────────────────────────┤
/// │ Block Data (entries)                             │
/// └──────────────────────────────────────────────────┘
pub const OPT009_BLOCK_HEADER_MAGIC: u32 = 0x4F505432; // "OPT2" = OPT-009 V2

/// OPT-009 Tail index format for V2 segments
///
/// Appended at end of segment file after all blocks:
/// ┌─────────────────────────────────────────┐
/// │ Tail Index                              │
/// │ ├─ magic: u32 (0x494E4458 = "INDX")    │
/// │ ├─ sparse_index_entries: Vec<u8>        │
/// │ ├─ zone_map_entries: Vec<u8>            │
/// │ ├─ checksum: u32 (CRC32C)               │
/// └─────────────────────────────────────────┘
pub const OPT009_TAIL_INDEX_MAGIC: u32 = 0x494E4458; // "INDX" = INDeX

/// OPT-009: Block-level metadata header for V2 segments
///
/// This header is placed before each block's entries (and before the compression BlockHeader if compression is enabled).
/// It provides block-level key range filtering and bloom filter support.
#[derive(Debug, Clone)]
pub struct Opt009BlockHeader {
    pub min_key: String,
    pub max_key: String,
    pub entry_count: u16,
    pub block_offset: u64,
    pub bloom_filter: Option<Vec<u8>>, // Serialized CustomBloom bits
}

impl Opt009BlockHeader {
    /// Serialize to bytes
    /// Format: [magic:u32][min_key_len:u16][min_key][max_key_len:u16][max_key][entry_count:u16][block_offset:u64][bloom_size:u32][bloom_bytes]
    pub fn to_bytes(&self) -> Vec<u8> {
        let min_key_bytes = self.min_key.as_bytes();
        let max_key_bytes = self.max_key.as_bytes();
        let bloom_size = self.bloom_filter.as_ref().map(|b| b.len() as u32).unwrap_or(0);

        let total_size = 4 + 2 + min_key_bytes.len() + 2 + max_key_bytes.len() + 2 + 8 + 4 + bloom_size as usize;
        let mut buf = Vec::with_capacity(total_size);

        buf.extend_from_slice(&OPT009_BLOCK_HEADER_MAGIC.to_le_bytes());
        buf.extend_from_slice(&(min_key_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(min_key_bytes);
        buf.extend_from_slice(&(max_key_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(max_key_bytes);
        buf.extend_from_slice(&self.entry_count.to_le_bytes());
        buf.extend_from_slice(&self.block_offset.to_le_bytes());
        buf.extend_from_slice(&bloom_size.to_le_bytes());
        if let Some(ref bloom) = self.bloom_filter {
            buf.extend_from_slice(bloom);
        }

        buf
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8], offset: &mut usize) -> Result<Self, FatalError> {
        let total_len = data.len();

        // Read magic
        if *offset + 4 > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for magic".to_string(),
            ));
        }
        let magic = u32::from_le_bytes(
            data[*offset..*offset + 4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 magic bytes: {}", e)))?,
        );
        *offset += 4;

        if magic != OPT009_BLOCK_HEADER_MAGIC {
            return Err(FatalError::Corruption(format!(
                "Invalid OPT-009 block header magic: expected {:08X}, got {:08X}",
                OPT009_BLOCK_HEADER_MAGIC, magic
            )));
        }

        // Read min_key
        if *offset + 2 > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for min_key_len".to_string(),
            ));
        }
        let min_key_len = u16::from_le_bytes(
            data[*offset..*offset + 2]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 min_key_len bytes: {}", e)))?,
        ) as usize;
        *offset += 2;

        if *offset + min_key_len > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for min_key".to_string(),
            ));
        }
        let min_key = String::from_utf8(data[*offset..*offset + min_key_len].to_vec())
            .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 min_key UTF-8: {}", e)))?;
        *offset += min_key_len;

        // Read max_key
        if *offset + 2 > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for max_key_len".to_string(),
            ));
        }
        let max_key_len = u16::from_le_bytes(
            data[*offset..*offset + 2]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 max_key_len bytes: {}", e)))?,
        ) as usize;
        *offset += 2;

        if *offset + max_key_len > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for max_key".to_string(),
            ));
        }
        let max_key = String::from_utf8(data[*offset..*offset + max_key_len].to_vec())
            .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 max_key UTF-8: {}", e)))?;
        *offset += max_key_len;

        // Read entry_count
        if *offset + 2 > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for entry_count".to_string(),
            ));
        }
        let entry_count = u16::from_le_bytes(
            data[*offset..*offset + 2]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 entry_count bytes: {}", e)))?,
        );
        *offset += 2;

        // Read block_offset
        if *offset + 8 > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for block_offset".to_string(),
            ));
        }
        let block_offset = u64::from_le_bytes(
            data[*offset..*offset + 8]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 block_offset bytes: {}", e)))?,
        );
        *offset += 8;

        // Read bloom_size
        if *offset + 4 > total_len {
            return Err(FatalError::Corruption(
                "Invalid OPT-009 block header: not enough data for bloom_size".to_string(),
            ));
        }
        let bloom_size = u32::from_le_bytes(
            data[*offset..*offset + 4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid OPT-009 bloom_size bytes: {}", e)))?,
        ) as usize;
        *offset += 4;

        // Read bloom_filter
        let bloom_filter = if bloom_size > 0 {
            if *offset + bloom_size > total_len {
                return Err(FatalError::Corruption(
                    "Invalid OPT-009 block header: not enough data for bloom_filter".to_string(),
                ));
            }
            let bloom_bytes = data[*offset..*offset + bloom_size].to_vec();
            *offset += bloom_size;
            Some(bloom_bytes)
        } else {
            None
        };

        Ok(Self {
            min_key,
            max_key,
            entry_count,
            block_offset,
            bloom_filter,
        })
    }

    /// Check if a key might exist in this block based on key range
    pub fn key_might_exist(&self, key: &str) -> bool {
        key >= self.min_key.as_str() && key <= self.max_key.as_str()
    }
}

/// Block header format for compressed blocks
///
/// When block compression is enabled, each block is prefixed with a header:
/// ┌─────────────────────────────────────┐
/// │ Block Header (22 bytes)             │
/// │ ├─ magic: u32 (0x424C4B48 = "BLKH") │
/// │ ├─ version: u32 (2)                 │
/// │ ├─ compressed_size: u32             │
/// │ ├─ uncompressed_size: u32           │
/// │ ├─ checksum: u32 (CRC32C of data)   │
/// │ ├─ is_compressed: u8 (1=yes, 0=no)  │
/// │ ├─ algorithm_id: u8 (0=none, 1=zstd, 2=snappy, 3=lz4) │
/// ├─────────────────────────────────────┤
/// │ Block Data (compressed or raw)      │
/// └─────────────────────────────────────┘
///
/// Version 1 (legacy): 21 bytes, no algorithm_id (assumes zstd)
/// Version 2 (current): 22 bytes, includes algorithm_id for multi-algorithm support
pub const BLOCK_HEADER_MAGIC: u32 = 0x424C4B48; // "BLKH" = BLocK Header
pub const BLOCK_HEADER_VERSION: u32 = 2; // V2: Added algorithm_id byte
pub const BLOCK_HEADER_SIZE: u64 = 22; // 4+4+4+4+4+1+1 (added algorithm_id)
/// Legacy V1 block header size (no algorithm_id)
pub const BLOCK_HEADER_SIZE_V1: u64 = 21;

/// Block header for compressed blocks
#[derive(Debug, Clone, Copy)]
pub struct BlockHeader {
    pub compressed_size: u32,
    pub uncompressed_size: u32,
    pub checksum: u32,
    pub is_compressed: bool,
    /// Compression algorithm ID: 0=none, 1=zstd, 2=snappy, 3=lz4
    pub algorithm_id: u8,
}

impl BlockHeader {
    /// Serialize block header to bytes
    pub fn to_bytes(&self) -> [u8; BLOCK_HEADER_SIZE as usize] {
        let mut buf = [0u8; BLOCK_HEADER_SIZE as usize];
        buf[0..4].copy_from_slice(&BLOCK_HEADER_MAGIC.to_le_bytes());
        buf[4..8].copy_from_slice(&BLOCK_HEADER_VERSION.to_le_bytes());
        buf[8..12].copy_from_slice(&self.compressed_size.to_le_bytes());
        buf[12..16].copy_from_slice(&self.uncompressed_size.to_le_bytes());
        buf[16..20].copy_from_slice(&self.checksum.to_le_bytes());
        buf[20] = if self.is_compressed { 1 } else { 0 };
        buf[21] = self.algorithm_id;
        buf
    }

    /// Deserialize block header from bytes
    pub fn from_bytes(buf: &[u8; BLOCK_HEADER_SIZE as usize]) -> Result<Self, FatalError> {
        let magic = u32::from_le_bytes(
            buf[0..4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid block magic bytes: {}", e)))?,
        );
        if magic != BLOCK_HEADER_MAGIC {
            return Err(FatalError::Corruption(format!(
                "Invalid block header magic: expected {:08X}, got {:08X}",
                BLOCK_HEADER_MAGIC, magic
            )));
        }
        let version = u32::from_le_bytes(
            buf[4..8]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid block version bytes: {}", e)))?,
        );
        let compressed_size = u32::from_le_bytes(
            buf[8..12]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid compressed size bytes: {}", e)))?,
        );
        let uncompressed_size = u32::from_le_bytes(
            buf[12..16]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid uncompressed size bytes: {}", e)))?,
        );
        let checksum = u32::from_le_bytes(
            buf[16..20]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid checksum bytes: {}", e)))?,
        );
        let is_compressed = buf[20] != 0;
        // V2 has algorithm_id at byte 21; V1 defaults to zstd (1) for backward compatibility
        let algorithm_id = if version >= 2 { buf[21] } else { 1 };
        Ok(Self {
            compressed_size,
            uncompressed_size,
            checksum,
            is_compressed,
            algorithm_id,
        })
    }

    /// Deserialize block header from V1 bytes (21 bytes, no algorithm_id)
    /// Used for backward compatibility with old segment files
    pub fn from_bytes_v1(buf: &[u8; BLOCK_HEADER_SIZE_V1 as usize]) -> Result<Self, FatalError> {
        let magic = u32::from_le_bytes(
            buf[0..4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid block magic bytes: {}", e)))?,
        );
        if magic != BLOCK_HEADER_MAGIC {
            return Err(FatalError::Corruption(format!(
                "Invalid block header magic: expected {:08X}, got {:08X}",
                BLOCK_HEADER_MAGIC, magic
            )));
        }
        let compressed_size = u32::from_le_bytes(
            buf[8..12]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid compressed size bytes: {}", e)))?,
        );
        let uncompressed_size = u32::from_le_bytes(
            buf[12..16]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid uncompressed size bytes: {}", e)))?,
        );
        let checksum = u32::from_le_bytes(
            buf[16..20]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid checksum bytes: {}", e)))?,
        );
        let is_compressed = buf[20] != 0;
        // V1 only supports zstd
        Ok(Self {
            compressed_size,
            uncompressed_size,
            checksum,
            is_compressed,
            algorithm_id: 1, // zstd
        })
    }
}

/// Scan result type alias for complex return type
pub type ScanResult = Option<(String, Vec<u8>, u64, u32)>;

/// CFG-003: Dense index entry type: (offset, key_len, value_len, checksum)
/// Internal format used within segment.rs (not persisted)
/// NOTE: Reserved for future dense index serialization support
#[allow(dead_code)]
type DenseIndexEntry = (u64, u32, u32, u32);

/// Segment 文件管理器
///
/// 1.2 OPTIMIZATION: Leveled Compaction Support
/// - `level`: Compaction level (L0=memtable flush, L1/L2/L3=compacted)
/// - L0 segments may have overlapping key ranges
/// - L1+ segments have non-overlapping key ranges within their level
///
/// Level-aware reading support:
/// - `min_key`: Minimum key in this segment (for range-based lookup)
/// - `max_key`: Maximum key in this segment (for range-based lookup)
pub struct SegmentFile {
    /// 段文件 ID
    pub id: u64,
    /// 1.2 OPTIMIZATION: Compaction level (0=memtable flush, 1+=compacted)
    pub level: u8,
    /// Minimum key in this segment (for L1+ range-based lookup)
    pub min_key: parking_lot::RwLock<Option<String>>,
    /// Maximum key in this segment (for L1+ range-based lookup)
    pub max_key: parking_lot::RwLock<Option<String>>,
    /// 文件路径
    pub path: PathBuf,
    /// Filesystem abstraction
    fs: Arc<dyn FileKVFileSystem>,
    /// Mmap filesystem abstraction (only set if fs implements MmapFileSystem)
    mmap_fs: Option<Arc<dyn crate::io::MmapFileSystem>>,
    /// 文件句柄（追加模式，用于写入）
    write_file: Mutex<Box<dyn FileKVFile>>,
    /// 当前文件大小
    size: AtomicU64,
    /// 条目数
    entry_count: AtomicU64,
    /// PERF-002: 持久 mmap 只读映射（用于所有读取操作）
    /// RES-001: 使用 ArcSwapOption 替代 `RwLock<Option<Arc<Mmap>>>` 简化并发管理
    mmap: Arc<ArcSwapOption<Arc<dyn MmapView>>>,
    /// P4-001: 是否使用持久 mmap（false = 每次读取时临时创建 mmap）
    use_persistent_mmap: bool,
    /// CFG-001: 预读倍数（0 = 禁用预读）
    readahead_multiplier: u32,
    /// CFG-003: 全内存密集索引 (key -> DenseIndexEntry)
    /// 使用 RwLock 保护以支持并发更新
    /// PERF-005 P2: Uses HashMap for O(1) lookups in the dense index.
    dense_index:
        Option<parking_lot::RwLock<std::collections::HashMap<String, crate::core::sparse_index::DenseIndexEntry>>>,
}

impl std::fmt::Debug for SegmentFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let min_key = self.min_key.read();
        let max_key = self.max_key.read();
        f.debug_struct("SegmentFile")
            .field("id", &self.id)
            .field("level", &self.level)
            .field("min_key", &*min_key)
            .field("max_key", &*max_key)
            .field("path", &self.path)
            .field("size", &self.size.load(Ordering::Relaxed))
            .field("entry_count", &self.entry_count.load(Ordering::Relaxed))
            .field("use_persistent_mmap", &self.use_persistent_mmap)
            .field("readahead_multiplier", &self.readahead_multiplier)
            .finish()
    }
}

impl SegmentFile {
    /// 创建新的 segment 文件
    ///
    /// 如果 preallocate_size > 0，会预分配指定大小的文件空间
    ///
    /// # Arguments
    /// * `fs` - Filesystem abstraction
    /// * `level` - 1.2 OPTIMIZATION: Compaction level (0=L0 memtable flush, 1+=compacted)
    /// * `readahead_multiplier` - CFG-001: 预读倍数
    /// * `dense_index_enabled` - CFG-003: 是否构建全内存密集索引
    #[allow(clippy::too_many_arguments)] // Segment creation requires many configuration parameters
    pub fn create(
        fs: Arc<dyn FileKVFileSystem>,
        id: u64,
        level: u8,
        path: &Path,
        preallocate_size: u64,
        use_persistent_mmap: bool,
        readahead_multiplier: u32,
        dense_index_enabled: bool,
    ) -> Result<Self, FatalError> {
        // Derive mmap_fs from fs if supported
        let mmap_fs: Option<Arc<dyn crate::io::MmapFileSystem>> = fs.clone_as_mmap_fs();
        // Check if file exists and has data
        let file_exists = fs.file_exists(path);
        let existing_size = if file_exists {
            fs.file_metadata(path).map(|m| m.len).unwrap_or(0)
        } else {
            0
        };

        let mut file = fs.open_file(path, true, true, true).map_err(FatalError::Io)?;

        // Write header for new empty files
        let initial_size = if !file_exists || existing_size == 0 {
            // Write 8-byte header (magic + version)
            let writer = &mut *file;
            writer.write_all(&SEGMENT_MAGIC.to_le_bytes()).map_err(FatalError::Io)?;
            writer
                .write_all(&SEGMENT_VERSION.to_le_bytes())
                .map_err(FatalError::Io)?;
            writer.flush().map_err(FatalError::Io)?;
            8u64
        } else {
            existing_size
        };

        if preallocate_size > initial_size {
            // No set_len on trait, we need to get metadata and skip
            // For StdFs, we can try to downcast, but for simplicity just write padding
            // Actually, the underlying file may support set_len via as_any downcast
            // For now, we write zero bytes to pad the file
            let current = file.metadata().map_err(FatalError::Io)?.len;
            if preallocate_size > current {
                let padding = vec![0u8; (preallocate_size - current) as usize];
                file.write_all(&padding).map_err(FatalError::Io)?;
            }
        }

        // Get the file handle for mmap (need a read-capable handle)
        let read_file = fs.open_file(path, true, false, false).map_err(FatalError::Io)?;

        // RES-001: Use ArcSwapOption for lock-free mmap management
        let mmap: Arc<ArcSwapOption<Arc<dyn MmapView>>> = Arc::new(ArcSwapOption::empty());

        // If file has data, create mmap
        if initial_size > 0 {
            if let Some(ref mmap_fs) = mmap_fs {
                let mmap_view = mmap_fs.mmap(read_file.as_ref()).map_err(FatalError::Io)?;
                mmap.store(Some(Arc::new(mmap_view)));
            }
        }

        // CFG-003: Build dense index if enabled (empty for new files)
        let dense_index = if dense_index_enabled {
            Some(parking_lot::RwLock::new(std::collections::HashMap::new()))
        } else {
            None
        };

        Ok(Self {
            id,
            level,
            min_key: parking_lot::RwLock::new(None),
            max_key: parking_lot::RwLock::new(None),
            path: path.to_path_buf(),
            fs,
            mmap_fs,
            write_file: Mutex::new(file),
            size: AtomicU64::new(initial_size),
            entry_count: AtomicU64::new(0),
            mmap,
            use_persistent_mmap,
            readahead_multiplier,
            dense_index,
        })
    }

    /// 打开现有 segment 文件
    ///
    /// # P1-006 FIX: Safety measures for mmap usage
    /// - File is opened read-only for mmap (separate handle for writes)
    /// - Mmap is created with read-only permissions
    /// - File size is validated before mmap
    /// - All mmap accesses include bounds checking
    ///
    /// # PERF-002: Persistent mmap
    /// - Creates mmap once at open time
    /// - Reuses mmap for all read operations
    /// - Uses RwLock for thread-safe access
    ///
    /// # Arguments
    /// * `fs` - Filesystem abstraction
    /// * `level` - 1.2 OPTIMIZATION: Compaction level (0=L0 memtable flush, 1+=compacted)
    /// * `readahead_multiplier` - CFG-001: 预读倍数
    /// * `dense_index_enabled` - CFG-003: 是否构建全内存密集索引
    pub fn open(
        fs: Arc<dyn FileKVFileSystem>,
        id: u64,
        level: u8,
        path: &Path,
        use_persistent_mmap: bool,
        readahead_multiplier: u32,
        dense_index_enabled: bool,
    ) -> Result<Self, FatalError> {
        // Derive mmap_fs from fs if supported
        let mmap_fs: Option<Arc<dyn crate::io::MmapFileSystem>> = fs.clone_as_mmap_fs();
        // Open file for reading and writing
        let file = fs.open_file(path, true, true, true).map_err(FatalError::Io)?;

        let metadata = file.metadata().map_err(FatalError::Io)?;
        let size = metadata.len;

        // P1-006 FIX: Validate file size before mmap
        // Files smaller than header (8 bytes) are invalid
        if size > 0 && size < 8 {
            return Err(FatalError::Corruption(format!(
                "Segment file too small: {} bytes (minimum: 8 bytes for header)",
                size
            )));
        }

        // PERF-002: Create persistent mmap once at open time (if enabled)
        // RES-001: Use ArcSwapOption for lock-free mmap management
        let mmap_arc: Arc<ArcSwapOption<Arc<dyn MmapView>>> = if use_persistent_mmap && size > 0 {
            // P1-006 FIX: Use trait mmap for explicit read-only mapping
            let mmap = if let Some(ref mmap_fs) = mmap_fs {
                mmap_fs.mmap(file.as_ref()).map_err(FatalError::Io)?
            } else {
                return Err(FatalError::Corruption(
                    "mmap filesystem not available for persistent mmap".to_string(),
                ));
            };

            // P1-006 FIX: Validate mmap contents
            if size >= 8 {
                // SAFETY: mmap size is validated >= 8
                let magic_buf: [u8; 4] = mmap.as_slice()[0..4]
                    .try_into()
                    .map_err(|_| FatalError::Corruption("Failed to read magic bytes from mmap".to_string()))?;
                let magic = u32::from_le_bytes(magic_buf);
                if magic != SEGMENT_MAGIC {
                    return Err(FatalError::Corruption(format!(
                        "Invalid segment file magic: expected {:08X}, got {:08X}",
                        SEGMENT_MAGIC, magic
                    )));
                }

                let version_buf: [u8; 4] = mmap.as_slice()[4..8]
                    .try_into()
                    .map_err(|_| FatalError::Corruption("Failed to read version bytes from mmap".to_string()))?;
                let version = u32::from_le_bytes(version_buf);
                if version != SEGMENT_VERSION {
                    return Err(FatalError::Corruption(format!(
                        "Unsupported segment version: expected {}, got {}",
                        SEGMENT_VERSION, version
                    )));
                }
            }

            Arc::new(ArcSwapOption::new(Some(Arc::new(mmap))))
        } else {
            Arc::new(ArcSwapOption::empty())
        };

        // CFG-003: Build dense index if enabled
        let dense_index = if dense_index_enabled && size > 8 {
            // 2.1 OPTIMIZATION: Try to load persisted dense index first
            let idx_path = path.with_extension("dense_idx");
            match Self::load_dense_index(fs.as_ref(), &idx_path) {
                Ok(index) => {
                    // Successfully loaded from file - skip expensive build_dense_index()
                    tracing::debug!(
                        segment_id = id,
                        "Loaded dense index from file ({} entries)",
                        index.entries.len()
                    );
                    Some(parking_lot::RwLock::new(index.entries))
                }
                Err(_) => {
                    // Fallback: build from scan
                    tracing::debug!(segment_id = id, "Building dense index from scan (no persisted index)");
                    let index = Self::build_dense_index(&mmap_arc, size)?;
                    Some(parking_lot::RwLock::new(index.entries))
                }
            }
        } else {
            None
        };

        Ok(Self {
            id,
            level,
            min_key: parking_lot::RwLock::new(None),
            max_key: parking_lot::RwLock::new(None),
            path: path.to_path_buf(),
            fs,
            mmap_fs,
            write_file: Mutex::new(file.try_clone().map_err(FatalError::Io)?),
            size: AtomicU64::new(size),
            entry_count: AtomicU64::new(0),
            mmap: mmap_arc,
            use_persistent_mmap,
            readahead_multiplier,
            dense_index,
        })
    }

    /// CFG-003: Build dense index from mmap
    /// Scans the entire segment and builds a DenseIndex of key -> (offset, key_len, value_len, checksum)
    fn build_dense_index(
        mmap_arc: &Arc<ArcSwapOption<Arc<dyn MmapView>>>,
        file_size: u64,
    ) -> Result<crate::core::sparse_index::DenseIndex, FatalError> {
        use crate::core::sparse_index::{DenseIndex, DenseIndexEntry as SparseDenseIndexEntry};

        // Default block size for sequential prefetch tracking
        const DEFAULT_BLOCK_SIZE: u64 = 8192;
        let mut index = DenseIndex::with_block_size(DEFAULT_BLOCK_SIZE);

        let mmap_guard = mmap_arc.load();
        let mmap = match &*mmap_guard {
            Some(m) => m,
            None => return Ok(index), // Empty file, return empty index
        };

        let mut pos = 8usize; // Skip header (magic + version)
        let file_size = file_size as usize;

        while pos + 4 <= file_size {
            let entry_start = pos as u64;

            let key_len = match mmap.as_slice()[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + key_len > file_size {
                break;
            }

            let key_bytes = &mmap.as_slice()[pos..pos + key_len];
            let key = match String::from_utf8(key_bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => break, // Invalid UTF-8, stop indexing
            };
            pos += key_len;

            if pos + 4 > file_size {
                break;
            }

            let value_len = match mmap.as_slice()[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf),
                Err(_) => break,
            };
            pos += 4;

            if pos + value_len as usize + 4 > file_size {
                break;
            }

            pos += value_len as usize;

            let checksum = match mmap.as_slice()[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf),
                Err(_) => break,
            };
            pos += 4;

            // GAP-C4: Calculate block_id for sequential prefetch
            let block_id = index.offset_to_block_id(entry_start);

            // CFG-003: Store in dense index
            // key_len is stored as u32 for consistency
            index.entries.insert(
                key,
                SparseDenseIndexEntry {
                    offset: entry_start,
                    key_len: key_len as u32,
                    value_len,
                    checksum,
                    seq_num: 0, // Not tracked in segment-level dense index
                    block_id,   // GAP-C4: Track block ID for prefetch
                },
            );
        }

        Ok(index)
    }

    /// 2.1 OPTIMIZATION: Save dense index to file
    /// Should be called after compaction or segment flush
    pub fn save_dense_index(
        fs: &dyn FileKVFileSystem,
        index: &crate::core::sparse_index::DenseIndex,
        path: &std::path::Path,
    ) -> Result<(), FatalError> {
        // Serialize using bincode for performance
        let buffer = bincode::serialize(index)
            .map_err(|e| FatalError::Corruption(format!("Failed to serialize dense index: {}", e)))?;

        // Write to temporary file first, then rename for atomicity
        let temp_path = path.with_extension("dense_idx.tmp");
        let mut temp_file = fs.create_file(&temp_path).map_err(FatalError::Io)?;
        temp_file.write_all(&buffer).map_err(FatalError::Io)?;
        temp_file.flush().map_err(FatalError::Io)?;
        drop(temp_file);
        fs.rename(&temp_path, path).map_err(FatalError::Io)?;

        Ok(())
    }

    /// 2.1 OPTIMIZATION: Load dense index from persisted file
    /// Returns Ok(DenseIndex) if successful, Err if file doesn't exist or is corrupted
    fn load_dense_index(
        fs: &dyn FileKVFileSystem,
        path: &std::path::Path,
    ) -> Result<crate::core::sparse_index::DenseIndex, FatalError> {
        use std::io::Read;

        if !fs.file_exists(path) {
            return Err(FatalError::Corruption("Dense index file not found".to_string()));
        }

        // FileKVFile doesn't have a read method, so for this rare cold path we use std::fs directly.
        let mut std_file = std::fs::File::open(path).map_err(FatalError::Io)?;
        let mut buffer = Vec::new();
        std_file.read_to_end(&mut buffer).map_err(FatalError::Io)?;

        // Deserialize using bincode for performance
        let index: crate::core::sparse_index::DenseIndex = bincode::deserialize(&buffer)
            .map_err(|e| FatalError::Corruption(format!("Failed to deserialize dense index: {}", e)))?;

        Ok(index)
    }

    /// 追加写入键值对
    ///
    /// 返回写入位置（offset, len, checksum）
    pub fn append(&self, key: &str, value: &[u8]) -> Result<(u64, u32, u32), FatalError> {
        let mut file = self.write_file.lock();
        let offset = self.size.load(Ordering::Relaxed);

        let key_bytes = key.as_bytes();
        let key_len = key_bytes.len() as u32;
        let value_len = value.len() as u32;

        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(key_bytes);
        hasher.write(value);
        let checksum = hasher.finish() as u32;

        file.write_all(&key_len.to_le_bytes())?;
        file.write_all(key_bytes)?;
        file.write_all(&value_len.to_le_bytes())?;
        file.write_all(value)?;
        file.write_all(&checksum.to_le_bytes())?;
        file.flush()?;

        let entry_size = 4 + key_bytes.len() + 4 + value.len() + 4;
        self.size.fetch_add(entry_size as u64, Ordering::Relaxed);
        self.entry_count.fetch_add(1, Ordering::Relaxed);

        // CFG-003: Update dense index if enabled
        if let Some(ref index) = self.dense_index {
            use crate::core::sparse_index::DenseIndexEntry as SparseDenseIndexEntry;
            // Calculate block_id for sequential prefetch
            const DEFAULT_BLOCK_SIZE: u64 = 8192;
            let block_id = if DEFAULT_BLOCK_SIZE > 0 {
                offset / DEFAULT_BLOCK_SIZE
            } else {
                0
            };
            index.write().insert(
                key.to_string(),
                SparseDenseIndexEntry {
                    offset,
                    key_len,
                    value_len,
                    checksum,
                    seq_num: 0, // Not tracked in segment-level dense index
                    block_id,   // GAP-C4: Track block ID for prefetch
                },
            );
        }

        // Update min_key/max_key for level-aware reading
        self.update_key_range(key);

        Ok((offset, value_len, checksum))
    }

    /// Refresh the mmap after data has been written
    ///
    /// PERF-002: Re-map the file after size changes
    /// RES-001: Use ArcSwapOption for lock-free update
    fn refresh_mmap(&self) -> Result<(), FatalError> {
        // If mmap is not enabled for this segment, skip refresh
        if !self.use_persistent_mmap {
            return Ok(());
        }

        // If no mmap filesystem available, skip refresh gracefully
        let mmap_fs = match &self.mmap_fs {
            Some(m) => m,
            None => return Ok(()), // No mmap support, skip gracefully
        };

        // Open file for reading
        let file = self
            .fs
            .open_file(&self.path, true, false, false)
            .map_err(FatalError::Io)?;

        let metadata = file.metadata().map_err(FatalError::Io)?;
        let file_size = metadata.len;

        if file_size == 0 {
            // Empty file, clear mmap
            self.mmap.store(None);
            return Ok(());
        }

        // Create new mmap
        let mmap = mmap_fs.mmap(file.as_ref()).map_err(FatalError::Io)?;

        self.mmap.store(Some(Arc::new(mmap)));
        Ok(())
    }

    /// Update min_key/max_key for level-aware reading
    /// Called after each append to track the key range of this segment
    fn update_key_range(&self, key: &str) {
        // Update min_key
        {
            let mut min = self.min_key.write();
            match min.as_ref() {
                Some(current_min) if key < current_min.as_str() => {
                    *min = Some(key.to_string());
                }
                None => {
                    *min = Some(key.to_string());
                }
                _ => {}
            }
        }

        // Update max_key
        {
            let mut max = self.max_key.write();
            match max.as_ref() {
                Some(current_max) if key > current_max.as_str() => {
                    *max = Some(key.to_string());
                }
                None => {
                    *max = Some(key.to_string());
                }
                _ => {}
            }
        }
    }

    /// Populate min_key/max_key from dense_index
    /// Called after opening or rebuilding a segment
    pub fn populate_key_range_from_dense_index(&self) {
        if let Some(ref index) = self.dense_index {
            let index_guard = index.read();
            // PERF-005 P2: AHashMap is unordered, so we iterate to find min/max
            if !index_guard.is_empty() {
                let mut min_key: Option<&str> = None;
                let mut max_key: Option<&str> = None;
                for key in index_guard.keys() {
                    let key_str = key.as_str();
                    if min_key.is_none() || key_str < min_key.unwrap() {
                        min_key = Some(key_str);
                    }
                    if max_key.is_none() || key_str > max_key.unwrap() {
                        max_key = Some(key_str);
                    }
                }
                if let Some(k) = min_key {
                    *self.min_key.write() = Some(k.to_string());
                }
                if let Some(k) = max_key {
                    *self.max_key.write() = Some(k.to_string());
                }
            }
        }
    }

    /// 通过偏移读取值
    ///
    /// # PERF-002: Uses persistent mmap (no re-mapping)
    /// # P1-006 FIX: Safety measures
    /// - Validates offset is within file bounds before mmap access
    /// - All slice accesses include bounds checking via try_into()
    ///
    /// # P4-001: When use_persistent_mmap is false
    /// - Creates a temporary mmap for each read
    /// - Slower but uses fewer file handles
    pub fn read_at(&self, offset: u64, len: u32) -> Result<Vec<u8>, FatalError> {
        self.flush()?;

        // P4-001: Support both persistent and temporary mmap modes
        if self.use_persistent_mmap {
            // RES-001: Use ArcSwapOption load() for lock-free access
            let mmap_guard = self.mmap.load();
            if let Some(m) = &*mmap_guard {
                return self.read_at_from_mmap(m, offset);
            }
        }

        // Fallback: create temporary mmap or read from file
        if let Some(ref mmap_fs) = self.mmap_fs {
            let file = self
                .fs
                .open_file(&self.path, true, false, false)
                .map_err(FatalError::Io)?;
            let mmap = mmap_fs.mmap(file.as_ref()).map_err(FatalError::Io)?;
            self.read_at_from_mmap(&mmap, offset)
        } else {
            // Final fallback: read from file directly (for MemFs testing)
            self.read_at_from_file(offset, len)
        }
    }

    /// Read from file directly without mmap (fallback for MemFs)
    fn read_at_from_file(&self, offset: u64, len: u32) -> Result<Vec<u8>, FatalError> {
        // Read entire file
        let file_size = self.fs.file_metadata(&self.path).map(|m| m.len).unwrap_or(0) as usize;
        if offset as usize >= file_size {
            return Err(FatalError::Corruption(format!(
                "Read offset {} out of bounds (file size: {})",
                offset, file_size
            )));
        }

        // We need to read the file from the offset
        // Since we can't seek with the trait, we read from beginning and skip
        let mut file = self
            .fs
            .open_file(&self.path, true, false, false)
            .map_err(FatalError::Io)?;
        let mut buf = vec![0u8; file_size];
        let mut pos = 0;
        while pos < file_size {
            match file.read(&mut buf[pos..]) {
                Ok(0) => break,
                Ok(n) => pos += n,
                Err(e) => return Err(FatalError::Io(e)),
            }
        }

        let end = (offset as usize + len as usize).min(file_size);
        Ok(buf[offset as usize..end].to_vec())
    }

    /// Helper: read from a given mmap
    fn read_at_from_mmap(&self, mmap: &Arc<dyn MmapView>, offset: u64) -> Result<Vec<u8>, FatalError> {
        let file_size = mmap.len();
        let data = mmap.as_slice();

        // P1-006 FIX: Validate offset is within bounds
        if offset as usize >= file_size {
            return Err(FatalError::Corruption(format!(
                "Read offset {} out of bounds (file size: {})",
                offset, file_size
            )));
        }

        // P1-006 FIX: Validate offset and all slice accesses
        let mut pos = offset as usize;

        if pos + 4 > file_size {
            return Err(FatalError::Corruption(format!(
                "Invalid offset: not enough data for key length (pos={}, mmap_size={})",
                pos, file_size
            )));
        }

        let key_len = u32::from_le_bytes(
            data[pos..pos + 4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid key length bytes: {}", e)))?,
        ) as usize;
        pos += 4;

        if pos + key_len > file_size {
            return Err(FatalError::Corruption(format!(
                "Invalid key length: extends beyond file (pos={}, key_len={}, mmap_size={})",
                pos, key_len, file_size
            )));
        }
        pos += key_len;

        if pos + 4 > file_size {
            return Err(FatalError::Corruption(format!(
                "Invalid offset: not enough data for value length (pos={}, mmap_size={})",
                pos, file_size
            )));
        }

        let value_len = u32::from_le_bytes(
            data[pos..pos + 4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid value length bytes: {}", e)))?,
        ) as usize;
        pos += 4;

        if pos + value_len > file_size {
            return Err(FatalError::Corruption(format!(
                "Invalid value length: extends beyond file (pos={}, value_len={}, mmap_size={})",
                pos, value_len, file_size
            )));
        }

        let value = data[pos..pos + value_len].to_vec();
        Ok(value)
    }

    /// 通过偏移和已知的 value_len 读取值（快速路径）
    ///
    /// # Arguments
    /// * `offset` - entry 起始偏移（key_len 位置）
    /// * `key_len` - key 长度（字节）
    /// * `value_len` - value 长度（字节）
    ///
    /// # PERF-003: Skip parsing header when value_len is known from DenseIndex
    /// - Avoids re-reading key_len and value_len from disk
    /// - Directly jumps to value data
    /// - ~30-40% faster than read_at() for known-length lookups
    ///
    /// # P1-006 FIX: Safety measures
    /// - Validates all offsets before mmap access
    /// - All slice accesses include bounds checking
    ///
    /// # P4-001: When use_persistent_mmap is false
    /// - Creates a temporary mmap for this read
    pub fn read_at_fast(&self, offset: u64, key_len: usize, value_len: usize) -> Result<Vec<u8>, FatalError> {
        self.flush()?;

        // P4-001: Support both persistent and temporary mmap modes
        if self.use_persistent_mmap {
            // RES-001: Use ArcSwapOption load() for lock-free access
            let mmap_guard = self.mmap.load();
            if let Some(m) = &*mmap_guard {
                return self.read_at_fast_from_mmap(m, offset, key_len, value_len);
            }
        }

        // Fallback: create temporary mmap or read from file
        if let Some(ref mmap_fs) = self.mmap_fs {
            let file = self
                .fs
                .open_file(&self.path, true, false, false)
                .map_err(FatalError::Io)?;
            let mmap = mmap_fs.mmap(file.as_ref()).map_err(FatalError::Io)?;
            self.read_at_fast_from_mmap(&mmap, offset, key_len, value_len)
        } else {
            // Final fallback: read from file directly (for MemFs testing)
            self.read_at_fast_from_file(offset, key_len, value_len)
        }
    }

    /// Read from file directly without mmap (fallback for MemFs)
    fn read_at_fast_from_file(&self, offset: u64, _key_len: usize, value_len: usize) -> Result<Vec<u8>, FatalError> {
        // Entry layout: key_len(4) + key + value_len(4) + value + checksum(4)
        let entry_size = 4 + _key_len + 4 + value_len + 4;
        // Read the entire entry from file
        let data = self.read_at(offset, entry_size as u32)?;
        // Return just the value portion (after key_len + key + value_len)
        let value_start = 4 + _key_len + 4;
        if value_start + value_len > data.len() {
            return Err(FatalError::Corruption(format!(
                "Not enough data for value (need {} bytes, got {})",
                value_start + value_len,
                data.len()
            )));
        }
        Ok(data[value_start..value_start + value_len].to_vec())
    }

    /// Helper: fast read from a given mmap
    fn read_at_fast_from_mmap(
        &self,
        mmap: &Arc<dyn MmapView>,
        offset: u64,
        key_len: usize,
        value_len: usize,
    ) -> Result<Vec<u8>, FatalError> {
        let file_size = mmap.len();
        let data = mmap.as_slice();
        let mut pos = offset as usize;

        // P1-006 FIX: Validate entire entry fits in file
        // Entry layout: key_len(4) + key + value_len(4) + value + checksum(4)
        let entry_size = 4 + key_len + 4 + value_len + 4;
        if pos + entry_size > file_size {
            return Err(FatalError::Corruption(format!(
                "Read offset {} with len {} out of bounds (file size: {})",
                offset, entry_size, file_size
            )));
        }

        // Skip key_len (4 bytes) + key
        pos += 4 + key_len;

        // Skip value_len (4 bytes)
        pos += 4;

        // Directly read value
        let value = data[pos..pos + value_len].to_vec();
        Ok(value)
    }

    /// PERF-ZEROCOPY-001: Zero-copy fast read returning `bytes::Bytes` backed by mmap.
    ///
    /// Instead of allocating a new `Vec<u8>` on every read, this returns a `Bytes`
    /// that holds a clone of the `Arc<dyn MmapView>`, so the value slice is zero-copy.
    ///
    /// # Arguments
    /// * `offset` - entry 起始偏移（key_len 位置）
    /// * `key_len` - key 长度（字节）
    /// * `value_len` - value 长度（字节）
    pub fn read_at_fast_with_bytes(
        &self,
        offset: u64,
        key_len: usize,
        value_len: usize,
    ) -> Result<bytes::Bytes, FatalError> {
        self.flush()?;

        // Resolve which mmap to use
        let mmap: Arc<dyn MmapView> = if self.use_persistent_mmap {
            let mmap_guard = self.mmap.load();
            match &*mmap_guard {
                Some(inner_arc) => Arc::clone(&**inner_arc),
                None => return self.read_at_fast_with_bytes_fallback(offset, key_len, value_len),
            }
        } else if let Some(ref mmap_fs) = self.mmap_fs {
            let file = self
                .fs
                .open_file(&self.path, true, false, false)
                .map_err(FatalError::Io)?;
            mmap_fs.mmap(file.as_ref()).map_err(FatalError::Io)?
        } else {
            return self.read_at_fast_with_bytes_fallback(offset, key_len, value_len);
        };

        let file_size = mmap.len();
        let mut pos = offset as usize;

        let entry_size = 4 + key_len + 4 + value_len + 4;
        if pos + entry_size > file_size {
            return Err(FatalError::Corruption(format!(
                "Read offset {} with len {} out of bounds (file size: {})",
                offset, entry_size, file_size
            )));
        }

        // Skip key_len (4 bytes) + key
        pos += 4 + key_len;
        // Skip value_len (4 bytes)
        pos += 4;

        // Zero-copy: create a Bytes that owns the mmap reference
        let value_start = pos;
        let mmap_owner = MmapSliceOwner {
            mmap,
            offset: value_start,
            len: value_len,
        };
        Ok(bytes::Bytes::from_owner(mmap_owner))
    }

    /// Fallback: allocate Vec<u8> when mmap is not available
    fn read_at_fast_with_bytes_fallback(
        &self,
        offset: u64,
        key_len: usize,
        value_len: usize,
    ) -> Result<bytes::Bytes, FatalError> {
        let value = self.read_at_fast(offset, key_len, value_len)?;
        Ok(bytes::Bytes::from(value))
    }

    /// POL-004: Quick check if key might exist in this segment (using dense index)
    ///
    /// PERF-005 P2: This is a fast O(1) HashMap lookup that avoids expensive bloom filter
    /// and zone map overhead when the dense index can definitively answer.
    ///
    /// # Returns
    /// - Some(true) - key exists in dense index, proceed to read
    /// - Some(false) - key definitely not in this segment (dense index says no)
    /// - None - dense index not enabled, caller must use bloom/zone map
    #[allow(dead_code)]
    pub fn key_might_exist_in_dense_index(&self, key: &str) -> Option<bool> {
        if let Some(ref index) = self.dense_index {
            let index_read = index.read();
            Some(index_read.contains_key(key))
        } else {
            None
        }
    }

    /// CFG-003: Get value by key using dense index (if enabled)
    ///
    /// # Returns
    /// - Some(value) - key found, value returned
    /// - None - key not found or dense index not enabled
    pub fn get_by_key(&self, key: &str) -> Result<Option<Vec<u8>>, FatalError> {
        // CFG-003: Try dense index first if enabled
        if let Some(ref index) = self.dense_index {
            let index_read = index.read();
            if let Some(entry) = index_read.get(key) {
                // Found in dense index, read directly
                let offset = entry.offset;
                let key_len = entry.key_len;
                let value_len = entry.value_len;
                let checksum = entry.checksum;
                drop(index_read);

                self.flush()?;

                let mmap_guard = self.mmap.load();
                let mmap = match &*mmap_guard {
                    Some(m) => m,
                    None => {
                        return Err(FatalError::Corruption("Segment file not mapped or empty".to_string()));
                    }
                };

                let file_size = mmap.len();
                let data = mmap.as_slice();
                let pos = offset as usize;
                let entry_size = 4 + key_len as usize + 4 + value_len as usize + 4;

                if pos + entry_size > file_size {
                    // Index may be stale, fallback to None
                    return Ok(None);
                }

                // Skip key_len (4 bytes) + key
                let value_pos = pos + 4 + key_len as usize + 4;

                // Read value and validate checksum
                let value = data[value_pos..value_pos + value_len as usize].to_vec();

                // Validate checksum
                let checksum_pos = value_pos + value_len as usize;
                let checksum_bytes = data[checksum_pos..checksum_pos + 4].try_into();
                let stored_checksum = match checksum_bytes {
                    Ok(bytes) => u32::from_le_bytes(bytes),
                    Err(_) => return Ok(None), // Corrupted entry, treat as not found
                };
                if stored_checksum == checksum {
                    return Ok(Some(value));
                }
                // Checksum mismatch, index may be stale
                return Ok(None);
            }
        }

        // Not found in dense index or dense index not enabled
        Ok(None)
    }

    /// CFG-001: Read with automatic readahead based on configured multiplier
    ///
    /// Uses `self.readahead_multiplier` to determine how many additional blocks to pre-read.
    /// Useful for sequential scan workloads where consecutive entries are likely to be accessed.
    /// P4-001: Read with readahead (预读)
    ///
    /// # Arguments
    /// * `offset` - entry 起始偏移
    /// * `key_len` - key 长度
    /// * `value_len` - value 长度
    /// * `readahead_blocks` - 预读的额外 block 数量（0 = 不预读）
    ///
    /// # Returns
    /// - Ok((value, readahead_data)) - 读取的值和预读的数据
    ///
    /// # P4-001: Readahead mechanism
    /// - When readahead_blocks > 0, reads additional consecutive blocks
    /// - Useful for sequential scan workloads
    /// - Returns preloaded data that can be cached for future reads
    pub fn read_at_with_readahead(
        &self,
        offset: u64,
        key_len: usize,
        value_len: usize,
        readahead_blocks: u32,
    ) -> Result<(Vec<u8>, Vec<Vec<u8>>), FatalError> {
        let value = self.read_at_fast(offset, key_len, value_len)?;

        if readahead_blocks == 0 {
            return Ok((value, Vec::new()));
        }

        // P4-001: Read additional blocks for readahead
        // Assume average block size is ~value_len for simplicity
        let mut readahead_data = Vec::with_capacity(readahead_blocks as usize);
        let next_offset = offset + 4 + key_len as u64 + 4 + value_len as u64 + 4; // Skip to next entry

        for i in 0..readahead_blocks {
            let next_off = next_offset + (i as u64 * (value_len as u64 + 20)); // Estimate next entry offset
            match self.read_at_fast(next_off, key_len, value_len) {
                Ok(data) => readahead_data.push(data),
                Err(_) => break, // Stop if we hit EOF or error
            }
        }

        Ok((value, readahead_data))
    }

    /// CFG-001: Read with automatic readahead based on configured multiplier
    ///
    /// Uses `self.readahead_multiplier` to determine how many additional blocks to pre-read.
    /// This is the primary read method for sequential scan workloads.
    ///
    /// # Returns
    /// - Ok((value, readahead_data)) - 读取的值和预读的数据
    pub fn read_at_with_configured_readahead(
        &self,
        offset: u64,
        key_len: usize,
        value_len: usize,
    ) -> Result<(Vec<u8>, Vec<Vec<u8>>), FatalError> {
        let readahead_blocks = self.readahead_multiplier;
        self.read_at_with_readahead(offset, key_len, value_len, readahead_blocks)
    }

    /// 通过偏移和已知的 value_len/checksum 读取并验证值（快速路径）
    ///
    /// # Arguments
    /// * `offset` - entry 起始偏移（key_len 位置）
    /// * `key_len` - key 长度（字节）
    /// * `value_len` - value 长度（字节）
    /// * `expected_checksum` - 期望的 CRC32C 校验和
    ///
    /// # Returns
    /// - `Ok(Some(value))` - 读取成功且校验和匹配
    /// - `Ok(None)` - 校验和不匹配（数据可能已损坏）
    /// - `Err` - I/O 错误
    ///
    /// # PERF-003: Fast path with checksum verification
    /// - Single mmap read, no re-parsing
    /// - Checksum verification included
    pub fn read_at_fast_verified(
        &self,
        offset: u64,
        key_len: usize,
        value_len: usize,
        expected_checksum: u32,
    ) -> Result<Option<Vec<u8>>, FatalError> {
        let value = self.read_at_fast(offset, key_len, value_len)?;

        // Verify checksum
        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(&value);
        let computed = hasher.finish() as u32;

        if computed != expected_checksum {
            tracing::warn!(
                "Checksum mismatch at offset {}: expected {:08X}, got {:08X}",
                offset,
                expected_checksum,
                computed
            );
            return Ok(None);
        }

        Ok(Some(value))
    }

    /// 读取键值对（需要知道偏移）
    ///
    /// # PERF-002: Uses persistent mmap (no re-mapping)
    /// # P1-006 FIX: Safety measures
    /// - Validates offset and all data accesses against file size
    /// - Uses try_into() for all slice conversions (panic-free)
    /// - Includes checksum verification for data integrity
    pub fn read_entry(&self, offset: u64) -> Result<(String, Vec<u8>, u32), FatalError> {
        self.flush()?;

        // RES-001: Use ArcSwapOption load() for lock-free access
        let mmap_guard = self.mmap.load();
        let mmap = match &*mmap_guard {
            Some(m) => m,
            None => {
                return Err(FatalError::Corruption("Segment file not mapped or empty".to_string()));
            }
        };

        let file_size = mmap.len();
        let data = mmap.as_slice();

        if offset as usize >= file_size {
            return Err(FatalError::Corruption(format!(
                "Read offset {} out of bounds (file size: {})",
                offset, file_size
            )));
        }

        let mut pos = offset as usize;

        // P1-006 FIX: Bounds-checked slice access
        if pos + 4 > file_size {
            return Err(FatalError::Corruption(
                "Invalid entry offset: not enough data for key length".to_string(),
            ));
        }

        let key_len = u32::from_le_bytes(
            data[pos..pos + 4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid key length bytes: {}", e)))?,
        ) as usize;
        pos += 4;

        if pos + key_len > file_size {
            return Err(FatalError::Corruption(
                "Invalid entry: key extends beyond file boundary".to_string(),
            ));
        }

        let key = String::from_utf8_lossy(&data[pos..pos + key_len]).to_string();
        pos += key_len;

        if pos + 4 > file_size {
            return Err(FatalError::Corruption(
                "Invalid entry: not enough data for value length".to_string(),
            ));
        }

        let value_len = u32::from_le_bytes(
            data[pos..pos + 4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid value length bytes: {}", e)))?,
        ) as usize;
        pos += 4;

        if pos + value_len > file_size {
            return Err(FatalError::Corruption(
                "Invalid entry: value extends beyond file boundary".to_string(),
            ));
        }

        let value = data[pos..pos + value_len].to_vec();
        pos += value_len;

        if pos + 4 > file_size {
            return Err(FatalError::Corruption(
                "Invalid entry: not enough data for checksum".to_string(),
            ));
        }

        let checksum = u32::from_le_bytes(
            data[pos..pos + 4]
                .try_into()
                .map_err(|e| FatalError::Corruption(format!("Invalid checksum bytes: {}", e)))?,
        );

        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(key.as_bytes());
        hasher.write(&value);
        let computed = hasher.finish() as u32;
        if checksum != computed {
            return Err(FatalError::Corruption(format!(
                "Checksum mismatch at offset {}: expected {:08X}, got {:08X}",
                offset, checksum, computed
            )));
        }

        Ok((key, value, checksum))
    }

    /// 从指定偏移开始扫描查找 key
    ///
    /// # PERF-002: Uses persistent mmap (no re-mapping)
    /// # P1-006 FIX: Safety measures
    /// - Validates start_offset before accessing mmap
    /// - All slice accesses use try_into() with match (no panic)
    /// - Scans at most 1000 entries to prevent infinite loops
    /// - Validates file size before each access
    pub fn scan_from(&self, start_offset: u64, target_key: &str) -> Result<ScanResult, FatalError> {
        self.flush()?;

        // RES-001: Use ArcSwapOption load() for lock-free access
        let mmap_guard = self.mmap.load();
        let mmap = match &*mmap_guard {
            Some(m) => m,
            None => {
                return Ok(None); // Empty file
            }
        };

        let file_size = mmap.len();
        let data = mmap.as_slice();
        let start_pos = start_offset as usize;

        if start_pos >= file_size {
            // Start offset beyond file - nothing to scan
            return Ok(None);
        }

        let mut pos = start_pos;
        let max_entries = 1000;
        let mut entries_scanned = 0;

        // P1-006 FIX: All bounds checking uses explicit comparisons
        while pos + 4 <= file_size && entries_scanned < max_entries {
            let key_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + key_len > file_size {
                break;
            }

            let key = String::from_utf8_lossy(&data[pos..pos + key_len]).to_string();
            pos += key_len;

            if pos + 4 > file_size {
                break;
            }

            let value_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + value_len + 4 > file_size {
                break;
            }

            let value = data[pos..pos + value_len].to_vec();
            pos += value_len;

            let checksum = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf),
                Err(_) => break,
            };
            pos += 4;

            entries_scanned += 1;

            if key == target_key {
                let mut hasher = crc32c::Crc32cHasher::default();
                hasher.write(key.as_bytes());
                hasher.write(&value);
                let computed = hasher.finish() as u32;
                if checksum == computed {
                    return Ok(Some((key, value, start_offset, checksum)));
                }
            }

            if key.as_str() > target_key {
                break;
            }
        }

        Ok(None)
    }

    /// 从指定偏移开始扫描返回下一个条目
    ///
    /// Used for range scan iteration - returns the next entry from start_offset
    /// regardless of key value (as long as it's within the segment).
    ///
    /// # Arguments
    /// * `start_offset` - Offset to start scanning from
    /// * `min_key` - Minimum key to return (entries with key < min_key are skipped)
    ///
    /// # Returns
    /// Some((key, value, offset, checksum)) if an entry is found, None otherwise
    ///
    /// ARCH-004: Added max_entries parameter to avoid hard-coded limit
    /// If max_entries is None, uses a default limit of 1000 for safety.
    pub fn scan_next(
        &self,
        start_offset: u64,
        min_key: &str,
        max_entries: Option<usize>,
    ) -> Result<ScanResult, FatalError> {
        self.flush()?;

        // RES-001: Use ArcSwapOption load() for lock-free access
        let mmap_guard = self.mmap.load();
        let mmap = match &*mmap_guard {
            Some(m) => m,
            None => {
                return Ok(None); // Empty file
            }
        };

        let file_size = mmap.len();
        let data = mmap.as_slice();
        let start_pos = start_offset as usize;

        if start_pos >= file_size {
            return Ok(None);
        }

        let mut pos = start_pos;
        let max_entries = max_entries.unwrap_or(1000); // ARCH-004: Configurable limit with safe default
        let mut entries_scanned = 0;

        while pos + 4 <= file_size && entries_scanned < max_entries {
            let entry_start = pos; // Record entry start position

            let key_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + key_len > file_size {
                break;
            }

            // PERF-003: Use String::from_utf8 directly instead of from_utf8_lossy + to_string
            // This avoids creating an intermediate Cow<str>
            let key_bytes = &data[pos..pos + key_len];
            let key = match String::from_utf8(key_bytes.to_vec()) {
                Ok(s) => s,
                Err(_) => {
                    // Invalid UTF-8, skip this entry
                    pos += key_len;
                    if pos + 4 > file_size {
                        break;
                    }
                    let value_len = match data[pos..pos + 4].try_into() {
                        Ok(buf) => u32::from_le_bytes(buf) as usize,
                        Err(_) => break,
                    };
                    pos += 4 + value_len + 4;
                    entries_scanned += 1;
                    continue;
                }
            };
            pos += key_len;

            if pos + 4 > file_size {
                break;
            }

            let value_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + value_len + 4 > file_size {
                break;
            }

            let value = data[pos..pos + value_len].to_vec();
            pos += value_len;

            let checksum = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf),
                Err(_) => break,
            };
            pos += 4;

            entries_scanned += 1;

            // Skip entries with key < min_key
            if key.as_str() < min_key {
                continue;
            }

            // Validate checksum and return
            let mut hasher = crc32c::Crc32cHasher::default();
            hasher.write(key.as_bytes());
            hasher.write(&value);
            let computed = hasher.finish() as u32;
            if checksum == computed {
                return Ok(Some((key, value, entry_start as u64, checksum)));
            }
        }

        Ok(None)
    }

    /// 获取文件大小
    pub fn size(&self) -> u64 {
        self.size.load(Ordering::Relaxed)
    }

    /// Read all segment data (excluding header) for streaming iteration
    ///
    /// Used by SegmentIterator during compaction to get a snapshot of
    /// the segment data without holding the mmap lock during iteration.
    ///
    /// # Returns
    /// Raw bytes of the segment file (including header)
    pub fn read_segment_data(&self) -> Result<Vec<u8>, FatalError> {
        // RES-001: Use ArcSwapOption load() for lock-free access
        let mmap_guard = self.mmap.load();
        if let Some(m) = &*mmap_guard {
            let data = m.as_slice();
            return Ok(data.to_vec());
        }

        // Fallback: read from file directly if mmap not available
        let file_size = self.fs.file_metadata(&self.path).map(|m| m.len).unwrap_or(0) as usize;
        if file_size <= 8 {
            return Ok(Vec::new());
        }

        // Use read_at to read the entire file in chunks
        let mut result = Vec::with_capacity(file_size);
        let chunk_size = 4096;
        let mut offset = 0u64;
        while offset < file_size as u64 {
            let remaining = (file_size as u64 - offset).min(chunk_size as u64) as u32;
            let chunk = self.read_at(offset, remaining)?;
            result.extend_from_slice(&chunk);
            offset += chunk.len() as u64;
        }
        Ok(result)
    }

    /// Update file size (used after flush_memtable writes data)
    pub fn update_size(&self, new_size: u64) {
        self.size.store(new_size, Ordering::Relaxed);
    }

    /// 获取条目数
    pub fn entry_count(&self) -> u64 {
        self.entry_count.load(Ordering::Relaxed)
    }

    /// 4.1 OPTIMIZATION: Get dense index memory usage in bytes
    pub fn dense_index_memory_bytes(&self) -> Option<u64> {
        self.dense_index.as_ref().map(|di| {
            let guard = di.read();
            // Approximate: each entry has key (String ~ 32 bytes avg) + DenseIndexEntry (~32 bytes)
            (guard.len() * 64) as u64
        })
    }

    /// 4.1 OPTIMIZATION: Check if persistent mmap is enabled
    pub fn use_persistent_mmap(&self) -> bool {
        self.use_persistent_mmap
    }

    /// 刷新到磁盘
    ///
    /// PERF-002: Refreshes mmap after flush to make new data visible
    pub fn flush(&self) -> Result<(), FatalError> {
        let mut file = self.write_file.lock();
        file.flush()?;
        drop(file); // Release lock before refreshing mmap

        // PERF-002: Refresh mmap to make new data visible to readers
        self.refresh_mmap()?;

        Ok(())
    }

    /// 关闭 segment
    pub fn close(&self) -> Result<(), FatalError> {
        self.flush()?;
        Ok(())
    }

    /// 遍历 segment 中所有条目
    ///
    /// # Arguments
    /// * `f` - 回调函数，对每个条目调用 (key, value, deleted)
    ///
    /// # Returns
    /// * `Ok(())` - 遍历成功
    /// * `Err(ContextError)` - 遍历失败
    ///
    /// # Note
    /// 此方法用于 checkpoint 创建时收集所有已 flush 的数据
    pub fn iterate_all<F>(&self, mut f: F) -> Result<(), FatalError>
    where
        F: FnMut(&str, &[u8], bool) -> Result<(), FatalError>,
    {
        self.flush()?;

        // RES-001: Use ArcSwapOption load() for lock-free access
        let mmap_guard = self.mmap.load();
        let mmap = match &*mmap_guard {
            Some(m) => m,
            None => {
                return Ok(()); // Empty file, nothing to iterate
            }
        };

        let file_size = mmap.len();
        let data = mmap.as_slice();
        let mut pos = 8usize; // Skip header (magic + version)

        // P1-006 FIX: All bounds checking uses explicit comparisons
        while pos + 4 <= file_size {
            let key_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + key_len > file_size {
                break;
            }

            let key = String::from_utf8_lossy(&data[pos..pos + key_len]).to_string();
            pos += key_len;

            if pos + 4 > file_size {
                break;
            }

            let value_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + value_len > file_size {
                break;
            }

            let value = &data[pos..pos + value_len];
            pos += value_len;

            if pos + 4 > file_size {
                break;
            }

            let _checksum = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf),
                Err(_) => break,
            };
            pos += 4;

            // For checkpoint, we don't have delete markers in segment files
            // All entries in segments are considered non-deleted
            // (deletes are tracked in MemTable with tombstones)
            f(&key, value, false)?;
        }

        Ok(())
    }

    /// Iterate over all entries in the segment file
    ///
    /// # Arguments
    /// * `f` - Callback function called for each entry with (key, value, deleted)
    ///
    /// # Returns
    /// * `Ok(())` - Iteration successful
    /// * `Err(ContextError)` - Iteration failed
    pub fn iterate_entries<F>(&self, f: F) -> Result<(), FatalError>
    where
        F: FnMut(&str, &[u8], bool) -> Result<(), FatalError>,
    {
        self.iterate_all(f)
    }

    /// Iterate over all entries in the segment file, providing byte offsets.
    ///
    /// Like `iterate_all()`, but the callback also receives the entry's byte offset
    /// within the segment file (the position of the key_length field).
    ///
    /// # Arguments
    /// * `f` - Callback function called for each entry with (key, value, offset, deleted)
    ///
    /// # Returns
    /// * `Ok(())` - Iteration successful
    /// * `Err(FatalError)` - Iteration failed
    pub fn iterate_all_with_offset<F>(&self, mut f: F) -> Result<(), FatalError>
    where
        F: FnMut(&str, &[u8], u64, bool) -> Result<(), FatalError>,
    {
        self.flush()?;

        let mmap_guard = self.mmap.load();
        let mmap = match &*mmap_guard {
            Some(m) => m,
            None => {
                return Ok(()); // Empty file, nothing to iterate
            }
        };

        let file_size = mmap.len();
        let data = mmap.as_slice();
        let mut pos = 8usize; // Skip header (magic + version)

        while pos + 4 <= file_size {
            let entry_offset = pos as u64; // Record entry start position

            let key_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + key_len > file_size {
                break;
            }

            let key = String::from_utf8_lossy(&data[pos..pos + key_len]).to_string();
            pos += key_len;

            if pos + 4 > file_size {
                break;
            }

            let value_len = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + value_len > file_size {
                break;
            }

            let value = &data[pos..pos + value_len];
            pos += value_len;

            if pos + 4 > file_size {
                break;
            }

            let _checksum = match data[pos..pos + 4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf),
                Err(_) => break,
            };
            pos += 4;

            f(&key, value, entry_offset, false)?;
        }

        Ok(())
    }
}

/// Holds an Arc<dyn MmapView> and a slice range, implementing Deref<Target=[u8]>
/// so that Bytes::from_owner can use it as a zero-copy owner.
struct MmapSliceOwner {
    mmap: Arc<dyn MmapView>,
    offset: usize,
    len: usize,
}

impl std::ops::Deref for MmapSliceOwner {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        &self.mmap.as_slice()[self.offset..self.offset + self.len]
    }
}

impl AsRef<[u8]> for MmapSliceOwner {
    fn as_ref(&self) -> &[u8] {
        &self.mmap.as_slice()[self.offset..self.offset + self.len]
    }
}

/// 段统计信息
#[derive(Debug, Clone)]
pub struct SegmentStats {
    pub id: u64,
    pub size_bytes: u64,
    pub entry_count: u64,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::StdFs;
    use std::thread;
    use tempfile::TempDir;

    fn test_fs() -> Arc<dyn FileKVFileSystem> {
        Arc::new(StdFs)
    }

    #[test]
    fn test_segment_file_append_read() {
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_0001.log");

        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap();

        let (offset, len, checksum) = segment.append("key1", b"value1").unwrap();

        assert!(offset > 0);
        assert_eq!(len, 6);
        assert!(checksum > 0);

        let value = segment.read_at(offset, len).unwrap();
        assert_eq!(value, b"value1");

        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(b"key1");
        hasher.write(b"value1");
        let computed = hasher.finish() as u32;
        assert_eq!(checksum, computed);
    }

    #[test]
    fn test_segment_file_read_entry() {
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_0001.log");

        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap();
        let (offset, _, _) = segment.append("test_key", b"test_value").unwrap();

        let (key, value, checksum) = segment.read_entry(offset).unwrap();

        assert_eq!(key, "test_key");
        assert_eq!(value, b"test_value");

        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(b"test_key");
        hasher.write(b"test_value");
        let expected = hasher.finish() as u32;
        assert_eq!(checksum, expected);
    }

    // ========================================================================
    // P1-006: Mmap Safety Tests
    // ========================================================================

    #[test]
    fn test_segment_mmap_safety_empty_file() {
        // P1-006: Test that opening an empty segment file is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_empty.log");

        // Create empty file (no header)
        let fs = test_fs();
        fs.create_file(&segment_path).unwrap();

        // Opening an empty file is allowed (size=0, mmap=None)
        // But reading from it should fail
        let result = SegmentFile::open(test_fs(), 999, 0, &segment_path, true, 0, false);
        assert!(result.is_ok()); // Empty file opens successfully with no mmap

        // Verify the segment has size 0
        let segment = result.unwrap();
        assert_eq!(segment.size(), 0);
    }

    #[test]
    fn test_segment_mmap_safety_truncated_file() {
        // P1-006: Test that opening a truncated segment file is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_truncated.log");

        // Create file with partial header (only 4 bytes instead of 8)
        let fs = test_fs();
        let mut file = fs.create_file(&segment_path).unwrap();
        file.write_all(&SEGMENT_MAGIC.to_le_bytes()).unwrap();
        drop(file);

        // Opening should fail with appropriate error
        let result = SegmentFile::open(test_fs(), 999, 0, &segment_path, true, 0, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("too small"));
    }

    #[test]
    fn test_segment_mmap_safety_invalid_magic() {
        // P1-006: Test that opening a file with invalid magic is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_invalid.log");

        // Create file with wrong magic
        let fs = test_fs();
        let mut file = fs.create_file(&segment_path).unwrap();
        file.write_all(&0xDEADBEEFu32.to_le_bytes()).unwrap();
        file.write_all(&SEGMENT_VERSION.to_le_bytes()).unwrap();
        drop(file);

        // Opening should fail with appropriate error
        let result = SegmentFile::open(test_fs(), 999, 0, &segment_path, true, 0, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid segment file magic"));
    }

    #[test]
    fn test_segment_mmap_safety_unsupported_version() {
        // P1-006: Test that opening a file with unsupported version is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_version.log");

        // Create file with unsupported version
        let fs = test_fs();
        let mut file = fs.create_file(&segment_path).unwrap();
        file.write_all(&SEGMENT_MAGIC.to_le_bytes()).unwrap();
        file.write_all(&99u32.to_le_bytes()).unwrap(); // Version 99
        drop(file);

        // Opening should fail with appropriate error
        let result = SegmentFile::open(test_fs(), 999, 0, &segment_path, true, 0, false);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unsupported segment version"));
    }

    #[test]
    fn test_segment_read_at_out_of_bounds() {
        // P1-006: Test that reading beyond file bounds is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_bounds.log");

        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap();
        let (offset, _, _) = segment.append("key1", b"value1").unwrap();

        // Try to read beyond file size
        let result = segment.read_at(offset + 10000, 100);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("out of bounds"));
    }

    #[test]
    fn test_segment_read_entry_corrupted_data() {
        // P1-006: Test that reading corrupted data is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_corrupt.log");

        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap();
        let (offset, _, _) = segment.append("key1", b"value1").unwrap();

        // Corrupt the file by truncating it in the middle of the entry
        drop(segment);
        let fs = test_fs();
        let file = fs.open_file(&segment_path, true, true, false).unwrap();
        // Truncate to partial entry data (keep header but corrupt entry)
        file.metadata().unwrap();
        // Note: StdFile doesn't expose set_len directly through the trait.
        // For this test, we'll use std::fs to truncate
        std::fs::OpenOptions::new()
            .write(true)
            .open(&segment_path)
            .unwrap()
            .set_len(offset + 5)
            .unwrap();

        // Reopen - may fail if file is too corrupted, or succeed but read fails
        let result = SegmentFile::open(test_fs(), 1, 0, &segment_path, true, 2, true);
        if let Ok(segment2) = result {
            // If open succeeds, reading should fail due to corrupted/incomplete data
            let read_result = segment2.read_entry(offset);
            assert!(read_result.is_err());
        }
        // If open fails (file too small/corrupted), that's also acceptable
    }

    #[test]
    fn test_segment_concurrent_read_write() {
        // P1-006: Test that concurrent reads work correctly with multiple readers
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_concurrent.log");

        let segment = Arc::new(SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap());

        // Write initial data
        let (offset1, _, _) = segment.append("key1", b"value1").unwrap();
        let (offset2, _, _) = segment.append("key2", b"value2").unwrap();
        segment.flush().unwrap(); // Ensure data is flushed to disk

        let segment_clone = segment.clone();

        // Spawn multiple reader threads
        let mut handles = vec![];
        for _ in 0..5 {
            let seg = segment_clone.clone();
            let off1 = offset1;
            let off2 = offset2;
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    // Read should succeed consistently
                    let v1 = seg.read_at(off1, 6).unwrap();
                    let v2 = seg.read_at(off2, 6).unwrap();
                    assert_eq!(v1, b"value1");
                    assert_eq!(v2, b"value2");
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        // Verify final state
        let value = segment.read_at(offset1, 6).unwrap();
        assert_eq!(value, b"value1");
    }

    #[test]
    fn test_segment_scan_from_bounds() {
        // P1-006: Test that scanning from invalid offset is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_scan.log");

        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap();
        segment.append("key1", b"value1").unwrap();

        // Scan from beyond file size
        let result = segment.scan_from(10000, "key1");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_segment_mmap_multiple_readers() {
        // P1-006: Test that multiple concurrent readers work correctly
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_multi.log");

        let segment = Arc::new(SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap());
        let (offset1, _, _) = segment.append("key1", b"value1").unwrap();
        let (offset2, _, _) = segment.append("key2", b"value2").unwrap();

        let mut handles = vec![];
        for _i in 0..10 {
            let seg = segment.clone();
            let off1 = offset1;
            let off2 = offset2;
            handles.push(thread::spawn(move || {
                for _ in 0..10 {
                    let v1 = seg.read_at(off1, 6).unwrap();
                    let v2 = seg.read_at(off2, 6).unwrap();
                    assert_eq!(v1, b"value1");
                    assert_eq!(v2, b"value2");
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    // ========================================================================
    // P4-001: Aggressive Config Tests
    // ========================================================================

    #[test]
    fn test_segment_persistent_mmap_disabled() {
        // P4-001: Test that segment works with persistent mmap disabled
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_no_mmap.log");

        // Create with persistent_mmap = false
        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, false, 0, false).unwrap();
        let (offset, len, _checksum) = segment.append("key1", b"value1").unwrap();
        segment.flush().unwrap();

        // Reopen with persistent_mmap = false
        let segment2 = SegmentFile::open(test_fs(), 1, 0, &segment_path, false, 0, false).unwrap();

        // Read should still work (uses temporary mmap)
        let value = segment2.read_at(offset, len).unwrap();
        assert_eq!(value, b"value1");
    }

    #[test]
    fn test_segment_read_with_readahead() {
        // P4-001: Test readahead mechanism
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_readahead.log");

        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap();

        // Write multiple entries
        let (off1, _, _) = segment.append("key1", b"value1").unwrap();
        let (_, _, _) = segment.append("key2", b"value2").unwrap();
        let (_, _, _) = segment.append("key3", b"value3").unwrap();
        segment.flush().unwrap();

        // Read with readahead
        let (value, _readahead) = segment.read_at_with_readahead(off1, 4, 6, 2).unwrap();

        assert_eq!(value, b"value1");
        // Readahead may or may not succeed depending on entry layout estimation
        // Just verify it doesn't crash
    }

    #[test]
    fn test_segment_readahead_disabled() {
        // P4-001: Test that readahead=0 returns empty vector
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_no_readahead.log");

        let segment = SegmentFile::create(test_fs(), 1, 0, &segment_path, 0, true, 2, true).unwrap();
        let (offset, _, _) = segment.append("key1", b"value1").unwrap();
        segment.flush().unwrap();

        let (value, readahead) = segment.read_at_with_readahead(offset, 4, 6, 0).unwrap();

        assert_eq!(value, b"value1");
        assert!(readahead.is_empty());
    }
}
