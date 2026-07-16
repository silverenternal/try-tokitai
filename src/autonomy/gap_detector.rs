//! 工具缺口检测器
//!
//! 从失败任务、低效任务中发现工具缺口
//!
//! ## 核心功能
//! - 分析任务失败原因，识别工具缺失
//! - 检测工具使用率低但需求高的场景
//! - 识别工具组合使用中的断点
//! - 生成工具缺口报告

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// 工具缺口类型
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GapType {
    /// 完全缺失的工具
    MissingTool,
    /// 工具功能不足
    InsufficientCapability,
    /// 工具组合断点
    CombinationGap,
    /// 性能瓶颈
    PerformanceBottleneck,
}

/// 工具缺口证据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapEvidence {
    /// 证据类型
    pub evidence_type: String,
    /// 证据描述
    pub description: String,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 相关任务 ID 列表
    pub related_task_ids: Vec<String>,
    /// 发生次数
    pub occurrence_count: u32,
}

/// 工具缺口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolGap {
    /// 缺口唯一 ID
    pub id: String,
    /// 缺口类型
    pub gap_type: GapType,
    /// 缺口描述
    pub description: String,
    /// 建议的工具名称
    pub suggested_tool_name: Option<String>,
    /// 建议的工具功能描述
    pub suggested_capabilities: Vec<String>,
    /// 优先级 (1-10)
    pub priority: u8,
    /// 证据列表
    pub evidence: Vec<GapEvidence>,
    /// 影响范围描述
    pub impact_scope: String,
}

/// 任务执行记录（简化版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionRecord {
    /// 任务 ID
    pub task_id: String,
    /// 任务描述
    pub task_description: String,
    /// 是否成功
    pub success: bool,
    /// 使用的工具列表
    pub used_tools: Vec<String>,
    /// 执行时间 (ms)
    pub execution_time_ms: u64,
    /// 失败原因（如果失败）
    pub failure_reason: Option<String>,
    /// 用户满意度评分 (1-5)
    pub user_satisfaction: Option<u8>,
}

/// 工具使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUsageStats {
    /// 工具名称
    pub tool_name: String,
    /// 使用次数
    pub usage_count: u32,
    /// 成功率
    pub success_rate: f32,
    /// 平均执行时间 (ms)
    pub avg_execution_time_ms: f64,
    /// 用户满意度
    pub avg_satisfaction: f32,
}

/// 工具缺口检测器
pub struct ToolGapDetector {
    /// 数据存储目录
    data_dir: PathBuf,
    /// 任务执行记录缓存
    task_records: Vec<TaskExecutionRecord>,
    /// 工具使用统计
    tool_stats: HashMap<String, ToolUsageStats>,
    /// 已识别的缺口
    identified_gaps: Vec<ToolGap>,
    /// 检测阈值
    config: GapDetectorConfig,
}

/// 检测器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDetectorConfig {
    /// 失败率阈值（超过此值认为存在缺口）
    pub failure_rate_threshold: f32,
    /// 低满意度阈值
    pub low_satisfaction_threshold: f32,
    /// 最小证据数量
    pub min_evidence_count: u32,
    /// 性能退化阈值（ms）
    pub performance_degradation_threshold_ms: u64,
}

impl Default for GapDetectorConfig {
    fn default() -> Self {
        Self {
            failure_rate_threshold: 0.5,
            low_satisfaction_threshold: 2.5,
            min_evidence_count: 3,
            performance_degradation_threshold_ms: 5000,
        }
    }
}

impl ToolGapDetector {
    /// 创建新的检测器
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;

        Ok(Self {
            data_dir,
            task_records: Vec::new(),
            tool_stats: HashMap::new(),
            identified_gaps: Vec::new(),
            config: GapDetectorConfig::default(),
        })
    }

    /// 从配置创建
    pub fn with_config(data_dir: PathBuf, config: GapDetectorConfig) -> Result<Self> {
        let mut detector = Self::new(data_dir)?;
        detector.config = config;
        Ok(detector)
    }

    /// 记录任务执行
    pub fn record_task(&mut self, record: TaskExecutionRecord) {
        // 更新工具统计
        for tool in &record.used_tools {
            let stats = self
                .tool_stats
                .entry(tool.clone())
                .or_insert_with(|| ToolUsageStats {
                    tool_name: tool.clone(),
                    usage_count: 0,
                    success_rate: 1.0,
                    avg_execution_time_ms: 0.0,
                    avg_satisfaction: 3.0,
                });

            stats.usage_count += 1;

            // 更新成功率
            let total = stats.usage_count as f32;
            let successes = if record.success {
                (stats.success_rate * (total - 1.0)) + 1.0
            } else {
                stats.success_rate * (total - 1.0)
            };
            stats.success_rate = successes / total;

            // 更新平均执行时间
            stats.avg_execution_time_ms = (stats.avg_execution_time_ms * (total - 1.0) as f64
                + record.execution_time_ms as f64)
                / total as f64;

            // 更新满意度
            if let Some(sat) = record.user_satisfaction {
                stats.avg_satisfaction =
                    (stats.avg_satisfaction * (total - 1.0) + sat as f32) / total;
            }
        }

        self.task_records.push(record);
    }

    /// 分析并检测工具缺口
    pub fn analyze_and_detect(&mut self) -> Vec<&ToolGap> {
        self.identified_gaps.clear();

        // 1. 分析失败任务
        self.detect_from_failures();

        // 2. 分析低满意度任务
        self.detect_from_low_satisfaction();

        // 3. 分析性能瓶颈
        self.detect_from_performance_issues();

        // 4. 分析工具组合断点
        self.detect_combination_gaps();

        // 按优先级排序
        self.identified_gaps
            .sort_by(|a, b| b.priority.cmp(&a.priority));

        self.identified_gaps.iter().collect()
    }

    /// 从失败任务中检测缺口
    fn detect_from_failures(&mut self) {
        let failed_tasks: Vec<_> = self.task_records.iter().filter(|r| !r.success).collect();

        if failed_tasks.is_empty() {
            return;
        }

        // 分析失败原因
        let mut failure_patterns: HashMap<String, Vec<&TaskExecutionRecord>> = HashMap::new();

        for record in &failed_tasks {
            if let Some(reason) = &record.failure_reason {
                // 提取关键词模式
                let pattern = self.extract_failure_pattern(reason);
                failure_patterns.entry(pattern).or_default().push(record);
            }
        }

        // 生成缺口
        for (pattern, records) in failure_patterns {
            if records.len() as u32 >= self.config.min_evidence_count {
                let gap = ToolGap {
                    id: format!("gap_{}", pattern.replace(' ', "_")),
                    gap_type: GapType::MissingTool,
                    description: format!("检测到重复的失败模式：{}", pattern),
                    suggested_tool_name: None,
                    suggested_capabilities: vec![format!("处理与{}相关的需求", pattern)],
                    priority: self.calculate_priority(records.len()),
                    evidence: vec![GapEvidence {
                        evidence_type: "failure_pattern".to_string(),
                        description: pattern,
                        confidence: 0.8,
                        related_task_ids: records.iter().map(|r| r.task_id.clone()).collect(),
                        occurrence_count: records.len() as u32,
                    }],
                    impact_scope: format!("影响{}个任务", records.len()),
                };

                self.identified_gaps.push(gap);
            }
        }
    }

    /// 从低满意度任务中检测缺口
    fn detect_from_low_satisfaction(&mut self) {
        let low_sat_tasks: Vec<_> = self
            .task_records
            .iter()
            .filter(|r| {
                r.user_satisfaction
                    .is_some_and(|s| s as f32 <= self.config.low_satisfaction_threshold)
            })
            .collect();

        if low_sat_tasks.is_empty() {
            return;
        }

        // 分析共同特征
        let mut common_issues: HashMap<String, Vec<&TaskExecutionRecord>> = HashMap::new();

        for record in &low_sat_tasks {
            let issue = self.extract_satisfaction_issue(record);
            common_issues.entry(issue).or_default().push(record);
        }

        for (issue, records) in common_issues {
            if records.len() as u32 >= self.config.min_evidence_count {
                let gap = ToolGap {
                    id: format!("gap_sat_{}", issue.replace(' ', "_")),
                    gap_type: GapType::InsufficientCapability,
                    description: format!("用户满意度低，可能原因：{}", issue),
                    suggested_tool_name: None,
                    suggested_capabilities: vec![format!("改进与{}相关的功能", issue)],
                    priority: self.calculate_priority(records.len()),
                    evidence: vec![GapEvidence {
                        evidence_type: "low_satisfaction".to_string(),
                        description: issue,
                        confidence: 0.7,
                        related_task_ids: records.iter().map(|r| r.task_id.clone()).collect(),
                        occurrence_count: records.len() as u32,
                    }],
                    impact_scope: format!("影响{}个任务的用户体验", records.len()),
                };

                self.identified_gaps.push(gap);
            }
        }
    }

    /// 从性能问题中检测缺口
    fn detect_from_performance_issues(&mut self) {
        for (tool_name, stats) in &self.tool_stats {
            if stats.avg_execution_time_ms > self.config.performance_degradation_threshold_ms as f64
            {
                let gap = ToolGap {
                    id: format!("gap_perf_{}", tool_name),
                    gap_type: GapType::PerformanceBottleneck,
                    description: format!(
                        "工具{}执行时间过长（平均{:.0}ms）",
                        tool_name, stats.avg_execution_time_ms
                    ),
                    suggested_tool_name: Some(format!("{}_optimized", tool_name)),
                    suggested_capabilities: vec![
                        "优化执行性能".to_string(),
                        "支持异步/批量操作".to_string(),
                    ],
                    priority: 7,
                    evidence: vec![GapEvidence {
                        evidence_type: "performance_issue".to_string(),
                        description: format!(
                            "平均执行时间{:.0}ms，超过阈值",
                            stats.avg_execution_time_ms
                        ),
                        confidence: 0.9,
                        related_task_ids: Vec::new(),
                        occurrence_count: stats.usage_count,
                    }],
                    impact_scope: format!("影响{}次工具调用", stats.usage_count),
                };

                self.identified_gaps.push(gap);
            }
        }
    }

    /// 检测工具组合断点
    fn detect_combination_gaps(&mut self) {
        // 分析经常一起出现但缺少直接组合的工具
        let mut tool_pairs: HashMap<(String, String), u32> = HashMap::new();

        for record in &self.task_records {
            let tools = &record.used_tools;
            for i in 0..tools.len() {
                for j in (i + 1)..tools.len() {
                    let pair = (tools[i].clone(), tools[j].clone());
                    *tool_pairs.entry(pair).or_insert(0) += 1;
                }
            }
        }

        // 找出频繁共现但没有组合工具的情况
        for ((tool1, tool2), count) in &tool_pairs {
            if *count >= self.config.min_evidence_count {
                // 检查是否存在组合工具
                let combo_name = format!("{}_{}", tool1, tool2);
                if !self.tool_stats.contains_key(&combo_name) {
                    let gap = ToolGap {
                        id: format!("gap_combo_{}_{}", tool1, tool2),
                        gap_type: GapType::CombinationGap,
                        description: format!(
                            "工具{}和{}经常一起使用，建议创建组合工具",
                            tool1, tool2
                        ),
                        suggested_tool_name: Some(combo_name),
                        suggested_capabilities: vec![
                            format!("整合{}的功能", tool1),
                            format!("整合{}的功能", tool2),
                            "提供一站式解决方案".to_string(),
                        ],
                        priority: 6,
                        evidence: vec![GapEvidence {
                            evidence_type: "combination_gap".to_string(),
                            description: format!("{}和{}共现{}次", tool1, tool2, count),
                            confidence: 0.75,
                            related_task_ids: Vec::new(),
                            occurrence_count: *count,
                        }],
                        impact_scope: format!("影响{}个任务", count),
                    };

                    self.identified_gaps.push(gap);
                }
            }
        }
    }

    /// 提取失败模式关键词
    fn extract_failure_pattern(&self, reason: &str) -> String {
        // 简化实现：提取前 20 个字符作为模式标识
        let reason = reason.to_lowercase();
        if reason.len() > 30 {
            reason[..30].to_string()
        } else {
            reason
        }
    }

    /// 提取满意度问题
    fn extract_satisfaction_issue(&self, record: &TaskExecutionRecord) -> String {
        if let Some(reason) = &record.failure_reason {
            self.extract_failure_pattern(reason)
        } else if record.execution_time_ms > 3000 {
            "执行时间过长".to_string()
        } else {
            "功能不符合预期".to_string()
        }
    }

    /// 计算优先级
    fn calculate_priority(&self, occurrence_count: usize) -> u8 {
        match occurrence_count {
            0..=2 => 3,
            3..=5 => 5,
            6..=10 => 7,
            _ => 9,
        }
    }

    /// 获取已识别的缺口
    pub fn get_gaps(&self) -> &[ToolGap] {
        &self.identified_gaps
    }

    /// 获取工具统计
    pub fn get_tool_stats(&self) -> &HashMap<String, ToolUsageStats> {
        &self.tool_stats
    }

    /// 获取任务记录
    pub fn get_task_records(&self) -> &[TaskExecutionRecord] {
        &self.task_records
    }

    /// 保存检测数据
    pub fn save_to_file(&self) -> Result<()> {
        let file_path = self.data_dir.join("gap_analysis.json");
        let json = serde_json::to_string_pretty(&self.identified_gaps)?;
        std::fs::write(file_path, json)?;
        Ok(())
    }

    /// 加载检测数据
    pub fn load_from_file(&mut self) -> Result<()> {
        let file_path = self.data_dir.join("gap_analysis.json");
        if file_path.exists() {
            let json = std::fs::read_to_string(file_path)?;
            self.identified_gaps = serde_json::from_str(&json)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_detector_creation() {
        let temp_dir = TempDir::new().unwrap();
        let detector = ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(detector.identified_gaps.is_empty());
    }

    #[test]
    fn test_record_task() {
        let temp_dir = TempDir::new().unwrap();
        let mut detector = ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap();

        let record = TaskExecutionRecord {
            task_id: "test_1".to_string(),
            task_description: "Test task".to_string(),
            success: true,
            used_tools: vec!["tool_a".to_string()],
            execution_time_ms: 100,
            failure_reason: None,
            user_satisfaction: Some(5),
        };

        detector.record_task(record);
        assert_eq!(detector.task_records.len(), 1);
        assert!(detector.tool_stats.contains_key("tool_a"));
    }

    #[test]
    fn test_detect_from_failures() {
        let temp_dir = TempDir::new().unwrap();
        let mut detector = ToolGapDetector::new(temp_dir.path().to_path_buf()).unwrap();

        // 记录多个相似失败任务
        for i in 0..5 {
            detector.record_task(TaskExecutionRecord {
                task_id: format!("task_{}", i),
                task_description: "Test".to_string(),
                success: false,
                used_tools: vec![],
                execution_time_ms: 100,
                failure_reason: Some("文件不存在".to_string()),
                user_satisfaction: Some(1),
            });
        }

        let gaps = detector.analyze_and_detect();
        assert!(!gaps.is_empty());
    }
}
