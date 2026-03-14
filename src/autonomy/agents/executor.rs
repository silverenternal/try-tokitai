//! 执行 Agent - 按计划执行任务
//!
//! # 职责
//! - 按计划步骤执行具体任务
//! - 调用工具完成工作
//! - 记录执行结果
//! - 报告进度和异常

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// 执行错误类型
#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("执行失败：{0}")]
    ExecutionFailed(String),
    #[error("文件操作失败：{0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON 处理失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// 执行状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Paused,
}

/// 步骤执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepExecutionResult {
    /// 步骤 ID
    pub step_id: String,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 执行结果输出
    pub output: Option<String>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行时间戳
    pub executed_at: i64,
    /// 实际耗时（秒）
    pub duration_secs: Option<u64>,
}

/// 执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// 执行 ID
    pub id: String,
    /// 关联的计划 ID
    pub plan_id: String,
    /// 创建时间戳
    pub created_at: i64,
    /// 结束时间戳
    pub ended_at: Option<i64>,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 步骤执行结果
    pub step_results: Vec<StepExecutionResult>,
    /// 执行总结
    pub summary: Option<String>,
}

impl ExecutionRecord {
    /// 创建新的执行记录
    pub fn new(plan_id: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            plan_id,
            created_at: now,
            ended_at: None,
            status: ExecutionStatus::Pending,
            step_results: vec![],
            summary: None,
        }
    }

    /// 添加步骤结果
    pub fn add_step_result(&mut self, result: StepExecutionResult) {
        self.step_results.push(result);
    }

    /// 获取完成的步骤数
    pub fn completed_steps(&self) -> usize {
        self.step_results.iter().filter(|r| r.status == ExecutionStatus::Completed).count()
    }

    /// 获取失败的步骤数
    pub fn failed_steps(&self) -> usize {
        self.step_results.iter().filter(|r| r.status == ExecutionStatus::Failed).count()
    }

    /// 获取进度百分比
    pub fn progress_percentage(&self) -> f64 {
        if self.step_results.is_empty() {
            return 0.0;
        }
        (self.completed_steps() as f64 / self.step_results.len() as f64) * 100.0
    }
}

/// 执行 Agent
pub struct ExecutorAgent {
    /// 存储目录
    storage_dir: PathBuf,
    /// 执行记录
    records: Vec<ExecutionRecord>,
}

impl ExecutorAgent {
    /// 创建新的执行 Agent
    pub fn new(storage_dir: PathBuf) -> Result<Self, ExecutorError> {
        fs::create_dir_all(&storage_dir)?;
        
        let mut agent = Self {
            storage_dir,
            records: vec![],
        };

        agent.load_records()?;

        Ok(agent)
    }

    /// 开始执行计划
    pub fn start_execution(&mut self, plan_id: String) -> &ExecutionRecord {
        let record = ExecutionRecord::new(plan_id);
        self.records.push(record);
        self.records.last_mut().unwrap()
    }

    /// 记录步骤开始
    pub fn record_step_start(&mut self, record_id: &str, step_id: String) -> Result<(), ExecutorError> {
        if let Some(record) = self.records.iter_mut().find(|r| r.id == record_id) {
            record.status = ExecutionStatus::Running;
            record.add_step_result(StepExecutionResult {
                step_id,
                status: ExecutionStatus::Running,
                output: None,
                error: None,
                executed_at: chrono::Utc::now().timestamp(),
                duration_secs: None,
            });
            self.save_records()?;
        }
        Ok(())
    }

    /// 记录步骤完成
    pub fn record_step_complete(&mut self, record_id: &str, step_id: String, output: String, duration_secs: u64) -> Result<(), ExecutorError> {
        if let Some(record) = self.records.iter_mut().find(|r| r.id == record_id) {
            if let Some(result) = record.step_results.iter_mut().find(|r| r.step_id == step_id) {
                result.status = ExecutionStatus::Completed;
                result.output = Some(output);
                result.duration_secs = Some(duration_secs);
            }
            self.save_records()?;
        }
        Ok(())
    }

    /// 记录步骤失败
    pub fn record_step_failed(&mut self, record_id: &str, step_id: String, error: String) -> Result<(), ExecutorError> {
        if let Some(record) = self.records.iter_mut().find(|r| r.id == record_id) {
            if let Some(result) = record.step_results.iter_mut().find(|r| r.step_id == step_id) {
                result.status = ExecutionStatus::Failed;
                result.error = Some(error);
            }
            self.save_records()?;
        }
        Ok(())
    }

    /// 完成执行
    pub fn complete_execution(&mut self, record_id: &str, summary: String) -> Result<(), ExecutorError> {
        if let Some(record) = self.records.iter_mut().find(|r| r.id == record_id) {
            record.status = ExecutionStatus::Completed;
            record.ended_at = Some(chrono::Utc::now().timestamp());
            record.summary = Some(summary);
            self.save_records()?;
        }
        Ok(())
    }

    /// 失败执行
    pub fn fail_execution(&mut self, record_id: &str, reason: String) -> Result<(), ExecutorError> {
        if let Some(record) = self.records.iter_mut().find(|r| r.id == record_id) {
            record.status = ExecutionStatus::Failed;
            record.ended_at = Some(chrono::Utc::now().timestamp());
            record.summary = Some(format!("失败：{}", reason));
            self.save_records()?;
        }
        Ok(())
    }

    /// 获取最近的执行记录
    pub fn last_record(&self) -> Option<&ExecutionRecord> {
        self.records.last()
    }

    /// 获取执行记录
    pub fn get_record(&self, record_id: &str) -> Option<&ExecutionRecord> {
        self.records.iter().find(|r| r.id == record_id)
    }

    /// 保存记录
    fn save_records(&self) -> Result<(), ExecutorError> {
        let records_path = self.storage_dir.join("executions.json");
        let content = serde_json::to_string_pretty(&self.records)?;
        fs::write(&records_path, content)?;
        Ok(())
    }

    /// 加载记录
    fn load_records(&mut self) -> Result<(), ExecutorError> {
        let records_path = self.storage_dir.join("executions.json");
        if records_path.exists() {
            let content = fs::read_to_string(&records_path)?;
            self.records = serde_json::from_str(&content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_executor_agent() {
        let temp_dir = TempDir::new().unwrap();
        let mut executor = ExecutorAgent::new(temp_dir.path().to_path_buf()).unwrap();

        let record = executor.start_execution("plan_123".to_string());
        assert_eq!(record.plan_id, "plan_123");
        assert_eq!(record.status, ExecutionStatus::Pending);
    }

    #[test]
    fn test_execution_record() {
        let mut record = ExecutionRecord::new("plan_456".to_string());
        
        record.add_step_result(StepExecutionResult {
            step_id: "step_1".to_string(),
            status: ExecutionStatus::Completed,
            output: Some("成功".to_string()),
            error: None,
            executed_at: chrono::Utc::now().timestamp(),
            duration_secs: Some(10),
        });

        record.add_step_result(StepExecutionResult {
            step_id: "step_2".to_string(),
            status: ExecutionStatus::Pending,
            output: None,
            error: None,
            executed_at: chrono::Utc::now().timestamp(),
            duration_secs: None,
        });

        assert_eq!(record.completed_steps(), 1);
        assert_eq!(record.failed_steps(), 0);
        assert!((record.progress_percentage() - 50.0).abs() < 0.1);
    }
}
