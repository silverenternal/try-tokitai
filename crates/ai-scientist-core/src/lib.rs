//! AI Scientist Core — Agent Framework
//!
//! Provides the foundational traits and types for building
//! AI Scientist agents with a unified message bus communication pattern.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────┐  ┌──────────────┐  ┌─────────────┐
//! │  Agent   │  │  MessageBus  │  │  Scheduler  │
//! │  Trait   │  │    Trait     │  │             │
//! └──────────┘  └──────────────┘  └─────────────┘
//!      ▲               ▲                ▲
//!      │               │                │
//! ┌─────┴─────┐  ┌──────┴──────┐  ┌──────┴──────┐
//! │ Research  │  │  Channel    │  │  RoundRobin │
//! │ Hypothesis│  │  MessageBus │  │  Priority   │
//! │ Experiment│  └─────────────┘  └─────────────┘
//! │ Verify    │
//! │ Report    │
//! └───────────┘
//! ```

pub mod agent;
pub mod bus;
pub mod config;
pub mod scheduler;
pub mod tool;

pub use agent::{
    Agent, AgentContext, AgentMessage, AgentResponse, AgentRole, AgentStatus, Capability,
};
pub use bus::{ChannelMessageBus, MessageBus};
pub use config::ScientistConfig;
pub use scheduler::{RoundRobinScheduler, Scheduler};
pub use tool::{Tool, ToolDefinition, ToolError, ToolParameter, ToolResult, ToolType};

/// Prelude for convenient imports
pub mod prelude {
    pub use super::agent::{
        Agent, AgentContext, AgentMessage, AgentResponse, AgentRole, AgentStatus, Capability,
    };
    pub use super::bus::{ChannelMessageBus, MessageBus};
    pub use super::config::ScientistConfig;
    pub use super::scheduler::{RoundRobinScheduler, Scheduler};
    pub use super::tool::{Tool, ToolDefinition, ToolError, ToolParameter, ToolResult, ToolType};
}
