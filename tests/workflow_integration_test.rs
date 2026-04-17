//! 工作流引擎集成测试
//!
//! 测试声明式工作流的完整执行流程

use ai_assistant::{
    AgentRole, DeclarativeWorkflow, DeclarativeWorkflowStep, ErrorHandler, ErrorStrategy,
    RetryConfig, Stage, StageStatus, Step, StepStatus, Workflow, WorkflowEngine, WorkflowStatus,
};
use serde_json::json;

/// 测试基本工作流执行
#[test]
fn test_basic_workflow_execution() {
    // 创建一个简单的工作流
    let mut workflow = Workflow::new(
        "test_workflow".to_string(),
        "Test Workflow".to_string(),
        "A simple test workflow".to_string(),
    );

    // 添加第一阶段
    let mut stage1 = Stage::new(
        "stage1".to_string(),
        "分析项目结构".to_string(),
        "分析项目的目录结构和代码组织".to_string(),
    );
    stage1.add_step(Step::new(
        "step1".to_string(),
        "读取项目目录".to_string(),
        AgentRole::Planner,
    ));
    stage1.add_step(Step::new(
        "step2".to_string(),
        "分析代码结构".to_string(),
        AgentRole::Executor,
    ));
    workflow.add_stage(stage1);

    // 添加第二阶段
    let mut stage2 = Stage::new(
        "stage2".to_string(),
        "生成报告".to_string(),
        "生成项目分析报告".to_string(),
    );
    stage2.add_step(Step::new(
        "step3".to_string(),
        "生成分析报告".to_string(),
        AgentRole::Reviewer,
    ));
    workflow.add_stage(stage2);

    // 创建工作流引擎并执行
    let mut engine = WorkflowEngine::new(workflow);
    let result = engine.execute();

    assert!(result.is_ok(), "工作流执行失败：{:?}", result.err());

    let workflow_result = result.unwrap();
    assert_eq!(workflow_result.status, WorkflowStatus::Completed);
    assert!(
        workflow_result.steps_completed > 0,
        "应该至少执行了一个步骤"
    );
}

/// 测试工作流变量传递
#[test]
fn test_workflow_variables() {
    let mut workflow = Workflow::new(
        "test_variables".to_string(),
        "Test Variables".to_string(),
        "A workflow with variables".to_string(),
    );

    // 设置变量
    workflow.set_variable("project_name".to_string(), "test-project".to_string());
    workflow.set_variable("version".to_string(), "1.0.0".to_string());

    // 验证变量
    assert_eq!(
        workflow.get_variable("project_name"),
        Some(&"test-project".to_string())
    );
    assert_eq!(workflow.get_variable("version"), Some(&"1.0.0".to_string()));
    assert_eq!(workflow.get_variable("nonexistent"), None);
}

/// 测试声明式工作流
#[test]
fn test_declarative_workflow_structure() {
    let mut workflow = DeclarativeWorkflow::new(
        "test_declarative".to_string(),
        "Test Declarative Workflow".to_string(),
        "A declarative test workflow".to_string(),
    );

    // 添加步骤
    workflow.steps = vec![
        DeclarativeWorkflowStep {
            id: "step1".to_string(),
            description: "第一步".to_string(),
            tool: "read_file".to_string(),
            arguments: json!({"path": "README.md"}),
            depends_on: vec![],
            retry: RetryConfig::default(),
            timeout_secs: Some(30),
            on_error: None,
            role: AgentRole::Executor,
        },
        DeclarativeWorkflowStep {
            id: "step2".to_string(),
            description: "第二步".to_string(),
            tool: "analyze_code".to_string(),
            arguments: json!({"path": "src/main.rs"}),
            depends_on: vec!["step1".to_string()],
            retry: RetryConfig::default(),
            timeout_secs: Some(60),
            on_error: Some(ErrorHandler {
                strategy: ErrorStrategy::Skip,
                fallback_tool: None,
                max_errors: Some(3),
            }),
            role: AgentRole::Reviewer,
        },
    ];

    // 验证工作流结构
    assert_eq!(workflow.steps.len(), 2);
    assert_eq!(workflow.steps[0].tool, "read_file");
    assert_eq!(workflow.steps[1].tool, "analyze_code");
    assert_eq!(workflow.steps[1].depends_on, vec!["step1".to_string()]);
}

/// 测试工作流超时配置
#[test]
fn test_workflow_timeout_config() {
    let mut workflow = Workflow::new(
        "test_timeout".to_string(),
        "Test Timeout".to_string(),
        "A workflow with timeout".to_string(),
    );

    let mut stage1 = Stage::new(
        "stage1".to_string(),
        "测试阶段".to_string(),
        "测试超时配置".to_string(),
    );
    stage1.add_step(Step::new(
        "step1".to_string(),
        "快速步骤".to_string(),
        AgentRole::Executor,
    ));
    workflow.add_stage(stage1);

    // 设置超时
    let engine = WorkflowEngine::new(workflow).with_timeout(300); // 5 分钟超时

    // 验证超时已设置（通过编译检查）
    drop(engine);
}

/// 测试工作流错误处理配置
#[test]
fn test_workflow_error_handling_config() {
    // 测试 ErrorHandler 的创建
    let handler_skip = ErrorHandler {
        strategy: ErrorStrategy::Skip,
        fallback_tool: None,
        max_errors: None,
    };
    assert_eq!(handler_skip.strategy, ErrorStrategy::Skip);

    let handler_fallback = ErrorHandler {
        strategy: ErrorStrategy::Fallback,
        fallback_tool: Some("backup_tool".to_string()),
        max_errors: Some(3),
    };
    assert_eq!(handler_fallback.strategy, ErrorStrategy::Fallback);
    assert_eq!(
        handler_fallback.fallback_tool,
        Some("backup_tool".to_string())
    );
    assert_eq!(handler_fallback.max_errors, Some(3));
}

/// 测试 Stage 功能
#[test]
fn test_stage_operations() {
    let mut stage = Stage::new(
        "test_stage".to_string(),
        "测试阶段".to_string(),
        "测试 Stage 操作".to_string(),
    );

    // 初始状态应该是 Pending
    assert_eq!(stage.status, StageStatus::Pending);
    assert!(stage.steps.is_empty());

    // 添加步骤
    stage.add_step(Step::new(
        "step1".to_string(),
        "步骤 1".to_string(),
        AgentRole::Executor,
    ));
    stage.add_step(Step::new(
        "step2".to_string(),
        "步骤 2".to_string(),
        AgentRole::Reviewer,
    ));

    assert_eq!(stage.steps.len(), 2);
    assert_eq!(stage.steps[0].id, "step1");
    assert_eq!(stage.steps[1].id, "step2");
}

/// 测试 Step 创建
#[test]
fn test_step_creation() {
    let step = Step::new(
        "test_step".to_string(),
        "测试步骤".to_string(),
        AgentRole::Planner,
    );

    assert_eq!(step.id, "test_step");
    assert_eq!(step.description, "测试步骤");
    assert_eq!(step.role, AgentRole::Planner);
    assert_eq!(step.status, StepStatus::Pending);
    assert!(step.dependencies.is_empty());
}
