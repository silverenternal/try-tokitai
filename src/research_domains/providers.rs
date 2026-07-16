use super::model::{
    DomainAsset, DomainInference, DomainPluginDescriptor, DomainVisualizationDescriptor,
    DomainWorkspace,
};
use crate::visualization::model::VisualizationDocument;
use anyhow::Result;
use serde_json::Value;
use std::path::Path;

pub struct DomainProviderContext<'a> {
    pub workspace_root: &'a Path,
    pub query: Option<&'a str>,
    pub runtime: &'a Value,
}

#[allow(non_camel_case_types)]
pub trait IDataProvider: Send + Sync {
    fn discover_assets(&self, context: &DomainProviderContext<'_>) -> Result<Vec<DomainAsset>>;
}

#[allow(non_camel_case_types)]
pub trait IVisualizationProvider: Send + Sync {
    fn visualization_document(
        &self,
        context: &DomainProviderContext<'_>,
        asset: &DomainAsset,
        visualization_id: Option<&str>,
    ) -> Result<VisualizationDocument>;
}

#[allow(non_camel_case_types)]
pub trait IAgentContextProvider: Send + Sync {
    fn agent_context(
        &self,
        context: &DomainProviderContext<'_>,
        inference: &DomainInference,
        assets: &[DomainAsset],
    ) -> Result<String>;
}

#[allow(non_camel_case_types)]
pub trait IPreviewProvider: Send + Sync {
    fn preview_metadata(
        &self,
        document: &VisualizationDocument,
    ) -> Result<serde_json::Map<String, Value>>;
}

#[allow(non_camel_case_types)]
pub trait IRenderProvider: Send + Sync {
    fn renderers(&self) -> Vec<DomainVisualizationDescriptor>;
}

#[allow(non_camel_case_types)]
pub trait IExecutionProvider: Send + Sync {
    fn execution_context(&self, context: &DomainProviderContext<'_>) -> Result<Value>;
}

#[allow(non_camel_case_types)]
pub trait IDomainPlugin:
    IDataProvider
    + IVisualizationProvider
    + IAgentContextProvider
    + IPreviewProvider
    + IRenderProvider
    + IExecutionProvider
    + Send
    + Sync
{
    fn descriptor(&self) -> &DomainPluginDescriptor;

    fn on_register(&self, _context: &DomainProviderContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_activate(&self, _context: &DomainProviderContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_deactivate(&self, _context: &DomainProviderContext<'_>) -> Result<()> {
        Ok(())
    }

    fn on_workspace_change(
        &self,
        _context: &DomainProviderContext<'_>,
        _workspace: &DomainWorkspace,
    ) -> Result<()> {
        Ok(())
    }
}
