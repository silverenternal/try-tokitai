//! 分支摘要生成器
//!
//! 使用 LLM 生成分支进展摘要，记录分支目的、进展和关键变更
//!
//! # 核心功能
//! - 生成分支进展摘要
//! - 总结关键变更和决策
//! - 提取分支时间线
//! - 生成合并时的摘要融合

use std::sync::Arc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::ai::purpose::BranchType;
use crate::ai::client::LLMClient;

/// 摘要生成请求
#[derive(Debug, Clone)]
pub struct SummaryGenerationRequest {
    /// 分支名称
    pub branch_name: String,
    /// 分支目的
    pub purpose: Option<String>,
    /// 分支类型
    pub branch_type: Option<BranchType>,
    /// 分支创建时间
    pub created_at: DateTime<Utc>,
    /// 最后活动时间
    pub last_activity: DateTime<Utc>,
    /// 对话轮数
    pub conversation_turns: u32,
    /// 对话历史摘要（每轮对话的摘要）
    pub conversation_summaries: Vec<String>,
    /// 关键变更列表
    pub key_changes: Vec<String>,
    /// 重要决策列表
    pub key_decisions: Vec<String>,
    /// 当前状态描述
    pub current_status: String,
    /// 相关文件/项目列表
    pub files_modified: Vec<String>,
}

/// 摘要生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryGenerationResult {
    /// 简短标题（20 字以内）
    pub title: String,
    /// 完整摘要（200-500 字）
    pub summary: String,
    /// 关键成就列表
    pub key_achievements: Vec<String>,
    /// 时间线
    pub timeline: Vec<TimelineEvent>,
    /// 重要决策摘要
    pub decision_summary: String,
    /// 当前状态评估
    pub status_assessment: StatusAssessment,
    /// 下一步建议
    pub next_steps: Vec<String>,
    /// 合并建议（如果适用）
    pub merge_readiness: MergeReadiness,
}

/// 时间线事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 事件描述
    pub description: String,
    /// 事件类型
    pub event_type: String,
}

/// 状态评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusAssessment {
    /// 完成度（0.0-1.0）
    pub completion_ratio: f32,
    /// 质量评分（1-10）
    pub quality_score: u32,
    /// 稳定性评估（Stable, Developing, Experimental）
    pub stability: String,
    /// 总体评估
    pub overall_assessment: String,
}

/// 合并就绪状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeReadiness {
    /// 是否准备就绪
    pub ready: bool,
    /// 就绪度评分（0.0-1.0）
    pub readiness_score: f32,
    /// 前置条件列表
    pub prerequisites: Vec<String>,
    /// 合并注意事项
    pub merge_notes: Vec<String>,
}

/// AI 分支摘要生成器
pub struct AIBranchSummarizer {
    /// LLM 客户端
    llm_client: Arc<dyn LLMClient>,
    /// 生成的摘要历史
    summary_history: Vec<SummaryGenerationResult>,
    /// 统计信息
    stats: SummarizerStats,
}

/// 摘要生成器统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SummarizerStats {
    /// 总摘要生成次数
    pub total_summaries: u32,
    /// 平均摘要长度（字）
    pub avg_summary_length: u32,
    /// 平均完成度评分
    pub avg_completion_ratio: f32,
    /// 平均质量评分
    pub avg_quality_score: f32,
    /// 高就绪度分支数量（readiness_score > 0.8）
    pub high_readiness_count: u32,
}

impl AIBranchSummarizer {
    /// 创建新的摘要生成器
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            summary_history: Vec::new(),
            stats: SummarizerStats::default(),
        }
    }

    /// 生成完整摘要
    pub async fn generate_summary(
        &mut self,
        request: SummaryGenerationRequest,
    ) -> Result<SummaryGenerationResult> {
        tracing::info!(
            "Generating summary for branch: {} ({} turns, {} changes)",
            request.branch_name,
            request.conversation_turns,
            request.key_changes.len()
        );

        // 构建 prompt
        let prompt = self.build_summary_prompt(&request);

        // 定义 JSON Schema
        let schema = self.get_summary_schema();

        // 调用 LLM
        let response_text = self
            .llm_client
            .chat_with_schema(&prompt, &schema)
            .await
            .context("LLM call failed for summary generation")?;

        // 解析响应
        let result: SummaryGenerationResult =
            serde_json::from_str(&response_text).context("Failed to parse LLM response")?;

        // 验证结果
        self.validate_result(&result)?;

        // 更新统计
        self.update_stats(&result);

        // 记录历史
        self.summary_history.push(result.clone());

        tracing::info!(
            "Summary generated: title={}, completion={:.2}, quality={}/10",
            result.title,
            result.status_assessment.completion_ratio,
            result.status_assessment.quality_score
        );

        Ok(result)
    }

    /// 生成快速摘要（仅基于关键变更）
    pub async fn generate_quick_summary(
        &self,
        branch_name: &str,
        purpose: Option<&str>,
        key_changes: &[String],
    ) -> Result<QuickSummary> {
        let prompt = self.build_quick_prompt(branch_name, purpose, key_changes);

        let response_text = self.llm_client.chat(&prompt).await?;

        // 尝试解析为结构化摘要
        let summary: QuickSummary = serde_json::from_str(&response_text)
            .unwrap_or_else(|_| QuickSummary {
                title: format!("{} 分支摘要", branch_name),
                summary: "快速摘要不可用".to_string(),
                key_changes: key_changes.to_vec(),
            });

        Ok(summary)
    }

    /// 生成合并时的摘要融合
    pub async fn generate_merge_summary(
        &self,
        source_summary: &SummaryGenerationResult,
        target_summary: &SummaryGenerationResult,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<String> {
        let prompt = self.build_merge_fusion_prompt(
            source_summary,
            target_summary,
            source_branch,
            target_branch,
        );

        let fusion_summary = self.llm_client.chat(&prompt).await?;

        Ok(fusion_summary)
    }

    /// 构建摘要 prompt
    fn build_summary_prompt(&self, request: &SummaryGenerationRequest) -> String {
        let purpose_context = request
            .purpose
            .clone()
            .unwrap_or_else(|| "未指定目的".to_string());

        let branch_type_context = request
            .branch_type
            .as_ref()
            .map(|t: &BranchType| t.to_string())
            .unwrap_or_else(|| "未指定类型".to_string());

        let conversation_context = if request.conversation_summaries.is_empty() {
            "无对话摘要".to_string()
        } else {
            request
                .conversation_summaries
                .iter()
                .enumerate()
                .map(|(i, s)| format!("{}. {}", i + 1, s))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let changes_context = if request.key_changes.is_empty() {
            "无关键变更".to_string()
        } else {
            request.key_changes.join("\n")
        };

        let decisions_context = if request.key_decisions.is_empty() {
            "无重要决策".to_string()
        } else {
            request.key_decisions.join("\n")
        };

        let files_context = if request.files_modified.is_empty() {
            "无文件修改".to_string()
        } else {
            request.files_modified.join(", ")
        };

        let branch_age_hours =
            (request.last_activity - request.created_at).num_hours() as u32;

        format!(
            r#"# 分支摘要生成任务

你是一个 AI Agent 内存系统的技术文档专家。你的任务是为分支生成清晰、准确的进展摘要。

## 分支基本信息
- **分支名称**: {branch_name}
- **分支目的**: {purpose}
- **分支类型**: {branch_type}
- **存在时间**: {age_hours} 小时
- **创建时间**: {created_at}
- **最后活动**: {last_activity}
- **对话轮数**: {turns}

## 对话历史摘要
{conversations}

## 关键变更
{changes}

## 重要决策
{decisions}

## 修改的文件
{files}

## 当前状态
{status}

## 任务要求

请综合分析上述信息，以 JSON 格式生成摘要：

```json
{{
    "title": "简洁的标题（20 字以内）",
    "summary": "完整的摘要（200-500 字）",
    "key_achievements": ["成就 1", "成就 2", ...],
    "timeline": [
        {{
            "timestamp": "ISO8601 时间戳",
            "description": "事件描述",
            "event_type": "类型（如 conversation, change, decision 等）"
        }}
    ],
    "decision_summary": "重要决策的摘要（100 字以内）",
    "status_assessment": {{
        "completion_ratio": 0.0-1.0 的浮点数",
        "quality_score": 1-10 的整数",
        "stability": "Stable|Developing|Experimental",
        "overall_assessment": "总体评估文本"
    }},
    "next_steps": ["下一步 1", "下一步 2", ...],
    "merge_readiness": {{
        "ready": true/false,
        "readiness_score": 0.0-1.0 的浮点数",
        "prerequisites": ["前置条件 1", ...],
        "merge_notes": ["注意事项 1", ...]
    }}
}}
```

## 写作指南

### 摘要内容
- 客观描述分支的进展和成就
- 突出关键决策和转折点
- 指出当前的挑战和待解决问题
- 语言简洁、专业

### 完成度评估
- 0.0-0.3: 初期探索阶段
- 0.3-0.6: 开发进行中
- 0.6-0.8: 接近完成
- 0.8-1.0: 已完成/可合并

### 质量评分标准
- 1-3: 初步探索，质量不稳定
- 4-6: 基本可用，有待完善
- 7-8: 质量良好，推荐使用
- 9-10: 高质量，可直接合并

### 合并就绪判断
- 完成度 > 0.8
- 质量评分 >= 7
- 稳定性为 Stable
- 无重大未解决问题

现在请生成摘要：
"#,
            branch_name = request.branch_name,
            purpose = purpose_context,
            branch_type = branch_type_context,
            age_hours = branch_age_hours,
            created_at = request.created_at.format("%Y-%m-%d %H:%M:%S"),
            last_activity = request.last_activity.format("%Y-%m-%d %H:%M:%S"),
            turns = request.conversation_turns,
            conversations = conversation_context,
            changes = changes_context,
            decisions = decisions_context,
            files = files_context,
            status = request.current_status,
        )
    }

    /// 构建快速摘要 prompt
    fn build_quick_prompt(
        &self,
        branch_name: &str,
        purpose: Option<&str>,
        key_changes: &[String],
    ) -> String {
        let purpose_context = purpose.unwrap_or("未指定");
        let changes_context = if key_changes.is_empty() {
            "无关键变更".to_string()
        } else {
            key_changes.join("\n")
        };

        format!(
            r#"# 快速摘要生成

为分支生成简要摘要。

## 分支名称
{branch_name}

## 分支目的
{purpose}

## 关键变更
{changes}

## 任务要求

以 JSON 格式返回：

```json
{{
    "title": "简洁标题",
    "summary": "100 字以内的摘要",
    "key_changes": ["变更 1", "变更 2", ...]
}}
```

现在生成：
"#,
            branch_name = branch_name,
            purpose = purpose_context,
            changes = changes_context,
        )
    }

    /// 构建合并摘要融合 prompt
    fn build_merge_fusion_prompt(
        &self,
        source_summary: &SummaryGenerationResult,
        target_summary: &SummaryGenerationResult,
        source_branch: &str,
        target_branch: &str,
    ) -> String {
        format!(
            r#"# 合并摘要融合

将源分支和目标分支的摘要融合为一个连贯的整体摘要。

## 源分支 ({source_branch})
**标题**: {source_title}
**摘要**: {source_summary}
**关键成就**: {source_achievements}

## 目标分支 ({target_branch})
**标题**: {target_title}
**摘要**: {target_summary}
**关键成就**: {target_achievements}

## 任务要求

生成一个融合后的摘要（300 字以内），说明：
1. 合并带来的整体价值
2. 两个分支的互补性
3. 合并后的新能力

直接返回融合后的摘要文本：
"#,
            source_branch = source_branch,
            source_title = source_summary.title,
            source_summary = source_summary.summary,
            source_achievements = source_summary.key_achievements.join(", "),
            target_branch = target_branch,
            target_title = target_summary.title,
            target_summary = target_summary.summary,
            target_achievements = target_summary.key_achievements.join(", "),
        )
    }

    /// 获取 JSON Schema
    fn get_summary_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "简洁标题"
                },
                "summary": {
                    "type": "string",
                    "description": "完整摘要"
                },
                "key_achievements": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "timeline": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "timestamp": { "type": "string" },
                            "description": { "type": "string" },
                            "event_type": { "type": "string" }
                        },
                        "required": ["timestamp", "description", "event_type"]
                    }
                },
                "decision_summary": {
                    "type": "string"
                },
                "status_assessment": {
                    "type": "object",
                    "properties": {
                        "completion_ratio": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                        "quality_score": { "type": "integer", "minimum": 1, "maximum": 10 },
                        "stability": { "type": "string", "enum": ["Stable", "Developing", "Experimental"] },
                        "overall_assessment": { "type": "string" }
                    },
                    "required": ["completion_ratio", "quality_score", "stability", "overall_assessment"]
                },
                "next_steps": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "merge_readiness": {
                    "type": "object",
                    "properties": {
                        "ready": { "type": "boolean" },
                        "readiness_score": { "type": "number", "minimum": 0.0, "maximum": 1.0 },
                        "prerequisites": { "type": "array", "items": { "type": "string" } },
                        "merge_notes": { "type": "array", "items": { "type": "string" } }
                    },
                    "required": ["ready", "readiness_score", "prerequisites", "merge_notes"]
                }
            },
            "required": [
                "title", "summary", "key_achievements", "timeline",
                "decision_summary", "status_assessment", "next_steps", "merge_readiness"
            ]
        })
    }

    /// 验证结果
    fn validate_result(&self, result: &SummaryGenerationResult) -> Result<()> {
        if result.status_assessment.completion_ratio < 0.0
            || result.status_assessment.completion_ratio > 1.0
        {
            anyhow::bail!("Completion ratio must be between 0.0 and 1.0");
        }

        if result.status_assessment.quality_score < 1
            || result.status_assessment.quality_score > 10
        {
            anyhow::bail!("Quality score must be between 1 and 10");
        }

        if result.merge_readiness.readiness_score < 0.0
            || result.merge_readiness.readiness_score > 1.0
        {
            anyhow::bail!("Readiness score must be between 0.0 and 1.0");
        }

        if result.title.len() > 50 {
            anyhow::bail!("Title too long (max 50 chars)");
        }

        Ok(())
    }

    /// 更新统计
    fn update_stats(&mut self, result: &SummaryGenerationResult) {
        self.stats.total_summaries += 1;
        self.stats.avg_summary_length = ((self.stats.avg_summary_length
            * (self.stats.total_summaries - 1))
            + result.summary.len() as u32)
            / self.stats.total_summaries;

        // 更新平均完成度
        let total = self.stats.total_summaries as f32;
        self.stats.avg_completion_ratio = ((self.stats.avg_completion_ratio * (total - 1.0))
            + result.status_assessment.completion_ratio)
            / total;

        // 更新平均质量评分
        let total = self.stats.total_summaries as f32;
        self.stats.avg_quality_score = ((self.stats.avg_quality_score * (total - 1.0))
            + result.status_assessment.quality_score as f32)
            / total;

        // 更新高就绪度计数
        if result.merge_readiness.readiness_score > 0.8 {
            self.stats.high_readiness_count += 1;
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> SummarizerStats {
        self.stats.clone()
    }

    /// 获取摘要历史
    pub fn get_history(&self) -> &[SummaryGenerationResult] {
        &self.summary_history
    }
}

/// 快速摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickSummary {
    pub title: String,
    pub summary: String,
    pub key_changes: Vec<String>,
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
    async fn test_summarizer_creation() {
        let mock_client = Arc::new(MockLLMClient::new(r#"{"title": "Test", "summary": "Test summary", "key_achievements": [], "timeline": [], "decision_summary": "Test", "status_assessment": {"completion_ratio": 0.5, "quality_score": 5, "stability": "Developing", "overall_assessment": "Test"}, "next_steps": [], "merge_readiness": {"ready": false, "readiness_score": 0.5, "prerequisites": [], "merge_notes": []}}"#));
        let summarizer = AIBranchSummarizer::new(mock_client);

        assert_eq!(summarizer.stats.total_summaries, 0);
    }

    #[tokio::test]
    async fn test_generate_summary() {
        let response_json = r#"{
            "title": "用户认证功能实现",
            "summary": "本分支实现了完整的用户认证功能，包括 JWT token 验证、密码加密存储、登录注册 API。经过 15 轮对话和 3 次迭代，功能已趋于稳定。",
            "key_achievements": [
                "完成 JWT token 验证逻辑",
                "实现密码 bcrypt 加密",
                "添加登录注册 API 端点"
            ],
            "timeline": [
                {
                    "timestamp": "2026-03-26T10:00:00Z",
                    "description": "分支创建，开始实现认证功能",
                    "event_type": "branch_created"
                }
            ],
            "decision_summary": "选择 JWT 作为认证方案，使用 bcrypt 加密密码",
            "status_assessment": {
                "completion_ratio": 0.85,
                "quality_score": 8,
                "stability": "Stable",
                "overall_assessment": "功能完整，质量良好，可合并"
            },
            "next_steps": [
                "添加单元测试",
                "更新文档"
            ],
            "merge_readiness": {
                "ready": true,
                "readiness_score": 0.9,
                "prerequisites": [],
                "merge_notes": ["建议在工作时间合并以便监控"]
            }
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut summarizer = AIBranchSummarizer::new(mock_client);

        let now = Utc::now();
        let request = SummaryGenerationRequest {
            branch_name: "feature-auth".to_string(),
            purpose: Some("实现用户认证功能".to_string()),
            branch_type: Some(BranchType::Feature),
            created_at: now - chrono::Duration::hours(24),
            last_activity: now,
            conversation_turns: 15,
            conversation_summaries: vec!["讨论认证方案".to_string()],
            key_changes: vec!["添加 JWT 验证".to_string()],
            key_decisions: vec!["选择 JWT 方案".to_string()],
            current_status: "功能完成，等待测试".to_string(),
            files_modified: vec!["auth.rs".to_string()],
        };

        let result = summarizer.generate_summary(request).await.unwrap();

        assert_eq!(result.status_assessment.completion_ratio, 0.85);
        assert_eq!(result.status_assessment.quality_score, 8);
        assert!(result.merge_readiness.ready);
        assert_eq!(summarizer.stats.total_summaries, 1);
    }

    #[tokio::test]
    async fn test_quick_summary() {
        let response_json = r#"{
            "title": "快速摘要",
            "summary": "分支实现了基础功能",
            "key_changes": ["变更 1", "变更 2"]
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let summarizer = AIBranchSummarizer::new(mock_client);

        let summary = summarizer
            .generate_quick_summary(
                "test-branch",
                Some("测试"),
                &["变更 1".to_string(), "变更 2".to_string()],
            )
            .await
            .unwrap();

        assert!(!summary.title.is_empty());
        assert!(!summary.summary.is_empty());
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let mut summarizer = AIBranchSummarizer::new(Arc::new(MockLLMClient::new(
            r#"{"title": "Test", "summary": "Test summary content here", "key_achievements": [], "timeline": [], "decision_summary": "Test", "status_assessment": {"completion_ratio": 0.8, "quality_score": 8, "stability": "Stable", "overall_assessment": "Good"}, "next_steps": [], "merge_readiness": {"ready": true, "readiness_score": 0.85, "prerequisites": [], "merge_notes": []}}"#,
        )));

        for _ in 0..3 {
            let now = Utc::now();
            let request = SummaryGenerationRequest {
                branch_name: "test".to_string(),
                purpose: None,
                branch_type: None,
                created_at: now - chrono::Duration::hours(1),
                last_activity: now,
                conversation_turns: 5,
                conversation_summaries: vec![],
                key_changes: vec![],
                key_decisions: vec![],
                current_status: "Test".to_string(),
                files_modified: vec![],
            };

            let _ = summarizer.generate_summary(request).await.unwrap();
        }

        let stats = summarizer.get_stats();

        assert_eq!(stats.total_summaries, 3);
        assert!(stats.avg_completion_ratio > 0.7);
        assert!(stats.avg_quality_score >= 7.0);
        assert_eq!(stats.high_readiness_count, 3);
    }
}
