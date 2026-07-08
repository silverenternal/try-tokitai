//! MessageBus — decoupled agent communication
//!
//! Agents never call each other directly. All communication flows through
//! the MessageBus, which supports topic-based publish/subscribe.
//!
//! The default implementation uses Tokio MPSC channels for in-process
//! communication. Future implementations may use Redis, NATS, or Kafka.

use super::agent::AgentMessage;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::mpsc;

// ============================================================================
// MessageBus Trait
// ============================================================================

/// Trait for agent message passing.
///
/// Implementations can be in-process (channels), cross-process (IPC),
/// or distributed (Redis/NATS/Kafka).
#[async_trait]
pub trait MessageBus: Send + Sync {
    /// Publish a message to a topic.
    ///
    /// Topics follow the convention: `agent.{role}.{action}`
    /// e.g., `agent.researcher.search_paper`
    async fn publish(&self, topic: &str, msg: AgentMessage) -> Result<(), BusError>;

    /// Subscribe to a topic and receive messages.
    ///
    /// Returns a receiver that yields messages published to this topic.
    /// Wildcards are implementation-defined.
    async fn subscribe(&self, topic: &str) -> Result<mpsc::Receiver<AgentMessage>, BusError>;

    /// List all active topics
    async fn topics(&self) -> Vec<String>;

    /// Number of subscribers for a topic
    async fn subscriber_count(&self, topic: &str) -> usize;
}

// ============================================================================
// BusError
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    #[error("Channel closed")]
    ChannelClosed,

    #[error("Bus internal error: {0}")]
    Internal(String),
}

// ============================================================================
// ChannelMessageBus (in-process implementation)
// ============================================================================

/// In-process MessageBus using Tokio MPSC channels.
///
/// One sender per topic, multiple receivers via broadcast fan-out.
pub struct ChannelMessageBus {
    /// Map from topic name to a list of senders
    topics: tokio::sync::RwLock<HashMap<String, Vec<mpsc::Sender<AgentMessage>>>>,
}

impl ChannelMessageBus {
    /// Create a new empty channel bus
    pub fn new() -> Self {
        Self {
            topics: tokio::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for ChannelMessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageBus for ChannelMessageBus {
    async fn publish(&self, topic: &str, msg: AgentMessage) -> Result<(), BusError> {
        let topics = self.topics.read().await;
        let senders = topics
            .get(topic)
            .ok_or_else(|| BusError::TopicNotFound(topic.to_string()))?;

        // Fan-out to all subscribers
        for sender in senders {
            if sender.send(msg.clone()).await.is_err() {
                // Receiver dropped — this is OK, subscriber unsubscribed
                tracing::debug!("Subscriber dropped for topic: {}", topic);
            }
        }
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<mpsc::Receiver<AgentMessage>, BusError> {
        let (tx, rx) = mpsc::channel(256);
        let mut topics = self.topics.write().await;
        topics
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(tx);
        Ok(rx)
    }

    async fn topics(&self) -> Vec<String> {
        self.topics.read().await.keys().cloned().collect()
    }

    async fn subscriber_count(&self, topic: &str) -> usize {
        self.topics
            .read()
            .await
            .get(topic)
            .map(|s| s.len())
            .unwrap_or(0)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentRole, MessageType};

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = ChannelMessageBus::new();
        let mut rx = bus.subscribe("agent.researcher.search").await.unwrap();

        let msg = AgentMessage::new(
            AgentRole::Orchestrator,
            Some(AgentRole::Researcher),
            MessageType::Request,
            serde_json::json!({"query": "test"}),
        );

        bus.publish("agent.researcher.search", msg).await.unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.from, AgentRole::Orchestrator);
        assert_eq!(received.payload["query"], "test");
    }

    #[tokio::test]
    async fn test_publish_to_nonexistent_topic() {
        let bus = ChannelMessageBus::new();
        let msg = AgentMessage::new(
            AgentRole::Orchestrator,
            None,
            MessageType::Notification,
            serde_json::json!({}),
        );

        let result = bus.publish("nonexistent", msg).await;
        assert!(result.is_err());
    }
}
