use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const VISUALIZATION_SCHEMA_VERSION: &str = "atlas.visualization.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationCatalog {
    pub schema_version: String,
    pub generated_at: String,
    pub types: Vec<VisualizationTypeDescriptor>,
    pub sources: Vec<VisualizationSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationTypeDescriptor {
    pub kind: String,
    pub label: String,
    pub description: String,
    pub adapter_id: String,
    pub plugin_api_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationSource {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub source_type: String,
    pub live: bool,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationDocument {
    pub schema_version: String,
    pub id: String,
    pub kind: String,
    pub title: String,
    pub generated_at: String,
    pub source: VisualizationSource,
    #[serde(default)]
    pub nodes: Vec<VisualizationNode>,
    #[serde(default)]
    pub edges: Vec<VisualizationEdge>,
    #[serde(default)]
    pub series: Vec<VisualizationSeries>,
    #[serde(default)]
    pub events: Vec<VisualizationEvent>,
    #[serde(default)]
    pub frames: Vec<VisualizationFrame>,
    #[serde(default)]
    pub diagnostics: Vec<VisualizationDiagnostic>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationNode {
    pub id: String,
    pub label: String,
    pub category: String,
    #[serde(default)]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub status: String,
    #[serde(default = "default_edge_weight")]
    pub weight: f64,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

fn default_edge_weight() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationSeries {
    pub id: String,
    pub label: String,
    pub unit: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub points: Vec<VisualizationPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationPoint {
    pub timestamp_ms: i64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationEvent {
    pub id: String,
    pub sequence: usize,
    pub label: String,
    pub category: String,
    #[serde(default)]
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationFrame {
    pub id: String,
    pub sequence: usize,
    pub label: String,
    #[serde(default)]
    pub active_nodes: Vec<String>,
    #[serde(default)]
    pub active_edges: Vec<String>,
    #[serde(default)]
    pub metrics: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationDiagnostic {
    pub level: String,
    pub message: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

impl VisualizationDocument {
    pub fn empty(
        kind: impl Into<String>,
        title: impl Into<String>,
        source: VisualizationSource,
    ) -> Self {
        let kind = kind.into();
        Self {
            schema_version: VISUALIZATION_SCHEMA_VERSION.to_string(),
            id: format!("{}:{}", kind, source.id),
            kind,
            title: title.into(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            source,
            nodes: Vec::new(),
            edges: Vec::new(),
            series: Vec::new(),
            events: Vec::new(),
            frames: Vec::new(),
            diagnostics: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }
}

impl VisualizationNode {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            category: category.into(),
            status: String::new(),
            parent_id: None,
            metrics: BTreeMap::new(),
            metadata: BTreeMap::new(),
        }
    }
}

impl VisualizationEdge {
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        label: impl Into<String>,
        category: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            label: label.into(),
            category: category.into(),
            status: String::new(),
            weight: 1.0,
            metadata: BTreeMap::new(),
        }
    }
}
