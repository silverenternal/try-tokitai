//! 迭代状态追踪器
//!
//! 可视化自主迭代的完整过程，支持用户监控和干预
//!
//! # 设计原则
//! - 状态机 + 事件溯源
//! - 纯文件存储，支持增量追加
//! - TUI 实时显示状态和进度

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use uuid::Uuid;

/// 迭代追踪错误类型
#[derive(Error, Debug)]
pub enum IterationTrackerError {
    #[error("状态转换失败：{0}")]
    StateTransitionFailed(String),
    #[error("文件操作失败：{0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON 处理失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// 迭代状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IterationState {
    /// 初始化中
    Initializing,
    /// 调研中
    Researching,
    /// 规划中
    Planning,
    /// 执行中
    Executing,
    /// 审查中
    Reviewing,
    /// 改进中
    Refining,
    /// 验证中
    Validating,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已暂停（等待用户干预）
    Paused,
}

impl std::fmt::Display for IterationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IterationState::Initializing => write!(f, "初始化"),
            IterationState::Researching => write!(f, "调研中"),
            IterationState::Planning => write!(f, "规划中"),
            IterationState::Executing => write!(f, "执行中"),
            IterationState::Reviewing => write!(f, "审查中"),
            IterationState::Refining => write!(f, "改进中"),
            IterationState::Validating => write!(f, "验证中"),
            IterationState::Completed => write!(f, "已完成"),
            IterationState::Failed => write!(f, "失败"),
            IterationState::Paused => write!(f, "已暂停"),
        }
    }
}

/// 迭代事件类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IterationEvent {
    /// 迭代开始
    IterationStarted {
        goal: String,
        context: Option<String>,
    },
    /// 状态转换
    StateChanged {
        from: IterationState,
        to: IterationState,
        reason: Option<String>,
    },
    /// 任务开始
    TaskStarted {
        task_id: String,
        task_description: String,
    },
    /// 任务完成
    TaskCompleted {
        task_id: String,
        result: String,
    },
    /// 任务失败
    TaskFailed {
        task_id: String,
        error: String,
    },
    /// 审查提交
    ReviewSubmitted {
        score: String,
        summary: String,
        issues: Vec<String>,
    },
    /// 改进应用
    RefinementApplied {
        changes: Vec<String>,
    },
    /// 用户干预
    UserIntervention {
        action: String,
        details: Option<String>,
    },
    /// 迭代完成
    IterationCompleted {
        summary: String,
        success: bool,
    },
    /// 迭代失败
    IterationFailed {
        reason: String,
    },
    /// 检查点
    Checkpoint {
        checkpoint_type: String,
        requires_approval: bool,
    },
}

/// 事件记录（带时间戳）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub timestamp: i64,
    pub event: IterationEvent,
}

/// 迭代会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IterationSession {
    /// 迭代唯一标识
    pub id: String,
    /// 迭代目标
    pub goal: String,
    /// 当前状态
    pub current_state: IterationState,
    /// 开始时间戳
    pub started_at: i64,
    /// 结束时间戳（可选）
    pub ended_at: Option<i64>,
    /// 事件历史
    pub events: Vec<EventRecord>,
    /// 当前上下文摘要
    pub context_summary: Option<String>,
    /// 迭代总结
    pub summary: Option<String>,
    /// 是否成功
    pub success: Option<bool>,
}

impl IterationSession {
    /// 创建新的迭代会话
    pub fn new(goal: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            goal,
            current_state: IterationState::Initializing,
            started_at: now,
            ended_at: None,
            events: vec![],
            context_summary: None,
            summary: None,
            success: None,
        }
    }

    /// 添加事件
    pub fn add_event(&mut self, event: IterationEvent) {
        self.events.push(EventRecord {
            timestamp: chrono::Utc::now().timestamp(),
            event,
        });
    }

    /// 获取迭代持续时间（秒）
    pub fn duration(&self) -> Option<i64> {
        self.ended_at.map(|end| end - self.started_at)
    }

    /// 获取事件数量
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// 获取当前进度百分比
    pub fn progress_percentage(&self) -> f64 {
        match self.current_state {
            IterationState::Initializing => 0.0,
            IterationState::Researching => 10.0,
            IterationState::Planning => 20.0,
            IterationState::Executing => 50.0,
            IterationState::Reviewing => 70.0,
            IterationState::Refining => 80.0,
            IterationState::Validating => 90.0,
            IterationState::Completed => 100.0,
            IterationState::Failed => 0.0,
            IterationState::Paused => 0.0,
        }
    }

    /// 获取最新事件
    pub fn last_event(&self) -> Option<&IterationEvent> {
        self.events.last().map(|e| &e.event)
    }
}

/// 迭代追踪器
pub struct IterationTracker {
    /// 存储目录
    storage_dir: PathBuf,
    /// 当前迭代会话
    current_session: Option<IterationSession>,
    /// 历史迭代 ID 列表
    history: Vec<String>,
}

impl IterationTracker {
    /// 创建新的迭代追踪器
    pub fn new(storage_dir: PathBuf) -> Result<Self, IterationTrackerError> {
        fs::create_dir_all(&storage_dir)?;
        
        let mut tracker = Self {
            storage_dir,
            current_session: None,
            history: vec![],
        };

        // 加载历史
        tracker.load_history()?;

        Ok(tracker)
    }

    /// 开始新的迭代
    pub fn start_iteration(&mut self, goal: String, context: Option<String>) -> Result<&IterationSession, IterationTrackerError> {
        let mut session = IterationSession::new(goal.clone());
        session.add_event(IterationEvent::IterationStarted {
            goal: goal.clone(),
            context: context.clone(),
        });
        session.context_summary = context;

        self.current_session = Some(session);
        self.save_current()?;

        Ok(self.current_session.as_ref().unwrap())
    }

    /// 转换状态
    pub fn transition_state(
        &mut self,
        new_state: IterationState,
        reason: Option<String>,
    ) -> Result<(), IterationTrackerError> {
        let session = self.current_session.as_mut()
            .ok_or_else(|| IterationTrackerError::StateTransitionFailed("没有活跃的迭代会话".to_string()))?;

        let old_state = session.current_state.clone();
        
        // 验证状态转换合法性
        if !Self::is_valid_transition(&old_state, &new_state) {
            return Err(IterationTrackerError::StateTransitionFailed(
                format!("无效的状态转换：{} -> {}", old_state, new_state)
            ));
        }

        session.add_event(IterationEvent::StateChanged {
            from: old_state,
            to: new_state.clone(),
            reason: reason.clone(),
        });
        session.current_state = new_state;
        self.save_current()?;

        Ok(())
    }

    /// 记录任务开始
    pub fn record_task_started(&mut self, task_id: String, task_description: String) -> Result<(), IterationTrackerError> {
        if let Some(session) = self.current_session.as_mut() {
            session.add_event(IterationEvent::TaskStarted {
                task_id,
                task_description,
            });
            self.save_current()?;
        }
        Ok(())
    }

    /// 记录任务完成
    pub fn record_task_completed(&mut self, task_id: String, result: String) -> Result<(), IterationTrackerError> {
        if let Some(session) = self.current_session.as_mut() {
            session.add_event(IterationEvent::TaskCompleted {
                task_id,
                result,
            });
            self.save_current()?;
        }
        Ok(())
    }

    /// 记录任务失败
    pub fn record_task_failed(&mut self, task_id: String, error: String) -> Result<(), IterationTrackerError> {
        if let Some(session) = self.current_session.as_mut() {
            session.add_event(IterationEvent::TaskFailed {
                task_id,
                error,
            });
            self.save_current()?;
        }
        Ok(())
    }

    /// 记录审查结果
    pub fn record_review(&mut self, score: String, summary: String, issues: Vec<String>) -> Result<(), IterationTrackerError> {
        if let Some(session) = self.current_session.as_mut() {
            session.add_event(IterationEvent::ReviewSubmitted {
                score,
                summary,
                issues,
            });
            self.save_current()?;
        }
        Ok(())
    }

    /// 记录改进应用
    pub fn record_refinement(&mut self, changes: Vec<String>) -> Result<(), IterationTrackerError> {
        if let Some(session) = self.current_session.as_mut() {
            session.add_event(IterationEvent::RefinementApplied { changes });
            self.save_current()?;
        }
        Ok(())
    }

    /// 记录用户干预
    pub fn record_user_intervention(&mut self, action: String, details: Option<String>) -> Result<(), IterationTrackerError> {
        if let Some(session) = self.current_session.as_mut() {
            session.add_event(IterationEvent::UserIntervention { action, details });
            self.save_current()?;
        }
        Ok(())
    }

    /// 完成迭代
    pub fn complete_iteration(&mut self, summary: String, success: bool) -> Result<(), IterationTrackerError> {
        let session = self.current_session.as_mut()
            .ok_or_else(|| IterationTrackerError::StateTransitionFailed("没有活跃的迭代会话".to_string()))?;

        session.add_event(IterationEvent::IterationCompleted {
            summary: summary.clone(),
            success,
        });
        session.summary = Some(summary);
        session.success = Some(success);
        session.ended_at = Some(chrono::Utc::now().timestamp());
        session.current_state = if success {
            IterationState::Completed
        } else {
            IterationState::Failed
        };

        // 移动到历史
        if let Some(session) = self.current_session.take() {
            self.save_session_to_history(&session)?;
            self.history.push(session.id.clone());
            self.save_history()?;
        }

        Ok(())
    }

    /// 失败迭代
    pub fn fail_iteration(&mut self, reason: String) -> Result<(), IterationTrackerError> {
        let session = self.current_session.as_mut()
            .ok_or_else(|| IterationTrackerError::StateTransitionFailed("没有活跃的迭代会话".to_string()))?;

        session.add_event(IterationEvent::IterationFailed { reason: reason.clone() });
        session.summary = Some(format!("失败：{}", reason));
        session.success = Some(false);
        session.ended_at = Some(chrono::Utc::now().timestamp());
        session.current_state = IterationState::Failed;

        // 移动到历史
        if let Some(session) = self.current_session.take() {
            self.save_session_to_history(&session)?;
            self.history.push(session.id.clone());
            self.save_history()?;
        }

        Ok(())
    }

    /// 暂停迭代（等待用户干预）
    pub fn pause_iteration(&mut self, reason: Option<String>) -> Result<(), IterationTrackerError> {
        self.transition_state(IterationState::Paused, reason)
    }

    /// 恢复迭代
    pub fn resume_iteration(&mut self, new_state: IterationState) -> Result<(), IterationTrackerError> {
        self.transition_state(new_state, Some("用户恢复迭代".to_string()))
    }

    /// 获取当前会话
    pub fn current_session(&self) -> Option<&IterationSession> {
        self.current_session.as_ref()
    }

    /// 获取当前状态
    pub fn current_state(&self) -> Option<&IterationState> {
        self.current_session.as_ref().map(|s| &s.current_state)
    }

    /// 获取进度百分比
    pub fn progress(&self) -> Option<f64> {
        self.current_session.as_ref().map(|s| s.progress_percentage())
    }

    /// 获取历史迭代列表
    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// 加载历史迭代
    fn load_history(&mut self) -> Result<(), IterationTrackerError> {
        let history_path = self.storage_dir.join("history.json");
        if history_path.exists() {
            let content = fs::read_to_string(history_path)?;
            self.history = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    /// 保存当前会话
    fn save_current(&self) -> Result<(), IterationTrackerError> {
        if let Some(session) = &self.current_session {
            let session_path = self.storage_dir.join(format!("{}.json", session.id));
            let content = serde_json::to_string_pretty(session)?;
            fs::write(&session_path, &content)?;

            // 同时保存到 current.json 用于快速访问
            let current_path = self.storage_dir.join("current.json");
            fs::write(&current_path, &content)?;
        }
        Ok(())
    }

    /// 保存会话到历史
    fn save_session_to_history(&self, session: &IterationSession) -> Result<(), IterationTrackerError> {
        let session_path = self.storage_dir.join(format!("{}.json", session.id));
        let content = serde_json::to_string_pretty(session)?;
        fs::write(&session_path, content)?;
        Ok(())
    }

    /// 保存历史列表
    fn save_history(&self) -> Result<(), IterationTrackerError> {
        let history_path = self.storage_dir.join("history.json");
        let content = serde_json::to_string_pretty(&self.history)?;
        fs::write(&history_path, content)?;
        Ok(())
    }

    /// 验证状态转换是否合法
    fn is_valid_transition(from: &IterationState, to: &IterationState) -> bool {
        match from {
            IterationState::Initializing => {
                matches!(to, IterationState::Researching | IterationState::Planning | IterationState::Failed)
            }
            IterationState::Researching => {
                matches!(to, IterationState::Planning | IterationState::Failed | IterationState::Paused)
            }
            IterationState::Planning => {
                matches!(to, IterationState::Executing | IterationState::Failed | IterationState::Paused)
            }
            IterationState::Executing => {
                matches!(to, IterationState::Reviewing | IterationState::Failed | IterationState::Paused)
            }
            IterationState::Reviewing => {
                matches!(to, IterationState::Refining | IterationState::Validating | IterationState::Failed | IterationState::Paused)
            }
            IterationState::Refining => {
                matches!(to, IterationState::Executing | IterationState::Reviewing | IterationState::Failed)
            }
            IterationState::Validating => {
                matches!(to, IterationState::Completed | IterationState::Refining | IterationState::Failed)
            }
            IterationState::Completed | IterationState::Failed => {
                false // 终态
            }
            IterationState::Paused => {
                matches!(to, IterationState::Executing | IterationState::Planning | IterationState::Failed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_tracker() -> (IterationTracker, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let tracker = IterationTracker::new(temp_dir.path().to_path_buf()).unwrap();
        (tracker, temp_dir)
    }

    #[test]
    fn test_iteration_lifecycle() {
        let (mut tracker, _temp_dir) = create_test_tracker();

        // 开始迭代
        tracker.start_iteration("测试目标".to_string(), None).unwrap();
        assert_eq!(tracker.current_state(), Some(&IterationState::Initializing));

        // 状态转换
        tracker.transition_state(IterationState::Planning, None).unwrap();
        assert_eq!(tracker.current_state(), Some(&IterationState::Planning));

        // 完成迭代
        tracker.complete_iteration("测试完成".to_string(), true).unwrap();
        assert!(tracker.current_session().is_none());
    }

    #[test]
    fn test_invalid_transition() {
        let (mut tracker, _temp_dir) = create_test_tracker();

        tracker.start_iteration("测试目标".to_string(), None).unwrap();
        
        // 尝试非法转换
        let result = tracker.transition_state(IterationState::Completed, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_progress_calculation() {
        let (mut tracker, _temp_dir) = create_test_tracker();

        tracker.start_iteration("测试目标".to_string(), None).unwrap();
        assert_eq!(tracker.progress(), Some(0.0));

        // 合法的状态转换
        tracker.transition_state(IterationState::Planning, None).unwrap();
        assert_eq!(tracker.progress(), Some(20.0));

        tracker.transition_state(IterationState::Executing, None).unwrap();
        assert_eq!(tracker.progress(), Some(50.0));

        tracker.transition_state(IterationState::Reviewing, None).unwrap();
        tracker.transition_state(IterationState::Validating, None).unwrap();
        tracker.transition_state(IterationState::Completed, None).unwrap();
        assert_eq!(tracker.progress(), Some(100.0));
    }
}
