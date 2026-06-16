//! ReportAgent — Scientific paper/report generation

use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;

pub struct ReportAgent {
    id: String,
}

impl ReportAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl Agent for ReportAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn role(&self) -> AgentRole {
        AgentRole::Reporter
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability {
            name: "paper_generation".into(),
            description: "Generate structured scientific papers with LaTeX".into(),
            required_tools: vec!["generate_latex".into(), "format_citations".into()],
        }]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let _all_results = msg.payload.get("all_results")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        Ok(AgentResponse::ok(serde_json::json!({
            "agent": self.id,
            "paper": {
                "title": "[Generated Title]",
                "abstract": "[Generated Abstract]",
                "sections": ["Introduction", "Methods", "Results", "Discussion", "Conclusion"],
                "format": "latex"
            },
            "status": "Report generated"
        })))
    }
}
