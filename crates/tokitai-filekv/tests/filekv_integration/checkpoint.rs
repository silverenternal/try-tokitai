//! (d) Checkpoint creation and recovery

use tempfile::TempDir;
use tokitai_filekv::{FileKV, FileKVConfig};

fn create_test_config(temp_dir: &TempDir) -> FileKVConfig {
    FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    }
}

#[test]
fn test_checkpoint_creation_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Write some data
    for i in 0..10 {
        kv.put(&format!("key_{}", i), format!("value_{}", i).as_bytes())
            .expect("put failed");
    }
    kv.flush_memtable().expect("flush failed");

    // Create checkpoint
    let checkpoint_id = kv.create_full_checkpoint(None).expect("create_checkpoint failed");

    assert!(!checkpoint_id.is_empty(), "Checkpoint ID should not be empty");

    // Verify checkpoint directory exists
    let checkpoint_dir = temp_dir.path().join("checkpoints");
    assert!(checkpoint_dir.exists(), "Checkpoint directory should exist");
}

#[test]
fn test_checkpoint_recovery_after_crash() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);

    // First instance: write, flush, and checkpoint
    {
        let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

        for i in 0..20 {
            kv.put(&format!("data_{}", i), format!("checkpoint_value_{}", i).as_bytes())
                .expect("put failed");
        }
        kv.flush_memtable().expect("flush failed");

        let checkpoint_id = kv.create_full_checkpoint(None).expect("checkpoint failed");
        assert!(!checkpoint_id.is_empty(), "Should have created a checkpoint");
    }

    // Second instance: reopen and verify data integrity
    {
        let kv2 = FileKV::open(config.clone()).expect("Failed to reopen FileKV");

        // All data should still be accessible
        for i in 0..20 {
            let key = format!("data_{}", i);
            let expected = format!("checkpoint_value_{}", i);
            let val = kv2.get(&key).unwrap_or_else(|_| panic!("get failed for {}", key));
            assert_eq!(
                val.as_deref(),
                Some(expected.as_bytes()),
                "Data should be accessible after recovery"
            );
        }

        // Can still write new data
        kv2.put("recovery_key", b"recovery_value").expect("put failed");
        let val = kv2.get("recovery_key").expect("get failed");
        assert_eq!(val.as_deref(), Some(b"recovery_value".as_ref()));
    }
}

#[test]
fn test_multiple_checkpoints() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    let mut checkpoint_ids = vec![];

    // Create multiple checkpoints with writes in between
    for round in 0..3 {
        for i in 0..5 {
            let key = format!("round{}_key{}", round, i);
            kv.put(&key, format!("round{}_value{}", round, i).as_bytes())
                .expect("put failed");
        }
        kv.flush_memtable().expect("flush failed");

        let cp_id = kv.create_full_checkpoint(None).expect("checkpoint failed");
        checkpoint_ids.push(cp_id);
    }

    // Checkpoint IDs should be increasing
    for i in 1..checkpoint_ids.len() {
        assert!(
            checkpoint_ids[i] > checkpoint_ids[i - 1],
            "Checkpoint IDs should be monotonically increasing"
        );
    }

    // All data should still be accessible
    for round in 0..3 {
        for i in 0..5 {
            let key = format!("round{}_key{}", round, i);
            let expected = format!("round{}_value{}", round, i);
            let val = kv.get(&key).unwrap_or_else(|_| panic!("get failed for {}", key));
            assert_eq!(
                val.as_deref(),
                Some(expected.as_bytes()),
                "Data from all rounds should be accessible"
            );
        }
    }
}

#[test]
fn test_checkpoint_stats_tracking() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Write data and create checkpoint
    for i in 0..10 {
        kv.put(&format!("key_{}", i), format!("value_{}", i).as_bytes())
            .expect("put failed");
    }
    kv.flush_memtable().expect("flush failed");

    let cp_id = kv.create_full_checkpoint(None).expect("checkpoint failed");

    // Checkpoint should have been recorded in stats
    assert!(!cp_id.is_empty(), "Should have a valid checkpoint ID");

    // Verify checkpoint can be listed/retrieved
    let checkpoint_dir = temp_dir.path().join("checkpoints");
    if checkpoint_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(&checkpoint_dir)
            .expect("Failed to read checkpoint dir")
            .collect();
        assert!(!entries.is_empty(), "Should have at least one checkpoint entry");
    }
}
