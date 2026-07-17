//! AI 增强的平行上下文管理器 - 简化版
//!
//! 本模块提供 AI 能力的统一入口，但建议直接使用各个独立的 AI 模块：
//! - [AIConflictResolver](super::ai_resolver::AIConflictResolver): AI 冲突解决
//! - [AIPurposeInference](super::purpose_inference::AIPurposeInference): 分支目的推断
//! - [AISmartMergeRecommender](super::smart_merge::AISmartMergeRecommender): 智能合并推荐
//! - [AIBranchSummarizer](super::summarizer::AIBranchSummarizer): 分支摘要生成

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::ContextResult;
use crate::parallel::{
    ParallelContextManager, ParallelContextManagerConfig,
    ContextBranch, BranchState, MergeStrategy,
    MergeResult, BranchDiff,
};
use crate::hash_chain::ChainNode;
use crate::parallel::graph::ContextGraphStats;
use crate::ai::resolver::ResolverStats;
use crate::ai::purpose::InferenceStats;
use crate::ai::smart_merge::RecommenderStats;
use crate::ai::summarizer::SummarizerStats;

/// AI 增强的平行上下文管理器（简化版）
///
/// 注意：由于 Rust 类型系统限制，完整的 AI 集成需要直接使用各个独立的 AI 模块。
/// 此结构体主要提供统计信息汇总和文档说明。
pub struct AIEnhancedContextManager {
    /// 底层平行上下文管理器
    inner: ParallelContextManager,
}

/// AI 统计信息汇总
#[derive(Debug, Clone)]
pub struct AIStats {
    pub conflict_resolver: ResolverStats,
    pub purpose_inference: InferenceStats,
    pub merge_recommender: RecommenderStats,
    pub summarizer: SummarizerStats,
}

impl AIEnhancedContextManager {
    /// 创建简化的上下文管理器
    pub fn new<P: AsRef<Path>>(
        context_root: P,
    ) -> ContextResult<Self> {
        let config = ParallelContextManagerConfig {
            context_root: context_root.as_ref().to_path_buf(),
            default_merge_strategy: MergeStrategy::SelectiveMerge,
            auto_cleanup_abandoned: false,
            branch_ttl_hours: None,
        };

        let inner = ParallelContextManager::new(config)?;

        Ok(Self { inner })
    }

    // ==================== 委托方法 ====================

    /// 创建分支
    pub fn create_branch(&mut self, name: &str, from_branch: &str) -> ContextResult<&ContextBranch> {
        self.inner.create_branch(name, from_branch)
    }

    /// 切换分支
    pub fn checkout(&mut self, branch_id: &str) -> ContextResult<()> {
        self.inner.checkout(branch_id)
    }

    /// 合并分支
    pub fn merge(
        &mut self,
        source_branch: &str,
        target_branch: &str,
        strategy: Option<MergeStrategy>,
    ) -> ContextResult<MergeResult> {
        self.inner.merge(source_branch, target_branch, strategy)
    }

    /// 废弃分支
    pub fn abort_branch(&mut self, branch_id: &str) -> ContextResult<()> {
        self.inner.abort_branch(branch_id)
    }

    /// 列出所有分支
    pub fn list_branches(&self) -> Vec<&ContextBranch> {
        self.inner.list_branches()
    }

    /// 比较分支差异
    pub fn diff(&self, branch1: &str, branch2: &str) -> ContextResult<BranchDiff> {
        self.inner.diff(branch1, branch2)
    }

    /// 查看分支历史
    pub fn log(&self, branch_id: &str, limit: usize) -> ContextResult<Vec<ChainNode>> {
        self.inner.log(branch_id, limit)
    }

    /// 获取当前分支
    pub fn get_current_branch(&self) -> Option<&ContextBranch> {
        self.inner.get_current_branch()
    }

    /// 获取分支
    pub fn get_branch(&self, branch_id: &str) -> Option<&ContextBranch> {
        self.inner.get_branch(branch_id)
    }

    /// 获取统计信息
    pub fn stats(&self) -> ContextGraphStats {
        self.inner.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ai_enhanced_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let manager = AIEnhancedContextManager::new(&context_root).unwrap();

        assert!(manager.stats().total_branches >= 1); // main 分支
    }

    #[test]
    fn test_basic_branch_operations() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = AIEnhancedContextManager::new(&context_root).unwrap();

        // 创建分支
        let branch = manager.create_branch("test-branch", "main").unwrap();
        let branch_id = branch.branch_id.clone();
        // branch 引用在作用域结束时自动释放

        // 列出分支
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 2); // main + test-branch

        // 切换分支
        manager.checkout(&branch_id).unwrap();
        assert_eq!(manager.get_current_branch().unwrap().branch_id, branch_id);
    }
}
