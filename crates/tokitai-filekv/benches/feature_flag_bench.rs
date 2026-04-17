//! Feature Flag Performance Benchmarks
//!
//! Measures the performance of the Feature Flag system:
//! - Feature check latency
//! - Callback invocation overhead
//! - Stats tracking overhead

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use tokitai_filekv::{FeatureFlag, FeatureFlagController, FeatureStateChange};

/// Benchmark feature check latency
fn bench_feature_check(c: &mut Criterion) {
    let controller = FeatureFlagController::new();

    let mut group = c.benchmark_group("feature_check");
    group.throughput(Throughput::Elements(1));

    group.bench_function("is_enabled", |b| {
        b.iter(|| {
            let _ = controller.is_enabled(black_box(FeatureFlag::Inno002ZoneMapPruning));
        })
    });

    group.bench_function("set_enabled_true", |b| {
        b.iter(|| {
            controller.set_enabled(black_box(FeatureFlag::Inno002ZoneMapPruning), true);
        })
    });

    group.bench_function("set_enabled_false", |b| {
        b.iter(|| {
            controller.set_enabled(black_box(FeatureFlag::Inno002ZoneMapPruning), false);
        })
    });

    group.finish();
}

/// Benchmark callback invocation overhead
fn bench_callback_overhead(c: &mut Criterion) {
    let controller = FeatureFlagController::new();

    let mut group = c.benchmark_group("callback_overhead");
    group.throughput(Throughput::Elements(1));

    // Test with no callbacks
    group.bench_function("toggle_no_callback", |b| {
        b.iter(|| {
            controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, true);
            controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, false);
        })
    });

    // Test with 1 callback
    let callback_count_1 = Arc::new(AtomicUsize::new(0));
    let callback_count_1_clone = Arc::clone(&callback_count_1);
    let callback_1 = Arc::new(move |_change: FeatureStateChange| {
        callback_count_1_clone.fetch_add(1, Ordering::SeqCst);
    });
    controller.register_callback(callback_1);

    group.bench_function("toggle_1_callback", |b| {
        b.iter(|| {
            controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, true);
            controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, false);
        })
    });

    // Test with 5 callbacks
    for _i in 0..4 {
        let callback = Arc::new(move |_change: FeatureStateChange| {
            // Simulate some work
        });
        controller.register_callback(callback);
    }

    group.bench_function("toggle_5_callbacks", |b| {
        b.iter(|| {
            controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, true);
            controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, false);
        })
    });

    group.finish();
}

/// Benchmark stats tracking
fn bench_feature_stats(c: &mut Criterion) {
    let controller = FeatureFlagController::new();

    let mut group = c.benchmark_group("feature_stats");
    group.throughput(Throughput::Elements(1));

    group.bench_function("get_stats", |b| {
        b.iter(|| {
            let _ = controller.get_stats();
        })
    });

    group.bench_function("generate_report", |b| {
        b.iter(|| {
            let _ = controller.generate_report();
        })
    });

    group.finish();
}

/// Benchmark batch operations
fn bench_batch_operations(c: &mut Criterion) {
    let controller = FeatureFlagController::new();

    let mut group = c.benchmark_group("batch_operations");
    group.throughput(Throughput::Elements(1));

    group.bench_function("enable_inno001", |b| {
        b.iter(|| {
            controller.enable_inno001();
        })
    });

    group.bench_function("enable_inno002", |b| {
        b.iter(|| {
            controller.enable_inno002();
        })
    });

    group.bench_function("disable_inno001", |b| {
        b.iter(|| {
            controller.disable_inno001();
        })
    });

    group.bench_function("disable_inno002", |b| {
        b.iter(|| {
            controller.disable_inno002();
        })
    });

    group.finish();
}

/// Benchmark callback filtering
fn bench_callback_filtering(c: &mut Criterion) {
    let controller = FeatureFlagController::new();

    let callback_count = Arc::new(AtomicUsize::new(0));
    let callback_count_clone = Arc::clone(&callback_count);

    let callback = Arc::new(move |_change: FeatureStateChange| {
        callback_count_clone.fetch_add(1, Ordering::SeqCst);
    });

    controller.register_callback(callback);

    let mut group = c.benchmark_group("callback_filtering");
    group.throughput(Throughput::Elements(1));

    group.bench_function("toggle_inno001", |b| {
        b.iter(|| {
            controller.enable_inno001();
            controller.disable_inno001();
        })
    });

    group.bench_function("toggle_inno002", |b| {
        b.iter(|| {
            controller.enable_inno002();
            controller.disable_inno002();
        })
    });

    group.finish();
}

criterion_group!(
    name = feature_flag_benches;
    config = Criterion::default()
        .sample_size(100)
        .measurement_time(std::time::Duration::from_secs(10));
    targets =
        bench_feature_check,
        bench_callback_overhead,
        bench_feature_stats,
        bench_batch_operations,
        bench_callback_filtering,
);

criterion_main!(feature_flag_benches);
