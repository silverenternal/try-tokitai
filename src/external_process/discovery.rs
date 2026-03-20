//! External Process Wrapper - Auto-Discovery System
//!
//! Automatic discovery of external tools from:
//! - System PATH (executables)
//! - Script directories
//! - OpenAPI/Swagger specifications
//!
//! ## Overview
//! This module provides the `ExternalToolDiscovery` struct that scans
//! for external tools and generates tool metadata using AI assistance.

use crate::external_process::metadata::{
    ExternalToolMetadata,
    ExternalToolType,
    ProcessConfig,
    ScriptConfig,
    RiskLevel,
};
use crate::external_process::script_wrapper::script_scanner;
use crate::external_process::http_wrapper::openapi_parser;
use crate::external_process::wrapper::ExternalTool;
use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// External tool discovery system
///
/// Scans for external tools and generates metadata for registration.
///
/// ## Example
/// ```rust,ignore
/// use crate::external_process::discovery::ExternalToolDiscovery;
///
/// let discovery = ExternalToolDiscovery::new();
///
/// // Scan system PATH for executables
/// let tools = discovery.scan_executables().await?;
///
/// // Scan directory for scripts
/// let scripts = discovery.scan_scripts("./scripts").await?;
///
/// // Load from OpenAPI spec
/// let http_tools = discovery.from_openapi("https://api.example.com/openapi.json").await?;
/// ```
pub struct ExternalToolDiscovery {
    /// Search paths for executables
    search_paths: Vec<PathBuf>,
    /// Discovered tools
    discovered_tools: Vec<ExternalToolMetadata>,
    /// Script directories to scan
    script_dirs: Vec<PathBuf>,
    /// Created by identifier
    created_by: String,
}

impl ExternalToolDiscovery {
    /// Create a new discovery system
    pub fn new() -> Self {
        Self {
            search_paths: get_system_path(),
            discovered_tools: Vec::new(),
            script_dirs: Vec::new(),
            created_by: "discovery_system".to_string(),
        }
    }

    /// Set created by identifier
    pub fn with_created_by(mut self, created_by: impl Into<String>) -> Self {
        self.created_by = created_by.into();
        self
    }

    /// Add script directory to scan
    pub fn with_script_dir(mut self, dir: PathBuf) -> Self {
        self.script_dirs.push(dir);
        self
    }

    /// Get system PATH
    pub fn get_search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Scan system PATH for executables
    ///
    /// This method scans common system directories for executable files
    /// and generates metadata for each discovered executable.
    ///
    /// # Returns
    /// * `Result<Vec<ExternalToolMetadata>>` - Discovered tool metadata
    pub async fn scan_executables(&mut self) -> Result<Vec<ExternalToolMetadata>> {
        info!("Scanning system PATH for executables...");
        
        let mut tools = Vec::new();
        
        // Common CLI tools to check for
        let common_tools = vec![
            ("git", "version_control", "Git version control"),
            ("docker", "container", "Docker container management"),
            ("npm", "package_manager", "Node.js package manager"),
            ("yarn", "package_manager", "Yarn package manager"),
            ("cargo", "package_manager", "Rust package manager"),
            ("python3", "interpreter", "Python 3 interpreter"),
            ("python", "interpreter", "Python interpreter"),
            ("node", "interpreter", "Node.js runtime"),
            ("curl", "network", "curl HTTP client"),
            ("wget", "network", "wget download utility"),
            ("grep", "text_processing", "grep text search"),
            ("sed", "text_processing", "sed stream editor"),
            ("awk", "text_processing", "awk pattern scanning"),
            ("jq", "data_processing", "jq JSON processor"),
            ("find", "file_system", "find file search"),
            ("rsync", "file_system", "rsync file synchronization"),
            ("tar", "file_system", "tar archiving utility"),
            ("zip", "file_system", "zip compression utility"),
            ("unzip", "file_system", "unzip extraction utility"),
            ("ssh", "network", "SSH client"),
            ("scp", "network", "SCP file transfer"),
            ("rsync", "network", "rsync remote synchronization"),
        ];

        for (tool_name, domain, description) in common_tools {
            if let Some(tool_path) = find_executable(tool_name, &self.search_paths) {
                debug!("Found executable: {} at {:?}", tool_name, tool_path);
                
                // Create basic metadata
                let config = ProcessConfig::new(tool_name)
                    .with_timeout(30000);
                
                // Create minimal input schema (can be enriched later)
                let input_schema = serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "description": format!("Execute {} command", tool_name)
                });
                
                let metadata = ExternalToolMetadata::new(
                    format!("cli_{}", tool_name.replace('.', "_")),
                    format!("{} - {}", description, tool_path.display()),
                    ExternalToolType::process(config),
                    input_schema,
                    domain,
                    &self.created_by,
                )
                .with_tags(vec![
                    "cli".to_string(),
                    "auto_discovered".to_string(),
                    tool_name.to_string(),
                ])
                .with_risk_level(RiskLevel::Medium);
                
                tools.push(metadata);
            }
        }

        info!("Discovered {} executables", tools.len());
        self.discovered_tools.extend(tools.clone());
        Ok(tools)
    }

    /// Scan directory for script files
    ///
    /// # Arguments
    /// * `dir` - Directory to scan
    ///
    /// # Returns
    /// * `Result<Vec<ExternalToolMetadata>>` - Discovered script tool metadata
    pub async fn scan_scripts<P: AsRef<Path>>(&mut self, dir: P) -> Result<Vec<ExternalToolMetadata>> {
        let dir = dir.as_ref();
        info!("Scanning directory for scripts: {:?}", dir);
        
        if !dir.exists() {
            bail!("Directory does not exist: {:?}", dir);
        }

        let scripts = script_scanner::scan_directory(dir, true);
        let mut tools = Vec::new();

        for script_path in &scripts {
            if let Some(filename) = script_path.file_stem().and_then(|s| s.to_str()) {
                // Generate tool name
                let tool_name = format!("script_{}", filename.replace('.', "_"));
                
                // Generate description
                let description = format!("Execute script: {}", script_path.display());
                
                // Create config
                let config = ScriptConfig::new(script_path.clone());
                
                // Create minimal input schema
                let input_schema = serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "description": description
                });
                
                let metadata = ExternalToolMetadata::new(
                    &tool_name,
                    &description,
                    ExternalToolType::script(config),
                    input_schema,
                    "script",
                    &self.created_by,
                )
                .with_tags(vec![
                    "script".to_string(),
                    "auto_discovered".to_string(),
                    filename.to_string(),
                ])
                .with_risk_level(RiskLevel::Medium);
                
                tools.push(metadata);
            }
        }

        info!("Discovered {} scripts", tools.len());
        self.discovered_tools.extend(tools.clone());
        Ok(tools)
    }

    /// Load HTTP tools from OpenAPI specification
    ///
    /// # Arguments
    /// * `openapi_url_or_path` - URL or file path to OpenAPI spec
    ///
    /// # Returns
    /// * `Result<Vec<ExternalToolMetadata>>` - Discovered HTTP tool metadata
    pub async fn from_openapi(&mut self, openapi_url_or_path: &str) -> Result<Vec<ExternalToolMetadata>> {
        info!("Loading tools from OpenAPI spec: {}", openapi_url_or_path);
        
        // Fetch or load OpenAPI spec
        let spec = load_openapi_spec(openapi_url_or_path).await?;
        
        // Parse OpenAPI and generate wrappers
        let wrappers = openapi_parser::parse_openapi(&spec, &self.created_by)?;
        
        // Convert wrappers to metadata
        let mut tools = Vec::new();
        for wrapper in &wrappers {
            tools.push(wrapper.metadata().clone());
        }
        
        info!("Discovered {} HTTP tools from OpenAPI", tools.len());
        self.discovered_tools.extend(tools.clone());
        Ok(tools)
    }

    /// AI-enrich metadata for a tool
    ///
    /// Uses LLM to generate better descriptions and input schemas.
    ///
    /// # Arguments
    /// * `tool` - Tool metadata to enrich
    ///
    /// # Returns
    /// * `Result<ExternalToolMetadata>` - Enriched metadata
    pub async fn ai_enrich_metadata(&self, mut tool: ExternalToolMetadata) -> Result<ExternalToolMetadata> {
        // This would normally call an LLM to generate better metadata
        // For now, we provide a simplified implementation
        
        debug!("Enriching metadata for tool: {}", tool.name);
        
        // Generate a better description based on tool type and name
        let enriched_description = match &tool.tool_type {
            ExternalToolType::Process { config } => {
                format!(
                    "Execute the '{}' command. This is a CLI tool that can be used for {} operations.",
                    config.executable,
                    tool.domain
                )
            }
            ExternalToolType::Http { config } => {
                format!(
                    "Make a {} request to {}. This HTTP endpoint is part of the {} domain.",
                    config.method,
                    config.path_template,
                    tool.domain
                )
            }
            ExternalToolType::Script { config } => {
                format!(
                    "Execute the script at {:?}. This script is used for {} operations.",
                    config.script_path,
                    tool.domain
                )
            }
        };
        
        tool.description = enriched_description;
        
        // Add common tags
        if !tool.tags.contains(&"auto_generated".to_string()) {
            tool.tags.push("auto_generated".to_string());
        }
        
        Ok(tool)
    }

    /// Get all discovered tools
    pub fn get_discovered_tools(&self) -> &[ExternalToolMetadata] {
        &self.discovered_tools
    }

    /// Clear discovered tools
    pub fn clear_discovered_tools(&mut self) {
        self.discovered_tools.clear();
    }
}

impl Default for ExternalToolDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Get system PATH environment variable
fn get_system_path() -> Vec<PathBuf> {
    let path_env = std::env::var("PATH").unwrap_or_default();
    
    #[cfg(windows)]
    let separator = ';';
    #[cfg(not(windows))]
    let separator = ':';
    
    path_env
        .split(separator)
        .map(PathBuf::from)
        .collect()
}

/// Find executable in search paths
fn find_executable(name: &str, search_paths: &[PathBuf]) -> Option<PathBuf> {
    // Check if it's an absolute path
    let absolute = PathBuf::from(name);
    if absolute.is_absolute() && absolute.exists() {
        return Some(absolute);
    }

    // Search in PATH
    for path_dir in search_paths {
        let candidate = path_dir.join(name);
        
        #[cfg(windows)]
        {
            // Try with .exe extension
            let candidate_exe = candidate.with_extension("exe");
            if candidate_exe.exists() {
                return Some(candidate_exe);
            }
        }
        
        if candidate.exists() {
            // Check if executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(metadata) = candidate.metadata() {
                    let mode = metadata.permissions().mode();
                    if mode & 0o111 != 0 {
                        return Some(candidate);
                    }
                }
            }
            
            #[cfg(windows)]
            {
                return Some(candidate);
            }
            
            #[cfg(not(any(unix, windows)))]
            {
                return Some(candidate);
            }
        }
    }

    None
}

/// Load OpenAPI spec from URL or file
async fn load_openapi_spec(source: &str) -> Result<Value> {
    if source.starts_with("http://") || source.starts_with("https://") {
        // Fetch from URL
        debug!("Fetching OpenAPI spec from URL: {}", source);
        let response = reqwest::get(source)
            .await
            .context("Failed to fetch OpenAPI spec")?;
        
        if !response.status().is_success() {
            bail!("Failed to fetch OpenAPI spec: {}", response.status());
        }
        
        let spec: Value = response.json()
            .await
            .context("Failed to parse OpenAPI spec JSON")?;
        
        Ok(spec)
    } else {
        // Load from file
        let path = PathBuf::from(source);
        debug!("Loading OpenAPI spec from file: {:?}", path);
        
        if !path.exists() {
            bail!("OpenAPI spec file not found: {:?}", path);
        }
        
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read OpenAPI spec file: {:?}", path))?;

        // Try JSON first
        if let Ok(spec) = serde_json::from_str::<Value>(&content) {
            return Ok(spec);
        }

        // Try YAML (note: YAML support requires the 'yaml' feature)
        // To enable: add `yaml = ["serde_yaml"]` to Cargo.toml features
        #[cfg(feature = "yaml")]
        {
            if let Ok(spec) = serde_yaml::from_str::<Value>(&content) {
                return Ok(spec);
            }
        }

        bail!("Failed to parse OpenAPI spec as JSON or YAML");
    }
}

/// Scan and discover all tools in a workspace
pub async fn discover_workspace_tools(
    script_dirs: &[PathBuf],
    openapi_specs: &[String],
    created_by: &str,
) -> Result<Vec<ExternalToolMetadata>> {
    info!("Discovering workspace tools...");
    
    let mut discovery = ExternalToolDiscovery::new()
        .with_created_by(created_by);
    
    // Add script directories
    for dir in script_dirs {
        discovery = discovery.with_script_dir(dir.clone());
    }
    
    let mut all_tools = Vec::new();
    
    // Scan executables
    match discovery.scan_executables().await {
        Ok(tools) => {
            info!("Found {} CLI tools", tools.len());
            all_tools.extend(tools);
        }
        Err(e) => {
            warn!("Failed to scan executables: {}", e);
        }
    }
    
    // Scan script directories
    for dir in script_dirs {
        match discovery.scan_scripts(dir).await {
            Ok(tools) => {
                info!("Found {} scripts in {:?}", tools.len(), dir);
                all_tools.extend(tools);
            }
            Err(e) => {
                warn!("Failed to scan scripts in {:?}: {}", dir, e);
            }
        }
    }
    
    // Load OpenAPI specs
    for spec_source in openapi_specs {
        match discovery.from_openapi(spec_source).await {
            Ok(tools) => {
                info!("Found {} HTTP tools from OpenAPI", tools.len());
                all_tools.extend(tools);
            }
            Err(e) => {
                warn!("Failed to load OpenAPI from {}: {}", spec_source, e);
            }
        }
    }
    
    info!("Total discovered tools: {}", all_tools.len());
    Ok(all_tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use crate::external_process::metadata::ProcessConfig;

    #[tokio::test]
    async fn test_discovery_creation() {
        let discovery = ExternalToolDiscovery::new();
        assert!(!discovery.search_paths.is_empty());
        assert!(discovery.discovered_tools.is_empty());
    }

    #[tokio::test]
    async fn test_scan_executables() {
        let mut discovery = ExternalToolDiscovery::new();
        let tools = discovery.scan_executables().await.unwrap();
        
        // Should find at least some common tools
        assert!(!tools.is_empty());
        
        // Verify metadata structure
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.tags.contains(&"auto_discovered".to_string()));
        }
    }

    #[tokio::test]
    async fn test_scan_scripts() {
        let temp_dir = TempDir::new().unwrap();
        
        // Create test scripts
        fs::write(temp_dir.path().join("test1.sh"), "#!/bin/bash\necho test").unwrap();
        fs::write(temp_dir.path().join("test2.py"), "#!/usr/bin/env python3\nprint('test')").unwrap();
        
        let mut discovery = ExternalToolDiscovery::new();
        let tools = discovery.scan_scripts(temp_dir.path()).await.unwrap();
        
        assert_eq!(tools.len(), 2);
        
        for tool in &tools {
            assert!(tool.name.starts_with("script_"));
            assert_eq!(tool.tags.iter().filter(|t| *t == "auto_discovered").count(), 1);
        }
    }

    #[test]
    fn test_find_executable() {
        let paths = get_system_path();
        
        // Should find common executables
        let found = find_executable("ls", &paths);
        #[cfg(unix)]
        assert!(found.is_some());
        
        // Should not find non-existent executable
        let not_found = find_executable("nonexistent_binary_xyz123", &paths);
        assert!(not_found.is_none());
    }

    #[tokio::test]
    async fn test_ai_enrichment() {
        let discovery = ExternalToolDiscovery::new();

        let config = ProcessConfig::new("git");
        let tool = ExternalToolMetadata::new(
            "cli_git",
            "Git CLI",
            ExternalToolType::process(config),
            serde_json::json!({"type": "object"}),
            "version_control",
            "test",
        );

        let enriched = discovery.ai_enrich_metadata(tool).await.unwrap();

        assert!(!enriched.description.is_empty());
        // Description should be enriched with more details
        assert!(enriched.description.len() > "Git CLI".len());
        assert!(enriched.tags.contains(&"auto_generated".to_string()));
    }
}
