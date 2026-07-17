//! Query Optimizer - Cost-based query optimization and execution planning
//!
//! This module provides sophisticated query optimization capabilities including:
//! - Query parsing and analysis
//! - Cost-based optimization with statistics-driven decisions
//! - Multiple execution strategies (sequential, parallel, pipelined)
//! - Index selection and join ordering
//! - Query plan caching and reuse
//!
//! ## Architecture
//!
//! ```text
//! Query Optimizer
//! ├── Query Parser → Logical Plan
//! ├── Optimizer → Physical Plan (cost-based)
//! ├── Executor → Results
//! └── Plan Cache → Reuse optimized plans
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use tokitai_context::query_optimizer::{QueryOptimizer, Query, ExecutionStrategy};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let optimizer = QueryOptimizer::new();
//!
//! let query = Query::scan("users")
//!     .filter("age > 25")
//!     .project(&["name", "email"])
//!     .limit(100);
//!
//! let plan = optimizer.optimize(query)?;
//! let results = optimizer.execute(plan).await?;
//! # Ok(())
//! # }
//! ```

pub mod types;
pub mod plan;
pub mod cost;
pub mod query;
pub mod optimizer;
pub mod executor;

// Re-exports for backward compatibility
pub use types::{QueryOp, QueryValue, QueryPredicate, SortOrder, AggregateFunction, JoinType, JoinCondition};
pub use plan::{LogicalPlan, PhysicalPlan, PlanNode, PlanStatistics, SortAlgorithm, DistinctMethod};
pub use cost::{CostModel, TableStatistics, ColumnStatistics, IndexStatistics, Histogram, HistogramBucket};
pub use query::Query;
pub use optimizer::{OptimizerConfig, QueryOptimizer};
pub use executor::{QueryExecutor, QueryResult, QueryRow, ExecutionStats};

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;

    /// 保留：验证 Query Builder 基本构建逻辑
    #[test]
    fn test_query_builder_scan() {
        let query = Query::scan("users");
        let plan = query.build();

        assert_eq!(plan.tables.len(), 1);
        assert!(plan.tables.contains("users"));
        assert_eq!(plan.operations.len(), 1);
    }

    /// 保留：验证 filter 操作添加
    #[test]
    fn test_query_builder_filter() {
        let query = Query::scan("users")
            .filter_eq("age", QueryValue::Int(25));

        let plan = query.build();
        assert_eq!(plan.operations.len(), 2);
    }

    /// 保留：验证链式调用构建完整查询
    #[test]
    fn test_query_builder_chain() {
        let query = Query::scan("users")
            .filter_eq("age", QueryValue::Int(25))
            .project(&["name", "email"])
            .limit(100);

        let plan = query.build();
        assert_eq!(plan.operations.len(), 4);
        assert_eq!(plan.tables.len(), 1);
    }

    /// 保留：验证 optimizer 配置（使用公共方法验证）
    #[test]
    fn test_optimizer_with_config() {
        let config = OptimizerConfig {
            cost_based_optimization: false,
            predicate_pushdown: true,
            projection_pushdown: false,
            join_reordering: true,
            parallel_execution: true,
            plan_caching: false,
            max_plans_explored: 500,
            cost_threshold: 5000.0,
        };
        let optimizer = QueryOptimizer::with_config(config);
        // Note: Can't access config directly due to privacy, but we can test behavior
        assert_eq!(optimizer.cache_size(), 0);
    }

    /// 保留：验证 CostModel 默认值
    #[test]
    fn test_cost_model_default() {
        let cost_model = CostModel::default();
        assert_eq!(cost_model.seq_page_cost, 1.0);
        assert_eq!(cost_model.random_page_cost, 4.0);
        assert_eq!(cost_model.cpu_tuple_cost, 0.01);
    }

    /// 保留：验证 QueryValue 显示
    #[test]
    fn test_query_value_display() {
        assert_eq!(format!("{}", QueryValue::Int(42)), "42");
        assert_eq!(format!("{}", QueryValue::String("hello".to_string())), "'hello'");
        assert_eq!(format!("{}", QueryValue::Bool(true)), "true");
    }

    /// 保留：验证 QueryPredicate 显示
    #[test]
    fn test_query_predicate_display() {
        let pred = QueryPredicate::Eq {
            column: "age".to_string(),
            value: QueryValue::Int(25),
        };
        assert_eq!(format!("{}", pred), "age = 25");
    }

    /// 保留：验证 LogicalPlan 创建
    #[test]
    fn test_logical_plan_new() {
        let plan = LogicalPlan::new();
        assert_eq!(plan.operations.len(), 0);
        assert_eq!(plan.tables.len(), 0);
    }

    /// 保留：验证 LogicalPlan with_operations
    #[test]
    fn test_logical_plan_with_operations() {
        let ops = vec![QueryOp::Scan { table: "users".to_string() }];
        let plan = LogicalPlan::with_operations(ops);
        assert_eq!(plan.operations.len(), 1);
        assert!(plan.tables.contains("users"));
    }

    /// 保留：验证 QueryOptimizer 创建
    #[test]
    fn test_query_optimizer_new() {
        let optimizer = QueryOptimizer::new();
        // Note: Can't access config directly due to privacy
        assert_eq!(optimizer.cache_size(), 0);
    }

    /// 保留：验证 PlanStatistics 默认值
    #[test]
    fn test_plan_statistics_default() {
        let stats = PlanStatistics::default();
        assert_eq!(stats.estimated_rows, 0);
        assert_eq!(stats.estimated_cost, 0.0);
        assert_eq!(stats.estimated_memory_bytes, 0);
    }

    /// 保留：验证 SortOrder 显示
    #[test]
    fn test_sort_order_display() {
        assert_eq!(format!("{}", SortOrder::Asc), "ASC");
        assert_eq!(format!("{}", SortOrder::Desc), "DESC");
    }

    /// 保留：验证 JoinType 显示
    #[test]
    fn test_join_type_display() {
        assert_eq!(format!("{}", JoinType::Inner), "INNER JOIN");
        assert_eq!(format!("{}", JoinType::Left), "LEFT JOIN");
    }

    /// 保留：验证 AggregateFunction 显示
    #[test]
    fn test_aggregate_function_display() {
        let func = AggregateFunction::Count { alias: None };
        assert_eq!(format!("{}", func), "COUNT(*) AS count");
    }

    /// 保留：验证 QueryOptimizer 缓存操作
    #[test]
    fn test_query_optimizer_cache() {
        let optimizer = QueryOptimizer::new();
        assert_eq!(optimizer.cache_size(), 0);
        optimizer.clear_cache();
        assert_eq!(optimizer.cache_size(), 0);
    }

    /// 保留：验证 QueryRow 基本操作
    #[test]
    fn test_query_row() {
        let row = QueryRow::new(vec![QueryValue::Int(1), QueryValue::String("test".to_string())]);
        assert_eq!(row.values.len(), 2);
        assert_eq!(row.get(0), Some(&QueryValue::Int(1)));
        assert_eq!(row.get(1), Some(&QueryValue::String("test".to_string())));
        assert_eq!(row.get(2), None);
    }

    /// 保留：验证 ExecutionStats 默认值
    #[test]
    fn test_execution_stats_default() {
        let stats = ExecutionStats::default();
        assert_eq!(stats.total_queries, 0);
        assert_eq!(stats.successful_queries, 0);
        assert_eq!(stats.failed_queries, 0);
    }

    /// 保留：验证 QueryExecutor 创建
    #[test]
    fn test_query_executor_new() {
        let optimizer = Arc::new(QueryOptimizer::new());
        let executor = QueryExecutor::new(optimizer);
        let stats = executor.get_stats();
        assert_eq!(stats.total_queries, 0);
    }

    /// 保留：验证 TableStatistics 默认值
    #[test]
    fn test_table_statistics_default() {
        let stats = TableStatistics::default();
        assert_eq!(stats.row_count, 0);
        assert_eq!(stats.page_count, 0);
        assert_eq!(stats.avg_row_size, 0);
    }

    /// 保留：验证 ColumnStatistics 默认值
    #[test]
    fn test_column_statistics_default() {
        let stats = ColumnStatistics::default();
        assert_eq!(stats.distinct_count, 0);
        assert_eq!(stats.null_count, 0);
        assert!(stats.most_common_values.is_empty());
    }

    /// 保留：验证 IndexStatistics 默认值
    #[test]
    fn test_index_statistics_default() {
        let stats = IndexStatistics::default();
        assert_eq!(stats.name, String::new());
        assert_eq!(stats.columns.len(), 0);
        assert!(!stats.is_unique);
    }

    /// 保留：验证 Histogram 创建
    #[test]
    fn test_histogram() {
        let histogram = Histogram {
            buckets: vec![],
        };
        assert!(histogram.buckets.is_empty());
    }

    /// 保留：验证 HistogramBucket 创建
    #[test]
    fn test_histogram_bucket() {
        let bucket = HistogramBucket {
            upper_bound: QueryValue::Int(100),
            cumulative_count: 50,
        };
        assert_eq!(bucket.upper_bound, QueryValue::Int(100));
        assert_eq!(bucket.cumulative_count, 50);
    }

    /// 保留：验证 SortAlgorithm 变体
    #[test]
    fn test_sort_algorithm() {
        let quick_sort = SortAlgorithm::QuickSort;
        let external_sort = SortAlgorithm::ExternalMergeSort { chunk_size: 1024 };
        let topk_heap = SortAlgorithm::TopKHeap { k: 10 };
        
        match quick_sort {
            SortAlgorithm::QuickSort => {},
            _ => panic!("Expected QuickSort"),
        }
        match external_sort {
            SortAlgorithm::ExternalMergeSort { chunk_size } => assert_eq!(chunk_size, 1024),
            _ => panic!("Expected ExternalMergeSort"),
        }
        match topk_heap {
            SortAlgorithm::TopKHeap { k } => assert_eq!(k, 10),
            _ => panic!("Expected TopKHeap"),
        }
    }

    /// 保留：验证 DistinctMethod 变体
    #[test]
    fn test_distinct_method() {
        let hash = DistinctMethod::Hash;
        let sort = DistinctMethod::Sort;
        let bloom = DistinctMethod::BloomFilter { false_positive_rate: 0.01 };
        
        match hash {
            DistinctMethod::Hash => {},
            _ => panic!("Expected Hash"),
        }
        match sort {
            DistinctMethod::Sort => {},
            _ => panic!("Expected Sort"),
        }
        match bloom {
            DistinctMethod::BloomFilter { false_positive_rate } => {
                assert_eq!(false_positive_rate, 0.01);
            },
            _ => panic!("Expected BloomFilter"),
        }
    }

    /// 保留：验证 QueryValue Array 变体
    #[test]
    fn test_query_value_array() {
        let arr = QueryValue::Array(vec![
            QueryValue::Int(1),
            QueryValue::Int(2),
            QueryValue::Int(3),
        ]);
        assert_eq!(format!("{}", arr), "[1, 2, 3]");
    }

    /// 保留：验证 QueryPredicate 组合
    #[test]
    fn test_query_predicate_combination() {
        let pred1 = QueryPredicate::Eq {
            column: "age".to_string(),
            value: QueryValue::Int(25),
        };
        let pred2 = QueryPredicate::Gt {
            column: "score".to_string(),
            value: QueryValue::Int(80),
        };
        let combined = QueryPredicate::And(vec![pred1, pred2]);
        assert!(format!("{}", combined).contains("AND"));
    }

    /// 保留：验证 QueryPredicate 复杂组合
    #[test]
    fn test_query_predicate_or() {
        let pred1 = QueryPredicate::Eq {
            column: "status".to_string(),
            value: QueryValue::String("active".to_string()),
        };
        let pred2 = QueryPredicate::Eq {
            column: "status".to_string(),
            value: QueryValue::String("pending".to_string()),
        };
        let combined = QueryPredicate::Or(vec![pred1, pred2]);
        assert!(format!("{}", combined).contains("OR"));
    }

    /// 保留：验证 QueryPredicate Not
    #[test]
    fn test_query_predicate_not() {
        let pred = QueryPredicate::Eq {
            column: "deleted".to_string(),
            value: QueryValue::Bool(true),
        };
        let not_pred = QueryPredicate::Not(Box::new(pred));
        assert!(format!("{}", not_pred).contains("NOT"));
    }

    /// 保留：验证 QueryPredicate IsNull
    #[test]
    fn test_query_predicate_is_null() {
        let pred = QueryPredicate::IsNull {
            column: "email".to_string(),
        };
        assert_eq!(format!("{}", pred), "email IS NULL");
    }

    /// 保留：验证 QueryPredicate IsNotNull
    #[test]
    fn test_query_predicate_is_not_null() {
        let pred = QueryPredicate::IsNotNull {
            column: "name".to_string(),
        };
        assert_eq!(format!("{}", pred), "name IS NOT NULL");
    }

    /// 保留：验证 QueryPredicate In
    #[test]
    fn test_query_predicate_in() {
        let pred = QueryPredicate::In {
            column: "status".to_string(),
            values: vec![
                QueryValue::String("active".to_string()),
                QueryValue::String("pending".to_string()),
            ],
        };
        assert!(format!("{}", pred).contains("IN"));
    }

    /// 保留：验证 QueryPredicate Like
    #[test]
    fn test_query_predicate_like() {
        let pred = QueryPredicate::Like {
            column: "name".to_string(),
            pattern: "%test%".to_string(),
        };
        assert_eq!(format!("{}", pred), "name LIKE '%test%'");
    }

    /// 保留：验证 QueryPredicate 比较操作符
    #[test]
    fn test_query_predicate_comparisons() {
        let col = "value".to_string();
        let val = QueryValue::Int(10);
        
        assert_eq!(format!("{}", QueryPredicate::Eq { column: col.clone(), value: val.clone() }), "value = 10");
        assert_eq!(format!("{}", QueryPredicate::Ne { column: col.clone(), value: val.clone() }), "value != 10");
        assert_eq!(format!("{}", QueryPredicate::Lt { column: col.clone(), value: val.clone() }), "value < 10");
        assert_eq!(format!("{}", QueryPredicate::Le { column: col.clone(), value: val.clone() }), "value <= 10");
        assert_eq!(format!("{}", QueryPredicate::Gt { column: col.clone(), value: val.clone() }), "value > 10");
        assert_eq!(format!("{}", QueryPredicate::Ge { column: col.clone(), value: val.clone() }), "value >= 10");
    }

    /// 保留：验证 AggregateFunction 各种类型
    #[test]
    fn test_aggregate_functions() {
        let sum = AggregateFunction::Sum { column: "amount".to_string(), alias: Some("total".to_string()) };
        assert_eq!(format!("{}", sum), "SUM(amount) AS total");
        
        let avg = AggregateFunction::Avg { column: "price".to_string(), alias: None };
        assert_eq!(format!("{}", avg), "AVG(price) AS avg");
        
        let min = AggregateFunction::Min { column: "score".to_string(), alias: Some("min_score".to_string()) };
        assert_eq!(format!("{}", min), "MIN(score) AS min_score");
        
        let max = AggregateFunction::Max { column: "salary".to_string(), alias: None };
        assert_eq!(format!("{}", max), "MAX(salary) AS max");
    }

    /// 保留：验证 JoinCondition 各种类型
    #[test]
    fn test_join_conditions() {
        let on = JoinCondition::On {
            left_column: "user_id".to_string(),
            right_column: "id".to_string(),
        };
        assert!(format!("{:?}", on).contains("On"));
        
        let using = JoinCondition::Using {
            column: "id".to_string(),
        };
        assert!(format!("{:?}", using).contains("Using"));
        
        let natural = JoinCondition::Natural;
        assert!(format!("{:?}", natural).contains("Natural"));
        
        let custom = JoinCondition::Custom {
            expression: "complex condition".to_string(),
        };
        assert!(format!("{:?}", custom).contains("Custom"));
    }

    /// 保留：验证所有 JoinType 变体
    #[test]
    fn test_all_join_types() {
        assert_eq!(format!("{}", JoinType::Inner), "INNER JOIN");
        assert_eq!(format!("{}", JoinType::Left), "LEFT JOIN");
        assert_eq!(format!("{}", JoinType::Right), "RIGHT JOIN");
        assert_eq!(format!("{}", JoinType::Full), "FULL JOIN");
        assert_eq!(format!("{}", JoinType::Cross), "CROSS JOIN");
    }

    /// 保留：验证 QueryValue Bytes 编码
    #[test]
    fn test_query_value_bytes() {
        let bytes = QueryValue::Bytes(vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
        assert!(format!("{}", bytes).starts_with("0x"));
    }

    /// 保留：验证 QueryValue Null
    #[test]
    fn test_query_value_null() {
        assert_eq!(format!("{}", QueryValue::Null), "NULL");
        assert_eq!(QueryValue::Null, QueryValue::Null);
    }

    /// 保留：验证 QueryValue 部分排序
    #[test]
    fn test_query_value_partial_ord() {
        assert!(QueryValue::Int(5) > QueryValue::Int(3));
        assert!(QueryValue::Float(std::f64::consts::PI) > QueryValue::Float(std::f64::consts::E));
    }

    /// 保留：验证 Query 构建器范围过滤
    #[test]
    fn test_query_filter_range() {
        let query = Query::scan("users")
            .filter_range("age", Some(QueryValue::Int(18)), Some(QueryValue::Int(65)));
        let plan = query.build();
        assert_eq!(plan.operations.len(), 2);
    }

    /// 保留：验证 Query 构建器排序
    #[test]
    fn test_query_order_by() {
        let query = Query::scan("users")
            .order_by("name", SortOrder::Asc);
        let plan = query.build();
        assert_eq!(plan.operations.len(), 2);
    }

    /// 保留：验证 Query 构建器聚合
    #[test]
    fn test_query_aggregate() {
        let query = Query::scan("sales")
            .aggregate(
                vec![AggregateFunction::Sum { column: "amount".to_string(), alias: None }],
                vec!["region".to_string()],
            );
        let plan = query.build();
        assert_eq!(plan.operations.len(), 2);
    }

    /// 保留：验证 Query 构建器索引扫描
    #[test]
    fn test_query_index_scan() {
        let query = Query::index_scan("users", "idx_email", QueryValue::String("test@example.com".to_string()));
        let plan = query.build();
        assert_eq!(plan.tables.len(), 1);
        assert!(plan.tables.contains("users"));
    }

    /// 保留：验证 OptimizerConfig 所有字段
    #[test]
    fn test_optimizer_config_all_fields() {
        let config = OptimizerConfig {
            cost_based_optimization: true,
            predicate_pushdown: false,
            projection_pushdown: true,
            join_reordering: false,
            parallel_execution: true,
            plan_caching: false,
            max_plans_explored: 100,
            cost_threshold: 1000.0,
        };
        assert!(config.cost_based_optimization);
        assert!(!config.predicate_pushdown);
        assert!(config.projection_pushdown);
        assert!(!config.join_reordering);
        assert!(config.parallel_execution);
        assert!(!config.plan_caching);
        assert_eq!(config.max_plans_explored, 100);
        assert_eq!(config.cost_threshold, 1000.0);
    }

    /// 保留：验证 QueryOptimizer 注册和获取表统计
    #[test]
    fn test_query_optimizer_table_stats() {
        let optimizer = QueryOptimizer::new();
        let stats = TableStatistics {
            row_count: 1000,
            page_count: 100,
            avg_row_size: 128,
            column_stats: HashMap::new(),
            index_stats: HashMap::new(),
        };
        optimizer.register_table_stats("users", stats.clone());
        
        let retrieved = optimizer.get_table_stats("users");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().row_count, 1000);
        
        // Non-existent table
        let missing = optimizer.get_table_stats("nonexistent");
        assert!(missing.is_none());
    }
}
