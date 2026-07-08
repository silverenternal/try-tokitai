//! Literature Tools - paper search, fetch, and citation
//!
//! The CS IDE uses a remote-first literature workflow:
//! - search mainstream paper APIs first with a unified `cs_paper_v1` schema
//! - hydrate accessible full text from remote PDF or landing-page sources whenever possible
//! - fall back to local markdown/pdf/txt notes only when remote retrieval misses
//! - preserve provider-specific IDs so later workflow steps can fetch or re-verify metadata

use ai_scientist_rag::{parser::ParsedPaper, PdfParser};
use chrono::Datelike;
use reqwest::blocking::{Client, RequestBuilder};
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::Builder as TempFileBuilder;
use tokitai::tool;

pub struct LiteratureTools;

const PAPER_SCHEMA_VERSION: &str = "cs_paper_v1";
const DEFAULT_ARXIV_API_URL: &str = "https://export.arxiv.org/api/query";
const DEFAULT_SEMANTIC_SCHOLAR_API_URL: &str = "https://api.semanticscholar.org/graph/v1";
const DEFAULT_CROSSREF_API_URL: &str = "https://api.crossref.org";
const DEFAULT_OPENALEX_API_URL: &str = "https://api.openalex.org";
const DEFAULT_OPENREVIEW_API_URL: &str = "https://api2.openreview.net";
const DEFAULT_ACL_ANTHOLOGY_API_URL: &str = "https://aclanthology.org";
const DEFAULT_UNPAYWALL_API_URL: &str = "https://api.unpaywall.org/v2";

#[derive(Debug, Clone)]
struct LocalPaperRecord {
    paper_id: String,
    path: PathBuf,
    title: String,
    snippet: String,
    source: String,
}

#[derive(Debug, Clone, Serialize, Default)]
struct PaperExternalIds {
    doi: Option<String>,
    arxiv_id: Option<String>,
    semantic_scholar_id: Option<String>,
    openalex_id: Option<String>,
    openreview_id: Option<String>,
    acl_anthology_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct PaperUrls {
    landing_page: Option<String>,
    pdf: Option<String>,
    local_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UnifiedPaperRecord {
    paper_id: String,
    title: String,
    authors: Vec<String>,
    abstract_text: Option<String>,
    snippet: Option<String>,
    venue: Option<String>,
    year: Option<u32>,
    provider: String,
    source_format: String,
    external_ids: PaperExternalIds,
    urls: PaperUrls,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RemoteProvider {
    Arxiv,
    SemanticScholar,
    Crossref,
    OpenAlex,
    OpenReview,
    AclAnthology,
}

impl RemoteProvider {
    fn as_str(self) -> &'static str {
        match self {
            Self::Arxiv => "arxiv",
            Self::SemanticScholar => "semantic_scholar",
            Self::Crossref => "crossref",
            Self::OpenAlex => "openalex",
            Self::OpenReview => "openreview",
            Self::AclAnthology => "acl_anthology",
        }
    }
}

#[derive(Debug, Clone)]
struct RemotePaperFetch {
    provider: String,
    paper: UnifiedPaperRecord,
    content: Option<String>,
    abstract_text: Option<String>,
    raw_metadata: Value,
    content_hydration: Option<RemoteContentHydration>,
}

#[derive(Debug, Clone, Serialize)]
struct RemoteContentHydration {
    status: String,
    source: String,
    source_url: String,
    format: String,
    parser: String,
    attempted_pdf_url: Option<String>,
    downloaded_bytes: usize,
    body_text: String,
    sections: Vec<String>,
    section_blocks: Vec<StructuredSectionBlock>,
    references: Vec<String>,
    page_count: usize,
    file_hash: String,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct StructuredSectionBlock {
    index: usize,
    title: String,
    level: usize,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct StructuredReferenceEntry {
    index: usize,
    text: String,
}

#[derive(Debug, Clone, Serialize)]
struct StructuredDocumentProvenance {
    source_preference: String,
    primary_source: String,
    provider: String,
    content_source: Option<String>,
    attempted_pdf_url: Option<String>,
    source_url: Option<String>,
    format: Option<String>,
    parser: Option<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct StructuredDocumentQuality {
    completeness: String,
    extraction_path: String,
    has_full_body_text: bool,
    has_section_structure: bool,
    has_references: bool,
    body_text_chars: usize,
    section_count: usize,
    reference_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct StructuredPaperDocument {
    schema_version: String,
    paper_schema_version: String,
    paper_id: String,
    provider: String,
    title: String,
    authors: Vec<String>,
    abstract_text: Option<String>,
    body_text: Option<String>,
    sections: Vec<StructuredSectionBlock>,
    references: Vec<StructuredReferenceEntry>,
    venue: Option<String>,
    year: Option<u32>,
    page_count: Option<usize>,
    file_hash: Option<String>,
    external_ids: PaperExternalIds,
    urls: PaperUrls,
    provenance: StructuredDocumentProvenance,
    quality: StructuredDocumentQuality,
}

#[derive(Debug, Clone)]
struct HtmlExtractResult {
    body_text: String,
    sections: Vec<String>,
    section_blocks: Vec<StructuredSectionBlock>,
    references: Vec<String>,
}

#[derive(Debug, Clone)]
struct RemoteClients {
    client: Client,
    arxiv_api_url: String,
    semantic_scholar_api_url: String,
    semantic_scholar_api_key: Option<String>,
    crossref_api_url: String,
    crossref_mailto: Option<String>,
    openalex_api_url: String,
    openalex_mailto: Option<String>,
    openreview_api_url: String,
    acl_anthology_api_url: String,
    unpaywall_api_url: String,
    unpaywall_email: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SemanticScholarSearchResponse {
    #[serde(default)]
    data: Vec<SemanticScholarPaper>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SemanticScholarPaper {
    #[serde(rename = "paperId")]
    paper_id: Option<String>,
    #[serde(default)]
    title: String,
    #[serde(rename = "abstract")]
    abstract_text: Option<String>,
    #[serde(default)]
    authors: Vec<SemanticScholarAuthor>,
    venue: Option<String>,
    year: Option<u32>,
    #[serde(rename = "externalIds", default)]
    external_ids: HashMap<String, String>,
    url: Option<String>,
    #[serde(rename = "openAccessPdf")]
    open_access_pdf: Option<SemanticScholarPdf>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SemanticScholarAuthor {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct SemanticScholarPdf {
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossrefSearchResponse {
    message: CrossrefMessage,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossrefMessage {
    #[serde(default)]
    items: Vec<CrossrefWork>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossrefWork {
    #[serde(rename = "DOI")]
    doi: Option<String>,
    #[serde(default)]
    title: Vec<String>,
    #[serde(default)]
    author: Vec<CrossrefAuthor>,
    #[serde(rename = "container-title", default)]
    container_title: Vec<String>,
    #[serde(rename = "abstract")]
    abstract_field: Option<String>,
    issued: Option<CrossrefDateParts>,
    published: Option<CrossrefDateParts>,
    #[serde(rename = "published-print")]
    published_print: Option<CrossrefDateParts>,
    resource: Option<CrossrefResource>,
    #[serde(rename = "URL")]
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossrefAuthor {
    given: Option<String>,
    family: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossrefDateParts {
    #[serde(rename = "date-parts")]
    date_parts: Vec<Vec<u32>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossrefResource {
    primary: Option<CrossrefPrimaryResource>,
}

#[derive(Debug, Deserialize, Serialize)]
struct CrossrefPrimaryResource {
    #[serde(rename = "URL")]
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAlexSearchResponse {
    #[serde(default)]
    results: Vec<OpenAlexWork>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAlexWork {
    id: Option<String>,
    doi: Option<String>,
    title: Option<String>,
    display_name: Option<String>,
    publication_year: Option<u32>,
    #[serde(default)]
    authorships: Vec<OpenAlexAuthorship>,
    primary_location: Option<OpenAlexLocation>,
    best_oa_location: Option<OpenAlexLocation>,
    open_access: Option<OpenAlexOpenAccess>,
    abstract_inverted_index: Option<HashMap<String, Vec<usize>>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAlexAuthorship {
    author: Option<OpenAlexAuthor>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAlexAuthor {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAlexLocation {
    landing_page_url: Option<String>,
    pdf_url: Option<String>,
    source: Option<OpenAlexSource>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAlexSource {
    display_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenAlexOpenAccess {
    oa_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UnpaywallRecord {
    best_oa_location: Option<UnpaywallLocation>,
}

#[derive(Debug, Deserialize)]
struct UnpaywallLocation {
    url: Option<String>,
    url_for_pdf: Option<String>,
}

fn derive_local_paper_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .or_else(|| path.file_name().and_then(|s| s.to_str()))
        .unwrap_or("unknown-paper")
        .to_string()
}

fn looks_like_doi(value: &str) -> bool {
    value.starts_with("10.") && value.contains('/')
}

fn looks_like_arxiv_id(value: &str) -> bool {
    let trimmed = value
        .strip_prefix("arXiv:")
        .or_else(|| value.strip_prefix("arxiv:"))
        .unwrap_or(value);
    let mut parts = trimmed.split('.');
    let left = parts.next().unwrap_or("");
    let right = parts.next().unwrap_or("");
    !left.is_empty()
        && !right.is_empty()
        && parts.next().is_none()
        && left.chars().all(|c| c.is_ascii_digit())
        && right.chars().all(|c| c.is_ascii_digit() || c.eq(&'v'))
}

fn normalize_doi(value: &str) -> String {
    value
        .trim()
        .trim_start_matches("https://doi.org/")
        .trim_start_matches("http://doi.org/")
        .trim_start_matches("doi:")
        .to_string()
}

fn short_openalex_id(value: &str) -> String {
    value.rsplit('/').next().unwrap_or(value).to_string()
}

fn external_ids_from_primary_id(primary_id: &str, doi: Option<String>) -> PaperExternalIds {
    let mut ids = PaperExternalIds {
        doi: doi.map(|d| normalize_doi(&d)),
        ..PaperExternalIds::default()
    };

    if ids.doi.is_none() && looks_like_doi(primary_id) {
        ids.doi = Some(normalize_doi(primary_id));
    }

    if looks_like_arxiv_id(primary_id) {
        ids.arxiv_id = Some(
            primary_id
                .strip_prefix("arXiv:")
                .or_else(|| primary_id.strip_prefix("arxiv:"))
                .unwrap_or(primary_id)
                .to_string(),
        );
    }

    ids
}

fn local_urls(path: &Path) -> PaperUrls {
    PaperUrls {
        local_path: Some(path.to_string_lossy().to_string()),
        ..PaperUrls::default()
    }
}

fn build_local_search_paper(record: &LocalPaperRecord) -> UnifiedPaperRecord {
    UnifiedPaperRecord {
        paper_id: record.paper_id.clone(),
        title: record.title.clone(),
        authors: Vec::new(),
        abstract_text: None,
        snippet: Some(record.snippet.clone()),
        venue: None,
        year: None,
        provider: "local".to_string(),
        source_format: record.source.clone(),
        external_ids: external_ids_from_primary_id(&record.paper_id, None),
        urls: local_urls(&record.path),
    }
}

fn build_local_markdown_paper(
    paper_id: &str,
    path: &Path,
    title: String,
    content: &str,
) -> UnifiedPaperRecord {
    let snippet = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .take(6)
        .collect::<Vec<_>>()
        .join(" ");

    UnifiedPaperRecord {
        paper_id: paper_id.to_string(),
        title,
        authors: Vec::new(),
        abstract_text: None,
        snippet: if snippet.is_empty() {
            None
        } else {
            Some(snippet)
        },
        venue: None,
        year: None,
        provider: "local".to_string(),
        source_format: path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("txt")
            .to_lowercase(),
        external_ids: external_ids_from_primary_id(paper_id, None),
        urls: local_urls(path),
    }
}

fn build_local_pdf_paper(paper_id: &str, path: &Path, parsed: &ParsedPaper) -> UnifiedPaperRecord {
    let title = parsed
        .title
        .clone()
        .unwrap_or_else(|| derive_local_paper_id(path));

    let snippet = parsed.abstract_text.clone().or_else(|| {
        let body_excerpt = parsed
            .body_text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .take(4)
            .collect::<Vec<_>>()
            .join(" ");
        if body_excerpt.is_empty() {
            None
        } else {
            Some(body_excerpt)
        }
    });

    UnifiedPaperRecord {
        paper_id: paper_id.to_string(),
        title,
        authors: parsed.authors.clone(),
        abstract_text: parsed.abstract_text.clone(),
        snippet,
        venue: None,
        year: parsed.year,
        provider: "local".to_string(),
        source_format: "pdf".to_string(),
        external_ids: external_ids_from_primary_id(paper_id, parsed.doi.clone()),
        urls: local_urls(path),
    }
}

fn papers_roots() -> Vec<PathBuf> {
    if let Ok(dir) = std::env::var("AI_SCIENTIST_PAPERS_DIR") {
        return vec![PathBuf::from(dir)];
    }

    let mut roots = Vec::new();

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

fn search_local_papers(query: &str, limit: usize) -> Vec<LocalPaperRecord> {
    let query_tokens = tokenize(query);
    let mut candidates: Vec<(usize, LocalPaperRecord)> = Vec::new();

    for root in papers_roots() {
        if !root.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(Result::ok)
        {
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
                    snippet = parsed
                        .abstract_text
                        .clone()
                        .or_else(|| {
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
                        })
                        .unwrap_or_default();
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
                        paper_id: derive_local_paper_id(path),
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
    candidates
        .into_iter()
        .take(limit)
        .map(|(_, rec)| rec)
        .collect()
}

impl RemoteClients {
    fn from_env() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(25))
            .connect_timeout(Duration::from_secs(10))
            .user_agent("ai-assistant/0.1 literature")
            .build()
            .map_err(|e| format!("failed to build literature HTTP client: {}", e))?;

        Ok(Self {
            client,
            arxiv_api_url: std::env::var("ARXIV_API_URL")
                .unwrap_or_else(|_| DEFAULT_ARXIV_API_URL.to_string()),
            semantic_scholar_api_url: std::env::var("SEMANTIC_SCHOLAR_API_URL")
                .unwrap_or_else(|_| DEFAULT_SEMANTIC_SCHOLAR_API_URL.to_string()),
            semantic_scholar_api_key: std::env::var("SEMANTIC_SCHOLAR_API_KEY").ok(),
            crossref_api_url: std::env::var("CROSSREF_API_URL")
                .unwrap_or_else(|_| DEFAULT_CROSSREF_API_URL.to_string()),
            crossref_mailto: std::env::var("CROSSREF_MAILTO").ok(),
            openalex_api_url: std::env::var("OPENALEX_API_URL")
                .unwrap_or_else(|_| DEFAULT_OPENALEX_API_URL.to_string()),
            openalex_mailto: std::env::var("OPENALEX_MAILTO").ok(),
            openreview_api_url: std::env::var("OPENREVIEW_API_URL")
                .unwrap_or_else(|_| DEFAULT_OPENREVIEW_API_URL.to_string()),
            acl_anthology_api_url: std::env::var("ACL_ANTHOLOGY_API_URL")
                .unwrap_or_else(|_| DEFAULT_ACL_ANTHOLOGY_API_URL.to_string()),
            unpaywall_api_url: std::env::var("UNPAYWALL_API_URL")
                .unwrap_or_else(|_| DEFAULT_UNPAYWALL_API_URL.to_string()),
            unpaywall_email: std::env::var("UNPAYWALL_EMAIL").ok(),
        })
    }

    fn providers_for_source(&self, source: &str) -> Vec<RemoteProvider> {
        let normalized = source.trim().to_lowercase();
        match normalized.as_str() {
            "arxiv" => vec![RemoteProvider::Arxiv],
            "semantic_scholar" | "semanticscholar" | "s2" => {
                vec![RemoteProvider::SemanticScholar]
            }
            "crossref" => vec![RemoteProvider::Crossref],
            "openalex" => vec![RemoteProvider::OpenAlex],
            "openreview" => vec![RemoteProvider::OpenReview],
            "acl" | "acl_anthology" | "acl-anthology" | "acl anthology" => {
                vec![RemoteProvider::AclAnthology]
            }
            "local" => Vec::new(),
            _ => vec![
                RemoteProvider::SemanticScholar,
                RemoteProvider::OpenAlex,
                RemoteProvider::Arxiv,
                RemoteProvider::Crossref,
                RemoteProvider::OpenReview,
            ],
        }
    }

    fn with_semantic_scholar_headers(&self, request: RequestBuilder) -> RequestBuilder {
        if let Some(api_key) = &self.semantic_scholar_api_key {
            request.header("x-api-key", api_key)
        } else {
            request
        }
    }

    fn search_remote_papers(
        &self,
        query: &str,
        source: &str,
        limit: usize,
    ) -> Result<(Vec<UnifiedPaperRecord>, Vec<String>), String> {
        let providers = self.providers_for_source(source);
        if providers.is_empty() {
            return Err(format!(
                "search_paper: no local paper matched query '{}'. Put papers in ./papers, ./docs, ./downloads, or set AI_SCIENTIST_PAPERS_DIR.",
                query
            ));
        }

        let mut all_results = Vec::new();
        let mut errors = Vec::new();

        for provider in providers {
            let provider_limit = limit.min(10);
            let result = match provider {
                RemoteProvider::Arxiv => self.search_arxiv(query, provider_limit),
                RemoteProvider::SemanticScholar => {
                    self.search_semantic_scholar(query, provider_limit)
                }
                RemoteProvider::Crossref => self.search_crossref(query, provider_limit),
                RemoteProvider::OpenAlex => self.search_openalex(query, provider_limit),
                RemoteProvider::OpenReview => self.search_openreview(query, provider_limit),
                RemoteProvider::AclAnthology => self.search_acl_anthology(query, provider_limit),
            };

            match result {
                Ok(mut provider_results) => all_results.append(&mut provider_results),
                Err(err) => errors.push(format!("{}: {}", provider.as_str(), err)),
            }
        }

        if let Some(records) = dedupe_papers(all_results, limit) {
            Ok((records, errors))
        } else if errors.is_empty() {
            Err(format!(
                "search_paper: no remote paper matched query '{}'.",
                query
            ))
        } else {
            Err(format!(
                "search_paper: no remote paper matched query '{}'. Provider errors: {}",
                query,
                errors.join(" | ")
            ))
        }
    }

    fn search_arxiv(&self, query: &str, limit: usize) -> Result<Vec<UnifiedPaperRecord>, String> {
        let encoded_query = urlencoding::encode(query);
        let url = format!(
            "{}?search_query=all:{}&start=0&max_results={}&sortBy=relevance&sortOrder=descending",
            self.arxiv_api_url, encoded_query, limit
        );

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let body = response
            .text()
            .map_err(|e| format!("read body failed: {}", e))?;
        Ok(parse_arxiv_records(&body, limit))
    }

    fn search_semantic_scholar(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<UnifiedPaperRecord>, String> {
        let fields = "title,abstract,authors,venue,year,externalIds,url,openAccessPdf";
        let url = format!(
            "{}/paper/search?query={}&limit={}&fields={}",
            self.semantic_scholar_api_url,
            urlencoding::encode(query),
            limit,
            fields
        );
        let request = self.client.get(&url);
        let response = self
            .with_semantic_scholar_headers(request)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: SemanticScholarSearchResponse = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        Ok(payload
            .data
            .iter()
            .map(build_semantic_scholar_record)
            .collect())
    }

    fn search_crossref(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<UnifiedPaperRecord>, String> {
        let mut url = format!(
            "{}/works?query.bibliographic={}&rows={}",
            self.crossref_api_url,
            urlencoding::encode(query),
            limit
        );
        if let Some(mailto) = &self.crossref_mailto {
            url.push_str("&mailto=");
            url.push_str(&urlencoding::encode(mailto));
        }
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: CrossrefSearchResponse = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        Ok(payload
            .message
            .items
            .iter()
            .map(build_crossref_record)
            .collect())
    }

    fn search_openalex(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<UnifiedPaperRecord>, String> {
        let mut url = format!(
            "{}/works?search={}&per-page={}",
            self.openalex_api_url,
            urlencoding::encode(query),
            limit
        );
        if let Some(mailto) = &self.openalex_mailto {
            url.push_str("&mailto=");
            url.push_str(&urlencoding::encode(mailto));
        }
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: OpenAlexSearchResponse = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        Ok(payload.results.iter().map(build_openalex_record).collect())
    }

    fn search_openreview(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<UnifiedPaperRecord>, String> {
        let url = format!(
            "{}/notes?term={}&limit={}",
            self.openreview_api_url,
            urlencoding::encode(query),
            limit
        );
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: Value = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        Ok(parse_openreview_search_results(&payload))
    }

    fn search_acl_anthology(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<UnifiedPaperRecord>, String> {
        let url = format!(
            "{}/search/?q={}",
            self.acl_anthology_api_url.trim_end_matches('/'),
            urlencoding::encode(query)
        );
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let body = response
            .text()
            .map_err(|e| format!("read body failed: {}", e))?;
        Ok(parse_acl_anthology_search_results(&body, limit))
    }

    fn fetch_remote_paper(&self, paper_id: &str) -> Result<RemotePaperFetch, String> {
        let normalized = paper_id.trim();

        if let Some(id) = normalized.strip_prefix("arxiv:") {
            return self.fetch_arxiv_by_id(id);
        }
        if let Some(id) = normalized.strip_prefix("arXiv:") {
            return self.fetch_arxiv_by_id(id);
        }
        if let Some(id) = normalized.strip_prefix("doi:") {
            return self.fetch_by_doi(id);
        }
        if let Some(id) = normalized.strip_prefix("s2:") {
            return self.fetch_semantic_scholar_by_id(id);
        }
        if let Some(id) = normalized.strip_prefix("openalex:") {
            return self.fetch_openalex_by_id(id);
        }
        if let Some(id) = normalized.strip_prefix("openreview:") {
            return self.fetch_openreview_by_id(id);
        }
        if looks_like_doi(normalized) {
            return self.fetch_by_doi(normalized);
        }
        if looks_like_arxiv_id(normalized) {
            return self.fetch_arxiv_by_id(
                normalized
                    .strip_prefix("arXiv:")
                    .or_else(|| normalized.strip_prefix("arxiv:"))
                    .unwrap_or(normalized),
            );
        }
        if normalized.starts_with("W") && normalized.len() >= 2 {
            if let Ok(record) = self.fetch_openalex_by_id(normalized) {
                return Ok(record);
            }
        }

        let mut errors = Vec::new();

        match self.fetch_semantic_scholar_by_id(normalized) {
            Ok(result) => return Ok(result),
            Err(err) => errors.push(format!("semantic_scholar: {}", err)),
        }
        match self.fetch_openreview_by_id(normalized) {
            Ok(result) => return Ok(result),
            Err(err) => errors.push(format!("openreview: {}", err)),
        }
        match self.fetch_openalex_by_id(normalized) {
            Ok(result) => return Ok(result),
            Err(err) => errors.push(format!("openalex: {}", err)),
        }

        Err(format!(
            "fetch_paper: unable to resolve remote paper '{}'. {}",
            paper_id,
            errors.join(" | ")
        ))
    }

    fn hydrate_remote_content(&self, paper: &UnifiedPaperRecord) -> Option<RemoteContentHydration> {
        if let Some(pdf_url) = &paper.urls.pdf {
            if let Ok(hydration) = self.download_and_parse_pdf(pdf_url) {
                return Some(hydration);
            }
        }

        if let Some(landing_page) = &paper.urls.landing_page {
            if let Ok(Some(pdf_url)) = self.discover_pdf_from_landing_page(landing_page) {
                if let Ok(hydration) = self.download_and_parse_pdf(&pdf_url) {
                    return Some(RemoteContentHydration {
                        source: "remote_pdf_discovered".to_string(),
                        source_url: pdf_url,
                        warnings: vec![
                            "PDF link was discovered from the landing page before parsing."
                                .to_string(),
                        ],
                        ..hydration
                    });
                }
            }
            if let Ok(hydration) = self.fetch_and_extract_text_page(landing_page) {
                return Some(hydration);
            }
        }

        None
    }

    fn discover_pdf_from_landing_page(
        &self,
        landing_page_url: &str,
    ) -> Result<Option<String>, String> {
        let response = self
            .client
            .get(landing_page_url)
            .send()
            .map_err(|e| format!("landing page request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("landing page HTTP {}", response.status()));
        }
        let body = response
            .text()
            .map_err(|e| format!("landing page read failed: {}", e))?;
        Ok(discover_pdf_url_from_html(landing_page_url, &body))
    }

    fn download_and_parse_pdf(&self, pdf_url: &str) -> Result<RemoteContentHydration, String> {
        let response = self
            .client
            .get(pdf_url)
            .send()
            .map_err(|e| format!("PDF request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("PDF HTTP {}", response.status()));
        }

        let bytes = response
            .bytes()
            .map_err(|e| format!("PDF read failed: {}", e))?;
        if bytes.is_empty() {
            return Err("downloaded PDF is empty".to_string());
        }

        let mut temp_file = TempFileBuilder::new()
            .prefix("tokitai-paper-")
            .suffix(".pdf")
            .tempfile()
            .map_err(|e| format!("failed to create temp PDF: {}", e))?;
        use std::io::Write as _;
        temp_file
            .write_all(&bytes)
            .map_err(|e| format!("failed to write temp PDF: {}", e))?;

        let parser = PdfParser::new();
        let parsed = parser
            .parse(temp_file.path())
            .map_err(|e| format!("PDF parse failed: {}", e))?;
        let section_blocks =
            infer_structured_sections_from_text(&parsed.body_text, Some(&parsed.sections));

        Ok(RemoteContentHydration {
            status: "success".to_string(),
            source: "remote_pdf".to_string(),
            source_url: pdf_url.to_string(),
            format: "pdf".to_string(),
            parser: "pdftotext".to_string(),
            attempted_pdf_url: Some(pdf_url.to_string()),
            downloaded_bytes: bytes.len(),
            body_text: parsed.body_text,
            sections: parsed.sections,
            section_blocks,
            references: parsed.references,
            page_count: parsed.page_count,
            file_hash: parsed.file_hash,
            warnings: Vec::new(),
        })
    }

    fn fetch_and_extract_text_page(
        &self,
        landing_page_url: &str,
    ) -> Result<RemoteContentHydration, String> {
        let response = self
            .client
            .get(landing_page_url)
            .send()
            .map_err(|e| format!("landing page request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("landing page HTTP {}", response.status()));
        }

        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_lowercase();
        let bytes = response
            .bytes()
            .map_err(|e| format!("landing page read failed: {}", e))?;
        let downloaded_bytes = bytes.len();
        if downloaded_bytes == 0 {
            return Err("downloaded page is empty".to_string());
        }

        let body_text = String::from_utf8_lossy(&bytes).to_string();
        let attempted_pdf_url = if content_type.contains("html") || body_text.contains("<html") {
            discover_pdf_url_from_html(landing_page_url, &body_text)
        } else {
            None
        };
        let extracted = if content_type.contains("html") || body_text.contains("<html") {
            extract_text_from_html(&body_text)
        } else {
            HtmlExtractResult {
                body_text,
                sections: Vec::new(),
                section_blocks: Vec::new(),
                references: Vec::new(),
            }
        };
        let normalized = trim_whitespace(&extracted.body_text);
        if normalized.is_empty() {
            return Err("no readable text extracted from landing page".to_string());
        }

        let sections = if extracted.sections.is_empty() {
            infer_sections_from_text(&normalized)
        } else {
            extracted.sections
        };
        let section_blocks = if extracted.section_blocks.is_empty() {
            infer_structured_sections_from_text(&normalized, Some(&sections))
        } else {
            extracted.section_blocks
        };
        let references = if extracted.references.is_empty() {
            infer_references_from_text(&normalized)
        } else {
            extracted.references
        };
        let hash = blake3::hash(normalized.as_bytes()).to_hex().to_string();

        Ok(RemoteContentHydration {
            status: "partial".to_string(),
            source: "remote_text".to_string(),
            source_url: landing_page_url.to_string(),
            format: if content_type.contains("html") {
                "html".to_string()
            } else {
                "text".to_string()
            },
            parser: if content_type.contains("html") {
                "html_text_extractor".to_string()
            } else {
                "plain_text".to_string()
            },
            attempted_pdf_url,
            downloaded_bytes,
            body_text: normalized,
            sections,
            section_blocks,
            references,
            page_count: 1,
            file_hash: hash,
            warnings: vec![
                "Full PDF parse unavailable; used lightweight text extraction.".to_string(),
            ],
        })
    }

    fn fetch_by_doi(&self, doi: &str) -> Result<RemotePaperFetch, String> {
        let normalized = normalize_doi(doi);
        if let Ok(result) = self.fetch_crossref_by_doi(&normalized) {
            return Ok(result);
        }
        if let Ok(result) = self.fetch_semantic_scholar_by_id(&format!("DOI:{}", normalized)) {
            return Ok(result);
        }
        if let Ok(result) = self.fetch_openalex_by_id(&format!("doi:{}", normalized)) {
            return Ok(result);
        }
        Err(format!("no DOI provider returned '{}'", normalized))
    }

    fn fetch_arxiv_by_id(&self, arxiv_id: &str) -> Result<RemotePaperFetch, String> {
        let url = format!(
            "{}?id_list={}",
            self.arxiv_api_url,
            urlencoding::encode(arxiv_id)
        );
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let body = response
            .text()
            .map_err(|e| format!("read body failed: {}", e))?;
        let mut records = parse_arxiv_records(&body, 1);
        let paper = records
            .pop()
            .ok_or_else(|| format!("no arXiv paper found for '{}'", arxiv_id))?;
        let content_hydration = self.hydrate_remote_content(&paper);
        Ok(RemotePaperFetch {
            provider: "arxiv".to_string(),
            content: content_hydration
                .as_ref()
                .map(|hydration| hydration.body_text.clone())
                .or_else(|| paper.abstract_text.clone()),
            abstract_text: paper.abstract_text.clone(),
            raw_metadata: serde_json::json!({ "provider": "arxiv", "arxiv_id": arxiv_id }),
            paper,
            content_hydration,
        })
    }

    fn fetch_semantic_scholar_by_id(&self, paper_id: &str) -> Result<RemotePaperFetch, String> {
        let fields = "title,abstract,authors,venue,year,externalIds,url,openAccessPdf";
        let url = format!(
            "{}/paper/{}?fields={}",
            self.semantic_scholar_api_url,
            urlencoding::encode(paper_id),
            fields
        );
        let request = self.client.get(&url);
        let response = self
            .with_semantic_scholar_headers(request)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: SemanticScholarPaper = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let paper = build_semantic_scholar_record(&payload);
        let content_hydration = self.hydrate_remote_content(&paper);
        Ok(RemotePaperFetch {
            provider: "semantic_scholar".to_string(),
            content: content_hydration
                .as_ref()
                .map(|hydration| hydration.body_text.clone())
                .or_else(|| paper.abstract_text.clone()),
            abstract_text: paper.abstract_text.clone(),
            raw_metadata: serde_json::to_value(&payload).unwrap_or(Value::Null),
            paper,
            content_hydration,
        })
    }

    fn fetch_crossref_by_doi(&self, doi: &str) -> Result<RemotePaperFetch, String> {
        let mut url = format!(
            "{}/works/{}",
            self.crossref_api_url,
            urlencoding::encode(doi)
        );
        if let Some(mailto) = &self.crossref_mailto {
            url.push_str("?mailto=");
            url.push_str(&urlencoding::encode(mailto));
        }
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: Value = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let work_value = payload
            .get("message")
            .cloned()
            .ok_or_else(|| "missing Crossref message".to_string())?;
        let work: CrossrefWork = serde_json::from_value(work_value.clone())
            .map_err(|e| format!("invalid Crossref payload: {}", e))?;
        let mut paper = build_crossref_record(&work);
        self.enrich_with_unpaywall(&mut paper);
        let content_hydration = self.hydrate_remote_content(&paper);
        Ok(RemotePaperFetch {
            provider: "crossref".to_string(),
            content: content_hydration
                .as_ref()
                .map(|hydration| hydration.body_text.clone())
                .or_else(|| paper.abstract_text.clone()),
            abstract_text: paper.abstract_text.clone(),
            raw_metadata: work_value,
            paper,
            content_hydration,
        })
    }

    fn fetch_openalex_by_id(&self, id: &str) -> Result<RemotePaperFetch, String> {
        let mut url = if id.starts_with("doi:") {
            format!(
                "{}/works/{}",
                self.openalex_api_url,
                urlencoding::encode(id)
            )
        } else {
            format!(
                "{}/works/{}",
                self.openalex_api_url,
                urlencoding::encode(id)
            )
        };
        if let Some(mailto) = &self.openalex_mailto {
            if url.contains('?') {
                url.push_str("&mailto=");
            } else {
                url.push_str("?mailto=");
            }
            url.push_str(&urlencoding::encode(mailto));
        }
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: OpenAlexWork = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let mut paper = build_openalex_record(&payload);
        self.enrich_with_unpaywall(&mut paper);
        let content_hydration = self.hydrate_remote_content(&paper);
        Ok(RemotePaperFetch {
            provider: "openalex".to_string(),
            content: content_hydration
                .as_ref()
                .map(|hydration| hydration.body_text.clone())
                .or_else(|| paper.abstract_text.clone()),
            abstract_text: paper.abstract_text.clone(),
            raw_metadata: serde_json::to_value(&payload).unwrap_or(Value::Null),
            paper,
            content_hydration,
        })
    }

    fn fetch_openreview_by_id(&self, note_id: &str) -> Result<RemotePaperFetch, String> {
        let url = format!(
            "{}/notes?id={}",
            self.openreview_api_url,
            urlencoding::encode(note_id)
        );
        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| format!("request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(format!("HTTP {}", response.status()));
        }
        let payload: Value = response
            .json()
            .map_err(|e| format!("invalid JSON: {}", e))?;
        let notes = openreview_notes_array(&payload);
        let note = notes
            .first()
            .cloned()
            .ok_or_else(|| format!("no OpenReview note found for '{}'", note_id))?;
        let paper = build_openreview_record(&note);
        let content_hydration = self.hydrate_remote_content(&paper);
        Ok(RemotePaperFetch {
            provider: "openreview".to_string(),
            content: content_hydration
                .as_ref()
                .map(|hydration| hydration.body_text.clone())
                .or_else(|| paper.abstract_text.clone()),
            abstract_text: paper.abstract_text.clone(),
            raw_metadata: note,
            paper,
            content_hydration,
        })
    }

    fn enrich_with_unpaywall(&self, paper: &mut UnifiedPaperRecord) {
        let Some(doi) = paper.external_ids.doi.clone() else {
            return;
        };
        let Some(email) = self.unpaywall_email.clone() else {
            return;
        };

        let url = format!(
            "{}/{}?email={}",
            self.unpaywall_api_url,
            urlencoding::encode(&doi),
            urlencoding::encode(&email)
        );

        let Ok(response) = self.client.get(&url).send() else {
            return;
        };
        if !response.status().is_success() {
            return;
        }

        let Ok(payload) = response.json::<UnpaywallRecord>() else {
            return;
        };
        let Some(location) = payload.best_oa_location else {
            return;
        };

        if paper.urls.landing_page.is_none() {
            paper.urls.landing_page = location.url;
        }
        if paper.urls.pdf.is_none() {
            paper.urls.pdf = location.url_for_pdf;
        }
    }
}

fn should_prefer_local_source(source: &str) -> bool {
    matches!(
        source.trim().to_lowercase().as_str(),
        "local" | "cache" | "cached"
    )
}

fn is_official_api_only_source(source: &str) -> bool {
    matches!(
        source.trim().to_lowercase().as_str(),
        "official_api" | "official-api" | "remote" | "remote_only" | "remote-only"
    )
}

fn local_paper_fallback_disabled() -> bool {
    std::env::var("AI_SCIENTIST_DISABLE_LOCAL_PAPER_FALLBACK")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn dedupe_papers(papers: Vec<UnifiedPaperRecord>, limit: usize) -> Option<Vec<UnifiedPaperRecord>> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for paper in papers {
        let key = paper_dedupe_key(&paper);
        if seen.insert(key) {
            deduped.push(paper);
        }
        if deduped.len() >= limit {
            break;
        }
    }

    if deduped.is_empty() {
        None
    } else {
        Some(deduped)
    }
}

fn paper_dedupe_key(paper: &UnifiedPaperRecord) -> String {
    if let Some(doi) = &paper.external_ids.doi {
        return format!("doi:{}", doi.to_lowercase());
    }
    if let Some(arxiv_id) = &paper.external_ids.arxiv_id {
        return format!("arxiv:{}", arxiv_id.to_lowercase());
    }
    if let Some(s2) = &paper.external_ids.semantic_scholar_id {
        return format!("s2:{}", s2.to_lowercase());
    }
    if let Some(openalex_id) = &paper.external_ids.openalex_id {
        return format!("openalex:{}", openalex_id.to_lowercase());
    }
    if let Some(openreview_id) = &paper.external_ids.openreview_id {
        return format!("openreview:{}", openreview_id.to_lowercase());
    }
    paper.title.to_lowercase()
}

fn parse_arxiv_records(xml: &str, limit: usize) -> Vec<UnifiedPaperRecord> {
    let mut results = Vec::new();
    let mut current_entry = String::new();
    let mut in_entry = false;

    for line in xml.lines() {
        let line = line.trim();

        if line.contains("<entry>") {
            in_entry = true;
            current_entry.clear();
        }

        if in_entry {
            current_entry.push_str(line);
        }

        if line.contains("</entry>") {
            in_entry = false;

            if let Some(id) = extract_xml_tag(&current_entry, "id") {
                let arxiv_id = id
                    .split("/abs/")
                    .last()
                    .unwrap_or(&id)
                    .split("/pdf/")
                    .last()
                    .unwrap_or(&id)
                    .trim_end_matches(".pdf")
                    .to_string();
                let title = trim_whitespace(
                    &extract_xml_tag(&current_entry, "title").unwrap_or_else(|| arxiv_id.clone()),
                );
                let abstract_text =
                    extract_xml_tag(&current_entry, "summary").map(|s| trim_whitespace(&s));
                let authors = extract_all_xml_tags(&current_entry, "name")
                    .into_iter()
                    .map(|name| trim_whitespace(&name))
                    .filter(|name| !name.is_empty())
                    .collect::<Vec<_>>();
                let year = extract_xml_tag(&current_entry, "published")
                    .and_then(|value| value.get(0..4).and_then(|year| year.parse::<u32>().ok()));
                let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);

                results.push(UnifiedPaperRecord {
                    paper_id: format!("arxiv:{}", arxiv_id),
                    title,
                    authors,
                    snippet: abstract_text.clone(),
                    abstract_text,
                    venue: Some("arXiv".to_string()),
                    year,
                    provider: "arxiv".to_string(),
                    source_format: "remote_metadata".to_string(),
                    external_ids: PaperExternalIds {
                        arxiv_id: Some(arxiv_id.clone()),
                        ..PaperExternalIds::default()
                    },
                    urls: PaperUrls {
                        landing_page: Some(format!("https://arxiv.org/abs/{}", arxiv_id)),
                        pdf: Some(pdf_url),
                        local_path: None,
                    },
                });
            }
        }
    }

    results.into_iter().take(limit).collect()
}

fn extract_xml_tag(content: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);

    if let Some(start) = content.find(&open_tag) {
        let start = start + open_tag.len();
        if let Some(end) = content[start..].find(&close_tag) {
            return Some(content[start..start + end].to_string());
        }
    }
    None
}

fn extract_all_xml_tags(content: &str, tag: &str) -> Vec<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);
    let mut values = Vec::new();
    let mut cursor = 0usize;

    while let Some(start) = content[cursor..].find(&open_tag) {
        let content_start = cursor + start + open_tag.len();
        let Some(end) = content[content_start..].find(&close_tag) else {
            break;
        };
        values.push(content[content_start..content_start + end].to_string());
        cursor = content_start + end + close_tag.len();
    }

    values
}

fn build_semantic_scholar_record(paper: &SemanticScholarPaper) -> UnifiedPaperRecord {
    let doi = paper.external_ids.get("DOI").cloned();
    let arxiv_id = paper
        .external_ids
        .get("ArXiv")
        .or_else(|| paper.external_ids.get("ARXIV"))
        .cloned();
    let s2_id = paper.paper_id.clone();
    let canonical_id = if let Some(id) = &s2_id {
        format!("s2:{}", id)
    } else if let Some(doi) = &doi {
        format!("doi:{}", normalize_doi(doi))
    } else if let Some(arxiv_id) = &arxiv_id {
        format!("arxiv:{}", arxiv_id)
    } else {
        format!("semantic_scholar:{}", slugify_title(&paper.title))
    };

    UnifiedPaperRecord {
        paper_id: canonical_id,
        title: paper.title.clone(),
        authors: paper.authors.iter().map(|a| a.name.clone()).collect(),
        abstract_text: paper.abstract_text.clone(),
        snippet: paper.abstract_text.clone(),
        venue: paper.venue.clone(),
        year: paper.year,
        provider: "semantic_scholar".to_string(),
        source_format: "remote_metadata".to_string(),
        external_ids: PaperExternalIds {
            doi: doi.map(|value| normalize_doi(&value)),
            arxiv_id,
            semantic_scholar_id: s2_id,
            ..PaperExternalIds::default()
        },
        urls: PaperUrls {
            landing_page: paper.url.clone(),
            pdf: paper
                .open_access_pdf
                .as_ref()
                .and_then(|pdf| pdf.url.clone()),
            local_path: None,
        },
    }
}

fn build_crossref_record(work: &CrossrefWork) -> UnifiedPaperRecord {
    let doi = work.doi.clone().map(|value| normalize_doi(&value));
    let title = work
        .title
        .first()
        .cloned()
        .unwrap_or_else(|| doi.clone().unwrap_or_else(|| "crossref-paper".to_string()));
    let abstract_text = work
        .abstract_field
        .as_ref()
        .map(|text| strip_xml_tags(text));
    let year = extract_crossref_year(work);

    UnifiedPaperRecord {
        paper_id: doi
            .as_ref()
            .map(|value| format!("doi:{}", value))
            .unwrap_or_else(|| format!("crossref:{}", slugify_title(&title))),
        title,
        authors: work
            .author
            .iter()
            .map(|author| match (&author.given, &author.family) {
                (Some(given), Some(family)) => format!("{} {}", given, family),
                (Some(given), None) => given.clone(),
                (None, Some(family)) => family.clone(),
                (None, None) => String::new(),
            })
            .filter(|name| !name.is_empty())
            .collect(),
        abstract_text: abstract_text.clone(),
        snippet: abstract_text,
        venue: work.container_title.first().cloned(),
        year,
        provider: "crossref".to_string(),
        source_format: "remote_metadata".to_string(),
        external_ids: PaperExternalIds {
            doi,
            ..PaperExternalIds::default()
        },
        urls: PaperUrls {
            landing_page: work
                .resource
                .as_ref()
                .and_then(|resource| resource.primary.as_ref())
                .and_then(|primary| primary.url.clone())
                .or_else(|| work.url.clone()),
            pdf: None,
            local_path: None,
        },
    }
}

fn build_openalex_record(work: &OpenAlexWork) -> UnifiedPaperRecord {
    let title = work
        .display_name
        .clone()
        .or_else(|| work.title.clone())
        .unwrap_or_else(|| "openalex-paper".to_string());
    let openalex_id = work.id.as_ref().map(|value| short_openalex_id(value));
    let mut landing_page = work
        .primary_location
        .as_ref()
        .and_then(|location| location.landing_page_url.clone());
    let mut pdf = work
        .primary_location
        .as_ref()
        .and_then(|location| location.pdf_url.clone());
    if landing_page.is_none() {
        landing_page = work
            .best_oa_location
            .as_ref()
            .and_then(|location| location.landing_page_url.clone());
    }
    if pdf.is_none() {
        pdf = work
            .best_oa_location
            .as_ref()
            .and_then(|location| location.pdf_url.clone())
            .or_else(|| {
                work.open_access
                    .as_ref()
                    .and_then(|open_access| open_access.oa_url.clone())
            });
    }

    UnifiedPaperRecord {
        paper_id: openalex_id
            .as_ref()
            .map(|id| format!("openalex:{}", id))
            .or_else(|| {
                work.doi
                    .as_ref()
                    .map(|doi| format!("doi:{}", normalize_doi(doi)))
            })
            .unwrap_or_else(|| format!("openalex:{}", slugify_title(&title))),
        title,
        authors: work
            .authorships
            .iter()
            .filter_map(|authorship| {
                authorship
                    .author
                    .as_ref()
                    .and_then(|author| author.display_name.clone())
            })
            .collect(),
        abstract_text: rebuild_openalex_abstract(work.abstract_inverted_index.as_ref()),
        snippet: rebuild_openalex_abstract(work.abstract_inverted_index.as_ref()),
        venue: work
            .primary_location
            .as_ref()
            .and_then(|location| location.source.as_ref())
            .and_then(|source| source.display_name.clone()),
        year: work.publication_year,
        provider: "openalex".to_string(),
        source_format: "remote_metadata".to_string(),
        external_ids: PaperExternalIds {
            doi: work.doi.as_ref().map(|value| normalize_doi(value)),
            openalex_id,
            ..PaperExternalIds::default()
        },
        urls: PaperUrls {
            landing_page,
            pdf,
            local_path: None,
        },
    }
}

fn parse_openreview_search_results(payload: &Value) -> Vec<UnifiedPaperRecord> {
    openreview_notes_array(payload)
        .iter()
        .map(build_openreview_record)
        .collect()
}

fn build_openreview_record(note: &Value) -> UnifiedPaperRecord {
    let note_id = note
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("openreview-note");
    let content = note.get("content").unwrap_or(&Value::Null);
    let title = openreview_content_field(content, "title").unwrap_or_else(|| note_id.to_string());
    let abstract_text = openreview_content_field(content, "abstract");
    let venue = openreview_content_field(content, "venue");
    let pdf_url = note
        .get("pdf")
        .and_then(Value::as_str)
        .map(|value| value.to_string())
        .or_else(|| openreview_content_field(content, "pdf"));
    let authors = content
        .get("authors")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let doi = openreview_content_field(content, "doi");

    UnifiedPaperRecord {
        paper_id: format!("openreview:{}", note_id),
        title,
        authors,
        abstract_text: abstract_text.clone(),
        snippet: abstract_text,
        venue,
        year: note
            .get("cdate")
            .and_then(Value::as_i64)
            .and_then(timestamp_millis_to_year),
        provider: "openreview".to_string(),
        source_format: "remote_metadata".to_string(),
        external_ids: PaperExternalIds {
            doi: doi.as_deref().map(normalize_doi),
            openreview_id: Some(note_id.to_string()),
            ..PaperExternalIds::default()
        },
        urls: PaperUrls {
            landing_page: note
                .get("forum")
                .and_then(Value::as_str)
                .map(|forum| format!("https://openreview.net/forum?id={}", forum))
                .or_else(|| Some(format!("https://openreview.net/forum?id={}", note_id))),
            pdf: pdf_url,
            local_path: None,
        },
    }
}

fn openreview_notes_array(payload: &Value) -> Vec<Value> {
    payload
        .get("notes")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| payload.get("results").and_then(Value::as_array).cloned())
        .unwrap_or_default()
}

fn openreview_content_field(content: &Value, key: &str) -> Option<String> {
    let value = content.get(key)?;
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    if let Some(inner) = value.get("value").and_then(Value::as_str) {
        return Some(inner.to_string());
    }
    None
}

fn parse_acl_anthology_search_results(body: &str, limit: usize) -> Vec<UnifiedPaperRecord> {
    let document = scraper::Html::parse_document(body);
    let selector = scraper::Selector::parse("a[href^=\"/\"]").expect("valid selector");
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    for node in document.select(&selector) {
        let Some(href) = node.value().attr("href") else {
            continue;
        };
        let acl_id = href.trim_matches('/').to_string();
        if acl_id.is_empty()
            || !acl_id.chars().any(|ch| ch.is_ascii_digit())
            || !seen.insert(acl_id.clone())
        {
            continue;
        }
        let title = trim_whitespace(&node.text().collect::<Vec<_>>().join(" "));
        if title.is_empty() {
            continue;
        }
        results.push(UnifiedPaperRecord {
            paper_id: format!("acl:{}", acl_id),
            title,
            authors: Vec::new(),
            abstract_text: None,
            snippet: Some("ACL Anthology search result".to_string()),
            venue: Some("ACL Anthology".to_string()),
            year: infer_acl_year(&acl_id),
            provider: "acl_anthology".to_string(),
            source_format: "remote_html_search".to_string(),
            external_ids: PaperExternalIds {
                acl_anthology_id: Some(acl_id.clone()),
                ..PaperExternalIds::default()
            },
            urls: PaperUrls {
                landing_page: Some(format!("https://aclanthology.org/{}/", acl_id)),
                pdf: Some(format!("https://aclanthology.org/{}.pdf", acl_id)),
                local_path: None,
            },
        });
        if results.len() >= limit {
            break;
        }
    }

    results
}

fn infer_acl_year(anthology_id: &str) -> Option<u32> {
    let digits = anthology_id
        .chars()
        .skip_while(|ch| !ch.is_ascii_digit())
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.len() == 2 {
        digits.parse::<u32>().ok().map(|value| 2000 + value)
    } else if digits.len() == 4 {
        digits.parse::<u32>().ok()
    } else {
        None
    }
}

fn rebuild_openalex_abstract(
    abstract_index: Option<&HashMap<String, Vec<usize>>>,
) -> Option<String> {
    let index = abstract_index?;
    let max_position = index
        .values()
        .flat_map(|positions| positions.iter().copied())
        .max()?;
    let mut words = vec![String::new(); max_position + 1];
    for (word, positions) in index {
        for position in positions {
            if let Some(slot) = words.get_mut(*position) {
                *slot = word.clone();
            }
        }
    }
    let text = words.join(" ");
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

fn extract_crossref_year(work: &CrossrefWork) -> Option<u32> {
    work.issued
        .as_ref()
        .or(work.published.as_ref())
        .or(work.published_print.as_ref())
        .and_then(|date_parts| {
            date_parts
                .date_parts
                .first()
                .and_then(|parts| parts.first().copied())
        })
}

fn strip_xml_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    trim_whitespace(&output)
}

fn extract_text_from_html(html: &str) -> HtmlExtractResult {
    let document = scraper::Html::parse_document(html);
    let selector =
        scraper::Selector::parse("h1, h2, h3, h4, h5, h6, p, li").expect("valid selector");
    let mut body_segments = Vec::new();
    let mut section_titles = Vec::new();
    let mut section_blocks = Vec::new();
    let mut references = Vec::new();
    let mut current_title = "Document".to_string();
    let mut current_level = 1usize;
    let mut current_lines: Vec<String> = Vec::new();

    let flush_section = |blocks: &mut Vec<StructuredSectionBlock>,
                         title: &str,
                         level: usize,
                         lines: &mut Vec<String>| {
        let content = trim_whitespace(&lines.join(" "));
        if content.is_empty() {
            lines.clear();
            return;
        }
        blocks.push(StructuredSectionBlock {
            index: blocks.len(),
            title: title.to_string(),
            level,
            content,
        });
        lines.clear();
    };

    for node in document.select(&selector) {
        let tag = node.value().name();
        let text = trim_whitespace(&node.text().collect::<Vec<_>>().join(" "));
        if text.is_empty() {
            continue;
        }
        body_segments.push(text.clone());
        if matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
            flush_section(
                &mut section_blocks,
                &current_title,
                current_level,
                &mut current_lines,
            );
            current_level = tag
                .strip_prefix('h')
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(1);
            current_title = text.clone();
            section_titles.push(text);
            continue;
        }

        let lower = text.to_lowercase();
        if lower.starts_with("[")
            || lower.starts_with("doi:")
            || (lower.contains(" et al.") && lower.chars().any(|c| c.is_ascii_digit()))
        {
            references.push(text.clone());
        }
        current_lines.push(text);
    }

    flush_section(
        &mut section_blocks,
        &current_title,
        current_level,
        &mut current_lines,
    );

    HtmlExtractResult {
        body_text: body_segments.join("\n"),
        sections: section_titles,
        section_blocks,
        references,
    }
}

fn discover_pdf_url_from_html(base_url: &str, html: &str) -> Option<String> {
    let document = scraper::Html::parse_document(html);

    let meta_selector = scraper::Selector::parse(
        r#"meta[name="citation_pdf_url"], meta[property="citation_pdf_url"]"#,
    )
    .expect("valid selector");
    for node in document.select(&meta_selector) {
        if let Some(content) = node.value().attr("content") {
            if let Some(url) = resolve_candidate_url(base_url, content) {
                return Some(url);
            }
        }
    }

    let link_selector = scraper::Selector::parse("a[href], link[href]").expect("valid selector");
    for node in document.select(&link_selector) {
        let Some(href) = node.value().attr("href") else {
            continue;
        };
        let lower = href.to_lowercase();
        let text = node.text().collect::<Vec<_>>().join(" ").to_lowercase();
        if lower.ends_with(".pdf")
            || lower.contains("/pdf/")
            || text.contains("pdf")
            || text.contains("download pdf")
        {
            if let Some(url) = resolve_candidate_url(base_url, href) {
                return Some(url);
            }
        }
    }

    None
}

fn resolve_candidate_url(base_url: &str, candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(url) = url::Url::parse(trimmed) {
        return Some(url.to_string());
    }
    let base = url::Url::parse(base_url).ok()?;
    base.join(trimmed).ok().map(|url| url.to_string())
}

fn infer_sections_from_text(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| {
            let markdown_heading = line.trim_start_matches('#').trim();
            if markdown_heading.len() != line.len() && !markdown_heading.is_empty() {
                return Some(markdown_heading.to_string());
            }
            if (line.len() < 80
                && line.chars().all(|c| {
                    c.is_ascii_digit() || c == '.' || c.is_alphabetic() || c.is_whitespace()
                }))
                || (line.len() < 60
                    && line
                        .chars()
                        .all(|c| c.is_uppercase() || c.is_whitespace() || c.is_ascii_digit()))
            {
                Some(line.to_string())
            } else {
                None
            }
        })
        .take(24)
        .collect()
}

fn infer_references_from_text(text: &str) -> Vec<String> {
    let mut references = Vec::new();
    let mut in_references = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed.trim_start_matches('#').trim();
        let lower = normalized.to_lowercase();
        if lower.starts_with("references") || lower.starts_with("bibliography") {
            in_references = true;
            continue;
        }
        if in_references {
            references.push(normalized.to_string());
            if references.len() >= 64 {
                break;
            }
        }
    }
    references
}

fn infer_structured_sections_from_text(
    text: &str,
    section_titles: Option<&[String]>,
) -> Vec<StructuredSectionBlock> {
    let normalized_titles = section_titles
        .map(|titles| {
            titles
                .iter()
                .map(|title| trim_whitespace(title))
                .filter(|title| !title.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Vec::new();
    }

    let mut blocks = Vec::new();
    let mut current_title = normalized_titles
        .first()
        .cloned()
        .unwrap_or_else(|| "Document".to_string());
    let mut current_lines = Vec::new();
    let mut title_index = 0usize;

    for line in lines {
        let normalized_line = trim_whitespace(line);
        let looks_like_title = normalized_titles
            .get(title_index + usize::from(!blocks.is_empty()))
            .map(|candidate| candidate.eq_ignore_ascii_case(&normalized_line))
            .unwrap_or(false)
            || infer_sections_from_text(&normalized_line)
                .first()
                .map(|candidate| candidate.eq_ignore_ascii_case(&normalized_line))
                .unwrap_or(false);

        if looks_like_title && !current_lines.is_empty() {
            blocks.push(StructuredSectionBlock {
                index: blocks.len(),
                title: current_title.clone(),
                level: 1,
                content: trim_whitespace(&current_lines.join(" ")),
            });
            current_title = normalized_line.clone();
            current_lines.clear();
            title_index = title_index.saturating_add(1);
            continue;
        }

        if looks_like_title && current_lines.is_empty() {
            current_title = normalized_line;
            title_index = title_index.saturating_add(1);
            continue;
        }

        current_lines.push(normalized_line);
    }

    if !current_lines.is_empty() {
        blocks.push(StructuredSectionBlock {
            index: blocks.len(),
            title: current_title,
            level: 1,
            content: trim_whitespace(&current_lines.join(" ")),
        });
    }

    if blocks.is_empty() {
        blocks.push(StructuredSectionBlock {
            index: 0,
            title: "Document".to_string(),
            level: 1,
            content: trim_whitespace(text),
        });
    }

    blocks
}

fn build_structured_references(references: &[String]) -> Vec<StructuredReferenceEntry> {
    references
        .iter()
        .enumerate()
        .map(|(index, text)| StructuredReferenceEntry {
            index,
            text: text.clone(),
        })
        .collect()
}

fn build_document_quality(
    body_text: Option<&str>,
    sections: &[StructuredSectionBlock],
    references: &[StructuredReferenceEntry],
    content_source: Option<&str>,
) -> StructuredDocumentQuality {
    let body_len = body_text.map(str::len).unwrap_or(0);
    let section_count = sections.len();
    let reference_count = references.len();
    let extraction_path = content_source.unwrap_or("metadata_only").to_string();
    let completeness =
        if extraction_path == "remote_pdf" || extraction_path == "local_file" && body_len >= 4000 {
            "full_text".to_string()
        } else if body_len >= 1500 && section_count >= 3 {
            "substantial_text".to_string()
        } else if body_len >= 200 {
            "partial_text".to_string()
        } else {
            "metadata_only".to_string()
        };

    StructuredDocumentQuality {
        completeness,
        extraction_path,
        has_full_body_text: body_len >= 1500,
        has_section_structure: section_count >= 1,
        has_references: reference_count >= 1,
        body_text_chars: body_len,
        section_count,
        reference_count,
    }
}

fn build_remote_structured_document(remote: &RemotePaperFetch) -> StructuredPaperDocument {
    let hydration = remote.content_hydration.as_ref();
    let sections = hydration
        .map(|item| item.section_blocks.clone())
        .unwrap_or_default();
    let references = hydration
        .map(|item| build_structured_references(&item.references))
        .unwrap_or_default();
    let body_text = hydration
        .map(|item| item.body_text.clone())
        .or_else(|| remote.content.clone());
    let quality = build_document_quality(
        body_text.as_deref(),
        &sections,
        &references,
        hydration.map(|item| item.source.as_str()),
    );

    StructuredPaperDocument {
        schema_version: "structured_paper_document_v1".to_string(),
        paper_schema_version: PAPER_SCHEMA_VERSION.to_string(),
        paper_id: remote.paper.paper_id.clone(),
        provider: remote.provider.clone(),
        title: remote.paper.title.clone(),
        authors: remote.paper.authors.clone(),
        abstract_text: remote.abstract_text.clone(),
        body_text,
        sections,
        references,
        venue: remote.paper.venue.clone(),
        year: remote.paper.year,
        page_count: hydration.map(|item| item.page_count),
        file_hash: hydration.map(|item| item.file_hash.clone()),
        external_ids: remote.paper.external_ids.clone(),
        urls: remote.paper.urls.clone(),
        provenance: StructuredDocumentProvenance {
            source_preference: "remote_first".to_string(),
            primary_source: "remote".to_string(),
            provider: remote.provider.clone(),
            content_source: hydration.map(|item| item.source.clone()),
            attempted_pdf_url: hydration.and_then(|item| item.attempted_pdf_url.clone()),
            source_url: hydration.map(|item| item.source_url.clone()),
            format: hydration.map(|item| item.format.clone()),
            parser: hydration.map(|item| item.parser.clone()),
            warnings: hydration
                .map(|item| item.warnings.clone())
                .unwrap_or_default(),
        },
        quality,
    }
}

fn build_fulltext_summary(document: &StructuredPaperDocument) -> Value {
    serde_json::json!({
        "status": if document.quality.has_full_body_text {
            "ready"
        } else if document.quality.body_text_chars > 0 {
            "partial"
        } else {
            "metadata_only"
        },
        "primary_source": document.provenance.primary_source,
        "provider": document.provider,
        "content_source": document.provenance.content_source,
        "attempted_pdf_url": document.provenance.attempted_pdf_url,
        "source_url": document.provenance.source_url,
        "format": document.provenance.format,
        "parser": document.provenance.parser,
        "completeness": document.quality.completeness,
        "extraction_path": document.quality.extraction_path,
        "has_body_text": document.quality.body_text_chars > 0,
        "has_section_structure": document.quality.has_section_structure,
        "has_references": document.quality.has_references,
        "body_text_chars": document.quality.body_text_chars,
        "section_count": document.quality.section_count,
        "reference_count": document.quality.reference_count,
        "warnings": document.provenance.warnings,
    })
}

fn build_batch_fulltext_summary(results: &[Value]) -> Value {
    let mut ready = 0usize;
    let mut partial = 0usize;
    let mut metadata_only = 0usize;
    let mut provider_counts: HashMap<String, usize> = HashMap::new();
    let mut completeness_counts: HashMap<String, usize> = HashMap::new();

    for item in results {
        if let Some(status) = item
            .get("fulltext")
            .and_then(|summary| summary.get("status"))
            .and_then(Value::as_str)
        {
            match status {
                "ready" => ready += 1,
                "partial" => partial += 1,
                _ => metadata_only += 1,
            }
        } else {
            metadata_only += 1;
        }

        if let Some(provider) = item
            .get("fulltext")
            .and_then(|summary| summary.get("provider"))
            .and_then(Value::as_str)
        {
            *provider_counts.entry(provider.to_string()).or_insert(0) += 1;
        }

        if let Some(completeness) = item
            .get("fulltext")
            .and_then(|summary| summary.get("completeness"))
            .and_then(Value::as_str)
        {
            *completeness_counts
                .entry(completeness.to_string())
                .or_insert(0) += 1;
        }
    }

    serde_json::json!({
        "requested_documents": results.len(),
        "ready_documents": ready,
        "partial_documents": partial,
        "metadata_only_documents": metadata_only,
        "provider_counts": provider_counts,
        "completeness_counts": completeness_counts,
    })
}

fn build_local_structured_document(
    paper: &UnifiedPaperRecord,
    body_text: Option<String>,
    sections: Vec<String>,
    references: Vec<String>,
    page_count: Option<usize>,
    file_hash: Option<String>,
    format: &str,
    parser: &str,
) -> StructuredPaperDocument {
    let structured_sections = if let Some(body) = body_text.as_deref() {
        infer_structured_sections_from_text(body, Some(&sections))
    } else {
        Vec::new()
    };
    let structured_references = build_structured_references(&references);
    let quality = build_document_quality(
        body_text.as_deref(),
        &structured_sections,
        &structured_references,
        Some("local_file"),
    );

    StructuredPaperDocument {
        schema_version: "structured_paper_document_v1".to_string(),
        paper_schema_version: PAPER_SCHEMA_VERSION.to_string(),
        paper_id: paper.paper_id.clone(),
        provider: paper.provider.clone(),
        title: paper.title.clone(),
        authors: paper.authors.clone(),
        abstract_text: paper.abstract_text.clone(),
        body_text,
        sections: structured_sections,
        references: structured_references,
        venue: paper.venue.clone(),
        year: paper.year,
        page_count,
        file_hash,
        external_ids: paper.external_ids.clone(),
        urls: paper.urls.clone(),
        provenance: StructuredDocumentProvenance {
            source_preference: "remote_first".to_string(),
            primary_source: "local_fallback".to_string(),
            provider: paper.provider.clone(),
            content_source: Some("local_file".to_string()),
            attempted_pdf_url: None,
            source_url: paper.urls.local_path.clone(),
            format: Some(format.to_string()),
            parser: Some(parser.to_string()),
            warnings: Vec::new(),
        },
        quality,
    }
}

fn fetch_single_paper_payload(paper_id: &str) -> Result<Value, String> {
    let remote_clients = RemoteClients::from_env()?;
    if let Ok(remote) = remote_clients.fetch_remote_paper(paper_id) {
        let structured_document = build_remote_structured_document(&remote);
        let fulltext = build_fulltext_summary(&structured_document);
        return Ok(serde_json::json!({
            "status": "success",
            "mode": "remote",
            "provider": remote.provider,
            "paper_schema_version": PAPER_SCHEMA_VERSION,
            "paper_id": paper_id,
            "paper": remote.paper,
            "title": remote.paper.title,
            "authors": remote.paper.authors,
            "abstract": remote.abstract_text,
            "content": remote.content,
            "body_text": remote.content_hydration.as_ref().map(|hydration| hydration.body_text.clone()),
            "sections": remote.content_hydration.as_ref().map(|hydration| hydration.sections.clone()).unwrap_or_default(),
            "references": remote.content_hydration.as_ref().map(|hydration| hydration.references.clone()).unwrap_or_default(),
            "page_count": remote.content_hydration.as_ref().map(|hydration| hydration.page_count),
            "file_hash": remote.content_hydration.as_ref().map(|hydration| hydration.file_hash.clone()),
            "content_hydration": remote.content_hydration,
            "fulltext": fulltext,
            "structured_document": structured_document,
            "metadata": remote.raw_metadata,
        }));
    }

    if local_paper_fallback_disabled() {
        return Err(format!(
            "fetch_paper: remote fetch failed and local fallback is disabled for '{}'.",
            paper_id
        ));
    }

    for root in papers_roots() {
        if !root.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(&root)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            let path_str = path.to_string_lossy();
            let matches_requested_id =
                stem == paper_id || file_name == paper_id || path_str.contains(paper_id);
            if !matches_requested_id {
                continue;
            }

            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext == "pdf" {
                let parser = PdfParser::new();
                if let Ok(parsed) = parser.parse(path) {
                    let paper = build_local_pdf_paper(paper_id, path, &parsed);
                    let structured_document = build_local_structured_document(
                        &paper,
                        Some(parsed.body_text.clone()),
                        parsed.sections.clone(),
                        parsed.references.clone(),
                        Some(parsed.page_count),
                        Some(parsed.file_hash.clone()),
                        "pdf",
                        "pdftotext",
                    );
                    let fulltext = build_fulltext_summary(&structured_document);
                    return Ok(serde_json::json!({
                        "status": "success",
                        "mode": "local",
                        "paper_schema_version": PAPER_SCHEMA_VERSION,
                        "paper_id": paper_id,
                        "path": path.to_string_lossy(),
                        "paper": paper,
                        "title": parsed.title,
                        "authors": parsed.authors,
                        "abstract": parsed.abstract_text,
                        "body_text": parsed.body_text,
                        "sections": parsed.sections,
                        "references": parsed.references,
                        "year": parsed.year,
                        "doi": parsed.doi,
                        "page_count": parsed.page_count,
                        "file_hash": parsed.file_hash,
                        "fulltext": fulltext,
                        "structured_document": structured_document
                    }));
                }
            } else if let Ok(content) = fs::read_to_string(path) {
                let title = extract_title_from_md(&content, stem);
                let paper = build_local_markdown_paper(paper_id, path, title.clone(), &content);
                let structured_document = build_local_structured_document(
                    &paper,
                    Some(content.clone()),
                    infer_sections_from_text(&content),
                    infer_references_from_text(&content),
                    None,
                    Some(blake3::hash(content.as_bytes()).to_hex().to_string()),
                    &ext,
                    "plain_text",
                );
                let fulltext = build_fulltext_summary(&structured_document);
                return Ok(serde_json::json!({
                    "status": "success",
                    "mode": "local",
                    "paper_schema_version": PAPER_SCHEMA_VERSION,
                    "paper_id": paper_id,
                    "path": path.to_string_lossy(),
                    "paper": paper,
                    "content": content,
                    "title": title,
                    "fulltext": fulltext,
                    "structured_document": structured_document,
                }));
            }
        }
    }

    Err(format!(
        "fetch_paper: remote fetch failed and local cache miss for '{}'.",
        paper_id
    ))
}

fn trim_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn slugify_title(title: &str) -> String {
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    slug.trim_matches('-').to_string()
}

fn timestamp_millis_to_year(value: i64) -> Option<u32> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(value).map(|dt| dt.year() as u32)
}

#[tool]
impl LiteratureTools {
    /// Search academic papers with remote APIs first, then fall back to local files if needed.
    pub fn search_paper(
        &self,
        query: String,
        source: Option<String>,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let source = source.unwrap_or_else(|| "auto".into());
        let limit = limit.unwrap_or(10).min(50);
        let prefer_local = should_prefer_local_source(&source);

        if prefer_local {
            let local_results = search_local_papers(&query, limit);
            if !local_results.is_empty() {
                let normalized_results = local_results
                    .iter()
                    .map(build_local_search_paper)
                    .collect::<Vec<_>>();
                return Ok(serde_json::json!({
                    "status": "success",
                    "mode": "local",
                    "query": query,
                    "source": source,
                    "paper_schema_version": PAPER_SCHEMA_VERSION,
                    "total": normalized_results.len(),
                    "results": normalized_results
                }));
            }
        }

        let remote_clients = RemoteClients::from_env()?;
        match remote_clients.search_remote_papers(&query, &source, limit) {
            Ok((results, provider_errors)) => {
                let consulted = remote_clients
                    .providers_for_source(&source)
                    .into_iter()
                    .map(|provider| provider.as_str().to_string())
                    .collect::<Vec<_>>();

                Ok(serde_json::json!({
                    "status": "success",
                    "mode": "remote",
                    "query": query,
                    "source": source,
                    "paper_schema_version": PAPER_SCHEMA_VERSION,
                    "total": results.len(),
                    "results": results,
                    "providers_consulted": consulted,
                    "provider_warnings": provider_errors,
                }))
            }
            Err(remote_error) => {
                if is_official_api_only_source(&source) {
                    return Err(remote_error);
                }
                let local_results = search_local_papers(&query, limit);
                if !local_results.is_empty() {
                    let normalized_results = local_results
                        .iter()
                        .map(build_local_search_paper)
                        .collect::<Vec<_>>();
                    Ok(serde_json::json!({
                        "status": "success",
                        "mode": "local_fallback",
                        "query": query,
                        "source": source,
                        "paper_schema_version": PAPER_SCHEMA_VERSION,
                        "total": normalized_results.len(),
                        "results": normalized_results,
                        "fallback_reason": remote_error,
                    }))
                } else {
                    Err(remote_error)
                }
            }
        }
    }

    /// Fetch a paper's metadata and accessible text by DOI, arXiv ID, provider ID, or local fallback.
    pub fn fetch_paper(&self, paper_id: String) -> Result<Value, String> {
        fetch_single_paper_payload(&paper_id)
    }

    /// Fetch up to three papers' structured full text in one call, preferring remote sources.
    pub fn fetch_papers(
        &self,
        paper_ids: Vec<String>,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        if paper_ids.is_empty() {
            return Err("fetch_papers: provide at least one paper_id.".to_string());
        }

        let effective_limit = limit.unwrap_or(3).clamp(1, 3);
        let selected_ids = paper_ids
            .into_iter()
            .filter(|id| !id.trim().is_empty())
            .take(effective_limit)
            .collect::<Vec<_>>();

        if selected_ids.is_empty() {
            return Err("fetch_papers: provide at least one non-empty paper_id.".to_string());
        }

        let mut results = Vec::new();
        let mut errors = Vec::new();

        for paper_id in &selected_ids {
            match fetch_single_paper_payload(paper_id) {
                Ok(payload) => results.push(payload),
                Err(err) => errors.push(serde_json::json!({
                    "paper_id": paper_id,
                    "error": err,
                })),
            }
        }

        if results.is_empty() {
            return Err(format!(
                "fetch_papers: failed to fetch any requested paper. {}",
                errors
                    .iter()
                    .filter_map(|item| item.get("error").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }

        Ok(serde_json::json!({
            "status": if errors.is_empty() { "success" } else { "partial" },
            "operation": "fetch_papers",
            "paper_schema_version": PAPER_SCHEMA_VERSION,
            "requested_count": selected_ids.len(),
            "fetched_count": results.len(),
            "limit_applied": effective_limit,
            "fulltext_bundle": build_batch_fulltext_summary(&results),
            "results": results,
            "errors": errors,
        }))
    }

    /// Generate a citation in specified format.
    pub fn cite_paper(&self, paper_id: String, format: Option<String>) -> Result<Value, String> {
        let fmt = format.unwrap_or_else(|| "bibtex".into());
        let normalized = if let Some(id) = paper_id.strip_prefix("doi:") {
            id.to_string()
        } else {
            paper_id.clone()
        };

        if normalized.starts_with("10.") || normalized.contains('/') {
            Ok(serde_json::json!({
                "status": "partial",
                "operation": "cite_paper",
                "paper_id": paper_id,
                "format": fmt,
                "warning": "Citation format is basic - use external API for complete metadata",
                "citation": format!(
                    "@article{{{},\n  title={{[Title not resolved]}},\n  author={{[Authors not resolved]}},\n  doi={{{}}},\n  note={{Citation generated by AI Scientist - verify before use}}\n}}",
                    normalized.replace(['.', '/'], "_"),
                    normalized
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
    use once_cell::sync::Lazy;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard,
    };
    use std::thread;

    static TEST_ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn test_env_guard() -> MutexGuard<'static, ()> {
        TEST_ENV_MUTEX
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    struct TestHttpServer {
        base_url: String,
        shutdown: Arc<AtomicBool>,
    }

    impl TestHttpServer {
        fn with_handler<F>(handler: F) -> Self
        where
            F: Fn(&str) -> (u16, &'static str, String) + Send + Sync + 'static,
        {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            let base_url = format!("http://127.0.0.1:{}", port);
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_flag = shutdown.clone();
            let handler = Arc::new(handler);

            thread::spawn(move || {
                listener.set_nonblocking(true).ok();
                while !shutdown_flag.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let mut buffer = [0u8; 8192];
                            let bytes = stream.read(&mut buffer).unwrap_or(0);
                            let request = String::from_utf8_lossy(&buffer[..bytes]).to_string();
                            let path = request
                                .lines()
                                .next()
                                .and_then(|line| line.split_whitespace().nth(1))
                                .unwrap_or("/")
                                .to_string();
                            let (status, content_type, body) = handler(&path);
                            let status_text = if status == 200 { "OK" } else { "ERROR" };
                            let response = format!(
                                "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                                status,
                                status_text,
                                content_type,
                                body.len(),
                                body
                            );
                            let _ = stream.write_all(response.as_bytes());
                        }
                        Err(_) => thread::sleep(Duration::from_millis(10)),
                    }
                }
            });

            Self { base_url, shutdown }
        }
    }

    impl Drop for TestHttpServer {
        fn drop(&mut self) {
            self.shutdown.store(true, Ordering::Relaxed);
            let _ = reqwest::blocking::get(&self.base_url);
        }
    }

    fn with_env_var<T>(key: &str, value: &str, f: impl FnOnce() -> T) -> T {
        let old = std::env::var(key).ok();
        std::env::set_var(key, value);
        let result = f();
        if let Some(old) = old {
            std::env::set_var(key, old);
        } else {
            std::env::remove_var(key);
        }
        result
    }

    fn with_literature_remote_env<T>(base_url: &str, f: impl FnOnce() -> T) -> T {
        with_env_var("ARXIV_API_URL", base_url, || {
            with_env_var("SEMANTIC_SCHOLAR_API_URL", base_url, || {
                with_env_var("CROSSREF_API_URL", base_url, || {
                    with_env_var("OPENALEX_API_URL", base_url, || {
                        with_env_var("OPENREVIEW_API_URL", base_url, || {
                            with_env_var("UNPAYWALL_API_URL", base_url, || {
                                with_env_var("UNPAYWALL_EMAIL", "test@example.com", f)
                            })
                        })
                    })
                })
            })
        })
    }

    fn with_remote_env_and_empty_local<T>(base_url: &str, f: impl FnOnce() -> T) -> T {
        let empty_dir = tempfile::tempdir().unwrap();
        with_env_var(
            "AI_SCIENTIST_PAPERS_DIR",
            &empty_dir.path().to_string_lossy(),
            || with_literature_remote_env(base_url, f),
        )
    }

    #[test]
    fn test_local_search_finds_markdown_file() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let paper_path = temp_dir.path().join("quantum_notes.md");
        let mut file = fs::File::create(&paper_path).unwrap();
        writeln!(
            file,
            "# Quantum Notes\n\nThis paper discusses quantum computing and verification."
        )
        .unwrap();

        std::env::set_var("AI_SCIENTIST_PAPERS_DIR", temp_dir.path());
        let results = search_local_papers("quantum computing", 5);
        std::env::remove_var("AI_SCIENTIST_PAPERS_DIR");

        assert!(!results.is_empty());
        assert_eq!(results[0].paper_id, "quantum_notes");
        assert!(results[0].title.contains("Quantum"));
        assert!(results[0].snippet.contains("quantum computing"));
    }

    #[test]
    fn test_fetch_paper_remote_cache_miss_is_clear_when_providers_fail() {
        let _guard = test_env_guard();
        let server = TestHttpServer::with_handler(|_path| {
            (
                404,
                "application/json",
                r#"{"error":"missing"}"#.to_string(),
            )
        });

        let tool = LiteratureTools;
        let err = with_remote_env_and_empty_local(&server.base_url, || {
            tool.fetch_paper("missing-paper".into()).unwrap_err()
        });
        assert!(err.contains("remote fetch failed"));
    }

    #[test]
    fn test_search_paper_returns_unified_schema() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let paper_path = temp_dir.path().join("transformers_note.md");
        let mut file = fs::File::create(&paper_path).unwrap();
        writeln!(
            file,
            "# Transformers Note\n\nThis note compares transformer baselines for sequence modeling."
        )
        .unwrap();

        std::env::set_var("AI_SCIENTIST_PAPERS_DIR", temp_dir.path());
        let payload = LiteratureTools
            .search_paper(
                "transformer baselines".into(),
                Some("local".into()),
                Some(5),
            )
            .unwrap();
        std::env::remove_var("AI_SCIENTIST_PAPERS_DIR");

        assert_eq!(payload["paper_schema_version"], PAPER_SCHEMA_VERSION);
        let first = &payload["results"][0];
        assert_eq!(first["paper_id"], "transformers_note");
        assert_eq!(first["provider"], "local");
        assert_eq!(first["source_format"], "md");
        assert_eq!(
            first["urls"]["local_path"],
            paper_path.to_string_lossy().to_string()
        );
    }

    #[test]
    fn test_search_paper_prefers_remote_over_local_for_auto_source() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let local_path = temp_dir.path().join("transformer_local.md");
        fs::write(
            &local_path,
            "# Local Transformer Note\n\nThis local note should only be used as fallback.",
        )
        .unwrap();

        let base_url_slot = Arc::new(Mutex::new(String::new()));
        let base_url_for_handler = base_url_slot.clone();
        let server = TestHttpServer::with_handler(move |path| {
            let base_url = base_url_for_handler.lock().unwrap().clone();
            if path.starts_with("/works?search=") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                            "results": [{{
                                "id": "https://openalex.org/W999",
                                "doi": "https://doi.org/10.1000/remote.999",
                                "display_name": "Remote Transformer Systems Paper",
                                "publication_year": 2025,
                                "authorships": [{{"author": {{"display_name": "Grace Hopper"}}}}],
                                "primary_location": {{
                                    "landing_page_url": "{}/remote-paper",
                                    "pdf_url": null,
                                    "source": {{"display_name": "SOSP"}}
                                }},
                                "abstract_inverted_index": {{"Remote": [0], "paper": [1]}}
                            }}]
                        }}"#,
                        base_url
                    ),
                );
            }
            if path == "/remote-paper" {
                return (
                    200,
                    "text/html",
                    "<html><body><h1>Remote Transformer Systems Paper</h1><p>Remote content.</p></body></html>".to_string(),
                );
            }
            (404, "application/json", "{}".to_string())
        });
        *base_url_slot.lock().unwrap() = server.base_url.clone();

        let payload = with_env_var(
            "AI_SCIENTIST_PAPERS_DIR",
            &temp_dir.path().to_string_lossy(),
            || {
                with_remote_env_and_empty_local(&server.base_url, || {
                    LiteratureTools
                        .search_paper("transformer".into(), Some("auto".into()), Some(1))
                        .unwrap()
                })
            },
        );

        assert_eq!(payload["mode"], "remote");
        assert_eq!(payload["results"][0]["provider"], "openalex");
        assert_eq!(
            payload["results"][0]["title"],
            "Remote Transformer Systems Paper"
        );
    }

    #[test]
    fn test_search_paper_falls_back_to_local_when_remote_fails() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let local_path = temp_dir.path().join("fallback_local.md");
        fs::write(
            &local_path,
            "# Fallback Local Note\n\nThis local fallback should be used when remote providers fail.",
        )
        .unwrap();

        let server = TestHttpServer::with_handler(|_path| {
            (
                500,
                "application/json",
                r#"{"error":"unavailable"}"#.to_string(),
            )
        });

        let payload = with_env_var(
            "AI_SCIENTIST_PAPERS_DIR",
            &temp_dir.path().to_string_lossy(),
            || {
                with_literature_remote_env(&server.base_url, || {
                    LiteratureTools
                        .search_paper("fallback local".into(), Some("auto".into()), Some(1))
                        .unwrap()
                })
            },
        );

        assert_eq!(payload["mode"], "local_fallback");
        assert_eq!(payload["results"][0]["provider"], "local");
        assert!(payload["fallback_reason"].is_string());
    }

    #[test]
    fn test_search_paper_official_api_does_not_fallback_to_local() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let local_path = temp_dir.path().join("fallback_local.md");
        fs::write(
            &local_path,
            "# Fallback Local Note\n\nThis local fallback must not be used in official_api mode.",
        )
        .unwrap();

        let server = TestHttpServer::with_handler(|_path| {
            (
                500,
                "application/json",
                r#"{"error":"unavailable"}"#.to_string(),
            )
        });

        let err = with_env_var(
            "AI_SCIENTIST_PAPERS_DIR",
            &temp_dir.path().to_string_lossy(),
            || {
                with_literature_remote_env(&server.base_url, || {
                    LiteratureTools
                        .search_paper(
                            "fallback local".into(),
                            Some("official_api".into()),
                            Some(1),
                        )
                        .unwrap_err()
                })
            },
        );

        assert!(err.contains("no remote paper matched") || err.contains("Provider errors"));
    }

    #[test]
    fn test_fetch_paper_official_api_mode_does_not_fallback_to_local() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let local_path = temp_dir.path().join("fallback_local.md");
        fs::write(
            &local_path,
            "# Fallback Local Note\n\nThis local paper must not be used when official API mode disables local fallback.",
        )
        .unwrap();

        let server = TestHttpServer::with_handler(|_path| {
            (
                500,
                "application/json",
                r#"{"error":"unavailable"}"#.to_string(),
            )
        });

        let err = with_env_var(
            "AI_SCIENTIST_PAPERS_DIR",
            &temp_dir.path().to_string_lossy(),
            || {
                with_env_var("AI_SCIENTIST_DISABLE_LOCAL_PAPER_FALLBACK", "1", || {
                    with_literature_remote_env(&server.base_url, || {
                        LiteratureTools
                            .fetch_paper("fallback_local".into())
                            .unwrap_err()
                    })
                })
            },
        );

        assert!(err.contains("local fallback is disabled"));
    }

    #[test]
    fn test_fetch_paper_returns_unified_markdown_paper() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let paper_path = temp_dir.path().join("runtime_paper.md");
        let mut file = fs::File::create(&paper_path).unwrap();
        writeln!(
            file,
            "# Runtime Paper\n\nThis note documents a runtime benchmark and evaluation setup."
        )
        .unwrap();

        std::env::set_var("AI_SCIENTIST_PAPERS_DIR", temp_dir.path());
        let payload = LiteratureTools.fetch_paper("runtime_paper".into()).unwrap();
        std::env::remove_var("AI_SCIENTIST_PAPERS_DIR");

        assert_eq!(payload["paper_schema_version"], PAPER_SCHEMA_VERSION);
        assert_eq!(payload["paper"]["paper_id"], "runtime_paper");
        assert_eq!(payload["paper"]["title"], "Runtime Paper");
        assert_eq!(payload["paper"]["provider"], "local");
        assert_eq!(
            payload["paper"]["urls"]["local_path"],
            paper_path.to_string_lossy().to_string()
        );
        assert!(payload["content"]
            .as_str()
            .unwrap_or("")
            .contains("runtime benchmark"));
    }

    #[test]
    fn test_search_paper_remote_aggregates_mainstream_providers() {
        let _guard = test_env_guard();
        let server = TestHttpServer::with_handler(|path| {
            if path.starts_with("/paper/search") {
                return (
                    200,
                    "application/json",
                    r#"{
                        "data": [{
                            "paperId": "s2-123",
                            "title": "Semantic Retrieval for Agents",
                            "abstract": "Semantic Scholar abstract",
                            "authors": [{"name": "Ada Lovelace"}],
                            "venue": "NeurIPS",
                            "year": 2024,
                            "externalIds": {"DOI": "10.1000/semantic.123"},
                            "url": "https://www.semanticscholar.org/paper/s2-123",
                            "openAccessPdf": {"url": "https://example.com/s2.pdf"}
                        }]
                    }"#
                    .to_string(),
                );
            }
            if path.starts_with("/works?search=") {
                return (
                    200,
                    "application/json",
                    r#"{
                        "results": [{
                            "id": "https://openalex.org/W123",
                            "doi": "https://doi.org/10.1000/openalex.123",
                            "display_name": "OpenAlex Systems Benchmark",
                            "publication_year": 2023,
                            "authorships": [{"author": {"display_name": "Grace Hopper"}}],
                            "primary_location": {
                                "landing_page_url": "https://openalex.example/landing",
                                "pdf_url": null,
                                "source": {"display_name": "OSDI"}
                            },
                            "abstract_inverted_index": {"Systems": [0], "benchmark": [1]}
                        }]
                    }"#
                    .to_string(),
                );
            }
            if path.starts_with("/works?query.bibliographic=") {
                return (
                    200,
                    "application/json",
                    r#"{
                        "message": {
                            "items": [{
                                "DOI": "10.1000/crossref.123",
                                "title": ["Crossref Security Evaluation"],
                                "author": [{"given": "Barbara", "family": "Liskov"}],
                                "container-title": ["IEEE S&P"],
                                "issued": {"date-parts": [[2022]]},
                                "URL": "https://doi.org/10.1000/crossref.123"
                            }]
                        }
                    }"#
                    .to_string(),
                );
            }
            if path.starts_with("/notes?term=") {
                return (
                    200,
                    "application/json",
                    r#"{
                        "notes": [{
                            "id": "or-123",
                            "forum": "or-123",
                            "cdate": 1711929600000,
                            "content": {
                                "title": "OpenReview Agent Evaluation",
                                "abstract": "OpenReview abstract",
                                "authors": ["Alan Turing"],
                                "venue": "ICLR 2024"
                            },
                            "pdf": "https://openreview.example/or-123.pdf"
                        }]
                    }"#
                    .to_string(),
                );
            }
            if path.starts_with("/?search_query=all:") {
                return (
                    200,
                    "application/atom+xml",
                    r#"
                    <feed xmlns="http://www.w3.org/2005/Atom">
                      <entry>
                        <id>http://arxiv.org/abs/2401.12345</id>
                        <title>arXiv Deep Learning Runtime</title>
                        <summary>arXiv abstract text.</summary>
                        <published>2024-01-01T00:00:00Z</published>
                        <author><name>Geoff Hinton</name></author>
                      </entry>
                    </feed>
                    "#
                    .to_string(),
                );
            }
            (404, "application/json", "{}".to_string())
        });

        let payload = with_remote_env_and_empty_local(&server.base_url, || {
            LiteratureTools
                .search_paper("agent benchmark".into(), Some("auto".into()), Some(10))
                .unwrap()
        });

        assert_eq!(payload["mode"], "remote");
        assert_eq!(payload["paper_schema_version"], PAPER_SCHEMA_VERSION);
        assert!(payload["total"].as_u64().unwrap_or(0) >= 4);
        let providers = payload["results"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|item| item["provider"].as_str())
            .collect::<Vec<_>>();
        assert!(providers.contains(&"semantic_scholar"));
        assert!(providers.contains(&"openalex"));
        assert!(providers.contains(&"crossref"));
        assert!(providers.contains(&"openreview"));
    }

    #[test]
    fn test_fetch_paper_remote_by_doi_enriches_unpaywall_pdf() {
        let _guard = test_env_guard();
        let base_url_slot = Arc::new(Mutex::new(String::new()));
        let base_url_for_handler = base_url_slot.clone();
        let server = TestHttpServer::with_handler(move |path| {
            let base_url = base_url_for_handler.lock().unwrap().clone();
            if path.contains("/works/10.1000") && path.contains("crossref.123") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                        "message": {{
                            "DOI": "10.1000/crossref.123",
                            "title": ["Crossref Security Evaluation"],
                            "author": [{{"given": "Barbara", "family": "Liskov"}}],
                            "container-title": ["IEEE S&P"],
                            "issued": {{"date-parts": [[2022]]}},
                            "URL": "{}/paper-page"
                        }}
                    }}"#,
                        base_url
                    ),
                );
            }
            if path.contains("/10.1000") && path.contains("crossref.123") && path.contains("email=")
            {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                        "best_oa_location": {{
                            "url": "{}/paper-page",
                            "url_for_pdf": "{}/paper.pdf"
                        }}
                    }}"#,
                        base_url, base_url
                    ),
                );
            }
            if path == "/paper-page" {
                return (
                    200,
                    "text/html",
                    "<html><body><h1>Crossref Security Evaluation</h1><p>Remote landing page text.</p></body></html>".to_string(),
                );
            }
            if path == "/paper.pdf" {
                return (200, "application/pdf", "%PDF-1.4 fake".to_string());
            }
            (404, "application/json", "{}".to_string())
        });
        *base_url_slot.lock().unwrap() = server.base_url.clone();

        let payload = with_remote_env_and_empty_local(&server.base_url, || {
            LiteratureTools
                .fetch_paper("doi:10.1000/crossref.123".into())
                .unwrap()
        });

        assert_eq!(payload["mode"], "remote");
        assert_eq!(payload["provider"], "crossref");
        assert_eq!(
            payload["paper"]["external_ids"]["doi"],
            "10.1000/crossref.123"
        );
        assert_eq!(
            payload["paper"]["urls"]["pdf"],
            format!("{}/paper.pdf", server.base_url)
        );
    }

    #[test]
    fn test_fetch_paper_remote_by_provider_specific_id() {
        let _guard = test_env_guard();
        let base_url_slot = Arc::new(Mutex::new(String::new()));
        let base_url_for_handler = base_url_slot.clone();
        let server = TestHttpServer::with_handler(move |path| {
            let base_url = base_url_for_handler.lock().unwrap().clone();
            if path.contains("/paper/s2-123") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                        "paperId": "s2-123",
                        "title": "Semantic Retrieval for Agents",
                        "abstract": "Semantic Scholar abstract",
                        "authors": [{{"name": "Ada Lovelace"}}],
                        "venue": "NeurIPS",
                        "year": 2024,
                        "externalIds": {{"DOI": "10.1000/semantic.123"}},
                        "url": "{}/paper-page",
                        "openAccessPdf": {{"url": "{}/s2.pdf"}}
                    }}"#,
                        base_url, base_url
                    ),
                );
            }
            if path == "/paper-page" {
                return (
                    200,
                    "text/html",
                    "<html><body><h1>Semantic Retrieval for Agents</h1><p>Readable semantic scholar page body.</p></body></html>".to_string(),
                );
            }
            if path == "/s2.pdf" {
                return (200, "application/pdf", "%PDF-1.4 fake".to_string());
            }
            (404, "application/json", "{}".to_string())
        });
        *base_url_slot.lock().unwrap() = server.base_url.clone();

        let payload = with_remote_env_and_empty_local(&server.base_url, || {
            LiteratureTools.fetch_paper("s2:s2-123".into()).unwrap()
        });

        assert_eq!(payload["mode"], "remote");
        assert_eq!(payload["provider"], "semantic_scholar");
        assert_eq!(payload["paper"]["paper_id"], "s2:s2-123");
        assert_eq!(payload["paper"]["authors"][0], "Ada Lovelace");
        assert_eq!(
            payload["paper"]["urls"]["pdf"],
            format!("{}/s2.pdf", server.base_url)
        );
        assert_eq!(
            payload["structured_document"]["schema_version"],
            "structured_paper_document_v1"
        );
        assert_eq!(
            payload["structured_document"]["provenance"]["primary_source"],
            "remote"
        );
    }

    #[test]
    fn test_fetch_paper_remote_hydrates_html_body_text_when_pdf_unavailable() {
        let _guard = test_env_guard();
        let base_url_slot = Arc::new(Mutex::new(String::new()));
        let base_url_for_handler = base_url_slot.clone();
        let server = TestHttpServer::with_handler(move |path| {
            let base_url = base_url_for_handler.lock().unwrap().clone();
            if path.contains("/paper/s2-html?fields=") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                            "paperId": "s2-html",
                            "title": "Structured HTML Retrieval",
                            "abstract": "short abstract",
                            "authors": [{{"name": "Ada Lovelace"}}],
                            "venue": "ICML",
                            "year": 2024,
                            "externalIds": {{}},
                            "url": "{}/paper-page",
                            "openAccessPdf": null
                        }}"#,
                        base_url
                    ),
                );
            }
            if path == "/paper-page" {
                return (
                    200,
                    "text/html",
                    "<html><body><h1>Structured HTML Retrieval</h1><h2>Introduction</h2><p>This remote page contains readable body text for the agent.</p><h2>References</h2><p>[1] Example Ref</p></body></html>".to_string(),
                );
            }
            (404, "application/json", "{}".to_string())
        });
        *base_url_slot.lock().unwrap() = server.base_url.clone();

        let payload = with_remote_env_and_empty_local(&server.base_url, || {
            LiteratureTools.fetch_paper("s2:s2-html".into()).unwrap()
        });

        assert_eq!(payload["mode"], "remote");
        assert_eq!(payload["content_hydration"]["source"], "remote_text");
        assert!(payload["body_text"]
            .as_str()
            .unwrap_or("")
            .contains("readable body text for the agent"));
        assert_eq!(payload["content_hydration"]["format"], "html");
        assert_eq!(
            payload["structured_document"]["provenance"]["content_source"],
            "remote_text"
        );
        assert_eq!(
            payload["structured_document"]["quality"]["extraction_path"],
            "remote_text"
        );
        assert!(
            payload["structured_document"]["sections"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 1
        );
        assert!(payload["structured_document"]["body_text"]
            .as_str()
            .unwrap_or("")
            .contains("readable body text for the agent"));
    }

    #[test]
    fn test_fetch_paper_discovers_pdf_from_landing_page_before_text_fallback() {
        let _guard = test_env_guard();
        let base_url_slot = Arc::new(Mutex::new(String::new()));
        let base_url_for_handler = base_url_slot.clone();
        let server = TestHttpServer::with_handler(move |path| {
            let base_url = base_url_for_handler.lock().unwrap().clone();
            if path.contains("/paper/s2-discover?fields=") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                            "paperId": "s2-discover",
                            "title": "Landing PDF Discovery",
                            "abstract": "short abstract",
                            "authors": [{{"name": "Ada Lovelace"}}],
                            "venue": "ICML",
                            "year": 2024,
                            "externalIds": {{}},
                            "url": "{}/paper-page",
                            "openAccessPdf": null
                        }}"#,
                        base_url
                    ),
                );
            }
            if path == "/paper-page" {
                return (
                    200,
                    "text/html",
                    format!(
                        "<html><head><meta name=\"citation_pdf_url\" content=\"{}/paper.pdf\" /></head><body><h1>Landing PDF Discovery</h1><p>Readable fallback text.</p></body></html>",
                        base_url
                    ),
                );
            }
            if path == "/paper.pdf" {
                return (200, "application/pdf", "%PDF-1.4 fake".to_string());
            }
            (404, "application/json", "{}".to_string())
        });
        *base_url_slot.lock().unwrap() = server.base_url.clone();

        let payload = with_remote_env_and_empty_local(&server.base_url, || {
            LiteratureTools
                .fetch_paper("s2:s2-discover".into())
                .unwrap()
        });

        assert_eq!(payload["mode"], "remote");
        assert_eq!(
            payload["structured_document"]["provenance"]["attempted_pdf_url"],
            format!("{}/paper.pdf", server.base_url)
        );
        assert!(
            payload["content_hydration"]["source"] == "remote_pdf_discovered"
                || payload["content_hydration"]["source"] == "remote_text"
        );
        assert_eq!(
            payload["structured_document"]["quality"]["extraction_path"],
            payload["content_hydration"]["source"]
        );
        assert_eq!(
            payload["fulltext"]["attempted_pdf_url"],
            format!("{}/paper.pdf", server.base_url)
        );
        assert_eq!(
            payload["fulltext"]["completeness"],
            payload["structured_document"]["quality"]["completeness"]
        );
    }

    #[test]
    fn test_fetch_paper_local_markdown_exposes_structured_document_bundle() {
        let _guard = test_env_guard();
        let temp_dir = tempfile::tempdir().unwrap();
        let paper_path = temp_dir.path().join("structured_local.md");
        fs::write(
            &paper_path,
            "# Structured Local Paper\n\n## Introduction\nLocal full text body.\n\n## References\n[1] Local Ref",
        )
        .unwrap();

        std::env::set_var("AI_SCIENTIST_PAPERS_DIR", temp_dir.path());
        let payload = LiteratureTools
            .fetch_paper("structured_local".into())
            .unwrap();
        std::env::remove_var("AI_SCIENTIST_PAPERS_DIR");

        assert_eq!(payload["mode"], "local");
        assert_eq!(
            payload["structured_document"]["provenance"]["primary_source"],
            "local_fallback"
        );
        assert_eq!(payload["fulltext"]["primary_source"], "local_fallback");
        assert!(payload["structured_document"]["body_text"]
            .as_str()
            .unwrap_or("")
            .contains("Local full text body"));
        assert!(
            payload["structured_document"]["references"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 1
        );
        assert_eq!(
            payload["structured_document"]["quality"]["extraction_path"],
            "local_file"
        );
    }

    #[test]
    fn test_fetch_papers_returns_up_to_three_structured_remote_papers() {
        let _guard = test_env_guard();
        let base_url_slot = Arc::new(Mutex::new(String::new()));
        let base_url_for_handler = base_url_slot.clone();
        let server = TestHttpServer::with_handler(move |path| {
            let base_url = base_url_for_handler.lock().unwrap().clone();
            if path.contains("/paper/s2-a?fields=") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                        "paperId": "s2-a",
                        "title": "Paper A",
                        "abstract": "abstract a",
                        "authors": [{{"name": "Ada"}}],
                        "venue": "ICML",
                        "year": 2024,
                        "externalIds": {{}},
                        "url": "{}/a",
                        "openAccessPdf": null
                    }}"#,
                        base_url
                    ),
                );
            }
            if path.contains("/paper/s2-b?fields=") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                        "paperId": "s2-b",
                        "title": "Paper B",
                        "abstract": "abstract b",
                        "authors": [{{"name": "Barbara"}}],
                        "venue": "NeurIPS",
                        "year": 2023,
                        "externalIds": {{}},
                        "url": "{}/b",
                        "openAccessPdf": null
                    }}"#,
                        base_url
                    ),
                );
            }
            if path.contains("/paper/s2-c?fields=") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                        "paperId": "s2-c",
                        "title": "Paper C",
                        "abstract": "abstract c",
                        "authors": [{{"name": "Claude"}}],
                        "venue": "ICLR",
                        "year": 2022,
                        "externalIds": {{}},
                        "url": "{}/c",
                        "openAccessPdf": null
                    }}"#,
                        base_url
                    ),
                );
            }
            if path == "/a" || path == "/b" || path == "/c" {
                return (
                    200,
                    "text/html",
                    "<html><body><h1>Title</h1><h2>Introduction</h2><p>Readable remote body text.</p><h2>References</h2><p>[1] Ref</p></body></html>".to_string(),
                );
            }
            (404, "application/json", "{}".to_string())
        });
        *base_url_slot.lock().unwrap() = server.base_url.clone();

        let payload = with_remote_env_and_empty_local(&server.base_url, || {
            LiteratureTools
                .fetch_papers(
                    vec![
                        "s2:s2-a".into(),
                        "s2:s2-b".into(),
                        "s2:s2-c".into(),
                        "s2:s2-d".into(),
                    ],
                    None,
                )
                .unwrap()
        });

        assert_eq!(payload["status"], "success");
        assert_eq!(payload["fetched_count"], 3);
        assert_eq!(payload["limit_applied"], 3);
        assert_eq!(
            payload["results"].as_array().unwrap_or(&Vec::new()).len(),
            3
        );
        assert_eq!(payload["fulltext_bundle"]["requested_documents"], 3);
        assert_eq!(payload["fulltext_bundle"]["ready_documents"], 0);
        assert_eq!(payload["fulltext_bundle"]["partial_documents"], 3);
        assert_eq!(
            payload["results"][0]["structured_document"]["provenance"]["primary_source"],
            "remote"
        );
        assert_eq!(
            payload["results"][0]["fulltext"]["primary_source"],
            "remote"
        );
    }

    #[test]
    fn test_fetch_papers_reports_partial_failures() {
        let _guard = test_env_guard();
        let base_url_slot = Arc::new(Mutex::new(String::new()));
        let base_url_for_handler = base_url_slot.clone();
        let server = TestHttpServer::with_handler(move |path| {
            let base_url = base_url_for_handler.lock().unwrap().clone();
            if path.contains("/paper/s2-ok?fields=") {
                return (
                    200,
                    "application/json",
                    format!(
                        r#"{{
                        "paperId": "s2-ok",
                        "title": "Paper OK",
                        "abstract": "abstract ok",
                        "authors": [{{"name": "Ada"}}],
                        "venue": "ICML",
                        "year": 2024,
                        "externalIds": {{}},
                        "url": "{}/ok",
                        "openAccessPdf": null
                    }}"#,
                        base_url
                    ),
                );
            }
            if path == "/ok" {
                return (
                    200,
                    "text/html",
                    "<html><body><h1>Paper OK</h1><p>Readable remote body text.</p></body></html>"
                        .to_string(),
                );
            }
            (404, "application/json", "{}".to_string())
        });
        *base_url_slot.lock().unwrap() = server.base_url.clone();

        let payload = with_remote_env_and_empty_local(&server.base_url, || {
            LiteratureTools
                .fetch_papers(vec!["s2:s2-ok".into(), "missing-paper".into()], Some(2))
                .unwrap()
        });

        assert_eq!(payload["status"], "partial");
        assert_eq!(payload["fetched_count"], 1);
        assert_eq!(payload["errors"].as_array().unwrap_or(&Vec::new()).len(), 1);
        assert_eq!(payload["fulltext_bundle"]["requested_documents"], 1);
        assert_eq!(payload["fulltext_bundle"]["partial_documents"], 1);
    }
}
