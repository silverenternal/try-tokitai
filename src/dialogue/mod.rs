//! 对话模块
//!
//! 实现对话状态机和任务追踪

pub mod state_machine;

pub use state_machine::{DialogueState, DialogueStateMachine, DialogueContext};
