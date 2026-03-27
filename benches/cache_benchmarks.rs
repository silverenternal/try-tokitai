//! 平行上下文缓存优化基准测试
//!
//! 测试 LRU 分支缓存和祖先链缓存的性能提升

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use ai_assistant::context::{BranchCache, AncestorCache, ContextBranch};
use std::collections::HashMap;
use tempfile::TempDir;

/// 创建测试分支
fn create_test_branch(id: &str, name: &str, parent: &str) -> ContextBranch {
    let temp_dir = TempDir::new().unwrap();
    let branch_dir = temp_dir.path().join(id);
    ContextBranch::new(id, name, parent, branch_dir).unwrap()
}

/// 基准测试：无缓存的分支访问
fn bench_branch_access_no_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("branch_access");
    group.throughput(Throughput::Elements(1));

    // 模拟磁盘加载（带延迟）
    let mut branches = HashMap::new();
    for i in 0..100 {
        let branch = create_test_branch(&format!("branch-{}", i), &format!("Branch {}", i), "main");
        branches.insert(format!("branch-{}", i), branch);
    }

    let loader = |id: &str| -> Option<ContextBranch> {
        // 模拟磁盘 I/O 延迟
        std::thread::sleep(std::time::Duration::from_millis(10));
        branches.get(id).cloned()
    };

    group.bench_function("no_cache", |b| {
        b.iter(|| {
            // 每次都"从磁盘加载"
            loader(black_box("branch-50"))
        })
    });

    group.finish();
}

/// 基准测试：有 LRU 缓存的分支访问
fn bench_branch_access_with_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("branch_access");
    group.throughput(Throughput::Elements(1));

    // 创建缓存
    let cache = BranchCache::new(50);

    // 预加载一些分支
    for i in 0..50 {
        let branch = create_test_branch(&format!("branch-{}", i), &format!("Branch {}", i), "main");
        cache.insert(format!("branch-{}", i), branch);
    }

    group.bench_function("with_cache_hit", |b| {
        b.iter(|| {
            cache.get(black_box("branch-25"))
        })
    });

    group.bench_function("with_cache_miss", |b| {
        b.iter(|| {
            // 模拟未命中时的磁盘加载
            std::thread::sleep(std::time::Duration::from_millis(10));
            cache.get(black_box("branch-999"))
        })
    });

    group.finish();
}

/// 基准测试：混合缓存命中率
fn bench_branch_access_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("branch_access_mixed");
    
    let cache = BranchCache::new(50);

    // 预加载 50 个分支
    for i in 0..50 {
        let branch = create_test_branch(&format!("branch-{}", i), &format!("Branch {}", i), "main");
        cache.insert(format!("branch-{}", i), branch);
    }

    // 80% 命中率场景
    group.bench_function("80pct_hit_rate", |b| {
        b.iter(|| {
            let id = if rand::random::<u100>() < 80 {
                // 80% 概率访问缓存中的分支
                format!("branch-{}", rand::random::<u8>() % 50)
            } else {
                // 20% 概率访问不存在的分支
                format!("missing-{}", rand::random::<u8>())
            };
            cache.get(&id)
        })
    });

    group.finish();
}

/// 基准测试：无缓存的祖先链查询
fn bench_ancestor_query_no_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("ancestor_query");
    
    // 创建深层分支层次：main <- f1 <- f2 <- ... <- f50
    let mut parent_map = HashMap::new();
    parent_map.insert("f0".to_string(), "main".to_string());
    for i in 1..=50 {
        parent_map.insert(format!("f{}", i), format!("f{}", i - 1));
    }
    parent_map.insert("main".to_string(), "".to_string());

    let loader = |id: &str| -> Option<String> {
        // 模拟磁盘 I/O 延迟
        std::thread::sleep(std::time::Duration::from_millis(1));
        parent_map.get(id).cloned()
    };

    group.bench_function("no_cache_deep_chain", |b| {
        b.iter(|| {
            // 查询深度为 50 的祖先链
            let mut ancestors = Vec::new();
            let mut current = black_box("f50");
            while let Some(parent) = loader(current) {
                if !parent.is_empty() {
                    ancestors.push(parent.clone());
                    current = &parent;
                } else {
                    break;
                }
            }
            ancestors
        })
    });

    group.finish();
}

/// 基准测试：有缓存的祖先链查询
fn bench_ancestor_query_with_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("ancestor_query");
    
    // 创建分支层次
    let mut parent_map = HashMap::new();
    parent_map.insert("f0".to_string(), "main".to_string());
    for i in 1..=50 {
        parent_map.insert(format!("f{}", i), format!("f{}", i - 1));
    }
    parent_map.insert("main".to_string(), "".to_string());

    let loader = |id: &str| -> Option<String> {
        std::thread::sleep(std::time::Duration::from_millis(1));
        parent_map.get(id).cloned()
    };

    let cache = AncestorCache::new();

    // 预热缓存
    cache.get_ancestors("f50", &loader);

    group.bench_function("with_cache_hit", |b| {
        b.iter(|| {
            cache.get_ancestors(black_box("f50"), &loader)
        })
    });

    group.finish();
}

/// 基准测试：后代查询性能
fn bench_is_descendant_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("is_descendant_query");
    
    let mut parent_map = HashMap::new();
    parent_map.insert("f0".to_string(), "main".to_string());
    for i in 1..=100 {
        parent_map.insert(format!("f{}", i), format!("f{}", i - 1));
    }
    parent_map.insert("main".to_string(), "".to_string());

    let loader = |id: &str| -> Option<String> {
        parent_map.get(id).cloned()
    };

    let cache = AncestorCache::new();

    // 预热
    for i in 0..=100 {
        cache.get_ancestors(&format!("f{}", i), &loader);
    }

    group.bench_function("cached_is_descendant", |b| {
        b.iter(|| {
            cache.is_descendant_of(
                black_box("f100"),
                black_box("main"),
                &loader,
            )
        })
    });

    group.finish();
}

/// 基准测试：公共祖先查找
fn bench_common_ancestor_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("common_ancestor_query");
    
    // 创建树状结构
    // main <- feature-a <- sub-a1
    //      <- feature-b <- sub-b1
    let mut parent_map = HashMap::new();
    parent_map.insert("feature-a".to_string(), "main".to_string());
    parent_map.insert("feature-b".to_string(), "main".to_string());
    parent_map.insert("sub-a1".to_string(), "feature-a".to_string());
    parent_map.insert("sub-b1".to_string(), "feature-b".to_string());
    parent_map.insert("main".to_string(), "".to_string());

    let loader = |id: &str| -> Option<String> {
        parent_map.get(id).cloned()
    };

    let cache = AncestorCache::new();

    // 预热
    cache.get_ancestors("sub-a1", &loader);
    cache.get_ancestors("sub-b1", &loader);

    group.bench_function("cached_common_ancestor", |b| {
        b.iter(|| {
            cache.find_common_ancestor(
                black_box("sub-a1"),
                black_box("sub-b1"),
                &loader,
                "main",
            )
        })
    });

    group.finish();
}

/// 基准测试：缓存统计
fn bench_cache_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_operations");
    
    let cache = BranchCache::new(100);

    // 插入 100 个分支
    for i in 0..100 {
        let branch = create_test_branch(&format!("branch-{}", i), &format!("Branch {}", i), "main");
        cache.insert(format!("branch-{}", i), branch);
    }

    // 访问所有分支
    for i in 0..100 {
        cache.get(&format!("branch-{}", i));
    }

    group.bench_function("get_stats", |b| {
        b.iter(|| {
            cache.stats()
        })
    });

    group.finish();
}

/// 基准测试：LRU 驱逐性能
fn bench_lru_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_eviction");
    
    let cache = BranchCache::new(50);

    // 填满缓存
    for i in 0..50 {
        let branch = create_test_branch(&format!("branch-{}", i), &format!("Branch {}", i), "main");
        cache.insert(format!("branch-{}", i), branch);
    }

    group.bench_function("eviction_throughput", |b| {
        b.iter(|| {
            // 插入新元素会触发驱逐
            let id = format!("new-branch-{}", black_box(rand::random::<u32>()));
            let branch = create_test_branch(&id, "New Branch", "main");
            cache.insert(id, branch);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_branch_access_no_cache,
    bench_branch_access_with_cache,
    bench_branch_access_mixed,
    bench_ancestor_query_no_cache,
    bench_ancestor_query_with_cache,
    bench_is_descendant_query,
    bench_common_ancestor_query,
    bench_cache_stats,
    bench_lru_eviction,
);

criterion_main!(benches);
