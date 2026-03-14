//! 角色切换器
//!
//! 实现基于任务类型的自动角色切换

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 核心角色定义
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// 规划师 - 任务分解和规划
    Planner,
    /// 执行师 - 按计划执行任务
    Executor,
    /// 审查师 - 代码审查和质量把关
    Reviewer,
    /// 研究员 - 信息收集和调研
    Researcher,
    /// 通用角色 - 未分类时使用
    General,
}

impl AgentRole {
    /// 从字符串解析角色
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "planner" | "规划师" => AgentRole::Planner,
            "executor" | "执行师" => AgentRole::Executor,
            "reviewer" | "审查师" => AgentRole::Reviewer,
            "researcher" | "研究员" => AgentRole::Researcher,
            _ => AgentRole::General,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentRole::Planner => "Planner",
            AgentRole::Executor => "Executor",
            AgentRole::Reviewer => "Reviewer",
            AgentRole::Researcher => "Researcher",
            AgentRole::General => "General",
        }
    }

    /// 获取角色描述
    pub fn description(&self) -> &'static str {
        match self {
            AgentRole::Planner => "任务分解和规划",
            AgentRole::Executor => "按计划执行任务",
            AgentRole::Reviewer => "代码审查和质量把关",
            AgentRole::Researcher => "信息收集和调研",
            AgentRole::General => "通用助手",
        }
    }
}

/// 角色切换结果
#[derive(Debug, Clone)]
pub struct RoleSwitchResult {
    /// 原角色
    pub previous_role: AgentRole,
    /// 新角色
    pub new_role: AgentRole,
    /// 切换原因
    pub reason: String,
    /// 是否需要重新加载工具
    pub need_reload_tools: bool,
}

/// 角色切换器
pub struct RoleSwitcher {
    /// 当前角色
    current_role: AgentRole,
    /// 角色历史
    role_history: Vec<AgentRole>,
    /// 角色决策矩阵（关键词到角色的映射）
    decision_matrix: HashMap<Vec<String>, AgentRole>,
    /// 是否启用自动切换
    auto_switch_enabled: bool,
    /// 手动覆盖的角色（如果设置则忽略自动判断）
    manual_override: Option<AgentRole>,
}

impl Default for RoleSwitcher {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleSwitcher {
    /// 创建新的角色切换器
    pub fn new() -> Self {
        let mut switcher = Self {
            current_role: AgentRole::General,
            role_history: Vec::new(),
            decision_matrix: HashMap::new(),
            auto_switch_enabled: true,
            manual_override: None,
        };

        // 初始化决策矩阵
        switcher.init_decision_matrix();

        switcher
    }

    /// 初始化决策矩阵
    fn init_decision_matrix(&mut self) {
        // 规划类任务关键词
        self.decision_matrix.insert(
            vec![
                "计划".to_string(),
                "规划".to_string(),
                "设计".to_string(),
                "方案".to_string(),
                "分析".to_string(),
                "架构".to_string(),
                "strategy".to_string(),
                "plan".to_string(),
                "design".to_string(),
                "analyze".to_string(),
            ],
            AgentRole::Planner,
        );

        // 执行类任务关键词
        self.decision_matrix.insert(
            vec![
                "执行".to_string(),
                "实现".to_string(),
                "创建".to_string(),
                "修改".to_string(),
                "删除".to_string(),
                "运行".to_string(),
                "构建".to_string(),
                "implement".to_string(),
                "create".to_string(),
                "build".to_string(),
                "run".to_string(),
            ],
            AgentRole::Executor,
        );

        // 审查类任务关键词
        self.decision_matrix.insert(
            vec![
                "审查".to_string(),
                "检查".to_string(),
                "审核".to_string(),
                "review".to_string(),
                "check".to_string(),
                "audit".to_string(),
                "优化".to_string(),
                "改进".to_string(),
                "optimize".to_string(),
                "improve".to_string(),
            ],
            AgentRole::Reviewer,
        );

        // 调研类任务关键词
        self.decision_matrix.insert(
            vec![
                "搜索".to_string(),
                "调研".to_string(),
                "研究".to_string(),
                "查找".to_string(),
                "了解".to_string(),
                "search".to_string(),
                "research".to_string(),
                "find".to_string(),
                "learn".to_string(),
                "explore".to_string(),
            ],
            AgentRole::Researcher,
        );
    }

    /// 分析输入并识别合适的角色
    pub fn identify_role(&self, input: &str) -> AgentRole {
        // 如果有手动覆盖，返回覆盖的角色
        if let Some(override_role) = &self.manual_override {
            return override_role.clone();
        }

        // 如果不启用自动切换，返回当前角色
        if !self.auto_switch_enabled {
            return self.current_role.clone();
        }

        let input_lower = input.to_lowercase();

        // 检查是否包含角色切换命令
        if let Some(role) = self.parse_role_command(&input_lower) {
            return role;
        }

        // 匹配决策矩阵
        let mut best_match = AgentRole::General;
        let mut best_score = 0;

        for (keywords, role) in &self.decision_matrix {
            let score = keywords
                .iter()
                .filter(|kw| input_lower.contains(&kw.to_lowercase()))
                .count();

            if score > best_score {
                best_score = score;
                best_match = role.clone();
            }
        }

        // 至少需要匹配 1 个关键词
        if best_score > 0 {
            best_match
        } else {
            AgentRole::General
        }
    }

    /// 解析角色切换命令（如 "/role planner"）
    fn parse_role_command(&self, input: &str) -> Option<AgentRole> {
        // 支持多种命令格式
        let prefixes = ["/role ", "/switch ", "切换角色 ", "使用 ", "as "];

        for prefix in &prefixes {
            if input.starts_with(prefix) {
                let role_str = input.trim_start_matches(prefix).trim();
                return Some(AgentRole::from_str(role_str));
            }
        }

        // 检查是否包含角色名称
        if input.contains("planner") || input.contains("规划师") {
            return Some(AgentRole::Planner);
        }
        if input.contains("executor") || input.contains("执行师") {
            return Some(AgentRole::Executor);
        }
        if input.contains("reviewer") || input.contains("审查师") {
            return Some(AgentRole::Reviewer);
        }
        if input.contains("researcher") || input.contains("研究员") {
            return Some(AgentRole::Researcher);
        }

        None
    }

    /// 切换角色
    pub fn switch_role(&mut self, input: &str) -> RoleSwitchResult {
        let previous_role = self.current_role.clone();
        let new_role = self.identify_role(input);

        let need_reload_tools = previous_role != new_role;

        if need_reload_tools {
            // 记录到历史
            self.role_history.push(previous_role.clone());

            // 更新当前角色
            self.current_role = new_role.clone();
        }

        RoleSwitchResult {
            previous_role,
            new_role,
            reason: format!("根据输入 '{}' 识别角色", input),
            need_reload_tools,
        }
    }

    /// 手动设置角色
    pub fn set_role(&mut self, role: AgentRole) {
        let previous = self.current_role.clone();
        let role_clone = role.clone();
        if previous != role {
            self.role_history.push(previous);
        }
        self.current_role = role;
        self.manual_override = Some(role_clone);
    }

    /// 清除手动覆盖
    pub fn clear_override(&mut self) {
        self.manual_override = None;
    }

    /// 启用/禁用自动切换
    pub fn set_auto_switch(&mut self, enabled: bool) {
        self.auto_switch_enabled = enabled;
    }

    /// 获取当前角色
    pub fn current_role(&self) -> &AgentRole {
        &self.current_role
    }

    /// 获取角色历史
    pub fn role_history(&self) -> &[AgentRole] {
        &self.role_history
    }

    /// 获取上一个角色
    pub fn previous_role(&self) -> Option<&AgentRole> {
        self.role_history.last()
    }

    /// 添加决策规则
    pub fn add_decision_rule(&mut self, keywords: Vec<String>, role: AgentRole) {
        self.decision_matrix.insert(keywords, role);
    }
}

/// 角色状态（用于序列化）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleState {
    pub current_role: String,
    pub role_history: Vec<String>,
    pub auto_switch_enabled: bool,
}

impl RoleState {
    pub fn from_switcher(switcher: &RoleSwitcher) -> Self {
        Self {
            current_role: switcher.current_role.as_str().to_string(),
            role_history: switcher
                .role_history
                .iter()
                .map(|r| r.as_str().to_string())
                .collect(),
            auto_switch_enabled: switcher.auto_switch_enabled,
        }
    }

    pub fn to_switcher(&self) -> RoleSwitcher {
        let mut switcher = RoleSwitcher::new();
        switcher.current_role = AgentRole::from_str(&self.current_role);
        switcher.role_history = self
            .role_history
            .iter()
            .map(|s| AgentRole::from_str(s))
            .collect();
        switcher.auto_switch_enabled = self.auto_switch_enabled;
        switcher
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_identification() {
        let switcher = RoleSwitcher::new();

        // 测试规划类
        assert_eq!(
            switcher.identify_role("帮我规划一下项目架构"),
            AgentRole::Planner
        );

        // 测试执行类
        assert_eq!(
            switcher.identify_role("创建一个新文件"),
            AgentRole::Executor
        );

        // 测试审查类
        assert_eq!(
            switcher.identify_role("审查这段代码"),
            AgentRole::Reviewer
        );

        // 测试调研类
        assert_eq!(
            switcher.identify_role("搜索相关信息"),
            AgentRole::Researcher
        );
    }

    #[test]
    fn test_role_switching() {
        let mut switcher = RoleSwitcher::new();

        let result = switcher.switch_role("帮我规划一下");
        assert_eq!(result.new_role, AgentRole::Planner);
        assert!(result.need_reload_tools);

        // 重置 switcher 以测试下一个角色
        let mut switcher = RoleSwitcher::new();
        let result = switcher.switch_role("执行这个计划");
        assert_eq!(result.new_role, AgentRole::Executor);
        assert!(result.need_reload_tools);
    }

    #[test]
    fn test_manual_override() {
        let mut switcher = RoleSwitcher::new();

        switcher.set_role(AgentRole::Planner);
        assert_eq!(switcher.current_role(), &AgentRole::Planner);

        // 手动覆盖后，自动识别应该被忽略
        assert_eq!(
            switcher.identify_role("搜索信息"),
            AgentRole::Planner
        );

        switcher.clear_override();
        assert_eq!(
            switcher.identify_role("搜索信息"),
            AgentRole::Researcher
        );
    }
}
