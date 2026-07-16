//! Research Privacy Guard
//!
//! Protects sensitive research content from leaking through external AI APIs.
//! Strategy: Local-first for confidential phases, cloud-ok for literature review.

use std::sync::Arc;

/// Security classification for research content
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SecurityLevel {
    /// Public info only — safe for any API (literature search, public datasets)
    Public,
    /// Internal use — may contain unpublished ideas (hypothesis, experiment design)
    Internal,
    /// Confidential — novel results, must use local model only
    Confidential,
    /// Strict — paper draft, final results, no external API at all
    Strict,
}

impl SecurityLevel {
    pub fn label(&self) -> &str {
        match self {
            SecurityLevel::Public => "PUBLIC",
            SecurityLevel::Internal => "INTERNAL",
            SecurityLevel::Confidential => "CONFIDENTIAL",
            SecurityLevel::Strict => "STRICT",
        }
    }

    pub fn color(&self) -> &str {
        match self {
            SecurityLevel::Public => "green",
            SecurityLevel::Internal => "yellow",
            SecurityLevel::Confidential => "orange",
            SecurityLevel::Strict => "red",
        }
    }

    /// Can this level use cloud (external) APIs?
    pub fn allows_cloud(&self) -> bool {
        matches!(self, SecurityLevel::Public | SecurityLevel::Internal)
    }

    /// Does this level require local-only processing?
    pub fn requires_local(&self) -> bool {
        matches!(self, SecurityLevel::Confidential | SecurityLevel::Strict)
    }
}

/// Maps research phases to security levels
pub fn phase_security_level(phase: &crate::tui::research_pipeline::ResearchPhase) -> SecurityLevel {
    use crate::tui::research_pipeline::ResearchPhase;
    match phase {
        ResearchPhase::Idle => SecurityLevel::Public,
        ResearchPhase::LiteratureReview => SecurityLevel::Public,
        ResearchPhase::HypothesisGeneration => SecurityLevel::Internal,
        ResearchPhase::ExperimentDesign => SecurityLevel::Internal,
        ResearchPhase::Execution => SecurityLevel::Internal,
        ResearchPhase::Validation => SecurityLevel::Confidential,
        ResearchPhase::PaperWriting => SecurityLevel::Strict,
        ResearchPhase::Review => SecurityLevel::Strict,
        ResearchPhase::Complete => SecurityLevel::Strict,
    }
}

/// Privacy guard state
pub struct PrivacyGuard {
    /// Current security level
    pub level: SecurityLevel,
    /// Whether privacy mode is enforced
    pub enforced: bool,
    /// Whether a local model (Ollama) is available as fallback
    pub local_model_available: bool,
    /// The local provider (if configured)
    pub local_provider: Option<Arc<dyn crate::llm::LLMProvider>>,
    /// The cloud provider (for public phases)
    pub cloud_provider: Option<Arc<dyn crate::llm::LLMProvider>>,
    /// Log of all external API calls (for audit)
    pub audit_log: Vec<AuditEntry>,
    /// Warnings issued
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: String,
    pub phase: String,
    pub security_level: String,
    pub provider: String,
    pub model: String,
    pub tokens: usize,
}

impl PrivacyGuard {
    pub fn new() -> Self {
        Self {
            level: SecurityLevel::Public,
            enforced: false,
            local_model_available: false,
            local_provider: None,
            cloud_provider: None,
            audit_log: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Update security level based on research phase
    pub fn update_for_phase(&mut self, phase: &crate::tui::research_pipeline::ResearchPhase) {
        let new_level = phase_security_level(phase);
        if new_level != self.level {
            self.level = new_level.clone();
            if self.enforced && new_level.requires_local() && !self.local_model_available {
                self.warnings.push(format!(
                    "[{}] Phase requires local model but none configured. Research cannot proceed safely.",
                    chrono::Local::now().format("%H:%M:%S")
                ));
            }
        }
    }

    /// Check if the current request is safe to send to the given provider
    pub fn is_safe_to_send(&self, is_cloud_provider: bool) -> SafetyVerdict {
        if !self.enforced {
            return SafetyVerdict::Allowed;
        }

        if is_cloud_provider && self.level.requires_local() {
            SafetyVerdict::Blocked {
                reason: format!(
                    "Security level {} requires local model. Cloud API blocked.",
                    self.level.label()
                ),
            }
        } else if is_cloud_provider && self.level == SecurityLevel::Internal {
            SafetyVerdict::Warning {
                message: format!(
                    "Sending internal-level content to cloud API. Level: {}",
                    self.level.label()
                ),
            }
        } else {
            SafetyVerdict::Allowed
        }
    }

    /// Record an API call in the audit log
    pub fn record_api_call(&mut self, provider: &str, model: &str, tokens: usize, phase: &str) {
        self.audit_log.push(AuditEntry {
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            phase: phase.to_string(),
            security_level: self.level.label().to_string(),
            provider: provider.to_string(),
            model: model.to_string(),
            tokens,
        });
    }

    /// Generate a privacy report
    pub fn report(&self) -> String {
        let mut r = String::from("# Privacy Audit Report\n\n");
        r.push_str(&format!(
            "Privacy mode: {}\n",
            if self.enforced { "ENFORCED" } else { "OFF" }
        ));
        r.push_str(&format!("Current level: {}\n", self.level.label()));
        r.push_str(&format!(
            "Local model: {}\n",
            if self.local_model_available {
                "Available"
            } else {
                "Not configured"
            }
        ));
        r.push_str(&format!("External API calls: {}\n\n", self.audit_log.len()));

        if !self.audit_log.is_empty() {
            r.push_str("| Time | Phase | Level | Provider | Model | Tokens |\n");
            r.push_str("|------|-------|-------|----------|-------|--------|\n");
            for entry in &self.audit_log {
                r.push_str(&format!(
                    "| {} | {} | {} | {} | {} | {} |\n",
                    entry.timestamp,
                    entry.phase,
                    entry.security_level,
                    entry.provider,
                    entry.model,
                    entry.tokens,
                ));
            }
        }

        if !self.warnings.is_empty() {
            r.push_str("\n## Warnings\n\n");
            for w in &self.warnings {
                r.push_str(&format!("- {}\n", w));
            }
        }

        r
    }
}

impl Default for PrivacyGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a safety check
pub enum SafetyVerdict {
    Allowed,
    Warning { message: String },
    Blocked { reason: String },
}

/// Scan content for potential sensitive patterns
pub fn scan_sensitive_content(content: &str) -> Vec<String> {
    let mut findings = Vec::new();

    let sensitive_patterns = [
        ("proprietary", "May contain proprietary information"),
        ("patent", "May contain patent-related content"),
        ("novel algorithm", "May contain novel algorithmic details"),
        ("unpublished", "References unpublished work"),
        ("confidential", "Marked as confidential"),
        ("trade secret", "May contain trade secrets"),
        ("breakthrough", "May describe breakthrough findings"),
        ("first demonstration", "May describe first-of-kind results"),
    ];

    for (pattern, warning) in &sensitive_patterns {
        if content.to_lowercase().contains(pattern) {
            findings.push(warning.to_string());
        }
    }

    findings
}
