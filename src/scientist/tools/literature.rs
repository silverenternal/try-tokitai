//! Literature Tools - paper search, fetch, and citation
//!
//! Local-first implementation:
//! - search local markdown/pdf files first
//! - provide explicit fallback errors for remote APIs when not configured

use ai_scientist_rag::PdfParser;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tokitai::tool;

pub struct LiteratureTools;

#[derive(Debug, Clone)]
struct LocalPaperRecord {
    path: PathBuf,
    title: String,
    snippet: String,
    source: String,
}

fn papers_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    if let Ok(dir) = std::env::var("AI_SCIENTIST_PAPERS_DIR") {
        roots.push(PathBuf::from(dir));
    }

    if let Ok(cwd) = std::env::current_dir() {
        roots.push(cwd.join("papers"));
        roots.push(cwd.join("docs"));
        roots.push(cwd.join("downloads"));
        roots.push(cwd);
    }

    roots
}

fn tokenize(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(|s| s.to_string())
        .collect()
}

fn score_text(query_tokens: &[String], text: &str) -> usize {
    let text_lower = text.to_lowercase();
    query_tokens
        .iter()
        .map(|token| text_lower.matches(token).count())
        .sum()
}

fn extract_title_from_md(content: &str, fallback: &str) -> String {
    content
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("# "))
        .map(|line| line.trim_start_matches("# ").trim().to_string())
        .unwrap_or_else(|| fallback.to_string())
}

fn read_text_excerpt(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let excerpt = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .join(" ");
    if excerpt.is_empty() { None } else { Some(excerpt) }
}

fn search_local_papers(query: &str, limit: usize) -> Vec<LocalPaperRecord> {
    let query_tokens = tokenize(query);
    let mut candidates: Vec<(usize, LocalPaperRecord)> = Vec::new();

    for root in papers_roots() {
        if !root.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();

            if ext != "md" && ext != "markdown" && ext != "txt" && ext != "pdf" {
                continue;
            }

            let file_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");

            let mut title = file_name.to_string();
            let mut snippet = String::new();
            let mut source = ext.clone();
            let mut content_for_scoring = file_name.to_string();

            if ext == "pdf" {
                let parser = PdfParser::new();
                if let Ok(parsed) = parser.parse(path) {
                    title = parsed.title.unwrap_or_else(|| file_name.to_string());
                    snippet = parsed.abstract_text.clone().or_else(|| {
                        Some(
                            parsed
                                .body_text
                                .lines()
                                .map(str::trim)
                                .filter(|line| !line.is_empty())
                                .take(6)
                                .collect::<Vec<_>>()
                                .join(" "),
                        )
                    }).unwrap_or_default();
                    content_for_scoring = format!(
                        "{} {} {}",
                        title,
                        parsed.abstract_text.unwrap_or_default(),
                        parsed.body_text
                    );
                    source = "pdf".to_string();
                }
            } else if let Ok(content) = fs::read_to_string(path) {
                title = extract_title_from_md(&content, file_name);
                snippet = content
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty() && !line.starts_with('#'))
                    .take(6)
                    .collect::<Vec<_>>()
                    .join(" ");
                content_for_scoring = format!("{} {}", title, content);
                source = ext;
            }

            let score = score_text(&query_tokens, &content_for_scoring);
            if score > 0 {
                candidates.push((
                    score,
                    LocalPaperRecord {
                        path: path.to_path_buf(),
                        title,
                        snippet,
                        source,
                    },
                ));
            }
        }
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().take(limit).map(|(_, rec)| rec).collect()
}

#[tool]
impl LiteratureTools {
    /// Search academic papers across local files first, then report remote fallback.
    pub fn search_paper(
        &self,
        query: String,
        source: Option<String>,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let source = source.unwrap_or_else(|| "local".into());
        let limit = limit.unwrap_or(10).min(50);

        let local_results = search_local_papers(&query, limit);
        if !local_results.is_empty() {
            return Ok(serde_json::json!({
                "status": "success",
                "mode": "local",
                "query": query,
                "source": source,
                "total": local_results.len(),
                "results": local_results.into_iter().map(|paper| {
                    serde_json::json!({
                        "title": paper.title,
                        "path": paper.path.to_string_lossy(),
                        "source": paper.source,
                        "snippet": paper.snippet,
                    })
                }).collect::<Vec<_>>()
            }));
        }

        let remote_enabled = std::env::var("ARXIV_API_URL").is_ok()
            || std::env::var("SEMANTIC_SCHOLAR_API_KEY").is_ok()
            || std::env::var("CROSSREF_API_URL").is_ok();

        if remote_enabled {
            Err(format!(
                "search_paper: remote API is configured but not implemented yet for query '{}'.",
                query
            ))
        } else {
            Err(format!(
                "search_paper: no local paper matched query '{}', and no remote API is configured.\n\
                 Put papers in ./papers, ./docs, ./downloads, or set AI_SCIENTIST_PAPERS_DIR.",
                query
            ))
        }
    }

    /// Fetch a paper's full text and metadata by DOI or arXiv ID, or from local cache.
    pub fn fetch_paper(&self, paper_id: String) -> Result<Value, String> {
        for root in papers_roots() {
            if !root.exists() {
                continue;
            }

            for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                if stem != paper_id && file_name != paper_id && path.to_string_lossy().contains(&paper_id) {
                    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                    if ext == "pdf" {
                        let parser = PdfParser::new();
                        if let Ok(parsed) = parser.parse(path) {
                            return Ok(serde_json::json!({
                                "status": "success",
                                "mode": "local",
                                "paper_id": paper_id,
                                "path": path.to_string_lossy(),
                                "title": parsed.title,
                                "authors": parsed.authors,
                                "abstract": parsed.abstract_text,
                                "body_text": parsed.body_text,
                                "sections": parsed.sections,
                                "references": parsed.references,
                                "year": parsed.year,
                                "doi": parsed.doi,
                                "page_count": parsed.page_count,
                                "file_hash": parsed.file_hash
                            }));
                        }
                    } else if let Ok(content) = fs::read_to_string(path) {
                        return Ok(serde_json::json!({
                            "status": "success",
                            "mode": "local",
                            "paper_id": paper_id,
                            "path": path.to_string_lossy(),
                            "content": content,
                            "title": extract_title_from_md(&content, stem),
                        }));
                    }
                }
            }
        }

        Err(format!(
            "fetch_paper: local cache miss for '{}'. Configure arXiv/CrossRef/Unpaywall API for remote fetch.",
            paper_id
        ))
    }

    /// Generate a citation in specified format.
    pub fn cite_paper(&self, paper_id: String, format: Option<String>) -> Result<Value, String> {
        let fmt = format.unwrap_or_else(|| "bibtex".into());

        if paper_id.starts_with("10.") || paper_id.contains('/') {
            Ok(serde_json::json!({
                "status": "partial",
                "operation": "cite_paper",
                "paper_id": paper_id,
                "format": fmt,
                "warning": "Citation format is basic - use external API for complete metadata",
                "citation": format!(
                    "@article{{{},\n  title={{[Title not resolved]}},\n  author={{[Authors not resolved]}},\n  doi={{{}}},\n  note={{Citation generated by AI Scientist - verify before use}}\n}}",
                    paper_id.replace(['.', '/'], "_"),
                    paper_id
                )
            }))
        } else {
            Err(format!(
                "cite_paper: Cannot generate citation for ID '{}'. Provide a valid DOI or arXiv ID.",
                paper_id
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_local_search_finds_markdown_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let paper_path = temp_dir.path().join("quantum_notes.md");
        let mut file = fs::File::create(&paper_path).unwrap();
        writeln!(
            file,
            "# Quantum Notes\n\nThis paper discusses quantum computing and verification."
        )
        .unwrap();

        let results = search_local_papers("quantum computing", 5);

        assert!(!results.is_empty());
        assert!(results[0].title.contains("Quantum"));
        assert!(results[0].snippet.contains("quantum computing"));
    }

    #[test]
    fn test_fetch_paper_local_cache_miss_is_clear() {
        let tool = LiteratureTools;
        let err = tool.fetch_paper("missing-paper".into()).unwrap_err();
        assert!(err.contains("local cache miss"));
    }
}
