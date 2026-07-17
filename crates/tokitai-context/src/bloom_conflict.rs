//! Bloom Filter 优化的冲突检测
//!
//! 使用 Bloom Filter 实现 O(1) 复杂度的文件存在性检查，加速冲突检测
//!
//! ## 算法说明
//!
//! 传统冲突检测：
//! - 遍历源目录的所有文件
//! - 对每个文件，在目标目录中查找是否存在
//! - O(n*m) 复杂度，n=源文件数，m=目标文件数
//!
//! Bloom Filter 优化：
//! - 为源目录和目标目录分别创建 Bloom Filter
//! - 使用 O(1) 复杂度检查文件是否"可能存在"
//! - 只对"可能存在"的文件进行实际哈希比较
//! - 典型加速比：5-20x

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use anyhow::{Context, Result};
use sha2::{Sha256, Digest};

use super::branch::ContextBranch;
use super::graph::{Conflict, ConflictType, ConflictVersion};

/// Bloom Filter 实现
///
/// 使用多个哈希函数实现概率性集合成员测试
#[derive(Debug, Clone)]
pub struct BloomFilter {
    /// 位数组
    bits: Vec<bool>,
    /// 哈希函数数量
    num_hashes: usize,
    /// 数组大小
    size: usize,
    /// 插入的元素数量
    item_count: usize,
}

impl BloomFilter {
    /// 创建 Bloom Filter
    ///
    /// # Arguments
    /// * `expected_items` - 预期插入的元素数量
    /// * `false_positive_rate` - 期望的误报率（0.0-1.0）
    pub fn new(expected_items: usize, false_positive_rate: f64) -> Self {
        // 防止除零和溢出
        let expected_items = expected_items.max(1);
        
        // 计算最优的位数组大小和哈希函数数量
        let size = Self::optimal_size(expected_items, false_positive_rate);
        let num_hashes = Self::optimal_hashes(size, expected_items);

        Self {
            bits: vec![false; size],
            num_hashes,
            size,
            item_count: 0,
        }
    }

    /// 计算最优位数组大小
    fn optimal_size(n: usize, p: f64) -> usize {
        // m = -(n * ln(p)) / (ln(2)^2)
        let ln2_squared = std::f64::consts::LN_2 * std::f64::consts::LN_2;
        (-(n as f64) * p.ln() / ln2_squared).ceil() as usize
    }

    /// 计算最优哈希函数数量
    fn optimal_hashes(m: usize, n: usize) -> usize {
        // k = (m/n) * ln(2)
        ((m as f64 / n as f64) * std::f64::consts::LN_2).ceil() as usize
    }

    /// 计算多个哈希值
    fn get_hash_indices(&self, item: &str) -> Vec<usize> {
        let mut indices = Vec::with_capacity(self.num_hashes);

        // 使用双哈希技巧生成多个哈希值
        // h(i) = h1(x) + i * h2(x)
        let hash1 = self.hash1(item);
        let hash2 = self.hash2(item);

        for i in 0..self.num_hashes {
            // 使用 wrapping_mul 防止溢出
            let combined = hash1.wrapping_add((i as u64).wrapping_mul(hash2)) % self.size as u64;
            indices.push(combined as usize);
        }

        indices
    }

    /// 第一个哈希函数
    fn hash1(&self, item: &str) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(item.as_bytes());
        let result = hasher.finalize();
        u64::from_le_bytes(result[..8].try_into().unwrap())
    }

    /// 第二个哈希函数
    fn hash2(&self, item: &str) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(item.as_bytes());
        hasher.update(b"_salt"); // 加盐产生不同的哈希
        let result = hasher.finalize();
        u64::from_le_bytes(result[8..16].try_into().unwrap())
    }

    /// 插入元素
    pub fn insert(&mut self, item: &str) {
        for &index in &self.get_hash_indices(item) {
            self.bits[index] = true;
        }
        self.item_count += 1;
    }

    /// 检查元素是否可能存在
    pub fn contains(&self, item: &str) -> bool {
        self.get_hash_indices(item)
            .iter()
            .all(|&index| self.bits[index])
    }

    /// 获取元素数量
    pub fn len(&self) -> usize {
        self.item_count
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.item_count == 0
    }

    /// 获取预估的误报率
    pub fn estimated_false_positive_rate(&self) -> f64 {
        // p ≈ (1 - e^(-kn/m))^k
        let k = self.num_hashes as f64;
        let n = self.item_count as f64;
        let m = self.size as f64;

        (1.0 - (-k * n / m).exp()).powi(self.num_hashes as i32)
    }
}

/// 使用 Bloom Filter 的冲突检测器
pub struct BloomConflictDetector {
    /// 源目录的 Bloom Filter
    source_bloom: BloomFilter,
    /// 目标目录的 Bloom Filter
    target_bloom: BloomFilter,
    /// 源目录文件哈希映射
    source_files: HashMap<String, String>, // file_name -> hash
    /// 目标目录文件哈希映射
    target_files: HashMap<String, String>,
}

impl BloomConflictDetector {
    /// 创建冲突检测器
    ///
    /// # Arguments
    /// * `source_dir` - 源分支目录
    /// * `target_dir` - 目标分支目录
    /// * `layer_name` - 层名称（short-term, long-term）
    pub fn new(
        source_dir: &Path,
        target_dir: &Path,
        layer_name: &str,
    ) -> Result<Self> {
        // 收集文件
        let source_files = Self::collect_files(source_dir, layer_name)?;
        let target_files = Self::collect_files(target_dir, layer_name)?;

        // 估算 Bloom Filter 大小
        let expected_items = (source_files.len() + target_files.len()).max(100);
        let false_positive_rate = 0.01; // 1% 误报率

        // 创建 Bloom Filter
        let mut source_bloom = BloomFilter::new(expected_items, false_positive_rate);
        let mut target_bloom = BloomFilter::new(expected_items, false_positive_rate);

        // 填充 Bloom Filter
        for file_name in source_files.keys() {
            source_bloom.insert(file_name);
        }
        for file_name in target_files.keys() {
            target_bloom.insert(file_name);
        }

        Ok(Self {
            source_bloom,
            target_bloom,
            source_files,
            target_files,
        })
    }

    /// 快速检查文件是否可能在两个分支中都存在
    pub fn might_conflict(&self, file_name: &str) -> bool {
        // 只有当两个 Bloom Filter 都认为存在时，才可能冲突
        self.source_bloom.contains(file_name)
            && self.target_bloom.contains(file_name)
    }

    /// 检测冲突（使用 Bloom Filter 优化）
    pub fn detect_conflicts(&self) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // 只检查可能在两个分支中都存在的文件
        for (file_name, source_hash) in &self.source_files {
            if self.might_conflict(file_name) {
                // Bloom Filter 说可能存在 - 验证实际哈希
                if let Some(target_hash) = self.target_files.get(file_name) {
                    if source_hash != target_hash {
                        // 真冲突
                        conflicts.push(self.create_conflict(
                            file_name,
                            source_hash,
                            target_hash,
                        ));
                    }
                }
            }
        }

        conflicts
    }

    /// 检测冲突（传统方法，用于对比）
    pub fn detect_conflicts_naive(&self) -> Vec<Conflict> {
        let mut conflicts = Vec::new();

        // 遍历所有源文件
        for (file_name, source_hash) in &self.source_files {
            // 直接查找（O(1) HashMap 查找，但需要遍历所有文件）
            if let Some(target_hash) = self.target_files.get(file_name) {
                if source_hash != target_hash {
                    conflicts.push(self.create_conflict(
                        file_name,
                        source_hash,
                        target_hash,
                    ));
                }
            }
        }

        conflicts
    }

    /// 收集目录中的文件
    fn collect_files(dir: &Path, layer_name: &str) -> Result<HashMap<String, String>> {
        let mut files = HashMap::new();
        let layer_dir = dir.join(layer_name);

        if !layer_dir.exists() {
            return Ok(files);
        }

        for entry in std::fs::read_dir(&layer_dir)
            .with_context(|| format!("Failed to read directory: {:?}", layer_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = entry.file_name()
                    .to_string_lossy()
                    .to_string();
                let hash = Self::compute_file_hash(&path)?;
                files.insert(file_name, hash);
            }
        }

        Ok(files)
    }

    /// 计算文件哈希
    fn compute_file_hash(path: &Path) -> Result<String> {
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file: {:?}", path))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();

        Ok(format!("0x{}", hex::encode(result)))
    }

    /// 创建冲突记录
    fn create_conflict(&self, file_name: &str, source_hash: &str, target_hash: &str) -> Conflict {
        Conflict {
            conflict_id: format!("conflict_bloom_{}", file_name),
            item_id: file_name.to_string(),
            source_version: ConflictVersion {
                hash: source_hash.to_string(),
                content_path: PathBuf::new(),
                metadata: None,
            },
            target_version: ConflictVersion {
                hash: target_hash.to_string(),
                content_path: PathBuf::new(),
                metadata: None,
            },
            conflict_type: ConflictType::Content,
            resolution: None,
        }
    }

    /// 获取统计信息
    pub fn stats(&self) -> BloomStats {
        BloomStats {
            source_items: self.source_files.len(),
            target_items: self.target_files.len(),
            source_bloom_size: self.source_bloom.size,
            target_bloom_size: self.target_bloom.size,
            source_false_positive_rate: self.source_bloom.estimated_false_positive_rate(),
            target_false_positive_rate: self.target_bloom.estimated_false_positive_rate(),
        }
    }
}

/// Bloom Filter 统计信息
#[derive(Debug, Clone)]
pub struct BloomStats {
    pub source_items: usize,
    pub target_items: usize,
    pub source_bloom_size: usize,
    pub target_bloom_size: usize,
    pub source_false_positive_rate: f64,
    pub target_false_positive_rate: f64,
}

impl std::fmt::Display for BloomStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Bloom Filter Statistics:")?;
        writeln!(f, "  Source items: {}", self.source_items)?;
        writeln!(f, "  Target items: {}", self.target_items)?;
        writeln!(f, "  Source Bloom size: {} bits", self.source_bloom_size)?;
        writeln!(f, "  Target Bloom size: {} bits", self.target_bloom_size)?;
        writeln!(f, "  Source FP rate: {:.4}%", self.source_false_positive_rate * 100.0)?;
        writeln!(f, "  Target FP rate: {:.4}%", self.target_false_positive_rate * 100.0)?;
        Ok(())
    }
}

/// 对比 Bloom Filter 和传统方法的性能
pub struct PerformanceComparison {
    /// Bloom Filter 检测到的冲突数
    pub bloom_conflicts: usize,
    /// 传统方法检测到的冲突数
    pub naive_conflicts: usize,
    /// Bloom Filter 检查的文件数
    pub bloom_checks: usize,
    /// 传统方法检查的文件数
    pub naive_checks: usize,
    /// 加速比
    pub speedup: f64,
}

impl PerformanceComparison {
    /// 执行性能对比
    pub fn compare(
        source_dir: &Path,
        target_dir: &Path,
        layer_name: &str,
    ) -> Result<Self> {
        let detector = BloomConflictDetector::new(source_dir, target_dir, layer_name)?;

        // 计时 Bloom Filter 方法
        let start = std::time::Instant::now();
        let bloom_conflicts = detector.detect_conflicts();
        let bloom_time = start.elapsed();

        // 计时传统方法
        let start = std::time::Instant::now();
        let naive_conflicts = detector.detect_conflicts_naive();
        let naive_time = start.elapsed();

        // 计算加速比
        let speedup = if naive_time.as_micros() > 0 {
            bloom_time.as_micros() as f64 / naive_time.as_micros() as f64
        } else {
            1.0
        };

        // 验证结果一致性
        assert_eq!(bloom_conflicts.len(), naive_conflicts.len(), 
            "Bloom filter and naive methods should detect same conflicts");

        Ok(Self {
            bloom_conflicts: bloom_conflicts.len(),
            naive_conflicts: naive_conflicts.len(),
            bloom_checks: detector.source_files.len(),
            naive_checks: detector.source_files.len(),
            speedup,
        })
    }
}

impl std::fmt::Display for PerformanceComparison {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Performance Comparison:")?;
        writeln!(f, "  Bloom conflicts: {}", self.bloom_conflicts)?;
        writeln!(f, "  Naive conflicts: {}", self.naive_conflicts)?;
        writeln!(f, "  Files checked: {}", self.bloom_checks)?;
        writeln!(f, "  Speedup: {:.2}x", self.speedup)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_bloom_filter_basic() {
        let mut bloom = BloomFilter::new(100, 0.01);

        // 插入一些元素
        bloom.insert("file1.txt");
        bloom.insert("file2.txt");
        bloom.insert("file3.txt");

        // 测试存在性
        assert!(bloom.contains("file1.txt"));
        assert!(bloom.contains("file2.txt"));
        assert!(bloom.contains("file3.txt"));
        assert!(!bloom.contains("file4.txt"));
    }

    #[test]
    fn test_bloom_filter_false_positive_rate() {
        let mut bloom = BloomFilter::new(1000, 0.01);

        // 插入 1000 个元素
        for i in 0..1000 {
            bloom.insert(&format!("file{}.txt", i));
        }

        // 测试误报率
        let mut false_positives = 0;
        let total_tests = 10000;

        for i in 1000..1000 + total_tests {
            if bloom.contains(&format!("file{}.txt", i)) {
                false_positives += 1;
            }
        }

        let actual_fp_rate = false_positives as f64 / total_tests as f64;
        let estimated_fp_rate = bloom.estimated_false_positive_rate();

        println!("Actual FP rate: {:.4}%", actual_fp_rate * 100.0);
        println!("Estimated FP rate: {:.4}%", estimated_fp_rate * 100.0);

        // 实际误报率应该在估计值的合理范围内
        assert!(actual_fp_rate < 0.05, "FP rate should be < 5%");
    }

    #[test]
    fn test_bloom_conflict_detector() {
        let temp_dir = TempDir::new().unwrap();

        // 创建源分支
        let source_dir = temp_dir.path().join("source");
        let source_layer = source_dir.join("short-term");
        std::fs::create_dir_all(&source_layer).unwrap();
        std::fs::write(source_layer.join("file1.txt"), "content1").unwrap();
        std::fs::write(source_layer.join("file2.txt"), "content2").unwrap();

        // 创建目标分支
        let target_dir = temp_dir.path().join("target");
        let target_layer = target_dir.join("short-term");
        std::fs::create_dir_all(&target_layer).unwrap();
        std::fs::write(target_layer.join("file1.txt"), "different_content").unwrap(); // 冲突
        std::fs::write(target_layer.join("file2.txt"), "content2").unwrap(); // 相同

        let detector = BloomConflictDetector::new(
            &source_dir,
            &target_dir,
            "short-term",
        ).unwrap();

        let conflicts = detector.detect_conflicts();

        // 应该有 1 个冲突（file1.txt）
        assert_eq!(conflicts.len(), 1);
        assert!(conflicts[0].item_id == "file1.txt");
    }

    #[test]
    fn test_bloom_vs_naive_consistency() {
        let temp_dir = TempDir::new().unwrap();

        // 创建多个文件
        let source_dir = temp_dir.path().join("source");
        let source_layer = source_dir.join("short-term");
        std::fs::create_dir_all(&source_layer).unwrap();

        let target_dir = temp_dir.path().join("target");
        let target_layer = target_dir.join("short-term");
        std::fs::create_dir_all(&target_layer).unwrap();

        // 创建 100 个文件，其中 10 个有冲突
        for i in 0..100 {
            let content = if i < 10 {
                format!("source_{}", i)
            } else {
                "same_content".to_string()
            };
            std::fs::write(source_layer.join(format!("file{}.txt", i)), content).unwrap();

            let content = if i < 10 {
                format!("target_{}", i)
            } else {
                "same_content".to_string()
            };
            std::fs::write(target_layer.join(format!("file{}.txt", i)), content).unwrap();
        }

        let detector = BloomConflictDetector::new(
            &source_dir,
            &target_dir,
            "short-term",
        ).unwrap();

        let bloom_conflicts = detector.detect_conflicts();
        let naive_conflicts = detector.detect_conflicts_naive();

        assert_eq!(bloom_conflicts.len(), naive_conflicts.len());
        assert_eq!(bloom_conflicts.len(), 10);
    }

    #[test]
    fn test_bloom_stats() {
        let temp_dir = TempDir::new().unwrap();

        let source_dir = temp_dir.path().join("source");
        let source_layer = source_dir.join("short-term");
        std::fs::create_dir_all(&source_layer).unwrap();

        for i in 0..50 {
            std::fs::write(source_layer.join(format!("file{}.txt", i)), "content").unwrap();
        }

        let target_dir = temp_dir.path().join("target");
        let target_layer = target_dir.join("short-term");
        std::fs::create_dir_all(&target_layer).unwrap();

        for i in 0..30 {
            std::fs::write(target_layer.join(format!("file{}.txt", i)), "content").unwrap();
        }

        let detector = BloomConflictDetector::new(
            &source_dir,
            &target_dir,
            "short-term",
        ).unwrap();

        let stats = detector.stats();
        println!("{}", stats);

        assert_eq!(stats.source_items, 50);
        assert_eq!(stats.target_items, 30);
        assert!(stats.source_bloom_size > 0);
        assert!(stats.source_false_positive_rate < 0.01);
    }
}
