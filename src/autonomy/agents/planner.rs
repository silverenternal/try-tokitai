//! 规划 Agent - 制定执行计划
//!
//! # 职责
//! - 分析任务目标和约束条件
//! - 调研相关技术和最佳实践
//! - 制定分步实施计划
//! - 预估风险和回滚方案

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

/// 规划错误类型
#[derive(Error, Debug)]
pub enum PlannerError {
    #[error("规划失败：{0}")]
    PlanningFailed(String),
    #[error("文件操作失败：{0}")]
    IoError(#[from] std::io::Error),
    #[error("JSON 处理失败：{0}")]
    JsonError(#[from] serde_json::Error),
}

/// 计划步骤
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// 步骤 ID
    pub id: String,
    /// 步骤描述
    pub description: String,
    /// 使用的工具
    pub tools: Vec<String>,
    /// 预期结果
    pub expected_output: String,
    /// 依赖的步骤 ID
    pub dependencies: Vec<String>,
    /// 预估耗时（分钟）
    pub estimated_minutes: u32,
    /// 风险等级
    pub risk_level: RiskLevel,
}

/// 风险等级
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// 实施计划
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    /// 计划 ID
    pub id: String,
    /// 任务目标
    pub goal: String,
    /// 创建时间戳
    pub created_at: i64,
    /// 计划步骤
    pub steps: Vec<PlanStep>,
    /// 风险评估
    pub risks: Vec<String>,
    /// 回滚方案
    pub rollback_plan: Option<String>,
    /// 备注
    pub notes: Option<String>,
}

impl ImplementationPlan {
    /// 创建新的实施计划
    pub fn new(goal: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string()[..8].to_string(),
            goal,
            created_at: chrono::Utc::now().timestamp(),
            steps: vec![],
            risks: vec![],
            rollback_plan: None,
            notes: None,
        }
    }

    /// 添加步骤
    pub fn add_step(&mut self, description: String, tools: Vec<String>, expected_output: String, dependencies: Vec<String>, estimated_minutes: u32, risk_level: RiskLevel) {
        self.steps.push(PlanStep {
            id: format!("step_{}", self.steps.len() + 1),
            description,
            tools,
            expected_output,
            dependencies,
            estimated_minutes,
            risk_level,
        });
    }

    /// 获取总预估时间
    pub fn total_estimated_minutes(&self) -> u32 {
        self.steps.iter().map(|s| s.estimated_minutes).sum()
    }

    /// 获取第一步（无依赖的步骤）
    pub fn first_steps(&self) -> Vec<&PlanStep> {
        self.steps
            .iter()
            .filter(|s| s.dependencies.is_empty())
            .collect()
    }

    /// 获取下一步（依赖已完成的步骤）
    pub fn next_step(&self, completed_step_ids: &[String]) -> Option<&PlanStep> {
        self.steps
            .iter()
            .find(|s| {
                !completed_step_ids.contains(&s.id) &&
                s.dependencies.iter().all(|dep| completed_step_ids.contains(dep))
            })
    }
}

/// 规划 Agent
pub struct PlannerAgent {
    /// 存储目录
    storage_dir: PathBuf,
    /// 历史计划
    plans: Vec<ImplementationPlan>,
}

impl PlannerAgent {
    /// 创建新的规划 Agent
    pub fn new(storage_dir: PathBuf) -> Result<Self, PlannerError> {
        fs::create_dir_all(&storage_dir)?;
        
        let mut agent = Self {
            storage_dir,
            plans: vec![],
        };

        agent.load_plans()?;

        Ok(agent)
    }

    /// 创建计划
    pub fn create_plan(&mut self, goal: String) -> &ImplementationPlan {
        let plan = ImplementationPlan::new(goal);
        self.plans.push(plan);
        self.plans.last().unwrap()
    }

    /// 从 LLM 响应解析并创建计划
    pub fn parse_and_create_plan(&mut self, goal: String, llm_response: &str) -> Result<&ImplementationPlan, PlannerError> {
        // 尝试解析 LLM 生成的 JSON 计划
        let plan: ImplementationPlan = serde_json::from_str(llm_response)
            .or_else(|_| -> Result<ImplementationPlan, PlannerError> {
                // 如果解析失败，创建基础计划
                Ok(ImplementationPlan::new(goal))
            })?;

        self.plans.push(plan);
        self.save_plans()?;
        Ok(self.plans.last().unwrap())
    }

    /// 添加步骤到当前计划
    pub fn add_step_to_plan(
        &mut self,
        plan_id: &str,
        description: String,
        tools: Vec<String>,
        expected_output: String,
        dependencies: Vec<String>,
        estimated_minutes: u32,
        risk_level: RiskLevel,
    ) -> Result<(), PlannerError> {
        if let Some(plan) = self.plans.iter_mut().find(|p| p.id == plan_id) {
            plan.add_step(description, tools, expected_output, dependencies, estimated_minutes, risk_level);
            self.save_plans()?;
        }
        Ok(())
    }

    /// 获取最近的计划
    pub fn last_plan(&self) -> Option<&ImplementationPlan> {
        self.plans.last()
    }

    /// 获取所有计划
    pub fn plans(&self) -> &[ImplementationPlan] {
        &self.plans
    }

    /// 保存计划
    fn save_plans(&self) -> Result<(), PlannerError> {
        let plans_path = self.storage_dir.join("plans.json");
        let content = serde_json::to_string_pretty(&self.plans)?;
        fs::write(&plans_path, content)?;
        Ok(())
    }

    /// 加载计划
    fn load_plans(&mut self) -> Result<(), PlannerError> {
        let plans_path = self.storage_dir.join("plans.json");
        if plans_path.exists() {
            let content = fs::read_to_string(&plans_path)?;
            self.plans = serde_json::from_str(&content)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_planner_agent() {
        let temp_dir = TempDir::new().unwrap();
        let mut planner = PlannerAgent::new(temp_dir.path().to_path_buf()).unwrap();

        let plan = planner.create_plan("测试目标".to_string());
        assert_eq!(plan.goal, "测试目标");
        assert!(plan.steps.is_empty());
    }

    #[test]
    fn test_plan_steps() {
        let mut plan = ImplementationPlan::new("改进错误处理".to_string());
        
        plan.add_step(
            "调研 Rust 错误处理最佳实践".to_string(),
            vec!["web_search".to_string()],
            "收集最佳实践文档".to_string(),
            vec![],
            30,
            RiskLevel::Low,
        );

        plan.add_step(
            "分析当前代码的错误处理方式".to_string(),
            vec!["read_file".to_string(), "grep".to_string()],
            "识别问题点".to_string(),
            vec![],
            20,
            RiskLevel::Low,
        );

        plan.add_step(
            "设计新的错误类型定义".to_string(),
            vec!["write_file".to_string()],
            "创建 error.rs".to_string(),
            vec!["step_1".to_string(), "step_2".to_string()],
            15,
            RiskLevel::Medium,
        );

        assert_eq!(plan.steps.len(), 3);
        assert_eq!(plan.total_estimated_minutes(), 65);
    }
}
