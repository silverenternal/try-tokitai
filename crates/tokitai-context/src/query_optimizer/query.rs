//! Query Builder - Fluent query construction API
//!
//! This module provides a fluent interface for building queries:
//! - Query: Builder for constructing logical plans

use std::collections::HashSet;

use crate::query_optimizer::types::{QueryOp, QueryValue, QueryPredicate, SortOrder, AggregateFunction};
use crate::query_optimizer::plan::LogicalPlan;

/// Fluent query builder
#[derive(Debug, Clone)]
pub struct Query {
    operations: Vec<QueryOp>,
    tables: HashSet<String>,
}

impl Query {
    /// Create a scan query
    pub fn scan(table: &str) -> Self {
        Self {
            operations: vec![QueryOp::Scan { table: table.to_string() }],
            tables: HashSet::from([table.to_string()]),
        }
    }

    /// Create an index scan query
    pub fn index_scan(table: &str, index: &str, key: QueryValue) -> Self {
        Self {
            operations: vec![QueryOp::IndexScan {
                table: table.to_string(),
                index: index.to_string(),
                key,
            }],
            tables: HashSet::from([table.to_string()]),
        }
    }

    /// Add filter predicate
    pub fn filter(mut self, predicate: QueryPredicate) -> Self {
        self.operations.push(QueryOp::Filter { predicate });
        self
    }

    /// Add equality filter
    pub fn filter_eq(mut self, column: &str, value: QueryValue) -> Self {
        self.operations.push(QueryOp::Filter {
            predicate: QueryPredicate::Eq {
                column: column.to_string(),
                value,
            },
        });
        self
    }

    /// Add range filter
    pub fn filter_range(
        mut self,
        column: &str,
        min: Option<QueryValue>,
        max: Option<QueryValue>,
    ) -> Self {
        let mut predicates = Vec::new();
        if let Some(min_val) = min {
            predicates.push(QueryPredicate::Gt {
                column: column.to_string(),
                value: min_val,
            });
        }
        if let Some(max_val) = max {
            predicates.push(QueryPredicate::Lt {
                column: column.to_string(),
                value: max_val,
            });
        }
        if predicates.len() == 1 {
            self.operations.push(QueryOp::Filter {
                predicate: predicates.remove(0),
            });
        } else if predicates.len() == 2 {
            self.operations.push(QueryOp::Filter {
                predicate: QueryPredicate::And(predicates),
            });
        }
        self
    }

    /// Add projection
    pub fn project(mut self, columns: &[&str]) -> Self {
        self.operations.push(QueryOp::Project {
            columns: columns.iter().map(|s| s.to_string()).collect(),
        });
        self
    }

    /// Add limit
    pub fn limit(mut self, count: usize) -> Self {
        self.operations.push(QueryOp::Limit { count });
        self
    }

    /// Add order by
    pub fn order_by(mut self, column: &str, order: SortOrder) -> Self {
        self.operations.push(QueryOp::Sort {
            columns: vec![(column.to_string(), order)],
        });
        self
    }

    /// Add aggregation
    pub fn aggregate(mut self, functions: Vec<AggregateFunction>, group_by: Vec<String>) -> Self {
        self.operations.push(QueryOp::Aggregate {
            functions,
            group_by,
        });
        self
    }

    /// Build the logical plan
    pub fn build(self) -> LogicalPlan {
        LogicalPlan::with_operations(self.operations)
    }
}
