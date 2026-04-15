//! CacheBudget: Informational memory budget tracking
//!
//! Tracks budget allocations for BlockCache and BloomFilterCache.
//! This is informational only - the caches enforce their own memory limits
//! via max_memory_bytes/max_items configuration.

/// Sub-budget for a single cache category
#[derive(Debug)]
pub struct SubBudget {
    /// Maximum allowed bytes
    max: u64,
}

impl SubBudget {
    fn new(max: u64) -> Self {
        Self { max }
    }

    /// Current max budget
    pub fn max_budget(&self) -> u64 {
        self.max
    }
}

/// Full usage report
#[derive(Debug, Clone)]
pub struct CacheUsageReport {
    pub total_budget: u64,
    pub total_used: u64,
    pub usage_percent: f64,
    pub block_cache_used: u64,
    pub block_cache_max: u64,
    pub block_cache_hit_rate: f64,
    pub bloom_filter_used: u64,
    pub bloom_filter_max: u64,
    pub bloom_filter_hit_rate: f64,
}

impl std::fmt::Display for CacheUsageReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Cache Usage Report:")?;
        writeln!(f, "  Total: {:.1}MB / {:.1}MB ({:.1}%)",
            self.total_used as f64 / 1024.0 / 1024.0,
            self.total_budget as f64 / 1024.0 / 1024.0,
            self.usage_percent)?;
        writeln!(f, "  BlockCache: {:.1}MB / {:.1}MB (hit rate: {:.1}%)",
            self.block_cache_used as f64 / 1024.0 / 1024.0,
            self.block_cache_max as f64 / 1024.0 / 1024.0,
            self.block_cache_hit_rate * 100.0)?;
        writeln!(f, "  BloomFilter: {:.1}MB / {:.1}MB (hit rate: {:.1}%)",
            self.bloom_filter_used as f64 / 1024.0 / 1024.0,
            self.bloom_filter_max as f64 / 1024.0 / 1024.0,
            self.bloom_filter_hit_rate * 100.0)
    }
}

/// Global cache budget tracker (informational only)
pub struct CacheBudget {
    pub max_bytes: u64,
    pub block_cache: SubBudget,
    pub bloom_filter: SubBudget,
}

impl CacheBudget {
    /// Create a new cache budget with percentage-based sub-budgets
    pub fn new(max_bytes: u64, block_pct: f64, bloom_pct: f64) -> Self {
        let block_max = (max_bytes as f64 * block_pct) as u64;
        let bloom_max = (max_bytes as f64 * bloom_pct) as u64;

        Self {
            max_bytes,
            block_cache: SubBudget::new(block_max),
            bloom_filter: SubBudget::new(bloom_max),
        }
    }
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_allocation() {
        let budget = CacheBudget::new(1000, 0.6, 0.25);
        assert_eq!(budget.block_cache.max_budget(), 600);
        assert_eq!(budget.bloom_filter.max_budget(), 250);
    }

    #[test]
    fn test_usage_report() {
        let budget = CacheBudget::new(1_000_000, 0.6, 0.25);

        let report = CacheUsageReport {
            total_budget: budget.max_bytes,
            total_used: 100_000,
            usage_percent: 0.1,
            block_cache_used: 60_000,
            block_cache_max: budget.block_cache.max_budget(),
            block_cache_hit_rate: 0.0,
            bloom_filter_used: 25_000,
            bloom_filter_max: budget.bloom_filter.max_budget(),
            bloom_filter_hit_rate: 0.0,
        };
        assert_eq!(report.total_budget, 1_000_000);
        assert_eq!(report.block_cache_max, 600_000);
    }
}
