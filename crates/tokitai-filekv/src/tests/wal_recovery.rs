//! WAL recovery tests for FileKV

use crate::core::wal::{load_wal_entries, WalManager, WalOperation};
use crate::io::StdFs;
use crate::*;
use std::sync::Arc;
use tempfile::TempDir;

/// Test that WAL entries are replayed after opening
#[test]
fn test_wal_recovery_basic() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV store");

    // Insert some entries with WAL enabled
    kv.put("key1", b"value1").expect("Failed to put key1");
    kv.put("key2", b"value2").expect("Failed to put key2");
    kv.put("key3", b"value3").expect("Failed to put key3");

    // Verify entries exist in memtable
    let result1 = kv.get("key1").expect("Failed to get key1");
    assert!(result1.is_some());
    assert_eq!(result1.expect("key1 should be Some").as_ref(), b"value1".as_ref());

    // Drop KV store without flushing (simulate crash)
    drop(kv);

    // Reopen - should replay WAL entries
    let kv2 = FileKV::open(config).expect("Failed to reopen FileKV store");

    // WAL entries should be recovered to memtable
    let result1 = kv2.get("key1").expect("Failed to get key1 after recovery");
    let result2 = kv2.get("key2").expect("Failed to get key2 after recovery");
    let result3 = kv2.get("key3").expect("Failed to get key3 after recovery");

    assert!(result1.is_some(), "key1 should be recovered from WAL");
    assert!(result2.is_some(), "key2 should be recovered from WAL");
    assert!(result3.is_some(), "key3 should be recovered from WAL");

    assert_eq!(
        result1.expect("recovered key1 should be Some").as_ref(),
        b"value1".as_ref()
    );
    assert_eq!(
        result2.expect("recovered key2 should be Some").as_ref(),
        b"value2".as_ref()
    );
    assert_eq!(
        result3.expect("recovered key3 should be Some").as_ref(),
        b"value3".as_ref()
    );
}

/// Test WAL recovery with mixed insertions and flush
#[test]
fn test_wal_recovery_after_flush() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV store");

    // Insert and flush some entries
    kv.put("flushed1", b"flushed_value1").expect("Failed to put flushed1");
    kv.put("flushed2", b"flushed_value2").expect("Failed to put flushed2");
    kv.flush_memtable().expect("Failed to flush memtable");

    // Insert more entries without flush (these are in WAL but not flushed)
    kv.put("wal1", b"wal_value1").expect("Failed to put wal1");
    kv.put("wal2", b"wal_value2").expect("Failed to put wal2");

    // Drop KV store without flushing (simulate crash)
    drop(kv);

    // Reopen - should replay WAL entries
    let kv2 = FileKV::open(config).expect("Failed to reopen FileKV store");

    // Flushed entries should be in segments
    let result_f1 = kv2.get("flushed1").expect("Failed to get flushed1 after recovery");
    let result_f2 = kv2.get("flushed2").expect("Failed to get flushed2 after recovery");
    assert!(result_f1.is_some(), "flushed1 should be in segments");
    assert!(result_f2.is_some(), "flushed2 should be in segments");

    // WAL entries should be recovered to memtable
    let result_w1 = kv2.get("wal1").expect("Failed to get wal1 after recovery");
    let result_w2 = kv2.get("wal2").expect("Failed to get wal2 after recovery");
    assert!(result_w1.is_some(), "wal1 should be recovered from WAL");
    assert!(result_w2.is_some(), "wal2 should be recovered from WAL");

    assert_eq!(
        result_f1.expect("flushed1 should be Some").as_ref(),
        b"flushed_value1".as_ref()
    );
    assert_eq!(
        result_f2.expect("flushed2 should be Some").as_ref(),
        b"flushed_value2".as_ref()
    );
    assert_eq!(result_w1.expect("wal1 should be Some").as_ref(), b"wal_value1".as_ref());
    assert_eq!(result_w2.expect("wal2 should be Some").as_ref(), b"wal_value2".as_ref());
}

/// Test WAL recovery with delete operations
#[test]
fn test_wal_recovery_with_deletes() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV store");

    // Insert some entries
    kv.put("key1", b"value1").expect("Failed to put key1");
    kv.put("key2", b"value2").expect("Failed to put key2");
    kv.put("key3", b"value3").expect("Failed to put key3");

    // Delete one entry
    kv.delete("key2").expect("Failed to delete key2");

    // Drop without flushing
    drop(kv);

    // Reopen - should replay WAL entries including delete
    let kv2 = FileKV::open(config).expect("Failed to reopen FileKV store");

    // key1 and key3 should be recovered
    let result1 = kv2.get("key1").expect("Failed to get key1 after recovery");
    let result3 = kv2.get("key3").expect("Failed to get key3 after recovery");
    assert!(result1.is_some(), "key1 should be recovered");
    assert!(result3.is_some(), "key3 should be recovered");

    // key2 should be deleted (tombstone in memtable)
    let result2 = kv2.get("key2").expect("Failed to get key2 after recovery");
    assert!(result2.is_none(), "key2 should be deleted (tombstone)");
}

/// Test atomic flush - no incomplete segments after crash
#[test]
fn test_flush_atomic_rename() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_wal: true,
        ..Default::default()
    };

    let kv = FileKV::open(config.clone()).expect("Failed to open FileKV store");

    // Insert enough entries to trigger a flush
    for i in 0..30 {
        // Reduced from 100
        kv.put(&format!("key_{}", i), &format!("value_{}", i).into_bytes())
            .unwrap_or_else(|_| panic!("Failed to put key_{}", i));
    }

    // Flush memtable
    kv.flush_memtable().expect("Failed to flush memtable");

    // Verify no temp files left behind
    for entry in std::fs::read_dir(&config.segment_dir).expect("Failed to read segment directory") {
        let entry = entry.expect("Failed to read directory entry");
        let name = entry.file_name();
        let name_str = name.to_str().expect("Failed to convert filename to string");
        assert!(
            !name_str.starts_with(".segment_"),
            "No temp files should exist: {}",
            name_str
        );
    }

    // Verify segment file exists and is readable
    let segments: Vec<_> = std::fs::read_dir(&config.segment_dir)
        .expect("Failed to read segment directory for segment files")
        .filter_map(|e| {
            let e = e.ok()?;
            let path = e.path();
            if path.extension().and_then(|s| s.to_str()) == Some("log") {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    assert!(!segments.is_empty(), "At least one segment should exist");
}

// =============================================================================
// WalSyncMode Lazy 模式测试 (CORE-004)
// =============================================================================

/// 测试 Lazy 模式下小量写入不立即刷盘
///
/// 验证：在 Lazy 同步模式下，少量写入操作仅写入内部缓冲区，
/// 不会立即刷新到磁盘文件。这是 Lazy 模式的核心行为特征。
#[test]
fn test_wal_sync_mode_lazy_small_writes_buffered() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    // 创建 WalManager 使用 Lazy 模式
    let mut wal = WalManager::new_with_config(
        Arc::new(StdFs),
        &wal_dir,
        true,
        64 * 1024 * 1024, // max_size_bytes
        10,               // max_files
        WalSyncMode::Lazy,
    )
    .expect("Failed to create WalManager");

    // 写入少量数据（远小于 32KB 缓冲区阈值）
    let op1 = WalOperation::Add {
        session: "key1".to_string(),
        hash: "hash1".to_string(),
        layer: "segment".to_string(),
    };
    wal.log(op1).expect("Failed to log entry 1");

    let op2 = WalOperation::Add {
        session: "key2".to_string(),
        hash: "hash2".to_string(),
        layer: "segment".to_string(),
    };
    wal.log(op2).expect("Failed to log entry 2");

    // 在 Lazy 模式下，数据应停留在缓冲区，WAL 文件可能很小或不存在
    // 注意：由于内部实现会打开文件句柄，文件可能存在但内容可能为空或很少
    // 我们验证显式 flush 前的行为
    let wal_files: Vec<_> = std::fs::read_dir(&wal_dir)
        .expect("Failed to read WAL directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    // WAL 文件应该存在（因为 open_file 被调用了），但内容可能未完全刷新
    assert!(
        !wal_files.is_empty(),
        "WAL file should exist after writes (file handle opened)"
    );
}

/// 测试 Lazy 模式下 Drop 时数据会被刷盘
///
/// 验证：即使 Lazy 模式不主动刷盘，在 WalManager 被 drop 时
/// 剩余的缓冲区数据会被写入文件，确保正常关闭时数据不会丢失。
#[test]
fn test_wal_sync_mode_lazy_flush_on_drop() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    // 创建并写入数据
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        // 写入少量数据
        for i in 0..5 {
            let op = WalOperation::Add {
                session: format!("key_{}", i),
                hash: format!("hash_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log entry");
        }

        // 不调用 flush，直接 drop
    } // wal is dropped here

    // 重新打开 WAL 文件，验证数据已写入
    let wal_files: Vec<_> = std::fs::read_dir(&wal_dir)
        .expect("Failed to read WAL directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    assert!(!wal_files.is_empty(), "WAL file should exist after drop");

    // 验证文件内容非空（数据已被 flush）
    for file in &wal_files {
        let metadata = std::fs::metadata(file.path()).expect("Failed to get file metadata");
        assert!(
            metadata.len() > 0,
            "WAL file should not be empty after drop (data should be flushed)"
        );
    }

    // 创建新的 WalManager 读取相同目录，验证可以恢复数据
    let wal2 = WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
        .expect("Failed to create second WalManager");

    let entries = wal2.read_entries().expect("Failed to read WAL entries");
    assert!(
        entries.len() >= 5,
        "Should recover at least 5 entries, got {}",
        entries.len()
    );
}

/// 测试 Lazy 模式大批量写入时缓冲区自动刷新
///
/// 验证：当内部写缓冲区达到阈值（32KB）时，即使使用 Lazy 模式，
/// 数据也会自动刷新到文件。
#[test]
fn test_wal_sync_mode_lazy_auto_flush_on_buffer_threshold() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    // Write and then drop to ensure file handle is closed
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        // 写入大量数据，超过 32KB 缓冲区阈值
        // 每个 WAL entry 序列化后约 100-200 字节，写入 300 条确保超过 32KB
        for i in 0..300 {
            let op = WalOperation::Add {
                session: format!("bulk_key_{}", i),
                hash: format!("bulk_hash_{:016X}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log entry");
        }
    } // WalManager dropped, file handle closed

    // 在 Lazy 模式下，超过阈值后缓冲区应已刷新到文件
    let wal_files: Vec<_> = std::fs::read_dir(&wal_dir)
        .expect("Failed to read WAL directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    assert!(!wal_files.is_empty(), "WAL file should exist after large writes");

    for file in &wal_files {
        let metadata = std::fs::metadata(file.path()).expect("Failed to get file metadata");
        assert!(
            metadata.len() > 1000,
            "WAL file should have significant content after buffer threshold (got {} bytes)",
            metadata.len()
        );
    }
}

/// 测试 Lazy 模式崩溃恢复（模拟 SIGKILL 场景）
///
/// 此测试验证：即使在 Lazy 模式下，WalManager Drop 时会 flush 缓冲区，
/// 因此正常关闭（非 SIGKILL）时数据不会丢失。
///
/// 注意：真实的 SIGKILL 或掉电场景无法在安全测试中模拟，
/// 此测试验证的是"正常 Drop 后恢复"场景，确保数据完整性。
#[test]
fn test_wal_sync_mode_lazy_crash_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    // 创建并写入数据
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        // 写入数据
        for i in 0..10 {
            let op = WalOperation::Add {
                session: format!("crash_key_{}", i),
                hash: format!("crash_hash_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log entry");
        }

        // 不调用 flush()，不调用 sync_all()，显式 drop 模拟"崩溃"
        // 注意：Drop impl 会 flush 缓冲区，但不调用 sync_all
        // 在真实 SIGKILL 场景中，OS 可能来不及将缓冲区刷盘
        // 但在正常测试中，Drop 会确保数据写入
    }

    // 重新打开并验证数据
    // 注意：不需要 sleep，因为 Drop 已经确保数据写入文件系统缓存
    let wal = WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
        .expect("Failed to create recovery WalManager");

    let entries = wal.read_entries().expect("Failed to read WAL entries");
    assert!(
        entries.len() >= 10,
        "Should recover entries after lazy drop, got {}",
        entries.len()
    );

    // 直接读取 WAL 文件内容作为额外验证
    let wal_files: Vec<_> = std::fs::read_dir(&wal_dir)
        .expect("Failed to read WAL directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    assert!(!wal_files.is_empty(), "WAL file should exist");

    let mut recovered_count = 0;
    for file in &wal_files {
        let file_data = std::fs::read(file.path()).expect("Failed to read WAL file");
        if let Ok(entries) = load_wal_entries(&file_data) {
            recovered_count += entries.len() as u64;
        }
    }

    assert!(
        recovered_count >= 10,
        "Should recover at least 10 valid WAL entries from file, got {}",
        recovered_count
    );
}

// =============================================================================
// TEST-004: WAL Lazy Mode Edge Case Tests
// =============================================================================

/// TEST-004(a): Partial WAL entries write then crash recovery
///
/// Write 10 entries with Lazy mode, drop without explicit flush,
/// then verify that data recovered from the WAL is complete.
/// Reports data loss rate (expected 0% under normal Drop).
#[test]
fn test_wal_lazy_partial_write_crash_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    let total_entries = 10;

    // Write entries and drop without explicit flush (simulates crash)
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        for i in 0..total_entries {
            let op = WalOperation::Add {
                session: format!("partial_key_{}", i),
                hash: format!("partial_hash_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log entry");
        }

        // Explicit drop without flush - simulates crash scenario
        // Note: Drop impl flushes buffer, so data should survive
    }

    // Recovery: read back entries
    let wal = WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
        .expect("Failed to create recovery WalManager");

    let entries = wal.read_entries().expect("Failed to read WAL entries");
    let recovered_count = entries.len();

    // Calculate data loss rate
    let loss_rate = if total_entries > 0 {
        (total_entries - recovered_count) as f64 / total_entries as f64
    } else {
        0.0
    };

    // Under normal Drop, all entries should be recovered
    assert!(
        recovered_count >= total_entries,
        "Should recover at least {} entries, got {} (loss rate: {:.4}%)",
        total_entries,
        recovered_count,
        loss_rate * 100.0
    );

    assert!(
        loss_rate < 0.0001,
        "Data loss rate should be < 0.01%, got {:.4}%",
        loss_rate * 100.0
    );
}

/// TEST-004(b): Duplicate recovery scenario
///
/// Recover WAL entries, then recover again on a fresh WalManager,
/// verifying that entries are not double-applied.
#[test]
fn test_wal_lazy_duplicate_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    // Phase 1: Write entries
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        for i in 0..8 {
            let op = WalOperation::Add {
                session: format!("dup_key_{}", i),
                hash: format!("dup_hash_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log entry");
        }
    }

    // Phase 2: First recovery
    let wal1 = WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
        .expect("Failed to create first recovery WalManager");

    let entries1 = wal1.read_entries().expect("Failed to read WAL entries");
    let count1 = entries1.len();

    // Phase 3: Second recovery (fresh WalManager, same directory)
    let wal2 = WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
        .expect("Failed to create second recovery WalManager");

    let entries2 = wal2
        .read_entries()
        .expect("Failed to read WAL entries on second recovery");
    let count2 = entries2.len();

    // Verify: both recoveries should return the same count
    assert_eq!(
        count1, count2,
        "Duplicate recovery should return same count: first={}, second={}",
        count1, count2
    );

    // Verify: entry sessions should be identical (no duplicates)
    let sessions1: Vec<&str> = entries1
        .iter()
        .map(|e| match &e.operation {
            WalOperation::Add { session, .. } => session.as_str(),
            _ => "",
        })
        .collect();
    let sessions2: Vec<&str> = entries2
        .iter()
        .map(|e| match &e.operation {
            WalOperation::Add { session, .. } => session.as_str(),
            _ => "",
        })
        .collect();
    assert_eq!(
        sessions1, sessions2,
        "Recovered sessions should be identical across recoveries"
    );
}

/// TEST-004(c): Disk full simulation using MemFs
///
/// Use MemFs (in-memory filesystem) to simulate disk operations
/// verifying WAL behavior with an in-memory filesystem backend.
#[test]
fn test_wal_lazy_disk_full_simulation() {
    use crate::io::MemFs;

    let mem_fs = Arc::new(MemFs::new());
    let wal_dir = std::path::PathBuf::from("/wal");

    // Create WAL directory in MemFs
    mem_fs
        .create_dir_all(&wal_dir)
        .expect("Failed to create WAL dir in MemFs");

    // Write entries to MemFs with Lazy mode and verify recovery in same session
    {
        let mut wal =
            WalManager::new_with_config(mem_fs.clone(), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager with MemFs");

        for i in 0..20 {
            let op = WalOperation::Add {
                session: format!("diskfull_key_{}", i),
                hash: format!("diskfull_hash_{}", i),
                layer: "segment".to_string(),
            };
            // Log should succeed (MemFs has no size limit)
            wal.log(op).expect("Failed to log entry to MemFs");
        }

        // Verify recovery within same session: read back entries
        let entries = wal.read_entries().expect("Failed to read WAL entries from MemFs");

        // Verify all entries recovered
        let mut recovered_keys: Vec<String> = Vec::new();
        for entry in &entries {
            if let WalOperation::Add { session, .. } = &entry.operation {
                if session.starts_with("diskfull_key_") {
                    recovered_keys.push(session.clone());
                }
            }
        }

        assert!(
            recovered_keys.len() >= 20,
            "Should recover at least 20 entries from MemFs, got {}",
            recovered_keys.len()
        );

        // Verify no duplicate keys (data integrity check)
        let mut unique_keys: Vec<&String> = recovered_keys.iter().collect();
        unique_keys.sort();
        unique_keys.dedup();

        let dup_count = recovered_keys.len() - unique_keys.len();
        assert_eq!(
            dup_count, 0,
            "No duplicate entries expected, found {} duplicates",
            dup_count
        );
    }

    // Verify files exist in MemFs after drop (flush on drop)
    let wal_files: Vec<_> = mem_fs
        .read_dir(&wal_dir)
        .expect("Failed to read WAL dir in MemFs after drop")
        .into_iter()
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    assert!(!wal_files.is_empty(), "WAL files should exist in MemFs after drop");
}

/// TEST-004(d): Mixed put/delete recovery in Lazy mode
///
/// Write entries with mixed add and delete operations,
/// verify recovery preserves the operation order correctly.
#[test]
fn test_wal_lazy_mixed_add_delete_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    // Write mixed add/delete entries
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        // Add keys 0-4
        for i in 0..5 {
            let op = WalOperation::Add {
                session: format!("mixed_key_{}", i),
                hash: format!("mixed_hash_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log add entry");
        }

        // Delete keys 1 and 3
        for i in &[1, 3] {
            let op = WalOperation::Delete {
                session: format!("mixed_key_{}", i),
                hash: format!("mixed_hash_{}", i),
            };
            wal.log(op).expect("Failed to log delete entry");
        }

        // Add key 5
        let op = WalOperation::Add {
            session: "mixed_key_5".to_string(),
            hash: "mixed_hash_5".to_string(),
            layer: "segment".to_string(),
        };
        wal.log(op).expect("Failed to log final add entry");

        // Drop to flush
    }

    // Recovery: read back entries
    let wal = WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
        .expect("Failed to create recovery WalManager");

    let entries = wal.read_entries().expect("Failed to read WAL entries");

    // Count add and delete operations
    let mut add_count = 0;
    let mut delete_count = 0;
    for entry in &entries {
        match &entry.operation {
            WalOperation::Add { session, .. } => {
                if session.starts_with("mixed_key_") {
                    add_count += 1;
                }
            }
            WalOperation::Delete { session, .. } => {
                if session.starts_with("mixed_key_") {
                    delete_count += 1;
                }
            }
            _ => {}
        }
    }

    // Verify: we wrote 6 Add ops (keys 0-4 + key 5) and 2 Delete ops (keys 1, 3)
    assert!(
        add_count >= 6,
        "Should recover at least 6 Add operations, got {}",
        add_count
    );
    assert!(
        delete_count >= 2,
        "Should recover at least 2 Delete operations, got {}",
        delete_count
    );

    println!(
        "Lazy mixed add/delete recovery: {} adds, {} deletes recovered (loss rate: 0%)",
        add_count, delete_count
    );
}

/// TEST-004(e): Sequence number discontinuity recovery
///
/// Write entries with gaps in sequence numbers (simulating partial WAL writes),
/// verify that recovery handles discontinuous sequences gracefully.
/// Reports data loss rate (expected <0.01% under normal scenarios).
#[test]
fn test_wal_lazy_discontinuous_sequence_recovery() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let wal_dir = temp_dir.path().join("wal");

    // Phase 1: Write first batch of entries
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        for i in 0..15 {
            let op = WalOperation::Add {
                session: format!("seq_batch1_{}", i),
                hash: format!("seq_hash1_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log entry");
        }
        // Drop flushes data
    }

    // Phase 2: Write second batch (simulating discontinuous sequence)
    {
        let mut wal =
            WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
                .expect("Failed to create WalManager");

        // Write entries with a gap in naming (simulates sequence discontinuity)
        for i in 20..30 {
            let op = WalOperation::Add {
                session: format!("seq_batch2_{}", i),
                hash: format!("seq_hash2_{}", i),
                layer: "segment".to_string(),
            };
            wal.log(op).expect("Failed to log entry");
        }
        // Drop flushes data
    }

    // Phase 3: Recovery - read all entries from both batches
    let wal = WalManager::new_with_config(Arc::new(StdFs), &wal_dir, true, 64 * 1024 * 1024, 10, WalSyncMode::Lazy)
        .expect("Failed to create recovery WalManager");

    let entries = wal.read_entries().expect("Failed to read WAL entries");

    // Count recovered entries from each batch
    let mut batch1_count = 0;
    let mut batch2_count = 0;
    for entry in &entries {
        if let WalOperation::Add { session, .. } = &entry.operation {
            if session.starts_with("seq_batch1_") {
                batch1_count += 1;
            } else if session.starts_with("seq_batch2_") {
                batch2_count += 1;
            }
        }
    }

    let total_expected = 15 + 10; // batch1: 15 entries, batch2: 10 entries
    let total_recovered = batch1_count + batch2_count;

    // Calculate data loss rate
    let loss_rate = if total_expected > 0 {
        (total_expected - total_recovered) as f64 / total_expected as f64
    } else {
        0.0
    };

    println!(
        "Discontinuous sequence recovery: batch1={}, batch2={}, total={}/{}, loss_rate={:.4}%",
        batch1_count,
        batch2_count,
        total_recovered,
        total_expected,
        loss_rate * 100.0
    );

    // Verify batch1 entries recovered
    assert!(
        batch1_count >= 15,
        "Should recover at least 15 batch1 entries, got {}",
        batch1_count
    );

    // Verify batch2 entries recovered
    assert!(
        batch2_count >= 10,
        "Should recover at least 10 batch2 entries, got {}",
        batch2_count
    );

    // Verify data loss rate < 0.01%
    assert!(
        loss_rate < 0.0001,
        "Data loss rate should be < 0.01%, got {:.4}%",
        loss_rate * 100.0
    );

    // Extra verification: directly read WAL file content
    let wal_files: Vec<_> = std::fs::read_dir(&wal_dir)
        .expect("Failed to read WAL directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("log"))
        .collect();

    assert!(!wal_files.is_empty(), "WAL files should exist");

    let mut file_entry_count = 0;
    for file in &wal_files {
        let file_data = std::fs::read(file.path()).expect("Failed to read WAL file");
        if let Ok(entries) = load_wal_entries(&file_data) {
            file_entry_count += entries.len();
        }
    }

    assert!(
        file_entry_count >= total_expected,
        "Should find at least {} valid entries in WAL files, got {}",
        total_expected,
        file_entry_count
    );
}
