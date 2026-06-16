//! ResearchAgent — Literature search, paper retrieval, knowledge extraction

use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;

pub struct ResearchAgent {
    id: String,
}

impl ResearchAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

#[async_trait]
impl Agent for ResearchAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn role(&self) -> AgentRole {
        AgentRole::Researcher
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability {
                name: "literature_search".into(),
                description: "Search academic papers across arXiv, Semantic Scholar, PubMed".into(),
                required_tools: vec!["search_paper".into(), "fetch_paper".into()],
            },
            Capability {
                name: "paper_analysis".into(),
                description: "Extract key findings, methods, and results from papers".into(),
                required_tools: vec!["parse_pdf".into(), "summarize_text".into()],
            },
            Capability {
                name: "knowledge_extraction".into(),
                description: "Build structured knowledge from paper corpus".into(),
                required_tools: vec!["extract_entities".into(), "build_knowledge_graph".into()],
            },
        ]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        // Determine action from message type and payload
        let action = msg.payload.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("search");

        match action {
            "search" => {
                let query = msg.payload.get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(AgentResponse::ok(serde_json::json!({
                    "agent": self.id,
                    "action": "search",
                    "query": query,
                    "status": "Literature search initiated"
                }))
                .with_next_role(AgentRole::Hypothesizer))
            }
            "analyze" => {
                let paper_id = msg.payload.get("paper_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(AgentResponse::ok(serde_json::json!({
                    "agent": self.id,
                    "action": "analyze",
                    "paper_id": paper_id,
                    "status": "Paper analysis complete"
                })))
            }
            _ => Ok(AgentResponse::ok(serde_json::json!({
                "agent": self.id,
                "status": "Research phase ready"
            }))),
        }
    }
}
