//! 分支目的自动推断器
//!
//! 使用 LLM 分析分支上下文内容，自动推断并标注分支目的
//!
//! # 核心功能
//! - 分析分支的对话历史和上下文内容
//! - 推断分支的目的和意图
//! - 生成简洁准确的目的描述
//! - 自动分类分支类型（feature, bugfix, experiment, research 等）
//! - 建议相关标签

use std::sync::Arc;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::parallel::BranchMetadata;
use crate::ai::client::LLMClient;

/// 分支目的推断请求
#[derive(Debug, Clone)]
pub struct PurposeInferenceRequest {
    /// 分支名称
    pub branch_name: String,
    /// 父分支名称
    pub parent_branch: String,
    /// 分支创建后的对话轮数
    pub conversation_turns: u32,
    /// 最近的对话内容（最近 N 轮）
    pub recent_conversations: Vec<String>,
    /// 分支中的关键文件/上下文项列表
    pub key_items: Vec<String>,
    /// 分支创建时的用户指令（如果有）
    pub initial_instruction: Option<String>,
}

/// 分支目的推断结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurposeInferenceResult {
    /// 推断的目的描述
    pub purpose: String,
    /// 分支类型
    pub branch_type: BranchType,
    /// 建议的标签
    pub suggested_tags: Vec<String>,
    /// 置信度（0.0-1.0）
    pub confidence: f32,
    /// 推断理由
    pub reasoning: String,
    /// 是否建议自动合并
    pub suggest_auto_merge: bool,
    /// 建议的合并策略
    pub suggested_merge_strategy: String,
}

/// 分支类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BranchType {
    /// 功能开发
    Feature,
    /// Bug 修复
    Bugfix,
    /// 实验性探索
    Experiment,
    /// 研究/调研
    Research,
    /// 重构
    Refactor,
    /// 性能优化
    Performance,
    /// 文档更新
    Documentation,
    /// 测试添加
    Testing,
    /// 配置更改
    Configuration,
    /// 其他
    Other,
}

impl std::fmt::Display for BranchType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchType::Feature => write!(f, "feature"),
            BranchType::Bugfix => write!(f, "bugfix"),
            BranchType::Experiment => write!(f, "experiment"),
            BranchType::Research => write!(f, "research"),
            BranchType::Refactor => write!(f, "refactor"),
            BranchType::Performance => write!(f, "performance"),
            BranchType::Documentation => write!(f, "documentation"),
            BranchType::Testing => write!(f, "testing"),
            BranchType::Configuration => write!(f, "configuration"),
            BranchType::Other => write!(f, "other"),
        }
    }
}

/// AI 分支目的推断器
pub struct AIPurposeInference {
    /// LLM 客户端
    llm_client: Arc<dyn LLMClient>,
    /// 推断历史
    inference_history: Vec<PurposeInferenceResult>,
    /// 统计信息
    stats: InferenceStats,
}

/// 推断统计信息
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InferenceStats {
    /// 总推断次数
    pub total_inferences: u32,
    /// 各分支类型计数
    pub feature_count: u32,
    pub bugfix_count: u32,
    pub experiment_count: u32,
    pub research_count: u32,
    pub refactor_count: u32,
    pub other_count: u32,
    /// 平均置信度
    pub avg_confidence: f32,
    /// 高置信度比例（>0.8）
    pub high_confidence_ratio: f32,
}

impl AIPurposeInference {
    /// 创建新的推断器
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            inference_history: Vec::new(),
            stats: InferenceStats::default(),
        }
    }

    /// 推断分支目的
    pub async fn infer_purpose(
        &mut self,
        request: PurposeInferenceRequest,
    ) -> Result<PurposeInferenceResult> {
        tracing::info!(
            "Inferring purpose for branch: {} (parent: {})",
            request.branch_name,
            request.parent_branch
        );

        // 构建 prompt
        let prompt = self.build_inference_prompt(&request);

        // 定义 JSON Schema
        let schema = self.get_inference_schema();

        // 调用 LLM
        let response_text = self
            .llm_client
            .chat_with_schema(&prompt, &schema)
            .await
            .context("LLM call failed for purpose inference")?;

        // 解析响应
        let result: PurposeInferenceResult =
            serde_json::from_str(&response_text).context("Failed to parse LLM response")?;

        // 验证结果
        self.validate_result(&result)?;

        // 更新统计
        self.update_stats(&result);

        // 记录历史
        self.inference_history.push(result.clone());

        tracing::info!(
            "Purpose inferred: type={}, purpose={}, confidence={:.2}",
            result.branch_type,
            result.purpose,
            result.confidence
        );

        Ok(result)
    }

    /// 快速推断（仅基于分支名称和初始指令）
    pub async fn quick_infer(
        &self,
        branch_name: &str,
        initial_instruction: Option<&str>,
    ) -> Result<PurposeInferenceResult> {
        let prompt = self.build_quick_prompt(branch_name, initial_instruction);

        let response_text = self.llm_client.chat(&prompt).await?;

        // 尝试解析为结构化结果
        let result: PurposeInferenceResult = serde_json::from_str(&response_text)
            .unwrap_or_else(|_| PurposeInferenceResult {
                purpose: format!("Branch: {}", branch_name),
                branch_type: BranchType::Other,
                suggested_tags: Vec::new(),
                confidence: 0.5,
                reasoning: "Quick inference based on branch name only".to_string(),
                suggest_auto_merge: false,
                suggested_merge_strategy: "selective_merge".to_string(),
            });

        Ok(result)
    }

    /// 构建推断 prompt
    fn build_inference_prompt(&self, request: &PurposeInferenceRequest) -> String {
        let conversations_context = if request.recent_conversations.is_empty() {
            "无对话内容".to_string()
        } else {
            request
                .recent_conversations
                .iter()
                .enumerate()
                .map(|(i, c)| format!("{}. {}", i + 1, c))
                .collect::<Vec<_>>()
                .join("\n")
        };

        let key_items_context = if request.key_items.is_empty() {
            "无关键项目".to_string()
        } else {
            request.key_items.join(", ")
        };

        let initial_instruction_context = match &request.initial_instruction {
            Some(instruction) => instruction.clone(),
            None => "无初始指令".to_string(),
        };

        format!(
            r#"# 分支目的推断任务

你是一个 AI Agent 内存系统的分支分析专家。你的任务是分析分支的上下文内容，推断分支的目的和类型。

## 分支信息
- **分支名称**: {branch_name}
- **父分支**: {parent_branch}
- **对话轮数**: {conversation_turns}

## 初始指令
{initial_instruction}

## 最近对话内容
{conversations}

## 关键项目/文件
{key_items}

## 任务要求

请分析上述信息，并以 JSON 格式返回你的推断：

```json
{{
    "purpose": "简洁明了的目的描述（50 字以内）",
    "branch_type": "Feature|Bugfix|Experiment|Research|Refactor|Performance|Documentation|Testing|Configuration|Other",
    "suggested_tags": ["标签 1", "标签 2", ...],
    "confidence": 0.0-1.0 之间的浮点数",
    "reasoning": "详细的推断理由（100 字以内）",
    "suggest_auto_merge": true/false,
    "suggested_merge_strategy": "建议的合并策略名称"
}}
```

## 分支类型定义

- **Feature**: 新功能开发或功能增强
- **Bugfix**: 修复错误或问题
- **Experiment**: 实验性尝试，可能不成功
- **Research**: 调研、探索性研究
- **Refactor**: 代码重构，不改变功能
- **Performance**: 性能优化
- **Documentation**: 文档更新
- **Testing**: 添加或修改测试
- **Configuration**: 配置文件更改
- **Other**: 其他类型

## 判断指南

- 如果分支名称包含明确意图（如 "fix-", "feature-", "refactor-"），应给予重视
- 如果对话内容显示探索性质，建议 Experiment 或 Research
- 如果有明确的功能开发痕迹，选择 Feature
- 如果涉及错误修复，选择 Bugfix
- 高置信度（>0.8）时建议自动合并
- 实验性分支通常不建议自动合并

现在请分析并返回 JSON 结果：
"#,
            branch_name = request.branch_name,
            parent_branch = request.parent_branch,
            conversation_turns = request.conversation_turns,
            initial_instruction = initial_instruction_context,
            conversations = conversations_context,
            key_items = key_items_context,
        )
    }

    /// 构建快速推断 prompt
    fn build_quick_prompt(
        &self,
        branch_name: &str,
        initial_instruction: Option<&str>,
    ) -> String {
        let instruction_context = initial_instruction.unwrap_or("无");

        format!(
            r#"# 快速分支目的推断

基于分支名称和初始指令快速推断分支目的。

## 分支名称
{branch_name}

## 初始指令
{instruction}

## 任务要求

请以 JSON 格式返回推断结果：

```json
{{
    "purpose": "简洁的目的描述",
    "branch_type": "Feature|Bugfix|Experiment|Research|Refactor|Performance|Documentation|Testing|Configuration|Other",
    "suggested_tags": [],
    "confidence": 0.5,
    "reasoning": "简短理由",
    "suggest_auto_merge": false,
    "suggested_merge_strategy": "selective_merge"
}}
```

现在请推断：
"#,
            branch_name = branch_name,
            instruction = instruction_context,
        )
    }

    /// 获取 JSON Schema
    fn get_inference_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "purpose": {
                    "type": "string",
                    "description": "分支目的描述"
                },
                "branch_type": {
                    "type": "string",
                    "enum": [
                        "Feature", "Bugfix", "Experiment", "Research",
                        "Refactor", "Performance", "Documentation",
                        "Testing", "Configuration", "Other"
                    ]
                },
                "suggested_tags": {
                    "type": "array",
                    "items": { "type": "string" }
                },
                "confidence": {
                    "type": "number",
                    "minimum": 0.0,
                    "maximum": 1.0
                },
                "reasoning": {
                    "type": "string",
                    "description": "推断理由"
                },
                "suggest_auto_merge": {
                    "type": "boolean",
                    "description": "是否建议自动合并"
                },
                "suggested_merge_strategy": {
                    "type": "string",
                    "description": "建议的合并策略"
                }
            },
            "required": [
                "purpose", "branch_type", "suggested_tags",
                "confidence", "reasoning", "suggest_auto_merge",
                "suggested_merge_strategy"
            ]
        })
    }

    /// 验证结果
    fn validate_result(&self, result: &PurposeInferenceResult) -> Result<()> {
        // 验证置信度范围
        if result.confidence < 0.0 || result.confidence > 1.0 {
            anyhow::bail!("Confidence must be between 0.0 and 1.0");
        }

        // 验证目的描述长度
        if result.purpose.len() > 200 {
            anyhow::bail!("Purpose description too long (max 200 chars)");
        }

        Ok(())
    }

    /// 更新统计信息
    fn update_stats(&mut self, result: &PurposeInferenceResult) {
        self.stats.total_inferences += 1;

        match result.branch_type {
            BranchType::Feature => self.stats.feature_count += 1,
            BranchType::Bugfix => self.stats.bugfix_count += 1,
            BranchType::Experiment => self.stats.experiment_count += 1,
            BranchType::Research => self.stats.research_count += 1,
            BranchType::Refactor => self.stats.refactor_count += 1,
            _ => self.stats.other_count += 1,
        }

        // 更新平均置信度
        let total = self.stats.total_inferences as f32;
        self.stats.avg_confidence =
            ((self.stats.avg_confidence * (total - 1.0)) + result.confidence) / total;

        // 更新高置信度比例
        let high_conf_count = if result.confidence > 0.8 { 1 } else { 0 };
        let prev_high_conf =
            (self.stats.high_confidence_ratio * (total - 1.0)).round() as u32;
        self.stats.high_confidence_ratio =
            ((prev_high_conf + high_conf_count) as f32) / total;
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> InferenceStats {
        self.stats.clone()
    }

    /// 更新分支元数据
    pub fn update_branch_metadata(
        &self,
        result: &PurposeInferenceResult,
        metadata: &mut BranchMetadata,
    ) {
        metadata.purpose = Some(result.purpose.clone());
        metadata.tags = result.suggested_tags.clone();
        metadata.auto_merge = result.suggest_auto_merge;
        metadata.merge_strategy = match result.suggested_merge_strategy.as_str() {
            "fast_forward" => crate::parallel::MergeStrategy::FastForward,
            "ai_assisted" => crate::parallel::MergeStrategy::AIAssisted,
            "manual" => crate::parallel::MergeStrategy::Manual,
            "ours" => crate::parallel::MergeStrategy::Ours,
            "theirs" => crate::parallel::MergeStrategy::Theirs,
            _ => crate::parallel::MergeStrategy::SelectiveMerge,
        };
    }
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
    async fn test_inference_creation() {
        let mock_client = Arc::new(MockLLMClient::new(r#"{"purpose": "Test", "branch_type": "feature", "suggested_tags": [], "confidence": 0.9, "reasoning": "Test", "suggest_auto_merge": true, "suggested_merge_strategy": "selective_merge"}"#));
        let inference = AIPurposeInference::new(mock_client);

        assert_eq!(inference.stats.total_inferences, 0);
    }

    #[tokio::test]
    async fn test_infer_feature_branch() {
        let response_json = r#"{
            "purpose": "实现用户认证功能",
            "branch_type": "feature",
            "suggested_tags": ["authentication", "security", "user-management"],
            "confidence": 0.92,
            "reasoning": "分支名称包含 'feature-auth'，对话内容涉及登录、注册等功能开发",
            "suggest_auto_merge": true,
            "suggested_merge_strategy": "selective_merge"
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut inference = AIPurposeInference::new(mock_client);

        let request = PurposeInferenceRequest {
            branch_name: "feature-auth".to_string(),
            parent_branch: "main".to_string(),
            conversation_turns: 5,
            recent_conversations: vec![
                "用户要求实现登录功能".to_string(),
                "讨论 JWT token 验证".to_string(),
                "实现密码加密存储".to_string(),
            ],
            key_items: vec!["auth.rs".to_string(), "login_handler.json".to_string()],
            initial_instruction: Some("帮我实现用户认证功能".to_string()),
        };

        let result = inference.infer_purpose(request).await.unwrap();

        assert_eq!(result.branch_type, BranchType::Feature);
        assert!(result.confidence > 0.9);
        assert!(!result.suggested_tags.is_empty());
        assert_eq!(inference.stats.total_inferences, 1);
        assert_eq!(inference.stats.feature_count, 1);
    }

    #[tokio::test]
    async fn test_infer_bugfix_branch() {
        let response_json = r#"{
            "purpose": "修复空指针异常",
            "branch_type": "bugfix",
            "suggested_tags": ["bugfix", "crash", "null-pointer"],
            "confidence": 0.88,
            "reasoning": "分支名称 'fix-null-pointer' 明确表明是 bug 修复，对话讨论错误日志",
            "suggest_auto_merge": true,
            "suggested_merge_strategy": "fast_forward"
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut inference = AIPurposeInference::new(mock_client);

        let request = PurposeInferenceRequest {
            branch_name: "fix-null-pointer".to_string(),
            parent_branch: "main".to_string(),
            conversation_turns: 3,
            recent_conversations: vec![
                "发现空指针崩溃".to_string(),
                "定位问题到用户数据解析".to_string(),
                "添加空值检查".to_string(),
            ],
            key_items: vec!["user_parser.rs".to_string()],
            initial_instruction: Some("修复这个崩溃问题".to_string()),
        };

        let result = inference.infer_purpose(request).await.unwrap();

        assert_eq!(result.branch_type, BranchType::Bugfix);
        assert!(result.confidence > 0.8);
    }

    #[tokio::test]
    async fn test_infer_experiment_branch() {
        let response_json = r#"{
            "purpose": "尝试新的缓存策略",
            "branch_type": "experiment",
            "suggested_tags": ["experiment", "cache", "performance"],
            "confidence": 0.85,
            "reasoning": "对话内容显示探索性质，使用'试试'、'实验'等词汇",
            "suggest_auto_merge": false,
            "suggested_merge_strategy": "manual"
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let mut inference = AIPurposeInference::new(mock_client);

        let request = PurposeInferenceRequest {
            branch_name: "exp-cache-strategy".to_string(),
            parent_branch: "main".to_string(),
            conversation_turns: 4,
            recent_conversations: vec![
                "想试试 LRU 缓存策略".to_string(),
                "实验不同的淘汰算法".to_string(),
                "测试性能提升".to_string(),
            ],
            key_items: vec!["cache.rs".to_string()],
            initial_instruction: Some("帮我实验一下新的缓存策略".to_string()),
        };

        let result = inference.infer_purpose(request).await.unwrap();

        assert_eq!(result.branch_type, BranchType::Experiment);
        assert!(!result.suggest_auto_merge);
        assert_eq!(inference.stats.experiment_count, 1);
    }

    #[tokio::test]
    async fn test_quick_inference() {
        let response_json = r#"{
            "purpose": "功能开发分支",
            "branch_type": "feature",
            "suggested_tags": ["feature"],
            "confidence": 0.6,
            "reasoning": "基于分支名称前缀 'feature-'",
            "suggest_auto_merge": false,
            "suggested_merge_strategy": "selective_merge"
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let inference = AIPurposeInference::new(mock_client);

        let result = inference
            .quick_infer("feature-new-api", Some("添加新 API"))
            .await
            .unwrap();

        assert_eq!(result.branch_type, BranchType::Feature);
        assert!(result.confidence < 0.7); // 快速推断置信度较低
    }

    #[tokio::test]
    async fn test_update_metadata() {
        let response_json = r#"{
            "purpose": "测试功能",
            "branch_type": "testing",
            "suggested_tags": ["test", "unit-test"],
            "confidence": 0.9,
            "reasoning": "测试",
            "suggest_auto_merge": true,
            "suggested_merge_strategy": "ai_assisted"
        }"#;

        let mock_client = Arc::new(MockLLMClient::new(response_json));
        let inference = AIPurposeInference::new(mock_client);

        let mut metadata = BranchMetadata::default();
        let result = PurposeInferenceResult {
            purpose: "测试功能".to_string(),
            branch_type: BranchType::Testing,
            suggested_tags: vec!["test".to_string(), "unit-test".to_string()],
            confidence: 0.9,
            reasoning: "测试".to_string(),
            suggest_auto_merge: true,
            suggested_merge_strategy: "ai_assisted".to_string(),
        };

        inference.update_branch_metadata(&result, &mut metadata);

        assert_eq!(metadata.purpose, Some("测试功能".to_string()));
        assert_eq!(metadata.tags.len(), 2);
        assert!(metadata.auto_merge);
    }

    #[tokio::test]
    async fn test_stats_tracking() {
        let mut inference = AIPurposeInference::new(Arc::new(MockLLMClient::new(
            r#"{"purpose": "Test", "branch_type": "feature", "suggested_tags": [], "confidence": 0.9, "reasoning": "Test", "suggest_auto_merge": true, "suggested_merge_strategy": "selective_merge"}"#,
        )));

        // 模拟 5 次推断
        for _ in 0..5 {
            let request = PurposeInferenceRequest {
                branch_name: "test-branch".to_string(),
                parent_branch: "main".to_string(),
                conversation_turns: 1,
                recent_conversations: vec!["test".to_string()],
                key_items: vec![],
                initial_instruction: None,
            };

            let _ = inference.infer_purpose(request).await.unwrap();
        }

        let stats = inference.get_stats();

        assert_eq!(stats.total_inferences, 5);
        assert_eq!(stats.feature_count, 5);
        assert!(stats.avg_confidence > 0.8);
    }
}
