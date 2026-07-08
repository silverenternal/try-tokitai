//! Agent/skill loader for markdown-backed prompts.

use crate::agent_skills::{AgentSkillCatalog, AgentSkillDef};
use std::path::PathBuf;
use std::sync::OnceLock;

/// An agent definition parsed from a skill markdown file.
#[derive(Debug, Clone)]
pub struct AgentDef {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub tools: Vec<String>,
    pub prompt: String,
}

/// Loader for workspace and user agent definitions.
pub struct AgentLoader {
    catalog: AgentSkillCatalog,
}

impl AgentLoader {
    pub fn new() -> Self {
        Self {
            catalog: AgentSkillCatalog::new(),
        }
    }

    /// Get the primary skills directory path being searched.
    pub fn skills_dir(&self) -> &PathBuf {
        self.catalog.roots().first().unwrap_or_else(|| {
            static FALLBACK: OnceLock<PathBuf> = OnceLock::new();
            FALLBACK.get_or_init(|| PathBuf::from("skills").join("agents"))
        })
    }

    /// List all available agent names.
    pub fn list_agents(&self) -> Vec<(String, String)> {
        self.catalog
            .list_skills()
            .into_iter()
            .map(|skill| (skill.name, skill.description))
            .collect()
    }

    /// Load a specific agent by name.
    pub fn load_agent(&self, name: &str) -> Option<AgentDef> {
        self.catalog.load_skill(name).map(convert_skill)
    }

    /// Auto-match research-oriented skills for a user request.
    pub fn auto_match_research_agents(&self, user_text: &str, mode: Option<&str>) -> Vec<AgentDef> {
        self.catalog
            .auto_match_research_skills(user_text, mode, 2)
            .into_iter()
            .map(convert_skill)
            .collect()
    }
}

fn convert_skill(skill: AgentSkillDef) -> AgentDef {
    AgentDef {
        name: skill.name,
        description: skill.description,
        domain: skill.domain,
        tools: skill.tools,
        prompt: skill.prompt,
    }
}

/// Active agent state for the TUI.
#[derive(Debug, Clone)]
pub struct ActiveAgent {
    pub name: String,
    pub prompt: String,
    pub is_active: bool,
}

impl ActiveAgent {
    pub fn none() -> Self {
        Self {
            name: "default".to_string(),
            prompt: String::new(),
            is_active: false,
        }
    }

    pub fn from_def(def: &AgentDef) -> Self {
        Self {
            name: def.name.clone(),
            prompt: def.prompt.clone(),
            is_active: true,
        }
    }

    /// Get the system prompt for the current agent.
    pub fn system_prompt(&self) -> String {
        if self.is_active && !self.prompt.is_empty() {
            format!("{}\n\nCurrent working directory: {{cwd}}", self.prompt)
        } else {
            "You are a helpful AI assistant with access to tools.\nCurrent working directory: {cwd}"
                .to_string()
        }
    }
}
