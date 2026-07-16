//! ResearchAgent — CS literature search, paper retrieval, knowledge extraction

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
                description: "Search computer science papers across arXiv, Semantic Scholar, and technical web sources".into(),
                required_tools: vec!["search_paper".into(), "fetch_paper".into(), "fetch_papers".into()],
            },
            Capability {
                name: "paper_analysis".into(),
                description: "Extract key methods, datasets, baselines, and results from CS papers".into(),
                required_tools: vec!["parse_pdf".into(), "summarize_text".into()],
            },
            Capability {
                name: "knowledge_extraction".into(),
                description: "Build structured knowledge from a corpus of CS papers and benchmarks".into(),
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
        let action = msg
            .payload
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("search");
        let paper_dataset_hints = msg
            .payload
            .get("paper_dataset_hints")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));

        match action {
            "search" => {
                let query = msg
                    .payload
                    .get("query")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(AgentResponse::ok(serde_json::json!({
                    "agent": self.id,
                    "action": "search",
                    "query": query,
                    "paper_dataset_hints": paper_dataset_hints,
                    "status": "CS literature search initiated"
                }))
                .with_next_role(AgentRole::Hypothesizer))
            }
            "analyze" => {
                let paper_id = msg
                    .payload
                    .get("paper_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                Ok(AgentResponse::ok(serde_json::json!({
                    "agent": self.id,
                    "action": "analyze",
                    "paper_id": paper_id,
                    "paper_dataset_hints": paper_dataset_hints,
                    "status": "Paper analysis complete"
                })))
            }
            _ => Ok(AgentResponse::ok(serde_json::json!({
                "agent": self.id,
                "paper_dataset_hints": paper_dataset_hints,
                "status": "Research phase ready"
            }))),
        }
    }
}
