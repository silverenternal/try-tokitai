//! 工作流程引擎
//!
//! 支持多步骤任务的编排执行
//!
//! ## 核心概念
//! - **Workflow**: 定义一个完整的工作流程，包含多个阶段
//! - **Stage**: 工作流程中的一个阶段，包含具体的执行步骤
//! - **Step**: 最小执行单元，执行一个具体任务
//! - **Context**: 工作流程执行的上下文，传递各阶段的数据

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::orchestrator::{AgentRole, ContextOptimizer, ContextMessage, MessageType, RoleSwitcher};

/// 步骤状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已跳过
    Skipped,
}

/// 执行步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    /// 步骤 ID
    pub id: String,
    /// 步骤描述
    pub description: String,
    /// 执行该步骤的角色
    pub role: AgentRole,
    /// 步骤状态
    pub status: StepStatus,
    /// 输入数据
    pub input: Option<String>,
    /// 输出数据
    pub output: Option<String>,
    /// 错误信息
    pub error: Option<String>,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
    /// 是否需要用户确认
    pub requires_approval: bool,
    /// 前置步骤 ID 列表
    pub dependencies: Vec<String>,
}

impl Step {
    pub fn new(id: String, description: String, role: AgentRole) -> Self {
        Self {
            id,
            description,
            role,
            status: StepStatus::Pending,
            input: None,
            output: None,
            error: None,
            duration_ms: None,
            requires_approval: false,
            dependencies: Vec::new(),
        }
    }

    pub fn with_dependencies(
        id: String,
        description: String,
        role: AgentRole,
        dependencies: Vec<String>,
    ) -> Self {
        let mut step = Self::new(id, description, role);
        step.dependencies = dependencies;
        step
    }

    pub fn requires_approval(mut self, required: bool) -> Self {
        self.requires_approval = required;
        self
    }
}

/// 阶段状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StageStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 部分完成
    PartiallyCompleted,
}

/// 工作流程阶段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stage {
    /// 阶段 ID
    pub id: String,
    /// 阶段名称
    pub name: String,
    /// 阶段描述
    pub description: String,
    /// 包含的步骤
    pub steps: Vec<Step>,
    /// 阶段状态
    pub status: StageStatus,
    /// 阶段输出
    pub output: Option<String>,
}

impl Stage {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            steps: Vec::new(),
            status: StageStatus::Pending,
            output: None,
        }
    }

    pub fn add_step(&mut self, step: Step) {
        self.steps.push(step);
    }

    /// 检查是否所有步骤都已完成
    pub fn is_complete(&self) -> bool {
        self.steps.iter().all(|s| {
            s.status == StepStatus::Completed || s.status == StepStatus::Skipped
        })
    }

    /// 检查是否有步骤失败
    pub fn has_failed(&self) -> bool {
        self.steps.iter().any(|s| s.status == StepStatus::Failed)
    }

    /// 获取可执行的步骤（依赖已满足）
    pub fn get_ready_steps(&self) -> Vec<&Step> {
        let completed_ids: std::collections::HashSet<&str> = self
            .steps
            .iter()
            .filter(|s| s.status == StepStatus::Completed)
            .map(|s| s.id.as_str())
            .collect();

        self.steps
            .iter()
            .filter(|s| {
                s.status == StepStatus::Pending
                    && s.dependencies
                        .iter()
                        .all(|dep| completed_ids.contains(dep.as_str()))
            })
            .collect()
    }
}

/// 工作流程状态
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    /// 待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 已暂停（等待用户确认）
    Paused,
    /// 已取消
    Cancelled,
}

/// 工作流程定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// 工作流 ID
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 包含的阶段
    pub stages: Vec<Stage>,
    /// 工作流状态
    pub status: WorkflowStatus,
    /// 创建时间
    pub created_at: u64,
    /// 开始时间
    pub started_at: Option<u64>,
    /// 完成时间
    pub completed_at: Option<u64>,
    /// 工作流变量
    pub variables: HashMap<String, String>,
}

impl Workflow {
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            stages: Vec::new(),
            status: WorkflowStatus::Pending,
            created_at: 0,
            started_at: None,
            completed_at: None,
            variables: HashMap::new(),
        }
    }

    pub fn add_stage(&mut self, stage: Stage) {
        self.stages.push(stage);
    }

    pub fn set_variable(&mut self, key: String, value: String) {
        self.variables.insert(key, value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.variables.get(key)
    }

    /// 获取当前可执行的阶段
    pub fn get_current_stage(&mut self) -> Option<&mut Stage> {
        self.stages
            .iter_mut()
            .find(|s| s.status == StageStatus::Pending || s.status == StageStatus::Running)
    }

    /// 检查是否所有阶段都已完成
    pub fn is_complete(&self) -> bool {
        self.stages.iter().all(|s| {
            s.status == StageStatus::Completed || s.status == StageStatus::PartiallyCompleted
        })
    }

    /// 检查是否有阶段失败
    pub fn has_failed(&self) -> bool {
        self.stages.iter().any(|s| s.status == StageStatus::Failed)
    }
}

/// 工作流程执行上下文
pub struct WorkflowContext {
    /// 当前工作流
    pub workflow: Workflow,
    /// 角色切换器
    pub role_switcher: RoleSwitcher,
    /// 上下文优化器
    pub context_optimizer: ContextOptimizer,
    /// 执行历史
    pub execution_history: Vec<ExecutionRecord>,
    /// 是否启用详细日志
    pub verbose: bool,
}

/// 执行记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    /// 时间戳
    pub timestamp: u64,
    /// 阶段 ID
    pub stage_id: String,
    /// 步骤 ID
    pub step_id: String,
    /// 执行角色
    pub role: String,
    /// 执行状态
    pub status: String,
    /// 输出或错误信息
    pub message: String,
    /// 执行耗时（毫秒）
    pub duration_ms: Option<u64>,
}

/// 工作流程执行器
pub struct WorkflowEngine {
    /// 当前上下文
    context: WorkflowContext,
    /// 执行超时（秒）
    timeout_secs: u64,
    /// 是否在错误时停止
    stop_on_error: bool,
    /// 回调函数：步骤执行前
    on_before_step: Option<Box<dyn Fn(&Step) + Send + Sync>>,
    /// 回调函数：步骤执行后
    on_after_step: Option<Box<dyn Fn(&Step, &str) + Send + Sync>>,
    /// 回调函数：步骤失败
    on_step_error: Option<Box<dyn Fn(&Step, &str) + Send + Sync>>,
}

impl WorkflowEngine {
    /// 创建新的工作流程引擎
    pub fn new(workflow: Workflow) -> Self {
        let context = WorkflowContext {
            workflow,
            role_switcher: RoleSwitcher::new(),
            context_optimizer: ContextOptimizer::new(),
            execution_history: Vec::new(),
            verbose: false,
        };

        Self {
            context,
            timeout_secs: 300, // 默认 5 分钟超时
            stop_on_error: true,
            on_before_step: None,
            on_after_step: None,
            on_step_error: None,
        }
    }

    /// 设置超时时间
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// 设置是否在错误时停止
    pub fn with_stop_on_error(mut self, stop: bool) -> Self {
        self.stop_on_error = stop;
        self
    }

    /// 设置详细模式
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.context.verbose = verbose;
        self
    }

    /// 注册步骤执行前回调
    pub fn on_before_step<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Step) + Send + Sync + 'static,
    {
        self.on_before_step = Some(Box::new(callback));
        self
    }

    /// 注册步骤执行后回调
    pub fn on_after_step<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Step, &str) + Send + Sync + 'static,
    {
        self.on_after_step = Some(Box::new(callback));
        self
    }

    /// 注册步骤失败回调
    pub fn on_step_error<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Step, &str) + Send + Sync + 'static,
    {
        self.on_step_error = Some(Box::new(callback));
        self
    }

    /// 执行工作流
    pub fn execute(&mut self) -> Result<WorkflowResult> {
        let start_time = Instant::now();
        let timeout = Duration::from_secs(self.timeout_secs);

        self.context.workflow.status = WorkflowStatus::Running;
        self.context.workflow.started_at = Some(current_timestamp());

        self.log("开始执行工作流", &self.context.workflow.name);

        while !self.context.workflow.is_complete() && !self.context.workflow.has_failed() {
            // 检查超时
            if start_time.elapsed() > timeout {
                self.context.workflow.status = WorkflowStatus::Failed;
                return Err(anyhow::anyhow!("工作流执行超时"));
            }

            // 获取当前阶段的 ID
            let current_stage_id = self.context.workflow.stages
                .iter()
                .position(|s| s.status == StageStatus::Pending || s.status == StageStatus::Running);

            if current_stage_id.is_none() {
                break;
            }

            let stage_idx = current_stage_id.unwrap();

            // 执行阶段
            let stage_result = self.execute_stage_by_index(stage_idx);

            if let Err(e) = stage_result {
                if self.stop_on_error {
                    self.context.workflow.status = WorkflowStatus::Failed;
                    return Err(e);
                }
            }
        }

        self.context.workflow.completed_at = Some(current_timestamp());

        if self.context.workflow.has_failed() {
            self.context.workflow.status = WorkflowStatus::Failed;
        } else {
            self.context.workflow.status = WorkflowStatus::Completed;
        }

        Ok(WorkflowResult {
            workflow_id: self.context.workflow.id.clone(),
            status: self.context.workflow.status.clone(),
            total_duration_ms: start_time.elapsed().as_millis() as u64,
            stages_completed: self
                .context
                .workflow
                .stages
                .iter()
                .filter(|s| s.status == StageStatus::Completed)
                .count(),
            steps_completed: self
                .context
                .workflow
                .stages
                .iter()
                .flat_map(|s| s.steps.iter())
                .filter(|s| s.status == StepStatus::Completed)
                .count(),
            steps_failed: self
                .context
                .workflow
                .stages
                .iter()
                .flat_map(|s| s.steps.iter())
                .filter(|s| s.status == StepStatus::Failed)
                .count(),
            step_results: HashMap::new(),
            error: None,
            duration_ms: None,
        })
    }

    /// 执行单个阶段
    fn execute_stage(&mut self, stage: &mut Stage) -> Result<()> {
        self.log("开始执行阶段", &stage.name);
        stage.status = StageStatus::Running;

        // 获取可执行的步骤
        let ready_steps = stage.get_ready_steps();

        if ready_steps.is_empty() {
            // 没有可执行的步骤，检查是否已完成
            if stage.is_complete() {
                stage.status = StageStatus::Completed;
                self.log("阶段完成", &stage.name);
                return Ok(());
            } else if stage.has_failed() {
                stage.status = StageStatus::Failed;
                return Err(anyhow::anyhow!("阶段 {} 执行失败", stage.name));
            } else {
                // 存在无法执行的步骤（依赖未满足）
                stage.status = StageStatus::PartiallyCompleted;
                return Ok(());
            }
        }

        // 执行步骤
        for step in ready_steps {
            let step_result = self.execute_step(step);

            if let Err(e) = step_result {
                if self.stop_on_error {
                    stage.status = StageStatus::Failed;
                    return Err(e);
                }
            }
        }

        // 检查阶段完成状态
        if stage.has_failed() {
            stage.status = StageStatus::Failed;
        } else if stage.is_complete() {
            stage.status = StageStatus::Completed;
        } else {
            stage.status = StageStatus::PartiallyCompleted;
        }

        Ok(())
    }

    /// 通过索引执行阶段
    fn execute_stage_by_index(&mut self, idx: usize) -> Result<()> {
        if idx >= self.context.workflow.stages.len() {
            return Err(anyhow::anyhow!("无效的阶段索引"));
        }

        let stage_name = self.context.workflow.stages[idx].name.clone();
        self.log("开始执行阶段", &stage_name);
        self.context.workflow.stages[idx].status = StageStatus::Running;

        // 收集可执行的步骤 ID
        let step_ids: Vec<String> = {
            let stage = &self.context.workflow.stages[idx];
            stage.get_ready_steps().iter().map(|s| s.id.clone()).collect()
        };

        if step_ids.is_empty() {
            // 没有可执行的步骤，检查是否已完成
            if self.context.workflow.stages[idx].is_complete() {
                self.context.workflow.stages[idx].status = StageStatus::Completed;
                self.log("阶段完成", &stage_name);
                return Ok(());
            } else if self.context.workflow.stages[idx].has_failed() {
                self.context.workflow.stages[idx].status = StageStatus::Failed;
                return Err(anyhow::anyhow!("阶段 {} 执行失败", stage_name));
            } else {
                // 存在无法执行的步骤（依赖未满足）
                self.context.workflow.stages[idx].status = StageStatus::PartiallyCompleted;
                return Ok(());
            }
        }

        // 执行步骤
        for step_id in step_ids {
            let step_result = self.execute_step_by_id(&step_id);

            if let Err(e) = step_result {
                if self.stop_on_error {
                    self.context.workflow.stages[idx].status = StageStatus::Failed;
                    return Err(e);
                }
            }
        }

        // 检查阶段完成状态
        if self.context.workflow.stages[idx].has_failed() {
            self.context.workflow.stages[idx].status = StageStatus::Failed;
        } else if self.context.workflow.stages[idx].is_complete() {
            self.context.workflow.stages[idx].status = StageStatus::Completed;
        } else {
            self.context.workflow.stages[idx].status = StageStatus::PartiallyCompleted;
        }

        Ok(())
    }

    /// 执行单个步骤
    fn execute_step(&mut self, step: &Step) -> Result<()> {
        // 克隆步骤信息用于回调
        let step_id = step.id.clone();
        let step_desc = step.description.clone();
        let step_role = step.role.clone();

        self.log("执行步骤", &format!("{}: {}", step_id, step_desc));

        // 调用 before 回调
        if let Some(callback) = &self.on_before_step {
            callback(step);
        }

        // 切换角色
        self.context.role_switcher.set_role(step_role.clone());

        // 记录到上下文
        self.context
            .context_optimizer
            .add_message(ContextMessage::with_importance(
                MessageType::System,
                format!("[{}] {}", step_role.as_str(), step_desc),
                7,
            ));

        let start = Instant::now();

        // 模拟执行（实际使用时应该调用 AI 或具体逻辑）
        let output = self.simulate_execute(step)?;

        let duration_ms = start.elapsed().as_millis() as u64;

        // 调用 after 回调
        if let Some(callback) = &self.on_after_step {
            callback(step, &output);
        }

        // 记录执行历史
        self.context.execution_history.push(ExecutionRecord {
            timestamp: current_timestamp(),
            stage_id: String::new(), // TODO: 填入阶段 ID
            step_id: step_id.clone(),
            role: step_role.as_str().to_string(),
            status: "completed".to_string(),
            message: output.clone(),
            duration_ms: Some(duration_ms),
        });

        // 更新步骤状态
        if let Some(step_mut) = self.get_step_mut(&step_id) {
            step_mut.status = StepStatus::Completed;
            step_mut.output = Some(output);
            step_mut.duration_ms = Some(duration_ms);
        }

        Ok(())
    }

    /// 通过 ID 执行步骤
    fn execute_step_by_id(&mut self, step_id: &str) -> Result<()> {
        // 克隆步骤信息
        let (step_desc, step_role) = {
            let mut step_desc = String::new();
            let mut step_role = AgentRole::General;
            
            for stage in &self.context.workflow.stages {
                if let Some(step) = stage.steps.iter().find(|s| s.id == step_id) {
                    step_desc = step.description.clone();
                    step_role = step.role.clone();
                    break;
                }
            }
            
            if step_desc.is_empty() {
                return Err(anyhow::anyhow!("步骤 {} 不存在", step_id));
            }
            
            (step_desc, step_role)
        };

        self.log("执行步骤", &format!("{}: {}", step_id, step_desc));

        // 切换角色
        self.context.role_switcher.set_role(step_role.clone());

        // 记录到上下文
        self.context
            .context_optimizer
            .add_message(ContextMessage::with_importance(
                MessageType::System,
                format!("[{}] {}", step_role.as_str(), step_desc),
                7,
            ));

        let start = Instant::now();

        // 模拟执行
        let output = format!("[模拟执行] 步骤 {} 由 {} 完成", step_desc, step_role.as_str());

        let duration_ms = start.elapsed().as_millis() as u64;

        // 记录执行历史
        self.context.execution_history.push(ExecutionRecord {
            timestamp: current_timestamp(),
            stage_id: String::new(),
            step_id: step_id.to_string(),
            role: step_role.as_str().to_string(),
            status: "completed".to_string(),
            message: output.clone(),
            duration_ms: Some(duration_ms),
        });

        // 更新步骤状态
        if let Some(step_mut) = self.get_step_mut(step_id) {
            step_mut.status = StepStatus::Completed;
            step_mut.output = Some(output);
            step_mut.duration_ms = Some(duration_ms);
        }

        Ok(())
    }

    /// 模拟执行步骤（实际使用时替换为真实逻辑）
    fn simulate_execute(&self, step: &Step) -> Result<String> {
        // 这里应该调用 AI 服务或执行具体任务
        // 现在只是模拟返回
        Ok(format!(
            "[模拟执行] 步骤 {} 由 {} 完成，输入：{:?}",
            step.description,
            step.role.as_str(),
            step.input
        ))
    }

    /// 获取步骤的可变引用
    fn get_step_mut(&mut self, step_id: &str) -> Option<&mut Step> {
        for stage in &mut self.context.workflow.stages {
            if let Some(step) = stage.steps.iter_mut().find(|s| s.id == step_id) {
                return Some(step);
            }
        }
        None
    }

    /// 暂停执行（等待用户确认）
    pub fn pause(&mut self) {
        self.context.workflow.status = WorkflowStatus::Paused;
        self.log("工作流已暂停", "");
    }
    pub fn cancel(&mut self) {
        self.context.workflow.status = WorkflowStatus::Cancelled;
        self.log("工作流已取消", "");
    }

    /// 获取当前状态
    pub fn get_status(&self) -> &WorkflowStatus {
        &self.context.workflow.status
    }

    /// 获取工作流
    pub fn get_workflow(&self) -> &Workflow {
        &self.context.workflow
    }

    /// 获取执行历史
    pub fn get_execution_history(&self) -> &[ExecutionRecord] {
        &self.context.execution_history
    }

    /// 日志输出
    fn log(&self, action: &str, detail: &str) {
        if self.context.verbose {
            println!("[Workflow] {}: {}", action, detail);
        }
    }
}

// ============================================================================
// WorkflowEngine 扩展 - 声明式工作流支持
// ============================================================================

impl WorkflowEngine {
    /// 执行声明式工作流
    pub async fn execute_declarative(
        &mut self,
        workflow: &DeclarativeWorkflow,
        input: &Value,
    ) -> Result<WorkflowResult> {
        let start_time = Instant::now();

        self.log("执行声明式工作流", &workflow.name);

        let mut step_results: HashMap<String, Value> = HashMap::new();
        let mut executed_steps: std::collections::HashSet<String> = std::collections::HashSet::new();

        // 执行步骤（按依赖顺序）
        loop {
            // 检查是否所有步骤都已执行
            if executed_steps.len() >= workflow.steps.len() {
                break;
            }

            // 获取可执行的步骤（依赖已满足）
            let ready_steps: Vec<&DeclarativeWorkflowStep> = workflow
                .steps
                .iter()
                .filter(|step| {
                    !executed_steps.contains(&step.id)
                        && step
                            .depends_on
                            .iter()
                            .all(|dep| executed_steps.contains(dep))
                })
                .collect();

            if ready_steps.is_empty() {
                // 没有可执行的步骤，但还有未执行的步骤，说明存在循环依赖
                if executed_steps.len() < workflow.steps.len() {
                    return Err(anyhow::anyhow!(
                        "检测到循环依赖或无法执行的步骤"
                    ));
                }
                break;
            }

            // 并行执行可执行的步骤
            for step in ready_steps {
                match self.execute_step_with_retry(step, &step_results, input).await {
                    Ok(result) => {
                        step_results.insert(step.id.clone(), result);
                        executed_steps.insert(step.id.clone());
                    }
                    Err(e) => {
                        // 错误处理
                        match &step.on_error {
                            Some(handler) => match handler.strategy {
                                ErrorStrategy::Skip => {
                                    self.log("跳过步骤", &format!("{}: {}", step.id, e));
                                    executed_steps.insert(step.id.clone());
                                }
                                ErrorStrategy::Fail => {
                                    return Ok(WorkflowResult::failure(
                                        workflow.id.clone(),
                                        format!("步骤 {} 执行失败：{}", step.id, e),
                                    ));
                                }
                                ErrorStrategy::Retry => {
                                    // 已经重试过了，还是失败
                                    return Ok(WorkflowResult::failure(
                                        workflow.id.clone(),
                                        format!("步骤 {} 重试后仍失败：{}", step.id, e),
                                    ));
                                }
                                ErrorStrategy::Fallback => {
                                    // TODO: 执行 fallback 工具
                                    return Ok(WorkflowResult::failure(
                                        workflow.id.clone(),
                                        format!("步骤 {} 执行失败，fallback 未实现：{}", step.id, e),
                                    ));
                                }
                            },
                            None => {
                                if self.stop_on_error {
                                    return Ok(WorkflowResult::failure(
                                        workflow.id.clone(),
                                        format!("步骤 {} 执行失败：{}", step.id, e),
                                    ));
                                } else {
                                    executed_steps.insert(step.id.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(WorkflowResult {
            workflow_id: workflow.id.clone(),
            step_results,
            status: WorkflowStatus::Completed,
            error: None,
            duration_ms: Some(duration_ms),
            total_duration_ms: duration_ms,
            stages_completed: 0,
            steps_completed: executed_steps.len(),
            steps_failed: 0,
        })
    }

    /// 执行单个步骤（带重试）
    async fn execute_step_with_retry(
        &self,
        step: &DeclarativeWorkflowStep,
        _step_results: &HashMap<String, Value>,
        _input: &Value,
    ) -> Result<Value> {
        let mut attempts = 0;
        let mut delay = step.retry.retry_interval_ms;

        loop {
            match self.execute_single_step(step).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts >= step.retry.max_retries {
                        return Err(e);
                    }

                    // 等待重试
                    tokio::time::sleep(Duration::from_millis(delay)).await;

                    // 指数退避
                    if step.retry.exponential_backoff {
                        delay *= 2;
                    }

                    self.log("重试步骤", &format!("{} (第 {}/{} 次)", step.id, attempts, step.retry.max_retries));
                }
            }
        }
    }

    /// 执行单个步骤
    async fn execute_single_step(&self, step: &DeclarativeWorkflowStep) -> Result<Value> {
        // 设置超时
        let timeout = step.timeout_secs.or(Some(self.timeout_secs)).unwrap_or(60);

        let result = tokio::time::timeout(
            Duration::from_secs(timeout),
            self.do_execute_step(step),
        )
        .await
        .map_err(|_| anyhow::anyhow!("步骤 {} 执行超时 ({}s)", step.id, timeout))??;

        Ok(result)
    }

    /// 实际执行步骤（调用工具）
    async fn do_execute_step(&self, step: &DeclarativeWorkflowStep) -> Result<Value> {
        // TODO: 实际调用工具矩阵执行工具
        // 这里返回模拟结果
        self.log("执行步骤", &format!("{}: 调用工具 {}", step.id, step.tool));

        Ok(json!({
            "tool": step.tool,
            "status": "simulated",
            "message": format!("步骤 {} 执行完成", step.id)
        }))
    }
}

/// 工作流执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    /// 工作流 ID
    pub workflow_id: String,
    /// 最终状态
    pub status: WorkflowStatus,
    /// 总执行时间（毫秒）
    pub total_duration_ms: u64,
    /// 完成的阶段数
    pub stages_completed: usize,
    /// 完成的步骤数
    pub steps_completed: usize,
    /// 失败的步骤数
    pub steps_failed: usize,
    /// 步骤执行结果（声明式工作流使用）
    #[serde(default)]
    pub step_results: HashMap<String, Value>,
    /// 错误信息
    #[serde(default)]
    pub error: Option<String>,
    /// 执行耗时（毫秒）
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

impl WorkflowResult {
    /// 创建成功结果
    pub fn success(workflow_id: String, step_results: HashMap<String, Value>) -> Self {
        Self {
            workflow_id,
            step_results,
            status: WorkflowStatus::Completed,
            error: None,
            duration_ms: None,
            total_duration_ms: 0,
            stages_completed: 0,
            steps_completed: 0,
            steps_failed: 0,
        }
    }

    /// 创建失败结果
    pub fn failure(workflow_id: String, error: String) -> Self {
        Self {
            workflow_id,
            step_results: HashMap::new(),
            status: WorkflowStatus::Failed,
            error: Some(error),
            duration_ms: None,
            total_duration_ms: 0,
            stages_completed: 0,
            steps_completed: 0,
            steps_failed: 0,
        }
    }
}

/// 获取当前时间戳（秒）
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 预定义的工作流模板
pub mod templates {
    use super::*;

    /// 创建代码审查工作流
    pub fn create_code_review_workflow() -> Workflow {
        let mut workflow = Workflow::new(
            "code_review".to_string(),
            "代码审查工作流".to_string(),
            "自动执行代码审查流程".to_string(),
        );

        // 阶段 1: 准备
        let mut prepare_stage = Stage::new(
            "prepare".to_string(),
            "准备阶段".to_string(),
            "准备代码审查所需的信息".to_string(),
        );

        prepare_stage.add_step(Step::new(
            "analyze_changes".to_string(),
            "分析代码变更".to_string(),
            AgentRole::Reviewer,
        ));

        prepare_stage.add_step(Step::new(
            "load_context".to_string(),
            "加载项目上下文".to_string(),
            AgentRole::Executor,
        ));

        workflow.add_stage(prepare_stage);

        // 阶段 2: 审查
        let mut review_stage = Stage::new(
            "review".to_string(),
            "审查阶段".to_string(),
            "执行代码审查".to_string(),
        );

        review_stage.add_step(Step::new(
            "check_style".to_string(),
            "检查代码风格".to_string(),
            AgentRole::Reviewer,
        ));

        review_stage.add_step(Step::new(
            "check_logic".to_string(),
            "检查逻辑正确性".to_string(),
            AgentRole::Reviewer,
        ));

        review_stage.add_step(Step::new(
            "check_performance".to_string(),
            "检查性能问题".to_string(),
            AgentRole::Reviewer,
        ));

        workflow.add_stage(review_stage);

        // 阶段 3: 报告
        let mut report_stage = Stage::new(
            "report".to_string(),
            "报告阶段".to_string(),
            "生成审查报告".to_string(),
        );

        report_stage.add_step(Step::new(
            "generate_report".to_string(),
            "生成审查报告".to_string(),
            AgentRole::Reviewer,
        ));

        workflow.add_stage(report_stage);

        workflow
    }

    /// 创建任务分解工作流
    pub fn create_task_decomposition_workflow() -> Workflow {
        let mut workflow = Workflow::new(
            "task_decomposition".to_string(),
            "任务分解工作流".to_string(),
            "将复杂任务分解为可执行的小步骤".to_string(),
        );

        // 阶段 1: 理解任务
        let mut understand_stage = Stage::new(
            "understand".to_string(),
            "理解任务".to_string(),
            "分析和理解用户请求".to_string(),
        );

        understand_stage.add_step(Step::new(
            "parse_request".to_string(),
            "解析用户请求".to_string(),
            AgentRole::Planner,
        ));

        understand_stage.add_step(Step::new(
            "identify_goals".to_string(),
            "识别核心目标".to_string(),
            AgentRole::Planner,
        ));

        workflow.add_stage(understand_stage);

        // 阶段 2: 分解任务
        let mut decompose_stage = Stage::new(
            "decompose".to_string(),
            "分解任务".to_string(),
            "将任务分解为可执行的步骤".to_string(),
        );

        decompose_stage.add_step(Step::new(
            "break_down".to_string(),
            "拆解为子任务".to_string(),
            AgentRole::Planner,
        ));

        decompose_stage.add_step(Step::new(
            "define_dependencies".to_string(),
            "定义依赖关系".to_string(),
            AgentRole::Planner,
        ));

        workflow.add_stage(decompose_stage);

        // 阶段 3: 输出计划
        let mut plan_stage = Stage::new(
            "output".to_string(),
            "输出计划".to_string(),
            "生成最终执行计划".to_string(),
        );

        plan_stage.add_step(Step::new(
            "generate_plan".to_string(),
            "生成执行计划".to_string(),
            AgentRole::Planner,
        ));

        workflow.add_stage(plan_stage);

        workflow
    }
}

// ============================================================================
// 声明式工作流定义（服务化架构）
// ============================================================================

/// 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// 最大重试次数
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    #[serde(default = "default_retry_interval")]
    pub retry_interval_ms: u64,
    /// 是否指数退避
    #[serde(default)]
    pub exponential_backoff: bool,
}

fn default_max_retries() -> u32 { 3 }
fn default_retry_interval() -> u64 { 1000 }

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_interval_ms: default_retry_interval(),
            exponential_backoff: true,
        }
    }
}

/// 错误处理策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategy {
    /// 重试
    Retry,
    /// 跳过
    Skip,
    /// 失败
    Fail,
    /// 使用 fallback 工具
    Fallback,
}

impl Default for ErrorStrategy {
    fn default() -> Self {
        Self::Fail
    }
}

/// 错误处理器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ErrorHandler {
    /// 错误处理策略
    #[serde(default)]
    pub strategy: ErrorStrategy,
    /// fallback 工具（可选）
    pub fallback_tool: Option<String>,
    /// 最大错误数（超过则终止工作流）
    pub max_errors: Option<u32>,
}

/// 声明式工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeWorkflowStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤描述
    pub description: String,
    /// 使用的工具
    pub tool: String,
    /// 工具参数（支持模板）
    pub arguments: Value,
    /// 前置步骤依赖
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 重试配置
    #[serde(default)]
    pub retry: RetryConfig,
    /// 超时配置（秒）
    pub timeout_secs: Option<u64>,
    /// 错误处理
    #[serde(default)]
    pub on_error: Option<ErrorHandler>,
    /// 执行角色
    #[serde(default = "default_executor_role")]
    pub role: AgentRole,
}

fn default_executor_role() -> AgentRole {
    AgentRole::Executor
}

impl DeclarativeWorkflowStep {
    /// 创建新的步骤
    pub fn new(id: String, tool: String, arguments: Value) -> Self {
        Self {
            id,
            description: String::new(),
            tool,
            arguments,
            depends_on: Vec::new(),
            retry: RetryConfig::default(),
            timeout_secs: None,
            on_error: None,
            role: AgentRole::Executor,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// 设置依赖
    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }

    /// 设置重试配置
    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    /// 设置超时
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// 设置错误处理
    pub fn with_error_handler(mut self, handler: ErrorHandler) -> Self {
        self.on_error = Some(handler);
        self
    }

    /// 设置角色
    pub fn with_role(mut self, role: AgentRole) -> Self {
        self.role = role;
        self
    }
}

/// 声明式工作流定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclarativeWorkflow {
    /// 工作流 ID
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 工作流版本
    #[serde(default = "default_workflow_version")]
    pub version: String,
    /// 工作流步骤
    pub steps: Vec<DeclarativeWorkflowStep>,
    /// 工作流变量
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// 全局超时（秒）
    pub timeout_secs: Option<u64>,
    /// 全局错误处理
    #[serde(default)]
    pub on_error: Option<ErrorHandler>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_workflow_version() -> String {
    "1.0.0".to_string()
}

impl DeclarativeWorkflow {
    /// 创建新的声明式工作流
    pub fn new(id: String, name: String, description: String) -> Self {
        Self {
            id,
            name,
            description,
            version: default_workflow_version(),
            steps: Vec::new(),
            variables: HashMap::new(),
            timeout_secs: None,
            on_error: None,
            tags: Vec::new(),
        }
    }

    /// 添加步骤
    pub fn add_step(mut self, step: DeclarativeWorkflowStep) -> Self {
        self.steps.push(step);
        self
    }

    /// 设置变量
    pub fn with_variable(mut self, key: String, value: String) -> Self {
        self.variables.insert(key, value);
        self
    }

    /// 设置全局超时
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    /// 设置全局错误处理
    pub fn with_error_handler(mut self, handler: ErrorHandler) -> Self {
        self.on_error = Some(handler);
        self
    }

    /// 添加标签
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// 转换为传统 Workflow
    pub fn to_workflow(&self) -> Workflow {
        let mut workflow = Workflow::new(
            self.id.clone(),
            self.name.clone(),
            self.description.clone(),
        );

        // 创建一个包含所有步骤的阶段
        let mut stage = Stage::new(
            "default".to_string(),
            "执行阶段".to_string(),
            "执行声明式工作流步骤".to_string(),
        );

        for step in &self.steps {
            let mut wf_step = Step::new(
                step.id.clone(),
                step.description.clone(),
                step.role.clone(),
            );
            wf_step.dependencies = step.depends_on.clone();
            stage.add_step(wf_step);
        }

        workflow.add_stage(stage);
        workflow
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let mut workflow = Workflow::new(
            "test".to_string(),
            "测试工作流".to_string(),
            "用于测试".to_string(),
        );

        let mut stage = Stage::new(
            "stage1".to_string(),
            "第一阶段".to_string(),
            "描述".to_string(),
        );

        stage.add_step(Step::new(
            "step1".to_string(),
            "步骤 1".to_string(),
            AgentRole::Executor,
        ));

        workflow.add_stage(stage);

        assert_eq!(workflow.stages.len(), 1);
        assert_eq!(workflow.stages[0].steps.len(), 1);
    }

    #[test]
    fn test_workflow_engine() {
        let workflow = templates::create_code_review_workflow();
        let mut engine = WorkflowEngine::new(workflow);

        let result = engine.with_verbose(true).execute();

        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.status, WorkflowStatus::Completed);
        assert!(result.steps_completed > 0);
    }

    #[test]
    fn test_step_dependencies() {
        let mut step1 = Step::new("step1".to_string(), "步骤 1".to_string(), AgentRole::Executor);
        step1.status = StepStatus::Completed;

        let step2 = Step::with_dependencies(
            "step2".to_string(),
            "步骤 2".to_string(),
            AgentRole::Executor,
            vec!["step1".to_string()],
        );

        let mut stage = Stage::new(
            "stage1".to_string(),
            "阶段 1".to_string(),
            "描述".to_string(),
        );
        stage.steps.push(step1);
        stage.steps.push(step2);

        let ready_steps = stage.get_ready_steps();
        assert_eq!(ready_steps.len(), 1);
        assert_eq!(ready_steps[0].id, "step2");
    }
}
