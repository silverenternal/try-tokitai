//! Research Intelligence Engine (RIE).
//!
//! RIE is deliberately headless. Existing Atlas panels remain unchanged and
//! consume its object, plan, execution and recommendation records through the
//! compatibility APIs in `web.rs`.

mod execution;
mod model;
mod planning;
mod plugin;
mod query;
mod recommendation;

pub use execution::{ExecutionEngine, RuntimeAdapter, RuntimeRegistry};
pub use model::*;
pub use planning::PlanningEngine;
pub use plugin::{Plugin, PluginContext, PluginRegistry};
pub use query::{ObjectQueryEngine, QueryResult};
pub use recommendation::RecommendationEngine;

use crate::atlas_core::AtlasCore;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug)]
pub struct ResearchIntelligenceEngine {
    pub core: Arc<AtlasCore>,
    pub planning: PlanningEngine,
    pub execution: ExecutionEngine,
    pub recommendations: RecommendationEngine,
    pub query: ObjectQueryEngine,
    pub plugins: Arc<PluginRegistry>,
}

impl ResearchIntelligenceEngine {
    pub fn open(workspace_root: &Path) -> Result<Self> {
        let core = Arc::new(AtlasCore::open(workspace_root)?);
        let runtimes = Arc::new(RuntimeRegistry::default());
        let plugins = Arc::new(PluginRegistry::new(core.event_bus()));
        Ok(Self {
            planning: PlanningEngine::new(core.clone()),
            execution: ExecutionEngine::new(core.clone(), runtimes),
            recommendations: RecommendationEngine::new(core.clone()),
            query: ObjectQueryEngine::new(core.clone()),
            plugins,
            core,
        })
    }
}
