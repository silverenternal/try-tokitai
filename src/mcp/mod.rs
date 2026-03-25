//! MCP (Model Context Protocol) 模块
//! 
//! 提供 MCP Server 和 Client 模式支持
//! 
//! ## MCP Server 模式
//! 将 try-tokitai 的工具暴露为 MCP 服务，供其他 AI 客户端调用
//! 
//! ## MCP Client 模式
//! 作为 MCP 客户端，发现和调用外部 MCP Server 的工具

pub mod server;
pub mod client;

pub use server::start_mcp_mode;
pub use client::{McpClient, McpClientManager, McpServerDescription, DiscoveredTool};
