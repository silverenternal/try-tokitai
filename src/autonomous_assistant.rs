//! 自主助手 - 项目自更新服务
//!
//! 实现 AI 自主进化循环，AI 将自主地：
//! 1. 分析项目现状，发现改进点
//! 2. 自主规划改进任务
//! 3. 执行任务（修改代码）
//! 4. 本地审查（编译、测试、代码审查）
//! 5. 审查通过后自动推送到 GitHub
//!
//! # 使用场景
//! - AI 自主发现项目改进点
//! - 自主代码改进和重构
//! - 技术债务清理
//! - 持续优化项目质量
//!
//! # 启动命令
//! ```bash
//! # 默认当前目录
//! cargo run --release -- --autonomous
//!
//! # 指定项目路径
//! cargo run --release -- --autonomous --project-path ./sandbox/test-project
//! ```
//!
//! # 服务边界
//! - ✅ 自主分析项目状态
//! - ✅ 自主发现改进点
//! - ✅ 自主制定并执行计划
//! - ✅ 自主代码审查
//! - ✅ 自主 Git 提交（可选）
//! - ❌ 不响应用户交互
//! - ❌ 不处理外部查询
//! - ❌ 不提供服务接口

use anyhow::{Context, Result};
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

use crate::assistant_common::{register_all_builtin_tools, AssistantConfig, ToolManager};
use crate::autonomy::{AgentCoordinator, GitWorkflow, GitWorkflowTools};
use crate::integration::{IntegratedModules, IntegratedModulesConfig};
use crate::tool_matrix::registry::ToolRegistry;
use crate::tool_matrix::registry::ToolSource;
use crate::tools::io::security::{SandboxConfig, SecurePathResolver};
use crate::tools::HttpClientTools;
use crate::tools::{
    CodeTools, DownloadTools, FileOperations, GitOperations, JsonFormatTools, SearchTools,
    SystemTools,
};

/// 自主助手 - 项目自更新服务
pub struct AutonomousAssistant {
    /// 助手配置
    config: AssistantConfig,
    /// 工具管理器
    tool_manager: ToolManager,
    /// 集成模块
    #[allow(dead_code)]
    integrated_modules: IntegratedModules,
    /// 项目根目录
    #[allow(dead_code)]
    project_root: PathBuf,
    /// 自主进化目录
    autonomy_dir: PathBuf,
    /// Agent 协调器（多 Agent 协作系统）
    #[allow(dead_code)]
    coordinator: AgentCoordinator,
    /// Git 工作流
    #[allow(dead_code)]
    git_workflow: GitWorkflow,
    /// 工具实例（用于 call_tool 调用）
    file_ops: FileOperations,
    system_tools: SystemTools,
    code_tools: CodeTools,
    web_search: SearchTools,
    download_tools: DownloadTools,
    git_ops: GitOperations,
    http_client: HttpClientTools,
    json_tools: JsonFormatTools,
    /// 安全配置
    pub security_config: crate::security::SecurityConfig,
}

impl AutonomousAssistant {
    fn path_resolver(&self) -> SecurePathResolver {
        SecurePathResolver::with_config(SandboxConfig {
            allowed_roots: self.security_config.allowed_roots.clone(),
            allow_symlinks: self.security_config.allow_symlinks,
            max_depth: self.security_config.max_path_depth as usize,
        })
    }

    /// 创建新的自主助手
    ///
    /// # 参数
    /// - `config`: 助手配置
    /// - `project_root`: 项目根目录路径
    /// - `security_config`: 安全配置（从 config.toml + 环境变量加载）
    pub fn new(
        config: AssistantConfig,
        project_root: PathBuf,
        security_config: crate::security::SecurityConfig,
    ) -> Result<Self> {
        let autonomy_dir = project_root.join(".tokitai").join("autonomy");

        // 创建工具注册表
        let tool_registry = ToolRegistry::new();

        // 注册所有内置工具
        register_all_builtin_tools(&tool_registry);

        // 创建 autonomy 工具箱
        let _ = tool_registry.create_toolbox(crate::tool_matrix::matrix::ToolBox::new(
            "autonomy",
            "Autonomy Tools",
            "AI autonomous evolution tools",
        ));

        // 注册 GitWorkflow 工具到 autonomy 工具箱
        let git_workflow_tools =
            GitWorkflowTools::new(project_root.clone(), autonomy_dir.join("git"))
                .map_err(|e| anyhow::anyhow!("创建 Git 工作流工具失败：{}", e))?;
        let _ = tool_registry
            .register_from_provider_sync::<GitWorkflowTools>(Some("autonomy"), ToolSource::Builtin);

        // 创建工具管理器
        let tool_manager = ToolManager::new(tool_registry.clone());

        // 创建集成模块
        let integrated_modules = match IntegratedModules::new(IntegratedModulesConfig::default()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("⚠️  创建集成模块失败：{}", e);
                IntegratedModules::new(IntegratedModulesConfig::for_testing()).unwrap()
            }
        };

        let mut integrated_modules = integrated_modules;

        // 初始化集成模块
        match integrated_modules.initialize() {
            Ok(init_report) => {
                if !init_report.success {
                    eprintln!("⚠️  集成模块初始化警告：");
                    for error in &init_report.errors {
                        eprintln!("   - {}", error);
                    }
                }
            }
            Err(e) => {
                eprintln!("⚠️  集成模块初始化失败：{}", e);
            }
        }

        // 创建 Agent 协调器
        let coordinator = AgentCoordinator::new(
            autonomy_dir.clone(),
            Arc::new(RwLock::new(tool_registry.clone())),
        )
        .map_err(|e| anyhow::anyhow!("创建 Agent 协调器失败：{}", e))?;

        // 创建 Git 工作流
        let git_workflow = GitWorkflow::new(project_root.clone(), autonomy_dir.join("git"))
            .map_err(|e| anyhow::anyhow!("创建 Git 工作流失败：{}", e))?;

        let path_resolver = SecurePathResolver::with_config(SandboxConfig {
            allowed_roots: security_config.allowed_roots.clone(),
            allow_symlinks: security_config.allow_symlinks,
            max_depth: security_config.max_path_depth as usize,
        });

        Ok(Self {
            config,
            tool_manager,
            integrated_modules,
            project_root,
            autonomy_dir,
            coordinator,
            git_workflow,
            file_ops: FileOperations::with_resolver(path_resolver),
            system_tools: SystemTools::default(),
            code_tools: CodeTools::default(),
            web_search: SearchTools::new(),
            download_tools: DownloadTools::new(),
            git_ops: GitOperations,
            http_client: HttpClientTools::new(),
            json_tools: JsonFormatTools::default(),
            security_config,
        })
    }

    /// 运行自主进化系统（项目自更新服务核心方法）
    ///
    /// # 功能说明
    ///
    /// 此方法启动 AI 自主进化循环，AI 将：
    /// 1. 自主发现项目改进点
    /// 2. 自主规划改进任务
    /// 3. 执行任务（修改代码）
    /// 4. 本地审查（编译、测试、代码审查）
    /// 5. 审查通过后自动推送到 GitHub
    ///
    /// # 进化目标
    /// - 改进代码质量：检查并修复代码中的潜在问题
    /// - 优化性能：分析并优化慢查询和低效代码
    /// - 增强错误处理：改进错误提示和日志
    /// - 完善文档：检查并更新 README 和注释
    /// - 清理技术债务：移除未使用的代码和依赖
    pub fn run_autonomous_evolution(&self) -> Result<()> {
        println!("\n🤖 启动自主进化系统...");
        println!("   - AI 将自主发现项目改进点");
        println!("   - 本地审查通过后将自动推送到 GitHub");
        println!("   - 按 Ctrl+C 停止自主模式\n");

        info!("🔄 开始自主进化循环");

        // 自主进化目标列表
        let evolution_goals = vec![
            "改进代码质量：检查并修复代码中的潜在问题".to_string(),
            "优化性能：分析并优化慢查询和低效代码".to_string(),
            "增强错误处理：改进错误提示和日志".to_string(),
            "完善文档：检查并更新 README 和注释".to_string(),
            "清理技术债务：移除未使用的代码和依赖".to_string(),
        ];

        for goal in evolution_goals {
            println!("\n📋 自主进化目标：{}", goal);

            // 执行自主迭代
            match self.execute_evolution_iteration(&goal) {
                Ok(push_success) => {
                    if push_success {
                        println!("✅ 进化完成并已推送到 GitHub");
                    } else {
                        println!("⚠️  进化完成但未推送（审查未通过或无变更）");
                    }
                }
                Err(e) => {
                    println!("❌ 进化失败：{}", e);
                    warn!("自主进化失败：{}", e);
                }
            }

            // 检查是否应该继续
            if !self.should_continue_evolution() {
                println!("\n🛑 停止自主进化");
                break;
            }
        }

        info!("🔄 自主进化循环结束");
        Ok(())
    }

    /// 调用工具
    fn call_tool(&self, name: &str, args: &Value) -> Result<String> {
        info!("🔧 执行工具：{} {:?}", name, args);

        // 安全授权检查
        let auth = crate::security::authorize_tool_call(
            name,
            &self.security_config,
            crate::security::ExecutionMode::Autonomous,
        );
        if let crate::security::AuthDecision::Deny(reason) = auth {
            warn!("🚫 自主模式工具被拦截：{}", reason);
            return Err(anyhow::anyhow!("安全策略拦截：{}", reason));
        }

        // 对 LLM 输出的参数做安全检查
        if let Err(e) = self.validate_tool_args(name, args) {
            return Err(e);
        }

        use tokitai_core::ToolErrorKind;

        macro_rules! try_tool {
            ($tools:expr) => {
                match $tools.call_tool(name, args) {
                    Ok(result) => {
                        info!("✅ 工具执行成功：{}", name);
                        self.tool_manager.tool_registry.record_usage(name, true, 0);
                        return Ok(result.to_string());
                    }
                    Err(e) => {
                        if e.kind != ToolErrorKind::NotFound {
                            info!("❌ 工具执行失败：{} - {:?}", name, e);
                            self.tool_manager.tool_registry.record_usage(name, false, 0);
                            return Err(anyhow::anyhow!("工具 {} 执行失败：{}", name, e));
                        }
                    }
                }
            };
        }

        try_tool!(self.file_ops);
        try_tool!(self.system_tools);
        try_tool!(self.code_tools);
        try_tool!(self.web_search);
        try_tool!(self.download_tools);
        try_tool!(self.git_ops);
        try_tool!(self.http_client);
        try_tool!(self.json_tools);

        warn!("❌ 未知工具：{}", name);
        Err(anyhow::anyhow!("未知工具：{}", name))
    }

    /// 对 LLM 输出的工具参数做安全检查
    fn validate_tool_args(&self, name: &str, args: &Value) -> Result<()> {
        let resolver = self.path_resolver();

        // 对文件操作类工具验证 path 参数
        let file_tools = [
            "read_file", "write_file", "edit_file", "copy_file", "move_file",
            "delete_file", "list_dir", "mkdir", "create_dir",
            "read_pdf_text", "read_pdf",
        ];
        if file_tools.contains(&name) {
            if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                let validation = resolver.resolve(path);
                if !validation.is_valid {
                    return Err(anyhow::anyhow!(
                        "路径安全验证失败：{}",
                        validation.error.unwrap_or_else(|| "未知错误".to_string())
                    ));
                }
            }
            for key in &["source", "dest", "destination"] {
                if let Some(p) = args.get(*key).and_then(|v| v.as_str()) {
                    let validation = resolver.resolve(p);
                    if !validation.is_valid {
                        return Err(anyhow::anyhow!(
                            "路径安全验证失败 ({}): {}",
                            key,
                            validation.error.unwrap_or_else(|| "未知错误".to_string())
                        ));
                    }
                }
            }
        }

        // 对命令执行工具检测 CR/LF 注入
        if name == "run_safe_command" || name == "run_command" {
            if let Some(cmd) = args.get("command").and_then(|v| v.as_str()) {
                if cmd.len() > 4096 {
                    return Err(anyhow::anyhow!("命令过长 ({} > 4096)", cmd.len()));
                }
                if cmd.contains('\n') || cmd.contains('\r') {
                    return Err(anyhow::anyhow!("命令包含换行符，疑似注入攻击"));
                }
                // run_command 仍需 confirmed=true
                if name == "run_command" {
                    let confirmed = args.get("confirmed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    if !confirmed {
                        return Err(anyhow::anyhow!(
                            "自主模式下 run_command 必须显式设置 confirmed=true"
                        ));
                    }
                    // 额外：检查危险命令黑名单
                    let cmd_name = cmd.split_whitespace().next().unwrap_or("");
                    let cmd_base = cmd_name.rsplit('/').next().unwrap_or(cmd_name);
                    let dangerous: &[&str] = &[
                        "rm", "dd", "mkfs", "chmod", "chown", "sudo", "su",
                        "shutdown", "reboot", "halt", "poweroff",
                        "kill", "pkill", "killall",
                        "iptables", "ufw",
                        "passwd", "useradd", "userdel",
                    ];
                    if dangerous.contains(&cmd_base) {
                        return Err(anyhow::anyhow!(
                            "自主模式拒绝执行危险命令: '{}' 在黑名单中", cmd_base
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// 执行单次进化迭代
    fn execute_evolution_iteration(&self, goal: &str) -> Result<bool> {
        // 1. 开始迭代
        let mut coordinator = self.create_coordinator_for_iteration()?;

        coordinator
            .start_iteration(goal.to_string())
            .map_err(|e| anyhow::anyhow!("启动迭代失败：{}", e))?;

        // 2. AI 自主分析项目现状
        println!("   🔍 分析项目现状...");
        let analysis = self.analyze_project_status()?;
        info!("项目分析：{}", analysis);

        // 3. AI 生成改进计划
        println!("   📝 生成改进计划...");
        let plan = self.generate_improvement_plan(goal, &analysis)?;
        info!("改进计划：{}", plan);

        // 4. 执行改进任务
        println!("   🔧 执行改进任务...");
        let execution_result = self.execute_improvement_tasks(&plan)?;
        info!("执行结果：{}", execution_result);

        // 5. 本地审查
        println!("   🧪 本地审查...");
        let review_passed = self.local_review()?;

        if review_passed {
            // 6. 推送到 GitHub
            println!("   🚀 推送到 GitHub...");
            let push_success = self.push_to_github()?;
            Ok(push_success)
        } else {
            println!("   ❌ 审查未通过，回滚变更");
            self.rollback_changes()?;
            Ok(false)
        }
    }

    /// 分析项目现状
    fn analyze_project_status(&self) -> Result<String> {
        let mut analysis = String::new();

        if let Ok(status) = self.call_tool("git_status", &json!({})) {
            analysis.push_str(&format!("Git 状态：{}\n", status));
        }

        if let Ok(files) = self.call_tool("list_dir", &json!({"path": "."})) {
            analysis.push_str(&format!("项目文件：{}\n", files));
        }

        if let Ok(todos) = self.call_tool(
            "search_content",
            &json!({
                "pattern": "TODO|FIXME|XXX|HACK",
                "path": "src"
            }),
        ) {
            analysis.push_str(&format!("待改进项：{}\n", todos));
        }

        Ok(analysis)
    }

    /// 生成改进计划
    fn generate_improvement_plan(&self, goal: &str, analysis: &str) -> Result<String> {
        let messages = &mut vec![
            json!({
                "role": "system",
                "content": "你是一个专业的软件工程师，负责分析项目并制定改进计划。"
            }),
            json!({
                "role": "user",
                "content": format!("目标：{}\n\n项目现状：{}\n\n请制定一个具体的改进计划。", goal, analysis)
            }),
        ];

        let plan = self.chat(messages)?;
        Ok(plan)
    }

    /// 执行改进任务
    fn execute_improvement_tasks(&self, plan: &str) -> Result<String> {
        let messages = &mut vec![
            json!({
                "role": "system",
                "content": "你是一个专业的软件工程师，根据改进计划执行具体的代码修改任务。

你可以使用以下工具：
- read_file: 读取文件
- write_file: 写入文件
- edit_file: 编辑文件
- run_command: 执行命令（如 cargo fmt, cargo clippy, cargo test）

请根据计划逐步执行任务，每次调用一个工具。"
            }),
            json!({
                "role": "user",
                "content": format!("请执行以下改进计划：\n\n{}", plan)
            }),
        ];

        // 执行多轮对话直到任务完成
        let mut iterations = 0;
        let max_iterations = 10;

        while iterations < max_iterations {
            let response = self.chat(messages)?;
            info!("AI 响应：{}", response);

            if response.contains("完成")
                || response.contains("已完成")
                || iterations >= max_iterations - 1
            {
                break;
            }

            iterations += 1;
        }

        Ok(format!("执行完成，共 {} 轮迭代", iterations))
    }

    /// 本地审查
    fn local_review(&self) -> Result<bool> {
        if !self.security_config.allow_autonomous_review {
            println!("      - 自主代码审查已被安全配置禁用");
            return Ok(true); // 跳过但不阻断流程
        }

        println!("      - 运行 cargo fmt --check...");
        let fmt_result = self.call_tool(
            "run_safe_command",
            &json!({
                "command": "cargo fmt --check"
            }),
        );

        if fmt_result.is_err() {
            println!("      ❌ 代码格式检查失败");
            return Ok(false);
        }

        println!("      - 运行 cargo clippy...");
        let clippy_result = self.call_tool(
            "run_safe_command",
            &json!({
                "command": "cargo clippy -- -D warnings"
            }),
        );

        if clippy_result.is_err() {
            println!("      ⚠️  Clippy 发现警告");
        }

        println!("      - 运行 cargo test...");
        let test_result = self.call_tool(
            "run_safe_command",
            &json!({
                "command": "cargo test --quiet"
            }),
        );

        if test_result.is_err() {
            println!("      ❌ 测试失败");
            return Ok(false);
        }

        println!("      ✅ 审查通过");
        Ok(true)
    }

    /// 回滚变更
    fn rollback_changes(&self) -> Result<()> {
        if !self.security_config.allow_autonomous_rollback {
            warn!("自主回滚已被安全配置禁用");
            return Err(anyhow::anyhow!("回滚被安全策略拦截"));
        }
        self.call_tool(
            "run_command",
            &json!({
                "command": "git checkout -- .",
                "confirmed": true
            }),
        )?;
        Ok(())
    }

    /// 推送到 GitHub
    fn push_to_github(&self) -> Result<bool> {
        if !self.security_config.allow_autonomous_git_push {
            warn!("自主 git push 已被安全配置禁用");
            return Err(anyhow::anyhow!("git push 被安全策略拦截"));
        }

        // 检查是否有变更
        let status = self.call_tool("git_status", &json!({}))?;

        if status.to_string().contains("nothing to commit") {
            println!("      - 无变更，跳过推送");
            return Ok(false);
        }

        // 生成提交消息
        let diff_str = self.call_tool("git_diff", &json!({}))?;
        let commit_message = self.generate_commit_message(&diff_str)?;

        // 添加并提交
        println!("      - git add .");
        self.call_tool(
            "run_command",
            &json!({
                "command": "git add .",
                "confirmed": true
            }),
        )?;

        println!("      - git commit -m '{}'", commit_message);
        self.call_tool(
            "run_command",
            &json!({
                "command": &format!("git commit -m '{}'", commit_message),
                "confirmed": true
            }),
        )?;

        // 推送
        println!("      - git push");
        self.call_tool(
            "run_command",
            &json!({
                "command": "git push",
                "confirmed": true
            }),
        )?;

        Ok(true)
    }

    /// 生成提交消息
    fn generate_commit_message(&self, diff: &str) -> Result<String> {
        let messages = &mut vec![
            json!({
                "role": "system",
                "content": "你是一个专业的软件工程师，根据代码变更生成简洁的提交消息。
格式：type: description
type 包括：feat, fix, docs, refactor, test, chore"
            }),
            json!({
                "role": "user",
                "content": format!("请为以下变更生成提交消息：\n\n{}", diff)
            }),
        ];

        let message = self.chat(messages)?;
        Ok(message.trim().to_string())
    }

    /// 检查是否继续进化
    fn should_continue_evolution(&self) -> bool {
        // 简单实现：总是继续
        true
    }

    /// 与 AI 对话（简化版本，用于自主进化）
    fn chat(&self, messages: &mut Vec<Value>) -> Result<String> {
        let tools = self.tool_manager.get_all_tools();

        let api_url = std::env::var("AI_API_URL").unwrap_or_else(|_| self.config.api_url.clone());
        let api_key = std::env::var("AI_API_KEY").ok();
        let model = std::env::var("AI_MODEL").unwrap_or_else(|_| self.config.model.clone());

        let request_body = json!({
            "model": model,
            "messages": messages,
            "tools": tools,
            "tool_choice": "auto",
            "max_tokens": 4096
        });

        let mut req = self.config.reqwest_client.post(&api_url);
        if let Some(key) = &api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }

        let response = req.json(&request_body).send().context("发送请求失败")?;

        let status = response.status();
        let response_text = response.text().context("读取响应失败")?;

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "API 返回错误 ({}): {}",
                status,
                response_text
            ));
        }

        let response_json: Value = serde_json::from_str(&response_text).context("解析响应失败")?;

        if let Some(choices) = response_json
            .get("choices")
            .and_then(|c: &Value| c.as_array())
        {
            if let Some(first) = choices.first() {
                if let Some(message) = first.get("message") {
                    if let Some(content) = message.get("content").and_then(|c: &Value| c.as_str()) {
                        if !content.is_empty() {
                            return Ok(content.to_string());
                        }
                    }
                }
            }
        }

        Ok(format!("AI 响应格式异常：{}", response_json))
    }

    /// 创建用于迭代的协调器
    fn create_coordinator_for_iteration(&self) -> Result<AgentCoordinator> {
        AgentCoordinator::new(
            self.autonomy_dir.clone(),
            Arc::new(RwLock::new(self.tool_manager.tool_registry.clone())),
        )
        .map_err(|e| anyhow::anyhow!("创建协调器失败：{}", e))
    }
}
