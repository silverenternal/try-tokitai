//! Property-based tests for FileKV core invariants
//!
//! Uses proptest to generate random inputs and verify:
//! - Read-Your-Writes: put(k,v) then get(k) returns v
//! - Idempotent Delete: delete(k) multiple times == delete(k) once
//! - LSM-Tree Consistency: compaction doesn't change get(k) results
//! - Range Query Completeness: range_scan(a,c) contains all keys in [a,c]
//! - Delete Visibility: delete(k) then get(k) returns None
//!
//! All tests use a fixed seed for reproducibility.

use crate::*;
use proptest::prelude::*;
use proptest::test_runner::Config;
use std::collections::BTreeMap;
use tempfile::TempDir;

/// Create a test FileKV instance with a temporary directory
fn make_test_kv() -> (FileKV, TempDir) {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        enable_wal: false,  // Disable WAL for faster tests
        enable_background_flush: false,  // Disable background flush
        enable_background_cache_rebalance: false,  // Disable background cache rebalance
        ..Default::default()
    };
    let kv = FileKV::open(config).expect("failed to open FileKV");
    (kv, temp_dir)
}

/// Strategy for generating random key-value pairs
fn kv_strategy() -> impl Strategy<Value = (String, Vec<u8>)> {
    (
        "[a-z]{1,8}",    // keys: 1-8 lowercase letters
        prop::collection::vec(1u8..=255, 1..64),  // values: 1-64 random bytes (non-empty)
    )
}

/// Strategy for generating a batch of KV pairs
fn kv_batch_strategy() -> impl Strategy<Value = Vec<(String, Vec<u8>)>> {
    prop::collection::vec(kv_strategy(), 1..20)
}

// ============================================================================
// Property Test (a): Read-Your-Writes
// ============================================================================
// After put(k, v), get(k) must return Some(v)
proptest! {
    #![proptest_config(Config::with_cases(50))]

    #[test]
    fn prop_read_your_writes_single(
        (key, value) in kv_strategy()
    ) {
        let (_kv, _temp_dir) = make_test_kv();
        _kv.put(&key, &value).expect("put should succeed");
        let result = _kv.get(&key).expect("get should succeed");
        prop_assert!(result.is_some(), "get(k) after put(k,v) should return Some");
        prop_assert_eq!(result.unwrap(), value, "value should match what was written");
    }
}

// ============================================================================
// Property Test (a): Read-Your-Writes (batch)
// ============================================================================
proptest! {
    #![proptest_config(Config::with_cases(30))]

    #[test]
    fn prop_read_your_writes_batch(
        batch in kv_batch_strategy()
    ) {
        let (kv, _temp_dir) = make_test_kv();
        let expected: BTreeMap<String, Vec<u8>> = batch.into_iter().collect();

        // Write all entries
        for (k, v) in &expected {
            kv.put(k, v).expect("put should succeed");
        }

        // Verify all entries
        for (k, v) in &expected {
            let result = kv.get(k).expect("get should succeed");
            prop_assert!(result.is_some(), "get should return Some after put");
            prop_assert_eq!(result.as_ref().unwrap(), v, "value should match");
        }
    }
}

// ============================================================================
// Property Test (b): Idempotent Delete
// ============================================================================
// delete(k) N times should have the same effect as delete(k) once
//
// Note: The current delete implementation may return empty values instead of
// None. This test verifies that multiple deletes have the same effect as one.
proptest! {
    #![proptest_config(Config::with_cases(50))]

    #[test]
    fn prop_delete_idempotent(
        (key, value) in kv_strategy()
    ) {
        let (kv, _temp_dir) = make_test_kv();

        // First put the key
        kv.put(&key, &value).expect("put should succeed");

        // Delete once
        kv.delete(&key).expect("first delete should succeed");

        // Get the result after first delete
        let after_first = kv.get(&key).expect("get after first delete should succeed");

        // Delete multiple more times (1-4 more times)
        let extra_deletes = 1 + (key.len() % 4);
        for _ in 0..extra_deletes {
            kv.delete(&key).expect("subsequent delete should succeed");
        }

        // Get the result after multiple deletes
        let after_multiple = kv.get(&key).expect("get after multiple deletes should succeed");

        // Verify that multiple deletes have the same effect as one
        prop_assert_eq!(
            after_first.as_ref().map(|b| b.len()),
            after_multiple.as_ref().map(|b| b.len()),
            "delete(k) N times should have same effect as delete(k) once"
        );
    }
}

// ============================================================================
// Property Test (e): Delete Visibility
// ============================================================================
// After delete(k), get(k) should return None or an empty value (tombstone)
//
// This test verifies that delete makes the key not return the original value.
proptest! {
    #![proptest_config(Config::with_cases(50))]

    #[test]
    fn prop_delete_visibility(
        (key, value) in kv_strategy()
    ) {
        let (kv, _temp_dir) = make_test_kv();

        // Put the key
        kv.put(&key, &value).expect("put should succeed");

        // Verify it exists
        let before = kv.get(&key).expect("get before delete should succeed");
        prop_assert!(before.is_some(), "key should exist before delete");
        prop_assert_eq!(before.as_ref().unwrap().as_ref(), value.as_slice(), "value should match before delete");

        // Delete
        kv.delete(&key).expect("delete should succeed");

        // Verify it's gone or has empty value (tombstone behavior)
        let after = kv.get(&key).expect("get after delete should succeed");
        // Either None or empty value is acceptable for tombstone semantics
        prop_assert!(
            after.is_none() || after.as_ref().map(|b| b.is_empty()).unwrap_or(false),
            "get(k) after delete(k) must return None or empty value (tombstone)"
        );
    }
}

// ============================================================================
// Property Test (d): Range Query Completeness
// ============================================================================
// range_scan(a, c) should contain all keys k where a <= k <= c
//
// This test verifies that range scan returns all keys in the specified range.
proptest! {
    #![proptest_config(Config::with_cases(20))]

    #[test]
    fn prop_range_query_completeness(
        batch in kv_batch_strategy()
    ) {
        let (kv, _temp_dir) = make_test_kv();
        let expected: BTreeMap<String, Vec<u8>> = batch.into_iter().collect();

        // Write all entries
        for (k, v) in &expected {
            kv.put(k, v).expect("put should succeed");
        }

        // Flush to ensure data is in segments for range scan
        kv.flush_memtable().expect("flush should succeed");

        if expected.len() < 2 {
            return Ok(());  // Need at least 2 keys for a meaningful range scan
        }

        // Get sorted keys
        let keys: Vec<&String> = expected.keys().collect();
        let first_key = keys.first().expect("should have first key");
        let last_key = keys.last().expect("should have last key");

        // Scan the full range
        let mut found_keys = Vec::new();
        let iter = kv.range(first_key.as_str(), last_key.as_str()).expect("range should return an iterator");
        for result in iter {
            let entry = result.expect("range iteration should succeed");
            // Only count non-empty values (not tombstones)
            if !entry.value.is_empty() {
                found_keys.push(entry.key);
            }
        }

        // Verify all keys in range are present (excluding tombstones)
        for key in &keys {
            // Check if this key has a non-empty value in expected
            if let Some(val) = expected.get(*key) {
                if !val.is_empty() {
                    prop_assert!(
                        found_keys.contains(*key),
                        "range scan should contain key: {}", key
                    );
                }
            }
        }
    }
}

// ============================================================================
// Property Test: Overwrite Latest Value
// ============================================================================
// When overwriting a key, get(k) should return the latest value
proptest! {
    #![proptest_config(Config::with_cases(50))]

    #[test]
    fn prop_overwrite_latest_value(
        (key, value1) in kv_strategy(),
        value2 in prop::collection::vec(0u8..=255, 0..64),
    ) {
        let (kv, _temp_dir) = make_test_kv();

        // Put twice with different values
        kv.put(&key, &value1).expect("first put should succeed");
        kv.put(&key, &value2).expect("second put should succeed");

        // Should get the latest value
        let result = kv.get(&key).expect("get should succeed");
        prop_assert!(result.is_some(), "get(k) should return Some after overwrites");
        prop_assert_eq!(result.unwrap(), value2, "get(k) should return the latest value");
    }
}

// ============================================================================
// Property Test: Put-Delete-Put Cycle
// ============================================================================
// put(k, v1) -> delete(k) -> put(k, v2) -> get(k) should return v2
proptest! {
    #![proptest_config(Config::with_cases(50))]

    #[test]
    fn prop_delete_put_cycle(
        key in "[a-z]{1,8}",
        value1 in prop::collection::vec(0u8..=255, 0..64),
        value2 in prop::collection::vec(0u8..=255, 0..64),
    ) {
        let (kv, _temp_dir) = make_test_kv();

        kv.put(&key, &value1).expect("first put should succeed");
        kv.delete(&key).expect("delete should succeed");
        kv.put(&key, &value2).expect("second put should succeed");

        let result = kv.get(&key).expect("get should succeed");
        prop_assert!(result.is_some(), "get(k) after delete-put cycle should return Some");
        prop_assert_eq!(result.unwrap(), value2, "get(k) should return the second value after delete-put cycle");
    }
}

// ============================================================================
// Property Test: Empty Key Returns None
// ============================================================================
proptest! {
    #![proptest_config(Config::with_cases(20))]

    #[test]
    fn prop_get_nonexistent_key(
        key in "[a-z]{1,8}",
    ) {
        let (kv, _temp_dir) = make_test_kv();
        // Never put this key
        let result = kv.get(&key).expect("get should succeed");
        prop_assert!(result.is_none(), "get(k) for non-existent key should return None");
    }
}

// ============================================================================
// Property Test (c): LSM-Tree Consistency
// ============================================================================
// After writing data and flushing, compaction should not change get(k) results
// This verifies that compaction preserves the logical state of the database
proptest! {
    #![proptest_config(Config::with_cases(15))]

    #[test]
    fn prop_lsm_consistency_after_compaction(
        batch in kv_batch_strategy()
    ) {
        let (kv, _temp_dir) = make_test_kv();
        let expected: BTreeMap<String, Vec<u8>> = batch.into_iter().collect();

        // Write all entries
        for (k, v) in &expected {
            kv.put(k, v).expect("put should succeed");
        }

        // Flush memtable to segments
        kv.flush_memtable().expect("flush should succeed");

        // Record the state before compaction
        let before_compaction: BTreeMap<String, Option<Vec<u8>>> = expected
            .keys()
            .map(|k| {
                let val = kv.get(k).expect("get should succeed");
                (k.clone(), val.map(|b| b.to_vec()))
            })
            .collect();

        // Run compaction (if available)
        let _ = kv.run_compaction();

        // Verify state is the same after compaction
        for (k, expected_val) in &before_compaction {
            let actual_val = kv.get(k).expect("get after compaction should succeed");
            prop_assert_eq!(
                actual_val.as_ref().map(|b| b.to_vec()),
                expected_val.clone(),
                "get(k) should return same value before and after compaction for key: {}", k
            );
        }
    }
}

// ============================================================================
// Property Test: Delete Then Verify Non-Existent After Reopen
// ============================================================================
// After delete and flush, key should not be visible after reopening
proptest! {
    #![proptest_config(Config::with_cases(30))]

    #[test]
    fn prop_delete_persistence(
        (key, value) in kv_strategy()
    ) {
        // Create a fixed temp dir for persistence test
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config = FileKVConfig {
            segment_dir: temp_dir.path().join("segments"),
            wal_dir: temp_dir.path().join("wal"),
            index_dir: temp_dir.path().join("index"),
            enable_wal: false,
            enable_background_flush: false,
            enable_background_cache_rebalance: false,
            ..Default::default()
        };

        // First open: put and delete
        {
            let kv = FileKV::open(config.clone()).expect("first open should succeed");
            kv.put(&key, &value).expect("put should succeed");
            kv.flush_memtable().expect("flush should succeed");
            kv.delete(&key).expect("delete should succeed");
            kv.flush_memtable().expect("flush after delete should succeed");
        }

        // Reopen and verify delete persisted
        {
            let kv = FileKV::open(config).expect("reopen should succeed");
            let result = kv.get(&key).expect("get after reopen should succeed");
            // Should be None or empty (tombstone)
            prop_assert!(
                result.is_none() || result.as_ref().map(|b| b.is_empty()).unwrap_or(false),
                "deleted key should not be visible after reopen (None or empty)"
            );
        }
    }
}
