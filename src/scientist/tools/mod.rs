//! AI Scientist Tools
//!
//! Reusable tools for computer science research and engineering workflows.

pub mod computation;
pub mod data;
#[cfg(feature = "domain-science")]
pub mod domain_science;
pub mod github;
pub mod literature;
pub mod privacy;
pub mod security;
pub mod sympy_tool;
pub mod verification_center;

#[cfg(test)]
mod integration_tests;
