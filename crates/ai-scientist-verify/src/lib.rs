//! AI Scientist Verification — Math & Formal Verification
//!
//! Two verification pipelines:
//! - **SymPy**: Mathematical verification (simplify, solve, integrate, differentiate, matrix ops)
//! - **Lean4**: Formal theorem proving (verify theorem/lemma/proof correctness)

pub mod lean4;
pub mod sympy;

pub use lean4::{LeanVerifier, LeanVerificationResult};
pub use sympy::{SymPyVerifier, SymPyResult};
