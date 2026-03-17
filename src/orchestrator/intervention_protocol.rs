//! 用户干预协议
//!
//! 定义用户在自主迭代中介入的标准协议，支持检查点审批
//!
//! ## 检查点类型
//! - `PlanReady` - 规划完成待审批
//! - `ReviewComplete` - 审查完成待确认
//! - `IterationDone` - 迭代完成待验收
//! - `ErrorRecovery` - 错误恢复待决策
//!
//! ## 用户操作
//! - `approve` - 批准继续
//! - `modify` - 修改后继续
//! - `reject` - 驳回重来
//! - `abort` - 终止迭代

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;
use anyhow::Result;

/// 检查点类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckpointType {
    /// 规划完成待审批
    PlanReady,
    /// 审查完成待确认
    ReviewComplete,
    /// 迭代完成待验收
    IterationDone,
    /// 错误恢复待决策
    ErrorRecovery,
    /// 工具创建待确认
    ToolCreation,
    /// 重大修改待审批
    MajorChange,
}

/// 用户操作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UserAction {
    /// 批准继续
    Approve,
    /// 修改后继续
    Modify(String),
    /// 驳回重来
    Reject(String),
    /// 终止迭代
    Abort(String),
    /// 暂停（稍后决定）
    Pause,
}

/// 检查点状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CheckpointStatus {
    /// 等待用户响应
    Pending,
    /// 用户已批准
    Approved,
    /// 用户已修改
    Modified,
    /// 用户已驳回
    Rejected,
    /// 用户已终止
    Aborted,
    /// 已暂停
    Paused,
    /// 超时
    TimedOut,
}

/// 检查点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    /// 唯一 ID
    pub id: String,
    /// 检查点类型
    pub checkpoint_type: CheckpointType,
    /// 检查点描述
    pub description: String,
    /// 创建时间戳
    pub created_at: u64,
    /// 响应截止时间戳（0 表示无限制）
    pub deadline_at: u64,
    /// 状态
    pub status: CheckpointStatus,
    /// 关联的迭代 ID
    pub iteration_id: Option<String>,
    /// 相关数据（JSON）
    pub payload: Option<String>,
    /// 用户响应
    pub user_response: Option<UserResponse>,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u64>,
}

/// 用户响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    /// 用户操作
    pub action: UserAction,
    /// 响应时间戳
    pub responded_at: u64,
    /// 用户备注
    pub comment: Option<String>,
}

/// 干预请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionRequest {
    /// 检查点类型
    pub checkpoint_type: CheckpointType,
    /// 描述
    pub description: String,
    /// 迭代 ID
    pub iteration_id: Option<String>,
    /// 相关数据
    pub payload: Option<serde_json::Value>,
    /// 超时时间（秒）
    pub timeout_seconds: Option<u64>,
}

/// 干预结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionResult {
    /// 检查点 ID
    pub checkpoint_id: String,
    /// 用户操作
    pub user_action: UserAction,
    /// 响应时间（ms）
    pub response_time_ms: u64,
    /// 是否超时
    pub is_timeout: bool,
}

/// 用户干预协议
pub struct InterventionProtocol {
    /// 数据目录
    data_dir: PathBuf,
    /// 活跃检查点
    checkpoints: Vec<Checkpoint>,
    /// 历史检查点
    history: Vec<Checkpoint>,
    /// 配置
    config: ProtocolConfig,
    /// 回调函数（用于通知用户）
    notification_callback: Option<Arc<dyn Fn(&Checkpoint) + Send + Sync>>,
}

/// 协议配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    /// 默认超时时间（秒）
    pub default_timeout_seconds: u64,
    /// 是否启用超时
    pub timeout_enabled: bool,
    /// 是否自动清理历史
    pub auto_cleanup_history: bool,
    /// 历史保留数量
    pub history_retention_count: usize,
    /// 是否需要确认所有检查点
    pub require_all_checkpoints: bool,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            default_timeout_seconds: 300, // 5 分钟
            timeout_enabled: true,
            auto_cleanup_history: true,
            history_retention_count: 100,
            require_all_checkpoints: false,
        }
    }
}

impl InterventionProtocol {
    /// 创建新的干预协议
    pub fn new<P: AsRef<Path>>(data_dir: P) -> Result<Self> {
        let data_dir = data_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&data_dir)?;
        
        let mut protocol = Self {
            data_dir,
            checkpoints: Vec::new(),
            history: Vec::new(),
            config: ProtocolConfig::default(),
            notification_callback: None,
        };
        
        // 加载已有数据
        protocol.load_state().ok();
        
        Ok(protocol)
    }

    /// 从配置创建
    pub fn with_config<P: AsRef<Path>>(data_dir: P, config: ProtocolConfig) -> Result<Self> {
        let mut protocol = Self::new(data_dir)?;
        protocol.config = config;
        Ok(protocol)
    }

    /// 创建检查点
    pub fn create_checkpoint(&mut self, request: InterventionRequest) -> Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let timeout = request.timeout_seconds.unwrap_or(self.config.default_timeout_seconds);
        let deadline = if self.config.timeout_enabled && timeout > 0 {
            now + timeout
        } else {
            0
        };
        
        // 生成 ID
        let id = format!("ckpt_{}_{}", request.checkpoint_type.as_str(), now);
        
        let checkpoint = Checkpoint {
            id: id.clone(),
            checkpoint_type: request.checkpoint_type,
            description: request.description,
            created_at: now,
            deadline_at: deadline,
            status: CheckpointStatus::Pending,
            iteration_id: request.iteration_id,
            payload: request.payload.map(|v| serde_json::to_string(&v).unwrap()),
            user_response: None,
            timeout_seconds: request.timeout_seconds,
        };
        
        // 通知用户
        if let Some(ref callback) = self.notification_callback {
            callback(&checkpoint);
        }
        
        self.checkpoints.push(checkpoint);
        self.save_state()?;
        
        Ok(id)
    }

    /// 等待用户响应
    pub fn wait_for_response(&self, checkpoint_id: &str, timeout_ms: u64) -> Result<InterventionResult> {
        let start_time = std::time::Instant::now();
        
        // 查找检查点
        let checkpoint = self.checkpoints.iter()
            .find(|c| c.id == checkpoint_id)
            .ok_or_else(|| anyhow::anyhow!("Checkpoint not found: {}", checkpoint_id))?;
        
        // 检查是否已有响应
        if let Some(ref response) = checkpoint.user_response {
            return Ok(InterventionResult {
                checkpoint_id: checkpoint_id.to_string(),
                user_action: response.action.clone(),
                response_time_ms: start_time.elapsed().as_millis() as u64,
                is_timeout: false,
            });
        }
        
        // 检查是否超时
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        if checkpoint.deadline_at > 0 && now > checkpoint.deadline_at {
            return Ok(InterventionResult {
                checkpoint_id: checkpoint_id.to_string(),
                user_action: UserAction::Pause, // 超时视为暂停
                response_time_ms: start_time.elapsed().as_millis() as u64,
                is_timeout: true,
            });
        }
        
        // 轮询等待（实际使用中应该用异步或事件通知）
        let poll_interval = std::time::Duration::from_millis(100);
        let timeout_duration = std::time::Duration::from_millis(timeout_ms);
        
        while start_time.elapsed() < timeout_duration {
            // 重新加载状态
            let mut found_checkpoint = None;
            for cp in &self.checkpoints {
                if cp.id == checkpoint_id {
                    found_checkpoint = Some(cp.clone());
                    break;
                }
            }
            
            if let Some(cp) = found_checkpoint {
                if let Some(ref response) = cp.user_response {
                    return Ok(InterventionResult {
                        checkpoint_id: checkpoint_id.to_string(),
                        user_action: response.action.clone(),
                        response_time_ms: start_time.elapsed().as_millis() as u64,
                        is_timeout: false,
                    });
                }
            }
            
            std::thread::sleep(poll_interval);
        }
        
        // 超时
        Ok(InterventionResult {
            checkpoint_id: checkpoint_id.to_string(),
            user_action: UserAction::Pause,
            response_time_ms: timeout_ms,
            is_timeout: true,
        })
    }

    /// 响应用户操作
    pub fn respond(&mut self, checkpoint_id: &str, action: UserAction, comment: Option<String>) -> Result<()> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 查找并更新检查点
        for checkpoint in &mut self.checkpoints {
            if checkpoint.id == checkpoint_id {
                let status = match &action {
                    UserAction::Approve => CheckpointStatus::Approved,
                    UserAction::Modify(_) => CheckpointStatus::Modified,
                    UserAction::Reject(_) => CheckpointStatus::Rejected,
                    UserAction::Abort(_) => CheckpointStatus::Aborted,
                    UserAction::Pause => CheckpointStatus::Paused,
                };
                
                checkpoint.status = status;
                checkpoint.user_response = Some(UserResponse {
                    action,
                    responded_at: now,
                    comment,
                });
                
                // 移动到历史
                let completed = self.checkpoints.iter()
                    .position(|c| c.id == checkpoint_id)
                    .unwrap();
                let completed_checkpoint = self.checkpoints.remove(completed);
                self.history.push(completed_checkpoint);
                
                // 清理历史
                if self.config.auto_cleanup_history 
                    && self.history.len() > self.config.history_retention_count 
                {
                    self.history.remove(0);
                }
                
                self.save_state()?;
                return Ok(());
            }
        }
        
        anyhow::bail!("Checkpoint not found: {}", checkpoint_id)
    }

    /// 获取待处理的检查点
    pub fn get_pending_checkpoints(&self) -> Vec<&Checkpoint> {
        self.checkpoints.iter()
            .filter(|cp| cp.status == CheckpointStatus::Pending)
            .collect()
    }

    /// 获取特定类型的检查点
    pub fn get_checkpoints_by_type(&self, checkpoint_type: CheckpointType) -> Vec<&Checkpoint> {
        self.checkpoints.iter()
            .filter(|cp| cp.checkpoint_type == checkpoint_type)
            .collect()
    }

    /// 获取检查点
    pub fn get_checkpoint(&self, checkpoint_id: &str) -> Option<&Checkpoint> {
        self.checkpoints.iter().find(|cp| cp.id == checkpoint_id)
    }

    /// 设置通知回调
    pub fn set_notification_callback<F>(&mut self, callback: F)
    where
        F: Fn(&Checkpoint) + Send + Sync + 'static,
    {
        self.notification_callback = Some(Arc::new(callback));
    }

    /// 检查是否需要用户干预
    pub fn needs_intervention(&self) -> bool {
        !self.checkpoints.is_empty()
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> InterventionStats {
        let pending_count = self.checkpoints.iter()
            .filter(|cp| cp.status == CheckpointStatus::Pending)
            .count();
        
        let approved_count = self.history.iter()
            .filter(|cp| cp.status == CheckpointStatus::Approved)
            .count();
        
        let rejected_count = self.history.iter()
            .filter(|cp| cp.status == CheckpointStatus::Rejected)
            .count();
        
        let aborted_count = self.history.iter()
            .filter(|cp| cp.status == CheckpointStatus::Aborted)
            .count();
        
        InterventionStats {
            pending_count,
            total_checkpoints: self.checkpoints.len() + self.history.len(),
            approved_count,
            rejected_count,
            aborted_count,
            paused_count: self.history.iter()
                .filter(|cp| cp.status == CheckpointStatus::Paused)
                .count(),
        }
    }

    /// 保存状态
    fn save_state(&self) -> Result<()> {
        let state = ProtocolState {
            checkpoints: self.checkpoints.clone(),
            history: self.history.clone(),
            last_updated: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        let file_path = self.data_dir.join("intervention_state.json");
        let json = serde_json::to_string_pretty(&state)?;
        std::fs::write(file_path, json)?;
        Ok(())
    }

    /// 加载状态
    fn load_state(&mut self) -> Result<()> {
        let file_path = self.data_dir.join("intervention_state.json");
        if file_path.exists() {
            let json = std::fs::read_to_string(file_path)?;
            let state: ProtocolState = serde_json::from_str(&json)?;
            self.checkpoints = state.checkpoints;
            self.history = state.history;
        }
        Ok(())
    }

    /// 清空所有检查点
    pub fn clear(&mut self) -> Result<()> {
        self.checkpoints.clear();
        self.history.clear();
        self.save_state()?;
        Ok(())
    }
}

/// 协议状态
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProtocolState {
    checkpoints: Vec<Checkpoint>,
    history: Vec<Checkpoint>,
    last_updated: u64,
}

/// 干预统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterventionStats {
    /// 待处理数量
    pub pending_count: usize,
    /// 总检查点数
    pub total_checkpoints: usize,
    /// 批准数量
    pub approved_count: usize,
    /// 驳回数量
    pub rejected_count: usize,
    /// 终止数量
    pub aborted_count: usize,
    /// 暂停数量
    pub paused_count: usize,
}

impl CheckpointType {
    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckpointType::PlanReady => "PlanReady",
            CheckpointType::ReviewComplete => "ReviewComplete",
            CheckpointType::IterationDone => "IterationDone",
            CheckpointType::ErrorRecovery => "ErrorRecovery",
            CheckpointType::ToolCreation => "ToolCreation",
            CheckpointType::MajorChange => "MajorChange",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_protocol_creation() {
        let temp_dir = TempDir::new().unwrap();
        let protocol = InterventionProtocol::new(temp_dir.path()).unwrap();
        assert!(!protocol.needs_intervention());
    }

    #[test]
    fn test_create_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let mut protocol = InterventionProtocol::new(temp_dir.path()).unwrap();
        
        let request = InterventionRequest {
            checkpoint_type: CheckpointType::PlanReady,
            description: "Plan is ready for review".to_string(),
            iteration_id: Some("iter_1".to_string()),
            payload: Some(serde_json::json!({"plan": "test"})),
            timeout_seconds: Some(60),
        };
        
        let checkpoint_id = protocol.create_checkpoint(request).unwrap();
        assert!(checkpoint_id.starts_with("ckpt_"));
        assert!(protocol.needs_intervention());
    }

    #[test]
    fn test_respond_to_checkpoint() {
        let temp_dir = TempDir::new().unwrap();
        let mut protocol = InterventionProtocol::new(temp_dir.path()).unwrap();
        
        let request = InterventionRequest {
            checkpoint_type: CheckpointType::PlanReady,
            description: "Plan is ready".to_string(),
            iteration_id: None,
            payload: None,
            timeout_seconds: None,
        };
        
        let checkpoint_id = protocol.create_checkpoint(request).unwrap();

        // 响应用户操作
        protocol.respond(&checkpoint_id, UserAction::Approve, Some("Looks good!".to_string())).unwrap();
        
        let stats = protocol.get_stats();
        assert_eq!(stats.approved_count, 1);
        assert_eq!(stats.pending_count, 0);
    }

    #[test]
    fn test_checkpoint_types() {
        assert_eq!(CheckpointType::PlanReady.as_str(), "PlanReady");
        assert_eq!(CheckpointType::ReviewComplete.as_str(), "ReviewComplete");
        assert_eq!(CheckpointType::IterationDone.as_str(), "IterationDone");
    }
}
