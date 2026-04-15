//! Compaction Manifest - Crash-safe compaction tracking
//!
//! This module provides manifest-based crash recovery for compaction operations.
//! Before compaction starts, a manifest file is written atomically recording the
//! input segments and intended output. If the process crashes during compaction,
//! the manifest is used to clean up incomplete output and restore references to
//! the input segments.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::io::FileKVFileSystem;

/// Compaction manifest file magic number
const COMPACTION_MANIFEST_MAGIC: u32 = 0x434D414E; // "CMAN"

/// Compaction manifest file version
const COMPACTION_MANIFEST_VERSION: u32 = 1;

/// Status of a compaction operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompactionStatus {
    /// Compaction is in progress - started but not yet committed
    InProgress,
    /// Compaction completed successfully
    Completed,
    /// Compaction was aborted (clean shutdown)
    Aborted,
}

/// Recovery action returned by recover_incomplete
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// No incomplete compaction found
    None,
    /// Cleaned up incomplete compaction output
    CleanedUp {
        compaction_id: u64,
        deleted_output_segments: Vec<u64>,
        restored_input_segments: Vec<u64>,
    },
}

/// Compaction manifest - records the state of a compaction operation
///
/// This is written atomically BEFORE compaction starts, so if the process
/// crashes, we can detect the incomplete compaction and clean up.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionManifest {
    /// Unique identifier for this compaction
    pub compaction_id: u64,
    /// Input segment IDs being compacted
    pub input_segments: Vec<u64>,
    /// Output segment IDs that will be created
    pub output_segments: Vec<u64>,
    /// Output level for the new segments
    pub output_level: u8,
    /// Current status of the compaction
    pub status: CompactionStatus,
    /// Timestamp when compaction started (Unix epoch seconds)
    pub started_at: u64,
    /// Timestamp when compaction completed/aborted (Unix epoch seconds)
    pub completed_at: Option<u64>,
    /// Optional: estimated size of output in bytes
    pub estimated_output_size_bytes: Option<u64>,
}

impl CompactionManifest {
    /// Create a new manifest for a compaction that is about to start
    pub fn new(
        compaction_id: u64,
        input_segments: Vec<u64>,
        output_segments: Vec<u64>,
        output_level: u8,
    ) -> Self {
        Self {
            compaction_id,
            input_segments,
            output_segments,
            output_level,
            status: CompactionStatus::InProgress,
            started_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            completed_at: None,
            estimated_output_size_bytes: None,
        }
    }

    /// Mark the manifest as completed
    pub fn mark_completed(&mut self) {
        self.status = CompactionStatus::Completed;
        self.completed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Mark the manifest as aborted
    pub fn mark_aborted(&mut self) {
        self.status = CompactionStatus::Aborted;
        self.completed_at = Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    /// Serialize the manifest to JSON bytes with header
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let json = serde_json::to_vec_pretty(self)?;
        let mut buf = Vec::with_capacity(8 + json.len());
        buf.extend_from_slice(&COMPACTION_MANIFEST_MAGIC.to_le_bytes());
        buf.extend_from_slice(&COMPACTION_MANIFEST_VERSION.to_le_bytes());
        buf.extend_from_slice(&json);
        Ok(buf)
    }

    /// Deserialize the manifest from bytes
    pub fn from_bytes(buf: &[u8]) -> anyhow::Result<Self> {
        if buf.len() < 8 {
            anyhow::bail!("Manifest buffer too short");
        }

        let magic = u32::from_le_bytes(buf[0..4].try_into()?);
        let version = u32::from_le_bytes(buf[4..8].try_into()?);

        if magic != COMPACTION_MANIFEST_MAGIC {
            anyhow::bail!("Invalid manifest magic: expected 0x{:08X}, got 0x{:08X}",
                COMPACTION_MANIFEST_MAGIC, magic);
        }

        if version != COMPACTION_MANIFEST_VERSION {
            anyhow::bail!("Unsupported manifest version: {}", version);
        }

        let manifest: CompactionManifest = serde_json::from_slice(&buf[8..])?;
        Ok(manifest)
    }

    /// Write the manifest to a file atomically (write to temp + rename)
    pub fn write_atomic(
        &self,
        fs: &dyn FileKVFileSystem,
        manifest_dir: &Path,
        compaction_id: u64,
    ) -> anyhow::Result<PathBuf> {
        let manifest_bytes = self.to_bytes()?;

        let temp_path = manifest_dir.join(format!(".compaction_{}.manifest.tmp", compaction_id));
        let final_path = manifest_dir.join(format!("compaction_{}.manifest", compaction_id));

        // Write to temp file
        {
            let mut file = fs.create_file(&temp_path)?;
            file.write_all(&manifest_bytes)?;
            file.flush()?;
            file.sync_all()?;
        }

        // Atomic rename
        fs.rename(&temp_path, &final_path)?;

        // Sync directory to ensure rename is persisted
        let _ = fs.sync_dir(manifest_dir);

        Ok(final_path)
    }

    /// Read and parse a manifest from a file
    pub fn read_from_file(fs: &dyn FileKVFileSystem, path: &Path) -> anyhow::Result<Self> {
        let mut file = fs.open_file(path, true, false, false)?;
        let metadata = file.metadata()?;
        let file_size = metadata.len as usize;
        if file_size == 0 {
            anyhow::bail!("Manifest file is empty: {}", path.display());
        }
        let mut buf = vec![0u8; file_size];
        file.read_exact(&mut buf).map_err(|e|
            anyhow::anyhow!("Failed to read manifest file {}: {}", path.display(), e)
        )?;
        Self::from_bytes(&buf)
    }

    /// Update the manifest file on disk
    pub fn persist(&self, fs: &dyn FileKVFileSystem, manifest_path: &Path) -> anyhow::Result<()> {
        // For safety, write atomically via temp + rename
        let manifest_bytes = self.to_bytes()?;
        let temp_path = manifest_path.with_extension("manifest.tmp");

        {
            let mut file = fs.create_file(&temp_path)?;
            file.write_all(&manifest_bytes)?;
            file.flush()?;
            file.sync_all()?;
        }

        fs.rename(&temp_path, manifest_path)?;
        let _ = fs.sync_dir(manifest_path.parent().unwrap_or(Path::new(".")));

        Ok(())
    }
}

/// Compaction executor - manages the lifecycle of a compaction operation
pub struct CompactionExecutor {
    fs: Arc<dyn FileKVFileSystem>,
    manifest_dir: PathBuf,
    current_manifest_path: Option<PathBuf>,
}

impl CompactionExecutor {
    pub fn new(fs: Arc<dyn FileKVFileSystem>, manifest_dir: PathBuf) -> Self {
        Self {
            fs,
            manifest_dir,
            current_manifest_path: None,
        }
    }

    /// Prepare: Write manifest before compaction starts
    ///
    /// This records the intended compaction operation atomically.
    /// If the process crashes after this point, recover_incomplete()
    /// will detect the InProgress manifest and clean up.
    pub fn prepare(&mut self, manifest: &CompactionManifest) -> anyhow::Result<PathBuf> {
        // Ensure manifest directory exists
        self.fs.create_dir_all(&self.manifest_dir)?;

        let path = manifest.write_atomic(
            self.fs.as_ref(),
            &self.manifest_dir,
            manifest.compaction_id,
        )?;

        self.current_manifest_path = Some(path.clone());
        tracing::info!(
            compaction_id = manifest.compaction_id,
            "Compaction manifest written: {}",
            path.display()
        );

        Ok(path)
    }

    /// Commit: Mark compaction as completed
    pub fn commit(&mut self, manifest: &mut CompactionManifest) -> anyhow::Result<()> {
        manifest.mark_completed();

        if let Some(ref path) = self.current_manifest_path {
            manifest.persist(self.fs.as_ref(), path)?;
            tracing::info!(
                compaction_id = manifest.compaction_id,
                "Compaction committed: {}",
                path.display()
            );
        }

        // After commit, we can optionally delete the manifest file
        // Keeping it for audit/recovery purposes
        Ok(())
    }

    /// Abort: Mark compaction as aborted
    pub fn abort(&mut self, manifest: &mut CompactionManifest) -> anyhow::Result<()> {
        manifest.mark_aborted();

        if let Some(ref path) = self.current_manifest_path {
            manifest.persist(self.fs.as_ref(), path)?;
            tracing::warn!(
                compaction_id = manifest.compaction_id,
                "Compaction aborted: {}",
                path.display()
            );
        }

        Ok(())
    }

    /// Get the current manifest path if any
    pub fn current_manifest_path(&self) -> Option<&Path> {
        self.current_manifest_path.as_deref()
    }
}

/// Scan for incomplete compactions and recover
///
/// This function:
/// 1. Scans the manifest directory for .manifest files
/// 2. Parses each manifest
/// 3. For InProgress manifests: deletes output segments, returns recovery action
/// 4. For Aborted/Completed manifests: cleans up the manifest file
pub fn recover_incomplete(
    fs: &dyn FileKVFileSystem,
    manifest_dir: &Path,
    segment_dir: &Path,
) -> anyhow::Result<Vec<RecoveryAction>> {
    let mut actions = Vec::new();

    // Try to read the manifest directory - if it doesn't exist, nothing to recover
    let entries = match fs.read_dir(manifest_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(actions), // Directory doesn't exist or can't be read
    };

    for entry in entries {
        let path = entry;

        // Only process .manifest files
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if !name.starts_with("compaction_") || !name.ends_with(".manifest") {
                continue;
            }

            // Try to parse the manifest
            let manifest = match CompactionManifest::read_from_file(fs, &path) {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!(
                        "Failed to parse compaction manifest {}: {}",
                        path.display(),
                        e
                    );
                    // Try to clean up the corrupt manifest
                    let _ = fs.remove_file(&path);
                    continue;
                }
            };

            match manifest.status {
                CompactionStatus::InProgress => {
                    tracing::warn!(
                        "Found incomplete compaction {} (ID: {}), recovering...",
                        path.display(),
                        manifest.compaction_id
                    );

                    // Delete any output segments that were created
                    let mut deleted_outputs = Vec::new();
                    for &output_id in &manifest.output_segments {
                        let output_path = segment_dir.join(format!("segment_{}.log", output_id));
                        if fs.file_exists(&output_path) {
                            if let Err(e) = fs.remove_file(&output_path) {
                                tracing::error!(
                                    "Failed to delete incomplete output segment {}: {}",
                                    output_id,
                                    e
                                );
                            } else {
                                tracing::info!(
                                    "Deleted incomplete compaction output segment {}",
                                    output_id
                                );
                                deleted_outputs.push(output_id);
                            }
                        }

                        // Also delete dense index if exists
                        let dense_idx_path = segment_dir.join(format!("segment_{}.dense_idx", output_id));
                        if fs.file_exists(&dense_idx_path) {
                            let _ = fs.remove_file(&dense_idx_path);
                        }

                        // Also delete sparse index if exists
                        let sparse_idx_path = segment_dir.join(format!("segment_{}.idx", output_id));
                        if fs.file_exists(&sparse_idx_path) {
                            let _ = fs.remove_file(&sparse_idx_path);
                        }
                    }

                    // Clean up the manifest
                    let _ = fs.remove_file(&path);

                    actions.push(RecoveryAction::CleanedUp {
                        compaction_id: manifest.compaction_id,
                        deleted_output_segments: deleted_outputs,
                        restored_input_segments: manifest.input_segments.clone(),
                    });
                }
                CompactionStatus::Completed => {
                    tracing::debug!(
                        "Found completed compaction manifest {}, cleaning up",
                        path.display()
                    );
                    let _ = fs.remove_file(&path);
                }
                CompactionStatus::Aborted => {
                    tracing::debug!(
                        "Found aborted compaction manifest {}, cleaning up",
                        path.display()
                    );
                    let _ = fs.remove_file(&path);
                }
            }
        }
    }

    Ok(actions)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::MemFs;
    use std::path::PathBuf;

    #[test]
    fn test_manifest_serialization_roundtrip() {
        let manifest = CompactionManifest::new(
            42,
            vec![1, 2, 3],
            vec![4],
            1,
        );

        let bytes = manifest.to_bytes().unwrap();
        let restored = CompactionManifest::from_bytes(&bytes).unwrap();

        assert_eq!(manifest.compaction_id, restored.compaction_id);
        assert_eq!(manifest.input_segments, restored.input_segments);
        assert_eq!(manifest.output_segments, restored.output_segments);
        assert_eq!(manifest.output_level, restored.output_level);
        assert_eq!(manifest.status, restored.status);
        assert_eq!(manifest.started_at, restored.started_at);
        assert_eq!(manifest.completed_at, restored.completed_at);
    }

    #[test]
    fn test_manifest_write_atomic_and_read() {
        let fs = Arc::new(MemFs::default());
        let manifest = CompactionManifest::new(
            100,
            vec![10, 20],
            vec![30],
            2,
        );

        let dir = PathBuf::from("/manifests");
        fs.create_dir_all(&dir).unwrap();

        let path = manifest.write_atomic(fs.as_ref(), &dir, 100).unwrap();

        let restored = CompactionManifest::read_from_file(fs.as_ref(), &path).unwrap();

        assert_eq!(restored.compaction_id, 100);
        assert_eq!(restored.status, CompactionStatus::InProgress);
        assert_eq!(restored.input_segments, vec![10, 20]);
    }

    #[test]
    fn test_manifest_mark_completed() {
        let mut manifest = CompactionManifest::new(1, vec![1], vec![2], 0);
        assert_eq!(manifest.status, CompactionStatus::InProgress);
        assert!(manifest.completed_at.is_none());

        manifest.mark_completed();
        assert_eq!(manifest.status, CompactionStatus::Completed);
        assert!(manifest.completed_at.is_some());
    }

    #[test]
    fn test_manifest_mark_aborted() {
        let mut manifest = CompactionManifest::new(1, vec![1], vec![2], 0);
        manifest.mark_aborted();
        assert_eq!(manifest.status, CompactionStatus::Aborted);
    }

    #[test]
    fn test_compaction_executor_prepare() {
        let fs = Arc::new(MemFs::default());
        let mut executor = CompactionExecutor::new(
            fs.clone(),
            PathBuf::from("/manifests"),
        );

        let manifest = CompactionManifest::new(
            99,
            vec![1, 2, 3],
            vec![4],
            1,
        );

        let path = executor.prepare(&manifest).unwrap();
        assert!(path.to_string_lossy().contains("compaction_99.manifest"));
        assert_eq!(executor.current_manifest_path(), Some(path.as_path()));
    }

    #[test]
    fn test_recover_incomplete_no_manifests() {
        let fs = MemFs::default();
        let manifest_dir = PathBuf::from("/manifests");
        let segment_dir = PathBuf::from("/segments");

        fs.create_dir_all(&manifest_dir).unwrap();
        fs.create_dir_all(&segment_dir).unwrap();

        let actions = recover_incomplete(&fs, &manifest_dir, &segment_dir).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_recover_incomplete_deletes_output_segments() {
        let fs = Arc::new(MemFs::default());
        let manifest_dir = PathBuf::from("/manifests");
        let segment_dir = PathBuf::from("/segments");

        fs.create_dir_all(&manifest_dir).unwrap();
        fs.create_dir_all(&segment_dir).unwrap();

        // Write an InProgress manifest
        let manifest = CompactionManifest::new(
            55,
            vec![1, 2],
            vec![10],
            1,
        );
        let _path = manifest.write_atomic(fs.as_ref(), &manifest_dir, 55).unwrap();

        // Create a fake output segment (simulating crash during compaction)
        let output_path = segment_dir.join("segment_10.log");
        fs.create_file(&output_path).unwrap();

        // Run recovery
        let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

        assert_eq!(actions.len(), 1);
        match &actions[0] {
            RecoveryAction::CleanedUp {
                compaction_id,
                deleted_output_segments,
                restored_input_segments,
            } => {
                assert_eq!(*compaction_id, 55);
                assert_eq!(deleted_output_segments, &vec![10]);
                assert_eq!(restored_input_segments, &vec![1, 2]);
            }
            _ => panic!("Expected CleanedUp action"),
        }

        // Verify output segment was deleted
        assert!(!fs.file_exists(&output_path));
    }

    #[test]
    fn test_recover_incomplete_cleans_up_completed_manifest() {
        let fs = Arc::new(MemFs::default());
        let manifest_dir = PathBuf::from("/manifests");
        let segment_dir = PathBuf::from("/segments");

        fs.create_dir_all(&manifest_dir).unwrap();
        fs.create_dir_all(&segment_dir).unwrap();

        // Write and complete a manifest
        let mut manifest = CompactionManifest::new(77, vec![1], vec![2], 0);
        manifest.mark_completed();
        manifest.write_atomic(fs.as_ref(), &manifest_dir, 77).unwrap();

        // Run recovery - should clean up the completed manifest
        let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();
        assert!(actions.is_empty()); // No cleanup action needed for completed compactions

        // Verify manifest file was deleted
        let manifest_path = manifest_dir.join("compaction_77.manifest");
        assert!(!fs.file_exists(&manifest_path));
    }
}
