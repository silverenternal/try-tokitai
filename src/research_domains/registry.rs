use super::adapters as domain_adapters;
use super::model::{
    DomainAsset, DomainContextSnapshot, DomainInference, DomainIntentDescriptor,
    DomainLifecycleDescriptor, DomainMetadata, DomainPluginDescriptor, DomainProviderDescriptor,
    DomainVisualizationDescriptor,
    DomainWorkbenchDescriptor, DomainWorkbenchStageDescriptor, DomainWorkbenchToolDescriptor,
    DomainWorkspace, DomainWorkspaceSummary, ResearchDomainCatalog,
    VisualizationRendererDescriptor, RESEARCH_DOMAIN_PLUGIN_API_VERSION,
    RESEARCH_DOMAIN_SCHEMA_VERSION,
};
use super::providers::{
    DomainProviderContext, IAgentContextProvider, IDataProvider, IDomainPlugin, IExecutionProvider,
    IPreviewProvider, IRenderProvider, IVisualizationProvider,
};
use super::state::{read_workspace_state, update_workspace_state};
use crate::visualization::model::{
    VisualizationDiagnostic, VisualizationDocument, VisualizationEdge, VisualizationFrame,
    VisualizationNode, VisualizationPoint, VisualizationSeries, VisualizationSource,
};
use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use walkdir::{DirEntry, WalkDir};

const MAX_DISCOVERED_ASSETS: usize = 240;
const MAX_WALKED_FILES: usize = 12_000;
const MAX_STRUCTURED_NODES: usize = 500;
const MAX_TEXT_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
struct WorkspacePluginManifest {
    plugin: DomainPluginDescriptor,
    #[serde(default)]
    match_keywords: Vec<String>,
    #[serde(default)]
    content_markers: Vec<String>,
}

#[derive(Debug, Clone)]
struct DeclarativeDomainPlugin {
    descriptor: DomainPluginDescriptor,
    match_keywords: Vec<String>,
    content_markers: Vec<String>,
}

pub struct ResearchDomainRegistry {
    plugins: RwLock<Vec<Arc<dyn IDomainPlugin>>>,
}

impl Default for ResearchDomainRegistry {
    fn default() -> Self {
        Self {
            plugins: RwLock::new(
                builtin_plugins()
                    .into_iter()
                    .map(|plugin| Arc::new(plugin) as Arc<dyn IDomainPlugin>)
                    .collect(),
            ),
        }
    }
}

impl ResearchDomainRegistry {
    pub fn register<P>(&self, plugin: P) -> Result<()>
    where
        P: IDomainPlugin + 'static,
    {
        let id = plugin.descriptor().metadata.id.clone();
        let mut plugins = self
            .plugins
            .write()
            .map_err(|_| anyhow!("research domain registry lock poisoned"))?;
        if plugins
            .iter()
            .any(|candidate| candidate.descriptor().metadata.id == id)
        {
            return Err(anyhow!(
                "research domain plugin '{id}' is already registered"
            ));
        }
        plugins.push(Arc::new(plugin));
        Ok(())
    }

    pub fn catalog(
        &self,
        context: &DomainProviderContext<'_>,
        query: Option<&str>,
    ) -> Result<ResearchDomainCatalog> {
        let plugins = self.plugins_for_workspace(context.workspace_root)?;
        let mut summaries = Vec::with_capacity(plugins.len());
        let mut discovered = HashMap::<String, Vec<DomainAsset>>::new();
        for plugin in &plugins {
            let assets = plugin.discover_assets(context)?;
            summaries.push(workspace_summary(plugin.descriptor(), &assets));
            discovered.insert(plugin.descriptor().metadata.id.clone(), assets);
        }
        let active_domain = infer_from_assets(&plugins, &discovered, query);
        Ok(ResearchDomainCatalog {
            schema_version: RESEARCH_DOMAIN_SCHEMA_VERSION.to_string(),
            generated_at: Utc::now().to_rfc3339(),
            plugin_api_version: RESEARCH_DOMAIN_PLUGIN_API_VERSION.to_string(),
            plugins: plugins
                .iter()
                .map(|plugin| plugin.descriptor().clone())
                .collect(),
            renderers: shared_renderers(),
            workspaces: summaries,
            active_domain,
        })
    }

    pub fn descriptor_catalog(
        &self,
        context: &DomainProviderContext<'_>,
        query: Option<&str>,
    ) -> Result<ResearchDomainCatalog> {
        let plugins = self.plugins_for_workspace(context.workspace_root)?;
        let active_domain = infer_from_query(&plugins, query);
        Ok(ResearchDomainCatalog {
            schema_version: RESEARCH_DOMAIN_SCHEMA_VERSION.to_string(),
            generated_at: Utc::now().to_rfc3339(),
            plugin_api_version: RESEARCH_DOMAIN_PLUGIN_API_VERSION.to_string(),
            plugins: plugins
                .iter()
                .map(|plugin| plugin.descriptor().clone())
                .collect(),
            renderers: shared_renderers(),
            workspaces: plugins
                .iter()
                .map(|plugin| workspace_summary(plugin.descriptor(), &[]))
                .collect(),
            active_domain,
        })
    }

    pub fn workspace(
        &self,
        context: &DomainProviderContext<'_>,
        domain_id: &str,
    ) -> Result<DomainWorkspace> {
        let plugin = self.plugin(context.workspace_root, domain_id)?;
        plugin.on_activate(context)?;
        let mut assets = plugin.discover_assets(context)?;
        assets.sort_by(|left, right| right.modified_at.cmp(&left.modified_at));
        let asset_revision = workspace_revision(&assets);
        let workspace_state = read_workspace_state(context.workspace_root, domain_id)?;
        let state_revision = workspace_state
            .get("revision")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let revision = blake3::hash(format!("{asset_revision}:{state_revision}").as_bytes())
            .to_hex()
            .to_string();
        let workspace = DomainWorkspace {
            schema_version: RESEARCH_DOMAIN_SCHEMA_VERSION.to_string(),
            generated_at: Utc::now().to_rfc3339(),
            domain: plugin.descriptor().clone(),
            assets,
            revision,
            execution: plugin.execution_context(context)?,
            state: workspace_state,
        };
        plugin.on_workspace_change(context, &workspace)?;
        Ok(workspace)
    }

    pub fn context_snapshot(
        &self,
        context: &DomainProviderContext<'_>,
        domain_id: Option<&str>,
    ) -> Result<DomainContextSnapshot> {
        let plugins = self.plugins_for_workspace(context.workspace_root)?;
        let (plugin, inference, assets) = if let Some(domain_id) = domain_id {
            let plugin = plugins
                .iter()
                .find(|plugin| plugin.descriptor().metadata.id == domain_id)
                .cloned()
                .ok_or_else(|| anyhow!("unknown research domain: {domain_id}"))?;
            let assets = plugin.discover_assets(context)?;
            (
                plugin,
                DomainInference {
                    domain_id: domain_id.to_string(),
                    confidence: 1.0,
                    reasons: vec!["explicit domain selection".to_string()],
                },
                assets,
            )
        } else {
            if let Some(inference) = infer_from_query(&plugins, context.query) {
                let plugin = plugins
                    .iter()
                    .find(|plugin| plugin.descriptor().metadata.id == inference.domain_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("inferred domain plugin is unavailable"))?;
                let assets = plugin.discover_assets(context)?;
                let agent_context = plugin.agent_context(context, &inference, &assets)?;
                return Ok(DomainContextSnapshot {
                    schema_version: RESEARCH_DOMAIN_SCHEMA_VERSION.to_string(),
                    generated_at: Utc::now().to_rfc3339(),
                    inference,
                    plugin: plugin.descriptor().clone(),
                    assets,
                    agent_context,
                });
            }
            let mut discovered = HashMap::new();
            for plugin in &plugins {
                discovered.insert(
                    plugin.descriptor().metadata.id.clone(),
                    plugin.discover_assets(context)?,
                );
            }
            let inference =
                infer_from_assets(&plugins, &discovered, context.query).ok_or_else(|| {
                    anyhow!("no research domain matched the current task or workspace")
                })?;
            let plugin = plugins
                .iter()
                .find(|plugin| plugin.descriptor().metadata.id == inference.domain_id)
                .cloned()
                .ok_or_else(|| anyhow!("inferred domain plugin is unavailable"))?;
            let assets = discovered.remove(&inference.domain_id).unwrap_or_default();
            (plugin, inference, assets)
        };
        let agent_context = plugin.agent_context(context, &inference, &assets)?;
        Ok(DomainContextSnapshot {
            schema_version: RESEARCH_DOMAIN_SCHEMA_VERSION.to_string(),
            generated_at: Utc::now().to_rfc3339(),
            inference,
            plugin: plugin.descriptor().clone(),
            assets,
            agent_context,
        })
    }

    pub fn visualization(
        &self,
        context: &DomainProviderContext<'_>,
        domain_id: &str,
        asset_id: &str,
        visualization_id: Option<&str>,
    ) -> Result<VisualizationDocument> {
        let plugin = self.plugin(context.workspace_root, domain_id)?;
        let assets = plugin.discover_assets(context)?;
        let asset = assets
            .iter()
            .find(|asset| {
                asset.id == asset_id || asset.source_id == asset_id || asset.path == asset_id
            })
            .ok_or_else(|| anyhow!("domain asset is no longer available: {asset_id}"))?;
        plugin.visualization_document(context, asset, visualization_id)
    }

    pub fn execution_context(
        &self,
        context: &DomainProviderContext<'_>,
        domain_id: &str,
    ) -> Result<Value> {
        self.plugin(context.workspace_root, domain_id)?
            .execution_context(context)
    }

    pub fn workspace_state(
        &self,
        context: &DomainProviderContext<'_>,
        domain_id: &str,
    ) -> Result<Value> {
        self.plugin(context.workspace_root, domain_id)?;
        read_workspace_state(context.workspace_root, domain_id)
    }

    pub fn update_workspace_state(
        &self,
        context: &DomainProviderContext<'_>,
        domain_id: &str,
        patch: &Value,
        updated_by: &str,
    ) -> Result<Value> {
        self.plugin(context.workspace_root, domain_id)?;
        update_workspace_state(context.workspace_root, domain_id, patch, updated_by)
    }

    fn plugin(&self, workspace_root: &Path, id: &str) -> Result<Arc<dyn IDomainPlugin>> {
        self.plugins_for_workspace(workspace_root)?
            .into_iter()
            .find(|plugin| plugin.descriptor().metadata.id == id)
            .ok_or_else(|| anyhow!("unknown research domain: {id}"))
    }

    fn plugins_for_workspace(&self, workspace_root: &Path) -> Result<Vec<Arc<dyn IDomainPlugin>>> {
        let mut plugins = self
            .plugins
            .read()
            .map_err(|_| anyhow!("research domain registry lock poisoned"))?
            .clone();
        let mut known = plugins
            .iter()
            .map(|plugin| plugin.descriptor().metadata.id.clone())
            .collect::<HashSet<_>>();
        for plugin in workspace_manifest_plugins(workspace_root)? {
            let id = plugin.descriptor.metadata.id.clone();
            if known.insert(id) {
                plugins.push(Arc::new(plugin));
            }
        }
        Ok(plugins)
    }
}

fn infer_from_query(
    plugins: &[Arc<dyn IDomainPlugin>],
    query: Option<&str>,
) -> Option<DomainInference> {
    let query = query.unwrap_or_default().trim().to_ascii_lowercase();
    if query.is_empty() {
        return None;
    }
    let mut ranked = plugins
        .iter()
        .filter_map(|plugin| {
            let descriptor = plugin.descriptor();
            let terms = descriptor
                .capabilities
                .iter()
                .chain(
                    descriptor
                        .supported_file_types
                        .iter()
                        .filter(|value| !is_ambiguous_file_type(&value.to_ascii_lowercase())),
                )
                .chain(descriptor.sdk_adapters.iter())
                .map(|value| value.to_ascii_lowercase())
                .chain(std::iter::once(
                    descriptor.metadata.label.to_ascii_lowercase(),
                ))
                .chain(std::iter::once(descriptor.metadata.id.clone()))
                .filter(|term| term.len() >= 3 && query.contains(term))
                .collect::<Vec<_>>();
            if terms.is_empty() {
                return None;
            }
            let score = terms.len() as f64;
            Some((
                score,
                DomainInference {
                    domain_id: descriptor.metadata.id.clone(),
                    confidence: (0.58 + score * 0.1).min(0.98),
                    reasons: vec![format!(
                        "task matched domain terms: {}",
                        terms.into_iter().take(5).collect::<Vec<_>>().join(", ")
                    )],
                },
            ))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.into_iter().next().map(|(_, inference)| inference)
}

impl IDataProvider for DeclarativeDomainPlugin {
    fn discover_assets(&self, context: &DomainProviderContext<'_>) -> Result<Vec<DomainAsset>> {
        discover_assets(self, context.workspace_root)
    }
}

impl IVisualizationProvider for DeclarativeDomainPlugin {
    fn visualization_document(
        &self,
        context: &DomainProviderContext<'_>,
        asset: &DomainAsset,
        visualization_id: Option<&str>,
    ) -> Result<VisualizationDocument> {
        let visualization = visualization_id
            .and_then(|id| asset.visualizations.iter().find(|item| item.id == id))
            .or_else(|| asset.visualizations.first())
            .ok_or_else(|| anyhow!("asset has no registered visualization provider"))?;
        build_asset_document(
            context.workspace_root,
            &self.descriptor,
            asset,
            visualization,
        )
    }
}

impl IAgentContextProvider for DeclarativeDomainPlugin {
    fn agent_context(
        &self,
        context: &DomainProviderContext<'_>,
        inference: &DomainInference,
        assets: &[DomainAsset],
    ) -> Result<String> {
        let workspace_state =
            read_workspace_state(context.workspace_root, &self.descriptor.metadata.id)?;
        let paths = assets
            .iter()
            .take(24)
            .map(|asset| {
                format!(
                    "- {} [{}; revision={}]",
                    asset.path, asset.file_type, asset.content_revision
                )
            })
            .collect::<Vec<_>>();
        Ok(format!(
            "Research Domain context (schema={}):\n- Active domain: {} ({})\n- Confidence: {:.2}\n- Domain APIs: research_domain_context, research_domain_workspace, research_domain_visualization, research_domain_execution_context, research_domain_workspace_state, research_domain_task, research_domain_action\n- Data provider: {}\n- Visualization provider: {}\n- Execution provider: {}\n- Supported SDK adapters: {}\n- Live workspace UI/state: {}\n- Real workspace assets:\n{}\nThe live workspace state is shared with the user interface. Read it before acting and update it when you change the active tab, focus object, filters, parameters, notes, or generated artifact. When active_task is present, read it through research_domain_task and update its real stage/status as work progresses. List registered native actions before execution; never substitute an Agent prompt for a native SDK action. Use only current workspace artifacts and runtime/tool evidence. Mark a domain task completed only after its real artifact paths and verification evidence have been recorded. When a completed output has a supported visual representation, register or refresh its visualization instead of inventing example data.",
            RESEARCH_DOMAIN_SCHEMA_VERSION,
            self.descriptor.metadata.label,
            self.descriptor.metadata.id,
            inference.confidence,
            self.descriptor.data_provider.id,
            self.descriptor.visualization_provider.id,
            self.descriptor.execution_provider.id,
            if self.descriptor.sdk_adapters.is_empty() {
                "none declared".to_string()
            } else {
                self.descriptor.sdk_adapters.join(", ")
            },
            serde_json::to_string(&workspace_state)?,
            if paths.is_empty() {
                "- No matching artifact is currently present.".to_string()
            } else {
                paths.join("\n")
            },
        ))
    }
}

impl IPreviewProvider for DeclarativeDomainPlugin {
    fn preview_metadata(
        &self,
        document: &VisualizationDocument,
    ) -> Result<serde_json::Map<String, Value>> {
        Ok(serde_json::Map::from_iter([
            ("domain_id".to_string(), json!(self.descriptor.metadata.id)),
            ("nodes".to_string(), json!(document.nodes.len())),
            ("edges".to_string(), json!(document.edges.len())),
            ("series".to_string(), json!(document.series.len())),
            ("frames".to_string(), json!(document.frames.len())),
            ("generated_at".to_string(), json!(document.generated_at)),
        ]))
    }
}

impl IRenderProvider for DeclarativeDomainPlugin {
    fn renderers(&self) -> Vec<DomainVisualizationDescriptor> {
        self.descriptor.supported_visualizations.clone()
    }
}

impl IExecutionProvider for DeclarativeDomainPlugin {
    fn execution_context(&self, context: &DomainProviderContext<'_>) -> Result<Value> {
        Ok(json!({
            "domain_id": self.descriptor.metadata.id,
            "provider_id": self.descriptor.execution_provider.id,
            "workspace_root": context.workspace_root,
            "sdk_adapters": self.descriptor.sdk_adapters,
            "capabilities": self.descriptor.capabilities,
            "workbench": self.descriptor.workbench,
            "adapter_status": domain_adapters::sdk_statuses(&self.descriptor.sdk_adapters),
            "execution_policy": "Use Atlas native tools and the configured workspace toolchain; never synthesize execution results."
        }))
    }
}

impl IDomainPlugin for DeclarativeDomainPlugin {
    fn descriptor(&self) -> &DomainPluginDescriptor {
        &self.descriptor
    }
}

fn workspace_manifest_plugins(root: &Path) -> Result<Vec<DeclarativeDomainPlugin>> {
    let directory = root.join(".atlas").join("domains");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut plugins = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        match fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<WorkspacePluginManifest>(&raw).ok())
        {
            Some(manifest) => plugins.push(DeclarativeDomainPlugin {
                descriptor: normalize_descriptor(manifest.plugin),
                match_keywords: normalized_list(manifest.match_keywords),
                content_markers: normalized_list(manifest.content_markers),
            }),
            None => {
                tracing::warn!(path = %path.display(), "invalid research domain plugin manifest")
            }
        }
    }
    Ok(plugins)
}

fn discover_assets(plugin: &DeclarativeDomainPlugin, root: &Path) -> Result<Vec<DomainAsset>> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let supported = normalized_file_types(&plugin.descriptor.supported_file_types);
    let mut assets = Vec::new();
    let mut walked_files = 0usize;
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(9)
        .into_iter()
        .filter_entry(include_walk_entry)
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        walked_files += 1;
        if walked_files > MAX_WALKED_FILES || assets.len() >= MAX_DISCOVERED_ASSETS {
            break;
        }
        let path = entry.path();
        let relative = path.strip_prefix(root).unwrap_or(path);
        let relative_display = relative.to_string_lossy().replace('\\', "/");
        if relative_display.starts_with(".atlas/")
            && !relative_display.starts_with(".atlas/domain-actions/")
        {
            continue;
        }
        let action_domain = domain_action_path_domain(&relative_display);
        if relative_display.starts_with(".atlas/domain-actions/") {
            if action_domain != Some(plugin.descriptor.metadata.id.as_str())
                || relative_display.ends_with("/run.json")
            {
                continue;
            }
        }
        let extension = file_type(path);
        let is_action_result = action_domain.is_some() && relative_display.ends_with("/result.json");
        let is_render_output = action_domain.is_some()
            && matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "exr");
        if !is_action_result
            && !is_render_output
            && !asset_matches(
                path,
                &relative_display,
                &extension,
                &supported,
                &plugin.match_keywords,
                &plugin.content_markers,
            )
        {
            continue;
        }
        let metadata = match fs::metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        let modified = metadata
            .modified()
            .ok()
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(Utc::now);
        let revision = blake3::hash(
            format!(
                "{}:{}:{}",
                relative_display,
                metadata.len(),
                modified.timestamp_millis()
            )
            .as_bytes(),
        )
        .to_hex()[..16]
            .to_string();
        let raw_preview = if metadata.len() <= 2 * 1024 * 1024 && is_text_file_type(&extension) {
            fs::read_to_string(path).ok()
        } else {
            None
        };
        let visualizations = if is_action_result {
            vec![visualization("action-result", "SDK Action Result", "table", &["json"])]
        } else if is_render_output {
            vec![visualization(
                "render-output",
                "Rendered Output",
                "2d",
                &["png", "jpg", "jpeg", "webp", "exr"],
            )]
        } else {
            plugin
                .descriptor
                .supported_visualizations
                .iter()
                .filter(|visualization| {
                    (visualization.compatible_file_types.is_empty()
                        || normalized_file_types(&visualization.compatible_file_types)
                            .contains(&extension))
                        && domain_adapters::supports_visualization(
                            &plugin.descriptor.metadata.id,
                            visualization,
                            path,
                            &extension,
                            raw_preview.as_deref(),
                        )
                })
                .cloned()
                .collect::<Vec<_>>()
        };
        if visualizations.is_empty() {
            continue;
        }
        let id = format!(
            "domain:{}:{}",
            plugin.descriptor.metadata.id,
            blake3::hash(relative_display.as_bytes()).to_hex()[..20].to_string()
        );
        let mut asset_metadata = BTreeMap::new();
        asset_metadata.insert("absolute_path".to_string(), json!(path));
        asset_metadata.insert("workspace_relative".to_string(), json!(relative_display));
        asset_metadata.insert(
            "provider_id".to_string(),
            json!(plugin.descriptor.data_provider.id),
        );
        if action_domain.is_some() {
            asset_metadata.insert("generated_by_domain_action".to_string(), json!(true));
            asset_metadata.insert("action_result".to_string(), json!(is_action_result));
        }
        assets.push(DomainAsset {
            id,
            source_id: format!("workspace:{relative_display}"),
            domain_id: plugin.descriptor.metadata.id.clone(),
            path: relative_display,
            name: path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("artifact")
                .to_string(),
            file_type: extension,
            size_bytes: metadata.len(),
            modified_at: modified.to_rfc3339(),
            content_revision: revision,
            visualizations,
            metadata: asset_metadata,
        });
    }
    assets.sort_by(|left, right| right.modified_at.cmp(&left.modified_at));
    Ok(assets)
}

fn include_walk_entry(entry: &DirEntry) -> bool {
    if entry.depth() == 0 || !entry.file_type().is_dir() {
        return true;
    }
    !matches!(
        entry
            .file_name()
            .to_string_lossy()
            .to_ascii_lowercase()
            .as_str(),
        ".git"
            | ".svn"
            | ".hg"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | "vendor"
            | ".cache"
            | ".venv"
            | "venv"
            | "__pycache__"
            | ".tokitai"
    )
}

fn domain_action_path_domain(relative: &str) -> Option<&str> {
    let mut parts = relative.split('/');
    if parts.next()? != ".atlas" || parts.next()? != "domain-actions" {
        return None;
    }
    parts.next().filter(|value| !value.is_empty())
}

fn asset_matches(
    path: &Path,
    relative: &str,
    extension: &str,
    supported: &HashSet<String>,
    keywords: &[String],
    content_markers: &[String],
) -> bool {
    let lower_path = relative.to_ascii_lowercase();
    let direct_type = supported.contains(extension)
        || path
            .file_name()
            .and_then(|value| value.to_str())
            .map(|name| supported.contains(&name.to_ascii_lowercase()))
            .unwrap_or(false);
    let keyword_match = keywords.iter().any(|keyword| lower_path.contains(keyword));
    if keyword_match {
        return true;
    }
    if direct_type && !is_ambiguous_file_type(extension) {
        return true;
    }
    if !(direct_type || is_text_file_type(extension)) || content_markers.is_empty() {
        return false;
    }
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() > 2 * 1024 * 1024 {
        return direct_type && !is_ambiguous_file_type(extension);
    }
    let Ok(raw) = fs::read_to_string(path) else {
        return false;
    };
    let preview = raw
        .chars()
        .take(64_000)
        .collect::<String>()
        .to_ascii_lowercase();
    content_markers
        .iter()
        .any(|marker| preview.contains(marker))
}

fn workspace_summary(
    descriptor: &DomainPluginDescriptor,
    assets: &[DomainAsset],
) -> DomainWorkspaceSummary {
    DomainWorkspaceSummary {
        domain_id: descriptor.metadata.id.clone(),
        asset_count: assets.len(),
        visualization_count: assets.iter().map(|asset| asset.visualizations.len()).sum(),
        revision: workspace_revision(assets),
        latest_modified_at: assets.first().map(|asset| asset.modified_at.clone()),
    }
}

fn workspace_revision(assets: &[DomainAsset]) -> String {
    let mut input = String::new();
    for asset in assets {
        input.push_str(&asset.id);
        input.push(':');
        input.push_str(&asset.content_revision);
        input.push('|');
    }
    blake3::hash(input.as_bytes()).to_hex()[..20].to_string()
}

fn infer_from_assets(
    plugins: &[Arc<dyn IDomainPlugin>],
    discovered: &HashMap<String, Vec<DomainAsset>>,
    query: Option<&str>,
) -> Option<DomainInference> {
    let normalized_query = query.unwrap_or_default().to_ascii_lowercase();
    let mut ranked = plugins
        .iter()
        .filter_map(|plugin| {
            let descriptor = plugin.descriptor();
            let id = descriptor.metadata.id.clone();
            let assets = discovered.get(&id).map(Vec::as_slice).unwrap_or_default();
            let terms = descriptor
                .capabilities
                .iter()
                .chain(descriptor.supported_file_types.iter())
                .chain(descriptor.sdk_adapters.iter())
                .map(|value| value.to_ascii_lowercase())
                .chain(std::iter::once(
                    descriptor.metadata.label.to_ascii_lowercase(),
                ))
                .chain(std::iter::once(id.clone()))
                .collect::<Vec<_>>();
            let matched = terms
                .iter()
                .filter(|term| !term.is_empty() && normalized_query.contains(term.as_str()))
                .count();
            let asset_score = assets.len().min(12) as f64 * 0.025;
            let query_score = matched as f64 * 0.24;
            let score = query_score + asset_score;
            if score <= 0.0 {
                return None;
            }
            let mut reasons = Vec::new();
            if matched > 0 {
                reasons.push(format!("{matched} task term(s) matched plugin metadata"));
            }
            if !assets.is_empty() {
                reasons.push(format!(
                    "{} matching real workspace artifact(s)",
                    assets.len()
                ));
            }
            Some((
                score,
                DomainInference {
                    domain_id: id,
                    confidence: score.min(0.99),
                    reasons,
                },
            ))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.into_iter().next().map(|(_, inference)| inference)
}

fn build_asset_document(
    workspace_root: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
    visualization: &DomainVisualizationDescriptor,
) -> Result<VisualizationDocument> {
    let mut document =
        match domain_adapters::parse_registered_adapter(workspace_root, descriptor, asset)? {
            Some(document) => document,
            None => build_generic_asset_document(workspace_root, descriptor, asset, visualization)?,
        };
    domain_adapters::adapt_document(
        workspace_root,
        descriptor,
        asset,
        visualization,
        &mut document,
    )?;
    document.kind = "research-domain".to_string();
    document.id = format!(
        "research-domain:{}:{}:{}",
        descriptor.metadata.id, asset.content_revision, visualization.id
    );
    document.title = format!(
        "{} · {} · {}",
        descriptor.metadata.label, asset.name, visualization.label
    );
    document.source.id = asset.id.clone();
    document.source.kind = "research-domain".to_string();
    document.source.label = asset.path.clone();
    document.source.source_type = asset.file_type.clone();
    document
        .source
        .metadata
        .insert("domain_id".to_string(), json!(descriptor.metadata.id));
    document
        .source
        .metadata
        .insert("path".to_string(), json!(asset.path));
    document.source.metadata.insert(
        "content_revision".to_string(),
        json!(asset.content_revision),
    );
    document
        .metadata
        .insert("domain_id".to_string(), json!(descriptor.metadata.id));
    document
        .metadata
        .insert("domain_label".to_string(), json!(descriptor.metadata.label));
    document
        .metadata
        .insert("visualization_id".to_string(), json!(visualization.id));
    document
        .metadata
        .insert("renderer".to_string(), json!(visualization.renderer));
    document
        .metadata
        .insert("asset_path".to_string(), json!(asset.path));
    document
        .metadata
        .insert("asset_revision".to_string(), json!(asset.content_revision));
    Ok(document)
}

fn build_generic_asset_document(
    workspace_root: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
    visualization: &DomainVisualizationDescriptor,
) -> Result<VisualizationDocument> {
    let root = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let path = root.join(&asset.path);
    let canonical = path
        .canonicalize()
        .map_err(|error| anyhow!("domain asset is unavailable: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err(anyhow!("domain asset is outside the active workspace"));
    }
    let source = VisualizationSource {
        id: asset.id.clone(),
        kind: "research-domain".to_string(),
        label: asset.path.clone(),
        source_type: asset.file_type.clone(),
        live: false,
        metadata: BTreeMap::from([
            ("domain_id".to_string(), json!(descriptor.metadata.id)),
            ("path".to_string(), json!(asset.path)),
            (
                "content_revision".to_string(),
                json!(asset.content_revision),
            ),
        ]),
    };
    let mut document = VisualizationDocument::empty(
        "research-domain",
        format!("{} · {}", descriptor.metadata.label, asset.name),
        source,
    );
    document
        .metadata
        .insert("domain_id".to_string(), json!(descriptor.metadata.id));
    document
        .metadata
        .insert("domain_label".to_string(), json!(descriptor.metadata.label));
    document
        .metadata
        .insert("visualization_id".to_string(), json!(visualization.id));
    document
        .metadata
        .insert("renderer".to_string(), json!(visualization.renderer));
    document
        .metadata
        .insert("asset_path".to_string(), json!(asset.path));
    document
        .metadata
        .insert("asset_revision".to_string(), json!(asset.content_revision));

    let mut root_node = VisualizationNode::new(
        "asset",
        asset.name.clone(),
        format!("{} artifact", descriptor.metadata.label),
    );
    root_node
        .metrics
        .insert("bytes".to_string(), asset.size_bytes as f64);
    root_node
        .metadata
        .insert("path".to_string(), json!(asset.path));
    root_node
        .metadata
        .insert("file_type".to_string(), json!(asset.file_type));
    root_node
        .metadata
        .insert("modified_at".to_string(), json!(asset.modified_at));
    document.nodes.push(root_node);

    if asset.size_bytes <= MAX_TEXT_BYTES {
        if let Ok(raw) = fs::read_to_string(&canonical) {
            match asset.file_type.as_str() {
                "json" | "ipynb" | "gltf" | "geojson" => {
                    if let Ok(value) = serde_json::from_str::<Value>(&raw) {
                        append_json_structure(&value, &mut document);
                    }
                }
                "jsonl" | "ndjson" => append_jsonl_series(&raw, &mut document),
                "csv" | "tsv" => append_delimited_series(
                    &raw,
                    if asset.file_type == "tsv" { '\t' } else { ',' },
                    &mut document,
                ),
                "obj" | "ply" | "stl" => {
                    append_mesh_metadata(&raw, asset.file_type.as_str(), &mut document)
                }
                _ => append_text_structure(&raw, &mut document),
            }
        }
    }
    if document.nodes.len() == 1 && document.series.is_empty() {
        document.diagnostics.push(VisualizationDiagnostic {
            level: "info".to_string(),
            message: "Only real artifact metadata is available for this file; no structural records were parsed."
                .to_string(),
            metadata: BTreeMap::new(),
        });
    }
    Ok(document)
}

fn append_json_structure(value: &Value, document: &mut VisualizationDocument) {
    fn visit(
        value: &Value,
        label: &str,
        parent: &str,
        path: &str,
        document: &mut VisualizationDocument,
        count: &mut usize,
    ) {
        if *count >= MAX_STRUCTURED_NODES {
            return;
        }
        match value {
            Value::Object(object) => {
                for (key, child) in object {
                    if *count >= MAX_STRUCTURED_NODES {
                        break;
                    }
                    let child_path = format!("{path}/{key}");
                    let id = stable_domain_id("json", &child_path);
                    let mut node = VisualizationNode::new(
                        id.clone(),
                        key.clone(),
                        match child {
                            Value::Array(_) => "array",
                            Value::Object(_) => "object",
                            _ => "value",
                        },
                    );
                    if !matches!(child, Value::Array(_) | Value::Object(_)) {
                        node.metadata.insert("value".to_string(), child.clone());
                    }
                    document.nodes.push(node);
                    document.edges.push(VisualizationEdge::new(
                        stable_domain_id("json-edge", &child_path),
                        parent,
                        &id,
                        "contains",
                        "structure",
                    ));
                    *count += 1;
                    visit(child, key, &id, &child_path, document, count);
                }
            }
            Value::Array(values) => {
                for (index, child) in values
                    .iter()
                    .take(MAX_STRUCTURED_NODES - *count)
                    .enumerate()
                {
                    let child_path = format!("{path}/{index}");
                    let id = stable_domain_id("json", &child_path);
                    let child_label = child
                        .get("name")
                        .or_else(|| child.get("id"))
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| format!("{label} {index}"));
                    let mut node = VisualizationNode::new(
                        id.clone(),
                        child_label,
                        if child.is_object() { "record" } else { "item" },
                    );
                    if !matches!(child, Value::Array(_) | Value::Object(_)) {
                        node.metadata.insert("value".to_string(), child.clone());
                    }
                    document.nodes.push(node);
                    document.edges.push(VisualizationEdge::new(
                        stable_domain_id("json-edge", &child_path),
                        parent,
                        &id,
                        "contains",
                        "structure",
                    ));
                    *count += 1;
                    visit(child, label, &id, &child_path, document, count);
                    if *count >= MAX_STRUCTURED_NODES {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    let mut count = 0usize;
    visit(value, "item", "asset", "$", document, &mut count);
}

fn append_jsonl_series(raw: &str, document: &mut VisualizationDocument) {
    let values = raw
        .lines()
        .take(2_000)
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>();
    append_numeric_records(&values, document);
}

fn append_delimited_series(raw: &str, delimiter: char, document: &mut VisualizationDocument) {
    let mut lines = raw.lines();
    let headers = lines
        .next()
        .unwrap_or_default()
        .split(delimiter)
        .map(|value| value.trim().trim_matches('"').to_string())
        .collect::<Vec<_>>();
    let records = lines
        .take(2_000)
        .map(|line| {
            let fields = line
                .split(delimiter)
                .map(|value| value.trim().trim_matches('"'))
                .collect::<Vec<_>>();
            let mut object = serde_json::Map::new();
            for (index, header) in headers.iter().enumerate() {
                if let Some(value) = fields.get(index) {
                    object.insert(
                        header.clone(),
                        value
                            .parse::<f64>()
                            .map(Value::from)
                            .unwrap_or_else(|_| Value::String((*value).to_string())),
                    );
                }
            }
            Value::Object(object)
        })
        .collect::<Vec<_>>();
    append_numeric_records(&records, document);
}

fn append_numeric_records(records: &[Value], document: &mut VisualizationDocument) {
    let mut columns = BTreeMap::<String, Vec<VisualizationPoint>>::new();
    for (index, record) in records.iter().enumerate() {
        let Some(object) = record.as_object() else {
            continue;
        };
        for (key, value) in object {
            if let Some(number) = value
                .as_f64()
                .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
            {
                columns
                    .entry(key.clone())
                    .or_default()
                    .push(VisualizationPoint {
                        timestamp_ms: index as i64,
                        value: number,
                    });
            }
        }
    }
    for (index, (name, points)) in columns.into_iter().take(12).enumerate() {
        let node_id = stable_domain_id("column", &name);
        if !document.nodes.iter().any(|node| node.id == node_id) {
            document.nodes.push(VisualizationNode::new(
                node_id.clone(),
                name.clone(),
                "numeric field",
            ));
            document.edges.push(VisualizationEdge::new(
                stable_domain_id("column-edge", &name),
                "asset",
                &node_id,
                "contains",
                "schema",
            ));
        }
        document.series.push(VisualizationSeries {
            id: format!("series:{index}"),
            label: name,
            unit: String::new(),
            node_id: Some(node_id),
            category: "observed-data".to_string(),
            points,
        });
    }
    if !document.series.is_empty() {
        let frame_count = document
            .series
            .iter()
            .map(|series| series.points.len())
            .max()
            .unwrap_or_default();
        document.frames = (0..frame_count)
            .map(|sequence| VisualizationFrame {
                id: format!("observed-record:{sequence}"),
                sequence,
                label: format!("record {}", sequence + 1),
                active_nodes: document
                    .series
                    .iter()
                    .filter(|series| series.points.get(sequence).is_some())
                    .filter_map(|series| series.node_id.clone())
                    .collect(),
                active_edges: Vec::new(),
                metrics: document
                    .series
                    .iter()
                    .filter_map(|series| {
                        series
                            .points
                            .get(sequence)
                            .map(|point| (series.label.clone(), point.value))
                    })
                    .collect(),
            })
            .collect();
    }
}

fn append_text_structure(raw: &str, document: &mut VisualizationDocument) {
    let declaration = Regex::new(
        r"(?m)^\s*(?:pub\s+|export\s+|async\s+)*(?:fn|def|class|struct|enum|interface|function|module|table|CREATE\s+TABLE)\s+([A-Za-z_][A-Za-z0-9_]*)",
    )
    .unwrap();
    let heading = Regex::new(r"(?m)^\s{0,3}#{1,6}\s+(.+?)\s*$").unwrap();
    let explicit_edge =
        Regex::new(r"(?m)([A-Za-z_][A-Za-z0-9_.:-]*)\s*(?:->|=>)\s*([A-Za-z_][A-Za-z0-9_.:-]*)")
            .unwrap();
    let mut ids = HashMap::<String, String>::new();
    for capture in declaration.captures_iter(raw).take(MAX_STRUCTURED_NODES) {
        let Some(name) = capture.get(1).map(|value| value.as_str()) else {
            continue;
        };
        let id = stable_domain_id("symbol", name);
        ids.insert(name.to_string(), id.clone());
        document
            .nodes
            .push(VisualizationNode::new(&id, name, "declared symbol"));
        document.edges.push(VisualizationEdge::new(
            stable_domain_id("symbol-edge", name),
            "asset",
            &id,
            "declares",
            "structure",
        ));
    }
    for capture in heading
        .captures_iter(raw)
        .take(MAX_STRUCTURED_NODES.saturating_sub(document.nodes.len()))
    {
        let Some(name) = capture.get(1).map(|value| value.as_str().trim()) else {
            continue;
        };
        let id = stable_domain_id("section", name);
        document
            .nodes
            .push(VisualizationNode::new(&id, name, "section"));
        document.edges.push(VisualizationEdge::new(
            stable_domain_id("section-edge", name),
            "asset",
            &id,
            "contains",
            "document-structure",
        ));
    }
    for (index, capture) in explicit_edge.captures_iter(raw).take(500).enumerate() {
        let source_name = capture.get(1).unwrap().as_str();
        let target_name = capture.get(2).unwrap().as_str();
        let source = ids
            .entry(source_name.to_string())
            .or_insert_with(|| stable_domain_id("reference", source_name))
            .clone();
        let target = ids
            .entry(target_name.to_string())
            .or_insert_with(|| stable_domain_id("reference", target_name))
            .clone();
        if !document.nodes.iter().any(|node| node.id == source) {
            document
                .nodes
                .push(VisualizationNode::new(&source, source_name, "reference"));
        }
        if !document.nodes.iter().any(|node| node.id == target) {
            document
                .nodes
                .push(VisualizationNode::new(&target, target_name, "reference"));
        }
        document.edges.push(VisualizationEdge::new(
            format!("explicit-edge:{index}"),
            source,
            target,
            "declared flow",
            "explicit-relationship",
        ));
    }
}

fn append_mesh_metadata(raw: &str, file_type: &str, document: &mut VisualizationDocument) {
    let mut points = Vec::<[f64; 3]>::new();
    let mut faces = Vec::<Vec<usize>>::new();
    match file_type {
        "obj" => {
            for line in raw.lines() {
                let values = line.split_whitespace().collect::<Vec<_>>();
                if values.first() == Some(&"v") && values.len() >= 4 && points.len() < 10_000 {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (values[1].parse(), values[2].parse(), values[3].parse())
                    {
                        points.push([x, y, z]);
                    }
                } else if values.first() == Some(&"f") && values.len() >= 4 && faces.len() < 10_000
                {
                    faces.push(
                        values[1..]
                            .iter()
                            .filter_map(|value| value.split('/').next()?.parse::<usize>().ok())
                            .map(|value| value.saturating_sub(1))
                            .collect(),
                    );
                }
            }
        }
        "stl" => {
            for line in raw.lines() {
                let values = line.split_whitespace().collect::<Vec<_>>();
                if values.first() == Some(&"vertex") && values.len() >= 4 && points.len() < 10_000 {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (values[1].parse(), values[2].parse(), values[3].parse())
                    {
                        points.push([x, y, z]);
                    }
                }
            }
            for start in (0..points.len()).step_by(3) {
                if start + 2 < points.len() {
                    faces.push(vec![start, start + 1, start + 2]);
                }
            }
        }
        "ply" => {
            let mut in_header = true;
            for line in raw.lines() {
                if in_header {
                    in_header = line.trim() != "end_header";
                    continue;
                }
                let values = line.split_whitespace().collect::<Vec<_>>();
                if values.len() >= 3 && points.len() < 10_000 {
                    if let (Ok(x), Ok(y), Ok(z)) =
                        (values[0].parse(), values[1].parse(), values[2].parse())
                    {
                        points.push([x, y, z]);
                        continue;
                    }
                }
                if let Some(count) = values.first().and_then(|value| value.parse::<usize>().ok()) {
                    if values.len() > count && faces.len() < 10_000 {
                        faces.push(
                            values[1..=count]
                                .iter()
                                .filter_map(|value| value.parse().ok())
                                .collect(),
                        );
                    }
                }
            }
        }
        _ => {}
    }
    if let Some(asset) = document.nodes.first_mut() {
        asset
            .metrics
            .insert("vertices".to_string(), points.len() as f64);
        asset
            .metrics
            .insert("faces".to_string(), faces.len() as f64);
    }
    if !points.is_empty() {
        document.metadata.insert(
            "geometry".to_string(),
            json!({ "points": points, "faces": faces }),
        );
    }
}

fn stable_domain_id(prefix: &str, value: &str) -> String {
    format!(
        "{}:{}",
        prefix,
        &blake3::hash(value.as_bytes()).to_hex()[..16]
    )
}

fn normalized_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalized_file_types(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .map(|value| {
            value
                .trim()
                .trim_start_matches("*.")
                .trim_start_matches('.')
                .to_ascii_lowercase()
        })
        .filter(|value| !value.is_empty() && !value.contains(' '))
        .collect()
}

fn normalize_descriptor(mut descriptor: DomainPluginDescriptor) -> DomainPluginDescriptor {
    descriptor.metadata.id = descriptor.metadata.id.trim().to_ascii_lowercase();
    if descriptor.metadata.version.trim().is_empty() {
        descriptor.metadata.version = "1.0.0".to_string();
    }
    descriptor.plugin_api_version = RESEARCH_DOMAIN_PLUGIN_API_VERSION.to_string();
    descriptor.lifecycle.states = vec![
        "registered".to_string(),
        "activating".to_string(),
        "active".to_string(),
        "suspended".to_string(),
        "disposed".to_string(),
    ];
    descriptor.lifecycle.supports_hot_reload = true;
    descriptor.lifecycle.supports_workspace_sync = true;
    descriptor
}

fn file_type(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            path.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("file")
                .to_ascii_lowercase()
        })
}

fn is_ambiguous_file_type(extension: &str) -> bool {
    matches!(
        extension,
        "py" | "rs"
            | "c"
            | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "java"
            | "kt"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "go"
            | "json"
            | "jsonl"
            | "yaml"
            | "yml"
            | "toml"
            | "xml"
            | "csv"
            | "txt"
            | "log"
            | "md"
    )
}

fn is_text_file_type(extension: &str) -> bool {
    is_ambiguous_file_type(extension)
        || matches!(
            extension,
            "sql"
                | "ll"
                | "wat"
                | "proto"
                | "dot"
                | "graphml"
                | "obj"
                | "ply"
                | "stl"
                | "urdf"
                | "xacro"
                | "sdf"
                | "glsl"
                | "hlsl"
                | "vert"
                | "frag"
                | "cu"
                | "cuh"
                | "cl"
                | "m"
                | "jl"
        )
}

fn provider(domain: &str, provider_type: &str) -> DomainProviderDescriptor {
    DomainProviderDescriptor {
        id: format!("atlas.domain.{domain}.{provider_type}"),
        api_version: RESEARCH_DOMAIN_PLUGIN_API_VERSION.to_string(),
        provider_type: provider_type.to_string(),
    }
}

fn visualization(
    id: &str,
    label: &str,
    renderer: &str,
    compatible: &[&str],
) -> DomainVisualizationDescriptor {
    DomainVisualizationDescriptor {
        id: id.to_string(),
        label: label.to_string(),
        renderer: renderer.to_string(),
        compatible_file_types: compatible
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        adapter: format!("atlas.domain.{id}"),
        workbench_region: "primary".to_string(),
        requires_sdk: Vec::new(),
    }
}

fn workbench_tool(
    id: &str,
    label: &str,
    kind: &str,
    description: &str,
    sdk: &str,
) -> DomainWorkbenchToolDescriptor {
    DomainWorkbenchToolDescriptor {
        id: id.to_string(),
        label: label.to_string(),
        kind: kind.to_string(),
        description: description.to_string(),
        sdk: sdk.to_string(),
    }
}

fn workbench_stage(
    id: &str,
    label: &str,
    description: &str,
    agent: &str,
    inputs: &[&str],
    outputs: &[&str],
    gate: &str,
) -> DomainWorkbenchStageDescriptor {
    DomainWorkbenchStageDescriptor {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        agent: agent.to_string(),
        inputs: inputs.iter().map(|value| (*value).to_string()).collect(),
        outputs: outputs.iter().map(|value| (*value).to_string()).collect(),
        gate: gate.to_string(),
    }
}

fn workflow_for(domain: &str) -> Vec<DomainWorkbenchStageDescriptor> {
    let stages: [(&str, &str, &str, &str, &[&str], &[&str], &str); 4] = match domain {
        "ai-ml" => [
            ("data-contract", "Data contract", "Profile splits, schema, leakage, and reproducibility inputs.", "research", &["dataset", "split policy"], &["data profile", "baseline"], "Schema, provenance, and leakage checks pass"),
            ("train", "Train & track", "Run the configured experiment and retain parameters, logs, and checkpoints.", "training", &["training config", "dataset revision"], &["run record", "checkpoint"], "Run is reproducible from a recorded config"),
            ("evaluate", "Evaluate", "Compare metrics, slices, calibration, and failure cases against the baseline.", "evaluation", &["checkpoint", "evaluation set"], &["metric report", "error slices"], "Primary metric and regression thresholds pass"),
            ("package", "Package inference", "Validate the exported model and its serving contract on representative inputs.", "inference", &["approved checkpoint", "input contract"], &["model artifact", "inference report"], "Export parity and latency budget pass"),
        ],
        "computer-vision" => [
            ("curate", "Curate media", "Audit image/video quality, annotation coverage, classes, and split balance.", "annotation", &["media", "annotations"], &["dataset audit", "validated split"], "Broken media and annotation defects are resolved"),
            ("calibrate", "Calibrate pipeline", "Verify transforms, camera calibration, coordinate conventions, and overlays.", "vision", &["calibration", "preprocessing"], &["overlay evidence", "transform contract"], "Ground truth aligns in the target coordinate space"),
            ("infer", "Run inference", "Execute the configured detector, segmenter, or reconstruction pipeline.", "vision", &["model", "media selection"], &["predictions", "timings"], "Outputs retain model and input revisions"),
            ("score", "Score failures", "Measure task metrics and review false positives, misses, and hard slices.", "evaluation", &["predictions", "ground truth"], &["metric report", "failure gallery"], "Quality thresholds pass on required slices"),
        ],
        "nlp" => [
            ("corpus", "Corpus contract", "Validate encoding, language mix, document boundaries, labels, and leakage.", "corpus", &["corpus", "label schema"], &["corpus profile", "clean split"], "Deduplication and leakage checks pass"),
            ("linguistics", "Annotate & parse", "Apply the configured tokenizer, span rules, parser, and normalization.", "nlp", &["clean corpus", "tokenizer"], &["tokens", "linguistic annotations"], "Offsets round-trip to original text"),
            ("model", "Model run", "Execute training or inference with the recorded model and decoding configuration.", "nlp", &["annotations", "model config"], &["predictions", "run record"], "Seed, model revision, and decoding are recorded"),
            ("language-eval", "Language evaluation", "Score aggregate and linguistic slices, then inspect grounded error examples.", "evaluation", &["predictions", "reference set"], &["metrics", "error taxonomy"], "Required quality and safety slices pass"),
        ],
        "computer-graphics" => [
            ("scene-ingest", "Scene ingest", "Resolve geometry, materials, textures, units, axes, and external references.", "graphics", &["scene assets", "asset manifest"], &["resolved scene", "dependency report"], "No missing references or unit mismatches"),
            ("geometry", "Geometry QA", "Check topology, normals, UVs, bounds, degeneracy, and draw complexity.", "graphics", &["resolved geometry"], &["mesh diagnostics", "repair list"], "Geometry meets target renderer constraints"),
            ("shade", "Shade & light", "Compile shaders and validate material, color-space, and lighting behavior.", "shader", &["materials", "shaders", "lights"], &["compile report", "look-dev capture"], "Shaders compile and render deterministically"),
            ("render", "Render validation", "Render reference frames and compare quality, timing, and visual regressions.", "render", &["approved scene", "render config"], &["frames", "performance trace"], "Reference diff and frame budget pass"),
        ],
        "cad" => [
            ("import", "Model integrity", "Validate units, bodies, topology, naming, and imported geometry health.", "geometry", &["CAD model"], &["integrity report", "healed model"], "Bodies are manifold and units are explicit"),
            ("constraints", "Constraints & intent", "Inspect feature order, sketch constraints, references, and design parameters.", "cad", &["feature tree", "sketches"], &["constraint report", "parameter map"], "No broken references or unintended degrees of freedom"),
            ("recompute", "Recompute", "Rebuild the model through the configured CAD kernel and capture failures.", "cad", &["parameter set", "feature tree"], &["rebuilt model", "recompute log"], "Full feature tree recomputes without error"),
            ("manufacture", "Manufacturing release", "Check tolerances, clearances, export format, and fabrication constraints.", "manufacturing", &["approved body", "process rules"], &["release geometry", "manufacturing report"], "Geometry and tolerances pass process checks"),
        ],
        "robotics" => [
            ("robot-contract", "Robot contract", "Validate links, joints, limits, inertia, frames, and controller interfaces.", "robotics", &["robot model", "controller config"], &["model diagnostics", "frame tree"], "Model and TF tree are complete and consistent"),
            ("perception", "Perception & state", "Synchronize sensor streams, localization, maps, and state estimates.", "control", &["sensor logs", "calibration"], &["state estimate", "timing report"], "Timestamps, frames, and covariance are valid"),
            ("plan", "Plan & simulate", "Plan collision-aware trajectories and replay them in the configured simulator.", "planning", &["robot state", "planning scene", "goal"], &["trajectory", "simulation trace"], "Limits, collision, and goal tolerances pass"),
            ("validate-robot", "Validate execution", "Compare commanded and observed motion, safety limits, and recovery behavior.", "control", &["trajectory", "execution log"], &["tracking metrics", "safety evidence"], "Tracking and safety thresholds pass"),
        ],
        "computer-networks" => [
            ("capture-scope", "Capture contract", "Define interfaces, filters, clocks, topology, and privacy boundaries.", "network", &["capture source", "topology"], &["capture manifest", "clock status"], "Scope and timestamps are trustworthy"),
            ("decode-flows", "Decode & reassemble", "Decode protocols, reconstruct flows, and retain malformed-packet evidence.", "protocol", &["packet capture"], &["flow table", "protocol diagnostics"], "Reassembly and checksum policy are explicit"),
            ("measure-network", "Measure behavior", "Calculate loss, retransmission, latency, throughput, and route changes.", "observability", &["decoded flows"], &["network metrics", "anomaly windows"], "Metrics trace back to packet ranges"),
            ("diagnose-network", "Diagnose", "Correlate symptoms with topology, endpoints, protocol state, and runtime changes.", "network", &["metrics", "topology", "runtime evidence"], &["root-cause report", "reproduction"], "Claim is supported by capture evidence"),
        ],
        "operating-systems" => [
            ("record-os", "Record evidence", "Capture process, scheduler, memory, I/O, and kernel events with symbol context.", "systems", &["target process", "trace profile"], &["trace", "environment manifest"], "Trace loss and clock quality are acceptable"),
            ("correlate-os", "Correlate timelines", "Align threads, CPU scheduling, syscalls, faults, handles, and I/O.", "kernel", &["trace", "symbols"], &["correlated timeline", "hot intervals"], "Events resolve to processes, threads, and symbols"),
            ("inspect-os", "Inspect state", "Inspect dumps or live state around selected anomalies and resource pressure.", "systems", &["hot interval", "dump or process"], &["state snapshot", "resource findings"], "Finding includes address/time/process provenance"),
            ("verify-os", "Verify diagnosis", "Reproduce the condition and compare traces before and after mitigation.", "performance", &["hypothesis", "reproduction"], &["comparison trace", "verification report"], "Observed delta supports the diagnosis"),
        ],
        "compiler" => [
            ("frontend", "Frontend", "Parse sources, resolve symbols and types, and retain diagnostics with source spans.", "compiler", &["source", "compile flags"], &["AST", "symbol table", "diagnostics"], "Frontend succeeds with expected language semantics"),
            ("lower", "Lower to IR", "Lower through configured dialects while checking invariants and debug locations.", "compiler", &["typed AST", "target config"], &["IR", "verification log"], "Every IR stage passes its verifier"),
            ("optimize", "Optimize", "Run named pass pipelines and compare IR, cost, and semantic equivalence.", "optimization", &["verified IR", "pass pipeline"], &["optimized IR", "pass remarks"], "Equivalence and regression checks pass"),
            ("codegen", "Code generation", "Generate target code and validate execution, size, and performance evidence.", "analysis", &["optimized IR", "target"], &["binary", "disassembly", "benchmark"], "Tests pass on the target configuration"),
        ],
        "database" => [
            ("connect", "Connection & schema", "Resolve the target connection, snapshot schema, statistics, and transaction scope.", "database", &["connection", "schema"], &["schema snapshot", "statistics status"], "Target and isolation level are explicit"),
            ("author-query", "Query contract", "Bind parameters, validate semantics, and define expected result invariants.", "query", &["SQL", "parameters"], &["validated query", "result contract"], "Query is safe for the selected environment"),
            ("plan-query", "Plan & profile", "Inspect actual operators, cardinalities, I/O, memory, and timing.", "query", &["validated query", "statistics"], &["actual plan", "profile"], "Plan evidence uses the target engine"),
            ("verify-data", "Verify results", "Check row-level invariants, lineage, regressions, and transactional effects.", "data", &["results", "result contract"], &["validation report", "lineage"], "Result and side-effect assertions pass"),
        ],
        "software-engineering" => [
            ("scope-change", "Scope the change", "Map requirements to modules, owners, dependencies, tests, and risk boundaries.", "architecture", &["task", "repository index"], &["change plan", "impact map"], "Acceptance criteria and affected surfaces are explicit"),
            ("implement-change", "Implement", "Edit the smallest coherent set of files while preserving repository conventions.", "coding", &["change plan", "source"], &["patch", "change notes"], "Diff is focused and internally consistent"),
            ("verify-change", "Build & test", "Run targeted checks followed by the repository's required validation gates.", "testing", &["patch", "test targets"], &["test report", "build evidence"], "Required checks pass with recorded commands"),
            ("review-change", "Review & handoff", "Inspect behavior, security, compatibility, migrations, and operational impact.", "review", &["diff", "verification evidence"], &["review findings", "handoff"], "Findings are resolved or explicitly accepted"),
        ],
        "program-analysis" => [
            ("analysis-target", "Target model", "Resolve binaries/sources, build flags, dependencies, entry points, and assumptions.", "analysis", &["analysis target", "build metadata"], &["target manifest", "fact schema"], "Target revision and analysis scope are reproducible"),
            ("extract-facts", "Extract facts", "Generate CFG, call, type, data-flow, and runtime facts with source mappings.", "analysis", &["target manifest"], &["fact database", "coverage report"], "Fact extraction coverage is measured"),
            ("run-analysis", "Run queries", "Execute named analyses with explicit sources, sinks, lattices, or trace predicates.", "security", &["fact database", "query"], &["findings", "paths"], "Each finding has a complete evidence path"),
            ("validate-findings", "Validate findings", "Deduplicate, reproduce, classify confidence, and suppress with rationale.", "verification", &["findings", "runtime evidence"], &["triage report", "validated findings"], "Reported findings are reproducible"),
        ],
        "cyber-security" => [
            ("security-scope", "Scope & threat model", "Define authorization, assets, trust boundaries, attack surfaces, and exclusions.", "threat-model", &["authorized target", "architecture"], &["scope record", "threat model"], "Authorization and test boundaries are recorded"),
            ("collect-security", "Collect evidence", "Run configured scanners and collect code, dependency, traffic, or runtime evidence.", "security", &["scope record", "target"], &["raw findings", "scan logs"], "Evidence retains tool versions and target revision"),
            ("triage-security", "Triage & validate", "Reproduce findings, trace exploitability, remove duplicates, and assign severity.", "audit", &["raw findings", "target context"], &["validated findings", "attack paths"], "Severity is evidence-based and reproducible"),
            ("remediate-security", "Remediate & retest", "Apply bounded fixes and rerun the original evidence path plus regressions.", "security", &["validated finding", "fix"], &["retest report", "residual risk"], "Original finding is closed without regression"),
        ],
        "hpc" => [
            ("hpc-baseline", "Reproducible baseline", "Record hardware, drivers, ranks, affinity, input size, and warm-up policy.", "hpc", &["workload", "launch config"], &["baseline", "environment manifest"], "Variance and environment are documented"),
            ("profile-hpc", "Profile timeline", "Capture CPU, GPU, MPI, transfers, synchronization, and kernel timelines.", "profiling", &["baseline run", "profile config"], &["trace", "hotspot table"], "Profiler overhead and trace loss are acceptable"),
            ("analyze-hpc", "Analyze kernels & ranks", "Inspect occupancy, bandwidth, divergence, imbalance, and communication costs.", "parallel", &["trace", "kernel metadata"], &["bottleneck model", "optimization candidates"], "Bottleneck claim maps to measured counters"),
            ("benchmark-hpc", "Benchmark scaling", "Compare corrected runs across problem sizes, devices, and rank counts.", "hpc", &["optimized workload", "benchmark matrix"], &["scaling report", "regression gate"], "Correctness and performance thresholds pass"),
        ],
        "distributed-systems" => [
            ("topology-contract", "Topology & SLO", "Resolve services, versions, dependencies, traffic, replicas, and SLO budgets.", "distributed", &["deployment", "service metadata"], &["topology snapshot", "SLO contract"], "Observed topology matches intended deployment"),
            ("collect-spans", "Collect telemetry", "Correlate traces, metrics, logs, RPC metadata, and cluster events.", "observability", &["telemetry sources", "time range"], &["trace set", "signal quality report"], "Trace coverage and clock alignment are measured"),
            ("analyze-distributed", "Analyze behavior", "Inspect critical paths, retries, queues, consensus, replicas, and failure propagation.", "reliability", &["trace set", "topology"], &["causal timeline", "failure hypothesis"], "Hypothesis is supported across signals"),
            ("verify-resilience", "Verify resilience", "Run bounded failure scenarios and compare recovery, correctness, and SLO impact.", "reliability", &["failure hypothesis", "scenario"], &["experiment report", "recovery evidence"], "Safety and recovery invariants pass"),
        ],
        "scientific-computing" => [
            ("numerical-contract", "Numerical contract", "Define equations, units, domains, boundary conditions, tolerances, and reference cases.", "scientific", &["model", "input data"], &["problem specification", "reference case"], "Units and mathematical assumptions are explicit"),
            ("discretize", "Discretize & configure", "Validate mesh/grid quality, solver configuration, convergence criteria, and stability limits.", "numerical", &["problem specification", "mesh"], &["solver config", "mesh diagnostics"], "Discretization and stability checks pass"),
            ("simulate", "Run simulation", "Execute the model while retaining residuals, iterations, checkpoints, and environment data.", "simulation", &["solver config", "initial state"], &["fields", "convergence history"], "Solver converges under the declared criteria"),
            ("validate-science", "Validate & quantify", "Compare against analytic/reference data and quantify error, sensitivity, and uncertainty.", "scientific", &["simulation outputs", "reference case"], &["validation report", "uncertainty bounds"], "Error and conservation thresholds pass"),
        ],
        _ => [
            ("scope", "Scope", "Resolve the real workspace inputs and define the acceptance contract.", "research", &["workspace assets"], &["task contract"], "Inputs and acceptance criteria are explicit"),
            ("execute", "Execute", "Use the configured domain tools against the selected workspace evidence.", "research", &["task contract"], &["domain outputs"], "Execution evidence is retained"),
            ("verify", "Verify", "Check outputs against domain invariants and reproducible evidence.", "verification", &["domain outputs"], &["verification report"], "Required checks pass"),
            ("handoff", "Handoff", "Register artifacts and summarize provenance, limitations, and next actions.", "research", &["verified outputs"], &["artifact manifest"], "Every claim links to an artifact or runtime result"),
        ],
    };
    stages
        .into_iter()
        .map(|(id, label, description, agent, inputs, outputs, gate)| {
            workbench_stage(id, label, description, agent, inputs, outputs, gate)
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn workbench_intent(
    id: &str,
    label: &str,
    description: &str,
    agent: &str,
    input_contract: &str,
    expected_outputs: &[&str],
    recommended_actions: &[&str],
    required_sdks: &[&str],
    workflow_stages: &[&str],
    preview_kind: &str,
    gate: &str,
    asset_required: bool,
) -> DomainIntentDescriptor {
    DomainIntentDescriptor {
        id: id.to_string(),
        label: label.to_string(),
        description: description.to_string(),
        agent: agent.to_string(),
        input_contract: input_contract.to_string(),
        expected_outputs: expected_outputs.iter().map(|value| (*value).to_string()).collect(),
        recommended_actions: recommended_actions.iter().map(|value| (*value).to_string()).collect(),
        required_sdks: required_sdks.iter().map(|value| (*value).to_string()).collect(),
        workflow_stages: workflow_stages.iter().map(|value| (*value).to_string()).collect(),
        preview_kind: preview_kind.to_string(),
        gate: gate.to_string(),
        asset_required,
    }
}

fn intents_for(domain: &str) -> Vec<DomainIntentDescriptor> {
    match domain {
        "ai-ml" => vec![
            workbench_intent("text-to-experiment", "Design & run experiment", "Turn a research objective into a reproducible data contract, training configuration, tracked run, checkpoint and evaluation report.", "training", "Research objective, dataset or workspace data contract, target metric, model constraints and compute budget.", &["experiment config", "run record", "checkpoint", "metric and error-slice report"], &["inspect-tensor", "inspect-onnx"], &["PyTorch", "NumPy", "ONNX Runtime"], &["data-contract", "train", "evaluate", "package"], "run-report", "Recorded configuration reproduces the run and required evaluation thresholds pass.", false),
            workbench_intent("model-evaluation", "Evaluate & compare models", "Inspect real models and checkpoints, execute the project evaluation path and compare metrics, calibration and failure slices.", "evaluation", "Selected model/checkpoint, evaluation dataset revision, baseline and acceptance thresholds.", &["model inspection", "comparison report", "failure slices", "approved checkpoint"], &["inspect-onnx", "inspect-tensor"], &["ONNX Runtime", "NumPy"], &["evaluate", "package"], "model-comparison", "Metrics trace to exact model and dataset revisions and regression gates pass.", true),
            workbench_intent("representation-analysis", "Analyze tensors & representations", "Profile real tensors, embeddings or activations and produce evidence-backed representation diagnostics.", "research", "Selected tensor/array artifact, semantic labels and analysis question.", &["tensor profile", "embedding or activation artifact", "analysis report"], &["inspect-tensor"], &["NumPy"], &["data-contract", "evaluate"], "representation-report", "All plotted values derive from the selected artifact and numerical checks pass.", true),
        ],
        "computer-vision" => vec![
            workbench_intent("vision-inference-study", "Run vision inference study", "Execute a detector, segmenter or tracker on real media and review overlays, timings and failure cases.", "vision", "Selected media, model revision, class schema, thresholds and evaluation slice.", &["prediction artifact", "overlay evidence", "timing report", "failure gallery"], &["inspect-image"], &["OpenCV", "PyTorch Vision"], &["curate", "calibrate", "infer", "score"], "media-result", "Predictions retain input/model revisions and required task metrics pass.", true),
            workbench_intent("annotation-quality", "Audit & refine annotations", "Audit annotation coverage, geometry and class consistency, then produce a versioned correction set.", "annotation", "Media selection, annotation revision, class schema and review policy.", &["annotation audit", "corrected annotation artifact", "coverage report"], &["inspect-image"], &["OpenCV"], &["curate", "score"], "annotation-review", "Offsets, classes and geometry round-trip to the original media without unresolved defects.", true),
            workbench_intent("reconstruct-scene", "Reconstruct a 3D scene", "Build and validate camera poses, point clouds or meshes from calibrated real inputs.", "vision", "Calibrated image/video set, camera model, reconstruction method and coordinate convention.", &["camera pose artifact", "point cloud or mesh", "reconstruction diagnostics"], &["inspect-image", "inspect-geometry"], &["OpenCV", "Open3D"], &["curate", "calibrate", "infer", "score"], "reconstruction-result", "Reprojection, scale and topology checks pass on the generated geometry.", true),
        ],
        "nlp" => vec![
            workbench_intent("prompt-rag-study", "Build a prompt / RAG study", "Create a versioned prompt and retrieval pipeline, execute it on real corpus evidence and evaluate grounded outputs.", "nlp", "Task objective, corpus/index revision, prompt constraints, retrieval policy and evaluation rubric.", &["prompt pipeline", "retrieval traces", "grounded responses", "evaluation report"], &["tokenize", "dependency-parse"], &["Hugging Face Transformers", "Sentence Transformers"], &["corpus", "linguistics", "model", "language-eval"], "reasoning-trace", "Every response claim links to retrieved evidence and required language-quality gates pass.", false),
            workbench_intent("corpus-analysis", "Analyze & annotate corpus", "Tokenize, parse and profile a real corpus while preserving source offsets and provenance.", "corpus", "Selected corpus, language/model policy, label schema and split rules.", &["token document", "dependency/entity annotations", "corpus profile"], &["tokenize", "dependency-parse"], &["spaCy", "NLTK"], &["corpus", "linguistics", "language-eval"], "corpus-report", "Offsets round-trip to source text and corpus leakage/quality checks pass.", true),
            workbench_intent("language-evaluation", "Evaluate a language system", "Compare real predictions across linguistic, retrieval, safety and error slices.", "evaluation", "Prediction artifact, reference set, decoding configuration, baseline and rubric.", &["metric table", "error taxonomy", "comparison report"], &["tokenize", "dependency-parse"], &["spaCy", "NLTK"], &["language-eval"], "language-evaluation", "Metrics and qualitative examples trace to exact inputs and thresholded slices pass.", true),
        ],
        "computer-graphics" => vec![
            workbench_intent("text-to-scene", "Create scene from brief", "Author a reproducible scene, geometry, camera, materials and lighting from a design brief using real graphics tooling.", "graphics", "Scene brief, units, reference assets, output format, camera and render constraints.", &["native scene artifact", "mesh/material manifest", "reference render"], &["validate-mesh", "render-scene"], &["Blender Python API", "Open3D"], &["scene-ingest", "geometry", "shade", "render"], "render-frame", "Scene dependencies resolve, geometry is valid and the reference render completes deterministically.", false),
            workbench_intent("mesh-material-qa", "Validate mesh & materials", "Inspect real topology, bounds, normals, UV/material dependencies and produce actionable repair evidence.", "graphics", "Selected mesh/scene, target renderer constraints and quality budget.", &["mesh diagnostics", "material dependency report", "repair list"], &["validate-mesh"], &["Open3D"], &["scene-ingest", "geometry", "shade"], "geometry-qa", "Geometry and material contracts pass without invented topology or missing dependencies.", true),
            workbench_intent("render-validation", "Render & diagnose frame", "Render a real scene headlessly, retain outputs and diagnose visual or performance regressions.", "render", "Selected native scene, frame/camera, render configuration and comparison baseline.", &["rendered frame", "render log", "comparison and timing report"], &["render-scene"], &["Blender Python API"], &["shade", "render"], "render-validation", "Render succeeds with recorded settings and reference-diff/frame-budget gates pass.", true),
        ],
        "cad" => vec![
            workbench_intent("text-to-parametric-model", "Create parametric model from brief", "Translate dimensional design intent into a reviewable parametric source/model, recompute it in a real CAD kernel and export verified geometry.", "cad", "Part brief, dimensions, units, constraints, tolerances, material/process assumptions and target formats.", &["parametric source or FCStd", "feature/parameter map", "STEP or STL geometry", "recompute report"], &["recompute", "export"], &["FreeCAD", "CadQuery"], &["import", "constraints", "recompute", "manufacture"], "cad-release", "The full feature tree recomputes and exported geometry passes topology, unit and tolerance checks.", false),
            workbench_intent("engineering-drawing", "Produce engineering drawing", "Generate drawing views, dimensions and release metadata from a verified model using real CAD artifacts.", "manufacturing", "Selected model, drawing standard, projection, required views, dimensions, tolerances and sheet format.", &["drawing artifact", "dimension/tolerance report", "release manifest"], &["recompute", "export"], &["FreeCAD"], &["constraints", "recompute", "manufacture"], "drawing-release", "Views reference the approved model revision and mandatory dimensions/tolerances are present.", true),
            workbench_intent("constraint-redesign", "Revise constraints & features", "Modify a real parametric design while retaining feature intent, parameter provenance and before/after validation.", "geometry", "Selected model, requested change, locked design constraints and manufacturing gate.", &["revised model", "parameter/constraint diff", "recompute evidence"], &["recompute", "export"], &["FreeCAD", "CadQuery"], &["constraints", "recompute", "manufacture"], "design-revision", "No broken references remain and old/new configurations both satisfy declared invariants.", true),
        ],
        "robotics" => vec![
            workbench_intent("robot-model-workflow", "Create / validate robot model", "Author or revise a URDF/SDF robot contract and validate links, joints, limits, inertias and frames with real tooling.", "robotics", "Robot morphology, kinematics, limits, inertial data, sensors, frames and controller interfaces.", &["URDF/SDF artifact", "TF/kinematic graph", "model validation report"], &["validate-urdf"], &["ROS 2", "check_urdf"], &["robot-contract", "perception", "plan", "validate-robot"], "robot-model", "The model parses, TF is connected and physical/joint constraints are internally consistent.", false),
            workbench_intent("motion-simulation", "Plan & simulate motion", "Create a collision-aware trajectory, execute it in an installed simulator and compare commanded versus observed motion.", "planning", "Robot model/state, planning scene, goal, limits, collision policy and simulator configuration.", &["trajectory", "simulation trace", "tracking and safety report"], &["validate-urdf"], &["ROS 2", "MuJoCo"], &["robot-contract", "perception", "plan", "validate-robot"], "robot-run", "Collision, joint, goal-tolerance and tracking gates pass with synchronized evidence.", true),
            workbench_intent("ros-recording-diagnosis", "Diagnose ROS recording", "Inspect a real bag/MCAP recording and correlate topics, frames, timing and observed behavior.", "control", "Selected recording, fixed frame, topic scope, time range and diagnosis hypothesis.", &["bag metadata", "synchronized event trace", "diagnosis report"], &["bag-info"], &["ROS 2"], &["perception", "validate-robot"], "ros-diagnosis", "Timestamp/frame integrity is measured and diagnosis traces to recorded messages.", true),
        ],
        "computer-networks" => vec![
            workbench_intent("capture-diagnosis", "Diagnose packet capture", "Decode and reassemble a real capture, measure flows and produce a packet-evidenced root-cause report.", "network", "Selected PCAP/PCAPNG, display/capture filters, topology, endpoints and symptom window.", &["packet table", "protocol hierarchy", "flow metrics", "diagnosis report"], &["decode-packets", "protocol-hierarchy"], &["Wireshark"], &["capture-scope", "decode-flows", "measure-network", "diagnose-network"], "packet-trace", "Every diagnosis links to exact packet/flow ranges and checksum/reassembly policy is explicit.", true),
            workbench_intent("protocol-investigation", "Investigate protocol behavior", "Trace a real protocol or TCP state progression and quantify latency, retransmission and anomalies.", "protocol", "Selected capture/flow, protocol field path, clock policy and expected state contract.", &["state/field trace", "latency and retransmission report", "anomaly packet set"], &["decode-packets", "protocol-hierarchy"], &["Wireshark"], &["decode-flows", "measure-network", "diagnose-network"], "protocol-trace", "Protocol claims map to decoded fields and packet timing evidence.", true),
            workbench_intent("topology-experiment", "Design topology experiment", "Create a reproducible topology scenario, execute it only with an installed emulator/runtime and capture resulting traffic.", "observability", "Node/link specification, routing or fault scenario, traffic profile, metrics and safety scope.", &["topology specification", "runtime/capture artifact", "experiment report"], &[], &["Scapy"], &["capture-scope", "measure-network", "diagnose-network"], "topology-experiment", "Runtime topology matches the specification and measured outcomes trace to capture evidence.", false),
        ],
        "operating-systems" => vec![
            workbench_intent("performance-diagnosis", "Diagnose system performance", "Record or inspect a real OS performance trace and correlate CPU, scheduler, memory and I/O evidence.", "performance", "Target host/process, trace profile, symbol policy, symptom range and baseline.", &["trace artifact", "hot interval/stack evidence", "diagnosis report"], &["trace-info", "perf-report"], &["Windows ETW", "Linux perf"], &["record-os", "correlate-os", "inspect-os", "verify-os"], "performance-trace", "Trace loss/symbol quality is known and before/after evidence supports the diagnosis.", true),
            workbench_intent("process-memory-analysis", "Analyze process & memory state", "Inspect real process, thread, memory-region or dump evidence around a selected anomaly.", "systems", "Trace/dump/process evidence, process/thread selection, symbols and hypothesis.", &["state snapshot", "memory/thread findings", "provenance report"], &["trace-info", "perf-report"], &["Windows ETW", "Linux perf"], &["correlate-os", "inspect-os", "verify-os"], "system-state", "Every finding includes process, address/time and symbol provenance.", true),
            workbench_intent("trace-comparison", "Compare traces before / after", "Align two real traces and quantify scheduler, memory, syscall and I/O deltas after a change.", "kernel", "Baseline and candidate traces, matching workload/host contract and comparison ranges.", &["aligned trace comparison", "regression table", "verification report"], &["trace-info", "perf-report"], &["Windows ETW", "Linux perf"], &["correlate-os", "verify-os"], "trace-comparison", "Workload/environment are comparable and observed deltas meet the stated gate.", true),
        ],
        "compiler" => vec![
            workbench_intent("source-to-pipeline", "Trace source through compiler", "Compile real source and correlate spans across AST, LLVM IR/SSA, diagnostics and target assembly.", "compiler", "Selected translation unit, compiler flags, language mode, target and source range.", &["AST", "LLVM IR", "source-to-stage correlation", "diagnostics"], &["build-ast", "emit-ir"], &["Clang", "LLVM"], &["frontend", "lower", "optimize", "codegen"], "compiler-stage", "Frontend and IR verifiers pass and every stage retains source/target provenance.", true),
            workbench_intent("optimization-study", "Design optimization study", "Compare named optimization/pass pipelines on real source while checking semantic equivalence, size and performance.", "optimization", "Source, baseline/candidate flags or pass pipelines, target and correctness benchmark.", &["before/after IR", "pass remarks", "code-size/performance comparison", "equivalence evidence"], &["emit-ir", "build-ast"], &["Clang", "LLVM"], &["lower", "optimize", "codegen"], "optimization-report", "Correctness/equivalence passes before any performance claim is accepted.", true),
            workbench_intent("compiler-diagnosis", "Diagnose compiler failure", "Reproduce and localize a parser, lowering, verifier or code-generation failure using minimized real evidence.", "analysis", "Failing source, exact command/tool version, target and observed diagnostic or miscompile.", &["reproduction", "localized stage evidence", "minimized test or diagnosis report"], &["build-ast", "emit-ir"], &["Clang", "LLVM"], &["frontend", "lower", "optimize", "codegen"], "compiler-diagnosis", "Failure reproduces with recorded versions and the claim is isolated to a verified stage.", true),
        ],
        "database" => vec![
            workbench_intent("query-investigation", "Investigate query & plan", "Execute a bounded real query, inspect schema and actual plan evidence, and validate result invariants.", "query", "Selected database, read-only SQL, parameters, row/time limits and expected result contract.", &["query result", "schema/plan evidence", "validation report"], &["sqlite-schema", "sqlite-query"], &["SQLite"], &["connect", "author-query", "plan-query", "verify-data"], "query-result", "Query is safe, result invariants pass and plan claims use the target engine.", true),
            workbench_intent("schema-design-review", "Design / review schema", "Turn a data contract into a reviewable schema or audit a real schema, indexes and relationships.", "database", "Entity/data contract, workload assumptions, consistency rules, target engine and migration constraints.", &["schema or migration artifact", "ER/index rationale", "schema validation report"], &["sqlite-schema"], &["SQLite"], &["connect", "author-query", "plan-query", "verify-data"], "schema-review", "Keys, constraints, migrations and representative query plans meet declared invariants.", false),
            workbench_intent("transaction-diagnosis", "Diagnose transactions & locks", "Correlate real queries, lock/transaction evidence and storage behavior around contention or correctness symptoms.", "data", "Database/trace, transaction scope, isolation level, time window and suspected conflict.", &["transaction timeline", "lock/dependency graph", "root-cause and verification report"], &["sqlite-schema", "sqlite-query"], &["SQLite"], &["connect", "plan-query", "verify-data"], "transaction-report", "Diagnosis is reproduced under the stated isolation level and links to real statements/locks.", true),
        ],
        "software-engineering" => vec![
            workbench_intent("agent-engineering-change", "Plan, implement & verify change", "Turn a change request into an impact map, focused implementation, targeted checks and reviewable handoff.", "coding", "Change objective, acceptance criteria, repository revision, constraints and quality gate.", &["change plan", "focused patch", "test/build evidence", "handoff summary"], &["cargo-metadata", "npm-metadata"], &["Cargo", "npm"], &["scope-change", "implement-change", "verify-change", "review-change"], "change-review", "Acceptance criteria map to passing checks and every changed file belongs to the declared scope.", false),
            workbench_intent("architecture-analysis", "Analyze architecture & impact", "Map real modules, ownership and dependency impact for a proposed change or research question.", "architecture", "Repository revision, scope/modules, question, ownership source and architecture rules.", &["module/dependency graph", "impact and ownership report", "risk boundaries"], &["cargo-metadata", "npm-metadata"], &["Cargo", "npm"], &["scope-change", "review-change"], "architecture-report", "Graph edges derive from repository metadata/source and affected surfaces are explicit.", false),
            workbench_intent("release-readiness", "Assess release readiness", "Assemble real build, test, dependency, migration and operational evidence against release gates.", "review", "Release target, channel, required checks, compatibility/migration policy and known risks.", &["release manifest", "quality-gate evidence", "residual-risk report"], &["cargo-metadata", "npm-metadata"], &["Cargo", "npm"], &["scope-change", "verify-change", "review-change"], "release-report", "All required checks are recorded and unresolved risk is explicitly accepted or blocks release.", false),
        ],
        "program-analysis" => vec![
            workbench_intent("static-analysis-study", "Run static analysis study", "Build a reproducible target/fact model, execute real analyzers and validate findings with evidence paths.", "analysis", "Selected source/target, build metadata, scope, rules/queries and confidence policy.", &["target manifest", "findings", "CFG/call/data-flow evidence", "triage report"], &["semgrep-scan", "clang-cfg"], &["Semgrep", "Clang"], &["analysis-target", "extract-facts", "run-analysis", "validate-findings"], "analysis-finding", "Each reported finding traces to target revision and a reproducible evidence path.", true),
            workbench_intent("dataflow-investigation", "Trace data / control flow", "Investigate definitions, uses, control dependencies or taint paths in a real target.", "security", "Target revision, entry points, source/sink or value question, build flags and assumptions.", &["CFG/data-flow graph", "path evidence", "analysis report"], &["clang-cfg", "semgrep-scan"], &["Clang", "Semgrep"], &["analysis-target", "extract-facts", "run-analysis", "validate-findings"], "dataflow-report", "Paths are complete under the declared model and false positives are reproduced or excluded with rationale.", true),
            workbench_intent("execution-coverage-study", "Analyze execution & coverage", "Correlate real runtime traces, profiles or coverage with static structure and selected hypotheses.", "verification", "Target/build, test or trace source, entry points, time/test range and coverage question.", &["execution trace", "coverage/profile artifact", "correlation report"], &["clang-cfg"], &["Clang"], &["analysis-target", "extract-facts", "validate-findings"], "execution-coverage", "Runtime evidence maps to the exact binary/source revision and coverage gaps are measured.", true),
        ],
        "cyber-security" => vec![
            workbench_intent("authorized-security-assessment", "Run authorized assessment", "Define an explicit authorization boundary, execute configured scanners and validate findings against real target evidence.", "security", "Authorized target/scope, exclusions, threat model, ruleset and evidence-retention policy.", &["scope record", "scan result", "validated findings", "risk report"], &["semgrep-scan", "yara-scan"], &["Semgrep", "YARA"], &["security-scope", "collect-security", "triage-security", "remediate-security"], "security-finding", "Authorization is recorded and every finding retains tool/target versions plus reproducible evidence.", true),
            workbench_intent("malware-binary-triage", "Triage binary / malware evidence", "Analyze authorized binary or rule evidence, retain indicators and distinguish validated behavior from hypotheses.", "audit", "Authorized artifact, hash/provenance, YARA rules or analysis scope and isolation policy.", &["binary/rule result", "indicator set", "triage and confidence report"], &["yara-scan"], &["YARA"], &["security-scope", "collect-security", "triage-security"], "malware-triage", "Artifact identity is fixed and all behavioral claims are separated from unverified indicators.", true),
            workbench_intent("remediation-retest", "Remediate & retest finding", "Apply a bounded fix and rerun the original evidence path plus regression checks.", "security", "Validated finding, target revision, remediation constraints, original command/rules and acceptance gate.", &["remediation diff", "retest evidence", "residual-risk report"], &["semgrep-scan", "yara-scan"], &["Semgrep", "YARA"], &["triage-security", "remediate-security"], "security-retest", "Original finding is no longer reproducible and required regressions pass.", true),
        ],
        "hpc" => vec![
            workbench_intent("gpu-profile-study", "Profile CPU / GPU workload", "Analyze a real Nsight trace or profile with synchronized kernels, transfers, counters and environment evidence.", "profiling", "Selected report/workload, hardware/driver manifest, launch command, inputs and baseline.", &["timeline/profile summary", "hotspot table", "bottleneck report"], &["nsys-stats", "ncu-import"], &["Nsight Systems", "Nsight Compute"], &["hpc-baseline", "profile-hpc", "analyze-hpc", "benchmark-hpc"], "profile-report", "Profiler overhead is known and bottleneck claims map to measured timelines/counters.", true),
            workbench_intent("kernel-optimization", "Optimize GPU kernel", "Use real profiler evidence to modify a kernel, then compare correctness and performance against the fixed baseline.", "parallel", "Kernel/source and profile, target GPU, input/correctness contract and performance baseline.", &["kernel change", "before/after profiles", "correctness and performance report"], &["ncu-import", "nsys-stats"], &["Nsight Compute", "Nsight Systems"], &["hpc-baseline", "profile-hpc", "analyze-hpc", "benchmark-hpc"], "kernel-optimization", "Correctness passes and statistically credible speedup is measured under the same environment.", true),
            workbench_intent("mpi-scaling-study", "Run MPI / scaling study", "Plan and execute a reproducible rank/node scaling matrix and analyze communication imbalance.", "hpc", "Workload, problem sizes, rank/node matrix, scheduler/affinity, correctness and variance policy.", &["run matrix", "communication/profile evidence", "scaling report"], &["nsys-stats"], &["Nsight Systems", "MPI"], &["hpc-baseline", "profile-hpc", "analyze-hpc", "benchmark-hpc"], "scaling-report", "Every run passes correctness and scaling claims include variance and environment metadata.", false),
        ],
        "distributed-systems" => vec![
            workbench_intent("cluster-diagnosis", "Diagnose cluster / service", "Snapshot a real Kubernetes context and correlate workloads, services and events around an incident or SLO symptom.", "distributed", "Cluster context, namespace, service scope, time window, SLO and symptom.", &["cluster snapshot", "ordered event evidence", "service/topology diagnosis"], &["cluster-snapshot", "cluster-events"], &["Kubernetes"], &["topology-contract", "collect-spans", "analyze-distributed", "verify-resilience"], "cluster-event", "Context/namespace are explicit and diagnosis links to real resource revisions and events.", false),
            workbench_intent("request-consensus-analysis", "Trace request / consensus behavior", "Correlate real request traces or consensus/replication events across nodes and services.", "observability", "Telemetry/cluster evidence, selected request or term, time range, topology and expected invariant.", &["request or consensus timeline", "replication/service graph", "causal analysis"], &["cluster-snapshot", "cluster-events"], &["Kubernetes"], &["topology-contract", "collect-spans", "analyze-distributed"], "distributed-trace", "Clock/signal quality is measured and causal claims hold across available telemetry.", false),
            workbench_intent("resilience-experiment", "Plan bounded resilience experiment", "Create a safe failure scenario, execute only inside the authorized cluster scope and verify recovery/correctness/SLO impact.", "reliability", "Authorized cluster scope, failure hypothesis, safety limits, recovery invariant and rollback plan.", &["experiment plan", "before/during/after snapshots", "recovery report"], &["cluster-snapshot", "cluster-events"], &["Kubernetes"], &["topology-contract", "collect-spans", "analyze-distributed", "verify-resilience"], "resilience-report", "Safety limits hold, rollback succeeds and recovery/correctness gates pass.", false),
        ],
        "scientific-computing" => vec![
            workbench_intent("equation-to-simulation", "Build & solve numerical model", "Turn equations and boundary conditions into a reviewable numerical model, run a real solver and retain convergence/results.", "simulation", "Equations, units, domain, initial/boundary conditions, solver/tolerance and reference case.", &["problem specification", "solver configuration", "field/array outputs", "convergence and validation report"], &["inspect-array", "inspect-vtk", "cmake-project"], &["NumPy", "SciPy", "VTK"], &["numerical-contract", "discretize", "simulate", "validate-science"], "simulation-result", "Units and assumptions are explicit, solver converges and error/conservation gates pass.", false),
            workbench_intent("parameter-sweep-study", "Run parameter / optimization study", "Execute a reproducible parameter matrix on the installed model and compare convergence, objective and sensitivity.", "numerical", "Model revision, parameter ranges, sampling/optimization method, solver gate and reference baseline.", &["parameter manifest", "run/result matrix", "sensitivity or optimization report"], &["inspect-array", "cmake-project"], &["NumPy", "SciPy"], &["numerical-contract", "discretize", "simulate", "validate-science"], "parameter-study", "Every result maps to a parameter set and failed/non-converged runs are not silently discarded.", false),
            workbench_intent("field-mesh-validation", "Validate fields, arrays & mesh", "Inspect real NumPy/VTK data for dimensions, finite ranges, topology, units and numerical invariants.", "scientific", "Selected array/field/mesh, expected dimensions/units, invariants and comparison reference.", &["array or mesh profile", "field/geometry diagnostics", "validation report"], &["inspect-array", "inspect-vtk"], &["NumPy", "VTK"], &["discretize", "validate-science"], "field-validation", "All displayed values derive from the selected data and declared numerical/mesh invariants pass.", true),
        ],
        _ => Vec::new(),
    }
}

fn workbench_for(domain: &str) -> DomainWorkbenchDescriptor {
    let (layout, explorer, primary, inspector, bottom, tools) = match domain {
        "ai-ml" => (
            "experiment",
            "Artifacts & Runs",
            "Model / Training Lab",
            "Tensor & Run Inspector",
            "Training & Evaluation",
            vec![
                workbench_tool(
                    "inspect-model",
                    "Inspect Model",
                    "inspect",
                    "Read model layers, tensors, shapes, and parameters.",
                    "ONNX Runtime",
                ),
                workbench_tool(
                    "run-training",
                    "Run Training",
                    "execute",
                    "Launch the selected training entry point in the workspace.",
                    "PyTorch",
                ),
                workbench_tool(
                    "evaluate",
                    "Evaluate",
                    "execute",
                    "Run evaluation against the selected checkpoint and dataset.",
                    "PyTorch",
                ),
            ],
        ),
        "computer-vision" => (
            "media",
            "Media & Annotations",
            "Vision Lab",
            "Detection Inspector",
            "Inference & Evaluation",
            vec![
                workbench_tool(
                    "inspect-image",
                    "Inspect Image",
                    "inspect",
                    "Inspect dimensions, channels, annotations, and overlays.",
                    "OpenCV",
                ),
                workbench_tool(
                    "run-inference",
                    "Run Inference",
                    "execute",
                    "Run the configured detector or segmenter on the selected media.",
                    "ONNX Runtime",
                ),
                workbench_tool(
                    "reconstruct",
                    "Reconstruct 3D",
                    "execute",
                    "Build point-cloud or mesh output from calibrated inputs.",
                    "Open3D",
                ),
            ],
        ),
        "nlp" => (
            "corpus",
            "Corpus & Models",
            "Language Lab",
            "Token & Span Inspector",
            "Parsing & Evaluation",
            vec![
                workbench_tool(
                    "tokenize",
                    "Tokenize",
                    "execute",
                    "Tokenize the selected corpus with the configured tokenizer.",
                    "Hugging Face Transformers",
                ),
                workbench_tool(
                    "dependency-parse",
                    "Dependency Parse",
                    "execute",
                    "Run syntactic parsing and refresh linguistic annotations.",
                    "spaCy",
                ),
                workbench_tool(
                    "evaluate",
                    "Evaluate Corpus",
                    "execute",
                    "Run the configured language evaluation suite.",
                    "NLTK",
                ),
            ],
        ),
        "computer-graphics" => (
            "scene",
            "Scene & Assets",
            "3D Scene Viewport",
            "Object / Material Inspector",
            "Shader & Render Tools",
            vec![
                workbench_tool(
                    "frame",
                    "Frame Selection",
                    "view",
                    "Fit the active mesh or scene in the viewport.",
                    "Open3D",
                ),
                workbench_tool(
                    "validate-mesh",
                    "Validate Mesh",
                    "inspect",
                    "Check topology, normals, degenerate faces, and bounds.",
                    "Blender Python API",
                ),
                workbench_tool(
                    "render",
                    "Render Scene",
                    "execute",
                    "Render the selected scene with its actual materials and lights.",
                    "Blender Python API",
                ),
            ],
        ),
        "cad" => (
            "cad",
            "Models & Features",
            "Parametric 3D Viewport",
            "Feature / Constraint Inspector",
            "CAD Operations",
            vec![
                workbench_tool(
                    "measure",
                    "Measure",
                    "inspect",
                    "Measure real model bounds, distances, and selected geometry.",
                    "Open CASCADE",
                ),
                workbench_tool(
                    "recompute",
                    "Recompute Model",
                    "execute",
                    "Rebuild the feature tree after parameter changes.",
                    "FreeCAD",
                ),
                workbench_tool(
                    "export",
                    "Export Geometry",
                    "execute",
                    "Export the selected body using the configured CAD backend.",
                    "CadQuery",
                ),
            ],
        ),
        "robotics" => (
            "robotics",
            "Robots, Bags & Maps",
            "Robot Scene & Trajectory",
            "Joint / Frame Inspector",
            "ROS & Simulation",
            vec![
                workbench_tool(
                    "inspect-tf",
                    "Inspect TF",
                    "inspect",
                    "Inspect coordinate frames and parent-child transforms.",
                    "ROS 2",
                ),
                workbench_tool(
                    "play-bag",
                    "Play Bag",
                    "execute",
                    "Replay the selected bag or MCAP data source.",
                    "ROS 2",
                ),
                workbench_tool(
                    "plan-motion",
                    "Plan Motion",
                    "execute",
                    "Plan a trajectory for the selected robot state.",
                    "MoveIt",
                ),
            ],
        ),
        "computer-networks" => (
            "packet",
            "Captures & Flows",
            "Topology / Packet Analysis",
            "Packet & Protocol Inspector",
            "Capture & Diagnostics",
            vec![
                workbench_tool(
                    "decode",
                    "Decode Packets",
                    "inspect",
                    "Decode real packet layers and protocol fields.",
                    "Wireshark",
                ),
                workbench_tool(
                    "capture",
                    "Start Capture",
                    "execute",
                    "Capture packets from a selected runtime interface.",
                    "libpcap",
                ),
                workbench_tool(
                    "simulate",
                    "Run Topology",
                    "execute",
                    "Execute the selected emulated topology.",
                    "Mininet",
                ),
            ],
        ),
        "operating-systems" => (
            "trace",
            "Traces, Processes & Dumps",
            "Kernel / Process Analysis",
            "Process & Memory Inspector",
            "Trace & Debug Tools",
            vec![
                workbench_tool(
                    "record-trace",
                    "Record Trace",
                    "execute",
                    "Record a real scheduler and process trace.",
                    "Windows ETW",
                ),
                workbench_tool(
                    "inspect-process",
                    "Inspect Process",
                    "inspect",
                    "Inspect process, threads, handles, and memory regions.",
                    "Windows ETW",
                ),
                workbench_tool(
                    "trace-syscalls",
                    "Trace Syscalls",
                    "execute",
                    "Trace file and system calls for the selected process.",
                    "strace",
                ),
            ],
        ),
        "compiler" => (
            "compiler",
            "Sources, AST & IR",
            "Compiler Pipeline",
            "Symbol / Block Inspector",
            "Passes & Diagnostics",
            vec![
                workbench_tool(
                    "parse",
                    "Build AST",
                    "execute",
                    "Parse the selected source with the configured frontend.",
                    "Clang",
                ),
                workbench_tool(
                    "emit-ir",
                    "Emit IR",
                    "execute",
                    "Emit LLVM IR or MLIR for the selected source.",
                    "LLVM",
                ),
                workbench_tool(
                    "run-passes",
                    "Run Passes",
                    "execute",
                    "Run and compare configured optimization passes.",
                    "MLIR",
                ),
            ],
        ),
        "database" => (
            "database",
            "Connections, Schemas & Data",
            "Query & Schema Workbench",
            "Column / Plan Inspector",
            "Query Results",
            vec![
                workbench_tool(
                    "run-query",
                    "Run Query",
                    "execute",
                    "Execute the current query against a configured connection.",
                    "DuckDB",
                ),
                workbench_tool(
                    "explain",
                    "Explain Plan",
                    "inspect",
                    "Generate the actual query plan without inventing operators.",
                    "PostgreSQL",
                ),
                workbench_tool(
                    "profile",
                    "Profile Query",
                    "execute",
                    "Collect timing and operator metrics for the current query.",
                    "DataFusion",
                ),
            ],
        ),
        "software-engineering" => (
            "repository",
            "Repository & Modules",
            "Architecture & Change Workbench",
            "Module / Change Inspector",
            "Build, Test & Review",
            vec![
                workbench_tool(
                    "build",
                    "Build",
                    "execute",
                    "Run the repository's detected build system.",
                    "Cargo",
                ),
                workbench_tool(
                    "test",
                    "Test",
                    "execute",
                    "Run the detected test targets and retain results.",
                    "npm",
                ),
                workbench_tool(
                    "dependencies",
                    "Audit Dependencies",
                    "inspect",
                    "Inspect declared and resolved dependency data.",
                    "Cargo",
                ),
            ],
        ),
        "program-analysis" => (
            "analysis",
            "Targets, Facts & Traces",
            "Program Analysis Workbench",
            "Fact / Finding Inspector",
            "Analysis Queries",
            vec![
                workbench_tool(
                    "callgraph",
                    "Build Call Graph",
                    "execute",
                    "Generate calls from the selected real analysis target.",
                    "LLVM",
                ),
                workbench_tool(
                    "dataflow",
                    "Run Data Flow",
                    "execute",
                    "Run data-flow analysis for the selected target.",
                    "CodeQL",
                ),
                workbench_tool(
                    "taint",
                    "Run Taint Query",
                    "execute",
                    "Trace selected sources to sinks with real analysis facts.",
                    "Joern",
                ),
            ],
        ),
        "cyber-security" => (
            "security",
            "Targets, Scans & Findings",
            "Security Analysis Workbench",
            "Finding / Evidence Inspector",
            "Scan & Triage",
            vec![
                workbench_tool(
                    "scan",
                    "Run Scan",
                    "execute",
                    "Run the configured static or dynamic scanner.",
                    "Semgrep",
                ),
                workbench_tool(
                    "query",
                    "Run Security Query",
                    "execute",
                    "Execute security queries against the selected target.",
                    "CodeQL",
                ),
                workbench_tool(
                    "triage",
                    "Triage Finding",
                    "inspect",
                    "Inspect evidence, path, severity, and remediation state.",
                    "OWASP ZAP",
                ),
            ],
        ),
        "hpc" => (
            "hpc",
            "Profiles, Kernels & Ranks",
            "Parallel Performance Workbench",
            "Kernel / Rank Inspector",
            "Profiler & Launch Tools",
            vec![
                workbench_tool(
                    "profile",
                    "Profile GPU",
                    "execute",
                    "Capture a real GPU execution timeline.",
                    "Nsight Systems",
                ),
                workbench_tool(
                    "kernel",
                    "Inspect Kernel",
                    "inspect",
                    "Inspect launch geometry, occupancy, and memory access.",
                    "Nsight Compute",
                ),
                workbench_tool(
                    "mpi",
                    "Run MPI",
                    "execute",
                    "Launch the selected parallel workload across configured ranks.",
                    "MPI",
                ),
            ],
        ),
        "distributed-systems" => (
            "distributed",
            "Services, Traces & State",
            "Distributed Systems Workbench",
            "Span / Replica Inspector",
            "Operations & Reliability",
            vec![
                workbench_tool(
                    "collect-trace",
                    "Collect Trace",
                    "execute",
                    "Collect real distributed spans from configured telemetry.",
                    "OpenTelemetry",
                ),
                workbench_tool(
                    "inspect-rpc",
                    "Inspect RPC",
                    "inspect",
                    "Inspect RPC metadata, latency, retries, and status.",
                    "gRPC",
                ),
                workbench_tool(
                    "inspect-cluster",
                    "Inspect Cluster",
                    "inspect",
                    "Read current workload, replica, and service state.",
                    "Kubernetes",
                ),
            ],
        ),
        "scientific-computing" => (
            "scientific",
            "Arrays, Simulations & Meshes",
            "Numerical Computing Workbench",
            "Variable / Cell Inspector",
            "Run & Convergence",
            vec![
                workbench_tool(
                    "run-simulation",
                    "Run Simulation",
                    "execute",
                    "Run the selected numerical model with workspace inputs.",
                    "SciPy",
                ),
                workbench_tool(
                    "inspect-array",
                    "Inspect Array",
                    "inspect",
                    "Inspect shape, dtype, ranges, and slices.",
                    "NumPy",
                ),
                workbench_tool(
                    "export-vtk",
                    "Export VTK",
                    "execute",
                    "Export real fields and meshes for downstream analysis.",
                    "VTK",
                ),
            ],
        ),
        _ => (
            "research",
            "Research Assets",
            "Research Workbench",
            "Inspector",
            "Tools & Results",
            Vec::new(),
        ),
    };
    DomainWorkbenchDescriptor {
        layout: layout.to_string(),
        explorer_label: explorer.to_string(),
        primary_label: primary.to_string(),
        inspector_label: inspector.to_string(),
        bottom_panel_label: bottom.to_string(),
        tools,
        workflow: workflow_for(domain),
        intents: intents_for(domain),
    }
}

#[allow(clippy::too_many_arguments)]
fn plugin(
    id: &str,
    label: &str,
    description: &str,
    capabilities: &[&str],
    file_types: &[&str],
    visualizations: Vec<DomainVisualizationDescriptor>,
    agents: &[&str],
    sdk_adapters: &[&str],
    keywords: &[&str],
    content_markers: &[&str],
) -> DeclarativeDomainPlugin {
    let descriptor = DomainPluginDescriptor {
        metadata: DomainMetadata {
            id: id.to_string(),
            label: label.to_string(),
            description: description.to_string(),
            version: "1.0.0".to_string(),
            category: "computer-science".to_string(),
        },
        capabilities: capabilities
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        supported_file_types: file_types
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        supported_visualizations: visualizations,
        supported_agents: agents.iter().map(|value| (*value).to_string()).collect(),
        context_provider: provider(id, "context"),
        preview_provider: provider(id, "preview"),
        execution_provider: provider(id, "execution"),
        data_provider: provider(id, "data"),
        visualization_provider: provider(id, "visualization"),
        render_provider: provider(id, "render"),
        lifecycle: DomainLifecycleDescriptor {
            states: vec![
                "registered".to_string(),
                "activating".to_string(),
                "active".to_string(),
                "suspended".to_string(),
                "disposed".to_string(),
            ],
            supports_hot_reload: true,
            supports_workspace_sync: true,
        },
        sdk_adapters: sdk_adapters
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        plugin_api_version: RESEARCH_DOMAIN_PLUGIN_API_VERSION.to_string(),
        workbench: workbench_for(id),
    };
    DeclarativeDomainPlugin {
        descriptor,
        match_keywords: keywords
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect(),
        content_markers: content_markers
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect(),
    }
}

fn shared_renderers() -> Vec<VisualizationRendererDescriptor> {
    [
        ("2d", "2D Viewer", "2d"),
        ("3d", "3D Viewer", "3d"),
        ("graph", "Graph Viewer", "2d"),
        ("timeline", "Timeline Viewer", "2d"),
        ("pipeline", "Pipeline Viewer", "2d"),
        ("tensor", "Tensor Viewer", "nd"),
        ("mesh", "Mesh Viewer", "3d"),
        ("volume", "Volume Viewer", "3d"),
        ("point-cloud", "Point Cloud Viewer", "3d"),
        ("topology", "Topology Viewer", "2d"),
        ("heatmap", "Heatmap Viewer", "2d"),
        ("trace", "Trace Viewer", "2d"),
        ("tree", "Tree Viewer", "2d"),
        ("table", "Table Viewer", "2d"),
        ("chart", "Chart Viewer", "2d"),
        ("markdown", "Markdown Viewer", "2d"),
        ("code", "Code Viewer", "2d"),
        ("equation", "Equation Viewer", "2d"),
        ("whiteboard", "Whiteboard Viewer", "2d"),
    ]
    .into_iter()
    .map(|(id, label, dimensions)| VisualizationRendererDescriptor {
        id: id.to_string(),
        label: label.to_string(),
        dimensions: dimensions.to_string(),
        supports_zoom: true,
        supports_pan: true,
        supports_animation: matches!(
            id,
            "2d" | "3d"
                | "graph"
                | "timeline"
                | "pipeline"
                | "mesh"
                | "point-cloud"
                | "topology"
                | "trace"
                | "chart"
        ),
    })
    .collect()
}

fn builtin_plugins() -> Vec<DeclarativeDomainPlugin> {
    vec![
        plugin(
            "ai-ml",
            "AI / Machine Learning",
            "Models, datasets, training runs, checkpoints, tensors, evaluation, and inference.",
            &["Dataset", "Training", "Checkpoint", "Tensor", "Model", "Evaluation", "Inference"],
            &["py", "ipynb", "onnx", "pt", "pth", "ckpt", "safetensors", "tflite", "h5", "npy", "npz", "csv", "json", "jsonl"],
            vec![
                visualization("model-graph", "Model Graph", "graph", &["py", "ipynb", "onnx", "json", "safetensors"]),
                visualization("loss-curve", "Loss Curve", "chart", &["csv", "json", "jsonl"]),
                visualization("attention-map", "Attention Map", "heatmap", &["npy", "npz", "json"]),
                visualization("embedding", "Embedding", "point-cloud", &["npy", "npz", "json"]),
                visualization("inference-pipeline", "Inference Pipeline", "pipeline", &["py", "ipynb", "onnx", "json"]),
                visualization("tensor", "Tensor", "tensor", &["npy", "npz", "safetensors", "pt", "pth"]),
            ],
            &["research", "training", "evaluation", "inference"],
            &["PyTorch", "TensorFlow", "JAX", "ONNX Runtime", "Hugging Face Transformers"],
            &["model", "training", "checkpoint", "tensor", "transformer", "cnn", "embedding", "attention"],
            &["torch", "tensorflow", "jax", "transformers", "onnx", "safetensors", "loss", "attention"],
        ),
        plugin(
            "computer-vision",
            "Computer Vision",
            "Images, video, feature maps, detections, segmentation, calibration, and reconstruction.",
            &["Image", "Video", "Detection", "Segmentation", "Calibration", "Reconstruction"],
            &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "mp4", "avi", "mov", "onnx", "npy", "npz", "ply", "obj", "py", "json"],
            vec![
                visualization("image-overlay", "Bounding Box", "2d", &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp", "json"]),
                visualization("segmentation", "Segmentation", "heatmap", &["png", "jpg", "jpeg", "npy", "npz", "json"]),
                visualization("feature-map", "Feature Map", "heatmap", &["npy", "npz", "json"]),
                visualization("reconstruction", "3D Reconstruction", "point-cloud", &["ply", "obj", "npy", "npz", "json"]),
                visualization("vision-model", "Vision Model", "graph", &["onnx", "py", "json"]),
            ],
            &["vision", "annotation", "evaluation"],
            &["OpenCV", "PyTorch Vision", "ONNX Runtime", "Open3D"],
            &["vision", "image", "detection", "segmentation", "camera", "opencv", "reconstruction"],
            &["cv2", "opencv", "torchvision", "yolo", "segmentation", "bounding box", "camera matrix"],
        ),
        plugin(
            "nlp",
            "Natural Language Processing",
            "Corpora, tokenization, linguistic structures, language models, embeddings, and evaluation.",
            &["Corpus", "Tokenizer", "Language Model", "Embedding", "Evaluation", "Generation"],
            &["txt", "md", "json", "jsonl", "conll", "conllu", "spacy", "model", "safetensors", "onnx", "py"],
            vec![
                visualization("attention-flow", "Attention Flow", "heatmap", &["json", "npy", "npz", "py"]),
                visualization("token-flow", "Token Flow", "pipeline", &["txt", "json", "jsonl", "conll", "conllu"]),
                visualization("dependency-tree", "Dependency Tree", "tree", &["conll", "conllu", "json"]),
                visualization("embedding", "Embedding", "point-cloud", &["json", "npy", "npz", "safetensors"]),
                visualization("language-model", "Language Model", "graph", &["onnx", "json", "py"]),
            ],
            &["nlp", "corpus", "evaluation"],
            &["Hugging Face Transformers", "spaCy", "NLTK", "SentenceTransformers"],
            &["nlp", "token", "corpus", "language model", "embedding", "attention", "transformer"],
            &["tokenizer", "spacy", "nltk", "transformers", "sentencepiece", "language_model"],
        ),
        plugin(
            "computer-graphics",
            "Computer Graphics",
            "Meshes, scenes, materials, shaders, lighting, topology, and rendering assets.",
            &["Mesh", "Scene", "Material", "Shader", "Lighting", "Topology"],
            &["obj", "fbx", "gltf", "glb", "ply", "mtl", "vert", "frag", "glsl", "hlsl", "wgsl", "blend"],
            vec![
                visualization("mesh-viewer", "Mesh Viewer", "mesh", &["obj", "fbx", "gltf", "glb", "ply", "blend"]),
                visualization("material", "Material", "3d", &["mtl", "gltf", "glb", "blend"]),
                visualization("lighting", "Lighting", "3d", &["gltf", "glb", "blend"]),
                visualization("wireframe", "Wireframe", "topology", &["obj", "fbx", "gltf", "glb", "ply"]),
                visualization("shader", "Shader", "code", &["vert", "frag", "glsl", "hlsl", "wgsl"]),
            ],
            &["graphics", "render", "shader"],
            &["OpenGL", "Vulkan", "DirectX", "WebGPU", "Blender Python API", "Open3D"],
            &["mesh", "shader", "render", "material", "lighting", "gltf", "graphics"],
            &["opengl", "vulkan", "wgpu", "three.js", "shader", "mesh", "material"],
        ),
        plugin(
            "cad",
            "CAD & Geometric Modeling",
            "Parametric solids, sketches, constraints, assemblies, feature trees, and manufacturing geometry.",
            &["Solid", "Sketch", "Constraint", "Feature", "Assembly", "Text to CAD", "Constraint Editing", "Feature Editing"],
            &["step", "stp", "iges", "igs", "stl", "dxf", "dwg", "fcstd", "scad", "brep", "obj"],
            vec![
                visualization("parametric-model", "Parametric Model", "3d", &["step", "stp", "iges", "igs", "stl", "fcstd", "scad", "brep", "obj"]),
                visualization("feature-tree", "Feature Tree", "tree", &["step", "stp", "fcstd", "scad", "brep"]),
                visualization("constraint-graph", "Constraint Graph", "graph", &["fcstd", "scad", "dxf"]),
                visualization("exploded-view", "Exploded View", "3d", &["step", "stp", "fcstd"]),
            ],
            &["cad", "geometry", "manufacturing"],
            &["Open CASCADE", "FreeCAD", "CadQuery", "OpenSCAD"],
            &["cad", "gearbox", "feature tree", "constraint", "sketch", "solid", "assembly"],
            &["cadquery", "freecad", "openscad", "occ", "constraint", "sketch"],
        ),
        plugin(
            "robotics",
            "Robotics",
            "Robot models, joints, trajectories, coordinate frames, sensor logs, maps, and control.",
            &["Robot Model", "Trajectory", "SLAM", "Joint State", "Coordinate Frame", "Control"],
            &["urdf", "xacro", "sdf", "world", "bag", "db3", "mcap", "pgm", "png", "yaml", "yml", "csv", "json", "py", "cpp"],
            vec![
                visualization("robot-model", "3D Robot", "3d", &["urdf", "xacro", "sdf"]),
                visualization("trajectory", "Trajectory", "3d", &["bag", "db3", "mcap", "csv", "json"]),
                visualization("slam-map", "SLAM Map", "2d", &["pgm", "png", "yaml", "bag", "db3"]),
                visualization("joint-state", "Joint State", "timeline", &["bag", "db3", "mcap", "csv", "json"]),
                visualization("coordinate-frame", "Coordinate Frame", "3d", &["urdf", "xacro", "sdf", "json"]),
            ],
            &["robotics", "planning", "control"],
            &["ROS 2", "Gazebo", "MoveIt", "MuJoCo", "Open3D"],
            &["robot", "trajectory", "slam", "joint", "urdf", "ros", "coordinate frame"],
            &["rclpy", "rclcpp", "sensor_msgs", "geometry_msgs", "moveit", "mujoco", "urdf"],
        ),
        plugin(
            "computer-networks",
            "Computer Networks",
            "Topologies, packets, protocols, flows, routes, latency, and distributed communication.",
            &["Topology", "Packet", "Flow", "PCAP", "Protocol", "Routing", "Bandwidth", "Latency"],
            &["pcap", "pcapng", "har", "log", "json", "jsonl", "csv", "netxml", "graphml", "dot"],
            vec![
                visualization("network-graph", "Network Graph", "graph", &["pcap", "pcapng", "har", "log", "json", "jsonl", "netxml", "graphml", "dot"]),
                visualization("packet-animation", "Packet Animation", "timeline", &["pcap", "pcapng", "har", "log", "jsonl"]),
                visualization("tcp-state", "TCP State", "trace", &["pcap", "pcapng", "log", "jsonl"]),
                visualization("bandwidth", "Bandwidth", "chart", &["pcap", "pcapng", "csv", "jsonl"]),
                visualization("latency", "Latency", "chart", &["pcap", "pcapng", "har", "csv", "jsonl"]),
            ],
            &["network", "protocol", "observability"],
            &["Wireshark", "libpcap", "Scapy", "Mininet", "ns-3"],
            &["network", "packet", "pcap", "routing", "tcp", "latency", "bandwidth", "topology"],
            &["scapy", "socket", "tcp", "udp", "pcap", "wireshark", "routing"],
        ),
        plugin(
            "operating-systems",
            "Operating Systems",
            "Processes, threads, scheduling, virtual memory, filesystems, and kernel traces.",
            &["Process", "Thread", "Scheduler", "Memory", "File System", "Kernel Trace"],
            &["etl", "perf", "trace", "log", "dmp", "core", "csv", "json", "jsonl", "strace"],
            vec![
                visualization("cpu-timeline", "CPU Timeline", "timeline", &["etl", "perf", "trace", "log", "csv", "jsonl"]),
                visualization("thread-timeline", "Thread Timeline", "timeline", &["etl", "perf", "trace", "log", "csv", "jsonl"]),
                visualization("memory-layout", "Memory Layout", "topology", &["dmp", "core", "json", "csv"]),
                visualization("process-tree", "Process Tree", "tree", &["etl", "perf", "trace", "log", "json", "jsonl"]),
                visualization("file-system-tree", "File System Tree", "tree", &["json", "log", "strace"]),
            ],
            &["systems", "kernel", "performance"],
            &["Windows ETW", "Linux perf", "eBPF", "strace"],
            &["operating system", "kernel", "process", "thread", "scheduler", "memory layout", "filesystem"],
            &["process", "thread", "scheduler", "virtual memory", "ebpf", "perf_event", "kernel"],
        ),
        plugin(
            "compiler",
            "Compiler",
            "Source syntax, ASTs, control flow, SSA, IR, optimization, and code generation.",
            &["AST", "CFG", "LLVM IR", "SSA", "Optimization", "Code Generation"],
            &["ll", "bc", "mlir", "wat", "wasm", "ast", "cfg", "dot", "graphml", "c", "cpp", "rs", "go", "java"],
            vec![
                visualization("syntax-tree", "Syntax Tree", "tree", &["ast", "json", "c", "cpp", "rs", "go", "java"]),
                visualization("control-flow", "Control Flow Graph", "graph", &["cfg", "dot", "graphml", "ll", "mlir", "c", "cpp", "rs"]),
                visualization("optimization", "Optimization Graph", "graph", &["ll", "bc", "mlir", "json", "dot"]),
                visualization("ir-viewer", "IR Viewer", "code", &["ll", "mlir", "wat"]),
                visualization("ssa", "SSA", "graph", &["ll", "mlir", "cfg", "dot"]),
            ],
            &["compiler", "optimization", "analysis"],
            &["LLVM", "MLIR", "Clang", "GCC", "rustc"],
            &["compiler", "ast", "cfg", "llvm", "ssa", "ir", "optimization"],
            &["llvm", "mlir", "basicblock", "ast", "control flow", "ssa", "codegen"],
        ),
        plugin(
            "database",
            "Database",
            "Schemas, tables, queries, execution plans, transactions, indexes, and data lineage.",
            &["Schema", "Table", "Query", "Execution Plan", "Transaction", "Index", "Lineage"],
            &["sql", "db", "sqlite", "sqlite3", "duckdb", "parquet", "arrow", "csv", "json", "jsonl", "plan"],
            vec![
                visualization("schema", "Schema Graph", "graph", &["sql", "db", "sqlite", "sqlite3", "duckdb", "json"]),
                visualization("query-plan", "Query Plan", "tree", &["sql", "plan", "json"]),
                visualization("table", "Table Viewer", "table", &["db", "sqlite", "sqlite3", "duckdb", "parquet", "arrow", "csv", "json", "jsonl"]),
                visualization("lineage", "Data Lineage", "graph", &["sql", "json", "dot", "graphml"]),
            ],
            &["database", "query", "data"],
            &["SQLite", "DuckDB", "PostgreSQL", "Apache Arrow", "DataFusion"],
            &["database", "schema", "sql", "query plan", "table", "index", "transaction"],
            &["select ", "create table", "sqlite", "duckdb", "postgres", "datafusion", "query plan"],
        ),
        plugin(
            "software-engineering",
            "Software Engineering",
            "Repositories, modules, builds, tests, dependencies, architecture, and change history.",
            &["Repository", "Module", "Build", "Test", "Dependency", "Architecture", "Change"],
            &["rs", "py", "js", "ts", "tsx", "jsx", "go", "java", "kt", "c", "cpp", "h", "hpp", "cs", "toml", "yaml", "yml", "json", "xml"],
            vec![
                visualization("architecture", "Architecture Graph", "graph", &[]),
                visualization("dependency", "Dependency Graph", "graph", &[]),
                visualization("pipeline", "Build Pipeline", "pipeline", &["yaml", "yml", "toml", "json", "xml"]),
                visualization("code", "Code Viewer", "code", &[]),
            ],
            &["coding", "testing", "review", "architecture"],
            &["Cargo", "npm", "Maven", "Gradle", ".NET", "CMake"],
            &["software", "repository", "dependency", "build", "test", "architecture", "module"],
            &["package", "dependency", "module", "test", "build", "workspace"],
        ),
        plugin(
            "program-analysis",
            "Program Analysis",
            "Static and dynamic program evidence including calls, dependencies, data flow, taint, and traces.",
            &["Call Graph", "Dependency Graph", "DFG", "Taint Flow", "Execution Trace"],
            &["sarif", "dot", "graphml", "gexf", "cfg", "dfg", "trace", "log", "json", "jsonl", "rs", "py", "c", "cpp", "java", "js", "ts"],
            vec![
                visualization("call-graph", "Call Graph", "graph", &[]),
                visualization("dependency-graph", "Dependency Graph", "graph", &[]),
                visualization("data-flow", "DFG", "graph", &["dfg", "dot", "graphml", "json"]),
                visualization("taint-flow", "Taint Flow", "trace", &["sarif", "json", "jsonl", "dot"]),
                visualization("execution-trace", "Execution Trace", "trace", &["trace", "log", "jsonl", "json"]),
            ],
            &["analysis", "security", "verification"],
            &["LLVM", "CodeQL", "Semgrep", "Joern", "Valgrind"],
            &["program analysis", "call graph", "data flow", "taint", "execution trace", "dependency graph"],
            &["callgraph", "dataflow", "taint", "cfg", "static analysis", "dynamic analysis"],
        ),
        plugin(
            "cyber-security",
            "Cyber Security",
            "Findings, attack paths, vulnerabilities, taint evidence, packet data, and security telemetry.",
            &["Finding", "Vulnerability", "Attack Path", "Taint", "Packet", "Security Trace"],
            &["sarif", "yara", "nessus", "pcap", "pcapng", "har", "log", "json", "jsonl", "csv"],
            vec![
                visualization("attack-graph", "Attack Graph", "graph", &["sarif", "json", "jsonl", "yara", "nessus"]),
                visualization("taint-flow", "Taint Flow", "trace", &["sarif", "json", "jsonl"]),
                visualization("packet-flow", "Packet Flow", "timeline", &["pcap", "pcapng", "har", "log"]),
                visualization("findings", "Findings", "table", &["sarif", "nessus", "json", "csv"]),
            ],
            &["security", "audit", "threat-model"],
            &["Semgrep", "CodeQL", "YARA", "Wireshark", "OWASP ZAP"],
            &["security", "vulnerability", "attack", "taint", "sarif", "yara", "threat"],
            &["vulnerability", "cve", "taint", "security", "semgrep", "codeql", "yara"],
        ),
        plugin(
            "hpc",
            "High Performance Computing",
            "GPU kernels, MPI communication, occupancy, memory bandwidth, and parallel timelines.",
            &["GPU Kernel", "CUDA", "MPI", "Memory Bandwidth", "Occupancy", "Parallel Trace"],
            &["cu", "cuh", "hip", "cl", "ptx", "nsys-rep", "ncu-rep", "otf2", "trace", "csv", "json", "jsonl"],
            vec![
                visualization("gpu-timeline", "GPU Timeline", "timeline", &["nsys-rep", "ncu-rep", "trace", "csv", "jsonl"]),
                visualization("cuda-kernel", "CUDA Kernel", "code", &["cu", "cuh", "ptx"]),
                visualization("mpi-communication", "MPI Communication", "graph", &["otf2", "trace", "json", "jsonl"]),
                visualization("memory-bandwidth", "Memory Bandwidth", "chart", &["ncu-rep", "csv", "json", "jsonl"]),
                visualization("occupancy", "Occupancy", "heatmap", &["ncu-rep", "csv", "json"]),
            ],
            &["hpc", "profiling", "parallel"],
            &["CUDA", "ROCm", "OpenCL", "MPI", "Nsight Systems", "Nsight Compute"],
            &["hpc", "cuda", "gpu kernel", "mpi", "occupancy", "memory bandwidth", "parallel"],
            &["__global__", "cuda", "hip", "opencl", "mpi_", "occupancy", "kernel"],
        ),
        plugin(
            "distributed-systems",
            "Distributed Systems",
            "Services, RPCs, traces, consensus, queues, replicas, and distributed state transitions.",
            &["Service", "RPC", "Trace", "Consensus", "Queue", "Replica", "Distributed State"],
            &["proto", "har", "trace", "log", "json", "jsonl", "yaml", "yml", "dot", "graphml"],
            vec![
                visualization("service-topology", "Service Topology", "topology", &[]),
                visualization("request-trace", "Request Trace", "trace", &["har", "trace", "log", "json", "jsonl"]),
                visualization("communication", "Distributed Communication", "graph", &["proto", "trace", "json", "jsonl", "dot", "graphml"]),
                visualization("state-lifecycle", "State Lifecycle", "timeline", &["trace", "log", "json", "jsonl"]),
            ],
            &["distributed", "observability", "reliability"],
            &["OpenTelemetry", "gRPC", "Jaeger", "Kubernetes", "etcd"],
            &["distributed", "service", "rpc", "consensus", "replica", "trace", "microservice"],
            &["grpc", "opentelemetry", "jaeger", "kubernetes", "raft", "consensus", "replica"],
        ),
        plugin(
            "scientific-computing",
            "Scientific Computing",
            "Numerical models, arrays, simulations, equations, fields, meshes, and measured results.",
            &["Array", "Simulation", "Equation", "Field", "Mesh", "Experiment", "Numerical Method"],
            &["npy", "npz", "mat", "h5", "hdf5", "nc", "vtk", "vtu", "csv", "json", "jsonl", "m", "jl", "py", "ipynb", "md"],
            vec![
                visualization("array", "Array", "tensor", &["npy", "npz", "mat", "h5", "hdf5", "nc"]),
                visualization("field", "Field", "heatmap", &["npy", "npz", "mat", "h5", "hdf5", "nc", "vtk", "vtu"]),
                visualization("mesh", "Scientific Mesh", "mesh", &["vtk", "vtu"]),
                visualization("result-chart", "Result Chart", "chart", &["csv", "json", "jsonl"]),
                visualization("equation", "Equation", "equation", &["m", "jl", "py", "ipynb", "md"]),
            ],
            &["scientific", "simulation", "numerical"],
            &["NumPy", "SciPy", "Julia", "MATLAB", "PETSc", "VTK"],
            &["scientific", "simulation", "numerical", "equation", "matrix", "pde", "ode", "field"],
            &["numpy", "scipy", "differentialequations", "petsc", "vtk", "finite element", "simulation"],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn builtin_registry_exposes_all_domain_contracts() {
        let registry = ResearchDomainRegistry::default();
        let root = tempdir().unwrap();
        let runtime = json!({});
        let context = DomainProviderContext {
            workspace_root: root.path(),
            query: Some("transformer training"),
            runtime: &runtime,
        };
        let catalog = registry.catalog(&context, context.query).unwrap();
        assert_eq!(catalog.plugins.len(), 16);
        assert!(catalog.plugins.iter().all(|plugin| {
            !plugin.capabilities.is_empty()
                && !plugin.supported_visualizations.is_empty()
                && !plugin.context_provider.id.is_empty()
                && !plugin.execution_provider.id.is_empty()
                && plugin.workbench.tools.len() == 3
                && plugin.workbench.workflow.len() == 4
                && plugin.workbench.intents.len() == 3
                && plugin.workbench.intents.iter().all(|intent| {
                    !intent.agent.is_empty()
                        && !intent.input_contract.is_empty()
                        && !intent.expected_outputs.is_empty()
                        && !intent.workflow_stages.is_empty()
                        && !intent.gate.is_empty()
                })
                && plugin
                    .workbench
                    .workflow
                    .iter()
                    .all(|stage| !stage.agent.is_empty() && !stage.gate.is_empty())
        }));
        assert_eq!(catalog.active_domain.unwrap().domain_id, "ai-ml");
        assert!(catalog
            .renderers
            .iter()
            .any(|renderer| renderer.id == "mesh"));
    }

    #[test]
    fn assets_and_visualizations_are_derived_from_real_workspace_files() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("training_metrics.csv"),
            "step,loss,accuracy\n1,0.8,0.6\n2,0.4,0.85\n",
        )
        .unwrap();
        let runtime = json!({});
        let context = DomainProviderContext {
            workspace_root: root.path(),
            query: Some("training loss"),
            runtime: &runtime,
        };
        let registry = ResearchDomainRegistry::default();
        let workspace = registry.workspace(&context, "ai-ml").unwrap();
        let asset = workspace
            .assets
            .iter()
            .find(|asset| asset.path == "training_metrics.csv")
            .unwrap();
        let document = registry
            .visualization(&context, "ai-ml", &asset.id, Some("loss-curve"))
            .unwrap();
        // The domain adapter treats step/epoch as coordinates, not measured series.
        assert_eq!(document.series.len(), 2);
        assert_eq!(document.series[1].points[1].value, 0.4);
        assert_eq!(document.metadata["asset_revision"], asset.content_revision);
    }

    #[test]
    fn domain_action_result_is_isolated_and_visualizable() {
        let root = tempdir().unwrap();
        let directory = root
            .path()
            .join(".atlas")
            .join("domain-actions")
            .join("nlp")
            .join("token-document")
            .join("run-1");
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("result.json"),
            serde_json::to_vec_pretty(&json!({
                "schema_version": "atlas.domain-action-result.v1",
                "action": "tokenize",
                "status": "completed",
                "result": {
                    "sdk": "spacy",
                    "language": "en",
                    "tokens": [
                        {"text":"Atlas","start":0,"end":5},
                        {"text":"research","start":6,"end":14}
                    ]
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let runtime = json!({});
        let context = DomainProviderContext {
            workspace_root: root.path(),
            query: None,
            runtime: &runtime,
        };
        let registry = ResearchDomainRegistry::default();
        let workspace = registry.workspace(&context, "nlp").unwrap();
        let result = workspace
            .assets
            .iter()
            .find(|asset| asset.path.ends_with("/result.json"))
            .unwrap();
        assert_eq!(result.visualizations[0].id, "action-result");
        let document = registry
            .visualization(&context, "nlp", &result.id, Some("action-result"))
            .unwrap();
        assert!(document.nodes.iter().any(|node| node.label == "Atlas"));
        assert_eq!(document.metadata["domain_action"]["id"], "tokenize");

        let other = registry
            .workspace(&context, "computer-vision")
            .unwrap();
        assert!(!other
            .assets
            .iter()
            .any(|asset| asset.path.contains("domain-actions/nlp/")));
    }
}
