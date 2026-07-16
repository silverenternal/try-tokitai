//! Atlas Core: the object-oriented kernel behind the existing Atlas IDE.
//!
//! UI surfaces keep their current contracts. Adapters translate files, domain
//! tasks and legacy Research OS records into stable [`ScientificObject`]s.

mod adapters;
mod core;
mod event_bus;
mod model;
mod storage;

pub use adapters::{
    domain_asset_object, domain_task_object, domain_workspace_object, legacy_research_object,
};
pub use core::{AtlasCore, ObjectComparison, ObjectGraph};
pub use event_bus::{EventBus, EventListener};
pub use model::*;
pub use storage::{FileObjectStore, ObjectStore};
