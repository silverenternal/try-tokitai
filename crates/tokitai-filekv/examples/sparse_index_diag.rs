//! SparseIndex 性能诊断测试
//!
//! 验证 HashMap 优化是否真正生效

use std::time::Instant;
use tempfile::tempdir;
use tokitai_filekv::{FileKV, FileKVConfig};

fn main() {
    println!("=== SparseIndex 性能诊断测试 ===\n");
    
    // 创建测试数据
    let num_keys = 10_000;
    let dir = tempdir().unwrap();
    let mut config = FileKVConfig::default();
    config.segment_dir = dir.path().to_path_buf();
    config.wal_dir = dir.path().join("wal");
    config.index_dir = dir.path().join("index");
    config.enable_background_flush = false;
    config.cache.max_items = 50_000;
    
    let kv = FileKV::open(config).unwrap();
    
    println!("1. 写入 {} keys...", num_keys);
    let write_start = Instant::now();
    for i in 0..num_keys {
        let key = format!("key_{:08}", i);
        let value = format!("value_{:06}", i);
        kv.put(&key, value.as_bytes()).unwrap();
    }
    let write_duration = write_start.elapsed();
    println!("   写入耗时: {:.2} ms\n", write_duration.as_secs_f64() * 1000.0);
    
    println!("2. Flush 到 segment...");
    kv.flush_memtable().unwrap();
    println!("   Flush 完成\n");
    
    // 测试不同大小的随机读取
    let test_sizes = [100, 1000, 5000, 10000];
    
    for &size in &test_sizes {
        println!("3. 测试随机读取 {} keys...", size);
        
        // 不预热 cache - 测试真实磁盘 I/O + SparseIndex
        let test_keys: Vec<String> = (0..size)
            .map(|i| format!("key_{:08}", i * (num_keys / size)))
            .collect();
        
        let start = Instant::now();
        let mut found = 0;
        for key in &test_keys {
            if let Ok(Some(_)) = kv.get(key) {
                found += 1;
            }
        }
        let duration = start.elapsed();
        let per_key = duration.as_micros() as f64 / size as f64;
        
        println!("   找到: {}/{}", found, size);
        println!("   总耗时: {:.2} ms", duration.as_secs_f64() * 1000.0);
        println!("   平均每个 key: {:.2} µs\n", per_key);
    }
    
    // 测试 cache 预热后的性能
    println!("4. 预热所有 keys 到 cache...");
    let warmup_start = Instant::now();
    for i in 0..num_keys {
        let key = format!("key_{:08}", i);
        let _ = kv.get(&key);
    }
    let warmup_duration = warmup_start.elapsed();
    println!("   预热耗时: {:.2} ms\n", warmup_duration.as_secs_f64() * 1000.0);
    
    println!("5. 测试 cache 命中后的随机读取...");
    let test_keys: Vec<String> = (0..1000)
        .map(|i| format!("key_{:08}", i * 10))
        .collect();
    
    let start = Instant::now();
    for key in &test_keys {
        let _ = kv.get(key);
    }
    let duration = start.elapsed();
    let per_key = duration.as_micros() as f64 / 1000.0;
    
    println!("   总耗时: {:.2} ms", duration.as_secs_f64() * 1000.0);
    println!("   平均每个 key: {:.2} µs", per_key);
    
    println!("\n=== 诊断完成 ===");
    println!("\n预期结果:");
    println!("  - 未预热 cache: ~10-100 µs/key (磁盘 I/O + 索引查找)");
    println!("  - 已预热 cache: ~0.1-1 µs/key (纯内存查找)");
    println!("\n如果 SparseIndex 优化生效:");
    println!("  - O(1) HashMap 查找应该 < 1 µs");
    println!("  - 不应该看到 O(n) 线性扫描的性能特征");
}
