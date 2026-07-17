//! Query types - Core data structures for query representation
//!
//! This module defines the fundamental types used to represent queries:
//! - QueryOp: Query operations (scan, filter, join, etc.)
//! - QueryValue: Runtime values in queries
//! - QueryPredicate: Filter conditions
//! - SortOrder, AggregateFunction, JoinType, JoinCondition

use std::fmt;
use serde::{Deserialize, Serialize};

/// Query value types
#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum QueryValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<QueryValue>),
}

impl fmt::Display for QueryValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryValue::Null => write!(f, "NULL"),
            QueryValue::Bool(v) => write!(f, "{}", v),
            QueryValue::Int(v) => write!(f, "{}", v),
            QueryValue::Float(v) => write!(f, "{}", v),
            QueryValue::String(v) => write!(f, "'{}'", v),
            QueryValue::Bytes(v) => write!(f, "0x{}", hex::encode(v)),
            QueryValue::Array(v) => {
                let items: Vec<_> = v.iter().map(|x| x.to_string()).collect();
                write!(f, "[{}]", items.join(", "))
            }
        }
    }
}

/// Query predicates for filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryPredicate {
    /// Column = value
    Eq { column: String, value: QueryValue },
    /// Column != value
    Ne { column: String, value: QueryValue },
    /// Column < value
    Lt { column: String, value: QueryValue },
    /// Column <= value
    Le { column: String, value: QueryValue },
    /// Column > value
    Gt { column: String, value: QueryValue },
    /// Column >= value
    Ge { column: String, value: QueryValue },
    /// Column IN (values)
    In { column: String, values: Vec<QueryValue> },
    /// Column LIKE pattern
    Like { column: String, pattern: String },
    /// Column IS NULL
    IsNull { column: String },
    /// Column IS NOT NULL
    IsNotNull { column: String },
    /// AND of predicates
    And(Vec<QueryPredicate>),
    /// OR of predicates
    Or(Vec<QueryPredicate>),
    /// NOT of predicate
    Not(Box<QueryPredicate>),
    /// Custom expression
    Custom { expression: String },
}

impl fmt::Display for QueryPredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueryPredicate::Eq { column, value } => write!(f, "{} = {}", column, value),
            QueryPredicate::Ne { column, value } => write!(f, "{} != {}", column, value),
            QueryPredicate::Lt { column, value } => write!(f, "{} < {}", column, value),
            QueryPredicate::Le { column, value } => write!(f, "{} <= {}", column, value),
            QueryPredicate::Gt { column, value } => write!(f, "{} > {}", column, value),
            QueryPredicate::Ge { column, value } => write!(f, "{} >= {}", column, value),
            QueryPredicate::In { column, values } => {
                let vals: Vec<_> = values.iter().map(|v| v.to_string()).collect();
                write!(f, "{} IN ({})", column, vals.join(", "))
            }
            QueryPredicate::Like { column, pattern } => write!(f, "{} LIKE '{}'", column, pattern),
            QueryPredicate::IsNull { column } => write!(f, "{} IS NULL", column),
            QueryPredicate::IsNotNull { column } => write!(f, "{} IS NOT NULL", column),
            QueryPredicate::And(preds) => {
                let strs: Vec<_> = preds.iter().map(|p| p.to_string()).collect();
                write!(f, "({})", strs.join(" AND "))
            }
            QueryPredicate::Or(preds) => {
                let strs: Vec<_> = preds.iter().map(|p| p.to_string()).collect();
                write!(f, "({})", strs.join(" OR "))
            }
            QueryPredicate::Not(pred) => write!(f, "NOT ({})", pred),
            QueryPredicate::Custom { expression } => write!(f, "{}", expression),
        }
    }
}

/// Sort order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl fmt::Display for SortOrder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SortOrder::Asc => write!(f, "ASC"),
            SortOrder::Desc => write!(f, "DESC"),
        }
    }
}

/// Aggregate functions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count { alias: Option<String> },
    Sum { column: String, alias: Option<String> },
    Avg { column: String, alias: Option<String> },
    Min { column: String, alias: Option<String> },
    Max { column: String, alias: Option<String> },
    First { column: String, alias: Option<String> },
    Last { column: String, alias: Option<String> },
}

impl fmt::Display for AggregateFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AggregateFunction::Count { alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("count");
                write!(f, "COUNT(*) AS {}", name)
            }
            AggregateFunction::Sum { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("sum");
                write!(f, "SUM({}) AS {}", column, name)
            }
            AggregateFunction::Avg { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("avg");
                write!(f, "AVG({}) AS {}", column, name)
            }
            AggregateFunction::Min { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("min");
                write!(f, "MIN({}) AS {}", column, name)
            }
            AggregateFunction::Max { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("max");
                write!(f, "MAX({}) AS {}", column, name)
            }
            AggregateFunction::First { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("first");
                write!(f, "FIRST({}) AS {}", column, name)
            }
            AggregateFunction::Last { column, alias } => {
                let name = alias.as_ref().map(|s| s.as_str()).unwrap_or("last");
                write!(f, "LAST({}) AS {}", column, name)
            }
        }
    }
}

/// Join types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

impl fmt::Display for JoinType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JoinType::Inner => write!(f, "INNER JOIN"),
            JoinType::Left => write!(f, "LEFT JOIN"),
            JoinType::Right => write!(f, "RIGHT JOIN"),
            JoinType::Full => write!(f, "FULL JOIN"),
            JoinType::Cross => write!(f, "CROSS JOIN"),
        }
    }
}

/// Join condition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JoinCondition {
    /// ON left.col = right.col
    On {
        left_column: String,
        right_column: String,
    },
    /// USING (column)
    Using {
        column: String,
    },
    /// Natural join (implicit column matching)
    Natural,
    /// Custom condition
    Custom { expression: String },
}

/// Query operation types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum QueryOp {
    /// Full table scan
    Scan {
        table: String,
    },
    /// Index-based lookup
    IndexScan {
        table: String,
        index: String,
        key: QueryValue,
    },
    /// Range scan using index
    RangeScan {
        table: String,
        index: String,
        lower_bound: Option<QueryValue>,
        upper_bound: Option<QueryValue>,
        inclusive: (bool, bool),
    },
    /// Filter rows
    Filter {
        predicate: QueryPredicate,
    },
    /// Project specific columns
    Project {
        columns: Vec<String>,
    },
    /// Limit results
    Limit {
        count: usize,
    },
    /// Sort results
    Sort {
        columns: Vec<(String, SortOrder)>,
    },
    /// Aggregate functions
    Aggregate {
        functions: Vec<AggregateFunction>,
        group_by: Vec<String>,
    },
    /// Join two tables
    Join {
        join_type: JoinType,
        left_table: String,
        right_table: String,
        condition: JoinCondition,
    },
    /// Union of results
    Union {
        all: bool,
    },
    /// Distinct values
    Distinct,
}
