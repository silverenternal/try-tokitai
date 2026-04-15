//! (b) Concurrent put/get verification

use std::sync::Arc;
use std::thread;
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
fn test_concurrent_puts_different_keys() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    let num_threads = 4;
    let keys_per_thread = 50;
    let mut handles = vec![];

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let handle = thread::spawn(move || {
            for i in 0..keys_per_thread {
                let key = format!("thread{}_key{}", t, i);
                let value = format!("value_from_thread{}_key{}", t, i);
                kv_clone
                    .put(&key, value.as_bytes())
                    .unwrap_or_else(|e| panic!("Thread {} put failed: {}", t, e));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Verify all keys are present
    let expected_total = num_threads * keys_per_thread;
    let mut found = 0;
    for t in 0..num_threads {
        for i in 0..keys_per_thread {
            let key = format!("thread{}_key{}", t, i);
            let expected = format!("value_from_thread{}_key{}", t, i);
            let val = kv.get(&key).expect("get failed");
            if val.as_deref() == Some(expected.as_bytes()) {
                found += 1;
            }
        }
    }

    assert_eq!(
        found, expected_total,
        "Should find all {} keys, found {}",
        expected_total, found
    );
}

#[test]
fn test_concurrent_puts_same_key_last_write_wins() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    let num_threads = 8;
    let mut handles = vec![];

    for t in 0..num_threads {
        let kv_clone = kv.clone();
        let handle = thread::spawn(move || {
            let value = format!("value_from_thread_{}", t);
            kv_clone.put("shared_key", value.as_bytes()).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Should have some value (last write wins semantics)
    let final_val = kv.get("shared_key").expect("get failed");
    assert!(final_val.is_some(), "shared_key should exist");

    // Value should be from one of the threads
    let val_str = String::from_utf8(final_val.unwrap().to_vec()).expect("Invalid UTF-8");
    assert!(
        val_str.starts_with("value_from_thread_"),
        "Value should be from a thread, got: {}",
        val_str
    );
}

#[test]
fn test_concurrent_gets_no_contention() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let config = create_test_config(&temp_dir);
    let kv = Arc::new(FileKV::open(config.clone()).expect("Failed to open FileKV"));

    // Pre-populate data
    for i in 0..100 {
        kv.put(&format!("key_{}", i), format!("value_{}", i).as_bytes())
            .expect("put failed");
    }

    let num_threads = 4;
    let mut handles = vec![];

    for _t in 0..num_threads {
        let kv_clone = kv.clone();
        let handle = thread::spawn(move || {
            let mut results = vec![];
            for i in 0..100 {
                let key = format!("key_{}", i);
                let val = kv_clone.get(&key).expect("get failed");
                results.push(val);
            }
            results
        });
        handles.push(handle);
    }

    for handle in handles {
        let results = handle.join().expect("Thread panicked");
        for (i, val) in results.iter().enumerate() {
            let expected = format!("value_{}", i);
            assert_eq!(
                val.as_deref(),
                Some(expected.as_bytes()),
                "Thread got wrong value for key_{}",
                i
            );
        }
    }
}
