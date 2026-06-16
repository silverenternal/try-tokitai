//! AI Scientist Agent Implementations
//!
//! Five specialized agents:
//! - ResearchAgent: literature search, paper analysis
//! - HypothesisAgent: hypothesis generation and refinement
//! - ExperimentAgent: experiment design and execution
//! - VerificationAgent: mathematical and formal verification
//! - ReportAgent: paper/report generation

mod research;
mod hypothesis;
mod experiment;
mod verification;
mod report;

pub use research::ResearchAgent;
pub use hypothesis::HypothesisAgent;
pub use experiment::ExperimentAgent;
pub use verification::VerificationAgent;
pub use report::ReportAgent;
