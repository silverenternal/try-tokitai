//! Tests for FileKV

use super::*;
use tempfile::TempDir;

#[test]
fn test_filekv_open() {
    let temp_dir = TempDir::new().unwrap();
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();
    let stats = kv.stats();
    assert_eq!(stats.segment_count, 0);
}

#[test]
fn test_filekv_put_get() {
    let temp_dir = TempDir::new().unwrap();
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        enable_wal: false,
        write_coalescing_enabled: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    kv.put("key1", b"value1").unwrap();
    kv.put("key2", b"value2").unwrap();

    let val1 = kv.get("key1").unwrap();
    assert_eq!(val1, Some(b"value1".to_vec()));

    let val2 = kv.get("key2").unwrap();
    assert_eq!(val2, Some(b"value2".to_vec()));

    let val3 = kv.get("key3").unwrap();
    assert_eq!(val3, None);
}

#[test]
fn test_filekv_delete() {
    let temp_dir = TempDir::new().unwrap();
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        enable_wal: false,
        write_coalescing_enabled: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    kv.put("key1", b"value1").unwrap();
    kv.delete("key1").unwrap();

    let val = kv.get("key1").unwrap();
    assert_eq!(val, None);
}

#[test]
fn test_filekv_stats() {
    let _temp_dir = TempDir::new().unwrap();
    let config = FileKVConfig {
        write_coalescing_enabled: false,
        ..Default::default()
    };
    let kv = FileKV::open(config).unwrap();

    let stats = kv.stats();
    assert_eq!(stats.write_count, 0);
    assert_eq!(stats.read_count, 0);

    kv.put("key1", b"value1").unwrap();
    kv.put("key2", b"value2").unwrap();

    let stats = kv.stats();
    assert_eq!(stats.write_count, 2);
    assert!(stats.memtable_size > 0);
    assert_eq!(stats.memtable_entries, 2);
}

#[test]
fn test_filekv_put_batch() {
    let temp_dir = TempDir::new().unwrap();
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        enable_wal: false,
        ..Default::default()
    };

    let kv = FileKV::open(config).unwrap();

    let entries: Vec<(&str, &[u8])> = vec![
        ("key1", b"value1"),
        ("key2", b"value2"),
        ("key3", b"value3"),
    ];

    let count = kv.put_batch(&entries).unwrap();
    assert_eq!(count, 3);

    assert_eq!(kv.get("key1").unwrap(), Some(b"value1".to_vec()));
    assert_eq!(kv.get("key2").unwrap(), Some(b"value2".to_vec()));
    assert_eq!(kv.get("key3").unwrap(), Some(b"value3".to_vec()));
}
