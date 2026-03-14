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
