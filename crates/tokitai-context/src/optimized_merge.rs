//! 高级合并算法优化
//!
//! 实现更先进的合并算法：
//! - **diff3 算法**: Git 风格的三向文本合并
//! - **LCS (Longest Common Subsequence)**: 最长公共子序列比对（使用 Hirschberg 算法优化空间）
//! - **语义块合并**: 基于语义分块的智能合并
//! - **内容感知去重**: 检测并合并重复内容

use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use super::branch::ContextBranch;
use super::merge::{MergeResult, Merger};
use super::graph::{Conflict, ConflictType, ConflictVersion};
use crate::optimization::algorithms::lcs::HirschbergLCS;

/// diff3 合并标记
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diff3Hunk {
    /// 三者相同
    Identical(String),
    /// 只有源分支修改
    SourceOnly(String),
    /// 只有目标分支修改
    TargetOnly(String),
    /// 两者都修改（可能冲突）
    BothModified {
        base: String,
        source: String,
        target: String,
    },
    /// 真冲突
    Conflict {
        base: String,
        source: String,
        target: String,
    },
}

/// LCS 比对结果
#[derive(Debug, Clone)]
pub struct LcsAlignment {
    /// 最长公共子序列
    pub lcs: Vec<String>,
    /// 源分支的差异
    pub source_diffs: Vec<Diff>,
    /// 目标分支的差异
    pub target_diffs: Vec<Diff>,
}

/// 差异类型
#[derive(Debug, Clone)]
pub enum Diff {
    /// 插入
    Insert(String),
    /// 删除
    Delete(String),
    /// 保持不变
    Keep(String),
    /// 替换
    Replace { old: String, new: String },
}

/// 语义块
#[derive(Debug, Clone)]
pub struct SemanticBlock {
    /// 块 ID
    pub id: String,
    /// 块内容
    pub content: String,
    /// 块类型（函数、类、变量等）
    pub block_type: String,
    /// 起始行号
    pub start_line: usize,
    /// 结束行号
    pub end_line: usize,
    /// 依赖的其他块 ID
    pub dependencies: Vec<String>,
}

/// 高级合并器
pub struct AdvancedMerger {
    /// 基础合并器
    base_merger: Merger,
    /// 是否启用智能冲突标记
    enable_smart_markers: bool,
    /// 最大文本大小（字节）用于 diff3
    max_diff3_size: usize,
}

impl AdvancedMerger {
    /// 创建高级合并器
    pub fn new(branches_dir: &Path, merge_logs_dir: &Path) -> Result<Self> {
        let base_merger = Merger::new(branches_dir, merge_logs_dir)?;

        Ok(Self {
            base_merger,
            enable_smart_markers: true,
            max_diff3_size: 10 * 1024 * 1024, // 10MB
        })
    }

    /// 使用 diff3 算法合并两个文本
    pub fn diff3_merge(
        &self,
        base_content: &str,
        source_content: &str,
        target_content: &str,
    ) -> Result<Diff3Result> {
        // 检查文本大小
        let total_size = base_content.len() + source_content.len() + target_content.len();
        if total_size > self.max_diff3_size {
            return Err(anyhow::anyhow!(
                "Content too large for diff3 merge ({} bytes, max {})",
                total_size,
                self.max_diff3_size
            ));
        }

        // 按行分割
        let base_lines: Vec<&str> = base_content.lines().collect();
        let source_lines: Vec<&str> = source_content.lines().collect();
        let target_lines: Vec<&str> = target_content.lines().collect();

        // 生成 hunks（内部会计算 LCS）
        let hunks = self.generate_diff3_hunks(
            &base_lines,
            &source_lines,
            &target_lines,
        );

        // 合并 hunks
        let (merged_lines, conflicts) = self.merge_hunks(&hunks);

        Ok(Diff3Result {
            merged_content: merged_lines.join("\n"),
            conflicts: conflicts.clone(),
            hunks,
            success: conflicts.is_empty(),
        })
    }

    /// 计算最长公共子序列（使用 Hirschberg 算法优化空间复杂度）
    /// 返回 (base_idx, other_idx) 对
    pub fn compute_lcs_pairs<T: PartialEq + Clone>(a: &[T], b: &[T]) -> Vec<(usize, usize)> {
        // 使用 Hirschberg 算法，空间复杂度从 O(m*n) 优化到 O(min(m,n))
        HirschbergLCS::compute_lcs(a, b)
    }

    /// 生成 diff3 hunks - 使用标准 diff3 算法
    fn generate_diff3_hunks(
        &self,
        base: &[&str],
        source: &[&str],
        target: &[&str],
    ) -> Vec<Diff3Hunk> {
        // 计算完整的 LCS 对
        let lcs_source_pairs = Self::compute_lcs_pairs(base, source);
        let lcs_target_pairs = Self::compute_lcs_pairs(base, target);
        
        let mut hunks = Vec::new();
        let mut prev_base = 0;
        let mut prev_source = 0;
        let mut prev_target = 0;

        // 使用双指针找到共同的锚点
        let mut si = 0;
        let mut ti = 0;

        while si < lcs_source_pairs.len() && ti < lcs_target_pairs.len() {
            let (base_idx_s, source_idx) = lcs_source_pairs[si];
            let (base_idx_t, target_idx) = lcs_target_pairs[ti];

            if base_idx_s == base_idx_t {
                // 找到共同锚点
                let base_idx = base_idx_s;

                // 处理这个锚点之前的区域
                let base_chunk = &base[prev_base..base_idx];
                let source_chunk = &source[prev_source..source_idx];
                let target_chunk = &target[prev_target..target_idx];

                if !base_chunk.is_empty() || !source_chunk.is_empty() || !target_chunk.is_empty() {
                    let hunk = Self::classify_hunk(base_chunk, source_chunk, target_chunk);
                    if let Some(h) = hunk {
                        hunks.push(h);
                    }
                }

                // 添加锚点（相同行）
                hunks.push(Diff3Hunk::Identical(base[base_idx].to_string()));

                prev_base = base_idx + 1;
                prev_source = source_idx + 1;
                prev_target = target_idx + 1;

                si += 1;
                ti += 1;
            } else if base_idx_s < base_idx_t {
                si += 1;
            } else {
                ti += 1;
            }
        }

        // 处理最后一个锚点之后的剩余部分
        let base_chunk = &base[prev_base..];
        let source_chunk = &source[prev_source..];
        let target_chunk = &target[prev_target..];

        if !base_chunk.is_empty() || !source_chunk.is_empty() || !target_chunk.is_empty() {
            let hunk = Self::classify_hunk(base_chunk, source_chunk, target_chunk);
            if let Some(h) = hunk {
                hunks.push(h);
            }
        }

        hunks
    }

    /// 分类 hunk 类型
    fn classify_hunk(
        base: &[&str],
        source: &[&str],
        target: &[&str],
    ) -> Option<Diff3Hunk> {
        let base_empty = base.is_empty();
        let source_empty = source.is_empty();
        let target_empty = target.is_empty();

        let base_str = if base_empty { String::new() } else { base.join("\n") };
        let source_str = if source_empty { String::new() } else { source.join("\n") };
        let target_str = if target_empty { String::new() } else { target.join("\n") };

        // 三者都空，跳过
        if base_empty && source_empty && target_empty {
            return None;
        }

        // 情况 1: 都没有变化
        if !base_empty && !source_empty && !target_empty 
           && base_str == source_str && source_str == target_str {
            return Some(Diff3Hunk::Identical(base_str));
        }

        // 情况 2: 只有源修改
        if !base_empty && !source_empty && target_empty {
            return Some(Diff3Hunk::SourceOnly(source_str));
        }

        // 情况 3: 只有目标修改
        if !base_empty && source_empty && !target_empty {
            return Some(Diff3Hunk::TargetOnly(target_str));
        }

        // 情况 4: 两者都修改
        if !source_empty && !target_empty {
            if source_str == target_str {
                // 相同的修改
                return Some(Diff3Hunk::Identical(source_str));
            } else if base_empty {
                // 两者都添加不同内容 - 冲突
                return Some(Diff3Hunk::Conflict {
                    base: String::new(),
                    source: source_str,
                    target: target_str,
                });
            } else {
                // 检查是否一方与 base 相同
                if base_str == source_str {
                    return Some(Diff3Hunk::TargetOnly(target_str));
                } else if base_str == target_str {
                    return Some(Diff3Hunk::SourceOnly(source_str));
                } else {
                    // 真冲突
                    return Some(Diff3Hunk::Conflict {
                        base: base_str,
                        source: source_str,
                        target: target_str,
                    });
                }
            }
        }

        // 情况 5: 只有一方有内容
        if !source_empty && target_empty {
            return Some(Diff3Hunk::SourceOnly(source_str));
        }
        if source_empty && !target_empty {
            return Some(Diff3Hunk::TargetOnly(target_str));
        }

        // 情况 6: base 有内容，但 source 和 target 都删除了
        if !base_empty && source_empty && target_empty {
            return Some(Diff3Hunk::BothModified {
                base: base_str,
                source: String::new(),
                target: String::new(),
            });
        }

        None
    }

    /// 合并 hunks
    fn merge_hunks(&self, hunks: &[Diff3Hunk]) -> (Vec<String>, Vec<Conflict>) {
        let mut merged = Vec::new();
        let mut conflicts = Vec::new();
        let mut conflict_idx = 0;

        for hunk in hunks {
            match hunk {
                Diff3Hunk::Identical(content) => {
                    merged.push(content.clone());
                }
                Diff3Hunk::SourceOnly(content) => {
                    merged.push(content.clone());
                }
                Diff3Hunk::TargetOnly(content) => {
                    merged.push(content.clone());
                }
                Diff3Hunk::BothModified {
                    base: _,
                    source,
                    target,
                } => {
                    // 非冲突的修改，优先采用源
                    if source == target {
                        merged.push(source.clone());
                    } else {
                        // 采用源分支
                        merged.push(source.clone());
                    }
                }
                Diff3Hunk::Conflict {
                    base,
                    source,
                    target,
                } => {
                    // 生成冲突标记
                    if self.enable_smart_markers {
                        merged.push("<<<<<<< SOURCE".to_string());
                        merged.push(source.clone());
                        merged.push("=======".to_string());
                        if !base.is_empty() {
                            merged.push("-ORIG-".to_string());
                            merged.push(base.clone());
                            merged.push("-------".to_string());
                        }
                        merged.push(target.clone());
                        merged.push(">>>>>>> TARGET".to_string());
                    } else {
                        merged.push(source.clone());
                    }

                    // 记录冲突
                    conflicts.push(Conflict {
                        conflict_id: format!("diff3_conflict_{}", conflict_idx),
                        item_id: format!("hunk_{}", conflict_idx),
                        source_version: ConflictVersion {
                            hash: format!("0x{}", hex::encode(source.as_bytes())),
                            content_path: PathBuf::new(),
                            metadata: None,
                        },
                        target_version: ConflictVersion {
                            hash: format!("0x{}", hex::encode(target.as_bytes())),
                            content_path: PathBuf::new(),
                            metadata: None,
                        },
                        conflict_type: ConflictType::Content,
                        resolution: None,
                    });
                    conflict_idx += 1;
                }
            }
        }

        (merged, conflicts)
    }

    /// 基于语义块的合并
    pub fn semantic_merge(
        &self,
        source: &ContextBranch,
        target: &ContextBranch,
        base: &ContextBranch,
    ) -> Result<SemanticMergeResult> {
        let mut merged_blocks = Vec::new();
        let mut conflicts = Vec::new();

        // 解析语义块（这里简化处理，实际应该使用 parser）
        let source_blocks = self.parse_semantic_blocks(source)?;
        let target_blocks = self.parse_semantic_blocks(target)?;
        let base_blocks = self.parse_semantic_blocks(base)?;

        // 创建块的哈希映射
        let base_map: HashMap<_, _> = base_blocks
            .iter()
            .map(|b| (b.id.clone(), b))
            .collect();
        let source_map: HashMap<_, _> = source_blocks
            .iter()
            .map(|b| (b.id.clone(), b))
            .collect();
        let target_map: HashMap<_, _> = target_blocks
            .iter()
            .map(|b| (b.id.clone(), b))
            .collect();

        // 合并所有块 ID
        let all_ids: HashSet<_> = source_map
            .keys()
            .chain(target_map.keys())
            .chain(base_map.keys())
            .cloned()
            .collect();

        // 对每个块执行三路合并
        for block_id in &all_ids {
            let base_block = base_map.get(block_id);
            let source_block = source_map.get(block_id);
            let target_block = target_map.get(block_id);

            let merged_block = self.merge_semantic_block(
                block_id,
                base_block.map(|b| &**b),
                source_block.map(|s| &**s),
                target_block.map(|t| &**t),
            )?;

            match merged_block {
                SemanticMergeOutcome::Merged(block) => {
                    merged_blocks.push(*block);
                }
                SemanticMergeOutcome::Conflict(block, conflict) => {
                    merged_blocks.push(*block);
                    conflicts.push(conflict);
                }
            }
        }

        Ok(SemanticMergeResult {
            merged_blocks,
            conflicts: conflicts.clone(),
            success: conflicts.is_empty(),
        })
    }

    /// 解析语义块（简化版本）
    fn parse_semantic_blocks(&self, branch: &ContextBranch) -> Result<Vec<SemanticBlock>> {
        let mut blocks = Vec::new();

        // 遍历短期层的所有文件
        if branch.short_term_dir.exists() {
            for entry in std::fs::read_dir(&branch.short_term_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_file() {
                    let content = std::fs::read_to_string(&path)?;
                    let file_name = path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();

                    // 简化：每个文件作为一个语义块
                    let content_clone = content.clone();
                    blocks.push(SemanticBlock {
                        id: file_name.clone(),
                        content,
                        block_type: "file".to_string(),
                        start_line: 0,
                        end_line: content_clone.lines().count(),
                        dependencies: Vec::new(),
                    });
                }
            }
        }

        Ok(blocks)
    }

    /// 合并单个语义块
    fn merge_semantic_block(
        &self,
        block_id: &str,
        base: Option<&SemanticBlock>,
        source: Option<&SemanticBlock>,
        target: Option<&SemanticBlock>,
    ) -> Result<SemanticMergeOutcome> {
        match (source, target, base) {
            (Some(s), Some(t), Some(b)) if s.content == t.content && t.content == b.content => {
                // 三者相同
                Ok(SemanticMergeOutcome::Merged(Box::new(s.clone())))
            }
            (Some(s), Some(t), _) if s.content == t.content => {
                // 源和目标相同
                Ok(SemanticMergeOutcome::Merged(Box::new(s.clone())))
            }
            (Some(s), Some(t), Some(b)) if s.content == b.content => {
                // 只有目标修改
                Ok(SemanticMergeOutcome::Merged(Box::new(t.clone())))
            }
            (Some(s), Some(t), Some(b)) if t.content == b.content => {
                // 只有源修改
                Ok(SemanticMergeOutcome::Merged(Box::new(s.clone())))
            }
            (Some(s), Some(t), Some(b)) => {
                // 三者都不同，使用 diff3
                let diff3_result =
                    self.diff3_merge(&b.content, &s.content, &t.content)?;

                if diff3_result.success {
                    let mut merged_block = s.clone();
                    merged_block.content = diff3_result.merged_content;
                    Ok(SemanticMergeOutcome::Merged(Box::new(merged_block)))
                } else {
                    // 有冲突
                    let mut merged_block = s.clone();
                    merged_block.content = diff3_result.merged_content;

                    let conflict = Conflict {
                        conflict_id: format!("semantic_conflict_{}", block_id),
                        item_id: block_id.to_string(),
                        source_version: ConflictVersion {
                            hash: format!("0x{}", hex::encode(s.content.as_bytes())),
                            content_path: PathBuf::new(),
                            metadata: None,
                        },
                        target_version: ConflictVersion {
                            hash: format!("0x{}", hex::encode(t.content.as_bytes())),
                            content_path: PathBuf::new(),
                            metadata: None,
                        },
                        conflict_type: ConflictType::Content,
                        resolution: None,
                    };

                    Ok(SemanticMergeOutcome::Conflict(Box::new(merged_block), conflict))
                }
            }
            (Some(s), None, _) => Ok(SemanticMergeOutcome::Merged(Box::new(s.clone()))),
            (None, Some(t), _) => Ok(SemanticMergeOutcome::Merged(Box::new(t.clone()))),
            (None, None, _) | (Some(_), Some(_), None) => {
                // 边缘情况：都删除或无 base
                Ok(SemanticMergeOutcome::Merged(Box::new(SemanticBlock {
                    id: block_id.to_string(),
                    content: String::new(),
                    block_type: "deleted".to_string(),
                    start_line: 0,
                    end_line: 0,
                    dependencies: Vec::new(),
                })))
            }
        }
    }
}

/// diff3 合并结果
#[derive(Debug, Clone)]
pub struct Diff3Result {
    pub merged_content: String,
    pub conflicts: Vec<Conflict>,
    pub hunks: Vec<Diff3Hunk>,
    pub success: bool,
}

/// 语义合并结果
#[derive(Debug, Clone)]
pub struct SemanticMergeResult {
    pub merged_blocks: Vec<SemanticBlock>,
    pub conflicts: Vec<Conflict>,
    pub success: bool,
}

/// 语义块合并结果
#[allow(clippy::large_enum_variant)]
pub enum SemanticMergeOutcome {
    Merged(Box<SemanticBlock>),
    Conflict(Box<SemanticBlock>, Conflict),
}

/// 内容去重器
pub struct ContentDeduplicator {
    /// 内容哈希集合
    content_hashes: HashSet<String>,
    /// 重复检测计数
    duplicate_count: usize,
}

impl ContentDeduplicator {
    pub fn new() -> Self {
        Self {
            content_hashes: HashSet::new(),
            duplicate_count: 0,
        }
    }

    /// 检查并去重内容
    pub fn deduplicate(&mut self, content: &str) -> DedupResult {
        let hash = self.compute_hash(content);

        if self.content_hashes.contains(&hash) {
            self.duplicate_count += 1;
            DedupResult::Duplicate(hash)
        } else {
            self.content_hashes.insert(hash.clone());
            DedupResult::Unique(hash)
        }
    }

    /// 批量去重
    pub fn deduplicate_batch(&mut self, contents: &[&str]) -> Vec<DedupResult> {
        contents.iter().map(|c| self.deduplicate(c)).collect()
    }

    /// 计算内容哈希
    fn compute_hash(&self, content: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("0x{}", hex::encode(hasher.finalize()))
    }

    /// 获取统计信息
    pub fn stats(&self) -> DedupStats {
        DedupStats {
            unique_count: self.content_hashes.len(),
            duplicate_count: self.duplicate_count,
            dedup_ratio: if self.content_hashes.len() + self.duplicate_count > 0 {
                self.duplicate_count as f64
                    / (self.content_hashes.len() + self.duplicate_count) as f64
            } else {
                0.0
            },
        }
    }

    /// 清除状态
    pub fn clear(&mut self) {
        self.content_hashes.clear();
        self.duplicate_count = 0;
    }
}

impl Default for ContentDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// 去重结果
#[derive(Debug, Clone)]
pub enum DedupResult {
    Unique(String),
    Duplicate(String),
}

/// 去重统计
#[derive(Debug, Clone)]
pub struct DedupStats {
    pub unique_count: usize,
    pub duplicate_count: usize,
    pub dedup_ratio: f64,
}

impl std::fmt::Display for DedupStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Deduplication Statistics:")?;
        writeln!(f, "  Unique items: {}", self.unique_count)?;
        writeln!(f, "  Duplicates: {}", self.duplicate_count)?;
        writeln!(f, "  Dedup ratio: {:.2}%", self.dedup_ratio * 100.0)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lcs_computation() {
        let a = vec!["A", "B", "C", "D", "E"];
        let b = vec!["A", "C", "D", "F", "E"];

        let lcs = AdvancedMerger::compute_lcs_pairs(&a, &b);

        // LCS 应该是 ["A", "C", "D", "E"]
        assert_eq!(lcs.len(), 4);
        assert_eq!(a[lcs[0].0], "A");
        assert_eq!(a[lcs[1].0], "C");
        assert_eq!(a[lcs[2].0], "D");
        assert_eq!(a[lcs[3].0], "E");
    }

    #[test]
    fn test_diff3_merge_no_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let merger = AdvancedMerger::new(temp_dir.path(), temp_dir.path()).unwrap();

        let base = "line1\nline2\nline3";
        let source = "line1\nmodified\nline3";
        let target = "line1\nline2\nline3";

        let result = merger.diff3_merge(base, source, target).unwrap();

        assert!(result.success);
        assert!(result.conflicts.is_empty());
        assert!(result.merged_content.contains("modified"));
    }

    #[test]
    fn test_diff3_merge_with_conflict() {
        let temp_dir = TempDir::new().unwrap();
        let merger = AdvancedMerger::new(temp_dir.path(), temp_dir.path()).unwrap();

        let base = "line1\nline2\nline3";
        let source = "line1\nsource_change\nline3";
        let target = "line1\ntarget_change\nline3";

        let result = merger.diff3_merge(base, source, target).unwrap();

        assert!(!result.success);
        assert_eq!(result.conflicts.len(), 1);
        assert!(result.merged_content.contains("<<<<<<< SOURCE"));
        assert!(result.merged_content.contains(">>>>>>> TARGET"));
    }

    #[test]
    fn test_diff3_merge_same_change() {
        let temp_dir = TempDir::new().unwrap();
        let merger = AdvancedMerger::new(temp_dir.path(), temp_dir.path()).unwrap();

        let base = "line1\nline2\nline3";
        let source = "line1\nsame_change\nline3";
        let target = "line1\nsame_change\nline3";

        let result = merger.diff3_merge(base, source, target).unwrap();

        assert!(result.success);
        assert!(result.conflicts.is_empty());
        assert!(result.merged_content.contains("same_change"));
    }

    #[test]
    fn test_content_deduplicator() {
        let mut dedup = ContentDeduplicator::new();

        // 添加唯一内容
        let result1 = dedup.deduplicate("content1");
        assert!(matches!(result1, DedupResult::Unique(_)));

        // 添加重复内容
        let result2 = dedup.deduplicate("content1");
        assert!(matches!(result2, DedupResult::Duplicate(_)));

        // 添加另一个唯一内容
        let result3 = dedup.deduplicate("content2");
        assert!(matches!(result3, DedupResult::Unique(_)));

        let stats = dedup.stats();
        assert_eq!(stats.unique_count, 2);
        assert_eq!(stats.duplicate_count, 1);
        assert!((stats.dedup_ratio - 0.333).abs() < 0.01);
    }

    #[test]
    fn test_batch_deduplication() {
        let mut dedup = ContentDeduplicator::new();
        let contents = vec!["a", "b", "a", "c", "b", "a"];

        let results = dedup.deduplicate_batch(&contents);

        assert_eq!(results.len(), 6);
        assert!(matches!(results[0], DedupResult::Unique(_)));
        assert!(matches!(results[1], DedupResult::Unique(_)));
        assert!(matches!(results[2], DedupResult::Duplicate(_)));
        assert!(matches!(results[3], DedupResult::Unique(_)));
        assert!(matches!(results[4], DedupResult::Duplicate(_)));
        assert!(matches!(results[5], DedupResult::Duplicate(_)));

        let stats = dedup.stats();
        assert_eq!(stats.unique_count, 3);
        assert_eq!(stats.duplicate_count, 3);
        assert!((stats.dedup_ratio - 0.5).abs() < 0.01);
    }
}
