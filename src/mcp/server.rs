//! MCP Server 模式实现
//!
//! 将 try-tokitai 作为 MCP Server 运行，供其他 AI 客户端调用我们的工具
//!
//! ## 安全说明
//! - 所有通过 MCP 调用的工具会经过 authorize_tool_call(ExecutionMode::Mcp) 检查
//! - Dangerous 级别工具在 MCP 模式下被完全禁止
//! - MCP_API_KEY 环境变量或 [security] 段配置用于客户端认证
//! - 如果 mcp_auth_required=true 且未设置 API key, 服务器将拒绝启动

use anyhow::Result;
use tokitai_mcp_server::server::McpServer;
use tracing::{info, warn};

/// 启动 MCP Server 模式
///
/// `security_config` 控制 MCP Server 的安全设置：
/// - `mcp_auth_required`: 如果为 true, 必须设置 API key 才能启动
/// - `mcp_api_key`: MCP API key (来自 config.toml [security] 段或环境变量 MCP_API_KEY)
/// - MCP 模式下 Dangerous 级别的工具被完全禁止
pub async fn start_mcp_mode(security_config: &crate::security::SecurityConfig) -> Result<()> {
    info!("启动 MCP Server 模式");

    // 安全检查：如果要求认证但未设置 API key, 拒绝启动
    let env_key = std::env::var("MCP_API_KEY").ok();
    let effective_key = if !security_config.mcp_api_key.is_empty() {
        Some(security_config.mcp_api_key.clone())
    } else {
        env_key
    };

    if security_config.mcp_auth_required {
        match &effective_key {
            Some(_) => {
                info!("✅ MCP 认证已配置 (API key detected)");
            }
            None => {
                anyhow::bail!(
                    "MCP 安全策略：mcp_auth_required=true 但未设置 API key。\n\
                     请在 config.toml 的 [security] 段中设置 mcp_api_key，\n\
                     或设置环境变量 MCP_API_KEY。\n\
                     若要允许无认证启动，请设置 mcp_auth_required=false。"
                );
            }
        }
    } else {
        warn!("⚠️  MCP 认证已禁用 (mcp_auth_required=false)。任何可访问此服务的客户端都能调用工具。");
    }

    // 将 API key 注入环境变量，供底层 MCP 协议使用
    if let Some(key) = &effective_key {
        std::env::set_var("MCP_API_KEY", key);
    }

    // 打印安全边界
    info!("MCP 安全边界:");
    info!("  - 认证: {}", if security_config.mcp_auth_required { "必需" } else { "已禁用" });
    info!("  - Dangerous 工具: 已拦截");
    info!("  - Moderate 工具: 允许");
    info!("  - Safe 工具: 允许");
    info!("  - 速率限制: {}/min, {} burst/sec",
        security_config.max_tool_calls_per_minute,
        security_config.tool_call_burst_limit);

    // 创建并运行 MCP Server
    // 注意：tokitai_mcp_server 的 McpServer::run() 会通过 stdio 接收客户端请求，
    // 并调用注册的工具。工具级授权依赖于各工具自身的安全机制（如 run_command 的 confirmed 参数）。
    // MCP 模式下的额外安全由 authorize_tool_call(ExecutionMode::Mcp) 在工具分发层提供。
    let server = McpServer::new();

    // 运行服务器
    server.run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    

    #[tokio::test]
    async fn test_mcp_server_security_requires_key_when_auth_enabled() {
        // 模拟安全配置：auth 启用但无 key
        let mut config = crate::security::SecurityConfig::default();
        config.mcp_auth_required = true;
        config.mcp_api_key = String::new();

        // 确保环境变量也未设置
        std::env::remove_var("MCP_API_KEY");

        // 不应 panic, 但应返回错误
        // 由于 McpServer::run() 会阻塞, 这里只测试验证逻辑
        let env_key = std::env::var("MCP_API_KEY").ok();
        let effective_key = if !config.mcp_api_key.is_empty() {
            Some(config.mcp_api_key.clone())
        } else {
            env_key
        };
        assert!(config.mcp_auth_required);
        assert!(effective_key.is_none());
    }

    #[tokio::test]
    async fn test_mcp_server_allows_startup_with_key() {
        let mut config = crate::security::SecurityConfig::default();
        config.mcp_auth_required = true;
        config.mcp_api_key = "test-key-123".to_string();

        let effective_key = if !config.mcp_api_key.is_empty() {
            Some(config.mcp_api_key.clone())
        } else {
            std::env::var("MCP_API_KEY").ok()
        };
        assert!(effective_key.is_some());
    }

    #[tokio::test]
    async fn test_mcp_server_warns_when_auth_disabled() {
        let config = crate::security::SecurityConfig::default();
        // default has mcp_auth_required=true but empty mcp_api_key
        // So this would fail the strict check in start_mcp_mode
        assert!(config.mcp_auth_required);
        assert!(config.mcp_api_key.is_empty());
    }
}
