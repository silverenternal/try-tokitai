//! 提示词模板管理器
//!
//! 实现模板加载、注册、渲染和缓存功能

use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use parking_lot::RwLock;

use crate::prompt_engineering::renderer::PromptRenderer;
use crate::prompt_engineering::template::PromptTemplate;

/// 模板缓存条目
#[derive(Debug, Clone)]
struct CacheEntry {
    template: PromptTemplate,
    loaded_at: chrono::DateTime<chrono::Local>,
}

/// 提示词模板管理器
pub struct PromptTemplateManager {
    /// 模板存储根目录
    templates_dir: PathBuf,
    /// 模板缓存
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// 渲染器
    renderer: PromptRenderer,
    /// 是否启用缓存
    use_cache: bool,
}

impl Default for PromptTemplateManager {
    fn default() -> Self {
        Self::new(".context/prompt_templates".to_string())
    }
}

impl PromptTemplateManager {
    /// 创建新的模板管理器
    pub fn new(templates_dir: String) -> Self {
        Self {
            templates_dir: PathBuf::from(templates_dir),
            cache: Arc::new(RwLock::new(HashMap::new())),
            renderer: PromptRenderer::new(),
            use_cache: true,
        }
    }

    /// 创建带自定义目录的模板管理器
    pub fn with_path<P: AsRef<Path>>(templates_dir: P) -> Result<Self> {
        let path = templates_dir.as_ref().to_path_buf();
        
        // 确保目录存在
        if !path.exists() {
            fs::create_dir_all(&path)
                .with_context(|| format!("Failed to create templates directory: {:?}", path))?;
        }

        Ok(Self {
            templates_dir: path,
            cache: Arc::new(RwLock::new(HashMap::new())),
            renderer: PromptRenderer::new(),
            use_cache: true,
        })
    }

    /// 加载模板
    pub fn load_template(&self, role: &str) -> Result<PromptTemplate> {
        let cache_key = format!("role_{}", role);

        // 尝试从缓存获取
        if self.use_cache {
            if let Some(entry) = self.cache.read().get(&cache_key) {
                return Ok(entry.template.clone());
            }
        }

        // 从文件加载
        let template_path = self.templates_dir.join("roles").join(format!("{}.json", role));
        
        if !template_path.exists() {
            anyhow::bail!("Template not found: {:?}", template_path);
        }

        let content = fs::read_to_string(&template_path)
            .with_context(|| format!("Failed to read template: {:?}", template_path))?;

        let mut template: PromptTemplate = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse template: {:?}", template_path))?;

        // 更新加载时间
        let now = chrono::Local::now();
        template.updated_at = Some(now.to_rfc3339());

        // 缓存模板
        if self.use_cache {
            self.cache.write().insert(
                cache_key,
                CacheEntry {
                    template: template.clone(),
                    loaded_at: now,
                },
            );
        }

        Ok(template)
    }

    /// 加载任务模板
    pub fn load_task_template(&self, task_name: &str) -> Result<PromptTemplate> {
        let cache_key = format!("task_{}", task_name);

        // 尝试从缓存获取
        if self.use_cache {
            if let Some(entry) = self.cache.read().get(&cache_key) {
                return Ok(entry.template.clone());
            }
        }

        // 从文件加载
        let template_path = self
            .templates_dir
            .join("tasks")
            .join(format!("{}.json", task_name));

        if !template_path.exists() {
            anyhow::bail!("Task template not found: {:?}", template_path);
        }

        let content = fs::read_to_string(&template_path)
            .with_context(|| format!("Failed to read task template: {:?}", template_path))?;

        let mut template: PromptTemplate = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse task template: {:?}", template_path))?;

        // 更新加载时间
        let now = chrono::Local::now();
        template.updated_at = Some(now.to_rfc3339());

        // 缓存模板
        if self.use_cache {
            self.cache.write().insert(
                cache_key,
                CacheEntry {
                    template: template.clone(),
                    loaded_at: now,
                },
            );
        }

        Ok(template)
    }

    /// 渲染模板
    pub fn render(&self, template: &PromptTemplate, variables: &Value) -> Result<String> {
        self.renderer.render(&template.system_prompt, variables)
    }

    /// 获取系统提示词（快捷方法）
    pub fn get_system_prompt(&self, role: &str, variables: &Value) -> Result<String> {
        let template = self.load_template(role)?;
        self.render(&template, variables)
    }

    /// 注册模板
    pub fn register_template(&mut self, template: PromptTemplate) -> Result<()> {
        let cache_key = format!("role_{}", template.role);

        // 保存到文件
        let template_path = self
            .templates_dir
            .join("roles")
            .join(format!("{}.json", template.role));

        // 确保目录存在
        if let Some(parent) = template_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(&template)
            .with_context(|| "Failed to serialize template")?;

        fs::write(&template_path, content)
            .with_context(|| format!("Failed to write template: {:?}", template_path))?;

        // 更新缓存
        let now = chrono::Local::now();
        self.cache.write().insert(
            cache_key,
            CacheEntry {
                template,
                loaded_at: now,
            },
        );

        Ok(())
    }

    /// 注册任务模板
    pub fn register_task_template(&mut self, template: PromptTemplate) -> Result<()> {
        let cache_key = format!("task_{}", template.id);

        // 保存到文件
        let template_path = self
            .templates_dir
            .join("tasks")
            .join(format!("{}.json", template.id));

        // 确保目录存在
        if let Some(parent) = template_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {:?}", parent))?;
        }

        let content = serde_json::to_string_pretty(&template)
            .with_context(|| "Failed to serialize template")?;

        fs::write(&template_path, content)
            .with_context(|| format!("Failed to write template: {:?}", template_path))?;

        // 更新缓存
        let now = chrono::Local::now();
        self.cache.write().insert(
            cache_key,
            CacheEntry {
                template,
                loaded_at: now,
            },
        );

        Ok(())
    }

    /// 获取所有角色
    pub fn get_all_roles(&self) -> Result<Vec<String>> {
        let roles_dir = self.templates_dir.join("roles");

        if !roles_dir.exists() {
            return Ok(Vec::new());
        }

        let mut roles = Vec::new();

        for entry in fs::read_dir(&roles_dir)
            .with_context(|| format!("Failed to read roles directory: {:?}", roles_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    roles.push(stem.to_string());
                }
            }
        }

        Ok(roles)
    }

    /// 获取所有任务模板
    pub fn get_all_task_templates(&self) -> Result<Vec<String>> {
        let tasks_dir = self.templates_dir.join("tasks");

        if !tasks_dir.exists() {
            return Ok(Vec::new());
        }

        let mut tasks = Vec::new();

        for entry in fs::read_dir(&tasks_dir)
            .with_context(|| format!("Failed to read tasks directory: {:?}", tasks_dir))?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    tasks.push(stem.to_string());
                }
            }
        }

        Ok(tasks)
    }

    /// 清除缓存
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// 热加载模板（重新从文件加载）
    pub fn reload_template(&self, role: &str) -> Result<PromptTemplate> {
        // 从缓存中移除
        let cache_key = format!("role_{}", role);
        self.cache.write().remove(&cache_key);

        // 重新加载
        self.load_template(role)
    }

    /// 检查模板是否存在
    pub fn template_exists(&self, role: &str) -> bool {
        let template_path = self.templates_dir.join("roles").join(format!("{}.json", role));
        template_path.exists()
    }

    /// 检查任务模板是否存在
    pub fn task_template_exists(&self, task_name: &str) -> bool {
        let template_path = self
            .templates_dir
            .join("tasks")
            .join(format!("{}.json", task_name));
        template_path.exists()
    }
}

/// 便捷函数：创建默认管理器并获取系统提示词
pub fn get_system_prompt_for_role(role: &str, variables: &Value) -> Result<String> {
    let manager = PromptTemplateManager::default();
    manager.get_system_prompt(role, variables)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt_engineering::template::{Example, Variable};
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PromptTemplateManager::with_path(temp_dir.path()).unwrap();

        assert!(manager.templates_dir.exists());
    }

    #[test]
    fn test_template_registration() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PromptTemplateManager::with_path(temp_dir.path()).unwrap();

        let template = PromptTemplate::new(
            "test_id",
            "Test Template",
            "Planner",
            "You are a planner",
            "1.0.0",
        );

        manager.register_template(template.clone()).unwrap();

        // 验证文件已创建
        let template_path = temp_dir.path().join("roles/Planner.json");
        assert!(template_path.exists());

        // 验证可以重新加载
        let loaded = manager.load_template("Planner").unwrap();
        assert_eq!(loaded.id, template.id);
    }

    #[test]
    fn test_template_rendering() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PromptTemplateManager::with_path(temp_dir.path()).unwrap();

        let template = PromptTemplate::new(
            "test_id",
            "Test",
            "Executor",
            "You are {{role_name}} with tools: {{tools}}",
            "1.0.0",
        );

        manager.register_template(template).unwrap();

        let variables = json!({
            "role_name": "an executor",
            "tools": "read_file, write_file"
        });

        let result = manager.get_system_prompt("Executor", &variables).unwrap();
        assert!(result.contains("an executor"));
        assert!(result.contains("read_file, write_file"));
    }

    #[test]
    fn test_get_all_roles() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = PromptTemplateManager::with_path(temp_dir.path()).unwrap();

        // 注册几个模板
        for role in &["Planner", "Executor", "Reviewer"] {
            let template = PromptTemplate::new(
                format!("{}_id", role.to_lowercase()),
                role.to_string(),
                role.to_string(),
                format!("You are a {}", role.to_lowercase()),
                "1.0.0",
            );
            manager.register_template(template).unwrap();
        }

        let roles = manager.get_all_roles().unwrap();
        assert_eq!(roles.len(), 3);
        assert!(roles.contains(&"Planner".to_string()));
        assert!(roles.contains(&"Executor".to_string()));
        assert!(roles.contains(&"Reviewer".to_string()));
    }
}
