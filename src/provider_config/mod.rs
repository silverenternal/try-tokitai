//! AI 供应商配置管理
//!
//! 支持多供应商配置和循环切换

#![allow(dead_code)]

mod provider_queue;

#[allow(unused_imports)]
pub use provider_queue::ProviderQueue;

use std::collections::HashMap;
use std::env;
use std::fs;

/// AI 供应商配置
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// 供应商名称（如 ollama, openai, anthropic）
    pub name: String,
    /// API URL
    pub api_url: String,
    /// API Key（可选）
    pub api_key: Option<String>,
    /// 默认模型
    pub model: String,
}

/// 供应商管理器
pub struct ProviderManager {
    /// 当前供应商
    current: ProviderConfig,
    /// 可用供应商列表
    providers: Vec<ProviderConfig>,
    /// 当前索引
    current_index: usize,
}

impl ProviderManager {
    /// 从 .env 文件加载供应商配置
    pub fn from_env_file(env_path: Option<&str>) -> Result<Self, String> {
        let env_path = env_path.unwrap_or(".env");

        // 读取 .env 文件
        let env_content =
            fs::read_to_string(env_path).map_err(|e| format!("读取 .env 文件失败：{}", e))?;

        // 解析所有供应商配置
        let providers = Self::parse_providers(&env_content)?;

        if providers.is_empty() {
            return Err("没有找到可用的 AI 供应商配置".to_string());
        }

        // 找到当前正在使用的供应商（通过环境变量）
        let current_url = env::var("AI_API_URL").ok();
        let current_index = providers
            .iter()
            .position(|p| Some(&p.api_url) == current_url.as_ref())
            .unwrap_or(0);

        let current = providers[current_index].clone();

        Ok(Self {
            current,
            providers,
            current_index,
        })
    }

    /// 解析 .env 文件中的多供应商配置
    fn parse_providers(env_content: &str) -> Result<Vec<ProviderConfig>, String> {
        let mut providers = Vec::new();
        let provider_groups: HashMap<String, HashMap<String, String>> = HashMap::new();

        // 首先检查是否有 PROVIDERS 配置
        let mut provider_names = Vec::new();

        for line in env_content.lines() {
            let line = line.trim();

            // 跳过注释和空行
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // 检查 PROVIDERS 配置
            if line.starts_with("PROVIDERS=") {
                let value = line.trim_start_matches("PROVIDERS=").trim();
                provider_names = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                break;
            }
        }

        // 如果没有 PROVIDERS 配置，使用默认的单供应商配置
        if provider_names.is_empty() {
            let mut props = HashMap::new();

            for line in env_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();

                    // 跳过 PROVIDERS 行
                    if key == "PROVIDERS" {
                        continue;
                    }

                    // 只保留标准配置
                    if !key.starts_with("PROVIDER_") {
                        props.insert(key.to_string(), value.to_string());
                    }
                }
            }

            // 创建默认供应商
            let name = props
                .get("AI_API_URL")
                .map(|url| {
                    if url.contains("ollama") {
                        "ollama"
                    } else if url.contains("openai") {
                        "openai"
                    } else if url.contains("anthropic") {
                        "anthropic"
                    } else {
                        "default"
                    }
                })
                .unwrap_or("default")
                .to_string();

            let provider = Self::create_provider_from_props(&name, &props);
            providers.push(provider);

            return Ok(providers);
        }

        // 解析每个供应商的配置
        for name in &provider_names {
            let mut props = HashMap::new();

            // 收集该供应商的所有配置
            for line in env_content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim();
                    let value = value.trim();

                    // 检查是否是该供应商的配置
                    let provider_key = format!("PROVIDER_{}_", name.to_uppercase());
                    if key.starts_with(&provider_key) {
                        let sub_key = key.trim_start_matches(&provider_key);
                        props.insert(sub_key.to_string(), value.to_string());
                    }
                }
            }

            if !props.is_empty() {
                let provider = Self::create_provider_from_props(name, &props);
                providers.push(provider);
            }
        }

        Ok(providers)
    }

    /// 从属性创建供应商配置
    fn create_provider_from_props(name: &str, props: &HashMap<String, String>) -> ProviderConfig {
        // 尝试多种可能的键名
        let api_url = props
            .get("API_URL")
            .or_else(|| props.get("AI_API_URL"))
            .cloned()
            .unwrap_or_else(|| "https://api.openai.com/v1/chat/completions".to_string());

        let api_key = props
            .get("API_KEY")
            .or_else(|| props.get("AI_API_KEY"))
            .cloned();

        let model = props
            .get("MODEL")
            .or_else(|| props.get("AI_MODEL"))
            .cloned()
            .unwrap_or_else(|| "gpt-3.5-turbo".to_string());

        ProviderConfig {
            name: name.to_string(),
            api_url,
            api_key,
            model,
        }
    }

    /// 获取当前供应商
    pub fn current(&self) -> &ProviderConfig {
        &self.current
    }

    /// 获取所有供应商
    pub fn providers(&self) -> &[ProviderConfig] {
        &self.providers
    }

    /// 切换到下一个供应商（循环）
    pub fn switch_to_next(&mut self) -> &ProviderConfig {
        if self.providers.len() <= 1 {
            return &self.current;
        }

        self.current_index = (self.current_index + 1) % self.providers.len();
        self.current = self.providers[self.current_index].clone();
        &self.current
    }

    /// 切换到指定供应商
    pub fn switch_to(&mut self, name: &str) -> Result<&ProviderConfig, String> {
        let index = self
            .providers
            .iter()
            .position(|p| p.name.to_lowercase() == name.to_lowercase())
            .ok_or_else(|| format!("未找到供应商：{}", name))?;

        self.current_index = index;
        self.current = self.providers[index].clone();
        Ok(&self.current)
    }

    /// 获取供应商列表（用于显示）
    pub fn list_providers(&self) -> String {
        let mut output = String::from("📋 可用的 AI 供应商：\n\n");

        for (i, provider) in self.providers.iter().enumerate() {
            let marker = if i == self.current_index {
                "👉"
            } else {
                "  "
            };
            output.push_str(&format!(
                "{} {} - {} (模型：{})\n",
                marker, provider.name, provider.api_url, provider.model
            ));
        }

        output.push_str(&format!(
            "\n当前：{} (索引：{}/{})\n",
            self.current.name,
            self.current_index + 1,
            self.providers.len()
        ));

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_provider() {
        let env_content = r#"
AI_API_URL=https://ollama.com/v1/chat/completions
AI_API_KEY=test_key
AI_MODEL=qwen3.5:397b
"#;

        let providers = ProviderManager::parse_providers(env_content).unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].name, "ollama");
        assert_eq!(
            providers[0].api_url,
            "https://ollama.com/v1/chat/completions"
        );
        assert_eq!(providers[0].api_key, Some("test_key".to_string()));
        assert_eq!(providers[0].model, "qwen3.5:397b");
    }

    #[test]
    fn test_parse_multiple_providers() {
        let env_content = r#"
PROVIDERS=ollama,openai

PROVIDER_OLLAMA_API_URL=https://ollama.com/v1/chat/completions
PROVIDER_OLLAMA_API_KEY=ollama_key
PROVIDER_OLLAMA_MODEL=qwen3.5:397b

PROVIDER_OPENAI_API_URL=https://api.openai.com/v1/chat/completions
PROVIDER_OPENAI_API_KEY=sk-xxx
PROVIDER_OPENAI_MODEL=gpt-4
"#;

        let providers = ProviderManager::parse_providers(env_content).unwrap();
        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].name, "ollama");
        assert_eq!(providers[1].name, "openai");
    }
}
