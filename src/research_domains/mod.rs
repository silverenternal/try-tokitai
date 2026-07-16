mod actions;
mod adapters;
pub mod model;
mod providers;
mod registry;
mod state;
mod tasks;

pub use actions::{
    list_actions, run_action, DomainActionDescriptor, DomainActionRunRequest,
    DomainActionRunResponse,
};
pub use providers::{
    DomainProviderContext, IAgentContextProvider, IDataProvider, IDomainPlugin, IExecutionProvider,
    IPreviewProvider, IRenderProvider, IVisualizationProvider,
};
pub use registry::ResearchDomainRegistry;
pub use state::{read_workspace_state, update_workspace_state};
pub use tasks::{
    begin_task, intent_catalog, read_task, read_tasks, update_task, DomainTaskArtifact,
    DomainTaskBeginRequest, DomainTaskEvidence, DomainTaskRecord, DomainTaskUpdateRequest,
};
