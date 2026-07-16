//! HypothesisAgent — CS problem framing and research hypothesis refinement

use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;
use serde_json::Value;

pub struct HypothesisAgent {
    id: String,
}

impl HypothesisAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn extract_topic_hint(topic: &str, knowledge_summary: &str) -> String {
    let explicit = topic.trim();
    if !explicit.is_empty() {
        return explicit.to_string();
    }
    let knowledge = knowledge_summary.trim();
    if let Some(prefix_index) = knowledge.find("for '") {
        let rest = &knowledge[prefix_index + 5..];
        if let Some(suffix_index) = rest.find('\'') {
            let extracted = rest[..suffix_index].trim();
            if !extracted.is_empty() {
                return extracted.to_string();
            }
        }
    }
    if let Some(prefix_index) = knowledge.find("for \"") {
        let rest = &knowledge[prefix_index + 5..];
        if let Some(suffix_index) = rest.find('"') {
            let extracted = rest[..suffix_index].trim();
            if !extracted.is_empty() {
                return extracted.to_string();
            }
        }
    }
    String::new()
}

fn primary_dataset_hint(hints: &[String]) -> String {
    hints
        .iter()
        .map(|item| item.trim())
        .find(|item| !item.is_empty())
        .unwrap_or("")
        .to_string()
}

fn infer_problem_formulation(
    topic_hint: &str,
    knowledge_summary: &str,
    dataset_hints: &[String],
) -> String {
    let topic = extract_topic_hint(topic_hint, knowledge_summary);
    let lowered = topic.to_ascii_lowercase();
    let dataset = primary_dataset_hint(dataset_hints);
    if topic.is_empty() {
        return if dataset.is_empty() {
            "Evaluate the current computer science task with a reproducible benchmark and explicit evidence grounding."
                .to_string()
        } else {
            format!(
                "Evaluate the current computer science task on {} with a reproducible benchmark and explicit evidence grounding.",
                dataset
            )
        };
    }

    if contains_any(
        &lowered,
        &[
            "iris",
            "classification",
            "classifier",
            "sklearn",
            "logistic regression",
            "decision tree",
            "random forest",
            "svm",
            "cross validation",
        ],
    ) {
        let dataset_label = if dataset.is_empty() {
            if lowered.contains("iris") {
                "the iris dataset".to_string()
            } else {
                "the target dataset".to_string()
            }
        } else {
            dataset
        };
        return format!(
            "Compare lightweight classification baselines on {} with reproducible accuracy, F1, and error analysis.",
            dataset_label
        );
    }

    if contains_any(
        &lowered,
        &[
            "latency",
            "throughput",
            "qps",
            "memory",
            "scalability",
            "load test",
            "benchmark",
            "runtime",
            "service",
        ],
    ) {
        return format!(
            "Evaluate {} with reproducible latency, throughput, and resource measurements.",
            topic
        );
    }

    if contains_any(
        &lowered,
        &[
            "deep learning",
            "transformer",
            "neural",
            "checkpoint",
            "epoch",
            "pytorch",
        ],
    ) {
        return format!(
            "Study {} with a reproducible training, validation, and checkpointing workflow.",
            topic
        );
    }

    topic
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
            name: "problem_formulation".into(),
            description: "Turn literature review findings into a testable CS research question or benchmark hypothesis".into(),
            required_tools: vec!["search_paper".into(), "fetch_paper".into(), "fetch_papers".into(), "summarize_text".into()],
        }]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let knowledge = msg
            .payload
            .get("knowledge_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let topic = msg
            .payload
            .get("topic")
            .and_then(Value::as_str)
            .unwrap_or("");
        let paper_dataset_hints = msg
            .payload
            .get("paper_dataset_hints")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([]));
        let dataset_hint_list = paper_dataset_hints
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let problem_formulation = infer_problem_formulation(topic, knowledge, &dataset_hint_list);

        // Generate a CS-oriented research question based on the current knowledge summary.
        Ok(AgentResponse::ok(serde_json::json!({
            "agent": self.id,
            "problem_formulation": problem_formulation,
            "paper_dataset_hints": paper_dataset_hints,
            "status": "Problem formulation generated",
            "testable": true
        }))
        .with_next_role(AgentRole::Experimenter))
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_topic_hint, infer_problem_formulation};

    #[test]
    fn hypothesis_prefers_explicit_topic_over_placeholder_text() {
        let formulation = infer_problem_formulation(
            "tiny iris classifier comparison",
            "Retrieved 1 paper candidate(s) for 'tiny iris classifier comparison'.",
            &["iris".to_string()],
        );
        assert!(formulation.contains("classification baselines"));
        assert!(formulation.contains("iris"));
        assert!(!formulation.contains("[generated"));
    }

    #[test]
    fn hypothesis_extracts_topic_from_knowledge_summary_when_missing() {
        let topic = extract_topic_hint(
            "",
            "Retrieved 2 paper candidate(s) for 'latency benchmark for model serving'; primary evidence anchor is Foo.",
        );
        assert_eq!(topic, "latency benchmark for model serving");
    }
}
