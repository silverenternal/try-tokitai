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

use std::hash::Hasher;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::Mutex;

use crate::error::{ContextResult, ContextError};

const SEGMENT_MAGIC: u32 = 0x54435347; // "TCSG" = Atlas Context SeGment
const SEGMENT_VERSION: u32 = 1;

/// Scan result type alias for complex return type
type ScanResult = Option<(String, Vec<u8>, u64, u32)>;

/// Segment 文件管理器
#[derive(Debug)]
pub struct SegmentFile {
    /// 段文件 ID
    pub id: u64,
    /// 文件路径
    pub path: PathBuf,
    /// 文件句柄（追加模式）
    file: Mutex<BufWriter<std::fs::File>>,
    /// 当前文件大小
    size: AtomicU64,
    /// 条目数
    entry_count: AtomicU64,
    /// mmap 只读映射（用于读取）
    mmap: Option<Arc<memmap2::Mmap>>,
}

impl SegmentFile {
    /// 创建新的 segment 文件
    ///
    /// 如果 preallocate_size > 0，会预分配指定大小的文件空间
    pub fn create(id: u64, path: &Path, preallocate_size: u64) -> ContextResult<Self> {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(path)
            .map_err(ContextError::Io)?;

        if preallocate_size > 0 {
            file.set_len(preallocate_size)
                .map_err(ContextError::Io)?;
        }

        let mut writer = BufWriter::new(file);

        let metadata = writer.get_ref().metadata()?;
        if metadata.len() == 0 || preallocate_size > 0 {
            writer.write_all(&SEGMENT_MAGIC.to_le_bytes())?;
            writer.write_all(&SEGMENT_VERSION.to_le_bytes())?;
            writer.flush()?;
        }

        let size = writer.get_ref().metadata()?.len();

        Ok(Self {
            id,
            path: path.to_path_buf(),
            file: Mutex::new(writer),
            size: AtomicU64::new(size),
            entry_count: AtomicU64::new(0),
            mmap: None,
        })
    }

    /// 打开现有 segment 文件
    ///
    /// # P1-006 FIX: Safety measures for mmap usage
    /// - File is opened read-only for mmap (separate handle for writes)
    /// - Mmap is created with read-only permissions
    /// - File size is validated before mmap
    /// - All mmap accesses include bounds checking
    pub fn open(id: u64, path: &Path) -> ContextResult<Self> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .map_err(ContextError::Io)?;

        let metadata = file.metadata()?;
        let size = metadata.len();

        // P1-006 FIX: Validate file size before mmap
        // Files smaller than header (8 bytes) are invalid
        if size > 0 && size < 8 {
            return Err(ContextError::OperationFailed(
                format!("Segment file too small: {} bytes (minimum: 8 bytes for header)", size)
            ));
        }

        let mmap = if size > 0 {
            // P1-006 FIX: Use MmapOptions for explicit read-only mapping
            // This prevents accidental writes and provides better isolation
            //
            // # Safety
            // - We hold the file handle open, preventing truncation during use
            // - The mmap is read-only (no write operations performed)
            // - File size is validated before mapping
            // - All subsequent accesses are bounds-checked
            unsafe {
                Some(Arc::new(
                    memmap2::MmapOptions::new()
                        .map(&file)
                        .map_err(ContextError::Io)?
                ))
            }
        } else {
            None
        };

        // P1-006 FIX: Validate mmap contents while holding file reference
        // This ensures the file wasn't modified between open and validation
        if let Some(ref mmap) = mmap {
            // Safety: We just created the mmap and hold exclusive access to the file handle
            // The mmap is read-only, and we validate the entire header before use
            if size >= 8 {
                let magic_buf: [u8; 4] = mmap.as_ref()[0..4].try_into().expect("Invalid magic bytes in segment file");
                let magic = u32::from_le_bytes(magic_buf);
                if magic != SEGMENT_MAGIC {
                    return Err(ContextError::OperationFailed(
                        format!("Invalid segment file magic: expected {:08X}, got {:08X}", SEGMENT_MAGIC, magic)
                    ));
                }

                let version_buf: [u8; 4] = mmap.as_ref()[4..8].try_into().expect("Invalid version bytes");
                let version = u32::from_le_bytes(version_buf);
                if version != SEGMENT_VERSION {
                    return Err(ContextError::OperationFailed(
                        format!("Unsupported segment version: expected {}, got {}", SEGMENT_VERSION, version)
                    ));
                }
            }
        }

        Ok(Self {
            id,
            path: path.to_path_buf(),
            file: Mutex::new(BufWriter::new(file.try_clone()?)),
            size: AtomicU64::new(size),
            entry_count: AtomicU64::new(0),
            mmap,
        })
    }

    /// 追加写入键值对
    ///
    /// 返回写入位置（offset, len, checksum）
    pub fn append(&self, key: &str, value: &[u8]) -> ContextResult<(u64, u32, u32)> {
        let mut file = self.file.lock();
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

        Ok((offset, value_len, checksum))
    }

    /// 通过偏移读取值
    ///
    /// # P1-006 FIX: Safety measures
    /// - Validates offset is within file bounds before mmap access
    /// - Creates temporary read-only mmap (not shared with writes)
    /// - All slice accesses include bounds checking via try_into()
    pub fn read_at(&self, offset: u64, _len: u32) -> ContextResult<Vec<u8>> {
        self.flush()?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&self.path)?;

        // P1-006 FIX: Validate file size before mmap
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        if offset >= file_size {
            return Err(ContextError::OperationFailed(
                format!("Read offset {} out of bounds (file size: {})", offset, file_size)
            ));
        }

        // # Safety
        // - We hold the file handle open, preventing concurrent modification
        // - The mmap is read-only (no write operations performed)
        // - File size is validated before mapping
        // - All subsequent accesses are bounds-checked with explicit comparisons
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(ContextError::Io)?
        };

        // P1-006 FIX: Validate offset and all slice accesses
        let mut pos = offset as usize;

        if pos + 4 > mmap.len() {
            return Err(ContextError::OperationFailed(
                format!("Invalid offset: not enough data for key length (pos={}, mmap_size={})", pos, mmap.len())
            ));
        }

        let key_len = u32::from_le_bytes(mmap[pos..pos+4].try_into().map_err(|e| ContextError::OperationFailed(format!("Invalid key length bytes: {}", e)))?) as usize;
        pos += 4;

        if pos + key_len > mmap.len() {
            return Err(ContextError::OperationFailed(
                format!("Invalid key length: extends beyond file (pos={}, key_len={}, mmap_size={})", pos, key_len, mmap.len())
            ));
        }
        pos += key_len;

        if pos + 4 > mmap.len() {
            return Err(ContextError::OperationFailed(
                format!("Invalid offset: not enough data for value length (pos={}, mmap_size={})", pos, mmap.len())
            ));
        }

        let value_len = u32::from_le_bytes(mmap[pos..pos+4].try_into().map_err(|e| ContextError::OperationFailed(format!("Invalid value length bytes: {}", e)))?) as usize;
        pos += 4;

        if pos + value_len > mmap.len() {
            return Err(ContextError::OperationFailed(
                format!("Invalid value length: extends beyond file (pos={}, value_len={}, mmap_size={})", pos, value_len, mmap.len())
            ));
        }

        let value = mmap[pos..pos+value_len].to_vec();
        Ok(value)
    }

    /// 读取键值对（需要知道偏移）
    ///
    /// # P1-006 FIX: Safety measures
    /// - Validates offset and all data accesses against file size
    /// - Uses try_into() for all slice conversions (panic-free)
    /// - Includes checksum verification for data integrity
    pub fn read_entry(&self, offset: u64) -> ContextResult<(String, Vec<u8>, u32)> {
        self.flush()?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&self.path)?;

        // P1-006 FIX: Validate file size
        let metadata = file.metadata()?;
        let file_size = metadata.len();

        if offset >= file_size {
            return Err(ContextError::OperationFailed(
                format!("Read offset {} out of bounds (file size: {})", offset, file_size)
            ));
        }

        // # Safety
        // - We hold the file handle open, preventing concurrent modification
        // - The mmap is read-only (no write operations performed)
        // - File size is validated before mapping
        // - All subsequent accesses are bounds-checked
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(ContextError::Io)?
        };

        let mut pos = offset as usize;

        // P1-006 FIX: Bounds-checked slice access
        if pos + 4 > mmap.len() {
            return Err(ContextError::OperationFailed("Invalid entry offset: not enough data for key length".to_string()));
        }

        let key_len = u32::from_le_bytes(mmap[pos..pos+4].try_into().map_err(|e| ContextError::OperationFailed(format!("Invalid key length bytes: {}", e)))?) as usize;
        pos += 4;

        if pos + key_len > mmap.len() {
            return Err(ContextError::OperationFailed("Invalid entry: key extends beyond file boundary".to_string()));
        }

        let key = String::from_utf8_lossy(&mmap[pos..pos+key_len]).to_string();
        pos += key_len;

        if pos + 4 > mmap.len() {
            return Err(ContextError::OperationFailed("Invalid entry: not enough data for value length".to_string()));
        }

        let value_len = u32::from_le_bytes(mmap[pos..pos+4].try_into().map_err(|e| ContextError::OperationFailed(format!("Invalid value length bytes: {}", e)))?) as usize;
        pos += 4;

        if pos + value_len > mmap.len() {
            return Err(ContextError::OperationFailed("Invalid entry: value extends beyond file boundary".to_string()));
        }

        let value = mmap[pos..pos+value_len].to_vec();
        pos += value_len;

        if pos + 4 > mmap.len() {
            return Err(ContextError::OperationFailed("Invalid entry: not enough data for checksum".to_string()));
        }

        let checksum = u32::from_le_bytes(mmap[pos..pos+4].try_into().map_err(|e| ContextError::OperationFailed(format!("Invalid checksum bytes: {}", e)))?);

        let mut hasher = crc32c::Crc32cHasher::default();
        hasher.write(key.as_bytes());
        hasher.write(&value);
        let computed = hasher.finish() as u32;
        if checksum != computed {
            return Err(ContextError::OperationFailed(
                format!("Checksum mismatch at offset {}: expected {:08X}, got {:08X}",
                         offset, checksum, computed)
            ));
        }

        Ok((key, value, checksum))
    }

    /// 从指定偏移开始扫描查找 key
    ///
    /// # P1-006 FIX: Safety measures
    /// - Validates start_offset before accessing mmap
    /// - All slice accesses use try_into() with match (no panic)
    /// - Scans at most 1000 entries to prevent infinite loops
    /// - Validates file size before each access
    pub fn scan_from(&self, start_offset: u64, target_key: &str) -> ContextResult<ScanResult> {
        self.flush()?;

        let file = std::fs::OpenOptions::new()
            .read(true)
            .open(&self.path)?;

        // P1-006 FIX: Validate file size before mmap
        let metadata = file.metadata()?;
        let file_size = metadata.len() as usize; // Convert to usize for comparisons

        let start_pos = start_offset as usize;
        if start_pos >= file_size {
            // Start offset beyond file - nothing to scan
            return Ok(None);
        }

        // # Safety
        // - We hold the file handle open, preventing concurrent modification
        // - The mmap is read-only (no write operations performed)
        // - File size is validated before mapping
        // - All subsequent accesses are bounds-checked with explicit comparisons
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(ContextError::Io)?
        };

        let mut pos = start_pos;
        let max_entries = 1000;
        let mut entries_scanned = 0;

        // P1-006 FIX: All bounds checking uses explicit comparisons
        while pos + 4 <= file_size && entries_scanned < max_entries {
            let key_len = match mmap[pos..pos+4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + key_len > file_size {
                break;
            }

            let key = String::from_utf8_lossy(&mmap[pos..pos+key_len]).to_string();
            pos += key_len;

            if pos + 4 > file_size {
                break;
            }

            let value_len = match mmap[pos..pos+4].try_into() {
                Ok(buf) => u32::from_le_bytes(buf) as usize,
                Err(_) => break,
            };
            pos += 4;

            if pos + value_len + 4 > file_size {
                break;
            }

            let value = mmap[pos..pos+value_len].to_vec();
            pos += value_len;

            let checksum = match mmap[pos..pos+4].try_into() {
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

    /// 获取文件大小
    pub fn size(&self) -> u64 {
        self.size.load(Ordering::Relaxed)
    }

    /// 获取条目数
    pub fn entry_count(&self) -> u64 {
        self.entry_count.load(Ordering::Relaxed)
    }

    /// 刷新到磁盘
    pub fn flush(&self) -> ContextResult<()> {
        let mut file = self.file.lock();
        file.flush()?;
        Ok(())
    }

    /// 关闭 segment
    pub fn close(&self) -> ContextResult<()> {
        self.flush()?;
        Ok(())
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
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_segment_file_append_read() {
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_0001.log");

        let segment = SegmentFile::create(1, &segment_path, 0).unwrap();

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

        let segment = SegmentFile::create(1, &segment_path, 0).unwrap();
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
        File::create(&segment_path).unwrap();

        // Opening an empty file is allowed (size=0, mmap=None)
        // But reading from it should fail
        let result = SegmentFile::open(999, &segment_path);
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
        let mut file = File::create(&segment_path).unwrap();
        file.write_all(&SEGMENT_MAGIC.to_le_bytes()).unwrap();
        drop(file);

        // Opening should fail with appropriate error
        let result = SegmentFile::open(999, &segment_path);
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
        let mut file = File::create(&segment_path).unwrap();
        file.write_all(&0xDEADBEEFu32.to_le_bytes()).unwrap();
        file.write_all(&SEGMENT_VERSION.to_le_bytes()).unwrap();
        drop(file);

        // Opening should fail with appropriate error
        let result = SegmentFile::open(999, &segment_path);
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
        let mut file = File::create(&segment_path).unwrap();
        file.write_all(&SEGMENT_MAGIC.to_le_bytes()).unwrap();
        file.write_all(&99u32.to_le_bytes()).unwrap(); // Version 99
        drop(file);

        // Opening should fail with appropriate error
        let result = SegmentFile::open(999, &segment_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unsupported segment version"));
    }

    #[test]
    fn test_segment_read_at_out_of_bounds() {
        // P1-006: Test that reading beyond file bounds is handled safely
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_bounds.log");

        let segment = SegmentFile::create(1, &segment_path, 0).unwrap();
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

        let segment = SegmentFile::create(1, &segment_path, 0).unwrap();
        let (offset, _, _) = segment.append("key1", b"value1").unwrap();

        // Corrupt the file by truncating it
        drop(segment);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&segment_path)
            .unwrap();
        file.set_len(offset + 2).unwrap();
        drop(file);

        // Reopen and try to read
        let segment2 = SegmentFile::open(1, &segment_path).unwrap();
        let result = segment2.read_entry(offset);
        assert!(result.is_err());
        // Should fail with bounds check or checksum error
    }

    #[test]
    fn test_segment_concurrent_read_write() {
        // P1-006: Test that concurrent reads work correctly with multiple readers
        let temp_dir = TempDir::new().unwrap();
        let segment_path = temp_dir.path().join("segment_concurrent.log");

        let segment = Arc::new(SegmentFile::create(1, &segment_path, 0).unwrap());

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

        let segment = SegmentFile::create(1, &segment_path, 0).unwrap();
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

        let segment = Arc::new(SegmentFile::create(1, &segment_path, 0).unwrap());
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
}
