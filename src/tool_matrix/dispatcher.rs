//! 工具调用分发器
//!
//! 统一工具调用接口，支持：
//! - 基于工具名称自动路由到对应执行器
//! - 与 tokitai ToolProvider 深度集成
//! - 工具调用统计和追踪
//!
//! # 设计原则
//! - 统一工具调度接口
//! - 支持运行时动态注册工具

#![allow(dead_code)]
//! - 与 LightweightToolSelector 无缝配合

use crate::tool_matrix::matrix::ToolDefinition;
use crate::tool_matrix::tool_selector::{LightweightToolSelector, ToolSearchResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

/// 工具执行 trait（简化版，实际应该使用 tokitai 的 ToolExecutor）
#[async_trait::async_trait]
pub trait ToolExecutor: Send + Sync {
    /// 执行工具
    async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, String>;
}

/// 工具执行器包装
pub struct ExecutorWrapper {
    executor: Box<dyn ToolExecutor>,
    tool_names: Vec<String>,
}

impl ExecutorWrapper {
    pub fn new<E: ToolExecutor + 'static>(executor: E, tool_names: Vec<String>) -> Self {
        Self {
            executor: Box::new(executor),
            tool_names,
        }
    }
}

/// 工具调用分发器
pub struct ToolDispatcher {
    /// 工具选择器
    selector: Arc<LightweightToolSelector>,
    /// 工具执行器注册表：工具名 -> 执行器
    executors: Arc<RwLock<HashMap<String, Arc<dyn ToolExecutor>>>>,
    /// 工具调用统计：工具名 -> 调用次数
    call_stats: Arc<RwLock<HashMap<String, u64>>>,
}

impl ToolDispatcher {
    /// 创建新的分发器
    pub fn new(selector: Arc<LightweightToolSelector>) -> Self {
        Self {
            selector,
            executors: Arc::new(RwLock::new(HashMap::new())),
            call_stats: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册工具执行器
    pub async fn register_executor<E: ToolExecutor + 'static>(
        &self,
        tools: Vec<ToolDefinition>,
        executor: E,
    ) {
        let executor_arc = Arc::new(executor);
        let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

        // 注册到执行器表
        {
            let mut executors = self.executors.write().await;
            for tool_name in &tool_names {
                executors.insert(tool_name.clone(), executor_arc.clone());
                debug!("注册工具执行器：{}", tool_name);
            }
        }

        // 添加到选择器索引
        for tool in tools {
            let _ = self.selector.add_tool_async(tool).await;
        }

        info!("注册 {} 个工具执行器", tool_names.len());
    }

    /// 调用工具
    pub async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, String> {
        // 查找执行器
        let executors = self.executors.read().await;
        let executor = executors
            .get(tool_name)
            .ok_or_else(|| format!("工具未找到：{}", tool_name))?;

        // 更新统计
        {
            let mut stats = self.call_stats.write().await;
            *stats.entry(tool_name.to_string()).or_insert(0) += 1;
        }

        // 执行工具
        let result = executor.execute(tool_name, args).await;

        match &result {
            Ok(_) => debug!("工具调用成功：{}", tool_name),
            Err(e) => warn!("工具调用失败：{} - {}", tool_name, e),
        }

        result
    }

    /// 搜索工具
    pub async fn search_tools(&self, query: &str) -> Vec<ToolSearchResult> {
        self.selector.search(query).await
    }

    /// 获取工具调用统计
    pub async fn get_call_stats(&self) -> HashMap<String, u64> {
        self.call_stats.read().await.clone()
    }

    /// 获取所有已注册工具
    pub async fn get_all_tools(&self) -> Vec<ToolDefinition> {
        self.selector.get_all_tools().await
    }

    /// 按分类获取工具
    pub async fn get_tools_by_category(
        &self,
        category: &crate::tool_matrix::matrix::ServiceCategory,
    ) -> Vec<ToolDefinition> {
        self.selector.get_tools_by_category(category).await
    }
}

// ============================================================================
// 默认工具执行器实现（用于测试）
// ============================================================================

/// 默认工具执行器（测试用）
pub struct DefaultToolExecutor {
    handler: Box<dyn Fn(&str, &Value) -> Result<Value, String> + Send + Sync>,
}

impl DefaultToolExecutor {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(&str, &Value) -> Result<Value, String> + Send + Sync + 'static,
    {
        Self {
            handler: Box::new(handler),
        }
    }
}

#[async_trait::async_trait]
impl ToolExecutor for DefaultToolExecutor {
    async fn execute(&self, tool_name: &str, args: &Value) -> Result<Value, String> {
        (self.handler)(tool_name, args)
    }
}

// ============================================================================
// 与 tokitai 集成的执行器
// ============================================================================

// 注意：tokitai 集成需要根据实际 API 调整
// 当前实现提供一个框架，具体的 tokitai 集成需要参考 tokitai 的 ToolProvider trait

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_matrix::matrix::ServiceCategory;
    use serde_json::json;

    #[tokio::test]
    async fn test_tool_dispatcher() {
        // 创建选择器
        let selector = Arc::new(LightweightToolSelector::new_without_ai(
            vec![ToolDefinition::new("test_tool", "A test tool", r#"{}"#)],
            None,
        ));

        // 创建分发器
        let dispatcher = ToolDispatcher::new(selector);

        // 注册执行器
        let executor = DefaultToolExecutor::new(|name, args| {
            Ok(json!({"tool": name, "args": args}))
        });

        let tools = vec![ToolDefinition::new("test_tool", "A test tool", r#"{}"#)
            .with_category(ServiceCategory::Utility)];

        dispatcher.register_executor(tools, executor).await;

        // 调用工具
        let result = dispatcher
            .execute("test_tool", &json!({"key": "value"}))
            .await
            .unwrap();

        assert_eq!(result["tool"], "test_tool");
        assert_eq!(result["args"]["key"], "value");

        // 检查统计
        let stats = dispatcher.get_call_stats().await;
        assert_eq!(stats.get("test_tool"), Some(&1));
    }
}
