//! 对话状态机
//!
//! 实现结构化的对话状态管理，追踪任务进度
//!
//! # 设计原则
//! - 定义明确的对话状态
//! - 每个状态维护专属上下文和可用工具子集
//! - 状态转换时自动保存/恢复相关上下文
//! - 支持用户中断和状态回滚

#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// 对话状态机错误类型
#[derive(Error, Debug)]
pub enum DialogueError {
    #[error("无效的状态转换：{0} -> {1}")]
    InvalidTransition(String, String),
    #[error("文件操作失败：{0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON 处理失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// 对话状态枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DialogueState {
    /// 空闲状态 - 等待用户输入
    Idle,
    /// 澄清中 - 需要用户澄清需求
    Clarifying,
    /// 规划中 - 正在制定任务计划
    Planning,
    /// 执行中 - 正在执行任务
    Executing,
    /// 审查中 - 正在审查结果
    Reviewing,
    /// 完成 - 任务完成
    Completed,
    /// 错误 - 发生错误
    Error,
    /// 等待用户确认
    WaitingForConfirmation,
}

impl std::fmt::Display for DialogueState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DialogueState::Idle => write!(f, "空闲"),
            DialogueState::Clarifying => write!(f, "澄清中"),
            DialogueState::Planning => write!(f, "规划中"),
            DialogueState::Executing => write!(f, "执行中"),
            DialogueState::Reviewing => write!(f, "审查中"),
            DialogueState::Completed => write!(f, "完成"),
            DialogueState::Error => write!(f, "错误"),
            DialogueState::WaitingForConfirmation => write!(f, "等待确认"),
        }
    }
}

/// 对话上下文
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DialogueContext {
    /// 当前任务目标
    pub current_goal: Option<String>,
    /// 任务计划
    pub plan: Option<String>,
    /// 已执行的工具
    pub executed_tools: Vec<String>,
    /// 临时变量
    pub variables: HashMap<String, String>,
    /// 用户偏好
    pub user_preferences: HashMap<String, String>,
    /// 待确认事项
    pub pending_confirmations: Vec<String>,
}

impl DialogueContext {
    /// 创建新的上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置任务目标
    pub fn with_goal(mut self, goal: String) -> Self {
        self.current_goal = Some(goal);
        self
    }

    /// 设置变量
    pub fn with_variable(mut self, key: String, value: String) -> Self {
        self.variables.insert(key, value);
        self
    }

    /// 获取变量
    pub fn get(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// 设置变量
    pub fn set(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    /// 添加工具执行记录
    pub fn add_tool_execution(&mut self, tool_name: String) {
        self.executed_tools.push(tool_name);
    }

    /// 添加待确认事项
    pub fn add_pending_confirmation(&mut self, confirmation: String) {
        self.pending_confirmations.push(confirmation);
    }

    /// 清除上下文
    pub fn clear(&mut self) {
        self.current_goal = None;
        self.plan = None;
        self.executed_tools.clear();
        self.variables.clear();
        self.pending_confirmations.clear();
    }
}

/// 状态转换记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// 从状态
    pub from: DialogueState,
    /// 到状态
    pub to: DialogueState,
    /// 转换原因
    pub reason: Option<String>,
    /// 时间戳
    pub timestamp: i64,
}

/// 对话历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueHistory {
    /// 状态转换历史
    pub transitions: Vec<StateTransition>,
    /// 上下文快照
    pub context_snapshots: Vec<(i64, DialogueContext)>,
}

impl DialogueHistory {
    /// 创建新的历史
    pub fn new() -> Self {
        Self {
            transitions: vec![],
            context_snapshots: vec![],
        }
    }

    /// 添加状态转换
    pub fn add_transition(&mut self, from: DialogueState, to: DialogueState, reason: Option<String>) {
        self.transitions.push(StateTransition {
            from,
            to,
            reason,
            timestamp: chrono::Utc::now().timestamp(),
        });
    }

    /// 添加上下文快照
    pub fn add_snapshot(&mut self, context: &DialogueContext) {
        self.context_snapshots.push((chrono::Utc::now().timestamp(), context.clone()));
    }

    /// 获取最近的上下文
    pub fn last_context(&self) -> Option<&DialogueContext> {
        self.context_snapshots.last().map(|(_, ctx)| ctx)
    }

    /// 获取状态转换次数
    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }
}

impl Default for DialogueHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// 对话状态机
pub struct DialogueStateMachine {
    /// 当前状态
    current_state: DialogueState,
    /// 当前上下文
    context: DialogueContext,
    /// 历史记录
    history: DialogueHistory,
    /// 存储目录
    storage_dir: PathBuf,
}

impl DialogueStateMachine {
    /// 创建新的状态机
    pub fn new(storage_dir: PathBuf) -> Result<Self, DialogueError> {
        fs::create_dir_all(&storage_dir)?;

        let mut machine = Self {
            current_state: DialogueState::Idle,
            context: DialogueContext::new(),
            history: DialogueHistory::new(),
            storage_dir,
        };

        machine.load_state()?;

        Ok(machine)
    }
    
    /// 创建不带持久化的状态机（用于测试）
    pub fn new_without_persistence() -> Self {
        use std::env;
        let temp_dir = env::temp_dir().join("dialogue_test");
        Self::new(temp_dir.clone()).unwrap_or_else(|_| Self {
            current_state: DialogueState::Idle,
            context: DialogueContext::new(),
            history: DialogueHistory::new(),
            storage_dir: temp_dir,
        })
    }

    /// 转换状态
    pub fn transition(&mut self, new_state: DialogueState, reason: Option<String>) -> Result<(), DialogueError> {
        let old_state = self.current_state.clone();

        // 验证状态转换合法性
        if !Self::is_valid_transition(&old_state, &new_state) {
            return Err(DialogueError::InvalidTransition(
                old_state.to_string(),
                new_state.to_string(),
            ));
        }

        // 记录转换
        self.history.add_transition(old_state.clone(), new_state.clone(), reason);

        // 保存上下文快照
        self.history.add_snapshot(&self.context);

        // 更新状态
        self.current_state = new_state;

        // 持久化
        self.save_state()?;

        Ok(())
    }

    /// 保存状态（包含上下文）
    pub fn save_state_with_context(&mut self) -> Result<(), DialogueError> {
        self.save_state()
    }

    /// 获取当前状态
    pub fn current_state(&self) -> &DialogueState {
        &self.current_state
    }

    /// 获取当前上下文（可变引用）
    pub fn context_mut(&mut self) -> &mut DialogueContext {
        &mut self.context
    }

    /// 获取当前上下文（不可变引用）
    pub fn context(&self) -> &DialogueContext {
        &self.context
    }

    /// 获取历史
    pub fn history(&self) -> &DialogueHistory {
        &self.history
    }
    
    /// 获取状态转换历史
    pub fn get_history(&self) -> &Vec<StateTransition> {
        &self.history.transitions
    }
    
    /// 设置任务目标
    pub fn set_goal(&mut self, goal: String) -> Result<(), DialogueError> {
        self.context.current_goal = Some(goal);
        self.save_state()
    }
    
    /// 设置任务计划
    pub fn set_plan(&mut self, plan: String) -> Result<(), DialogueError> {
        self.context.plan = Some(plan);
        self.save_state()
    }
    
    /// 记录工具执行
    pub fn record_tool(&mut self, tool_name: String) -> Result<(), DialogueError> {
        self.context.executed_tools.push(tool_name);
        self.save_state()
    }
    
    /// 添加待确认事项
    pub fn add_confirmation(&mut self, item: String) -> Result<(), DialogueError> {
        self.context.pending_confirmations.push(item);
        self.save_state()
    }
    
    /// 清除待确认事项
    pub fn clear_confirmations(&mut self) -> Result<(), DialogueError> {
        self.context.pending_confirmations.clear();
        self.save_state()
    }
    
    /// 设置变量
    pub fn set_variable(&mut self, key: String, value: String) -> Result<(), DialogueError> {
        self.context.variables.insert(key, value);
        self.save_state()
    }
    
    /// 获取变量
    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.context.variables.get(key)
    }
    
    /// 保存到文件
    pub fn save_to_file(&self, path: &str) -> Result<(), DialogueError> {
        let state = DialogueStateFile {
            current_state: self.current_state.clone(),
            context: self.context.clone(),
            history: self.history.clone(),
        };
        let content = serde_json::to_string_pretty(&state)?;
        fs::write(path, content)?;
        Ok(())
    }
    
    /// 从文件加载
    pub fn load_from_file(&mut self, path: &str) -> Result<(), DialogueError> {
        let content = fs::read_to_string(path)?;
        let state: DialogueStateFile = serde_json::from_str(&content)?;
        self.current_state = state.current_state;
        self.context = state.context;
        self.history = state.history;
        Ok(())
    }

    /// 重置状态机
    pub fn reset(&mut self) -> Result<(), DialogueError> {
        self.current_state = DialogueState::Idle;
        self.context.clear();
        self.history = DialogueHistory::new();
        self.save_state()?;
        Ok(())
    }

    /// 回滚到上一个状态
    pub fn rollback(&mut self) -> Result<(), DialogueError> {
        if let Some(last_transition) = self.history.transitions.last() {
            let previous_state = last_transition.from.clone();
            self.current_state = previous_state;
            
            if let Some(last_snapshot) = self.history.context_snapshots.iter().rev().nth(1) {
                self.context = last_snapshot.1.clone();
            }
            
            self.save_state()?;
        }
        Ok(())
    }

    /// 保存状态
    fn save_state(&self) -> Result<(), DialogueError> {
        let state_path = self.storage_dir.join("dialogue_state.json");
        
        let state = DialogueStateFile {
            current_state: self.current_state.clone(),
            context: self.context.clone(),
            history: self.history.clone(),
        };

        let content = serde_json::to_string_pretty(&state)?;
        fs::write(&state_path, content)?;

        Ok(())
    }

    /// 加载状态
    fn load_state(&mut self) -> Result<(), DialogueError> {
        let state_path = self.storage_dir.join("dialogue_state.json");
        
        if state_path.exists() {
            let content = fs::read_to_string(&state_path)?;
            let state: DialogueStateFile = serde_json::from_str(&content)?;

            self.current_state = state.current_state;
            self.context = state.context;
            self.history = state.history;
        }

        Ok(())
    }

    /// 验证状态转换是否合法
    fn is_valid_transition(from: &DialogueState, to: &DialogueState) -> bool {
        match from {
            DialogueState::Idle => {
                matches!(to, DialogueState::Clarifying | DialogueState::Planning | DialogueState::Executing)
            }
            DialogueState::Clarifying => {
                matches!(to, DialogueState::Idle | DialogueState::Planning | DialogueState::Error)
            }
            DialogueState::Planning => {
                matches!(to, DialogueState::Executing | DialogueState::Clarifying | DialogueState::Error | DialogueState::WaitingForConfirmation)
            }
            DialogueState::Executing => {
                matches!(to, DialogueState::Reviewing | DialogueState::Planning | DialogueState::Error | DialogueState::WaitingForConfirmation)
            }
            DialogueState::Reviewing => {
                matches!(to, DialogueState::Executing | DialogueState::Planning | DialogueState::Completed | DialogueState::Error)
            }
            DialogueState::WaitingForConfirmation => {
                matches!(to, DialogueState::Executing | DialogueState::Planning | DialogueState::Idle)
            }
            DialogueState::Completed | DialogueState::Error => {
                matches!(to, DialogueState::Idle)
            }
        }
    }
}

/// 持久化结构
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DialogueStateFile {
    current_state: DialogueState,
    context: DialogueContext,
    history: DialogueHistory,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_state_machine_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let mut machine = DialogueStateMachine::new(temp_dir.path().to_path_buf()).unwrap();

        // 初始状态
        assert_eq!(machine.current_state(), &DialogueState::Idle);

        // 转换到规划
        machine.transition(DialogueState::Planning, Some("用户请求".to_string())).unwrap();
        assert_eq!(machine.current_state(), &DialogueState::Planning);

        // 转换到执行
        machine.transition(DialogueState::Executing, None).unwrap();
        assert_eq!(machine.current_state(), &DialogueState::Executing);

        // 转换到审查
        machine.transition(DialogueState::Reviewing, None).unwrap();
        assert_eq!(machine.current_state(), &DialogueState::Reviewing);

        // 转换到完成
        machine.transition(DialogueState::Completed, None).unwrap();
        assert_eq!(machine.current_state(), &DialogueState::Completed);

        // 验证历史
        assert_eq!(machine.history().transition_count(), 4);
    }

    #[test]
    fn test_invalid_transition() {
        let temp_dir = TempDir::new().unwrap();
        let mut machine = DialogueStateMachine::new(temp_dir.path().to_path_buf()).unwrap();

        // 尝试从 Idle 直接到 Completed（非法）
        let result = machine.transition(DialogueState::Completed, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_context_management() {
        let temp_dir = TempDir::new().unwrap();
        let mut machine = DialogueStateMachine::new(temp_dir.path().to_path_buf()).unwrap();

        // 设置上下文
        machine.context_mut().set("task_id".to_string(), "123".to_string());
        machine.context_mut().set("goal".to_string(), "测试目标".to_string());

        assert_eq!(machine.context().get("task_id"), Some(&"123".to_string()));
        assert_eq!(machine.context().get("goal"), Some(&"测试目标".to_string()));
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();

        {
            let mut machine = DialogueStateMachine::new(temp_dir.path().to_path_buf()).unwrap();
            machine.transition(DialogueState::Planning, None).unwrap();
            machine.context_mut().set("key".to_string(), "value".to_string());
            machine.save_state().unwrap(); // 保存上下文
        }

        // 重新加载
        let machine2 = DialogueStateMachine::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(machine2.current_state(), &DialogueState::Planning);
        assert_eq!(machine2.context().get("key"), Some(&"value".to_string()));
    }
}
