//! 自进化闭环系统
//!
//! 整合 ToolGapDetector + ToolOptimizer + SystemReflector + ToolCreator + ExternalToolDiscovery
//! 实现从 0 到 1 的工具发现和创造
//!
//! ## 工作流程
//! 1. **检测** - ToolGapDetector 从失败任务中发现工具缺口
//! 2. **优化** - ToolOptimizer 分析现有工具，提出优化建议
//! 3. **反思** - SystemReflector 生成系统体检报告，发现覆盖不足的领域
//! 4. **创造** - ToolCreator 根据缺口和建议创造新工具（Rust 代码或外部工具封装）
//! 5. **外部工具封装** - ExternalToolDiscovery 发现并封装现有 CLI/HTTP/脚本工具
//!
//! ## 外部工具封装决策树
//! ```text
//! IF gap.requires_high_performance AND gap.is_complex → 创造 Rust 工具
//! IF existing_cli_matches_gap → 封装 CLI 工具
//! IF http_api_available → 封装 HTTP 服务
//! IF rapid_prototyping_needed → 封装脚本文件
//! ELSE → 创造 Rust 工具
//! ```
//!
//! ## 使用示例
//! ```rust,ignore
//! let evolution = SelfImprovementLoop::new(project_root)?;
//!
//! // 记录任务执行
//! evolution.record_task(task_record);
//!
//! // 运行自进化循环
//! let report = evolution.run_evolution_cycle()?;
//!
//! // 查看创建的工具
//! for tool in &report.created_tools {
//!     println!("Created tool: {}", tool.tool_name);
//! }
//! ```

#![allow(dead_code)]

use crate::autonomy::prompt_gap_detector::LLMClient;
use crate::autonomy::{
    gap_detector::{GapType, TaskExecutionRecord, ToolGap},
    hybrid_gap_detector::{HybridConfig, HybridGapDetector, HybridToolGap},
    system_reflector::SystemReflector,
    tool_creator::{ParameterDef, ToolCreationRequest, ToolCreator},
    tool_optimizer::{ToolMetrics, ToolOptimizer},
};
use crate::external_process::{
    metadata::ExternalToolMetadata, ExternalToolDiscovery, ExternalToolRegistry,
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

/// 自进化循环报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionCycleReport {
    /// 循环执行时间戳
    pub timestamp: u64,
    /// 检测到的工具缺口数量
    pub detected_gaps_count: u32,
    /// 优化建议数量
    pub optimization_suggestions_count: u32,
    /// 系统健康度评分
    pub system_health_score: f32,
    /// 创建的工具列表
    pub created_tools: Vec<String>,
    /// 注册的工具列表
    pub registered_tools: Vec<String>,
    /// 循环耗时 (ms)
    pub cycle_duration_ms: u64,
    /// 循环状态
    pub status: CycleStatus,
}

/// 循环状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CycleStatus {
    /// 成功完成
    Success,
    /// 部分成功
    PartialSuccess,
    /// 失败
    Failed(String),
}

/// 自进化闭环系统
pub struct SelfImprovementLoop {
    /// 项目根目录
    project_root: PathBuf,
    /// 数据目录
    data_dir: PathBuf,
    /// 工具缺口检测器（混合版本）
    gap_detector: Arc<RwLock<HybridGapDetector>>,
    /// 工具优化器
    optimizer: Arc<RwLock<ToolOptimizer>>,
    /// 系统反思器
    reflector: Arc<RwLock<SystemReflector>>,
    /// 工具创建器
    creator: Arc<RwLock<ToolCreator>>,
    /// 外部工具发现器
    external_tool_discovery: Arc<RwLock<ExternalToolDiscovery>>,
    /// 外部工具注册表
    external_tool_registry: Arc<RwLock<ExternalToolRegistry>>,
    /// 配置
    config: EvolutionConfig,
    /// 是否启用
    enabled: bool,
    /// LLM 客户端（用于因果分析）
    llm_client: Option<Arc<dyn LLMClient>>,
}

/// 进化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionConfig {
    /// 是否自动运行进化循环
    pub auto_run_enabled: bool,
    /// 自动运行间隔（秒）
    pub auto_run_interval_seconds: u64,
    /// 触发进化的最小任务数
    pub min_tasks_for_evolution: u32,
    /// 是否自动创建工具（需要用户确认）
    pub auto_create_tools: bool,
    /// 工具创建优先级阈值
    pub tool_creation_priority_threshold: u8,
    /// 是否自动发现外部工具（安全：默认关闭）
    pub auto_discover_external_tools: bool,
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            auto_run_enabled: false,
            auto_run_interval_seconds: 3600, // 1 小时
            min_tasks_for_evolution: 10,
            auto_create_tools: false, // 默认需要用户确认
            tool_creation_priority_threshold: 7,
            auto_discover_external_tools: false,
        }
    }
}

impl SelfImprovementLoop {
    /// 创建新的自进化系统（仅统计模式，不需要 LLM）
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let data_dir = project_root.join(".atlas").join("evolution");

        // 创建数据目录
        std::fs::create_dir_all(&data_dir)
            .with_context(|| "Failed to create evolution data directory")?;

        // 创建各组件（使用 HybridGapDetector）
        let gap_detector = HybridGapDetector::new_statistical_only(data_dir.join("gaps"))?;
        let optimizer = ToolOptimizer::new(data_dir.join("optimizer"))?;
        let reflector = SystemReflector::new(data_dir.join("reflector"))?;
        let creator = ToolCreator::new(&project_root)?;

        // 创建外部工具发现器和注册表
        let external_tool_discovery =
            ExternalToolDiscovery::new().with_created_by("self_improvement_loop");
        let external_tool_registry =
            ExternalToolRegistry::with_storage_dir(data_dir.join("external_tools"))?;

        Ok(Self {
            project_root,
            data_dir,
            gap_detector: Arc::new(RwLock::new(gap_detector)),
            optimizer: Arc::new(RwLock::new(optimizer)),
            reflector: Arc::new(RwLock::new(reflector)),
            creator: Arc::new(RwLock::new(creator)),
            external_tool_discovery: Arc::new(RwLock::new(external_tool_discovery)),
            external_tool_registry: Arc::new(RwLock::new(external_tool_registry)),
            config: EvolutionConfig::default(),
            enabled: true,
            llm_client: None,
        })
    }

    /// 创建新的自进化系统（带 LLM 客户端，启用因果分析）
    pub fn with_llm<P: AsRef<Path>>(
        project_root: P,
        llm_client: Arc<dyn LLMClient>,
    ) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let data_dir = project_root.join(".atlas").join("evolution");

        // 创建数据目录
        std::fs::create_dir_all(&data_dir)
            .with_context(|| "Failed to create evolution data directory")?;

        // 创建各组件（使用 HybridGapDetector 带 LLM）
        let hybrid_config = HybridConfig::default();
        let gap_detector = HybridGapDetector::new(
            data_dir.join("gaps"),
            Arc::clone(&llm_client),
            hybrid_config,
        )?;
        let optimizer = ToolOptimizer::new(data_dir.join("optimizer"))?;
        let reflector = SystemReflector::new(data_dir.join("reflector"))?;
        let creator = ToolCreator::new(&project_root)?;

        // 创建外部工具发现器和注册表
        let external_tool_discovery =
            ExternalToolDiscovery::new().with_created_by("self_improvement_loop");
        let external_tool_registry =
            ExternalToolRegistry::with_storage_dir(data_dir.join("external_tools"))?;

        Ok(Self {
            project_root,
            data_dir,
            gap_detector: Arc::new(RwLock::new(gap_detector)),
            optimizer: Arc::new(RwLock::new(optimizer)),
            reflector: Arc::new(RwLock::new(reflector)),
            creator: Arc::new(RwLock::new(creator)),
            external_tool_discovery: Arc::new(RwLock::new(external_tool_discovery)),
            external_tool_registry: Arc::new(RwLock::new(external_tool_registry)),
            config: EvolutionConfig::default(),
            enabled: true,
            llm_client: Some(llm_client),
        })
    }

    /// 从配置创建
    pub fn with_config<P: AsRef<Path>>(project_root: P, config: EvolutionConfig) -> Result<Self> {
        let mut system = Self::new(project_root)?;
        system.config = config;
        Ok(system)
    }

    /// 记录任务执行（用于缺口检测）
    pub fn record_task(&self, record: TaskExecutionRecord) {
        if !self.enabled {
            return;
        }

        let detector = self.gap_detector.clone();
        futures::executor::block_on(async {
            let mut d = detector.write().await;
            d.record_task(record);
        });
    }

    /// 更新工具指标（用于优化分析）
    pub fn update_tool_metrics(&self, metrics: ToolMetrics) {
        if !self.enabled {
            return;
        }

        let optimizer = self.optimizer.clone();
        futures::executor::block_on(async {
            let mut o = optimizer.write().await;
            o.update_metrics(metrics);
        });
    }

    /// 运行进化循环（异步版本，支持因果分析）
    pub async fn run_evolution_cycle_async(&self) -> Result<EvolutionCycleReport> {
        let start_time = std::time::Instant::now();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut created_tools = Vec::new();
        let mut registered_tools = Vec::new();
        let mut status = CycleStatus::Success;

        // 0. 重置周期统计（重要：清理 API 预算计数）
        {
            let mut detector = self.gap_detector.write().await;
            detector.reset_cycle_stats();
            // 锁在此处释放
        }

        // 1. 检测工具缺口（使用混合检测器）
        let gaps: Vec<HybridToolGap> = {
            let mut detector = self.gap_detector.write().await;
            let gaps = detector.detect_gaps().await;
            // 锁在此处释放
            gaps
        };

        // 2. 分析工具优化建议
        let suggestions = {
            let mut optimizer = self.optimizer.write().await;
            optimizer.analyze_and_optimize();
            let suggestions = optimizer.get_suggestions().to_vec();
            // 锁在此处释放
            suggestions
        };

        // 3. 生成系统健康报告
        let health_score = {
            let report_result = {
                let mut reflector = self.reflector.write().await;
                reflector.generate_health_report()
            };
            match report_result {
                Ok(report) => report.system_health.overall_health,
                Err(e) => {
                    tracing::warn!("System reflection failed: {}", e);
                    if status == CycleStatus::Success {
                        status = CycleStatus::PartialSuccess;
                    }
                    0.0
                }
            }
        };

        // 4. 根据缺口创建工具（如果启用）
        if self.config.auto_create_tools {
            for gap in &gaps {
                if gap.priority >= self.config.tool_creation_priority_threshold {
                    // 决策：创建 Rust 工具还是封装外部工具？
                    match self.decide_tool_creation_strategy_from_hybrid(gap) {
                        ToolCreationStrategy::ExternalTool => {
                            // 尝试封装外部工具
                            match self.wrap_external_tool_for_hybrid_gap(gap) {
                                Ok(tool_name) => {
                                    let tool_name_clone = tool_name.clone();
                                    created_tools.push(tool_name.clone());
                                    registered_tools.push(tool_name_clone);
                                    tracing::info!(
                                        "Created external tool wrapper for gap {}: {}",
                                        gap.id,
                                        tool_name
                                    );
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to wrap external tool for gap {}: {}",
                                        gap.id,
                                        e
                                    );
                                    // 回退到创建 Rust 工具
                                    match self.create_tool_from_hybrid_gap(gap) {
                                        Ok(name) => {
                                            created_tools.push(name.clone());
                                            registered_tools.push(name);
                                        }
                                        Err(e2) => {
                                            tracing::warn!(
                                                "Failed to create Rust tool for gap {}: {}",
                                                gap.id,
                                                e2
                                            );
                                            if status == CycleStatus::Success {
                                                status = CycleStatus::PartialSuccess;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ToolCreationStrategy::RustTool => {
                            match self.create_tool_from_hybrid_gap(gap) {
                                Ok(tool_name) => {
                                    created_tools.push(tool_name.clone());
                                    registered_tools.push(tool_name);
                                }
                                Err(e) => {
                                    tracing::warn!(
                                        "Failed to create tool from gap {}: {}",
                                        gap.id,
                                        e
                                    );
                                    if status == CycleStatus::Success {
                                        status = CycleStatus::PartialSuccess;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 5. 扫描并注册现有外部工具
        match self.discover_and_register_external_tools().await {
            Ok(tools) => {
                registered_tools.extend(tools);
            }
            Err(e) => {
                tracing::warn!("Failed to discover external tools: {}", e);
                if status == CycleStatus::Success {
                    status = CycleStatus::PartialSuccess;
                }
            }
        }

        let cycle_duration_ms = start_time.elapsed().as_millis() as u64;

        let report = EvolutionCycleReport {
            timestamp,
            detected_gaps_count: gaps.len() as u32,
            optimization_suggestions_count: suggestions.len() as u32,
            system_health_score: health_score,
            created_tools,
            registered_tools,
            cycle_duration_ms,
            status,
        };

        // 保存报告
        self.save_cycle_report(&report)?;

        Ok(report)
    }

    /// 运行进化循环（同步版本，仅统计模式）
    pub fn run_evolution_cycle(&self) -> Result<EvolutionCycleReport> {
        // 使用 futures executor 运行异步版本
        futures::executor::block_on(self.run_evolution_cycle_async())
    }

    /// 从缺口创建工具（同步版本，使用 block_on）
    fn create_tool_from_gap(&self, gap: &ToolGap) -> Result<String> {
        let creator = self.creator.clone();
        let tool_name = if let Some(ref suggested) = gap.suggested_tool_name {
            suggested.clone()
        } else {
            format!("auto_{}", gap.id.replace("gap_", ""))
        };

        // 生成参数定义（从缺口描述中提取）
        let parameters = self.extract_parameters_from_gap(gap);

        let request = ToolCreationRequest {
            tool_name: tool_name.clone(),
            description: gap.description.clone(),
            domain: self.infer_domain_from_gap(gap),
            tags: vec!["auto_generated".to_string(), format!("gap_{}", gap.id)],
            parameters,
            return_type: "String".to_string(),
            creation_reason: gap.description.clone(),
            priority: gap.priority,
        };

        futures::executor::block_on(async {
            let c = creator.read().await;
            c.create_tool(request)
        })?;
        Ok(tool_name)
    }

    /// 从缺口提取参数
    fn extract_parameters_from_gap(&self, gap: &ToolGap) -> Vec<ParameterDef> {
        // 简化实现：根据缺口类型生成基本参数
        match gap.gap_type {
            crate::autonomy::gap_detector::GapType::MissingTool => {
                vec![ParameterDef {
                    name: "input".to_string(),
                    param_type: "string".to_string(),
                    description: "输入参数".to_string(),
                    required: true,
                    default_value: None,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// 从缺口推断领域
    fn infer_domain_from_gap(&self, gap: &ToolGap) -> String {
        // 根据缺口描述关键词推断领域
        let desc_lower = gap.description.to_lowercase();

        if desc_lower.contains("文件") || desc_lower.contains("file") {
            "file_ops".to_string()
        } else if desc_lower.contains("网络")
            || desc_lower.contains("network")
            || desc_lower.contains("http")
        {
            "network_ops".to_string()
        } else if desc_lower.contains("系统")
            || desc_lower.contains("system")
            || desc_lower.contains("进程")
        {
            "system_ops".to_string()
        } else if desc_lower.contains("代码") || desc_lower.contains("code") {
            "code_ops".to_string()
        } else if desc_lower.contains("数据")
            || desc_lower.contains("data")
            || desc_lower.contains("json")
        {
            "data_ops".to_string()
        } else {
            "general".to_string()
        }
    }

    /// 保存循环报告
    fn save_cycle_report(&self, report: &EvolutionCycleReport) -> Result<()> {
        let reports_dir = self.data_dir.join("reports");
        std::fs::create_dir_all(&reports_dir)?;

        let file_path = reports_dir.join(format!("cycle_{}.json", report.timestamp));
        let json = serde_json::to_string_pretty(report)?;
        std::fs::write(&file_path, &json)?;

        // 保存最新报告
        let latest_path = reports_dir.join("latest_cycle.json");
        std::fs::write(&latest_path, &json)?;

        Ok(())
    }

    /// 从混合缺口创建工具（同步版本，使用 block_on）
    fn create_tool_from_hybrid_gap(&self, gap: &HybridToolGap) -> Result<String> {
        let creator = self.creator.clone();
        let tool_name = if let Some(ref suggested) = gap.suggested_tool_name {
            suggested.clone()
        } else {
            format!("auto_{}", gap.id.replace("gap_", ""))
        };

        // 生成参数定义（从缺口描述中提取）
        let parameters = self.extract_parameters_from_hybrid_gap(gap);

        let request = ToolCreationRequest {
            tool_name: tool_name.clone(),
            description: gap.description.clone(),
            domain: self.infer_domain_from_hybrid_gap(gap),
            tags: vec!["auto_generated".to_string(), format!("gap_{}", gap.id)],
            parameters,
            return_type: "String".to_string(),
            creation_reason: gap.description.clone(),
            priority: gap.priority,
        };

        futures::executor::block_on(async {
            let c = creator.read().await;
            c.create_tool(request)
        })?;
        Ok(tool_name)
    }

    /// 从混合缺口提取参数
    fn extract_parameters_from_hybrid_gap(&self, gap: &HybridToolGap) -> Vec<ParameterDef> {
        // 简化实现：根据缺口类型生成基本参数
        match gap.gap_type {
            GapType::MissingTool => {
                vec![ParameterDef {
                    name: "input".to_string(),
                    param_type: "string".to_string(),
                    description: "输入参数".to_string(),
                    required: true,
                    default_value: None,
                }]
            }
            _ => Vec::new(),
        }
    }

    /// 从混合缺口推断领域
    fn infer_domain_from_hybrid_gap(&self, gap: &HybridToolGap) -> String {
        // 根据缺口描述关键词推断领域
        let desc_lower = gap.description.to_lowercase();

        if desc_lower.contains("文件") || desc_lower.contains("file") {
            "file_ops".to_string()
        } else if desc_lower.contains("网络")
            || desc_lower.contains("network")
            || desc_lower.contains("http")
        {
            "network_ops".to_string()
        } else if desc_lower.contains("系统")
            || desc_lower.contains("system")
            || desc_lower.contains("进程")
        {
            "system_ops".to_string()
        } else if desc_lower.contains("代码") || desc_lower.contains("code") {
            "code_ops".to_string()
        } else if desc_lower.contains("数据")
            || desc_lower.contains("data")
            || desc_lower.contains("json")
        {
            "data_ops".to_string()
        } else {
            "general".to_string()
        }
    }

    /// 决策工具创建策略（混合缺口版本）
    fn decide_tool_creation_strategy_from_hybrid(
        &self,
        gap: &HybridToolGap,
    ) -> ToolCreationStrategy {
        let desc_lower = gap.description.to_lowercase();

        // 检查是否有匹配的现有 CLI 工具
        let has_matching_cli = self.check_existing_cli(&desc_lower);

        // 检查是否需要快速原型
        let is_rapid_proto = desc_lower.contains("script")
            || desc_lower.contains("quick")
            || desc_lower.contains("prototype")
            || desc_lower.contains("temporary");

        // 检查是否需要高性能
        let requires_high_perf = desc_lower.contains("high performance")
            || desc_lower.contains("low latency")
            || desc_lower.contains("real-time");

        // 检查任务是否复杂
        let is_complex = desc_lower.contains("complex")
            || desc_lower.contains("algorithm")
            || desc_lower.contains("optimization")
            || desc_lower.contains("concurrent");

        // 如果有因果证据，考虑其影响
        let has_strong_causal_evidence = gap
            .causal_evidence
            .as_ref()
            .map(|c| c.llm_confidence > 0.8)
            .unwrap_or(false);

        // 决策树
        if requires_high_perf && is_complex {
            ToolCreationStrategy::RustTool
        } else if has_matching_cli || is_rapid_proto || (has_strong_causal_evidence && !is_complex)
        {
            // 有匹配 CLI、快速原型需求、或强因果证据但任务不复杂，优先封装外部工具
            ToolCreationStrategy::ExternalTool
        } else {
            ToolCreationStrategy::RustTool
        }
    }

    /// 为混合缺口封装外部工具（同步版本，使用 block_on）
    fn wrap_external_tool_for_hybrid_gap(&self, gap: &HybridToolGap) -> Result<String> {
        let desc_lower = gap.description.to_lowercase();

        // 尝试找到匹配的 CLI 工具
        let cli_tools = vec![
            (
                "git",
                "version_control",
                vec!["git", "version control", "commit", "branch"],
            ),
            ("docker", "container", vec!["docker", "container", "image"]),
            ("curl", "network", vec!["curl", "http request", "api"]),
            ("grep", "text_processing", vec!["grep", "search text"]),
            ("jq", "data_processing", vec!["jq", "json"]),
        ];

        for (cli, domain, keywords) in cli_tools {
            for keyword in &keywords {
                if desc_lower.contains(keyword) {
                    // 创建外部工具包装器
                    let metadata = self.create_cli_tool_metadata(cli, domain, gap)?;

                    // 注册到外部工具注册表
                    let registry = self.external_tool_registry.clone();
                    futures::executor::block_on(async {
                        let r = registry.read().await;
                        r.register_from_metadata(metadata.clone())
                    })?;

                    tracing::info!("Created external tool wrapper for CLI: {}", cli);
                    return Ok(metadata.name);
                }
            }
        }

        // 未找到匹配的外部工具
        anyhow::bail!("No matching external tool found for gap")
    }

    /// 创建 CLI 工具元数据（混合缺口版本）
    fn create_cli_tool_metadata(
        &self,
        cli: &str,
        domain: &str,
        gap: &HybridToolGap,
    ) -> Result<ExternalToolMetadata> {
        use crate::external_process::metadata::{ExternalToolType, ProcessConfig, RiskLevel};

        let config = ProcessConfig::new(cli).with_timeout(30000);

        let input_schema = serde_json::json!({
            "type": "object",
            "properties": {
                "args": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Command line arguments"
                }
            }
        });

        let metadata = ExternalToolMetadata::new(
            format!("cli_{}", cli),
            format!("CLI tool: {} - {}", cli, gap.description),
            ExternalToolType::process(config),
            input_schema,
            domain,
            "self_improvement_loop",
        )
        .with_tags(vec![
            "cli".to_string(),
            "auto_generated".to_string(),
            cli.to_string(),
        ])
        .with_risk_level(RiskLevel::Medium);

        Ok(metadata)
    }

    /// 发现并注册外部工具（异步版本）
    async fn discover_and_register_external_tools(&self) -> Result<Vec<String>> {
        if !self.config.auto_discover_external_tools {
            tracing::info!("外部工具自动发现已禁用（安全配置）");
            return Ok(Vec::new());
        }

        let mut registered_names = Vec::new();

        // 扫描常见目录中的脚本
        let script_dirs = vec![
            self.project_root.join("scripts"),
            self.project_root.join("tools"),
        ];

        for dir in &script_dirs {
            if dir.exists() {
                let tools = {
                    let mut discovery = self.external_tool_discovery.write().await;
                    let tools = discovery.scan_scripts(dir).await;
                    // 锁在此处释放
                    tools
                };

                match tools {
                    Ok(tools) => {
                        for tool in tools {
                            let name = tool.name.clone();
                            let registry = self.external_tool_registry.read().await;
                            if let Err(e) = registry.register_from_metadata(tool) {
                                tracing::warn!("Failed to register script {}: {}", name, e);
                            } else {
                                registered_names.push(name);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to scan scripts in {:?}: {}", dir, e);
                    }
                }
            }
        }

        Ok(registered_names)
    }

    /// 获取缺口检测器（混合版本）
    pub fn get_gap_detector(&self) -> Arc<RwLock<HybridGapDetector>> {
        Arc::clone(&self.gap_detector)
    }

    /// 获取工具优化器
    pub fn get_optimizer(&self) -> Arc<RwLock<ToolOptimizer>> {
        Arc::clone(&self.optimizer)
    }

    /// 获取系统反思器
    pub fn get_reflector(&self) -> Arc<RwLock<SystemReflector>> {
        Arc::clone(&self.reflector)
    }

    /// 启用/禁用自进化
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// 检查是否启用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 获取配置
    pub fn get_config(&self) -> &EvolutionConfig {
        &self.config
    }
}

/// 工具指标构建器（简化指标创建）
pub struct ToolMetricsBuilder {
    tool_name: String,
    total_calls: u32,
    success_count: u32,
    avg_execution_time_ms: f64,
    avg_satisfaction: f32,
    tags: Vec<String>,
    dependencies: Vec<String>,
}

impl ToolMetricsBuilder {
    pub fn new(tool_name: &str) -> Self {
        Self {
            tool_name: tool_name.to_string(),
            total_calls: 0,
            success_count: 0,
            avg_execution_time_ms: 0.0,
            avg_satisfaction: 3.0,
            tags: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    pub fn calls(mut self, total: u32, success: u32) -> Self {
        self.total_calls = total;
        self.success_count = success;
        self
    }

    pub fn execution_time(mut self, ms: f64) -> Self {
        self.avg_execution_time_ms = ms;
        self
    }

    pub fn satisfaction(mut self, score: f32) -> Self {
        self.avg_satisfaction = score.clamp(1.0, 5.0);
        self
    }

    pub fn tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn depends_on<S: Into<String>>(mut self, dep: S) -> Self {
        self.dependencies.push(dep.into());
        self
    }

    pub fn build(self) -> ToolMetrics {
        ToolMetrics {
            tool_name: self.tool_name,
            total_calls: self.total_calls,
            success_count: self.success_count,
            failure_count: self.total_calls - self.success_count,
            avg_execution_time_ms: self.avg_execution_time_ms,
            last_used_timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            avg_satisfaction: self.avg_satisfaction,
            tags: self.tags,
            dependencies: self.dependencies,
        }
    }
}

/// Tool creation strategy decision
#[derive(Debug, Clone, PartialEq)]
enum ToolCreationStrategy {
    /// Create a Rust tool
    RustTool,
    /// Wrap an external tool (CLI/HTTP/Script)
    ExternalTool,
}

impl SelfImprovementLoop {
    /// Decide whether to create a Rust tool or wrap an external tool
    fn decide_tool_creation_strategy(&self, gap: &ToolGap) -> ToolCreationStrategy {
        let desc_lower = gap.description.to_lowercase();

        // Check if there's an existing CLI tool that matches the gap
        let has_matching_cli = self.check_existing_cli(&desc_lower);

        // Check if rapid prototyping is needed (keywords indicating simple tasks)
        let is_rapid_proto = desc_lower.contains("script")
            || desc_lower.contains("quick")
            || desc_lower.contains("prototype")
            || desc_lower.contains("temporary");

        // Check if high performance is required
        let requires_high_perf = desc_lower.contains("high performance")
            || desc_lower.contains("low latency")
            || desc_lower.contains("real-time");

        // Check if the task is complex (requires Rust)
        let is_complex = desc_lower.contains("complex")
            || desc_lower.contains("algorithm")
            || desc_lower.contains("optimization")
            || desc_lower.contains("concurrent");

        // Decision tree
        if requires_high_perf && is_complex {
            ToolCreationStrategy::RustTool
        } else if has_matching_cli || is_rapid_proto {
            ToolCreationStrategy::ExternalTool
        } else {
            ToolCreationStrategy::RustTool
        }
    }

    /// Check if there's an existing CLI tool that matches the gap description
    fn check_existing_cli(&self, description: &str) -> bool {
        // Common CLI tools that might match gap descriptions
        let cli_keywords = vec![
            (
                "git",
                vec!["git", "version control", "commit", "branch", "merge"],
            ),
            ("docker", vec!["docker", "container", "image", "compose"]),
            (
                "npm",
                vec!["npm", "node package", "yarn", "install package"],
            ),
            ("cargo", vec!["cargo", "rust package", "rust build"]),
            ("curl", vec!["curl", "http request", "api call"]),
            ("grep", vec!["grep", "search text", "pattern search"]),
            ("jq", vec!["jq", "json parse", "json query"]),
            ("find", vec!["find", "search file", "locate file"]),
        ];

        for (cli, keywords) in cli_keywords {
            for keyword in keywords {
                if description.contains(keyword) {
                    // Check if the CLI exists
                    if which::which(cli).is_ok() {
                        return true;
                    }
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_evolution_system_creation() {
        let temp_dir = TempDir::new().unwrap();
        let evolution = SelfImprovementLoop::new(temp_dir.path()).unwrap();
        assert!(evolution.is_enabled());
    }

    #[test]
    fn test_record_task() {
        let temp_dir = TempDir::new().unwrap();
        let evolution = SelfImprovementLoop::new(temp_dir.path()).unwrap();

        let record = TaskExecutionRecord {
            task_id: "test_1".to_string(),
            task_description: "Test".to_string(),
            success: false,
            used_tools: vec![],
            execution_time_ms: 100,
            failure_reason: Some("Test failure".to_string()),
            user_satisfaction: Some(1),
        };

        evolution.record_task(record);

        // 验证任务被记录（简化测试）
        let _detector = evolution.get_gap_detector();
    }

    #[test]
    fn test_metrics_builder() {
        let metrics = ToolMetricsBuilder::new("test_tool")
            .calls(100, 95)
            .execution_time(50.0)
            .satisfaction(4.5)
            .tag("file")
            .tag("io")
            .depends_on("read_file")
            .build();

        assert_eq!(metrics.tool_name, "test_tool");
        assert_eq!(metrics.total_calls, 100);
        assert_eq!(metrics.success_count, 95);
        assert_eq!(metrics.tags.len(), 2);
        assert_eq!(metrics.dependencies.len(), 1);
    }

    /// 端到端测试：模拟完整的自进化闭环
    /// 1. 记录多个失败任务
    /// 2. 运行进化循环检测缺口
    /// 3. 验证缺口被正确识别
    #[test]
    fn test_end_to_end_evolution_cycle() {
        let temp_dir = TempDir::new().unwrap();

        // 创建配置（启用自动创建工具）
        let mut config = EvolutionConfig::default();
        config.auto_create_tools = false; // 测试中不实际创建工具
        config.tool_creation_priority_threshold = 5;

        let evolution = SelfImprovementLoop::with_config(temp_dir.path(), config).unwrap();

        // 1. 记录多个失败任务（模拟需要批量文件处理工具的场景）
        for i in 0..10 {
            let record = TaskExecutionRecord {
                task_id: format!("task_{}", i),
                task_description: "Batch process multiple files".to_string(),
                success: false,
                used_tools: vec!["read_file".to_string(), "write_file".to_string()],
                execution_time_ms: 500 + i * 100,
                failure_reason: Some(
                    "No efficient way to process multiple files at once".to_string(),
                ),
                user_satisfaction: Some(2),
            };
            evolution.record_task(record);
        }

        // 记录一些成功任务作为对比
        for i in 0..5 {
            let record = TaskExecutionRecord {
                task_id: format!("success_task_{}", i),
                task_description: "Single file operation".to_string(),
                success: true,
                used_tools: vec!["read_file".to_string()],
                execution_time_ms: 100,
                failure_reason: None,
                user_satisfaction: Some(5),
            };
            evolution.record_task(record);
        }

        // 2. 运行进化循环
        let report = evolution.run_evolution_cycle().unwrap();

        // 3. 验证结果
        assert!(report.detected_gaps_count > 0, "应该检测到至少一个工具缺口");
        assert_eq!(report.status, CycleStatus::Success);

        println!("=== 自进化闭环端到端测试 ===");
        println!("检测到的缺口数量：{}", report.detected_gaps_count);
        println!("优化建议数量：{}", report.optimization_suggestions_count);
        println!("系统健康评分：{:.2}", report.system_health_score);
        println!("循环耗时：{}ms", report.cycle_duration_ms);

        // 验证缺口检测器中有数据
        let detector = evolution.get_gap_detector();
        let gaps = futures::executor::block_on(async {
            let mut d = detector.write().await;
            d.detect_gaps().await
        });
        assert!(!gaps.is_empty(), "缺口检测器应该包含检测到的缺口");
    }

    /// 测试：验证混合检测器的统计证据提取
    #[test]
    fn test_hybrid_gap_detection_from_failures() {
        let temp_dir = TempDir::new().unwrap();
        let evolution = SelfImprovementLoop::new(temp_dir.path()).unwrap();

        // 记录具有相同失败原因的任务
        let failure_reason = "Missing batch processing capability";
        for i in 0..5 {
            let record = TaskExecutionRecord {
                task_id: format!("batch_task_{}", i),
                task_description: "Process multiple files in batch".to_string(),
                success: false,
                used_tools: vec!["file_ops".to_string()],
                execution_time_ms: 1000,
                failure_reason: Some(failure_reason.to_string()),
                user_satisfaction: Some(2),
            };
            evolution.record_task(record);
        }

        // 运行检测
        let gaps = {
            let detector = evolution.gap_detector.clone();
            futures::executor::block_on(async {
                let mut d = detector.write().await;
                d.detect_gaps().await
            })
        };

        // 验证检测到了缺口
        assert!(!gaps.is_empty(), "应该检测到工具缺口");

        // 验证统计证据
        for gap in &gaps {
            assert!(gap.statistical_evidence.failure_rate > 0.0);
            assert!(gap.statistical_evidence.affected_tasks_count > 0);
            assert!(gap.hybrid_confidence >= 0.0 && gap.hybrid_confidence <= 1.0);
        }
    }
}
