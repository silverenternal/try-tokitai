//! Agent 协调器
//!
//! 协调 Planner-Executor-Reviewer 三个 Agent 的协作
//!
//! ## 工具矩阵集成
//! Coordinator 现在通过 ToolRegistry 创建 ExecutorAgent，
//! 使得执行 Agent 可以通过工具矩阵动态调用工具。

use super::{PlannerAgent, ExecutorAgent, ReviewerAgent};
use super::planner::RiskLevel;
use crate::autonomy::iteration_tracker::{IterationTracker, IterationState};
use crate::tool_matrix::registry::ToolRegistry;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use parking_lot::RwLock;
use thiserror::Error;

/// 协调器错误类型
#[derive(Error, Debug)]
pub enum CoordinatorError {
    #[error("规划失败：{0}")]
    PlanningFailed(String),
    #[error("执行失败：{0}")]
    ExecutionFailed(String),
    #[error("审查失败：{0}")]
    ReviewFailed(String),
    #[error("迭代追踪失败：{0}")]
    IterationFailed(String),
}

/// 协调器状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CoordinatorState {
    Idle,
    Planning,
    Executing,
    Reviewing,
    Completed,
    Failed,
}

/// Agent 协调器
pub struct AgentCoordinator {
    /// 规划 Agent
    pub planner: PlannerAgent,
    /// 执行 Agent
    pub executor: ExecutorAgent,
    /// 审查 Agent
    pub reviewer: ReviewerAgent,
    /// 迭代追踪器
    pub tracker: IterationTracker,
    /// 工具注册表（用于动态工具调用）
    pub tool_registry: Arc<RwLock<ToolRegistry>>,
    /// 当前状态
    pub state: CoordinatorState,
}

impl AgentCoordinator {
    /// 创建新的协调器
    pub fn new(base_dir: PathBuf, tool_registry: Arc<RwLock<ToolRegistry>>) -> Result<Self, CoordinatorError> {
        let planner_dir = base_dir.join("planner");
        let executor_dir = base_dir.join("executor");
        let reviewer_dir = base_dir.join("reviewer");
        let tracker_dir = base_dir.join("tracker");

        Ok(Self {
            planner: PlannerAgent::new(planner_dir).map_err(|e| CoordinatorError::PlanningFailed(e.to_string()))?,
            executor: ExecutorAgent::new(executor_dir, tool_registry.clone()).map_err(|e| CoordinatorError::ExecutionFailed(e.to_string()))?,
            reviewer: ReviewerAgent::new(reviewer_dir).map_err(|e| CoordinatorError::ReviewFailed(e.to_string()))?,
            tracker: IterationTracker::new(tracker_dir).map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?,
            tool_registry,
            state: CoordinatorState::Idle,
        })
    }

    /// 创建协调器（带自定义工具注册表）
    pub fn with_registry(base_dir: PathBuf, tool_registry: Arc<RwLock<ToolRegistry>>) -> Result<Self, CoordinatorError> {
        Self::new(base_dir, tool_registry)
    }

    /// 开始自主迭代
    pub fn start_iteration(&mut self, goal: String) -> Result<(), CoordinatorError> {
        // 开始迭代追踪
        self.tracker
            .start_iteration(goal.clone(), None)
            .map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?;

        // 转换到规划状态
        self.tracker
            .transition_state(IterationState::Planning, None)
            .map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?;

        self.state = CoordinatorState::Planning;

        // 创建计划
        let plan = self.planner.create_plan(goal);
        tracing::info!("创建计划：{}", plan.id);

        Ok(())
    }

    /// 添加计划步骤
    pub fn add_plan_step(
        &mut self,
        description: String,
        tools: Vec<String>,
        expected_output: String,
        dependencies: Vec<String>,
        estimated_minutes: u32,
        risk_level: RiskLevel,
    ) -> Result<(), CoordinatorError> {
        if let Some(plan) = self.planner.last_plan() {
            let plan_id = plan.id.clone();
            let _ = plan; // 释放借用

            self.planner.add_step_to_plan(
                &plan_id,
                description,
                tools,
                expected_output,
                dependencies,
                estimated_minutes,
                risk_level,
            ).map_err(|e| CoordinatorError::PlanningFailed(e.to_string()))?;
        }
        Ok(())
    }

    /// 开始执行计划
    pub fn start_execution(&mut self) -> Result<(), CoordinatorError> {
        // 转换到执行状态
        self.tracker
            .transition_state(IterationState::Executing, None)
            .map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?;

        self.state = CoordinatorState::Executing;

        // 开始执行
        if let Some(plan) = self.planner.last_plan() {
            let plan_id = plan.id.clone();
            let _ = plan;

            self.executor.start_execution(plan_id);
        }

        Ok(())
    }

    /// 审查结果
    pub fn review(&mut self, file_path: &std::path::Path, content: &str) -> Result<(), CoordinatorError> {
        // 转换到审查状态
        self.tracker
            .transition_state(IterationState::Reviewing, None)
            .map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?;

        self.state = CoordinatorState::Reviewing;

        // 执行审查
        let report = self.reviewer
            .review_file(file_path, content)
            .map_err(|e| CoordinatorError::ReviewFailed(e.to_string()))?;

        // 记录审查结果
        self.tracker
            .record_review(
                report.grade.to_string(),
                report.summary.clone(),
                report.issues.iter().map(|i| i.description.clone()).collect(),
            )
            .map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?;

        Ok(())
    }

    /// 完成迭代
    pub fn complete_iteration(&mut self, summary: String) -> Result<(), CoordinatorError> {
        self.tracker
            .complete_iteration(summary, true)
            .map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?;

        self.state = CoordinatorState::Completed;

        Ok(())
    }

    /// 失败迭代
    pub fn fail_iteration(&mut self, reason: String) -> Result<(), CoordinatorError> {
        self.tracker
            .fail_iteration(reason)
            .map_err(|e| CoordinatorError::IterationFailed(e.to_string()))?;

        self.state = CoordinatorState::Failed;

        Ok(())
    }

    /// 获取当前状态
    pub fn state(&self) -> &CoordinatorState {
        &self.state
    }

    /// 获取迭代进度
    pub fn progress(&self) -> Option<f64> {
        self.tracker.progress()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_registry() -> Arc<RwLock<ToolRegistry>> {
        Arc::new(RwLock::new(ToolRegistry::new()))
    }

    #[test]
    fn test_coordinator_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let registry = create_test_registry();
        let mut coordinator = AgentCoordinator::new(temp_dir.path().to_path_buf(), registry).unwrap();

        // 开始迭代
        coordinator.start_iteration("测试目标".to_string()).unwrap();
        assert_eq!(coordinator.state(), &CoordinatorState::Planning);

        // 添加步骤
        coordinator.add_plan_step(
            "测试步骤".to_string(),
            vec!["read_file".to_string()],
            "输出".to_string(),
            vec![],
            10,
            RiskLevel::Low,
        ).unwrap();

        // 开始执行
        coordinator.start_execution().unwrap();
        assert_eq!(coordinator.state(), &CoordinatorState::Executing);

        // 完成迭代
        coordinator.complete_iteration("测试完成".to_string()).unwrap();
        assert_eq!(coordinator.state(), &CoordinatorState::Completed);
    }
}
