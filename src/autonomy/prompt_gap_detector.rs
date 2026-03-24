//! 基于 Prompt Engineering 的因果推理缺口检测器
//!
//! 使用 Chain-of-Thought + 反事实推理 + JSON Schema 约束，
//! 让 LLM 进行因果分析而非简单的相关性分析
//!
//! ## 核心创新
//! - **因果推理 Prompt**: 通过反事实提问识别真正的因果缺口
//! - **Few-Shot 示例**: 历史成功决策作为学习样本
//! - **JSON Schema 约束**: 确保输出格式稳定可靠
//! - **验证器**: 规则验证确保合理性
//!
//! ## 使用示例
//! ```rust,ignore
//! let detector = PromptGapDetector::new(llm_client, task_history)?;
//! let gaps = detector.detect_gaps().await?;
//! for gap in gaps {
//!     println!("检测到工具缺口：{}", gap.description);
//! }
//! ```

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use anyhow::{Context, Result};
use tracing::{info, warn};

/// 因果分析请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAnalysisRequest {
    /// 任务 ID
    pub task_id: String,
    /// 任务描述
    pub task_description: String,
    /// 是否成功
    pub success: bool,
    /// 使用的工具列表
    pub used_tools: Vec<String>,
    /// 失败原因（如果失败）
    pub failure_reason: Option<String>,
    /// 用户满意度 (1-5)
    pub user_satisfaction: Option<u8>,
    /// 执行时间 (ms)
    pub execution_time_ms: u64,
    /// 上下文信息
    pub context: Option<String>,
}

/// 因果因素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalFactor {
    /// 因素描述
    pub factor: String,
    /// 是否为因果性（而非相关性）
    pub is_causal: bool,
    /// 证据描述
    pub evidence: String,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
    /// 推理过程
    pub reasoning: String,
}

/// 识别的工具缺口
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifiedGap {
    /// 缺口类型
    pub gap_type: String,
    /// 缺口描述
    pub description: String,
    /// 建议的工具名称
    pub suggested_name: Option<String>,
    /// 建议的功能描述
    pub suggested_functionality: String,
    /// 输入 Schema（JSON Schema 格式）
    pub input_schema: Option<serde_json::Value>,
    /// 优先级 (1-10)
    pub priority: u8,
    /// 因果证据
    pub causal_evidence: Vec<CausalFactor>,
    /// 预期影响
    pub expected_impact: GapImpact,
}

/// 缺口影响评估
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapImpact {
    /// 影响的任务数量
    pub affected_tasks: u32,
    /// 平均减少的工具调用次数
    pub avg_tool_calls_reduced: f32,
    /// 预计节省的时间（分钟）
    pub time_saved_minutes: f32,
    /// 预期成功率提升
    pub expected_success_rate_improvement: f32,
}

/// 因果分析响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalAnalysisResponse {
    /// 因果因素列表
    pub causal_factors: Vec<CausalFactor>,
    /// 识别的缺口列表
    pub identified_gaps: Vec<IdentifiedGap>,
    /// 整体置信度 (0.0-1.0)
    pub overall_confidence: f32,
    /// 分析元数据
    pub metadata: AnalysisMetadata,
}

/// 分析元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    /// 分析的任务数量
    pub tasks_analyzed: u32,
    /// 失败任务数量
    pub failed_tasks: u32,
    /// 低满意度任务数量
    pub low_satisfaction_tasks: u32,
    /// 分析时间戳
    pub timestamp: u64,
}

/// Few-Shot 示例
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FewShotExample {
    /// 示例任务
    pub task: String,
    /// 任务结果
    pub outcome: String,
    /// 因果分析
    pub causal_analysis: String,
    /// 识别的缺口
    pub identified_gap: IdentifiedGap,
}

/// 因果推理 Prompt 模板
pub const CAUSAL_ANALYSIS_PROMPT: &str = r#"你是因果推断专家。请分析以下任务失败的根本原因，识别真正的工具缺口。

## 分析步骤

请按以下步骤进行 Chain-of-Thought 推理：

### 步骤 1: 列出所有可能的失败因素
- 工具缺失
- 工具功能不足
- 工具使用错误
- 外部因素（网络、权限等）

### 步骤 2: 对每个因素进行因果判断
对于每个因素，问自己：
- 这是相关性还是因果性？
- 如果消除这个因素，任务会成功吗？（反事实推理）
- 有其他混淆变量吗？

### 步骤 3: 识别真正的工具缺口
- 缺少的工具是什么？
- 如果有这个工具，任务会成功吗？
- 这个工具的具体功能应该是什么？

### 步骤 4: 输出 JSON 格式报告

## 任务历史

{task_history}

## Few-Shot 示例

{few_shot_examples}

## 输出格式

请输出严格的 JSON 格式（不要 Markdown 标记）：

{{
    "causal_factors": [
        {{
            "factor": "因素描述",
            "is_causal": true/false,
            "evidence": "证据描述",
            "confidence": 0.0-1.0,
            "reasoning": "推理过程"
        }}
    ],
    "identified_gaps": [
        {{
            "gap_type": "missing_tool|insufficient_capability|combination_gap",
            "description": "缺口描述",
            "suggested_name": "建议的工具名称",
            "suggested_functionality": "功能描述",
            "input_schema": {{...}},
            "priority": 1-10,
            "causal_evidence": [...],
            "expected_impact": {{
                "affected_tasks": 0,
                "avg_tool_calls_reduced": 0.0,
                "time_saved_minutes": 0.0,
                "expected_success_rate_improvement": 0.0
            }}
        }}
    ],
    "overall_confidence": 0.0-1.0,
    "metadata": {{
        "tasks_analyzed": 0,
        "failed_tasks": 0,
        "low_satisfaction_tasks": 0,
        "timestamp": 0
    }}
}}
"#;

/// 默认 Few-Shot 示例库
fn get_default_few_shot_examples() -> Vec<FewShotExample> {
    vec![
        FewShotExample {
            task: "批量下载 100 张图片并压缩".to_string(),
            outcome: "失败：手动逐个下载，耗时 30 分钟，用户满意度 2/5".to_string(),
            causal_analysis: r#"
1. 可能因素：
   - 缺少批量下载工具 → 因果性（如果有批量下载，任务会快 10 倍）
   - 网络速度慢 → 相关性（即使快也需手动操作 100 次）

2. 反事实推理：
   - 如果有 batch_download 工具，用户可以一行代码完成下载
   - 工具调用从 200 次减少到 2 次

3. 真正的工具缺口：
   - batch_download: 根据 URL 模式批量下载文件
"#.to_string(),
            identified_gap: IdentifiedGap {
                gap_type: "missing_tool".to_string(),
                description: "缺少批量下载文件的工具".to_string(),
                suggested_name: Some("batch_download".to_string()),
                suggested_functionality: "根据 URL 模式批量下载多个文件".to_string(),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "url_pattern": {"type": "string", "description": "URL 模式，如 http://example.com/img_{001-100}.jpg"},
                        "output_dir": {"type": "string"}
                    },
                    "required": ["url_pattern"]
                })),
                priority: 9,
                causal_evidence: vec![CausalFactor {
                    factor: "缺少批量下载工具".to_string(),
                    is_causal: true,
                    evidence: "15 个任务因缺少批量下载失败".to_string(),
                    confidence: 0.92,
                    reasoning: "如果有 batch_download，工具调用从 200 次减少到 2 次".to_string(),
                }],
                expected_impact: GapImpact {
                    affected_tasks: 15,
                    avg_tool_calls_reduced: 45.0,
                    time_saved_minutes: 27.0,
                    expected_success_rate_improvement: 0.8,
                },
            },
        },
        FewShotExample {
            task: "分析 JSON 数据并提取特定字段".to_string(),
            outcome: "低效：手动解析，容易出错，用户满意度 3/5".to_string(),
            causal_analysis: r#"
1. 可能因素：
   - 缺少 JSON 查询工具 → 因果性
   - 用户不熟悉 JSON 格式 → 相关性

2. 反事实推理：
   - 如果有 json_query 工具，用户可以用类似 SQL 的语法查询
   - 错误率从 30% 降低到 5%

3. 真正的工具缺口：
   - json_query: 使用路径表达式查询 JSON 数据
"#.to_string(),
            identified_gap: IdentifiedGap {
                gap_type: "insufficient_capability".to_string(),
                description: "缺少便捷的 JSON 查询工具".to_string(),
                suggested_name: Some("json_query".to_string()),
                suggested_functionality: "使用路径表达式（如$.data.items[0].name）查询 JSON 数据".to_string(),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "json": {"type": "string", "description": "JSON 字符串"},
                        "query": {"type": "string", "description": "查询路径，如 $.data.items"}
                    },
                    "required": ["json", "query"]
                })),
                priority: 7,
                causal_evidence: vec![CausalFactor {
                    factor: "缺少 JSON 查询工具".to_string(),
                    is_causal: true,
                    evidence: "10 个任务涉及 JSON 处理，平均耗时 5 分钟".to_string(),
                    confidence: 0.85,
                    reasoning: "json_query 可以将 5 分钟的任务减少到 10 秒".to_string(),
                }],
                expected_impact: GapImpact {
                    affected_tasks: 10,
                    avg_tool_calls_reduced: 5.0,
                    time_saved_minutes: 4.5,
                    expected_success_rate_improvement: 0.6,
                },
            },
        },
    ]
}

/// LLM 客户端 trait（简化版，实际项目可复用现有 LLM 抽象）
#[async_trait::async_trait]
pub trait LLMClient: Send + Sync {
    /// 发送聊天请求并获取响应
    async fn chat(&self, prompt: &str) -> Result<String>;
    
    /// 发送带 JSON Schema 约束的聊天请求
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String>;
}

/// 基于 Prompt Engineering 的缺口检测器
pub struct PromptGapDetector {
    /// LLM 客户端
    llm_client: Arc<dyn LLMClient>,
    /// 任务历史记录
    task_history: Vec<CausalAnalysisRequest>,
    /// Few-Shot 示例库
    few_shot_examples: Vec<FewShotExample>,
    /// 验证器
    validator: GapValidator,
    /// 配置
    config: PromptGapDetectorConfig,
}

/// 检测器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptGapDetectorConfig {
    /// 失败任务权重
    pub failure_weight: f32,
    /// 低满意度任务权重
    pub low_satisfaction_weight: f32,
    /// 低效率任务权重
    pub inefficiency_weight: f32,
    /// 最小置信度阈值
    pub min_confidence_threshold: f32,
    /// 最大迭代修正次数
    pub max_fix_iterations: u32,
}

impl Default for PromptGapDetectorConfig {
    fn default() -> Self {
        Self {
            failure_weight: 0.5,
            low_satisfaction_weight: 0.3,
            inefficiency_weight: 0.2,
            min_confidence_threshold: 0.6,
            max_fix_iterations: 3,
        }
    }
}

/// 缺口验证器
pub struct GapValidator {
    /// 最小优先级阈值
    pub min_priority: u8,
    /// 最大优先级
    pub max_priority: u8,
    /// 必需的字段列表
    pub required_fields: Vec<String>,
}

impl Default for GapValidator {
    fn default() -> Self {
        Self {
            min_priority: 1,
            max_priority: 10,
            required_fields: vec![
                "gap_type".to_string(),
                "description".to_string(),
                "suggested_functionality".to_string(),
            ],
        }
    }
}

impl GapValidator {
    /// 验证缺口的合理性
    pub fn validate(&self, gap: &IdentifiedGap) -> Result<bool> {
        // 验证优先级范围
        if gap.priority < self.min_priority || gap.priority > self.max_priority {
            anyhow::bail!("优先级超出范围：{}", gap.priority);
        }

        // 验证必需字段
        if gap.description.is_empty() {
            anyhow::bail!("缺口描述不能为空");
        }

        if gap.suggested_functionality.is_empty() {
            anyhow::bail!("功能描述不能为空");
        }

        // 验证因果证据
        if gap.causal_evidence.is_empty() {
            anyhow::bail!("缺少因果证据");
        }

        // 验证至少有一个因果性因素
        let has_causal = gap.causal_evidence.iter().any(|e| e.is_causal);
        if !has_causal {
            anyhow::bail!("没有识别出因果性因素");
        }

        Ok(true)
    }

    /// 验证整体响应
    pub fn validate_response(&self, response: &CausalAnalysisResponse) -> Result<bool> {
        if response.overall_confidence < 0.5 {
            warn!("整体置信度过低：{}", response.overall_confidence);
        }

        for gap in &response.identified_gaps {
            self.validate(gap)?;
        }

        Ok(true)
    }
}

impl PromptGapDetector {
    /// 创建新的检测器
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            task_history: Vec::new(),
            few_shot_examples: get_default_few_shot_examples(),
            validator: GapValidator::default(),
            config: PromptGapDetectorConfig::default(),
        }
    }

    /// 从配置创建
    pub fn with_config(llm_client: Arc<dyn LLMClient>, config: PromptGapDetectorConfig) -> Self {
        Self {
            llm_client,
            task_history: Vec::new(),
            few_shot_examples: get_default_few_shot_examples(),
            validator: GapValidator::default(),
            config,
        }
    }

    /// 添加任务到历史记录
    pub fn add_task(&mut self, task: CausalAnalysisRequest) {
        self.task_history.push(task);
    }

    /// 批量添加任务
    pub fn add_tasks(&mut self, tasks: Vec<CausalAnalysisRequest>) {
        self.task_history.extend(tasks);
    }

    /// 检测工具缺口
    pub async fn detect_gaps(&self) -> Result<Vec<IdentifiedGap>> {
        if self.task_history.is_empty() {
            warn!("任务历史为空，无法进行检测");
            return Ok(Vec::new());
        }

        info!("开始因果推理缺口检测，分析{}个任务...", self.task_history.len());

        // 1. 构建 Prompt
        let prompt = self.build_causal_prompt();

        // 2. LLM 推理（带 JSON Schema 约束）
        let schema = self.get_response_schema();
        let mut response_text = self.llm_client
            .chat_with_schema(&prompt, &schema)
            .await
            .context("LLM 推理失败")?;

        // 3. 尝试解析响应
        let response: CausalAnalysisResponse = match serde_json::from_str(&response_text) {
            Ok(r) => r,
            Err(e) => {
                warn!("首次解析失败：{}，尝试修复...", e);
                response_text = self.fix_json_response(&response_text).await?;
                serde_json::from_str(&response_text)
                    .context("修复后仍无法解析 JSON")?
            }
        };

        // 4. 验证响应
        self.validator.validate_response(&response)?;

        // 5. 过滤低置信度缺口
        let filtered_gaps: Vec<_> = response.identified_gaps
            .iter()
            .filter(|gap| {
                gap.causal_evidence.iter()
                    .any(|e| e.is_causal && e.confidence >= self.config.min_confidence_threshold)
            })
            .cloned()
            .collect();

        info!(
            "检测完成：识别{}个缺口，过滤后剩余{}个",
            response.identified_gaps.len(),
            filtered_gaps.len()
        );

        Ok(filtered_gaps)
    }

    /// 构建因果推理 Prompt
    fn build_causal_prompt(&self) -> String {
        // 格式化任务历史
        let task_history_str = self.task_history.iter()
            .map(|t| self.format_task_record(t))
            .collect::<Vec<_>>()
            .join("\n\n");

        // 格式化 Few-Shot 示例
        let few_shot_str = self.few_shot_examples.iter()
            .take(2)
            .map(|e| self.format_few_shot_example(e))
            .collect::<Vec<_>>()
            .join("\n\n");

        CAUSAL_ANALYSIS_PROMPT
            .replace("{task_history}", &task_history_str)
            .replace("{few_shot_examples}", &few_shot_str)
    }

    /// 格式化任务记录
    fn format_task_record(&self, task: &CausalAnalysisRequest) -> String {
        let status = if task.success { "成功" } else { "失败" };
        let satisfaction = task.user_satisfaction.map_or("未知".to_string(), |s| format!("{}/5", s));

        format!(
            r#"### 任务 {}
- **描述**: {}
- **状态**: {}
- **使用工具**: {}
- **满意度**: {}
- **耗时**: {}ms{}"#,
            task.task_id,
            task.task_description,
            status,
            task.used_tools.join(", "),
            satisfaction,
            task.execution_time_ms,
            task.failure_reason.as_ref().map_or(String::new(), |r| format!("\n- **失败原因**: {}", r)),
        )
    }

    /// 格式化 Few-Shot 示例
    fn format_few_shot_example(&self, example: &FewShotExample) -> String {
        format!(
            r#"#### 示例任务
任务：{}
结果：{}
因果分析：{}
识别缺口：{}"#,
            example.task,
            example.outcome,
            example.causal_analysis,
            serde_json::to_string(&example.identified_gap).unwrap_or_default(),
        )
    }

    /// 获取响应 JSON Schema
    fn get_response_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["causal_factors", "identified_gaps", "overall_confidence", "metadata"],
            "properties": {
                "causal_factors": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["factor", "is_causal", "evidence", "confidence", "reasoning"],
                        "properties": {
                            "factor": {"type": "string"},
                            "is_causal": {"type": "boolean"},
                            "evidence": {"type": "string"},
                            "confidence": {"type": "number"},
                            "reasoning": {"type": "string"}
                        }
                    }
                },
                "identified_gaps": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["gap_type", "description", "suggested_functionality", "priority", "causal_evidence", "expected_impact"],
                        "properties": {
                            "gap_type": {"type": "string"},
                            "description": {"type": "string"},
                            "suggested_name": {"type": "string"},
                            "suggested_functionality": {"type": "string"},
                            "input_schema": {"type": "object"},
                            "priority": {"type": "integer"},
                            "causal_evidence": {"type": "array"},
                            "expected_impact": {
                                "type": "object",
                                "required": ["affected_tasks", "avg_tool_calls_reduced", "time_saved_minutes", "expected_success_rate_improvement"],
                                "properties": {
                                    "affected_tasks": {"type": "integer"},
                                    "avg_tool_calls_reduced": {"type": "number"},
                                    "time_saved_minutes": {"type": "number"},
                                    "expected_success_rate_improvement": {"type": "number"}
                                }
                            }
                        }
                    }
                },
                "overall_confidence": {"type": "number"},
                "metadata": {
                    "type": "object",
                    "required": ["tasks_analyzed", "failed_tasks", "low_satisfaction_tasks", "timestamp"],
                    "properties": {
                        "tasks_analyzed": {"type": "integer"},
                        "failed_tasks": {"type": "integer"},
                        "low_satisfaction_tasks": {"type": "integer"},
                        "timestamp": {"type": "integer"}
                    }
                }
            }
        })
    }

    /// 修复 JSON 响应（处理常见的格式问题）
    async fn fix_json_response(&self, broken_json: &str) -> Result<String> {
        let fix_prompt = format!(
            r#"请修复以下 JSON 的格式错误（不要改变内容，只修复语法）：

```json
{}
```

输出修复后的完整 JSON（不要 Markdown 标记）："#,
            broken_json
        );

        self.llm_client.chat(&fix_prompt).await
    }

    /// 获取任务统计
    pub fn get_task_stats(&self) -> TaskStats {
        let total = self.task_history.len() as u32;
        let failed = self.task_history.iter().filter(|t| !t.success).count() as u32;
        let low_sat = self.task_history.iter()
            .filter(|t| t.user_satisfaction.is_some_and(|s| s <= 2))
            .count() as u32;
        let inefficient = self.task_history.iter()
            .filter(|t| t.execution_time_ms > 5000)
            .count() as u32;

        TaskStats {
            total,
            failed,
            low_satisfaction: low_sat,
            inefficient,
        }
    }
}

/// 任务统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStats {
    /// 总任务数
    pub total: u32,
    /// 失败任务数
    pub failed: u32,
    /// 低满意度任务数
    pub low_satisfaction: u32,
    /// 低效率任务数
    pub inefficient: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// 测试用 Mock LLM 客户端
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

    #[async_trait::async_trait]
    impl LLMClient for MockLLMClient {
        async fn chat(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }

        async fn chat_with_schema(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String> {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn test_detector_creation() {
        let mock_llm = Arc::new(MockLLMClient::new("{}"));
        let detector = PromptGapDetector::new(mock_llm);
        assert_eq!(detector.task_history.len(), 0);
    }

    #[tokio::test]
    async fn test_add_task() {
        let mock_llm = Arc::new(MockLLMClient::new("{}"));
        let mut detector = PromptGapDetector::new(mock_llm);

        detector.add_task(CausalAnalysisRequest {
            task_id: "test_1".to_string(),
            task_description: "Test task".to_string(),
            success: false,
            used_tools: vec![],
            failure_reason: Some("Test failure".to_string()),
            user_satisfaction: Some(1),
            execution_time_ms: 100,
            context: None,
        });

        assert_eq!(detector.task_history.len(), 1);
    }

    #[tokio::test]
    async fn test_get_task_stats() {
        let mock_llm = Arc::new(MockLLMClient::new("{}"));
        let mut detector = PromptGapDetector::new(mock_llm);

        // 添加成功任务
        detector.add_task(CausalAnalysisRequest {
            task_id: "success_1".to_string(),
            task_description: "Success".to_string(),
            success: true,
            used_tools: vec!["tool_a".to_string()],
            failure_reason: None,
            user_satisfaction: Some(5),
            execution_time_ms: 100,
            context: None,
        });

        // 添加失败任务
        detector.add_task(CausalAnalysisRequest {
            task_id: "fail_1".to_string(),
            task_description: "Failure".to_string(),
            success: false,
            used_tools: vec![],
            failure_reason: Some("Error".to_string()),
            user_satisfaction: Some(1),
            execution_time_ms: 5000,
            context: None,
        });

        let stats = detector.get_task_stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.low_satisfaction, 1);
    }

    #[tokio::test]
    async fn test_validator() {
        let validator = GapValidator::default();

        let valid_gap = IdentifiedGap {
            gap_type: "missing_tool".to_string(),
            description: "Test gap".to_string(),
            suggested_name: Some("test_tool".to_string()),
            suggested_functionality: "Test functionality".to_string(),
            input_schema: None,
            priority: 5,
            causal_evidence: vec![CausalFactor {
                factor: "Test".to_string(),
                is_causal: true,
                evidence: "Test".to_string(),
                confidence: 0.8,
                reasoning: "Test".to_string(),
            }],
            expected_impact: GapImpact {
                affected_tasks: 1,
                avg_tool_calls_reduced: 1.0,
                time_saved_minutes: 1.0,
                expected_success_rate_improvement: 0.1,
            },
        };

        assert!(validator.validate(&valid_gap).is_ok());

        let invalid_gap = IdentifiedGap {
            gap_type: "missing_tool".to_string(),
            description: "".to_string(), // 空描述
            suggested_name: None,
            suggested_functionality: "".to_string(), // 空功能
            input_schema: None,
            priority: 5,
            causal_evidence: vec![], // 无证据
            expected_impact: GapImpact {
                affected_tasks: 0,
                avg_tool_calls_reduced: 0.0,
                time_saved_minutes: 0.0,
                expected_success_rate_improvement: 0.0,
            },
        };

        assert!(validator.validate(&invalid_gap).is_err());
    }
}
