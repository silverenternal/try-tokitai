use super::util::live_source;
use crate::visualization::model::{
    VisualizationDocument, VisualizationEdge, VisualizationEvent, VisualizationFrame,
    VisualizationNode, VisualizationSource, VisualizationTypeDescriptor,
};
use crate::visualization::{type_descriptor, VisualizationAdapter, VisualizationContext};
use anyhow::Result;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

pub struct MultiAgentAdapter;

impl VisualizationAdapter for MultiAgentAdapter {
    fn descriptor(&self) -> VisualizationTypeDescriptor {
        type_descriptor(
            "multi-agent",
            "Multi-Agent",
            "Live agent topology, task flow, tools, messages, state, tokens, and lifecycle.",
            "atlas.multi-agent.runtime",
        )
    }

    fn discover(&self, context: &VisualizationContext<'_>) -> Result<Vec<VisualizationSource>> {
        let mut sources = Vec::new();
        if let Some(sessions) = context.runtime.get("sessions").and_then(Value::as_array) {
            for session in sessions {
                let Some(id) = session.get("session_id").and_then(Value::as_str) else {
                    continue;
                };
                let label = session
                    .get("title")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(id);
                sources.push(live_source(
                    &format!("runtime:agent:{id}"),
                    "multi-agent",
                    label,
                    "agent-runtime",
                ));
            }
        }
        if sources.is_empty() {
            sources.push(live_source(
                "runtime:agent:current",
                "multi-agent",
                "Current session",
                "agent-runtime",
            ));
        }
        Ok(sources)
    }

    fn parse(&self, context: &VisualizationContext<'_>) -> Result<VisualizationDocument> {
        let requested = context
            .source_id
            .and_then(|value| value.strip_prefix("runtime:agent:"));
        let sessions = context
            .runtime
            .get("sessions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let session = sessions
            .iter()
            .find(|session| requested == session.get("session_id").and_then(Value::as_str))
            .or_else(|| sessions.first());
        let source_id = session
            .and_then(|value| value.get("session_id"))
            .and_then(Value::as_str)
            .unwrap_or("current");
        let source = live_source(
            &format!("runtime:agent:{source_id}"),
            "multi-agent",
            session
                .and_then(|value| value.get("title"))
                .and_then(Value::as_str)
                .unwrap_or("Current session"),
            "agent-runtime",
        );
        let mut document =
            VisualizationDocument::empty("multi-agent", "Multi-Agent Runtime", source);
        let root_id = format!("agent:main:{source_id}");
        let mut root = VisualizationNode::new(&root_id, "Main Agent", "agent");
        root.status = session
            .and_then(|value| value.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("idle")
            .to_string();
        if let Some(tokens) = session
            .and_then(|value| value.get("context_used_tokens"))
            .and_then(Value::as_f64)
        {
            root.metrics.insert("tokens".to_string(), tokens);
        }
        if let Some(window) = session
            .and_then(|value| value.get("context_window"))
            .and_then(Value::as_f64)
        {
            root.metrics.insert("context_window".to_string(), window);
        }
        document.nodes.push(root);
        let mut agent_ids = HashMap::new();
        agent_ids.insert("main".to_string(), root_id.clone());
        let mut sequence = 0usize;
        if let Some(subagents) = session
            .and_then(|value| value.get("subagents"))
            .and_then(Value::as_array)
        {
            for (index, agent) in subagents.iter().enumerate() {
                let raw_id = agent
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("subagent");
                let id = format!("agent:{raw_id}");
                let label = agent
                    .get("name")
                    .and_then(Value::as_str)
                    .filter(|value| !value.is_empty())
                    .unwrap_or(raw_id);
                let mut node = VisualizationNode::new(&id, label, "agent");
                node.status = agent
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                node.parent_id = Some(root_id.clone());
                node.metadata.insert(
                    "purpose".to_string(),
                    agent.get("purpose").cloned().unwrap_or(Value::Null),
                );
                node.metadata.insert(
                    "input".to_string(),
                    agent.get("input").cloned().unwrap_or(Value::Null),
                );
                node.metadata.insert(
                    "output".to_string(),
                    agent.get("output").cloned().unwrap_or(Value::Null),
                );
                document.nodes.push(node);
                document.edges.push(VisualizationEdge::new(
                    format!("delegate:{index}"),
                    &root_id,
                    &id,
                    "delegates",
                    "task-flow",
                ));
                if let Some(input) = agent
                    .get("input")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                {
                    let message_id = format!("message:{raw_id}:input");
                    let mut message = VisualizationNode::new(
                        &message_id,
                        input.chars().take(90).collect::<String>(),
                        "message",
                    );
                    message.parent_id = Some(root_id.clone());
                    message.metadata.insert(
                        "direction".to_string(),
                        Value::String("main-to-agent".to_string()),
                    );
                    document.nodes.push(message);
                    document.edges.push(VisualizationEdge::new(
                        format!("message-input:{index}"),
                        &root_id,
                        &message_id,
                        "sends",
                        "message",
                    ));
                    document.edges.push(VisualizationEdge::new(
                        format!("message-delivery:{index}"),
                        &message_id,
                        &id,
                        "delivers",
                        "message",
                    ));
                }
                if let Some(output) = agent
                    .get("output")
                    .and_then(Value::as_str)
                    .filter(|value| !value.trim().is_empty())
                {
                    let message_id = format!("message:{raw_id}:output");
                    let mut message = VisualizationNode::new(
                        &message_id,
                        output.chars().take(90).collect::<String>(),
                        "message",
                    );
                    message.parent_id = Some(id.clone());
                    message.metadata.insert(
                        "direction".to_string(),
                        Value::String("agent-to-main".to_string()),
                    );
                    document.nodes.push(message);
                    document.edges.push(VisualizationEdge::new(
                        format!("message-output:{index}"),
                        &id,
                        &message_id,
                        "returns",
                        "message",
                    ));
                    document.edges.push(VisualizationEdge::new(
                        format!("message-result:{index}"),
                        &message_id,
                        &root_id,
                        "reports",
                        "message",
                    ));
                }
                if let Some(started_at) = agent.get("started_at").and_then(Value::as_str) {
                    document.events.push(VisualizationEvent {
                        id: format!("agent-start:{raw_id}"),
                        sequence,
                        label: format!("{label} started"),
                        category: "lifecycle".to_string(),
                        status: "running".to_string(),
                        timestamp: Some(started_at.to_string()),
                        source_id: Some(root_id.clone()),
                        target_id: Some(id.clone()),
                        metadata: BTreeMap::new(),
                    });
                    sequence += 1;
                }
                if let Some(completed_at) = agent.get("completed_at").and_then(Value::as_str) {
                    document.events.push(VisualizationEvent {
                        id: format!("agent-complete:{raw_id}"),
                        sequence,
                        label: format!("{label} completed"),
                        category: "lifecycle".to_string(),
                        status: agent
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("complete")
                            .to_string(),
                        timestamp: Some(completed_at.to_string()),
                        source_id: Some(id.clone()),
                        target_id: Some(root_id.clone()),
                        metadata: BTreeMap::new(),
                    });
                    sequence += 1;
                }
                agent_ids.insert(raw_id.to_string(), id);
            }
        }
        if let Some(timeline) = session
            .and_then(|value| value.get("timeline"))
            .and_then(Value::as_array)
        {
            for event in timeline {
                let agent = event.get("agent").and_then(Value::as_str).unwrap_or("main");
                let source = agent_ids
                    .get(agent)
                    .cloned()
                    .unwrap_or_else(|| root_id.clone());
                document
                    .events
                    .push(runtime_event(event, sequence, Some(source), None));
                sequence += 1;
            }
        }
        if let Some(tools) = session
            .and_then(|value| value.get("tool_events"))
            .and_then(Value::as_array)
        {
            for (index, tool) in tools.iter().enumerate() {
                let call_id = tool
                    .get("call_id")
                    .and_then(Value::as_str)
                    .unwrap_or("tool");
                let id = format!("tool:{call_id}");
                let mut node = VisualizationNode::new(
                    &id,
                    tool.get("name").and_then(Value::as_str).unwrap_or("tool"),
                    "tool",
                );
                node.status = tool
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                node.parent_id = Some(root_id.clone());
                node.metadata.insert(
                    "args".to_string(),
                    tool.get("args").cloned().unwrap_or(Value::Null),
                );
                node.metadata.insert(
                    "success".to_string(),
                    tool.get("success").cloned().unwrap_or(Value::Null),
                );
                document.nodes.push(node);
                document.edges.push(VisualizationEdge::new(
                    format!("tool-call:{index}"),
                    &root_id,
                    &id,
                    "calls",
                    "tool-call",
                ));
                document.events.push(VisualizationEvent {
                    id: format!("tool-event:{index}"),
                    sequence,
                    label: tool
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("tool")
                        .to_string(),
                    category: "tool-call".to_string(),
                    status: tool
                        .get("status")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    timestamp: tool
                        .get("updated_at")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    source_id: Some(root_id.clone()),
                    target_id: Some(id),
                    metadata: tool
                        .as_object()
                        .map(|object| {
                            object
                                .iter()
                                .map(|(key, value)| (key.clone(), value.clone()))
                                .collect()
                        })
                        .unwrap_or_default(),
                });
                sequence += 1;
            }
        }
        document.frames = document
            .events
            .iter()
            .map(|event| VisualizationFrame {
                id: format!("agent-frame:{}", event.sequence),
                sequence: event.sequence,
                label: event.label.clone(),
                active_nodes: [event.source_id.clone(), event.target_id.clone()]
                    .into_iter()
                    .flatten()
                    .collect(),
                active_edges: document
                    .edges
                    .iter()
                    .filter(|edge| {
                        event.source_id.as_deref() == Some(edge.source.as_str())
                            && event.target_id.as_deref() == Some(edge.target.as_str())
                    })
                    .map(|edge| edge.id.clone())
                    .collect(),
                metrics: BTreeMap::new(),
            })
            .collect();
        document.metadata.insert(
            "runtime".to_string(),
            session.cloned().unwrap_or(Value::Null),
        );
        Ok(document)
    }
}

fn runtime_event(
    value: &Value,
    sequence: usize,
    source_id: Option<String>,
    target_id: Option<String>,
) -> VisualizationEvent {
    VisualizationEvent {
        id: format!("agent-event:{sequence}"),
        sequence,
        label: value
            .get("title")
            .or_else(|| value.get("label"))
            .and_then(Value::as_str)
            .unwrap_or("event")
            .to_string(),
        category: value
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("lifecycle")
            .to_string(),
        status: value
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        timestamp: value
            .get("ts")
            .or_else(|| value.get("timestamp"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        source_id,
        target_id,
        metadata: value
            .as_object()
            .map(|object| {
                object
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    }
}
