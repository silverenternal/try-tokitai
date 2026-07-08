//! Agent trait and supporting types
//!
//! Defines the core `Agent` trait that all AI Scientist agents implement,
//! along with the communication primitives: `AgentMessage`, `AgentResponse`,
//! and `AgentContext`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// AgentRole
// ============================================================================

/// The role of an agent in the AI Scientist system
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    /// Literature search, paper retrieval, knowledge extraction
    Researcher,
    /// Hypothesis generation and refinement
    Hypothesizer,
    /// Experiment design and execution
    Experimenter,
    /// Mathematical and formal verification
    Verifier,
    /// Report generation and paper writing
    Reporter,
    /// Orchestration / coordination
    Orchestrator,
    /// Custom/user-defined role
    Custom(String),
}

impl std::fmt::Display for AgentRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentRole::Researcher => write!(f, "Researcher"),
            AgentRole::Hypothesizer => write!(f, "Hypothesizer"),
            AgentRole::Experimenter => write!(f, "Experimenter"),
            AgentRole::Verifier => write!(f, "Verifier"),
            AgentRole::Reporter => write!(f, "Reporter"),
            AgentRole::Orchestrator => write!(f, "Orchestrator"),
            AgentRole::Custom(s) => write!(f, "{}", s),
        }
    }
}

// ============================================================================
// Capability
// ============================================================================

/// A capability that an agent advertises
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// Capability name (e.g., "literature_search", "math_verification")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Tools this capability requires
    pub required_tools: Vec<String>,
}

// ============================================================================
// AgentStatus
// ============================================================================

/// Current status of an agent
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Ready to receive messages
    Idle,
    /// Processing a message
    Busy,
    /// Waiting for external input (tool result, human approval)
    Waiting,
    /// Encountered an error
    Error(String),
    /// Shut down
    Stopped,
}

// ============================================================================
// AgentMessage
// ============================================================================

/// A message exchanged between agents via the MessageBus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// Unique message ID (UUID v4)
    pub id: String,
    /// Sender agent role
    pub from: AgentRole,
    /// Target agent role (None = broadcast)
    pub to: Option<AgentRole>,
    /// Message type tag
    pub msg_type: MessageType,
    /// Message payload (structured JSON)
    pub payload: serde_json::Value,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Correlation ID for request-response pairing
    pub correlation_id: Option<String>,
    /// Priority (0 = low, 5 = critical)
    pub priority: u8,
}

impl AgentMessage {
    /// Create a new message
    pub fn new(
        from: AgentRole,
        to: Option<AgentRole>,
        msg_type: MessageType,
        payload: serde_json::Value,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            msg_type,
            payload,
            timestamp: Utc::now(),
            correlation_id: None,
            priority: 0,
        }
    }

    /// Set correlation ID for request-response tracking
    pub fn with_correlation(mut self, id: String) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, p: u8) -> Self {
        self.priority = p.min(5);
        self
    }
}

// ============================================================================
// MessageType
// ============================================================================

/// Classification of agent messages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    /// Request an action
    Request,
    /// Response to a request
    Response,
    /// Notification (fire-and-forget)
    Notification,
    /// Error report
    Error,
    /// Status update
    Status,
    /// Tool call result
    ToolResult,
    /// Workflow stage transition
    StageTransition,
}

// ============================================================================
// AgentResponse
// ============================================================================

/// Response from an agent after handling a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Response content (structured JSON)
    pub content: serde_json::Value,
    /// Whether the agent completed its task successfully
    pub success: bool,
    /// Optional error message
    pub error: Option<String>,
    /// Follow-up messages to publish
    pub follow_up: Vec<AgentMessage>,
    /// Suggested next agent role
    pub next_role: Option<AgentRole>,
}

impl AgentResponse {
    /// Create a successful response
    pub fn ok(content: serde_json::Value) -> Self {
        Self {
            content,
            success: true,
            error: None,
            follow_up: Vec::new(),
            next_role: None,
        }
    }

    /// Create an error response
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            content: serde_json::Value::Null,
            success: false,
            error: Some(msg.into()),
            follow_up: Vec::new(),
            next_role: None,
        }
    }

    /// Add a follow-up message
    pub fn with_follow_up(mut self, msg: AgentMessage) -> Self {
        self.follow_up.push(msg);
        self
    }

    /// Suggest next agent
    pub fn with_next_role(mut self, role: AgentRole) -> Self {
        self.next_role = Some(role);
        self
    }
}

// ============================================================================
// AgentContext
// ============================================================================

/// Context provided to an agent when handling a message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    /// Current workflow stage
    pub stage: String,
    /// Conversation/message history
    pub history: Vec<AgentMessage>,
    /// Shared memory / knowledge store
    pub memory: HashMap<String, serde_json::Value>,
    /// Available tool definitions
    pub available_tools: Vec<String>,
    /// Agent configuration
    pub config: serde_json::Value,
    /// Session ID
    pub session_id: String,
    /// Research topic/goal
    pub research_goal: Option<String>,
}

impl AgentContext {
    /// Create a new context
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            stage: "init".to_string(),
            history: Vec::new(),
            memory: HashMap::new(),
            available_tools: Vec::new(),
            config: serde_json::Value::Null,
            session_id: session_id.into(),
            research_goal: None,
        }
    }

    /// Set the research goal
    pub fn with_goal(mut self, goal: impl Into<String>) -> Self {
        self.research_goal = Some(goal.into());
        self
    }

    /// Store a value in shared memory
    pub fn remember(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.memory.insert(key.into(), value);
    }

    /// Recall a value from shared memory
    pub fn recall(&self, key: &str) -> Option<&serde_json::Value> {
        self.memory.get(key)
    }
}

// ============================================================================
// Agent Trait
// ============================================================================

/// Core trait for all AI Scientist agents.
///
/// Agents communicate exclusively through the MessageBus — they never call
/// each other directly. This ensures loose coupling and enables future
/// distributed deployment.
///
/// # Example
///
/// ```rust,ignore
/// use ai_scientist_core::prelude::*;
/// use async_trait::async_trait;
///
/// struct MyResearchAgent;
///
/// #[async_trait]
/// impl Agent for MyResearchAgent {
///     fn id(&self) -> &str { "research-1" }
///     fn role(&self) -> AgentRole { AgentRole::Researcher }
///     fn capabilities(&self) -> Vec<Capability> { vec![] }
///
///     async fn handle_message(&self, msg: AgentMessage, ctx: &AgentContext)
///         -> Result<AgentResponse, AgentError>
///     {
///         Ok(AgentResponse::ok(serde_json::json!({"found": 3})))
///     }
/// }
/// ```
#[async_trait]
pub trait Agent: Send + Sync {
    /// Unique identifier for this agent instance
    fn id(&self) -> &str;

    /// The role this agent fulfills
    fn role(&self) -> AgentRole;

    /// Handle an incoming message and produce a response.
    ///
    /// This is the primary entry point — all agent logic flows through here.
    async fn handle_message(
        &self,
        msg: AgentMessage,
        ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError>;

    /// List capabilities this agent provides
    fn capabilities(&self) -> Vec<Capability>;

    /// Current agent status
    fn status(&self) -> AgentStatus {
        AgentStatus::Idle
    }
}

// ============================================================================
// AgentError
// ============================================================================

/// Errors that can occur during agent message handling
#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Tool execution failed: {0}")]
    ToolExecutionFailed(String),

    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Context missing required field: {0}")]
    MissingContext(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Timeout")]
    Timeout,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = AgentMessage::new(
            AgentRole::Orchestrator,
            Some(AgentRole::Researcher),
            MessageType::Request,
            serde_json::json!({"query": "quantum computing"}),
        );

        assert!(!msg.id.is_empty());
        assert_eq!(msg.from, AgentRole::Orchestrator);
        assert_eq!(msg.to, Some(AgentRole::Researcher));
        assert_eq!(msg.msg_type, MessageType::Request);
    }

    #[test]
    fn test_response_ok() {
        let resp = AgentResponse::ok(serde_json::json!({"result": 42}));
        assert!(resp.success);
        assert_eq!(resp.content["result"], 42);
    }

    #[test]
    fn test_response_error() {
        let resp = AgentResponse::error("something went wrong");
        assert!(!resp.success);
        assert_eq!(resp.error.unwrap(), "something went wrong");
    }

    #[test]
    fn test_context_memory() {
        let mut ctx = AgentContext::new("session-1");
        ctx.remember("key1", serde_json::json!("value1"));
        assert_eq!(ctx.recall("key1").unwrap().as_str().unwrap(), "value1");
    }
}
