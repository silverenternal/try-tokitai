//! Knowledge Graph Module
//!
//! Persistent knowledge graph for papers, concepts, methods, and results.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectType,
};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeType {
    #[serde(rename = "paper")]
    Paper,
    #[serde(rename = "concept")]
    Concept,
    #[serde(rename = "method")]
    Method,
    #[serde(rename = "dataset")]
    Dataset,
    #[serde(rename = "result")]
    Result,
}

impl NodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Paper => "paper",
            Self::Concept => "concept",
            Self::Method => "method",
            Self::Dataset => "dataset",
            Self::Result => "result",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EdgeType {
    #[serde(rename = "cites")]
    Cites,
    #[serde(rename = "uses")]
    Uses,
    #[serde(rename = "extends")]
    Extends,
    #[serde(rename = "contradicts")]
    Contradicts,
}

impl EdgeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cites => "cites",
            Self::Uses => "uses",
            Self::Extends => "extends",
            Self::Contradicts => "contradicts",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphNode {
    pub schema_version: String,
    pub id: String,
    pub node_type: String,
    pub label: String,
    #[serde(default)]
    pub properties: Value,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphEdge {
    pub schema_version: String,
    pub id: String,
    pub from_id: String,
    pub to_id: String,
    pub edge_type: String,
    pub created_at: String,
}

const KG_NODE_SCHEMA: &str = "atlas.research-os.kg-node.v1";
const KG_EDGE_SCHEMA: &str = "atlas.research-os.kg-edge.v1";

pub fn create_kg_node(
    workspace_root: &Path,
    node_type: NodeType,
    label: &str,
    properties: Value,
) -> Result<KnowledgeGraphNode> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", node_type.as_str(), label, now));

    let node = KnowledgeGraphNode {
        schema_version: KG_NODE_SCHEMA.to_string(),
        id: id.clone(),
        node_type: node_type.as_str().to_string(),
        label: label.to_string(),
        properties,
        created_at: now,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::KnowledgeGraphNode,
        &id,
        &node,
    )?;

    Ok(node)
}

pub fn create_kg_edge(
    workspace_root: &Path,
    from_id: &str,
    to_id: &str,
    edge_type: EdgeType,
) -> Result<KnowledgeGraphEdge> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", from_id, to_id, edge_type.as_str()));

    let edge = KnowledgeGraphEdge {
        schema_version: KG_EDGE_SCHEMA.to_string(),
        id: id.clone(),
        from_id: from_id.to_string(),
        to_id: to_id.to_string(),
        edge_type: edge_type.as_str().to_string(),
        created_at: now,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::KnowledgeGraphEdge,
        &id,
        &edge,
    )?;

    Ok(edge)
}

pub fn get_kg_node(workspace_root: &Path, id: &str) -> Result<KnowledgeGraphNode> {
    read_research_object(workspace_root, ResearchObjectType::KnowledgeGraphNode, id)
}

pub fn get_kg_edge(workspace_root: &Path, id: &str) -> Result<KnowledgeGraphEdge> {
    read_research_object(workspace_root, ResearchObjectType::KnowledgeGraphEdge, id)
}

pub fn list_kg_nodes(workspace_root: &Path) -> Result<Vec<KnowledgeGraphNode>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::KnowledgeGraphNode)?;
    let mut nodes = Vec::new();

    for id in ids {
        if let Ok(node) = get_kg_node(workspace_root, &id) {
            nodes.push(node);
        }
    }

    Ok(nodes)
}

pub fn list_kg_edges(workspace_root: &Path) -> Result<Vec<KnowledgeGraphEdge>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::KnowledgeGraphEdge)?;
    let mut edges = Vec::new();

    for id in ids {
        if let Ok(edge) = get_kg_edge(workspace_root, &id) {
            edges.push(edge);
        }
    }

    Ok(edges)
}

pub fn get_knowledge_graph(
    workspace_root: &Path,
) -> Result<(Vec<KnowledgeGraphNode>, Vec<KnowledgeGraphEdge>)> {
    let nodes = list_kg_nodes(workspace_root)?;
    let edges = list_kg_edges(workspace_root)?;
    Ok((nodes, edges))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_kg_node() {
        let dir = tempdir().unwrap();
        let props = json!({"url": "https://arxiv.org/abs/1234.5678", "year": 2024});
        let node = create_kg_node(
            dir.path(),
            NodeType::Paper,
            "Attention Is All You Need",
            props.clone(),
        )
        .unwrap();

        assert_eq!(node.node_type, "paper");
        assert_eq!(node.label, "Attention Is All You Need");
        assert_eq!(node.properties, props);

        let retrieved = get_kg_node(dir.path(), &node.id).unwrap();
        assert_eq!(retrieved.label, node.label);
    }

    #[test]
    fn creates_and_retrieves_kg_edge() {
        let dir = tempdir().unwrap();
        let node1 = create_kg_node(dir.path(), NodeType::Paper, "Paper A", json!({})).unwrap();
        let node2 = create_kg_node(dir.path(), NodeType::Paper, "Paper B", json!({})).unwrap();

        let edge = create_kg_edge(dir.path(), &node1.id, &node2.id, EdgeType::Cites).unwrap();

        assert_eq!(edge.from_id, node1.id);
        assert_eq!(edge.to_id, node2.id);
        assert_eq!(edge.edge_type, "cites");

        let retrieved = get_kg_edge(dir.path(), &edge.id).unwrap();
        assert_eq!(retrieved.from_id, edge.from_id);
    }

    #[test]
    fn retrieves_full_knowledge_graph() {
        let dir = tempdir().unwrap();
        create_kg_node(dir.path(), NodeType::Concept, "Transformer", json!({})).unwrap();
        create_kg_node(dir.path(), NodeType::Method, "Self-Attention", json!({})).unwrap();

        let (nodes, _) = get_knowledge_graph(dir.path()).unwrap();
        assert_eq!(nodes.len(), 2);
    }
}
