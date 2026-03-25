//! 工具矩阵服务
//!
//! 基于动态工具箱和 Skills 文件的工具管理系统
//!
//! ## 设计理念
//! - **动态工具箱**：按领域分类的工具集合（如文件操作箱、网络工具箱）
//! - **Skills 文件**：每个工具箱的"说明书"，告诉 AI 如何正确使用工具
//! - **自我进化**：AI 发现工具不足时 → 快速开发新工具 → 注册到工具箱 → 更新 Skills 文件
//!
//! ## 模块结构
//! - `matrix`: 工具矩阵和工具箱结构定义
//! - `registry`: 工具注册表，支持运行时注册
//! - `skills_manager`: Skills 文件管理器
//! - `selector`: 动态工具选择器
//! - `tool_selector`: 轻量级工具选择器（AI 原生）
//! - `ai_classifier`: AI 工具箱分类器
//! - `dependency_analyzer`: AI 依赖关系分析器
//! - `dispatcher`: 工具调用分发器
//! - `metadata_enhancer`: tokitai 元数据增强器
//! - `rule_classifier`: 规则分类器（分层缓存 L3）
//! - `query_enhancer`: 查询增强器（同义词/意图识别）
//! - `tool_generator`: 工具生成器（模板系统）
//! - `trie_index`: Trie 树索引（IMP-003 搜索优化）
//! - `dynamic_registry`: 动态工具注册表（IMP-004 热加载）
//!
//! ## 使用示例
//! ```rust,ignore
//! use crate::tool_matrix::{ToolRegistry, ToolBox, ToolDefinition, SkillsManager, ToolSelector};
//!
//! // 创建注册表
//! let registry = ToolRegistry::new();
//!
//! // 创建工具箱
//! let mut file_box = ToolBox::new("file_ops", "File Operations", "File tools");
//! file_box.add_tool(ToolDefinition::new("read_file", "Read a file", "{}"));
//! registry.create_toolbox(file_box).unwrap();
//!
//! // 创建 Skills 管理器
//! let skills_mgr = SkillsManager::default();
//!
//! // 创建工具选择器
//! let selector = ToolSelector::new(registry);
//! let result = selector.select_tools_by_query("read file", 5);
//! ```

pub mod matrix;
pub mod registry;
pub mod skills_manager;
pub mod selector;
pub mod tool_selector;
pub mod ai_classifier;
pub mod dependency_analyzer;
pub mod dispatcher;
pub mod metadata_enhancer;
pub mod rule_classifier;
pub mod query_enhancer;
pub mod tool_generator;
pub mod trie_index;
pub mod dynamic_registry;
pub mod tool_definition;
pub mod ai_tool_generator;

// 注意：以下导出保留，供未来功能扩展使用
#[allow(unused_imports)]
pub use tool_selector::{
    LightweightToolSelector,
    ToolIndex,
    ToolSearchResult,
    SearchResultSource,
    SelectorConfig,
};
#[allow(unused_imports)]
pub use ai_classifier::{
    AIToolboxClassifier,
    LLMClient as AILLMClient,
    ToolboxAssignment,
    ToolboxAction,
    NewToolbox,
    ToolboxSummary,
    DefaultLLMClient,
};
#[allow(unused_imports)]
pub use dependency_analyzer::{
    AIDependencyAnalyzer,
    LLMClient as DependencyLLMClient,
    DependencyAnalysis,
    DependencyRelation,
    ToolCombination,
    ToolDependencyGraph,
    ToolCallSequence,
    SmartToolRecommender,
    ToolRecommendation,
};
#[allow(unused_imports)]
pub use dispatcher::{
    ToolDispatcher,
    ToolExecutor,
    DefaultToolExecutor,
};
#[allow(unused_imports)]
pub use metadata_enhancer::MetadataEnhancer;

// 新增模块导出
#[allow(unused_imports)]
pub use rule_classifier::{
    RuleClassifier,
    HierarchicalClassifier,
    RuleMatchResult,
    MatchType,
    ToolboxRulesConfig,
    ToolboxRule,
    CacheStats,
};
#[allow(unused_imports)]
pub use query_enhancer::{
    QueryEnhancer,
    EnhancedQuery,
    IntentRecognition,
    SynonymsConfig,
    IntentPatternsConfig,
    IntentPattern,
};
#[allow(unused_imports)]
pub use tool_generator::{
    ToolGenerator,
    ToolTemplate,
    ToolGenerationRequest,
    ToolGenerationResult,
    TemplateMetadata,
    ParameterDefinition,
    CodeTemplate,
    TestTemplate,
};

// Trie 索引模块导出
#[allow(unused_imports)]
pub use trie_index::{
    TrieIndex,
    TrieIndexStats,
    BKTree,
    BKTreeStats,
    HybridIndex,
    HybridIndexStats,
};

// 动态注册表模块导出
#[allow(unused_imports)]
pub use dynamic_registry::{
    DynamicToolRegistry,
    DynamicToolMetadata,
    DynamicRegistryStats,
    DynamicToolBuilder,
};

// TOML 工具定义模块导出
#[allow(unused_imports)]
pub use tool_definition::{
    TomlToolDefinition,
    TomlToolLoader,
    ToolMetadata,
    ParameterSpec,
    Permissions,
    RateLimit,
    ValidationRule,
};

// AI 工具生成器模块导出
#[allow(unused_imports)]
pub use ai_tool_generator::{
    AIToolGenerator,
    AIToolGenerationRequest,
    AIToolGenerationResult,
    ToolCategory,
};

