//! 实验日志系统
//!
//! 用于记录 HybridGapDetector 和 Prompt Engineering 自进化系统的实验数据
//!
//! ## 日志类型
//! - 任务执行日志：记录每个基准任务的执行情况
//! - 自进化日志：记录每次自主进化迭代的详细信息
//! - 指标日志：记录关键性能指标

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use anyhow::Result;

/// 实验日志记录器
#[allow(dead_code)]
pub struct ExperimentLogger {
    /// 日志目录
    log_dir: PathBuf,
    /// 实验组名称（Control, Ours-Full, Ours-Single, Ours-NoCoT, Ours-NoFix）
    experiment_group: String,
    /// 当前实验 ID
    experiment_id: String,
}

/// 任务执行日志
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecutionLog {
    /// 任务 ID
    pub task_id: String,
    /// 任务类别
    pub category: String,
    /// 任务难度
    pub difficulty: String,
    /// 任务描述
    pub description: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 实验组
    pub group: String,
    /// 执行结果
    pub execution: ExecutionResult,
    /// 进化信息
    pub evolution: EvolutionInfo,
}

/// 执行结果
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// 是否成功
    pub success: bool,
    /// 工具调用列表
    pub tool_calls: Vec<ToolCallRecord>,
    /// 总工具调用次数
    pub total_tool_calls: u32,
    /// 执行时间（毫秒）
    pub execution_time_ms: f64,
    /// 用户满意度（1-5）
    pub user_satisfaction: u8,
    /// 错误信息（如果失败）
    pub error_message: Option<String>,
}

/// 工具调用记录
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    /// 工具名称
    pub tool: String,
    /// 参数
    pub args: HashMap<String, serde_json::Value>,
    /// 结果
    pub result: String,
}

/// 进化信息
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvolutionInfo {
    /// 检测到的缺口数量
    pub gaps_detected: u32,
    /// 创建的工具数量
    pub tools_created: u32,
    /// 优化的工具数量
    pub tools_optimized: u32,
}

/// 自进化日志
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfEvolutionLog {
    /// 周期 ID
    pub cycle_id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 实验组
    pub group: String,
    /// 反思结果
    pub reflection: ReflectionResult,
    /// 检测到的缺口
    pub gaps_detected: Vec<GapRecord>,
    /// 执行的操作
    pub actions_taken: Vec<EvolutionAction>,
    /// 指标
    pub metrics: EvolutionMetrics,
}

/// 反思结果
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResult {
    /// 覆盖分数
    pub coverage_score: f32,
    /// 系统性问题
    pub systemic_issues: Vec<String>,
    /// 战略建议
    pub strategic_recommendations: Vec<String>,
}

/// 缺口记录
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapRecord {
    /// 缺口类型
    pub gap_type: String,
    /// 缺口描述
    pub description: String,
    /// 建议工具名称
    pub suggested_name: Option<String>,
    /// 优先级（1-10）
    pub priority: u8,
}

/// 进化操作
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionAction {
    /// 操作类型
    pub action_type: String,
    /// 工具名称
    pub tool_name: Option<String>,
    /// 结果
    pub result: String,
    /// 编译尝试次数
    pub compilation_attempts: Option<u32>,
}

/// 进化指标
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionMetrics {
    /// API 调用次数
    pub api_calls: u32,
    /// API 成本（美元）
    pub api_cost_usd: f32,
    /// 周期持续时间（毫秒）
    pub cycle_duration_ms: f64,
}

#[allow(dead_code)]
impl ExperimentLogger {
    /// 创建新的实验日志记录器
    pub fn new(log_dir: &Path, experiment_group: &str, experiment_id: &str) -> Result<Self> {
        fs::create_dir_all(log_dir)?;
        
        Ok(Self {
            log_dir: log_dir.to_path_buf(),
            experiment_group: experiment_group.to_string(),
            experiment_id: experiment_id.to_string(),
        })
    }

    /// 记录任务执行
    pub fn log_task_execution(&self, log: &TaskExecutionLog) -> Result<()> {
        let file_path = self.log_dir.join(format!("task_{}.jsonl", self.experiment_id));
        
        let json = serde_json::to_string(log)?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;
        
        writeln!(file, "{}", json)?;
        
        Ok(())
    }

    /// 记录自进化周期
    pub fn log_evolution_cycle(&self, log: &SelfEvolutionLog) -> Result<()> {
        let file_path = self.log_dir.join(format!("evolution_{}.jsonl", self.experiment_id));
        
        let json = serde_json::to_string(log)?;
        
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;
        
        writeln!(file, "{}", json)?;
        
        Ok(())
    }

    /// 导出实验结果为 JSON
    pub fn export_summary(&self) -> Result<ExperimentSummary> {
        let task_logs = self.load_task_logs()?;
        let evolution_logs = self.load_evolution_logs()?;
        
        let mut summary = ExperimentSummary::default();
        
        // 计算任务相关指标
        summary.total_tasks = task_logs.len();
        summary.successful_tasks = task_logs.iter().filter(|t| t.execution.success).count();
        summary.task_completion_rate = if summary.total_tasks > 0 {
            summary.successful_tasks as f64 / summary.total_tasks as f64
        } else {
            0.0
        };
        
        // 计算平均工具调用次数
        summary.avg_tool_calls = if !task_logs.is_empty() {
            task_logs.iter().map(|t| t.execution.total_tool_calls as f64).sum::<f64>() 
                / summary.total_tasks as f64
        } else {
            0.0
        };
        
        // 计算平均满意度
        summary.avg_satisfaction = if !task_logs.is_empty() {
            task_logs.iter().map(|t| t.execution.user_satisfaction as f64).sum::<f64>() 
                / summary.total_tasks as f64
        } else {
            0.0
        };
        
        // 计算进化相关指标
        summary.total_evolution_cycles = evolution_logs.len();
        summary.total_gaps_detected = evolution_logs.iter().map(|e| e.gaps_detected.len() as u32).sum();
        summary.total_tools_created = evolution_logs.iter()
            .flat_map(|e| &e.actions_taken)
            .filter(|a| a.action_type == "create_tool")
            .count() as u32;
        
        summary.group = self.experiment_group.clone();
        summary.experiment_id = self.experiment_id.clone();
        
        Ok(summary)
    }

    fn load_task_logs(&self) -> Result<Vec<TaskExecutionLog>> {
        let file_path = self.log_dir.join(format!("task_{}.jsonl", self.experiment_id));
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(file_path)?;
        let logs = content.lines()
            .filter_map(|line| serde_json::from_str::<TaskExecutionLog>(line).ok())
            .collect();
        
        Ok(logs)
    }

    fn load_evolution_logs(&self) -> Result<Vec<SelfEvolutionLog>> {
        let file_path = self.log_dir.join(format!("evolution_{}.jsonl", self.experiment_id));
        
        if !file_path.exists() {
            return Ok(Vec::new());
        }
        
        let content = fs::read_to_string(file_path)?;
        let logs = content.lines()
            .filter_map(|line| serde_json::from_str::<SelfEvolutionLog>(line).ok())
            .collect();
        
        Ok(logs)
    }
}

/// 实验摘要
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperimentSummary {
    /// 实验组
    pub group: String,
    /// 实验 ID
    pub experiment_id: String,
    /// 总任务数
    pub total_tasks: usize,
    /// 成功任务数
    pub successful_tasks: usize,
    /// 任务完成率
    pub task_completion_rate: f64,
    /// 平均工具调用次数
    pub avg_tool_calls: f64,
    /// 平均满意度
    pub avg_satisfaction: f64,
    /// 总进化周期数
    pub total_evolution_cycles: usize,
    /// 总检测缺口数
    pub total_gaps_detected: u32,
    /// 总创建工具数
    pub total_tools_created: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_experiment_logger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let logger = ExperimentLogger::new(temp_dir.path(), "Ours-Full", "test_001");
        
        assert!(logger.is_ok());
    }

    #[test]
    fn test_log_task_execution() {
        let temp_dir = TempDir::new().unwrap();
        let logger = ExperimentLogger::new(temp_dir.path(), "Ours-Full", "test_001").unwrap();
        
        let log = TaskExecutionLog {
            task_id: "file_001".to_string(),
            category: "file_ops".to_string(),
            difficulty: "easy".to_string(),
            description: "读取 README.md".to_string(),
            timestamp: Utc::now(),
            group: "Ours-Full".to_string(),
            execution: ExecutionResult {
                success: true,
                tool_calls: vec![],
                total_tool_calls: 1,
                execution_time_ms: 150.0,
                user_satisfaction: 5,
                error_message: None,
            },
            evolution: EvolutionInfo::default(),
        };
        
        let result = logger.log_task_execution(&log);
        assert!(result.is_ok());
    }

    #[test]
    fn test_export_summary() {
        let temp_dir = TempDir::new().unwrap();
        let logger = ExperimentLogger::new(temp_dir.path(), "Ours-Full", "test_001").unwrap();
        
        let summary = logger.export_summary().unwrap();
        
        assert_eq!(summary.group, "Ours-Full");
        assert_eq!(summary.experiment_id, "test_001");
    }
}
