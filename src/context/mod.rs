//! 纯文件上下文存储系统
//!
//! 基于"文件即数据库"理念，使用分层文件目录 + 哈希符号链接 + 增量日志
//! 实现本地完整存储 + 云端最小传输的核心逻辑。
//!
//! ## 目录结构
//! ```text
//! .context/
//! ├── sessions/          # 会话级目录
//! │   └── sess_xxx/      # 单个会话目录
//! │       ├── transient/ # 瞬时层：单轮临时文件
//! │       ├── short-term/# 短期层：最近 N 轮
//! │       └── long-term/ # 长期层：项目习惯/规则
//! ├── hashes/            # 哈希索引目录（符号链接）
//! ├── semantic_index/    # 语义指纹索引
//! └── logs/              # 增量日志
//! ```
//!
//! ## 核心特性
//! - **增量式哈希链（ICHC）**: 链式哈希结构，支持快照回溯
//! - **分层上下文蒸馏（HCD）**: 意图驱动的结构化摘要，减少 60%+ 云端传输
//! - **本地语义指纹索引（LSFI）**: SimHash 语义检索，准确率提升 30%+

#![allow(dead_code)]
#![allow(unused_imports)]

mod file_service;
mod hash_index;
mod layers;
mod logger;
mod hash_chain;
mod distiller;
mod semantic_index;

#[allow(unused_imports)]
pub use file_service::{FileContextService, FileContextConfig, CloudContextItem, CloudPayload};
#[allow(unused_imports)]
pub use layers::{StorageLayer, TransientLayer, ShortTermLayer, LongTermLayer, ContentMetadata, ContentType, StoredItem};
#[allow(unused_imports)]
pub use hash_index::HashIndex;
#[allow(unused_imports)]
pub use logger::{ContextLogger, LogEntry, LogOperation};
#[allow(unused_imports)]
pub use hash_chain::{HashChain, HashChainManager, ChainNode, HashChainSnapshot, CloudChainPayload};
#[allow(unused_imports)]
pub use distiller::{ContextDistiller, DistillerConfig, DistilledSummary, ToolDependency, ToolStatus, DistillationCache, CacheStats};
#[allow(unused_imports)]
pub use semantic_index::{SemanticIndex, SemanticIndexConfig, SemanticIndexManager, FingerprintIndexEntry as SearchIndexEntry, IndexStats, SearchResult};

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};

/// 上下文存储根目录管理器
pub struct ContextRoot {
    root: PathBuf,
    sessions_dir: PathBuf,
    hashes_dir: PathBuf,
    logs_dir: PathBuf,
}

impl ContextRoot {
    /// 创建或打开上下文根目录
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let sessions_dir = root.join("sessions");
        let hashes_dir = root.join("hashes");
        let logs_dir = root.join("logs");

        // 创建目录结构
        std::fs::create_dir_all(&sessions_dir)
            .with_context(|| format!("Failed to create sessions directory: {:?}", sessions_dir))?;
        std::fs::create_dir_all(&hashes_dir)
            .with_context(|| format!("Failed to create hashes directory: {:?}", hashes_dir))?;
        std::fs::create_dir_all(&logs_dir)
            .with_context(|| format!("Failed to create logs directory: {:?}", logs_dir))?;

        Ok(Self {
            root,
            sessions_dir,
            hashes_dir,
            logs_dir,
        })
    }

    /// 获取会话目录路径
    pub fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id)
    }

    /// 获取哈希索引目录
    pub fn hashes_dir(&self) -> &Path {
        &self.hashes_dir
    }

    /// 获取日志目录
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// 创建会话目录结构
    pub fn create_session(&self, session_id: &str) -> Result<SessionDirs> {
        let session_dir = self.session_dir(session_id);
        let transient_dir = session_dir.join("transient");
        let short_term_dir = session_dir.join("short-term");
        let long_term_dir = session_dir.join("long-term");

        std::fs::create_dir_all(&transient_dir)
            .with_context(|| format!("Failed to create transient directory: {:?}", transient_dir))?;
        std::fs::create_dir_all(&short_term_dir)
            .with_context(|| format!("Failed to create short-term directory: {:?}", short_term_dir))?;
        std::fs::create_dir_all(&long_term_dir)
            .with_context(|| format!("Failed to create long-term directory: {:?}", long_term_dir))?;

        // 创建长期层的子分类目录
        std::fs::create_dir_all(long_term_dir.join("git_rules"))?;
        std::fs::create_dir_all(long_term_dir.join("tool_configs"))?;
        std::fs::create_dir_all(long_term_dir.join("task_patterns"))?;

        Ok(SessionDirs {
            session_dir,
            transient_dir,
            short_term_dir,
            long_term_dir,
        })
    }

    /// 清理会话（删除整个会话目录）
    pub fn remove_session(&self, session_id: &str) -> Result<()> {
        let session_dir = self.session_dir(session_id);
        if session_dir.exists() {
            std::fs::remove_dir_all(&session_dir)
                .with_context(|| format!("Failed to remove session directory: {:?}", session_dir))?;
        }
        Ok(())
    }

    /// 获取根目录
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// 会话级目录结构
pub struct SessionDirs {
    pub session_dir: PathBuf,
    pub transient_dir: PathBuf,
    pub short_term_dir: PathBuf,
    pub long_term_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_context_root_creation() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = ContextRoot::new(temp_dir.path()).unwrap();
        
        assert!(context_root.root().exists());
        assert!(context_root.sessions_dir.exists());
        assert!(context_root.hashes_dir.exists());
        assert!(context_root.logs_dir.exists());
    }

    #[test]
    fn test_create_session() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = ContextRoot::new(temp_dir.path()).unwrap();
        
        let session_dirs = context_root.create_session("test_session").unwrap();
        
        assert!(session_dirs.session_dir.exists());
        assert!(session_dirs.transient_dir.exists());
        assert!(session_dirs.short_term_dir.exists());
        assert!(session_dirs.long_term_dir.exists());
        assert!(session_dirs.long_term_dir.join("git_rules").exists());
        assert!(session_dirs.long_term_dir.join("tool_configs").exists());
        assert!(session_dirs.long_term_dir.join("task_patterns").exists());
    }

    #[test]
    fn test_remove_session() {
        let temp_dir = TempDir::new().unwrap();
        let context_root = ContextRoot::new(temp_dir.path()).unwrap();
        
        context_root.create_session("test_session").unwrap();
        context_root.remove_session("test_session").unwrap();
        
        assert!(!context_root.session_dir("test_session").exists());
    }
}
