//! 实验数据收集器
//!
//! # 设计目标
//! 提供严谨、结构化的实验数据收集，支持学术论文的数据可复现性要求
//!
//! # 数据类型
//! 1. **任务执行日志**: 记录每次任务执行的详细信息
//! 2. **工具调用指标**: 记录工具调用的性能数据
//! 3. **缺口检测事件**: 记录缺口检测的过程和结果
//! 4. **API 调用追踪**: 记录所有 LLM API 调用及成本
//! 5. **系统状态快照**: 定期记录系统整体状态

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;
use parking_lot::RwLock;
use tracing::{info, debug};

use crate::autonomy::{
    gap_detector::TaskExecutionRecord,
    hybrid_gap_detector::{HybridToolGap},
};

/// 数据收集器版本（用于数据格式版本控制）
pub const DATA_COLLECTOR_VERSION: &str = "1.0.0";

// ============================================================================
// 数据结构定义
// ============================================================================

/// 任务执行日志（详细版）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedTaskLog {
    /// 任务唯一 ID
    pub task_id: String,
    /// 任务描述
    pub task_description: String,
    /// 任务类型
    pub task_type: TaskType,
    /// 输入参数
    pub input_parameters: HashMap<String, serde_json::Value>,
    /// 执行开始时间
    pub start_time: DateTime<Utc>,
    /// 执行结束时间
    pub end_time: Option<DateTime<Utc>>,
    /// 执行状态
    pub status: TaskStatus,
    /// 使用的工具序列
    pub tool_sequence: Vec<ToolCallLog>,
    /// 最终输出
    pub final_output: Option<serde_json::Value>,
    /// 错误信息（如果失败）
    pub error_message: Option<String>,
    /// 用户反馈
    pub user_feedback: Option<UserFeedback>,
    /// 实验组别
    pub experiment_group: String,
    /// 额外元数据
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    /// 代码生成
    CodeGeneration,
    /// 代码审查
    CodeReview,
    /// 代码重构
    CodeRefactoring,
    /// 调试
    Debugging,
    /// 文档生成
    Documentation,
    /// 研究分析
    Research,
    /// 文件操作
    FileOperation,
    /// 网络请求
    NetworkRequest,
    /// 系统命令
    SystemCommand,
    /// 其他
    Other,
}

impl TaskType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskType::CodeGeneration => "code_generation",
            TaskType::CodeReview => "code_review",
            TaskType::CodeRefactoring => "code_refactoring",
            TaskType::Debugging => "debugging",
            TaskType::Documentation => "documentation",
            TaskType::Research => "research",
            TaskType::FileOperation => "file_operation",
            TaskType::NetworkRequest => "network_request",
            TaskType::SystemCommand => "system_command",
            TaskType::Other => "other",
        }
    }
}

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 超时
    Timeout,
}

/// 工具调用日志
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallLog {
    /// 工具名称
    pub tool_name: String,
    /// 调用时间
    pub call_time: DateTime<Utc>,
    /// 输入参数
    pub input: serde_json::Value,
    /// 输出结果
    pub output: Option<serde_json::Value>,
    /// 执行时间（毫秒）
    pub execution_time_ms: u64,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
}

/// 用户反馈
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    /// 满意度评分（1-5）
    pub satisfaction: u8,
    /// 反馈文本
    pub comment: Option<String>,
    /// 反馈时间
    pub feedback_time: DateTime<Utc>,
}

/// 缺口检测事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapDetectionEvent {
    /// 事件 ID
    pub event_id: String,
    /// 检测时间
    pub detection_time: DateTime<Utc>,
    /// 检测器类型
    pub detector_type: String,
    /// 检测到的缺口
    pub gap: HybridToolGap,
    /// 触发检测的任务 ID 列表
    pub triggered_by_tasks: Vec<String>,
    /// 检测耗时（毫秒）
    pub detection_duration_ms: u64,
    /// 是否进行了因果分析
    pub causal_analysis_performed: bool,
    /// API 调用成本（美元）
    pub api_cost_usd: f64,
}

/// API 调用记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCallLog {
    /// 调用 ID
    pub call_id: String,
    /// 调用时间
    pub call_time: DateTime<Utc>,
    /// API 提供商
    pub provider: String,
    /// 模型名称
    pub model: String,
    /// 用途（缺口分析/代码生成等）
    pub purpose: String,
    /// 输入 token 数
    pub input_tokens: u32,
    /// 输出 token 数
    pub output_tokens: u32,
    /// 成本（美元）
    pub cost_usd: f64,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 是否成功
    pub success: bool,
    /// 错误信息
    pub error: Option<String>,
}

/// 系统状态快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    /// 快照时间
    pub timestamp: DateTime<Utc>,
    /// 工具总数
    pub total_tools: u32,
    /// 活跃工具数
    pub active_tools: u32,
    /// 累计任务数
    pub total_tasks: u32,
    /// 累计缺口数
    pub total_gaps_detected: u32,
    /// 累计创建工具数
    pub total_tools_created: u32,
    /// 累计 API 成本
    pub cumulative_api_cost_usd: f64,
    /// 系统运行时间（秒）
    pub uptime_seconds: u64,
}

// ============================================================================
// 数据收集器
// ============================================================================

/// 实验数据收集器
pub struct DataCollector {
    /// 实验 ID
    experiment_id: String,
    /// 实验组别
    group_id: String,
    /// 数据输出目录
    output_dir: PathBuf,
    /// 任务日志
    task_logs: Arc<RwLock<Vec<DetailedTaskLog>>>,
    /// 工具调用日志
    tool_call_logs: Arc<RwLock<Vec<ToolCallLog>>>,
    /// 缺口检测事件
    gap_events: Arc<RwLock<Vec<GapDetectionEvent>>>,
    /// API 调用日志
    api_logs: Arc<RwLock<Vec<ApiCallLog>>>,
    /// 系统快照
    snapshots: Arc<RwLock<Vec<SystemSnapshot>>>,
    /// 实验开始时间
    start_time: DateTime<Utc>,
    /// 累计 API 成本
    cumulative_api_cost: Arc<RwLock<f64>>,
}

impl DataCollector {
    /// 创建新的数据收集器
    pub fn new(
        experiment_id: &str,
        group_id: &str,
        output_dir: &Path,
    ) -> Result<Self> {
        // 创建输出目录结构
        let exp_dir = output_dir.join(experiment_id);
        let group_dir = exp_dir.join(group_id);
        
        fs::create_dir_all(&group_dir)?;
        fs::create_dir_all(group_dir.join("tasks"))?;
        fs::create_dir_all(group_dir.join("gaps"))?;
        fs::create_dir_all(group_dir.join("api"))?;
        fs::create_dir_all(group_dir.join("snapshots"))?;
        
        Ok(Self {
            experiment_id: experiment_id.to_string(),
            group_id: group_id.to_string(),
            output_dir: group_dir,
            task_logs: Arc::new(RwLock::new(Vec::new())),
            tool_call_logs: Arc::new(RwLock::new(Vec::new())),
            gap_events: Arc::new(RwLock::new(Vec::new())),
            api_logs: Arc::new(RwLock::new(Vec::new())),
            snapshots: Arc::new(RwLock::new(Vec::new())),
            start_time: Utc::now(),
            cumulative_api_cost: Arc::new(RwLock::new(0.0)),
        })
    }
    
    /// 记录任务开始
    pub fn record_task_start(
        &self,
        task_id: &str,
        task_description: &str,
        task_type: TaskType,
        input_parameters: HashMap<String, serde_json::Value>,
    ) -> String {
        let task_id = task_id.to_string();
        let log = DetailedTaskLog {
            task_id: task_id.clone(),
            task_description: task_description.to_string(),
            task_type,
            input_parameters,
            start_time: Utc::now(),
            end_time: None,
            status: TaskStatus::Running,
            tool_sequence: Vec::new(),
            final_output: None,
            error_message: None,
            user_feedback: None,
            experiment_group: self.group_id.clone(),
            metadata: HashMap::new(),
        };
        
        self.task_logs.write().push(log);
        task_id
    }
    
    /// 记录任务完成
    pub fn record_task_completion(
        &self,
        task_id: &str,
        output: Option<serde_json::Value>,
    ) -> Result<()> {
        let mut logs = self.task_logs.write();
        let log = logs.iter_mut()
            .find(|l| l.task_id == task_id)
            .context("Task not found")?;
        
        log.end_time = Some(Utc::now());
        log.status = TaskStatus::Completed;
        log.final_output = output;
        
        Ok(())
    }
    
    /// 记录任务失败
    pub fn record_task_failure(
        &self,
        task_id: &str,
        error_message: &str,
    ) -> Result<()> {
        let mut logs = self.task_logs.write();
        let log = logs.iter_mut()
            .find(|l| l.task_id == task_id)
            .context("Task not found")?;
        
        log.end_time = Some(Utc::now());
        log.status = TaskStatus::Failed;
        log.error_message = Some(error_message.to_string());
        
        Ok(())
    }
    
    /// 记录工具调用
    pub fn record_tool_call(
        &self,
        task_id: &str,
        tool_name: &str,
        input: serde_json::Value,
        output: Option<serde_json::Value>,
        execution_time_ms: u64,
        success: bool,
        error: Option<String>,
    ) -> Result<()> {
        let call_log = ToolCallLog {
            tool_name: tool_name.to_string(),
            call_time: Utc::now(),
            input,
            output,
            execution_time_ms,
            success,
            error,
        };
        
        // 添加到工具调用日志
        self.tool_call_logs.write().push(call_log.clone());
        
        // 添加到任务的工具序列
        let mut logs = self.task_logs.write();
        if let Some(log) = logs.iter_mut().find(|l| l.task_id == task_id) {
            log.tool_sequence.push(call_log);
        }
        
        Ok(())
    }
    
    /// 记录用户反馈
    pub fn record_user_feedback(
        &self,
        task_id: &str,
        satisfaction: u8,
        comment: Option<String>,
    ) -> Result<()> {
        if satisfaction < 1 || satisfaction > 5 {
            bail!("Satisfaction must be between 1 and 5");
        }
        
        let feedback = UserFeedback {
            satisfaction,
            comment,
            feedback_time: Utc::now(),
        };
        
        let mut logs = self.task_logs.write();
        if let Some(log) = logs.iter_mut().find(|l| l.task_id == task_id) {
            log.user_feedback = Some(feedback);
        }
        
        Ok(())
    }
    
    /// 记录缺口检测事件
    pub fn record_gap_detection(
        &self,
        gap: HybridToolGap,
        triggered_by_tasks: Vec<String>,
        detection_duration_ms: u64,
        causal_analysis_performed: bool,
        api_cost_usd: f64,
    ) -> String {
        let event_id = Uuid::new_v4().to_string();
        
        let event = GapDetectionEvent {
            event_id: event_id.clone(),
            detection_time: Utc::now(),
            detector_type: "HybridGapDetector".to_string(),
            gap,
            triggered_by_tasks,
            detection_duration_ms,
            causal_analysis_performed,
            api_cost_usd,
        };
        
        // 更新累计 API 成本
        *self.cumulative_api_cost.write() += api_cost_usd;
        
        self.gap_events.write().push(event);
        event_id
    }
    
    /// 记录 API 调用
    pub fn record_api_call(
        &self,
        provider: &str,
        model: &str,
        purpose: &str,
        input_tokens: u32,
        output_tokens: u32,
        cost_usd: f64,
        response_time_ms: u64,
        success: bool,
        error: Option<String>,
    ) -> String {
        let call_id = Uuid::new_v4().to_string();
        
        let log = ApiCallLog {
            call_id: call_id.clone(),
            call_time: Utc::now(),
            provider: provider.to_string(),
            model: model.to_string(),
            purpose: purpose.to_string(),
            input_tokens,
            output_tokens,
            cost_usd,
            response_time_ms,
            success,
            error,
        };
        
        // 更新累计 API 成本
        *self.cumulative_api_cost.write() += cost_usd;
        
        self.api_logs.write().push(log);
        call_id
    }
    
    /// 记录系统快照
    pub fn record_snapshot(
        &self,
        total_tools: u32,
        active_tools: u32,
        total_tasks: u32,
        total_gaps_detected: u32,
        total_tools_created: u32,
    ) {
        let uptime = Utc::now().signed_duration_since(self.start_time)
            .num_seconds() as u64;
        
        let snapshot = SystemSnapshot {
            timestamp: Utc::now(),
            total_tools,
            active_tools,
            total_tasks,
            total_gaps_detected,
            total_tools_created,
            cumulative_api_cost_usd: *self.cumulative_api_cost.read(),
            uptime_seconds: uptime,
        };
        
        self.snapshots.write().push(snapshot);
    }
    
    /// 保存所有数据到文件
    pub fn save_all_data(&self) -> Result<()> {
        info!("Saving experiment data for experiment {} group {}", 
              self.experiment_id, self.group_id);
        
        // 保存任务日志
        self.save_task_logs()?;
        
        // 保存工具调用日志
        self.save_tool_call_logs()?;
        
        // 保存缺口检测事件
        self.save_gap_events()?;
        
        // 保存 API 日志
        self.save_api_logs()?;
        
        // 保存系统快照
        self.save_snapshots()?;
        
        // 保存汇总统计
        self.save_summary()?;
        
        info!("All experiment data saved successfully");
        Ok(())
    }
    
    fn save_task_logs(&self) -> Result<()> {
        let logs = self.task_logs.read();
        let path = self.output_dir.join("tasks").join("task_logs.json");
        
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*logs)?;
        
        debug!("Saved {} task logs to {:?}", logs.len(), path);
        Ok(())
    }
    
    fn save_tool_call_logs(&self) -> Result<()> {
        let logs = self.tool_call_logs.read();
        let path = self.output_dir.join("tasks").join("tool_call_logs.json");
        
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*logs)?;
        
        debug!("Saved {} tool call logs to {:?}", logs.len(), path);
        Ok(())
    }
    
    fn save_gap_events(&self) -> Result<()> {
        let events = self.gap_events.read();
        let path = self.output_dir.join("gaps").join("gap_events.json");
        
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*events)?;
        
        debug!("Saved {} gap events to {:?}", events.len(), path);
        Ok(())
    }
    
    fn save_api_logs(&self) -> Result<()> {
        let logs = self.api_logs.read();
        let path = self.output_dir.join("api").join("api_logs.json");
        
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*logs)?;
        
        debug!("Saved {} API logs to {:?}", logs.len(), path);
        Ok(())
    }
    
    fn save_snapshots(&self) -> Result<()> {
        let snapshots = self.snapshots.read();
        let path = self.output_dir.join("snapshots").join("snapshots.json");
        
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &*snapshots)?;
        
        debug!("Saved {} snapshots to {:?}", snapshots.len(), path);
        Ok(())
    }
    
    fn save_summary(&self) -> Result<()> {
        let task_logs = self.task_logs.read();
        let gap_events = self.gap_events.read();
        let api_logs = self.api_logs.read();
        
        // 计算汇总统计
        let completed_tasks = task_logs.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let failed_tasks = task_logs.iter().filter(|t| t.status == TaskStatus::Failed).count();
        let total_api_cost = *self.cumulative_api_cost.read();
        
        let summary = serde_json::json!({
            "experiment_id": self.experiment_id,
            "group_id": self.group_id,
            "data_collector_version": DATA_COLLECTOR_VERSION,
            "start_time": self.start_time,
            "end_time": Utc::now(),
            "statistics": {
                "total_tasks": task_logs.len(),
                "completed_tasks": completed_tasks,
                "failed_tasks": failed_tasks,
                "completion_rate": if task_logs.is_empty() { 0.0 } else { completed_tasks as f64 / task_logs.len() as f64 },
                "total_gap_events": gap_events.len(),
                "total_api_calls": api_logs.len(),
                "total_api_cost_usd": total_api_cost,
                "causal_analyses_count": gap_events.iter().filter(|e| e.causal_analysis_performed).count(),
            }
        });
        
        let path = self.output_dir.join("summary.json");
        let file = File::create(&path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &summary)?;
        
        debug!("Saved summary to {:?}", path);
        Ok(())
    }
    
    /// 获取实验统计
    pub fn get_statistics(&self) -> ExperimentStatistics {
        let task_logs = self.task_logs.read();
        let tool_logs = self.tool_call_logs.read();
        let gap_events = self.gap_events.read();
        let api_logs = self.api_logs.read();
        
        let completed = task_logs.iter().filter(|t| t.status == TaskStatus::Completed).count();
        let failed = task_logs.iter().filter(|t| t.status == TaskStatus::Failed).count();
        
        let tool_successes = tool_logs.iter().filter(|t| t.success).count();
        let tool_failures = tool_logs.iter().filter(|t| !t.success).count();
        
        let avg_satisfaction: f64 = task_logs.iter()
            .filter_map(|t| t.user_feedback.as_ref().map(|f| f.satisfaction as f64))
            .sum::<f64>() / task_logs.iter().filter(|t| t.user_feedback.is_some()).count() as f64;
        
        ExperimentStatistics {
            total_tasks: task_logs.len(),
            completed_tasks: completed,
            failed_tasks: failed,
            completion_rate: if task_logs.is_empty() { 0.0 } else { completed as f64 / task_logs.len() as f64 },
            total_tool_calls: tool_logs.len(),
            tool_success_rate: if tool_logs.is_empty() { 0.0 } else { tool_successes as f64 / tool_logs.len() as f64 },
            avg_satisfaction: if task_logs.is_empty() { 0.0 } else { avg_satisfaction },
            total_gaps_detected: gap_events.len(),
            total_api_calls: api_logs.len(),
            total_api_cost_usd: *self.cumulative_api_cost.read(),
        }
    }
}

/// 实验统计
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExperimentStatistics {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub failed_tasks: usize,
    pub completion_rate: f64,
    pub total_tool_calls: usize,
    pub tool_success_rate: f64,
    pub avg_satisfaction: f64,
    pub total_gaps_detected: usize,
    pub total_api_calls: usize,
    pub total_api_cost_usd: f64,
}
