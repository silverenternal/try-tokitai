//! Configuration for the AI Scientist platform
//!
//! TOML-based configuration with serde deserialization.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level configuration for the AI Scientist platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScientistConfig {
    /// Agent configuration
    #[serde(default)]
    pub agents: AgentsConfig,

    /// RAG system configuration
    #[serde(default)]
    pub rag: RagConfig,

    /// Verification tools configuration
    #[serde(default)]
    pub verification: VerificationConfig,

    /// Security configuration
    #[serde(default)]
    pub security: SecurityConfig,

    /// Workflow configuration
    #[serde(default)]
    pub workflow: WorkflowConfig,

    /// Storage paths
    #[serde(default)]
    pub storage: StorageConfig,
}

impl Default for ScientistConfig {
    fn default() -> Self {
        Self {
            agents: AgentsConfig::default(),
            rag: RagConfig::default(),
            verification: VerificationConfig::default(),
            security: SecurityConfig::default(),
            workflow: WorkflowConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

impl ScientistConfig {
    /// Load from a TOML file
    pub fn load(path: Option<PathBuf>) -> Result<Self, config_loader_error::Error> {
        let path = path.unwrap_or_else(|| PathBuf::from("ai_scientist.toml"));
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }
}

// Workaround for error types
mod config_loader_error {
    #[derive(Debug)]
    pub enum Error {
        Io(std::io::Error),
        Parse(toml::de::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Io(e) => write!(f, "IO error: {}", e),
                Error::Parse(e) => write!(f, "Parse error: {}", e),
            }
        }
    }

    impl std::error::Error for Error {}

    impl From<std::io::Error> for Error {
        fn from(e: std::io::Error) -> Self {
            Error::Io(e)
        }
    }

    impl From<toml::de::Error> for Error {
        fn from(e: toml::de::Error) -> Self {
            Error::Parse(e)
        }
    }
}

// ============================================================================
// Sub-configs
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// Number of agents per role
    pub researcher_count: usize,
    pub hypothesizer_count: usize,
    pub experimenter_count: usize,
    pub verifier_count: usize,
    pub reporter_count: usize,
    /// Timeout per agent message (seconds)
    pub message_timeout_secs: u64,
    /// Max retries per message
    pub max_retries: u32,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            researcher_count: 1,
            hypothesizer_count: 1,
            experimenter_count: 1,
            verifier_count: 1,
            reporter_count: 1,
            message_timeout_secs: 300,
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagConfig {
    /// Embedding model name
    pub embedding_model: String,
    /// Vector DB URL (Qdrant)
    pub vector_db_url: String,
    /// Chunk size in characters
    pub chunk_size: usize,
    /// Chunk overlap in characters
    pub chunk_overlap: usize,
    /// Number of results for retrieval
    pub top_k: usize,
    /// Similarity threshold (0.0 - 1.0)
    pub similarity_threshold: f64,
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            embedding_model: "text-embedding-3-small".into(),
            vector_db_url: "http://localhost:6333".into(),
            chunk_size: 1000,
            chunk_overlap: 200,
            top_k: 10,
            similarity_threshold: 0.7,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Path to Python executable for SymPy
    pub python_path: String,
    /// Path to Lean4 lake executable
    pub lean_lake_path: String,
    /// Verification timeout (seconds)
    pub timeout_secs: u64,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            python_path: "python3".into(),
            lean_lake_path: "lake".into(),
            timeout_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable RBAC
    pub enable_rbac: bool,
    /// Enable sandbox execution
    pub enable_sandbox: bool,
    /// Enable prompt injection detection
    pub enable_injection_detection: bool,
    /// Allowed tool risk levels
    pub allowed_risk_levels: Vec<String>,
    /// Require approval for risk levels
    pub approval_risk_levels: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_rbac: true,
            enable_sandbox: false,
            enable_injection_detection: true,
            allowed_risk_levels: vec!["safe".into(), "low".into(), "moderate".into()],
            approval_risk_levels: vec!["high".into(), "critical".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowConfig {
    /// Path to workflow TOML files
    pub workflow_dir: String,
    /// Auto-advance stages
    pub auto_advance: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            workflow_dir: "workflows".into(),
            auto_advance: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Data directory
    pub data_dir: String,
    /// Paper storage directory
    pub papers_dir: String,
    /// Experiment results directory
    pub results_dir: String,
    /// Vector DB storage directory (for local/embedded DBs)
    pub vector_dir: String,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: ".ai_scientist/data".into(),
            papers_dir: ".ai_scientist/papers".into(),
            results_dir: ".ai_scientist/results".into(),
            vector_dir: ".ai_scientist/vectors".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ScientistConfig::default();
        assert_eq!(config.agents.researcher_count, 1);
        assert_eq!(config.rag.chunk_size, 1000);
    }

    #[test]
    fn test_config_serialization() {
        let config = ScientistConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: ScientistConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.agents.researcher_count, config.agents.researcher_count);
    }
}
