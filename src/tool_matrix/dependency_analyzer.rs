//! AI 依赖关系分析器
//!
//! AI 自主维护工具依赖关系：
//! - 分析新工具的依赖关系（前置依赖、后置依赖、工具组合）
//! - 从运行时日志学习（补充 AI 分析）
//! - 构建工具依赖图
//!
//! # 设计原则
//! - 依赖关系不是手动声明的，而是 AI 分析工具语义自动推断的
//! - 结合静态分析（AI 语义理解）和动态学习（运行时日志）

#![allow(dead_code)]

use crate::tool_matrix::matrix::{ServiceCategory, ToolDefinition};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

// ============================================================================
// 依赖关系数据结构
// ============================================================================

/// 依赖分析结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyAnalysis {
    /// 前置依赖（执行这个工具前需要先调用的工具）
    pub prerequisites: Vec<DependencyRelation>,
    /// 后置依赖（可能依赖这个工具输出的工具）
    pub dependents: Vec<DependencyRelation>,
    /// 工具组合（经常一起使用的工具）
    pub combinations: Vec<ToolCombination>,
}

/// 依赖关系
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRelation {
    /// 工具名称
    pub tool_name: String,
    /// 依赖理由
    pub reason: String,
    /// 置信度（0-1）
    pub confidence: f32,
}

/// 工具组合
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCombination {
    /// 工具名称
    pub tool_name: String,
    /// 使用场景
    pub scenario: String,
}

/// 工具依赖图
#[derive(Debug, Clone, Default)]
pub struct ToolDependencyGraph {
    /// 前置依赖：tool -> [prerequisites]
    prerequisites: HashMap<String, Vec<WeightedDependency>>,
    /// 后置依赖：tool -> [dependents]
    dependents: HashMap<String, Vec<WeightedDependency>>,
    /// 工具共现：(tool1, tool2) -> weight
    co_occurrences: HashMap<(String, String), f32>,
}

impl ToolDependencyGraph {
    /// 创建新的依赖图
    pub fn new() -> Self {
        Self::default()
    }

    /// 添加依赖关系
    pub fn add_dependency(&mut self, from: String, to: String, confidence: f32) {
        // 添加到 from 的后置依赖
        self.dependents
            .entry(from.clone())
            .or_default()
            .push(WeightedDependency {
                tool_name: to.clone(),
                confidence,
            });

        // 添加到 to 的前置依赖
        self.prerequisites
            .entry(to)
            .or_default()
            .push(WeightedDependency {
                tool_name: from,
                confidence,
            });
    }

    /// 添加工具共现关系
    pub fn add_co_occurrence(&mut self, tool1: String, tool2: String, weight: f32) {
        // 确保键的顺序一致
        let key = if tool1 < tool2 {
            (tool1, tool2)
        } else {
            (tool2, tool1)
        };

        // 累加权重
        let entry = self.co_occurrences.entry(key).or_insert(0.0);
        *entry = (*entry + weight).min(1.0);
    }

    /// 获取工具的前置依赖
    pub fn get_prerequisites(&self, tool: &str) -> Vec<&WeightedDependency> {
        self.prerequisites
            .get(tool)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// 获取工具的后置依赖
    pub fn get_dependents(&self, tool: &str) -> Vec<&WeightedDependency> {
        self.dependents
            .get(tool)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// 推荐后续工具（基于依赖图和共现关系）
    pub fn recommend_next_tools(
        &self,
        current_tools: &[String],
        max_recommendations: usize,
    ) -> Vec<String> {
        let mut scores: HashMap<String, f32> = HashMap::new();

        for tool in current_tools {
            // 基于后置依赖推荐
            for dep in self.get_dependents(tool) {
                let entry = scores.entry(dep.tool_name.clone()).or_insert(0.0);
                *entry += dep.confidence;
            }

            // 基于共现关系推荐
            for ((t1, t2), &weight) in &self.co_occurrences {
                if t1 == tool {
                    let entry = scores.entry(t2.clone()).or_insert(0.0);
                    *entry += weight;
                } else if t2 == tool {
                    let entry = scores.entry(t1.clone()).or_insert(0.0);
                    *entry += weight;
                }
            }
        }

        // 排序并返回 Top-N
        let mut recommendations: Vec<_> = scores.into_iter().collect();
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        recommendations
            .into_iter()
            .take(max_recommendations)
            .map(|(name, _)| name)
            .collect()
    }

    /// 获取所有工具
    pub fn get_all_tools(&self) -> HashSet<String> {
        let mut tools = HashSet::new();

        for from in self.prerequisites.keys() {
            tools.insert(from.clone());
        }
        for to in self.dependents.keys() {
            tools.insert(to.clone());
        }
        for (t1, t2) in self.co_occurrences.keys() {
            tools.insert(t1.clone());
            tools.insert(t2.clone());
        }

        tools
    }
}

/// 带权重的依赖
#[derive(Debug, Clone)]
pub struct WeightedDependency {
    pub tool_name: String,
    pub confidence: f32,
}

/// 工具调用序列（用于运行时学习）
#[derive(Debug, Clone)]
pub struct ToolCallSequence {
    /// 工具调用序列
    pub tools: Vec<String>,
    /// 调用时间戳（毫秒）
    pub timestamps: Vec<u64>,
}

// ============================================================================
// AI 依赖关系分析器
// ============================================================================

/// LLM 客户端 trait（与 ai_classifier 共享）
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

/// AI 依赖关系分析器
pub struct AIDependencyAnalyzer<T: LLMClient + ?Sized> {
    /// LLM 客户端
    llm_client: Arc<T>,
    /// 依赖图
    dependency_graph: Arc<RwLock<ToolDependencyGraph>>,
}

impl<T: LLMClient + ?Sized> AIDependencyAnalyzer<T> {
    /// 创建新的分析器
    pub fn new(llm_client: Arc<T>) -> Self {
        Self {
            llm_client,
            dependency_graph: Arc::new(RwLock::new(ToolDependencyGraph::new())),
        }
    }

    /// 分析新工具的依赖关系
    pub async fn analyze_dependencies(
        &self,
        tool: &ToolDefinition,
        all_tools: &[ToolDefinition],
    ) -> Result<DependencyAnalysis, String> {
        let prompt = self.build_dependency_prompt(tool, all_tools);

        let response = self.llm_client.chat(&prompt).await?;

        let analysis: DependencyAnalysis =
            serde_json::from_str(&response).map_err(|e| format!("解析依赖分析失败：{}", e))?;

        // 更新依赖图
        self.update_dependency_graph(tool, &analysis).await?;

        info!(
            "工具依赖分析完成：{} (前置：{}, 后置：{}, 组合：{})",
            tool.name,
            analysis.prerequisites.len(),
            analysis.dependents.len(),
            analysis.combinations.len()
        );

        Ok(analysis)
    }

    /// 构建依赖分析提示词
    fn build_dependency_prompt(
        &self,
        tool: &ToolDefinition,
        all_tools: &[ToolDefinition],
    ) -> String {
        format!(
            r#"你是一个工具依赖分析专家。请分析以下工具的依赖关系。

## 新工具
- **名称**: {}
- **描述**: {}
- **输入类型**: {}
- **输出类型**: {}
- **风险等级**: {}

## 现有工具列表
{}

## 任务
1. **前置依赖**: 执行这个工具前，通常需要先调用哪些工具？
   （例如：处理文件前需要先读取文件）

2. **后置依赖**: 哪些工具可能会依赖这个工具的输出？
   （例如：写入文件后可能需要验证文件内容）

3. **工具组合**: 这个工具经常和哪些工具一起使用？

## 输出格式（JSON）
{{
    "prerequisites": [
        {{"tool_name": "工具名", "reason": "依赖理由", "confidence": 0.0-1.0}}
    ],
    "dependents": [
        {{"tool_name": "工具名", "reason": "依赖理由", "confidence": 0.0-1.0}}
    ],
    "combinations": [
        {{"tool_name": "工具名", "scenario": "使用场景"}}
    ]
}}"#,
            tool.name,
            tool.description,
            Self::extract_input_types(tool),
            Self::extract_output_type(tool),
            tool.risk_level,
            all_tools
                .iter()
                .map(|t| format!("- **{}**: {}", t.name, t.description))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// 提取输入类型（从 input_schema 解析）
    fn extract_input_types(tool: &ToolDefinition) -> String {
        // 尝试从 JSON Schema 中解析属性类型
        if let Ok(schema) = serde_json::from_str::<Value>(&tool.input_schema) {
            if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
                let mut types = Vec::new();
                for (name, prop) in properties {
                    let prop_type = prop.get("type").and_then(|v| v.as_str()).unwrap_or("any");
                    types.push(format!("{}: {}", name, prop_type));
                }
                if !types.is_empty() {
                    return format!("{{{}}}", types.join(", "));
                }
            }
        }
        // 回退到分类名称
        format!("{:?}", tool.metadata.category)
    }

    /// 提取输出类型（基于工具分类推断）
    fn extract_output_type(tool: &ToolDefinition) -> String {
        // 根据工具分类推断典型输出类型
        match tool.metadata.category {
            ServiceCategory::File => "FileContent | FilePath | List<FilePath>".to_string(),
            ServiceCategory::Network => "HttpResponse | String | DownloadedFile".to_string(),
            ServiceCategory::Data => "Json | Xml | Csv".to_string(),
            ServiceCategory::Development => "CodeAnalysis | String".to_string(),
            ServiceCategory::Vcs | ServiceCategory::VersionControl => {
                "GitStatus | GitLog | String".to_string()
            }
            ServiceCategory::System => "ProcessOutput | SystemInfo | String".to_string(),
            ServiceCategory::Ai => "LLMResponse | Embedding | String".to_string(),
            ServiceCategory::Dialogue => "DialogueState | String".to_string(),
            ServiceCategory::Utility | ServiceCategory::Default => "String".to_string(),
        }
    }

    /// 更新依赖图
    async fn update_dependency_graph(
        &self,
        tool: &ToolDefinition,
        analysis: &DependencyAnalysis,
    ) -> Result<(), String> {
        let mut graph = self.dependency_graph.write().await;

        // 添加前置依赖
        for prereq in &analysis.prerequisites {
            graph.add_dependency(
                prereq.tool_name.clone(),
                tool.name.clone(),
                prereq.confidence,
            );
        }

        // 添加后置依赖
        for dependent in &analysis.dependents {
            graph.add_dependency(
                tool.name.clone(),
                dependent.tool_name.clone(),
                dependent.confidence,
            );
        }

        // 添加工具组合关系
        for combo in &analysis.combinations {
            graph.add_co_occurrence(
                tool.name.clone(),
                combo.tool_name.clone(),
                0.8, // 组合关系权重
            );
        }

        info!("工具依赖关系已更新：{}", tool.name);

        Ok(())
    }

    /// 从运行时日志学习（补充 AI 分析）
    pub async fn learn_from_runtime_logs(&self, logs: &[ToolCallSequence]) {
        let mut graph = self.dependency_graph.write().await;

        for seq in logs {
            // 分析工具调用序列，发现共现关系
            for i in 0..seq.tools.len() {
                for j in (i + 1)..seq.tools.len() {
                    // 时间窗口内的工具调用视为共现（30 秒内）
                    if seq.timestamps[j] - seq.timestamps[i] < 30000 {
                        graph.add_co_occurrence(
                            seq.tools[i].clone(),
                            seq.tools[j].clone(),
                            0.5, // 运行时学习的权重较低
                        );
                    }
                }
            }
        }

        info!("从运行时日志学习了 {} 个调用序列", logs.len());
    }

    /// 获取依赖图
    pub async fn get_dependency_graph(&self) -> ToolDependencyGraph {
        self.dependency_graph.read().await.clone()
    }

    /// 推荐后续工具
    pub async fn recommend_next_tools(
        &self,
        current_tools: &[String],
        max_recommendations: usize,
    ) -> Vec<String> {
        let graph = self.dependency_graph.read().await;
        graph.recommend_next_tools(current_tools, max_recommendations)
    }
}

// ============================================================================
// 与 ExecutorAgent 集成的辅助函数
// ============================================================================

/// 工具推荐结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRecommendation {
    /// 推荐的工具
    pub tool_name: String,
    /// 推荐理由
    pub reason: String,
    /// 推荐分数
    pub score: f32,
}

/// 智能工具推荐器（用于 ExecutorAgent）
pub struct SmartToolRecommender<T: LLMClient + ?Sized> {
    dependency_analyzer: Arc<AIDependencyAnalyzer<T>>,
}

impl<T: LLMClient + ?Sized> SmartToolRecommender<T> {
    /// 创建新的推荐器
    pub fn new(dependency_analyzer: Arc<AIDependencyAnalyzer<T>>) -> Self {
        Self {
            dependency_analyzer,
        }
    }

    /// 推荐后续工具
    pub async fn recommend_next(
        &self,
        current_tool: &str,
        max_recommendations: usize,
    ) -> Vec<ToolRecommendation> {
        let tools = self
            .dependency_analyzer
            .recommend_next_tools(&[current_tool.to_string()], max_recommendations)
            .await;

        tools
            .into_iter()
            .map(|tool_name| ToolRecommendation {
                tool_name,
                reason: "基于依赖关系推荐".to_string(),
                score: 0.8,
            })
            .collect()
    }
}

// 重新导出 ai_classifier 中的 DefaultLLMClient 以便使用
pub use crate::tool_matrix::ai_classifier::DefaultLLMClient;

// 为 DefaultLLMClient 实现 dependency_analyzer 的 LLMClient trait
#[async_trait::async_trait]
impl LLMClient for DefaultLLMClient {
    async fn chat(&self, prompt: &str) -> Result<String, String> {
        // 委托给 ai_classifier 的实现
        crate::tool_matrix::ai_classifier::LLMClient::chat(self, prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLLMClient;

    #[async_trait::async_trait]
    impl LLMClient for MockLLMClient {
        async fn chat(&self, _prompt: &str) -> Result<String, String> {
            Ok(r#"{
                "prerequisites": [
                    {"tool_name": "read_file", "reason": "需要先读取文件", "confidence": 0.9}
                ],
                "dependents": [
                    {"tool_name": "verify_file", "reason": "可能需要验证文件", "confidence": 0.7}
                ],
                "combinations": [
                    {"tool_name": "write_file", "scenario": "读写文件组合"}
                ]
            }"#
            .to_string())
        }
    }

    #[tokio::test]
    async fn test_dependency_analyzer() {
        let llm_client = Arc::new(MockLLMClient);
        let analyzer = AIDependencyAnalyzer::new(llm_client);

        let tool = ToolDefinition::new("process_file", "Process file content", r#"{}"#);
        let all_tools = vec![
            ToolDefinition::new("read_file", "Read file content", r#"{}"#),
            ToolDefinition::new("write_file", "Write file content", r#"{}"#),
        ];

        let analysis = analyzer
            .analyze_dependencies(&tool, &all_tools)
            .await
            .unwrap();

        assert_eq!(analysis.prerequisites.len(), 1);
        assert_eq!(analysis.dependents.len(), 1);
        assert_eq!(analysis.combinations.len(), 1);
    }

    #[tokio::test]
    async fn test_dependency_graph() {
        let mut graph = ToolDependencyGraph::new();

        graph.add_dependency("read_file".to_string(), "process_file".to_string(), 0.9);
        graph.add_co_occurrence("read_file".to_string(), "write_file".to_string(), 0.8);

        let recommendations = graph.recommend_next_tools(&["read_file".to_string()], 5);

        assert!(recommendations.contains(&"process_file".to_string()));
        assert!(recommendations.contains(&"write_file".to_string()));
    }
}
