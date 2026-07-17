//! Context storage root directory manager
//!
//! This module provides utilities for managing the context storage root directory
//! and session directory structure.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::error::{ContextResult, ContextError};
use crate::knowledge_index::{KnowledgeIndex, KnowledgeNode, KnowledgeStats};
use crate::knowledge_watcher::KnowledgeWatcher;

/// Context storage root directory manager
pub struct ContextRoot {
    root: PathBuf,
    sessions_dir: PathBuf,
    hashes_dir: PathBuf,
    logs_dir: PathBuf,
}

impl ContextRoot {
    /// Create or open context root directory
    pub fn new<P: AsRef<Path>>(root: P) -> ContextResult<Self> {
        let root = root.as_ref().to_path_buf();
        let sessions_dir = root.join("sessions");
        let hashes_dir = root.join("hashes");
        let logs_dir = root.join("logs");

        // Create directory structure
        std::fs::create_dir_all(&sessions_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create sessions directory: {:?}: {}", sessions_dir, e)))?;
        std::fs::create_dir_all(&hashes_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create hashes directory: {:?}: {}", hashes_dir, e)))?;
        std::fs::create_dir_all(&logs_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create logs directory: {:?}: {}", logs_dir, e)))?;

        Ok(Self {
            root,
            sessions_dir,
            hashes_dir,
            logs_dir,
        })
    }

    /// Get session directory path
    pub fn session_dir(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(session_id)
    }

    /// Get hashes directory
    pub fn hashes_dir(&self) -> &Path {
        &self.hashes_dir
    }

    /// Get logs directory
    pub fn logs_dir(&self) -> &Path {
        &self.logs_dir
    }

    /// Create session directory structure
    pub fn create_session(&self, session_id: &str) -> ContextResult<SessionDirs> {
        let session_dir = self.session_dir(session_id);
        let transient_dir = session_dir.join("transient");
        let short_term_dir = session_dir.join("short-term");
        let long_term_dir = session_dir.join("long-term");

        std::fs::create_dir_all(&transient_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create transient directory: {:?}: {}", transient_dir, e)))?;
        std::fs::create_dir_all(&short_term_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create short-term directory: {:?}: {}", short_term_dir, e)))?;
        std::fs::create_dir_all(&long_term_dir)
            .map_err(ContextError::Io)
            .map_err(|e| ContextError::OperationFailed(format!("Failed to create long-term directory: {:?}: {}", long_term_dir, e)))?;

        // Create subdirectories for long-term layer
        std::fs::create_dir_all(long_term_dir.join("git_rules"))
            .map_err(ContextError::Io)?;
        std::fs::create_dir_all(long_term_dir.join("tool_configs"))
            .map_err(ContextError::Io)?;
        std::fs::create_dir_all(long_term_dir.join("task_patterns"))
            .map_err(ContextError::Io)?;

        Ok(SessionDirs {
            session_dir,
            transient_dir,
            short_term_dir,
            long_term_dir,
        })
    }

    /// Remove session (delete entire session directory)
    pub fn remove_session(&self, session_id: &str) -> ContextResult<()> {
        let session_dir = self.session_dir(session_id);
        if session_dir.exists() {
            std::fs::remove_dir_all(&session_dir)
                .map_err(ContextError::Io)
                .map_err(|e| ContextError::OperationFailed(format!("Failed to remove session directory: {:?}: {}", session_dir, e)))?;
        }
        Ok(())
    }

    /// Get root directory
    pub fn root(&self) -> &Path {
        &self.root
    }
}

/// Session directory structure
pub struct SessionDirs {
    pub session_dir: PathBuf,
    pub transient_dir: PathBuf,
    pub short_term_dir: PathBuf,
    pub long_term_dir: PathBuf,
}

/// Knowledge manager - integrates indexing, watching, and recommendation
pub struct KnowledgeManager {
    index: Option<KnowledgeIndex>,
    #[allow(dead_code)]
    watcher: Option<KnowledgeWatcher>,
    auto_recommend: bool,
    recommend_threshold: f32,
    recommend_limit: usize,
}

impl KnowledgeManager {
    /// Create knowledge manager
    pub fn new(
        knowledge_root: Option<&str>,
        auto_recommend: bool,
        recommend_threshold: f32,
        recommend_limit: usize,
    ) -> ContextResult<Self> {
        let (index, watcher) = if let Some(root) = knowledge_root {
            let path = std::path::PathBuf::from(root);
            if path.exists() {
                let idx = KnowledgeIndex::from_directory(&path)?;
                let arc_idx = std::sync::Arc::new(std::sync::RwLock::new(idx.clone()));
                let watcher = match KnowledgeWatcher::new(&path, Arc::clone(&arc_idx)) {
                    Ok(w) => Some(w),
                    Err(e) => {
                        tracing::warn!("Failed to create knowledge watcher: {}", e);
                        None
                    }
                };
                (Some(idx), watcher)
            } else {
                tracing::warn!("Knowledge directory does not exist: {}", root);
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

    /// Recommend knowledge based on query
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

    /// Get knowledge index
    pub fn index(&self) -> Option<&KnowledgeIndex> {
        self.index.as_ref()
    }

    /// Get statistics
    pub fn stats(&self) -> Option<KnowledgeStats> {
        self.index.as_ref().map(|idx| idx.stats())
    }
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
