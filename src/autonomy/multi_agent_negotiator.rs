//! 多智能体协商器
//!
//! 使用多个 LLM 实例扮演不同角色，通过结构化对话达成进化决策共识
//!
//! ## 核心创新
//! - **Role-Playing**: 4 个 LLM 智能体扮演不同角色
//! - **结构化协商协议**: 4 轮对话达成共识
//! - **投票共识机制**: >60% 通过率才执行
//!
//! ## 智能体角色
//! - **Creator**: 工具创建者，倾向于创建新工具
//! - **Optimizer**: 工具优化者，倾向于改进现有工具
//! - **Eliminator**: 工具淘汰者，倾向于精简工具库
//! - **Planner**: 系统规划者，协调各方意见做出最终决策
//!
//! ## 协商流程
//! ```text
//! Round 1: 各智能体独立分析状态，提出建议
//! Round 2: 智能体互相评论对方建议
//! Round 3: Planner 汇总意见，做出决策
//! Round 4: 各智能体投票确认
//! ```

#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// 智能体角色定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentRole {
    /// 工具创建者
    Creator,
    /// 工具优化者
    Optimizer,
    /// 工具淘汰者
    Eliminator,
    /// 系统规划者
    Planner,
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Creator => write!(f, "Creator"),
            AgentRole::Optimizer => write!(f, "Optimizer"),
            AgentRole::Eliminator => write!(f, "Eliminator"),
            AgentRole::Planner => write!(f, "Planner"),
        }
    }
}

/// 智能体提案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProposal {
    /// 提出智能体
    pub agent: AgentRole,
    /// 提案描述
    pub proposal: String,
    /// 理由
    pub rationale: String,
    /// 建议的行动
    pub suggested_action: EvolutionAction,
    /// 优先级评分 (1-10)
    pub priority: u8,
    /// 置信度 (0.0-1.0)
    pub confidence: f32,
}

/// 智能体评论
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCritique {
    /// 评论智能体
    pub agent: AgentRole,
    /// 针对的提案
    pub target_proposal: AgentRole,
    /// 评论内容
    pub critique: String,
    /// 是否同意
    pub agreement: f32,
    /// 改进建议
    pub suggestions: Vec<String>,
}

/// 进化行动类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EvolutionAction {
    /// 创建新工具
    CreateTool {
        tool_name: String,
        functionality: String,
    },
    /// 合并工具
    MergeTools {
        tools: Vec<String>,
        new_tool_name: String,
    },
    /// 废弃工具
    DeprecateTool {
        tool_name: String,
        replacement: Option<String>,
    },
    /// 改进工具
    ImproveTool {
        tool_name: String,
        improvements: Vec<String>,
    },
    /// 保持现状
    MaintainStatusQuo,
}

/// 智能体投票
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentVote {
    /// 投票智能体
    pub agent: AgentRole,
    /// 是否同意
    pub approve: bool,
    /// 投票理由
    pub reason: String,
}

/// 协商决策结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationDecision {
    /// 最终决策
    pub decision: EvolutionAction,
    /// 投票结果
    pub votes: Vec<AgentVote>,
    /// 通过率
    pub approval_rate: f32,
    /// 决策理由
    pub rationale: String,
    /// 协商历史
    pub negotiation_history: NegotiationHistory,
}

/// 协商历史
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationHistory {
    /// Round 1 提案
    pub round1_proposals: Vec<AgentProposal>,
    /// Round 2 评论
    pub round2_critiques: Vec<AgentCritique>,
    /// Round 3 Planner 决策
    pub round3_decision: String,
    /// Round 4 投票
    pub round4_votes: Vec<AgentVote>,
}

/// 进化状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvolutionState {
    /// 工具库规模
    pub tool_count: u32,
    /// 任务完成率
    pub success_rate: f32,
    /// 低使用率工具列表
    pub underused_tools: Vec<String>,
    /// 高频失败工具列表
    pub high_failure_tools: Vec<String>,
    /// 检测到的工具缺口
    pub detected_gaps: Vec<String>,
    /// 优化建议
    pub optimization_suggestions: Vec<String>,
    /// 系统健康度评分
    pub health_score: f32,
}

/// 智能体角色 Prompt 定义
pub const AGENT_ROLE_PROMPTS: &[(&str, &str)] = &[
    (
        "Creator",
        r#"
你是工具创建者（Creator）。你的目标是发现新工具机会，扩展工具库能力。

你的特点：
- 积极发现工具缺口
- 倾向于创建新工具来解决问题
- 但需要考虑工具库的整体健康度
- 避免创建冗余工具

当你分析状态时，请问自己：
1. 有哪些明显的工具缺口？
2. 创建什么新工具可以显著提升效率？
3. 新工具与现有工具的关系是什么？
"#,
    ),
    (
        "Optimizer",
        r#"
你是工具优化者（Optimizer）。你的目标是改进现有工具。

你的特点：
- 认为应该优先改进而非新建
- 关注工具的使用率、失败率、用户满意度
- 相信小改进可以带来大效果

当你分析状态时，请问自己：
1. 哪些工具可以通过改进获得显著提升？
2. 工具的失败原因是什么？如何修复？
3. 工具的功能是否可以扩展？
"#,
    ),
    (
        "Eliminator",
        r#"
你是工具淘汰者（Eliminator）。你的目标是移除冗余工具，保持工具库精简。

你的特点：
- 倾向于合并功能重叠的工具
- 主张废弃低使用率工具
- 相信"少即是多"

当你分析状态时，请问自己：
1. 哪些工具是冗余的？
2. 哪些工具使用率极低，应该废弃？
3. 如何简化接口，降低用户认知负担？
"#,
    ),
    (
        "Planner",
        r#"
你是系统规划者（Planner）。你的目标是整体工具库健康。

你的特点：
- 协调 Creator、Optimizer、Eliminator 的意见
- 做出平衡的最终决策
- 考虑短期效果 vs 长期影响

当你决策时，请问自己：
1. 各智能体的论据质量如何？
2. 什么决策最有利于系统长期健康？
3. 如何在创新与稳定之间取得平衡？
"#,
    ),
];

/// 协商协议 Prompt 模板
pub const NEGOTIATION_PROMPT: &str = r#"
## 进化状态

工具库规模：{tool_count}
任务完成率：{success_rate:.1}%
低使用率工具：{underused_tools}
高频失败工具：{high_failure_tools}
检测到的缺口：{detected_gaps}
优化建议：{optimization_suggestions}
系统健康度：{health_score:.2}

## 智能体角色

{role_prompts}

## 协商流程

请按照以下流程进行协商：

### Round 1: 独立分析
每个智能体独立分析状态，提出建议。

### Round 2: 互相评论
智能体互相评论对方建议，提出改进意见。

### Round 3: Planner 决策
Planner 汇总各方意见，做出最终决策。

### Round 4: 投票确认
各智能体投票确认决策。

## 输出格式

请输出严格的 JSON 格式：

{{
    "round1_proposals": [
        {{
            "agent": "Creator|Optimizer|Eliminator|Planner",
            "proposal": "提案描述",
            "rationale": "理由",
            "suggested_action": {{...}},
            "priority": 1-10,
            "confidence": 0.0-1.0
        }}
    ],
    "round2_critiques": [
        {{
            "agent": "Creator|Optimizer|Eliminator",
            "target_proposal": "Creator|Optimizer|Eliminator",
            "critique": "评论内容",
            "agreement": 0.0-1.0,
            "suggestions": ["改进建议 1", "改进建议 2"]
        }}
    ],
    "round3_decision": "Planner 的决策理由",
    "round4_votes": [
        {{
            "agent": "Creator|Optimizer|Eliminator|Planner",
            "approve": true/false,
            "reason": "投票理由"
        }}
    ],
    "final_decision": {{...}},
    "approval_rate": 0.0-1.0,
    "rationale": "最终决策理由"
}}
"#;

/// 简化的 LLM Client trait
#[async_trait::async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(&self, prompt: &str) -> Result<String>;
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String>;
}

/// 多智能体协商器
pub struct MultiAgentNegotiator {
    /// LLM 客户端（4 个智能体可以共享同一个）
    llm_client: Arc<dyn LLMClient>,
    /// 配置
    config: NegotiatorConfig,
}

/// 协商器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiatorConfig {
    /// 通过阈值（>60% 默认）
    pub approval_threshold: f32,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否启用详细日志
    pub verbose_logging: bool,
}

impl Default for NegotiatorConfig {
    fn default() -> Self {
        Self {
            approval_threshold: 0.6,
            max_retries: 2,
            verbose_logging: true,
        }
    }
}

impl MultiAgentNegotiator {
    /// 创建新的协商器
    pub fn new(llm_client: Arc<dyn LLMClient>) -> Self {
        Self {
            llm_client,
            config: NegotiatorConfig::default(),
        }
    }

    /// 从配置创建
    pub fn with_config(llm_client: Arc<dyn LLMClient>, config: NegotiatorConfig) -> Self {
        Self { llm_client, config }
    }

    /// 执行协商
    pub async fn negotiate(&self, state: &EvolutionState) -> Result<NegotiationDecision> {
        info!("开始多智能体协商...");

        // 构建协商 Prompt
        let prompt = self.build_negotiation_prompt(state);

        // 获取 JSON Schema
        let schema = self.get_negotiation_schema();

        // LLM 推理
        let response_text = self
            .llm_client
            .chat_with_schema(&prompt, &schema)
            .await
            .context("LLM 推理失败")?;

        // 解析响应
        let response: NegotiationResponse =
            serde_json::from_str(&response_text).context("解析协商响应失败")?;

        // 验证决策
        self.validate_decision(&response)?;

        // 记录协商历史
        if self.config.verbose_logging {
            self.log_negotiation_history(&response);
        }

        info!(
            "协商完成：通过率{:.1}%，决策：{}",
            response.approval_rate * 100.0,
            self.format_action(&response.final_decision)
        );

        Ok(NegotiationDecision {
            decision: response.final_decision,
            votes: response.round4_votes.clone(),
            approval_rate: response.approval_rate,
            rationale: response.rationale,
            negotiation_history: NegotiationHistory {
                round1_proposals: response.round1_proposals,
                round2_critiques: response.round2_critiques,
                round3_decision: response.round3_decision,
                round4_votes: response.round4_votes,
            },
        })
    }

    /// 构建协商 Prompt
    fn build_negotiation_prompt(&self, state: &EvolutionState) -> String {
        // 格式化角色 Prompts
        let role_prompts = AGENT_ROLE_PROMPTS
            .iter()
            .map(|(role, prompt)| format!("### {}\n{}", role, prompt))
            .collect::<Vec<_>>()
            .join("\n\n");

        // 格式化状态
        let underused = if state.underused_tools.is_empty() {
            "无".to_string()
        } else {
            state.underused_tools.join(", ")
        };

        let high_failure = if state.high_failure_tools.is_empty() {
            "无".to_string()
        } else {
            state.high_failure_tools.join(", ")
        };

        let gaps = if state.detected_gaps.is_empty() {
            "无".to_string()
        } else {
            state.detected_gaps.join(", ")
        };

        let suggestions = if state.optimization_suggestions.is_empty() {
            "无".to_string()
        } else {
            state.optimization_suggestions.join(", ")
        };

        NEGOTIATION_PROMPT
            .replace("{tool_count}", &state.tool_count.to_string())
            .replace(
                "{success_rate}",
                &format!("{:.1}", state.success_rate * 100.0),
            )
            .replace("{underused_tools}", &underused)
            .replace("{high_failure_tools}", &high_failure)
            .replace("{detected_gaps}", &gaps)
            .replace("{optimization_suggestions}", &suggestions)
            .replace("{health_score}", &format!("{:.2}", state.health_score))
            .replace("{role_prompts}", &role_prompts)
    }

    /// 获取协商响应 Schema
    fn get_negotiation_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["round1_proposals", "round2_critiques", "round3_decision", "round4_votes", "final_decision", "approval_rate", "rationale"],
            "properties": {
                "round1_proposals": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["agent", "proposal", "rationale", "suggested_action", "priority", "confidence"],
                        "properties": {
                            "agent": {"type": "string", "enum": ["Creator", "Optimizer", "Eliminator", "Planner"]},
                            "proposal": {"type": "string"},
                            "rationale": {"type": "string"},
                            "suggested_action": {"type": "object"},
                            "priority": {"type": "integer"},
                            "confidence": {"type": "number"}
                        }
                    }
                },
                "round2_critiques": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["agent", "target_proposal", "critique", "agreement", "suggestions"],
                        "properties": {
                            "agent": {"type": "string"},
                            "target_proposal": {"type": "string"},
                            "critique": {"type": "string"},
                            "agreement": {"type": "number"},
                            "suggestions": {"type": "array", "items": {"type": "string"}}
                        }
                    }
                },
                "round3_decision": {"type": "string"},
                "round4_votes": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["agent", "approve", "reason"],
                        "properties": {
                            "agent": {"type": "string"},
                            "approve": {"type": "boolean"},
                            "reason": {"type": "string"}
                        }
                    }
                },
                "final_decision": {
                    "type": "object",
                    "required": ["action_type"],
                    "properties": {
                        "action_type": {"type": "string", "enum": ["CreateTool", "MergeTools", "DeprecateTool", "ImproveTool", "MaintainStatusQuo"]},
                        "tool_name": {"type": "string"},
                        "functionality": {"type": "string"},
                        "tools": {"type": "array", "items": {"type": "string"}},
                        "new_tool_name": {"type": "string"},
                        "replacement": {"type": "string"},
                        "improvements": {"type": "array", "items": {"type": "string"}}
                    }
                },
                "approval_rate": {"type": "number"},
                "rationale": {"type": "string"}
            }
        })
    }

    /// 验证决策
    fn validate_decision(&self, response: &NegotiationResponse) -> Result<()> {
        if response.approval_rate < self.config.approval_threshold {
            warn!(
                "通过率{:.1}% 低于阈值{:.1}%",
                response.approval_rate * 100.0,
                self.config.approval_threshold * 100.0
            );
            // 不返回错误，只是警告
        }

        Ok(())
    }

    /// 记录协商历史
    fn log_negotiation_history(&self, response: &NegotiationResponse) {
        debug!("=== Round 1: 提案 ===");
        for proposal in &response.round1_proposals {
            debug!(
                "[{}] {}: {}",
                proposal.agent, proposal.proposal, proposal.rationale
            );
        }

        debug!("\n=== Round 2: 评论 ===");
        for critique in &response.round2_critiques {
            debug!(
                "[{} 评论 {}]: 同意度{:.0}%, {}",
                critique.agent,
                critique.target_proposal,
                critique.agreement * 100.0,
                critique.critique
            );
        }

        debug!("\n=== Round 3: Planner 决策 ===");
        debug!("{}", response.round3_decision);

        debug!("\n=== Round 4: 投票 ===");
        for vote in &response.round4_votes {
            debug!(
                "[{}] {} - {}",
                vote.agent,
                if vote.approve {
                    "✅ 同意"
                } else {
                    "❌ 反对"
                },
                vote.reason
            );
        }
    }

    /// 格式化行动
    fn format_action(&self, action: &EvolutionAction) -> String {
        match action {
            EvolutionAction::CreateTool { tool_name, .. } => format!("创建工具：{}", tool_name),
            EvolutionAction::MergeTools { new_tool_name, .. } => {
                format!("合并工具：{}", new_tool_name)
            }
            EvolutionAction::DeprecateTool { tool_name, .. } => format!("废弃工具：{}", tool_name),
            EvolutionAction::ImproveTool { tool_name, .. } => format!("改进工具：{}", tool_name),
            EvolutionAction::MaintainStatusQuo => "保持现状".to_string(),
        }
    }

    /// 重新协商（如果未达成共识）
    pub async fn renegotiate(&self, state: &EvolutionState) -> Result<NegotiationDecision> {
        info!("未达成共识，重新协商...");

        for i in 0..self.config.max_retries {
            let decision = self.negotiate(state).await?;

            if decision.approval_rate >= self.config.approval_threshold {
                info!("第{}次重试达成共识", i + 1);
                return Ok(decision);
            }

            warn!(
                "第{}次重试仍未达成共识，通过率{:.1}%",
                i + 1,
                decision.approval_rate * 100.0
            );
        }

        anyhow::bail!("经过{}次重试仍未达成共识", self.config.max_retries);
    }
}

/// 协商响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NegotiationResponse {
    pub round1_proposals: Vec<AgentProposal>,
    pub round2_critiques: Vec<AgentCritique>,
    pub round3_decision: String,
    pub round4_votes: Vec<AgentVote>,
    pub final_decision: EvolutionAction,
    pub approval_rate: f32,
    pub rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    struct MockLLMClient {
        response: String,
    }

    impl MockLLMClient {
        fn new(response: &str) -> Self {
            Self {
                response: response.to_string(),
            }
        }
    }

    #[async_trait::async_trait]
    impl LLMClient for MockLLMClient {
        async fn chat(&self, _prompt: &str) -> Result<String> {
            Ok(self.response.clone())
        }

        async fn chat_with_schema(
            &self,
            _prompt: &str,
            _schema: &serde_json::Value,
        ) -> Result<String> {
            Ok(self.response.clone())
        }
    }

    #[test]
    fn test_negotiator_creation() {
        let mock_llm = Arc::new(MockLLMClient::new("{}"));
        let negotiator = MultiAgentNegotiator::new(mock_llm);
        assert_eq!(negotiator.config.approval_threshold, 0.6);
    }

    #[test]
    fn test_evolution_state() {
        let state = EvolutionState {
            tool_count: 50,
            success_rate: 0.75,
            underused_tools: vec!["old_tool".to_string()],
            high_failure_tools: vec!["buggy_tool".to_string()],
            detected_gaps: vec!["missing_batch_download".to_string()],
            optimization_suggestions: vec!["merge_similar_tools".to_string()],
            health_score: 0.7,
        };

        assert_eq!(state.tool_count, 50);
        assert_eq!(state.success_rate, 0.75);
    }

    #[test]
    fn test_agent_roles() {
        assert_eq!(AGENT_ROLE_PROMPTS.len(), 4);

        let creator_prompt = AGENT_ROLE_PROMPTS
            .iter()
            .find(|(role, _)| *role == "Creator");
        assert!(creator_prompt.is_some());

        let eliminator_prompt = AGENT_ROLE_PROMPTS
            .iter()
            .find(|(role, _)| *role == "Eliminator");
        assert!(eliminator_prompt.is_some());
    }

    #[test]
    fn test_action_formatting() {
        let mock_llm = Arc::new(MockLLMClient::new("{}"));
        let negotiator = MultiAgentNegotiator::new(mock_llm);

        let create_action = EvolutionAction::CreateTool {
            tool_name: "test_tool".to_string(),
            functionality: "test".to_string(),
        };
        assert_eq!(
            negotiator.format_action(&create_action),
            "创建工具：test_tool"
        );

        let maintain_action = EvolutionAction::MaintainStatusQuo;
        assert_eq!(negotiator.format_action(&maintain_action), "保持现状");
    }
}
