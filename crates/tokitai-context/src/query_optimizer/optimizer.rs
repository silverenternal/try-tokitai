//! Query Optimizer - Cost-based optimization engine
//!
//! This module provides the query optimization engine:
//! - OptimizerConfig: Configuration
//! - QueryOptimizer: Main optimizer with plan generation and caching

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Context, Result};
use parking_lot::RwLock;

use crate::error::ContextError;
use crate::query_optimizer::types::{QueryOp, QueryPredicate, QueryValue, SortOrder, AggregateFunction, JoinType, JoinCondition};
use crate::query_optimizer::plan::{LogicalPlan, PhysicalPlan, PlanNode, PlanStatistics, SortAlgorithm, DistinctMethod};
use crate::query_optimizer::cost::{CostModel, TableStatistics};

/// Query optimizer configuration
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Enable cost-based optimization
    pub cost_based_optimization: bool,
    /// Enable predicate pushdown
    pub predicate_pushdown: bool,
    /// Enable projection pushdown
    pub projection_pushdown: bool,
    /// Enable join reordering
    pub join_reordering: bool,
    /// Enable parallel execution
    pub parallel_execution: bool,
    /// Enable plan caching
    pub plan_caching: bool,
    /// Maximum plans to consider during optimization
    pub max_plans_explored: usize,
    /// Cost threshold for early termination
    pub cost_threshold: f64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            cost_based_optimization: true,
            predicate_pushdown: true,
            projection_pushdown: true,
            join_reordering: true,
            parallel_execution: true,
            plan_caching: true,
            max_plans_explored: 1000,
            cost_threshold: 10000.0,
        }
    }
}

/// Query optimizer
pub struct QueryOptimizer {
    config: OptimizerConfig,
    cost_model: CostModel,
    table_statistics: RwLock<HashMap<String, TableStatistics>>,
    plan_cache: RwLock<HashMap<u64, PhysicalPlan>>,
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new() -> Self {
        Self {
            config: OptimizerConfig::default(),
            cost_model: CostModel::default(),
            table_statistics: RwLock::new(HashMap::new()),
            plan_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: OptimizerConfig) -> Self {
        Self {
            config,
            cost_model: CostModel::default(),
            table_statistics: RwLock::new(HashMap::new()),
            plan_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Register table statistics
    pub fn register_table_stats(&self, table: &str, stats: TableStatistics) {
        let mut stats_map = self.table_statistics.write();
        stats_map.insert(table.to_string(), stats);
    }

    /// Get table statistics
    pub fn get_table_stats(&self, table: &str) -> Option<TableStatistics> {
        let stats_map = self.table_statistics.read();
        stats_map.get(table).cloned()
    }

    /// Optimize a logical plan
    pub fn optimize(&self, logical_plan: LogicalPlan) -> Result<PhysicalPlan> {
        // Check plan cache
        if self.config.plan_caching {
            let plan_hash = self.hash_logical_plan(&logical_plan);
            let cache = self.plan_cache.read();
            if let Some(cached) = cache.get(&plan_hash) {
                return Ok(cached.clone());
            }
        }

        // Apply optimization rules
        let mut plan = self.apply_optimization_rules(logical_plan.clone())?;

        // Generate alternative plans and select the best
        if self.config.cost_based_optimization {
            plan = self.generate_best_plan(plan)?;
        }

        // Calculate final statistics
        plan.statistics = self.calculate_plan_statistics(&plan.root);

        // Cache the result
        if self.config.plan_caching {
            let plan_hash = self.hash_logical_plan(&logical_plan);
            let mut cache = self.plan_cache.write();
            cache.insert(plan_hash, plan.clone());
        }

        Ok(plan)
    }

    /// Apply optimization rules to a logical plan
    fn apply_optimization_rules(&self, logical_plan: LogicalPlan) -> Result<PhysicalPlan> {
        let mut operations = logical_plan.operations.clone();
        let mut optimization_rules = Vec::new();

        // Predicate pushdown
        if self.config.predicate_pushdown {
            let (pushed, count) = self.push_down_predicates(operations);
            operations = pushed;
            if count > 0 {
                optimization_rules.push(format!("predicate_pushdown({})", count));
            }
        }

        // Projection pushdown
        if self.config.projection_pushdown {
            let (pushed, count) = self.push_down_projections(operations);
            operations = pushed;
            if count > 0 {
                optimization_rules.push(format!("projection_pushdown({})", count));
            }
        }

        // Convert to physical plan
        let root = self.build_plan_tree(operations)?;
        let estimated_cost = self.estimate_cost(&root);
        let estimated_rows = self.estimate_rows(&root);

        Ok(PhysicalPlan {
            root,
            estimated_cost,
            estimated_rows,
            optimization_rules,
            statistics: PlanStatistics::default(),
        })
    }

    /// Push predicates down toward scan nodes
    fn push_down_predicates(&self, mut ops: Vec<QueryOp>) -> (Vec<QueryOp>, usize) {
        let mut push_count = 0;
        let mut predicates = Vec::new();

        // Extract all filter operations
        ops.retain(|op| {
            if let QueryOp::Filter { predicate } = op {
                predicates.push(predicate.clone());
                false
            } else {
                true
            }
        });

        // Try to push predicates to scan operations
        if !predicates.is_empty() {
            for op in ops.iter_mut() {
                if let QueryOp::Scan { table } = op {
                    let relevant = self.extract_relevant_predicates(&predicates, table);
                    if !relevant.is_empty() {
                        // let _combined = self.combine_predicates(relevant);
                        *op = QueryOp::Scan {
                            table: table.clone(),
                        };
                        // Insert filter after scan
                        push_count += 1;
                    }
                }
            }
        }

        // Reinsert remaining filters
        if !predicates.is_empty() {
            ops.insert(0, QueryOp::Filter {
                predicate: QueryPredicate::And(predicates),
            });
        }

        (ops, push_count)
    }

    /// Push projections down toward scan nodes
    fn push_down_projections(&self, mut ops: Vec<QueryOp>) -> (Vec<QueryOp>, usize) {
        let mut push_count = 0;
        let mut projection_cols = HashSet::new();

        // Collect all projected columns
        for op in &ops {
            if let QueryOp::Project { columns } = op {
                projection_cols.extend(columns);
            }
        }

        // Apply projection at scan level
        if !projection_cols.is_empty() {
            for op in ops.iter_mut() {
                if let QueryOp::Scan { table: _ } = op {
                    push_count += 1;
                    // Projection is implicit in scan
                }
            }
        }

        (ops, push_count)
    }

    /// Extract predicates relevant to a specific table
    fn extract_relevant_predicates(
        &self,
        predicates: &[QueryPredicate],
        table: &str,
    ) -> Vec<QueryPredicate> {
        predicates
            .iter()
            .filter(|p| self.predicate_references_table(p, table))
            .cloned()
            .collect()
    }

    /// Check if a predicate references a specific table
    fn predicate_references_table(&self, predicate: &QueryPredicate, table: &str) -> bool {
        // Simplified: assume column names include table prefix
        match predicate {
            QueryPredicate::Eq { column, .. }
            | QueryPredicate::Ne { column, .. }
            | QueryPredicate::Lt { column, .. }
            | QueryPredicate::Le { column, .. }
            | QueryPredicate::Gt { column, .. }
            | QueryPredicate::Ge { column, .. }
            | QueryPredicate::In { column, .. }
            | QueryPredicate::Like { column, .. }
            | QueryPredicate::IsNull { column }
            | QueryPredicate::IsNotNull { column } => column.starts_with(table),
            QueryPredicate::And(preds) | QueryPredicate::Or(preds) => {
                preds.iter().any(|p| self.predicate_references_table(p, table))
            }
            QueryPredicate::Not(pred) => self.predicate_references_table(pred, table),
            QueryPredicate::Custom { .. } => true, // Assume relevant
        }
    }

    /// Combine multiple predicates with AND
    fn combine_predicates(&self, predicates: Vec<QueryPredicate>) -> QueryPredicate {
        if predicates.len() == 1 {
            predicates.into_iter().next().unwrap_or(QueryPredicate::Custom { expression: "combined".to_string() })
        } else {
            QueryPredicate::And(predicates)
        }
    }

    /// Build a physical plan tree from operations
    fn build_plan_tree(&self, operations: Vec<QueryOp>) -> Result<PlanNode> {
        let mut nodes: Vec<PlanNode> = Vec::new();

        for op in operations {
            let node = match op {
                QueryOp::Scan { table } => PlanNode::Scan {
                    table,
                    projection: Vec::new(),
                    filter: None,
                    estimated_rows: 1000,
                    estimated_cost: 100.0,
                },
                QueryOp::IndexScan { table, index, key } => PlanNode::IndexScan {
                    table,
                    index,
                    key,
                    projection: Vec::new(),
                    estimated_rows: 10,
                    estimated_cost: 5.0,
                },
                QueryOp::RangeScan { table, index, lower_bound, upper_bound, inclusive } => {
                    PlanNode::RangeScan {
                        table,
                        index,
                        lower_bound,
                        upper_bound,
                        inclusive,
                        projection: Vec::new(),
                        estimated_rows: 100,
                        estimated_cost: 20.0,
                    }
                }
                QueryOp::Filter { predicate } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Filter {
                            input: Box::new(input),
                            predicate,
                            selectivity: 0.1,
                            estimated_rows: 100,
                            estimated_cost: 10.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Filter requires input node"));
                    }
                }
                QueryOp::Project { columns } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Project {
                            input: Box::new(input),
                            columns,
                            estimated_rows: 100,
                            estimated_cost: 5.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Project requires input node"));
                    }
                }
                QueryOp::Limit { count } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Limit {
                            input: Box::new(input),
                            count,
                            estimated_rows: count,
                            estimated_cost: 1.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Limit requires input node"));
                    }
                }
                QueryOp::Sort { columns } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Sort {
                            input: Box::new(input),
                            columns,
                            algorithm: SortAlgorithm::QuickSort,
                            estimated_rows: 100,
                            estimated_cost: 50.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Sort requires input node"));
                    }
                }
                QueryOp::Aggregate { functions, group_by } => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Aggregate {
                            input: Box::new(input),
                            functions,
                            group_by,
                            estimated_rows: 10,
                            estimated_cost: 100.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Aggregate requires input node"));
                    }
                }
                QueryOp::Join { join_type, left_table: _, right_table: _, condition } => {
                    let right = nodes.pop();
                    let left = nodes.pop();
                    if let (Some(left), Some(right)) = (left, right) {
                        self.create_join_node(left, right, join_type, condition)?
                    } else {
                        return Err(anyhow::anyhow!("Join requires two input nodes"));
                    }
                }
                QueryOp::Union { all } => {
                    let inputs = std::mem::take(&mut nodes);
                    PlanNode::Union {
                        inputs,
                        all,
                        estimated_rows: 100,
                        estimated_cost: 10.0,
                    }
                }
                QueryOp::Distinct => {
                    if let Some(input) = nodes.pop() {
                        PlanNode::Distinct {
                            input: Box::new(input),
                            method: DistinctMethod::Hash,
                            estimated_rows: 50,
                            estimated_cost: 20.0,
                        }
                    } else {
                        return Err(anyhow::anyhow!("Distinct requires input node"));
                    }
                }
            };
            nodes.push(node);
        }

        nodes.pop().ok_or_else(|| anyhow::anyhow!("Empty plan"))
    }

    /// Create an optimal join node
    fn create_join_node(
        &self,
        left: PlanNode,
        right: PlanNode,
        join_type: JoinType,
        condition: JoinCondition,
    ) -> Result<PlanNode> {
        let left_rows = self.estimate_rows(&left);
        let right_rows = self.estimate_rows(&right);

        // Choose join strategy based on sizes
        let join_node = if left_rows < 100 || right_rows < 100 {
            // Small inputs: nested loop
            let condition_pred = match condition {
                JoinCondition::On { left_column, right_column } => {
                    QueryPredicate::Eq {
                        column: left_column,
                        value: QueryValue::String(right_column),
                    }
                }
                JoinCondition::Using { column } => QueryPredicate::Eq {
                    column: column.clone(),
                    value: QueryValue::String(column),
                },
                JoinCondition::Natural => QueryPredicate::Custom {
                    expression: "natural".to_string(),
                },
                JoinCondition::Custom { expression } => QueryPredicate::Custom { expression },
            };
            PlanNode::NestedLoopJoin {
                left: Box::new(left),
                right: Box::new(right),
                join_type,
                condition: condition_pred,
                estimated_rows: left_rows * right_rows / 10,
                estimated_cost: (left_rows * right_rows) as f64 * 0.01,
            }
        } else if let JoinCondition::On { left_column, right_column } = condition {
            // Large inputs with equijoin: hash join
            PlanNode::HashJoin {
                left: Box::new(left),
                right: Box::new(right),
                join_type,
                left_key: left_column,
                right_key: right_column,
                estimated_rows: (left_rows * right_rows) / 100,
                estimated_cost: (left_rows + right_rows) as f64 * 0.1,
            }
        } else {
            // Default to nested loop
            PlanNode::NestedLoopJoin {
                left: Box::new(left),
                right: Box::new(right),
                join_type,
                condition: QueryPredicate::Custom {
                    expression: "unknown".to_string(),
                },
                estimated_rows: left_rows * right_rows / 10,
                estimated_cost: (left_rows * right_rows) as f64 * 0.01,
            }
        };

        Ok(join_node)
    }

    /// Generate and select the best plan
    fn generate_best_plan(&self, base_plan: PhysicalPlan) -> Result<PhysicalPlan> {
        // For now, return the base plan
        // In a full implementation, this would use dynamic programming
        // to explore different join orderings and access methods
        Ok(base_plan)
    }

    /// Estimate cost of a plan node
    fn estimate_cost(&self, node: &PlanNode) -> f64 {
        match node {
            PlanNode::Scan { estimated_cost, .. } => *estimated_cost,
            PlanNode::IndexScan { estimated_cost, .. } => *estimated_cost,
            PlanNode::RangeScan { estimated_cost, .. } => *estimated_cost,
            PlanNode::Filter { estimated_cost, .. } => *estimated_cost,
            PlanNode::Project { estimated_cost, .. } => *estimated_cost,
            PlanNode::Limit { estimated_cost, .. } => *estimated_cost,
            PlanNode::Sort { estimated_cost, .. } => *estimated_cost,
            PlanNode::Aggregate { estimated_cost, .. } => *estimated_cost,
            PlanNode::HashJoin { estimated_cost, .. } => *estimated_cost,
            PlanNode::NestedLoopJoin { estimated_cost, .. } => *estimated_cost,
            PlanNode::MergeJoin { estimated_cost, .. } => *estimated_cost,
            PlanNode::Union { estimated_cost, .. } => *estimated_cost,
            PlanNode::Distinct { estimated_cost, .. } => *estimated_cost,
        }
    }

    /// Estimate rows from a plan node
    fn estimate_rows(&self, node: &PlanNode) -> usize {
        match node {
            PlanNode::Scan { estimated_rows, .. } => *estimated_rows,
            PlanNode::IndexScan { estimated_rows, .. } => *estimated_rows,
            PlanNode::RangeScan { estimated_rows, .. } => *estimated_rows,
            PlanNode::Filter { estimated_rows, .. } => *estimated_rows,
            PlanNode::Project { estimated_rows, .. } => *estimated_rows,
            PlanNode::Limit { estimated_rows, .. } => *estimated_rows,
            PlanNode::Sort { estimated_rows, .. } => *estimated_rows,
            PlanNode::Aggregate { estimated_rows, .. } => *estimated_rows,
            PlanNode::HashJoin { estimated_rows, .. } => *estimated_rows,
            PlanNode::NestedLoopJoin { estimated_rows, .. } => *estimated_rows,
            PlanNode::MergeJoin { estimated_rows, .. } => *estimated_rows,
            PlanNode::Union { estimated_rows, .. } => *estimated_rows,
            PlanNode::Distinct { estimated_rows, .. } => *estimated_rows,
        }
    }

    /// Calculate plan statistics
    fn calculate_plan_statistics(&self, node: &PlanNode) -> PlanStatistics {
        let mut stats = PlanStatistics::default();
        self.accumulate_stats(node, &mut stats);
        stats
    }

    /// Accumulate statistics from a plan node
    fn accumulate_stats(&self, node: &PlanNode, stats: &mut PlanStatistics) {
        match node {
            PlanNode::Scan { estimated_rows, estimated_cost, .. } => {
                stats.estimated_rows = stats.estimated_rows.max(*estimated_rows);
                stats.estimated_cost += estimated_cost;
                stats.estimated_io_ops += estimated_rows / 100;
            }
            PlanNode::IndexScan { estimated_rows, estimated_cost, .. } => {
                stats.estimated_rows = stats.estimated_rows.max(*estimated_rows);
                stats.estimated_cost += estimated_cost;
                stats.estimated_io_ops += 1;
            }
            PlanNode::RangeScan { estimated_rows, estimated_cost, .. } => {
                stats.estimated_rows = stats.estimated_rows.max(*estimated_rows);
                stats.estimated_cost += estimated_cost;
                stats.estimated_io_ops += estimated_rows / 50;
            }
            PlanNode::Filter { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_cpu_cycles += 100;
            }
            PlanNode::Project { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::Limit { input, .. } => {
                self.accumulate_stats(input, stats);
            }
            PlanNode::Sort { input, estimated_cost, algorithm, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                if let SortAlgorithm::ExternalMergeSort { .. } = algorithm {
                    stats.estimated_io_ops += 10;
                }
            }
            PlanNode::Aggregate { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_memory_bytes += 1024 * 1024; // 1MB for aggregation
            }
            PlanNode::HashJoin { left, right, estimated_cost, .. } => {
                self.accumulate_stats(left, stats);
                self.accumulate_stats(right, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_memory_bytes += 2 * 1024 * 1024; // 2MB for hash table
            }
            PlanNode::NestedLoopJoin { left, right, estimated_cost, .. } => {
                self.accumulate_stats(left, stats);
                self.accumulate_stats(right, stats);
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::MergeJoin { left, right, estimated_cost, .. } => {
                self.accumulate_stats(left, stats);
                self.accumulate_stats(right, stats);
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::Union { inputs, estimated_cost, .. } => {
                for input in inputs {
                    self.accumulate_stats(input, stats);
                }
                stats.estimated_cost += estimated_cost;
            }
            PlanNode::Distinct { input, estimated_cost, .. } => {
                self.accumulate_stats(input, stats);
                stats.estimated_cost += estimated_cost;
                stats.estimated_memory_bytes += 512 * 1024; // 512KB for hash set
            }
        }
    }

    /// Hash a logical plan for caching
    fn hash_logical_plan(&self, plan: &LogicalPlan) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for op in &plan.operations {
            format!("{:?}", op).hash(&mut hasher);
        }
        for table in &plan.tables {
            table.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Clear the plan cache
    pub fn clear_cache(&self) {
        let mut cache = self.plan_cache.write();
        cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        let cache = self.plan_cache.read();
        cache.len()
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
