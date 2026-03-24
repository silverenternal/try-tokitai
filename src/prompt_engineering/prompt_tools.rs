//! 提示词工具集
//!
//! 将 prompt_engineering 模块封装为 tokitai ToolProvider

#![allow(dead_code)]

use tokitai::tool;
use tokitai::Value;
use super::manager::PromptTemplateManager;
use super::template::PromptTemplate;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::time::Instant;
use chrono::Local;

/// 模板渲染统计信息
#[derive(Debug, Clone, Default)]
pub struct RenderStats {
    pub total_renders: usize,
    pub successful_renders: usize,
    pub failed_renders: usize,
    pub avg_render_time_ms: f64,
    pub last_render_time: Option<chrono::DateTime<Local>>,
    pub renders_by_template: HashMap<String, usize>,
}

impl RenderStats {
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "total_renders": self.total_renders,
            "successful_renders": self.successful_renders,
            "failed_renders": self.failed_renders,
            "avg_render_time_ms": format!("{:.2}", self.avg_render_time_ms),
            "last_render_time": self.last_render_time.map(|t| t.to_rfc3339()),
            "renders_by_template": self.renders_by_template,
        })
    }
}

/// 提示词工具集
#[tool]
pub struct PromptTools {
    manager: Arc<RwLock<PromptTemplateManager>>,
    render_stats: Arc<RwLock<RenderStats>>,
}

impl PromptTools {
    /// 创建新的提示词工具集
    pub fn new() -> Result<Self, String> {
        let manager = PromptTemplateManager::default();
        
        Ok(Self {
            manager: Arc::new(RwLock::new(manager)),
            render_stats: Arc::new(RwLock::new(RenderStats::default())),
        })
    }

    /// 使用共享管理器创建工具集
    pub fn with_shared_manager(manager: Arc<RwLock<PromptTemplateManager>>) -> Self {
        Self {
            manager,
            render_stats: Arc::new(RwLock::new(RenderStats::default())),
        }
    }

    /// 获取共享管理器的引用
    pub fn get_manager(&self) -> Arc<RwLock<PromptTemplateManager>> {
        self.manager.clone()
    }

    /// 获取渲染统计
    fn get_render_stats_internal(&self) -> RenderStats {
        self.render_stats.read().clone()
    }

    /// 带统计的模板渲染
    fn render_with_stats(&self, template: &PromptTemplate, variables: &Value) -> Result<String, String> {
        let start = Instant::now();
        
        let manager = self.manager.read();
        let result = manager.render(template, variables);
        
        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        
        {
            let mut stats = self.render_stats.write();
            stats.total_renders += 1;
            stats.last_render_time = Some(Local::now());
            
            if result.is_ok() {
                stats.successful_renders += 1;
            } else {
                stats.failed_renders += 1;
            }

            let total_renders = stats.total_renders as f64;
            stats.avg_render_time_ms = 
                (stats.avg_render_time_ms * (total_renders - 1.0) + elapsed_ms) / total_renders;

            let template_key = format!("{}:{}", template.role, template.id);
            *stats.renders_by_template.entry(template_key).or_insert(0) += 1;
        }

        result.map_err(|e| format!("渲染模板失败：{}", e))
    }

    /// 预热模板缓存
    fn warmup_cache_internal(&self) -> Result<Vec<String>, String> {
        let manager = self.manager.read();
        let mut loaded = Vec::new();

        let roles = manager.get_all_roles()
            .unwrap_or_else(|_| Vec::new());
        for role in &roles {
            if manager.load_template(role).is_ok() {
                loaded.push(format!("role:{}", role));
            }
        }

        let tasks = manager.get_all_task_templates()
            .unwrap_or_else(|_| Vec::new());
        for task in &tasks {
            if manager.load_task_template(task).is_ok() {
                loaded.push(format!("task:{}", task));
            }
        }

        Ok(loaded)
    }
}

impl Default for PromptTools {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

#[tool]
impl PromptTools {
    /// 加载角色提示词模板
    #[tool(description = "加载指定角色的提示词模板")]
    pub fn load_role_template(&self, role: String) -> Result<String, String> {
        let manager = self.manager.read();
        let template = manager.load_template(&role)
            .map_err(|e| format!("加载角色模板失败：{}", e))?;
        Ok(template.system_prompt)
    }

    /// 列出所有可用模板
    #[tool(description = "列出所有可用的提示词模板")]
    pub fn list_available_templates(&self) -> Result<Value, String> {
        let manager = self.manager.read();
        
        let roles = manager.get_all_roles().unwrap_or_else(|_| Vec::new());
        let tasks = manager.get_all_task_templates().unwrap_or_else(|_| Vec::new());

        let role_details: Vec<Value> = roles
            .iter()
            .map(|role| {
                serde_json::json!({
                    "type": "role",
                    "name": role,
                })
            })
            .collect();

        let task_details: Vec<Value> = tasks
            .iter()
            .map(|task| {
                serde_json::json!({
                    "type": "task",
                    "name": task,
                })
            })
            .collect();

        Ok(serde_json::json!({
            "roles": role_details,
            "tasks": task_details,
            "total_roles": roles.len(),
            "total_tasks": tasks.len(),
        }))
    }

    /// 检查模板是否存在
    #[tool(description = "检查指定角色的模板是否存在")]
    pub fn has_template(&self, role: String) -> Result<bool, String> {
        let manager = self.manager.read();
        Ok(manager.template_exists(&role))
    }

    /// 渲染角色模板
    #[tool(description = "使用给定变量渲染角色模板")]
    pub fn render_template(&self, role: String, variables: Value) -> Result<String, String> {
        let manager = self.manager.read();
        let template = manager.load_template(&role)
            .map_err(|e| format!("加载模板失败：{}", role))?;
        
        self.render_with_stats(&template, &variables)
    }

    /// 渲染任务模板
    #[tool(description = "使用给定变量渲染任务模板")]
    pub fn render_task_template(&self, task_name: String, variables: Value) -> Result<String, String> {
        let manager = self.manager.read();
        let template = manager.load_task_template(&task_name)
            .map_err(|e| format!("加载任务模板失败：{}", task_name))?;
        
        self.render_with_stats(&template, &variables)
    }

    /// 清除模板缓存
    #[tool(description = "清除提示词缓存")]
    pub fn clear_cache(&self) -> Result<String, String> {
        let manager = self.manager.read();
        manager.clear_cache();
        Ok("模板缓存已清除".to_string())
    }

    /// 热加载模板
    #[tool(description = "重新从文件加载指定模板")]
    pub fn reload_template(&self, role: String) -> Result<String, String> {
        let manager = self.manager.read();
        manager.reload_template(&role)
            .map_err(|e| format!("重新加载模板失败：{}", role))?;
        Ok(format!("模板已重新加载：{}", role))
    }

    /// 获取渲染统计
    #[tool(description = "获取模板渲染统计信息")]
    pub fn get_render_stats(&self) -> Result<Value, String> {
        let stats = self.get_render_stats_internal();
        Ok(stats.to_json())
    }

    /// 预热模板缓存
    #[tool(description = "预先加载所有可用模板到缓存中")]
    pub fn warmup_cache(&self) -> Result<Value, String> {
        let loaded = self.warmup_cache_internal()?;
        Ok(serde_json::json!({
            "message": "模板缓存预热完成",
            "loaded_templates": loaded,
            "total_count": loaded.len(),
        }))
    }

    /// 获取所有角色
    #[tool(description = "获取所有可用的角色模板名称列表")]
    pub fn get_all_roles(&self) -> Result<Value, String> {
        let manager = self.manager.read();
        let roles = manager.get_all_roles().unwrap_or_else(|_| Vec::new());
        Ok(serde_json::json!(roles))
    }

    /// 获取所有任务模板
    #[tool(description = "获取所有可用的任务模板名称列表")]
    pub fn get_all_task_templates(&self) -> Result<Value, String> {
        let manager = self.manager.read();
        let tasks = manager.get_all_task_templates().unwrap_or_else(|_| Vec::new());
        Ok(serde_json::json!(tasks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        let tools = PromptTools::new().unwrap();
        let templates = tools.list_available_templates().unwrap();
        assert!(templates.is_object());
        assert!(templates.get("roles").is_some());
        assert!(templates.get("tasks").is_some());
    }

    #[test]
    fn test_shared_manager() {
        let shared_manager = Arc::new(RwLock::new(PromptTemplateManager::default()));
        let tools1 = PromptTools::with_shared_manager(shared_manager.clone());
        let tools2 = PromptTools::with_shared_manager(shared_manager.clone());

        assert!(Arc::ptr_eq(&tools1.get_manager(), &tools2.get_manager()));
    }
}
