//! 工具优化器
//!
//! 分析工具使用率、失败率、冗余度，决定合并/废弃/改进
//!
//! ## 核心功能
//! - 工具使用率分析
//! - 工具冗余检测
//! - 工具废弃建议
//! - 工具改进建议

#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// 工具优化建议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    /// 合并工具
    Merge,
    /// 废弃工具
    Deprecate,
    /// 改进工具
    Improve,
    /// 拆分工具
    Split,
    /// 重命名工具
    Rename,
}

/// 工具优化建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    /// 建议 ID
    pub id: String,
    /// 优化类型
    pub optimization_type: OptimizationType,
    /// 涉及的工具列表
    pub affected_tools: Vec<String>,
    /// 建议描述
    pub description: String,
    /// 理由
    pub rationale: String,
    /// 预期收益
    pub expected_benefit: String,
    /// 实施优先级 (1-10)
    pub priority: u8,
    /// 实施难度 (1-5)
    pub difficulty: u8,
}

/// 工具健康度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolHealth {
    /// 工具名称
    pub tool_name: String,
    /// 健康度评分 (0.0-1.0)
    pub health_score: f32,
    /// 使用频率评分 (0.0-1.0)
    pub usage_score: f32,
    /// 可靠性评分 (0.0-1.0)
    pub reliability_score: f32,
    /// 必要性评分 (0.0-1.0)
    pub necessity_score: f32,
    /// 问题列表
    pub issues: Vec<String>,
}

/// 工具冗余信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRedundancy {
    /// 冗余组 ID
    pub group_id: String,
    /// 冗余工具列表
    pub redundant_tools: Vec<String>,
    /// 冗余原因
    pub reason: String,
    /// 建议保留的工具
    pub suggested_to_keep: String,
    /// 相似度 (0.0-1.0)
    pub similarity: f32,
}

/// 工具使用指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// 工具名称
    pub tool_name: String,
    /// 总调用次数
    pub total_calls: u32,
    /// 成功次数
    pub success_count: u32,
    /// 失败次数
    pub failure_count: u32,
    /// 平均执行时间 (ms)
    pub avg_execution_time_ms: f64,
    /// 最近使用时间戳
    pub last_used_timestamp: u64,
    /// 用户满意度 (1-5)
    pub avg_satisfaction: f32,
    /// 功能标签
    pub tags: Vec<String>,
    /// 依赖的工具
    pub dependencies: Vec<String>,
}

/// 工具优化器
pub struct ToolOptimizer {
    /// 数据存储目录
    data_dir: PathBuf,
    /// 工具指标
    tool_metrics: HashMap<String, ToolMetrics>,
    /// 优化建议
    suggestions: Vec<OptimizationSuggestion>,
    /// 工具健康度
    health_scores: HashMap<String, ToolHealth>,
    /// 配置
    config: OptimizerConfig,
}

/// 优化器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    /// 低使用率阈值（低于此值考虑废弃）
    pub low_usage_threshold: u32,
    /// 低健康度阈值
    pub low_health_threshold: f32,
    /// 冗余相似度阈值
    pub redundancy_similarity_threshold: f32,
    /// 工具名称相似度计算权重
    pub name_similarity_weight: f32,
    /// 功能相似度权重
    pub functionality_similarity_weight: f32,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            low_usage_threshold: 5,
            low_health_threshold: 0.4,
            redundancy_similarity_threshold: 0.8,
            name_similarity_weight: 0.4,
            functionality_similarity_weight: 0.6,
        }
    }
}

impl ToolOptimizer {
    /// 创建新的优化器
    pub fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;

        Ok(Self {
            data_dir,
            tool_metrics: HashMap::new(),
            suggestions: Vec::new(),
            health_scores: HashMap::new(),
            config: OptimizerConfig::default(),
        })
    }

    /// 从配置创建
    pub fn with_config(data_dir: PathBuf, config: OptimizerConfig) -> Result<Self> {
        let mut optimizer = Self::new(data_dir)?;
        optimizer.config = config;
        Ok(optimizer)
    }

    /// 更新工具指标
    pub fn update_metrics(&mut self, metrics: ToolMetrics) {
        self.tool_metrics.insert(metrics.tool_name.clone(), metrics);
    }

    /// 批量更新指标
    pub fn update_metrics_batch(&mut self, metrics_list: Vec<ToolMetrics>) {
        for metrics in metrics_list {
            self.update_metrics(metrics);
        }
    }

    /// 分析并生成优化建议
    pub fn analyze_and_optimize(&mut self) -> Vec<&OptimizationSuggestion> {
        self.suggestions.clear();

        // 1. 计算工具健康度
        self.calculate_health_scores();

        // 2. 检测低使用率工具
        self.detect_low_usage_tools();

        // 3. 检测冗余工具
        self.detect_redundant_tools();

        // 4. 检测需要改进的工具
        self.detect_tools_needing_improvement();

        // 5. 检测工具拆分机会
        self.detect_split_opportunities();

        // 按优先级排序
        self.suggestions.sort_by(|a, b| b.priority.cmp(&a.priority));

        self.suggestions.iter().collect()
    }

    /// 计算工具健康度
    fn calculate_health_scores(&mut self) {
        self.health_scores.clear();

        for (tool_name, metrics) in &self.tool_metrics {
            let usage_score = self.calculate_usage_score(metrics);
            let reliability_score = self.calculate_reliability_score(metrics);
            let necessity_score = self.calculate_necessity_score(metrics);

            let health_score =
                (usage_score * 0.3 + reliability_score * 0.4 + necessity_score * 0.3)
                    .clamp(0.0, 1.0);

            let mut issues = Vec::new();

            if usage_score < 0.3 {
                issues.push("使用率极低".to_string());
            }
            if reliability_score < 0.5 {
                issues.push("可靠性不足".to_string());
            }
            if necessity_score < 0.4 {
                issues.push("必要性存疑".to_string());
            }

            self.health_scores.insert(
                tool_name.clone(),
                ToolHealth {
                    tool_name: tool_name.clone(),
                    health_score,
                    usage_score,
                    reliability_score,
                    necessity_score,
                    issues,
                },
            );
        }
    }

    /// 计算使用率评分
    fn calculate_usage_score(&self, metrics: &ToolMetrics) -> f32 {
        if metrics.total_calls == 0 {
            return 0.0;
        }

        // 基于调用次数的对数评分
        let call_score = (metrics.total_calls as f32).ln() / 10.0;
        let call_score = call_score.min(1.0);

        // 基于满意度的评分
        let satisfaction_score = metrics.avg_satisfaction / 5.0;

        (call_score * 0.6 + satisfaction_score * 0.4).min(1.0)
    }

    /// 计算可靠性评分
    fn calculate_reliability_score(&self, metrics: &ToolMetrics) -> f32 {
        if metrics.total_calls == 0 {
            return 0.5; // 无数据时取中间值
        }

        let success_rate = metrics.success_count as f32 / metrics.total_calls as f32;

        // 执行时间评分（假设 1000ms 以内为优秀）
        let time_score = if metrics.avg_execution_time_ms < 100.0 {
            1.0
        } else if metrics.avg_execution_time_ms < 1000.0 {
            1.0 - ((metrics.avg_execution_time_ms - 100.0) / 900.0) as f32
        } else {
            0.0
        };

        ((success_rate * 0.7) as f64 + (time_score * 0.3) as f64).min(1.0) as f32
    }

    /// 计算必要性评分
    fn calculate_necessity_score(&self, metrics: &ToolMetrics) -> f32 {
        // 基于依赖关系数量
        let dependency_count = metrics.dependencies.len();
        let dependency_score = (dependency_count as f32 / 5.0).min(1.0);

        // 基于功能标签数量
        let tag_count = metrics.tags.len();
        let tag_score = (tag_count as f32 / 3.0).min(1.0);

        (dependency_score * 0.5 + tag_score * 0.5).min(1.0)
    }

    /// 检测低使用率工具
    fn detect_low_usage_tools(&mut self) {
        for (tool_name, metrics) in &self.tool_metrics {
            if metrics.total_calls < self.config.low_usage_threshold {
                let suggestion = OptimizationSuggestion {
                    id: format!("opt_low_usage_{}", tool_name),
                    optimization_type: OptimizationType::Deprecate,
                    affected_tools: vec![tool_name.clone()],
                    description: format!(
                        "工具{}使用率极低（{}次调用）",
                        tool_name, metrics.total_calls
                    ),
                    rationale: "低使用率工具增加维护成本，建议考虑废弃或合并".to_string(),
                    expected_benefit: "减少代码复杂度，降低维护成本".to_string(),
                    priority: 4,
                    difficulty: 2,
                };

                self.suggestions.push(suggestion);
            }
        }
    }

    /// 检测冗余工具
    fn detect_redundant_tools(&mut self) {
        let tool_names: Vec<_> = self.tool_metrics.keys().cloned().collect();
        let mut processed_pairs: HashSet<(String, String)> = HashSet::new();

        for i in 0..tool_names.len() {
            for j in (i + 1)..tool_names.len() {
                let tool1 = &tool_names[i];
                let tool2 = &tool_names[j];

                if processed_pairs.contains(&(tool1.clone(), tool2.clone())) {
                    continue;
                }

                let similarity = self.calculate_tool_similarity(tool1, tool2);

                if similarity >= self.config.redundancy_similarity_threshold {
                    processed_pairs.insert((tool1.clone(), tool2.clone()));

                    // 决定保留哪个工具
                    let metrics1 = &self.tool_metrics[tool1];
                    let metrics2 = &self.tool_metrics[tool2];

                    let to_keep = if metrics1.total_calls >= metrics2.total_calls {
                        tool1.clone()
                    } else {
                        tool2.clone()
                    };

                    let to_deprecate = if to_keep == *tool1 {
                        tool2.clone()
                    } else {
                        tool1.clone()
                    };

                    let suggestion = OptimizationSuggestion {
                        id: format!("opt_redundant_{}_{}", tool1, tool2),
                        optimization_type: OptimizationType::Merge,
                        affected_tools: vec![tool1.clone(), tool2.clone()],
                        description: format!(
                            "工具{}和{}功能冗余（相似度{:.0}%）",
                            tool1,
                            tool2,
                            similarity * 100.0
                        ),
                        rationale: format!("建议保留{}，废弃{}", to_keep, to_deprecate),
                        expected_benefit: "消除冗余，简化代码结构".to_string(),
                        priority: 6,
                        difficulty: 3,
                    };

                    self.suggestions.push(suggestion);
                }
            }
        }
    }

    /// 检测需要改进的工具
    fn detect_tools_needing_improvement(&mut self) {
        for (tool_name, health) in &self.health_scores {
            if health.health_score < self.config.low_health_threshold {
                let issues_str = health.issues.join(", ");

                let suggestion = OptimizationSuggestion {
                    id: format!("opt_improve_{}", tool_name),
                    optimization_type: OptimizationType::Improve,
                    affected_tools: vec![tool_name.clone()],
                    description: format!(
                        "工具{}健康度低（{:.1}分）",
                        tool_name,
                        health.health_score * 100.0
                    ),
                    rationale: format!("存在问题：{}", issues_str),
                    expected_benefit: "提升工具质量和用户体验".to_string(),
                    priority: 7,
                    difficulty: 4,
                };

                self.suggestions.push(suggestion);
            }
        }
    }

    /// 检测工具拆分机会
    fn detect_split_opportunities(&mut self) {
        for (tool_name, metrics) in &self.tool_metrics {
            // 如果工具标签很多且执行时间差异大，可能可以拆分
            if metrics.tags.len() >= 5 && metrics.avg_execution_time_ms > 500.0 {
                let suggestion = OptimizationSuggestion {
                    id: format!("opt_split_{}", tool_name),
                    optimization_type: OptimizationType::Split,
                    affected_tools: vec![tool_name.clone()],
                    description: format!(
                        "工具{}功能复杂（{}个标签），建议拆分",
                        tool_name,
                        metrics.tags.len()
                    ),
                    rationale: "复杂工具难以维护，拆分为专用工具可提升性能".to_string(),
                    expected_benefit: "提升性能和可维护性".to_string(),
                    priority: 5,
                    difficulty: 4,
                };

                self.suggestions.push(suggestion);
            }
        }
    }

    /// 计算工具相似度
    fn calculate_tool_similarity(&self, tool1: &str, tool2: &str) -> f32 {
        let metrics1 = &self.tool_metrics[tool1];
        let metrics2 = &self.tool_metrics[tool2];

        // 名称相似度
        let name_sim = self.levenshtein_similarity(tool1, tool2);

        // 标签相似度
        let tag_sim = self.jaccard_similarity(&metrics1.tags, &metrics2.tags);

        // 依赖相似度
        let dep_sim = self.jaccard_similarity(&metrics1.dependencies, &metrics2.dependencies);

        (name_sim * self.config.name_similarity_weight
            + tag_sim * self.config.functionality_similarity_weight
            + dep_sim * (1.0 - self.config.functionality_similarity_weight))
            .min(1.0)
    }

    /// Levenshtein 距离相似度
    fn levenshtein_similarity(&self, s1: &str, s2: &str) -> f32 {
        let s1_chars: Vec<char> = s1.chars().collect();
        let s2_chars: Vec<char> = s2.chars().collect();

        let len1 = s1_chars.len();
        let len2 = s2_chars.len();

        if len1 == 0 && len2 == 0 {
            return 1.0;
        }

        let distance = self.levenshtein_distance(&s1_chars, &s2_chars);
        let max_len = len1.max(len2);

        1.0 - (distance as f32 / max_len as f32)
    }

    /// Levenshtein 距离计算
    fn levenshtein_distance(&self, s1: &[char], s2: &[char]) -> usize {
        let len1 = s1.len();
        let len2 = s2.len();

        let mut dp = vec![vec![0; len2 + 1]; len1 + 1];

        for (i, row) in dp.iter_mut().enumerate().take(len1 + 1) {
            row[0] = i;
        }
        for (j, val) in dp[0].iter_mut().enumerate().take(len2 + 1) {
            *val = j;
        }

        for i in 1..=len1 {
            for j in 1..=len2 {
                let cost = if s1[i - 1] == s2[j - 1] { 0 } else { 1 };
                dp[i][j] = (dp[i - 1][j] + 1)
                    .min(dp[i][j - 1] + 1)
                    .min(dp[i - 1][j - 1] + cost);
            }
        }

        dp[len1][len2]
    }

    /// Jaccard 相似度
    fn jaccard_similarity(&self, set1: &[String], set2: &[String]) -> f32 {
        let set1: HashSet<_> = set1.iter().collect();
        let set2: HashSet<_> = set2.iter().collect();

        let intersection = set1.intersection(&set2).count();
        let union = set1.union(&set2).count();

        if union == 0 {
            1.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// 获取优化建议
    pub fn get_suggestions(&self) -> &[OptimizationSuggestion] {
        &self.suggestions
    }

    /// 获取工具健康度
    pub fn get_health_scores(&self) -> &HashMap<String, ToolHealth> {
        &self.health_scores
    }

    /// 保存优化数据
    pub fn save_to_file(&self) -> Result<()> {
        let file_path = self.data_dir.join("optimization_suggestions.json");
        let json = serde_json::to_string_pretty(&self.suggestions)?;
        std::fs::write(file_path, json)?;
        Ok(())
    }

    /// 加载优化数据
    pub fn load_from_file(&mut self) -> Result<()> {
        let file_path = self.data_dir.join("optimization_suggestions.json");
        if file_path.exists() {
            let json = std::fs::read_to_string(file_path)?;
            self.suggestions = serde_json::from_str(&json)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_optimizer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let optimizer = ToolOptimizer::new(temp_dir.path().to_path_buf()).unwrap();
        assert!(optimizer.suggestions.is_empty());
    }

    #[test]
    fn test_health_score_calculation() {
        let temp_dir = TempDir::new().unwrap();
        let mut optimizer = ToolOptimizer::new(temp_dir.path().to_path_buf()).unwrap();

        optimizer.update_metrics(ToolMetrics {
            tool_name: "test_tool".to_string(),
            total_calls: 100,
            success_count: 95,
            failure_count: 5,
            avg_execution_time_ms: 50.0,
            last_used_timestamp: 0,
            avg_satisfaction: 4.5,
            tags: vec!["file".to_string(), "io".to_string()],
            dependencies: vec![],
        });

        optimizer.calculate_health_scores();

        let health = optimizer.health_scores.get("test_tool").unwrap();
        assert!(health.health_score > 0.5);
    }

    #[test]
    fn test_redundancy_detection() {
        let temp_dir = TempDir::new().unwrap();
        let mut optimizer = ToolOptimizer::new(temp_dir.path().to_path_buf()).unwrap();

        // 添加两个相似工具
        optimizer.update_metrics(ToolMetrics {
            tool_name: "read_file".to_string(),
            total_calls: 50,
            success_count: 48,
            failure_count: 2,
            avg_execution_time_ms: 30.0,
            last_used_timestamp: 0,
            avg_satisfaction: 4.0,
            tags: vec!["file".to_string(), "read".to_string()],
            dependencies: vec![],
        });

        optimizer.update_metrics(ToolMetrics {
            tool_name: "file_read".to_string(),
            total_calls: 10,
            success_count: 9,
            failure_count: 1,
            avg_execution_time_ms: 35.0,
            last_used_timestamp: 0,
            avg_satisfaction: 3.5,
            tags: vec!["file".to_string(), "read".to_string()],
            dependencies: vec![],
        });

        let suggestions = optimizer.analyze_and_optimize();
        assert!(!suggestions.is_empty());
    }
}
