use super::{PluginContributions, PluginManifest, PluginState};
use crate::atlas_core::{AtlasCore, AtlasEvent, EventBus, EventListener};
use crate::research_intelligence::RuntimeAdapter;
use anyhow::{anyhow, Result};
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct PluginContext {
    pub core: Arc<AtlasCore>,
}

pub trait Plugin: EventListener + Send + Sync {
    fn manifest(&self) -> PluginManifest;
    fn install(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }
    fn enable(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }
    fn disable(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }
    fn unload(&self, _context: &PluginContext) -> Result<()> {
        Ok(())
    }
    fn runtime_adapters(&self) -> Vec<Arc<dyn RuntimeAdapter>> {
        Vec::new()
    }
    fn contributions(&self) -> PluginContributions {
        PluginContributions::default()
    }
}

struct PluginEntry {
    plugin: Arc<dyn Plugin>,
    state: PluginState,
}

pub struct PluginRegistry {
    plugins: RwLock<BTreeMap<String, PluginEntry>>,
    events: Arc<EventBus>,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PluginRegistry")
            .field(
                "plugin_count",
                &self.plugins.read().map(|value| value.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl PluginRegistry {
    pub fn new(events: Arc<EventBus>) -> Self {
        Self {
            plugins: RwLock::new(BTreeMap::new()),
            events,
        }
    }

    pub fn install(&self, plugin: Arc<dyn Plugin>, context: &PluginContext) -> Result<()> {
        let manifest = plugin.manifest();
        validate_manifest(&manifest)?;
        plugin.install(context)?;
        self.events.subscribe(plugin.clone());
        self.plugins
            .write()
            .map_err(|_| anyhow!("plugin registry lock poisoned"))?
            .insert(
                manifest.name,
                PluginEntry {
                    plugin,
                    state: PluginState::Installed,
                },
            );
        Ok(())
    }

    pub fn enable(&self, name: &str, context: &PluginContext) -> Result<()> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|_| anyhow!("plugin registry lock poisoned"))?;
        let entry = plugins
            .get_mut(name)
            .ok_or_else(|| anyhow!("unknown Atlas plugin"))?;
        entry.plugin.enable(context)?;
        entry.state = PluginState::Enabled;
        Ok(())
    }

    pub fn disable(&self, name: &str, context: &PluginContext) -> Result<()> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|_| anyhow!("plugin registry lock poisoned"))?;
        let entry = plugins
            .get_mut(name)
            .ok_or_else(|| anyhow!("unknown Atlas plugin"))?;
        entry.plugin.disable(context)?;
        entry.state = PluginState::Disabled;
        Ok(())
    }

    pub fn hot_reload(&self, name: &str, context: &PluginContext) -> Result<()> {
        self.disable(name, context)?;
        self.enable(name, context)
    }

    pub fn unload(&self, name: &str, context: &PluginContext) -> Result<()> {
        let mut plugins = self
            .plugins
            .write()
            .map_err(|_| anyhow!("plugin registry lock poisoned"))?;
        let entry = plugins
            .get_mut(name)
            .ok_or_else(|| anyhow!("unknown Atlas plugin"))?;
        entry.plugin.unload(context)?;
        entry.state = PluginState::Unloaded;
        Ok(())
    }

    pub fn remove(&self, name: &str, context: &PluginContext) -> Result<()> {
        self.unload(name, context)?;
        self.plugins
            .write()
            .map_err(|_| anyhow!("plugin registry lock poisoned"))?
            .remove(name);
        Ok(())
    }

    pub fn manifests(&self) -> Result<Vec<(PluginManifest, PluginState)>> {
        Ok(self
            .plugins
            .read()
            .map_err(|_| anyhow!("plugin registry lock poisoned"))?
            .values()
            .map(|entry| (entry.plugin.manifest(), entry.state.clone()))
            .collect())
    }

    pub fn contributions(&self) -> Result<Vec<(String, PluginContributions)>> {
        Ok(self
            .plugins
            .read()
            .map_err(|_| anyhow!("plugin registry lock poisoned"))?
            .values()
            .map(|entry| (entry.plugin.manifest().name, entry.plugin.contributions()))
            .collect())
    }
}

fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    if manifest.name.trim().is_empty() || manifest.version.trim().is_empty() {
        return Err(anyhow!("Atlas plugin manifest requires name and version"));
    }
    Ok(())
}

impl EventListener for PluginRegistry {
    fn on_event(&self, _event: &AtlasEvent) -> Result<()> {
        Ok(())
    }
}
