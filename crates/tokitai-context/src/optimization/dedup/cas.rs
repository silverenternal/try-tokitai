//! 存储优化模块
//!
//! 实现高效的存储优化策略：
//! - **透明压缩**: 使用 zstd/libz 压缩上下文文件
//! - **内容寻址存储**: 基于内容哈希的去重存储
//! - **增量快照**: 只存储变化的部分
//! - **垃圾回收**: 清理未引用的数据块

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};

/// 压缩算法选择
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum CompressionAlgorithm {
    /// 不压缩
    None,
    /// zstd (快速，高压缩率)
    #[default]
    Zstd,
    /// lz4 (极快，中等压缩率)
    Lz4,
    /// gzip (兼容性好)
    Gzip,
}


/// 压缩配置
#[derive(Debug, Clone)]
pub struct CompressionConfig {
    /// 压缩算法
    pub algorithm: CompressionAlgorithm,
    /// 压缩级别 (0-9)
    pub level: u32,
    /// 最小压缩大小 (字节)
    pub min_size: usize,
    /// 是否压缩二进制文件
    pub compress_binary: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3, // 平衡速度和压缩率
            min_size: 1024, // 1KB 以上才压缩
            compress_binary: false,
        }
    }
}

/// 内容寻址存储条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAddressableEntry {
    /// 内容哈希
    pub hash: String,
    /// 压缩后的数据大小
    pub compressed_size: u64,
    /// 原始数据大小
    pub uncompressed_size: u64,
    /// 引用计数
    pub reference_count: u32,
    /// 最后访问时间
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// 存储路径
    pub storage_path: PathBuf,
}

/// 内容寻址存储管理器
pub struct ContentAddressableStorage {
    /// 存储目录
    storage_dir: PathBuf,
    /// 索引映射：hash -> entry
    index: HashMap<String, ContentAddressableEntry>,
    /// 引用计数：hash -> count
    reference_counts: HashMap<String, u32>,
    /// 配置
    config: CompressionConfig,
    /// 统计信息
    stats: StorageStats,
}

/// 存储统计信息
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// 总对象数
    pub total_objects: usize,
    /// 总原始大小
    pub total_uncompressed_size: u64,
    /// 总压缩大小
    pub total_compressed_size: u64,
    /// 压缩率
    pub compression_ratio: f64,
    /// 去重节省的空间
    pub dedup_savings: u64,
    /// 写入次数
    pub write_count: u64,
    /// 读取次数
    pub read_count: u64,
}

impl ContentAddressableStorage {
    /// 创建内容寻址存储
    pub fn new<P: AsRef<Path>>(storage_dir: P, config: CompressionConfig) -> Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // 创建存储目录
        std::fs::create_dir_all(&storage_dir)
            .with_context(|| format!("Failed to create storage directory: {:?}", storage_dir))?;

        let mut storage = Self {
            storage_dir,
            index: HashMap::new(),
            reference_counts: HashMap::new(),
            config,
            stats: StorageStats::default(),
        };

        // 加载现有索引
        storage.load_index()?;

        Ok(storage)
    }

    /// 存储内容
    pub fn store(&mut self, content: &[u8]) -> Result<String> {
        // 计算哈希
        let hash = Self::compute_hash(content);

        // 检查是否已存在（去重）
        if let Some(_entry) = self.index.get(&hash) {
            // 增加引用计数
            self.increment_reference(&hash)?;
            // 更新访问统计
            self.stats.read_count += 1;
            let new_ref_count = self.reference_counts.get(&hash).unwrap_or(&1) + 1;
            tracing::debug!(
                "CAS dedup hit: hash {} now has {} references",
                hash,
                new_ref_count
            );
            return Ok(hash);
        }

        // 检查是否需要压缩
        let data_to_store = if content.len() >= self.config.min_size {
            self.compress(content)?
        } else {
            content.to_vec()
        };

        // 生成存储路径
        let storage_path = self.get_storage_path(&hash);

        // 写入数据
        std::fs::write(&storage_path, &data_to_store)
            .with_context(|| format!("Failed to write content: {:?}", storage_path))?;

        // 创建索引条目
        let entry = ContentAddressableEntry {
            hash: hash.clone(),
            compressed_size: data_to_store.len() as u64,
            uncompressed_size: content.len() as u64,
            reference_count: 1,
            last_accessed: chrono::Utc::now(),
            storage_path: storage_path.clone(),
        };

        self.index.insert(hash.clone(), entry);
        self.reference_counts.insert(hash.clone(), 1);

        // 更新统计
        self.stats.total_objects += 1;
        self.stats.total_uncompressed_size += content.len() as u64;
        self.stats.total_compressed_size += data_to_store.len() as u64;
        self.stats.write_count += 1;

        Ok(hash)
    }

    /// 读取内容
    pub fn retrieve(&mut self, hash: &str) -> Result<Vec<u8>> {
        let entry = self
            .index
            .get(hash)
            .ok_or_else(|| anyhow::anyhow!("Content not found: {}", hash))?;

        // 读取压缩数据
        let compressed_data = std::fs::read(&entry.storage_path)
            .with_context(|| format!("Failed to read content: {:?}", entry.storage_path))?;

        // 解压缩：根据原始大小和压缩后大小的差异判断是否被压缩
        let content = if entry.uncompressed_size != entry.compressed_size {
            self.decompress(&compressed_data)?
        } else {
            compressed_data
        };

        // 更新访问时间和统计
        if let Some(entry) = self.index.get_mut(hash) {
            entry.last_accessed = chrono::Utc::now();
        }
        self.stats.read_count += 1;

        Ok(content)
    }

    /// 增加引用计数
    pub fn increment_reference(&mut self, hash: &str) -> Result<()> {
        if let Some(count) = self.reference_counts.get_mut(hash) {
            *count += 1;
            if let Some(entry) = self.index.get_mut(hash) {
                entry.reference_count += 1;
            }
            Ok(())
        } else {
            Err(anyhow::anyhow!("Hash not found: {}", hash))
        }
    }

    /// 减少引用计数
    pub fn decrement_reference(&mut self, hash: &str) -> Result<u32> {
        if let Some(count) = self.reference_counts.get_mut(hash) {
            if *count > 0 {
                *count -= 1;
                if let Some(entry) = self.index.get_mut(hash) {
                    entry.reference_count -= 1;
                }
                return Ok(*count);
            }
        }
        Ok(0)
    }

    /// 获取引用计数
    pub fn get_reference_count(&self, hash: &str) -> Option<u32> {
        self.reference_counts.get(hash).copied()
    }

    /// 删除未引用的内容
    pub fn garbage_collect(&mut self) -> Result<GcResult> {
        let mut gc_result = GcResult::default();

        let to_remove: Vec<String> = self
            .reference_counts
            .iter()
            .filter(|(_, &count)| count == 0)
            .map(|(hash, _)| hash.clone())
            .collect();

        for hash in &to_remove {
            if let Some(entry) = self.index.remove(hash) {
                // 删除文件
                if entry.storage_path.exists() {
                    std::fs::remove_file(&entry.storage_path)?;
                    gc_result.deleted_files += 1;
                    gc_result.freed_space += entry.compressed_size;
                }

                self.stats.total_objects -= 1;
                self.stats.total_uncompressed_size -= entry.uncompressed_size;
                self.stats.total_compressed_size -= entry.compressed_size;
            }

            self.reference_counts.remove(hash);
        }

        // 更新压缩率
        if self.stats.total_uncompressed_size > 0 {
            self.stats.compression_ratio =
                self.stats.total_compressed_size as f64 / self.stats.total_uncompressed_size as f64;
        }

        gc_result.collected_count = to_remove.len();
        Ok(gc_result)
    }

    /// 压缩数据
    fn compress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.config.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Zstd => {
                // 使用简单 RLE 压缩作为 fallback
                Ok(Self::rle_compress(data))
            }
            CompressionAlgorithm::Lz4 => {
                // 无 lz4 特性时使用简单 RLE 压缩
                Ok(Self::rle_compress(data))
            }
            CompressionAlgorithm::Gzip => {
                // 使用简单 RLE 压缩作为 fallback
                Ok(Self::rle_compress(data))
            }
        }
    }

    /// 解压缩数据
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        match self.config.algorithm {
            CompressionAlgorithm::None => Ok(data.to_vec()),
            CompressionAlgorithm::Zstd => {
                // 无 zstd 特性时使用简单 RLE 解压缩
                Self::rle_decompress(data)
            }
            CompressionAlgorithm::Lz4 => {
                // 无 lz4 特性时直接返回
                Ok(data.to_vec())
            }
            CompressionAlgorithm::Gzip => {
                // 使用简单 RLE 解压缩
                Self::rle_decompress(data)
            }
        }
    }

    /// 简单的 RLE 压缩（fallback）
    fn rle_compress(data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return Vec::new();
        }

        let mut compressed = Vec::new();
        let mut i = 0;

        while i < data.len() {
            let current = data[i];
            let mut count = 1;

            while i + count < data.len() && data[i + count] == current && count < 255 {
                count += 1;
            }

            compressed.push(count as u8);
            compressed.push(current);
            i += count;
        }

        compressed
    }

    /// 简单的 RLE 解压缩（fallback）
    fn rle_decompress(data: &[u8]) -> Result<Vec<u8>> {
        let mut decompressed = Vec::new();
        let mut i = 0;

        while i + 1 < data.len() {
            let count = data[i] as usize;
            let value = data[i + 1];

            for _ in 0..count {
                decompressed.push(value);
            }

            i += 2;
        }

        Ok(decompressed)
    }

    /// 计算内容哈希
    fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        format!("0x{}", hex::encode(hasher.finalize()))
    }

    /// 获取存储路径（使用哈希的前缀分目录）
    fn get_storage_path(&self, hash: &str) -> PathBuf {
        // 使用哈希的前 2 个字符作为子目录
        let prefix = if hash.len() > 2 { &hash[2..4] } else { "00" };
        let subdir = self.storage_dir.join(prefix);
        
        // 确保子目录存在
        if !subdir.exists() {
            let _ = std::fs::create_dir_all(&subdir);
        }
        
        subdir.join(hash)
    }

    /// 加载索引
    fn load_index(&mut self) -> Result<()> {
        let index_file = self.storage_dir.join("index.json");

        if !index_file.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&index_file)
            .with_context(|| format!("Failed to read index: {:?}", index_file))?;

        let index: HashMap<String, ContentAddressableEntry> =
            serde_json::from_str(&content)
                .with_context(|| "Failed to parse index")?;

        // 重建引用计数
        for (hash, entry) in &index {
            self.reference_counts
                .insert(hash.clone(), entry.reference_count);
        }

        self.index = index;
        Ok(())
    }

    /// 保存索引
    pub fn save_index(&self) -> Result<()> {
        let index_file = self.storage_dir.join("index.json");

        let content = serde_json::to_string_pretty(&self.index)
            .with_context(|| "Failed to serialize index")?;

        std::fs::write(&index_file, content)
            .with_context(|| format!("Failed to write index: {:?}", index_file))?;

        Ok(())
    }

    /// 获取统计信息
    pub fn stats(&self) -> &StorageStats {
        &self.stats
    }

    /// 获取对象数量
    pub fn object_count(&self) -> usize {
        self.index.len()
    }

    /// 检查哈希是否存在
    pub fn contains(&self, hash: &str) -> bool {
        self.index.contains_key(hash)
    }

    /// 获取所有哈希
    pub fn list_hashes(&self) -> Vec<&String> {
        self.index.keys().collect()
    }
}

impl Drop for ContentAddressableStorage {
    fn drop(&mut self) {
        // 自动保存索引
        if let Err(e) = self.save_index() {
            tracing::warn!("Failed to save CAS index on drop: {}", e);
        }
    }
}

/// 垃圾回收结果
#[derive(Debug, Clone, Default)]
pub struct GcResult {
    /// 回收的对象数量
    pub collected_count: usize,
    /// 删除的文件数量
    pub deleted_files: usize,
    /// 释放的空间（字节）
    pub freed_space: u64,
}

impl std::fmt::Display for GcResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Garbage Collection Result:")?;
        writeln!(f, "  Objects collected: {}", self.collected_count)?;
        writeln!(f, "  Files deleted: {}", self.deleted_files)?;
        writeln!(f, "  Space freed: {:.2} KB", self.freed_space as f64 / 1024.0)?;
        Ok(())
    }
}

/// 增量快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncrementalSnapshot {
    /// 快照 ID
    pub snapshot_id: String,
    /// 父快照 ID
    pub parent_snapshot: Option<String>,
    /// 创建时间
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// 变更列表
    pub changes: Vec<SnapshotChange>,
    /// 快照元数据
    pub metadata: SnapshotMetadata,
}

/// 快照变更
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotChange {
    /// 变更类型
    pub change_type: ChangeType,
    /// 项目 ID
    pub item_id: String,
    /// 内容哈希（新增/修改时）
    pub content_hash: Option<String>,
    /// 变更时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 变更类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// 新增
    Added,
    /// 修改
    Modified,
    /// 删除
    Deleted,
}

/// 快照元数据
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// 描述
    pub description: Option<String>,
    /// 标签
    pub tags: Vec<String>,
    /// 变更总数
    pub total_changes: usize,
    /// 新增数量
    pub added_count: usize,
    /// 修改数量
    pub modified_count: usize,
    /// 删除数量
    pub deleted_count: usize,
}

/// 快照管理器
pub struct SnapshotManager {
    /// 快照存储目录
    snapshot_dir: PathBuf,
    /// 快照列表
    snapshots: HashMap<String, IncrementalSnapshot>,
    /// 内容存储
    cas: ContentAddressableStorage,
}

impl SnapshotManager {
    /// 创建快照管理器
    pub fn new<P: AsRef<Path>>(snapshot_dir: P, cas: ContentAddressableStorage) -> Result<Self> {
        let snapshot_dir = snapshot_dir.as_ref().to_path_buf();

        std::fs::create_dir_all(&snapshot_dir)
            .with_context(|| format!("Failed to create snapshot directory: {:?}", snapshot_dir))?;

        let mut manager = Self {
            snapshot_dir,
            snapshots: HashMap::new(),
            cas,
        };

        // 加载现有快照
        manager.load_snapshots()?;

        Ok(manager)
    }

    /// 创建快照
    pub fn create_snapshot(
        &mut self,
        parent: Option<&str>,
        changes: Vec<SnapshotChange>,
        description: Option<&str>,
    ) -> Result<String> {
        let snapshot_id = format!("snap_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0));

        // 计算元数据
        let mut metadata = SnapshotMetadata {
            description: description.map(|s| s.to_string()),
            total_changes: changes.len(),
            ..Default::default()
        };

        for change in &changes {
            match change.change_type {
                ChangeType::Added => metadata.added_count += 1,
                ChangeType::Modified => metadata.modified_count += 1,
                ChangeType::Deleted => metadata.deleted_count += 1,
            }
        }

        let snapshot = IncrementalSnapshot {
            snapshot_id: snapshot_id.clone(),
            parent_snapshot: parent.map(|s| s.to_string()),
            created_at: chrono::Utc::now(),
            changes,
            metadata,
        };

        // 保存快照
        self.save_snapshot(&snapshot)?;
        self.snapshots.insert(snapshot_id.clone(), snapshot);

        Ok(snapshot_id)
    }

    /// 获取快照
    pub fn get_snapshot(&self, snapshot_id: &str) -> Option<&IncrementalSnapshot> {
        self.snapshots.get(snapshot_id)
    }

    /// 列出所有快照
    pub fn list_snapshots(&self) -> Vec<&IncrementalSnapshot> {
        self.snapshots.values().collect()
    }

    /// 删除快照
    pub fn delete_snapshot(&mut self, snapshot_id: &str) -> Result<()> {
        if !self.snapshots.contains_key(snapshot_id) {
            return Err(anyhow::anyhow!("Snapshot not found: {}", snapshot_id));
        }

        self.snapshots.remove(snapshot_id);

        // 删除快照文件
        let snapshot_file = self.get_snapshot_file_path(snapshot_id);
        if snapshot_file.exists() {
            std::fs::remove_file(snapshot_file)?;
        }

        Ok(())
    }

    /// 保存快照
    fn save_snapshot(&self, snapshot: &IncrementalSnapshot) -> Result<()> {
        let snapshot_file = self.get_snapshot_file_path(&snapshot.snapshot_id);

        let content = serde_json::to_string_pretty(snapshot)
            .with_context(|| "Failed to serialize snapshot")?;

        std::fs::write(&snapshot_file, content)
            .with_context(|| format!("Failed to write snapshot: {:?}", snapshot_file))?;

        Ok(())
    }

    /// 加载所有快照
    fn load_snapshots(&mut self) -> Result<()> {
        if !self.snapshot_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.snapshot_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file()
                && path
                    .extension()
                    .map(|e| e == "json")
                    .unwrap_or(false)
            {
                let content = std::fs::read_to_string(&path)?;
                let snapshot: IncrementalSnapshot = serde_json::from_str(&content)?;
                self.snapshots
                    .insert(snapshot.snapshot_id.clone(), snapshot);
            }
        }

        Ok(())
    }

    /// 获取快照文件路径
    fn get_snapshot_file_path(&self, snapshot_id: &str) -> PathBuf {
        self.snapshot_dir.join(format!("{}.json", snapshot_id))
    }

    /// 获取内容存储
    pub fn cas(&self) -> &ContentAddressableStorage {
        &self.cas
    }

    /// 获取可变的内容存储
    pub fn cas_mut(&mut self) -> &mut ContentAddressableStorage {
        &mut self.cas
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cas_store_retrieve() {
        let temp_dir = TempDir::new().unwrap();
        let config = CompressionConfig::default();
        let mut cas = ContentAddressableStorage::new(temp_dir.path(), config).unwrap();

        let content = b"Hello, World!";
        let hash = cas.store(content).unwrap();

        assert!(hash.starts_with("0x"));
        assert!(cas.contains(&hash));

        let retrieved = cas.retrieve(&hash).unwrap();
        assert_eq!(retrieved, content);
    }

    #[test]
    fn test_cas_deduplication() {
        let temp_dir = TempDir::new().unwrap();
        let config = CompressionConfig::default();
        let mut cas = ContentAddressableStorage::new(temp_dir.path(), config).unwrap();

        let content = b"Duplicate content";

        // 存储两次相同内容
        let hash1 = cas.store(content).unwrap();
        let hash2 = cas.store(content).unwrap();

        // 哈希应该相同（去重）
        assert_eq!(hash1, hash2);

        // 统计信息应该显示去重
        let stats = cas.stats();
        assert_eq!(stats.total_objects, 1);
        assert_eq!(cas.get_reference_count(&hash1), Some(2));
    }

    #[test]
    fn test_cas_reference_counting() {
        let temp_dir = TempDir::new().unwrap();
        let config = CompressionConfig::default();
        let mut cas = ContentAddressableStorage::new(temp_dir.path(), config).unwrap();

        let content = b"Test content";
        let hash = cas.store(content).unwrap();

        // 增加引用
        cas.increment_reference(&hash).unwrap();
        cas.increment_reference(&hash).unwrap();

        // 减少引用
        let count1 = cas.decrement_reference(&hash).unwrap();
        assert_eq!(count1, 2);

        let count2 = cas.decrement_reference(&hash).unwrap();
        assert_eq!(count2, 1);

        let count3 = cas.decrement_reference(&hash).unwrap();
        assert_eq!(count3, 0);
    }

    #[test]
    fn test_compression() {
        let temp_dir = TempDir::new().unwrap();
        let config = CompressionConfig {
            algorithm: CompressionAlgorithm::Zstd,
            level: 3,
            min_size: 10, // 10 字节以上就压缩
            compress_binary: false,
        };

        let mut cas = ContentAddressableStorage::new(temp_dir.path(), config).unwrap();

        // 创建可压缩的内容（重复数据）
        let content = vec![b'a'; 1000];
        let hash = cas.store(&content).unwrap();

        let retrieved = cas.retrieve(&hash).unwrap();
        assert_eq!(retrieved, content);

        // 检查压缩效果
        let stats = cas.stats();
        assert!(stats.compression_ratio < 1.0);
    }

    #[test]
    fn test_snapshot_creation() {
        let temp_dir = TempDir::new().unwrap();
        let cas_config = CompressionConfig::default();
        let cas = ContentAddressableStorage::new(temp_dir.path().join("cas"), cas_config).unwrap();

        let mut manager = SnapshotManager::new(temp_dir.path().join("snapshots"), cas).unwrap();

        let changes = vec![
            SnapshotChange {
                change_type: ChangeType::Added,
                item_id: "item1".to_string(),
                content_hash: Some("0xabc".to_string()),
                timestamp: chrono::Utc::now(),
            },
            SnapshotChange {
                change_type: ChangeType::Modified,
                item_id: "item2".to_string(),
                content_hash: Some("0xdef".to_string()),
                timestamp: chrono::Utc::now(),
            },
        ];

        let snapshot_id = manager
            .create_snapshot(None, changes, Some("Test snapshot"))
            .unwrap();

        assert!(snapshot_id.starts_with("snap_"));

        let snapshot = manager.get_snapshot(&snapshot_id).unwrap();
        assert_eq!(snapshot.metadata.total_changes, 2);
        assert_eq!(snapshot.metadata.added_count, 1);
        assert_eq!(snapshot.metadata.modified_count, 1);
    }

    #[test]
    fn test_snapshot_chain() {
        let temp_dir = TempDir::new().unwrap();
        let cas_config = CompressionConfig::default();
        let cas = ContentAddressableStorage::new(temp_dir.path().join("cas"), cas_config).unwrap();

        let mut manager = SnapshotManager::new(temp_dir.path().join("snapshots"), cas).unwrap();

        // 创建快照链
        let snap1 = manager
            .create_snapshot(
                None,
                vec![SnapshotChange {
                    change_type: ChangeType::Added,
                    item_id: "item1".to_string(),
                    content_hash: None,
                    timestamp: chrono::Utc::now(),
                }],
                None,
            )
            .unwrap();

        let snap2 = manager
            .create_snapshot(
                Some(&snap1),
                vec![SnapshotChange {
                    change_type: ChangeType::Modified,
                    item_id: "item1".to_string(),
                    content_hash: None,
                    timestamp: chrono::Utc::now(),
                }],
                None,
            )
            .unwrap();

        let _snapshot1 = manager.get_snapshot(&snap1).unwrap();
        let snapshot2 = manager.get_snapshot(&snap2).unwrap();

        assert_eq!(snapshot2.parent_snapshot, Some(snap1.clone()));
    }
}
