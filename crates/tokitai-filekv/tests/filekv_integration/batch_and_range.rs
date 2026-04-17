//! (e) Batch writes and range queries

use tempfile::TempDir;
use tokitai_filekv::{FileKV, FileKVConfig};

fn create_test_config(temp_dir: &TempDir) -> FileKVConfig {
    FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        enable_wal: false,
        ..Default::default()
    }
}

#[test]
fn test_batch_write_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    let entries: Vec<(&str, &[u8])> = vec![
        ("batch_key1", b"batch_value1"),
        ("batch_key2", b"batch_value2"),
        ("batch_key3", b"batch_value3"),
        ("batch_key4", b"batch_value4"),
        ("batch_key5", b"batch_value5"),
    ];

    kv.put_batch(&entries).expect("put_batch failed");

    // Verify all batch entries are accessible
    for (key, expected) in &entries {
        let val = kv.get(key).unwrap_or_else(|_| panic!("get failed for {}", key));
        assert_eq!(
            val.as_deref(),
            Some(*expected),
            "Batch key {} should have correct value",
            key
        );
    }
}

#[test]
fn test_batch_write_with_flush() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Write multiple batches
    for batch_num in 0..5 {
        let entries: Vec<(&str, &[u8])> = (0..20)
            .map(|i| {
                let key = format!("batch{}_key{:03}", batch_num, i);
                let value = format!("batch{}_value{:03}", batch_num, i);
                (leak_string(key), leak_string_bytes(value))
            })
            .collect();

        kv.put_batch(&entries).expect("put_batch failed");
    }

    kv.flush_memtable().expect("flush failed");

    // Verify all data is accessible after flush
    for batch_num in 0..5 {
        for i in 0..20 {
            let key = format!("batch{}_key{:03}", batch_num, i);
            let expected = format!("batch{}_value{:03}", batch_num, i);
            let val = kv.get(&key).unwrap_or_else(|_| panic!("get failed for {}", key));
            assert_eq!(
                val.as_deref(),
                Some(expected.as_bytes()),
                "Batch data should be accessible after flush"
            );
        }
    }
}

#[test]
fn test_range_scan_iteration() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Insert ordered keys
    for i in 0..100 {
        let key = format!("key_{:04}", i);
        let value = format!("value_{:04}", i);
        kv.put(&key, value.as_bytes()).expect("put failed");
    }
    kv.flush_memtable().expect("flush failed");

    // Range scan using the public API
    let start = "key_0010";
    let end = "key_0020";
    let results: Vec<_> = kv.range_collect(start, end, 0).expect("range_scan failed");

    // Should return keys in range [key_0010, key_0020]
    assert!(!results.is_empty(), "Range scan should return results");

    for (key, value) in &results {
        assert!(key.as_str() >= start, "Key {} should be >= start", key);
        assert!(key.as_str() <= end, "Key {} should be <= end", key);
        let expected_value = format!("value_{}", key.strip_prefix("key_").unwrap());
        assert_eq!(
            value.as_slice(),
            expected_value.as_bytes(),
            "Value for key {} should match",
            key
        );
    }
}

#[test]
fn test_range_scan_with_prefix() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Insert keys with different prefixes
    for i in 0..20 {
        kv.put(&format!("user:{}", i), format!("user_data_{}", i).as_bytes())
            .expect("put failed");
    }
    for i in 0..20 {
        kv.put(&format!("order:{}", i), format!("order_data_{}", i).as_bytes())
            .expect("put failed");
    }
    kv.flush_memtable().expect("flush failed");

    // Range scan for user: prefix
    let start = "user:";
    let end = "user;"; // ';' comes after ':' in ASCII
    let results: Vec<_> = kv.range_collect(start, end, 0).expect("range_scan failed");

    assert_eq!(results.len(), 20, "Should find 20 user keys");

    for (key, _) in &results {
        assert!(
            key.starts_with("user:"),
            "All keys should start with 'user:', got {}",
            key
        );
    }
}

#[test]
fn test_batch_write_atomic_semantics() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    // Write initial data
    kv.put("existing_key", b"existing_value").expect("put failed");

    // Batch write with mix of new and existing keys
    let entries: Vec<(&str, &[u8])> = vec![
        ("new_key1", b"new_value1"),
        ("existing_key", b"updated_value"),
        ("new_key2", b"new_value2"),
    ];

    kv.put_batch(&entries).expect("put_batch failed");

    // Verify all keys are consistent
    assert_eq!(
        kv.get("new_key1").expect("get failed").as_deref(),
        Some(b"new_value1".as_ref())
    );
    assert_eq!(
        kv.get("existing_key").expect("get failed").as_deref(),
        Some(b"updated_value".as_ref()),
        "existing_key should be updated"
    );
    assert_eq!(
        kv.get("new_key2").expect("get failed").as_deref(),
        Some(b"new_value2".as_ref())
    );
}

#[test]
fn test_large_batch_write() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV");

    let num_entries = 1000;
    let entries: Vec<(&str, &[u8])> = (0..num_entries)
        .map(|i| {
            let key = format!("large_batch_key{:05}", i);
            let value = format!("large_batch_value{:05}", i);
            (leak_string(key), leak_string_bytes(value))
        })
        .collect();

    kv.put_batch(&entries).expect("put_batch failed for large batch");

    // Spot check some entries
    for i in [0, 100, 500, 999].iter() {
        let key = format!("large_batch_key{:05}", i);
        let expected = format!("large_batch_value{:05}", i);
        let val = kv.get(&key).unwrap_or_else(|_| panic!("get failed for {}", key));
        assert_eq!(val.as_deref(), Some(expected.as_bytes()));
    }

    let stats = kv.get_stats();
    assert!(
        stats.write_count >= num_entries as u64,
        "Should have at least {} writes",
        num_entries
    );
}

// Helper to leak strings for &'static str in batch entries
fn leak_string(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn leak_string_bytes(s: String) -> &'static [u8] {
    Box::leak(s.into_bytes().into_boxed_slice())
}
