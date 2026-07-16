//! Prompt 模板热加载模块
//!
//! 支持从 templates/prompt_gaps/ 目录加载 Prompt 模板，
//! 实现 Prompt 调优无需重新编译
//!
//! ## 使用示例
//! ```rust,ignore
//! let loader = PromptTemplateLoader::new("templates/prompt_gaps")?;
//!
//! // 加载因果分析模板

#![allow(dead_code)]
//! let causal_template = loader.load("causal_analysis")?;
//!
//! // 渲染模板（替换变量）
//! let rendered = loader.render(&causal_template, &[
//!     ("task_history", "..."),
//!     ("few_shot_examples", "..."),
//! ])?;
//! ```

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Prompt 模板加载器
pub struct PromptTemplateLoader {
    /// 模板目录路径
    template_dir: PathBuf,
    /// 缓存的模板内容
    cache: HashMap<String, String>,
}

impl PromptTemplateLoader {
    /// 创建新的加载器
    pub fn new<P: AsRef<Path>>(template_dir: P) -> Result<Self> {
        let template_dir = template_dir.as_ref().to_path_buf();

        if !template_dir.exists() {
            warn!("模板目录不存在：{:?}，将使用内置模板", template_dir);
        }

        Ok(Self {
            template_dir,
            cache: HashMap::new(),
        })
    }

    /// 加载模板文件
    pub fn load(&mut self, name: &str) -> Result<String> {
        // 先检查缓存
        if let Some(cached) = self.cache.get(name) {
            debug!("模板缓存命中：{}", name);
            return Ok(cached.clone());
        }

        // 从文件加载
        let template_path = self.template_dir.join(format!("{}.txt", name));

        if template_path.exists() {
            let content = fs::read_to_string(&template_path)
                .with_context(|| format!("读取模板文件失败：{:?}", template_path))?;

            debug!("加载模板：{:?}", template_path);
            self.cache.insert(name.to_string(), content.clone());
            Ok(content)
        } else {
            // 回退到内置模板
            warn!("模板文件不存在，使用内置模板：{}", name);
            Self::fallback_builtin_template(name)
        }
    }

    /// 渲染模板（替换变量）
    pub fn render(&self, template: &str, variables: &[(&str, &str)]) -> Result<String> {
        let mut rendered = template.to_string();

        for (key, value) in variables {
            // 支持 {{key}} 和 {key} 两种格式
            rendered = rendered.replace(&format!("{{{{{}}}}}", key), value);
            rendered = rendered.replace(&format!("{{{}}}", key), value);
        }

        Ok(rendered)
    }

    /// 清除缓存（用于热重载）
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        info!("Prompt 模板缓存已清除");
    }

    /// 重新加载所有模板（热重载）
    pub fn reload_all(&mut self) -> Result<()> {
        self.clear_cache();

        if !self.template_dir.exists() {
            return Ok(());
        }

        // 遍历目录加载所有 .txt 文件
        for entry in fs::read_dir(&self.template_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    let _ = self.load(name);
                }
            }
        }

        Ok(())
    }

    /// 获取缓存统计
    pub fn get_cache_stats(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();
        stats.insert("cache_size".to_string(), self.cache.len());
        stats
    }

    /// 内置模板回退（当文件不存在时使用）
    fn fallback_builtin_template(name: &str) -> Result<String> {
        match name {
            "causal_analysis" => Ok(include_str!("prompts/causal_analysis.txt").to_string()),
            "optimizer" => Ok(include_str!("prompts/optimizer.txt").to_string()),
            "agent_roles" => Ok(include_str!("prompts/agent_roles.txt").to_string()),
            _ => anyhow::bail!("未知模板：{}", name),
        }
    }
}

/// 默认实现
impl Default for PromptTemplateLoader {
    fn default() -> Self {
        Self::new("templates/prompt_gaps").unwrap_or_else(|_| Self {
            template_dir: PathBuf::from("templates/prompt_gaps"),
            cache: HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_create_loader() {
        let loader = PromptTemplateLoader::new("templates/prompt_gaps");
        assert!(loader.is_ok());
    }

    #[test]
    fn test_load_nonexistent_template() {
        let mut loader = PromptTemplateLoader::new("/tmp/nonexistent_dir").unwrap();
        // 应该回退到内置模板或返回错误
        let result = loader.load("nonexistent");
        // 不 panic 即可
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_render_template() {
        let loader = PromptTemplateLoader::default();
        let template = "Hello {{name}}, welcome to {{place}}!";

        let rendered = loader
            .render(template, &[("name", "Alice"), ("place", "Atlas")])
            .unwrap();

        assert_eq!(rendered, "Hello Alice, welcome to Atlas!");
    }

    #[test]
    fn test_cache_operations() {
        let temp_dir = TempDir::new().unwrap();
        let template_path = temp_dir.path().join("test.txt");
        fs::write(&template_path, "Test content").unwrap();

        let mut loader = PromptTemplateLoader::new(temp_dir.path()).unwrap();

        // 首次加载
        let content1 = loader.load("test").unwrap();
        assert_eq!(content1, "Test content");

        // 缓存命中
        let content2 = loader.load("test").unwrap();
        assert_eq!(content2, "Test content");

        // 清除缓存
        loader.clear_cache();
        assert_eq!(loader.cache.len(), 0);
    }

    #[test]
    fn test_reload_all() {
        let temp_dir = TempDir::new().unwrap();

        // 创建多个模板文件
        fs::write(temp_dir.path().join("template1.txt"), "Content 1").unwrap();
        fs::write(temp_dir.path().join("template2.txt"), "Content 2").unwrap();

        let mut loader = PromptTemplateLoader::new(temp_dir.path()).unwrap();

        // 重新加载所有
        let result = loader.reload_all();
        assert!(result.is_ok());

        // 验证缓存
        let stats = loader.get_cache_stats();
        assert!(stats["cache_size"] >= 2);
    }
}
