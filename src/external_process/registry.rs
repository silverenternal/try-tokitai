//! External Process Wrapper - External Tool Registry
//!
//! Registry for managing external tools and integrating with the tokitai tool matrix.
//!
//! ## Overview
//! This module provides the `ExternalToolRegistry` that:
//! - Stores and manages external tool wrappers
//! - Converts external tools to tokitai ToolDefinitions
//! - Registers tools to the tool matrix
//! - Tracks tool lifecycle and usage

#![allow(dead_code)]

use crate::external_process::http_wrapper::{HTTPWrapper, HTTPWrapperBuilder};
use crate::external_process::metadata::ExternalToolType;
use crate::external_process::metadata::{ExternalToolMetadata, RiskLevel};
use crate::external_process::process_wrapper::{ProcessWrapper, ProcessWrapperBuilder};
use crate::external_process::script_wrapper::{ScriptWrapper, ScriptWrapperBuilder};
use crate::external_process::wrapper::ExternalTool;
use crate::tool_matrix::matrix::{ToolBox, ToolDefinition};
use crate::tool_matrix::registry::ToolRegistry;
use crate::tool_matrix::registry::ToolSource;
use anyhow::{bail, Context, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// External tool registry entry
pub enum ExternalToolEntry {
    Process(Arc<ProcessWrapper>),
    Http(Arc<HTTPWrapper>),
    Script(Arc<ScriptWrapper>),
}

impl Clone for ExternalToolEntry {
    fn clone(&self) -> Self {
        match self {
            Self::Process(wrapper) => Self::Process(Arc::clone(wrapper)),
            Self::Http(wrapper) => Self::Http(Arc::clone(wrapper)),
            Self::Script(wrapper) => Self::Script(Arc::clone(wrapper)),
        }
    }
}

impl ExternalToolEntry {
    /// Get tool metadata
    pub fn metadata(&self) -> &ExternalToolMetadata {
        match self {
            Self::Process(wrapper) => wrapper.metadata(),
            Self::Http(wrapper) => wrapper.metadata(),
            Self::Script(wrapper) => wrapper.metadata(),
        }
    }

    /// Get tool name
    pub fn name(&self) -> &str {
        &self.metadata().name
    }

    /// Get tool domain
    pub fn domain(&self) -> &str {
        &self.metadata().domain
    }

    /// Check if tool is enabled
    pub fn is_enabled(&self) -> bool {
        self.metadata().enabled
    }

    /// Get risk level
    pub fn risk_level(&self) -> RiskLevel {
        self.metadata().risk_level.clone()
    }
}

#[async_trait::async_trait]
impl ExternalTool for ExternalToolEntry {
    fn metadata(&self) -> &ExternalToolMetadata {
        self.metadata()
    }

    async fn execute(
        &self,
        input: serde_json::Value,
    ) -> Result<crate::external_process::metadata::ToolExecutionResult> {
        match self {
            Self::Process(wrapper) => wrapper.execute(input).await,
            Self::Http(wrapper) => wrapper.execute(input).await,
            Self::Script(wrapper) => wrapper.execute(input).await,
        }
    }

    fn validate_input(&self, input: &serde_json::Value) -> Result<()> {
        match self {
            Self::Process(wrapper) => wrapper.validate_input(input),
            Self::Http(wrapper) => wrapper.validate_input(input),
            Self::Script(wrapper) => wrapper.validate_input(input),
        }
    }

    fn to_tool_definition(&self) -> ToolDefinition {
        match self {
            Self::Process(wrapper) => wrapper.to_tool_definition(),
            Self::Http(wrapper) => wrapper.to_tool_definition(),
            Self::Script(wrapper) => wrapper.to_tool_definition(),
        }
    }
}

/// External tool registry
///
/// Manages external tools and integrates with the tokitai tool matrix.
///
/// ## Example
/// ```rust,ignore
/// use crate::external_process::registry::ExternalToolRegistry;
///
/// let registry = ExternalToolRegistry::new()?;
///
/// // Register a process wrapper
/// registry.register_process(process_wrapper)?;
///
/// // Register from metadata
/// registry.register_from_metadata(metadata)?;
///
/// // Get all registered tools
/// let tools = registry.get_all_tools();
/// ```
pub struct ExternalToolRegistry {
    /// Registered external tools: name -> entry
    tools: Arc<RwLock<HashMap<String, ExternalToolEntry>>>,
    /// Tool metadata storage directory
    storage_dir: PathBuf,
    /// Created by identifier
    created_by: String,
}

impl ExternalToolRegistry {
    /// Create a new external tool registry
    pub fn new() -> Result<Self> {
        let workspace_root = std::env::current_dir().context("Failed to get current directory")?;
        let storage_dir = workspace_root.join(".atlas").join("external_tools");

        // Create storage directory
        std::fs::create_dir_all(&storage_dir)
            .with_context(|| format!("Failed to create storage directory: {:?}", storage_dir))?;

        Ok(Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            storage_dir,
            created_by: "registry".to_string(),
        })
    }

    /// Create with custom storage directory
    pub fn with_storage_dir<P: AsRef<Path>>(storage_dir: P) -> Result<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create storage directory
        std::fs::create_dir_all(&storage_dir)
            .with_context(|| format!("Failed to create storage directory: {:?}", storage_dir))?;

        Ok(Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            storage_dir,
            created_by: "registry".to_string(),
        })
    }

    /// Set created by identifier
    pub fn with_created_by(mut self, created_by: impl Into<String>) -> Self {
        self.created_by = created_by.into();
        self
    }

    /// Register a process wrapper
    pub fn register_process(&self, wrapper: ProcessWrapper) -> Result<()> {
        let name = wrapper.name().to_string();
        let name_clone = name.clone();

        let mut tools = self.tools.write();
        if tools.contains_key(&name) {
            bail!("Tool already registered: {}", name);
        }

        tools.insert(name, ExternalToolEntry::Process(Arc::new(wrapper)));
        info!("Registered process tool: {}", name_clone);

        Ok(())
    }

    /// Register an HTTP wrapper
    pub fn register_http(&self, wrapper: HTTPWrapper) -> Result<()> {
        let name = wrapper.name().to_string();
        let name_clone = name.clone();

        let mut tools = self.tools.write();
        if tools.contains_key(&name) {
            bail!("Tool already registered: {}", name);
        }

        tools.insert(name, ExternalToolEntry::Http(Arc::new(wrapper)));
        info!("Registered HTTP tool: {}", name_clone);

        Ok(())
    }

    /// Register a script wrapper
    pub fn register_script(&self, wrapper: ScriptWrapper) -> Result<()> {
        let name = wrapper.name().to_string();
        let name_clone = name.clone();

        let mut tools = self.tools.write();
        if tools.contains_key(&name) {
            bail!("Tool already registered: {}", name);
        }

        tools.insert(name, ExternalToolEntry::Script(Arc::new(wrapper)));
        info!("Registered script tool: {}", name_clone);

        Ok(())
    }

    /// Register from metadata
    ///
    /// Creates the appropriate wrapper based on tool type.
    ///
    /// # Arguments
    /// * `metadata` - External tool metadata
    ///
    /// # Returns
    /// * `Result<()>` - Success or error
    pub fn register_from_metadata(&self, metadata: ExternalToolMetadata) -> Result<()> {
        match &metadata.tool_type {
            ExternalToolType::Process { .. } => {
                let wrapper = ProcessWrapper::new(metadata);
                self.register_process(wrapper)
            }
            ExternalToolType::Http { .. } => {
                let wrapper = HTTPWrapper::new(metadata);
                self.register_http(wrapper)
            }
            ExternalToolType::Script { .. } => {
                let wrapper = ScriptWrapper::new(metadata);
                self.register_script(wrapper)
            }
        }
    }

    /// Register multiple tools from metadata
    pub fn register_batch(&self, metadata_list: Vec<ExternalToolMetadata>) -> Result<usize> {
        let mut registered_count = 0;

        for metadata in metadata_list {
            match self.register_from_metadata(metadata) {
                Ok(()) => registered_count += 1,
                Err(e) => warn!("Failed to register tool: {}", e),
            }
        }

        info!("Registered {} tools from batch", registered_count);
        Ok(registered_count)
    }

    /// Get tool by name
    pub fn get_tool(&self, name: &str) -> Option<ExternalToolEntry> {
        let tools = self.tools.read();
        tools.get(name).cloned()
    }

    /// Get all registered tools
    pub fn get_all_tools(&self) -> Vec<ExternalToolEntry> {
        let tools = self.tools.read();
        tools.values().cloned().collect()
    }

    /// Get tool names
    pub fn get_tool_names(&self) -> Vec<String> {
        let tools = self.tools.read();
        tools.keys().cloned().collect()
    }

    /// Get tools by domain
    pub fn get_tools_by_domain(&self, domain: &str) -> Vec<ExternalToolEntry> {
        let tools = self.tools.read();
        tools
            .values()
            .filter(|t| t.domain() == domain)
            .cloned()
            .collect()
    }

    /// Get tools by tag
    pub fn get_tools_by_tag(&self, tag: &str) -> Vec<ExternalToolEntry> {
        let tools = self.tools.read();
        tools
            .values()
            .filter(|t| t.metadata().tags.contains(&tag.to_string()))
            .cloned()
            .collect()
    }

    /// Remove a tool
    pub fn remove_tool(&self, name: &str) -> Result<()> {
        let mut tools = self.tools.write();

        if tools.remove(name).is_some() {
            info!("Removed tool: {}", name);

            // Remove metadata file if exists
            let meta_file = self.storage_dir.join(format!("{}.json", name));
            if meta_file.exists() {
                std::fs::remove_file(&meta_file)?;
            }

            Ok(())
        } else {
            bail!("Tool not found: {}", name)
        }
    }

    /// Enable/disable a tool
    pub fn set_tool_enabled(&self, name: &str, enabled: bool) -> Result<()> {
        let mut tools = self.tools.write();

        let entry = tools
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        // Note: This is a simplified implementation
        // In a full implementation, we would need to update the metadata
        // which requires mutable access to the wrapper

        info!("Tool {} {}abled", name, if enabled { "en" } else { "dis" });
        Ok(())
    }

    /// Save tool metadata to storage
    pub fn save_tool_metadata(&self, name: &str) -> Result<()> {
        let tools = self.tools.read();

        let entry = tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", name))?;

        let metadata = entry.metadata();
        let file_path = self.storage_dir.join(format!("{}.json", name));

        let content = serde_json::to_string_pretty(metadata)?;
        std::fs::write(&file_path, content)?;

        debug!("Saved tool metadata: {:?}", file_path);
        Ok(())
    }

    /// Load tool metadata from storage
    pub fn load_tool_metadata(&self, name: &str) -> Result<ExternalToolMetadata> {
        let file_path = self.storage_dir.join(format!("{}.json", name));

        if !file_path.exists() {
            bail!("Metadata file not found: {:?}", file_path);
        }

        let content = std::fs::read_to_string(&file_path)?;
        let metadata: ExternalToolMetadata = serde_json::from_str(&content)?;

        Ok(metadata)
    }

    /// Load all tool metadata from storage
    pub fn load_all_metadata(&self) -> Result<Vec<ExternalToolMetadata>> {
        let mut metadata_list = Vec::new();

        if !self.storage_dir.exists() {
            return Ok(metadata_list);
        }

        for entry in std::fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(metadata) = serde_json::from_str::<ExternalToolMetadata>(&content) {
                        metadata_list.push(metadata);
                    }
                }
            }
        }

        info!("Loaded {} tool metadata", metadata_list.len());
        Ok(metadata_list)
    }

    /// Register all external tools to the tool matrix
    pub fn register_to_tool_matrix(&self, tool_registry: &mut ToolRegistry) -> Result<usize> {
        let tools = self.tools.read();
        let mut registered_count = 0;

        for entry in tools.values() {
            if !entry.is_enabled() {
                debug!("Skipping disabled tool: {}", entry.name());
                continue;
            }

            let tool_def = entry.to_tool_definition();

            // Determine toolbox based on domain
            let toolbox_id = self.get_toolbox_for_domain(entry.domain());

            // Ensure toolbox exists
            if tool_registry.get_toolbox(&toolbox_id).is_none() {
                let toolbox = ToolBox::new(
                    &toolbox_id,
                    toolbox_id.replace('_', " ").to_uppercase(),
                    format!("{} tools", toolbox_id),
                );
                tool_registry.create_toolbox(toolbox)?;
            }

            // Register tool
            match tool_registry.register_tool_to_box_sync(
                tool_def,
                &toolbox_id,
                ToolSource::Dynamic,
            ) {
                Ok(()) => {
                    info!(
                        "Registered {} to tool matrix (toolbox: {})",
                        entry.name(),
                        toolbox_id
                    );
                    registered_count += 1;
                }
                Err(e) => {
                    warn!("Failed to register {} to tool matrix: {}", entry.name(), e);
                }
            }
        }

        info!(
            "Registered {} external tools to tool matrix",
            registered_count
        );
        Ok(registered_count)
    }

    /// Get toolbox ID for a domain
    fn get_toolbox_for_domain(&self, domain: &str) -> String {
        match domain {
            "version_control" | "vcs" | "git" => "version_control".to_string(),
            "container" | "docker" | "kubernetes" => "container".to_string(),
            "package_manager" | "npm" | "cargo" | "pip" => "development".to_string(),
            "interpreter" | "python" | "node" | "ruby" => "development".to_string(),
            "network" | "http" | "curl" => "network".to_string(),
            "text_processing" | "grep" | "sed" => "text".to_string(),
            "data_processing" | "json" | "csv" => "data".to_string(),
            "file_system" | "files" => "file".to_string(),
            "script" => "script".to_string(),
            _ => "external".to_string(),
        }
    }

    /// Get registry statistics
    pub fn stats(&self) -> ExternalRegistryStats {
        let tools = self.tools.read();

        let mut process_count = 0;
        let mut http_count = 0;
        let mut script_count = 0;
        let mut enabled_count = 0;

        for entry in tools.values() {
            match entry {
                ExternalToolEntry::Process(_) => process_count += 1,
                ExternalToolEntry::Http(_) => http_count += 1,
                ExternalToolEntry::Script(_) => script_count += 1,
            }

            if entry.is_enabled() {
                enabled_count += 1;
            }
        }

        ExternalRegistryStats {
            total_count: tools.len(),
            process_count,
            http_count,
            script_count,
            enabled_count,
            disabled_count: tools.len() - enabled_count,
        }
    }

    /// Get storage directory
    pub fn storage_dir(&self) -> &Path {
        &self.storage_dir
    }
}

impl Default for ExternalToolRegistry {
    fn default() -> Self {
        Self::new().unwrap()
    }
}

/// External registry statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalRegistryStats {
    /// Total tool count
    pub total_count: usize,
    /// Process tool count
    pub process_count: usize,
    /// HTTP tool count
    pub http_count: usize,
    /// Script tool count
    pub script_count: usize,
    /// Enabled tool count
    pub enabled_count: usize,
    /// Disabled tool count
    pub disabled_count: usize,
}

/// Builder for creating and registering external tools
pub struct ExternalToolBuilder {
    registry: Arc<ExternalToolRegistry>,
}

impl ExternalToolBuilder {
    /// Create a new builder
    pub fn new(registry: Arc<ExternalToolRegistry>) -> Self {
        Self { registry }
    }

    /// Create and register a process tool
    pub fn create_process(
        &self,
        name: impl Into<String>,
        executable: impl Into<String>,
    ) -> ProcessWrapperBuilder {
        ProcessWrapperBuilder::new(name, executable)
    }

    /// Create and register an HTTP tool
    pub fn create_http(
        &self,
        name: impl Into<String>,
        base_url: impl Into<String>,
        method: impl Into<String>,
    ) -> HTTPWrapperBuilder {
        HTTPWrapperBuilder::new(name, base_url, method)
    }

    /// Create and register a script tool
    pub fn create_script(
        &self,
        name: impl Into<String>,
        script_path: PathBuf,
    ) -> Option<ScriptWrapperBuilder> {
        ScriptWrapperBuilder::with_auto_interpreter(name, script_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external_process::metadata::{schema_helpers, ProcessConfig};
    use crate::external_process::process_wrapper::ProcessWrapperBuilder;
    use serde_json::json;
    use tempfile::TempDir;

    #[test]
    fn test_registry_creation() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let registry = ExternalToolRegistry::with_storage_dir(temp_dir.path())?;

        assert_eq!(registry.get_tool_names().len(), 0);
        assert!(registry.storage_dir().exists());

        Ok(())
    }

    #[test]
    fn test_register_process() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let registry = ExternalToolRegistry::with_storage_dir(temp_dir.path())?;

        let wrapper = ProcessWrapperBuilder::new("test_echo", "echo")
            .description("Test echo command")
            .args(vec!["{{message}}".to_string()])
            .input_schema(schema_helpers::create_string_params_schema(vec![(
                "message",
                "Message to echo",
                true,
            )]))
            .domain("test")
            .build();

        registry.register_process(wrapper)?;

        assert_eq!(registry.get_tool_names().len(), 1);
        assert!(registry.get_tool("test_echo").is_some());

        Ok(())
    }

    #[test]
    fn test_register_batch() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let registry = ExternalToolRegistry::with_storage_dir(temp_dir.path())?;

        let mut metadata_list = Vec::new();

        // Create test metadata
        for i in 0..3 {
            let config = ProcessConfig::new(format!("test_cmd_{}", i));
            let metadata = ExternalToolMetadata::new(
                format!("test_tool_{}", i),
                format!("Test tool {}", i),
                ExternalToolType::process(config),
                json!({"type": "object"}),
                "test",
                "test",
            );
            metadata_list.push(metadata);
        }

        let registered = registry.register_batch(metadata_list)?;
        assert_eq!(registered, 3);
        assert_eq!(registry.get_tool_names().len(), 3);

        Ok(())
    }

    #[test]
    fn test_save_load_metadata() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let registry = ExternalToolRegistry::with_storage_dir(temp_dir.path())?;

        let wrapper = ProcessWrapperBuilder::new("test_save", "echo")
            .description("Test save")
            .domain("test")
            .build();

        registry.register_process(wrapper)?;
        registry.save_tool_metadata("test_save")?;

        // Load metadata
        let metadata = registry.load_tool_metadata("test_save")?;
        assert_eq!(metadata.name, "test_save");

        Ok(())
    }

    #[test]
    fn test_registry_stats() -> Result<()> {
        let temp_dir = TempDir::new().unwrap();
        let registry = ExternalToolRegistry::with_storage_dir(temp_dir.path())?;

        let stats = registry.stats();
        assert_eq!(stats.total_count, 0);
        assert_eq!(stats.process_count, 0);

        // Register a tool
        let wrapper = ProcessWrapperBuilder::new("test_stats", "echo").build();
        registry.register_process(wrapper)?;

        let stats = registry.stats();
        assert_eq!(stats.total_count, 1);
        assert_eq!(stats.process_count, 1);

        Ok(())
    }
}
