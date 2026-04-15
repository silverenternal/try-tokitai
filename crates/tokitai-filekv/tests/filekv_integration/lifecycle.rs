//! (a) Complete lifecycle test: open -> put -> get -> flush -> recover -> get

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
fn test_complete_lifecycle_open_put_get_flush() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);

    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Put some data
    kv.put("user:1", b"Alice").expect("Failed to put user:1");
    kv.put("user:2", b"Bob").expect("Failed to put user:2");
    kv.put("user:3", b"Charlie").expect("Failed to put user:3");

    // Verify data before flush
    assert_eq!(
        kv.get("user:1").expect("get failed").as_deref(),
        Some(b"Alice".as_ref())
    );
    assert_eq!(
        kv.get("user:2").expect("get failed").as_deref(),
        Some(b"Bob".as_ref())
    );

    // Flush memtable to segments
    kv.flush_memtable().expect("flush failed");

    // Verify data after flush
    assert_eq!(
        kv.get("user:1").expect("get failed").as_deref(),
        Some(b"Alice".as_ref())
    );
    assert_eq!(
        kv.get("user:3").expect("get failed").as_deref(),
        Some(b"Charlie".as_ref())
    );

    let stats = kv.get_stats();
    assert!(stats.segment_count >= 1, "Should have at least 1 segment after flush");
}

#[test]
fn test_lifecycle_recovery_after_reopen() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);

    // First instance: write and flush
    {
        let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");
        kv.put("persistent_key1", b"persistent_value1").expect("put failed");
        kv.put("persistent_key2", b"persistent_value2").expect("put failed");
        kv.flush_memtable().expect("flush failed");
        // kv is dropped here, closing the store
    }

    // Second instance: reopen and verify data persists
    {
        let kv2 = FileKV::open(config.clone()).expect("Failed to reopen FileKV");

        let val1 = kv2.get("persistent_key1").expect("get failed");
        assert_eq!(
            val1.as_deref(),
            Some(b"persistent_value1".as_ref()),
            "Data should persist after reopen"
        );

        let val2 = kv2.get("persistent_key2").expect("get failed");
        assert_eq!(
            val2.as_deref(),
            Some(b"persistent_value2".as_ref()),
            "Data should persist after reopen"
        );

        // Write more data after recovery
        kv2.put("post_recovery_key", b"post_recovery_value").expect("put failed");
        let val3 = kv2.get("post_recovery_key").expect("get failed");
        assert_eq!(
            val3.as_deref(),
            Some(b"post_recovery_value".as_ref()),
            "Should be able to write after recovery"
        );
    }

    // Third instance: verify everything still persists
    {
        let kv3 = FileKV::open(config.clone()).expect("Failed to reopen FileKV again");
        assert_eq!(
            kv3.get("persistent_key1").expect("get failed").as_deref(),
            Some(b"persistent_value1".as_ref())
        );
        assert_eq!(
            kv3.get("post_recovery_key").expect("get failed").as_deref(),
            Some(b"post_recovery_value".as_ref())
        );
    }
}

#[test]
fn test_lifecycle_delete_and_verify() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Put data
    kv.put("delete_me", b"to_be_deleted").expect("put failed");
    kv.put("keep_me", b"to_be_kept").expect("put failed");

    // Verify both exist
    assert!(kv.get("delete_me").expect("get failed").is_some());
    assert!(kv.get("keep_me").expect("get failed").is_some());

    // Delete one key
    kv.delete("delete_me").expect("delete failed");

    // After delete, get returns Some(b"") (empty value tombstone)
    let deleted_val = kv.get("delete_me").expect("get failed");
    assert_eq!(deleted_val.as_deref(), Some(b"".as_ref()));

    // Other key should still exist
    assert_eq!(
        kv.get("keep_me").expect("get failed").as_deref(),
        Some(b"to_be_kept".as_ref())
    );
}
