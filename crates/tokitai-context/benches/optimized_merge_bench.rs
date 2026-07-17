//! Optimized Merge Benchmark Suite
//!
//! This benchmark suite measures the performance of the diff3 merge algorithm
//! after fixing the infinite loop issue in generate_diff3_hunks.

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use tempfile::TempDir;
use tokitai_context::optimized_merge::AdvancedMerger;

/// Create an AdvancedMerger instance for benchmarking
fn setup_merger() -> (TempDir, AdvancedMerger) {
    let temp_dir = TempDir::new().unwrap();
    let merger = AdvancedMerger::new(temp_dir.path(), temp_dir.path()).unwrap();
    (temp_dir, merger)
}

/// Benchmark: Simple 3-way merge (no conflicts)
fn bench_diff3_merge_simple(c: &mut Criterion) {
    let mut group = c.benchmark_group("Diff3 Merge Simple");
    group.throughput(Throughput::Elements(1));

    group.bench_function("3 lines no conflict", |b| {
        let (_temp_dir, merger) = setup_merger();
        let base = "line1\nline2\nline3";
        let source = "line1\nmodified\nline3";
        let target = "line1\nline2\nline3";

        b.iter(|| {
            black_box(merger.diff3_merge(base, source, target)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark: Medium-sized merge (no conflicts)
fn bench_diff3_merge_medium(c: &mut Criterion) {
    let mut group = c.benchmark_group("Diff3 Merge Medium");
    group.throughput(Throughput::Elements(1));

    group.bench_function("100 lines no conflict", |b| {
        let (_temp_dir, merger) = setup_merger();
        let base: String = (1..=100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let source: String = (1..=100)
            .map(|i| if i == 50 { "line 50 modified".to_string() } else { format!("line {}", i) })
            .collect::<Vec<_>>()
            .join("\n");
        let target = base.clone();

        b.iter(|| {
            black_box(merger.diff3_merge(&base, &source, &target)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark: Large merge (no conflicts)
fn bench_diff3_merge_large(c: &mut Criterion) {
    let mut group = c.benchmark_group("Diff3 Merge Large");
    group.throughput(Throughput::Elements(1));

    group.bench_function("1000 lines no conflict", |b| {
        let (_temp_dir, merger) = setup_merger();
        let base: String = (1..=1000)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let source: String = (1..=1000)
            .map(|i| {
                if i == 500 {
                    "line 500 modified".to_string()
                } else {
                    format!("line {}", i)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let target = base.clone();

        b.iter(|| {
            black_box(merger.diff3_merge(&base, &source, &target)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark: Merge with conflicts
fn bench_diff3_merge_conflict(c: &mut Criterion) {
    let mut group = c.benchmark_group("Diff3 Merge Conflict");
    group.throughput(Throughput::Elements(1));

    group.bench_function("3 lines with conflict", |b| {
        let (_temp_dir, merger) = setup_merger();
        let base = "line1\nline2\nline3";
        let source = "line1\nsource_change\nline3";
        let target = "line1\ntarget_change\nline3";

        b.iter(|| {
            black_box(merger.diff3_merge(base, source, target)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark: Multiple changes (no conflicts)
fn bench_diff3_merge_multiple_changes(c: &mut Criterion) {
    let mut group = c.benchmark_group("Diff3 Merge Multiple Changes");
    group.throughput(Throughput::Elements(1));

    group.bench_function("100 lines multiple changes", |b| {
        let (_temp_dir, merger) = setup_merger();
        let base: String = (1..=100)
            .map(|i| format!("line {}", i))
            .collect::<Vec<_>>()
            .join("\n");
        let source: String = (1..=100)
            .map(|i| {
                if i % 20 == 0 {
                    format!("line {} modified", i)
                } else {
                    format!("line {}", i)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let target = base.clone();

        b.iter(|| {
            black_box(merger.diff3_merge(&base, &source, &target)).unwrap();
        });
    });

    group.finish();
}

/// Benchmark: LCS computation (core algorithm)
fn bench_lcs_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("LCS Computation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("LCS 100 elements", |b| {
        let a: Vec<&str> = (1..=100).map(|i| if i % 10 == 0 { "changed" } else { "same" }).collect();
        let b_vec: Vec<&str> = (1..=100).map(|i| if i % 15 == 0 { "different" } else { "same" }).collect();

        b.iter(|| {
            black_box(AdvancedMerger::compute_lcs_pairs(&a, &b_vec));
        });
    });

    group.finish();
}

criterion_group!(
    name = optimized_merge_benches;
    config = Criterion::default().sample_size(50);
    targets =
        bench_diff3_merge_simple,
        bench_diff3_merge_medium,
        bench_diff3_merge_large,
        bench_diff3_merge_conflict,
        bench_diff3_merge_multiple_changes,
        bench_lcs_computation,
);

criterion_main!(optimized_merge_benches);
