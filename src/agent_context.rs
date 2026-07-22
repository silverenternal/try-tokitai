//! Explicit context contracts for main-agent to sub-agent delegation.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::tui::components::message_block::MessageBlock;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubagentContextMode {
    Minimal,
    Manual,
    #[default]
    Automatic,
    LlmGenerated,
}

impl SubagentContextMode {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "minimal" => Self::Minimal,
            "manual" => Self::Manual,
            "llm_generated" | "llm" => Self::LlmGenerated,
            _ => Self::Automatic,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Manual => "manual",
            Self::Automatic => "automatic",
            Self::LlmGenerated => "llm_generated",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentContextConfig {
    pub mode: SubagentContextMode,
    pub manual_context: String,
    pub recent_turns: usize,
    pub privacy_rules: String,
}

impl Default for SubagentContextConfig {
    fn default() -> Self {
        Self {
            mode: SubagentContextMode::Automatic,
            manual_context: String::new(),
            recent_turns: 3,
            privacy_rules:
                "Do not share credentials, payment data, tokens, private keys, or unrelated personal data."
                    .to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentContextObject {
    pub schema: String,
    pub mode: SubagentContextMode,
    pub task: String,
    pub shared_facts: Vec<String>,
    pub recent_dialogue: Vec<Value>,
    pub relevant_tool_results: Vec<Value>,
    pub privacy_applied: Vec<String>,
}

impl SubagentContextObject {
    pub fn to_prompt(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| {
            json!({"schema":"atlas.subagent-context.v1","mode":"minimal","task":self.task})
                .to_string()
        })
    }
}

pub fn deterministic_context(
    config: &SubagentContextConfig,
    messages: &[MessageBlock],
    task: &str,
) -> SubagentContextObject {
    match config.mode {
        SubagentContextMode::Minimal => SubagentContextObject {
            schema: "atlas.subagent-context.v1".into(),
            mode: config.mode,
            task: redact(task),
            shared_facts: Vec::new(),
            recent_dialogue: Vec::new(),
            relevant_tool_results: Vec::new(),
            privacy_applied: vec!["Only explicit invocation arguments were shared.".into()],
        },
        SubagentContextMode::Manual => SubagentContextObject {
            schema: "atlas.subagent-context.v1".into(),
            mode: config.mode,
            task: redact(task),
            shared_facts: config
                .manual_context
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(redact)
                .take(40)
                .collect(),
            recent_dialogue: Vec::new(),
            relevant_tool_results: Vec::new(),
            privacy_applied: vec![
                "Only the manually selected context and task were shared.".into(),
                config.privacy_rules.clone(),
            ],
        },
        SubagentContextMode::Automatic | SubagentContextMode::LlmGenerated => {
            automatic_context(config, messages, task)
        }
    }
}

pub fn automatic_context(
    config: &SubagentContextConfig,
    messages: &[MessageBlock],
    task: &str,
) -> SubagentContextObject {
    let dialogue_limit = config.recent_turns.clamp(1, 10) * 2;
    let recent_dialogue = messages
        .iter()
        .rev()
        .filter_map(|block| match block {
            MessageBlock::User { content, .. } => {
                Some(json!({"role":"user","content":redact(content)}))
            }
            MessageBlock::Assistant { content } => {
                Some(json!({"role":"assistant","content":truncate(&redact(content), 1200)}))
            }
            _ => None,
        })
        .take(dialogue_limit)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let relevant_tool_results = messages
        .iter()
        .rev()
        .filter_map(|block| match block {
            MessageBlock::ToolResult {
                call_id,
                result,
                success,
            } => Some(json!({
                "call_id": call_id,
                "success": success,
                "result": truncate(&redact(result), 1000),
            })),
            MessageBlock::Diff { diff } => Some(json!({
                "kind":"diff",
                "path":diff.file_path,
                "added":diff.added,
                "removed":diff.removed,
            })),
            _ => None,
        })
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    SubagentContextObject {
        schema: "atlas.subagent-context.v1".into(),
        mode: config.mode,
        task: redact(task),
        shared_facts: Vec::new(),
        recent_dialogue,
        relevant_tool_results,
        privacy_applied: vec![
            format!("Kept the most recent {} dialogue turns.", config.recent_turns.clamp(1, 10)),
            config.privacy_rules.clone(),
        ],
    }
}

pub fn parse_llm_context(
    raw: &str,
    fallback: SubagentContextObject,
) -> SubagentContextObject {
    let trimmed = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return fallback;
    };
    SubagentContextObject {
        schema: "atlas.subagent-context.v1".into(),
        mode: SubagentContextMode::LlmGenerated,
        task: value
            .get("task")
            .and_then(Value::as_str)
            .map(redact)
            .unwrap_or(fallback.task),
        shared_facts: value
            .get("shared_facts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(redact)
            .take(40)
            .collect(),
        recent_dialogue: value
            .get("recent_dialogue")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(redact_json)
            .take(20)
            .collect(),
        relevant_tool_results: value
            .get("relevant_tool_results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(redact_json)
            .take(12)
            .collect(),
        privacy_applied: value
            .get("privacy_applied")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_string)
            .chain(std::iter::once(fallback.privacy_applied.join(" ")))
            .take(20)
            .collect(),
    }
}

fn redact(input: &str) -> String {
    input
        .split_whitespace()
        .map(|word| {
            let lowered = word.to_ascii_lowercase();
            let sensitive_key = [
                "password", "passwd", "secret", "api_key", "apikey", "token", "authorization",
                "credit_card", "card_number", "cvv", "private_key",
            ]
            .iter()
            .any(|key| lowered.contains(key));
            let long_credential = word.len() >= 24
                && word.chars().all(|character| character.is_ascii_alphanumeric() || "-_=.".contains(character));
            if sensitive_key || long_credential {
                "[REDACTED]".to_string()
            } else {
                word.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_json(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(redact(&text)),
        Value::Array(items) => Value::Array(items.into_iter().map(redact_json).collect()),
        Value::Object(items) => Value::Object(
            items
                .into_iter()
                .map(|(key, value)| (key, redact_json(value)))
                .collect(),
        ),
        value => value,
    }
}

fn truncate(input: &str, limit: usize) -> String {
    let mut output = input.chars().take(limit).collect::<String>();
    if input.chars().count() > limit {
        output.push('…');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimal_mode_does_not_leak_history() {
        let messages = vec![MessageBlock::User {
            content: "password=hunter2".into(),
            branch_id: "main".into(),
        }];
        let context = deterministic_context(
            &SubagentContextConfig {
                mode: SubagentContextMode::Minimal,
                ..Default::default()
            },
            &messages,
            "query order 12345",
        );
        assert!(context.recent_dialogue.is_empty());
        assert_eq!(context.task, "query order 12345");
    }

    #[test]
    fn automatic_mode_redacts_sensitive_values() {
        let messages = vec![MessageBlock::ToolResult {
            call_id: "1".into(),
            result: "authorization abcdefghijklmnopqrstuvwxyz1234".into(),
            success: true,
        }];
        let context = automatic_context(&SubagentContextConfig::default(), &messages, "review");
        let serialized = context.to_prompt();
        assert!(!serialized.contains("abcdefghijklmnopqrstuvwxyz1234"));
        assert!(serialized.contains("REDACTED"));
    }
}
