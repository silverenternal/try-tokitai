//! AI 工具箱分类器
//!
//! AI 自主管理工具箱体系：
//! - 为新工具选择或创建合适的工具箱
//! - AI 生成工具箱摘要
//! - 工具箱缓存优化
//!
//! # 设计原则
//! - 工具箱不是预先设计的，而是 AI 在创造工具过程中自然演化的
//! - 减少人工干预，让 AI 自主管理工具索引、分类和依赖关系

#![allow(dead_code)]

use crate::tool_matrix::matrix::{ToolDefinition, ToolBox};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use tracing::{info, warn, debug};

// ============================================================================
// 工具箱分配结果
// ============================================================================

/// 工具箱分配结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolboxAssignment {
    /// 分配动作
    pub action: ToolboxAction,
    /// 现有工具箱 ID（如果放入现有）
    pub toolbox_id: Option<String>,
    /// 新工具箱信息（如果创建新的）
    pub new_toolbox: Option<NewToolbox>,
    /// 置信度（0-1）
    pub confidence: f32,
    /// 分类理由
    pub reason: String,
}

/// 工具箱分配动作
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolboxAction {
    /// 添加到现有工具箱
    AddToExisting,
    /// 创建新工具箱
    CreateNew,
}

/// 新工具箱信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewToolbox {
    /// 工具箱名称
    pub name: String,
    /// 工具箱描述
    pub description: String,
    /// 使用场景
    pub use_cases: Vec<String>,
}

impl Default for ToolboxAssignment {
    fn default() -> Self {
        Self {
            action: ToolboxAction::AddToExisting,
            toolbox_id: Some("utility".to_string()),
            new_toolbox: None,
            confidence: 0.5,
            reason: "默认分配到通用工具箱".to_string(),
        }
    }
}

/// 工具箱摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolboxSummary {
    /// 工具箱 ID
    pub toolbox_id: String,
    /// 工具箱名称
    pub name: String,
    /// 工具箱描述
    pub description: String,
    /// 使用场景
    pub use_cases: Vec<String>,
    /// 关键词（用于搜索）
    pub keywords: Vec<String>,
}

// ============================================================================
// 摘要缓存
// ============================================================================

/// 摘要缓存
#[derive(Debug, Clone, Default)]
pub struct SummaryCache {
    cache: HashMap<String, ToolboxSummary>,
}

impl SummaryCache {
    /// 创建新的缓存
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取缓存
    pub fn get(&self, toolbox_id: &str) -> Option<&ToolboxSummary> {
        self.cache.get(toolbox_id)
    }

    /// 设置缓存
    pub fn insert(&mut self, toolbox_id: String, summary: ToolboxSummary) {
        self.cache.insert(toolbox_id, summary);
    }

    /// 清除缓存
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

// ============================================================================
// AI 工具箱分类器
// ============================================================================

/// LLM 客户端 trait（简化版，实际应该使用 tokitai 的客户端）
#[async_trait::async_trait]
pub trait LLMClient: Send + Sync {
    /// 聊天对话
    async fn chat(&self, prompt: &str) -> Result<String, String>;
}

// 为 Arc<T> 实现 LLMClient
#[async_trait::async_trait]
impl<T: LLMClient + ?Sized> LLMClient for Arc<T> {
    async fn chat(&self, prompt: &str) -> Result<String, String> {
        self.as_ref().chat(prompt).await
    }
}

/// AI 工具箱分类器
pub struct AIToolboxClassifier<T: LLMClient> {
    /// LLM 客户端
    llm_client: Arc<T>,
    /// 工具箱注册表
    toolboxes: Arc<RwLock<HashMap<String, ToolBox>>>,
    /// 摘要缓存
    summary_cache: Arc<RwLock<SummaryCache>>,
}

impl<T: LLMClient> AIToolboxClassifier<T> {
    /// 创建新的分类器
    pub fn new(
        llm_client: Arc<T>,
        toolboxes: Arc<RwLock<HashMap<String, ToolBox>>>,
    ) -> Self {
        Self {
            llm_client,
            toolboxes,
            summary_cache: Arc::new(RwLock::new(SummaryCache::new())),
        }
    }

    /// 为新工具选择或创建工具箱
    pub async fn classify_tool(&self, tool: &ToolDefinition) -> Result<ToolboxAssignment, String> {
        // 获取现有工具箱摘要（parking_lot RwLock 是同步的）
        let toolboxes = self.toolboxes.read();
        let toolbox_summaries = self.get_toolbox_summaries(&toolboxes).await?;
        drop(toolboxes);  // 释放锁

        // 构建 AI 提示词
        let prompt = self.build_classification_prompt(tool, &toolbox_summaries);

        // 调用 AI
        let response = self.llm_client.chat(&prompt).await?;

        // 解析 AI 响应
        let assignment: ToolboxAssignment = serde_json::from_str(&response)
            .map_err(|e| format!("解析 AI 响应失败：{}", e))?;

        // 如果 AI 建议创建新工具箱，自动执行
        if matches!(assignment.action, ToolboxAction::CreateNew) {
            if let Some(new_tb) = &assignment.new_toolbox {
                self.create_new_toolbox(new_tb).await?;
            }
        }

        info!(
            "工具分类完成：{} -> {:?} (置信度：{:.2})",
            tool.name,
            assignment.action,
            assignment.confidence
        );

        Ok(assignment)
    }

    /// 构建分类提示词
    fn build_classification_prompt(
        &self,
        tool: &ToolDefinition,
        toolbox_summaries: &[ToolboxSummary],
    ) -> String {
        format!(
            r#"你是一个工具分类专家。请为新工具选择最合适的工具箱。

## 新工具
- **名称**: {}
- **描述**: {}
- **类别**: {:?}
- **标签**: {:?}

## 现有工具箱
{}

## 任务
1. 判断新工具应该放入哪个工具箱
2. 如果现有工具箱都不合适，建议创建新工具箱
3. 给出理由

## 输出格式（JSON）
{{
    "action": "add_to_existing" | "create_new",
    "toolbox_id": "现有工具箱 ID（如果放入现有）",
    "new_toolbox": {{
        "name": "新工具箱名称",
        "description": "新工具箱简介",
        "use_cases": ["使用场景 1", "使用场景 2"]
    }}（如果创建新的）,
    "confidence": 0.0-1.0,
    "reason": "分类理由"
}}"#,
            tool.name,
            tool.description,
            tool.metadata.category,
            tool.tags,
            toolbox_summaries
                .iter()
                .map(|tb| format!("- **{}**: {}", tb.name, tb.description))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// 获取工具箱摘要列表
    async fn get_toolbox_summaries(
        &self,
        toolboxes: &HashMap<String, ToolBox>,
    ) -> Result<Vec<ToolboxSummary>, String> {
        let mut summaries = Vec::new();

        for toolbox in toolboxes.values() {
            let summary = self.get_or_generate_toolbox_summary(toolbox).await?;
            summaries.push(summary);
        }

        Ok(summaries)
    }

    /// 获取或生成工具箱摘要
    async fn get_or_generate_toolbox_summary(
        &self,
        toolbox: &ToolBox,
    ) -> Result<ToolboxSummary, String> {
        // 1. 尝试从缓存读取（parking_lot RwLock 是同步的）
        {
            let cache = self.summary_cache.read();
            if let Some(cached) = cache.get(&toolbox.id) {
                debug!("工具箱摘要缓存命中：{}", toolbox.id);
                return Ok(cached.clone());
            }
        }

        // 2. AI 生成摘要
        let tools = toolbox.get_all_tools().into_iter().cloned().collect::<Vec<_>>();
        let summary = self.generate_toolbox_summary(&toolbox.id, &toolbox.name, &tools).await?;

        // 3. 写入缓存（parking_lot RwLock 是同步的）
        {
            let mut cache = self.summary_cache.write();
            cache.insert(toolbox.id.clone(), summary.clone());
        }

        info!("工具箱摘要生成并缓存：{}", toolbox.id);

        Ok(summary)
    }

    /// AI 生成工具箱摘要
    async fn generate_toolbox_summary(
        &self,
        toolbox_id: &str,
        toolbox_name: &str,
        tools: &[ToolDefinition],
    ) -> Result<ToolboxSummary, String> {
        let prompt = format!(
            r#"你是一个工具分类专家。请为以下工具箱生成简介。

## 工具箱名称
{}

## 包含工具（{} 个）
{}

## 任务
1. 生成工具箱简介（50 字以内）
2. 列出典型使用场景（3-5 个）
3. 提取关键词（5-10 个，用于搜索）

## 输出格式（JSON）
{{
    "toolbox_id": "{}",
    "name": "{}",
    "description": "工具箱简介",
    "use_cases": ["场景 1", "场景 2", "场景 3"],
    "keywords": ["关键词 1", "关键词 2"]
}}"#,
            toolbox_name,
            tools.len(),
            tools
                .iter()
                .map(|t| format!("- **{}**: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n"),
            toolbox_id,
            toolbox_name
        );

        let response = self.llm_client.chat(&prompt).await?;
        let mut summary: ToolboxSummary = serde_json::from_str(&response)
            .map_err(|e| format!("解析工具箱摘要失败：{}", e))?;

        // 确保 toolbox_id 和 name 正确
        summary.toolbox_id = toolbox_id.to_string();
        summary.name = toolbox_name.to_string();

        Ok(summary)
    }

    /// 创建新工具箱
    async fn create_new_toolbox(&self, new_tb: &NewToolbox) -> Result<(), String> {
        let toolbox_id = new_tb.name.to_lowercase().replace(' ', "_");

        // parking_lot RwLock 是同步的
        let mut toolboxes = self.toolboxes.write();

        // 检查是否已存在
        if toolboxes.contains_key(&toolbox_id) {
            warn!("工具箱已存在：{}", toolbox_id);
            return Ok(());
        }

        // 创建新工具箱
        let mut toolbox = ToolBox::new(
            &toolbox_id,
            &new_tb.name,
            &new_tb.description,
        );
        toolbox.tags = vec!["ai_created".to_string()];

        toolboxes.insert(toolbox_id.clone(), toolbox);

        info!("创建新工具箱：{} ({})", toolbox_id, new_tb.name);

        Ok(())
    }

    /// 清除摘要缓存
    pub async fn clear_cache(&self) {
        let mut cache = self.summary_cache.write();
        cache.clear();
        info!("工具箱摘要缓存已清除");
    }
}

// ============================================================================
// 默认 LLM 客户端实现（用于测试）
// ============================================================================

/// 默认 LLM 客户端（生产用）
pub struct DefaultLLMClient {
    api_url: String,
    api_key: String,
    model: String,
    client: reqwest::Client,
}

impl DefaultLLMClient {
    /// 创建新的客户端
    pub fn new(api_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            api_key: api_key.into(),
            model: "gpt-3.5-turbo".to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// 创建带自定义模型的客户端
    pub fn with_model(api_url: impl Into<String>, api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl LLMClient for DefaultLLMClient {
    async fn chat(&self, prompt: &str) -> Result<String, String> {
        // 构建请求体
        let request_body = serde_json::json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.1,
            "max_tokens": 1024
        });

        // 发送请求
        let mut request = self.client
            .post(&self.api_url)
            .json(&request_body)
            .header("Content-Type", "application/json");

        // 添加 API Key（如果提供）
        if !self.api_key.is_empty() {
            request = request.header("Authorization", format!("Bearer {}", self.api_key));
        }

        let response = request
            .send()
            .await
            .map_err(|e| format!("发送请求失败：{}", e))?;

        // 检查响应状态
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("API 返回错误 ({}): {}", status, error_text));
        }

        // 解析响应
        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("解析响应失败：{}", e))?;

        // 提取 AI 响应内容
        let content = response_json
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("message"))
            .and_then(|msg| msg.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| "AI 响应格式异常：无法提取内容".to_string())?;

        Ok(content.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_matrix::matrix::ServiceCategory;

    struct MockLLMClient;

    #[async_trait::async_trait]
    impl LLMClient for MockLLMClient {
        async fn chat(&self, _prompt: &str) -> Result<String, String> {
            Ok(r#"{
                "action": "add_to_existing",
                "toolbox_id": "file_ops",
                "confidence": 0.9,
                "reason": "这是一个文件操作工具"
            }"#.to_string())
        }
    }

    #[tokio::test]
    async fn test_toolbox_classifier() {
        let llm_client = Arc::new(MockLLMClient);
        let toolboxes = Arc::new(RwLock::new(HashMap::new()));

        let classifier = AIToolboxClassifier::new(llm_client, toolboxes);

        let tool = ToolDefinition::new("read_file", "Read file content", r#"{}"#)
            .with_category(ServiceCategory::File);

        let assignment = classifier.classify_tool(&tool).await.unwrap();

        assert_eq!(assignment.confidence, 0.9);
        assert_eq!(assignment.toolbox_id, Some("file_ops".to_string()));
    }
}
