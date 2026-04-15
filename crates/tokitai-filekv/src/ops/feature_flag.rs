//! Feature Flag Runtime Control
//!
//! Provides runtime control for experimental features.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Feature flags for INNO-001 and INNO-002
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeatureFlag {
    Inno001AdaptiveBloomCache,
    Inno002ZoneMapPruning,
    Inno002SequentialPrefetch,
}

impl FeatureFlag {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Inno001AdaptiveBloomCache => "inno_001_adaptive_bloom_cache",
            Self::Inno002ZoneMapPruning => "inno_002_zone_map_pruning",
            Self::Inno002SequentialPrefetch => "inno_002_sequential_prefetch",
        }
    }
}

/// Feature state
#[derive(Debug, Clone)]
pub struct FeatureState {
    pub enabled: bool,
    pub hits: u64,
    pub misses: u64,
}

/// Feature statistics
#[derive(Debug, Clone, Default)]
pub struct FeatureFlagStats {
    pub total_checks: u64,
    pub enabled_hits: u64,
    pub total_toggles: u64,
}

/// Feature report
#[derive(Debug, Clone)]
pub struct FeatureReport {
    pub features: HashMap<String, FeatureState>,
    pub total_toggles: u64,
}

impl std::fmt::Display for FeatureReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Feature Flag Report ===")?;
        writeln!(f, "Total toggles: {}", self.total_toggles)?;
        for (name, state) in &self.features {
            let status = if state.enabled { "ON" } else { "OFF" };
            writeln!(f, "  {} [{}] hits={} misses={}", name, status, state.hits, state.misses)?;
        }
        Ok(())
    }
}

/// Feature state change callback
pub type FeatureCallback = Arc<dyn Fn(FeatureStateChange) + Send + Sync>;

#[derive(Debug, Clone)]
pub struct FeatureStateChange {
    pub feature: FeatureFlag,
    pub old_enabled: bool,
    pub new_enabled: bool,
}

/// Feature flag controller
pub struct FeatureFlagController {
    states: RwLock<HashMap<FeatureFlag, FeatureState>>,
    callbacks: RwLock<HashMap<usize, FeatureCallback>>,
    next_callback_id: AtomicUsize,
    toggle_count: AtomicU64,
    total_checks: AtomicU64,
    enabled_hits: AtomicU64,
}

impl FeatureFlagController {
    pub fn new() -> Self {
        let mut states = HashMap::new();
        // Default: all features enabled
        states.insert(FeatureFlag::Inno001AdaptiveBloomCache, FeatureState { enabled: true, hits: 0, misses: 0 });
        states.insert(FeatureFlag::Inno002ZoneMapPruning, FeatureState { enabled: true, hits: 0, misses: 0 });
        states.insert(FeatureFlag::Inno002SequentialPrefetch, FeatureState { enabled: true, hits: 0, misses: 0 });

        Self {
            states: RwLock::new(states),
            callbacks: RwLock::new(HashMap::new()),
            next_callback_id: AtomicUsize::new(0),
            toggle_count: AtomicU64::new(0),
            total_checks: AtomicU64::new(0),
            enabled_hits: AtomicU64::new(0),
        }
    }

    pub fn feature_count(&self) -> usize {
        self.states.read().len()
    }

    pub fn is_zone_map_pruning_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::Inno002ZoneMapPruning)
    }

    pub fn is_sequential_prefetch_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::Inno002SequentialPrefetch)
    }

    pub fn is_inno001_fully_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::Inno001AdaptiveBloomCache)
    }

    pub fn is_inno002_fully_enabled(&self) -> bool {
        self.is_enabled(FeatureFlag::Inno002ZoneMapPruning)
            && self.is_enabled(FeatureFlag::Inno002SequentialPrefetch)
    }

    pub fn is_enabled(&self, flag: FeatureFlag) -> bool {
        let states = self.states.read();
        let enabled = states.get(&flag).map(|s| s.enabled).unwrap_or(false);
        drop(states);

        self.total_checks.fetch_add(1, Ordering::Relaxed);
        if enabled {
            self.enabled_hits.fetch_add(1, Ordering::Relaxed);
        }
        enabled
    }

    pub fn set_enabled(&self, flag: FeatureFlag, enabled: bool) {
        let change = {
            let mut states = self.states.write();
            if let Some(state) = states.get_mut(&flag) {
                let old = state.enabled;
                if old == enabled {
                    return; // No change needed
                }
                state.enabled = enabled;
                self.toggle_count.fetch_add(1, Ordering::Relaxed);

                Some(FeatureStateChange {
                    feature: flag,
                    old_enabled: old,
                    new_enabled: enabled,
                })
            } else {
                None
            }
        };

        // Clone callbacks to release the lock before invoking potentially slow callbacks
        if let Some(change) = change {
            let callbacks: Vec<_> = self.callbacks.read().values().cloned().collect();
            for callback in callbacks {
                callback(change.clone());
            }
        }
    }

    pub fn enable_inno001(&self) {
        self.set_enabled(FeatureFlag::Inno001AdaptiveBloomCache, true);
    }

    pub fn disable_inno001(&self) {
        self.set_enabled(FeatureFlag::Inno001AdaptiveBloomCache, false);
    }

    pub fn enable_inno002(&self) {
        self.set_enabled(FeatureFlag::Inno002ZoneMapPruning, true);
        self.set_enabled(FeatureFlag::Inno002SequentialPrefetch, true);
    }

    pub fn disable_inno002(&self) {
        self.set_enabled(FeatureFlag::Inno002ZoneMapPruning, false);
        self.set_enabled(FeatureFlag::Inno002SequentialPrefetch, false);
    }

    pub fn get_stats(&self) -> FeatureFlagStats {
        FeatureFlagStats {
            total_checks: self.total_checks.load(Ordering::Relaxed),
            enabled_hits: self.enabled_hits.load(Ordering::Relaxed),
            total_toggles: self.toggle_count.load(Ordering::Relaxed),
        }
    }

    pub fn generate_report(&self) -> FeatureReport {
        let states = self.states.read();
        let mut features = HashMap::new();
        for (flag, state) in states.iter() {
            features.insert(flag.name().to_string(), state.clone());
        }
        FeatureReport {
            features,
            total_toggles: self.toggle_count.load(Ordering::Relaxed),
        }
    }

    pub fn register_callback(&self, callback: FeatureCallback) -> usize {
        let id = self.next_callback_id.fetch_add(1, Ordering::Relaxed);
        self.callbacks.write().insert(id, callback);
        id
    }

    /// Reset all feature flag states to their defaults and clear statistics.
    /// Use this in test cleanup to prevent cross-test pollution.
    pub fn reset(&self) {
        let mut states = self.states.write();
        states.insert(FeatureFlag::Inno001AdaptiveBloomCache, FeatureState { enabled: true, hits: 0, misses: 0 });
        states.insert(FeatureFlag::Inno002ZoneMapPruning, FeatureState { enabled: true, hits: 0, misses: 0 });
        states.insert(FeatureFlag::Inno002SequentialPrefetch, FeatureState { enabled: true, hits: 0, misses: 0 });
        self.toggle_count.store(0, Ordering::Relaxed);
        self.total_checks.store(0, Ordering::Relaxed);
        self.enabled_hits.store(0, Ordering::Relaxed);
        self.callbacks.write().clear();
        self.next_callback_id.store(0, Ordering::Relaxed);
    }
}

impl Default for FeatureFlagController {
    fn default() -> Self {
        Self::new()
    }
}

/// Global feature flag controller (optional)
static GLOBAL_CONTROLLER: std::sync::OnceLock<FeatureFlagController> = std::sync::OnceLock::new();

pub fn global_controller() -> &'static FeatureFlagController {
    GLOBAL_CONTROLLER.get_or_init(FeatureFlagController::new)
}

pub fn is_enabled(flag: FeatureFlag) -> bool {
    global_controller().is_enabled(flag)
}

pub fn set_enabled(flag: FeatureFlag, enabled: bool) {
    global_controller().set_enabled(flag, enabled);
}
