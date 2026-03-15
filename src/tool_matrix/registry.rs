//! 工具注册表
//!
//! 实现工具的注册、发现和运行时添加功能
//! 支持与 tokitai::tool 宏生成的工具集成
//!
//! ## AI 原生特性
//! - 支持 AI 自主分类工具到工具箱
//! - 支持 AI 动态创建新工具箱
//! - 后台异步索引重建
//! - 运行时日志学习依赖关系

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::tool_matrix::matrix::{ToolDefinition, ToolBox, ToolUsageStats, ServiceMetadata};
use crate::tool_matrix::ai_classifier::{AIToolboxClassifier, DefaultLLMClient, ToolboxAction};
use crate::tool_matrix::dependency_analyzer::{AIDependencyAnalyzer, ToolCallSequence};
use tracing::{info, warn, debug};

/// 工具来源
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolSource {
    /// 内置工具（编译时注册）
    Builtin,
    /// 动态注册工具（运行时添加）
    Dynamic,
    /// 从文件加载的工具箱
    FileLoaded,
}

/// 注册的工具信息
#[derive(Debug, Clone)]
pub struct RegisteredTool {
    /// 工具定义
    pub definition: ToolDefinition,
    /// 工具来源
    pub source: ToolSource,
    /// 所属工具箱 ID
    pub toolbox_id: Option<String>,
}

/// 工具注册表
#[derive(Clone)]
pub struct ToolRegistry {
    /// 所有注册的工具
    tools: Arc<RwLock<HashMap<String, RegisteredTool>>>,
    /// 工具箱集合
    toolboxes: Arc<RwLock<HashMap<String, ToolBox>>>,
    /// 工具使用统计
    usage_stats: Arc<RwLock<HashMap<String, ToolUsageStats>>>,
    /// AI 工具箱分类器（可选，用于自主分类）
    ai_classifier: Option<Arc<AIToolboxClassifier<DefaultLLMClient>>>,
    /// AI 依赖关系分析器（可选，用于自主分析依赖）
    ai_dependency_analyzer: Option<Arc<AIDependencyAnalyzer<DefaultLLMClient>>>,
    /// 运行时工具调用序列（用于依赖学习）
    runtime_call_sequences: Arc<RwLock<Vec<ToolCallSequence>>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// 创建新的工具注册表（不带 AI）
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            toolboxes: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
            ai_classifier: None,
            ai_dependency_analyzer: None,
            runtime_call_sequences: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建带 AI 分类器的工具注册表
    pub fn with_ai_classifier(
        llm_client: Arc<DefaultLLMClient>,
    ) -> Self {
        let registry = Self::new();
        let classifier = Arc::new(AIToolboxClassifier::new(
            llm_client,
            registry.toolboxes.clone(),
        ));
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            toolboxes: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
            ai_classifier: Some(classifier),
            ai_dependency_analyzer: None,
            runtime_call_sequences: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建带 AI 依赖分析器的工具注册表
    pub fn with_ai_dependency_analyzer(
        llm_client: Arc<DefaultLLMClient>,
    ) -> Self {
        let registry = Self::new();
        let analyzer = Arc::new(AIDependencyAnalyzer::new(llm_client));
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            toolboxes: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
            ai_classifier: None,
            ai_dependency_analyzer: Some(analyzer),
            runtime_call_sequences: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 创建带完整 AI 功能的工具注册表
    pub fn with_full_ai(
        classifier_llm: Arc<DefaultLLMClient>,
        analyzer_llm: Arc<DefaultLLMClient>,
    ) -> Self {
        let registry = Self::new();
        let classifier = Arc::new(AIToolboxClassifier::new(
            classifier_llm,
            registry.toolboxes.clone(),
        ));
        let analyzer = Arc::new(AIDependencyAnalyzer::new(analyzer_llm));
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            toolboxes: Arc::new(RwLock::new(HashMap::new())),
            usage_stats: Arc::new(RwLock::new(HashMap::new())),
            ai_classifier: Some(classifier),
            ai_dependency_analyzer: Some(analyzer),
            runtime_call_sequences: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// 注册单个工具（AI 自主分类）
    pub async fn register_tool(&self, tool: ToolDefinition, source: ToolSource) -> Result<()> {
        let tool_name = tool.name.clone();

        // 检查是否已存在
        if self.tools.read().contains_key(&tool_name) {
            anyhow::bail!("工具 {} 已存在", tool_name);
        }

        // AI 自主分类（如果启用了分类器）
        let toolbox_assignment = if let Some(classifier) = &self.ai_classifier {
            match classifier.classify_tool(&tool).await {
                Ok(assignment) => {
                    info!("AI 分类工具 {}: {:?}", tool_name, assignment.action);
                    Some(assignment)
                }
                Err(e) => {
                    warn!("AI 分类失败，使用默认分类：{}", e);
                    None
                }
            }
        } else {
            None
        };

        // 确定工具箱 ID
        let toolbox_id = if let Some(assignment) = &toolbox_assignment {
            match &assignment.action {
                ToolboxAction::AddToExisting => assignment.toolbox_id.clone(),
                ToolboxAction::CreateNew => {
                    // AI 建议创建新工具箱，已经在 classifier 中处理
                    assignment.new_toolbox.as_ref().map(|tb| {
                        tb.name.to_lowercase().replace(' ', "_")
                    })
                }
            }
        } else {
            None
        };

        let registered_tool = RegisteredTool {
            definition: tool.clone(),
            source,
            toolbox_id: toolbox_id.clone(),
        };

        self.tools.write().insert(tool_name.clone(), registered_tool);

        // 添加到工具箱（如果 AI 指定了）
        if let Some(tb_id) = &toolbox_id {
            if let Some(box_ref) = self.toolboxes.write().get_mut(tb_id) {
                box_ref.add_tool(tool.clone());
                debug!("工具 {} 已添加到工具箱 {}", tool_name, tb_id);
            }
        }

        // 初始化使用统计
        self.usage_stats
            .write()
            .insert(tool_name.clone(), ToolUsageStats::new(tool_name));

        // AI 依赖分析（如果启用了分析器）
        if let Some(analyzer) = &self.ai_dependency_analyzer {
            let all_tools = self.get_all_tools();
            match analyzer.analyze_dependencies(&tool, &all_tools).await {
                Ok(analysis) => {
                    info!("AI 依赖分析完成：{}，发现 {} 个前置依赖", 
                        tool.name, analysis.prerequisites.len());
                }
                Err(e) => {
                    warn!("AI 依赖分析失败：{}", e);
                }
            }
        }

        Ok(())
    }

    /// 注册工具到指定工具箱（同步版本，用于初始化）
    pub fn register_tool_to_box_sync(
        &self,
        tool: ToolDefinition,
        toolbox_id: &str,
        source: ToolSource,
    ) -> Result<()> {
        let tool_name = tool.name.clone();

        // 检查工具箱是否存在
        if !self.toolboxes.read().contains_key(toolbox_id) {
            anyhow::bail!("工具箱 {} 不存在", toolbox_id);
        }

        // 注册工具（同步版本，不触发 AI 分类）
        let registered_tool = RegisteredTool {
            definition: tool.clone(),
            source,
            toolbox_id: Some(toolbox_id.to_string()),
        };

        self.tools.write().insert(tool_name.clone(), registered_tool);

        // 添加到工具箱
        let mut toolboxes = self.toolboxes.write();
        if let Some(box_ref) = toolboxes.get_mut(toolbox_id) {
            box_ref.add_tool(tool);
        }

        // 初始化使用统计
        self.usage_stats
            .write()
            .insert(tool_name.clone(), ToolUsageStats::new(tool_name));

        Ok(())
    }

    /// 从 tokitai ToolProvider 批量注册工具（同步版本，用于初始化）
    pub fn register_from_provider_sync<T: tokitai::ToolProvider>(
        &self,
        toolbox_id: Option<&str>,
        source: ToolSource,
    ) -> Result<Vec<String>> {
        let definitions = T::tool_definitions();
        let mut registered = Vec::new();

        for def in definitions {
            let tool_def = ToolDefinition {
                name: def.name.clone(),
                description: def.description.clone(),
                input_schema: def.input_schema.clone(),
                tags: Vec::new(),
                risk_level: "safe".to_string(),
                source: match source {
                    ToolSource::Builtin => "builtin".to_string(),
                    ToolSource::Dynamic => "dynamic".to_string(),
                    ToolSource::FileLoaded => "file_loaded".to_string(),
                },
                metadata: ServiceMetadata::default(),
            };

            let tool_name = tool_def.name.clone();

            // 注册工具（同步版本）
            let registered_tool = RegisteredTool {
                definition: tool_def.clone(),
                source: source.clone(),
                toolbox_id: toolbox_id.map(|s| s.to_string()),
            };

            self.tools.write().insert(tool_name.clone(), registered_tool);

            // 如果指定了工具箱，添加到工具箱
            if let Some(box_id) = toolbox_id {
                if let Some(box_ref) = self.toolboxes.write().get_mut(box_id) {
                    box_ref.add_tool(tool_def);
                }
            }

            // 初始化使用统计
            self.usage_stats
                .write()
                .insert(tool_name.clone(), ToolUsageStats::new(tool_name.clone()));

            registered.push(tool_name);
        }

        Ok(registered)
    }

    /// 注册工具到指定工具箱（异步，AI 自主分类）
    pub async fn register_tool_to_box(
        &self,
        tool: ToolDefinition,
        toolbox_id: &str,
        source: ToolSource,
    ) -> Result<()> {
        let tool_name = tool.name.clone();

        // 检查工具箱是否存在
        if !self.toolboxes.read().contains_key(toolbox_id) {
            anyhow::bail!("工具箱 {} 不存在", toolbox_id);
        }

        // 注册工具（AI 自主分类）
        self.register_tool(tool.clone(), source).await?;

        // 添加到工具箱
        let mut toolboxes = self.toolboxes.write();
        if let Some(box_ref) = toolboxes.get_mut(toolbox_id) {
            box_ref.add_tool(tool);
        }

        // 更新注册信息
        if let Some(tool_ref) = self.tools.write().get_mut(&tool_name) {
            tool_ref.toolbox_id = Some(toolbox_id.to_string());
        }

        Ok(())
    }

    /// 从 tokitai ToolProvider 批量注册工具（异步，AI 自主分类）
    pub async fn register_from_provider<T: tokitai::ToolProvider>(
        &self,
        toolbox_id: Option<&str>,
        source: ToolSource,
    ) -> Result<Vec<String>> {
        let definitions = T::tool_definitions();
        let mut registered = Vec::new();

        for def in definitions {
            let tool_def = ToolDefinition {
                name: def.name.clone(),
                description: def.description.clone(),
                input_schema: def.input_schema.clone(),
                tags: Vec::new(),
                risk_level: "safe".to_string(),
                source: match source {
                    ToolSource::Builtin => "builtin".to_string(),
                    ToolSource::Dynamic => "dynamic".to_string(),
                    ToolSource::FileLoaded => "file_loaded".to_string(),
                },
                metadata: ServiceMetadata::default(),
            };

            let tool_name = tool_def.name.clone();

            // 注册工具（AI 自主分类）
            self.register_tool(tool_def.clone(), source.clone()).await?;

            // 如果指定了工具箱，添加到工具箱
            if let Some(box_id) = toolbox_id {
                if let Some(box_ref) = self.toolboxes.write().get_mut(box_id) {
                    box_ref.add_tool(tool_def);
                }
                if let Some(tool_ref) = self.tools.write().get_mut(&tool_name) {
                    tool_ref.toolbox_id = Some(box_id.to_string());
                }
            }

            registered.push(tool_name);
        }

        Ok(registered)
    }

    /// 获取工具定义
    pub fn get_tool(&self, name: &str) -> Option<ToolDefinition> {
        self.tools
            .read()
            .get(name)
            .map(|rt| rt.definition.clone())
    }

    /// 获取所有工具
    pub fn get_all_tools(&self) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .values()
            .map(|rt| rt.definition.clone())
            .collect()
    }

    /// 获取工具箱中的所有工具
    pub fn get_tools_from_box(&self, toolbox_id: &str) -> Vec<ToolDefinition> {
        self.toolboxes
            .read()
            .get(toolbox_id)
            .map(|box_ref| box_ref.get_all_tools().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// 按标签过滤工具
    pub fn filter_by_tag(&self, tag: &str) -> Vec<ToolDefinition> {
        self.tools
            .read()
            .values()
            .filter(|rt| rt.definition.tags.contains(&tag.to_string()))
            .map(|rt| rt.definition.clone())
            .collect()
    }

    /// 按风险等级过滤工具
    pub fn filter_by_risk(&self, max_risk: &str) -> Vec<ToolDefinition> {
        let risk_order = ["safe", "medium", "dangerous"];
        let max_idx = risk_order.iter().position(|&r| r == max_risk).unwrap_or(2);

        self.tools
            .read()
            .values()
            .filter(|rt| {
                let idx = risk_order
                    .iter()
                    .position(|&r| r == rt.definition.risk_level.as_str())
                    .unwrap_or(0);
                idx <= max_idx
            })
            .map(|rt| rt.definition.clone())
            .collect()
    }

    /// 创建工具箱
    pub fn create_toolbox(&self, toolbox: ToolBox) -> Result<()> {
        let box_id = toolbox.id.clone();

        if self.toolboxes.read().contains_key(&box_id) {
            anyhow::bail!("工具箱 {} 已存在", box_id);
        }

        self.toolboxes.write().insert(box_id, toolbox);
        Ok(())
    }

    /// 获取工具箱
    pub fn get_toolbox(&self, id: &str) -> Option<ToolBox> {
        self.toolboxes.read().get(id).cloned()
    }

    /// 获取所有工具箱
    pub fn get_all_toolboxes(&self) -> Vec<ToolBox> {
        self.toolboxes.read().values().cloned().collect()
    }

    /// 删除工具箱
    pub fn remove_toolbox(&self, id: &str) -> Result<()> {
        if !self.toolboxes.read().contains_key(id) {
            anyhow::bail!("工具箱 {} 不存在", id);
        }

        // 移除工具箱中的工具引用
        if let Some(box_ref) = self.toolboxes.write().remove(id) {
            for tool_name in box_ref.tools.keys() {
                if let Some(tool_ref) = self.tools.write().get_mut(tool_name) {
                    tool_ref.toolbox_id = None;
                }
            }
        }

        Ok(())
    }

    /// 记录工具使用
    pub fn record_usage(&self, tool_name: &str, success: bool, execution_time_ms: u64) {
        if let Some(stats) = self.usage_stats.write().get_mut(tool_name) {
            stats.record_usage(success, execution_time_ms);
        }
    }

    /// 获取工具使用统计
    pub fn get_usage_stats(&self, tool_name: &str) -> Option<ToolUsageStats> {
        self.usage_stats.read().get(tool_name).cloned()
    }

    /// 获取按使用次数排序的工具
    pub fn get_popular_tools(&self, limit: usize) -> Vec<ToolDefinition> {
        let stats: Vec<ToolUsageStats> = {
            let binding = self.usage_stats.read();
            binding.values().cloned().collect()
        };
        let mut stats = stats;
        stats.sort_by(|a, b| b.usage_count.cmp(&a.usage_count));

        stats
            .into_iter()
            .take(limit)
            .filter_map(|s| self.get_tool(&s.tool_name))
            .collect()
    }

    /// 检查工具是否存在
    pub fn tool_exists(&self, name: &str) -> bool {
        self.tools.read().contains_key(name)
    }

    /// 获取工具数量
    pub fn tool_count(&self) -> usize {
        self.tools.read().len()
    }

    /// 获取工具箱数量
    pub fn toolbox_count(&self) -> usize {
        self.toolboxes.read().len()
    }

    /// 清空所有工具
    pub fn clear(&self) {
        self.tools.write().clear();
        self.toolboxes.write().clear();
        self.usage_stats.write().clear();
        self.runtime_call_sequences.write().clear();
    }

    /// 记录工具调用序列（用于依赖学习）
    pub fn record_call_sequence(&self, sequence: ToolCallSequence) {
        let mut sequences = self.runtime_call_sequences.write();
        sequences.push(sequence);
        
        // 保持最近 1000 条记录
        if sequences.len() > 1000 {
            sequences.remove(0);
        }
    }

    /// 从运行时日志学习依赖关系
    pub async fn learn_from_runtime_logs(&self) -> Result<usize> {
        if let Some(analyzer) = &self.ai_dependency_analyzer {
            let sequences = self.runtime_call_sequences.read().clone();
            if sequences.is_empty() {
                debug!("没有运行时日志可供学习");
                return Ok(0);
            }

            // 使用 analyzer 学习
            let _ = analyzer.learn_from_runtime_logs(&sequences);

            let learned_count = sequences.len();
            info!("从 {} 条运行时日志中学习依赖关系", learned_count);
            
            Ok(learned_count)
        } else {
            warn!("未启用 AI 依赖分析器，无法学习");
            Ok(0)
        }
    }

    /// 获取运行时调用序列
    pub fn get_runtime_sequences(&self) -> Vec<ToolCallSequence> {
        self.runtime_call_sequences.read().clone()
    }

    /// 清除运行时调用序列
    pub fn clear_runtime_sequences(&self) {
        self.runtime_call_sequences.write().clear();
    }
}

/// 便捷宏：从 ToolProvider 注册工具到注册表
#[macro_export]
macro_rules! register_tools {
    ($registry:expr, $($provider:ty),+ $(,)?) => {{
        let mut registered = Vec::new();
        $(
            match $registry.register_from_provider::<$provider>(None, $crate::tool_matrix::ToolSource::Builtin) {
                Ok(names) => registered.extend(names),
                Err(e) => tracing::warn!("注册工具 {:?} 失败：{}", stringify!($provider), e),
            }
        )*
        registered
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_tool() {
        let registry = ToolRegistry::new();
        let tool = ToolDefinition::new("test_tool", "A test tool", r#"{}"#);

        assert!(registry.register_tool(tool.clone(), ToolSource::Builtin).await.is_ok());
        assert!(registry.tool_exists("test_tool"));
        assert_eq!(registry.tool_count(), 1);
    }

    #[test]
    fn test_create_toolbox() {
        let registry = ToolRegistry::new();
        let toolbox = ToolBox::new("test_box", "Test Box", "A test toolbox");

        assert!(registry.create_toolbox(toolbox).is_ok());
        assert!(registry.get_toolbox("test_box").is_some());
        assert_eq!(registry.toolbox_count(), 1);
    }

    #[tokio::test]
    async fn test_register_tool_to_box() {
        let registry = ToolRegistry::new();

        // 创建工具箱
        let toolbox = ToolBox::new("test_box", "Test Box", "A test toolbox");
        registry.create_toolbox(toolbox).unwrap();

        // 注册工具到工具箱（使用同步版本）
        let tool = ToolDefinition::new("test_tool", "A test tool", r#"{}"#);
        registry
            .register_tool_to_box_sync(tool, "test_box", ToolSource::Builtin)
            .unwrap();

        // 验证
        assert!(registry.tool_exists("test_tool"));
        let tools = registry.get_tools_from_box("test_box");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_filter_by_tag() {
        let registry = ToolRegistry::new();

        let tool1 = ToolDefinition::new("tool1", "Tool 1", r#"{}"#).with_tag("utility");
        let tool2 = ToolDefinition::new("tool2", "Tool 2", r#"{}"#).with_tag("network");

        registry.register_tool(tool1, ToolSource::Builtin).await.unwrap();
        registry.register_tool(tool2, ToolSource::Builtin).await.unwrap();

        let utility_tools = registry.filter_by_tag("utility");
        assert_eq!(utility_tools.len(), 1);
        assert_eq!(utility_tools[0].name, "tool1");
    }
}
