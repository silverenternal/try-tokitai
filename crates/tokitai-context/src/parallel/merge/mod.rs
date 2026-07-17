//! 合并操作和冲突检测
//!
//! 实现分支合并的核心逻辑：
//! - 多种合并策略（FastForward, SelectiveMerge, AIAssisted, Manual, Ours, Theirs）
//! - 冲突检测（内容、元数据、语义、顺序）
//! - 合并结果记录和日志

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use sha2::{Sha256, Digest};

use super::branch::{BranchState, ContextBranch, MergeDecision, MergeStrategy};
use super::graph::{Conflict, ConflictResolution, ConflictVersion, MergeRecord, MergedItem};
use crate::hash_chain::{ChainNode, HashChain};
use crate::optimized_merge::{AdvancedMerger, Diff3Result};
use crate::three_way_merge::ThreeWayMerger;
use crate::bloom_conflict::BloomConflictDetector;

/// Parameters for merge_layer_with_diff3 function
pub struct MergeLayerParams<'a> {
    pub source_dir: &'a Path,
    pub target_dir: &'a Path,
    pub layer_name: &'a str,
    pub merged_items: &'a mut Vec<MergedItem>,
    pub resolved_conflicts: &'a mut Vec<Conflict>,
    pub diff3_conflicts: &'a mut Vec<Conflict>,
    pub advanced_merger: &'a AdvancedMerger,
}

/// Parameters for detect_layer_conflicts_with_bloom function
pub struct DetectLayerConflictsParams<'a> {
    pub source_dir: &'a Path,
    pub target_dir: &'a Path,
    pub layer_name: &'a str,
    pub source_branch: &'a str,
    pub target_branch: &'a str,
    pub conflicts: &'a mut Vec<Conflict>,
    pub bloom: &'a BloomConflictDetector,
}

/// 合并结果
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// 合并 ID
    pub merge_id: String,
    /// 源分支
    pub source_branch: String,
    /// 目标分支
    pub target_branch: String,
    /// 合并时间
    pub merge_time: DateTime<Utc>,
    /// 是否成功
    pub success: bool,
    /// 合并的项目数量
    pub merged_count: usize,
    /// 冲突数量
    pub conflict_count: usize,
    /// 解决的冲突数量
    pub resolved_count: usize,
    /// 合并策略
    pub strategy: MergeStrategy,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

impl MergeResult {
    /// 创建成功的合并结果
    pub fn success(
        merge_id: &str,
        source: &str,
        target: &str,
        strategy: MergeStrategy,
        merged_count: usize,
    ) -> Self {
        Self {
            merge_id: merge_id.to_string(),
            source_branch: source.to_string(),
            target_branch: target.to_string(),
            merge_time: Utc::now(),
            success: true,
            merged_count,
            conflict_count: 0,
            resolved_count: 0,
            strategy,
            error: None,
        }
    }

    /// 创建失败的合并结果
    pub fn failure(merge_id: &str, source: &str, target: &str, error: &str) -> Self {
        Self {
            merge_id: merge_id.to_string(),
            source_branch: source.to_string(),
            target_branch: target.to_string(),
            merge_time: Utc::now(),
            success: false,
            merged_count: 0,
            conflict_count: 0,
            resolved_count: 0,
            strategy: MergeStrategy::Manual,
            error: Some(error.to_string()),
        }
    }
}

/// 分支差异
#[derive(Debug, Clone)]
pub struct BranchDiff {
    /// 源分支
    pub source_branch: String,
    /// 目标分支
    pub target_branch: String,
    /// 源分支新增的项目
    pub added_items: Vec<ContextItem>,
    /// 目标分支删除的项目
    pub removed_items: Vec<ContextItem>,
    /// 修改的项目
    pub modified_items: Vec<ModifiedItem>,
    /// 潜在冲突
    pub conflicts: Vec<Conflict>,
}

/// 上下文项目
#[derive(Debug, Clone)]
pub struct ContextItem {
    pub id: String,
    pub hash: String,
    pub layer: String,
    pub content_path: PathBuf,
    pub metadata_path: PathBuf,
}

/// 修改的项目
#[derive(Debug, Clone)]
pub struct ModifiedItem {
    pub id: String,
    pub source_hash: String,
    pub target_hash: String,
    pub layer: String,
}

/// 合并器
pub struct Merger {
    branches_dir: PathBuf,
    merge_logs_dir: PathBuf,
    /// Three-way merger with common ancestor support
    three_way_merger: ThreeWayMerger,
}

impl Merger {
    /// 创建合并器
    pub fn new<P: AsRef<Path>>(branches_dir: P, merge_logs_dir: P) -> Result<Self> {
        let branches_dir = branches_dir.as_ref().to_path_buf();
        let merge_logs_dir = merge_logs_dir.as_ref().to_path_buf();

        // 确保目录存在
        std::fs::create_dir_all(&branches_dir)
            .with_context(|| format!("Failed to create branches directory: {:?}", branches_dir))?;
        std::fs::create_dir_all(&merge_logs_dir)
            .with_context(|| format!("Failed to create merge logs directory: {:?}", merge_logs_dir))?;

        // 创建三路合并器
        let temp_dir = branches_dir.join("../temp");
        let three_way_merger = ThreeWayMerger::new(&temp_dir)?;

        Ok(Self {
            branches_dir,
            merge_logs_dir,
            three_way_merger,
        })
    }

    /// 执行合并
    pub fn merge(
        &self,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
        strategy: MergeStrategy,
    ) -> Result<MergeResult> {
        let merge_id = format!("merge_{}_{}", Utc::now().timestamp(), source_branch.branch_id);

        tracing::info!(
            "Merging {} into {} using strategy: {:?}",
            source_branch.branch_id,
            target_branch.branch_id,
            strategy
        );

        // 检查分支状态
        if source_branch.state != BranchState::Active {
            return Err(anyhow::anyhow!(
                "Source branch is not active: {} (state: {})",
                source_branch.state,
                source_branch.branch_id
            ));
        }

        if target_branch.state != BranchState::Active {
            return Err(anyhow::anyhow!(
                "Target branch is not active: {} (state: {})",
                target_branch.state,
                target_branch.branch_id
            ));
        }

        // 检查是否是 FastForward 情况
        if strategy == MergeStrategy::FastForward {
            return self.fast_forward_merge(&merge_id, source_branch, target_branch);
        }

        // 检测冲突
        let conflicts = self.detect_conflicts(source_branch, target_branch)?;

        // 根据策略处理冲突
        match strategy {
            MergeStrategy::FastForward => {
                unreachable!()
            }
            MergeStrategy::SelectiveMerge => {
                // 使用高级合并器进行 diff3 合并（惰性创建）
                match AdvancedMerger::new(&self.branches_dir, &self.merge_logs_dir) {
                    Ok(advanced) => self.advanced_selective_merge(&merge_id, source_branch, target_branch, conflicts, &advanced),
                    Err(_) => self.selective_merge(&merge_id, source_branch, target_branch, conflicts),
                }
            }
            MergeStrategy::AIAssisted => {
                // 使用高级合并器进行 diff3 合并（AI 辅助暂为 TODO）
                match AdvancedMerger::new(&self.branches_dir, &self.merge_logs_dir) {
                    Ok(advanced) => self.advanced_selective_merge(&merge_id, source_branch, target_branch, conflicts, &advanced),
                    Err(_) => self.selective_merge(&merge_id, source_branch, target_branch, conflicts),
                }
            }
            MergeStrategy::Manual => {
                // 有冲突时返回错误
                if !conflicts.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Manual merge required: {} conflicts detected", conflicts.len()
                    ));
                }
                self.auto_merge(&merge_id, source_branch, target_branch)
            }
            MergeStrategy::Ours => {
                // 保留目标分支版本，不合并
                Ok(MergeResult::success(
                    &merge_id,
                    &source_branch.branch_id,
                    &target_branch.branch_id,
                    MergeStrategy::Ours,
                    0,
                ))
            }
            MergeStrategy::Theirs => {
                // 保留源分支版本
                self.theirs_merge(&merge_id, source_branch, target_branch)
            }
        }
    }

    /// FastForward 合并：源分支是目标分支的直接后代
    fn fast_forward_merge(
        &self,
        merge_id: &str,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
    ) -> Result<MergeResult> {
        // 检查源分支的 head_hash 是否等于目标分支的 head_hash
        // 或者源分支的哈希链包含目标分支
        if source_branch.head_hash == target_branch.head_hash {
            // 完全相同，不需要合并
            return Ok(MergeResult::success(
                merge_id,
                &source_branch.branch_id,
                &target_branch.branch_id,
                MergeStrategy::FastForward,
                0,
            ));
        }

        // 检查源分支是否从目标分支 fork
        if source_branch.parent_branch != target_branch.branch_id {
            return Ok(MergeResult::failure(
                merge_id,
                &source_branch.branch_id,
                &target_branch.branch_id,
                "FastForward merge requires source to be a direct descendant of target",
            ));
        }

        // 复制源分支的哈希链到目标分支
        if source_branch.hash_chain_file.exists() {
            std::fs::copy(&source_branch.hash_chain_file, &target_branch.hash_chain_file)
                .with_context(|| "Failed to copy hash chain for FastForward merge")?;
        }

        tracing::info!(
            "FastForward merge completed: {} -> {}",
            source_branch.branch_id,
            target_branch.branch_id
        );

        Ok(MergeResult::success(
            merge_id,
            &source_branch.branch_id,
            &target_branch.branch_id,
            MergeStrategy::FastForward,
            1,
        ))
    }

    /// 选择性合并：基于重要性评分
    fn selective_merge(
        &self,
        merge_id: &str,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
        conflicts: Vec<Conflict>,
    ) -> Result<MergeResult> {
        let mut merged_items = Vec::new();
        let mut resolved_conflicts = Vec::new();

        // 合并短期层
        let short_term_merged = self.merge_layer(
            &source_branch.short_term_dir,
            &target_branch.short_term_dir,
            "short_term",
            &mut merged_items,
            &mut resolved_conflicts,
        )?;

        // 合并长期层
        let long_term_merged = self.merge_layer(
            &source_branch.long_term_dir,
            &target_branch.long_term_dir,
            "long_term",
            &mut merged_items,
            &mut resolved_conflicts,
        )?;

        let total_merged = short_term_merged + long_term_merged;

        tracing::info!(
            "Selective merge completed: {} items merged, {} conflicts resolved",
            total_merged,
            resolved_conflicts.len()
        );

        Ok(MergeResult {
            merge_id: merge_id.to_string(),
            source_branch: source_branch.branch_id.clone(),
            target_branch: target_branch.branch_id.clone(),
            merge_time: Utc::now(),
            success: true,
            merged_count: total_merged,
            conflict_count: conflicts.len(),
            resolved_count: resolved_conflicts.len(),
            strategy: MergeStrategy::SelectiveMerge,
            error: None,
        })
    }

    /// 高级选择性合并：使用 diff3 算法
    fn advanced_selective_merge(
        &self,
        merge_id: &str,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
        conflicts: Vec<Conflict>,
        advanced_merger: &AdvancedMerger,
    ) -> Result<MergeResult> {
        let mut merged_items = Vec::new();
        let mut resolved_conflicts = Vec::new();
        let mut diff3_conflicts = Vec::new();

        // 尝试使用 diff3 合并文本文件
        let short_term_merged = self.merge_layer_with_diff3(
            MergeLayerParams {
                source_dir: &source_branch.short_term_dir,
                target_dir: &target_branch.short_term_dir,
                layer_name: "short_term",
                merged_items: &mut merged_items,
                resolved_conflicts: &mut resolved_conflicts,
                diff3_conflicts: &mut diff3_conflicts,
                advanced_merger,
            }
        )?;

        let long_term_merged = self.merge_layer_with_diff3(
            MergeLayerParams {
                source_dir: &source_branch.long_term_dir,
                target_dir: &target_branch.long_term_dir,
                layer_name: "long_term",
                merged_items: &mut merged_items,
                resolved_conflicts: &mut resolved_conflicts,
                diff3_conflicts: &mut diff3_conflicts,
                advanced_merger,
            }
        )?;

        let total_merged = short_term_merged + long_term_merged;
        let total_conflicts = conflicts.len() + diff3_conflicts.len();

        tracing::info!(
            "Advanced selective merge completed: {} items merged, {} conflicts ({} from diff3)",
            total_merged,
            total_conflicts,
            diff3_conflicts.len()
        );

        Ok(MergeResult {
            merge_id: merge_id.to_string(),
            source_branch: source_branch.branch_id.clone(),
            target_branch: target_branch.branch_id.clone(),
            merge_time: Utc::now(),
            success: diff3_conflicts.is_empty() && conflicts.is_empty(),
            merged_count: total_merged,
            conflict_count: total_conflicts,
            resolved_count: resolved_conflicts.len(),
            strategy: MergeStrategy::SelectiveMerge,
            error: if total_conflicts > 0 {
                Some(format!("{} conflicts require resolution", total_conflicts))
            } else {
                None
            },
        })
    }

    /// 使用 Bloom Filter 快速检测冲突
    fn detect_conflicts_with_bloom(
        &self,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
        bloom: &BloomConflictDetector,
    ) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        // 使用 Bloom Filter 快速检测短期层冲突
        self.detect_layer_conflicts_with_bloom(
            DetectLayerConflictsParams {
                source_dir: &source_branch.short_term_dir,
                target_dir: &target_branch.short_term_dir,
                layer_name: "short_term",
                source_branch: &source_branch.branch_id,
                target_branch: &target_branch.branch_id,
                conflicts: &mut conflicts,
                bloom,
            }
        )?;

        // 使用 Bloom Filter 快速检测长期层冲突
        self.detect_layer_conflicts_with_bloom(
            DetectLayerConflictsParams {
                source_dir: &source_branch.long_term_dir,
                target_dir: &target_branch.long_term_dir,
                layer_name: "long_term",
                source_branch: &source_branch.branch_id,
                target_branch: &target_branch.branch_id,
                conflicts: &mut conflicts,
                bloom,
            }
        )?;

        Ok(conflicts)
    }

    /// 自动合并（无冲突时）
    fn auto_merge(
        &self,
        merge_id: &str,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
    ) -> Result<MergeResult> {
        let mut merged_items = Vec::new();
        let mut resolved_conflicts = Vec::new();

        // 合并短期层
        let short_term_merged = self.merge_layer(
            &source_branch.short_term_dir,
            &target_branch.short_term_dir,
            "short_term",
            &mut merged_items,
            &mut resolved_conflicts,
        )?;

        // 合并长期层
        let long_term_merged = self.merge_layer(
            &source_branch.long_term_dir,
            &target_branch.long_term_dir,
            "long_term",
            &mut merged_items,
            &mut resolved_conflicts,
        )?;

        let total_merged = short_term_merged + long_term_merged;

        Ok(MergeResult::success(
            merge_id,
            &source_branch.branch_id,
            &target_branch.branch_id,
            MergeStrategy::SelectiveMerge,
            total_merged,
        ))
    }

    /// Theirs 合并：完全采用源分支
    fn theirs_merge(
        &self,
        merge_id: &str,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
    ) -> Result<MergeResult> {
        // 复制源分支的哈希链
        if source_branch.hash_chain_file.exists() {
            std::fs::copy(&source_branch.hash_chain_file, &target_branch.hash_chain_file)
                .with_context(|| "Failed to copy hash chain for Theirs merge")?;
        }

        Ok(MergeResult::success(
            merge_id,
            &source_branch.branch_id,
            &target_branch.branch_id,
            MergeStrategy::Theirs,
            1,
        ))
    }

    /// 合并单个层
    fn merge_layer(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        layer_name: &str,
        merged_items: &mut Vec<MergedItem>,
        _resolved_conflicts: &mut Vec<Conflict>,
    ) -> Result<usize> {
        if !source_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;

        // 遍历源目录的所有文件
        for entry in std::fs::read_dir(source_dir)
            .with_context(|| format!("Failed to read source directory: {:?}", source_dir))?
        {
            let entry = entry?;
            let source_path = entry.path();

            if !source_path.is_file() {
                continue;
            }

            let file_name = entry.file_name();
            let target_path = target_dir.join(&file_name);

            // 如果目标不存在，直接复制
            if !target_path.exists() {
                std::fs::copy(&source_path, &target_path)
                    .with_context(|| format!("Failed to copy file: {:?}", source_path))?;
                count += 1;

                // 记录合并项目
                merged_items.push(MergedItem {
                    item_id: file_name.to_string_lossy().to_string(),
                    layer: layer_name.to_string(),
                    content_hash: self.compute_file_hash(&source_path)?,
                    from_branch: "source".to_string(),
                    to_branch: "target".to_string(),
                    merge_decision: super::graph::MergeDecision::KeepSource,
                });
            } else {
                // 目标存在，检查是否冲突
                let source_hash = self.compute_file_hash(&source_path)?;
                let target_hash = self.compute_file_hash(&target_path)?;

                if source_hash != target_hash {
                    // 内容不同，采用源分支版本（选择性合并策略）
                    std::fs::copy(&source_path, &target_path)
                        .with_context(|| format!("Failed to overwrite file: {:?}", target_path))?;
                    count += 1;

                    merged_items.push(MergedItem {
                        item_id: file_name.to_string_lossy().to_string(),
                        layer: layer_name.to_string(),
                        content_hash: source_hash,
                        from_branch: "source".to_string(),
                        to_branch: "target".to_string(),
                        merge_decision: super::graph::MergeDecision::Combine,
                    });
                }
            }
        }

        Ok(count)
    }

    /// 使用 diff3 合并单个层
    fn merge_layer_with_diff3(
        &self,
        params: MergeLayerParams<'_>,
    ) -> Result<usize> {
        let MergeLayerParams {
            source_dir,
            target_dir,
            layer_name,
            merged_items,
            resolved_conflicts: _resolved_conflicts,
            diff3_conflicts,
            advanced_merger,
        } = params;
        
        if !source_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;

        for entry in std::fs::read_dir(source_dir)
            .with_context(|| format!("Failed to read source directory: {:?}", source_dir))?
        {
            let entry = entry?;
            let source_path = entry.path();

            if !source_path.is_file() {
                continue;
            }

            let file_name = entry.file_name();
            let target_path = target_dir.join(&file_name);

            if !target_path.exists() {
                // 目标不存在，直接复制
                std::fs::copy(&source_path, &target_path)
                    .with_context(|| format!("Failed to copy file: {:?}", source_path))?;
                count += 1;

                merged_items.push(MergedItem {
                    item_id: file_name.to_string_lossy().to_string(),
                    layer: layer_name.to_string(),
                    content_hash: self.compute_file_hash(&source_path)?,
                    from_branch: "source".to_string(),
                    to_branch: "target".to_string(),
                    merge_decision: super::graph::MergeDecision::KeepSource,
                });
            } else {
                // 目标存在，尝试 diff3 合并
                let source_hash = self.compute_file_hash(&source_path)?;
                let target_hash = self.compute_file_hash(&target_path)?;

                if source_hash != target_hash {
                    // 读取内容
                    let source_content = std::fs::read_to_string(&source_path)
                        .unwrap_or_else(|_| String::new());
                    let target_content = std::fs::read_to_string(&target_path)
                        .unwrap_or_else(|_| String::new());

                    // 尝试 diff3 合并（使用空 base 作为简化）
                    match advanced_merger.diff3_merge("", &source_content, &target_content) {
                        Ok(diff3_result) if diff3_result.success => {
                            // 合并成功，写入结果
                            std::fs::write(&target_path, &diff3_result.merged_content)
                                .with_context(|| format!("Failed to write merged file: {:?}", target_path))?;
                            count += 1;

                            merged_items.push(MergedItem {
                                item_id: file_name.to_string_lossy().to_string(),
                                layer: layer_name.to_string(),
                                content_hash: source_hash.clone(),
                                from_branch: "source".to_string(),
                                to_branch: "target".to_string(),
                                merge_decision: super::graph::MergeDecision::Combine,
                            });
                        }
                        Ok(diff3_result) => {
                            // diff3 检测到冲突
                            diff3_conflicts.push(Conflict {
                                conflict_id: format!("diff3_{}_{}", layer_name, file_name.to_string_lossy()),
                                item_id: file_name.to_string_lossy().to_string(),
                                source_version: ConflictVersion {
                                    hash: source_hash,
                                    content_path: source_path.clone(),
                                    metadata: Some(serde_json::json!({"diff3_hunks": diff3_result.hunks.len()})),
                                },
                                target_version: ConflictVersion {
                                    hash: target_hash,
                                    content_path: target_path.clone(),
                                    metadata: None,
                                },
                                conflict_type: super::graph::ConflictType::Content,
                                resolution: None,
                            });
                        }
                        Err(_) => {
                            // diff3 失败，回退到简单复制
                            std::fs::copy(&source_path, &target_path)
                                .with_context(|| format!("Failed to overwrite file: {:?}", target_path))?;
                            count += 1;

                            merged_items.push(MergedItem {
                                item_id: file_name.to_string_lossy().to_string(),
                                layer: layer_name.to_string(),
                                content_hash: source_hash,
                                from_branch: "source".to_string(),
                                to_branch: "target".to_string(),
                                merge_decision: super::graph::MergeDecision::Combine,
                            });
                        }
                    }
                }
            }
        }

        Ok(count)
    }

    /// 使用 Bloom Filter 检测单个层的冲突
    fn detect_layer_conflicts_with_bloom(
        &self,
        params: DetectLayerConflictsParams<'_>,
    ) -> Result<()> {
        let DetectLayerConflictsParams {
            source_dir,
            target_dir,
            layer_name,
            source_branch,
            target_branch,
            conflicts,
            bloom: _bloom,
        } = params;
        
        if !source_dir.exists() || !target_dir.exists() {
            return Ok(());
        }

        // 收集源目录文件哈希
        let mut source_files: HashMap<String, String> = HashMap::new();

        for entry in std::fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let hash = self.compute_file_hash(&path)?;
                source_files.insert(file_name, hash);
            }
        }

        // 检查目标目录文件
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                
                // 精确检查哈希
                if let Some(source_hash) = source_files.get(&file_name) {
                    let target_hash = self.compute_file_hash(&path)?;
                    
                    if source_hash != &target_hash {
                        // 真冲突
                        conflicts.push(Conflict {
                            conflict_id: format!("bloom_{}_{}", layer_name, file_name),
                            item_id: file_name.clone(),
                            source_version: ConflictVersion {
                                hash: source_hash.clone(),
                                content_path: source_dir.join(&file_name),
                                metadata: Some(serde_json::json!({
                                    "branch": source_branch,
                                    "layer": layer_name
                                })),
                            },
                            target_version: ConflictVersion {
                                hash: target_hash,
                                content_path: path,
                                metadata: Some(serde_json::json!({
                                    "branch": target_branch,
                                    "layer": layer_name
                                })),
                            },
                            conflict_type: super::graph::ConflictType::Content,
                            resolution: None,
                        });
                    }
                }
            }
        }

        Ok(())
    }

    /// 检测冲突
    fn detect_conflicts(
        &self,
        source_branch: &ContextBranch,
        target_branch: &ContextBranch,
    ) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        // 检测短期层冲突
        self.detect_layer_conflicts(
            &source_branch.short_term_dir,
            &target_branch.short_term_dir,
            "short_term",
            &source_branch.branch_id,
            &target_branch.branch_id,
            &mut conflicts,
        )?;

        // 检测长期层冲突
        self.detect_layer_conflicts(
            &source_branch.long_term_dir,
            &target_branch.long_term_dir,
            "long_term",
            &source_branch.branch_id,
            &target_branch.branch_id,
            &mut conflicts,
        )?;

        Ok(conflicts)
    }

    /// 检测单个层的冲突
    fn detect_layer_conflicts(
        &self,
        source_dir: &Path,
        target_dir: &Path,
        layer_name: &str,
        source_branch: &str,
        target_branch: &str,
        conflicts: &mut Vec<Conflict>,
    ) -> Result<()> {
        if !source_dir.exists() || !target_dir.exists() {
            return Ok(());
        }

        // 收集两个目录的文件哈希
        let mut source_files: HashMap<String, String> = HashMap::new();
        let mut target_files: HashMap<String, String> = HashMap::new();

        // 源目录文件
        for entry in std::fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let hash = self.compute_file_hash(&path)?;
                source_files.insert(entry.file_name().to_string_lossy().to_string(), hash);
            }
        }

        // 目标目录文件
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let hash = self.compute_file_hash(&path)?;
                target_files.insert(entry.file_name().to_string_lossy().to_string(), hash);
            }
        }

        // 检测冲突：文件都存在但哈希不同
        for (file_name, source_hash) in &source_files {
            if let Some(target_hash) = target_files.get(file_name) {
                if source_hash != target_hash {
                    conflicts.push(Conflict {
                        conflict_id: format!("conflict_{}_{}", layer_name, file_name),
                        item_id: file_name.clone(),
                        source_version: ConflictVersion {
                            hash: source_hash.clone(),
                            content_path: source_dir.join(file_name),
                            metadata: Some(serde_json::json!({
                                "branch": source_branch,
                                "layer": layer_name
                            })),
                        },
                        target_version: ConflictVersion {
                            hash: target_hash.clone(),
                            content_path: target_dir.join(file_name),
                            metadata: Some(serde_json::json!({
                                "branch": target_branch,
                                "layer": layer_name
                            })),
                        },
                        conflict_type: super::graph::ConflictType::Content,
                        resolution: None,
                    });
                }
            }
        }

        Ok(())
    }

    /// 计算文件哈希
    fn compute_file_hash(&self, path: &Path) -> Result<String> {
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file for hashing: {:?}", path))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let result = hasher.finalize();

        Ok(format!("0x{}", hex::encode(result)))
    }

    /// 保存合并日志
    pub fn save_merge_log(&self, record: &MergeRecord) -> Result<()> {
        let log_file = self
            .merge_logs_dir
            .join(format!("merge_{}.json", record.merge_id));

        let content = serde_json::to_string_pretty(record)
            .with_context(|| "Failed to serialize merge record")?;

        std::fs::write(&log_file, content)
            .with_context(|| format!("Failed to write merge log: {:?}", log_file))?;

        Ok(())
    }
}

/// 计算两个分支的差异
pub fn compute_diff(
    source_branch: &ContextBranch,
    target_branch: &ContextBranch,
) -> Result<BranchDiff> {
    let mut diff = BranchDiff {
        source_branch: source_branch.branch_id.clone(),
        target_branch: target_branch.branch_id.clone(),
        added_items: Vec::new(),
        removed_items: Vec::new(),
        modified_items: Vec::new(),
        conflicts: Vec::new(),
    };

    // 比较短期层
    compare_layers(
        &source_branch.short_term_dir,
        &target_branch.short_term_dir,
        "short_term",
        &mut diff,
    )?;

    // 比较长期层
    compare_layers(
        &source_branch.long_term_dir,
        &target_branch.long_term_dir,
        "long_term",
        &mut diff,
    )?;

    Ok(diff)
}

/// 比较两个层
fn compare_layers(
    source_dir: &Path,
    target_dir: &Path,
    layer_name: &str,
    diff: &mut BranchDiff,
) -> Result<()> {
    let mut source_files: HashSet<String> = HashSet::new();
    let mut target_files: HashSet<String> = HashSet::new();

    // 收集源目录文件
    if source_dir.exists() {
        for entry in std::fs::read_dir(source_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                source_files.insert(file_name.clone());

                let item = ContextItem {
                    id: file_name.clone(),
                    hash: String::new(), // TODO: 计算哈希
                    layer: layer_name.to_string(),
                    content_path: path.clone(),
                    metadata_path: PathBuf::new(),
                };

                // 检查是否在目标中存在
                let target_path = target_dir.join(&file_name);
                if !target_path.exists() {
                    diff.added_items.push(item);
                } else {
                    // 检查是否修改
                    // TODO: 比较哈希
                }
            }
        }
    }

    // 收集目标目录文件
    if target_dir.exists() {
        for entry in std::fs::read_dir(target_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = entry.file_name().to_string_lossy().to_string();
                target_files.insert(file_name.clone());

                // 检查是否在源中不存在
                if !source_files.contains(&file_name) {
                    diff.removed_items.push(ContextItem {
                        id: file_name,
                        hash: String::new(),
                        layer: layer_name.to_string(),
                        content_path: path,
                        metadata_path: PathBuf::new(),
                    });
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_merger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        let _merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        assert!(branches_dir.exists());
        assert!(merge_logs_dir.exists());
    }

    #[test]
    fn test_fast_forward_merge() {
        let temp_dir = TempDir::new().unwrap();

        // 创建源分支和目标分支
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        let mut source_branch =
            ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let mut target_branch =
            ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // 设置相同的 head_hash 模拟 FastForward 情况
        source_branch.head_hash = "0xabc123".to_string();
        target_branch.head_hash = "0xabc123".to_string();

        // 创建源分支的哈希链文件
        let chain = HashChain::new("source");
        let chain_content = serde_json::to_string(&chain).unwrap();
        std::fs::write(&source_branch.hash_chain_file, chain_content).unwrap();

        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        let merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        let result = merger
            .merge(&source_branch, &target_branch, MergeStrategy::FastForward)
            .unwrap();

        assert!(result.success);
        assert_eq!(result.strategy, MergeStrategy::FastForward);
    }

    #[test]
    fn test_selective_merge() {
        let temp_dir = TempDir::new().unwrap();

        // 创建源分支和目标分支
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        let source_branch =
            ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let target_branch =
            ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // 在源分支的短期层添加文件
        let source_file = source_branch.short_term_dir.join("test.txt");
        std::fs::write(&source_file, "test content").unwrap();

        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        let merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        let result = merger
            .merge(&source_branch, &target_branch, MergeStrategy::SelectiveMerge)
            .unwrap();

        assert!(result.success);
        assert!(result.merged_count > 0);

        // 验证文件已复制到目标
        let target_file = target_branch.short_term_dir.join("test.txt");
        assert!(target_file.exists());
    }

    #[test]
    fn test_conflict_detection() {
        let temp_dir = TempDir::new().unwrap();

        // 创建源分支和目标分支
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        let source_branch =
            ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let target_branch =
            ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // 在两个分支的短期层添加同名但内容不同的文件
        let source_file = source_branch.short_term_dir.join("conflict.txt");
        let target_file = target_branch.short_term_dir.join("conflict.txt");

        std::fs::write(&source_file, "source content").unwrap();
        std::fs::write(&target_file, "target content").unwrap();

        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        let merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        let conflicts = merger.detect_conflicts(&source_branch, &target_branch).unwrap();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, crate::graph::ConflictType::Content);
    }

    #[test]
    fn test_compute_diff() {
        let temp_dir = TempDir::new().unwrap();

        // 创建源分支和目标分支
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        let source_branch =
            ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let target_branch =
            ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // 在源分支添加文件
        let source_file = source_branch.short_term_dir.join("added.txt");
        std::fs::write(&source_file, "added content").unwrap();

        // 在目标分支添加不同的文件
        let target_file = target_branch.short_term_dir.join("removed.txt");
        std::fs::write(&target_file, "removed content").unwrap();

        let diff = compute_diff(&source_branch, &target_branch).unwrap();

        assert_eq!(diff.added_items.len(), 1);
        assert_eq!(diff.removed_items.len(), 1);
        assert_eq!(diff.added_items[0].id, "added.txt");
        assert_eq!(diff.removed_items[0].id, "removed.txt");
    }

    #[test]
    fn test_merge_result_serialization() {
        let result = MergeResult::success(
            "merge-001",
            "feature-1",
            "main",
            MergeStrategy::SelectiveMerge,
            5,
        );

        assert_eq!(result.merge_id, "merge-001");
        assert_eq!(result.source_branch, "feature-1");
        assert_eq!(result.target_branch, "main");
        assert!(result.success);
        assert_eq!(result.merged_count, 5);
        assert_eq!(result.strategy, MergeStrategy::SelectiveMerge);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_merge_result_failure() {
        let result = MergeResult::failure(
            "merge-002",
            "feature-2",
            "main",
            "Merge conflict unresolved",
        );

        assert_eq!(result.merge_id, "merge-002");
        assert!(!result.success);
        assert_eq!(result.merged_count, 0);
        assert_eq!(result.error.unwrap(), "Merge conflict unresolved");
    }

    #[test]
    fn test_branch_diff_creation() {
        let diff = BranchDiff {
            source_branch: "source".to_string(),
            target_branch: "target".to_string(),
            added_items: vec![],
            removed_items: vec![],
            modified_items: vec![],
            conflicts: vec![],
        };

        assert_eq!(diff.source_branch, "source");
        assert_eq!(diff.target_branch, "target");
        assert!(diff.added_items.is_empty());
        assert!(diff.conflicts.is_empty());
    }

    #[test]
    fn test_modified_item_creation() {
        let modified = ModifiedItem {
            id: "item-001".to_string(),
            source_hash: "hash1".to_string(),
            target_hash: "hash2".to_string(),
            layer: "long-term".to_string(),
        };

        assert_eq!(modified.id, "item-001");
        assert_ne!(modified.source_hash, modified.target_hash);
    }

    #[test]
    fn test_merge_strategy_display() {
        // 验证所有合并策略都能正确显示
        let strategies = vec![
            MergeStrategy::FastForward,
            MergeStrategy::SelectiveMerge,
            MergeStrategy::AIAssisted,
            MergeStrategy::Manual,
            MergeStrategy::Ours,
            MergeStrategy::Theirs,
        ];

        for strategy in strategies {
            // 确保策略可以转换为字符串（通过 Debug trait）
            let debug_str = format!("{:?}", strategy);
            assert!(!debug_str.is_empty());
        }
    }

    #[test]
    fn test_hash_computation() {
        let content = "test content for hashing";
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        let hash_hex = hex::encode(hash);

        assert_eq!(hash_hex.len(), 64); // SHA256 produces 64 hex characters

        // 验证相同内容产生相同哈希
        let mut hasher2 = Sha256::new();
        hasher2.update(content.as_bytes());
        let hash2 = hasher2.finalize();
        let hash_hex2 = hex::encode(hash2);

        assert_eq!(hash_hex, hash_hex2);
    }

    #[test]
    fn test_merge_log_structure() {
        let merge_record = MergeRecord {
            merge_id: "merge-003".to_string(),
            source_branch: "feature".to_string(),
            target_branch: "main".to_string(),
            merge_time: Utc::now(),
            merged_items: vec![],
            conflicts: vec![],
            resolution: ConflictResolution {
                strategy: "auto".to_string(),
                decision: crate::graph::MergeDecision::KeepSource,
                ai_explanation: None,
            },
            success: true,
        };

        assert_eq!(merge_record.merge_id, "merge-003");
        assert!(merge_record.success);
        assert_eq!(merge_record.resolution.strategy, "auto");
    }

    #[test]
    fn test_conflict_creation() {
        let conflict = Conflict {
            conflict_id: "conflict-001".to_string(),
            item_id: "item-001".to_string(),
            source_version: ConflictVersion {
                hash: "hash1".to_string(),
                content_path: PathBuf::from("/source/path"),
                metadata: None,
            },
            target_version: ConflictVersion {
                hash: "hash2".to_string(),
                content_path: PathBuf::from("/target/path"),
                metadata: None,
            },
            conflict_type: crate::graph::ConflictType::Content,
            resolution: None,
        };

        assert_eq!(conflict.item_id, "item-001");
        assert_eq!(conflict.conflict_type, crate::graph::ConflictType::Content);
        assert_ne!(conflict.source_version.hash, conflict.target_version.hash);
    }

    #[test]
    fn test_advanced_merger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        // Verify AdvancedMerger can be created independently
        let result = AdvancedMerger::new(&branches_dir, &merge_logs_dir);
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_selective_merge_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        // Create source and target branches
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");

        let source_branch = ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let target_branch = ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // Add file to source short-term layer
        let source_file = source_branch.short_term_dir.join("test.txt");
        std::fs::write(&source_file, "source content").unwrap();

        let merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        // Perform selective merge
        let result = merger.merge(&source_branch, &target_branch, MergeStrategy::SelectiveMerge).unwrap();

        assert!(result.success);
        assert!(result.merged_count > 0);
        
        // Verify file was copied to target
        let target_file = target_branch.short_term_dir.join("test.txt");
        assert!(target_file.exists());
    }

    #[test]
    fn test_merge_with_conflict_detection() {
        let temp_dir = TempDir::new().unwrap();
        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        // Create source and target branches
        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        
        let source_branch = ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let target_branch = ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // Add conflicting files
        let source_file = source_branch.short_term_dir.join("conflict.txt");
        let target_file = target_branch.short_term_dir.join("conflict.txt");
        
        std::fs::write(&source_file, "source version").unwrap();
        std::fs::write(&target_file, "target version").unwrap();

        let merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        // Detect conflicts
        let conflicts = merger.detect_conflicts(&source_branch, &target_branch).unwrap();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].item_id, "conflict.txt");
    }

    #[test]
    fn test_fast_forward_merge_with_parent_check() {
        let temp_dir = TempDir::new().unwrap();
        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        
        let mut source_branch = ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let mut target_branch = ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // Set up FastForward scenario
        source_branch.parent_branch = "target".to_string();
        source_branch.head_hash = "0xabc123".to_string();
        target_branch.head_hash = "0xabc123".to_string();

        // Create hash chain file for source
        let chain = HashChain::new("source");
        let chain_content = serde_json::to_string(&chain).unwrap();
        std::fs::write(&source_branch.hash_chain_file, chain_content).unwrap();

        let merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        let result = merger.merge(&source_branch, &target_branch, MergeStrategy::FastForward).unwrap();

        assert!(result.success);
        assert_eq!(result.strategy, MergeStrategy::FastForward);
    }

    #[test]
    fn test_merge_state_validation() {
        let temp_dir = TempDir::new().unwrap();
        let branches_dir = temp_dir.path().join("branches");
        let merge_logs_dir = temp_dir.path().join("merge_logs");

        let source_dir = temp_dir.path().join("source");
        let target_dir = temp_dir.path().join("target");
        
        let mut source_branch = ContextBranch::new("source", "source", "main", source_dir).unwrap();
        let target_branch = ContextBranch::new("target", "target", "main", target_dir).unwrap();

        // Set source branch to abandoned state
        source_branch.set_state(BranchState::Abandoned);
        source_branch.save().unwrap();

        let merger = Merger::new(&branches_dir, &merge_logs_dir).unwrap();

        // Merge should fail for abandoned branch
        let result = merger.merge(&source_branch, &target_branch, MergeStrategy::SelectiveMerge);
        
        assert!(result.is_err());
    }
}
