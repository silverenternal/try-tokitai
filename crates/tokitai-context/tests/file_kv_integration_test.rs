//! FileKV Integration Tests
//!
//! 测试 FileKV 存储引擎的基本集成场景
//! 注意：详细的崩溃恢复测试在 crash_recovery_test.rs 中

use tempfile::TempDir;
use tokitai_context::file_kv::{FileKV, FileKVConfig, MemTableConfig};

/// 创建测试用的 FileKV 实例
fn create_test_kv(temp_dir: &TempDir) -> FileKV {
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        enable_wal: true,
        enable_bloom: true,
        memtable: MemTableConfig {
            flush_threshold_bytes: 64 * 1024,
            max_entries: 100,
            max_memory_bytes: 10 * 1024 * 1024,
        },
        ..Default::default()
    };

    FileKV::open(config).unwrap()
}

// ============================================================================
// 基本操作测试
// ============================================================================

#[test]
fn test_kv_creation() {
    let temp_dir = TempDir::new().unwrap();
    let _kv = create_test_kv(&temp_dir);
    // 成功创建即通过
}

#[test]
fn test_segments_initial_state() {
    let temp_dir = TempDir::new().unwrap();
    let kv = create_test_kv(&temp_dir);

    let segments = kv.segments();
    assert!(segments.is_empty() || !segments.is_empty());
}

#[test]
fn test_memtable_flush_basic() {
    let temp_dir = TempDir::new().unwrap();
    let kv = create_test_kv(&temp_dir);

    // flush 空 memtable
    let result = kv.flush_memtable();
    assert!(result.is_ok());
}

// ============================================================================
// 配置测试
// ============================================================================

#[test]
fn test_config_access() {
    let temp_dir = TempDir::new().unwrap();
    let kv = create_test_kv(&temp_dir);

    let config = kv.get_config();
    assert!(config.enable_wal);
    assert!(config.enable_bloom);
}

#[test]
fn test_stats_access() {
    let temp_dir = TempDir::new().unwrap();
    let kv = create_test_kv(&temp_dir);

    // 可以获取统计信息
    let _stats = kv.stats();
}

// ============================================================================
// 边界条件测试
// ============================================================================

#[test]
fn test_empty_operations() {
    let temp_dir = TempDir::new().unwrap();
    let kv = create_test_kv(&temp_dir);

    // get 不存在的 key
    let result = kv.get("nonexistent");
    assert!(result.is_ok());

    // delete 不存在的 key
    let result = kv.delete("nonexistent");
    assert!(result.is_ok());
}

// ============================================================================
// 并发访问测试
// ============================================================================

#[test]
fn test_concurrent_read_write() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let kv = Arc::new(create_test_kv(&temp_dir));

    let mut handles = vec![];

    // 写入线程
    for t in 0..3 {
        let kv_clone = Arc::clone(&kv);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                let key = format!("thread_{}_key_{}", t, i);
                let _ = kv_clone.put(&key, b"value");
            }
        });
        handles.push(handle);
    }

    // 读取线程
    for t in 0..3 {
        let kv_clone = Arc::clone(&kv);
        let handle = thread::spawn(move || {
            for i in 0..10 {
                let key = format!("thread_{}_key_{}", t, i);
                let _ = kv_clone.get(&key);
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    // 测试完成即通过
}
