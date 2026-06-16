//! ExperimentAgent — Scientific experiment design and execution

use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;

pub struct ExperimentAgent {
    id: String,
}

impl ExperimentAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl Agent for ExperimentAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn role(&self) -> AgentRole {
        AgentRole::Experimenter
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability {
            name: "experiment_design".into(),
            description: "Design reproducible experiments to test hypotheses".into(),
            required_tools: vec!["design_experiment".into(), "compute_power_analysis".into()],
        }]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let hypothesis = msg.payload.get("hypothesis")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(AgentResponse::ok(serde_json::json!({
            "agent": self.id,
            "experiment": {
                "design": "Randomized controlled trial",
                "variables": { "independent": [], "dependent": [], "controlled": [] },
                "sample_size": 100,
                "methodology": "Statistical hypothesis testing"
            },
            "status": "Experiment designed"
        }))
        .with_next_role(AgentRole::Verifier))
    }
}
