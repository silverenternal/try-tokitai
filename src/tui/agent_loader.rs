//! Agent/Skill loader — loads Claude Code style agent definitions
//! from the .claude/skills directory and injects them as system prompts.

use std::path::PathBuf;

/// An agent definition parsed from a skill markdown file
#[derive(Debug, Clone)]
pub struct AgentDef {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub tools: Vec<String>,
    pub prompt: String,
}

/// Loader for .claude/skills agent definitions
pub struct AgentLoader {
    skills_dir: PathBuf,
}

impl AgentLoader {
    /// Create a new loader pointing to the user's .claude/skills directory
    pub fn new() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            skills_dir: home.join(".claude").join("skills").join("agents"),
        }
    }

    /// Get the skills directory path being searched
    pub fn skills_dir(&self) -> &PathBuf {
        &self.skills_dir
    }

    /// List all available agent names
    pub fn list_agents(&self) -> Vec<(String, String)> {
        let mut agents = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&self.skills_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    // Subdirectory (engineering, product, personas, etc.)
                    if let Ok(sub) = std::fs::read_dir(&path) {
                        for f in sub.flatten() {
                            if let Some(name) = f.file_name().to_str() {
                                if name.ends_with(".md") && name != "CLAUDE.md" && name != "README.md" && name != "TEMPLATE.md" {
                                    let agent_name = name.trim_end_matches(".md").to_string();
                                    let (desc, _domain) = Self::parse_frontmatter(&f.path());
                                    agents.push((agent_name, desc));
                                }
                            }
                        }
                    }
                } else if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.ends_with(".md") && name != "CLAUDE.md" {
                            let agent_name = name.trim_end_matches(".md").to_string();
                            let (desc, _domain) = Self::parse_frontmatter(&path);
                            agents.push((agent_name, desc));
                        }
                    }
                }
            }
        }
        agents
    }

    /// Load a specific agent by name
    pub fn load_agent(&self, name: &str) -> Option<AgentDef> {
        let filename = format!("{}.md", name);
        // Search recursively
        let path = self.find_file(&filename)?;

        // 安全检查：确保文件路径在 skills 目录内（防止符号链接逃逸）
        if let Ok(canonical_path) = path.canonicalize() {
            let canonical_skills = self.skills_dir.canonicalize().unwrap_or_else(|_| {
                self.skills_dir.clone()
            });
            if !canonical_path.starts_with(&canonical_skills) {
                tracing::warn!(
                    "Agent file {:?} is outside of skills directory {:?} — rejected",
                    canonical_path,
                    canonical_skills
                );
                return None;
            }
        }

        let content = std::fs::read_to_string(&path).ok()?;
        let (description, domain) = Self::parse_frontmatter(&path);
        let tools = Self::parse_tools(&content);

        // Extract the body (skip YAML frontmatter)
        let body = if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                content[3 + end + 3..].trim().to_string()
            } else {
                content.clone()
            }
        } else {
            content.clone()
        };

        // Build a compact system prompt from the agent definition
        let prompt = Self::build_prompt(name, &description, &domain, &tools, &body);

        Some(AgentDef {
            name: name.to_string(),
            description,
            domain,
            tools,
            prompt,
        })
    }

    fn find_file(&self, filename: &str) -> Option<PathBuf> {
        fn search(dir: &PathBuf, target: &str) -> Option<PathBuf> {
            if !dir.exists() { return None; }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(found) = search(&path, target) {
                            return Some(found);
                        }
                    } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name == target {
                            return Some(path.to_path_buf());
                        }
                    }
                }
            }
            None
        }
        search(&PathBuf::from(&self.skills_dir), filename)
    }

    fn parse_frontmatter(path: &PathBuf) -> (String, String) {
        let mut description = String::new();
        let mut domain = String::new();
        if let Ok(content) = std::fs::read_to_string(path) {
            if content.starts_with("---") {
                if let Some(end) = content[3..].find("---") {
                    let fm = &content[3..3 + end];
                    for line in fm.lines() {
                        let line = line.trim();
                        if let Some(val) = line.strip_prefix("description:") {
                            description = val.trim().to_string();
                        }
                        if let Some(val) = line.strip_prefix("domain:") {
                            domain = val.trim().to_string();
                        }
                    }
                }
            }
        }
        (description, domain)
    }

    fn parse_tools(content: &str) -> Vec<String> {
        if content.starts_with("---") {
            if let Some(end) = content[3..].find("---") {
                let fm = &content[3..3 + end];
                for line in fm.lines() {
                    if let Some(tools_line) = line.trim().strip_prefix("tools:") {
                        let tools_str = tools_line.trim().trim_start_matches('[').trim_end_matches(']');
                        return tools_str.split(',').map(|t| t.trim().to_string()).collect();
                    }
                }
            }
        }
        vec![]
    }

    fn build_prompt(name: &str, desc: &str, _domain: &str, _tools: &[String], body: &str) -> String {
        // Build a clean system prompt that captures the agent's role and expertise
        let mut prompt = String::new();
        prompt.push_str(&format!("You are acting as: **{}**\n", name));
        if !desc.is_empty() {
            prompt.push_str(&format!("{}\n\n", desc));
        }

        // Extract key sections from the body (skip frontmatter already handled)
        let sections = ["## Role & Expertise", "## Purpose", "## Core Workflows", "## Output Standards"];
        for section in &sections {
            if let Some(start) = body.find(section) {
                let remaining = &body[start..];
                // Find next ## section
                let end = remaining[2..].find("\n## ").map(|e| e + 2).unwrap_or(remaining.len());
                let section_body = &remaining[..end.min(remaining.len())];
                prompt.push_str(section_body);
                prompt.push_str("\n\n");
            }
        }

        prompt.push_str("\nFollow the above role guidelines when responding. Use available tools when needed.\n");
        prompt
    }
}

/// Active agent state for the TUI
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

    /// Get the system prompt for the current agent
    pub fn system_prompt(&self) -> String {
        if self.is_active && !self.prompt.is_empty() {
            format!("{}\n\nCurrent working directory: {{cwd}}", self.prompt)
        } else {
            "You are a helpful AI assistant with access to tools.\nCurrent working directory: {cwd}".to_string()
        }
    }
}
