//! AI Scientist Science — Computation Layer
//!
//! Trait-based interfaces for scientific computation domains:
//! - Chemistry (RDKit, Psi4)
//! - Biology (Biopython)
//! - Simulation (LAMMPS, OpenFOAM)
//!
//! All tools follow the unified Tool calling pattern for LLM integration.

pub mod biology;
pub mod chemistry;
pub mod environment;
pub mod python_bridge;
pub mod simulation;

pub use biology::{AutoBiologyTool, BiologyToolInterface, LocalBiologyTool};
pub use chemistry::{AutoChemistryTool, ChemistryToolInterface, LocalChemistryTool};
pub use environment::{detect_domain_environment, BackendStatus, DomainEnvironmentReport};
pub use simulation::{
    AutoSimulationTool, LocalSimulationTool, SimulationConfig, SimulationResult,
    SimulationToolInterface,
};
