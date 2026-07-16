//! 学术论文实验框架核心模块
//!
//! # 设计目标
//! 本模块提供严谨、可复现的实验框架，支持 AAAI/ACL/EMNLP 等顶会论文的实验需求
//!
//! # 实验类型
//! 1. **对比实验 (Comparative Experiment)**: 对比完整系统 vs 基线系统
//! 2. **消融实验 (Ablation Study)**: 验证各组件的贡献
//! 3. **案例分析 (Case Study)**: 深入分析典型场景
//! 4. **成本效益分析 (Cost-Benefit Analysis)**: API 成本 vs 性能提升
//!
//! # 使用示例
//! ```rust,ignore
//! use crate::experiments::framework::*;
//!
//! // 创建实验运行器
//! let mut runner = ExperimentRunner::new(ExperimentConfig {
//!     name: "aaai2027_main_experiment".to_string(),
//!     output_dir: PathBuf::from("experiments/aaai2027"),
//!     ..Default::default()
//! });
//!
//! // 运行对比实验
//! let comparative = ComparativeExperiment::new();
//! let results = runner.run_comparative(comparative).await?;
//!
//! // 运行消融实验
//! let ablation = AblationStudy::new();
//! let ablation_results = runner.run_ablation(ablation).await?;
//!
//! // 生成报告
//! let report = ReportGenerator::generate(&results)?;
//! ```

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use tokio::sync::RwLock;
use tracing::{info, debug};

use crate::autonomy::{
    hybrid_gap_detector::{HybridGapDetector, HybridConfig, HybridToolGap},
    gap_detector::TaskExecutionRecord,
    self_improvement_loop::{EvolutionConfig, EvolutionCycleReport},
};

/// 实验唯一标识符
pub type ExperimentId = String;

/// 实验组别标识符
pub type GroupId = String;

/// 实验框架版本（用于复现性）
pub const EXPERIMENT_FRAMEWORK_VERSION: &str = "1.0.0";

// ============================================================================
// 实验配置
// ============================================================================

/// 实验配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentConfig {
    /// 实验名称
    pub name: String,
    /// 实验描述
    pub description: String,
    /// 输出目录
    pub output_dir: PathBuf,
    /// Git 提交哈希（用于复现性）
    pub git_commit: String,
    /// 实验开始时间
    pub start_time: Option<DateTime<Utc>>,
    /// 实验结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 随机种子（用于可复现性）
    pub random_seed: u64,
    /// 实验元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for ExperimentConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            output_dir: PathBuf::from("experiments"),
            git_commit: get_git_commit().unwrap_or_else(|_| "unknown".to_string()),
            start_time: None,
            end_time: None,
            random_seed: 42,
            metadata: HashMap::new(),
        }
    }
}

/// 获取当前 Git 提交哈希
fn get_git_commit() -> Result<String> {
    // 尝试从环境变量获取（CI/CD 环境）
    if let Ok(commit) = std::env::var("GIT_COMMIT") {
        return Ok(commit);
    }
    
    // 尝试从.git 目录读取
    let git_path = PathBuf::from(".git/HEAD");
    if git_path.exists() {
        let content = std::fs::read_to_string(&git_path)?;
        if content.starts_with("ref:") {
            // 引用文件，需要解析
            let ref_path = content.trim_start_matches("ref: ").trim();
            let full_ref_path = PathBuf::from(".git").join(ref_path);
            if full_ref_path.exists() {
                return Ok(std::fs::read_to_string(&full_ref_path)?.trim().to_string());
            }
        } else {
            // 直接是提交哈希
            return Ok(content.trim().to_string());
        }
    }
    
    bail!("Could not determine git commit")
}

// ============================================================================
// 实验指标定义
// ============================================================================

/// 核心实验指标（用于论文评估）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreMetrics {
    // === 主要评估指标（论文 Table 1）===
    /// 任务完成率（0.0-1.0）
    pub task_completion_rate: f64,
    /// 平均工具调用次数
    pub avg_tool_calls: f64,
    /// 工具失败率（0.0-1.0）
    pub tool_failure_rate: f64,
    /// 用户满意度（1.0-5.0）
    pub user_satisfaction: f64,
    
    // === 次要评估指标（论文 Table 2）===
    /// 检测到的缺口数量
    pub gaps_detected: u32,
    /// 创建的工具数量
    pub tools_created: u32,
    /// 优化的工具数量
    pub tools_optimized: u32,
    /// 废弃的工具数量
    pub tools_deprecated: u32,
    
    // === 性能指标（论文 Table 3）===
    /// 平均检测延迟（毫秒）
    pub avg_detection_latency_ms: f64,
    /// 平均进化周期耗时（秒）
    pub avg_evolution_cycle_duration_s: f64,
    /// API 调用总次数
    pub total_api_calls: u32,
    /// API 总成本（美元）
    pub total_api_cost_usd: f64,
    
    // === 质量指标（论文 Table 4）===
    /// 缺口检测精确率（0.0-1.0）
    pub gap_detection_precision: f64,
    /// 缺口检测召回率（0.0-1.0）
    pub gap_detection_recall: f64,
    /// 缺口检测 F1 分数（0.0-1.0）
    pub gap_detection_f1: f64,
    /// 代码编译通过率（0.0-1.0）
    pub code_compilation_success_rate: f64,
}

/// 实验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult {
    /// 实验 ID
    pub experiment_id: ExperimentId,
    /// 实验组别
    pub group_id: GroupId,
    /// 实验配置
    pub config: ExperimentConfig,
    /// 核心指标
    pub metrics: CoreMetrics,
    /// 检测到的缺口列表
    pub detected_gaps: Vec<HybridToolGap>,
    /// 任务执行记录
    pub task_records: Vec<TaskExecutionRecord>,
    /// 进化周期报告
    pub evolution_reports: Vec<EvolutionCycleReport>,
    /// 实验日志
    pub logs: Vec<ExperimentLogEntry>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 实验日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentLogEntry {
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 日志级别
    pub level: String,
    /// 日志消息
    pub message: String,
    /// 附加数据
    pub data: Option<serde_json::Value>,
}

// ============================================================================
// 实验组定义
// ============================================================================

/// 实验组类型（用于对比实验和消融实验）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExperimentGroupType {
    // === 对比实验组别 ===
    /// 对照组：无自进化系统
    Control,
    /// 实验组：完整系统
    OursFull,
    
    // === 消融实验组别 ===
    /// 无 Chain-of-Thought 推理
    OursNoCoT,
    /// 无自修正循环
    OursNoFix,
    /// 单智能体（无多智能体协商）
    OursSingleAgent,
    /// 无统计过滤（纯因果推理）
    OursNoStatistical,
    /// 无因果推理（纯统计）
    OursNoCausal,
}

impl ExperimentGroupType {
    /// 获取组别名称
    pub fn name(&self) -> &'static str {
        match self {
            ExperimentGroupType::Control => "Control",
            ExperimentGroupType::OursFull => "Ours-Full",
            ExperimentGroupType::OursNoCoT => "Ours-No-CoT",
            ExperimentGroupType::OursNoFix => "Ours-No-Fix",
            ExperimentGroupType::OursSingleAgent => "Ours-Single-Agent",
            ExperimentGroupType::OursNoStatistical => "Ours-No-Statistical",
            ExperimentGroupType::OursNoCausal => "Ours-No-Causal",
        }
    }
    
    /// 获取组别描述
    pub fn description(&self) -> &'static str {
        match self {
            ExperimentGroupType::Control => "Baseline: No self-evolution system",
            ExperimentGroupType::OursFull => "Full system with all components",
            ExperimentGroupType::OursNoCoT => "Ablation: Remove Chain-of-Thought reasoning",
            ExperimentGroupType::OursNoFix => "Ablation: Remove self-correction loop",
            ExperimentGroupType::OursSingleAgent => "Ablation: Single LLM instead of multi-agent",
            ExperimentGroupType::OursNoStatistical => "Ablation: Pure causal reasoning without statistical filter",
            ExperimentGroupType::OursNoCausal => "Ablation: Pure statistical without causal reasoning",
        }
    }
}

/// 实验组配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentGroupConfig {
    /// 组别类型
    pub group_type: ExperimentGroupType,
    /// 混合检测器配置
    pub hybrid_config: HybridConfig,
    /// 进化配置
    pub evolution_config: EvolutionConfig,
    /// 是否启用因果推理
    pub enable_causal: bool,
    /// 是否启用统计过滤
    pub enable_statistical: bool,
    /// 是否启用多智能体协商
    pub enable_multi_agent: bool,
    /// 是否启用自修正循环
    pub enable_self_correction: bool,
}

impl ExperimentGroupConfig {
    /// 创建对照组配置
    pub fn control() -> Self {
        Self {
            group_type: ExperimentGroupType::Control,
            hybrid_config: HybridConfig::default(),
            evolution_config: EvolutionConfig::default(),
            enable_causal: false,
            enable_statistical: false,
            enable_multi_agent: false,
            enable_self_correction: false,
        }
    }
    
    /// 创建完整实验组配置
    pub fn ours_full() -> Self {
        Self {
            group_type: ExperimentGroupType::OursFull,
            hybrid_config: HybridConfig::default(),
            evolution_config: EvolutionConfig::default(),
            enable_causal: true,
            enable_statistical: true,
            enable_multi_agent: true,
            enable_self_correction: true,
        }
    }
    
    /// 创建消融实验组配置（无 CoT）
    pub fn ours_no_cot() -> Self {
        let mut config = Self::ours_full();
        config.group_type = ExperimentGroupType::OursNoCoT;
        config.enable_causal = true;
        // 禁用 CoT：使用简化版因果推理 Prompt
        config.hybrid_config.causal_weight = 0.0;
        config
    }
    
    /// 创建消融实验组配置（无自修正）
    pub fn ours_no_fix() -> Self {
        let mut config = Self::ours_full();
        config.group_type = ExperimentGroupType::OursNoFix;
        config.enable_self_correction = false;
        config
    }
    
    /// 创建消融实验组配置（单智能体）
    pub fn ours_single_agent() -> Self {
        let mut config = Self::ours_full();
        config.group_type = ExperimentGroupType::OursSingleAgent;
        config.enable_multi_agent = false;
        config
    }
    
    /// 创建消融实验组配置（无统计过滤）
    pub fn ours_no_statistical() -> Self {
        let mut config = Self::ours_full();
        config.group_type = ExperimentGroupType::OursNoStatistical;
        config.enable_statistical = false;
        config.hybrid_config.statistical_threshold = 0.0; // 不过滤
        config
    }
    
    /// 创建消融实验组配置（无因果推理）
    pub fn ours_no_causal() -> Self {
        let mut config = Self::ours_full();
        config.group_type = ExperimentGroupType::OursNoCausal;
        config.enable_causal = false;
        config.hybrid_config.enable_causal_analysis = false;
        config
    }
}

// ============================================================================
// 实验运行器
// ============================================================================

/// 实验运行器（协调所有实验的执行）
pub struct ExperimentRunner {
    /// 实验配置
    config: ExperimentConfig,
    /// 实验结果存储
    results: Arc<RwLock<HashMap<ExperimentId, ExperimentResult>>>,
    /// 数据目录
    data_dir: PathBuf,
}

impl ExperimentRunner {
    /// 创建新的实验运行器
    pub fn new(config: ExperimentConfig) -> Result<Self> {
        // 创建数据目录
        std::fs::create_dir_all(&config.output_dir)?;
        std::fs::create_dir_all(config.output_dir.join("raw_data"))?;
        std::fs::create_dir_all(config.output_dir.join("reports"))?;
        std::fs::create_dir_all(config.output_dir.join("logs"))?;
        
        Ok(Self {
            config,
            results: Arc::new(RwLock::new(HashMap::new())),
            data_dir: PathBuf::from("experiments"),
        })
    }
    
    /// 运行对比实验
    pub async fn run_comparative(
        &mut self,
        task_dataset: &[TaskExecutionRecord],
    ) -> Result<ComparativeExperimentResult> {
        info!("Starting comparative experiment");
        
        let mut results = HashMap::new();
        
        // 运行对照组
        info!("Running Control group...");
        let control_result = self.run_group(
            ExperimentGroupConfig::control(),
            task_dataset,
        ).await?;
        results.insert(ExperimentGroupType::Control, control_result);
        
        // 运行实验组
        info!("Running Ours-Full group...");
        let full_result = self.run_group(
            ExperimentGroupConfig::ours_full(),
            task_dataset,
        ).await?;
        results.insert(ExperimentGroupType::OursFull, full_result);
        
        Ok(ComparativeExperimentResult {
            experiment_id: Uuid::new_v4().to_string(),
            groups: results,
            timestamp: Utc::now(),
        })
    }
    
    /// 运行消融实验
    pub async fn run_ablation(
        &mut self,
        task_dataset: &[TaskExecutionRecord],
    ) -> Result<AblationExperimentResult> {
        info!("Starting ablation study");
        
        let mut results = HashMap::new();
        
        // 运行所有消融组
        let ablation_configs = vec![
            ExperimentGroupConfig::ours_no_cot(),
            ExperimentGroupConfig::ours_no_fix(),
            ExperimentGroupConfig::ours_single_agent(),
            ExperimentGroupConfig::ours_no_statistical(),
            ExperimentGroupConfig::ours_no_causal(),
        ];
        
        for config in ablation_configs {
            let group_type = config.group_type;
            info!("Running {} group...", group_type.name());
            let result = self.run_group(config, task_dataset).await?;
            results.insert(group_type, result);
        }
        
        Ok(AblationExperimentResult {
            experiment_id: Uuid::new_v4().to_string(),
            groups: results,
            timestamp: Utc::now(),
        })
    }
    
    /// 运行单个实验组
    async fn run_group(
        &self,
        config: ExperimentGroupConfig,
        task_dataset: &[TaskExecutionRecord],
    ) -> Result<GroupExperimentResult> {
        let start_time = Utc::now();
        let mut logs = Vec::new();
        
        // 创建数据目录
        let data_dir = self.config.output_dir.join(format!(
            "group_{}_{}",
            config.group_type.name(),
            start_time.timestamp()
        ));
        
        // 创建混合检测器（仅统计模式用于实验）
        let mut detector = HybridGapDetector::new_statistical_only(data_dir.clone())?;
        
        // 手动添加任务记录到统计检测器
        // 注意：这里使用简化的实验逻辑，实际使用时需要扩展 HybridGapDetector 的 API
        let gaps = Vec::new(); // 实验中简化处理
        
        // 计算指标
        let metrics = self.calculate_metrics(
            &config,
            task_dataset,
            &gaps,
            0.0, // 实验简化
        );
        
        logs.push(ExperimentLogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            message: format!(
                "Group {} completed: detected {} gaps",
                config.group_type.name(),
                gaps.len(),
            ),
            data: None,
        });
        
        Ok(GroupExperimentResult {
            group_type: config.group_type,
            config,
            metrics,
            detected_gaps: gaps,
            task_records: task_dataset.to_vec(),
            logs,
            duration_ms: 0,
        })
    }
    
    /// 计算实验指标
    fn calculate_metrics(
        &self,
        config: &ExperimentGroupConfig,
        task_dataset: &[TaskExecutionRecord],
        gaps: &[HybridToolGap],
        detection_latency_ms: f64,
    ) -> CoreMetrics {
        // 计算任务完成率
        let completed = task_dataset.iter().filter(|t| t.success).count();
        let completion_rate = if task_dataset.is_empty() {
            0.0
        } else {
            completed as f64 / task_dataset.len() as f64
        };
        
        // 计算平均工具调用次数
        let total_calls: usize = task_dataset.iter()
            .map(|t| t.used_tools.len())
            .sum();
        let avg_calls = if task_dataset.is_empty() {
            0.0
        } else {
            total_calls as f64 / task_dataset.len() as f64
        };
        
        // 计算工具失败率
        let failed = task_dataset.iter().filter(|t| !t.success).count();
        let failure_rate = if task_dataset.is_empty() {
            0.0
        } else {
            failed as f64 / task_dataset.len() as f64
        };
        
        // 计算平均满意度
        let satisfaction_sum: f64 = task_dataset.iter()
            .filter_map(|t| t.user_satisfaction)
            .map(|s| s as f64)
            .sum();
        let satisfaction_count = task_dataset.iter()
            .filter(|t| t.user_satisfaction.is_some())
            .count();
        let avg_satisfaction = if satisfaction_count == 0 {
            0.0
        } else {
            satisfaction_sum / satisfaction_count as f64
        };
        
        // 估算 API 成本（基于配置）
        let api_calls = if config.enable_causal {
            gaps.len() as u32
        } else {
            0
        };
        let api_cost = api_calls as f64 * 0.015; // $0.015 per API call
        
        CoreMetrics {
            task_completion_rate: completion_rate,
            avg_tool_calls: avg_calls,
            tool_failure_rate: failure_rate,
            user_satisfaction: avg_satisfaction,
            gaps_detected: gaps.len() as u32,
            tools_created: 0,      // 需要实际运行进化循环
            tools_optimized: 0,
            tools_deprecated: 0,
            avg_detection_latency_ms: detection_latency_ms,
            avg_evolution_cycle_duration_s: 0.0,
            total_api_calls: api_calls,
            total_api_cost_usd: api_cost,
            gap_detection_precision: 0.0,  // 需要人工标注
            gap_detection_recall: 0.0,
            gap_detection_f1: 0.0,
            code_compilation_success_rate: 0.0,
        }
    }
    
    /// 保存实验结果
    pub async fn save_results(&self, result: &ExperimentResult) -> Result<PathBuf> {
        let filename = format!(
            "experiment_{}_{}.json",
            result.group_id,
            result.timestamp.format("%Y%m%d_%H%M%S")
        );
        let path = self.config.output_dir.join("raw_data").join(&filename);
        
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, result)?;
        
        info!("Saved experiment results to {:?}", path);
        Ok(path)
    }
}

// ============================================================================
// 实验结果类型
// ============================================================================

/// 对比实验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparativeExperimentResult {
    /// 实验 ID
    pub experiment_id: ExperimentId,
    /// 各组结果
    pub groups: HashMap<ExperimentGroupType, GroupExperimentResult>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 消融实验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AblationExperimentResult {
    /// 实验 ID
    pub experiment_id: ExperimentId,
    /// 各组结果
    pub groups: HashMap<ExperimentGroupType, GroupExperimentResult>,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
}

/// 单组实验结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupExperimentResult {
    /// 组别类型
    pub group_type: ExperimentGroupType,
    /// 组别配置
    pub config: ExperimentGroupConfig,
    /// 核心指标
    pub metrics: CoreMetrics,
    /// 检测到的缺口
    pub detected_gaps: Vec<HybridToolGap>,
    /// 任务记录
    pub task_records: Vec<TaskExecutionRecord>,
    /// 实验日志
    pub logs: Vec<ExperimentLogEntry>,
    /// 实验耗时（毫秒）
    pub duration_ms: u64,
}

use std::io::BufWriter;
