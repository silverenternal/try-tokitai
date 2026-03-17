//! 自进化闭环系统
//!
//! 整合 ToolGapDetector + ToolOptimizer + SystemReflector + ToolCreator
//! 实现从 0 到 1 的工具发现和创造
//!
//! ## 工作流程
//! 1. **检测** - ToolGapDetector 从失败任务中发现工具缺口
//! 2. **优化** - ToolOptimizer 分析现有工具，提出优化建议
//! 3. **反思** - SystemReflector 生成系统体检报告，发现覆盖不足的领域
//! 4. **创造** - ToolCreator 根据缺口和建议创造新工具
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

use crate::autonomy::{
    gap_detector::{ToolGapDetector, TaskExecutionRecord, ToolGap},
    tool_optimizer::{ToolOptimizer, ToolMetrics},
    system_reflector::SystemReflector,
    tool_creator::{ToolCreator, ToolCreationRequest, ParameterDef},
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

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
    /// 工具缺口检测器
    gap_detector: Arc<RwLock<ToolGapDetector>>,
    /// 工具优化器
    optimizer: Arc<RwLock<ToolOptimizer>>,
    /// 系统反思器
    reflector: Arc<RwLock<SystemReflector>>,
    /// 工具创建器
    creator: Arc<RwLock<ToolCreator>>,
    /// 配置
    config: EvolutionConfig,
    /// 是否启用
    enabled: bool,
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
}

impl Default for EvolutionConfig {
    fn default() -> Self {
        Self {
            auto_run_enabled: false,
            auto_run_interval_seconds: 3600, // 1 小时
            min_tasks_for_evolution: 10,
            auto_create_tools: false, // 默认需要用户确认
            tool_creation_priority_threshold: 7,
        }
    }
}

impl SelfImprovementLoop {
    /// 创建新的自进化系统
    pub fn new<P: AsRef<Path>>(project_root: P) -> Result<Self> {
        let project_root = project_root.as_ref().to_path_buf();
        let data_dir = project_root.join(".tokitai").join("evolution");
        
        // 创建数据目录
        std::fs::create_dir_all(&data_dir)
            .with_context(|| "Failed to create evolution data directory")?;
        
        // 创建各组件
        let gap_detector = ToolGapDetector::new(data_dir.join("gaps"))?;
        let optimizer = ToolOptimizer::new(data_dir.join("optimizer"))?;
        let reflector = SystemReflector::new(data_dir.join("reflector"))?;
        let creator = ToolCreator::new(&project_root)?;
        
        Ok(Self {
            project_root,
            data_dir,
            gap_detector: Arc::new(RwLock::new(gap_detector)),
            optimizer: Arc::new(RwLock::new(optimizer)),
            reflector: Arc::new(RwLock::new(reflector)),
            creator: Arc::new(RwLock::new(creator)),
            config: EvolutionConfig::default(),
            enabled: true,
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
        
        let mut detector = self.gap_detector.write();
        detector.record_task(record);
    }

    /// 更新工具指标（用于优化分析）
    pub fn update_tool_metrics(&self, metrics: ToolMetrics) {
        if !self.enabled {
            return;
        }
        
        let mut optimizer = self.optimizer.write();
        optimizer.update_metrics(metrics);
    }

    /// 运行进化循环
    pub fn run_evolution_cycle(&self) -> Result<EvolutionCycleReport> {
        let start_time = std::time::Instant::now();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let mut created_tools = Vec::new();
        let mut registered_tools = Vec::new();
        let mut status = CycleStatus::Success;
        
        // 1. 检测工具缺口
        let gaps = {
            let mut detector = self.gap_detector.write();
            detector.analyze_and_detect();
            detector.get_gaps().to_vec()
        };
        
        // 2. 分析工具优化建议
        let suggestions = {
            let mut optimizer = self.optimizer.write();
            optimizer.analyze_and_optimize();
            optimizer.get_suggestions().to_vec()
        };
        
        // 3. 生成系统健康报告
        let health_score = {
            let mut reflector = self.reflector.write();
            match reflector.generate_health_report() {
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
                    match self.create_tool_from_gap(gap) {
                        Ok(tool_name) => {
                            created_tools.push(tool_name.clone());
                            registered_tools.push(tool_name);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to create tool from gap {}: {}", gap.id, e);
                            if status == CycleStatus::Success {
                                status = CycleStatus::PartialSuccess;
                            }
                        }
                    }
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

    /// 从缺口创建工具
    fn create_tool_from_gap(&self, gap: &ToolGap) -> Result<String> {
        let creator = self.creator.read();
        
        // 生成工具名称
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
        
        let result = creator.create_tool(request)?;
        
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
        } else if desc_lower.contains("网络") || desc_lower.contains("network") || desc_lower.contains("http") {
            "network_ops".to_string()
        } else if desc_lower.contains("系统") || desc_lower.contains("system") || desc_lower.contains("进程") {
            "system_ops".to_string()
        } else if desc_lower.contains("代码") || desc_lower.contains("code") {
            "code_ops".to_string()
        } else if desc_lower.contains("数据") || desc_lower.contains("data") || desc_lower.contains("json") {
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

    /// 获取缺口检测器
    pub fn get_gap_detector(&self) -> Arc<RwLock<ToolGapDetector>> {
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
        self.avg_satisfaction = score.min(5.0).max(1.0);
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
}
