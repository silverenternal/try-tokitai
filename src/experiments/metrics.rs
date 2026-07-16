//! 实验指标计算器
//!
//! # 设计目标
//! 计算学术论文所需的核心指标，包括：
//! - 主要评估指标（任务完成率、工具调用次数、失败率、满意度）
//! - 次要评估指标（缺口检测质量、工具创建效果）
//! - 性能指标（延迟、吞吐量、成本）
//! - 质量指标（精确率、召回率、F1 分数）

use std::collections::{HashMap, HashSet};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use crate::autonomy::{
    gap_detector::{TaskExecutionRecord, ToolGap, GapType},
    hybrid_gap_detector::{HybridToolGap},
};

// ============================================================================
// 核心指标定义（论文 Table 1）
// ============================================================================

/// 核心实验指标（用于论文主表）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreExperimentMetrics {
    // === 主要评估指标（Primary Metrics）===
    /// 任务完成率（0.0-1.0）
    pub task_completion_rate: f64,
    /// 平均工具调用次数
    pub avg_tool_calls: f64,
    /// 工具失败率（0.0-1.0）
    pub tool_failure_rate: f64,
    /// 用户满意度（1.0-5.0）
    pub user_satisfaction: f64,
    
    // === 次要评估指标（Secondary Metrics）===
    /// 检测到的缺口数量
    pub gaps_detected: u32,
    /// 创建的工具数量
    pub tools_created: u32,
    /// 优化的工具数量
    pub tools_optimized: u32,
    /// 废弃的工具数量
    pub tools_deprecated: u32,
    
    // === 性能指标（Performance Metrics）===
    /// 平均检测延迟（毫秒）
    pub avg_detection_latency_ms: f64,
    /// 平均进化周期耗时（秒）
    pub avg_evolution_cycle_duration_s: f64,
    /// API 调用总次数
    pub total_api_calls: u32,
    /// API 总成本（美元）
    pub total_api_cost_usd: f64,
    /// 缓存命中率（0.0-1.0）
    pub cache_hit_rate: f64,
    
    // === 质量指标（Quality Metrics）===
    /// 缺口检测精确率（0.0-1.0）
    pub gap_detection_precision: f64,
    /// 缺口检测召回率（0.0-1.0）
    pub gap_detection_recall: f64,
    /// 缺口检测 F1 分数（0.0-1.0）
    pub gap_detection_f1: f64,
    /// 代码编译通过率（0.0-1.0）
    pub code_compilation_success_rate: f64,
    /// 工具创建成功率（0.0-1.0）
    pub tool_creation_success_rate: f64,
}

impl CoreExperimentMetrics {
    /// 创建默认指标（所有值为 0）
    pub fn zeros() -> Self {
        Self {
            task_completion_rate: 0.0,
            avg_tool_calls: 0.0,
            tool_failure_rate: 0.0,
            user_satisfaction: 0.0,
            gaps_detected: 0,
            tools_created: 0,
            tools_optimized: 0,
            tools_deprecated: 0,
            avg_detection_latency_ms: 0.0,
            avg_evolution_cycle_duration_s: 0.0,
            total_api_calls: 0,
            total_api_cost_usd: 0.0,
            cache_hit_rate: 0.0,
            gap_detection_precision: 0.0,
            gap_detection_recall: 0.0,
            gap_detection_f1: 0.0,
            code_compilation_success_rate: 0.0,
            tool_creation_success_rate: 0.0,
        }
    }
}

// ============================================================================
// 指标计算器
// ============================================================================

/// 实验指标计算器
pub struct MetricsCalculator {
    /// 任务记录
    task_records: Vec<TaskExecutionRecord>,
    /// 详细任务日志
    detailed_logs: Vec<DetailedTaskLog>,
    /// 检测到的缺口
    detected_gaps: Vec<HybridToolGap>,
    /// 缺口检测事件
    gap_events: Vec<GapDetectionEvent>,
    /// 人工标注的真实缺口（用于计算精确率/召回率）
    ground_truth_gaps: Option<HashSet<String>>,
    /// API 调用总成本
    total_api_cost: f64,
    /// API 调用总次数
    total_api_calls: u32,
}

impl MetricsCalculator {
    /// 创建新的指标计算器
    pub fn new() -> Self {
        Self {
            task_records: Vec::new(),
            detailed_logs: Vec::new(),
            detected_gaps: Vec::new(),
            gap_events: Vec::new(),
            ground_truth_gaps: None,
            total_api_cost: 0.0,
            total_api_calls: 0,
        }
    }
    
    /// 添加任务记录
    pub fn add_task_records(&mut self, records: Vec<TaskExecutionRecord>) {
        self.task_records.extend(records);
    }
    
    /// 添加详细日志
    pub fn add_detailed_logs(&mut self, logs: Vec<DetailedTaskLog>) {
        self.detailed_logs.extend(logs);
    }
    
    /// 添加检测到的缺口
    pub fn add_detected_gaps(&mut self, gaps: Vec<HybridToolGap>) {
        self.detected_gaps.extend(gaps);
    }
    
    /// 添加缺口检测事件
    pub fn add_gap_events(&mut self, events: Vec<GapDetectionEvent>) {
        self.gap_events.extend(events);
    }
    
    /// 设置人工标注的真实缺口
    pub fn set_ground_truth(&mut self, gap_ids: HashSet<String>) {
        self.ground_truth_gaps = Some(gap_ids);
    }
    
    /// 记录 API 调用
    pub fn record_api_call(&mut self, cost: f64) {
        self.total_api_cost += cost;
        self.total_api_calls += 1;
    }
    
    /// 计算所有核心指标
    pub fn calculate_all_metrics(&self) -> CoreExperimentMetrics {
        CoreExperimentMetrics {
            task_completion_rate: self.calculate_completion_rate(),
            avg_tool_calls: self.calculate_avg_tool_calls(),
            tool_failure_rate: self.calculate_tool_failure_rate(),
            user_satisfaction: self.calculate_avg_satisfaction(),
            gaps_detected: self.detected_gaps.len() as u32,
            tools_created: 0,  // 需要额外数据
            tools_optimized: 0,
            tools_deprecated: 0,
            avg_detection_latency_ms: self.calculate_avg_detection_latency(),
            avg_evolution_cycle_duration_s: 0.0,  // 需要额外数据
            total_api_calls: self.total_api_calls,
            total_api_cost_usd: self.total_api_cost,
            cache_hit_rate: 0.0,  // 需要额外数据
            gap_detection_precision: self.calculate_precision(),
            gap_detection_recall: self.calculate_recall(),
            gap_detection_f1: self.calculate_f1(),
            code_compilation_success_rate: 0.0,  // 需要额外数据
            tool_creation_success_rate: 0.0,  // 需要额外数据
        }
    }
    
    // === 主要评估指标计算 ===
    
    /// 计算任务完成率
    pub fn calculate_completion_rate(&self) -> f64 {
        if self.task_records.is_empty() {
            return 0.0;
        }
        
        let completed = self.task_records.iter()
            .filter(|r| r.success)
            .count();
        
        completed as f64 / self.task_records.len() as f64
    }
    
    /// 计算平均工具调用次数
    pub fn calculate_avg_tool_calls(&self) -> f64 {
        if self.task_records.is_empty() {
            return 0.0;
        }
        
        let total_calls: usize = self.task_records.iter()
            .map(|r| r.used_tools.len())
            .sum();
        
        total_calls as f64 / self.task_records.len() as f64
    }
    
    /// 计算工具失败率
    pub fn calculate_tool_failure_rate(&self) -> f64 {
        if self.task_records.is_empty() {
            return 0.0;
        }
        
        let failed = self.task_records.iter()
            .filter(|r| !r.success)
            .count();
        
        failed as f64 / self.task_records.len() as f64
    }
    
    /// 计算平均用户满意度
    pub fn calculate_avg_satisfaction(&self) -> f64 {
        let ratings: Vec<u8> = self.task_records.iter()
            .filter_map(|r| r.user_satisfaction)
            .collect();

        if ratings.is_empty() {
            return 0.0;
        }

        let sum: f64 = ratings.iter().map(|&r| r as f64).sum();
        sum / ratings.len() as f64
    }
    
    // === 性能指标计算 ===
    
    /// 计算平均检测延迟
    pub fn calculate_avg_detection_latency(&self) -> f64 {
        if self.gap_events.is_empty() {
            return 0.0;
        }
        
        let total_latency: u64 = self.gap_events.iter()
            .map(|e| e.detection_duration_ms)
            .sum();
        
        total_latency as f64 / self.gap_events.len() as f64
    }
    
    /// 计算统计检测的平均延迟（仅统计方法）
    pub fn calculate_statistical_detection_latency(&self) -> f64 {
        let statistical_events: Vec<&GapDetectionEvent> = self.gap_events.iter()
            .filter(|e| !e.causal_analysis_performed)
            .collect();
        
        if statistical_events.is_empty() {
            return 0.0;
        }
        
        let total_latency: u64 = statistical_events.iter()
            .map(|e| e.detection_duration_ms)
            .sum();
        
        total_latency as f64 / statistical_events.len() as f64
    }
    
    /// 计算因果检测的平均延迟（统计 + 因果）
    pub fn calculate_causal_detection_latency(&self) -> f64 {
        let causal_events: Vec<&GapDetectionEvent> = self.gap_events.iter()
            .filter(|e| e.causal_analysis_performed)
            .collect();
        
        if causal_events.is_empty() {
            return 0.0;
        }
        
        let total_latency: u64 = causal_events.iter()
            .map(|e| e.detection_duration_ms)
            .sum();
        
        total_latency as f64 / causal_events.len() as f64
    }
    
    // === 质量指标计算 ===
    
    /// 计算精确率（Precision）
    /// Precision = TP / (TP + FP)
    pub fn calculate_precision(&self) -> f64 {
        let ground_truth = match &self.ground_truth_gaps {
            Some(gt) => gt,
            None => return 0.0,  // 没有真实标注，无法计算
        };
        
        if self.detected_gaps.is_empty() {
            return 0.0;
        }
        
        // 真阳性：检测到的缺口在真实标注中
        let true_positives = self.detected_gaps.iter()
            .filter(|gap| ground_truth.contains(&gap.id))
            .count();
        
        true_positives as f64 / self.detected_gaps.len() as f64
    }
    
    /// 计算召回率（Recall）
    /// Recall = TP / (TP + FN)
    pub fn calculate_recall(&self) -> f64 {
        let ground_truth = match &self.ground_truth_gaps {
            Some(gt) => gt,
            None => return 0.0,
        };
        
        if ground_truth.is_empty() {
            return 0.0;
        }
        
        // 真阳性：真实标注中的缺口被检测到
        let true_positives = self.detected_gaps.iter()
            .filter(|gap| ground_truth.contains(&gap.id))
            .count();
        
        true_positives as f64 / ground_truth.len() as f64
    }
    
    /// 计算 F1 分数
    /// F1 = 2 * (Precision * Recall) / (Precision + Recall)
    pub fn calculate_f1(&self) -> f64 {
        let precision = self.calculate_precision();
        let recall = self.calculate_recall();
        
        if precision + recall == 0.0 {
            return 0.0;
        }
        
        2.0 * (precision * recall) / (precision + recall)
    }
    
    // === 缺口类型分析 ===
    
    /// 按类型统计缺口
    pub fn count_gaps_by_type(&self) -> HashMap<GapType, u32> {
        let mut counts = HashMap::new();
        
        for gap in &self.detected_gaps {
            let count = counts.entry(gap.gap_type.clone()).or_insert(0);
            *count += 1;
        }
        
        counts
    }
    
    /// 统计高优先级缺口（priority >= 7）
    pub fn count_high_priority_gaps(&self) -> u32 {
        self.detected_gaps.iter()
            .filter(|g| g.priority >= 7)
            .count() as u32
    }
    
    /// 统计有因果证据的缺口
    pub fn count_causal_gaps(&self) -> u32 {
        self.detected_gaps.iter()
            .filter(|g| g.causal_evidence.is_some())
            .count() as u32
    }
    
    /// 统计仅有统计证据的缺口
    pub fn count_statistical_only_gaps(&self) -> u32 {
        self.detected_gaps.iter()
            .filter(|g| g.causal_evidence.is_none())
            .count() as u32
    }
    
    // === 成本效益分析 ===
    
    /// 计算每个缺口的平均 API 成本
    pub fn calculate_avg_cost_per_gap(&self) -> f64 {
        if self.detected_gaps.is_empty() {
            return 0.0;
        }
        
        self.total_api_cost / self.detected_gaps.len() as f64
    }
    
    /// 计算成本效益比（缺口数量 / API 成本）
    pub fn calculate_cost_effectiveness(&self) -> f64 {
        if self.total_api_cost == 0.0 {
            return f64::INFINITY;
        }
        
        self.detected_gaps.len() as f64 / self.total_api_cost
    }
    
    // === 详细报告生成 ===
    
    /// 生成完整的指标报告
    pub fn generate_detailed_report(&self) -> MetricsReport {
        let core_metrics = self.calculate_all_metrics();
        
        MetricsReport {
            core_metrics,
            gap_type_distribution: self.count_gaps_by_type(),
            high_priority_gaps: self.count_high_priority_gaps(),
            causal_gaps: self.count_causal_gaps(),
            statistical_only_gaps: self.count_statistical_only_gaps(),
            avg_detection_latency_ms: self.calculate_avg_detection_latency(),
            statistical_latency_ms: self.calculate_statistical_detection_latency(),
            causal_latency_ms: self.calculate_causal_detection_latency(),
            avg_cost_per_gap: self.calculate_avg_cost_per_gap(),
            cost_effectiveness: self.calculate_cost_effectiveness(),
            total_tasks: self.task_records.len(),
            total_api_calls: self.total_api_calls,
            total_api_cost: self.total_api_cost,
        }
    }
}

impl Default for MetricsCalculator {
    fn default() -> Self {
        Self::new()
    }
}

/// 完整指标报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsReport {
    /// 核心指标
    pub core_metrics: CoreExperimentMetrics,
    /// 缺口类型分布
    pub gap_type_distribution: HashMap<GapType, u32>,
    /// 高优先级缺口数量
    pub high_priority_gaps: u32,
    /// 因果缺口数量
    pub causal_gaps: u32,
    /// 仅统计缺口数量
    pub statistical_only_gaps: u32,
    /// 平均检测延迟
    pub avg_detection_latency_ms: f64,
    /// 统计检测延迟
    pub statistical_latency_ms: f64,
    /// 因果检测延迟
    pub causal_latency_ms: f64,
    /// 每个缺口的平均成本
    pub avg_cost_per_gap: f64,
    /// 成本效益比
    pub cost_effectiveness: f64,
    /// 总任务数
    pub total_tasks: usize,
    /// 总 API 调用数
    pub total_api_calls: u32,
    /// 总 API 成本
    pub total_api_cost: f64,
}

// ============================================================================
// 提升百分比计算（用于论文对比）
// ============================================================================

/// 计算指标提升百分比
pub fn calculate_improvement(baseline: f64, improved: f64) -> f64 {
    if baseline == 0.0 {
        return f64::INFINITY;
    }
    ((improved - baseline) / baseline).abs() * 100.0
}

/// 格式化提升百分比（带符号）
pub fn format_improvement(baseline: f64, improved: f64) -> String {
    let improvement = calculate_improvement(baseline, improved);
    let sign = if improved > baseline { "+" } else { "-" };
    format!("{}{:.1}%", sign, improvement)
}

/// 对比结果（用于论文表格）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// 指标名称
    pub metric_name: String,
    /// 基线值
    pub baseline_value: f64,
    /// 改进值
    pub improved_value: f64,
    /// 提升百分比
    pub improvement_percent: f64,
    /// 是否显著提升
    pub is_significant: bool,
    /// p 值
    pub p_value: Option<f64>,
}

impl ComparisonResult {
    /// 创建对比结果
    pub fn new(
        metric_name: &str,
        baseline_value: f64,
        improved_value: f64,
    ) -> Self {
        Self {
            metric_name: metric_name.to_string(),
            baseline_value,
            improved_value,
            improvement_percent: calculate_improvement(baseline_value, improved_value),
            is_significant: false,
            p_value: None,
        }
    }
    
    /// 设置显著性检验结果
    pub fn with_significance(mut self, p_value: f64, threshold: f64) -> Self {
        self.p_value = Some(p_value);
        self.is_significant = p_value < threshold;
        self
    }
}
