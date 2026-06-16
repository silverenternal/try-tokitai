//! HypothesisAgent — Scientific hypothesis generation and refinement

use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;

pub struct HypothesisAgent {
    id: String,
}

impl HypothesisAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl Agent for HypothesisAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn role(&self) -> AgentRole {
        AgentRole::Hypothesizer
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability {
            name: "hypothesis_generation".into(),
            description: "Generate testable scientific hypotheses from literature review".into(),
            required_tools: vec!["generate_hypothesis".into(), "validate_hypothesis".into()],
        }]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let knowledge = msg.payload.get("knowledge_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Generate hypothesis based on knowledge
        Ok(AgentResponse::ok(serde_json::json!({
            "agent": self.id,
            "hypothesis": format!("Based on {}: [generated hypothesis]", knowledge),
            "status": "Hypothesis generated",
            "testable": true
        }))
        .with_next_role(AgentRole::Experimenter))
    }
}
