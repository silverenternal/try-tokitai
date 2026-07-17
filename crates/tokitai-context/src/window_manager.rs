//! 上下文窗口智能管理器
//!
//! 基于重要性的上下文保留策略，防止关键上下文被裁剪
//!
//! ## 核心功能
//! - 多维度重要性评分（时间衰减、相关性、用户引用、工具结果、决策关键性）
//! - 智能上下文裁剪
//! - 纯文件存储
//! - 与 FileContextService 集成
//!
//! ## 评分因子
//! - `recency` - 时间衰减因子
//! - `relevance` - 与当前话题相关性
//! - `user_referenced` - 用户是否引用过
//! - `tool_result` - 是否是工具执行结果
//! - `decision_critical` - 是否影响关键决策

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use anyhow::{Context, Result};
use sha2::{Sha256, Digest};

/// 上下文项重要性评分
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceScore {
    /// 总分 (0.0-1.0)
    pub total: f32,
    /// 时间衰减评分 (0.0-1.0)
    pub recency: f32,
    /// 相关性评分 (0.0-1.0)
    pub relevance: f32,
    /// 用户引用评分 (0.0 或 1.0)
    pub user_referenced: f32,
    /// 工具结果评分 (0.0 或 1.0)
    pub tool_result: f32,
    /// 决策关键性评分 (0.0-1.0)
    pub decision_critical: f32,
}

/// 上下文项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// 唯一 ID（哈希）
    pub id: String,
    /// 内容
    pub content: String,
    /// 内容类型
    pub item_type: ContextItemType,
    /// 创建时间戳
    pub created_at: u64,
    /// 最后访问时间戳
    pub last_accessed_at: u64,
    /// 重要性评分
    pub importance: ImportanceScore,
    /// 是否被用户引用
    pub user_referenced: bool,
    /// 引用次数
    pub reference_count: u32,
    /// 关联的话题标签
    pub topic_tags: Vec<String>,
    /// 关联的工具名称（如果是工具结果）
    pub tool_name: Option<String>,
    /// 是否影响关键决策
    pub is_decision_critical: bool,
    /// 原始大小（字节）
    pub size_bytes: usize,
}

/// 上下文项类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextItemType {
    /// 用户消息
    UserMessage,
    /// AI 响应
    AssistantMessage,
    /// 工具调用请求
    ToolCall,
    /// 工具执行结果
    ToolResult,
    /// 系统消息
    SystemMessage,
    /// 文件引用
    FileReference,
    /// 代码片段
    CodeSnippet,
    /// 决策记录
    DecisionRecord,
}

/// 上下文窗口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    /// 所有上下文项
    pub items: Vec<ContextItem>,
    /// 当前话题标签
    pub current_topics: HashSet<String>,
    /// 总 token 数（估算）
    pub total_tokens: usize,
    /// 最后更新时间
    pub last_updated: u64,
}

/// 上下文窗口管理器
pub struct WindowManager {
    /// 数据目录
    data_dir: PathBuf,
    /// 当前窗口状态
    state: WindowState,
    /// 配置
    config: WindowManagerConfig,
    /// 话题关键词（用于相关性计算）
    topic_keywords: HashMap<String, Vec<String>>,
}

/// 管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowManagerConfig {
    /// 最大 token 数
    pub max_tokens: usize,
    /// 时间衰减半衰期（秒）
    pub decay_half_life_seconds: u64,
    /// 评分权重
    pub weights: ImportanceWeights,
    /// 话题关键词
    pub topic_keywords: HashMap<String, Vec<String>>,
}

/// 重要性评分权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceWeights {
    /// 时间衰减权重
    pub recency: f32,
    /// 相关性权重
    pub relevance: f32,
    /// 用户引用权重
    pub user_referenced: f32,
    /// 工具结果权重
    pub tool_result: f32,
    /// 决策关键性权重
    pub decision_critical: f32,
}

impl Default for ImportanceWeights {
    fn default() -> Self {
        Self {
            recency: 0.2,
            relevance: 0.25,
            user_referenced: 0.2,
            tool_result: 0.15,
            decision_critical: 0.2,
        }
    }
}

impl Default for WindowManagerConfig {
    fn default() -> Self {
        Self {
            max_tokens: 8000, // 典型上下文窗口限制
            decay_half_life_seconds: 1800, // 30 分钟半衰期
            weights: ImportanceWeights::default(),
            topic_keywords: HashMap::new(),
        }
    }
}

impl WindowManager {
    /// 创建新的窗口管理器
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        
        let state = WindowState {
            items: Vec::new(),
            current_topics: HashSet::new(),
            total_tokens: 0,
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let mut manager = Self {
            data_dir,
            state,
            config: WindowManagerConfig::default(),
            topic_keywords: HashMap::new(),
        };
        
        // 加载已有状态
        manager.load_state().ok();
        
        Ok(manager)
    }

    /// 从配置创建
    pub fn with_config<P: AsRef<Path>>(data_dir: P, config: WindowManagerConfig) -> Result<Self> {
        let mut manager = Self::new(data_dir)?;
        let topic_keywords = config.topic_keywords.clone();
        manager.config = config;
        manager.topic_keywords = topic_keywords;
        Ok(manager)
    }

    /// 添加上下文项
    pub fn add_item(&mut self, item: ContextItem) -> Result<()> {
        // 计算重要性评分
        let importance = self.calculate_importance(&item);
        
        let mut item = item;
        item.importance = importance;
        
        // 估算 token 数（简单估算：每 4 字符约 1 token）
        item.size_bytes = item.content.len();
        let estimated_tokens = item.content.len() / 4;
        
        self.state.total_tokens += estimated_tokens;
        self.state.items.push(item);
        self.state.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 如果超出限制，进行裁剪
        if self.state.total_tokens > self.config.max_tokens {
            self.prune_context()?;
        }
        
        // 保存状态
        self.save_state()?;
        
        Ok(())
    }

    /// 创建并添加上下文项
    pub fn create_item(
        &mut self,
        content: String,
        item_type: ContextItemType,
        topic_tags: Vec<String>,
        tool_name: Option<String>,
    ) -> Result<ContextItem> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 生成 ID（内容哈希）
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        let id = hex::encode(&hash[..8]);
        
        let item = ContextItem {
            id,
            content,
            item_type,
            created_at: now,
            last_accessed_at: now,
            importance: ImportanceScore {
                total: 0.0,
                recency: 1.0,
                relevance: 0.5,
                user_referenced: 0.0,
                tool_result: 0.0,
                decision_critical: 0.0,
            },
            user_referenced: false,
            reference_count: 0,
            topic_tags,
            tool_name,
            is_decision_critical: false,
            size_bytes: 0,
        };
        
        self.add_item(item.clone())?;
        
        Ok(item)
    }

    /// 计算重要性评分
    fn calculate_importance(&self, item: &ContextItem) -> ImportanceScore {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 1. 时间衰减评分（指数衰减）
        let age_seconds = now - item.created_at;
        let half_life = self.config.decay_half_life_seconds as f32;
        let recency_score = 2.0_f32.powf(-(age_seconds as f32) / half_life);
        
        // 2. 相关性评分（基于话题匹配）
        let relevance_score = self.calculate_relevance(item);
        
        // 3. 用户引用评分
        let user_ref_score = if item.user_referenced { 1.0 } else { 0.0 };
        
        // 4. 工具结果评分
        let tool_result_score = if item.tool_name.is_some() 
            || matches!(item.item_type, ContextItemType::ToolResult) 
            { 1.0 } else { 0.0 };
        
        // 5. 决策关键性评分
        let decision_critical_score = if item.is_decision_critical { 1.0 } else { 0.5 };
        
        // 计算加权总分
        let weights = &self.config.weights;
        let total = recency_score * weights.recency
            + relevance_score * weights.relevance
            + user_ref_score * weights.user_referenced
            + tool_result_score * weights.tool_result
            + decision_critical_score * weights.decision_critical;
        
        ImportanceScore {
            total: total.min(1.0),
            recency: recency_score.min(1.0),
            relevance: relevance_score.min(1.0),
            user_referenced: user_ref_score,
            tool_result: tool_result_score,
            decision_critical: decision_critical_score,
        }
    }

    /// 计算相关性评分
    fn calculate_relevance(&self, item: &ContextItem) -> f32 {
        if self.state.current_topics.is_empty() {
            return 0.5; // 无当前话题时返回中间值
        }
        
        // 计算话题匹配度
        let matching_topics = item.topic_tags.iter()
            .filter(|tag| self.state.current_topics.contains(*tag))
            .count();
        
        if matching_topics == 0 {
            // 检查内容关键词匹配
            let content_lower = item.content.to_lowercase();
            let keyword_matches = self.topic_keywords.values()
                .flatten()
                .filter(|keyword| content_lower.contains(keyword.as_str()))
                .count();
            
            if keyword_matches > 0 {
                (keyword_matches as f32 / 10.0).min(1.0)
            } else {
                0.3
            }
        } else {
            (matching_topics as f32 / self.state.current_topics.len() as f32).min(1.0)
        }
    }

    /// 裁剪上下文
    fn prune_context(&mut self) -> Result<()> {
        // 重新计算所有项的重要性评分
        // 先收集需要更新的数据，避免借用冲突
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let half_life = self.config.decay_half_life_seconds as f32;
        let weights = self.config.weights.clone();
        let current_topics = self.state.current_topics.clone();
        let topic_keywords = self.topic_keywords.clone();
        
        for item in &mut self.state.items {
            // 1. 时间衰减评分
            let age_seconds = now - item.created_at;
            let recency_score = 2.0_f32.powf(-(age_seconds as f32) / half_life);
            
            // 2. 相关性评分
            let relevance_score = if current_topics.is_empty() {
                0.5
            } else {
                let matching_topics = item.topic_tags.iter()
                    .filter(|tag| current_topics.contains(tag.as_str()))
                    .count();
                if matching_topics == 0 {
                    let content_lower = item.content.to_lowercase();
                    let keyword_matches = topic_keywords.values()
                        .flatten()
                        .filter(|keyword| content_lower.contains(keyword.as_str()))
                        .count();
                    if keyword_matches > 0 {
                        (keyword_matches as f32 / 10.0).min(1.0)
                    } else {
                        0.3
                    }
                } else {
                    (matching_topics as f32 / current_topics.len() as f32).min(1.0)
                }
            };
            
            // 3. 用户引用评分
            let user_ref_score = if item.user_referenced { 1.0 } else { 0.0 };
            
            // 4. 工具结果评分
            let tool_result_score = if item.tool_name.is_some()
                || matches!(item.item_type, ContextItemType::ToolResult)
                { 1.0 } else { 0.0 };
            
            // 5. 决策关键性评分
            let decision_critical_score = if item.is_decision_critical { 1.0 } else { 0.5 };
            
            // 计算加权总分
            let total = (recency_score * weights.recency
                + relevance_score * weights.relevance
                + user_ref_score * weights.user_referenced
                + tool_result_score * weights.tool_result
                + decision_critical_score * weights.decision_critical).min(1.0);
            
            item.importance = ImportanceScore {
                total,
                recency: recency_score.min(1.0),
                relevance: relevance_score.min(1.0),
                user_referenced: user_ref_score,
                tool_result: tool_result_score,
                decision_critical: decision_critical_score,
            };
        }

        // 按重要性排序
        // P2-005 FIX: Use unwrap_or for partial_cmp to handle NaN safely
        self.state.items.sort_by(|a, b| {
            b.importance.total.partial_cmp(&a.importance.total).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        // 保留最重要的项，直到达到 token 限制
        let mut retained_tokens = 0;
        let mut retain_count = 0;
        
        for item in &self.state.items {
            let item_tokens = item.content.len() / 4;
            if retained_tokens + item_tokens <= self.config.max_tokens {
                retained_tokens += item_tokens;
                retain_count += 1;
            } else {
                break;
            }
        }
        
        // 移除不重要的项
        if retain_count < self.state.items.len() {
            let removed = self.state.items.split_off(retain_count);
            tracing::info!("裁剪上下文：移除{}项，保留{}项", removed.len(), retain_count);
        }
        
        self.state.total_tokens = retained_tokens;
        
        Ok(())
    }

    /// 标记项为用户引用
    pub fn mark_as_referenced(&mut self, item_id: &str) -> Result<()> {
        for item in &mut self.state.items {
            if item.id == item_id {
                item.user_referenced = true;
                item.reference_count += 1;
                item.importance.user_referenced = 1.0;
                
                // 重新计算总分
                let weights = &self.config.weights;
                item.importance.total = (
                    item.importance.recency * weights.recency
                    + item.importance.relevance * weights.relevance
                    + 1.0 * weights.user_referenced
                    + item.importance.tool_result * weights.tool_result
                    + item.importance.decision_critical * weights.decision_critical
                ).min(1.0);
                
                break;
            }
        }
        
        self.save_state()?;
        Ok(())
    }

    /// 标记项为决策关键
    pub fn mark_as_decision_critical(&mut self, item_id: &str) -> Result<()> {
        for item in &mut self.state.items {
            if item.id == item_id {
                item.is_decision_critical = true;
                item.importance.decision_critical = 1.0;
                
                // 重新计算总分
                let weights = &self.config.weights;
                item.importance.total = (
                    item.importance.recency * weights.recency
                    + item.importance.relevance * weights.relevance
                    + item.importance.user_referenced * weights.user_referenced
                    + item.importance.tool_result * weights.tool_result
                    + 1.0 * weights.decision_critical
                ).min(1.0);
                
                break;
            }
        }
        
        self.save_state()?;
        Ok(())
    }

    /// 更新当前话题
    pub fn update_current_topics(&mut self, topics: HashSet<String>) -> Result<()> {
        self.state.current_topics = topics;

        // 重新计算所有项的相关性评分
        // 先收集需要的数据，避免借用冲突
        let current_topics = self.state.current_topics.clone();
        let topic_keywords = self.topic_keywords.clone();
        let weights = self.config.weights.clone();
        
        for item in &mut self.state.items {
            // 计算相关性评分
            let relevance_score = if current_topics.is_empty() {
                0.5
            } else {
                let matching_topics = item.topic_tags.iter()
                    .filter(|tag| current_topics.contains(tag.as_str()))
                    .count();
                if matching_topics == 0 {
                    let content_lower = item.content.to_lowercase();
                    let keyword_matches = topic_keywords.values()
                        .flatten()
                        .filter(|keyword| content_lower.contains(keyword.as_str()))
                        .count();
                    if keyword_matches > 0 {
                        (keyword_matches as f32 / 10.0).min(1.0)
                    } else {
                        0.3
                    }
                } else {
                    (matching_topics as f32 / current_topics.len() as f32).min(1.0)
                }
            };
            
            item.importance.relevance = relevance_score;

            // 重新计算总分
            item.importance.total = (
                item.importance.recency * weights.recency
                + item.importance.relevance * weights.relevance
                + item.importance.user_referenced * weights.user_referenced
                + item.importance.tool_result * weights.tool_result
                + item.importance.decision_critical * weights.decision_critical
            ).min(1.0);
        }

        self.save_state()?;
        Ok(())
    }

    /// 获取所有上下文项（按重要性排序）
    pub fn get_items(&self) -> Vec<&ContextItem> {
        let mut items: Vec<_> = self.state.items.iter().collect();
        // P2-005 FIX: Use unwrap_or for partial_cmp to handle NaN safely
        items.sort_by(|a, b| b.importance.total.partial_cmp(&a.importance.total).unwrap_or(std::cmp::Ordering::Equal));
        items
    }

    /// 获取最重要的 N 个项
    pub fn get_top_items(&self, n: usize) -> Vec<&ContextItem> {
        let mut items: Vec<_> = self.state.items.iter().collect();
        // P2-005 FIX: Use unwrap_or for partial_cmp to handle NaN safely
        items.sort_by(|a, b| b.importance.total.partial_cmp(&a.importance.total).unwrap_or(std::cmp::Ordering::Equal));
        items.into_iter().take(n).collect()
    }

    /// 获取窗口状态
    pub fn get_state(&self) -> &WindowState {
        &self.state
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> WindowStats {
        let total_items = self.state.items.len();
        let user_referenced_count = self.state.items.iter()
            .filter(|i| i.user_referenced)
            .count();
        let tool_result_count = self.state.items.iter()
            .filter(|i| i.tool_name.is_some() || matches!(i.item_type, ContextItemType::ToolResult))
            .count();
        let decision_critical_count = self.state.items.iter()
            .filter(|i| i.is_decision_critical)
            .count();
        
        let avg_importance = if total_items > 0 {
            self.state.items.iter().map(|i| i.importance.total).sum::<f32>() / total_items as f32
        } else {
            0.0
        };
        
        WindowStats {
            total_items,
            user_referenced_count,
            tool_result_count,
            decision_critical_count,
            total_tokens: self.state.total_tokens,
            avg_importance,
            token_usage_ratio: self.state.total_tokens as f32 / self.config.max_tokens as f32,
        }
    }

    /// 保存状态
    fn save_state(&self) -> Result<()> {
        let state_file = self.data_dir.join("window_state.json");
        let json = serde_json::to_string_pretty(&self.state)?;
        std::fs::write(state_file, json)?;
        Ok(())
    }

    /// 加载状态
    fn load_state(&mut self) -> Result<()> {
        let state_file = self.data_dir.join("window_state.json");
        if state_file.exists() {
            let json = std::fs::read_to_string(state_file)?;
            self.state = serde_json::from_str(&json)?;
        }
        Ok(())
    }

    /// 清空窗口
    pub fn clear(&mut self) -> Result<()> {
        self.state.items.clear();
        self.state.total_tokens = 0;
        self.state.current_topics.clear();
        self.state.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.save_state()?;
        Ok(())
    }
}

/// 窗口统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowStats {
    /// 总项数
    pub total_items: usize,
    /// 用户引用项数
    pub user_referenced_count: usize,
    /// 工具结果项数
    pub tool_result_count: usize,
    /// 决策关键项数
    pub decision_critical_count: usize,
    /// 总 token 数
    pub total_tokens: usize,
    /// 平均重要性
    pub avg_importance: f32,
    /// Token 使用率
    pub token_usage_ratio: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_window_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = WindowManager::new(temp_dir.path()).unwrap();
        assert_eq!(manager.state.items.len(), 0);
    }

    #[test]
    fn test_add_item() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = WindowManager::new(temp_dir.path()).unwrap();
        
        let item = ContextItem {
            id: "test_1".to_string(),
            content: "Test content".to_string(),
            item_type: ContextItemType::UserMessage,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            last_accessed_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            importance: ImportanceScore {
                total: 0.5,
                recency: 1.0,
                relevance: 0.5,
                user_referenced: 0.0,
                tool_result: 0.0,
                decision_critical: 0.0,
            },
            user_referenced: false,
            reference_count: 0,
            topic_tags: vec!["test".to_string()],
            tool_name: None,
            is_decision_critical: false,
            size_bytes: 0,
        };
        
        manager.add_item(item).unwrap();
        assert_eq!(manager.state.items.len(), 1);
    }

    #[test]
    fn test_create_item() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = WindowManager::new(temp_dir.path()).unwrap();
        
        let item = manager.create_item(
            "Test content".to_string(),
            ContextItemType::UserMessage,
            vec!["test".to_string()],
            None,
        ).unwrap();
        
        assert!(!item.id.is_empty());
        assert_eq!(manager.state.items.len(), 1);
    }

    #[test]
    fn test_mark_as_referenced() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = WindowManager::new(temp_dir.path()).unwrap();
        
        let item = manager.create_item(
            "Test content".to_string(),
            ContextItemType::UserMessage,
            vec![],
            None,
        ).unwrap();
        
        let item_id = item.id.clone();
        manager.mark_as_referenced(&item_id).unwrap();
        
        let updated_item = manager.state.items.iter()
            .find(|i| i.id == item_id)
            .unwrap();
        
        assert!(updated_item.user_referenced);
        assert_eq!(updated_item.reference_count, 1);
    }

    #[test]
    fn test_prune_context() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = WindowManagerConfig::default();
        let max_tokens = 100; // 设置很小的限制
        config.max_tokens = max_tokens;

        let mut manager = WindowManager::with_config(temp_dir.path(), config).unwrap();

        // 添加多个项
        for _i in 0..10 {
            manager.create_item(
                format!("Test content {}", "x".repeat(100)), // 每项约 25 tokens
                ContextItemType::UserMessage,
                vec![],
                None,
            ).unwrap();
        }

        // 应该触发裁剪
        assert!(manager.state.total_tokens <= max_tokens);
    }
}
