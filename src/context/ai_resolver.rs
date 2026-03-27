//! AI 辅助冲突解决器
//!
//! 使用 LLM 进行语义级冲突检测和合并决策
//!
//! # 核心功能
//! - 分析冲突内容的语义差异
//! - 生成合并建议（KeepSource, KeepTarget, Combine, Discard）
//! - 对于 Combine 决策，生成融合版本
//! - 提供冲突解决的可解释性理由

use std::path::PathBuf;
use std::sync::Arc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use super::branch::MergeDecision;
use super::graph::{Conflict, ConflictResolution, ConflictType};

/// LLM 客户端 trait（与 autonomy 模块保持一致）
#[async_trait]
pub trait LLMClient: Send + Sync {
    /// 发送聊天请求并获取响应
    async fn chat(&self, prompt: &str) -> Result<String>;

    /// 发送带 JSON Schema 约束的聊天请求
    async fn chat_with_schema(
        &self,
        prompt: &str,
        schema: &serde_json::Value,
    ) -> Result<String>;
}

/// AI 冲突解决请求
#[derive(Debug, Clone)]
pub struct ConflictResolutionRequest {
    /// 冲突 ID
    pub conflict_id: String,
    /// 源分支名称
    pub source_branch: String,
    /// 目标分支名称
    pub target_branch: String,
    /// 冲突类型
    pub conflict_type: ConflictType,
    /// 源分支内容
    pub source_content: String,
    /// 目标分支内容
    pub target_content: String,
    /// 项目 ID（文件/上下文项标识）
    pub item_id: String,
    /// 层类型（short_term, long_term）
    pub layer: String,
    /// 分支目的（可选，用于辅助决策）
    pub source_purpose: Option<String>,
    pub target_purpose: Option<String>,
}

/// AI 冲突解决响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictResolutionResponse {
    /// 合并决策
    pub decision: MergeDecision,
    /// 决策理由
    pub reasoning: String,
    /// 融合后的内容（仅当 decision=Combine 时）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combined_content: Option<String>,
    /// 置信度（0.0-1.0）
    pub confidence: f32,
    /// 建议的合并策略
    pub suggested_strategy: String,
}

/// AI 冲突解决器
pub struct AIConflictResolver {
    /// LLM 客户端
    llm_client: Arc<dyn LLMClient>,
    /// 解决历史
    resolution_history: Vec<ConflictResolutionResponse>,
    /// 统计信息
    stats: ResolverStats,
}

/// 解决器统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolverStats {
    /// 总解决次数
    pub total_resolutions: u32,
    /// 各决策类型计数
    pub keep_source_count: u32,
    pub keep_target_count: u32,
    pub combine_count: u32,
    pub discard_count: u32,
    /// 平均置信度
    pub avg_confidence: f32,
    /// 高置信度决策比例（>0.8）
    pub high_confidence_ratio: f32,
}

impl AIConflictResolver {
    /// 创建新的 AI 冲突解决器
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            resolution_history: Vec::new(),
            stats: ResolverStats::default(),
        }
    }

    /// 解决单个冲突
    pub async fn resolve_conflict(
        &mut self,
        request: ConflictResolutionRequest,
    ) -> Result<ConflictResolutionResponse> {
        tracing::info!(
            "Resolving conflict: {} (type: {:?})",
            request.conflict_id,
            request.conflict_type
        );

        // 构建 prompt
        let prompt = self.build_resolution_prompt(&request);

        // 定义 JSON Schema
        let schema = self.get_resolution_schema();

        // 调用 LLM
        let response_text = self
            .llm_client
            .chat_with_schema(&prompt, &schema)
            .await
            .context("LLM call failed for conflict resolution")?;

        // 解析响应
        let response: ConflictResolutionResponse =
            serde_json::from_str(&response_text).context("Failed to parse LLM response")?;

        // 验证响应
        self.validate_response(&response)?;

        // 更新统计
        self.update_stats(&response);

        // 记录历史
        self.resolution_history.push(response.clone());

        tracing::info!(
            "Conflict resolved: decision={:?}, confidence={:.2}, reasoning={}",
            response.decision,
            response.confidence,
            response.reasoning
        );

        Ok(response)
    }

    /// 批量解决冲突
    pub async fn resolve_conflicts_batch(
        &mut self,
        requests: Vec<ConflictResolutionRequest>,
    ) -> Result<Vec<ConflictResolutionResponse>> {
        let mut responses = Vec::with_capacity(requests.len());

        for request in requests {
            match self.resolve_conflict(request).await {
                Ok(response) => responses.push(response),
                Err(e) => {
                    tracing::warn!("Failed to resolve conflict, using default strategy: {}", e);
                    // 失败时使用默认策略（保留源分支）
                    responses.push(ConflictResolutionResponse {
                        decision: MergeDecision::KeepSource,
                        reasoning: format!("Default fallback due to error: {}", e),
                        combined_content: None,
                        confidence: 0.5,
                        suggested_strategy: "selective_merge".to_string(),
                    });
                }
            }
        }

        Ok(responses)
    }

    /// 分析冲突并生成详细报告
    pub async fn analyze_conflict(
        &self,
        request: &ConflictResolutionRequest,
    ) -> Result<ConflictAnalysisReport> {
        let prompt = self.build_analysis_prompt(request);

        let response_text = self.llm_client.chat(&prompt).await?;

        // 尝试解析为结构化报告
        let report: ConflictAnalysisReport = serde_json::from_str(&response_text)
            .unwrap_or_else(|_| ConflictAnalysisReport {
                semantic_similarity: 0.5,
                key_differences: Vec::new(),
                compatibility_assessment: "Unknown".to_string(),
                recommendation: "Manual review recommended".to_string(),
            });

        Ok(report)
    }

    /// 构建冲突解决 prompt
    fn build_resolution_prompt(&self, request: &ConflictResolutionRequest) -> String {
        let branch_context = match (&request.source_purpose, &request.target_purpose) {
            (Some(src_purpose), Some(tgt_purpose)) => format!(
                r#"源分支 "{}" 目的：{}
目标分支 "{}" 目的：{}"#,
                request.source_branch, src_purpose, request.target_branch, tgt_purpose
            ),
            _ => format!(
                r#"源分支：{}
目标分支：{}"#,
                request.source_branch, request.target_branch
            ),
        };

        format!(
            r#"# 上下文冲突解决任务

你是一个 AI Agent 内存系统的冲突解决专家。你的任务是分析两个分支之间的上下文冲突，并给出合并建议。

## 冲突信息
- **冲突 ID**: {conflict_id}
- **项目 ID**: {item_id}
- **层类型**: {layer}
- **冲突类型**: {conflict_type:?}

## 分支信息
{branch_context}

## 内容对比

### 源分支内容
```
{source_content}
```

### 目标分支内容
```
{target_content}
```

## 任务要求

请分析上述冲突，并以 JSON 格式返回你的决策：

```json
{{
    "decision": "KeepSource|KeepTarget|Combine|Discard",
    "reasoning": "详细的决策理由，解释为什么选择这个决策",
    "combined_content": "仅当 decision=Combine 时填写，否则为 null",
    "confidence": 0.0-1.0 之间的浮点数，表示决策置信度",
    "suggested_strategy": "建议的合并策略名称"
}}
```

## 决策指南

- **KeepSource**: 当源分支的内容更准确、更新或更符合分支目的时
- **KeepTarget**: 当目标分支的内容更稳定、更通用或源分支内容已过时
- **Combine**: 当两个分支的内容都有价值，可以互补融合时（需要生成融合版本）
- **Discard**: 当两个版本都不再需要（罕见情况）

## 输出要求
- reasoning 必须详细解释决策依据
- confidence 应基于内容相似度、分支目的相关性等因素
- suggested_strategy 应该是具体的合并策略名称（如 "selective_merge", "ai_assisted" 等）

现在请分析冲突并返回 JSON 决策：
"#,
            conflict_id = request.conflict_id,
            item_id = request.item_id,
            layer = request.layer,
            conflict_type = request.conflict_type,
            branch_context = branch_context,
            source_content = request.source_content,
            target_content = request.target_content,
        )
    }

    /// 构建冲突分析 prompt
    fn build_analysis_prompt(&self, request: &ConflictResolutionRequest) -> String {
        format!(
            r#"# 冲突分析任务

分析两个分支上下文的语义差异。

## 内容对比

### 源分支（{}）
```
{}
```

### 目标分支（{}）
```
{}
```

## 分析要求

请以 JSON 格式返回分析报告：

```json
{{
    "semantic_similarity": 0.0-1.0 的浮点数，表示语义相似度",
    "key_differences": ["差异点 1", "差异点 2", ...],
    "compatibility_assessment": "Compatible|Partially Compatible|Incompatible",
    "recommendation": "合并建议文本"
}}
```

现在请分析：
"#,
            request.source_branch,
            request.source_content,
            request.target_branch,
            request.target_content,
        )
    }

    /// 获取 JSON Schema
    fn get_resolution_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "decision": {
                    "type": "string",
                    "enum": ["KeepSource", "KeepTarget", "Combine", "Discard"]
                },
                "reasoning": {
                    "type": "string",
                    "description": "详细的决策理由"
                },
                "combined_content": {
                    "type": "string",
                    "description": "融合后的内容（仅当 decision=Combine）"
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0,
                    "description": "决策置信度"
                },
                "suggested_strategy": {
                    "type": "string",
                    "description": "建议的合并策略"
                }
            },
            "required": ["decision", "reasoning", "confidence", "suggested_strategy"]
        })
    }

    /// 验证响应
    fn validate_response(
        &self,
        response: &ConflictResolutionResponse,
    ) -> Result<()> {
        // 验证决策有效性
        match response.decision {
            MergeDecision::KeepSource
            | MergeDecision::KeepTarget
            | MergeDecision::Combine
            | MergeDecision::Discard => Ok::<_, anyhow::Error>(()),
            MergeDecision::AIResolved => {
                // AIResolved 不应该直接出现在响应中
                anyhow::bail!("Invalid decision: AIResolved should not be returned directly")
            }
        }?;

        // 验证置信度范围
        if response.confidence < 0.0 || response.confidence > 1.0 {
            anyhow::bail!("Confidence must be between 0.0 and 1.0");
        }

        // 验证 Combine 决策必须有融合内容
        if response.decision == MergeDecision::Combine
            && response.combined_content.is_none()
        {
            anyhow::bail!("Combine decision must include combined_content");
        }

        Ok(())
    }

    /// 更新统计信息
    fn update_stats(&mut self, response: &ConflictResolutionResponse) {
        self.stats.total_resolutions += 1;

        match response.decision {
            MergeDecision::KeepSource => self.stats.keep_source_count += 1,
            MergeDecision::KeepTarget => self.stats.keep_target_count += 1,
            MergeDecision::Combine => self.stats.combine_count += 1,
            MergeDecision::Discard => self.stats.discard_count += 1,
            _ => {}
        }

        // 更新平均置信度
        let total = self.stats.total_resolutions as f32;
        self.stats.avg_confidence = ((self.stats.avg_confidence * (total - 1.0))
            + response.confidence)
            / total;

        // 更新高置信度比例
        let high_conf_count = if response.confidence > 0.8 { 1 } else { 0 };
        let prev_high_conf =
            (self.stats.high_confidence_ratio * (total - 1.0)).round() as u32;
        self.stats.high_confidence_ratio =
            ((prev_high_conf + high_conf_count) as f32) / total;
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> ResolverStats {
        self.stats.clone()
    }

    /// 获取解决历史
    pub fn get_history(&self) -> &[ConflictResolutionResponse] {
        &self.resolution_history
    }

    /// 清除历史
    pub fn clear_history(&mut self) {
        self.resolution_history.clear();
    }
}

/// 冲突分析报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictAnalysisReport {
    /// 语义相似度（0.0-1.0）
    pub semantic_similarity: f32,
    /// 关键差异列表
    pub key_differences: Vec<String>,
    /// 兼容性评估
    pub compatibility_assessment: String,
    /// 推荐建议
    pub recommendation: String,
}

/// 从文件内容构建冲突解决请求
pub async fn build_conflict_request_from_file(
    source_path: &std::path::Path,
    target_path: &std::path::Path,
    conflict_id: &str,
    item_id: &str,
    layer: &str,
    source_branch: &str,
    target_branch: &str,
    source_purpose: Option<String>,
    target_purpose: Option<String>,
) -> Result<ConflictResolutionRequest> {
    let source_content = tokio::fs::read_to_string(source_path)
        .await
        .with_context(|| format!("Failed to read source file: {:?}", source_path))?;

    let target_content = tokio::fs::read_to_string(target_path)
        .await
        .with_context(|| format!("Failed to read target file: {:?}", target_path))?;

    Ok(ConflictResolutionRequest {
        conflict_id: conflict_id.to_string(),
        source_branch: source_branch.to_string(),
        target_branch: target_branch.to_string(),
        conflict_type: ConflictType::ContentConflict,
        source_content,
        target_content,
        item_id: item_id.to_string(),
        layer: layer.to_string(),
        source_purpose,
        target_purpose,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// Mock LLM 客户端用于测试
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
    }

    #[tokio::test]
    async fn test_resolver_creation() {
        let mock_client = Arc::new(MockLLMClient::new(r#"{"decision": "KeepSource", "reasoning": "Test", "confidence": 0.9, "suggested_strategy": "selective_merge"}"#));
        let resolver = AIConflictResolver::new(mock_client);

        assert_eq!(resolver.stats.total_resolutions, 0);
    }

    #[tokio::test]
    async fn test_resolve_conflict_keep_source() {
        let response_json = r#"{
            "decision": "keep_source",
            "reasoning": "源分支内容更新、更准确",
            "confidence": 0.9,
            "suggested_strategy": "selective_merge"
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut resolver = AIConflictResolver::new(mock_client);

        let request = ConflictResolutionRequest {
            conflict_id: "test_conflict_1".to_string(),
            source_branch: "feature-a".to_string(),
            target_branch: "main".to_string(),
            conflict_type: ConflictType::ContentConflict,
            source_content: "新版本内容".to_string(),
            target_content: "旧版本内容".to_string(),
            item_id: "test_item".to_string(),
            layer: "short_term".to_string(),
            source_purpose: Some("测试功能".to_string()),
            target_purpose: None,
        };

        let result = resolver.resolve_conflict(request).await.unwrap();

        assert_eq!(result.decision, MergeDecision::KeepSource);
        assert_eq!(result.confidence, 0.9);
        assert_eq!(resolver.stats.total_resolutions, 1);
        assert_eq!(resolver.stats.keep_source_count, 1);
    }

    #[tokio::test]
    async fn test_resolve_conflict_combine() {
        let response_json = r#"{
            "decision": "combine",
            "reasoning": "两个版本都有价值，需要融合",
            "combined_content": "融合后的内容",
            "confidence": 0.85,
            "suggested_strategy": "ai_assisted"
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut resolver = AIConflictResolver::new(mock_client);

        let request = ConflictResolutionRequest {
            conflict_id: "test_conflict_2".to_string(),
            source_branch: "feature-b".to_string(),
            target_branch: "main".to_string(),
            conflict_type: ConflictType::ContentConflict,
            source_content: "源分支补充内容".to_string(),
            target_content: "目标分支基础内容".to_string(),
            item_id: "test_item_2".to_string(),
            layer: "long_term".to_string(),
            source_purpose: None,
            target_purpose: None,
        };

        let result = resolver.resolve_conflict(request).await.unwrap();

        assert_eq!(result.decision, MergeDecision::Combine);
        assert!(result.combined_content.is_some());
        assert_eq!(result.combined_content.unwrap(), "融合后的内容");
        assert_eq!(resolver.stats.combine_count, 1);
    }

    #[tokio::test]
    async fn test_batch_resolution() {
        let responses = [
            r#"{"decision": "keep_source", "reasoning": "Test 1", "confidence": 0.9, "suggested_strategy": "selective_merge"}"#,
            r#"{"decision": "keep_target", "reasoning": "Test 2", "confidence": 0.8, "suggested_strategy": "selective_merge"}"#,
            r#"{"decision": "combine", "reasoning": "Test 3", "combined_content": "Combined", "confidence": 0.85, "suggested_strategy": "ai_assisted"}"#,
        ];

        let mut resolver = AIConflictResolver::new(Arc::new(MockLLMClient::new(
            responses[0]
        )));

        // 模拟多次调用
        let requests = vec![
            ConflictResolutionRequest {
                conflict_id: "conflict_1".to_string(),
                source_branch: "branch1".to_string(),
                target_branch: "main".to_string(),
                conflict_type: ConflictType::ContentConflict,
                source_content: "content1".to_string(),
                target_content: "content2".to_string(),
                item_id: "item1".to_string(),
                layer: "short_term".to_string(),
                source_purpose: None,
                target_purpose: None,
            },
            ConflictResolutionRequest {
                conflict_id: "conflict_2".to_string(),
                source_branch: "branch2".to_string(),
                target_branch: "main".to_string(),
                conflict_type: ConflictType::ContentConflict,
                source_content: "content3".to_string(),
                target_content: "content4".to_string(),
                item_id: "item2".to_string(),
                layer: "short_term".to_string(),
                source_purpose: None,
                target_purpose: None,
            },
        ];

        let results = resolver.resolve_conflicts_batch(requests).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(resolver.stats.total_resolutions, 2);
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let mut resolver = AIConflictResolver::new(Arc::new(MockLLMClient::new(
            r#"{"decision": "keep_source", "reasoning": "Test", "confidence": 0.95, "suggested_strategy": "selective_merge"}"#,
        )));

        // 模拟 5 次解决
        for i in 0..5 {
            let request = ConflictResolutionRequest {
                conflict_id: format!("conflict_{}", i),
                source_branch: "branch".to_string(),
                target_branch: "main".to_string(),
                conflict_type: ConflictType::ContentConflict,
                source_content: "source".to_string(),
                target_content: "target".to_string(),
                item_id: format!("item_{}", i),
                layer: "short_term".to_string(),
                source_purpose: None,
                target_purpose: None,
            };

            let _ = resolver.resolve_conflict(request).await.unwrap();
        }

        let stats = resolver.get_stats();

        assert_eq!(stats.total_resolutions, 5);
        assert_eq!(stats.keep_source_count, 5);
        assert!(stats.avg_confidence > 0.9);
        assert_eq!(stats.high_confidence_ratio, 1.0);
    }
}
