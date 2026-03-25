//! MCP Server 模式实现
//! 
//! 将 try-tokitai 作为 MCP Server 运行，供其他 AI 客户端调用我们的工具

use anyhow::Result;
use tracing::info;
use tokitai_mcp_server::server::McpServer;

/// 启动 MCP Server 模式
pub async fn start_mcp_mode() -> Result<()> {
    info!("启动 MCP Server 模式");
    
    // 创建 MCP Server
    let server = McpServer::new();
    
    // 运行服务器
    server.run().await?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mcp_server_creation() {
        let config = McpServerConfig::default();
        let server = McpServer::new();
        // 测试服务器创建
        assert!(true);
    }
}
