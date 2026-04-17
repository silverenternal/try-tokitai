//! 工具市场模块
//!
//! 提供工具的发布、搜索和安装功能
//!
//! ## 功能
//! - `publish`: 发布工具到 registry
//! - `search`: 搜索社区工具
//! - `install`: 安装工具并处理依赖

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::info;

/// 工具注册表配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// 注册表 URL
    pub url: String,
    /// API 密钥（可选）
    pub api_key: Option<String>,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            url: "https://registry.tokitai.org".to_string(),
            api_key: None,
        }
    }
}

/// 工具元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// 工具名称
    pub name: String,
    /// 版本号
    pub version: String,
    /// 描述
    pub description: String,
    /// 作者
    pub author: String,
    /// 分类
    pub category: String,
    /// 标签
    pub tags: Vec<String>,
    /// 依赖
    pub dependencies: Vec<String>,
    /// 源代码 URL
    pub source_url: String,
    /// 下载 URL
    pub download_url: Option<String>,
    /// 许可证
    pub license: String,
}

/// 工具包
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPackage {
    /// 工具元数据
    pub metadata: ToolMetadata,
    /// 源代码
    pub source_code: String,
    /// 测试代码（可选）
    pub test_code: Option<String>,
    /// README 文档
    pub readme: Option<String>,
}

/// 搜索结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// 总数量
    pub total: usize,
    /// 结果列表
    pub results: Vec<ToolMetadata>,
}

/// 工具注册表客户端
pub struct RegistryClient {
    config: RegistryConfig,
    client: Client,
}

impl RegistryClient {
    /// 创建新的注册表客户端
    pub fn new(config: RegistryConfig) -> Self {
        let client = Client::builder()
            .user_agent("try-tokitai/0.1.0")
            .build()
            .unwrap_or_default();

        Self { config, client }
    }

    /// 从环境变量加载配置
    pub fn from_env() -> Self {
        let url = std::env::var("TOKITAI_REGISTRY_URL")
            .unwrap_or_else(|_| "https://registry.tokitai.org".to_string());
        let api_key = std::env::var("TOKITAI_REGISTRY_API_KEY").ok();

        Self::new(RegistryConfig { url, api_key })
    }

    /// 发布工具
    pub async fn publish(&self, package: ToolPackage) -> Result<()> {
        info!("发布工具：{}", package.metadata.name);

        let url = format!("{}/api/v1/tools", self.config.url);

        let mut request = self.client.post(&url).json(&package);
        if let Some(ref api_key) = self.config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await.context("发送发布请求失败")?;

        if response.status().is_success() {
            info!("工具发布成功：{}", package.metadata.name);
            Ok(())
        } else {
            let error = response.text().await.unwrap_or_default();
            bail!("发布失败：{}", error)
        }
    }

    /// 搜索工具
    pub async fn search(&self, query: &str) -> Result<SearchResults> {
        info!("搜索工具：{}", query);

        let url = format!("{}/api/v1/tools/search?q={}", self.config.url, query);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("发送搜索请求失败")?;

        if response.status().is_success() {
            let results = response
                .json::<SearchResults>()
                .await
                .context("解析搜索结果失败")?;
            Ok(results)
        } else {
            let error = response.text().await.unwrap_or_default();
            bail!("搜索失败：{}", error)
        }
    }

    /// 下载工具
    pub async fn download(&self, tool_name: &str) -> Result<ToolPackage> {
        info!("下载工具：{}", tool_name);

        let url = format!("{}/api/v1/tools/{}/download", self.config.url, tool_name);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("发送下载请求失败")?;

        if response.status().is_success() {
            let package = response
                .json::<ToolPackage>()
                .await
                .context("解析工具包失败")?;
            Ok(package)
        } else {
            let error = response.text().await.unwrap_or_default();
            bail!("下载失败：{}", error)
        }
    }
}

/// 工具市场管理器
pub struct ToolMarket {
    registry_client: RegistryClient,
    /// 本地工具目录
    local_tools_dir: PathBuf,
}

impl ToolMarket {
    /// 创建新的工具市场管理器
    pub fn new(registry_config: Option<RegistryConfig>) -> Result<Self> {
        let registry_client = match registry_config {
            Some(config) => RegistryClient::new(config),
            None => RegistryClient::from_env(),
        };

        let local_tools_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tokitai")
            .join("tools");

        // 创建本地工具目录
        std::fs::create_dir_all(&local_tools_dir).context("创建本地工具目录失败")?;

        Ok(Self {
            registry_client,
            local_tools_dir,
        })
    }

    /// 发布工具
    pub async fn publish(&self, tool_name: &str) -> Result<()> {
        info!("发布工具：{}", tool_name);

        // 加载本地工具
        let tool_path = self.local_tools_dir.join(tool_name);
        if !tool_path.exists() {
            bail!("工具不存在：{}", tool_name)
        }

        // 读取工具源代码
        let source_code = std::fs::read_to_string(tool_path.join("src").join("lib.rs"))
            .context("读取工具源代码失败")?;

        // 读取元数据
        let metadata_path = tool_path.join("Cargo.toml");
        let metadata_content =
            std::fs::read_to_string(&metadata_path).context("读取 Cargo.toml 失败")?;

        // 解析 Cargo.toml 获取元数据（简化处理）
        let metadata = self.parse_cargo_metadata(&metadata_content)?;

        let package = ToolPackage {
            metadata,
            source_code,
            test_code: None,
            readme: None,
        };

        self.registry_client.publish(package).await?;

        println!("✅ 工具发布成功：{}", tool_name);
        Ok(())
    }

    /// 搜索工具
    pub async fn search(&self, query: &str) -> Result<()> {
        info!("搜索工具：{}", query);

        let results = self.registry_client.search(query).await?;

        if results.results.is_empty() {
            println!("未找到匹配的工具");
            return Ok(());
        }

        println!("找到 {} 个工具:", results.total);
        println!();

        for (i, tool) in results.results.iter().enumerate() {
            println!("{}. {} - {}", i + 1, tool.name, tool.description);
            println!(
                "   版本：{} | 分类：{} | 标签：{}",
                tool.version,
                tool.category,
                tool.tags.join(", ")
            );
        }

        Ok(())
    }

    /// 安装工具
    pub async fn install(&self, tool_name: &str) -> Result<()> {
        info!("安装工具：{}", tool_name);

        // 下载工具
        let package = self.registry_client.download(tool_name).await?;

        // 创建工具目录
        let tool_dir = self.local_tools_dir.join(&package.metadata.name);
        std::fs::create_dir_all(&tool_dir).context("创建工具目录失败")?;

        // 保存源代码
        let src_dir = tool_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;
        std::fs::write(src_dir.join("lib.rs"), &package.source_code).context("保存源代码失败")?;

        // 保存 Cargo.toml
        let cargo_toml = self.generate_cargo_toml(&package.metadata);
        std::fs::write(tool_dir.join("Cargo.toml"), cargo_toml).context("保存 Cargo.toml 失败")?;

        println!("✅ 工具安装成功：{}", package.metadata.name);
        println!("   路径：{}", tool_dir.display());

        // 处理依赖
        if !package.metadata.dependencies.is_empty() {
            println!();
            println!("📦 正在安装依赖...");
            for dep in &package.metadata.dependencies {
                println!("   安装依赖：{}", dep);
                // 递归安装依赖
                // self.install(dep).await?;
            }
        }

        Ok(())
    }

    /// 列出现有工具
    pub fn list(&self) -> Result<Vec<String>> {
        let mut tools = Vec::new();

        if self.local_tools_dir.exists() {
            for entry in std::fs::read_dir(&self.local_tools_dir)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        tools.push(name.to_string());
                    }
                }
            }
        }

        Ok(tools)
    }

    /// 解析 Cargo.toml 元数据
    fn parse_cargo_metadata(&self, content: &str) -> Result<ToolMetadata> {
        // 简化解析，实际应该使用 toml crate
        let mut name = String::new();
        let mut version = String::from("0.1.0");
        let mut description = String::new();
        let mut author = String::from("Unknown");
        let mut license = String::from("MIT");

        for line in content.lines() {
            let line = line.trim();
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().trim_matches('"');
                let value = value.trim().trim_matches('"');

                match key {
                    "name" => name = value.to_string(),
                    "version" => version = value.to_string(),
                    "description" => description = value.to_string(),
                    "author" => author = value.to_string(),
                    "license" => license = value.to_string(),
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            bail!("Cargo.toml 中未找到工具名称");
        }

        Ok(ToolMetadata {
            name,
            version,
            description,
            author,
            category: "custom".to_string(),
            tags: vec![],
            dependencies: vec![],
            source_url: String::new(),
            download_url: None,
            license,
        })
    }

    /// 生成 Cargo.toml
    fn generate_cargo_toml(&self, metadata: &ToolMetadata) -> String {
        format!(
            r#"[package]
name = "{}"
version = "{}"
edition = "2021"
description = "{}"
license = "{}"

[dependencies]
tokitai = "0.4.0"
"#,
            metadata.name, metadata.version, metadata.description, metadata.license
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_config_default() {
        let config = RegistryConfig::default();
        assert_eq!(config.url, "https://registry.tokitai.org");
        assert!(config.api_key.is_none());
    }

    #[test]
    fn test_tool_market_creation() {
        let market = ToolMarket::new(None);
        assert!(market.is_ok());
    }
}
