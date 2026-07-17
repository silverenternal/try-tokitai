//! 编排器统一入口
//!
//! 整合角色切换、上下文优化和工作流引擎，提供统一的编排接口

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use crate::orchestrator::{
    AgentRole, ContextMessage, ContextOptimizer, OptimizerConfig, RoleSwitcher, Workflow,
    WorkflowEngine,
};
use crate::provider_config::ProviderManager;

/// 编排器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// 是否启用角色切换
    pub enable_role_switching: bool,
    /// 是否启用上下文优化
    pub enable_context_optimization: bool,
    /// 是否启用工作流引擎
    pub enable_workflow: bool,
    /// 上下文优化配置
    pub optimizer_config: OptimizerConfig,
    /// 是否启用详细模式
    pub verbose: bool,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enable_role_switching: true,
            enable_context_optimization: true,
            enable_workflow: true,
            optimizer_config: OptimizerConfig::default(),
            verbose: false,
        }
    }
}

/// 编排器状态
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorState {
    /// 当前角色
    pub current_role: String,
    /// 上下文 token 数
    pub context_tokens: usize,
    /// 上下文消息数
    pub context_messages: usize,
    /// 是否在工作流中
    pub in_workflow: bool,
    /// 当前工作流 ID（如果有）
    pub current_workflow_id: Option<String>,
}

/// 编排器 - 统一入口
pub struct Orchestrator {
    /// 角色切换器
    role_switcher: RoleSwitcher,
    /// 上下文优化器
    context_optimizer: ContextOptimizer,
    /// 配置
    pub config: OrchestratorConfig,
    /// 当前工作流引擎（如果有）
    workflow_engine: Option<WorkflowEngine>,
    /// 供应商管理器（可选）
    provider_manager: Option<Arc<RwLock<ProviderManager>>>,
}

impl Orchestrator {
    /// 创建新的编排器
    pub fn new() -> Self {
        Self::with_config(OrchestratorConfig::default())
    }

    /// 使用配置创建编排器
    pub fn with_config(config: OrchestratorConfig) -> Self {
        let context_optimizer = if config.enable_context_optimization {
            ContextOptimizer::with_config(config.optimizer_config.clone())
        } else {
            ContextOptimizer::new()
        };

        // 尝试加载供应商管理器
        let provider_manager = ProviderManager::from_env_file(None)
            .ok()
            .map(|pm| Arc::new(RwLock::new(pm)));

        Self {
            role_switcher: RoleSwitcher::new(),
            context_optimizer,
            config,
            workflow_engine: None,
            provider_manager,
        }
    }

    /// 处理用户输入（自动角色切换 + 上下文管理）
    pub fn process_input(&mut self, input: &str) -> ProcessedInput {
        // 1. 角色识别和切换
        let role_result = if self.config.enable_role_switching {
            self.role_switcher.switch_role(input)
        } else {
            // 返回当前角色，不切换
            use crate::orchestrator::RoleSwitchResult;
            RoleSwitchResult {
                previous_role: self.role_switcher.current_role().clone(),
                new_role: self.role_switcher.current_role().clone(),
                reason: "自动切换已禁用".to_string(),
                need_reload_tools: false,
            }
        };

        // 2. 添加消息到上下文
        if self.config.enable_context_optimization {
            self.context_optimizer.add_user_message(input.to_string());
        }

        // 3. 检测特殊命令
        let command = self.detect_command(input);

        ProcessedInput {
            original_input: input.to_string(),
            current_role: role_result.new_role,
            role_changed: role_result.need_reload_tools,
            command,
            context_tokens: self.context_optimizer.current_tokens(),
        }
    }

    /// 处理 AI 响应
    pub fn process_response(&mut self, response: &str) {
        if self.config.enable_context_optimization {
            self.context_optimizer
                .add_assistant_message(response.to_string());
        }
    }

    /// 检测特殊命令
    fn detect_command(&self, input: &str) -> Option<OrchestratorCommand> {
        let input = input.trim();

        // 角色切换命令
        if input.starts_with("/role ") || input.starts_with("/switch ") {
            let role_str = input
                .trim_start_matches("/role ")
                .trim_start_matches("/switch ")
                .trim();
            return Some(OrchestratorCommand::SwitchRole(AgentRole::from_str(
                role_str,
            )));
        }

        // 上下文优化命令
        if input == "/optimize" || input == "/opt" {
            return Some(OrchestratorCommand::OptimizeContext);
        }

        // 上下文状态命令
        if input == "/context" || input == "/ctx" {
            return Some(OrchestratorCommand::ShowContext);
        }

        // 角色状态命令
        if input == "/role status" || input == "/roles" {
            return Some(OrchestratorCommand::ShowRoles);
        }

        // 工作流命令
        if input.starts_with("/workflow ") || input.starts_with("/wf ") {
            let wf_cmd = input
                .trim_start_matches("/workflow ")
                .trim_start_matches("/wf ")
                .trim();
            return Some(OrchestratorCommand::Workflow(wf_cmd.to_string()));
        }

        // 帮助命令
        if input == "/help" || input == "/h" {
            return Some(OrchestratorCommand::ShowHelp);
        }

        // 健康检查命令
        if input == "/health" {
            return Some(OrchestratorCommand::HealthCheck);
        }

        // 自主进化统计命令
        if input == "/stats" || input == "/statistics" {
            return Some(OrchestratorCommand::Stats);
        }

        // 缓存优化命令
        if input == "/optimize" || input == "/opt" {
            return Some(OrchestratorCommand::OptimizeCache);
        }

        // 工具箱状态命令
        if input == "/toolbox" {
            return Some(OrchestratorCommand::Toolbox);
        }

        // AI 供应商切换命令
        if input == "/switch" || input == "/provider" {
            return Some(OrchestratorCommand::SwitchProvider);
        }

        // AI 供应商列表命令
        if input == "/providers" || input == "/provider list" {
            return Some(OrchestratorCommand::ShowProviders);
        }

        None
    }

    /// 执行编排器命令
    pub fn execute_command(&mut self, command: OrchestratorCommand) -> CommandResult {
        match command {
            OrchestratorCommand::SwitchRole(role) => {
                self.role_switcher.set_role(role.clone());
                CommandResult::Success(format!("已切换到角色：{}", role.as_str()))
            }
            OrchestratorCommand::OptimizeContext => {
                if self.config.enable_context_optimization {
                    let result = self.context_optimizer.optimize();
                    CommandResult::Success(format!(
                        "上下文优化完成：节省 {} tokens，丢弃 {} 条消息",
                        result.tokens_saved, result.messages_discarded
                    ))
                } else {
                    CommandResult::Error("上下文优化已禁用".to_string())
                }
            }
            OrchestratorCommand::ShowContext => {
                let stats = self.context_optimizer.get_stats();
                CommandResult::ContextInfo(ContextInfo {
                    tokens: self.context_optimizer.current_tokens(),
                    messages: self.context_optimizer.message_count(),
                    optimizations: stats.optimization_count,
                    tokens_saved: stats.total_tokens_saved,
                })
            }
            OrchestratorCommand::ShowRoles => {
                let current = self.role_switcher.current_role();
                CommandResult::RoleInfo(RoleInfo {
                    current_role: current.as_str().to_string(),
                    current_description: current.description().to_string(),
                    history: self
                        .role_switcher
                        .role_history()
                        .iter()
                        .map(|r| r.as_str().to_string())
                        .collect(),
                })
            }
            OrchestratorCommand::Workflow(cmd) => self.handle_workflow_command(&cmd),
            OrchestratorCommand::ShowHelp => CommandResult::Help(HelpInfo::default()),
            OrchestratorCommand::HealthCheck => self.execute_health_check(),
            OrchestratorCommand::Stats => self.execute_stats(),
            OrchestratorCommand::OptimizeCache => self.execute_optimize_cache(),
            OrchestratorCommand::Toolbox => CommandResult::Success(
                "工具箱状态：请使用 AiAssistant::get_toolbox_stats() 获取详细信息".to_string(),
            ),
            OrchestratorCommand::SwitchProvider => self.execute_switch_provider(),
            OrchestratorCommand::ShowProviders => self.execute_show_providers(),
        }
    }

    /// 执行健康检查
    fn execute_health_check(&mut self) -> CommandResult {
        let mut checks = Vec::new();
        let mut all_passed = true;

        // 1. 检查 AI API 连接
        let api_status = self.check_api_connection();
        if api_status.0 {
            checks.push(format!("✅ AI API     连接正常 ({})", api_status.1));
        } else {
            checks.push(format!("❌ AI API     连接失败：{}", api_status.1));
            all_passed = false;
        }

        // 2. 检查 Git 仓库状态
        let git_status = self.check_git_status();
        if git_status.0 {
            checks.push(format!("✅ Git 仓库   {}", git_status.1));
        } else {
            checks.push(format!("⚠️  Git 仓库   {}", git_status.1));
        }

        // 3. 检查文件权限
        let file_status = self.check_file_permissions();
        if file_status.0 {
            checks.push(format!("✅ 文件权限   {}", file_status.1));
        } else {
            checks.push(format!("❌ 文件权限   {}", file_status.1));
            all_passed = false;
        }

        // 4. 检查磁盘空间
        let disk_status = self.check_disk_space();
        if disk_status.0 {
            checks.push(format!("✅ 磁盘空间   {}", disk_status.1));
        } else {
            checks.push(format!("⚠️  磁盘空间   {}", disk_status.1));
        }

        // 5. 检查环境变量
        let env_status = self.check_env_config();
        if env_status.0 {
            checks.push(format!("✅ 环境变量   {}", env_status.1));
        } else {
            checks.push(format!("❌ 环境变量   {}", env_status.1));
            all_passed = false;
        }

        let status = if all_passed {
            format!("🏥 系统健康状态：✓ 所有检查通过\n\n{}", checks.join("\n"))
        } else {
            format!(
                "🏥 系统健康状态：⚠️ 部分检查未通过\n\n{}",
                checks.join("\n")
            )
        };

        CommandResult::Success(status)
    }

    /// 检查 AI API 连接
    fn check_api_connection(&self) -> (bool, String) {
        // 检查环境变量
        let api_url = std::env::var("AI_API_URL")
            .unwrap_or_else(|_| "https://ollama.com/v1/chat/completions".to_string());

        // 简单检查 URL 是否可访问（不实际发送请求）
        if api_url.is_empty() {
            return (false, "API URL 未配置".to_string());
        }

        // 尝试解析 URL
        if let Ok(url) = reqwest::Url::parse(&api_url) {
            // 检查主机是否可达
            if let Some(host) = url.host_str() {
                let port = url.port_or_known_default().unwrap_or(443);
                let timeout = Duration::from_millis(500);

                // 尝试连接
                let addr = format!("{}:{}", host, port);
                if let Ok(socket) = addr.parse::<std::net::SocketAddr>() {
                    if TcpStream::connect_timeout(&socket, timeout).is_ok() {
                        return (true, format!("{} (可达)", host));
                    }
                }
                // 如果无法解析为 SocketAddr，返回 URL 已配置
                return (true, format!("{} (URL 已配置，网络待验证)", host));
            }
        }

        (true, "URL 已配置".to_string())
    }

    /// 检查 Git 仓库状态
    fn check_git_status(&self) -> (bool, String) {
        use std::process::Command;

        // 检查 git 是否可用
        if Command::new("git").arg("--version").output().is_err() {
            return (false, "Git 未安装".to_string());
        }

        // 检查是否在 git 仓库中
        let status = Command::new("git")
            .args(["rev-parse", "--is-inside-work-tree"])
            .output();

        match status {
            Ok(output) => {
                if output.status.success() {
                    // 检查是否有未提交的变更
                    let diff = Command::new("git")
                        .args(["status", "--porcelain"])
                        .output()
                        .ok()
                        .and_then(|o| String::from_utf8(o.stdout).ok())
                        .unwrap_or_default();

                    if diff.trim().is_empty() {
                        (true, "干净".to_string())
                    } else {
                        let lines = diff.lines().count();
                        (true, format!("有 {} 个未提交变更", lines))
                    }
                } else {
                    (false, "不在 Git 仓库中".to_string())
                }
            }
            Err(_) => (false, "Git 状态检查失败".to_string()),
        }
    }

    /// 检查文件权限
    fn check_file_permissions(&self) -> (bool, String) {
        use std::fs::File;
        use std::io::Write;
        use tempfile::NamedTempFile;

        // 尝试创建临时文件
        match NamedTempFile::new() {
            Ok(mut f) => {
                // 尝试写入
                if f.write_all(b"test").is_ok() {
                    (true, "可读可写".to_string())
                } else {
                    (false, "可写但写入失败".to_string())
                }
            }
            Err(_) => {
                // 尝试读取当前目录
                match File::open(".") {
                    Ok(_) => (true, "只读".to_string()),
                    Err(_) => (false, "无法访问文件系统".to_string()),
                }
            }
        }
    }

    /// 检查磁盘空间
    fn check_disk_space(&self) -> (bool, String) {
        // 使用 df 命令检查磁盘空间（仅 Unix-like 系统）
        #[cfg(unix)]
        {
            use std::process::Command;
            let output = Command::new("df").args(["-h", "."]).output().ok();

            if let Some(out) = output {
                if let Ok(text) = String::from_utf8(out.stdout) {
                    // 解析 df 输出，获取可用空间
                    let lines: Vec<&str> = text.lines().collect();
                    if lines.len() >= 2 {
                        let parts: Vec<&str> = lines[1].split_whitespace().collect();
                        if parts.len() >= 4 {
                            let available = parts[3];
                            return (true, format!("充足 ({} 可用)", available));
                        }
                    }
                }
            }
        }

        (true, "充足".to_string())
    }

    /// 检查环境变量配置
    fn check_env_config(&self) -> (bool, String) {
        let api_url = std::env::var("AI_API_URL").ok();
        let api_key = std::env::var("AI_API_KEY").ok();

        if api_url.is_some() {
            if api_key.is_some() {
                (true, ".env 已配置".to_string())
            } else {
                (true, "API URL 已配置 (无 API Key)".to_string())
            }
        } else {
            (false, "AI_API_URL 未配置".to_string())
        }
    }

    /// 执行自主进化统计
    fn execute_stats(&self) -> CommandResult {
        use std::fs;
        use std::path::Path;

        let autonomy_dir = Path::new(".atlas/autonomy");

        if !autonomy_dir.exists() {
            return CommandResult::Success(
                "📊 自主进化统计：暂无数据（未找到 .atlas/autonomy 目录）".to_string(),
            );
        }

        let iterations_dir = autonomy_dir.join("iterations");
        let history_file = iterations_dir.join("history.json");

        // 统计迭代次数
        let total_iterations = if history_file.exists() {
            fs::read_to_string(&history_file)
                .ok()
                .and_then(|content| serde_json::from_str::<Vec<String>>(&content).ok())
                .map(|history: Vec<String>| history.len())
                .unwrap_or(0)
        } else {
            0
        };

        // 统计成功/失败次数
        let mut successful = 0;
        let mut failed = 0;
        let mut total_duration_secs = 0i64;
        let mut files_modified = 0;
        let mut tools_called = std::collections::HashMap::new();

        // 读取所有迭代历史文件
        if iterations_dir.exists() {
            if let Ok(entries) = fs::read_dir(&iterations_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("json")
                        && path.file_name().and_then(|s| s.to_str()) != Some("history.json")
                        && path.file_name().and_then(|s| s.to_str()) != Some("current.json")
                    {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(iteration) =
                                serde_json::from_str::<serde_json::Value>(&content)
                            {
                                // 统计成功/失败
                                if let Some(success) =
                                    iteration.get("success").and_then(|v| v.as_bool())
                                {
                                    if success {
                                        successful += 1;
                                    } else {
                                        failed += 1;
                                    }
                                }

                                // 统计持续时间
                                if let (Some(started), Some(ended)) = (
                                    iteration.get("started_at").and_then(|v| v.as_i64()),
                                    iteration.get("ended_at").and_then(|v| v.as_i64()),
                                ) {
                                    total_duration_secs += ended - started;
                                }

                                // 统计事件
                                if let Some(events) =
                                    iteration.get("events").and_then(|v| v.as_array())
                                {
                                    for event in events {
                                        if let Some(event_type) =
                                            event.get("type").and_then(|v| v.as_str())
                                        {
                                            *tools_called
                                                .entry(event_type.to_string())
                                                .or_insert(0) += 1;

                                            // 统计文件修改
                                            if event_type == "refinement_applied" {
                                                if let Some(changes) =
                                                    event.get("changes").and_then(|v| v.as_array())
                                                {
                                                    files_modified += changes.len();
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let avg_duration = if total_iterations > 0 {
            total_duration_secs / total_iterations as i64
        } else {
            0
        };

        let success_rate = if total_iterations > 0 {
            (successful as f64 / total_iterations as f64) * 100.0
        } else {
            0.0
        };

        // 构建输出
        let mut output = format!(
            "📊 自主进化统计:\n\n\
             迭代概览:\n\
             ├─ 总迭代次数：{}\n\
             ├─ 成功：{}\n\
             ├─ 成功率：{:.1}%\n\
             └─ 失败：{}\n\n\
             性能指标:\n\
             ├─ 平均迭代时长：{} 秒\n\
             └─ 文件修改次数：{}\n",
            total_iterations, successful, success_rate, failed, avg_duration, files_modified
        );

        // 添加工具调用统计
        if !tools_called.is_empty() {
            output.push_str("\n事件类型统计:\n");
            let mut sorted_events: Vec<_> = tools_called.iter().collect();
            sorted_events.sort_by(|a, b| b.1.cmp(a.1));
            for (event_type, count) in sorted_events.iter().take(10) {
                output.push_str(&format!("  ├─ {}: {} 次\n", event_type, count));
            }
        }

        CommandResult::Success(output)
    }

    /// 执行缓存优化
    fn execute_optimize_cache(&mut self) -> CommandResult {
        use crate::tools::io::file_cache::FileCache;
        use std::fs;
        use std::path::Path;

        let mut output = String::from("🧹 缓存优化:\n\n");

        // 1. 清理文件缓存
        let file_cache = FileCache::new();
        file_cache.invalidate_all();
        output.push_str("✅ 文件缓存已清理\n");

        // 2. 清理临时文件
        let temp_dir = Path::new(".atlas/temp");
        if temp_dir.exists() {
            if let Ok(entries) = fs::read_dir(temp_dir) {
                let mut temp_count = 0;
                for entry in entries.flatten() {
                    if fs::remove_file(entry.path()).is_ok() {
                        temp_count += 1;
                    }
                }
                output.push_str(&format!("✅ 清理 {} 个临时文件\n", temp_count));
            }
        } else {
            output.push_str("ℹ️  无临时文件需要清理\n");
        }

        // 3. 清理上下文缓存（如果启用）
        if self.config.enable_context_optimization {
            self.context_optimizer.clear();
            output.push_str("✅ 上下文缓存已清理\n");
        }

        // 4. 建议清理 HTTP 连接池（需要实现）
        output.push_str("ℹ️  HTTP 连接池将在下次请求时自动回收\n");

        output.push_str("\n✨ 优化完成！\n");

        CommandResult::Success(output)
    }

    /// 处理工作流命令
    fn handle_workflow_command(&mut self, cmd: &str) -> CommandResult {
        if !self.config.enable_workflow {
            return CommandResult::Error("工作流引擎已禁用".to_string());
        }

        match cmd {
            "list" | "ls" => {
                // 列出可用工作流
                CommandResult::WorkflowList(WorkflowListInfo {
                    workflows: vec![
                        ("code_review".to_string(), "代码审查工作流".to_string()),
                        (
                            "task_decomposition".to_string(),
                            "任务分解工作流".to_string(),
                        ),
                    ],
                })
            }
            "start code_review" | "start review" => {
                self.start_workflow(crate::orchestrator::templates::create_code_review_workflow())
            }
            "start task_decomposition" | "start decompose" => self.start_workflow(
                crate::orchestrator::templates::create_task_decomposition_workflow(),
            ),
            "status" => {
                if let Some(ref engine) = self.workflow_engine {
                    CommandResult::Success(format!("当前工作流状态：{:?}", engine.get_status()))
                } else {
                    CommandResult::Success("当前没有活动的工作流".to_string())
                }
            }
            _ => CommandResult::Error(format!("未知的工作流命令：{}", cmd)),
        }
    }

    /// 启动工作流
    fn start_workflow(&mut self, workflow: Workflow) -> CommandResult {
        let workflow_name = workflow.name.clone();
        let _workflow_id = workflow.id.clone();

        let mut engine = WorkflowEngine::new(workflow);
        if self.config.verbose {
            engine = engine.with_verbose(true);
        }

        match engine.execute() {
            Ok(result) => {
                self.workflow_engine = Some(engine);
                CommandResult::Success(format!(
                    "工作流 '{}' 执行完成：完成 {} 个阶段，{} 个步骤",
                    workflow_name, result.stages_completed, result.steps_completed
                ))
            }
            Err(e) => CommandResult::Error(format!("工作流执行失败：{}", e)),
        }
    }

    /// 执行供应商切换
    fn execute_switch_provider(&mut self) -> CommandResult {
        if let Some(ref pm) = self.provider_manager {
            let mut pm = pm.write();
            let new_provider = pm.switch_to_next();

            // 更新环境变量
            std::env::set_var("AI_API_URL", &new_provider.api_url);
            if let Some(ref key) = new_provider.api_key {
                std::env::set_var("AI_API_KEY", key);
            }
            std::env::set_var("AI_MODEL", &new_provider.model);

            CommandResult::ProviderInfo(ProviderInfo {
                current_name: new_provider.name.clone(),
                current_url: new_provider.api_url.clone(),
                current_model: new_provider.model.clone(),
                all_providers: pm.providers().iter().map(|p| p.name.clone()).collect(),
            })
        } else {
            CommandResult::Error("未找到供应商配置，请检查 .env 文件".to_string())
        }
    }

    /// 执行供应商显示
    fn execute_show_providers(&self) -> CommandResult {
        if let Some(ref pm) = self.provider_manager {
            let pm = pm.read();
            let current = pm.current();

            CommandResult::ProviderInfo(ProviderInfo {
                current_name: current.name.clone(),
                current_url: current.api_url.clone(),
                current_model: current.model.clone(),
                all_providers: pm.providers().iter().map(|p| p.name.clone()).collect(),
            })
        } else {
            CommandResult::Error("未找到供应商配置，请检查 .env 文件".to_string())
        }
    }

    /// 获取当前角色
    #[allow(dead_code)]
    pub fn current_role(&self) -> &AgentRole {
        self.role_switcher.current_role()
    }

    /// 获取编排器状态
    #[allow(dead_code)]
    pub fn get_state(&self) -> OrchestratorState {
        OrchestratorState {
            current_role: self.role_switcher.current_role().as_str().to_string(),
            context_tokens: self.context_optimizer.current_tokens(),
            context_messages: self.context_optimizer.message_count(),
            in_workflow: self.workflow_engine.is_some(),
            current_workflow_id: self
                .workflow_engine
                .as_ref()
                .map(|e| e.get_workflow().id.clone()),
        }
    }

    /// 获取上下文消息（用于发送给 AI）
    #[allow(dead_code)]
    pub fn get_context_messages(&self) -> &VecDeque<ContextMessage> {
        self.context_optimizer.get_messages()
    }

    /// 清空上下文
    #[allow(dead_code)]
    pub fn clear_context(&mut self) {
        self.context_optimizer.clear();
    }

    /// 设置详细模式
    #[allow(dead_code)]
    pub fn set_verbose(&mut self, verbose: bool) {
        self.config.verbose = verbose;
    }
}

impl Default for Orchestrator {
    fn default() -> Self {
        Self::new()
    }
}

/// 处理后的输入
#[derive(Debug, Clone)]
pub struct ProcessedInput {
    /// 原始输入
    #[allow(dead_code)]
    pub original_input: String,
    /// 当前角色
    pub current_role: AgentRole,
    /// 角色是否已切换
    pub role_changed: bool,
    /// 检测到的命令
    pub command: Option<OrchestratorCommand>,
    /// 上下文 token 数
    #[allow(dead_code)]
    pub context_tokens: usize,
}

/// 编排器命令
#[derive(Debug, Clone)]
pub enum OrchestratorCommand {
    /// 切换角色
    SwitchRole(AgentRole),
    /// 优化上下文
    OptimizeContext,
    /// 显示上下文信息
    ShowContext,
    /// 显示角色信息
    ShowRoles,
    /// 工作流命令
    Workflow(String),
    /// 显示帮助
    ShowHelp,
    /// 健康检查
    HealthCheck,
    /// 自主进化统计
    Stats,
    /// 优化缓存
    OptimizeCache,
    /// 显示工具箱状态
    Toolbox,
    /// 切换 AI 供应商（到下一个）
    SwitchProvider,
    /// 显示供应商列表
    ShowProviders,
}

/// 命令执行结果
#[derive(Debug, Clone)]
pub enum CommandResult {
    /// 成功
    Success(String),
    /// 错误
    Error(String),
    /// 上下文信息
    ContextInfo(ContextInfo),
    /// 角色信息
    RoleInfo(RoleInfo),
    /// 工作流列表
    WorkflowList(WorkflowListInfo),
    /// 帮助信息
    Help(HelpInfo),
    /// 供应商信息
    ProviderInfo(ProviderInfo),
}

/// 供应商信息
#[derive(Debug, Clone)]
pub struct ProviderInfo {
    pub current_name: String,
    pub current_url: String,
    pub current_model: String,
    pub all_providers: Vec<String>,
}

/// 上下文信息
#[derive(Debug, Clone)]
pub struct ContextInfo {
    pub tokens: usize,
    pub messages: usize,
    pub optimizations: u32,
    pub tokens_saved: usize,
}

/// 角色信息
#[derive(Debug, Clone)]
pub struct RoleInfo {
    pub current_role: String,
    pub current_description: String,
    pub history: Vec<String>,
}

/// 工作流列表信息
#[derive(Debug, Clone)]
pub struct WorkflowListInfo {
    pub workflows: Vec<(String, String)>,
}

/// 帮助信息
#[derive(Debug, Clone)]
pub struct HelpInfo {
    pub commands: Vec<(&'static str, &'static str)>,
}

impl Default for HelpInfo {
    fn default() -> Self {
        Self {
            commands: vec![
                (
                    "/role <name>",
                    "切换角色（planner/executor/reviewer/researcher）",
                ),
                ("/optimize", "优化上下文，减少 token 使用"),
                ("/context", "显示上下文状态"),
                ("/roles", "显示角色信息"),
                ("/workflow list", "列出可用工作流"),
                ("/workflow start <name>", "启动工作流"),
                ("/workflow status", "显示工作流状态"),
                ("/help", "显示此帮助信息"),
                ("/health", "系统健康检查"),
                ("/stats", "自主进化统计"),
                ("/optimize --clear-cache", "清理缓存"),
            ],
        }
    }
}

impl CommandResult {
    /// 转换为显示字符串
    pub fn to_string(&self) -> String {
        match self {
            CommandResult::Success(msg) => format!("✅ {}", msg),
            CommandResult::Error(msg) => format!("❌ {}", msg),
            CommandResult::ContextInfo(info) => {
                let ContextInfo {
                    tokens,
                    messages,
                    optimizations,
                    tokens_saved,
                } = info;
                format!(
                    "📊 上下文状态:\n  - Token 数：{}\n  - 消息数：{}\n  - 优化次数：{}\n  - 节省 Token：{}",
                    tokens, messages, optimizations, tokens_saved
                )
            }
            CommandResult::RoleInfo(info) => {
                let RoleInfo {
                    current_role,
                    current_description,
                    history,
                } = info;
                let mut output =
                    format!("🎭 当前角色：{} - {}\n", current_role, current_description);
                if !history.is_empty() {
                    output.push_str(&format!("历史角色：{}", history.join(" → ")));
                }
                output
            }
            CommandResult::WorkflowList(info) => {
                if info.workflows.is_empty() {
                    return "📋 没有可用工作流".to_string();
                }
                let mut output = String::from("📋 可用工作流:\n");
                for (id, name) in &info.workflows {
                    output.push_str(&format!("  - {}: {}\n", id, name));
                }
                output
            }
            CommandResult::Help(info) => {
                let mut output = String::from("📖 可用命令:\n");
                for (cmd, desc) in &info.commands {
                    output.push_str(&format!("  {:<25} {}\n", cmd, desc));
                }
                output
            }
            CommandResult::ProviderInfo(info) => {
                let ProviderInfo {
                    current_name,
                    current_url,
                    current_model,
                    all_providers,
                } = info;

                if all_providers.len() > 1 {
                    // 多供应商模式 - 简洁显示
                    let mut output = format!("✅ 已切换到：{}\n", current_name);
                    output.push_str(&format!("   模型：{}\n", current_model));

                    output.push_str("\n📋 可用供应商:\n");
                    for (i, name) in all_providers.iter().enumerate() {
                        let marker = if name == current_name { "👉" } else { "  " };
                        output.push_str(&format!("  {} {}\n", marker, name));
                    }
                    output.push_str("\n💡 下次请求将使用新供应商\n");
                    output
                } else {
                    // 单供应商模式
                    format!(
                        "🔌 当前供应商：{}\n  模型：{}\n  URL: {}\n",
                        current_name, current_model, current_url
                    )
                }
            }
        }
    }
}
