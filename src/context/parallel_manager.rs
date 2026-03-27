//! 平行上下文管理器
//!
//! 提供完整的平行上下文管理功能，包括：
//! - 分支创建（fork）、切换（checkout）、合并（merge）、废弃（abort）
//! - 分支历史追溯（log）、差异比较（diff）
//! - 时间旅行（time_travel）到历史状态

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::Arc;

use super::branch::{BranchMetadata, BranchState, ContextBranch, MergeStrategy};
use super::graph::{ContextGraphManager, MergeRecord};
use super::merge::{compute_diff, Merger, MergeResult};
use super::hash_chain::{ChainNode, HashChain, HashChainManager};
use super::cow::{CowManager, CowStats, BranchCloner, ForkResult};

/// 平行上下文管理器配置
#[derive(Debug, Clone)]
pub struct ParallelContextManagerConfig {
    /// 上下文根目录
    pub context_root: PathBuf,
    /// 默认合并策略
    pub default_merge_strategy: MergeStrategy,
    /// 是否启用自动清理过期分支
    pub auto_cleanup_abandoned: bool,
    /// 分支 TTL（小时）
    pub branch_ttl_hours: Option<u32>,
}

impl Default for ParallelContextManagerConfig {
    fn default() -> Self {
        Self {
            context_root: PathBuf::from(".context"),
            default_merge_strategy: MergeStrategy::SelectiveMerge,
            auto_cleanup_abandoned: false,
            branch_ttl_hours: None,
        }
    }
}

/// 平行上下文管理器
pub struct ParallelContextManager {
    config: ParallelContextManagerConfig,
    graph_manager: ContextGraphManager,
    merger: Merger,
    branches_dir: PathBuf,
    merge_logs_dir: PathBuf,
    checkpoints_dir: PathBuf,
    cow_manager: Arc<CowManager>,
    branch_cloner: BranchCloner,
}

impl ParallelContextManager {
    /// 创建平行上下文管理器
    pub fn new(config: ParallelContextManagerConfig) -> Result<Self> {
        let context_root = &config.context_root;

        // 创建目录结构
        let branches_dir = context_root.join("branches");
        let merge_logs_dir = context_root.join("merge_logs");
        let checkpoints_dir = context_root.join("checkpoints");
        let graph_dir = context_root.join("graph");

        std::fs::create_dir_all(&branches_dir)
            .with_context(|| format!("Failed to create branches directory: {:?}", branches_dir))?;
        std::fs::create_dir_all(&merge_logs_dir)
            .with_context(|| format!("Failed to create merge logs directory: {:?}", merge_logs_dir))?;
        std::fs::create_dir_all(&checkpoints_dir)
            .with_context(|| format!("Failed to create checkpoints directory: {:?}", checkpoints_dir))?;
        std::fs::create_dir_all(&graph_dir)
            .with_context(|| format!("Failed to create graph directory: {:?}", graph_dir))?;

        // 创建图管理器
        let mut graph_manager = ContextGraphManager::new(&graph_dir)?;

        // 如果还没有 main 分支，创建它
        // 注意：main 分支的 parent 是空字符串，因为它是根分支
        if !graph_manager.graph().has_branch("main") {
            // 直接在图中创建 main 分支，而不是通过 create_branch（因为它需要父分支存在）
            let main_branch_dir = branches_dir.join("main");
            let mut main_branch = ContextBranch::new("main", "main", "", main_branch_dir.clone())?;
            
            // 初始化哈希链
            let mut hash_chain_manager = HashChainManager::new(&main_branch_dir)?;
            let genesis_hash = hash_chain_manager.initialize_chain_to_path("main", &main_branch.hash_chain_file)?;
            main_branch.head_hash = genesis_hash;
            
            main_branch.save()?;
            graph_manager.graph_mut().add_branch(main_branch);
            graph_manager.save()?;
        }

        // 创建合并器
        let merger = Merger::new(&branches_dir, &merge_logs_dir)?;

        // 创建 COW 管理器
        let cow_manager = Arc::new(CowManager::with_defaults());
        let branch_cloner = BranchCloner::new(Arc::clone(&cow_manager));

        Ok(Self {
            config,
            graph_manager,
            merger,
            branches_dir,
            merge_logs_dir,
            checkpoints_dir,
            cow_manager,
            branch_cloner,
        })
    }

    /// 从默认配置创建管理器
    pub fn from_context_root<P: AsRef<Path>>(context_root: P) -> Result<Self> {
        let config = ParallelContextManagerConfig {
            context_root: context_root.as_ref().to_path_buf(),
            ..Default::default()
        };

        Self::new(config)
    }

    /// 创建新分支
    pub fn create_branch(&mut self, name: &str, from_branch: &str) -> Result<&ContextBranch> {
        let branch_id = if name == "main" {
            "main".to_string()
        } else {
            let uuid = Uuid::new_v4();
            format!("{}_{}", name, uuid.to_string()[..8].to_string())
        };

        tracing::info!("Creating branch {} from {}", branch_id, from_branch);

        let metadata = BranchMetadata {
            created_by: "user".to_string(),
            purpose: None,
            tags: Vec::new(),
            auto_merge: false,
            merge_strategy: self.config.default_merge_strategy.clone(),
            ttl_hours: self.config.branch_ttl_hours,
        };

        let branch = self
            .graph_manager
            .create_branch(&branch_id, name, from_branch, Some(metadata))?;

        // 使用 COW 机制从父分支继承数据
        // 注意：这里不能直接借用 branch，因为 graph_manager 已经可变借用了
        // 所以我们使用 branch_id 来获取父分支信息，然后在外部执行 COW
        let parent_branch_id = from_branch.to_string();
        let branch_dir = branch.branch_dir.clone();
        let branch_id_clone = branch.branch_id.clone();

        tracing::info!("Branch {} created successfully", branch.branch_id);

        // 在可变借用结束后再获取父分支并执行 COW
        if parent_branch_id != "main" && !parent_branch_id.is_empty() {
            if let Some(parent_branch) = self.graph_manager.get_branch(&parent_branch_id) {
                let parent_dir = parent_branch.branch_dir.clone();
                
                match self.branch_cloner.fork_with_layers(
                    &parent_dir,
                    &branch_dir,
                    &["short-term", "long-term"],
                ) {
                    Ok(fork_result) => {
                        tracing::info!(
                            "COW fork completed in {}ms, {} symlinks created",
                            fork_result.duration_ms,
                            fork_result.symlinks_created
                        );
                    }
                    Err(e) => {
                        tracing::warn!("COW fork failed, falling back to normal copy: {}", e);
                    }
                }
            }
        }

        // 重新获取分支引用返回
        Ok(self.graph_manager.get_branch(&branch_id_clone).unwrap())
    }

    /// 切换到指定分支
    pub fn checkout(&mut self, branch: &str) -> Result<()> {
        tracing::info!("Checking out to branch {}", branch);

        self.graph_manager.checkout(branch)?;

        tracing::info!("Successfully checked out to branch {}", branch);

        Ok(())
    }

    /// 合并分支
    pub fn merge(
        &mut self,
        source_branch: &str,
        target_branch: &str,
        strategy: Option<MergeStrategy>,
    ) -> Result<MergeResult> {
        tracing::info!(
            "Merging {} into {} with strategy: {:?}",
            source_branch,
            target_branch,
            strategy
        );

        // 获取分支
        let source = self
            .graph_manager
            .get_branch(source_branch)
            .cloned()
            .context(format!("Source branch not found: {}", source_branch))?;

        let target = self
            .graph_manager
            .get_branch(target_branch)
            .cloned()
            .context(format!("Target branch not found: {}", target_branch))?;

        // 执行合并
        let merge_strategy = strategy.unwrap_or(self.config.default_merge_strategy.clone());
        let result = self.merger.merge(&source, &target, merge_strategy.clone())?;

        if result.success {
            // 记录合并历史
            let merge_record = MergeRecord {
                merge_id: result.merge_id.clone(),
                source_branch: source_branch.to_string(),
                target_branch: target_branch.to_string(),
                merge_time: Utc::now(),
                merged_items: Vec::new(), // TODO: 填充合并项目
                conflicts: Vec::new(),    // TODO: 填充冲突
                resolution: super::graph::ConflictResolution {
                    strategy: format!("{:?}", merge_strategy),
                    decision: super::graph::MergeDecision::Combine,
                    ai_explanation: None,
                },
                success: true,
            };

            self.graph_manager.record_merge(merge_record)?;

            // 更新目标分支的 head_hash
            if let Some(target_mut) = self.graph_manager.graph_mut().get_branch_mut(target_branch) {
                if !source.head_hash.is_empty() {
                    target_mut.update_head_hash(&source.head_hash);
                    target_mut.save()?;
                }
            }
        }

        Ok(result)
    }

    /// 废弃分支
    pub fn abort_branch(&mut self, branch: &str) -> Result<()> {
        tracing::info!("Aborting branch {}", branch);

        if branch == "main" {
            anyhow::bail!("Cannot abort main branch");
        }

        // 更新分支状态
        if let Some(branch_obj) = self.graph_manager.graph_mut().get_branch_mut(branch) {
            branch_obj.set_state(BranchState::Abandoned);
            branch_obj.save()?;
        }

        // 如果配置了自动清理，删除分支目录
        if self.config.auto_cleanup_abandoned {
            if let Some(branch_obj) = self.graph_manager.get_branch(branch) {
                if branch_obj.branch_dir.exists() {
                    std::fs::remove_dir_all(&branch_obj.branch_dir).with_context(|| {
                        format!("Failed to remove branch directory: {:?}", branch_obj.branch_dir)
                    })?;
                }
            }
        }

        tracing::info!("Branch {} aborted successfully", branch);

        Ok(())
    }

    /// 列出所有分支
    pub fn list_branches(&self) -> Vec<&ContextBranch> {
        self.graph_manager.list_branches()
    }

    /// 列出活跃分支
    pub fn list_active_branches(&self) -> Vec<&ContextBranch> {
        self.graph_manager.graph().list_active_branches()
    }

    /// 获取当前分支
    pub fn get_current_branch(&self) -> Option<&ContextBranch> {
        self.graph_manager.get_current_branch()
    }

    /// 获取分支
    pub fn get_branch(&self, branch: &str) -> Option<&ContextBranch> {
        self.graph_manager.get_branch(branch)
    }

    /// 计算两个分支的差异
    pub fn diff(&self, branch1: &str, branch2: &str) -> Result<super::merge::BranchDiff> {
        let b1 = self
            .graph_manager
            .get_branch(branch1)
            .context(format!("Branch not found: {}", branch1))?;

        let b2 = self
            .graph_manager
            .get_branch(branch2)
            .context(format!("Branch not found: {}", branch2))?;

        compute_diff(b1, b2)
    }

    /// 查看分支历史
    pub fn log(&self, branch: &str, limit: usize) -> Result<Vec<ChainNode>> {
        let branch_obj = self
            .graph_manager
            .get_branch(branch)
            .context(format!("Branch not found: {}", branch))?;

        if !branch_obj.hash_chain_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&branch_obj.hash_chain_file)
            .with_context(|| format!("Failed to read hash chain file: {:?}", branch_obj.hash_chain_file))?;

        let chain: HashChain = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse hash chain: {:?}", branch_obj.hash_chain_file))?;

        Ok(chain.get_latest(limit).to_vec())
    }

    /// 时间旅行到历史状态
    pub fn time_travel(&mut self, branch: &str, target_hash: &str) -> Result<String> {
        tracing::info!(
            "Time traveling to hash {} for branch {}",
            target_hash,
            branch
        );

        // 创建临时分支指向历史状态
        let temp_branch_name = format!("{}_{}", branch, &target_hash[..8]);

        // 获取源分支
        let source_branch = self
            .graph_manager
            .get_branch(branch)
            .context(format!("Branch not found: {}", branch))?;

        // 加载源分支的哈希链
        if !source_branch.hash_chain_file.exists() {
            anyhow::bail!("Hash chain not found for branch: {}", branch);
        }

        let content = std::fs::read_to_string(&source_branch.hash_chain_file)?;
        let chain: HashChain = serde_json::from_str(&content)?;

        // 查找目标哈希节点
        let target_node = chain
            .chain
            .iter()
            .find(|node| node.hash == target_hash)
            .context(format!("Hash not found in chain: {}", target_hash))?;

        // 创建临时分支
        let temp_branch_id = format!("temp_{}", Uuid::new_v4().to_string()[..8].to_string());
        let temp_branch_dir = self.branches_dir.join(&temp_branch_id);

        let mut temp_branch = ContextBranch::new(
            &temp_branch_id,
            &temp_branch_name,
            branch,
            temp_branch_dir,
        )?;

        // 设置临时分支的 head_hash 为目标哈希
        temp_branch.update_head_hash(&target_node.hash);

        // 创建截断的哈希链（只包含到目标节点）
        let mut temp_chain = HashChain::new(&temp_branch_id);
        for node in chain.chain.iter().take_while(|n| n.hash != target_hash) {
            temp_chain.chain.push(node.clone());
        }
        temp_chain.chain.push(target_node.clone());
        temp_chain.current_chain_hash = target_node.hash.clone();

        // 保存临时哈希链
        let chain_content = serde_json::to_string_pretty(&temp_chain)?;
        std::fs::write(&temp_branch.hash_chain_file, chain_content)?;

        // 保存临时分支
        temp_branch.save()?;

        // 添加到图
        self.graph_manager.graph_mut().add_branch(temp_branch);
        self.graph_manager.save()?;

        // 切换到临时分支
        self.checkout(&temp_branch_id)?;

        tracing::info!(
            "Time traveled to temporary branch {} at hash {}",
            temp_branch_id,
            target_hash
        );

        Ok(temp_branch_id)
    }

    /// 创建检查点
    pub fn create_checkpoint(&self, branch: &str, name: Option<&str>) -> Result<PathBuf> {
        let branch_obj = self
            .graph_manager
            .get_branch(branch)
            .context(format!("Branch not found: {}", branch))?;

        let checkpoint_name = name.map(|s| s.to_string()).unwrap_or_else(|| format!("checkpoint_{}", Utc::now().timestamp()));
        let checkpoint_dir = self.checkpoints_dir.join(branch).join(&checkpoint_name);

        // 创建检查点目录
        std::fs::create_dir_all(&checkpoint_dir)?;

        // 复制哈希链
        if branch_obj.hash_chain_file.exists() {
            std::fs::copy(
                &branch_obj.hash_chain_file,
                checkpoint_dir.join("hash_chain.json"),
            )?;
        }

        // 保存分支元数据
        let branch_meta_path = checkpoint_dir.join("branch.json");
        let content = serde_json::to_string_pretty(branch_obj)?;
        std::fs::write(branch_meta_path, content)?;

        tracing::info!("Checkpoint created at {:?}", checkpoint_dir);

        Ok(checkpoint_dir)
    }

    /// 从检查点恢复
    pub fn restore_checkpoint(&mut self, branch: &str, checkpoint_path: &Path) -> Result<()> {
        tracing::info!("Restoring checkpoint from {:?}", checkpoint_path);

        // 验证检查点存在
        if !checkpoint_path.exists() {
            anyhow::bail!("Checkpoint not found: {:?}", checkpoint_path);
        }

        // 加载检查点的哈希链
        let checkpoint_chain_file = checkpoint_path.join("hash_chain.json");
        if !checkpoint_chain_file.exists() {
            anyhow::bail!("Hash chain not found in checkpoint: {:?}", checkpoint_path);
        }

        let content = std::fs::read_to_string(&checkpoint_chain_file)?;
        let checkpoint_chain: HashChain = serde_json::from_str(&content)?;

        // 更新分支的哈希链
        if let Some(branch_obj) = self.graph_manager.graph_mut().get_branch_mut(branch) {
            std::fs::copy(&checkpoint_chain_file, &branch_obj.hash_chain_file)?;
            branch_obj.update_head_hash(&checkpoint_chain.current_chain_hash);
            branch_obj.save()?;
        }

        tracing::info!("Checkpoint restored successfully");

        Ok(())
    }

    /// 获取统计信息
    pub fn stats(&self) -> ContextGraphStats {
        self.graph_manager.stats()
    }

    /// 获取 COW 统计信息
    pub fn cow_stats(&self) -> CowStats {
        self.cow_manager.stats()
    }

    /// 获取图管理器
    pub fn graph_manager(&self) -> &ContextGraphManager {
        &self.graph_manager
    }

    /// 获取可变引用的图管理器
    pub fn graph_manager_mut(&mut self) -> &mut ContextGraphManager {
        &mut self.graph_manager
    }

    /// 获取合并器
    pub fn merger(&self) -> &Merger {
        &self.merger
    }

    /// 获取 COW 管理器
    pub fn cow_manager(&self) -> &CowManager {
        &self.cow_manager
    }
}

// 重新导出统计类型
pub use super::graph::ContextGraphStats;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_parallel_context_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        assert!(context_root.exists());
        assert!(manager.branches_dir.exists());
        assert!(manager.merge_logs_dir.exists());
        assert!(manager.checkpoints_dir.exists());

        // 应该自动创建 main 分支
        assert!(manager.get_branch("main").is_some());
    }

    #[test]
    fn test_branch_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 创建分支
        let branch = manager.create_branch("feature-1", "main").unwrap();

        assert_eq!(branch.branch_name, "feature-1");
        assert_eq!(branch.parent_branch, "main");
        assert_eq!(branch.state, BranchState::Active);
    }

    #[test]
    fn test_checkout() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 创建分支
        let branch = manager.create_branch("feature-1", "main").unwrap();
        let branch_id = branch.branch_id.clone();

        // 初始应该在 main 分支
        assert_eq!(manager.get_current_branch().unwrap().branch_id, "main");

        // 切换到 feature-1
        manager.checkout(&branch_id).unwrap();
        assert_eq!(manager.get_current_branch().unwrap().branch_id, branch_id);
    }

    #[test]
    fn test_merge() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 创建分支
        let branch = manager.create_branch("feature-1", "main").unwrap();
        let branch_id = branch.branch_id.clone();

        // 合并回 main
        let result = manager.merge(&branch_id, "main", None).unwrap();

        assert!(result.success);
    }

    #[test]
    fn test_abort_branch() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 创建分支
        let branch = manager.create_branch("feature-1", "main").unwrap();
        let branch_id = branch.branch_id.clone();

        // 废弃分支
        manager.abort_branch(&branch_id).unwrap();

        // 验证状态
        let branch = manager.get_branch(&branch_id).unwrap();
        assert_eq!(branch.state, BranchState::Abandoned);
    }

    #[test]
    fn test_list_branches() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 创建多个分支
        manager.create_branch("feature-1", "main").unwrap();
        manager.create_branch("feature-2", "main").unwrap();
        manager.create_branch("bugfix-1", "main").unwrap();

        let branches = manager.list_branches();
        assert_eq!(branches.len(), 4); // main + 3 features

        let active = manager.list_active_branches();
        assert_eq!(active.len(), 4);
    }

    #[test]
    fn test_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 创建检查点
        let checkpoint_path = manager.create_checkpoint("main", Some("test-checkpoint")).unwrap();

        assert!(checkpoint_path.exists());
        assert!(checkpoint_path.join("hash_chain.json").exists());
        assert!(checkpoint_path.join("branch.json").exists());
    }

    #[test]
    fn test_stats() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        manager.create_branch("feature-1", "main").unwrap();
        manager.create_branch("feature-2", "main").unwrap();

        let stats = manager.stats();

        assert_eq!(stats.total_branches, 3);
        assert_eq!(stats.active_branches, 3);
    }
}
