//! 上下文图管理
//!
//! 管理所有分支及其关系，包括：
//! - 分支关系图（父子关系、合并历史）
//! - 当前活跃分支追踪
//! - 分支历史操作日志

use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::error::ContextError;

/// Result type alias for graph operations
pub type Result<T> = std::result::Result<T, ContextError>;

use super::branch::{BranchMetadata, BranchState, ContextBranch, MergeStrategy};
use crate::hash_chain::{ChainNode, HashChainManager};

/// 合并记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRecord {
    /// 合并 ID
    pub merge_id: String,
    /// 源分支
    pub source_branch: String,
    /// 目标分支
    pub target_branch: String,
    /// 合并时间
    pub merge_time: DateTime<Utc>,
    /// 合并的项目
    #[serde(default)]
    pub merged_items: Vec<MergedItem>,
    /// 冲突列表
    #[serde(default)]
    pub conflicts: Vec<Conflict>,
    /// 解决策略
    pub resolution: ConflictResolution,
    /// 是否成功
    pub success: bool,
}

/// 合并的项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedItem {
    /// 项目 ID
    pub item_id: String,
    /// 存储层
    pub layer: String,
    /// 内容哈希
    pub content_hash: String,
    /// 源分支
    pub from_branch: String,
    /// 目标分支
    pub to_branch: String,
    /// 合并决策
    pub merge_decision: MergeDecision,
}

/// 合并决策
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeDecision {
    KeepSource,
    KeepTarget,
    Combine,
    Discard,
    AIResolved,
}

/// 冲突
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// 冲突 ID
    pub conflict_id: String,
    /// 项目 ID
    pub item_id: String,
    /// 源分支版本
    pub source_version: ConflictVersion,
    /// 目标分支版本
    pub target_version: ConflictVersion,
    /// 冲突类型
    pub conflict_type: ConflictType,
    /// 解决结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ConflictResolution>,
}

/// 冲突版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictVersion {
    pub hash: String,
    pub content_path: PathBuf,
    pub metadata: Option<serde_json::Value>,
}

/// 冲突类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    Content,
    Metadata,
    Semantic,
    Order,
}

/// 冲突解决策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolution {
    pub strategy: String,
    pub decision: MergeDecision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_explanation: Option<String>,
}

/// 分支点（fork 操作记录）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchPoint {
    /// 分支点 ID
    pub branch_point_id: String,
    /// 源分支
    pub source_branch: String,
    /// 新分支
    pub new_branch: String,
    /// 分支时间
    pub fork_time: DateTime<Utc>,
    /// 继承的哈希
    pub inherited_hash: String,
}

/// 上下文图数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextGraph {
    /// 所有分支
    #[serde(default)]
    pub branches: HashMap<String, ContextBranch>,
    /// 当前活跃分支
    pub current_branch: String,
    /// 主分支 ID
    pub main_branch: String,
    /// 合并历史
    #[serde(default)]
    pub merge_history: Vec<MergeRecord>,
    /// 分支点历史
    #[serde(default)]
    pub branch_points: Vec<BranchPoint>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 最后更新时间
    pub updated_at: DateTime<Utc>,
}

impl ContextGraph {
    /// 创建新的上下文图
    pub fn new(main_branch_name: &str) -> Self {
        let now = Utc::now();

        Self {
            branches: HashMap::new(),
            current_branch: main_branch_name.to_string(),
            main_branch: main_branch_name.to_string(),
            merge_history: Vec::new(),
            branch_points: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// 添加分支
    pub fn add_branch(&mut self, branch: ContextBranch) {
        self.branches.insert(branch.branch_id.clone(), branch);
        self.updated_at = Utc::now();
    }

    /// 获取分支
    pub fn get_branch(&self, branch_id: &str) -> Option<&ContextBranch> {
        self.branches.get(branch_id)
    }

    /// 检查分支是否存在
    pub fn has_branch(&self, branch_id: &str) -> bool {
        self.branches.contains_key(branch_id)
    }

    /// 获取可变分支引用
    pub fn get_branch_mut(&mut self, branch_id: &str) -> Option<&mut ContextBranch> {
        self.branches.get_mut(branch_id)
    }

    /// 切换当前分支
    pub fn checkout(&mut self, branch_id: &str) -> Result<()> {
        if !self.branches.contains_key(branch_id) {
            return Err(ContextError::BranchNotFound(branch_id.to_string()));
        }

        let branch = self.branches.get(branch_id).unwrap();
        if branch.state != BranchState::Active {
            return Err(ContextError::InvalidBranchState {
                branch: branch_id.to_string(),
                current_state: format!("{:?}", branch.state),
            });
        }

        self.current_branch = branch_id.to_string();
        self.updated_at = Utc::now();

        Ok(())
    }

    /// 记录合并操作
    pub fn record_merge(&mut self, record: MergeRecord) {
        self.merge_history.push(record);
        self.updated_at = Utc::now();
    }

    /// 记录分支点
    pub fn record_branch_point(&mut self, point: BranchPoint) {
        self.branch_points.push(point);
        self.updated_at = Utc::now();
    }

    /// 获取所有分支
    pub fn list_branches(&self) -> Vec<&ContextBranch> {
        self.branches.values().collect()
    }

    /// 获取活跃分支
    pub fn list_active_branches(&self) -> Vec<&ContextBranch> {
        self.branches
            .values()
            .filter(|b| b.state == BranchState::Active)
            .collect()
    }

    /// 获取当前分支
    pub fn get_current_branch(&self) -> Option<&ContextBranch> {
        self.branches.get(&self.current_branch)
    }

    /// 获取分支的子分支
    pub fn get_child_branches(&self, parent_branch: &str) -> Vec<&ContextBranch> {
        self.branches
            .values()
            .filter(|b| b.parent_branch == parent_branch)
            .collect()
    }

    /// 获取分支的祖先链
    pub fn get_ancestor_chain(&self, branch_id: &str) -> Vec<String> {
        let mut ancestors = Vec::new();
        let mut current = branch_id.to_string();

        while let Some(branch) = self.branches.get(&current) {
            if !branch.parent_branch.is_empty() {
                ancestors.push(branch.parent_branch.clone());
                current = branch.parent_branch.clone();
            } else {
                break;
            }
        }

        ancestors
    }

    /// 检查分支是否是另一个分支的后代
    pub fn is_descendant_of(&self, branch_id: &str, ancestor: &str) -> bool {
        self.get_ancestor_chain(branch_id).contains(&ancestor.to_string())
    }

    /// 获取两个分支的最近公共祖先
    pub fn find_common_ancestor(&self, branch1: &str, branch2: &str) -> Option<String> {
        let ancestors1 = self.get_ancestor_chain(branch1);
        let ancestors2 = self.get_ancestor_chain(branch2);

        // 包含分支自身
        let mut all_ancestors1 = vec![branch1.to_string()];
        all_ancestors1.extend(ancestors1);

        for ancestor in &ancestors2 {
            if all_ancestors1.contains(ancestor) {
                return Some(ancestor.clone());
            }
        }

        // 如果没有公共祖先，返回 main
        if all_ancestors1.contains(&self.main_branch) {
            return Some(self.main_branch.clone());
        }

        None
    }

    /// 获取图统计信息
    pub fn stats(&self) -> ContextGraphStats {
        ContextGraphStats {
            total_branches: self.branches.len(),
            active_branches: self.list_active_branches().len(),
            merged_branches: self
                .branches
                .values()
                .filter(|b| b.state == BranchState::Merged)
                .count(),
            abandoned_branches: self
                .branches
                .values()
                .filter(|b| b.state == BranchState::Abandoned)
                .count(),
            total_merges: self.merge_history.len(),
            successful_merges: self.merge_history.iter().filter(|m| m.success).count(),
        }
    }
}

/// 上下文图统计信息
#[derive(Debug, Clone)]
pub struct ContextGraphStats {
    pub total_branches: usize,
    pub active_branches: usize,
    pub merged_branches: usize,
    pub abandoned_branches: usize,
    pub total_merges: usize,
    pub successful_merges: usize,
}

impl std::fmt::Display for ContextGraphStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Context Graph Statistics:")?;
        writeln!(f, "  Total branches: {}", self.total_branches)?;
        writeln!(f, "  Active branches: {}", self.active_branches)?;
        writeln!(f, "  Merged branches: {}", self.merged_branches)?;
        writeln!(f, "  Abandoned branches: {}", self.abandoned_branches)?;
        writeln!(f, "  Total merges: {}", self.total_merges)?;
        writeln!(f, "  Successful merges: {}", self.successful_merges)
    }
}

/// 上下文图管理器
pub struct ContextGraphManager {
    graph_dir: PathBuf,
    graph: ContextGraph,
}

impl ContextGraphManager {
    /// 创建上下文图管理器
    pub fn new<P: AsRef<Path>>(graph_dir: P) -> Result<Self> {
        let graph_dir = graph_dir.as_ref().to_path_buf();

        // 确保目录存在
        std::fs::create_dir_all(&graph_dir)
            .map_err(ContextError::Io)?;

        let graph_file = graph_dir.join("graph.json");

        let graph = if graph_file.exists() {
            // 从文件加载
            let content = std::fs::read_to_string(&graph_file)
                .map_err(ContextError::Io)?;

            serde_json::from_str(&content)
                .map_err(ContextError::Serialization)?
        } else {
            // 创建新图
            ContextGraph::new("main")
        };

        let mut manager = Self { graph_dir, graph };

        // 如果不是新建的，需要重新加载分支数据
        if graph_file.exists() {
            manager.reload_branches()?;
        }

        Ok(manager)
    }

    /// 重新加载分支数据
    fn reload_branches(&mut self) -> Result<()> {
        let branches_dir = self.graph_dir.join("branches");

        if !branches_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(&branches_dir)
            .map_err(ContextError::Io)?
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                let branch_file = path.join("branch.json");
                if branch_file.exists() {
                    match ContextBranch::from_file(&branch_file) {
                        Ok(branch) => {
                            self.graph.branches.insert(branch.branch_id.clone(), branch);
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

    /// 保存图到文件
    pub fn save(&self) -> Result<()> {
        let graph_file = self.graph_dir.join("graph.json");

        // 创建一个简化的可序列化版本（不包含分支数据，分支数据单独存储）
        let serializable = SerializableContextGraph {
            current_branch: &self.graph.current_branch,
            main_branch: &self.graph.main_branch,
            merge_history: &self.graph.merge_history,
            branch_points: &self.graph.branch_points,
            created_at: self.graph.created_at,
            updated_at: self.graph.updated_at,
            branch_ids: self.graph.branches.keys().cloned().collect(),
        };

        let content = serde_json::to_string_pretty(&serializable)
            .map_err(ContextError::Serialization)?;

        std::fs::write(&graph_file, content)
            .map_err(ContextError::Io)?;

        Ok(())
    }

    /// 获取图
    pub fn graph(&self) -> &ContextGraph {
        &self.graph
    }

    /// 获取可变图引用
    pub fn graph_mut(&mut self) -> &mut ContextGraph {
        &mut self.graph
    }

    /// 创建新分支
    pub fn create_branch(
        &mut self,
        branch_id: &str,
        branch_name: &str,
        parent_branch: &str,
        metadata: Option<BranchMetadata>,
    ) -> Result<&ContextBranch> {
        // 验证父分支存在（如果是根分支则跳过）
        if !parent_branch.is_empty() && !self.graph.branches.contains_key(parent_branch) {
            return Err(ContextError::ParentBranchNotFound(parent_branch.to_string()));
        }

        // 检查分支是否已存在
        if self.graph.branches.contains_key(branch_id) {
            return Err(ContextError::BranchAlreadyExists(branch_id.to_string()));
        }

        let branch_dir = self.graph_dir.join("branches").join(branch_id);
        let mut branch = ContextBranch::new(branch_id, branch_name, parent_branch, branch_dir.clone())?;

        // 设置元数据
        if let Some(meta) = metadata {
            branch.set_metadata(meta);
        }

        // 从父分支继承哈希链（如果父分支存在）
        if !parent_branch.is_empty() {
            if let Some(parent) = self.graph.branches.get(parent_branch) {
                if parent.hash_chain_file.exists() {
                    std::fs::copy(&parent.hash_chain_file, &branch.hash_chain_file)
                        .map_err(ContextError::Io)?;
                    branch.head_hash = parent.head_hash.clone();
                }
            }
        } else {
            // 根分支，初始化新的哈希链
            let mut hash_chain_manager = HashChainManager::new(&branch_dir)?;
            let genesis_hash = hash_chain_manager.initialize_chain_to_path(branch_id, &branch.hash_chain_file)?;
            branch.head_hash = genesis_hash;
        }

        // 保存分支
        branch.save()?;

        // 记录分支点（如果不是根分支）
        if !parent_branch.is_empty() {
            let branch_point = BranchPoint {
                branch_point_id: format!("bp_{}_{}", branch_id, Utc::now().timestamp()),
                source_branch: parent_branch.to_string(),
                new_branch: branch_id.to_string(),
                fork_time: Utc::now(),
                inherited_hash: branch.head_hash.clone(),
            };
            self.graph.record_branch_point(branch_point);
        }

        // 添加到图
        self.graph.add_branch(branch);
        self.save()?;

        Ok(self.graph.branches.get(branch_id).unwrap())
    }

    /// 切换分支
    pub fn checkout(&mut self, branch_id: &str) -> Result<()> {
        self.graph.checkout(branch_id)?;
        self.save()?;
        Ok(())
    }

    /// 列出所有分支
    pub fn list_branches(&self) -> Vec<&ContextBranch> {
        self.graph.list_branches()
    }

    /// 获取当前分支
    pub fn get_current_branch(&self) -> Option<&ContextBranch> {
        self.graph.get_current_branch()
    }

    /// 获取分支
    pub fn get_branch(&self, branch_id: &str) -> Option<&ContextBranch> {
        self.graph.get_branch(branch_id)
    }

    /// 更新分支
    pub fn update_branch(&mut self, branch: &ContextBranch) -> Result<()> {
        branch.save()?;
        self.graph.branches.insert(branch.branch_id.clone(), branch.clone());
        self.save()?;
        Ok(())
    }

    /// 更新分支元数据
    pub fn update_branch_metadata(
        &mut self,
        branch_id: &str,
        metadata: &super::branch::BranchMetadata,
    ) -> Result<()> {
        let branch = self
            .graph
            .branches
            .get_mut(branch_id)
            .ok_or_else(|| ContextError::BranchNotFound(branch_id.to_string()))?;

        branch.metadata = metadata.clone();
        branch.save()?;
        self.save()?;

        Ok(())
    }

    /// 记录合并
    pub fn record_merge(&mut self, record: MergeRecord) -> Result<()> {
        self.graph.record_merge(record);
        self.save()?;
        Ok(())
    }

    /// 获取统计信息
    pub fn stats(&self) -> ContextGraphStats {
        self.graph.stats()
    }

    /// 获取图目录
    pub fn graph_dir(&self) -> &Path {
        &self.graph_dir
    }

    /// 获取分支目录
    pub fn branches_dir(&self) -> PathBuf {
        self.graph_dir.join("branches")
    }
}

/// 用于序列化的上下文图（简化版）
#[derive(Debug, Clone, Serialize)]
struct SerializableContextGraph<'a> {
    pub current_branch: &'a str,
    pub main_branch: &'a str,
    pub merge_history: &'a Vec<MergeRecord>,
    pub branch_points: &'a Vec<BranchPoint>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub branch_ids: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_context_graph_creation() {
        let graph = ContextGraph::new("main");

        assert_eq!(graph.main_branch, "main");
        assert_eq!(graph.current_branch, "main");
        assert!(graph.branches.is_empty());
        assert!(graph.merge_history.is_empty());
    }

    #[test]
    fn test_context_graph_branches() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = ContextGraphManager::new(temp_dir.path()).unwrap();

        // 创建 main 分支（使用空字符串作为父分支）
        manager.create_branch("main", "main", "", None).unwrap();

        // 创建特性分支
        manager
            .create_branch("feature-1", "feature-1", "main", None)
            .unwrap();
        manager
            .create_branch("feature-2", "feature-2", "main", None)
            .unwrap();

        // 列出分支
        let branches = manager.list_branches();
        assert_eq!(branches.len(), 3);

        // 获取当前分支
        let current = manager.get_current_branch().unwrap();
        assert_eq!(current.branch_id, "main");

        // 切换分支
        manager.checkout("feature-1").unwrap();
        let current = manager.get_current_branch().unwrap();
        assert_eq!(current.branch_id, "feature-1");
    }

    #[test]
    fn test_ancestor_chain() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = ContextGraphManager::new(temp_dir.path()).unwrap();

        // 创建分支层次结构
        manager.create_branch("main", "main", "", None).unwrap();
        manager
            .create_branch("feature", "feature", "main", None)
            .unwrap();
        manager
            .create_branch("sub-feature", "sub-feature", "feature", None)
            .unwrap();

        let graph = manager.graph();

        // 测试祖先链
        let ancestors = graph.get_ancestor_chain("sub-feature");
        assert_eq!(ancestors.len(), 2);
        assert!(ancestors.contains(&"feature".to_string()));
        assert!(ancestors.contains(&"main".to_string()));

        // 测试是否是后代
        assert!(graph.is_descendant_of("sub-feature", "main"));
        assert!(graph.is_descendant_of("sub-feature", "feature"));
        assert!(!graph.is_descendant_of("main", "feature"));

        // 测试公共祖先
        let common = graph.find_common_ancestor("feature", "sub-feature");
        assert_eq!(common, Some("feature".to_string()));
    }

    #[test]
    fn test_graph_stats() {
        let temp_dir = TempDir::new().unwrap();

        let mut manager = ContextGraphManager::new(temp_dir.path()).unwrap();

        manager.create_branch("main", "main", "", None).unwrap();
        manager
            .create_branch("feature-1", "feature-1", "main", None)
            .unwrap();
        manager
            .create_branch("feature-2", "feature-2", "main", None)
            .unwrap();

        // 更新一个分支状态为 Merged
        {
            let branch = manager.get_branch("feature-1").unwrap().clone();
            let mut updated = branch.clone();
            updated.set_state(BranchState::Merged);
            manager.update_branch(&updated).unwrap();
        }

        let stats = manager.stats();
        assert_eq!(stats.total_branches, 3);
        assert_eq!(stats.active_branches, 2);
        assert_eq!(stats.merged_branches, 1);
    }

    #[test]
    fn test_graph_persistence() {
        let temp_dir = TempDir::new().unwrap();

        // 创建图并添加数据
        {
            let mut manager = ContextGraphManager::new(temp_dir.path()).unwrap();

            manager.create_branch("main", "main", "", None).unwrap();
            manager
                .create_branch("feature", "feature", "main", None)
                .unwrap();

            manager.checkout("feature").unwrap();
        }

        // 重新加载图
        {
            let manager = ContextGraphManager::new(temp_dir.path()).unwrap();

            assert_eq!(manager.list_branches().len(), 2);
            assert_eq!(manager.get_current_branch().unwrap().branch_id, "feature");
        }
    }
}
