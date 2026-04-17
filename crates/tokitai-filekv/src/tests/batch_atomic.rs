//! Tests for atomic put_batch functionality
//!
//! These tests verify:
//! 1. Atomic batch write - all entries written or none
//! 2. Batch WAL recovery after crash
//! 3. Memtable batch insert correctness

use crate::*;
use tempfile::TempDir;

/// Test basic batch write
#[test]
fn test_put_batch_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Write batch
    let entries = vec![
        ("key1", b"value1".as_slice()),
        ("key2", b"value2".as_slice()),
        ("key3", b"value3".as_slice()),
    ];

    kv.put_batch(&entries).expect("put_batch should succeed");

    // Verify all entries are readable
    for (key, expected_value) in &entries {
        let result = kv.get(key).expect("get should succeed");
        assert!(result.is_some(), "Key {} should exist", key);
        assert_eq!(
            result
                .as_ref()
                .unwrap_or_else(|| panic!("Key {} should have a value", key)),
            *expected_value
        );
    }

    // Verify entry count
    let stats = kv.get_stats();
    assert_eq!(stats.write_count, 3);
}

/// Test empty batch
#[test]
fn test_put_batch_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Empty batch should succeed
    let entries: Vec<(&str, &[u8])> = vec![];
    kv.put_batch(&entries)
        .expect("put_batch with empty entries should succeed");

    let stats = kv.get_stats();
    assert_eq!(stats.write_count, 0);
}

/// Test WAL recovery after batch write
#[test]
fn test_put_batch_wal_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    // Open KV and write batch
    {
        let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

        let entries = vec![
            ("batch1_key1", b"batch1_value1".as_slice()),
            ("batch1_key2", b"batch1_value2".as_slice()),
        ];

        kv.put_batch(&entries).expect("put_batch should succeed");

        // Drop without flush - simulates crash
        drop(kv);
    }

    // Reopen - should recover from WAL
    let kv2 = FileKV::open(config).expect("Failed to reopen FileKV");

    // Verify recovered entries
    let value1 = kv2.get("batch1_key1").expect("get should succeed for batch1_key1");
    let value2 = kv2.get("batch1_key2").expect("get should succeed for batch1_key2");

    assert!(value1.is_some(), "batch1_key1 should be recovered");
    assert!(value2.is_some(), "batch1_key2 should be recovered");
    assert_eq!(
        value1.as_ref().expect("batch1_key1 value missing").as_ref(),
        b"batch1_value1".as_ref()
    );
    assert_eq!(
        value2.as_ref().expect("batch1_key2 value missing").as_ref(),
        b"batch1_value2".as_ref()
    );
}

/// Test multiple batches
#[test]
fn test_put_batch_multiple() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Write multiple batches
    let batch1 = vec![("m_key1", b"m_value1".as_slice()), ("m_key2", b"m_value2".as_slice())];

    let batch2 = vec![
        ("m_key3", b"m_value3".as_slice()),
        ("m_key4", b"m_value4".as_slice()),
        ("m_key5", b"m_value5".as_slice()),
    ];

    kv.put_batch(&batch1).expect("put_batch batch1 should succeed");
    kv.put_batch(&batch2).expect("put_batch batch2 should succeed");

    // Verify all entries
    for i in 1..=5 {
        let key = format!("m_key{}", i);
        let expected = format!("m_value{}", i);
        let result = kv.get(&key).expect("get should succeed");
        assert!(result.is_some(), "Key {} should exist", key);
        assert_eq!(
            result
                .as_ref()
                .unwrap_or_else(|| panic!("Key {} should have a value", key)),
            expected.as_bytes()
        );
    }

    let stats = kv.get_stats();
    assert_eq!(stats.write_count, 5);
}

/// Test batch overwrite
#[test]
fn test_put_batch_overwrite() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Write initial values
    kv.put("ow_key1", b"initial1").expect("put ow_key1 should succeed");
    kv.put("ow_key2", b"initial2").expect("put ow_key2 should succeed");

    // Batch overwrite
    let entries = vec![("ow_key1", b"new1".as_slice()), ("ow_key2", b"new2".as_slice())];

    kv.put_batch(&entries).expect("put_batch overwrite should succeed");

    // Verify new values
    assert_eq!(
        kv.get("ow_key1")
            .expect("get ow_key1 should succeed")
            .expect("ow_key1 value missing")
            .as_ref(),
        b"new1"
    );
    assert_eq!(
        kv.get("ow_key2")
            .expect("get ow_key2 should succeed")
            .expect("ow_key2 value missing")
            .as_ref(),
        b"new2"
    );
}

/// Test single entry batch
#[test]
fn test_put_batch_single() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Single entry batch
    let entries = vec![("single_key", b"single_value".as_slice())];
    kv.put_batch(&entries).expect("put_batch single entry should succeed");

    let result = kv.get("single_key").expect("get should succeed");
    assert!(result.is_some());
    assert_eq!(
        result.as_ref().expect("single_key value missing").as_ref(),
        b"single_value"
    );
}

/// Test basic get_batch
#[test]
fn test_get_batch_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Write some keys
    kv.put("gb_key1", b"gb_value1").unwrap();
    kv.put("gb_key2", b"gb_value2").unwrap();
    kv.put("gb_key3", b"gb_value3").unwrap();

    // Batch get: mix of existing and non-existing
    let results = kv.get_batch(&["gb_key1", "gb_key2", "gb_key4", "gb_key3"]).unwrap();

    assert_eq!(results.len(), 4);
    assert_eq!(results[0].as_ref().unwrap().as_ref(), b"gb_value1");
    assert_eq!(results[1].as_ref().unwrap().as_ref(), b"gb_value2");
    assert!(results[2].is_none(), "gb_key4 should not exist");
    assert_eq!(results[3].as_ref().unwrap().as_ref(), b"gb_value3");
}

/// Test get_batch empty
#[test]
fn test_get_batch_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    let results: Vec<Option<Bytes>> = kv.get_batch(&[]).unwrap();
    assert!(results.is_empty());
}

/// Test get_batch after delete_batch
#[test]
fn test_delete_batch_and_verify() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Write keys
    kv.put_batch(&[("db_key1", b"v1"), ("db_key2", b"v2"), ("db_key3", b"v3")])
        .unwrap();

    // Delete batch
    kv.delete_batch(&["db_key1", "db_key3"]).unwrap();

    // Verify tombstones
    let results = kv.get_batch(&["db_key1", "db_key2", "db_key3"]).unwrap();
    // Deleted keys return empty value (tombstone)
    assert_eq!(results[0].as_ref().map(|b| b.as_ref()), Some(b"".as_ref()));
    assert_eq!(results[1].as_ref().unwrap().as_ref(), b"v2");
    assert_eq!(results[2].as_ref().map(|b| b.as_ref()), Some(b"".as_ref()));
}

/// Test delete_batch empty
#[test]
fn test_delete_batch_empty() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Empty delete should succeed
    kv.delete_batch(&[]).unwrap();
}

/// Test delete_batch idempotent (deleting non-existent key is safe)
#[test]
fn test_delete_batch_non_existent() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).expect("Failed to open FileKV");

    // Deleting non-existent keys should not error
    kv.delete_batch(&["no_such_key", "also_no_such_key"]).unwrap();
}
