//! OPT-004 Performance Test: DashMap 分片优化 + MemTable 内存布局
//!
//! 测试目标：
//! - 32 线程并发写入吞吐 > 500K entries/s
//! - MemTable per-entry overhead < 50 bytes
//! - DashMap 锁等待时间 P99 < 1µs

use std::sync::Arc;
use std::thread;
use std::time::Instant;

use tokitai_filekv::core::memtable::{MemTable, MemTableConfig};

/// 测试 OPT-004: 高并发写入吞吐
#[test]
fn test_opt004_high_concurrency_write_throughput() {
    let num_threads = 32;
    let entries_per_thread = 10_000;
    let total_entries = num_threads * entries_per_thread;

    // OPT-004: 使用 num_cpus*4 分片
    let config_optimized = MemTableConfig {
        flush_threshold_bytes: 256 * 1024 * 1024, // 256MB - 不触发 flush
        max_entries: 1_000_000,
        max_memory_bytes: 512 * 1024 * 1024, // 512MB
        shards: num_cpus::get() * 4,
        enable_async_flush: false,
        max_immutable_memtables: 1,
        immutable_flush_threshold_bytes: 256 * 1024 * 1024,
    };
    let mt_optimized = Arc::new(MemTable::new(config_optimized));

    let start = Instant::now();
    let mut handles = Vec::new();

    for t in 0..num_threads {
        let mt_clone = Arc::clone(&mt_optimized);
        let handle = thread::spawn(move || {
            for i in 0..entries_per_thread {
                let key = format!("t{}_k{}", t, i);
                let value = format!("value_{}", i);
                mt_clone.insert(key, value.as_bytes());
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    let throughput = total_entries as f64 / elapsed.as_secs_f64();

    println!("OPT-004 32-thread concurrent write:");
    println!("  Total entries: {}", total_entries);
    println!("  Time: {:.3}s", elapsed.as_secs_f64());
    println!(
        "  Throughput: {:.0} entries/s ({:.1}K entries/s)",
        throughput,
        throughput / 1000.0
    );
    println!("  Shards: {}", num_cpus::get() * 4);

    // 验证吞吐 > 500K entries/s
    assert!(
        throughput > 500_000.0,
        "32-thread write throughput should be > 500K entries/s, got {:.0} entries/s",
        throughput
    );
}

/// 测试 OPT-004: 内存布局优化
#[test]
fn test_opt004_memory_layout_efficiency() {
    let config = MemTableConfig {
        flush_threshold_bytes: 256 * 1024 * 1024,
        max_entries: 1_000_000,
        max_memory_bytes: 512 * 1024 * 1024,
        shards: 64,
        enable_async_flush: false,
        max_immutable_memtables: 1,
        immutable_flush_threshold_bytes: 256 * 1024 * 1024,
    };
    let mt = MemTable::new(config);

    // Insert entries with known sizes
    let key = "test_key_12345".to_string(); // 14 bytes
    let value = vec![0u8; 100]; // 100 bytes

    let (size, _) = mt.insert(key.clone(), &value);

    // Expected: key_len (14) + value_len (100) + PER_ENTRY_OVERHEAD (48) = 162 bytes
    let expected_size = 14 + 100 + 48;

    println!("OPT-004 memory layout:");
    println!("  Key length: 14 bytes");
    println!("  Value length: 100 bytes");
    println!("  Per-entry overhead: 48 bytes");
    println!("  Expected total: {} bytes", expected_size);
    println!("  Actual total: {} bytes", size);
    println!(
        "  MemTableEntry size: {} bytes",
        std::mem::size_of::<tokitai_filekv::MemTableEntry>()
    );

    assert_eq!(size, expected_size, "Memory calculation should match expected");

    // Verify MemTableEntry struct size is reasonable
    // Note: Option<Bytes> is 24 bytes (fat pointer) + discriminant, Option<ValuePointer> is 32 bytes
    // Total struct size includes these fields plus seq_num and deleted
    let entry_struct_size = std::mem::size_of::<tokitai_filekv::MemTableEntry>();
    println!(
        "  MemTableEntry struct size: {} bytes (includes Option<Bytes> and Option<ValuePointer>)",
        entry_struct_size
    );

    // The per-entry overhead (48 bytes) tracks: MemTableEntry struct + DashMap overhead + String/Bytes headers
    // This is already optimized from the original 64 bytes
    assert!(
        entry_struct_size < 100,
        "MemTableEntry struct size should be < 100 bytes, got {} bytes",
        entry_struct_size
    );
}

/// 测试 OPT-004: 对比不同分片数的性能
#[test]
fn test_opt004_shard_comparison() {
    let num_threads = 16;
    let entries_per_thread = 5_000;

    // Test with different shard counts
    let shard_configs = vec![
        ("num_cpus*2", num_cpus::get() * 2),
        ("num_cpus*4", num_cpus::get() * 4),
        ("num_cpus*8", num_cpus::get() * 8),
    ];

    for (name, shards) in shard_configs {
        let config = MemTableConfig {
            flush_threshold_bytes: 256 * 1024 * 1024,
            max_entries: 1_000_000,
            max_memory_bytes: 512 * 1024 * 1024,
            shards,
            enable_async_flush: false,
            max_immutable_memtables: 1,
            immutable_flush_threshold_bytes: 256 * 1024 * 1024,
        };
        let mt = Arc::new(MemTable::new(config));

        let start = Instant::now();
        let mut handles = Vec::new();

        for t in 0..num_threads {
            let mt_clone = Arc::clone(&mt);
            let handle = thread::spawn(move || {
                for i in 0..entries_per_thread {
                    let key = format!("t{}_k{}", t, i);
                    mt_clone.insert(key, b"value");
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let elapsed = start.elapsed();
        let total = num_threads * entries_per_thread;
        let throughput = total as f64 / elapsed.as_secs_f64();

        println!(
            "Shard config '{}': shards={}, throughput={:.0} entries/s, time={:.3}s",
            name,
            shards,
            throughput,
            elapsed.as_secs_f64()
        );
    }
}

/// 测试 OPT-004: batch insert 性能
#[test]
fn test_opt004_batch_insert_performance() {
    let config = MemTableConfig {
        flush_threshold_bytes: 256 * 1024 * 1024,
        max_entries: 1_000_000,
        max_memory_bytes: 512 * 1024 * 1024,
        shards: num_cpus::get() * 4,
        enable_async_flush: false,
        max_immutable_memtables: 1,
        immutable_flush_threshold_bytes: 256 * 1024 * 1024,
    };
    let mt = MemTable::new(config);

    let batch_size = 1000;
    let batch: Vec<(String, Vec<u8>)> = (0..batch_size)
        .map(|i| (format!("batch_key_{}", i), vec![0u8; 64]))
        .collect();

    let start = Instant::now();

    for _ in 0..100 {
        mt.insert_batch(&batch);
    }

    let elapsed = start.elapsed();
    let total_entries = batch_size * 100;
    let throughput = total_entries as f64 / elapsed.as_secs_f64();

    println!("OPT-004 batch insert:");
    println!("  Batch size: {}", batch_size);
    println!("  Total entries: {}", total_entries);
    println!("  Time: {:.3}s", elapsed.as_secs_f64());
    println!("  Throughput: {:.0} entries/s", throughput);

    assert!(
        mt.entry_count() == batch_size,
        "Should have {} entries after batches",
        batch_size
    );
}
