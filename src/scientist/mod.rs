//! AI Scientist – Integration Layer
//!
//! Integrates the ai-scientist-* crates with Atlas.
//! Provides concrete agent implementations, CS-oriented tools, and workflows.

pub mod agents;
pub mod tools;
pub mod workflow;

pub use agents::{ExperimentAgent, HypothesisAgent, ReportAgent, ResearchAgent, VerificationAgent};
