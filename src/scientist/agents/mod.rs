//! AI Scientist Agent Implementations
//!
//! Five specialized agents:
//! - ResearchAgent: literature search, paper analysis
//! - HypothesisAgent: hypothesis generation and refinement
//! - ExperimentAgent: experiment design and execution
//! - VerificationAgent: mathematical and formal verification
//! - ReportAgent: paper/report generation

mod experiment;
mod hypothesis;
mod report;
mod research;
mod verification;

pub use experiment::ExperimentAgent;
pub use hypothesis::HypothesisAgent;
pub use report::ReportAgent;
pub use research::ResearchAgent;
pub use verification::VerificationAgent;
