//! Agent Scheduler
//!
//! Coordinates agent execution through the MessageBus.
//! Supports multiple scheduling strategies: RoundRobin, Priority, Dependency-based.

use super::agent::{Agent, AgentContext, AgentMessage, AgentResponse, AgentRole};
use super::bus::MessageBus;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tracing::{debug, info, warn};

// ============================================================================
// Scheduler Trait
// ============================================================================

/// Trait for agent scheduling strategies.
///
/// The Scheduler receives incoming messages (typically from an orchestrator)
/// and routes them to the appropriate agent based on role and priority.
#[async_trait::async_trait]
pub trait Scheduler: Send + Sync {
    /// Register an agent with the scheduler
    fn register(&mut self, agent: Arc<dyn Agent>);

    /// Dispatch a message to the appropriate agent(s)
    async fn dispatch(&self, msg: AgentMessage, ctx: &AgentContext) -> Vec<AgentResponse>;

    /// Get all registered agents
    fn agents(&self) -> Vec<(AgentRole, String)>;

    /// Get the message bus
    fn bus(&self) -> &Arc<dyn MessageBus>;
}

// ============================================================================
// RoundRobinScheduler
// ============================================================================

/// Simple round-robin scheduler.
///
/// Messages are routed to the first matching agent by role.
/// If multiple agents share a role, they take turns.
pub struct RoundRobinScheduler {
    agents: tokio::sync::RwLock<HashMap<AgentRole, Vec<Arc<dyn Agent>>>>,
    counters: tokio::sync::RwLock<HashMap<AgentRole, usize>>,
    bus: Arc<dyn MessageBus>,
}

impl RoundRobinScheduler {
    /// Create a new round-robin scheduler
    pub fn new(bus: Arc<dyn MessageBus>) -> Self {
        Self {
            agents: tokio::sync::RwLock::new(HashMap::new()),
            counters: tokio::sync::RwLock::new(HashMap::new()),
            bus,
        }
    }
}

#[async_trait::async_trait]
impl Scheduler for RoundRobinScheduler {
    fn register(&mut self, agent: Arc<dyn Agent>) {
        let role = agent.role();
        // This is synchronous but uses block_on for the RwLock
        // In practice, registration happens at startup before dispatch begins
        futures::executor::block_on(async {
            let mut agents = self.agents.write().await;
            agents
                .entry(role.clone())
                .or_insert_with(Vec::new)
                .push(agent);
            let mut counters = self.counters.write().await;
            counters.entry(role).or_insert(0);
        });
    }

    async fn dispatch(&self, msg: AgentMessage, ctx: &AgentContext) -> Vec<AgentResponse> {
        let target_role = match &msg.to {
            Some(role) => role.clone(),
            // Broadcast to all agents
            None => {
                let agents = self.agents.read().await;
                let mut responses = Vec::new();
                for agents_for_role in agents.values() {
                    for agent in agents_for_role {
                        match agent.handle_message(msg.clone(), ctx).await {
                            Ok(resp) => responses.push(resp),
                            Err(e) => {
                                warn!("Agent {} error: {}", agent.id(), e);
                                responses.push(AgentResponse::error(e.to_string()));
                            }
                        }
                    }
                }
                return responses;
            }
        };

        // Round-robin: pick next agent index
        let idx = {
            let agents = self.agents.read().await;
            let candidates = match agents.get(&target_role) {
                Some(list) => list,
                None => {
                    warn!("No agent registered for role: {}", target_role);
                    return vec![AgentResponse::error(format!(
                        "No agent for role: {}",
                        target_role
                    ))];
                }
            };

            if candidates.is_empty() {
                return vec![AgentResponse::error("No agents available".to_string())];
            }

            let mut counters = self.counters.write().await;
            let counter = counters.entry(target_role.clone()).or_insert(0);
            let i = *counter % candidates.len();
            *counter += 1;
            i
        }; // Both locks released here

        let agents = self.agents.read().await;
        let candidates = agents.get(&target_role).unwrap();
        let agent = &candidates[idx];

        debug!(
            "Dispatching {} -> agent {} (role: {})",
            msg.id,
            agent.id(),
            target_role
        );

        match agent.handle_message(msg, ctx).await {
            Ok(resp) => {
                info!("Agent {} completed successfully", agent.id());
                vec![resp]
            }
            Err(e) => {
                warn!("Agent {} failed: {}", agent.id(), e);
                vec![AgentResponse::error(e.to_string())]
            }
        }
    }

    fn agents(&self) -> Vec<(AgentRole, String)> {
        futures::executor::block_on(async {
            let agents = self.agents.read().await;
            agents
                .iter()
                .flat_map(|(role, list)| {
                    list.iter()
                        .map(|a| (role.clone(), a.id().to_string()))
                        .collect::<Vec<_>>()
                })
                .collect()
        })
    }

    fn bus(&self) -> &Arc<dyn MessageBus> {
        &self.bus
    }
}

// ============================================================================
// PriorityScheduler (alternative)
// ============================================================================

/// Priority-based scheduler: higher priority messages are dispatched first.
///
/// Maintains a priority queue of pending messages.
pub struct PriorityScheduler {
    agents: tokio::sync::RwLock<HashMap<AgentRole, Vec<Arc<dyn Agent>>>>,
    pending: tokio::sync::Mutex<VecDeque<AgentMessage>>,
    bus: Arc<dyn MessageBus>,
}

impl PriorityScheduler {
    pub fn new(bus: Arc<dyn MessageBus>) -> Self {
        Self {
            agents: tokio::sync::RwLock::new(HashMap::new()),
            pending: tokio::sync::Mutex::new(VecDeque::new()),
            bus,
        }
    }
}

#[async_trait::async_trait]
impl Scheduler for PriorityScheduler {
    fn register(&mut self, agent: Arc<dyn Agent>) {
        futures::executor::block_on(async {
            let mut agents = self.agents.write().await;
            agents
                .entry(agent.role())
                .or_insert_with(Vec::new)
                .push(agent);
        });
    }

    async fn dispatch(&self, msg: AgentMessage, ctx: &AgentContext) -> Vec<AgentResponse> {
        // Enqueue by priority
        let priority = msg.priority;
        {
            let mut pending = self.pending.lock().await;
            // Insert at correct position (higher priority = closer to front)
            let pos = pending
                .iter()
                .position(|m| m.priority < priority)
                .unwrap_or(pending.len());
            pending.insert(pos, msg.clone());
        }

        // Process highest priority first
        let next_msg = {
            let mut pending = self.pending.lock().await;
            pending.pop_front()
        };

        match next_msg {
            Some(msg) => {
                // Delegate to round-robin for the actual dispatch
                let agents = self.agents.read().await;
                if let Some(target) = &msg.to {
                    if let Some(candidates) = agents.get(target) {
                        if let Some(agent) = candidates.first() {
                            match agent.handle_message(msg, ctx).await {
                                Ok(resp) => return vec![resp],
                                Err(e) => return vec![AgentResponse::error(e.to_string())],
                            }
                        }
                    }
                }
                vec![AgentResponse::error("No agent available".to_string())]
            }
            None => vec![AgentResponse::ok(serde_json::json!({"status": "idle"}))],
        }
    }

    fn agents(&self) -> Vec<(AgentRole, String)> {
        futures::executor::block_on(async {
            self.agents
                .read()
                .await
                .iter()
                .flat_map(|(r, list)| list.iter().map(|a| (r.clone(), a.id().to_string())))
                .collect()
        })
    }

    fn bus(&self) -> &Arc<dyn MessageBus> {
        &self.bus
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{AgentRole, Capability, MessageType};

    struct TestAgent {
        id: String,
        role: AgentRole,
    }

    #[async_trait::async_trait]
    impl Agent for TestAgent {
        fn id(&self) -> &str {
            &self.id
        }
        fn role(&self) -> AgentRole {
            self.role.clone()
        }
        fn capabilities(&self) -> Vec<Capability> {
            vec![]
        }
        async fn handle_message(
            &self,
            _msg: AgentMessage,
            _ctx: &AgentContext,
        ) -> Result<AgentResponse, crate::agent::AgentError> {
            Ok(AgentResponse::ok(
                serde_json::json!({"handled_by": self.id.clone()}),
            ))
        }
    }

    #[tokio::test]
    async fn test_round_robin_scheduler() {
        let bus = Arc::new(crate::bus::ChannelMessageBus::new());
        let mut scheduler = RoundRobinScheduler::new(bus);

        scheduler.register(Arc::new(TestAgent {
            id: "r1".into(),
            role: AgentRole::Researcher,
        }));
        scheduler.register(Arc::new(TestAgent {
            id: "r2".into(),
            role: AgentRole::Researcher,
        }));

        let registered = scheduler.agents();
        assert_eq!(registered.len(), 2);

        let ctx = AgentContext::new("test");
        let msg = AgentMessage::new(
            AgentRole::Orchestrator,
            Some(AgentRole::Researcher),
            MessageType::Request,
            serde_json::json!({}),
        );

        let responses = scheduler.dispatch(msg, &ctx).await;
        assert_eq!(responses.len(), 1);
        assert!(responses[0].success);
    }
}
