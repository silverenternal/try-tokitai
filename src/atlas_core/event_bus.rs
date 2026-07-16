use super::AtlasEvent;
use anyhow::Result;
use std::sync::{Arc, RwLock};

pub trait EventListener: Send + Sync {
    fn on_event(&self, event: &AtlasEvent) -> Result<()>;
}

#[derive(Default)]
pub struct EventBus {
    listeners: RwLock<Vec<Arc<dyn EventListener>>>,
}

impl std::fmt::Debug for EventBus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventBus")
            .field("listener_count", &self.listeners.read().map(|value| value.len()).unwrap_or(0))
            .finish()
    }
}

impl EventBus {
    pub fn subscribe(&self, listener: Arc<dyn EventListener>) {
        if let Ok(mut listeners) = self.listeners.write() {
            listeners.push(listener);
        }
    }

    pub fn publish(&self, event: &AtlasEvent) -> Result<()> {
        let listeners = self
            .listeners
            .read()
            .map_err(|_| anyhow::anyhow!("Atlas event bus lock poisoned"))?;
        for listener in listeners.iter() {
            listener.on_event(event)?;
        }
        Ok(())
    }
}
