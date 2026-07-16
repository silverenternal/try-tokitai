use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const RESEARCH_DOMAIN_SCHEMA_VERSION: &str = "atlas.research-domain.v1";
pub const RESEARCH_DOMAIN_PLUGIN_API_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainMetadata {
    pub id: String,
    pub label: String,
    pub description: String,
    pub version: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainProviderDescriptor {
    pub id: String,
    pub api_version: String,
    pub provider_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainVisualizationDescriptor {
    pub id: String,
    pub label: String,
    pub renderer: String,
    #[serde(default)]
    pub compatible_file_types: Vec<String>,
    #[serde(default)]
    pub adapter: String,
    #[serde(default)]
    pub workbench_region: String,
    #[serde(default)]
    pub requires_sdk: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DomainWorkbenchDescriptor {
    #[serde(default)]
    pub layout: String,
    #[serde(default)]
    pub explorer_label: String,
    #[serde(default)]
    pub primary_label: String,
    #[serde(default)]
    pub inspector_label: String,
    #[serde(default)]
    pub bottom_panel_label: String,
    #[serde(default)]
    pub tools: Vec<DomainWorkbenchToolDescriptor>,
    #[serde(default)]
    pub workflow: Vec<DomainWorkbenchStageDescriptor>,
    #[serde(default)]
    pub intents: Vec<DomainIntentDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DomainIntentDescriptor {
    pub id: String,
    pub label: String,
    pub description: String,
    pub agent: String,
    pub input_contract: String,
    #[serde(default)]
    pub expected_outputs: Vec<String>,
    #[serde(default)]
    pub recommended_actions: Vec<String>,
    #[serde(default)]
    pub required_sdks: Vec<String>,
    #[serde(default)]
    pub workflow_stages: Vec<String>,
    pub preview_kind: String,
    pub gate: String,
    #[serde(default)]
    pub asset_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DomainWorkbenchToolDescriptor {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub sdk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DomainWorkbenchStageDescriptor {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub inputs: Vec<String>,
    #[serde(default)]
    pub outputs: Vec<String>,
    #[serde(default)]
    pub gate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualizationRendererDescriptor {
    pub id: String,
    pub label: String,
    pub dimensions: String,
    pub supports_zoom: bool,
    pub supports_pan: bool,
    pub supports_animation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainLifecycleDescriptor {
    pub states: Vec<String>,
    pub supports_hot_reload: bool,
    pub supports_workspace_sync: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainPluginDescriptor {
    pub metadata: DomainMetadata,
    pub capabilities: Vec<String>,
    pub supported_file_types: Vec<String>,
    pub supported_visualizations: Vec<DomainVisualizationDescriptor>,
    pub supported_agents: Vec<String>,
    pub context_provider: DomainProviderDescriptor,
    pub preview_provider: DomainProviderDescriptor,
    pub execution_provider: DomainProviderDescriptor,
    pub data_provider: DomainProviderDescriptor,
    pub visualization_provider: DomainProviderDescriptor,
    pub render_provider: DomainProviderDescriptor,
    pub lifecycle: DomainLifecycleDescriptor,
    #[serde(default)]
    pub sdk_adapters: Vec<String>,
    #[serde(default)]
    pub plugin_api_version: String,
    #[serde(default)]
    pub workbench: DomainWorkbenchDescriptor,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainAsset {
    pub id: String,
    pub source_id: String,
    pub domain_id: String,
    pub path: String,
    pub name: String,
    pub file_type: String,
    pub size_bytes: u64,
    pub modified_at: String,
    pub content_revision: String,
    pub visualizations: Vec<DomainVisualizationDescriptor>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainWorkspaceSummary {
    pub domain_id: String,
    pub asset_count: usize,
    pub visualization_count: usize,
    pub revision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_modified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainWorkspace {
    pub schema_version: String,
    pub generated_at: String,
    pub domain: DomainPluginDescriptor,
    pub assets: Vec<DomainAsset>,
    pub revision: String,
    #[serde(default)]
    pub execution: Value,
    #[serde(default)]
    pub state: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainInference {
    pub domain_id: String,
    pub confidence: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResearchDomainCatalog {
    pub schema_version: String,
    pub generated_at: String,
    pub plugin_api_version: String,
    pub plugins: Vec<DomainPluginDescriptor>,
    pub renderers: Vec<VisualizationRendererDescriptor>,
    pub workspaces: Vec<DomainWorkspaceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_domain: Option<DomainInference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainContextSnapshot {
    pub schema_version: String,
    pub generated_at: String,
    pub inference: DomainInference,
    pub plugin: DomainPluginDescriptor,
    pub assets: Vec<DomainAsset>,
    pub agent_context: String,
}
