//! 平行上下文分支管理
//!
//! 实现 AI Agent 记忆的 Git 式分支管理，支持：
//! - 分支创建（fork）、切换（checkout）、合并（merge）、废弃（abort）
//! - 分支状态管理（Active, Merged, Abandoned, Conflicted）
//! - 分支元数据和标签系统

use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::error::{ContextResult, ContextError};
use crate::hash_chain::{HashChain, HashChainManager};

/// 分支状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchState {
    /// 活跃分支，可读写
    Active,
    /// 已合并到父分支
    Merged,
    /// 已废弃
    Abandoned,
    /// 合并冲突中
    Conflicted,
}

impl std::fmt::Display for BranchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchState::Active => write!(f, "active"),
            BranchState::Merged => write!(f, "merged"),
            BranchState::Abandoned => write!(f, "abandoned"),
            BranchState::Conflicted => write!(f, "conflicted"),
        }
    }
}

/// 合并策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MergeStrategy {
    /// 源分支是目标分支的直接后代，直接移动指针
    FastForward,
    /// 选择性合并（基于重要性评分）
    #[default]
    SelectiveMerge,
    /// AI 辅助决策冲突解决
    AIAssisted,
    /// 用户手动解决所有冲突
    Manual,
    /// 始终保留目标分支版本
    Ours,
    /// 始终保留源分支版本
    Theirs,
}


impl std::fmt::Display for MergeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MergeStrategy::FastForward => write!(f, "fast_forward"),
            MergeStrategy::SelectiveMerge => write!(f, "selective_merge"),
            MergeStrategy::AIAssisted => write!(f, "ai_assisted"),
            MergeStrategy::Manual => write!(f, "manual"),
            MergeStrategy::Ours => write!(f, "ours"),
            MergeStrategy::Theirs => write!(f, "theirs"),
        }
    }
}

/// 合并决策
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeDecision {
    /// 保留源分支版本
    KeepSource,
    /// 保留目标分支版本
    KeepTarget,
    /// 合并两者
    Combine,
    /// 丢弃
    Discard,
    /// AI 辅助解决
    AIResolved,
}

/// 冲突类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// 内容冲突
    ContentConflict,
    /// 元数据冲突
    MetadataConflict,
    /// 语义冲突（AI 判断）
    SemanticConflict,
    /// 顺序冲突
    OrderConflict,
}

/// 分支元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchMetadata {
    /// 创建者（用户或 Agent）
    pub created_by: String,
    /// 分支目的描述
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
    /// 是否自动合并
    #[serde(default)]
    pub auto_merge: bool,
    /// 合并策略
    #[serde(default)]
    pub merge_strategy: MergeStrategy,
    /// 生存时间（可选自动清理，单位：小时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_hours: Option<u32>,
}

impl Default for BranchMetadata {
    fn default() -> Self {
        Self {
            created_by: "unknown".to_string(),
            purpose: None,
            tags: Vec::new(),
            auto_merge: false,
            merge_strategy: MergeStrategy::SelectiveMerge,
            ttl_hours: None,
        }
    }
}

/// 分支上下文数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextBranch {
    /// 分支唯一标识
    pub branch_id: String,
    /// 人类可读名称（如 'feature-refactor'）
    pub branch_name: String,
    /// 父分支 ID
    pub parent_branch: String,
    /// 分支创建时间
    pub fork_point: DateTime<Utc>,
    /// 当前哈希链头
    pub head_hash: String,
    /// 分支状态
    pub state: BranchState,
    /// 分支元数据
    pub metadata: BranchMetadata,
    /// 分支目录路径
    #[serde(skip)]
    pub branch_dir: PathBuf,
    /// 瞬时层目录
    #[serde(skip)]
    pub transient_dir: PathBuf,
    /// 短期层目录
    #[serde(skip)]
    pub short_term_dir: PathBuf,
    /// 长期层目录
    #[serde(skip)]
    pub long_term_dir: PathBuf,
    /// 哈希链文件路径
    #[serde(skip)]
    pub hash_chain_file: PathBuf,
}

impl ContextBranch {
    /// 创建新的分支
    pub fn new(
        branch_id: &str,
        branch_name: &str,
        parent_branch: &str,
        branch_dir: PathBuf,
    ) -> ContextResult<Self> {
        let transient_dir = branch_dir.join("transient");
        let short_term_dir = branch_dir.join("short-term");
        let long_term_dir = branch_dir.join("long-term");
        let hash_chain_file = branch_dir.join("hash_chain.json");

        // 创建目录结构
        std::fs::create_dir_all(&transient_dir)
            .map_err(ContextError::Io)?;
        std::fs::create_dir_all(&short_term_dir)
            .map_err(ContextError::Io)?;
        std::fs::create_dir_all(&long_term_dir)
            .map_err(ContextError::Io)?;

        Ok(Self {
            branch_id: branch_id.to_string(),
            branch_name: branch_name.to_string(),
            parent_branch: parent_branch.to_string(),
            fork_point: Utc::now(),
            head_hash: String::new(),
            state: BranchState::Active,
            metadata: BranchMetadata::default(),
            branch_dir,
            transient_dir,
            short_term_dir,
            long_term_dir,
            hash_chain_file,
        })
    }

    /// 从文件加载分支
    pub fn from_file(file_path: &Path) -> ContextResult<Self> {
        let content = std::fs::read_to_string(file_path)
            .map_err(ContextError::Io)?;

        let mut branch: ContextBranch = serde_json::from_str(&content)
            .map_err(ContextError::Serialization)?;

        // 重建路径
        let branch_dir = file_path.parent().unwrap().to_path_buf();
        branch.branch_dir = branch_dir.clone();
        branch.transient_dir = branch_dir.join("transient");
        branch.short_term_dir = branch_dir.join("short-term");
        branch.long_term_dir = branch_dir.join("long-term");
        branch.hash_chain_file = branch_dir.join("hash_chain.json");

        Ok(branch)
    }

    /// 保存到文件
    pub fn save(&self) -> ContextResult<()> {
        // 创建一个不含路径的序列化版本
        let serializable = SerializableBranch {
            branch_id: self.branch_id.clone(),
            branch_name: self.branch_name.clone(),
            parent_branch: self.parent_branch.clone(),
            fork_point: self.fork_point,
            head_hash: self.head_hash.clone(),
            state: self.state.clone(),
            metadata: self.metadata.clone(),
        };

        let content = serde_json::to_string_pretty(&serializable)
            .map_err(ContextError::Serialization)?;

        let branch_file = self.branch_dir.join("branch.json");
        std::fs::write(&branch_file, content)
            .map_err(ContextError::Io)?;

        Ok(())
    }

    /// 获取分支的哈希链
    pub fn load_hash_chain(&self) -> ContextResult<Option<crate::hash_chain::HashChain>> {
        if self.hash_chain_file.exists() {
            let content = std::fs::read_to_string(&self.hash_chain_file)
                .map_err(ContextError::Io)?;

            let chain: crate::hash_chain::HashChain = serde_json::from_str(&content)
                .map_err(ContextError::Serialization)?;

            Ok(Some(chain))
        } else {
            Ok(None)
        }
    }

    /// 更新哈希链头
    pub fn update_head_hash(&mut self, hash: &str) {
        self.head_hash = hash.to_string();
    }

    /// 设置元数据
    pub fn set_metadata(&mut self, metadata: BranchMetadata) {
        self.metadata = metadata;
    }

    /// 更新状态
    pub fn set_state(&mut self, state: BranchState) {
        self.state = state;
    }

    /// 检查分支是否可用
    pub fn is_available(&self) -> bool {
        self.state == BranchState::Active
    }
}

/// 用于序列化的分支结构（不包含路径）
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableBranch {
    pub branch_id: String,
    pub branch_name: String,
    pub parent_branch: String,
    pub fork_point: DateTime<Utc>,
    pub head_hash: String,
    pub state: BranchState,
    pub metadata: BranchMetadata,
}

/// 分支操作管理器
pub struct BranchManager {
    branches_dir: PathBuf,
    branches: HashMap<String, ContextBranch>,
}

impl BranchManager {
    /// 创建分支管理器
    pub fn new<P: AsRef<Path>>(branches_dir: P) -> ContextResult<Self> {
        let branches_dir = branches_dir.as_ref().to_path_buf();

        // 确保目录存在
        std::fs::create_dir_all(&branches_dir)
            .map_err(ContextError::Io)?;

        let mut manager = Self {
            branches_dir,
            branches: HashMap::new(),
        };

        // 加载现有分支
        manager.load_all_branches()?;

        Ok(manager)
    }

    /// 加载所有现有分支
    fn load_all_branches(&mut self) -> ContextResult<()> {
        if !self.branches_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&self.branches_dir)
            .map_err(ContextError::Io)?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let branch_file = path.join("branch.json");
                if branch_file.exists() {
                    match ContextBranch::from_file(&branch_file) {
                        Ok(branch) => {
                            self.branches.insert(branch.branch_id.clone(), branch);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load branch from {:?}: {}", branch_file, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 创建新分支
    pub fn create_branch(
        &mut self,
        branch_id: &str,
        branch_name: &str,
        parent_branch: &str,
    ) -> ContextResult<&ContextBranch> {
        // 检查是否已存在
        if self.branches.contains_key(branch_id) {
            return Err(ContextError::BranchAlreadyExists(branch_id.to_string()));
        }

        let branch_dir = self.branches_dir.join(branch_id);
        let mut branch = ContextBranch::new(branch_id, branch_name, parent_branch, branch_dir.clone())?;

        // 从父分支继承哈希链（如果存在）
        if let Some(parent) = self.branches.get(parent_branch) {
            if parent.hash_chain_file.exists() {
                // 复制父分支的哈希链
                std::fs::copy(&parent.hash_chain_file, &branch.hash_chain_file)
                    .map_err(ContextError::Io)?;

                // 继承父分支的 head_hash
                branch.head_hash = parent.head_hash.clone();
            }
        } else {
            // 如果没有父分支（如 main 分支），初始化新的哈希链
            let mut hash_chain_manager = HashChainManager::new(&branch_dir)?;
            let genesis_hash = hash_chain_manager.initialize_chain_to_path(branch_id, &branch.hash_chain_file)?;
            branch.head_hash = genesis_hash;
        }

        branch.save()?;
        self.branches.insert(branch_id.to_string(), branch);

        Ok(self.branches.get(branch_id).unwrap())
    }

    /// 获取分支
    pub fn get_branch(&self, branch_id: &str) -> Option<&ContextBranch> {
        self.branches.get(branch_id)
    }

    /// 获取可变分支引用
    pub fn get_branch_mut(&mut self, branch_id: &str) -> Option<&mut ContextBranch> {
        self.branches.get_mut(branch_id)
    }

    /// 列出所有分支
    pub fn list_branches(&self) -> Vec<&ContextBranch> {
        self.branches.values().collect()
    }

    /// 列出活跃分支
    pub fn list_active_branches(&self) -> Vec<&ContextBranch> {
        self.branches
            .values()
            .filter(|b| b.state == BranchState::Active)
            .collect()
    }

    /// 删除分支
    pub fn remove_branch(&mut self, branch_id: &str) -> ContextResult<()> {
        if !self.branches.contains_key(branch_id) {
            return Err(ContextError::BranchNotFound(branch_id.to_string()));
        }

        // 不允许删除 main 分支
        if branch_id == "main" {
            return Err(ContextError::OperationFailed("Cannot delete main branch".to_string()));
        }

        let branch = self.branches.get(branch_id).unwrap();
        let branch_dir = branch.branch_dir.clone();

        // 删除目录
        if branch_dir.exists() {
            std::fs::remove_dir_all(&branch_dir)
                .map_err(ContextError::Io)?;
        }

        self.branches.remove(branch_id);

        Ok(())
    }

    /// 更新分支
    pub fn update_branch(&mut self, branch: &ContextBranch) -> ContextResult<()> {
        branch.save()?;
        self.branches.insert(branch.branch_id.clone(), branch.clone());
        Ok(())
    }

    /// 获取分支数量
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// 检查分支是否存在
    pub fn has_branch(&self, branch_id: &str) -> bool {
        self.branches.contains_key(branch_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_branch_creation() {
        let temp_dir = TempDir::new().unwrap();
        let branch_dir = temp_dir.path().join("test_branch");

        let branch = ContextBranch::new("test-1", "test-branch", "main", branch_dir).unwrap();

        assert_eq!(branch.branch_id, "test-1");
        assert_eq!(branch.branch_name, "test-branch");
        assert_eq!(branch.parent_branch, "main");
        assert_eq!(branch.state, BranchState::Active);
        assert!(branch.transient_dir.exists());
        assert!(branch.short_term_dir.exists());
        assert!(branch.long_term_dir.exists());
    }

    #[test]
    fn test_branch_save_load() {
        let temp_dir = TempDir::new().unwrap();
        let branch_dir = temp_dir.path().join("test_branch");

        let mut branch = ContextBranch::new("test-1", "test-branch", "main", branch_dir).unwrap();
        branch.metadata.purpose = Some("Testing branch save/load".to_string());
        branch.metadata.tags = vec!["test".to_string(), "demo".to_string()];

        branch.save().unwrap();

        let branch_file = temp_dir.path().join("test_branch").join("branch.json");
        let loaded = ContextBranch::from_file(&branch_file).unwrap();

        assert_eq!(loaded.branch_id, "test-1");
        assert_eq!(loaded.metadata.purpose, Some("Testing branch save/load".to_string()));
        assert_eq!(loaded.metadata.tags.len(), 2);
    }

    #[test]
    fn test_branch_manager() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = BranchManager::new(temp_dir.path()).unwrap();

        // 创建 main 分支
        manager.create_branch("main", "main", "").unwrap();

        // 创建特性分支
        manager.create_branch("feature-1", "feature-1", "main").unwrap();
        manager.create_branch("feature-2", "feature-2", "main").unwrap();

        // 列出分支
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 3);

        let active = manager.list_active_branches();
        assert_eq!(active.len(), 3);

        // 获取分支
        let feature = manager.get_branch("feature-1").unwrap();
        assert_eq!(feature.branch_name, "feature-1");
        assert_eq!(feature.parent_branch, "main");

        // 更新分支状态
        {
            let feature = manager.get_branch_mut("feature-1").unwrap();
            feature.set_state(BranchState::Merged);
            feature.save().unwrap();
        }

        // 重新加载验证
        let feature = manager.get_branch("feature-1").unwrap();
        assert_eq!(feature.state, BranchState::Merged);

        // 删除分支
        manager.remove_branch("feature-2").unwrap();
        assert!(!manager.has_branch("feature-2"));
    }

    #[test]
    fn test_branch_inheritance() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = BranchManager::new(temp_dir.path()).unwrap();

        // 创建 main 分支
        manager.create_branch("main", "main", "").unwrap();

        // 更新 main 分支的 head_hash 并更新哈希链文件
        {
            let main = manager.get_branch_mut("main").unwrap();
            main.head_hash = "0xabc123".to_string();
            main.save().unwrap();
            
            // 也更新哈希链文件以保持一致
            let chain_file = &main.hash_chain_file;
            if chain_file.exists() {
                let content = std::fs::read_to_string(chain_file).unwrap();
                let mut chain: HashChain = serde_json::from_str(&content).unwrap();
                chain.current_chain_hash = "0xabc123".to_string();
                std::fs::write(chain_file, serde_json::to_string_pretty(&chain).unwrap()).unwrap();
            }
        }

        // 创建特性分支，应该继承 main 的 head_hash
        manager.create_branch("feature-1", "feature-1", "main").unwrap();

        let feature = manager.get_branch("feature-1").unwrap();
        assert_eq!(feature.head_hash, "0xabc123");
    }

    #[test]
    fn test_branch_metadata() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = BranchManager::new(temp_dir.path()).unwrap();
        manager.create_branch("main", "main", "").unwrap();

        // 创建带元数据的分支
        {
            let metadata = BranchMetadata {
                created_by: "test-user".to_string(),
                purpose: Some("Testing metadata".to_string()),
                tags: vec!["test".to_string(), "metadata".to_string()],
                auto_merge: true,
                merge_strategy: MergeStrategy::FastForward,
                ttl_hours: Some(24),
            };

            let branch = manager.get_branch_mut("main").unwrap();
            branch.set_metadata(metadata);
            branch.save().unwrap();
        }

        // 重新加载验证
        let branch = manager.get_branch("main").unwrap();
        assert_eq!(branch.metadata.created_by, "test-user");
        assert_eq!(branch.metadata.purpose, Some("Testing metadata".to_string()));
        assert_eq!(branch.metadata.tags.len(), 2);
        assert!(branch.metadata.auto_merge);
        assert_eq!(branch.metadata.merge_strategy, MergeStrategy::FastForward);
        assert_eq!(branch.metadata.ttl_hours, Some(24));
    }
}
