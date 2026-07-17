//! 共享上下文管理器
//!
//! 用户交互循环和自主迭代循环共享统一的上下文存储，避免信息孤岛
//!
//! ## 上下文层
//! - `shared` - 双循环共享，会话级 TTL
//! - `interactive` - 用户交互专属，对话级 TTL
//! - `autonomous` - 自主迭代专属，迭代级 TTL
//!
//! ## 合并策略
//! - 时间戳优先
//! - 用户确认

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use sha2::{Sha256, Digest};

use crate::error::{ContextResult, ContextError};

/// 上下文层类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContextLayerType {
    /// 双循环共享
    Shared,
    /// 用户交互专属
    Interactive,
    /// 自主迭代专属
    Autonomous,
}

/// 上下文项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedContextItem {
    /// 唯一 ID
    pub id: String,
    /// 内容
    pub content: String,
    /// 所属层
    pub layer: ContextLayerType,
    /// 创建时间戳
    pub created_at: u64,
    /// 过期时间戳（0 表示永不过期）
    pub expires_at: u64,
    /// 话题标签
    pub topic_tags: Vec<String>,
    /// 来源（用户/AI/工具）
    pub source: ContextSource,
    /// 关联的迭代 ID（如果是自主迭代）
    pub iteration_id: Option<String>,
    /// 关联的对话 ID（如果是用户交互）
    pub conversation_id: Option<String>,
    /// 是否需要用户确认
    pub requires_confirmation: bool,
    /// 是否已确认
    pub confirmed: bool,
}

/// 上下文来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextSource {
    /// 用户输入
    UserInput,
    /// AI 响应
    AIResponse,
    /// 工具执行结果
    ToolResult,
    /// 系统消息
    SystemMessage,
    /// 自主迭代发现
    AutonomousDiscovery,
}

/// 上下文层
pub struct ContextLayer {
    /// 层类型
    layer_type: ContextLayerType,
    /// 存储目录
    storage_dir: PathBuf,
    /// 上下文项
    items: Vec<UnifiedContextItem>,
    /// TTL（秒）
    ttl_seconds: u64,
}

/// 统一上下文管理器
pub struct UnifiedContextManager {
    /// 数据根目录
    data_dir: PathBuf,
    /// 共享层
    shared_layer: ContextLayer,
    /// 交互层
    interactive_layer: ContextLayer,
    /// 自主层
    autonomous_layer: ContextLayer,
    /// 当前会话 ID
    session_id: String,
    /// 当前迭代 ID（如果在自主迭代中）
    current_iteration_id: Option<String>,
    /// 配置
    config: UnifiedManagerConfig,
}

/// 管理器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedManagerConfig {
    /// 共享层 TTL（秒）
    pub shared_ttl_seconds: u64,
    /// 交互层 TTL（秒）
    pub interactive_ttl_seconds: u64,
    /// 自主层 TTL（秒）
    pub autonomous_ttl_seconds: u64,
    /// 是否自动清理过期项
    pub auto_cleanup_enabled: bool,
    /// 合并策略
    pub merge_strategy: MergeStrategy,
}

/// 合并策略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum MergeStrategy {
    /// 时间戳优先（最新的优先）
    TimestampPriority,
    /// 用户确认优先
    UserConfirmationPriority,
    /// 共享层优先
    SharedPriority,
}

impl Default for UnifiedManagerConfig {
    fn default() -> Self {
        Self {
            shared_ttl_seconds: 3600, // 1 小时
            interactive_ttl_seconds: 1800, // 30 分钟
            autonomous_ttl_seconds: 600, // 10 分钟
            auto_cleanup_enabled: true,
            merge_strategy: MergeStrategy::TimestampPriority,
        }
    }
}

impl ContextLayer {
    /// 创建新的上下文层
    pub fn new(layer_type: ContextLayerType, storage_dir: PathBuf, ttl_seconds: u64) -> ContextResult<Self> {
        std::fs::create_dir_all(&storage_dir)?;

        Ok(Self {
            layer_type,
            storage_dir,
            items: Vec::new(),
            ttl_seconds,
        })
    }

    /// 添加上下文项
    pub fn add_item(&mut self, item: UnifiedContextItem) -> ContextResult<()> {
        self.items.push(item);
        self.save()?;
        Ok(())
    }

    /// 获取所有项
    pub fn get_items(&self) -> &[UnifiedContextItem] {
        &self.items
    }

    /// 清理过期项
    pub fn cleanup_expired(&mut self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let initial_count = self.items.len();
        self.items.retain(|item| {
            item.expires_at == 0 || item.expires_at > now
        });

        let removed = initial_count - self.items.len();
        if removed > 0 {
            self.save().ok();
        }
        removed
    }

    /// 保存层数据
    fn save(&self) -> ContextResult<()> {
        let file_path = self.storage_dir.join("context.json");
        let json = serde_json::to_string_pretty(&self.items)?;
        std::fs::write(file_path, json)?;
        Ok(())
    }

    /// 加载层数据
    fn load(&mut self) -> ContextResult<()> {
        let file_path = self.storage_dir.join("context.json");
        if file_path.exists() {
            let json = std::fs::read_to_string(file_path)?;
            self.items = serde_json::from_str(&json)?;
        }
        Ok(())
    }
}

impl UnifiedContextManager {
    /// 创建新的统一上下文管理器
    pub fn new<P: AsRef<Path>>(data_dir: P, session_id: &str) -> ContextResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let session_dir = data_dir.join(session_id);

        // 创建各层目录
        let shared_dir = session_dir.join("shared");
        let interactive_dir = session_dir.join("interactive");
        let autonomous_dir = session_dir.join("autonomous");

        std::fs::create_dir_all(&shared_dir)?;
        std::fs::create_dir_all(&interactive_dir)?;
        std::fs::create_dir_all(&autonomous_dir)?;

        let config = UnifiedManagerConfig::default();

        let mut manager = Self {
            data_dir,
            shared_layer: ContextLayer::new(ContextLayerType::Shared, shared_dir, config.shared_ttl_seconds)?,
            interactive_layer: ContextLayer::new(ContextLayerType::Interactive, interactive_dir, config.interactive_ttl_seconds)?,
            autonomous_layer: ContextLayer::new(ContextLayerType::Autonomous, autonomous_dir, config.autonomous_ttl_seconds)?,
            session_id: session_id.to_string(),
            current_iteration_id: None,
            config,
        };

        // 加载已有数据
        manager.shared_layer.load().ok();
        manager.interactive_layer.load().ok();
        manager.autonomous_layer.load().ok();

        Ok(manager)
    }

    /// 从配置创建
    pub fn with_config<P: AsRef<Path>>(data_dir: P, session_id: &str, config: UnifiedManagerConfig) -> ContextResult<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        let session_dir = data_dir.join(session_id);

        let shared_dir = session_dir.join("shared");
        let interactive_dir = session_dir.join("interactive");
        let autonomous_dir = session_dir.join("autonomous");

        std::fs::create_dir_all(&shared_dir)?;
        std::fs::create_dir_all(&interactive_dir)?;
        std::fs::create_dir_all(&autonomous_dir)?;

        let mut manager = Self {
            data_dir,
            shared_layer: ContextLayer::new(ContextLayerType::Shared, shared_dir, config.shared_ttl_seconds)?,
            interactive_layer: ContextLayer::new(ContextLayerType::Interactive, interactive_dir, config.interactive_ttl_seconds)?,
            autonomous_layer: ContextLayer::new(ContextLayerType::Autonomous, autonomous_dir, config.autonomous_ttl_seconds)?,
            session_id: session_id.to_string(),
            current_iteration_id: None,
            config,
        };

        manager.shared_layer.load().ok();
        manager.interactive_layer.load().ok();
        manager.autonomous_layer.load().ok();

        Ok(manager)
    }

    /// 添加上下文项
    pub fn add_item(&mut self, content: String, layer: ContextLayerType, source: ContextSource, topic_tags: Vec<String>) -> ContextResult<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let ttl = match layer {
            ContextLayerType::Shared => self.config.shared_ttl_seconds,
            ContextLayerType::Interactive => self.config.interactive_ttl_seconds,
            ContextLayerType::Autonomous => self.config.autonomous_ttl_seconds,
        };
        
        let expires_at = if ttl > 0 { now + ttl } else { 0 };
        
        // 生成 ID
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        hasher.update(now.to_be_bytes());
        let hash = hasher.finalize();
        let id = hex::encode(&hash[..8]);
        
        let item = UnifiedContextItem {
            id: id.clone(),
            content,
            layer: layer.clone(),
            created_at: now,
            expires_at,
            topic_tags,
            source,
            iteration_id: self.current_iteration_id.clone(),
            conversation_id: Some(self.session_id.clone()),
            requires_confirmation: false,
            confirmed: false,
        };
        
        // 添加到对应层
        match layer {
            ContextLayerType::Shared => self.shared_layer.add_item(item)?,
            ContextLayerType::Interactive => self.interactive_layer.add_item(item)?,
            ContextLayerType::Autonomous => self.autonomous_layer.add_item(item)?,
        }
        
        Ok(id)
    }

    /// 设置当前迭代 ID
    pub fn set_current_iteration(&mut self, iteration_id: Option<String>) {
        self.current_iteration_id = iteration_id;
    }

    /// 获取所有可见上下文（合并策略）
    pub fn get_visible_context(&self) -> Vec<&UnifiedContextItem> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut all_items: Vec<&UnifiedContextItem> = Vec::new();
        
        // 共享层始终可见
        all_items.extend(self.shared_layer.get_items().iter()
            .filter(|item| item.expires_at == 0 || item.expires_at > now));
        
        // 交互层可见
        all_items.extend(self.interactive_layer.get_items().iter()
            .filter(|item| item.expires_at == 0 || item.expires_at > now));
        
        // 自主层仅在当前迭代中可见
        if self.current_iteration_id.is_some() {
            all_items.extend(self.autonomous_layer.get_items().iter()
                .filter(|item| {
                    (item.expires_at == 0 || item.expires_at > now)
                        && item.iteration_id == self.current_iteration_id
                }));
        }
        
        // 按合并策略排序
        match &self.config.merge_strategy {
            MergeStrategy::TimestampPriority => {
                all_items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
            MergeStrategy::UserConfirmationPriority => {
                all_items.sort_by(|a, b| {
                    let a_confirmed = if a.confirmed { 1 } else { 0 };
                    let b_confirmed = if b.confirmed { 1 } else { 0 };
                    b_confirmed.cmp(&a_confirmed)
                        .then(b.created_at.cmp(&a.created_at))
                });
            }
            MergeStrategy::SharedPriority => {
                all_items.sort_by(|a, b| {
                    let a_shared = if matches!(a.layer, ContextLayerType::Shared) { 1 } else { 0 };
                    let b_shared = if matches!(b.layer, ContextLayerType::Shared) { 1 } else { 0 };
                    b_shared.cmp(&a_shared)
                        .then(b.created_at.cmp(&a.created_at))
                });
            }
        }
        
        all_items
    }

    /// 获取特定层的上下文
    pub fn get_layer_items(&self, layer: ContextLayerType) -> Vec<&UnifiedContextItem> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        match layer {
            ContextLayerType::Shared => {
                self.shared_layer.get_items().iter()
                    .filter(|item| item.expires_at == 0 || item.expires_at > now)
                    .collect()
            }
            ContextLayerType::Interactive => {
                self.interactive_layer.get_items().iter()
                    .filter(|item| item.expires_at == 0 || item.expires_at > now)
                    .collect()
            }
            ContextLayerType::Autonomous => {
                self.autonomous_layer.get_items().iter()
                    .filter(|item| item.expires_at == 0 || item.expires_at > now)
                    .collect()
            }
        }
    }

    /// 标记项为已确认
    pub fn confirm_item(&mut self, item_id: &str) -> ContextResult<()> {
        // 在各层中查找并确认
        for layer in [&mut self.shared_layer, &mut self.interactive_layer, &mut self.autonomous_layer] {
            for item in layer.items.iter_mut() {
                if item.id == item_id {
                    item.confirmed = true;
                    layer.save()?;
                    return Ok(());
                }
            }
        }

        Err(ContextError::ItemNotFound(item_id.to_string()))
    }

    /// 清理过期上下文
    pub fn cleanup(&mut self) -> usize {
        let mut removed = 0;
        removed += self.shared_layer.cleanup_expired();
        removed += self.interactive_layer.cleanup_expired();
        removed += self.autonomous_layer.cleanup_expired();
        removed
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> UnifiedStats {
        UnifiedStats {
            shared_count: self.shared_layer.get_items().len(),
            interactive_count: self.interactive_layer.get_items().len(),
            autonomous_count: self.autonomous_layer.get_items().len(),
            current_iteration: self.current_iteration_id.clone(),
        }
    }
}

/// 统一统计信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedStats {
    /// 共享层项数
    pub shared_count: usize,
    /// 交互层项数
    pub interactive_count: usize,
    /// 自主层项数
    pub autonomous_count: usize,
    /// 当前迭代 ID
    pub current_iteration: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_unified_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = UnifiedContextManager::new(temp_dir.path(), "test_session").unwrap();
        assert_eq!(manager.get_stats().shared_count, 0);
    }

    #[test]
    fn test_add_item() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = UnifiedContextManager::new(temp_dir.path(), "test_session").unwrap();
        
        let id = manager.add_item(
            "Test content".to_string(),
            ContextLayerType::Shared,
            ContextSource::UserInput,
            vec!["test".to_string()],
        ).unwrap();
        
        assert!(!id.is_empty());
        assert_eq!(manager.get_stats().shared_count, 1);
    }

    #[test]
    fn test_get_visible_context() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = UnifiedContextManager::new(temp_dir.path(), "test_session").unwrap();
        
        // 添加共享层项
        manager.add_item(
            "Shared content".to_string(),
            ContextLayerType::Shared,
            ContextSource::UserInput,
            vec![],
        ).unwrap();
        
        // 添加交互层项
        manager.add_item(
            "Interactive content".to_string(),
            ContextLayerType::Interactive,
            ContextSource::AIResponse,
            vec![],
        ).unwrap();
        
        let visible = manager.get_visible_context();
        assert_eq!(visible.len(), 2);
    }

    #[test]
    fn test_confirm_item() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = UnifiedContextManager::new(temp_dir.path(), "test_session").unwrap();
        
        let id = manager.add_item(
            "Needs confirmation".to_string(),
            ContextLayerType::Shared,
            ContextSource::AutonomousDiscovery,
            vec![],
        ).unwrap();
        
        manager.confirm_item(&id).unwrap();
        
        let items = manager.get_layer_items(ContextLayerType::Shared);
        assert!(items.iter().any(|i| i.id == id && i.confirmed));
    }
}
