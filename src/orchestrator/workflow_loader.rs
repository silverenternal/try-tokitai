//! TOML 工作流加载器
//!
//! 从 TOML 文件加载声明式工作流定义
//!
//! ## 使用示例
//! ```rust
//! use crate::orchestrator::workflow_loader::WorkflowLoader;
//!
//! // 从文件加载工作流
//! let workflow = WorkflowLoader::load_from_file("workflows/code_review.toml")?;

#![allow(dead_code)]
//!
//! // 从字符串加载工作流
//! let toml_str = std::fs::read_to_string("workflows/code_review.toml")?;
//! let workflow = WorkflowLoader::load_from_str(&toml_str)?;
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::orchestrator::{
    workflow::{
        DeclarativeWorkflow, DeclarativeWorkflowStep, ErrorHandler, ErrorStrategy, RetryConfig,
    },
    AgentRole,
};

/// TOML 工作流定义（中间格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlWorkflow {
    /// 工作流配置
    pub workflow: TomlWorkflowConfig,
}

/// TOML 工作流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlWorkflowConfig {
    /// 工作流 ID
    pub id: String,
    /// 工作流名称
    pub name: String,
    /// 工作流描述
    pub description: String,
    /// 工作流版本
    #[serde(default = "default_version")]
    pub version: String,
    /// 全局超时（秒）
    pub timeout_secs: Option<u64>,
    /// 全局错误处理
    #[serde(default)]
    pub on_error: Option<TomlErrorHandler>,
    /// 工作流变量
    #[serde(default)]
    pub variables: HashMap<String, String>,
    /// 工作流步骤
    #[serde(default)]
    pub steps: Vec<TomlWorkflowStep>,
    /// 标签
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// TOML 工作流步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlWorkflowStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤描述
    pub description: String,
    /// 使用的工具
    pub tool: String,
    /// 工具参数
    #[serde(default)]
    pub arguments: HashMap<String, Value>,
    /// 前置步骤依赖
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// 重试配置
    #[serde(default)]
    pub retry: TomlRetryConfig,
    /// 超时配置（秒）
    pub timeout_secs: Option<u64>,
    /// 错误处理
    #[serde(default)]
    pub on_error: Option<TomlErrorHandler>,
    /// 执行角色
    #[serde(default = "default_role")]
    pub role: String,
}

fn default_role() -> String {
    "executor".to_string()
}

/// TOML 重试配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlRetryConfig {
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

fn default_max_retries() -> u32 {
    3
}
fn default_retry_interval() -> u64 {
    1000
}

impl Default for TomlRetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            retry_interval_ms: default_retry_interval(),
            exponential_backoff: true,
        }
    }
}

/// TOML 错误处理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlErrorHandler {
    /// 错误处理策略
    #[serde(default)]
    pub strategy: String,
    /// fallback 工具（可选）
    pub fallback_tool: Option<String>,
    /// 最大错误数
    pub max_errors: Option<u32>,
}

impl Default for TomlErrorHandler {
    fn default() -> Self {
        Self {
            strategy: "fail".to_string(),
            fallback_tool: None,
            max_errors: None,
        }
    }
}

/// 工作流加载器
pub struct WorkflowLoader;

impl WorkflowLoader {
    /// 从文件加载工作流
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<DeclarativeWorkflow> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .with_context(|| format!("读取工作流文件失败：{:?}", path))?;

        Self::load_from_str(&content)
    }

    /// 从字符串加载工作流
    pub fn load_from_str(content: &str) -> Result<DeclarativeWorkflow> {
        let toml_workflow: TomlWorkflow = toml::from_str(content)
            .with_context(|| "解析 TOML 工作流失败")?;

        Ok(Self::convert_to_declarative(toml_workflow))
    }

    /// 从目录加载所有工作流
    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Vec<DeclarativeWorkflow>> {
        let dir = dir.as_ref();
        let mut workflows = Vec::new();

        if !dir.exists() {
            return Ok(workflows);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "toml") {
                match Self::load_from_file(&path) {
                    Ok(workflow) => workflows.push(workflow),
                    Err(e) => tracing::warn!("加载工作流 {:?} 失败：{}", path, e),
                }
            }
        }

        Ok(workflows)
    }

    /// 转换为 DeclarativeWorkflow
    fn convert_to_declarative(toml: TomlWorkflow) -> DeclarativeWorkflow {
        let config = toml.workflow;

        let mut workflow = DeclarativeWorkflow::new(
            config.id.clone(),
            config.name.clone(),
            config.description.clone(),
        )
        .with_timeout(config.timeout_secs.unwrap_or(300))
        .with_tag(config.version.clone());

        // 添加变量
        for (key, value) in config.variables {
            workflow = workflow.with_variable(key, value);
        }

        // 添加全局错误处理
        if let Some(on_error) = config.on_error {
            workflow = workflow.with_error_handler(Self::convert_error_handler(on_error));
        }

        // 添加标签
        for tag in config.tags {
            workflow = workflow.with_tag(tag);
        }

        // 添加步骤
        for step in config.steps {
            workflow = workflow.add_step(Self::convert_step(step));
        }

        workflow
    }

    /// 转换步骤
    fn convert_step(toml_step: TomlWorkflowStep) -> DeclarativeWorkflowStep {
        let role = match toml_step.role.to_lowercase().as_str() {
            "planner" => AgentRole::Planner,
            "executor" => AgentRole::Executor,
            "reviewer" => AgentRole::Reviewer,
            "researcher" => AgentRole::Researcher,
            _ => AgentRole::Executor,
        };

        let mut step = DeclarativeWorkflowStep::new(
            toml_step.id,
            toml_step.tool,
            serde_json::to_value(toml_step.arguments).unwrap_or_default(),
        )
        .with_description(toml_step.description)
        .with_dependencies(toml_step.depends_on)
        .with_retry(Self::convert_retry(toml_step.retry))
        .with_role(role);

        if let Some(timeout) = toml_step.timeout_secs {
            step = step.with_timeout(timeout);
        }

        if let Some(on_error) = toml_step.on_error {
            step = step.with_error_handler(Self::convert_error_handler(on_error));
        }

        step
    }

    /// 转换重试配置
    fn convert_retry(toml_retry: TomlRetryConfig) -> RetryConfig {
        RetryConfig {
            max_retries: toml_retry.max_retries,
            retry_interval_ms: toml_retry.retry_interval_ms,
            exponential_backoff: toml_retry.exponential_backoff,
        }
    }

    /// 转换错误处理器
    fn convert_error_handler(toml_handler: TomlErrorHandler) -> ErrorHandler {
        let strategy = match toml_handler.strategy.to_lowercase().as_str() {
            "retry" => ErrorStrategy::Retry,
            "skip" => ErrorStrategy::Skip,
            "fail" => ErrorStrategy::Fail,
            "fallback" => ErrorStrategy::Fallback,
            _ => ErrorStrategy::Fail,
        };

        ErrorHandler {
            strategy,
            fallback_tool: toml_handler.fallback_tool,
            max_errors: toml_handler.max_errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_workflow_from_str() {
        let toml_str = r#"
            [workflow]
            id = "test"
            name = "测试工作流"
            description = "用于测试"
            version = "1.0.0"

            [[workflow.steps]]
            id = "step1"
            description = "步骤 1"
            tool = "test_tool"
        "#;

        let workflow = WorkflowLoader::load_from_str(toml_str).unwrap();
        assert_eq!(workflow.id, "test");
        assert_eq!(workflow.name, "测试工作流");
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.steps[0].id, "step1");
    }

    #[test]
    fn test_load_workflow_with_retry() {
        let toml_str = r#"
            [workflow]
            id = "test"
            name = "测试工作流"
            description = "用于测试"

            [[workflow.steps]]
            id = "step1"
            description = "步骤 1"
            tool = "test_tool"

            [workflow.steps.retry]
            max_retries = 5
            retry_interval_ms = 2000
            exponential_backoff = false
        "#;

        let workflow = WorkflowLoader::load_from_str(toml_str).unwrap();
        assert_eq!(workflow.steps[0].retry.max_retries, 5);
        assert_eq!(workflow.steps[0].retry.retry_interval_ms, 2000);
        assert!(!workflow.steps[0].retry.exponential_backoff);
    }

    #[test]
    fn test_load_workflow_with_error_handler() {
        let toml_str = r#"
            [workflow]
            id = "test"
            name = "测试工作流"
            description = "用于测试"

            [[workflow.steps]]
            id = "step1"
            description = "步骤 1"
            tool = "test_tool"

            [workflow.steps.on_error]
            strategy = "skip"
        "#;

        let workflow = WorkflowLoader::load_from_str(toml_str).unwrap();
        assert!(workflow.steps[0].on_error.is_some());
        assert_eq!(
            workflow.steps[0].on_error.as_ref().unwrap().strategy,
            ErrorStrategy::Skip
        );
    }
}
