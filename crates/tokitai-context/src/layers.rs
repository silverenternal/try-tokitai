//! 存储层级模块
//!
//! 实现三层存储架构：
//! - 瞬时层：单轮临时文件，会话结束删除
//! - 短期层：最近 N 轮，支持自动裁剪
//! - 长期层：项目习惯/规则，按关键词分类

use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{ContextResult, ContextError};

/// 存储层级 trait
pub trait StorageLayer {
    /// 存储内容
    fn store(&self, content: &[u8], metadata: &ContentMetadata) -> ContextResult<StoredItem>;

    /// 检索内容
    fn retrieve(&self, id: &str) -> ContextResult<Vec<u8>>;

    /// 删除内容
    fn delete(&self, id: &str) -> ContextResult<()>;

    /// 列出所有项目
    fn list(&self) -> ContextResult<Vec<String>>;
}

/// 内容元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMetadata {
    pub id: String,
    pub hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub content_type: ContentType,
    pub tags: Vec<String>,
    pub summary: Option<String>,
}

/// 内容类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    Transient,
    ShortTerm,
    LongTerm,
}

/// 存储的项目
#[derive(Debug, Clone)]
pub struct StoredItem {
    pub id: String,
    pub hash: String,
    pub summary_path: PathBuf,
    pub content_path: PathBuf,
    pub metadata_path: PathBuf,
}

// ============= 瞬时层 =============

/// 瞬时层管理器
pub struct TransientLayer {
    dir: PathBuf,
}

impl TransientLayer {
    pub fn new<P: AsRef<Path>>(dir: P) -> ContextResult<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(ContextError::Io)?;
        Ok(Self { dir })
    }

    /// 清理所有瞬时文件
    pub fn cleanup(&self) -> ContextResult<()> {
        if self.dir.exists() {
            std::fs::remove_dir_all(&self.dir)
                .map_err(ContextError::Io)?;
            std::fs::create_dir_all(&self.dir)?;
        }
        Ok(())
    }
}

impl StorageLayer for TransientLayer {
    fn store(&self, content: &[u8], metadata: &ContentMetadata) -> ContextResult<StoredItem> {
        let id = &metadata.id;

        // 瞬时层只存储临时文件，不保留元数据
        let content_path = self.dir.join(format!("{}_content.bin", id));

        std::fs::write(&content_path, content)
            .map_err(ContextError::Io)?;

        Ok(StoredItem {
            id: id.clone(),
            hash: metadata.hash.clone(),
            summary_path: PathBuf::new(), // 瞬时层不需要摘要
            content_path,
            metadata_path: PathBuf::new(), // 瞬时层不需要元数据文件
        })
    }

    fn retrieve(&self, id: &str) -> ContextResult<Vec<u8>> {
        let content_path = self.dir.join(format!("{}_content.bin", id));
        std::fs::read(&content_path)
            .map_err(ContextError::Io)
    }

    fn delete(&self, id: &str) -> ContextResult<()> {
        let content_path = self.dir.join(format!("{}_content.bin", id));
        if content_path.exists() {
            std::fs::remove_file(&content_path)?;
        }
        Ok(())
    }

    fn list(&self) -> ContextResult<Vec<String>> {
        let mut ids = Vec::new();

        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with("_content.bin") {
                    if let Some(id) = name.strip_suffix("_content.bin") {
                        ids.push(id.to_string());
                    }
                }
            }
        }

        Ok(ids)
    }
}

// ============= 短期层 =============

/// 短期层管理器
pub struct ShortTermLayer {
    dir: PathBuf,
    max_rounds: usize,
}

impl ShortTermLayer {
    pub fn new<P: AsRef<Path>>(dir: P, max_rounds: usize) -> ContextResult<Self> {
        let dir = dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&dir)
            .map_err(ContextError::Io)?;
        Ok(Self { dir, max_rounds })
    }

    /// 获取最大轮数
    pub fn max_rounds(&self) -> usize {
        self.max_rounds
    }

    /// 设置最大轮数
    pub fn set_max_rounds(&mut self, max_rounds: usize) {
        self.max_rounds = max_rounds;
    }

    /// 自动裁剪，保留最近 N 轮
    pub fn trim(&self) -> ContextResult<Vec<String>> {
        let mut items: Vec<(String, ContentMetadata)> = Vec::new();

        // 读取所有元数据
        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<ContentMetadata>(&content) {
                        items.push((path.file_stem().unwrap().to_string_lossy().to_string(), metadata));
                    }
                }
            }
        }

        // 按时间排序
        items.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));

        // 删除超出限制的项目
        let mut deleted = Vec::new();
        for (id, _) in items.iter().skip(self.max_rounds) {
            self.delete(id)?;
            deleted.push(id.clone());
        }

        Ok(deleted)
    }

    /// 获取所有项目的元数据
    pub fn get_all_metadata(&self) -> ContextResult<Vec<ContentMetadata>> {
        let mut metadata_list = Vec::new();

        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<ContentMetadata>(&content) {
                        metadata_list.push(metadata);
                    }
                }
            }
        }

        // 按时间排序
        metadata_list.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        Ok(metadata_list)
    }
}

impl StorageLayer for ShortTermLayer {
    fn store(&self, content: &[u8], metadata: &ContentMetadata) -> ContextResult<StoredItem> {
        let hash = &metadata.hash;

        let summary_path = self.dir.join(format!("{}_summary.txt", hash));
        let content_path = self.dir.join(format!("{}_content.bin", hash));
        let metadata_path = self.dir.join(format!("{}.json", hash));

        // 写入摘要
        if let Some(summary) = &metadata.summary {
            std::fs::write(&summary_path, summary)
                .map_err(ContextError::Io)?;
        }

        // 写入内容
        std::fs::write(&content_path, content)
            .map_err(ContextError::Io)?;

        // 写入元数据
        let metadata_json = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&metadata_path, metadata_json)
            .map_err(ContextError::Io)?;

        Ok(StoredItem {
            id: metadata.id.clone(),
            hash: metadata.hash.clone(),
            summary_path,
            content_path,
            metadata_path,
        })
    }

    fn retrieve(&self, id: &str) -> ContextResult<Vec<u8>> {
        // 尝试通过哈希或 ID 查找
        let content_path = self.dir.join(format!("{}_content.bin", id));

        if content_path.exists() {
            std::fs::read(&content_path)
                .map_err(ContextError::Io)
        } else {
            // 尝试查找匹配的哈希
            for entry in std::fs::read_dir(&self.dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(metadata) = serde_json::from_str::<ContentMetadata>(&content) {
                            if metadata.hash == id || metadata.id == id {
                                let content_path = self.dir.join(format!("{}_content.bin", metadata.hash));
                                return std::fs::read(&content_path)
                                    .map_err(ContextError::Io);
                            }
                        }
                    }
                }
            }

            Err(ContextError::ContentNotFound(id.to_string()))
        }
    }

    fn delete(&self, id: &str) -> ContextResult<()> {
        let summary_path = self.dir.join(format!("{}_summary.txt", id));
        let content_path = self.dir.join(format!("{}_content.bin", id));
        let metadata_path = self.dir.join(format!("{}.json", id));

        if summary_path.exists() {
            std::fs::remove_file(&summary_path)?;
        }
        if content_path.exists() {
            std::fs::remove_file(&content_path)?;
        }
        if metadata_path.exists() {
            std::fs::remove_file(&metadata_path)?;
        }

        Ok(())
    }

    fn list(&self) -> ContextResult<Vec<String>> {
        let mut hashes = Vec::new();

        for entry in std::fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    hashes.push(stem.to_string());
                }
            }
        }

        Ok(hashes)
    }
}

// ============= 长期层 =============

/// 长期层配置
#[derive(Debug, Clone)]
pub struct LongTermConfig {
    /// 知识库根目录
    pub knowledge_root: Option<PathBuf>,
    /// 是否从目录结构自动同步分类
    pub auto_sync_categories: bool,
    /// 手动配置的额外分类
    pub custom_categories: Vec<String>,
}

impl Default for LongTermConfig {
    fn default() -> Self {
        Self {
            knowledge_root: None,
            auto_sync_categories: false,
            custom_categories: vec!["git_rules".to_string(), "tool_configs".to_string(), "task_patterns".to_string()],
        }
    }
}

/// 长期层管理器
pub struct LongTermLayer {
    dir: PathBuf,
    categories: Vec<String>,
    config: LongTermConfig,
}

impl LongTermLayer {
    pub fn new<P: AsRef<Path>>(dir: P) -> ContextResult<Self> {
        Self::with_config(dir, LongTermConfig::default())
    }

    /// 根据配置创建长期层
    pub fn with_config<P: AsRef<Path>>(dir: P, config: LongTermConfig) -> ContextResult<Self> {
        let dir = dir.as_ref().to_path_buf();
        let mut categories = Vec::new();

        // 自动从目录结构同步分类
        if config.auto_sync_categories {
            if let Some(ref root) = config.knowledge_root {
                if root.exists() {
                    for entry in std::fs::read_dir(root)
                        .map_err(ContextError::Io)?
                    {
                        let entry = entry?;
                        if entry.file_type()?.is_dir() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if !categories.contains(&name) {
                                categories.push(name);
                            }
                        }
                    }
                }
            }
        }

        // 添加手动配置的分类
        for cat in &config.custom_categories {
            if !categories.contains(cat) {
                categories.push(cat.clone());
            }
        }

        // 创建分类目录
        for cat in &categories {
            std::fs::create_dir_all(dir.join(cat))
                .map_err(ContextError::Io)?;
        }

        Ok(Self { dir, categories, config })
    }

    /// 添加分类
    pub fn add_category(&self, category: &str) -> ContextResult<()> {
        let category_dir = self.dir.join(category);
        std::fs::create_dir_all(&category_dir)
            .map_err(ContextError::Io)?;
        Ok(())
    }

    /// 获取分类目录
    pub fn category_dir(&self, category: &str) -> PathBuf {
        self.dir.join(category)
    }

    /// 获取所有分类
    pub fn categories(&self) -> &[String] {
        &self.categories
    }

    /// 根据内容自动选择分类
    pub fn select_category(&self, content: &str, tags: &[String]) -> String {
        // 1. 优先匹配标签
        for tag in tags {
            if self.categories.iter().any(|c| c.to_lowercase() == tag.to_lowercase()) {
                return tag.clone();
            }
        }

        // 2. 基于关键词匹配
        let keywords = extract_keywords(content);
        for cat in &self.categories {
            if keywords.iter().any(|k| k.to_lowercase() == cat.to_lowercase()) {
                return cat.clone();
            }
        }

        // 3. 默认分类
        "task_patterns".to_string()
    }

    /// 按关键词搜索
    pub fn search_by_keyword(&self, keyword: &str) -> ContextResult<Vec<PathBuf>> {
        let mut results = Vec::new();

        for category in &self.categories {
            let category_dir = self.dir.join(category);

            if category_dir.exists() {
                for entry in std::fs::read_dir(&category_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_file() {
                        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                            if name.to_lowercase().contains(&keyword.to_lowercase()) {
                                results.push(path);
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// 合并重复的规则文件
    pub fn compact(&self) -> ContextResult<usize> {
        // 简单实现：统计文件数量
        let mut count = 0;

        for category in &self.categories {
            let category_dir = self.dir.join(category);

            if category_dir.exists() {
                for entry in std::fs::read_dir(&category_dir)? {
                    let _ = entry?;
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

/// 从文本提取关键词（简化版）
fn extract_keywords(text: &str) -> Vec<String> {
    let stop_words = ["的", "了", "是", "在", "和", "与", "或", "怎么", "如何", "哪些", "什么"];
    
    text.split(|c: char| c.is_whitespace() || c == '?' || c == '？' || c == ',' || c == '，')
        .filter(|s| !s.is_empty() && s.len() > 1)
        .filter(|s| !stop_words.contains(s))
        .map(|s| s.to_string())
        .collect()
}

impl StorageLayer for LongTermLayer {
    fn store(&self, content: &[u8], metadata: &ContentMetadata) -> ContextResult<StoredItem> {
        // 使用第一个标签作为分类，如果没有则使用默认分类
        let category = metadata.tags.first()
            .map(|s| s.as_str())
            .unwrap_or("task_patterns");

        let category_dir = self.dir.join(category);
        std::fs::create_dir_all(&category_dir)?;

        let content_path = category_dir.join(format!("{}.bin", metadata.hash));
        let metadata_path = category_dir.join(format!("{}.json", metadata.hash));
        let summary_path = category_dir.join(format!("{}_summary.txt", metadata.hash));

        // 写入内容
        std::fs::write(&content_path, content)
            .map_err(ContextError::Io)?;

        // 写入元数据
        let metadata_json = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&metadata_path, metadata_json)
            .map_err(ContextError::Io)?;

        // 写入摘要
        if let Some(summary) = &metadata.summary {
            std::fs::write(&summary_path, summary)
                .map_err(ContextError::Io)?;
        }

        Ok(StoredItem {
            id: metadata.id.clone(),
            hash: metadata.hash.clone(),
            summary_path,
            content_path,
            metadata_path,
        })
    }

    fn retrieve(&self, id: &str) -> ContextResult<Vec<u8>> {
        // 在所有分类目录中查找
        for category in &self.categories {
            let content_path = self.dir.join(category).join(format!("{}.bin", id));

            if content_path.exists() {
                return std::fs::read(&content_path)
                    .map_err(ContextError::Io);
            }
        }

        Err(ContextError::ContentNotFound(id.to_string()))
    }

    fn delete(&self, id: &str) -> ContextResult<()> {
        for category in &self.categories {
            let category_dir = self.dir.join(category);

            let summary_path = category_dir.join(format!("{}_summary.txt", id));
            let content_path = category_dir.join(format!("{}.bin", id));
            let metadata_path = category_dir.join(format!("{}.json", id));

            if summary_path.exists() {
                std::fs::remove_file(&summary_path)?;
            }
            if content_path.exists() {
                std::fs::remove_file(&content_path)?;
            }
            if metadata_path.exists() {
                std::fs::remove_file(&metadata_path)?;
            }
        }

        Ok(())
    }

    fn list(&self) -> ContextResult<Vec<String>> {
        let mut hashes = Vec::new();

        for category in &self.categories {
            let category_dir = self.dir.join(category);

            if category_dir.exists() {
                for entry in std::fs::read_dir(&category_dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            hashes.push(stem.to_string());
                        }
                    }
                }
            }
        }

        Ok(hashes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_metadata(id: &str, hash: &str) -> ContentMetadata {
        ContentMetadata {
            id: id.to_string(),
            hash: hash.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content_type: ContentType::ShortTerm,
            tags: vec!["test".to_string()],
            summary: Some("Test summary".to_string()),
        }
    }

    #[test]
    fn test_transient_layer() {
        let temp_dir = TempDir::new().unwrap();
        let layer = TransientLayer::new(temp_dir.path()).unwrap();

        let metadata = create_test_metadata("test1", "abc123");
        let item = layer.store(b"test content", &metadata).unwrap();

        assert!(item.content_path.exists());
        
        let content = layer.retrieve("test1").unwrap();
        assert_eq!(content, b"test content");

        layer.delete("test1").unwrap();
        assert!(!item.content_path.exists());
    }

    #[test]
    fn test_short_term_layer() {
        let temp_dir = TempDir::new().unwrap();
        let layer = ShortTermLayer::new(temp_dir.path(), 3).unwrap();

        // 存储 5 个项目
        for i in 0..5 {
            let metadata = create_test_metadata(&format!("test{}", i), &format!("hash{}", i));
            layer.store(format!("content{}", i).as_bytes(), &metadata).unwrap();
        }

        // 裁剪到 3 个
        let deleted = layer.trim().unwrap();
        assert_eq!(deleted.len(), 2);

        let list = layer.list().unwrap();
        assert!(list.len() <= 3);
    }

    #[test]
    fn test_long_term_layer() {
        let temp_dir = TempDir::new().unwrap();
        let layer = LongTermLayer::new(temp_dir.path()).unwrap();

        let metadata = ContentMetadata {
            id: "test1".to_string(),
            hash: "abc123".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content_type: ContentType::LongTerm,
            tags: vec!["git_rules".to_string()],
            summary: Some("Git rule summary".to_string()),
        };

        let item = layer.store(b"git rule content", &metadata).unwrap();

        assert!(item.content_path.exists());

        let content = layer.retrieve("abc123").unwrap();
        assert_eq!(content, b"git rule content");

        // 测试搜索（使用哈希值搜索）
        let results = layer.search_by_keyword("abc123").unwrap();
        assert!(!results.is_empty());
    }
}
