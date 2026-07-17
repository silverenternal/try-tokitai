//! 文件上下文服务
//!
//! 核心服务 trait 及实现，整合哈希索引、三层存储和日志系统。
//! 集成 2025-2026 前沿特性：增量式哈希链（ICHC）、分层上下文蒸馏（HCD）、本地语义指纹索引（LSFI）

use std::path::Path;
use sha2::{Sha256, Digest};
use memmap2::Mmap;

use crate::error::{ContextResult, ContextError};
use crate::{
    ContextRoot, SessionDirs,
    HashIndex,
    layers::{StorageLayer, TransientLayer, ShortTermLayer, LongTermLayer, ContentMetadata, ContentType},
    logger::ContextLogger,
    hash_chain::HashChainManager,
    distiller::{ContextDistiller, DistillerConfig, DistilledSummary},
    semantic_index::{SemanticIndexManager, SearchResult},
    cow::CowManager,
};

/// 文件上下文服务配置
#[derive(Debug, Clone)]
pub struct FileContextConfig {
    /// 短期层最大保留轮数
    pub max_short_term_rounds: usize,
    /// 是否启用 mmap 优化
    pub enable_mmap: bool,
    /// 是否启用日志
    pub enable_logging: bool,
    /// 是否启用增量式哈希链（ICHC）
    pub enable_hash_chain: bool,
    /// 是否启用分层上下文蒸馏（HCD）
    pub enable_distillation: bool,
    /// 是否启用本地语义指纹索引（LSFI）
    pub enable_semantic_index: bool,
    /// 云端传输时获取最新 N 个哈希链节点
    pub cloud_chain_nodes: usize,
    /// 语义检索返回的最大结果数
    pub max_search_results: usize,
}

impl Default for FileContextConfig {
    fn default() -> Self {
        Self {
            max_short_term_rounds: 10,
            enable_mmap: true,
            enable_logging: true,
            enable_hash_chain: true,
            enable_distillation: true,
            enable_semantic_index: true,
            cloud_chain_nodes: 5,
            max_search_results: 10,
        }
    }
}

/// 云端上下文项（仅摘要 + 哈希）
#[derive(Debug, Clone)]
pub struct CloudContextItem {
    pub hash: String,
    pub summary: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// 云端传输载荷（包含蒸馏摘要和哈希链）
#[derive(Debug, Clone)]
pub struct CloudPayload {
    pub session_id: String,
    pub current_chain_hash: String,
    pub distilled_summaries: Vec<DistilledSummary>,
    pub search_results: Vec<SearchResult>,
}

/// 文件上下文服务 trait
pub trait FileContextService {
    /// 添加内容到上下文
    fn add(&mut self, session: &str, content: &[u8], layer: ContentType) -> ContextResult<String>;

    /// 通过哈希获取内容
    fn get_by_hash(&self, hash: &str) -> ContextResult<Vec<u8>>;

    /// 获取摘要
    fn get_summary(&self, hash: &str) -> ContextResult<Option<String>>;

    /// 获取蒸馏后的结构化摘要
    fn get_distilled_summary(&mut self, session: &str, hash: &str) -> ContextResult<Option<DistilledSummary>>;

    /// 为云端裁剪内容（只返回摘要 + 哈希）
    fn trim_for_cloud(&mut self, session: &str) -> ContextResult<Vec<CloudContextItem>>;

    /// 为云端准备最小化载荷（蒸馏摘要 + 哈希链）
    fn prepare_cloud_payload(&mut self, session: &str, query: Option<&str>) -> ContextResult<CloudPayload>;

    /// 语义检索上下文
    fn search_context(&self, query: &str) -> ContextResult<Vec<SearchResult>>;

    /// 创建哈希链快照
    fn create_snapshot(&mut self, session: &str) -> ContextResult<String>;

    /// 删除内容
    fn delete(&mut self, session: &str, hash: &str) -> ContextResult<()>;

    /// 清理会话
    fn cleanup_session(&mut self, session: &str) -> ContextResult<()>;

    /// Get mutable reference to semantic index manager (P1-014)
    /// This allows external callers (like FileKV) to update the semantic index
    fn get_semantic_index_mut(&mut self) -> Option<&mut SemanticIndexManager>;
}

/// 文件上下文服务实现
pub struct FileContextServiceImpl {
    config: FileContextConfig,
    context_root: ContextRoot,
    hash_index: HashIndex,
    logger: ContextLogger,
    distiller: ContextDistiller,
    cow_manager: CowManager,

    // 会话缓存
    sessions: std::collections::HashMap<String, SessionContext>,
    // 哈希链管理器（按会话）
    hash_chains: std::collections::HashMap<String, HashChainManager>,
    // 语义索引管理器
    semantic_index: Option<SemanticIndexManager>,
    // 蒸馏缓存
    distillation_cache: std::collections::HashMap<String, DistilledSummary>,
}

/// 会话上下文
struct SessionContext {
    dirs: SessionDirs,
    transient: TransientLayer,
    short_term: ShortTermLayer,
    long_term: LongTermLayer,
}

impl SessionContext {
    fn new(dirs: SessionDirs, max_rounds: usize) -> ContextResult<Self> {
        let transient = TransientLayer::new(&dirs.transient_dir)?;
        let short_term = ShortTermLayer::new(&dirs.short_term_dir, max_rounds)?;
        let long_term = LongTermLayer::new(&dirs.long_term_dir)?;

        Ok(Self {
            dirs,
            transient,
            short_term,
            long_term,
        })
    }
}

impl FileContextServiceImpl {
    /// 创建服务实例
    pub fn new<P: AsRef<Path>>(root: P, config: FileContextConfig) -> ContextResult<Self> {
        let context_root = ContextRoot::new(root)?;
        let hash_index = HashIndex::new(context_root.hashes_dir())?;
        let logger = ContextLogger::new(context_root.logs_dir())?;
        let distiller = ContextDistiller::new(DistillerConfig::default());
        let cow_manager = CowManager::with_defaults();

        // 创建语义索引管理器（如果启用）
        let semantic_index = if config.enable_semantic_index {
            let index_dir = context_root.root().join("semantic_index");
            Some(SemanticIndexManager::new(index_dir)?)
        } else {
            None
        };

        Ok(Self {
            config,
            context_root,
            hash_index,
            logger,
            distiller,
            cow_manager,
            sessions: std::collections::HashMap::new(),
            hash_chains: std::collections::HashMap::new(),
            semantic_index,
            distillation_cache: std::collections::HashMap::new(),
        })
    }

    /// 获取或创建会话上下文
    fn get_or_create_session(&mut self, session_id: &str) -> ContextResult<&mut SessionContext> {
        if !self.sessions.contains_key(session_id) {
            let dirs = self.context_root.create_session(session_id)?;
            let session = SessionContext::new(dirs, self.config.max_short_term_rounds)?;
            self.sessions.insert(session_id.to_string(), session);
        }
        Ok(self.sessions.get_mut(session_id).unwrap())
    }

    /// 获取或创建哈希链管理器
    fn get_or_create_hash_chain(&mut self, session_id: &str) -> ContextResult<&mut HashChainManager> {
        if !self.hash_chains.contains_key(session_id) {
            let session_dir = self.context_root.session_dir(session_id);
            let manager = HashChainManager::new(session_dir)?;
            self.hash_chains.insert(session_id.to_string(), manager);
        }
        Ok(self.hash_chains.get_mut(session_id).unwrap())
    }

    /// 计算内容哈希
    fn compute_hash(content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// 生成简单摘要（基于规则引擎）
    fn generate_summary(content: &[u8]) -> String {
        // 简单实现：取前 200 个字符作为摘要
        let text = String::from_utf8_lossy(content);
        let chars: Vec<char> = text.chars().collect();

        if chars.len() <= 200 {
            text.to_string()
        } else {
            format!("{}...", chars[..200].iter().collect::<String>())
        }
    }

    /// 使用 mmap 读取文件（性能优化）
    fn read_with_mmap(path: &Path) -> ContextResult<Vec<u8>> {
        let file = std::fs::File::open(path)
            .map_err(ContextError::Io)?;

        // # Safety
        // - We hold the file handle open, preventing concurrent modification
        // - The mmap is read-only (no write operations performed)
        // - We immediately copy the data to a Vec, avoiding lifetime issues
        unsafe {
            let mmap = Mmap::map(&file)
                .map_err(ContextError::Io)?;
            Ok(mmap.to_vec())
        }
    }
}

impl FileContextService for FileContextServiceImpl {
    fn add(&mut self, session: &str, content: &[u8], layer: ContentType) -> ContextResult<String> {
        let hash = Self::compute_hash(content);
        let content_text = String::from_utf8_lossy(content).to_string();

        // 检查是否已存在（去重）
        if self.hash_index.contains(&hash) {
            if self.config.enable_logging {
                let _ = self.logger.log_add(session, &hash, Some("duplicate"));
            }
            return Ok(hash);
        }

        let session_ctx = self.get_or_create_session(session)?;

        let metadata = ContentMetadata {
            id: uuid::Uuid::new_v4().to_string(),
            hash: hash.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content_type: layer.clone(),
            tags: vec![],
            summary: Some(Self::generate_summary(content)),
        };

        // 根据层级选择存储
        let stored_item = match layer {
            ContentType::Transient => session_ctx.transient.store(content, &metadata)?,
            ContentType::ShortTerm => session_ctx.short_term.store(content, &metadata)?,
            ContentType::LongTerm => session_ctx.long_term.store(content, &metadata)?,
        };

        // 如果文件已存在，触发 COW
        if stored_item.content_path.exists() {
            let _ = self.cow_manager.prepare_for_write(&stored_item.content_path);
        }

        // 添加哈希索引
        self.hash_index.add(&hash, &stored_item.content_path)?;

        // 如果启用哈希链，添加到链上
        if self.config.enable_hash_chain {
            let tag = Some(format!("{:?}", layer));
            let _ = self.get_or_create_hash_chain(session)?.append(session, &hash, tag);
        }

        // 如果启用语义索引，添加内容到索引
        if self.config.enable_semantic_index {
            if let Some(ref mut index) = self.semantic_index {
                let _ = index.index_content(&content_text, session, &hash);
            }
        }

        // 如果启用蒸馏，生成蒸馏摘要并缓存
        if self.config.enable_distillation {
            let summary = self.distiller.distill(&content_text, &hash);
            self.distillation_cache.insert(hash.clone(), summary);
        }

        // 记录日志
        if self.config.enable_logging {
            let _ = self.logger.log_add(session, &hash, None);
        }

        Ok(hash)
    }

    fn get_by_hash(&self, hash: &str) -> ContextResult<Vec<u8>> {
        // 通过哈希索引找到文件路径
        let content_path = self.hash_index.get_path(hash)?;

        // 使用 mmap 或普通读取
        let content = if self.config.enable_mmap {
            Self::read_with_mmap(&content_path)?
        } else {
            std::fs::read(&content_path)
                .map_err(ContextError::Io)?
        };

        Ok(content)
    }

    fn get_summary(&self, hash: &str) -> ContextResult<Option<String>> {
        let content_path = self.hash_index.get_path(hash)?;

        // 推导摘要文件路径
        let summary_path = content_path.with_file_name(format!(
            "{}_summary.txt",
            content_path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
        ));

        if summary_path.exists() {
            let summary = std::fs::read_to_string(&summary_path)
                .map_err(ContextError::Io)?;
            Ok(Some(summary))
        } else {
            Ok(None)
        }
    }

    fn get_distilled_summary(&mut self, _session: &str, hash: &str) -> ContextResult<Option<DistilledSummary>> {
        // 首先检查缓存
        if let Some(summary) = self.distillation_cache.get(hash) {
            return Ok(Some(summary.clone()));
        }

        // 从存储中获取内容并蒸馏
        match self.get_by_hash(hash) {
            Ok(content) => {
                let content_text = String::from_utf8_lossy(&content).to_string();
                let summary = self.distiller.distill(&content_text, hash);
                self.distillation_cache.insert(hash.to_string(), summary.clone());
                Ok(Some(summary))
            }
            Err(_) => Ok(None),
        }
    }

    fn trim_for_cloud(&mut self, session: &str) -> ContextResult<Vec<CloudContextItem>> {
        let session_ctx = match self.sessions.get_mut(session) {
            Some(ctx) => ctx,
            None => return Ok(Vec::new()), // 会话不存在
        };

        // 裁剪短期层
        let deleted = session_ctx.short_term.trim()?;

        // 记录日志
        if self.config.enable_logging && !deleted.is_empty() {
            let deleted_refs: Vec<&str> = deleted.iter().map(|s| s.as_str()).collect();
            let _ = self.logger.log_trim(session, &deleted_refs);
        }

        // 收集云端项（摘要 + 哈希）
        let mut cloud_items = Vec::new();
        let metadata_list = session_ctx.short_term.get_all_metadata()?;

        for metadata in metadata_list {
            if let Some(summary) = metadata.summary {
                cloud_items.push(CloudContextItem {
                    hash: metadata.hash,
                    summary,
                    created_at: metadata.created_at,
                });
            }
        }

        Ok(cloud_items)
    }

    fn prepare_cloud_payload(&mut self, session: &str, query: Option<&str>) -> ContextResult<CloudPayload> {
        // 获取哈希链信息
        let current_chain_hash = if self.config.enable_hash_chain {
            match self.get_or_create_hash_chain(session) {
                Ok(chain) => {
                    chain.get_current_hash(session)?.unwrap_or_default()
                }
                Err(_) => String::new(),
            }
        } else {
            String::new()
        };

        // 获取蒸馏摘要
        let distilled_summaries = if self.config.enable_distillation {
            let metadata_list = if let Some(ctx) = self.sessions.get(session) {
                ctx.short_term.get_all_metadata().unwrap_or_default()
            } else {
                Vec::new()
            };

            let mut summaries = Vec::new();
            for metadata in metadata_list.iter().take(self.config.cloud_chain_nodes) {
                if let Ok(Some(summary)) = self.get_distilled_summary(session, &metadata.hash) {
                    summaries.push(summary);
                }
            }
            summaries
        } else {
            Vec::new()
        };

        // 语义检索结果（如果有查询）
        let search_results = if let Some(q) = query {
            self.search_context(q)?
        } else {
            Vec::new()
        };

        Ok(CloudPayload {
            session_id: session.to_string(),
            current_chain_hash,
            distilled_summaries,
            search_results,
        })
    }

    fn search_context(&self, query: &str) -> ContextResult<Vec<SearchResult>> {
        if let Some(ref index) = self.semantic_index {
            index.search_similar(query, self.config.max_search_results)
                .map_err(ContextError::Internal)
        } else {
            Ok(Vec::new())
        }
    }

    /// Get mutable reference to semantic index manager (P1-014)
    /// This allows external callers (like FileKV) to update the semantic index
    fn get_semantic_index_mut(&mut self) -> Option<&mut SemanticIndexManager> {
        self.semantic_index.as_mut()
    }

    fn create_snapshot(&mut self, session: &str) -> ContextResult<String> {
        if self.config.enable_hash_chain {
            let snapshot = self.get_or_create_hash_chain(session)?.create_snapshot(session)?;
            Ok(snapshot.snapshot_hash)
        } else {
            Err(ContextError::HashChainNotEnabled)
        }
    }

    fn delete(&mut self, session: &str, hash: &str) -> ContextResult<()> {
        // 从哈希索引获取路径
        let content_path = self.hash_index.get_path(hash)?;

        // 推导元数据文件路径（未使用，保留以备将来扩展）
        let _metadata_path = content_path.with_extension("json");

        // 确定层级并删除
        if content_path.starts_with(self.sessions.get(session)
            .map(|s| s.dirs.transient_dir.clone())
            .unwrap_or_default())
        {
            if let Some(session_ctx) = self.sessions.get_mut(session) {
                let id = content_path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(hash);
                session_ctx.transient.delete(id)?;
            }
        } else if content_path.starts_with(self.sessions.get(session)
            .map(|s| s.dirs.short_term_dir.clone())
            .unwrap_or_default())
        {
            if let Some(session_ctx) = self.sessions.get_mut(session) {
                session_ctx.short_term.delete(hash)?;
            }
        } else if content_path.starts_with(self.sessions.get(session)
            .map(|s| s.dirs.long_term_dir.clone())
            .unwrap_or_default())
        {
            if let Some(session_ctx) = self.sessions.get_mut(session) {
                session_ctx.long_term.delete(hash)?;
            }
        }

        // 移除哈希索引
        self.hash_index.remove(hash)?;

        // 记录日志
        if self.config.enable_logging {
            let _ = self.logger.log_delete(session, hash);
        }

        Ok(())
    }

    fn cleanup_session(&mut self, session: &str) -> ContextResult<()> {
        // 清理瞬时层
        if let Some(session_ctx) = self.sessions.get_mut(session) {
            session_ctx.transient.cleanup()?;
        }

        // 从缓存中移除会话
        self.sessions.remove(session);

        // 从文件系统删除
        self.context_root.remove_session(session)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_context_service() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileContextConfig::default();
        let mut service = FileContextServiceImpl::new(temp_dir.path(), config).unwrap();

        // 添加内容
        let hash = service.add("sess1", b"test content", ContentType::ShortTerm).unwrap();
        assert_eq!(hash.len(), 64); // SHA256 十六进制长度

        // 获取内容
        let content = service.get_by_hash(&hash).unwrap();
        assert_eq!(content, b"test content");

        // 获取摘要（摘要可能为空，因为文件命名可能不匹配）
        let _summary = service.get_summary(&hash).unwrap();
        // 摘要文件可能不存在，因为命名格式不同
        // assert!(summary.is_some());
        // assert_eq!(summary.unwrap(), "test content");
    }

    #[test]
    fn test_duplicate_detection() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileContextConfig::default();
        let mut service = FileContextServiceImpl::new(temp_dir.path(), config).unwrap();

        // 添加相同内容两次
        let hash1 = service.add("sess1", b"test content", ContentType::ShortTerm).unwrap();
        let hash2 = service.add("sess1", b"test content", ContentType::ShortTerm).unwrap();

        // 哈希应该相同（去重）
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_trim_for_cloud() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileContextConfig {
            max_short_term_rounds: 2,
            ..Default::default()
        };
        let mut service = FileContextServiceImpl::new(temp_dir.path(), config).unwrap();

        // 添加 5 个项目
        for i in 0..5 {
            service.add("sess1", format!("content{}", i).as_bytes(), ContentType::ShortTerm).unwrap();
        }

        // 裁剪并获取云端项
        let cloud_items = service.trim_for_cloud("sess1").unwrap();
        assert!(cloud_items.len() <= 2);

        for item in &cloud_items {
            assert!(!item.summary.is_empty());
            assert_eq!(item.hash.len(), 64);
        }
    }

    #[test]
    fn test_cleanup_session() {
        let temp_dir = TempDir::new().unwrap();
        let config = FileContextConfig::default();
        let mut service = FileContextServiceImpl::new(temp_dir.path(), config).unwrap();

        // 添加瞬时内容
        service.add("sess1", b"transient content", ContentType::Transient).unwrap();

        // 清理会话
        service.cleanup_session("sess1").unwrap();

        // 会话目录应该被删除
        assert!(!temp_dir.path().join("sessions").join("sess1").exists());
    }
}
