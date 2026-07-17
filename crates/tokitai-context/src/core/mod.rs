//! Core context storage modules
//! 
//! This module provides the fundamental context storage infrastructure
//! using a "files as database" approach with layered storage.

pub use super::file_service::{
    FileContextService, FileContextConfig, CloudContextItem, CloudPayload,
};
pub use super::layers::{
    StorageLayer, TransientLayer, ShortTermLayer, LongTermLayer,
    LongTermConfig, ContentMetadata, ContentType, StoredItem,
};
pub use super::hash_index::HashIndex;
pub use super::logger::{ContextLogger, LogEntry, LogOperation};
pub use super::hash_chain::{
    HashChain, HashChainManager, ChainNode, HashChainSnapshot, CloudChainPayload,
};
pub use super::distiller::{
    ContextDistiller, DistillerConfig, DistilledSummary,
    ToolDependency, ToolStatus, DistillationCache,
    CacheStats as DistillerCacheStats,
};
pub use super::knowledge_index::{KnowledgeIndex, KnowledgeNode, KnowledgeStats};
pub use super::knowledge_watcher::KnowledgeWatcher;
pub use super::path_resolver::resolve_paths;
