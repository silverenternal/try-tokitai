//! 平行上下文性能基准测试
//!
//! 测试平行上下文系统的各项性能指标：
//! - 分支操作延迟（fork, checkout, merge）
//! - 存储效率（压缩率，去重率）
//! - 合并算法性能对比
//! - 缓存命中率

use std::time::Instant;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

use tokitai_context::{
    ParallelContextManager, ParallelContextManagerConfig,
    MergeStrategy,
    AdvancedMerger, ContentDeduplicator,
    ContentAddressableStorage, CompressionConfig,
    BloomConflictDetector, ThreeWayMerger,
    HirschbergLCS, Merger, ContextBranch,
};

/// 创建测试用的平行上下文管理器
fn setup_context_manager() -> (tempfile::TempDir, ParallelContextManager) {
    let temp_dir = tempfile::tempdir().unwrap();
    let context_root = temp_dir.path().join(".context");

    let config = ParallelContextManagerConfig {
        context_root: context_root.clone(),
        default_merge_strategy: MergeStrategy::SelectiveMerge,
        auto_cleanup_abandoned: false,
        branch_ttl_hours: None,
    };

    let manager = ParallelContextManager::new(config).unwrap();
    (temp_dir, manager)
}

/// 基准测试：分支创建（fork）
fn bench_fork_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fork Operation");

    // 测试不同分支数量下的 fork 性能
    for &branch_count in &[1, 5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(branch_count),
            &branch_count,
            |b, &count| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::ZERO;

                    for _ in 0..iters {
                        let (_temp_dir, mut manager) = setup_context_manager();

                        // 创建指定数量的分支
                        let start = Instant::now();
                        for i in 0..count {
                            let branch_name = format!("feature-{}", i);
                            let _ = manager.create_branch(&branch_name, "main");
                        }
                        total_duration += start.elapsed();
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：分支切换（checkout）
fn bench_checkout_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Checkout Operation");

    for &branch_count in &[5, 10, 20, 50] {
        group.bench_with_input(
            BenchmarkId::from_parameter(branch_count),
            &branch_count,
            |b, &count| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::ZERO;

                    for _ in 0..iters {
                        let (_temp_dir, mut manager) = setup_context_manager();

                        // 预先创建分支
                        for i in 0..count {
                            let branch_name = format!("feature-{}", i);
                            let _ = manager.create_branch(&branch_name, "main");
                        }

                        // 测试切换性能
                        let start = Instant::now();
                        for i in 0..count {
                            let branch_name = format!("feature-{}", i);
                            // 获取分支 ID
                            if let Some(branch) = manager.get_branch(&branch_name) {
                                let branch_id = branch.branch_id.clone();
                                let _ = manager.checkout(&branch_id);
                            }
                        }
                        total_duration += start.elapsed();
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：合并操作
fn bench_merge_operation(c: &mut Criterion) {
    let mut group = c.benchmark_group("Merge Operation");

    for &strategy in &["fast_forward", "selective", "theirs"] {
        group.bench_with_input(
            BenchmarkId::from_parameter(strategy),
            &strategy,
            |b, &strat| {
                b.iter_custom(|iters| {
                    let mut total_duration = std::time::Duration::ZERO;

                    for _ in 0..iters {
                        let (_temp_dir, mut manager) = setup_context_manager();

                        // 创建分支
                        let branch = manager.create_branch("feature", "main").unwrap();
                        let branch_id = branch.branch_id.clone();

                        // 测试合并性能
                        let start = Instant::now();
                        let merge_strategy = match strat {
                            "fast_forward" => MergeStrategy::FastForward,
                            "theirs" => MergeStrategy::Theirs,
                            _ => MergeStrategy::SelectiveMerge,
                        };
                        let _ = manager.merge(&branch_id, "main", Some(merge_strategy));
                        total_duration += start.elapsed();
                    }

                    total_duration
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：diff3 合并算法
fn bench_diff3_merge(c: &mut Criterion) {
    let mut group = c.benchmark_group("Diff3 Merge");

    // 测试不同大小的文本
    for &line_count in &[10, 50, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(line_count),
            &line_count,
            |b, &lines| {
                b.iter(|| {
                    let temp_dir = tempfile::tempdir().unwrap();
                    let merger = AdvancedMerger::new(
                        temp_dir.path(),
                        temp_dir.path(),
                    ).unwrap();

                    // 生成测试内容
                    let base = generate_test_content(lines, 0);
                    let source = generate_test_content(lines, 1);
                    let target = generate_test_content(lines, 2);

                    black_box(merger.diff3_merge(&base, &source, &target)).unwrap();
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：LCS 计算
fn bench_lcs_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("LCS Computation");

    for &size in &[10, 50, 100, 200, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            &size,
            |bencher, &n| {
                let a: Vec<usize> = (0..n).collect();
                let b: Vec<usize> = (0..n).step_by(2).collect();

                bencher.iter(|| {
                    black_box(HirschbergLCS::compute_lcs(&a, &b));
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：内容去重
fn bench_content_deduplication(c: &mut Criterion) {
    let mut group = c.benchmark_group("Content Deduplication");

    for &item_count in &[100, 500, 1000, 5000, 10000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(item_count),
            &item_count,
            |b, &count| {
                b.iter(|| {
                    let mut dedup = ContentDeduplicator::new();

                    // 50% 重复率
                    for i in 0..count {
                        let content = if i % 2 == 0 {
                            format!("content_{}", i / 2)
                        } else {
                            format!("content_{}", i % (count / 2))
                        };
                        black_box(dedup.deduplicate(&content));
                    }

                    black_box(dedup.stats());
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：内容寻址存储
fn bench_cas_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("Content Addressable Storage");

    // 写入性能
    group.bench_function("CAS Write", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();
            let config = CompressionConfig::default();
            let mut cas = ContentAddressableStorage::new(temp_dir.path(), config).unwrap();

            let content = b"Test content for CAS benchmark";
            black_box(cas.store(content)).unwrap();
        });
    });

    // 读取性能
    group.bench_function("CAS Read", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();
            let config = CompressionConfig::default();
            let mut cas = ContentAddressableStorage::new(temp_dir.path(), config).unwrap();

            let content = b"Test content for CAS benchmark";
            let hash = cas.store(content).unwrap();
            black_box(cas.retrieve(&hash)).unwrap();
        });
    });

    // 去重性能
    group.bench_function("CAS Deduplication", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = std::time::Duration::ZERO;

            for _ in 0..iters {
                let temp_dir = tempfile::tempdir().unwrap();
                let config = CompressionConfig::default();
                let mut cas = ContentAddressableStorage::new(temp_dir.path(), config).unwrap();

                let content = b"Duplicate content";

                let start = Instant::now();
                // 存储 10 次相同内容
                for _ in 0..10 {
                    let _ = cas.store(content);
                }
                total_duration += start.elapsed();

                // 验证去重
                let stats = cas.stats();
                assert_eq!(stats.total_objects, 1);
            }

            total_duration
        });
    });

    group.finish();
}

/// 基准测试：Bloom Filter 冲突检测
fn bench_bloom_conflict_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("Bloom Conflict Detection");

    for &file_count in &[10, 50, 100, 500, 1000] {
        group.bench_with_input(
            BenchmarkId::from_parameter(file_count),
            &file_count,
            |b, &count| {
                b.iter(|| {
                    let temp_dir = tempfile::tempdir().unwrap();

                    // 创建源分支
                    let source_dir = temp_dir.path().join("source");
                    let source_layer = source_dir.join("short-term");
                    std::fs::create_dir_all(&source_layer).unwrap();

                    // 创建目标分支
                    let target_dir = temp_dir.path().join("target");
                    let target_layer = target_dir.join("short-term");
                    std::fs::create_dir_all(&target_layer).unwrap();

                    // 创建测试文件
                    for i in 0..count {
                        std::fs::write(
                            source_layer.join(format!("file_{}.txt", i)),
                            format!("content_{}", i),
                        ).unwrap();

                        // 50% 的文件有冲突
                        let target_content = if i % 2 == 0 {
                            format!("content_{}", i)
                        } else {
                            format!("different_{}", i)
                        };
                        std::fs::write(
                            target_layer.join(format!("file_{}.txt", i)),
                            target_content,
                        ).unwrap();
                    }

                    black_box(BloomConflictDetector::new(
                        &source_dir,
                        &target_dir,
                        "short-term",
                    ).unwrap());
                });
            },
        );
    }

    group.finish();
}

/// 基准测试：三路合并 vs 两路合并
fn bench_merge_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("Merge Comparison");

    group.bench_function("Two-way Merge", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();

            // 创建测试分支
            let source_dir = temp_dir.path().join("source");
            let source_branch = create_test_branch("source", "main", &source_dir, 20);

            let target_dir = temp_dir.path().join("target");
            let target_branch = create_test_branch("target", "main", &target_dir, 20);

            let merger = Merger::new(
                &temp_dir.path().join("branches"),
                &temp_dir.path().join("merge_logs"),
            ).unwrap();

            black_box(merger.merge(
                &source_branch,
                &target_branch,
                MergeStrategy::SelectiveMerge,
            )).unwrap();
        });
    });

    group.bench_function("Three-way Merge", |b| {
        b.iter(|| {
            let temp_dir = tempfile::tempdir().unwrap();

            // 创建测试分支
            let base_dir = temp_dir.path().join("base");
            let base_branch = create_test_branch("base", "main", &base_dir, 20);

            let source_dir = temp_dir.path().join("source");
            let source_branch = create_test_branch("source", "base", &source_dir, 20);

            let target_dir = temp_dir.path().join("target");
            let target_branch = create_test_branch("target", "base", &target_dir, 20);

            let merger = ThreeWayMerger::new(temp_dir.path().join("three_way")).unwrap();

            black_box(merger.merge(
                &source_branch,
                &target_branch,
                &base_branch,
            )).unwrap();
        });
    });

    group.finish();
}

/// 生成测试内容
fn generate_test_content(lines: usize, variant: usize) -> String {
    (0..lines)
        .map(|i| {
            if i % 10 == 0 {
                format!("Line {}: variant {}", i, variant)
            } else {
                format!("Line {}: common content", i)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// 创建测试分支
fn create_test_branch(
    id: &str,
    parent: &str,
    dir: &std::path::Path,
    file_count: usize,
) -> ContextBranch {
    let branch = ContextBranch::new(id, id, parent, dir.to_path_buf()).unwrap();

    // 创建测试文件
    for i in 0..file_count {
        let file_path = branch.short_term_dir.join(format!("file_{}.txt", i));
        std::fs::write(&file_path, format!("content_{}", i)).unwrap();
    }

    branch
}

criterion_group!(
    benches,
    bench_fork_operation,
    bench_checkout_operation,
    bench_merge_operation,
    bench_diff3_merge,
    bench_lcs_computation,
    bench_content_deduplication,
    bench_cas_operations,
    bench_bloom_conflict_detection,
    bench_merge_comparison,
);

criterion_main!(benches);
