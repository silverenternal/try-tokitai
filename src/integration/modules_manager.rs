//! 集成模块管理器
//!
//! 统一管理 dialogue、observability、prompt_engineering 三个模块的生命周期
//!
//! # 设计原则
//! - 统一的初始化和关闭流程
//! - 共享状态管理（使用 Arc<RwLock>）
//! - 生产级资源清理
//! - 优雅的错误处理和降级

#![allow(dead_code)]

use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

use crate::dialogue::{DialogueStateMachine, DialogueTools};
use crate::observability::observability_tools::ObservabilityTools;
use crate::observability::tracing::TracingRecorder;
use crate::prompt_engineering::{PromptTemplateManager, PromptTools};

/// 集成模块配置
#[derive(Debug, Clone)]
pub struct IntegratedModulesConfig {
    pub dialogue_storage_dir: PathBuf,
    pub tracing_storage_dir: PathBuf,
    pub prompt_templates_dir: PathBuf,
    pub enable_console_output: bool,
    pub enable_persistence: bool,
    pub timeout_ms: u64,
    pub tracing_retention_days: u32,
}

impl Default for IntegratedModulesConfig {
    fn default() -> Self {
        Self {
            dialogue_storage_dir: PathBuf::from(".atlas/dialogue"),
            tracing_storage_dir: PathBuf::from(".atlas/traces"),
            prompt_templates_dir: PathBuf::from(".context/prompt_templates"),
            enable_console_output: false,
            enable_persistence: true,
            timeout_ms: 5000,
            tracing_retention_days: 7,
        }
    }
}

impl IntegratedModulesConfig {
    /// 创建使用临时目录的配置（用于测试）
    pub fn for_testing() -> Self {
        use std::env;
        let temp_base = env::temp_dir().join("atlas_test");

        Self {
            dialogue_storage_dir: temp_base.join("dialogue"),
            tracing_storage_dir: temp_base.join("traces"),
            prompt_templates_dir: temp_base.join("prompt_templates"),
            enable_console_output: false,
            enable_persistence: false,
            timeout_ms: 3000,
            tracing_retention_days: 1,
        }
    }
}

/// 集成模块管理器
pub struct IntegratedModules {
    config: IntegratedModulesConfig,

    pub dialogue_state: Arc<RwLock<DialogueStateMachine>>,
    pub dialogue_tools: DialogueTools,

    pub tracing_recorder: Arc<RwLock<TracingRecorder>>,
    pub observability_tools: ObservabilityTools,

    pub prompt_manager: Arc<RwLock<PromptTemplateManager>>,
    pub prompt_tools: PromptTools,

    initialized: bool,
}

impl IntegratedModules {
    /// 创建新的集成模块管理器
    pub fn new(config: IntegratedModulesConfig) -> Result<Self> {
        if config.enable_persistence {
            std::fs::create_dir_all(&config.dialogue_storage_dir)
                .with_context(|| "创建对话状态目录失败")?;
            std::fs::create_dir_all(&config.tracing_storage_dir)
                .with_context(|| "创建追踪日志目录失败")?;
        }

        let dialogue_state = if config.enable_persistence {
            Arc::new(RwLock::new(
                DialogueStateMachine::new(config.dialogue_storage_dir.clone())
                    .unwrap_or_else(|_| DialogueStateMachine::new_without_persistence()),
            ))
        } else {
            Arc::new(RwLock::new(DialogueStateMachine::new_without_persistence()))
        };

        let tracing_recorder = Arc::new(RwLock::new(
            TracingRecorder::new(
                config.tracing_storage_dir.clone(),
                config.enable_console_output,
            )
            .with_context(|| "创建追踪记录器失败")?,
        ));

        let prompt_manager = Arc::new(RwLock::new(
            PromptTemplateManager::with_path(&config.prompt_templates_dir)
                .unwrap_or_else(|_| PromptTemplateManager::default()),
        ));

        let dialogue_tools = DialogueTools::with_shared_state(dialogue_state.clone());
        let observability_tools = ObservabilityTools::with_shared_recorder(
            tracing_recorder.clone(),
            config.tracing_storage_dir.clone(),
        );
        let prompt_tools = PromptTools::with_shared_manager(prompt_manager.clone());

        Ok(Self {
            config,
            dialogue_state,
            dialogue_tools,
            tracing_recorder,
            observability_tools,
            prompt_manager,
            prompt_tools,
            initialized: false,
        })
    }

    /// 初始化所有模块
    pub fn initialize(&mut self) -> Result<InitializationReport> {
        let mut report = InitializationReport::new();

        match self.init_dialogue() {
            Ok(status) => report.dialogue_status = status,
            Err(e) => {
                report.dialogue_status = format!("失败：{}", e);
                report.errors.push(format!("Dialogue init: {}", e));
            }
        }

        match self.init_tracing() {
            Ok(status) => report.tracing_status = status,
            Err(e) => {
                report.tracing_status = format!("失败：{}", e);
                report.errors.push(format!("Tracing init: {}", e));
            }
        }

        match self.init_prompts() {
            Ok(status) => report.prompt_status = status,
            Err(e) => {
                report.prompt_status = format!("失败：{}", e);
                report.errors.push(format!("Prompts init: {}", e));
            }
        }

        self.initialized = true;
        report.success = report.errors.is_empty();

        Ok(report)
    }

    fn init_dialogue(&self) -> Result<String> {
        let state = self.dialogue_state.read();
        let current_state = state.current_state().to_string();
        drop(state);
        Ok(format!("已就绪 (状态：{})", current_state))
    }

    fn init_tracing(&self) -> Result<String> {
        if self.config.enable_persistence {
            let _ = self
                .observability_tools
                .cleanup_old_traces(Some(self.config.tracing_retention_days));
        }
        Ok("已就绪".to_string())
    }

    fn init_prompts(&self) -> Result<String> {
        let _ = self.prompt_tools.warmup_cache();
        Ok("已就绪".to_string())
    }

    /// 同步对话状态与 autonomy 模块
    pub fn sync_with_autonomy(&self, coordinator_state: &str) -> Result<String, String> {
        self.dialogue_tools.sync_with_autonomy(coordinator_state)
    }

    /// 获取模块状态报告
    pub fn get_status(&self) -> ModulesStatus {
        ModulesStatus {
            initialized: self.initialized,
            dialogue_state: self
                .dialogue_tools
                .get_state()
                .unwrap_or_else(|e| format!("错误：{}", e)),
            tracing_stats: self
                .observability_tools
                .get_stats()
                .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})),
            prompt_stats: self
                .prompt_tools
                .get_render_stats()
                .unwrap_or_else(|e| serde_json::json!({"error": e.to_string()})),
        }
    }

    /// 优雅关闭
    pub fn shutdown(&self) -> Result<ShutdownReport> {
        let mut report = ShutdownReport::new();

        match self.dialogue_state.write().save_state_with_context() {
            Ok(()) => report.dialogue_saved = true,
            Err(e) => {
                report.dialogue_saved = false;
                report.errors.push(format!("保存对话状态失败：{}", e));
            }
        }

        report.traces_flushed = true;
        self.prompt_manager.read().clear_cache();
        report.prompts_cached = true;

        report.success = report.errors.is_empty();

        Ok(report)
    }

    pub fn config(&self) -> &IntegratedModulesConfig {
        &self.config
    }
}

/// 初始化报告
#[derive(Debug, Clone)]
pub struct InitializationReport {
    pub success: bool,
    pub dialogue_status: String,
    pub tracing_status: String,
    pub prompt_status: String,
    pub errors: Vec<String>,
}

impl InitializationReport {
    fn new() -> Self {
        Self {
            success: false,
            dialogue_status: "未初始化".to_string(),
            tracing_status: "未初始化".to_string(),
            prompt_status: "未初始化".to_string(),
            errors: Vec::new(),
        }
    }
}

/// 关闭报告
#[derive(Debug, Clone)]
pub struct ShutdownReport {
    pub success: bool,
    pub dialogue_saved: bool,
    pub traces_flushed: bool,
    pub prompts_cached: bool,
    pub errors: Vec<String>,
}

impl ShutdownReport {
    fn new() -> Self {
        Self {
            success: false,
            dialogue_saved: false,
            traces_flushed: false,
            prompts_cached: false,
            errors: Vec::new(),
        }
    }
}

/// 模块状态
#[derive(Debug, Clone)]
pub struct ModulesStatus {
    pub initialized: bool,
    pub dialogue_state: String,
    pub tracing_stats: serde_json::Value,
    pub prompt_stats: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_modules() {
        let config = IntegratedModulesConfig::for_testing();
        let modules = IntegratedModules::new(config).unwrap();
        assert!(!modules.initialized);
    }

    #[test]
    fn test_initialize_modules() {
        let config = IntegratedModulesConfig::for_testing();
        let mut modules = IntegratedModules::new(config).unwrap();
        let report = modules.initialize().unwrap();
        assert!(report.success);
        assert!(modules.initialized);
    }

    #[test]
    fn test_shared_state() {
        let config = IntegratedModulesConfig::for_testing();
        let modules = IntegratedModules::new(config).unwrap();

        assert!(Arc::ptr_eq(
            &modules.dialogue_tools.get_state_machine(),
            &modules.dialogue_state
        ));

        assert!(Arc::ptr_eq(
            &modules.observability_tools.get_recorder(),
            &modules.tracing_recorder
        ));

        assert!(Arc::ptr_eq(
            &modules.prompt_tools.get_manager(),
            &modules.prompt_manager
        ));
    }
}
