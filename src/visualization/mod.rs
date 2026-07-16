mod adapters;
pub mod model;

use anyhow::{anyhow, Result};
use model::{
    VisualizationCatalog, VisualizationDocument, VisualizationSource, VisualizationTypeDescriptor,
    VISUALIZATION_SCHEMA_VERSION,
};
use serde_json::Value;
use std::path::Path;

pub use adapters::{
    AlgorithmAdapter, MultiAgentAdapter, NetworkAdapter, PaperAdapter, SystemAdapter,
};

pub const VISUALIZATION_PLUGIN_API_VERSION: &str = "1";

pub struct VisualizationContext<'a> {
    pub workspace_root: &'a Path,
    pub source_id: Option<&'a str>,
    pub runtime: &'a Value,
}

/// Stable backend plugin boundary. Parsers produce the shared document model;
/// renderers never receive parser-specific types.
pub trait VisualizationAdapter: Send + Sync {
    fn descriptor(&self) -> VisualizationTypeDescriptor;
    fn discover(&self, context: &VisualizationContext<'_>) -> Result<Vec<VisualizationSource>>;
    fn parse(&self, context: &VisualizationContext<'_>) -> Result<VisualizationDocument>;
}

pub struct VisualizationRegistry {
    adapters: Vec<Box<dyn VisualizationAdapter>>,
}

impl Default for VisualizationRegistry {
    fn default() -> Self {
        let mut registry = Self {
            adapters: Vec::new(),
        };
        registry.register(SystemAdapter::default());
        registry.register(PaperAdapter);
        registry.register(MultiAgentAdapter);
        registry
    }
}

impl VisualizationRegistry {
    pub fn register<A>(&mut self, adapter: A)
    where
        A: VisualizationAdapter + 'static,
    {
        self.adapters.push(Box::new(adapter));
    }

    pub fn catalog(&self, context: &VisualizationContext<'_>) -> VisualizationCatalog {
        let mut types = Vec::with_capacity(self.adapters.len());
        let mut sources = Vec::new();
        for adapter in &self.adapters {
            types.push(adapter.descriptor());
            match adapter.discover(context) {
                Ok(mut discovered) => sources.append(&mut discovered),
                Err(error) => tracing::warn!(
                    adapter = %adapter.descriptor().adapter_id,
                    error = %error,
                    "visualization source discovery failed"
                ),
            }
        }
        VisualizationCatalog {
            schema_version: VISUALIZATION_SCHEMA_VERSION.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            types,
            sources,
        }
    }

    pub fn parse(
        &self,
        kind: &str,
        context: &VisualizationContext<'_>,
    ) -> Result<VisualizationDocument> {
        let adapter = self
            .adapters
            .iter()
            .find(|adapter| adapter.descriptor().kind == kind)
            .ok_or_else(|| anyhow!("unknown visualization kind: {kind}"))?;
        adapter.parse(context)
    }
}

pub fn type_descriptor(
    kind: &str,
    label: &str,
    description: &str,
    adapter_id: &str,
) -> VisualizationTypeDescriptor {
    VisualizationTypeDescriptor {
        kind: kind.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        adapter_id: adapter_id.to_string(),
        plugin_api_version: VISUALIZATION_PLUGIN_API_VERSION.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::fs;

    fn context<'a>(
        root: &'a Path,
        source_id: Option<&'a str>,
        runtime: &'a Value,
    ) -> VisualizationContext<'a> {
        VisualizationContext {
            workspace_root: root,
            source_id,
            runtime,
        }
    }

    #[test]
    fn registry_discovers_and_parses_real_workspace_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("paper.md"),
            "# Runtime Study\n\n## Method\nFirst collect measurements. Then analyze the trace [1].\n\n## Results\nThe runtime improved.\n\n## References\n[1] A measured systems paper.\n",
        )
        .unwrap();

        let runtime = json!({
            "sessions": [{
                "session_id": "session-real",
                "title": "Live task",
                "status": "running",
                "context_used_tokens": 512,
                "context_window": 4096,
                "tool_events": [{"call_id": "call-1", "name": "read_file", "status": "complete"}],
                "subagents": [{"id": "worker-1", "name": "Worker", "status": "running"}],
                "timeline": [{"kind": "task", "title": "Inspect", "status": "running", "agent": "worker-1", "ts": "2026-01-01T00:00:00Z"}]
            }]
        });
        let registry = VisualizationRegistry::default();
        let catalog = registry.catalog(&context(temp.path(), None, &runtime));
        assert_eq!(catalog.types.len(), 3);
        assert!(!catalog
            .types
            .iter()
            .any(|descriptor| matches!(descriptor.kind.as_str(), "algorithm" | "network")));
        assert!(catalog
            .sources
            .iter()
            .any(|source| source.id == "workspace:paper.md"));

        let paper = registry
            .parse(
                "paper",
                &context(temp.path(), Some("workspace:paper.md"), &runtime),
            )
            .unwrap();
        assert!(paper.nodes.iter().any(|node| node.category == "section"));
        assert!(paper.edges.iter().any(|edge| edge.category == "citation"));

        let agents = registry
            .parse(
                "multi-agent",
                &context(temp.path(), Some("runtime:agent:session-real"), &runtime),
            )
            .unwrap();
        assert!(agents
            .nodes
            .iter()
            .any(|node| node.category == "agent" && node.label == "Worker"));
        assert!(agents.nodes.iter().any(|node| node.category == "tool"));
        assert_eq!(agents.metadata["runtime"]["context_used_tokens"], 512);
    }

    #[test]
    fn adapter_registration_does_not_change_renderer_schema() {
        struct EmptyPlugin;
        impl VisualizationAdapter for EmptyPlugin {
            fn descriptor(&self) -> VisualizationTypeDescriptor {
                type_descriptor("plugin-test", "Plugin Test", "Test adapter", "test.plugin")
            }
            fn discover(
                &self,
                _context: &VisualizationContext<'_>,
            ) -> Result<Vec<VisualizationSource>> {
                Ok(Vec::new())
            }
            fn parse(&self, _context: &VisualizationContext<'_>) -> Result<VisualizationDocument> {
                Ok(VisualizationDocument::empty(
                    "plugin-test",
                    "Plugin Test",
                    VisualizationSource {
                        id: "plugin:test".to_string(),
                        kind: "plugin-test".to_string(),
                        label: "Plugin".to_string(),
                        source_type: "plugin".to_string(),
                        live: false,
                        metadata: Default::default(),
                    },
                ))
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let runtime = json!({});
        let mut registry = VisualizationRegistry::default();
        registry.register(EmptyPlugin);
        let document = registry
            .parse("plugin-test", &context(temp.path(), None, &runtime))
            .unwrap();
        assert_eq!(document.schema_version, VISUALIZATION_SCHEMA_VERSION);
        assert_eq!(document.kind, "plugin-test");
    }
}
