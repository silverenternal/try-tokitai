//! Compaction Crash Scenario Tests
//!
//! Tests 5 crash scenarios during compaction:
//! 1. Crash before manifest write
//! 2. Crash after manifest write, before output write
//! 3. Crash during output write (partial output)
//! 4. Crash after output write, before commit
//! 5. Crash during input segment deletion (partial cleanup)

use std::path::PathBuf;
use std::sync::Arc;

use super::manifest::{recover_incomplete, CompactionManifest, RecoveryAction};
use crate::io::{FileKVFileSystem, MemFs};

/// Scenario 1: Crash BEFORE manifest write
/// Expected: No manifest exists, no cleanup needed
#[test]
fn test_crash_before_manifest_write() {
    let fs = Arc::new(MemFs::default());
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // Simulate crash: no manifest written yet
    // Recovery should find nothing to clean up
    let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

    assert!(
        actions.is_empty(),
        "No manifest should exist, no recovery action expected"
    );
}

/// Scenario 2: Crash AFTER manifest write, BEFORE any output written
/// Expected: Manifest exists with InProgress status, recovery should restore input segments
#[test]
fn test_crash_after_manifest_write_before_output() {
    let fs = Arc::new(MemFs::default());
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // Write manifest (simulating successful prepare)
    let manifest = CompactionManifest::new(
        100,
        vec![1, 2, 3], // Input segments
        vec![10],      // Planned output
        1,
    );
    manifest.write_atomic(fs.as_ref(), &manifest_dir, 100).unwrap();

    // Create input segments (they should be preserved)
    for &id in &[1, 2, 3] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        fs.create_file(&path).unwrap();
    }

    // Crash happens here - no output written

    // Recovery
    let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        RecoveryAction::CleanedUp {
            compaction_id,
            deleted_output_segments,
            restored_input_segments,
        } => {
            assert_eq!(*compaction_id, 100);
            assert!(deleted_output_segments.is_empty(), "No output segments should exist");
            assert_eq!(restored_input_segments, &vec![1, 2, 3]);
        }
        _ => panic!("Expected CleanedUp action"),
    }

    // Verify input segments still exist
    for &id in &[1, 2, 3] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        assert!(fs.file_exists(&path), "Input segment {} should be preserved", id);
    }

    // Verify manifest was cleaned up
    let manifest_path = manifest_dir.join("compaction_100.manifest");
    assert!(!fs.file_exists(&manifest_path), "Manifest should be cleaned up");
}

/// Scenario 3: Crash DURING output write (partial output created)
/// Expected: Recovery deletes partial outputs, restores inputs
#[test]
fn test_crash_during_output_write_partial_output() {
    let fs = Arc::new(MemFs::default());
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // Write manifest
    let manifest = CompactionManifest::new(
        200,
        vec![5, 6],   // Input segments
        vec![20, 21], // Planned outputs (2 outputs)
        1,
    );
    manifest.write_atomic(fs.as_ref(), &manifest_dir, 200).unwrap();

    // Create input segments
    for &id in &[5, 6] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        fs.create_file(&path).unwrap();
    }

    // Simulate partial output: only first output segment written before crash
    let output_path_20 = segment_dir.join("segment_20.log");
    fs.create_file(&output_path_20).unwrap();
    // segment_21.log NOT written (crash before completion)

    // Also create dense index for partial output
    let dense_idx_20 = segment_dir.join("segment_20.dense_idx");
    fs.create_file(&dense_idx_20).unwrap();

    // Recovery
    let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        RecoveryAction::CleanedUp {
            compaction_id,
            deleted_output_segments,
            restored_input_segments,
        } => {
            assert_eq!(*compaction_id, 200);
            assert_eq!(deleted_output_segments, &vec![20]);
            assert_eq!(restored_input_segments, &vec![5, 6]);
        }
        _ => panic!("Expected CleanedUp action"),
    }

    // Verify partial output deleted
    assert!(
        !fs.file_exists(&output_path_20),
        "Partial output segment_20 should be deleted"
    );
    assert!(!fs.file_exists(&dense_idx_20), "Partial dense index should be deleted");

    // Verify input segments preserved
    for &id in &[5, 6] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        assert!(fs.file_exists(&path), "Input segment {} should be preserved", id);
    }
}

/// Scenario 4: Crash AFTER all outputs written, BEFORE commit
/// Expected: Recovery deletes ALL outputs, restores inputs
#[test]
fn test_crash_after_output_write_before_commit() {
    let fs = Arc::new(MemFs::default());
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // Write manifest
    let manifest = CompactionManifest::new(
        300,
        vec![7, 8],   // Input segments
        vec![30, 31], // Output segments (2 outputs)
        1,
    );
    manifest.write_atomic(fs.as_ref(), &manifest_dir, 300).unwrap();

    // Create input segments
    for &id in &[7, 8] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        fs.create_file(&path).unwrap();
    }

    // Simulate: all outputs written, but manifest not committed
    for &id in &[30, 31] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        fs.create_file(&path).unwrap();

        // Also create indexes
        let dense_idx = segment_dir.join(format!("segment_{}.dense_idx", id));
        fs.create_file(&dense_idx).unwrap();

        let sparse_idx = segment_dir.join(format!("segment_{}.idx", id));
        fs.create_file(&sparse_idx).unwrap();
    }

    // Crash before commit

    // Recovery
    let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        RecoveryAction::CleanedUp {
            compaction_id,
            deleted_output_segments,
            restored_input_segments,
        } => {
            assert_eq!(*compaction_id, 300);
            assert_eq!(deleted_output_segments, &vec![30, 31]);
            assert_eq!(restored_input_segments, &vec![7, 8]);
        }
        _ => panic!("Expected CleanedUp action"),
    }

    // Verify all outputs deleted
    for &id in &[30, 31] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        assert!(!fs.file_exists(&path), "Output segment {} should be deleted", id);

        let dense_idx = segment_dir.join(format!("segment_{}.dense_idx", id));
        assert!(!fs.file_exists(&dense_idx), "Dense index {} should be deleted", id);

        let sparse_idx = segment_dir.join(format!("segment_{}.idx", id));
        assert!(!fs.file_exists(&sparse_idx), "Sparse index {} should be deleted", id);
    }

    // Verify inputs preserved
    for &id in &[7, 8] {
        let path = segment_dir.join(format!("segment_{}.log", id));
        assert!(fs.file_exists(&path), "Input segment {} should be preserved", id);
    }
}

/// Scenario 5: Crash DURING input deletion (some inputs deleted, some remain)
/// Expected: Recovery cleans up remaining manifest, doesn't touch already-deleted inputs
#[test]
fn test_crash_during_input_deletion_partial() {
    let fs = Arc::new(MemFs::default());
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // Simulate: compaction completed and committed, manifest marked Completed
    let mut manifest = CompactionManifest::new(
        400,
        vec![40, 41], // Input segments (already deleted before crash)
        vec![50],     // Output segment
        1,
    );
    manifest.mark_completed();
    manifest.write_atomic(fs.as_ref(), &manifest_dir, 400).unwrap();

    // Create output segment (successfully created)
    let output_path = segment_dir.join("segment_50.log");
    fs.create_file(&output_path).unwrap();

    // Input segments already deleted (simulating successful compaction)
    // segment_40.log and segment_41.log don't exist

    // Crash after input deletion but before manifest cleanup

    // Recovery
    let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

    assert!(actions.is_empty(), "Completed compaction should not trigger cleanup");

    // Verify output segment preserved
    assert!(fs.file_exists(&output_path), "Output segment should be preserved");

    // Verify manifest cleaned up
    let manifest_path = manifest_dir.join("compaction_400.manifest");
    assert!(
        !fs.file_exists(&manifest_path),
        "Completed manifest should be cleaned up"
    );
}

/// Bonus Test: Multiple incomplete compactions
#[test]
fn test_multiple_incomplete_compactions() {
    let fs = Arc::new(MemFs::default());
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // Write 3 incomplete manifests
    for i in 1..=3 {
        let manifest = CompactionManifest::new(i * 100, vec![i], vec![i * 10], 0);
        manifest.write_atomic(fs.as_ref(), &manifest_dir, i * 100).unwrap();

        // Create partial output
        let output_path = segment_dir.join(format!("segment_{}.log", i * 10));
        fs.create_file(&output_path).unwrap();
    }

    // Recovery
    let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

    assert_eq!(actions.len(), 3, "Should recover 3 incomplete compactions");

    for (i, action) in actions.iter().enumerate() {
        let compaction_id = (i + 1) as u64 * 100;
        match action {
            RecoveryAction::CleanedUp {
                compaction_id: cid,
                deleted_output_segments,
                restored_input_segments,
            } => {
                assert_eq!(*cid, compaction_id);
                assert_eq!(deleted_output_segments, &vec![(i + 1) as u64 * 10]);
                assert_eq!(restored_input_segments, &vec![(i + 1) as u64]);
            }
            _ => panic!("Expected CleanedUp action"),
        }
    }

    // Verify all outputs deleted
    for i in 1..=3 {
        let output_path = segment_dir.join(format!("segment_{}.log", i * 10));
        assert!(
            !fs.file_exists(&output_path),
            "Output segment {} should be deleted",
            i * 10
        );
    }

    // Verify all manifests cleaned up
    for i in 1..=3 {
        let manifest_path = manifest_dir.join(format!("compaction_{}.manifest", i * 100));
        assert!(
            !fs.file_exists(&manifest_path),
            "Manifest {} should be cleaned up",
            i * 100
        );
    }
}

/// Bonus Test: Corrupt manifest recovery
#[test]
fn test_corrupt_manifest_recovery() {
    let fs = Arc::new(MemFs::default());
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // Write a corrupt manifest file
    let corrupt_path = manifest_dir.join("compaction_999.manifest");
    let mut file = fs.create_file(&corrupt_path).unwrap();
    file.write_all(b"this is not valid json").unwrap();
    file.sync_all().unwrap();

    // Recovery should handle corrupt manifest gracefully
    let actions = recover_incomplete(fs.as_ref(), &manifest_dir, &segment_dir).unwrap();

    assert!(actions.is_empty(), "Corrupt manifest should be skipped, not recovered");

    // Corrupt manifest should be cleaned up
    assert!(!fs.file_exists(&corrupt_path), "Corrupt manifest should be removed");
}

/// Bonus Test: Empty manifest directory
#[test]
fn test_empty_manifest_directory() {
    let fs = MemFs::default();
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    fs.create_dir_all(&manifest_dir).unwrap();
    fs.create_dir_all(&segment_dir).unwrap();

    // No manifests written
    let actions = recover_incomplete(&fs, &manifest_dir, &segment_dir).unwrap();

    assert!(actions.is_empty(), "Empty directory should not trigger recovery");
}

/// Bonus Test: Manifest directory doesn't exist
#[test]
fn test_manifest_directory_missing() {
    let fs = MemFs::default();
    let manifest_dir = PathBuf::from("/manifests");
    let segment_dir = PathBuf::from("/segments");

    // Don't create directories
    let actions = recover_incomplete(&fs, &manifest_dir, &segment_dir).unwrap();

    assert!(actions.is_empty(), "Missing directory should not cause error");
}
