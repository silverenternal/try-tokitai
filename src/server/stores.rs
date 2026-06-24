//! 共享状态：会话与工作流
//!
//! `SessionStore` 维护一个 UUID -> Session 的映射，存放在 `parking_lot::Mutex`
//! 后面以 `Arc` 共享，便于在多 handler 间安全读写。
//!
//! `WorkflowStore` 维护一个 UUID -> `WorkflowEngine` 的映射，用于跨请求
//! 持有正在执行 / 暂停 / 已取消的工作流。

use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::Mutex as PlMutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::orchestrator::workflow::{Workflow, WorkflowEngine, WorkflowStatus};

/// 单条对话会话：消息列表 + 元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    pub id: Uuid,
    pub name: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub messages: Vec<serde_json::Value>,
}

impl ConversationSession {
    pub fn new(name: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name,
            created_at: now,
            updated_at: now,
            messages: Vec::new(),
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}

/// 会话仓库
#[derive(Default)]
pub struct SessionStore {
    inner: HashMap<Uuid, ConversationSession>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create(&mut self, name: Option<String>) -> ConversationSession {
        let session = ConversationSession::new(name);
        self.inner.insert(session.id, session.clone());
        session
    }

    pub fn get(&self, id: &Uuid) -> Option<&ConversationSession> {
        self.inner.get(id)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut ConversationSession> {
        self.inner.get_mut(id)
    }

    pub fn list(&self) -> Vec<&ConversationSession> {
        self.inner.values().collect()
    }

    pub fn delete(&mut self, id: &Uuid) -> bool {
        self.inner.remove(id).is_some()
    }
}

/// 工作流仓库（保留 WorkflowEngine 状态以便执行/暂停/取消）
#[derive(Default)]
pub struct WorkflowStore {
    inner: HashMap<Uuid, WorkflowEngine>,
}

impl WorkflowStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, engine: WorkflowEngine) -> Uuid {
        let id = Uuid::new_v4();
        self.inner.insert(id, engine);
        id
    }

    pub fn get(&self, id: &Uuid) -> Option<&WorkflowEngine> {
        self.inner.get(id)
    }

    pub fn get_mut(&mut self, id: &Uuid) -> Option<&mut WorkflowEngine> {
        self.inner.get_mut(id)
    }

    pub fn list(&self) -> Vec<(Uuid, &Workflow)> {
        self.inner
            .iter()
            .map(|(id, engine)| (*id, engine.get_workflow()))
            .collect()
    }

    pub fn delete(&mut self, id: &Uuid) -> bool {
        self.inner.remove(id).is_some()
    }

    pub fn status_of(&self, id: &Uuid) -> Option<WorkflowStatus> {
        self.inner.get(id).map(|engine| engine.get_status().clone())
    }
}

/// 顶层共享的会话/工作流存储
#[derive(Clone, Default)]
pub struct SharedStores {
    pub sessions: Arc<PlMutex<SessionStore>>,
    pub workflows: Arc<PlMutex<WorkflowStore>>,
}

impl SharedStores {
    pub fn new() -> Self {
        Self::default()
    }
}
