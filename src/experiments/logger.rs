//! 实验日志系统
//!
//! 用于记录自进化实验的执行日志和指标
//!
//! # 使用示例
//!
//! ```rust,ignore
//! let logger = ExperimentLogger::new("experiments/logs/ours_full")?;
//!
//! // 记录任务执行
//! logger.log_task_execution(TaskExecutionLog {
//!     task_id: "task_001".to_string(),
//!     success: true,
//!     tool_calls: vec![...],
//!     ..Default::default()
//! })?;
//!
//! // 记录自进化周期
//! logger.log_evolution_cycle(EvolutionCycleLog {
//!     cycle_id: "cycle_001".to_string(),
//!     gaps_detected: 5,
//!     tools_created: 2,
//!     ..Default::default()
//! })?;
//!
//! // 生成报告
//! let report = logger.generate_report()?;
//! ```

use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

/// 实验日志记录器
pub struct ExperimentLogger {
    /// 日志目录
    log_dir: PathBuf,
    /// 任务执行日志文件
    task_log_file: PathBuf,
    /// 进化周期日志文件
    evolution_log_file: PathBuf,
    /// 指标日志文件
    metrics_log_file: PathBuf,
    /// 日志文件句柄
    task_writer: Option<BufWriter<File>>,
    evolution_writer: Option<BufWriter<File>>,
    metrics_writer: Option<BufWriter<File>>,
}

/// 任务执行日志
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
    /// 实验组名
    pub group: String,
    /// 执行详情
    pub execution: ExecutionDetails,
    /// 进化详情
    #[serde(default)]
    pub evolution: EvolutionDetails,
}

/// 执行详情
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionDetails {
    /// 是否成功
    pub success: bool,
    /// 工具调用列表
    pub tool_calls: Vec<ToolCallLog>,
    /// 总工具调用次数
    pub total_tool_calls: u32,
    /// 执行时间 (ms)
    pub execution_time_ms: u64,
    /// 用户满意度 (1-5)
    pub user_satisfaction: Option<u8>,
    /// 失败原因（如果失败）
    pub failure_reason: Option<String>,
}

/// 工具调用日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallLog {
    /// 工具名称
    pub tool: String,
    /// 工具参数
    pub args: serde_json::Value,
    /// 执行结果
    pub result: String,
    /// 执行时间 (ms)
    pub execution_time_ms: Option<u64>,
    /// 错误信息（如果失败）
    pub error: Option<String>,
}

/// 进化详情
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvolutionDetails {
    /// 检测到的缺口数量
    pub gaps_detected: u32,
    /// 创建的工具数量
    pub tools_created: u32,
    /// 优化的工具数量
    pub tools_optimized: u32,
    /// 废弃的工具数量
    pub tools_deprecated: u32,
}

/// 自进化周期日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionCycleLog {
    /// 周期 ID
    pub cycle_id: String,
    /// 时间戳
    pub timestamp: DateTime<Utc>,
    /// 实验组名
    pub group: String,
    /// 系统反思
    pub reflection: SystemReflection,
    /// 检测到的缺口
    pub gaps_detected: Vec<GapLog>,
    /// 采取的行动
    pub actions_taken: Vec<ActionLog>,
    /// 指标
    pub metrics: CycleMetrics,
}

/// 系统反思
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReflection {
    /// 覆盖率评分 (0.0-1.0)
    pub coverage_score: f32,
    /// 系统性问题
    pub systemic_issues: Vec<String>,
    /// 战略建议
    pub strategic_recommendations: Vec<String>,
}

/// 缺口日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapLog {
    /// 缺口类型
    pub gap_type: String,
    /// 缺口描述
    pub description: String,
    /// 建议的工具名称
    pub suggested_name: Option<String>,
    /// 优先级 (1-10)
    pub priority: u8,
}

/// 行动日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLog {
    /// 行动类型
    pub action_type: String,
    /// 工具名称
    pub tool_name: String,
    /// 执行结果
    pub result: String,
    /// 编译尝试次数
    pub compilation_attempts: Option<u32>,
}

/// 周期指标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleMetrics {
    /// API 调用次数
    pub api_calls: u32,
    /// API 成本 (美元)
    pub api_cost_usd: f32,
    /// 周期耗时 (ms)
    pub cycle_duration_ms: u64,
}

impl ExperimentLogger {
    /// 创建新的实验日志记录器
    pub fn new(log_dir: &str) -> Result<Self> {
        let log_dir = PathBuf::from(log_dir);
        
        // 创建日志目录
        fs::create_dir_all(&log_dir)
            .with_context(|| format!("Failed to create log directory: {:?}", log_dir))?;
        
        let task_log_file = log_dir.join("task_executions.jsonl");
        let evolution_log_file = log_dir.join("evolution_cycles.jsonl");
        let metrics_log_file = log_dir.join("metrics.jsonl");
        
        // 打开日志文件（追加模式）
        let task_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&task_log_file)
            .with_context(|| format!("Failed to open task log file: {:?}", task_log_file))?;
        
        let evolution_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&evolution_log_file)
            .with_context(|| format!("Failed to open evolution log file: {:?}", evolution_log_file))?;
        
        let metrics_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&metrics_log_file)
            .with_context(|| format!("Failed to open metrics log file: {:?}", metrics_log_file))?;
        
        Ok(Self {
            log_dir,
            task_log_file,
            evolution_log_file,
            metrics_log_file,
            task_writer: Some(BufWriter::new(task_file)),
            evolution_writer: Some(BufWriter::new(evolution_file)),
            metrics_writer: Some(BufWriter::new(metrics_file)),
        })
    }
    
    /// 记录任务执行日志
    pub fn log_task_execution(&self, log: TaskExecutionLog) -> Result<()> {
        let writer = self.task_writer.as_ref().unwrap();
        let mut writer = writer.try_clone().unwrap();
        
        let json = serde_json::to_string(&log)
            .with_context(|| "Failed to serialize task execution log")?;
        
        writeln!(writer, "{}", json)
            .with_context(|| "Failed to write task execution log")?;
        
        Ok(())
    }
    
    /// 记录进化周期日志
    pub fn log_evolution_cycle(&self, log: EvolutionCycleLog) -> Result<()> {
        let writer = self.evolution_writer.as_ref().unwrap();
        let mut writer = writer.try_clone().unwrap();
        
        let json = serde_json::to_string(&log)
            .with_context(|| "Failed to serialize evolution cycle log")?;
        
        writeln!(writer, "{}", json)
            .with_context(|| "Failed to write evolution cycle log")?;
        
        Ok(())
    }
    
    /// 记录指标日志
    pub fn log_metrics(&self, metrics: serde_json::Value) -> Result<()> {
        let writer = self.metrics_writer.as_ref().unwrap();
        let mut writer = writer.try_clone().unwrap();
        
        let json = serde_json::to_string(&metrics)
            .with_context(|| "Failed to serialize metrics log")?;
        
        writeln!(writer, "{}", json)
            .with_context(|| "Failed to write metrics log")?;
        
        Ok(())
    }
    
    /// 刷新所有缓冲区
    pub fn flush(&self) -> Result<()> {
        if let Some(writer) = &self.task_writer {
            writer.get_mut().flush()
                .with_context(|| "Failed to flush task log")?;
        }
        
        if let Some(writer) = &self.evolution_writer {
            writer.get_mut().flush()
                .with_context(|| "Failed to flush evolution log")?;
        }
        
        if let Some(writer) = &self.metrics_writer {
            writer.get_mut().flush()
                .with_context(|| "Failed to flush metrics log")?;
        }
        
        Ok(())
    }
    
    /// 读取任务执行日志
    pub fn read_task_executions(&self) -> Result<Vec<TaskExecutionLog>> {
        let file = File::open(&self.task_log_file)
            .with_context(|| format!("Failed to open task log file: {:?}", self.task_log_file))?;
        
        let reader = std::io::BufReader::new(file);
        let mut logs = Vec::new();
        
        for line in std::io::BufRead::lines(reader) {
            let line = line.with_context(|| "Failed to read line")?;
            if line.trim().is_empty() {
                continue;
            }
            
            let log: TaskExecutionLog = serde_json::from_str(&line)
                .with_context(|| "Failed to parse task execution log")?;
            logs.push(log);
        }
        
        Ok(logs)
    }
    
    /// 读取进化周期日志
    pub fn read_evolution_cycles(&self) -> Result<Vec<EvolutionCycleLog>> {
        let file = File::open(&self.evolution_log_file)
            .with_context(|| format!("Failed to open evolution log file: {:?}", self.evolution_log_file))?;
        
        let reader = std::io::BufReader::new(file);
        let mut logs = Vec::new();
        
        for line in std::io::BufRead::lines(reader) {
            let line = line.with_context(|| "Failed to read line")?;
            if line.trim().is_empty() {
                continue;
            }
            
            let log: EvolutionCycleLog = serde_json::from_str(&line)
                .with_context(|| "Failed to parse evolution cycle log")?;
            logs.push(log);
        }
        
        Ok(logs)
    }
    
    /// 生成实验报告
    pub fn generate_report(&self) -> Result<ExperimentReport> {
        let task_executions = self.read_task_executions()?;
        let evolution_cycles = self.read_evolution_cycles()?;
        
        // 计算统计信息
        let total_tasks = task_executions.len();
        let successful_tasks = task_executions.iter().filter(|t| t.execution.success).count();
        let success_rate = if total_tasks > 0 {
            successful_tasks as f32 / total_tasks as f32
        } else {
            0.0
        };
        
        let avg_tool_calls = if total_tasks > 0 {
            task_executions.iter().map(|t| t.execution.total_tool_calls as f32).sum::<f32>() / total_tasks as f32
        } else {
            0.0
        };
        
        let avg_satisfaction = task_executions.iter()
            .filter_map(|t| t.execution.user_satisfaction)
            .map(|s| s as f32)
            .collect::<Vec<_>>();
        
        let avg_satisfaction = if avg_satisfaction.is_empty() {
            0.0
        } else {
            avg_satisfaction.iter().sum::<f32>() / avg_satisfaction.len() as f32
        };
        
        let total_gaps_detected: u32 = evolution_cycles.iter().map(|c| c.gaps_detected.len() as u32).sum();
        let total_tools_created: u32 = evolution_cycles.iter()
            .flat_map(|c| c.actions_taken.iter().filter(|a| a.action_type == "create_tool"))
            .count() as u32;
        
        let total_api_cost: f32 = evolution_cycles.iter().map(|c| c.metrics.api_cost_usd).sum();
        
        Ok(ExperimentReport {
            total_tasks,
            successful_tasks,
            success_rate,
            avg_tool_calls,
            avg_satisfaction,
            total_gaps_detected,
            total_tools_created,
            total_api_cost,
            total_cycles: evolution_cycles.len(),
        })
    }
}

/// 实验报告
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentReport {
    /// 总任务数
    pub total_tasks: usize,
    /// 成功任务数
    pub successful_tasks: usize,
    /// 成功率
    pub success_rate: f32,
    /// 平均工具调用次数
    pub avg_tool_calls: f32,
    /// 平均满意度
    pub avg_satisfaction: f32,
    /// 总缺口检测数
    pub total_gaps_detected: u32,
    /// 总工具创建数
    pub total_tools_created: u32,
    /// 总 API 成本
    pub total_api_cost: f32,
    /// 总周期数
    pub total_cycles: usize,
}

impl Drop for ExperimentLogger {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_create_logger() {
        let dir = tempdir().unwrap();
        let log_dir = dir.path().join("test_logs");
        
        let logger = ExperimentLogger::new(log_dir.to_str().unwrap()).unwrap();
        
        assert!(logger.task_log_file.exists());
        assert!(logger.evolution_log_file.exists());
        assert!(logger.metrics_log_file.exists());
    }
    
    #[test]
    fn test_log_task_execution() {
        let dir = tempdir().unwrap();
        let log_dir = dir.path().join("test_logs");
        
        let logger = ExperimentLogger::new(log_dir.to_str().unwrap()).unwrap();
        
        let log = TaskExecutionLog {
            task_id: "test_001".to_string(),
            category: "test".to_string(),
            difficulty: "easy".to_string(),
            description: "Test task".to_string(),
            timestamp: Utc::now(),
            group: "Ours-Full".to_string(),
            execution: ExecutionDetails {
                success: true,
                tool_calls: vec![],
                total_tool_calls: 1,
                execution_time_ms: 100,
                user_satisfaction: Some(5),
                failure_reason: None,
            },
            evolution: EvolutionDetails::default(),
        };
        
        logger.log_task_execution(log).unwrap();
        logger.flush().unwrap();
        
        let executions = logger.read_task_executions().unwrap();
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].task_id, "test_001");
    }
}
