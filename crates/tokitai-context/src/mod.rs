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
//! - **上下文窗口管理（PEND-001）**: 基于重要性的上下文保留策略

#![allow(dead_code)]
#![allow(unused_imports)]

mod file_service;
mod hash_index;
mod layers;
mod logger;
mod hash_chain;
mod distiller;
mod semantic_index;
mod knowledge_index;
mod knowledge_watcher;
mod path_resolver;
mod window_manager;
mod unified_manager;
mod branch;
mod graph;
mod merge;
mod parallel_manager;
mod cow;
mod cache;
mod three_way_merge;
mod bloom_conflict;
mod optimized_merge;
mod storage_optimization;
mod parallel_merge;
mod lru_cache;
mod benchmarks;
mod hirschberg_lcs;
mod minhash_lsh;
mod cuckoo_filter;
mod dictionary_compression;
mod arc_cache;

// AI 集成模块（Phase 3）
mod ai_resolver;
mod purpose_inference;
mod smart_merge;
mod summarizer;
mod ai_enhanced_manager;

#[allow(unused_imports)]
pub use file_service::{FileContextService, FileContextConfig, CloudContextItem, CloudPayload};
#[allow(unused_imports)]
pub use layers::{StorageLayer, TransientLayer, ShortTermLayer, LongTermLayer, LongTermConfig, ContentMetadata, ContentType, StoredItem};
#[allow(unused_imports)]
pub use hash_index::HashIndex;
#[allow(unused_imports)]
pub use logger::{ContextLogger, LogEntry, LogOperation};
#[allow(unused_imports)]
pub use hash_chain::{HashChain, HashChainManager, ChainNode, HashChainSnapshot, CloudChainPayload};
#[allow(unused_imports)]
pub use distiller::{ContextDistiller, DistillerConfig, DistilledSummary, ToolDependency, ToolStatus, DistillationCache, CacheStats as DistillerCacheStats};
#[allow(unused_imports)]
pub use semantic_index::{SemanticIndex, SemanticIndexConfig, SemanticIndexManager, FingerprintIndexEntry as SearchIndexEntry, IndexStats, SearchResult};
#[allow(unused_imports)]
pub use knowledge_index::{KnowledgeIndex, KnowledgeNode, KnowledgeStats};
#[allow(unused_imports)]
pub use knowledge_watcher::KnowledgeWatcher;
#[allow(unused_imports)]
pub use path_resolver::resolve_paths;
#[allow(unused_imports)]
pub use window_manager::{
    WindowManager, WindowManagerConfig, ImportanceWeights,
    ContextItem, ContextItemType, ImportanceScore, WindowState, WindowStats,
};
#[allow(unused_imports)]
pub use unified_manager::{
    UnifiedContextManager, UnifiedManagerConfig, MergeStrategy as UnifiedMergeStrategy,
    UnifiedContextItem, ContextLayerType, ContextSource, UnifiedStats,
};

// Parallel context management exports
#[allow(unused_imports)]
pub use branch::{
    BranchMetadata, BranchState, ConflictType, ContextBranch, MergeDecision,
    MergeStrategy,
};
#[allow(unused_imports)]
pub use graph::{
    BranchPoint, Conflict, ConflictResolution, ConflictVersion, ContextGraph,
    ContextGraphManager, ContextGraphStats, MergeDecision as GraphMergeDecision,
    MergeRecord, MergedItem,
};
#[allow(unused_imports)]
pub use merge::{compute_diff, BranchDiff, ContextItem as MergeContextItem, Merger, MergeResult, ModifiedItem};
#[allow(unused_imports)]
pub use parallel_manager::{
    ParallelContextManager, ParallelContextManagerConfig,
};
#[allow(unused_imports)]
pub use cow::{
    BranchCloner, CowConfig, CowManager, CowStats, ForkResult,
};

// AI 集成模块导出（Phase 3）
#[allow(unused_imports)]
pub use ai_resolver::{
    AIConflictResolver, ConflictResolutionRequest, ConflictResolutionResponse,
    ConflictAnalysisReport, ResolverStats,
};
#[allow(unused_imports)]
pub use purpose_inference::{
    AIPurposeInference, PurposeInferenceRequest, PurposeInferenceResult,
    BranchType, InferenceStats,
};
#[allow(unused_imports)]
pub use smart_merge::{
    AISmartMergeRecommender, MergeRecommendationRequest, MergeRecommendation,
    TimingRecommendation, RiskAssessment, ChecklistItem, ChecklistStatus,
    QuickAssessment, RecommenderStats,
};
#[allow(unused_imports)]
pub use summarizer::{
    AIBranchSummarizer, SummaryGenerationRequest, SummaryGenerationResult,
    TimelineEvent, StatusAssessment, MergeReadiness, QuickSummary, SummarizerStats,
};
#[allow(unused_imports)]
pub use ai_enhanced_manager::{
    AIEnhancedContextManager, AIStats,
};

// 缓存优化模块
#[allow(unused_imports)]
pub use cache::{
    AncestorCache, AncestorCacheStats, BranchCache, CacheStats as CacheStatsV1,
    CacheWarmup, CacheWarmupConfig, CachedBranch as CachedBranchV1,
};

// 三路合并优化模块
#[allow(unused_imports)]
pub use three_way_merge::{
    FileMetadata, MergeOutcome, ThreeWayMerger, MergeComparison,
};

// Bloom Filter 冲突检测优化模块
#[allow(unused_imports)]
pub use bloom_conflict::{
    BloomFilter, BloomConflictDetector, BloomStats, PerformanceComparison,
};

// 高级合并算法优化模块
#[allow(unused_imports)]
pub use optimized_merge::{
    AdvancedMerger, ContentDeduplicator, DedupResult, DedupStats,
    Diff3Hunk, Diff3Result, LcsAlignment, SemanticBlock,
    SemanticMergeOutcome, SemanticMergeResult,
};

// 存储优化模块
#[allow(unused_imports)]
pub use storage_optimization::{
    ChangeType, CompressionAlgorithm, CompressionConfig,
    ContentAddressableEntry, ContentAddressableStorage,
    GcResult, IncrementalSnapshot, SnapshotChange, SnapshotManager,
    SnapshotMetadata, StorageStats,
};

// 并行合并优化模块
#[allow(unused_imports)]
pub use parallel_merge::{
    ParallelMerger, ParallelMergeConfig, ParallelMergeResult, ParallelMergeStats,
};

// LRU-K 缓存优化模块
#[allow(unused_imports)]
pub use lru_cache::{
    BranchLRUCache, BranchCacheConfig, CachedBranch, CacheStats,
    ThreadSafeBranchCache,
};

// 性能基准测试模块
#[allow(unused_imports)]
pub use benchmarks::{
    BenchmarkConfig, BenchmarkResult, BenchmarkSuite, run_benchmarks,
};

// Hirschberg LCS 优化模块
#[allow(unused_imports)]
pub use hirschberg_lcs::{HirschbergLCS, OptimizedLcsResult};

// MinHash+LSH 语义索引优化模块
#[allow(unused_imports)]
pub use minhash_lsh::{
    MinHashGenerator, MinHashSignature, LSHConfig, LSHIndex, LSHIndexStats,
    MinHashLSHIndex, DocumentMetadata,
};

// Cuckoo Filter 冲突检测优化模块
#[allow(unused_imports)]
pub use cuckoo_filter::{
    CuckooFilter, CuckooStats, CuckooConflictDetector,
};

// Zstd Dictionary 压缩优化模块
#[allow(unused_imports)]
pub use dictionary_compression::{
    DictionaryCompressor, DictionaryCompressionConfig, DictionaryStats,
    DictionaryMetadata, DictionaryContentAddressableStorage,
};

// ARC 自适应缓存替换算法
#[allow(unused_imports)]
pub use arc_cache::{
    ArcCache, ArcCacheConfig, ArcCacheStats, ArcEntry, BranchArcCache,
};

/// 知识管理器 - 整合知识索引、监听和推荐功能
pub struct KnowledgeManager {
    index: Option<KnowledgeIndex>,
    #[allow(dead_code)]
    watcher: Option<KnowledgeWatcher>,
    auto_recommend: bool,
    recommend_threshold: f32,
    recommend_limit: usize,
}

impl KnowledgeManager {
    /// 创建知识管理器
    pub fn new(
        knowledge_root: Option<&str>,
        auto_recommend: bool,
        recommend_threshold: f32,
        recommend_limit: usize,
    ) -> Result<Self> {
        let (index, watcher) = if let Some(root) = knowledge_root {
            let path = std::path::PathBuf::from(root);
            if path.exists() {
                let idx = KnowledgeIndex::from_directory(&path)?;
                let arc_idx = std::sync::Arc::new(std::sync::RwLock::new(idx.clone()));
                let watcher = match KnowledgeWatcher::new(&path, Arc::clone(&arc_idx)) {
                    Ok(w) => Some(w),
                    Err(e) => {
                        tracing::warn!("创建知识监听器失败：{}", e);
                        None
                    }
                };
                (Some(idx), watcher)
            } else {
                tracing::warn!("知识库目录不存在：{}", root);
                (None, None)
            }
        } else {
            (None, None)
        };

        Ok(Self {
            index,
            watcher,
            auto_recommend,
            recommend_threshold,
            recommend_limit,
        })
    }

    /// 根据问题推荐相关知识
    pub fn recommend(&self, query: &str) -> Vec<&KnowledgeNode> {
        if !self.auto_recommend {
            return Vec::new();
        }

        if let Some(ref idx) = self.index {
            idx.recommend(query, self.recommend_limit)
        } else {
            Vec::new()
        }
    }

    /// 获取知识索引
    pub fn index(&self) -> Option<&KnowledgeIndex> {
        self.index.as_ref()
    }

    /// 获取统计信息
    pub fn stats(&self) -> Option<KnowledgeStats> {
        self.index.as_ref().map(|idx| idx.stats())
    }
}

use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use std::sync::Arc;

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
