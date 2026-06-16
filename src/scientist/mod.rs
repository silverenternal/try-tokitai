//! AI Scientist — Integration Layer
//!
//! Integrates the ai-scientist-* crates with the tokitai project.
//! Provides concrete agent implementations, scientist tools, and workflows.

pub mod agents;
pub mod tools;
pub mod workflow;

pub use agents::{
    ExperimentAgent, HypothesisAgent, ReportAgent, ResearchAgent, VerificationAgent,
};
