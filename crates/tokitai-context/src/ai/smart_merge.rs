//! 智能合并推荐器
//!
//! 使用 LLM 分析分支状态，推荐最佳合并时机和策略
//!
//! # 核心功能
//! - 分析分支的成熟度和稳定性
//! - 推荐最佳合并时机
//! - 建议合适的合并策略
//! - 评估合并风险
//! - 提供合并前的检查清单

use std::sync::Arc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::parallel::MergeStrategy;
use crate::ai::client::LLMClient;

/// 合并推荐请求
#[derive(Debug, Clone)]
pub struct MergeRecommendationRequest {
    /// 源分支名称
    pub source_branch: String,
    /// 目标分支名称
    pub target_branch: String,
    /// 源分支目的
    pub source_purpose: Option<String>,
    /// 目标分支目的
    pub target_purpose: Option<String>,
    /// 分支存在时间（小时）
    pub branch_age_hours: u32,
    /// 源分支的对话轮数
    pub conversation_turns: u32,
    /// 检测到的冲突数量
    pub conflict_count: usize,
    /// 源分支的关键变更列表
    pub key_changes: Vec<String>,
    /// 分支类型
    pub branch_type: String,
    /// 分支元数据标签
    pub tags: Vec<String>,
}

/// 合并推荐结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeRecommendation {
    /// 是否建议合并
    pub recommend_merge: bool,
    /// 推荐的合并策略
    pub recommended_strategy: MergeStrategy,
    /// 推荐置信度（0.0-1.0）
    pub confidence: f32,
    /// 合并时机建议
    pub timing_recommendation: TimingRecommendation,
    /// 风险评估
    pub risk_assessment: RiskAssessment,
    /// 推荐详细理由
    pub reasoning: String,
    /// 合并前检查清单
    pub checklist: Vec<ChecklistItem>,
    /// 预计合并复杂度（1-10）
    pub estimated_complexity: u32,
}

/// 合并时机建议
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingRecommendation {
    /// 时机类型
    #[serde(rename = "type")]
    pub timing_type: TimingRecommendationType,
    /// 理由
    pub reason: String,
}

/// 合并时机建议类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingRecommendationType {
    /// 立即合并
    MergeNow,
    /// 等待特定条件
    WaitFor,
    /// 需要更多测试
    NeedsMoreTesting,
    /// 需要人工审查
    NeedsHumanReview,
    /// 暂不合并
    DoNotMerge,
}

impl std::fmt::Display for TimingRecommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.timing_type {
            TimingRecommendationType::MergeNow => write!(f, "立即合并"),
            TimingRecommendationType::WaitFor => write!(f, "等待：{}", self.reason),
            TimingRecommendationType::NeedsMoreTesting => write!(f, "需要更多测试"),
            TimingRecommendationType::NeedsHumanReview => write!(f, "需要人工审查"),
            TimingRecommendationType::DoNotMerge => write!(f, "暂不合并"),
        }
    }
}

/// 风险评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    /// 整体风险等级（Low, Medium, High, Critical）
    pub risk_level: String,
    /// 风险评分（0.0-1.0）
    pub risk_score: f32,
    /// 主要风险因素
    pub risk_factors: Vec<String>,
    /// 风险缓解建议
    pub mitigation_suggestions: Vec<String>,
}

/// 检查清单项目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// 检查项描述
    pub item: String,
    /// 是否必须
    pub required: bool,
    /// 当前状态
    pub status: ChecklistStatus,
    /// 备注
    pub notes: Option<String>,
}

/// 检查清单状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChecklistStatus {
    /// 已完成
    #[serde(rename = "completed")]
    Completed,
    /// 未完成
    #[serde(rename = "pending")]
    Pending,
    /// 不适用
    #[serde(rename = "not_applicable")]
    NotApplicable,
    /// 警告
    #[serde(rename = "warning")]
    Warning,
}

/// AI 智能合并推荐器
pub struct AISmartMergeRecommender {
    /// LLM 客户端
    llm_client: Arc<dyn LLMClient>,
    /// 推荐历史
    recommendation_history: Vec<MergeRecommendation>,
    /// 统计信息
    stats: RecommenderStats,
}

/// 推荐器统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RecommenderStats {
    /// 总推荐次数
    pub total_recommendations: u32,
    /// 建议合并的次数
    pub recommend_merge_count: u32,
    /// 各策略推荐次数
    pub fast_forward_count: u32,
    pub selective_merge_count: u32,
    pub ai_assisted_count: u32,
    pub manual_count: u32,
    /// 高风险评估次数
    pub high_risk_count: u32,
    /// 平均置信度
    pub avg_confidence: f32,
}

impl AISmartMergeRecommender {
    /// 创建新的推荐器
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            recommendation_history: Vec::new(),
            stats: RecommenderStats::default(),
        }
    }

    /// 生成合并推荐
    pub async fn recommend_merge(
        &mut self,
        request: MergeRecommendationRequest,
    ) -> Result<MergeRecommendation> {
        tracing::info!(
            "Generating merge recommendation for: {} -> {}",
            request.source_branch,
            request.target_branch
        );

        // 构建 prompt
        let prompt = self.build_recommendation_prompt(&request);

        // 定义 JSON Schema
        let schema = self.get_recommendation_schema();

        // 调用 LLM
        let response_text = self
            .llm_client
            .chat_with_schema(&prompt, &schema)
            .await
            .context("LLM call failed for merge recommendation")?;

        // 解析响应
        let recommendation: MergeRecommendation =
            serde_json::from_str(&response_text).context("Failed to parse LLM response")?;

        // 验证推荐
        self.validate_recommendation(&recommendation)?;

        // 更新统计
        self.update_stats(&recommendation);

        // 记录历史
        self.recommendation_history.push(recommendation.clone());

        tracing::info!(
            "Merge recommendation: recommend={}, strategy={:?}, confidence={:.2}, risk={}",
            recommendation.recommend_merge,
            recommendation.recommended_strategy,
            recommendation.confidence,
            recommendation.risk_assessment.risk_level
        );

        Ok(recommendation)
    }

    /// 快速评估（仅基于基础指标）
    pub fn quick_assess(
        &self,
        conflict_count: usize,
        branch_age_hours: u32,
        branch_type: &str,
    ) -> QuickAssessment {
        let mut risk_score = 0.0;
        let mut risk_factors = Vec::new();

        // 冲突风险评估
        if conflict_count > 5 {
            risk_score += 0.4;
            risk_factors.push(format!("{} 个冲突需要解决", conflict_count));
        } else if conflict_count > 0 {
            risk_score += 0.2;
            risk_factors.push(format!("{} 个轻微冲突", conflict_count));
        }

        // 分支年龄评估
        if branch_age_hours < 2 {
            risk_score += 0.2;
            risk_factors.push("分支创建时间短（<2 小时）".to_string());
        } else if branch_age_hours > 168 {
            // > 1 周
            risk_score += 0.3;
            risk_factors.push("分支存在时间过长（>1 周）".to_string());
        }

        // 分支类型评估
        if branch_type == "experiment" || branch_type == "research" {
            risk_score += 0.2;
            risk_factors.push("实验性分支需要额外审查".to_string());
        }

        let risk_level = if risk_score > 0.7 {
            "High"
        } else if risk_score > 0.4 {
            "Medium"
        } else {
            "Low"
        };

        let recommend_merge = risk_score < 0.5 && conflict_count == 0;
        let strategy = if conflict_count == 0 {
            MergeStrategy::FastForward
        } else {
            MergeStrategy::SelectiveMerge
        };

        QuickAssessment {
            recommend_merge,
            strategy,
            risk_level: risk_level.to_string(),
            risk_score,
            risk_factors,
        }
    }

    /// 构建推荐 prompt
    fn build_recommendation_prompt(&self, request: &MergeRecommendationRequest) -> String {
        let purpose_context = match (&request.source_purpose, &request.target_purpose) {
            (Some(src), Some(tgt)) => format!("源分支目的：{}\n目标分支目的：{}", src, tgt),
            (Some(src), None) => format!("源分支目的：{}", src),
            (None, Some(tgt)) => format!("目标分支目的：{}", tgt),
            (None, None) => "无目的信息".to_string(),
        };

        let changes_context = if request.key_changes.is_empty() {
            "无关键变更信息".to_string()
        } else {
            request.key_changes.join("\n")
        };

        let tags_context = if request.tags.is_empty() {
            "无标签".to_string()
        } else {
            request.tags.join(", ")
        };

        format!(
            r#"# 合并推荐任务

你是一个 AI Agent 内存系统的合并策略专家。你的任务是分析分支状态，给出合并推荐。

## 分支信息
- **源分支**: {source_branch}
- **目标分支**: {target_branch}
- **分支类型**: {branch_type}
- **存在时间**: {branch_age_hours} 小时
- **对话轮数**: {conversation_turns}

## 目的信息
{purpose_context}

## 关键变更
{changes}

## 标签
{tags}

## 冲突情况
检测到 {conflicts} 个冲突

## 任务要求

请综合分析上述信息，以 JSON 格式返回合并推荐：

```json
{{
    "recommend_merge": true/false,
    "recommended_strategy": "FastForward|SelectiveMerge|AIAssisted|Manual|Ours|Theirs",
    "confidence": 0.0-1.0 的浮点数",
    "timing_recommendation": {{
        "type": "MergeNow|WaitFor|NeedsMoreTesting|NeedsHumanReview|DoNotMerge",
        "reason": "时机建议的理由"
    }},
    "risk_assessment": {{
        "risk_level": "Low|Medium|High|Critical",
        "risk_score": 0.0-1.0 的浮点数",
        "risk_factors": ["风险因素 1", ...],
        "mitigation_suggestions": ["缓解建议 1", ...]
    }},
    "reasoning": "详细的推荐理由（200 字以内）",
    "checklist": [
        {{
            "item": "检查项描述",
            "required": true/false,
            "status": "Completed|Pending|NotApplicable|Warning",
            "notes": "备注（可选）"
        }}
    ],
    "estimated_complexity": 1-10 的整数
}}
```

## 判断指南

### 合并策略选择
- **FastForward**: 无冲突，源分支是目标分支的直接后代
- **SelectiveMerge**: 有少量冲突，可以自动解决
- **AIAssisted**: 有语义冲突，需要 AI 辅助判断
- **Manual**: 复杂冲突，需要人工干预
- **Ours/Theirs**: 特殊情况，完全保留某一方

### 时机建议
- **MergeNow**: 分支成熟，无风险或低风险
- **WaitFor**: 等待特定条件（如测试完成、审查通过）
- **NeedsMoreTesting**: 需要更多测试验证
- **NeedsHumanReview**: 需要人工审查
- **DoNotMerge**: 暂不合并（实验失败、质量不达标等）

### 风险评估因素
- 冲突数量和复杂度
- 分支存在时间（过短或过长都有风险）
- 分支类型（实验性分支风险较高）
- 变更范围和影响面
- 对话轮数（反映开发活跃度）

现在请分析并返回 JSON 推荐：
"#,
            source_branch = request.source_branch,
            target_branch = request.target_branch,
            branch_type = request.branch_type,
            branch_age_hours = request.branch_age_hours,
            conversation_turns = request.conversation_turns,
            purpose_context = purpose_context,
            changes = changes_context,
            tags = tags_context,
            conflicts = request.conflict_count,
        )
    }

    /// 获取 JSON Schema
    fn get_recommendation_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "recommend_merge": {
                    "type": "boolean",
                    "description": "是否建议合并"
                },
                "recommended_strategy": {
                    "type": "string",
                    "enum": ["FastForward", "SelectiveMerge", "AIAssisted", "Manual", "Ours", "Theirs"]
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0
                },
                "timing_recommendation": {
                    "type": "object",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["MergeNow", "WaitFor", "NeedsMoreTesting", "NeedsHumanReview", "DoNotMerge"]
                        },
                        "reason": { "type": "string" }
                    },
                    "required": ["type", "reason"]
                },
                "risk_assessment": {
                    "type": "object",
                    "properties": {
                        "risk_level": { "type": "string" },
                        "risk_score": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                        "risk_factors": { "type": "array", "items": { "type": "string" } },
                        "mitigation_suggestions": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["risk_level", "risk_score", "risk_factors", "mitigation_suggestions"]
                },
                "reasoning": { "type": "string" },
                "checklist": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "item": { "type": "string" },
                            "required": { "type": "boolean" },
                            "status": { "type": "string", "enum": ["Completed", "Pending", "NotApplicable", "Warning"] },
                            "notes": { "type": "string" }
                        },
                        "required": ["item", "required", "status"]
                    }
                },
                "estimated_complexity": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 10
                }
            },
            "required": [
                "recommend_merge", "recommended_strategy", "confidence",
                "timing_recommendation", "risk_assessment", "reasoning",
                "checklist", "estimated_complexity"
            ]
        })
    }

    /// 验证推荐
    fn validate_recommendation(&self, rec: &MergeRecommendation) -> Result<()> {
        if rec.confidence < 0.0 || rec.confidence > 1.0 {
            anyhow::bail!("Confidence must be between 0.0 and 1.0");
        }

        if rec.estimated_complexity < 1 || rec.estimated_complexity > 10 {
            anyhow::bail!("Complexity must be between 1 and 10");
        }

        if rec.risk_assessment.risk_score < 0.0
            || rec.risk_assessment.risk_score > 1.0
        {
            anyhow::bail!("Risk score must be between 0.0 and 1.0");
        }

        Ok(())
    }

    /// 更新统计
    fn update_stats(&mut self, rec: &MergeRecommendation) {
        self.stats.total_recommendations += 1;

        if rec.recommend_merge {
            self.stats.recommend_merge_count += 1;
        }

        match rec.recommended_strategy {
            MergeStrategy::FastForward => self.stats.fast_forward_count += 1,
            MergeStrategy::SelectiveMerge => self.stats.selective_merge_count += 1,
            MergeStrategy::AIAssisted => self.stats.ai_assisted_count += 1,
            MergeStrategy::Manual => self.stats.manual_count += 1,
            _ => {}
        }

        if rec.risk_assessment.risk_level == "High"
            || rec.risk_assessment.risk_level == "Critical"
        {
            self.stats.high_risk_count += 1;
        }

        // 更新平均置信度
        let total = self.stats.total_recommendations as f32;
        self.stats.avg_confidence =
            ((self.stats.avg_confidence * (total - 1.0)) + rec.confidence) / total;
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> RecommenderStats {
        self.stats.clone()
    }
}

/// 快速评估结果
#[derive(Debug, Clone)]
pub struct QuickAssessment {
    pub recommend_merge: bool,
    pub strategy: MergeStrategy,
    pub risk_level: String,
    pub risk_score: f32,
    pub risk_factors: Vec<String>,
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
            Self {
                response: response.to_string(),
            }
        }
    }

    #[async_trait]
    impl LLMClient for MockLLMClient {
        async fn chat(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }

        async fn chat_with_schema(
            &self,
            _prompt: &str,
            _schema: &serde_json::Value,
        ) -> Result<String> {
            Ok(self.response.clone())
        }

        async fn chat_with_timeout(&self, prompt: &str, _timeout_ms: u64) -> Result<String> {
            self.chat(prompt).await
        }

        fn model_name(&self) -> &str {
            "mock"
        }

        fn get_stats(&self) -> crate::ai::client::ClientStats {
            crate::ai::client::ClientStats::default()
        }
    }

    #[tokio::test]
    async fn test_recommender_creation() {
        let mock_client = Arc::new(MockLLMClient::new(r#"{"recommend_merge": true, "recommended_strategy": "SelectiveMerge", "confidence": 0.9, "timing_recommendation": {"type": "MergeNow", "reason": "Ready"}, "risk_assessment": {"risk_level": "Low", "risk_score": 0.2, "risk_factors": [], "mitigation_suggestions": []}, "reasoning": "Test", "checklist": [], "estimated_complexity": 3}"#));
        let recommender = AISmartMergeRecommender::new(mock_client);

        assert_eq!(recommender.stats.total_recommendations, 0);
    }

    #[tokio::test]
    async fn test_recommend_merge_low_risk() {
        let response_json = r#"{
            "recommend_merge": true,
            "recommended_strategy": "fast_forward",
            "confidence": 0.95,
            "timing_recommendation": {
                "type": "merge_now",
                "reason": "分支成熟，无冲突"
            },
            "risk_assessment": {
                "risk_level": "Low",
                "risk_score": 0.1,
                "risk_factors": [],
                "mitigation_suggestions": []
            },
            "reasoning": "源分支是目标分支的直接后代，无冲突，建议立即合并",
            "checklist": [
                {"item": "代码审查", "required": true, "status": "completed", "notes": null}
            ],
            "estimated_complexity": 1
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut recommender = AISmartMergeRecommender::new(mock_client);

        let request = MergeRecommendationRequest {
            source_branch: "feature-auth".to_string(),
            target_branch: "main".to_string(),
            source_purpose: Some("实现用户认证".to_string()),
            target_purpose: None,
            branch_age_hours: 24,
            conversation_turns: 10,
            conflict_count: 0,
            key_changes: vec!["添加登录功能".to_string()],
            branch_type: "Feature".to_string(),
            tags: vec!["authentication".to_string()],
        };

        let result = recommender.recommend_merge(request).await.unwrap();

        assert!(result.recommend_merge);
        assert_eq!(result.recommended_strategy, MergeStrategy::FastForward);
        assert_eq!(result.risk_assessment.risk_level, "Low");
        assert!(result.confidence > 0.9);
    }

    #[tokio::test]
    async fn test_recommend_merge_high_risk() {
        let response_json = r#"{
            "recommend_merge": false,
            "recommended_strategy": "manual",
            "confidence": 0.85,
            "timing_recommendation": {
                "type": "needs_human_review",
                "reason": "存在多个复杂冲突"
            },
            "risk_assessment": {
                "risk_level": "High",
                "risk_score": 0.8,
                "risk_factors": ["5 个内容冲突", "分支存在时间过长"],
                "mitigation_suggestions": ["人工审查冲突内容", "考虑分批次合并"]
            },
            "reasoning": "冲突较多且复杂，建议人工审查后再合并",
            "checklist": [
                {"item": "解决所有冲突", "required": true, "status": "pending", "notes": null}
            ],
            "estimated_complexity": 8
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut recommender = AISmartMergeRecommender::new(mock_client);

        let request = MergeRecommendationRequest {
            source_branch: "experimental-refactor".to_string(),
            target_branch: "main".to_string(),
            source_purpose: Some("重构核心模块".to_string()),
            target_purpose: None,
            branch_age_hours: 200,
            conversation_turns: 50,
            conflict_count: 5,
            key_changes: vec!["重构核心逻辑".to_string()],
            branch_type: "Refactor".to_string(),
            tags: vec!["experimental".to_string()],
        };

        let result = recommender.recommend_merge(request).await.unwrap();

        assert!(!result.recommend_merge);
        assert_eq!(result.risk_assessment.risk_level, "High");
        assert_eq!(recommender.stats.high_risk_count, 1);
    }

    #[tokio::test]
    async fn test_quick_assess() {
        let recommender = AISmartMergeRecommender::new(Arc::new(MockLLMClient::new("")));

        // 低风险情况
        let assessment1 = recommender.quick_assess(0, 24, "Feature");
        assert!(assessment1.recommend_merge);
        assert_eq!(assessment1.risk_level, "Low");
        assert!(assessment1.risk_score < 0.5);

        // 高风险情况
        let assessment2 = recommender.quick_assess(6, 1, "Experiment");
        assert!(!assessment2.recommend_merge);
        assert!(assessment2.risk_score > 0.5);
        assert!(assessment2.risk_factors.len() >= 2);
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let mut recommender = AISmartMergeRecommender::new(Arc::new(MockLLMClient::new(
            r#"{"recommend_merge": true, "recommended_strategy": "selective_merge", "confidence": 0.9, "timing_recommendation": {"type": "merge_now", "reason": "Ready"}, "risk_assessment": {"risk_level": "Low", "risk_score": 0.2, "risk_factors": [], "mitigation_suggestions": []}, "reasoning": "Test", "checklist": [], "estimated_complexity": 3}"#,
        )));

        for _ in 0..3 {
            let request = MergeRecommendationRequest {
                source_branch: "test".to_string(),
                target_branch: "main".to_string(),
                source_purpose: None,
                target_purpose: None,
                branch_age_hours: 10,
                conversation_turns: 5,
                conflict_count: 0,
                key_changes: vec![],
                branch_type: "Feature".to_string(),
                tags: vec![],
            };

            let _ = recommender.recommend_merge(request).await.unwrap();
        }

        let stats = recommender.get_stats();

        assert_eq!(stats.total_recommendations, 3);
        assert_eq!(stats.recommend_merge_count, 3);
        assert_eq!(stats.selective_merge_count, 3);
    }
}
