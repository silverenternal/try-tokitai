//! Feature Flag Integration Tests
//!
//! Tests for runtime feature flag control with FileKV

use std::sync::Arc;
use tempfile::TempDir;
use crate::{FileKV, FileKVConfig};
use crate::ops::feature_flag::FeatureFlag;

#[test]
fn test_filekv_feature_flag_controller_initialization() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");

    // Feature flag controller should be initialized
    let controller = filekv.get_feature_flag_controller();
    assert_eq!(controller.feature_count(), 3);

    // All features should be enabled by default
    assert!(controller.is_enabled(FeatureFlag::Inno001AdaptiveBloomCache));
    assert!(controller.is_enabled(FeatureFlag::Inno002ZoneMapPruning));
    assert!(controller.is_enabled(FeatureFlag::Inno002SequentialPrefetch));
}

#[test]
fn test_filekv_runtime_feature_toggle() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");
    let controller = filekv.get_feature_flag_controller();

    // Initially enabled
    assert!(controller.is_enabled(FeatureFlag::Inno002ZoneMapPruning));

    // Disable at runtime
    controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, false);
    assert!(!controller.is_enabled(FeatureFlag::Inno002ZoneMapPruning));

    // Re-enable at runtime
    controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, true);
    assert!(controller.is_enabled(FeatureFlag::Inno002ZoneMapPruning));
}

#[test]
fn test_filekv_enable_disable_inno002() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");
    let controller = filekv.get_feature_flag_controller();

    // Disable INNO-002
    filekv.disable_inno002();
    assert!(!controller.is_enabled(FeatureFlag::Inno002ZoneMapPruning));
    assert!(!controller.is_enabled(FeatureFlag::Inno002SequentialPrefetch));
    assert!(!controller.is_inno002_fully_enabled());

    // Enable INNO-002
    filekv.enable_inno002();
    assert!(controller.is_enabled(FeatureFlag::Inno002ZoneMapPruning));
    assert!(controller.is_enabled(FeatureFlag::Inno002SequentialPrefetch));
    assert!(controller.is_inno002_fully_enabled());
}

#[test]
fn test_filekv_enable_disable_inno001() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");
    let controller = filekv.get_feature_flag_controller();

    // Disable INNO-001
    filekv.disable_inno001();
    assert!(!controller.is_enabled(FeatureFlag::Inno001AdaptiveBloomCache));
    assert!(!controller.is_inno001_fully_enabled());

    // Enable INNO-001
    filekv.enable_inno001();
    assert!(controller.is_enabled(FeatureFlag::Inno001AdaptiveBloomCache));
    assert!(controller.is_inno001_fully_enabled());
}

#[test]
fn test_filekv_feature_flag_stats() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");

    // Initial stats
    let stats = filekv.get_feature_flag_stats();
    assert_eq!(stats.total_toggles, 0);

    // Toggle features
    filekv.disable_inno002(); // 2 toggles

    let stats = filekv.get_feature_flag_stats();
    assert!(stats.total_toggles >= 2);

    // Re-enable should also count
    filekv.enable_inno002(); // 2 more toggles
    let stats = filekv.get_feature_flag_stats();
    assert!(stats.total_toggles >= 4);
}

#[test]
fn test_filekv_feature_flag_report() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");

    // Generate report
    let report = filekv.generate_feature_flag_report();
    assert_eq!(report.features.len(), 3);

    // Toggle a feature
    filekv.disable_inno002();

    let report = filekv.generate_feature_flag_report();
    assert!(report.total_toggles >= 2);

    // Test display format
    let display = format!("{}", report);
    assert!(display.contains("Feature Flag Report"));
}

#[test]
fn test_filekv_pruner_prefetcher_respect_feature_flags() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_zone_map_pruning: true,
        enable_sequential_prefetch: true,
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");

    // Initially, both should be available (features enabled by default)
    assert!(filekv.get_range_query_pruner().is_some());
    assert!(filekv.get_sequential_prefetcher().is_some());

    // Disable INNO-002 at runtime
    filekv.disable_inno002();

    // Now both should return None due to feature flag check
    assert!(filekv.get_range_query_pruner().is_none());
    assert!(filekv.get_sequential_prefetcher().is_none());

    // Re-enable INNO-002
    filekv.enable_inno002();

    // Both should be available again
    assert!(filekv.get_range_query_pruner().is_some());
    assert!(filekv.get_sequential_prefetcher().is_some());
}

#[test]
fn test_filekv_individual_feature_toggles() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        enable_zone_map_pruning: true,
        enable_sequential_prefetch: true,
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");
    let controller = filekv.get_feature_flag_controller();

    // Disable only zone map pruning
    controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, false);
    assert!(!controller.is_zone_map_pruning_enabled());
    assert!(controller.is_sequential_prefetch_enabled());

    // Pruner should be unavailable, prefetcher should still work
    assert!(filekv.get_range_query_pruner().is_none());
    assert!(filekv.get_sequential_prefetcher().is_some());

    // Disable only sequential prefetch
    controller.set_enabled(FeatureFlag::Inno002SequentialPrefetch, false);
    assert!(!controller.is_sequential_prefetch_enabled());

    // Both should be unavailable now
    assert!(filekv.get_range_query_pruner().is_none());
    assert!(filekv.get_sequential_prefetcher().is_none());
}

#[test]
fn test_filekv_feature_flag_callback() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use crate::ops::feature_flag::FeatureStateChange;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = FileKV::open(config).expect("Failed to open FileKV store");
    let controller = filekv.get_feature_flag_controller();

    let callback_count = Arc::new(AtomicUsize::new(0));
    let callback_count_clone = Arc::clone(&callback_count);

    let callback = Arc::new(move |_change: FeatureStateChange| {
        callback_count_clone.fetch_add(1, Ordering::SeqCst);
    });

    controller.register_callback(callback);

    // Toggle should trigger callback
    filekv.disable_inno002();
    assert_eq!(callback_count.load(Ordering::SeqCst), 2); // 2 features disabled

    // Enable should also trigger callback
    filekv.enable_inno002();
    assert_eq!(callback_count.load(Ordering::SeqCst), 4); // 2 features enabled = 4 total
}

#[test]
fn test_filekv_concurrent_feature_toggles() {
    use std::thread;
    use std::time::Duration;

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config = FileKVConfig {
        segment_dir: temp_dir.path().join("segments"),
        wal_dir: temp_dir.path().join("wal"),
        index_dir: temp_dir.path().join("index"),
        checkpoint_dir: temp_dir.path().join("checkpoints"),
        ..Default::default()
    };

    let filekv = Arc::new(FileKV::open(config).expect("Failed to open FileKV store"));
    let filekv_clone = Arc::clone(&filekv);

    // Spawn multiple threads toggling features
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let kv = Arc::clone(&filekv);
            thread::spawn(move || {
                for _ in 0..50 {
                    if i % 2 == 0 {
                        kv.enable_inno002();
                        kv.disable_inno002();
                    } else {
                        kv.enable_inno001();
                        kv.disable_inno001();
                    }
                    thread::sleep(Duration::from_micros(100));
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Failed to join thread");
    }

    // Should not panic or deadlock
    let _ = filekv_clone.get_feature_flag_stats();
    let _ = filekv_clone.generate_feature_flag_report();
}
