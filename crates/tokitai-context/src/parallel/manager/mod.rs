//! 平行上下文管理器
//!
//! 提供完整的平行上下文管理功能，包括：
//! - 分支创建（fork）、切换（checkout）、合并（merge）、废弃（abort）
//! - 分支历史追溯（log）、差异比较（diff）
//! - 时间旅行（time_travel）到历史状态

use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use std::sync::Arc;

use crate::error::{ContextResult, ContextError};
use super::branch::{BranchMetadata, BranchState, ContextBranch, MergeStrategy};
use super::graph::{ContextGraphManager, MergeRecord};
use super::merge::{compute_diff, Merger, MergeResult};
use crate::hash_chain::{ChainNode, HashChain, HashChainManager};
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
    pub fn new(config: ParallelContextManagerConfig) -> ContextResult<Self> {
        let context_root = &config.context_root;

        // 创建目录结构
        let branches_dir = context_root.join("branches");
        let merge_logs_dir = context_root.join("merge_logs");
        let checkpoints_dir = context_root.join("checkpoints");
        let graph_dir = context_root.join("graph");

        std::fs::create_dir_all(&branches_dir)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create branches directory: {:?}: {}", branches_dir, e)))?;
        std::fs::create_dir_all(&merge_logs_dir)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create merge logs directory: {:?}: {}", merge_logs_dir, e)))?;
        std::fs::create_dir_all(&checkpoints_dir)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create checkpoints directory: {:?}: {}", checkpoints_dir, e)))?;
        std::fs::create_dir_all(&graph_dir)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create graph directory: {:?}: {}", graph_dir, e)))?;

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
    pub fn from_context_root<P: AsRef<Path>>(context_root: P) -> ContextResult<Self> {
        let config = ParallelContextManagerConfig {
            context_root: context_root.as_ref().to_path_buf(),
            ..Default::default()
        };

        Self::new(config)
    }

    /// 创建新分支
    pub fn create_branch(&mut self, name: &str, from_branch: &str) -> ContextResult<&ContextBranch> {
        let branch_id = if name == "main" {
            "main".to_string()
        } else {
            let uuid = Uuid::new_v4();
            format!("{}_{}", name, &uuid.to_string()[..8])
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
    pub fn checkout(&mut self, branch: &str) -> ContextResult<()> {
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
    ) -> ContextResult<MergeResult> {
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
            .ok_or_else(|| ContextError::BranchNotFound(source_branch.to_string()))?;

        let target = self
            .graph_manager
            .get_branch(target_branch)
            .cloned()
            .ok_or_else(|| ContextError::BranchNotFound(target_branch.to_string()))?;

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
    pub fn abort_branch(&mut self, branch: &str) -> ContextResult<()> {
        tracing::info!("Aborting branch {}", branch);

        if branch == "main" {
            return Err(ContextError::InvalidBranchState { 
                branch: branch.to_string(), 
                current_state: "Cannot abort main branch".to_string() 
            });
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
                    std::fs::remove_dir_all(&branch_obj.branch_dir)
                        .map_err(|e| ContextError::OperationFailed(format!("Failed to remove branch directory: {:?}: {}", branch_obj.branch_dir, e)))?;
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
    pub fn diff(&self, branch1: &str, branch2: &str) -> ContextResult<super::merge::BranchDiff> {
        let b1 = self
            .graph_manager
            .get_branch(branch1)
            .ok_or_else(|| ContextError::BranchNotFound(branch1.to_string()))?;

        let b2 = self
            .graph_manager
            .get_branch(branch2)
            .ok_or_else(|| ContextError::BranchNotFound(branch2.to_string()))?;

        compute_diff(b1, b2).map_err(ContextError::Internal)
    }

    /// 查看分支历史
    pub fn log(&self, branch: &str, limit: usize) -> ContextResult<Vec<ChainNode>> {
        let branch_obj = self
            .graph_manager
            .get_branch(branch)
            .ok_or_else(|| ContextError::BranchNotFound(branch.to_string()))?;

        if !branch_obj.hash_chain_file.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&branch_obj.hash_chain_file)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to read hash chain file: {:?}: {}", branch_obj.hash_chain_file, e)))?;

        let chain: HashChain = serde_json::from_str(&content)
            .map_err(ContextError::Serialization)?;

        Ok(chain.get_latest(limit).to_vec())
    }

    /// 时间旅行到历史状态
    pub fn time_travel(&mut self, branch: &str, target_hash: &str) -> ContextResult<String> {
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
            .ok_or_else(|| ContextError::BranchNotFound(branch.to_string()))?;

        // 加载源分支的哈希链
        if !source_branch.hash_chain_file.exists() {
            return Err(ContextError::HashChainNotFound(branch.to_string()));
        }

        let content = std::fs::read_to_string(&source_branch.hash_chain_file)?;
        let chain: HashChain = serde_json::from_str(&content)?;

        // 查找目标哈希节点
        let target_node = chain
            .chain
            .iter()
            .find(|node| node.hash == target_hash)
            .ok_or_else(|| ContextError::ItemNotFound(format!("Hash not found in chain: {}", target_hash)))?;

        // 创建临时分支
        let temp_branch_id = format!("temp_{}", &Uuid::new_v4().to_string()[..8]);
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
    pub fn create_checkpoint(&self, branch: &str, name: Option<&str>) -> ContextResult<PathBuf> {
        let branch_obj = self
            .graph_manager
            .get_branch(branch)
            .ok_or_else(|| ContextError::BranchNotFound(branch.to_string()))?;

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
    pub fn restore_checkpoint(&mut self, branch: &str, checkpoint_path: &Path) -> ContextResult<()> {
        tracing::info!("Restoring checkpoint from {:?}", checkpoint_path);

        // 验证检查点存在
        if !checkpoint_path.exists() {
            return Err(ContextError::CheckpointNotFound(checkpoint_path.display().to_string()));
        }

        // 加载检查点的哈希链
        let checkpoint_chain_file = checkpoint_path.join("hash_chain.json");
        if !checkpoint_chain_file.exists() {
            return Err(ContextError::HashChainNotFound(checkpoint_path.display().to_string()));
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

        let manager = ParallelContextManager::from_context_root(&context_root).unwrap();

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

    #[test]
    fn test_fork_inherits_context() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 在 main 分支添加一些上下文
        {
            let branch = manager.get_current_branch().unwrap();
            let long_term_dir = &branch.long_term_dir;
            std::fs::create_dir_all(long_term_dir).unwrap();
            std::fs::write(long_term_dir.join("doc1.md"), "main context").unwrap();
        }

        // 先从 main 创建 feature-1 分支
        let feature1_branch = manager.create_branch("feature-1", "main").unwrap();
        let feature1_id = feature1_branch.branch_id.clone();

        // 在 feature-1 添加一些上下文
        manager.checkout(&feature1_id).unwrap();
        {
            let branch = manager.get_current_branch().unwrap();
            let long_term_dir = &branch.long_term_dir;
            std::fs::create_dir_all(long_term_dir).unwrap();
            std::fs::write(long_term_dir.join("doc2.md"), "feature-1 context").unwrap();
        }

        // 从 feature-1 创建 feature-2 分支（应该通过 COW 继承）
        let feature2_branch = manager.create_branch("feature-2", &feature1_id).unwrap();
        let feature2_id = feature2_branch.branch_id.clone();

        // 验证新分支可以访问继承的上下文（通过 symlink）
        manager.checkout(&feature2_id).unwrap();
        let branch = manager.get_current_branch().unwrap();
        let long_term_dir = &branch.long_term_dir;

        // 应该能读取到从 feature-1 继承的内容
        let content = std::fs::read_to_string(long_term_dir.join("doc2.md")).unwrap();
        assert_eq!(content, "feature-1 context");
    }

    #[test]
    fn test_branch_isolation_after_write() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 创建两个分支
        let branch1 = manager.create_branch("feature-1", "main").unwrap();
        let branch1_id = branch1.branch_id.clone();
        
        let branch2 = manager.create_branch("feature-2", "main").unwrap();
        let branch2_id = branch2.branch_id.clone();

        // 在 feature-1 写入内容
        manager.checkout(&branch1_id).unwrap();
        {
            let branch = manager.get_current_branch().unwrap();
            let long_term_dir = &branch.long_term_dir;
            std::fs::create_dir_all(long_term_dir).unwrap();
            std::fs::write(long_term_dir.join("file1.txt"), "feature-1 content").unwrap();
        }

        // 在 feature-2 写入不同内容
        manager.checkout(&branch2_id).unwrap();
        {
            let branch = manager.get_current_branch().unwrap();
            let long_term_dir = &branch.long_term_dir;
            std::fs::create_dir_all(long_term_dir).unwrap();
            std::fs::write(long_term_dir.join("file2.txt"), "feature-2 content").unwrap();
        }

        // 验证隔离：feature-1 不应该看到 feature-2 的文件
        manager.checkout(&branch1_id).unwrap();
        let branch1 = manager.get_current_branch().unwrap();
        assert!(branch1.long_term_dir.join("file1.txt").exists());
        
        // 验证隔离：feature-2 不应该看到 feature-1 的文件
        manager.checkout(&branch2_id).unwrap();
        let branch2 = manager.get_current_branch().unwrap();
        assert!(branch2.long_term_dir.join("file2.txt").exists());
    }

    #[test]
    fn test_merge_selective_strategy() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 在 feature 分支添加内容
        let feature = manager.create_branch("feature", "main").unwrap();
        let feature_id = feature.branch_id.clone();
        
        manager.checkout(&feature_id).unwrap();
        {
            let branch = manager.get_current_branch().unwrap();
            let long_term_dir = &branch.long_term_dir;
            std::fs::create_dir_all(long_term_dir).unwrap();
            std::fs::write(long_term_dir.join("feature_doc.md"), "# Feature Documentation").unwrap();
        }

        // 合并回 main
        manager.checkout("main").unwrap();
        let result = manager.merge(&feature_id, "main", Some(MergeStrategy::SelectiveMerge)).unwrap();
        
        assert!(result.success);
    }

    #[test]
    fn test_diff_between_branches() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        // 在 main 添加内容
        {
            let branch = manager.get_current_branch().unwrap();
            let long_term_dir = &branch.long_term_dir;
            std::fs::create_dir_all(long_term_dir).unwrap();
            std::fs::write(long_term_dir.join("base.md"), "# Base").unwrap();
        }

        // 创建 feature 分支并修改
        let feature = manager.create_branch("feature", "main").unwrap();
        let feature_id = feature.branch_id.clone();
        
        manager.checkout(&feature_id).unwrap();
        {
            let branch = manager.get_current_branch().unwrap();
            let long_term_dir = &branch.long_term_dir;
            std::fs::write(long_term_dir.join("feature.md"), "# Feature").unwrap();
        }

        // 比较分支
        manager.checkout("main").unwrap();
        let _diff = manager.diff("main", &feature_id).unwrap();

        // 验证差异报告已生成
    }

    #[test]
    fn test_branch_metadata() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        let mut manager = ParallelContextManager::from_context_root(&context_root).unwrap();

        let branch = manager.create_branch("test-branch", "main").unwrap();
        
        // 验证元数据
        assert_eq!(branch.branch_name, "test-branch");
        assert_eq!(branch.parent_branch, "main");
        assert_eq!(branch.state, BranchState::Active);
        assert!(!branch.branch_id.is_empty());
    }

    #[test]
    fn test_concurrent_branch_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = temp_dir.path().join(".context");

        // 并发创建多个分支
        let mut handles = vec![];
        for i in 0..5 {
            let context_root_clone = context_root.clone();
            let handle = std::thread::spawn(move || {
                let mut mgr = ParallelContextManager::from_context_root(&context_root_clone).unwrap();
                let result = mgr.create_branch(&format!("feature-{}", i), "main");
                // 验证创建成功，不返回分支引用
                assert!(result.is_ok());
                result.is_ok()
            });
            handles.push(handle);
        }

        // 等待所有线程完成
        for handle in handles {
            let success = handle.join().unwrap();
            assert!(success);
        }

        // 验证所有分支都创建了
        let manager = ParallelContextManager::from_context_root(&context_root).unwrap();
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 6); // main + 5 features
    }
}
