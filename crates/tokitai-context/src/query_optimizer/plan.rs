//! Query Plan - Logical and physical plan representations
//!
//! This module defines the query plan structures:
//! - LogicalPlan: Abstract query representation
//! - PhysicalPlan: Optimized execution plan
//! - PlanNode: Execution tree nodes

use std::collections::HashSet;
use serde::{Deserialize, Serialize};

use crate::query_optimizer::types::{QueryOp, QueryPredicate, QueryValue, SortOrder, AggregateFunction, JoinType, JoinCondition};

/// Sort algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortAlgorithm {
    /// In-memory quicksort
    QuickSort,
    /// External merge sort for large datasets
    ExternalMergeSort { chunk_size: usize },
    /// Top-k heap for LIMIT + ORDER BY
    TopKHeap { k: usize },
}

/// Distinct methods
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum DistinctMethod {
    /// Hash-based deduplication
    Hash,
    /// Sort-based deduplication
    Sort,
    /// Bloom filter for approximate distinct
    BloomFilter { false_positive_rate: f64 },
}

/// Plan statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanStatistics {
    /// Estimated number of rows
    pub estimated_rows: usize,
    /// Estimated cost (arbitrary units)
    pub estimated_cost: f64,
    /// Estimated memory usage in bytes
    pub estimated_memory_bytes: usize,
    /// Estimated I/O operations
    pub estimated_io_ops: usize,
    /// Estimated CPU cycles
    pub estimated_cpu_cycles: u64,
    /// Parallelism degree
    pub parallelism_degree: usize,
}

/// Logical query plan (before optimization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogicalPlan {
    pub operations: Vec<QueryOp>,
    pub tables: HashSet<String>,
    pub estimated_rows: Option<usize>,
}

impl LogicalPlan {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            tables: HashSet::new(),
            estimated_rows: None,
        }
    }

    pub fn with_operations(ops: Vec<QueryOp>) -> Self {
        let tables = ops
            .iter()
            .filter_map(|op| match op {
                QueryOp::Scan { table } => Some(table.clone()),
                QueryOp::IndexScan { table, .. } => Some(table.clone()),
                QueryOp::RangeScan { table, .. } => Some(table.clone()),
                QueryOp::Join { left_table, right_table: _, .. } => {
                    Some(left_table.clone()) // Add left, right added separately
                }
                _ => None,
            })
            .collect();
        Self {
            operations: ops,
            tables,
            estimated_rows: None,
        }
    }
}

impl Default for LogicalPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Physical query plan (after optimization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalPlan {
    pub root: PlanNode,
    pub estimated_cost: f64,
    pub estimated_rows: usize,
    pub optimization_rules: Vec<String>,
    pub statistics: PlanStatistics,
}

/// Plan node in the execution tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanNode {
    /// Scan data from storage
    Scan {
        table: String,
        projection: Vec<String>,
        filter: Option<QueryPredicate>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Scan using index
    IndexScan {
        table: String,
        index: String,
        key: QueryValue,
        projection: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Range scan using index
    RangeScan {
        table: String,
        index: String,
        lower_bound: Option<QueryValue>,
        upper_bound: Option<QueryValue>,
        inclusive: (bool, bool),
        projection: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Filter rows
    Filter {
        input: Box<PlanNode>,
        predicate: QueryPredicate,
        selectivity: f64,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Project columns
    Project {
        input: Box<PlanNode>,
        columns: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Limit results
    Limit {
        input: Box<PlanNode>,
        count: usize,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Sort results
    Sort {
        input: Box<PlanNode>,
        columns: Vec<(String, SortOrder)>,
        algorithm: SortAlgorithm,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Aggregate functions
    Aggregate {
        input: Box<PlanNode>,
        functions: Vec<AggregateFunction>,
        group_by: Vec<String>,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Hash join
    HashJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        left_key: String,
        right_key: String,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Nested loop join
    NestedLoopJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        condition: QueryPredicate,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Merge join (for sorted inputs)
    MergeJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        join_type: JoinType,
        left_key: String,
        right_key: String,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Union results
    Union {
        inputs: Vec<PlanNode>,
        all: bool,
        estimated_rows: usize,
        estimated_cost: f64,
    },
    /// Distinct values
    Distinct {
        input: Box<PlanNode>,
        method: DistinctMethod,
        estimated_rows: usize,
        estimated_cost: f64,
    },
}
