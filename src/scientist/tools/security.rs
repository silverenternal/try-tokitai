//! Security Architecture for AI Scientist
//!
//! Layers: RBAC → Tool Permission → Sandbox → Injection Detection → Secrets → Audit

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================================
// RBAC
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    Researcher,
    Viewer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ToolExecute,
    ToolCreate,
    ConfigModify,
    DataRead,
    DataWrite,
    PaperDownload,
    CodeExecute,
    NetworkAccess,
}

pub struct Rbac {
    role_permissions: HashMap<Role, HashSet<Permission>>,
}

impl Rbac {
    pub fn new() -> Self {
        let mut role_permissions = HashMap::new();

        role_permissions.insert(
            Role::Admin,
            HashSet::from_iter([
                Permission::ToolExecute,
                Permission::ToolCreate,
                Permission::ConfigModify,
                Permission::DataRead,
                Permission::DataWrite,
                Permission::PaperDownload,
                Permission::CodeExecute,
                Permission::NetworkAccess,
            ]),
        );

        role_permissions.insert(
            Role::Researcher,
            HashSet::from_iter([
                Permission::ToolExecute,
                Permission::DataRead,
                Permission::DataWrite,
                Permission::PaperDownload,
                Permission::NetworkAccess,
            ]),
        );

        role_permissions.insert(Role::Viewer, HashSet::from_iter([Permission::DataRead]));

        Self { role_permissions }
    }

    pub fn has_permission(&self, role: &Role, perm: &Permission) -> bool {
        self.role_permissions
            .get(role)
            .map(|perms| perms.contains(perm))
            .unwrap_or(false)
    }
}

impl Default for Rbac {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tool Permission
// ============================================================================

pub struct ToolPermission {
    tool_risk_map: HashMap<String, String>, // tool_name -> risk_level
    rbac: Rbac,
}

impl ToolPermission {
    pub fn new(rbac: Rbac) -> Self {
        Self {
            tool_risk_map: HashMap::new(),
            rbac,
        }
    }

    pub fn register_tool(&mut self, name: &str, risk_level: &str) {
        self.tool_risk_map
            .insert(name.to_string(), risk_level.to_string());
    }

    pub fn can_execute(&self, role: &Role, tool_name: &str) -> bool {
        let risk = self
            .tool_risk_map
            .get(tool_name)
            .map(|s| s.as_str())
            .unwrap_or("safe");

        match risk {
            "safe" | "low" => self.rbac.has_permission(role, &Permission::ToolExecute),
            "moderate" => role == &Role::Researcher || role == &Role::Admin,
            "high" | "critical" => role == &Role::Admin,
            _ => false,
        }
    }
}

// ============================================================================
// Prompt Injection Detector
// ============================================================================

pub struct PromptInjectionDetector {
    patterns: Vec<String>,
}

impl PromptInjectionDetector {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                r"(?i)ignore\s+(all\s+)?(previous|above|prior)\s+instructions".into(),
                r"(?i)you\s+are\s+now\s+(a\s+)?\w+\s+(bot|assistant|agent)".into(),
                r"(?i)system\s*prompt\s*:".into(),
                r"(?i)<\|im_start\|>".into(),
                r"(?i)\[INST\].*\[/INST\]".into(),
                r"(?i)forget\s+everything".into(),
            ],
        }
    }

    pub fn detect(&self, input: &str) -> Option<String> {
        let lower = input.to_lowercase();
        for pattern in &self.patterns {
            // Simple substring match for known injection patterns
            let p = pattern.to_lowercase();
            let clean_p = p
                .replace("(?i)", "")
                .replace("\\s+", " ")
                .replace("\\s*", " ");
            if lower.contains(&clean_p.split_whitespace().collect::<Vec<_>>().join(" "))
                || lower.contains("ignore previous")
                || lower.contains("system prompt")
            {
                return Some("Potential prompt injection detected".into());
            }
        }
        None
    }
}

impl Default for PromptInjectionDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Secret Manager
// ============================================================================

pub struct SecretManager {
    secrets: HashMap<String, String>,
}

impl SecretManager {
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
        }
    }

    pub fn store(&mut self, key: &str, value: &str) {
        // In production: encrypt with AES256 before storing
        self.secrets.insert(key.to_string(), value.to_string());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.secrets.get(key).map(|s| s.as_str())
    }
}

// ============================================================================
// Audit Log
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub user: String,
    pub action: String,
    pub tool: Option<String>,
    pub success: bool,
    pub details: String,
}

pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn record(
        &mut self,
        user: &str,
        action: &str,
        tool: Option<&str>,
        success: bool,
        details: &str,
    ) {
        self.entries.push(AuditEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            user: user.to_string(),
            action: action.to_string(),
            tool: tool.map(|s| s.to_string()),
            success,
            details: details.to_string(),
        });
    }
}
