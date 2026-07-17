//! Cost Model - Statistics-driven cost estimation
//!
//! This module provides cost modeling for query optimization:
//! - CostModel: Cost parameters
//! - TableStatistics, ColumnStatistics, IndexStatistics

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::query_optimizer::types::QueryValue;

/// Cost model parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModel {
    /// Cost per sequential page read
    pub seq_page_cost: f64,
    /// Cost per random page read
    pub random_page_cost: f64,
    /// Cost per tuple (row) processing
    pub cpu_tuple_cost: f64,
    /// Cost per index tuple processing
    pub cpu_index_tuple_cost: f64,
    /// Cost per operator evaluation
    pub cpu_operator_cost: f64,
    /// Memory size in bytes (for work_mem)
    pub memory_size_bytes: usize,
    /// Default page size in bytes
    pub page_size: usize,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            seq_page_cost: 1.0,
            random_page_cost: 4.0,
            cpu_tuple_cost: 0.01,
            cpu_index_tuple_cost: 0.005,
            cpu_operator_cost: 0.0025,
            memory_size_bytes: 4 * 1024 * 1024, // 4MB
            page_size: 8192,
        }
    }
}

/// Table statistics for cost estimation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TableStatistics {
    /// Total number of rows
    pub row_count: usize,
    /// Number of pages
    pub page_count: usize,
    /// Average row size in bytes
    pub avg_row_size: usize,
    /// Column statistics
    pub column_stats: HashMap<String, ColumnStatistics>,
    /// Index statistics
    pub index_stats: HashMap<String, IndexStatistics>,
}

/// Column statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColumnStatistics {
    /// Number of distinct values
    pub distinct_count: usize,
    /// Number of null values
    pub null_count: usize,
    /// Most common values
    pub most_common_values: Vec<(QueryValue, f64)>, // (value, frequency)
    /// Histogram for range queries
    pub histogram: Option<Histogram>,
    /// Minimum value
    pub min_value: Option<QueryValue>,
    /// Maximum value
    pub max_value: Option<QueryValue>,
}

/// Histogram for range estimation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    pub buckets: Vec<HistogramBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    pub upper_bound: QueryValue,
    pub cumulative_count: usize,
}

/// Index statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexStatistics {
    /// Index name
    pub name: String,
    /// Indexed columns
    pub columns: Vec<String>,
    /// Is unique index
    pub is_unique: bool,
    /// Number of entries
    pub entry_count: usize,
    /// Index depth (B-tree levels)
    pub depth: usize,
    /// Leaf pages
    pub leaf_pages: usize,
}
