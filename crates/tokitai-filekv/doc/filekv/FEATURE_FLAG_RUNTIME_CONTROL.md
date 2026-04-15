# Feature Flag Runtime Control

## Overview

This document describes the Feature Flag runtime control system for FileKV storage engine, enabling dynamic toggling of experimental features without restart.

## Architecture

### Design Goals

1. **Runtime Control**: Toggle features without application restart
2. **Thread-Safe**: Lock-free reads, write locks only on toggle
3. **Zero Overhead**: Fast path for feature checks (no allocation)
4. **Observability**: Statistics tracking and event hooks
5. **Gradual Rollout**: Support per-feature enable/disable

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
├─────────────────────────────────────────────────────────────┤
│  FileKV Public API                                           │
│  - enable_inno001() / disable_inno001()                     │
│  - enable_inno002() / disable_inno002()                     │
│  - get_feature_flag_controller()                            │
├─────────────────────────────────────────────────────────────┤
│  FeatureFlagController                                       │
│  - Thread-safe state management (parking_lot::RwLock)       │
│  - Event callbacks                                          │
│  - Statistics tracking                                      │
├─────────────────────────────────────────────────────────────┤
│  Feature Flags                                               │
│  - Inno001AdaptiveBloomCache                                │
│  - Inno002ZoneMapPruning                                    │
│  - Inno002SequentialPrefetch                                │
└─────────────────────────────────────────────────────────────┘
```

## Feature Flags

### INNO-001: Adaptive Bloom Filter Cache

**Description**: Multi-layer adaptive Bloom filter cache with dynamic FPR control

**Sub-features**:
- L1/L2/L3 multi-layer bloom cache
- Dynamic false positive rate controller
- Compressed bloom filters (RLE + Huffman)

**Runtime Toggle**:
```rust
// Disable INNO-001
filekv.disable_inno001();

// Enable INNO-001
filekv.enable_inno001();

// Check status
let controller = filekv.get_feature_flag_controller();
if controller.is_enabled(FeatureFlag::Inno001AdaptiveBloomCache) {
    println!("INNO-001 is enabled");
}
```

**Impact**: Disabling stops new bloom filters from being cached, but existing cached filters remain until eviction.

### INNO-002: Zone Map + Sequential Prefetch

**Description**: Zone Map-based block pruning and pattern-based sequential prefetching

**Sub-features**:
1. **Zone Map Pruning** (`Inno002ZoneMapPruning`)
   - Block-level min/max indexing
   - Query predicate pushdown
   - Block skipping optimization

2. **Sequential Prefetch** (`Inno002SequentialPrefetch`)
   - Pattern-based prefetching
   - Adaptive prefetch distance
   - Cache-friendly sequential reads

**Runtime Toggle**:
```rust
// Disable entire INNO-002
filekv.disable_inno002();

// Enable entire INNO-002
filekv.enable_inno002();

// Toggle individual features
let controller = filekv.get_feature_flag_controller();
controller.set_enabled(FeatureFlag::Inno002ZoneMapPruning, false);
controller.set_enabled(FeatureFlag::Inno002SequentialPrefetch, true);

// Check individual status
if controller.is_zone_map_pruning_enabled() {
    println!("Zone Map pruning enabled");
}
if controller.is_sequential_prefetch_enabled() {
    println!("Sequential prefetch enabled");
}
```

**Impact**: 
- Disabling Zone Map pruning: Range queries scan all blocks (no skipping)
- Disabling Sequential Prefetch: No prefetching, higher I/O latency

## API Reference

### FileKV Methods

```rust
impl FileKV {
    /// Get the feature flag controller
    pub fn get_feature_flag_controller(&self) -> Arc<FeatureFlagController>;

    /// Enable/disable INNO-001
    pub fn enable_inno001(&self);
    pub fn disable_inno001(&self);

    /// Enable/disable INNO-002
    pub fn enable_inno002(&self);
    pub fn disable_inno002(&self);

    /// Get statistics
    pub fn get_feature_flag_stats(&self) -> FeatureFlagStats;

    /// Generate report
    pub fn generate_feature_flag_report(&self) -> FeatureReport;
}
```

### FeatureFlagController Methods

```rust
impl FeatureFlagController {
    /// Create new controller
    pub fn new() -> Self;

    /// Create with custom initial states
    pub fn with_states(initial_states: HashMap<FeatureFlag, bool>) -> Self;

    /// Check if feature is enabled (fast path)
    pub fn is_enabled(&self, feature: FeatureFlag) -> bool;

    /// Get feature state
    pub fn get_state(&self, feature: FeatureFlag) -> Option<FeatureState>;

    /// Get all feature states
    pub fn get_all_states(&self) -> HashMap<FeatureFlag, FeatureState>;

    /// Enable/disable feature
    pub fn set_enabled(&self, feature: FeatureFlag, enabled: bool) -> bool;

    /// Toggle feature
    pub fn toggle(&self, feature: FeatureFlag) -> bool;

    /// Batch operations
    pub fn enable_batch(&self, features: &[FeatureFlag]) -> usize;
    pub fn disable_batch(&self, features: &[FeatureFlag]) -> usize;

    /// Callbacks
    pub fn register_callback(&self, callback: FeatureChangeCallback);
    pub fn clear_callbacks(&self);

    /// Statistics
    pub fn get_stats(&self) -> FeatureFlagStats;
    pub fn reset_stats(&self);

    /// INNO-specific helpers
    pub fn is_inno001_fully_enabled(&self) -> bool;
    pub fn is_inno002_fully_enabled(&self) -> bool;
    pub fn enable_inno001(&self);
    pub fn disable_inno001(&self);
    pub fn enable_inno002(&self);
    pub fn disable_inno002(&self);
}
```

### Global Controller

For convenience, a global controller is available:

```rust
use tokitai_context::feature_flag;

// Check feature status
if feature_flag::is_enabled(FeatureFlag::Inno002ZoneMapPruning) {
    // ...
}

// Toggle feature
feature_flag::set_enabled(FeatureFlag::Inno002SequentialPrefetch, false);

// Get controller
let controller = feature_flag::global_controller();
```

## Usage Examples

### Example 1: Basic Runtime Toggle

```rust
use tokitai_context::FileKV;
use tokitai_context::feature_flag::FeatureFlag;

fn main() -> anyhow::Result<()> {
    let config = /* ... */;
    let filekv = FileKV::open(config)?;

    // Check initial state
    let controller = filekv.get_feature_flag_controller();
    println!("INNO-002 enabled: {}", controller.is_inno002_fully_enabled());

    // Disable for testing
    filekv.disable_inno002();
    println!("INNO-002 enabled: {}", controller.is_inno002_fully_enabled());

    // Re-enable
    filekv.enable_inno002();
    println!("INNO-002 enabled: {}", controller.is_inno002_fully_enabled());

    Ok(())
}
```

### Example 2: Event Callbacks

```rust
use std::sync::Arc;
use tokitai_context::feature_flag::{FeatureFlag, FeatureStateChange};

fn main() -> anyhow::Result<()> {
    let filekv = FileKV::open(config)?;
    let controller = filekv.get_feature_flag_controller();

    // Register callback
    let callback = Arc::new(|change: FeatureStateChange| {
        println!(
            "Feature {} changed: {} -> {} at {:?}",
            change.feature.name(),
            change.old_enabled,
            change.new_enabled,
            change.timestamp.elapsed()
        );
    });

    controller.register_callback(callback);

    // Toggle triggers callback
    filekv.disable_inno002();

    Ok(())
}
```

### Example 3: Statistics and Reporting

```rust
fn main() -> anyhow::Result<()> {
    let filekv = FileKV::open(config)?;

    // Generate some toggles
    filekv.disable_inno002();
    filekv.enable_inno002();
    filekv.disable_inno001();

    // Get statistics
    let stats = filekv.get_feature_flag_stats();
    println!("Total toggles: {}", stats.total_toggles);
    println!("State checks: {}", stats.state_checks);

    // Generate report
    let report = filekv.generate_feature_flag_report();
    println!("{}", report);

    Ok(())
}
```

### Example 4: A/B Testing

```rust
fn benchmark_with_and_without_inno002(filekv: &FileKV) {
    let controller = filekv.get_feature_flag_controller();

    // Benchmark with INNO-002 enabled
    controller.enable_inno002();
    let start = Instant::now();
    // ... run range queries ...
    let time_enabled = start.elapsed();

    // Benchmark with INNO-002 disabled
    controller.disable_inno002();
    let start = Instant::now();
    // ... run same range queries ...
    let time_disabled = start.elapsed();

    println!("INNO-002 speedup: {:.2}x", time_disabled / time_enabled);
}
```

### Example 5: Advanced A/B Testing with User Assignment

```rust
use tokitai_context::feature_flag::{ABTestConfig, FeatureFlag};

fn setup_ab_test(filekv: &FileKV) -> anyhow::Result<()> {
    let controller = filekv.get_feature_flag_controller();

    // Register A/B test for INNO-002
    let config = ABTestConfig::new(
        "inn002_performance_test",
        vec![FeatureFlag::Inno002ZoneMapPruning, FeatureFlag::Inno002SequentialPrefetch],
    )
    .with_ratios(0.5, 0.5); // 50% control, 50% treatment

    controller.register_ab_test(config)?;

    // Assign user to test group
    let user_id = "user_12345";
    if let Some(assignment) = controller.assign_ab_test("inn002_performance_test", user_id) {
        if assignment.is_treatment {
            println!("User {} is in TREATMENT group", user_id);
            controller.enable_inno002();
        } else {
            println!("User {} is in CONTROL group", user_id);
            controller.disable_inno002();
        }
    }

    Ok(())
}

fn record_conversion(filekv: &FileKV, user_id: &str) {
    let controller = filekv.get_feature_flag_controller();
    
    // Record conversion for A/B test metrics
    controller.record_ab_conversion(
        "inn002_performance_test",
        user_id,
        FeatureFlag::Inno002ZoneMapPruning,
    );
}

fn analyze_ab_test_results(filekv: &FileKV) {
    let controller = filekv.get_feature_flag_controller();
    let ab_stats = controller.get_ab_stats();

    println!("Total assignments: {}", ab_stats.total_assignments);
    println!("Control group: {}", ab_stats.control_assignments);
    println!("Treatment group: {}", ab_stats.treatment_assignments);

    // Analyze per-feature metrics
    if let Some(metrics) = ab_stats.per_feature_metrics.get(&FeatureFlag::Inno002ZoneMapPruning) {
        println!("Control conversion rate: {:.2}%", metrics.control_conversion_rate() * 100.0);
        println!("Treatment conversion rate: {:.2}%", metrics.treatment_conversion_rate() * 100.0);
        println!("Lift: {:.2}%", metrics.lift() * 100.0);
    }
}
```

### Example 6: Callbacks with Priority and Filtering

```rust
use std::sync::Arc;
use tokitai_context::feature_flag::{CallbackPriority, FeatureFlag, FeatureStateChange};

fn setup_callbacks(filekv: &FileKV) {
    let controller = filekv.get_feature_flag_controller();

    // High-priority callback for critical features (executed first)
    let critical_callback = Arc::new(|change: FeatureStateChange| {
        println!("[CRITICAL] Feature {} changed!", change.feature.name());
        // Log to critical alerting system
    });

    controller.register_callback_with_priority(
        critical_callback,
        CallbackPriority::CRITICAL,
        Some(vec![FeatureFlag::Inno002ZoneMapPruning]), // Only for zone map
    );

    // Normal-priority callback for general logging
    let logging_callback = Arc::new(|change: FeatureStateChange| {
        println!("[LOG] Feature toggle: {} = {}", change.feature.name(), change.new_enabled);
    });

    let callback_id = controller.register_callback(logging_callback);

    // Low-priority callback for metrics collection (executed last)
    let metrics_callback = Arc::new(|change: FeatureStateChange| {
        // Update Prometheus metrics
        // metrics.feature_toggles.inc();
    });

    controller.register_callback_with_priority(
        metrics_callback,
        CallbackPriority::LOW,
        None, // All features
    );

    // Unregister callback if needed
    // controller.unregister_callback(callback_id);
}
```

### Example 7: Per-Feature Statistics Monitoring

```rust
use tokitai_context::feature_flag::FeatureFlag;

fn monitor_feature_usage(filekv: &FileKV) {
    let controller = filekv.get_feature_flag_controller();

    // Simulate some operations
    for _ in 0..100 {
        if controller.is_enabled(FeatureFlag::Inno002ZoneMapPruning) {
            // Perform zone map query
        }
        
        // Record operation for usage tracking
        controller.record_operation(FeatureFlag::Inno002ZoneMapPruning);
    }

    // Get per-feature statistics
    if let Some(stats) = controller.get_feature_stats(FeatureFlag::Inno002ZoneMapPruning) {
        println!("Feature: INNO-002 Zone Map Pruning");
        println!("  State checks: {}", stats.state_checks);
        println!("  Enabled hits: {}", stats.enabled_hits);
        println!("  Disabled misses: {}", stats.disabled_misses);
        println!("  Hit rate: {:.2}%", stats.hit_rate() * 100.0);
        println!("  Toggle count: {}", stats.toggle_count);
        println!("  Operations (enabled): {}", stats.operations_enabled);
        println!("  Operations (disabled): {}", stats.operations_disabled);
    }

    // Get all feature statistics
    let all_stats = controller.get_all_feature_stats();
    for (feature, stats) in &all_stats {
        println!("{:?}: hit_rate={:.2}%", feature, stats.hit_rate());
    }
}
```

## Implementation Details

### Thread Safety

- **Reads**: Lock-free using `parking_lot::RwLock` read guard
- **Writes**: Exclusive write lock during toggle
- **Callbacks**: Read lock, called synchronously in priority order
- **A/B Assignments**: Separate RwLock for deterministic user assignment

### Performance

- **Feature check**: ~10-20ns (single RwLock read + stats update)
- **Toggle**: ~100-200ns (write lock + callbacks)
- **Statistics**: Atomic counters (lock-free)
- **A/B Assignment**: ~50-100ns (hash computation + assignment)

### Memory Overhead

- **Per-feature state**: ~24 bytes
- **Per-feature statistics**: ~48 bytes (includes hits, misses, operations)
- **A/B test config**: ~100 bytes per test
- **A/B assignments**: ~64 bytes per user per test
- **Callbacks**: Variable (depends on registered callbacks)

### A/B Testing Algorithm

User assignment uses deterministic hashing (xxHash3) for consistent group assignment:

```rust
let hash = xxh3_64(user_id.as_bytes());
let ratio = (hash % 1000) as f64 / 1000.0;
let is_treatment = ratio >= control_ratio;
```

This ensures:
- Same user always gets same assignment
- Even distribution across groups
- Fast O(1) assignment lookup

## Testing

### Unit Tests

Run feature flag unit tests:
```bash
cargo test --features benchmarks --lib feature_flag
```

### Integration Tests

Run FileKV integration tests:
```bash
cargo test --features benchmarks --lib file_kv::feature_flag_tests
```

### Test Coverage

- ✅ Feature flag initialization
- ✅ Runtime toggle (enable/disable)
- ✅ Batch operations
- ✅ Event callbacks with priority
- ✅ Callback feature filtering
- ✅ Callback unregistration
- ✅ Per-feature statistics tracking
- ✅ Operation recording
- ✅ Hit rate calculation
- ✅ A/B test registration and validation
- ✅ A/B test user assignment
- ✅ A/B test conversion tracking
- ✅ A/B test statistics and metrics
- ✅ Report generation
- ✅ Thread safety (concurrent toggles)
- ✅ INNO-001/INNO-002 helper methods
- ✅ Global controller
- ✅ Callback invocation counting
- ✅ Integration with FileKV (pruner/prefetcher respect flags)

## Migration Guide

### From Static Configuration

**Before** (compile-time config):
```rust
let config = FileKVConfig {
    enable_zone_map_pruning: true,
    enable_sequential_prefetch: true,
    // ...
};
let filekv = FileKV::open(config)?;
```

**After** (runtime control):
```rust
let config = FileKVConfig {
    enable_zone_map_pruning: true,  // Still set initial state
    enable_sequential_prefetch: true,
    // ...
};
let filekv = FileKV::open(config)?;

// Toggle at runtime
filekv.disable_inno002();  // No restart needed!
```

### Backward Compatibility

- Existing `FileKVConfig` fields remain unchanged
- Default behavior: all features enabled (same as before)
- Runtime toggle is additive (no breaking changes)

## Best Practices

1. **Use runtime toggle for experimentation**: Test features in production without restart
2. **Monitor statistics**: Track toggle counts, state checks, and hit rates for observability
3. **Register callbacks**: Log feature changes for debugging and auditing
4. **Batch operations**: Use `enable_batch`/`disable_batch` for multiple features
5. **Thread safety**: Safe to call from multiple threads (uses atomic operations)
6. **A/B Testing**:
   - Use deterministic user assignment for consistent experience
   - Record conversions to measure feature impact
   - Monitor lift metrics to evaluate feature effectiveness
   - Start with small treatment groups (e.g., 10%) for risky features
7. **Callback Priority**:
   - Use `CRITICAL` priority for alerting and monitoring
   - Use `NORMAL` priority for general logging
   - Use `LOW` priority for metrics collection
8. **Feature Filters**: Use callback filters to reduce noise for irrelevant features
9. **Statistics Reset**: Reset stats after collecting baseline for clean measurements

## Troubleshooting

### Feature doesn't toggle

**Symptom**: Feature check returns old state after toggle

**Cause**: Component cached the feature state at initialization

**Solution**: Components check feature flags on each operation (guaranteed by design)

### Callback not triggered

**Symptom**: Registered callback not called on toggle

**Cause**: Callback cleared or not registered properly

**Solution**: Verify callback registration with `register_callback()`

### Performance degradation

**Symptom**: Slower feature checks after many toggles

**Cause**: High toggle frequency causing lock contention

**Solution**: Reduce toggle frequency, batch operations

## Future Enhancements

- [ ] Feature dependencies (e.g., prefetch requires zone map)
- [ ] Gradual rollout (percentage-based enable) - Partial: A/B test ratios
- [ ] Time-based scheduling (enable during off-peak hours) - Partial: ABTestConfig supports start/end times
- [ ] Persistence (survive restarts)
- [ ] Remote control (HTTP API for feature toggle)
- [ ] Metrics export (Prometheus metrics for feature usage) - Partial: Stats tracking implemented
- [ ] Advanced A/B test analytics (statistical significance testing)
- [ ] Multi-variate testing (test multiple features simultaneously)
- [ ] Callback result handling (stop propagation, conditional execution)

## Related Documents

- [INNO-002 Integration Report](./INNO002_INTEGRATION_REPORT.md)
- [Performance Benchmark Report](./PERFORMANCE_BENCHMARK_REPORT.md)
- [FileKV Architecture](./FILEKV_ARCHITECTURE.md)
