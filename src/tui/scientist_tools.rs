//! AI Scientist supplementary tools
//!
//! Statistical tests, citation manager, knowledge graph, error fix loop.

use serde_json::{json, Value};

/// Run statistical analysis on experiment results
pub struct StatsRunner;

impl StatsRunner {
    /// Generate a Python script that runs statistical tests and returns JSON
    pub fn generate_stats_script(data_a: &[f64], data_b: &[f64], labels: (&str, &str)) -> String {
        format!(
            r#"
import numpy as np
from scipy import stats
import json

data_a = np.array({data_a:?})
data_b = np.array({data_b:?})

results = {{}}

# Descriptive stats
results['descriptive'] = {{
    '{label_a}': {{'mean': float(np.mean(data_a)), 'std': float(np.std(data_a)), 'n': len(data_a)}},
    '{label_b}': {{'mean': float(np.mean(data_b)), 'std': float(np.std(data_b)), 'n': len(data_b)}}
}}

# Normality test (Shapiro-Wilk)
_, p_a = stats.shapiro(data_a) if len(data_a) >= 3 and len(data_a) <= 5000 else (0, 1.0)
_, p_b = stats.shapiro(data_b) if len(data_b) >= 3 and len(data_b) <= 5000 else (0, 1.0)
results['normality'] = {{
    '{label_a}': {{'shapiro_p': float(p_a), 'normal': p_a > 0.05}},
    '{label_b}': {{'shapiro_p': float(p_b), 'normal': p_b > 0.05}}
}}

# Student's t-test (independent)
t_stat, t_p = stats.ttest_ind(data_a, data_b)
results['ttest'] = {{
    'statistic': float(t_stat), 'p_value': float(t_p),
    'significant': t_p < 0.05, 'effect_size_cohens_d': float(cohens_d(data_a, data_b))
}}

# Mann-Whitney U (non-parametric)
u_stat, u_p = stats.mannwhitneyu(data_a, data_b, alternative='two-sided')
results['mann_whitney'] = {{
    'statistic': float(u_stat), 'p_value': float(u_p), 'significant': u_p < 0.05
}}

# Effect size (Cohen's d)
def cohens_d(x, y):
    nx, ny = len(x), len(y)
    dof = nx + ny - 2
    pooled_std = np.sqrt(((nx-1)*np.std(x, ddof=1)**2 + (ny-1)*np.std(y, ddof=1)**2) / dof)
    return (np.mean(x) - np.mean(y)) / pooled_std if pooled_std > 0 else 0

# Summary
results['summary'] = ("Statistically significant difference found (p={{:.4f}}, d={{:.3f}})"
    .format(t_p, cohens_d(data_a, data_b)) if t_p < 0.05
    else "No statistically significant difference (p={{:.4f}}, d={{:.3f}})"
    .format(t_p, cohens_d(data_a, data_b)))

print(json.dumps(results, indent=2))
"#,
            data_a = data_a,
            data_b = data_b,
            label_a = labels.0,
            label_b = labels.1,
        )
    }

    /// Generate ANOVA script for multiple groups
    pub fn generate_anova_script(groups: &[(&str, Vec<f64>)]) -> String {
        let mut group_defs = String::new();
        let mut group_names = Vec::new();
        for (name, data) in groups {
            group_defs.push_str(&format!(
                "groups['{name}'] = np.array({data:?})\n",
                name = name, data = data
            ));
            group_names.push(*name);
        }

        format!(
            r#"
import numpy as np
from scipy import stats
import json

groups = {{}}
{group_defs}

# One-way ANOVA
all_data = [groups[name] for name in groups]
f_stat, p_val = stats.f_oneway(*all_data)
results = {{
    'anova': {{'f_statistic': float(f_stat), 'p_value': float(p_val), 'significant': p_val < 0.05}},
    'group_stats': {{}}
}}

for name, data in groups.items():
    results['group_stats'][name] = {{
        'mean': float(np.mean(data)), 'std': float(np.std(data)),
        'min': float(np.min(data)), 'max': float(np.max(data)), 'n': len(data)
    }}

# Pairwise Tukey HSD (if ANOVA significant)
if p_val < 0.05:
    from itertools import combinations
    results['pairwise'] = []
    for (n1, d1), (n2, d2) in combinations(groups.items(), 2):
        t, p = stats.ttest_ind(d1, d2)
        d = (np.mean(d1) - np.mean(d2)) / np.sqrt((np.std(d1)**2 + np.std(d2)**2) / 2) if len(d1)+len(d2)>0 else 0
        results['pairwise'].append({{
            'pair': [n1, n2], 't_stat': float(t), 'p_value': float(p),
            'cohens_d': float(d), 'significant': p < 0.05
        }})

results['summary'] = ("Significant differences found (ANOVA p={{:.4f}})".format(p_val)
    if p_val < 0.05 else "No significant differences (ANOVA p={{:.4f}})".format(p_val))

print(json.dumps(results, indent=2))
"#,
            group_defs = group_defs,
        )
    }

    /// Write stats script to workspace and return the command to run it
    pub fn write_and_run_command(workspace: &str, script_name: &str, script_content: &str) -> String {
        format!(
            "cat > {}/code/{}.py << 'PYEOF'\n{}\nPYEOF\npython {}/code/{}.py",
            workspace, script_name, script_content, workspace, script_name
        )
    }
}

/// Citation and reference manager
pub struct CitationManager;

impl CitationManager {
    /// Format a citation in BibTeX format
    pub fn to_bibtex(
        key: &str, authors: &str, title: &str, venue: &str, year: u32, url: &str,
    ) -> String {
        format!(
            r#"@article{{{key},
  author = {{{authors}}},
  title = {{{title}}},
  journal = {{{venue}}},
  year = {{{year}}},
  url = {{{url}}}
}}"#,
            key = key, authors = authors, title = title, venue = venue, year = year, url = url
        )
    }

    /// Generate a references.bib file from a list of citations
    pub fn generate_bib_file(citations: &[Value]) -> String {
        let mut bib = String::from("% References auto-generated by AI Scientist\n\n");
        for (i, c) in citations.iter().enumerate() {
            let default_key = format!("ref{}", i);
            let key = c.get("key").and_then(|v| v.as_str()).unwrap_or(&default_key);
            let authors = c.get("authors").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let title = c.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled");
            let venue = c.get("venue").and_then(|v| v.as_str()).unwrap_or("arXiv");
            let year = c.get("year").and_then(|v| v.as_u64()).unwrap_or(2024) as u32;
            let url = c.get("url").and_then(|v| v.as_str()).unwrap_or("");
            bib.push_str(&Self::to_bibtex(key, authors, title, venue, year, url));
            bib.push_str("\n\n");
        }
        bib
    }

    /// Format inline citation: "Author et al. (Year)"
    pub fn format_inline(citation: &Value) -> String {
        let authors = citation.get("authors").and_then(|v| v.as_str()).unwrap_or("Unknown");
        let year = citation.get("year").and_then(|v| v.as_u64()).unwrap_or(2024);
        let first_author = authors.split(',').next().unwrap_or(authors).trim();
        if authors.split(',').count() > 1 {
            format!("{} et al. ({})", first_author, year)
        } else {
            format!("{} ({})", first_author, year)
        }
    }
}

/// Simple knowledge graph: (entity, relation, entity) triples
#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    pub entities: Vec<GraphEntity>,
    pub relations: Vec<GraphRelation>,
}

#[derive(Debug, Clone)]
pub struct GraphEntity {
    pub id: String,
    pub name: String,
    pub entity_type: String, // "method", "dataset", "metric", "paper", "task"
}

#[derive(Debug, Clone)]
pub struct GraphRelation {
    pub from: String,
    pub relation: String, // "uses", "outperforms", "evaluated_on", "proposed_in"
    pub to: String,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self { entities: Vec::new(), relations: Vec::new() }
    }

    pub fn add_paper(&mut self, paper_title: &str, methods: &[&str], datasets: &[&str], metrics: &[&str]) {
        let paper_id = format!("paper_{}", self.entities.len());
        self.entities.push(GraphEntity {
            id: paper_id.clone(), name: paper_title.to_string(), entity_type: "paper".to_string(),
        });

        for m in methods {
            let mid = format!("method_{}", self.entities.len());
            self.entities.push(GraphEntity {
                id: mid.clone(), name: m.to_string(), entity_type: "method".to_string(),
            });
            self.relations.push(GraphRelation {
                from: mid.clone(), relation: "proposed_in".to_string(), to: paper_id.clone(),
            });
        }

        for d in datasets {
            let did = format!("dataset_{}", self.entities.len());
            self.entities.push(GraphEntity {
                id: did.clone(), name: d.to_string(), entity_type: "dataset".to_string(),
            });
            self.relations.push(GraphRelation {
                from: paper_id.clone(), relation: "evaluated_on".to_string(), to: did,
            });
        }

        for m in metrics {
            let mid = format!("metric_{}", self.entities.len());
            self.entities.push(GraphEntity {
                id: mid.clone(), name: m.to_string(), entity_type: "metric".to_string(),
            });
            self.relations.push(GraphRelation {
                from: paper_id.clone(), relation: "uses_metric".to_string(), to: mid,
            });
        }
    }

    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("```mermaid\ngraph TD\n");
        for e in &self.entities {
            let icon = match e.entity_type.as_str() {
                "method" => "⚙", "dataset" => "📊", "metric" => "📏", "paper" => "📄", _ => "•"
            };
            out.push_str(&format!("  {}[\"{} {} ({})\"]\n", e.id, icon, e.name, e.entity_type));
        }
        for r in &self.relations {
            out.push_str(&format!("  {} -->|{}| {}\n", r.from, r.relation, r.to));
        }
        out.push_str("```\n");
        out
    }

    pub fn to_json(&self) -> Value {
        json!({
            "entities": self.entities.iter().map(|e| json!({
                "id": e.id, "name": e.name, "type": e.entity_type
            })).collect::<Vec<_>>(),
            "relations": self.relations.iter().map(|r| json!({
                "from": r.from, "relation": r.relation, "to": r.to
            })).collect::<Vec<_>>()
        })
    }
}

/// Error-fix loop for research experiments
pub struct ErrorFixLoop {
    pub max_retries: usize,
    pub retry_count: usize,
    pub errors: Vec<String>,
    pub fixes: Vec<String>,
}

impl ErrorFixLoop {
    pub fn new(max_retries: usize) -> Self {
        Self { max_retries, retry_count: 0, errors: Vec::new(), fixes: Vec::new() }
    }

    /// Record an error and generate a fix prompt
    pub fn analyze_error(&mut self, error: &str, code: &str) -> String {
        self.errors.push(error.to_string());
        self.retry_count += 1;

        if self.retry_count > self.max_retries {
            return format!(
                "Maximum retries ({}) exceeded. Last error: {}\nPlease fix manually.",
                self.max_retries, error
            );
        }

        format!(
            r#"The experiment failed with this error:
```
{}
```

Current code:
```python
{}
```

Fix the code and try again. Common issues:
1. Missing imports (import numpy, scipy, etc.)
2. Package not installed (pip install <pkg>)
3. Wrong file path
4. Data format mismatch
5. API version change

Analyze the error, fix the code, and re-run. This is attempt {}/{}."#,
            error, code, self.retry_count, self.max_retries
        )
    }

    /// Generate the fix prompt for the LLM
    pub fn fix_prompt(&self, error: &str, code_path: &str) -> String {
        format!(
            r#"Error in {}:
{}

Please:
1. Read the file to see the current code
2. Identify the root cause
3. Fix the code
4. Re-run the experiment
5. Report the fix"#,
            code_path, error
        )
    }
}

impl Default for ErrorFixLoop {
    fn default() -> Self { Self::new(3) }
}
