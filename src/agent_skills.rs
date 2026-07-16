use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

static TOKEN_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[A-Za-z0-9]+|[\u4E00-\u9FFF]+").expect("token regex"));

const EN_STOPWORDS: &[&str] = &[
    "a",
    "agent",
    "an",
    "and",
    "any",
    "before",
    "by",
    "computer",
    "cs",
    "does",
    "for",
    "from",
    "how",
    "implementation",
    "in",
    "is",
    "it",
    "of",
    "on",
    "or",
    "paper",
    "research",
    "science",
    "skill",
    "task",
    "the",
    "to",
    "use",
    "when",
    "workflow",
];

const ZH_STOPWORDS: &[&str] = &[
    "\u{4e00}\u{4e2a}",
    "\u{4e00}\u{4e9b}",
    "\u{4efb}\u{52a1}",
    "\u{4f7f}\u{7528}",
    "\u{5199}\u{4f5c}",
    "\u{5de5}\u{4f5c}\u{6d41}",
    "\u{5f53}\u{524d}",
    "\u{5982}\u{679c}",
    "\u{5b9e}\u{9a8c}",
    "\u{6d41}\u{7a0b}",
    "\u{7814}\u{7a76}",
    "\u{8ba1}\u{7b97}\u{673a}",
    "\u{9886}\u{57df}",
    "\u{9700}\u{8981}",
];

const HEADING_TRIGGER_ZH: &str = "## \u{89e6}\u{53d1}\u{6761}\u{4ef6}";
const HEADING_WORKFLOW_ZH: &str = "## \u{6807}\u{51c6}\u{5316}\u{6d41}\u{7a0b}";
const HEADING_ANTIPATTERN_ZH: &str = "## \u{53cd}\u{6a21}\u{5f0f}";
const HEADING_VERIFY_ZH: &str = "## \u{9a8c}\u{8bc1}\u{65b9}\u{6cd5}";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillKind {
    Workflow,
    Subfield,
    General,
}

#[derive(Debug, Clone)]
pub struct AgentSkillDef {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub tools: Vec<String>,
    pub prompt: String,
    pub path: PathBuf,
    pub trigger_hints: Vec<String>,
    pub kind: SkillKind,
}

#[derive(Debug, Clone)]
pub struct AgentSkillMatch {
    pub name: String,
    pub description: String,
    pub kind: SkillKind,
}

#[derive(Debug, Clone)]
pub struct AgentSkillCatalog {
    roots: Vec<PathBuf>,
}

impl AgentSkillCatalog {
    pub fn new() -> Self {
        Self {
            roots: discover_skill_roots(),
        }
    }

    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    pub fn list_skills(&self) -> Vec<AgentSkillDef> {
        let mut loaded = Vec::new();
        let mut seen = HashSet::new();
        for path in self.skill_files() {
            if let Some(skill) = load_skill_from_path(&path) {
                if seen.insert(skill.name.clone()) {
                    loaded.push(skill);
                }
            }
        }
        loaded.sort_by(|left, right| left.name.cmp(&right.name));
        loaded
    }

    pub fn load_skill(&self, name: &str) -> Option<AgentSkillDef> {
        let target = name.trim().to_ascii_lowercase();
        for path in self.skill_files() {
            let file_stem = path
                .file_stem()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase());
            if file_stem.as_deref() != Some(target.as_str()) {
                continue;
            }
            if let Some(skill) = load_skill_from_path(&path) {
                return Some(skill);
            }
        }

        self.list_skills()
            .into_iter()
            .find(|skill| skill.name.eq_ignore_ascii_case(name))
    }

    pub fn auto_match_research_skills(
        &self,
        user_text: &str,
        mode: Option<&str>,
        limit: usize,
    ) -> Vec<AgentSkillDef> {
        if limit == 0 || !should_consider_research_skills(user_text, mode) {
            return Vec::new();
        }

        let user_tokens = tokens_without_stopwords(user_text);
        let mut scored = self
            .list_skills()
            .into_iter()
            .filter(is_research_skill)
            .filter_map(|skill| {
                let score = score_skill(&skill, user_text, &user_tokens, mode);
                if score > 0 {
                    Some((score, skill))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        scored.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.name.cmp(&right.1.name))
        });

        let mut selected = Vec::new();
        let mut has_workflow = false;
        let mut has_subfield = false;
        for (score, skill) in scored {
            if selected.len() >= limit || score < 2 {
                continue;
            }
            match skill.kind {
                SkillKind::Workflow => {
                    if has_workflow {
                        continue;
                    }
                    has_workflow = true;
                }
                SkillKind::Subfield => {
                    if has_subfield {
                        continue;
                    }
                    has_subfield = true;
                }
                SkillKind::General => {}
            }
            selected.push(skill);
        }

        selected.truncate(limit);
        selected
    }

    pub fn auto_match_prompt(&self, user_text: &str, mode: Option<&str>) -> Option<String> {
        if should_select_browser_computer_skill(user_text) {
            if let Some(skill) = self.load_skill("browser-computer-workflow") {
                return Some(render_skill_bundle_prompt(&[skill]));
            }
        }
        let skills = self.auto_match_research_skills(user_text, mode, 2);
        if skills.is_empty() {
            None
        } else {
            Some(render_skill_bundle_prompt(&skills))
        }
    }

    pub fn auto_match_metadata(&self, user_text: &str, mode: Option<&str>) -> Vec<AgentSkillMatch> {
        if should_select_browser_computer_skill(user_text) {
            if let Some(skill) = self.load_skill("browser-computer-workflow") {
                return vec![AgentSkillMatch {
                    name: skill.name,
                    description: skill.description,
                    kind: skill.kind,
                }];
            }
        }
        self.auto_match_research_skills(user_text, mode, 2)
            .into_iter()
            .map(|skill| AgentSkillMatch {
                name: skill.name,
                description: skill.description,
                kind: skill.kind,
            })
            .collect()
    }

    fn skill_files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for root in &self.roots {
            collect_markdown_files(root, &mut files);
        }
        files
    }
}

fn should_select_browser_computer_skill(user_text: &str) -> bool {
    let lowered = user_text.to_ascii_lowercase();
    let mentions_browser = [
        "browser",
        "webpage",
        "website",
        "web page",
        "computer use",
        "浏览器",
        "网页",
        "网站",
        "页面",
    ]
    .iter()
    .any(|needle| lowered.contains(needle));
    let requests_interaction = [
        "open", "navigate", "click", "type", "scroll", "fill", "submit", "inspect", "打开", "访问",
        "点击", "输入", "滚动", "填写", "提交", "操作", "检查",
    ]
    .iter()
    .any(|needle| lowered.contains(needle));
    mentions_browser && requests_interaction
}

pub fn render_skill_bundle_prompt(skills: &[AgentSkillDef]) -> String {
    let summary = skills
        .iter()
        .map(|skill| format!("- {}: {}", skill.name, skill.description))
        .collect::<Vec<_>>()
        .join("\n");
    let bodies = skills
        .iter()
        .map(|skill| format!("[Skill: {}]\n{}", skill.name, skill.prompt))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!(
        "Auto-selected workspace skills:\n{summary}\n\nSkill application policy:\n- Apply these skill instructions when they fit the request.\n- Keep claims evidence-bounded and adapt only when concrete workspace or tool evidence requires it.\n- Prefer the most specific subfield skill plus the most specific workflow skill when both are provided.\n\n{bodies}"
    )
}

fn discover_skill_roots() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(explicit) = std::env::var("TOKITAI_AGENT_SKILLS_DIR") {
        candidates.push(PathBuf::from(explicit));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("skills").join("agents"));
    }
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".claude").join("skills").join("agents"));
        candidates.push(home.join(".codex").join("skills").join("agents"));
    }

    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    for candidate in candidates {
        let normalized = candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.clone());
        if !normalized.exists() || !normalized.is_dir() {
            continue;
        }
        let key = normalized.to_string_lossy().to_ascii_lowercase();
        if seen.insert(key) {
            roots.push(normalized);
        }
    }
    roots
}

fn collect_markdown_files(root: &Path, out: &mut Vec<PathBuf>) {
    if !root.exists() {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, out);
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !name.ends_with(".md") || matches!(name, "CLAUDE.md" | "README.md" | "TEMPLATE.md") {
            continue;
        }
        out.push(path);
    }
}

fn load_skill_from_path(path: &Path) -> Option<AgentSkillDef> {
    let content = std::fs::read_to_string(path).ok()?;
    let frontmatter = parse_frontmatter(&content);
    let body = extract_body(&content);
    let fallback_name = path.file_stem()?.to_string_lossy().to_string();
    let name = frontmatter.name.unwrap_or(fallback_name);
    let description = frontmatter.description.unwrap_or_default();
    let prompt = build_prompt(&name, &description, frontmatter.domain.as_deref(), &body);
    let trigger_hints = extract_trigger_hints(&body);
    let kind = infer_skill_kind(&name, path);

    Some(AgentSkillDef {
        name,
        description,
        domain: frontmatter.domain.unwrap_or_default(),
        tools: frontmatter.tools,
        prompt,
        path: path.to_path_buf(),
        trigger_hints,
        kind,
    })
}

#[derive(Default)]
struct FrontmatterFields {
    name: Option<String>,
    description: Option<String>,
    domain: Option<String>,
    tools: Vec<String>,
}

fn parse_frontmatter(content: &str) -> FrontmatterFields {
    let mut fields = FrontmatterFields::default();
    let Some(frontmatter) = extract_frontmatter(content) else {
        return fields;
    };
    for raw_line in frontmatter.lines() {
        let line = raw_line.trim();
        if let Some(value) = line.strip_prefix("name:") {
            fields.name = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("description:") {
            fields.description = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("domain:") {
            fields.domain = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("tools:") {
            fields.tools = value
                .trim()
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split(',')
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect();
        }
    }
    fields
}

fn extract_frontmatter(content: &str) -> Option<&str> {
    if !content.starts_with("---") {
        return None;
    }
    let end = content[3..].find("---")?;
    Some(&content[3..3 + end])
}

fn extract_body(content: &str) -> String {
    if let Some(end) = content
        .starts_with("---")
        .then(|| content[3..].find("---"))
        .flatten()
    {
        return content[3 + end + 3..].trim().to_string();
    }
    content.trim().to_string()
}

fn build_prompt(name: &str, desc: &str, domain: Option<&str>, body: &str) -> String {
    let mut prompt = String::new();
    prompt.push_str(&format!("You are acting as: **{}**\n", name));
    if !desc.is_empty() {
        prompt.push_str(desc);
        prompt.push_str("\n\n");
    }
    if let Some(domain) = domain {
        if !domain.trim().is_empty() {
            prompt.push_str(&format!("Domain: {}\n\n", domain.trim()));
        }
    }

    if body.chars().count() <= 4000 {
        prompt.push_str(body.trim());
    } else {
        let sections = [
            "## Role & Expertise",
            "## Purpose",
            "## Core Workflows",
            "## Output Standards",
            HEADING_TRIGGER_ZH,
            HEADING_WORKFLOW_ZH,
            HEADING_ANTIPATTERN_ZH,
            HEADING_VERIFY_ZH,
        ];
        let extracted = sections
            .iter()
            .filter_map(|heading| extract_markdown_section(body, heading))
            .collect::<Vec<_>>()
            .join("\n\n");
        if extracted.is_empty() {
            prompt.push_str(body.trim());
        } else {
            prompt.push_str(&extracted);
        }
    }

    prompt.push_str(
        "\n\nFollow the above role and workflow guidance when it fits the request. Keep the response concrete and evidence-bounded.",
    );
    prompt
}

fn extract_markdown_section(body: &str, heading: &str) -> Option<String> {
    let start = body.find(heading)?;
    let remaining = &body[start..];
    let end = remaining[1..]
        .find("\n## ")
        .map(|index| index + 1)
        .unwrap_or(remaining.len());
    Some(remaining[..end].trim().to_string())
}

fn extract_trigger_hints(body: &str) -> Vec<String> {
    extract_markdown_section(body, HEADING_TRIGGER_ZH)
        .or_else(|| extract_markdown_section(body, "## Trigger Conditions"))
        .map(|section| {
            section
                .lines()
                .skip(1)
                .map(str::trim)
                .filter(|line| line.starts_with("- "))
                .map(|line| line.trim_start_matches("- ").trim().to_string())
                .filter(|line| !line.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn infer_skill_kind(name: &str, path: &Path) -> SkillKind {
    let lowered_name = name.to_ascii_lowercase();
    let lowered_path = path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    if lowered_name.ends_with("-workflow") {
        SkillKind::Workflow
    } else if lowered_name.ends_with("-research") || lowered_path.contains("skills/agents/research")
    {
        SkillKind::Subfield
    } else {
        SkillKind::General
    }
}

fn should_consider_research_skills(user_text: &str, mode: Option<&str>) -> bool {
    if matches!(
        mode.unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "research" | "spec"
    ) {
        return true;
    }

    let lowered = user_text.to_ascii_lowercase();
    let cs_markers = [
        "research",
        "paper",
        "literature",
        "benchmark",
        "dataset",
        "ablation",
        "reproduce",
        "rebuttal",
        "reviewer",
        "hypothesis",
        "experiment",
        "theorem",
        "proof",
        "compiler",
        "database",
        "agent evaluation",
        "label noise",
        "\u{8bba}\u{6587}",
        "\u{6587}\u{732e}",
        "\u{57fa}\u{51c6}",
        "\u{6570}\u{636e}\u{96c6}",
        "\u{6d88}\u{878d}",
        "\u{590d}\u{73b0}",
        "\u{5b9e}\u{9a8c}",
        "\u{5047}\u{8bbe}",
        "\u{5ba1}\u{7a3f}",
        "\u{53cd}\u{9a73}",
        "\u{5b9a}\u{7406}",
        "\u{8bc1}\u{660e}",
        "\u{7f16}\u{8bd1}\u{5668}",
        "\u{6570}\u{636e}\u{5e93}",
        "\u{667a}\u{80fd}\u{4f53}\u{8bc4}\u{6d4b}",
    ];

    cs_markers.iter().any(|marker| lowered.contains(marker))
}

fn is_research_skill(skill: &AgentSkillDef) -> bool {
    let lowered = skill
        .path
        .to_string_lossy()
        .replace('\\', "/")
        .to_ascii_lowercase();
    lowered.contains("skills/agents/research") || skill.name.contains("scientist")
}

fn score_skill(
    skill: &AgentSkillDef,
    user_text: &str,
    user_tokens: &HashSet<String>,
    mode: Option<&str>,
) -> i32 {
    let mut score = 0i32;
    let lowered_user = user_text.to_ascii_lowercase();
    let lowered_name = skill.name.to_ascii_lowercase();

    if lowered_user.contains(&lowered_name) {
        score += 20;
    }

    let name_phrase = lowered_name
        .replace("cs-", "")
        .replace("-research", "")
        .replace("-workflow", "")
        .replace('-', " ");
    if !name_phrase.trim().is_empty() && lowered_user.contains(name_phrase.trim()) {
        score += 12;
    }

    for token in tokens_without_stopwords(&skill.name) {
        if user_tokens.contains(&token) {
            score += 5;
        }
    }

    for token in tokens_without_stopwords(&skill.description) {
        if user_tokens.contains(&token) {
            score += 2;
        }
    }

    for hint in &skill.trigger_hints {
        let overlap = tokens_without_stopwords(hint)
            .into_iter()
            .filter(|token| user_tokens.contains(token))
            .count() as i32;
        score += overlap * 2;
    }

    if skill.kind == SkillKind::Workflow
        && matches!(
            mode.unwrap_or_default().to_ascii_lowercase().as_str(),
            "research" | "spec"
        )
    {
        score += 1;
    }

    score
}

fn tokens_without_stopwords(text: &str) -> HashSet<String> {
    TOKEN_REGEX
        .find_iter(text)
        .map(|capture| capture.as_str().to_ascii_lowercase())
        .filter(|token| token.len() >= 2)
        .filter(|token| {
            !EN_STOPWORDS.contains(&token.as_str()) && !ZH_STOPWORDS.contains(&token.as_str())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_paper_writing_workflow() {
        let root = std::env::current_dir()
            .unwrap()
            .join("skills")
            .join("agents");
        let catalog = AgentSkillCatalog::with_roots(vec![root]);
        let matches = catalog.auto_match_research_skills(
            "\u{8bf7}\u{6839}\u{636e}\u{5b9e}\u{9a8c}\u{7ed3}\u{679c}\u{548c}\u{5f15}\u{7528}\u{5199}\u{8bba}\u{6587}\u{ff0c}\u{5e76}\u{4fee}\u{590d} claim evidence \u{5bf9}\u{9f50}\u{95ee}\u{9898}",
            Some("research"),
            2,
        );
        let names = matches
            .iter()
            .map(|skill| skill.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"cs-paper-writing-workflow"));
    }

    #[test]
    fn matches_systems_research_subfield() {
        let root = std::env::current_dir()
            .unwrap()
            .join("skills")
            .join("agents");
        let catalog = AgentSkillCatalog::with_roots(vec![root]);
        let matches = catalog.auto_match_research_skills(
            "Design a database systems benchmark with throughput, tail latency, and scaling evaluation.",
            Some("research"),
            2,
        );
        let names = matches
            .iter()
            .map(|skill| skill.name.as_str())
            .collect::<Vec<_>>();
        assert!(names.contains(&"cs-systems-research"));
    }

    #[test]
    fn selects_browser_computer_workflow_for_explicit_web_interaction() {
        let root = std::env::current_dir()
            .unwrap()
            .join("skills")
            .join("agents");
        let catalog = AgentSkillCatalog::with_roots(vec![root]);
        let matches = catalog.auto_match_metadata(
            "打开浏览器访问示例网站，然后点击页面中的链接",
            Some("agent"),
        );
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].name, "browser-computer-workflow");
    }

    #[test]
    fn does_not_select_browser_computer_for_plain_web_research() {
        assert!(!should_select_browser_computer_skill(
            "Search the web for recent database benchmark papers"
        ));
    }
}
