//! 在 server 模式下复刻 CliAssistant 的工具调用能力
//!
//! 因 CliAssistant 内部拥有大量 owned 工具实例，server 模式不便直接共享，
//! 故构造一份独立的 ToolSet 作为 AppState 的子组件。
//!
//! 实际调用走与 CliAssistant::call_tool 一致的 try-each 顺序：
//! 依次尝试每个工具实例；只有错误不是 NotFound 时才立即返回。

use anyhow::Result;
use serde_json::Value;
use std::sync::Arc;
use tokitai::ToolCaller;
use tokitai_core::ToolErrorKind;

use crate::tools::{
    CodeTools, DownloadTools, FileOperations, GitOperations, HttpClientTools, JsonFormatTools,
    SearchTools, SystemTools,
};

/// 聚合 server 模式下可用的全部工具实例
///
/// 8 个工具实例各自放在独立的 Arc 中，方便跨 handler 共享。
#[derive(Clone)]
pub struct ServerToolSet {
    pub file_ops: Arc<FileOperations>,
    pub system_tools: Arc<SystemTools>,
    pub code_tools: Arc<CodeTools>,
    pub web_search: Arc<SearchTools>,
    pub download_tools: Arc<DownloadTools>,
    pub git_ops: Arc<GitOperations>,
    pub http_client: Arc<HttpClientTools>,
    pub json_tools: Arc<JsonFormatTools>,
}

impl ServerToolSet {
    /// 构造默认的 ServerToolSet（每个工具使用 ::new() 或 ::default()）
    pub fn new_default() -> Self {
        Self {
            file_ops: Arc::new(FileOperations::new()),
            system_tools: Arc::new(SystemTools::default()),
            code_tools: Arc::new(CodeTools::default()),
            web_search: Arc::new(SearchTools::new()),
            download_tools: Arc::new(DownloadTools::new()),
            git_ops: Arc::new(GitOperations::new()),
            http_client: Arc::new(HttpClientTools::new()),
            json_tools: Arc::new(JsonFormatTools::new()),
        }
    }

    /// 按 CliAssistant 相同的顺序逐一尝试调用工具
    ///
    /// 行为等价于 CliAssistant::call_tool：
    /// - 第一个返回 Ok 的 provider 胜出；
    /// - 第一个返回非 NotFound 错误的 provider 立即以错误返回；
    /// - 所有 provider 都返回 NotFound 时报"未知工具"。
    pub fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        // 收集所有 provider，统一按 fallback 顺序尝试
        let providers: Vec<Arc<dyn ToolCallerDyn>> = vec![
            Arc::clone(&self.file_ops) as Arc<dyn ToolCallerDyn>,
            Arc::clone(&self.system_tools) as Arc<dyn ToolCallerDyn>,
            Arc::clone(&self.code_tools) as Arc<dyn ToolCallerDyn>,
            Arc::clone(&self.web_search) as Arc<dyn ToolCallerDyn>,
            Arc::clone(&self.download_tools) as Arc<dyn ToolCallerDyn>,
            Arc::clone(&self.git_ops) as Arc<dyn ToolCallerDyn>,
            Arc::clone(&self.http_client) as Arc<dyn ToolCallerDyn>,
            Arc::clone(&self.json_tools) as Arc<dyn ToolCallerDyn>,
        ];

        for provider in providers {
            match provider.call_tool_dyn(name, args) {
                Ok(r) => return Ok(r.to_string()),
                Err(e) => {
                    if e.kind != ToolErrorKind::NotFound {
                        return Err(anyhow::anyhow!("工具 {} 执行失败：{}", name, e));
                    }
                }
            }
        }

        Err(anyhow::anyhow!("未知工具：{}", name))
    }
}

/// 把 tokitai::ToolCaller 的同步 call_tool 包成可放进 Vec<dyn ...> 的 dyn 形式
trait ToolCallerDyn: Send + Sync {
    fn call_tool_dyn(&self, name: &str, args: &Value)
        -> Result<Value, tokitai_core::ToolError>;
}

impl<T: ToolCaller + Send + Sync> ToolCallerDyn for T {
    fn call_tool_dyn(
        &self,
        name: &str,
        args: &Value,
    ) -> Result<Value, tokitai_core::ToolError> {
        ToolCaller::call_tool(self, name, args)
    }
}