//! 稀疏索引模块
//!
//! 实现高效的段内查找机制：
//! - 每隔 N 条记录建立一个索引点
//! - 索引常驻内存，支持 O(log N) 查找
//! - 二分查找定位索引点，然后顺序扫描

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Index manager error types
#[derive(Debug, Error)]
pub enum IndexError {
    /// Index for segment not found
    #[error("Index for segment {0} not found")]
    IndexNotFound(u64),

    /// Index file too small
    #[error("Index file too small: {0}")]
    IndexFileTooSmall(PathBuf),

    /// Invalid index file magic
    #[error("Invalid index file magic: {0}")]
    InvalidIndexMagic(PathBuf),

    /// Unsupported index file version
    #[error("Unsupported index file version: {0}")]
    UnsupportedIndexVersion(u32),

    /// Segment ID mismatch in index file
    #[error("Segment ID mismatch in index file")]
    SegmentIdMismatch,

    /// Index file truncated
    #[error("Index file truncated")]
    IndexFileTruncated,

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// TryFromInt error
    #[error("Integer conversion error: {0}")]
    TryFromInt(#[from] std::num::TryFromIntError),

    /// TryFromSlice error
    #[error("Slice conversion error: {0}")]
    TryFromSlice(#[from] std::array::TryFromSliceError),

    /// UTF-8 conversion error
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),
}

/// Result type alias for index operations
pub type Result<T> = std::result::Result<T, IndexError>;

/// 索引点配置
#[derive(Debug, Clone)]
pub struct SparseIndexConfig {
    /// 每隔多少条记录建立一个索引点
    pub index_interval: usize,
}

impl Default for SparseIndexConfig {
    fn default() -> Self {
        Self {
            index_interval: 100, // 每 100 条记录一个索引点
        }
    }
}

/// 索引点数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexPoint {
    /// key（用于二分查找）
    pub key: String,
    /// 在 segment 文件中的偏移量
    pub offset: u64,
    /// 序列号（用于版本控制）
    pub seq_num: u64,
}

/// 段索引（稀疏索引）
///
/// 格式：
/// ```text
/// Segment 文件：
/// [Entry 0] [Entry 1] ... [Entry 99] [Entry 100] ...
///                              ↑
///                         索引点 (key="xxx", offset=12345)
/// ```
pub struct SparseIndex {
    /// 索引点列表（按 key 排序）
    index_points: Vec<IndexPoint>,
    /// 对应的 segment ID
    segment_id: u64,
    /// 配置
    config: SparseIndexConfig,
    /// 当前条目数
    entry_count: usize,
    /// 最后一个索引点的 key（用于快速判断是否需要添加新索引点）
    last_indexed_key: Option<String>,
}

impl SparseIndex {
    /// 创建新的稀疏索引
    pub fn new(segment_id: u64, config: SparseIndexConfig) -> Self {
        Self {
            index_points: Vec::new(),
            segment_id,
            config,
            entry_count: 0,
            last_indexed_key: None,
        }
    }

    /// 添加索引点（如果达到间隔阈值）
    ///
    /// 调用者需要在写入 entry 后调用此方法
    pub fn maybe_add_index_point(&mut self, key: &str, offset: u64, seq_num: u64) {
        self.entry_count += 1;

        // 检查是否需要添加索引点
        if self.entry_count.is_multiple_of(self.config.index_interval) {
            // 确保 key 比最后一个索引点大（保持有序）
            let should_add = match &self.last_indexed_key {
                None => true,
                Some(last_key) => key > last_key.as_str(),
            };

            if should_add {
                self.index_points.push(IndexPoint {
                    key: key.to_string(),
                    offset,
                    seq_num,
                });
                self.last_indexed_key = Some(key.to_string());
            }
        }
    }

    /// 查找 key 的位置
    ///
    /// 返回：
    /// - `Some(index_point_idx, scan_start_offset)`: 找到索引点，从该位置开始扫描
    /// - `None`: 索引为空，需要从头扫描
    ///
    /// 时间复杂度：O(log N)
    pub fn find(&self, key: &str) -> Option<(usize, u64)> {
        if self.index_points.is_empty() {
            return None;
        }

        // 二分查找：找到最后一个 <= target key 的索引点
        let idx = match self.index_points.binary_search_by(|p| p.key.as_str().cmp(key)) {
            Ok(i) => i, // 精确匹配
            Err(i) => {
                if i == 0 {
                    // 所有索引点的 key 都比 target 大，需要从头扫描
                    return None;
                }
                // 返回前一个索引点
                i - 1
            }
        };

        let point = &self.index_points[idx];
        Some((idx, point.offset))
    }

    /// 获取索引点数量
    pub fn index_point_count(&self) -> usize {
        self.index_points.len()
    }

    /// 获取条目总数（估计值）
    pub fn entry_count(&self) -> usize {
        self.entry_count
    }

    /// 获取最后一个索引点的偏移量（用于知道扫描范围）
    pub fn last_offset(&self) -> Option<u64> {
        self.index_points.last().map(|p| p.offset)
    }

    /// 获取所有索引点（用于持久化）
    pub fn get_index_points(&self) -> &[IndexPoint] {
        &self.index_points
    }

    /// 从索引点重建（加载时调用）
    pub fn from_index_points(segment_id: u64, points: Vec<IndexPoint>, entry_count: usize) -> Self {
        let last_indexed_key = points.last().map(|p| p.key.clone());
        Self {
            index_points: points,
            segment_id,
            config: SparseIndexConfig::default(),
            entry_count,
            last_indexed_key,
        }
    }
}

/// 索引文件管理器
///
/// 负责索引的持久化和加载
pub struct IndexManager {
    /// 索引目录
    index_dir: PathBuf,
    /// 内存中的索引（segment_id → SparseIndex）
    indexes: BTreeMap<u64, SparseIndex>,
}

impl IndexManager {
    /// 创建索引管理器
    pub fn new(index_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(index_dir)
            .map_err(IndexError::Io)?;

        Ok(Self {
            index_dir: index_dir.to_path_buf(),
            indexes: BTreeMap::new(),
        })
    }

    /// 创建新 segment 的索引
    pub fn create_index(&mut self, segment_id: u64) -> &mut SparseIndex {
        use std::collections::btree_map::Entry;
        
        match self.indexes.entry(segment_id) {
            Entry::Vacant(e) => e.insert(SparseIndex::new(segment_id, SparseIndexConfig::default())),
            Entry::Occupied(e) => e.into_mut(),
        }
    }

    /// 获取索引
    pub fn get_index(&self, segment_id: u64) -> Option<&SparseIndex> {
        self.indexes.get(&segment_id)
    }

    /// 获取可变索引
    pub fn get_index_mut(&mut self, segment_id: u64) -> Option<&mut SparseIndex> {
        self.indexes.get_mut(&segment_id)
    }

    /// 插入索引（用于 compaction）
    pub(crate) fn insert_index(&mut self, segment_id: u64, index: SparseIndex) {
        self.indexes.insert(segment_id, index);
    }

    /// 移除索引（用于 compaction）
    pub(crate) fn remove_index(&mut self, segment_id: u64) -> Option<SparseIndex> {
        self.indexes.remove(&segment_id)
    }

    /// 获取所有索引
    pub fn all_indexes(&self) -> &BTreeMap<u64, SparseIndex> {
        &self.indexes
    }

    /// 保存索引到文件
    pub fn save_index(&self, segment_id: u64) -> Result<()> {
        if let Some(index) = self.indexes.get(&segment_id) {
            let index_path = self.index_dir.join(format!("index_{:06}.bin", segment_id));

            let mut points_data = Vec::new();
            for point in index.get_index_points() {
                // 序列化：key_len (u32) + key + offset (u64) + seq_num (u64)
                let key_bytes = point.key.as_bytes();
                points_data.extend_from_slice(&(key_bytes.len() as u32).to_le_bytes());
                points_data.extend_from_slice(key_bytes);
                points_data.extend_from_slice(&point.offset.to_le_bytes());
                points_data.extend_from_slice(&point.seq_num.to_le_bytes());
            }

            // 写入文件头 + 数据
            let mut file = BufWriter::new(
                File::create(&index_path)
                    .map_err(IndexError::Io)?
            );

            // 文件头：magic (4) + version (4) + segment_id (8) + entry_count (8) + index_point_count (8)
            file.write_all(b"TCIX")?; // Atlas Context IndeX
            file.write_all(&1u32.to_le_bytes())?; // version
            file.write_all(&segment_id.to_le_bytes())?;
            file.write_all(&(index.entry_count() as u64).to_le_bytes())?;
            file.write_all(&(index.index_point_count() as u64).to_le_bytes())?;

            // 索引点数据
            file.write_all(&points_data)?;
            file.flush()?;

            Ok(())
        } else {
            Err(IndexError::IndexNotFound(segment_id))
        }
    }

    /// 从文件加载索引
    pub fn load_index(&mut self, segment_id: u64) -> Result<Option<SparseIndex>> {
        let index_path = self.index_dir.join(format!("index_{:06}.bin", segment_id));

        if !index_path.exists() {
            return Ok(None);
        }

        let data = std::fs::read(&index_path)
            .map_err(IndexError::Io)?;

        if data.len() < 32 {
            return Err(IndexError::IndexFileTooSmall(index_path.clone()));
        }

        // 验证文件头
        if &data[0..4] != b"TCIX" {
            return Err(IndexError::InvalidIndexMagic(index_path.clone()));
        }

        let version = u32::from_le_bytes(data[4..8].try_into()?);
        if version != 1 {
            return Err(IndexError::UnsupportedIndexVersion(version));
        }

        let loaded_segment_id = u64::from_le_bytes(data[8..16].try_into()?);
        if loaded_segment_id != segment_id {
            return Err(IndexError::SegmentIdMismatch);
        }

        let entry_count = u64::from_le_bytes(data[16..24].try_into()?) as usize;
        let index_point_count = u64::from_le_bytes(data[24..32].try_into()?) as usize;

        // 解析索引点
        let mut points = Vec::with_capacity(index_point_count);
        let mut pos = 32;

        for _ in 0..index_point_count {
            if pos + 4 > data.len() {
                return Err(IndexError::IndexFileTruncated);
            }

            let key_len = u32::from_le_bytes(data[pos..pos+4].try_into()?) as usize;
            pos += 4;

            if pos + key_len > data.len() {
                return Err(IndexError::IndexFileTruncated);
            }

            let key = String::from_utf8_lossy(&data[pos..pos+key_len]).to_string();
            pos += key_len;

            if pos + 16 > data.len() {
                return Err(IndexError::IndexFileTruncated);
            }

            let offset = u64::from_le_bytes(data[pos..pos+8].try_into()?);
            pos += 8;

            let seq_num = u64::from_le_bytes(data[pos..pos+8].try_into()?);
            pos += 8;

            points.push(IndexPoint { key, offset, seq_num });
        }

        let index = SparseIndex::from_index_points(segment_id, points, entry_count);
        Ok(Some(index))
    }

    /// 加载所有索引
    pub fn load_all_indexes(&mut self) -> Result<usize> {
        let mut count = 0;

        for entry in std::fs::read_dir(&self.index_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("bin") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    if let Some(id_str) = name.strip_prefix("index_") {
                        if let Ok(id) = id_str.parse::<u64>() {
                            if let Some(index) = self.load_index(id)? {
                                self.indexes.insert(id, index);
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        Ok(count)
    }
}

/// 全局索引查找器
///
/// 整合所有 segment 的索引，支持跨 segment 查找
pub struct GlobalIndexFinder {
    index_manager: IndexManager,
}

impl GlobalIndexFinder {
    /// 创建全局查找器
    pub fn new(index_dir: &Path) -> Result<Self> {
        let mut index_manager = IndexManager::new(index_dir)?;
        index_manager.load_all_indexes()?;

        Ok(Self { index_manager })
    }

    /// 查找 key 在哪个 segment 的哪个位置
    ///
    /// 返回：`Some((segment_id, start_offset))`
    pub fn find_key(&self, key: &str) -> Option<(u64, u64)> {
        // 从后往前找（最新的 segment 更可能有数据）
        for (&segment_id, index) in self.index_manager.all_indexes().iter().rev() {
            if let Some((_idx, offset)) = index.find(key) {
                return Some((segment_id, offset));
            }
        }
        None
    }

    /// 获取索引管理器
    pub fn index_manager(&self) -> &IndexManager {
        &self.index_manager
    }

    /// 获取可变索引管理器
    pub fn index_manager_mut(&mut self) -> &mut IndexManager {
        &mut self.index_manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sparse_index_creation() {
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());

        // 添加 150 条记录（应该有 1 个索引点，在 100 条时创建）
        for i in 0..150 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }

        assert_eq!(index.entry_count(), 150);
        assert_eq!(index.index_point_count(), 1); // 第 100 条时创建
    }

    #[test]
    fn test_sparse_index_find() {
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());

        // 添加 250 条记录
        for i in 0..250 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }

        assert_eq!(index.index_point_count(), 2); // 第 100、200 条时创建

        // 查找 key_0050（在第一个索引点之前）
        let result = index.find("key_0050");
        assert!(result.is_none()); // 需要从头扫描

        // 查找 key_0150（在第一个索引点之后）
        // 第一个索引点在第 100 条（i=99）时创建，offset = 99 * 100 = 9900
        let result = index.find("key_0150");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(offset, 99 * 100); // 第 100 条（i=99）的偏移

        // 查找 key_0220（在第二个索引点之后）
        // 第二个索引点在第 200 条（i=199）时创建，offset = 199 * 100 = 19900
        let result = index.find("key_0220");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 1);
        assert_eq!(offset, 199 * 100); // 第 200 条（i=199）的偏移
    }

    #[test]
    fn test_index_manager_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = IndexManager::new(temp_dir.path()).unwrap();

        // 创建索引
        let index = manager.create_index(1);
        for i in 0..250 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }

        // 保存
        manager.save_index(1).unwrap();

        // 重新加载
        let mut manager2 = IndexManager::new(temp_dir.path()).unwrap();
        let loaded_index = manager2.load_index(1).unwrap().unwrap();

        assert_eq!(loaded_index.entry_count(), 250);
        assert_eq!(loaded_index.index_point_count(), 2);
    }

    #[test]
    fn test_global_index_finder() {
        let temp_dir = TempDir::new().unwrap();

        // 创建多个 segment 的索引
        let mut manager = IndexManager::new(temp_dir.path()).unwrap();

        for seg_id in 0..3 {
            let index = manager.create_index(seg_id);
            for i in 0..150 {
                let key = format!("key_{:04}", seg_id * 1000 + i);
                index.maybe_add_index_point(&key, i * 100, i);
            }
            manager.save_index(seg_id).unwrap();
        }

        // 创建查找器
        let finder = GlobalIndexFinder::new(temp_dir.path()).unwrap();

        // 查找不同 segment 的 key
        let result = finder.find_key("key_0050");
        assert!(result.is_none()); // 在 segment 0 的第一个索引点之前

        let result = finder.find_key("key_1150");
        assert!(result.is_some()); // 在 segment 1

        let result = finder.find_key("key_2200");
        assert!(result.is_some()); // 在 segment 2
    }

    // ========================================================================
    // P1-008: SparseIndex Boundary Condition Tests
    // ========================================================================

    #[test]
    fn test_sparse_index_empty() {
        // P1-008: Test empty index
        let index = SparseIndex::new(1, SparseIndexConfig::default());
        
        assert_eq!(index.entry_count(), 0);
        assert_eq!(index.index_point_count(), 0);
        assert_eq!(index.last_offset(), None);
        
        // Find on empty index should return None
        assert_eq!(index.find("any_key"), None);
    }

    #[test]
    fn test_sparse_index_single_entry() {
        // P1-008: Test single entry (below interval threshold)
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.maybe_add_index_point("key_0001", 100, 1);
        
        assert_eq!(index.entry_count(), 1);
        assert_eq!(index.index_point_count(), 0); // No index points yet (interval is 100)
        assert_eq!(index.last_offset(), None);
        
        // Find should return None (no index points)
        assert_eq!(index.find("key_0001"), None);
    }

    #[test]
    fn test_sparse_index_exact_interval_boundary() {
        // P1-008: Test exact interval boundary (100 entries)
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        for i in 0..100 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        assert_eq!(index.entry_count(), 100);
        assert_eq!(index.index_point_count(), 1); // Index point created at entry 100
        
        // The index point should be at entry 100 (i=99)
        let points = index.get_index_points();
        assert_eq!(points[0].key, "key_0099");
        assert_eq!(points[0].offset, 99 * 100);
    }

    #[test]
    fn test_sparse_index_all_keys_greater_than_target() {
        // P1-008: Test when all indexed keys are greater than search key
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        // Add entries starting from key_100
        for i in 100..200 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Search for a key smaller than all indexed keys
        let result = index.find("key_0050");
        assert!(result.is_none()); // Should scan from beginning
    }

    #[test]
    fn test_sparse_index_all_keys_less_than_target() {
        // P1-008: Test when all indexed keys are less than search key
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        for i in 0..100 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Search for a key larger than all indexed keys
        let result = index.find("key_9999");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0); // Last index point
        assert_eq!(offset, 99 * 100);
    }

    #[test]
    fn test_sparse_index_exact_match_first_element() {
        // P1-008: Test exact match on first indexed element
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        for i in 0..150 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Search for the first indexed key (key_0099)
        let result = index.find("key_0099");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0); // Exact match at index 0
        assert_eq!(offset, 99 * 100);
    }

    #[test]
    fn test_sparse_index_exact_match_last_element() {
        // P1-008: Test exact match on last indexed element
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        for i in 0..250 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Search for the last indexed key (key_0199)
        let result = index.find("key_0199");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 1); // Exact match at index 1 (second index point)
        assert_eq!(offset, 199 * 100);
    }

    #[test]
    fn test_sparse_index_between_index_points() {
        // P1-008: Test search between two index points
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        for i in 0..250 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Search for a key between first (key_0099) and second (key_0199) index points
        let result = index.find("key_0150");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0); // Should return first index point
        assert_eq!(offset, 99 * 100);
    }

    #[test]
    fn test_sparse_index_just_before_index_point() {
        // P1-008: Test search just before an index point
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        for i in 0..250 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Search for key just before second index point (key_0199)
        let result = index.find("key_0198");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0); // Should return first index point
        assert_eq!(offset, 99 * 100);
    }

    #[test]
    fn test_sparse_index_just_after_index_point() {
        // P1-008: Test search just after an index point
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        for i in 0..250 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Search for key just after first index point (key_0099)
        let result = index.find("key_0100");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0); // Should return first index point
        assert_eq!(offset, 99 * 100);
    }

    #[test]
    fn test_sparse_index_monotonic_key_order() {
        // P1-008: Test that keys must be monotonically increasing
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        
        // Add keys in order
        for i in 0..150 {
            let key = format!("key_{:04}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }
        
        // Try to add a key that's smaller than last indexed key
        // This simulates out-of-order writes
        index.maybe_add_index_point("key_0050", 99999, 999);
        
        // The index point count should not increase (key rejected)
        assert_eq!(index.index_point_count(), 1);
    }

    #[test]
    fn test_sparse_index_multiple_intervals() {
        // P1-008: Test with multiple interval boundaries
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());

        // Add 500 entries (should create index points at 100, 200, 300, 400, 500)
        for i in 0..500 {
            let key = format!("key_{:05}", i);
            index.maybe_add_index_point(&key, i as u64 * 100, i as u64);
        }

        assert_eq!(index.entry_count(), 500);
        assert_eq!(index.index_point_count(), 5);

        // Verify all index points
        let points = index.get_index_points();
        assert_eq!(points[0].key, "key_00099");
        assert_eq!(points[1].key, "key_00199");
        assert_eq!(points[2].key, "key_00299");
        assert_eq!(points[3].key, "key_00399");
        assert_eq!(points[4].key, "key_00499");
    }

    // ========================================================================
    // P1-008: Binary Search Boundary Condition Tests
    // ========================================================================
    // These tests verify the correctness of SparseIndex::find() at boundary
    // conditions to prevent off-by-one errors and incorrect binary search behavior.

    #[test]
    fn test_find_empty_index() {
        // P1-008 FIX: Empty index should return None
        let index = SparseIndex::new(1, SparseIndexConfig::default());
        assert!(index.find("any_key").is_none());
    }

    #[test]
    fn test_find_single_index_point_exact_match() {
        // P1-008: Single index point, exact match
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        // Directly add index point for precise testing
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });

        let result = index.find("key_050");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(offset, 5000);
    }

    #[test]
    fn test_find_single_index_point_key_smaller() {
        // P1-008: Single index point, search key is smaller
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });

        let result = index.find("key_010");
        // All index points are larger, should return None (scan from start)
        assert!(result.is_none());
    }

    #[test]
    fn test_find_single_index_point_key_larger() {
        // P1-008: Single index point, search key is larger
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });

        let result = index.find("key_100");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(offset, 5000);
    }

    #[test]
    fn test_find_two_index_points_exact_match_first() {
        // P1-008: Two index points, exact match on first
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });
        index.index_points.push(IndexPoint {
            key: "key_150".to_string(),
            offset: 15000,
            seq_num: 150,
        });

        let result = index.find("key_050");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(offset, 5000);
    }

    #[test]
    fn test_find_two_index_points_exact_match_last() {
        // P1-008: Two index points, exact match on last
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });
        index.index_points.push(IndexPoint {
            key: "key_150".to_string(),
            offset: 15000,
            seq_num: 150,
        });

        let result = index.find("key_150");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 1);
        assert_eq!(offset, 15000);
    }

    #[test]
    fn test_find_two_index_points_between() {
        // P1-008: Two index points, search key is between them
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });
        index.index_points.push(IndexPoint {
            key: "key_150".to_string(),
            offset: 15000,
            seq_num: 150,
        });

        let result = index.find("key_100");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0); // Should return first index point
        assert_eq!(offset, 5000);
    }

    #[test]
    fn test_find_two_index_points_smaller_than_all() {
        // P1-008: Two index points, search key is smaller than all
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });
        index.index_points.push(IndexPoint {
            key: "key_150".to_string(),
            offset: 15000,
            seq_num: 150,
        });

        let result = index.find("key_010");
        // All index points are larger, should return None
        assert!(result.is_none());
    }

    #[test]
    fn test_find_two_index_points_larger_than_all() {
        // P1-008: Two index points, search key is larger than all
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "key_050".to_string(),
            offset: 5000,
            seq_num: 50,
        });
        index.index_points.push(IndexPoint {
            key: "key_150".to_string(),
            offset: 15000,
            seq_num: 150,
        });

        let result = index.find("key_200");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 1); // Should return last index point
        assert_eq!(offset, 15000);
    }

    #[test]
    fn test_find_many_index_points_exact_match_middle() {
        // P1-008: Many index points, exact match in middle
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        for i in 0..5 {
            let key = format!("key_{:03}", i * 100 + 99);
            index.index_points.push(IndexPoint {
                key,
                offset: (i * 100 + 99) as u64 * 100,
                seq_num: i as u64,
            });
        }
        // Keys are: key_099, key_199, key_299, key_399, key_499

        let result = index.find("key_299");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 2); // Index point 2 (key_299)
        assert_eq!(offset, 29900);
    }

    #[test]
    fn test_find_many_index_points_between_index_points() {
        // P1-008: Many index points, search key is between two index points
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        for i in 0..5 {
            let key = format!("key_{:03}", i * 100 + 99);
            index.index_points.push(IndexPoint {
                key,
                offset: (i * 100 + 99) as u64 * 100,
                seq_num: i as u64,
            });
        }
        // Keys are: key_099, key_199, key_299, key_399, key_499

        let result = index.find("key_250");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        // key_250 is between key_199 (idx 1) and key_299 (idx 2)
        // Binary search returns insertion point 2, we subtract 1 to get index 1
        assert_eq!(idx, 1); // Returns index point 1 (key_199)
        assert_eq!(offset, 19900);
    }

    #[test]
    fn test_find_many_index_points_first_entry() {
        // P1-008: Many index points, search for first indexed key
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        for i in 0..5 {
            let key = format!("key_{:03}", i * 100 + 99);
            index.index_points.push(IndexPoint {
                key,
                offset: (i * 100 + 99) as u64 * 100,
                seq_num: i as u64,
            });
        }

        let result = index.find("key_099");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(offset, 9900);
    }

    #[test]
    fn test_find_many_index_points_last_entry() {
        // P1-008: Many index points, search for last indexed key
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        for i in 0..5 {
            let key = format!("key_{:03}", i * 100 + 99);
            index.index_points.push(IndexPoint {
                key,
                offset: (i * 100 + 99) as u64 * 100,
                seq_num: i as u64,
            });
        }

        let result = index.find("key_499");
        assert!(result.is_some());
        let (idx, offset) = result.unwrap();
        assert_eq!(idx, 4); // Last index point
        assert_eq!(offset, 49900);
    }

    #[test]
    fn test_find_many_index_points_before_first() {
        // P1-008: Many index points, search key is before first index point
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        for i in 0..5 {
            let key = format!("key_{:03}", i * 100 + 99);
            index.index_points.push(IndexPoint {
                key,
                offset: (i * 100 + 99) as u64 * 100,
                seq_num: i as u64,
            });
        }

        let result = index.find("key_050");
        // All index points are larger, should return None
        assert!(result.is_none());
    }

    #[test]
    fn test_find_boundary_key_values() {
        // P1-008: Test with boundary key values (empty string, very long keys)
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        index.index_points.push(IndexPoint {
            key: "a".to_string(),
            offset: 100,
            seq_num: 1,
        });
        index.index_points.push(IndexPoint {
            key: "m".to_string(),
            offset: 200,
            seq_num: 2,
        });
        index.index_points.push(IndexPoint {
            key: "z".to_string(),
            offset: 300,
            seq_num: 3,
        });

        // Empty string (should be before all)
        assert!(index.find("").is_none());

        // Single character matches
        assert_eq!(index.find("a").unwrap().0, 0);
        assert_eq!(index.find("m").unwrap().0, 1);
        assert_eq!(index.find("z").unwrap().0, 2);

        // Between characters
        assert_eq!(index.find("b").unwrap().0, 0); // Between 'a' and 'm'
        assert_eq!(index.find("n").unwrap().0, 1); // Between 'm' and 'z'
    }

    #[test]
    fn test_find_lexicographic_ordering() {
        // P1-008: Test that binary search uses lexicographic ordering correctly
        let mut index = SparseIndex::new(1, SparseIndexConfig::default());
        // Note: lexicographic order: "10" < "2" < "9"
        index.index_points.push(IndexPoint {
            key: "10".to_string(),
            offset: 100,
            seq_num: 1,
        });
        index.index_points.push(IndexPoint {
            key: "2".to_string(),
            offset: 200,
            seq_num: 2,
        });
        index.index_points.push(IndexPoint {
            key: "9".to_string(),
            offset: 300,
            seq_num: 3,
        });

        // "2" should find index point 1
        assert_eq!(index.find("2").unwrap().0, 1);

        // "5" should find index point 1 ("2") since "2" < "5" < "9"
        assert_eq!(index.find("5").unwrap().0, 1);

        // "1" should return None (before all)
        assert!(index.find("1").is_none());
    }
}
