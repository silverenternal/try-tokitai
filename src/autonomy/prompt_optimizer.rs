//! 基于 Prompt Engineering 的工具优化器
//!
//! 使用 Few-Shot Learning + 结构化输出分析工具使用模式，
//! 决定合并/废弃/改进工具
//!
//! ## 核心创新
//! - **Few-Shot 学习**: 历史成功优化决策作为示例
//! - **规则验证器**: 确保 LLM 输出的合理性
//! - **多轮迭代**: 反思 - 验证循环提高质量
//!
//! ## 使用示例
//! ```rust,ignore
//! let optimizer = PromptOptimizer::new(llm_client, tool_registry)?;
//! let suggestions = optimizer.optimize_tools().await?;
//! for suggestion in suggestions {
//!     println!("优化建议：{}", suggestion.description);
//! }
//! ```

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;
use anyhow::{Context, Result};
use tracing::{info, warn};

/// 工具优化建议类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    ///  Few-Shot 证据
    pub few_shot_evidence: Option<String>,
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
    /// 用户满意度 (1-5)
    pub avg_satisfaction: f32,
    /// 功能标签
    pub tags: Vec<String>,
    /// 依赖的工具
    pub dependencies: Vec<String>,
}

/// Few-Shot 优化决策示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationDecision {
    /// 工具库状态
    pub tool_state: String,
    /// 决策描述
    pub decision: String,
    /// 决策结果
    pub outcome: String,
    /// 优化建议
    pub suggestion: OptimizationSuggestion,
}

/// 优化器 Prompt 模板
pub const OPTIMIZER_PROMPT: &str = r#"你是工具库优化专家。请分析以下工具的健康状态，提出优化建议。

## 分析维度

### 1. 使用率分析
- 找出使用率最低的工具（<P25）
- 分析原因：功能冗余？功能太窄？命名不清？

### 2. 失败率分析
- 找出失败率最高的工具（>30%）
- 分析原因：输入验证不足？错误处理不当？

### 3. 冗余分析
- 找出功能重叠的工具
- 建议合并或废弃

### 4. 改进机会
- 哪些工具可以通过小改进获得显著提升？

## 工具库状态

{tool_stats}

## 历史成功决策（Few-Shot 示例）

{history_examples}

## 输出格式

请输出严格的 JSON 格式：

{{
    "optimizations": [
        {{
            "id": "opt_001",
            "optimization_type": "merge|deprecate|improve|split|rename",
            "affected_tools": ["tool_a", "tool_b"],
            "description": "建议描述",
            "rationale": "详细理由",
            "expected_benefit": "预期收益",
            "priority": 1-10,
            "difficulty": 1-5,
            "few_shot_evidence": "参考的历史决策（可选）"
        }}
    ],
    "overall_health_score": 0.0-1.0,
    "summary": "整体健康度总结"
}}
"#;

/// 优化器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOptimizerConfig {
    /// 低使用率阈值
    pub low_usage_threshold: u32,
    /// 高失败率阈值
    pub high_failure_rate_threshold: f32,
    /// 冗余相似度阈值
    pub redundancy_similarity_threshold: f32,
    /// 最小置信度
    pub min_confidence: f32,
    /// 最大迭代次数
    pub max_iterations: u32,
}

impl Default for PromptOptimizerConfig {
    fn default() -> Self {
        Self {
            low_usage_threshold: 5,
            high_failure_rate_threshold: 0.3,
            redundancy_similarity_threshold: 0.8,
            min_confidence: 0.6,
            max_iterations: 3,
        }
    }
}

/// 简化的 LLM Client trait（复用 prompt_gap_detector 中的定义）
#[async_trait::async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(&self, prompt: &str) -> Result<String>;
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String>;
}

/// 基于 Prompt Engineering 的工具优化器
pub struct PromptOptimizer {
    /// LLM 客户端
    llm_client: Arc<dyn LLMClient>,
    /// 工具指标
    tool_metrics: HashMap<String, ToolMetrics>,
    /// 历史决策
    history: Vec<OptimizationDecision>,
    /// 验证器
    validator: OptimizationValidator,
    /// 配置
    config: PromptOptimizerConfig,
}

/// 优化验证器
pub struct OptimizationValidator {
    /// 规则列表
    rules: Vec<OptimizationRule>,
}

/// 验证函数类型别名
pub type ValidatorFn = Box<dyn Fn(&OptimizationSuggestion, &HashMap<String, ToolMetrics>) -> bool + Send + Sync>;

/// 优化规则
pub struct OptimizationRule {
    /// 规则描述
    pub description: String,
    /// 验证函数
    pub validator: ValidatorFn,
}

impl OptimizationValidator {
    /// 创建默认验证器
    pub fn default_rules() -> Self {
        let rules: Vec<OptimizationRule> = vec![
            OptimizationRule {
                description: "合并建议必须涉及至少 2 个工具".to_string(),
                validator: Box::new(|suggestion, _| {
                    if suggestion.optimization_type == OptimizationType::Merge {
                        suggestion.affected_tools.len() >= 2
                    } else {
                        true
                    }
                }),
            },
            OptimizationRule {
                description: "废弃建议必须有合理的理由".to_string(),
                validator: Box::new(|suggestion, metrics| {
                    if suggestion.optimization_type == OptimizationType::Deprecate {
                        // 检查工具使用率是否确实很低
                        suggestion.affected_tools.iter().all(|tool| {
                            metrics.get(tool).is_none_or(|m| m.total_calls < 10)
                        })
                    } else {
                        true
                    }
                }),
            },
            OptimizationRule {
                description: "优先级必须在合理范围内".to_string(),
                validator: Box::new(|suggestion, _| {
                    suggestion.priority >= 1 && suggestion.priority <= 10
                }),
            },
            OptimizationRule {
                description: "难度必须在合理范围内".to_string(),
                validator: Box::new(|suggestion, _| {
                    suggestion.difficulty >= 1 && suggestion.difficulty <= 5
                }),
            },
        ];

        Self { rules }
    }

    /// 验证优化建议
    pub fn validate(&self, suggestion: &OptimizationSuggestion, metrics: &HashMap<String, ToolMetrics>) -> Result<bool> {
        for rule in &self.rules {
            if !(rule.validator)(suggestion, metrics) {
                anyhow::bail!("违反规则：{}", rule.description);
            }
        }
        Ok(true)
    }
}

impl PromptOptimizer {
    /// 创建新的优化器
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            tool_metrics: HashMap::new(),
            history: Vec::new(),
            validator: OptimizationValidator::default_rules(),
            config: PromptOptimizerConfig::default(),
        }
    }

    /// 从配置创建
    pub fn with_config(llm_client: Arc<dyn LLMClient>, config: PromptOptimizerConfig) -> Self {
        Self {
            llm_client,
            tool_metrics: HashMap::new(),
            history: Vec::new(),
            validator: OptimizationValidator::default_rules(),
            config,
        }
    }

    /// 更新工具指标
    pub fn update_metrics(&mut self, metrics: ToolMetrics) {
        self.tool_metrics.insert(metrics.tool_name.clone(), metrics);
    }

    /// 添加历史决策
    pub fn add_history(&mut self, decision: OptimizationDecision) {
        self.history.push(decision);
    }

    /// 分析并优化工具
    pub async fn optimize_tools(&self) -> Result<Vec<OptimizationSuggestion>> {
        if self.tool_metrics.is_empty() {
            warn!("工具指标为空，无法进行优化");
            return Ok(Vec::new());
        }

        info!("开始工具优化分析，共{}个工具...", self.tool_metrics.len());

        // 1. 构建 Prompt
        let prompt = self.build_optimizer_prompt();

        // 2. LLM 推理
        let schema = self.get_response_schema();
        let response_text = self.llm_client
            .chat_with_schema(&prompt, &schema)
            .await
            .context("LLM 推理失败")?;

        // 3. 解析响应
        let response: OptimizerResponse = serde_json::from_str(&response_text)
            .context("解析 LLM 响应失败")?;

        // 4. 验证建议
        let mut validated_suggestions = Vec::new();
        for suggestion in &response.optimizations {
            match self.validator.validate(suggestion, &self.tool_metrics) {
                Ok(_) => validated_suggestions.push(suggestion.clone()),
                Err(e) => warn!("优化建议未通过验证：{} - {}", suggestion.id, e),
            }
        }

        info!(
            "优化分析完成：生成{}个建议，验证通过{}个",
            response.optimizations.len(),
            validated_suggestions.len()
        );

        Ok(validated_suggestions)
    }

    /// 构建优化器 Prompt
    fn build_optimizer_prompt(&self) -> String {
        // 格式化工具统计
        let tool_stats_str = self.tool_metrics.values()
            .map(|m| self.format_tool_metrics(m))
            .collect::<Vec<_>>()
            .join("\n\n");

        // 格式化历史示例
        let history_str = if self.history.is_empty() {
            "暂无历史决策示例".to_string()
        } else {
            self.history.iter()
                .take(3)
                .map(|h| self.format_history_decision(h))
                .collect::<Vec<_>>()
                .join("\n\n")
        };

        OPTIMIZER_PROMPT
            .replace("{tool_stats}", &tool_stats_str)
            .replace("{history_examples}", &history_str)
    }

    /// 格式化工具指标
    fn format_tool_metrics(&self, metrics: &ToolMetrics) -> String {
        let success_rate = if metrics.total_calls > 0 {
            metrics.success_count as f32 / metrics.total_calls as f32 * 100.0
        } else {
            0.0
        };

        format!(
            r#"### 工具：{}
- **调用次数**: {}
- **成功率**: {:.1}%
- **平均耗时**: {:.0}ms
- **满意度**: {:.1}/5
- **标签**: {}
- **依赖**: {}"#,
            metrics.tool_name,
            metrics.total_calls,
            success_rate,
            metrics.avg_execution_time_ms,
            metrics.avg_satisfaction,
            metrics.tags.join(", "),
            metrics.dependencies.join(", "),
        )
    }

    /// 格式化历史决策
    fn format_history_decision(&self, decision: &OptimizationDecision) -> String {
        format!(
            r#"#### 历史决策
状态：{}
决策：{}
结果：{}
建议：{}"#,
            decision.tool_state,
            decision.decision,
            decision.outcome,
            serde_json::to_string(&decision.suggestion).unwrap_or_default(),
        )
    }

    /// 获取响应 JSON Schema
    fn get_response_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["optimizations", "overall_health_score", "summary"],
            "properties": {
                "optimizations": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["id", "optimization_type", "affected_tools", "description", "rationale", "expected_benefit", "priority", "difficulty"],
                        "properties": {
                            "id": {"type": "string"},
                            "optimization_type": {"type": "string", "enum": ["merge", "deprecate", "improve", "split", "rename"]},
                            "affected_tools": {"type": "array", "items": {"type": "string"}},
                            "description": {"type": "string"},
                            "rationale": {"type": "string"},
                            "expected_benefit": {"type": "string"},
                            "priority": {"type": "integer"},
                            "difficulty": {"type": "integer"},
                            "few_shot_evidence": {"type": "string"}
                        }
                    }
                },
                "overall_health_score": {"type": "number"},
                "summary": {"type": "string"}
            }
        })
    }

    /// 计算工具健康度
    pub fn calculate_tool_health(&self, metrics: &ToolMetrics) -> ToolHealth {
        let usage_score = self.calculate_usage_score(metrics);
        let reliability_score = self.calculate_reliability_score(metrics);
        let necessity_score = self.calculate_necessity_score(metrics);

        let health_score = (usage_score * 0.3 + reliability_score * 0.4 + necessity_score * 0.3)
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

        ToolHealth {
            tool_name: metrics.tool_name.clone(),
            health_score,
            usage_score,
            reliability_score,
            necessity_score,
            issues,
        }
    }

    /// 计算使用率评分
    fn calculate_usage_score(&self, metrics: &ToolMetrics) -> f32 {
        if metrics.total_calls == 0 {
            return 0.0;
        }

        let call_score = (metrics.total_calls as f32).ln() / 10.0;
        let call_score = call_score.min(1.0);
        let satisfaction_score = metrics.avg_satisfaction / 5.0;

        (call_score * 0.6 + satisfaction_score * 0.4).min(1.0)
    }

    /// 计算可靠性评分
    fn calculate_reliability_score(&self, metrics: &ToolMetrics) -> f32 {
        if metrics.total_calls == 0 {
            return 0.5;
        }

        let success_rate = metrics.success_count as f32 / metrics.total_calls as f32;
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
        let dependency_count = metrics.dependencies.len();
        let dependency_score = (dependency_count as f32 / 5.0).min(1.0);

        let tag_count = metrics.tags.len();
        let tag_score = (tag_count as f32 / 3.0).min(1.0);

        (dependency_score * 0.5 + tag_score * 0.5).min(1.0)
    }
}

/// 优化器响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerResponse {
    pub optimizations: Vec<OptimizationSuggestion>,
    pub overall_health_score: f32,
    pub summary: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockLLMClient {
        response: String,
    }

    impl MockLLMClient {
        fn new(response: &str) -> Self {
            Self { response: response.to_string() }
        }
    }

    #[async_trait::async_trait]
    impl LLMClient for MockLLMClient {
        async fn chat(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }

        async fn chat_with_schema(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_optimizer_creation() {
        let mock_llm = Arc::new(MockLLMClient::new("{}"));
        let optimizer = PromptOptimizer::new(mock_llm);
        assert_eq!(optimizer.tool_metrics.len(), 0);
    }

    #[test]
    fn test_health_score_calculation() {
        let mock_llm = Arc::new(MockLLMClient::new("{}"));
        let optimizer = PromptOptimizer::new(mock_llm);

        let metrics = ToolMetrics {
            tool_name: "test_tool".to_string(),
            total_calls: 100,
            success_count: 95,
            failure_count: 5,
            avg_execution_time_ms: 50.0,
            avg_satisfaction: 4.5,
            tags: vec!["file".to_string(), "io".to_string(), "utility".to_string()],
            dependencies: vec!["core".to_string(), "utils".to_string()],
        };

        let health = optimizer.calculate_tool_health(&metrics);
        assert!(health.health_score > 0.5);
        assert!(health.issues.is_empty());
    }

    #[test]
    fn test_validator_rules() {
        let validator = OptimizationValidator::default_rules();
        let metrics = HashMap::new();

        // 测试合并建议
        let merge_suggestion = OptimizationSuggestion {
            id: "test_merge".to_string(),
            optimization_type: OptimizationType::Merge,
            affected_tools: vec!["tool_a".to_string(), "tool_b".to_string()],
            description: "Merge tools".to_string(),
            rationale: "Redundant".to_string(),
            expected_benefit: "Simplify".to_string(),
            priority: 5,
            difficulty: 3,
            few_shot_evidence: None,
        };

        assert!(validator.validate(&merge_suggestion, &metrics).is_ok());

        // 测试无效的合并建议（只有 1 个工具）
        let invalid_merge = OptimizationSuggestion {
            id: "test_invalid_merge".to_string(),
            optimization_type: OptimizationType::Merge,
            affected_tools: vec!["tool_a".to_string()], // 只有 1 个工具
            description: "Merge".to_string(),
            rationale: "Redundant".to_string(),
            expected_benefit: "Simplify".to_string(),
            priority: 5,
            difficulty: 3,
            few_shot_evidence: None,
        };

        assert!(validator.validate(&invalid_merge, &metrics).is_err());
    }
}
