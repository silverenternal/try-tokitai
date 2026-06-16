//! Research Workspace Manager
//!
//! Auto-creates a workspace directory outside the project root for each
//! research project. Structure:
//!   ~/tokitai_research/{topic_slug}/
//!   ├── code/        # Experiment scripts
//!   ├── data/        # Datasets
//!   ├── results/     # Output files, figures, logs
//!   └── paper/       # Paper drafts
//!
//! The agent writes code, runs experiments, and saves results here.

use anyhow::Result;
use std::path::PathBuf;

/// Manages a research workspace directory
#[derive(Debug, Clone)]
pub struct ResearchWorkspace {
    /// Root directory for all research projects
    pub base_dir: PathBuf,
    /// Current project directory
    pub project_dir: PathBuf,
    /// Subdirectories
    pub code_dir: PathBuf,
    pub data_dir: PathBuf,
    pub results_dir: PathBuf,
    pub paper_dir: PathBuf,
}

impl ResearchWorkspace {
    /// Create a new workspace for a research topic
    pub fn create(topic: &str) -> Result<Self> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let base_dir = home.join("tokitai_research");

        // Create a safe slug: use date + short hash to avoid Chinese chars in paths
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        topic.hash(&mut hasher);
        let hash = hasher.finish();
        let date = chrono::Local::now().format("%Y%m%d_%H%M");
        let slug = format!("research_{}_{:04x}", date, (hash & 0xFFFF) as u16);

        let project_dir = base_dir.join(slug);
        let code_dir = project_dir.join("code");
        let data_dir = project_dir.join("data");
        let results_dir = project_dir.join("results");
        let paper_dir = project_dir.join("paper");

        // Create all directories
        std::fs::create_dir_all(&code_dir)?;
        std::fs::create_dir_all(&data_dir)?;
        std::fs::create_dir_all(&results_dir)?;
        std::fs::create_dir_all(&paper_dir)?;

        // Register workspace as allowed path for tools
        crate::tools::io::security::add_allowed_root(project_dir.clone());
        crate::tools::io::security::add_allowed_root(base_dir.clone());

        // Create a README in the project dir
        let readme = format!(
            "# Research: {}\n\n\
             Created: {}\n\n\
             ## Structure\n\
             - `code/` — Experiment scripts and implementations\n\
             - `data/` — Datasets and data files\n\
             - `results/` — Output files, figures, logs, metrics\n\
             - `paper/` — Paper drafts and writing\n",
            topic,
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        );
        std::fs::write(project_dir.join("README.md"), readme)?;

        Ok(Self {
            base_dir,
            project_dir,
            code_dir,
            data_dir,
            results_dir,
            paper_dir,
        })
    }

    /// Get the workspace context for the system prompt
    pub fn context_for_prompt(&self) -> String {
        format!(
            r#"## Research Workspace
You have a dedicated workspace for this research project:

```
{project}/
├── code/      ← Write experiment scripts here
├── data/      ← Store datasets here
├── results/   ← Save results, figures, logs here
└── paper/     ← Write paper drafts here
```

**Rules for experiments:**
1. Write ALL experiment code to `{code}/`
2. Save ALL results (CSV, JSON, figures) to `{results}/`
3. Use `{data}/` for any datasets you need
4. Write paper drafts to `{paper}/`
5. You can run Python scripts with: `python {code}/script.py`
6. You can install packages with: `pip install <package>`
7. Always check if packages are installed before using them

**Current workspace path:** `{project}`
"#,
            project = self.project_dir.display(),
            code = self.code_dir.display(),
            data = self.data_dir.display(),
            results = self.results_dir.display(),
            paper = self.paper_dir.display(),
        )
    }
}
