//! GitHub search tools for agent-driven code and dataset discovery.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, USER_AGENT};
use serde::Deserialize;
use serde::Serialize;
use serde_json::{json, Value};
use std::process::Command;
use std::time::Duration;
use tokitai::tool;

pub struct GitHubTools;

const DEFAULT_GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_USER_AGENT: &str = "tokitai-ai-scientist/1.0";
const GITHUB_TOKEN_ENV_CANDIDATES: &[&str] = &[
    "GITHUB_TOKEN",
    "GITHUB_API_TOKEN",
    "GH_TOKEN",
    "GITHUB_PAT",
    "GITHUB_ACCESS_TOKEN",
];

#[derive(Debug, Clone, Serialize)]
struct GitHubRepositoryRecord {
    full_name: String,
    html_url: String,
    description: String,
    language: Option<String>,
    stargazers_count: u64,
    updated_at: Option<String>,
    topics: Vec<String>,
    default_branch: Option<String>,
    dataset_like: bool,
    match_reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct GitHubCodeRecord {
    name: String,
    path: String,
    html_url: String,
    repository_full_name: String,
    repository_url: String,
    repository_description: String,
    language: Option<String>,
    dataset_like: bool,
    match_reason: String,
    text_matches: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubSearchRepositoriesResponse {
    #[serde(default)]
    total_count: u64,
    #[serde(default)]
    items: Vec<GitHubRepositoryItem>,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubRepositoryItem {
    #[serde(default)]
    full_name: String,
    #[serde(default)]
    html_url: String,
    description: Option<String>,
    language: Option<String>,
    #[serde(default)]
    stargazers_count: u64,
    updated_at: Option<String>,
    #[serde(default)]
    topics: Vec<String>,
    default_branch: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubSearchCodeResponse {
    #[serde(default)]
    total_count: u64,
    #[serde(default)]
    items: Vec<GitHubCodeItem>,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubCodeItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    html_url: String,
    repository: GitHubRepositoryItem,
    #[serde(default)]
    text_matches: Vec<GitHubTextMatch>,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubTextMatch {
    fragment: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubRepoContentItem {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    sha: String,
    #[serde(default)]
    size: u64,
    #[serde(rename = "type", default)]
    item_type: String,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct GitHubRepoContentFile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    sha: String,
    #[serde(default)]
    size: u64,
    #[serde(rename = "type", default)]
    item_type: String,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    download_url: Option<String>,
    #[serde(default)]
    encoding: Option<String>,
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubRepoResponse {
    #[serde(default)]
    full_name: String,
    #[serde(default)]
    default_branch: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    stargazers_count: u64,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    html_url: String,
}

#[derive(Debug, Deserialize, Default)]
struct GitHubCommitListItem {
    #[serde(default)]
    sha: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    commit: GitHubCommitEnvelope,
    #[serde(default)]
    parents: Vec<GitHubCommitParent>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct GitHubCommitEnvelope {
    #[serde(default)]
    message: String,
    author: Option<GitHubCommitSignature>,
    committer: Option<GitHubCommitSignature>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct GitHubCommitSignature {
    name: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct GitHubCommitParent {
    #[serde(default)]
    sha: String,
    #[serde(default)]
    html_url: String,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct GitHubCommitDetailFile {
    #[serde(default)]
    sha: String,
    #[serde(default)]
    filename: String,
    previous_filename: Option<String>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    additions: u64,
    #[serde(default)]
    deletions: u64,
    #[serde(default)]
    changes: u64,
    patch: Option<String>,
    blob_url: Option<String>,
    raw_url: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct GitHubCommitDetailResponse {
    #[serde(default)]
    sha: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    commit: GitHubCommitEnvelope,
    #[serde(default)]
    parents: Vec<GitHubCommitParent>,
    #[serde(default)]
    files: Vec<GitHubCommitDetailFile>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct GitHubCompareResponse {
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    permalink_url: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    ahead_by: u64,
    #[serde(default)]
    behind_by: u64,
    #[serde(default)]
    total_commits: u64,
    #[serde(default)]
    files: Vec<GitHubCommitDetailFile>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct GitHubPreviewDiffHunk {
    header: String,
    lines: Vec<GitHubPreviewDiffLine>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct GitHubPreviewDiffLine {
    kind: String,
    old_number: Option<usize>,
    new_number: Option<usize>,
    content: String,
}

#[derive(Debug, Deserialize, Default)]
struct OnnxHubManifestEntry {
    #[serde(default)]
    model: String,
    #[serde(default)]
    model_path: String,
    #[serde(default)]
    onnx_version: String,
    #[serde(default)]
    opset_version: u64,
    #[serde(default)]
    metadata: OnnxHubManifestMetadata,
}

#[derive(Debug, Deserialize, Default)]
struct OnnxHubManifestMetadata {
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    model_bytes: Option<u64>,
    #[serde(default)]
    model_with_data_bytes: Option<u64>,
}

fn detect_github_api_base() -> String {
    std::env::var("GITHUB_API_BASE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_GITHUB_API_BASE.to_string())
}

#[derive(Debug, Clone)]
pub struct GitHubTokenDetection {
    pub token: String,
    pub source: String,
}

pub fn detect_github_api_base_public() -> String {
    detect_github_api_base()
}

pub fn detect_github_token() -> Option<GitHubTokenDetection> {
    for env_name in GITHUB_TOKEN_ENV_CANDIDATES {
        if let Ok(value) = std::env::var(env_name) {
            let token = value.trim().to_string();
            if !token.is_empty() {
                return Some(GitHubTokenDetection {
                    token,
                    source: format!("env:{}", env_name),
                });
            }
        }
    }

    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if token.is_empty() {
        return None;
    }

    Some(GitHubTokenDetection {
        token,
        source: "gh.auth.token".to_string(),
    })
}

fn github_client() -> Result<Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, HeaderValue::from_static(GITHUB_USER_AGENT));
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/vnd.github+json"),
    );
    if let Some(detected) = detect_github_token() {
        let header_value = format!("Bearer {}", detected.token);
        let parsed = HeaderValue::from_str(&header_value)
            .map_err(|err| format!("invalid GitHub token header: {}", err))?;
        headers.insert(AUTHORIZATION, parsed);
    }
    Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| format!("failed to build GitHub client: {}", err))
}

fn collapse_snippet(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed.trim();
    if trimmed.len() <= 220 {
        trimmed.to_string()
    } else {
        let mut snippet = trimmed.chars().take(217).collect::<String>();
        snippet.push_str("...");
        snippet
    }
}

fn onnx_model_search(limit: usize, query: &str) -> Result<Value, String> {
    let trimmed_query = query.trim();
    let manifest_file = github_fetch_file("onnx/models", "main", "ONNX_HUB_MANIFEST.json")
        .map_err(|err| format!("search_onnx_models: {}", err))?;
    let manifest_content = github_decode_file_content(&manifest_file);
    if manifest_content.trim().is_empty() {
        return Err("search_onnx_models: ONNX manifest content is empty".to_string());
    }
    let payload = serde_json::from_str::<Vec<OnnxHubManifestEntry>>(&manifest_content)
        .map_err(|err| format!("search_onnx_models: invalid ONNX manifest JSON: {}", err))?;

    let lowered_query = trimmed_query.to_ascii_lowercase();
    let query_tokens = lowered_query
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_string())
        .collect::<Vec<_>>();

    let mut ranked = payload
        .into_iter()
        .filter_map(|entry| {
            let model = entry.model.trim().to_string();
            let model_path = entry.model_path.trim().to_string();
            if model.is_empty() || model_path.is_empty() {
                return None;
            }
            let tags = entry
                .metadata
                .tags
                .into_iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>();
            let corpus = format!(
                "{} {} {}",
                model.to_ascii_lowercase(),
                model_path.to_ascii_lowercase(),
                tags.join(" ").to_ascii_lowercase()
            );
            let mut score = 0i32;
            if lowered_query.is_empty() {
                score = 1;
            } else {
                if corpus.contains(&lowered_query) {
                    score += 12;
                }
                if model.to_ascii_lowercase().contains(&lowered_query) {
                    score += 16;
                }
                for token in &query_tokens {
                    if corpus.contains(token) {
                        score += 3;
                    }
                    if model.to_ascii_lowercase().contains(token) {
                        score += 4;
                    }
                }
            }
            if score <= 0 {
                return None;
            }
            let family = model_path
                .split('/')
                .nth_back(2)
                .unwrap_or("")
                .replace('_', " ");
            let category = model_path
                .split('/')
                .nth(1)
                .or_else(|| model_path.split('/').next())
                .unwrap_or("")
                .replace('_', " ");
            let download_url = format!(
                "https://raw.githubusercontent.com/onnx/models/main/{}",
                model_path
            );
            let repo_url = format!(
                "https://github.com/onnx/models/blob/main/{}",
                model_path
            );
            let size_bytes = entry
                .metadata
                .model_with_data_bytes
                .or(entry.metadata.model_bytes);
            Some((
                score,
                json!({
                    "title": model,
                    "model_name": model,
                    "family": family,
                    "category": category,
                    "path": model_path,
                    "url": repo_url,
                    "download_url": download_url,
                    "provider": "onnx-model-zoo",
                    "onnx_version": entry.onnx_version,
                    "opset_version": entry.opset_version,
                    "tags": tags,
                    "size_bytes": size_bytes,
                    "snippet": format!(
                        "{} | opset {} | {}",
                        if category.is_empty() { "ONNX Model Zoo".to_string() } else { category.clone() },
                        entry.opset_version,
                        if family.is_empty() { "official model artifact".to_string() } else { family.clone() }
                    ),
                }),
            ))
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| {
                let left_title = left.1.get("title").and_then(Value::as_str).unwrap_or("");
                let right_title = right.1.get("title").and_then(Value::as_str).unwrap_or("");
                left_title.cmp(right_title)
            })
    });

    let results = ranked
        .into_iter()
        .take(limit.clamp(1, 20))
        .map(|(_, value)| value)
        .collect::<Vec<_>>();

    Ok(json!({
        "operation": "search_onnx_models",
        "query": trimmed_query,
        "provider": "onnx-model-zoo",
        "source_repo": "onnx/models",
        "manifest_url": "https://github.com/onnx/models/blob/main/ONNX_HUB_MANIFEST.json",
        "total": results.len(),
        "results": results,
        "status": if results.is_empty() { "empty" } else { "ok" },
        "hints": if results.is_empty() {
            vec!["No ONNX Model Zoo entries matched this query in the official hub manifest.".to_string()]
        } else {
            Vec::<String>::new()
        },
    }))
}

fn github_repo_match_reason(query: &str, item: &GitHubRepositoryItem) -> String {
    let lowered = format!(
        "{} {} {}",
        item.full_name.to_ascii_lowercase(),
        item.description
            .clone()
            .unwrap_or_default()
            .to_ascii_lowercase(),
        item.topics.join(" ").to_ascii_lowercase()
    );
    if looks_like_dataset_repo(item, query) {
        "dataset_or_benchmark_repo".to_string()
    } else if lowered.contains("benchmark") {
        "benchmark_repo".to_string()
    } else {
        "code_repository".to_string()
    }
}

fn looks_like_dataset_repo(item: &GitHubRepositoryItem, query: &str) -> bool {
    let corpus = format!(
        "{} {} {} {}",
        item.full_name,
        item.description.clone().unwrap_or_default(),
        item.topics.join(" "),
        query
    )
    .to_ascii_lowercase();
    [
        "dataset",
        "datasets",
        "corpus",
        "benchmark",
        "data",
        "task suite",
        "eval",
        "evaluation set",
    ]
    .iter()
    .any(|needle| corpus.contains(needle))
}

fn normalize_repository(item: GitHubRepositoryItem, query: &str) -> Option<GitHubRepositoryRecord> {
    let full_name = item.full_name.trim().to_string();
    let html_url = item.html_url.trim().to_string();
    if full_name.is_empty() || html_url.is_empty() {
        return None;
    }
    let dataset_like = looks_like_dataset_repo(&item, query);
    let match_reason = github_repo_match_reason(query, &item);
    Some(GitHubRepositoryRecord {
        full_name,
        html_url,
        description: collapse_snippet(item.description.as_deref().unwrap_or("")),
        language: item.language,
        stargazers_count: item.stargazers_count,
        updated_at: item.updated_at,
        topics: item.topics,
        default_branch: item.default_branch,
        dataset_like,
        match_reason,
    })
}

fn normalize_code_item(item: GitHubCodeItem, query: &str) -> Option<GitHubCodeRecord> {
    let repository_full_name = item.repository.full_name.trim().to_string();
    let repository_url = item.repository.html_url.trim().to_string();
    let html_url = item.html_url.trim().to_string();
    let path = item.path.trim().to_string();
    let name = item.name.trim().to_string();
    if repository_full_name.is_empty()
        || repository_url.is_empty()
        || html_url.is_empty()
        || path.is_empty()
        || name.is_empty()
    {
        return None;
    }
    let dataset_like = looks_like_dataset_repo(&item.repository, query);
    Some(GitHubCodeRecord {
        name,
        path,
        html_url,
        repository_full_name,
        repository_url,
        repository_description: collapse_snippet(
            item.repository.description.as_deref().unwrap_or(""),
        ),
        language: item.repository.language,
        dataset_like,
        match_reason: if dataset_like {
            "code_match_inside_dataset_or_benchmark_repo".to_string()
        } else {
            "code_match".to_string()
        },
        text_matches: item
            .text_matches
            .into_iter()
            .filter_map(|entry| entry.fragment)
            .map(|fragment| collapse_snippet(&fragment))
            .filter(|fragment| !fragment.is_empty())
            .take(3)
            .collect(),
    })
}

fn github_search_repositories(query: &str, limit: usize) -> Result<Value, String> {
    let client = github_client()?;
    let response = client
        .get(format!(
            "{}/search/repositories",
            detect_github_api_base().trim_end_matches('/')
        ))
        .query(&[
            ("q", query.trim()),
            ("sort", "stars"),
            ("order", "desc"),
            ("per_page", &limit.min(10).max(1).to_string()),
        ])
        .send()
        .map_err(|err| {
            format!(
                "search_github_repositories: failed to reach GitHub API: {}",
                err
            )
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "search_github_repositories: GitHub API returned HTTP {}",
            response.status()
        ));
    }
    let payload = response
        .json::<GitHubSearchRepositoriesResponse>()
        .map_err(|err| {
            format!(
                "search_github_repositories: invalid GitHub response JSON: {}",
                err
            )
        })?;
    let items = payload
        .items
        .into_iter()
        .filter_map(|item| normalize_repository(item, query))
        .collect::<Vec<_>>();
    Ok(json!({
        "operation": "search_github_repositories",
        "query": query,
        "total_count_hint": payload.total_count,
        "total": items.len(),
        "results": items,
    }))
}

fn github_search_code(query: &str, limit: usize) -> Result<Value, String> {
    let client = github_client()?;
    let response = client
        .get(format!(
            "{}/search/code",
            detect_github_api_base().trim_end_matches('/')
        ))
        .header(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github.text-match+json"),
        )
        .query(&[
            ("q", query.trim()),
            ("per_page", &limit.min(10).max(1).to_string()),
        ])
        .send()
        .map_err(|err| format!("search_github_code: failed to reach GitHub API: {}", err))?;
    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        let fallback = github_search_repositories(query, limit)?;
        return Ok(json!({
            "operation": "search_github_code",
            "query": query,
            "mode": "repository_fallback",
            "auth_required": true,
            "detail": format!("GitHub code search returned HTTP {}. Falling back to repository-level search; set GITHUB_TOKEN to enable direct code search.", response.status()),
            "fallback": fallback,
        }));
    }
    if !response.status().is_success() {
        return Err(format!(
            "search_github_code: GitHub API returned HTTP {}",
            response.status()
        ));
    }
    let payload = response
        .json::<GitHubSearchCodeResponse>()
        .map_err(|err| format!("search_github_code: invalid GitHub response JSON: {}", err))?;
    let items = payload
        .items
        .into_iter()
        .filter_map(|item| normalize_code_item(item, query))
        .collect::<Vec<_>>();
    Ok(json!({
        "operation": "search_github_code",
        "query": query,
        "total_count_hint": payload.total_count,
        "total": items.len(),
        "results": items,
    }))
}

fn parse_repo_full_name_and_path(raw: &str) -> Result<(String, Option<String>), String> {
    let trimmed = raw.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Err("github preview requires repo_full_name".to_string());
    }
    let parts = trimmed.split('/').collect::<Vec<_>>();
    if parts.len() < 2 {
        return Err("github preview expects owner/repo".to_string());
    }
    let repo_full_name = format!("{}/{}", parts[0], parts[1]);
    let path = if parts.len() > 2 {
        Some(parts[2..].join("/"))
    } else {
        None
    };
    Ok((
        repo_full_name,
        path.filter(|value| !value.trim().is_empty()),
    ))
}

fn github_fetch_repo(repo_full_name: &str) -> Result<GitHubRepoResponse, String> {
    let client = github_client()?;
    let response = client
        .get(format!(
            "{}/repos/{}",
            detect_github_api_base().trim_end_matches('/'),
            repo_full_name.trim()
        ))
        .send()
        .map_err(|err| format!("github_preview: failed to reach GitHub repo API: {}", err))?;
    if !response.status().is_success() {
        return Err(format!(
            "github_preview: repo API returned HTTP {}",
            response.status()
        ));
    }
    response
        .json::<GitHubRepoResponse>()
        .map_err(|err| format!("github_preview: invalid repo response JSON: {}", err))
}

fn github_fetch_tree(
    repo_full_name: &str,
    branch: &str,
    path: Option<&str>,
) -> Result<Vec<GitHubRepoContentItem>, String> {
    let client = github_client()?;
    let normalized_path = path
        .map(|value| value.trim().trim_matches('/').to_string())
        .filter(|value| !value.is_empty());
    let mut url = format!(
        "{}/repos/{}/contents",
        detect_github_api_base().trim_end_matches('/'),
        repo_full_name.trim()
    );
    if let Some(path) = normalized_path.as_deref() {
        url.push('/');
        url.push_str(path);
    }
    let response = client
        .get(url)
        .query(&[("ref", branch.trim())])
        .send()
        .map_err(|err| format!("github_preview: failed to reach repo contents API: {}", err))?;
    if !response.status().is_success() {
        return Err(format!(
            "github_preview: repo contents API returned HTTP {}",
            response.status()
        ));
    }
    response
        .json::<Vec<GitHubRepoContentItem>>()
        .map_err(|err| format!("github_preview: invalid repo contents JSON: {}", err))
}

fn github_fetch_file(
    repo_full_name: &str,
    reference: &str,
    path: &str,
) -> Result<GitHubRepoContentFile, String> {
    let client = github_client()?;
    let response = client
        .get(format!(
            "{}/repos/{}/contents/{}",
            detect_github_api_base().trim_end_matches('/'),
            repo_full_name.trim(),
            path.trim().trim_matches('/')
        ))
        .query(&[("ref", reference.trim())])
        .send()
        .map_err(|err| format!("github_preview: failed to reach file contents API: {}", err))?;
    if !response.status().is_success() {
        return Err(format!(
            "github_preview: file contents API returned HTTP {}",
            response.status()
        ));
    }
    response
        .json::<GitHubRepoContentFile>()
        .map_err(|err| format!("github_preview: invalid file contents JSON: {}", err))
}

fn github_fetch_commits(
    repo_full_name: &str,
    reference: &str,
    path: Option<&str>,
    limit: usize,
) -> Result<Vec<GitHubCommitListItem>, String> {
    let client = github_client()?;
    let mut params = vec![
        ("sha", reference.trim().to_string()),
        ("per_page", limit.min(12).max(1).to_string()),
    ];
    if let Some(path) = path
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        params.push(("path", path.to_string()));
    }
    let response = client
        .get(format!(
            "{}/repos/{}/commits",
            detect_github_api_base().trim_end_matches('/'),
            repo_full_name.trim()
        ))
        .query(&params)
        .send()
        .map_err(|err| format!("github_preview: failed to reach commits API: {}", err))?;
    if !response.status().is_success() {
        return Err(format!(
            "github_preview: commits API returned HTTP {}",
            response.status()
        ));
    }
    response
        .json::<Vec<GitHubCommitListItem>>()
        .map_err(|err| format!("github_preview: invalid commits response JSON: {}", err))
}

fn github_fetch_commit_detail(
    repo_full_name: &str,
    commit_sha: &str,
) -> Result<GitHubCommitDetailResponse, String> {
    let client = github_client()?;
    let response = client
        .get(format!(
            "{}/repos/{}/commits/{}",
            detect_github_api_base().trim_end_matches('/'),
            repo_full_name.trim(),
            commit_sha.trim()
        ))
        .send()
        .map_err(|err| format!("github_preview: failed to reach commit detail API: {}", err))?;
    if !response.status().is_success() {
        return Err(format!(
            "github_preview: commit detail API returned HTTP {}",
            response.status()
        ));
    }
    response
        .json::<GitHubCommitDetailResponse>()
        .map_err(|err| format!("github_preview: invalid commit detail JSON: {}", err))
}

fn github_fetch_compare(
    repo_full_name: &str,
    base_sha: &str,
    head_sha: &str,
) -> Result<GitHubCompareResponse, String> {
    let client = github_client()?;
    let response = client
        .get(format!(
            "{}/repos/{}/compare/{}...{}",
            detect_github_api_base().trim_end_matches('/'),
            repo_full_name.trim(),
            base_sha.trim(),
            head_sha.trim()
        ))
        .send()
        .map_err(|err| format!("github_preview: failed to reach compare API: {}", err))?;
    if !response.status().is_success() {
        return Err(format!(
            "github_preview: compare API returned HTTP {}",
            response.status()
        ));
    }
    response
        .json::<GitHubCompareResponse>()
        .map_err(|err| format!("github_preview: invalid compare JSON: {}", err))
}

fn github_decode_file_content(file: &GitHubRepoContentFile) -> String {
    let Some(raw_content) = file.content.as_deref() else {
        return String::new();
    };
    if file
        .encoding
        .as_deref()
        .is_some_and(|encoding| encoding.eq_ignore_ascii_case("base64"))
    {
        let compact = raw_content.lines().collect::<String>();
        return BASE64_STANDARD
            .decode(compact.as_bytes())
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .unwrap_or_default();
    }
    raw_content.replace("\r\n", "\n")
}

fn github_file_preview_snippet(content: &str) -> String {
    collapse_snippet(content)
}

fn github_blob_language(path: &str) -> &'static str {
    let normalized = path.trim().to_ascii_lowercase();
    if normalized.ends_with(".rs") {
        "rust"
    } else if normalized.ends_with(".js")
        || normalized.ends_with(".mjs")
        || normalized.ends_with(".cjs")
    {
        "javascript"
    } else if normalized.ends_with(".ts") || normalized.ends_with(".tsx") {
        "typescript"
    } else if normalized.ends_with(".py") {
        "python"
    } else if normalized.ends_with(".md") || normalized.ends_with(".markdown") {
        "markdown"
    } else if normalized.ends_with(".json") {
        "json"
    } else if normalized.ends_with(".html") || normalized.ends_with(".htm") {
        "html"
    } else if normalized.ends_with(".css") {
        "css"
    } else if normalized.ends_with(".yml") || normalized.ends_with(".yaml") {
        "yaml"
    } else if normalized.ends_with(".sh") || normalized.ends_with(".bash") {
        "shell"
    } else if normalized.ends_with(".go") {
        "go"
    } else if normalized.ends_with(".java") {
        "java"
    } else if normalized.ends_with(".c")
        || normalized.ends_with(".h")
        || normalized.ends_with(".cc")
        || normalized.ends_with(".cpp")
        || normalized.ends_with(".hpp")
    {
        "cpp"
    } else if normalized.ends_with(".tex") {
        "latex"
    } else if normalized.ends_with(".xml") {
        "xml"
    } else if normalized.ends_with(".toml") {
        "toml"
    } else if normalized.ends_with(".ini")
        || normalized.ends_with(".cfg")
        || normalized.ends_with(".conf")
    {
        "ini"
    } else {
        "text"
    }
}

fn github_commit_subject(message: &str) -> String {
    message
        .lines()
        .next()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "commit".to_string())
}

fn github_commit_message_preview(message: &str) -> String {
    collapse_snippet(message)
}

fn github_commit_author_name(commit: &GitHubCommitEnvelope) -> String {
    commit
        .author
        .as_ref()
        .and_then(|entry| entry.name.clone())
        .or_else(|| {
            commit
                .committer
                .as_ref()
                .and_then(|entry| entry.name.clone())
        })
        .unwrap_or_default()
}

fn github_commit_date(commit: &GitHubCommitEnvelope) -> String {
    commit
        .author
        .as_ref()
        .and_then(|entry| entry.date.clone())
        .or_else(|| {
            commit
                .committer
                .as_ref()
                .and_then(|entry| entry.date.clone())
        })
        .unwrap_or_default()
}

fn github_commit_matches_path(file: &GitHubCommitDetailFile, path: &str) -> bool {
    let normalized = path.trim().trim_matches('/');
    !normalized.is_empty()
        && (file.filename.eq_ignore_ascii_case(normalized)
            || file
                .previous_filename
                .as_deref()
                .is_some_and(|value| value.eq_ignore_ascii_case(normalized)))
}

fn github_find_commit_file<'a>(
    detail: &'a GitHubCommitDetailResponse,
    path: &str,
) -> Option<&'a GitHubCommitDetailFile> {
    detail
        .files
        .iter()
        .find(|file| github_commit_matches_path(file, path))
}

fn github_history_commit_payload(
    sha: &str,
    html_url: &str,
    commit: &GitHubCommitEnvelope,
    parents: &[GitHubCommitParent],
) -> Value {
    json!({
        "sha": sha,
        "short_sha": sha.chars().take(7).collect::<String>(),
        "html_url": html_url,
        "subject": github_commit_subject(&commit.message),
        "message": github_commit_message_preview(&commit.message),
        "author": github_commit_author_name(commit),
        "date": github_commit_date(commit),
        "parent_shas": parents
            .iter()
            .map(|parent| parent.sha.clone())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>(),
    })
}

fn parse_github_hunk_positions(header: &str) -> (usize, usize) {
    let mut old_start = 0usize;
    let mut new_start = 0usize;
    let parts = header.split_whitespace().collect::<Vec<_>>();
    if let Some(old_part) = parts.get(1) {
        old_start = old_part
            .trim_start_matches('-')
            .split(',')
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
    }
    if let Some(new_part) = parts.get(2) {
        new_start = new_part
            .trim_start_matches('+')
            .split(',')
            .next()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
    }
    (old_start, new_start)
}

fn parse_github_patch_hunks(patch: &str) -> Vec<GitHubPreviewDiffHunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<GitHubPreviewDiffHunk> = None;
    let mut old_line = 0usize;
    let mut new_line = 0usize;

    for line in patch.lines() {
        if line.starts_with("@@") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }
            let (old_start, new_start) = parse_github_hunk_positions(line);
            old_line = old_start;
            new_line = new_start;
            current_hunk = Some(GitHubPreviewDiffHunk {
                header: line.to_string(),
                lines: Vec::new(),
            });
            continue;
        }
        let Some(hunk) = current_hunk.as_mut() else {
            continue;
        };
        if line.starts_with('+') && !line.starts_with("+++") {
            hunk.lines.push(GitHubPreviewDiffLine {
                kind: "added".to_string(),
                old_number: None,
                new_number: Some(new_line),
                content: line[1..].to_string(),
            });
            new_line += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            hunk.lines.push(GitHubPreviewDiffLine {
                kind: "removed".to_string(),
                old_number: Some(old_line),
                new_number: None,
                content: line[1..].to_string(),
            });
            old_line += 1;
        } else if line.starts_with(' ') {
            hunk.lines.push(GitHubPreviewDiffLine {
                kind: "context".to_string(),
                old_number: Some(old_line),
                new_number: Some(new_line),
                content: line[1..].to_string(),
            });
            old_line += 1;
            new_line += 1;
        }
    }

    if let Some(hunk) = current_hunk.take() {
        hunks.push(hunk);
    }

    hunks
}

fn github_commit_diff_payload(detail: &GitHubCommitDetailResponse, path: &str) -> Value {
    let Some(file) = github_find_commit_file(detail, path) else {
        return json!({
            "available": false,
            "detail": "selected file was not changed in the chosen commit",
        });
    };
    github_diff_file_payload(
        file,
        detail.parents.first().map(|parent| parent.sha.clone()),
    )
}

fn github_commit_repo_diff_payload(detail: &GitHubCommitDetailResponse) -> Value {
    let files = detail
        .files
        .iter()
        .map(|file| {
            github_diff_file_payload(
                file,
                detail.parents.first().map(|parent| parent.sha.clone()),
            )
        })
        .collect::<Vec<_>>();
    let text_diff_file_count = files
        .iter()
        .filter(|entry| {
            entry
                .get("available")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    json!({
        "available": !files.is_empty(),
        "detail": if files.is_empty() {
            "The selected commit does not expose changed files."
        } else {
            ""
        },
        "scope": "repository",
        "file_count": files.len(),
        "text_diff_file_count": text_diff_file_count,
        "files": files,
    })
}

fn github_diff_file_payload(file: &GitHubCommitDetailFile, parent_sha: Option<String>) -> Value {
    let patch = file.patch.clone().unwrap_or_default();
    let hunks = if patch.trim().is_empty() {
        Vec::new()
    } else {
        parse_github_patch_hunks(&patch)
    };
    json!({
        "available": !patch.trim().is_empty() && !hunks.is_empty(),
        "path": file.filename,
        "previous_path": file.previous_filename.clone().unwrap_or_default(),
        "status": file.status,
        "sha": file.sha,
        "blob_url": file.blob_url,
        "raw_url": file.raw_url,
        "patch": patch,
        "additions": file.additions,
        "deletions": file.deletions,
        "changes": file.changes,
        "parent_sha": parent_sha.unwrap_or_default(),
        "hunks": hunks,
        "detail": if patch.trim().is_empty() {
            "GitHub did not return a text patch for this file revision."
        } else {
            ""
        }
    })
}

fn github_compare_payload(
    compare: &GitHubCompareResponse,
    base_sha: &str,
    head_sha: &str,
    base_commit: Option<&GitHubCommitDetailResponse>,
    head_commit: Option<&GitHubCommitDetailResponse>,
) -> Value {
    let files = compare
        .files
        .iter()
        .map(|file| github_diff_file_payload(file, Some(base_sha.to_string())))
        .collect::<Vec<_>>();
    let text_diff_file_count = files
        .iter()
        .filter(|entry| {
            entry
                .get("available")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let base_commit_payload = base_commit
        .map(|detail| {
            github_history_commit_payload(
                &detail.sha,
                &detail.html_url,
                &detail.commit,
                &detail.parents,
            )
        })
        .unwrap_or_else(|| {
            json!({
                "sha": base_sha,
                "short_sha": base_sha.chars().take(7).collect::<String>(),
            })
        });
    let head_commit_payload = head_commit
        .map(|detail| {
            github_history_commit_payload(
                &detail.sha,
                &detail.html_url,
                &detail.commit,
                &detail.parents,
            )
        })
        .unwrap_or_else(|| {
            json!({
                "sha": head_sha,
                "short_sha": head_sha.chars().take(7).collect::<String>(),
            })
        });
    json!({
        "available": !files.is_empty(),
        "html_url": compare.html_url,
        "permalink_url": compare.permalink_url,
        "status": compare.status,
        "ahead_by": compare.ahead_by,
        "behind_by": compare.behind_by,
        "total_commits": compare.total_commits,
        "file_count": files.len(),
        "text_diff_file_count": text_diff_file_count,
        "base_sha": base_sha,
        "head_sha": head_sha,
        "base_commit": base_commit_payload,
        "head_commit": head_commit_payload,
        "files": files,
        "detail": if files.is_empty() {
            "No files changed between the selected commits."
        } else {
            ""
        }
    })
}

fn github_selected_file_payload(
    file: GitHubRepoContentFile,
    html_url_override: Option<String>,
    download_url_override: Option<String>,
) -> Value {
    let content = github_decode_file_content(&file);
    let snippet = github_file_preview_snippet(&content);
    json!({
        "name": file.name,
        "path": file.path,
        "html_url": html_url_override.or(file.html_url),
        "download_url": download_url_override.or(file.download_url),
        "size": file.size,
        "sha": file.sha,
        "language": github_blob_language(&file.path),
        "content": content,
        "snippet": snippet
    })
}

fn github_readme_payload(
    file: GitHubRepoContentFile,
    html_url_override: Option<String>,
    download_url_override: Option<String>,
) -> Value {
    let content = github_decode_file_content(&file);
    let snippet = github_file_preview_snippet(&content);
    json!({
        "path": file.path,
        "name": file.name,
        "html_url": html_url_override.or(file.html_url),
        "download_url": download_url_override.or(file.download_url),
        "language": github_blob_language(&file.path),
        "content": content,
        "snippet": snippet
    })
}

fn parent_repo_path(path: &str) -> Option<String> {
    let normalized = path.trim().trim_matches('/');
    if normalized.is_empty() {
        return None;
    }
    let mut parts = normalized.split('/').collect::<Vec<_>>();
    if parts.len() <= 1 {
        return None;
    }
    parts.pop();
    Some(parts.join("/"))
}

fn github_history_scope_path(
    history_scope_mode: Option<&str>,
    selected_file_path: Option<&str>,
    resolved_path: Option<&str>,
) -> String {
    if history_scope_mode
        .map(|value| value.trim())
        .is_some_and(|value| value.eq_ignore_ascii_case("repository"))
    {
        return String::new();
    }
    selected_file_path
        .map(|value| value.trim().trim_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            resolved_path
                .map(|value| value.trim().trim_matches('/').to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn github_preview(
    repo_or_path: &str,
    branch: Option<String>,
    path: Option<String>,
    commit_sha: Option<String>,
    compare_base_sha: Option<String>,
    compare_head_sha: Option<String>,
    history_scope_mode: Option<String>,
) -> Result<Value, String> {
    let (repo_full_name, inferred_path) = parse_repo_full_name_and_path(repo_or_path)?;
    let repo = github_fetch_repo(&repo_full_name)?;
    let branch_name = branch
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or(repo.default_branch.clone())
        .unwrap_or_else(|| "main".to_string());
    let selected_commit_sha = commit_sha
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let selected_compare_base_sha = compare_base_sha
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let selected_compare_head_sha = compare_head_sha
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| selected_commit_sha.clone());
    let selected_commit_detail = selected_commit_sha
        .as_deref()
        .map(|sha| github_fetch_commit_detail(&repo_full_name, sha))
        .transpose()?;
    let content_reference = selected_commit_sha
        .clone()
        .unwrap_or_else(|| branch_name.clone());
    let preview_path = path
        .map(|value| value.trim().trim_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .or(inferred_path);
    let mut preview_target_kind = if preview_path.is_some() {
        "directory".to_string()
    } else {
        "repository_root".to_string()
    };
    let mut selected_file = None;
    let mut resolved_path = preview_path.clone();
    let selected_commit_file = selected_commit_detail
        .as_ref()
        .and_then(|detail| {
            preview_path
                .as_deref()
                .and_then(|entry| github_find_commit_file(detail, entry))
        })
        .cloned();
    let tree = if let Some(path) = preview_path.as_deref() {
        let mut file_candidates = vec![path.to_string()];
        if let Some(commit_file) = selected_commit_file.as_ref() {
            if !commit_file.filename.trim().is_empty()
                && !file_candidates
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(&commit_file.filename))
            {
                file_candidates.push(commit_file.filename.clone());
            }
            if let Some(previous_path) = commit_file.previous_filename.as_deref() {
                let previous_path = previous_path.trim().to_string();
                if !previous_path.is_empty()
                    && !file_candidates
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(&previous_path))
                {
                    file_candidates.push(previous_path);
                }
            }
        }
        let fetched_file = file_candidates.iter().find_map(|candidate| {
            github_fetch_file(&repo_full_name, &content_reference, candidate)
                .ok()
                .map(|file| (candidate.clone(), file))
        });
        match fetched_file {
            Some((_, file)) => {
                preview_target_kind = "file".to_string();
                selected_file = Some(file);
                resolved_path = selected_file.as_ref().map(|file| file.path.clone());
                let parent = parent_repo_path(
                    selected_file
                        .as_ref()
                        .map(|file| file.path.as_str())
                        .unwrap_or(path),
                );
                github_fetch_tree(&repo_full_name, &content_reference, parent.as_deref())
                    .unwrap_or_default()
            }
            None => github_fetch_tree(&repo_full_name, &content_reference, Some(path))
                .unwrap_or_default(),
        }
    } else {
        github_fetch_tree(&repo_full_name, &content_reference, None).unwrap_or_default()
    };
    let readme_path = tree
        .iter()
        .find(|item| {
            item.item_type.eq_ignore_ascii_case("file")
                && item.name.to_ascii_lowercase().starts_with("readme")
        })
        .map(|item| item.path.clone());
    let selected_commit_file_html = selected_commit_file
        .as_ref()
        .and_then(|file| file.blob_url.clone());
    let selected_commit_file_raw = selected_commit_file
        .as_ref()
        .and_then(|file| file.raw_url.clone());
    let readme = selected_file
        .as_ref()
        .filter(|file| file.name.to_ascii_lowercase().starts_with("readme"))
        .cloned()
        .or_else(|| {
            readme_path.as_deref().and_then(|candidate| {
                github_fetch_file(&repo_full_name, &content_reference, candidate).ok()
            })
        })
        .map(|file| {
            github_readme_payload(
                file,
                selected_commit_file_html.clone(),
                selected_commit_file_raw.clone(),
            )
        })
        .unwrap_or_else(|| json!({}));
    let history_scope_mode = history_scope_mode
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "selection".to_string());
    let history_scope_path = github_history_scope_path(
        Some(&history_scope_mode),
        selected_file.as_ref().map(|file| file.path.as_str()),
        resolved_path.as_deref(),
    );
    let mut history_commits = github_fetch_commits(
        &repo_full_name,
        &content_reference,
        if history_scope_path.is_empty() {
            None
        } else {
            Some(&history_scope_path)
        },
        10,
    )
    .unwrap_or_default();
    if let Some(detail) = selected_commit_detail.as_ref() {
        if !history_commits
            .iter()
            .any(|entry| entry.sha.eq_ignore_ascii_case(&detail.sha))
        {
            history_commits.insert(
                0,
                GitHubCommitListItem {
                    sha: detail.sha.clone(),
                    html_url: detail.html_url.clone(),
                    commit: detail.commit.clone(),
                    parents: detail.parents.clone(),
                },
            );
        }
    }
    let history_commit_payloads = history_commits
        .into_iter()
        .take(10)
        .map(|entry| {
            github_history_commit_payload(
                &entry.sha,
                &entry.html_url,
                &entry.commit,
                &entry.parents,
            )
        })
        .collect::<Vec<_>>();
    let selected_commit_payload = selected_commit_detail
        .as_ref()
        .map(|detail| {
            github_history_commit_payload(
                &detail.sha,
                &detail.html_url,
                &detail.commit,
                &detail.parents,
            )
        })
        .unwrap_or_else(|| json!({}));
    let selected_diff = if let Some(detail) = selected_commit_detail.as_ref() {
        if let Some(file) = selected_file.as_ref() {
            github_commit_diff_payload(detail, &file.path)
        } else {
            github_commit_repo_diff_payload(detail)
        }
    } else {
        json!({})
    };
    let compare_payload = if let (Some(base_sha), Some(head_sha)) = (
        selected_compare_base_sha.as_deref(),
        selected_compare_head_sha.as_deref(),
    ) {
        if base_sha.eq_ignore_ascii_case(head_sha) {
            json!({
                "available": false,
                "detail": "Choose two different commits to compare.",
                "base_sha": base_sha,
                "head_sha": head_sha,
            })
        } else {
            let compare = github_fetch_compare(&repo_full_name, base_sha, head_sha)?;
            let base_commit_detail = github_fetch_commit_detail(&repo_full_name, base_sha).ok();
            let head_commit_detail = if selected_commit_detail
                .as_ref()
                .is_some_and(|detail| detail.sha.eq_ignore_ascii_case(head_sha))
            {
                selected_commit_detail.clone()
            } else {
                github_fetch_commit_detail(&repo_full_name, head_sha).ok()
            };
            github_compare_payload(
                &compare,
                base_sha,
                head_sha,
                base_commit_detail.as_ref(),
                head_commit_detail.as_ref(),
            )
        }
    } else {
        json!({})
    };
    Ok(json!({
        "operation": "search_github_preview",
        "selection_key": if let Some(path) = preview_path.as_deref() {
            format!("{}::{}", repo_full_name, path)
        } else {
            repo_full_name.clone()
        },
        "repository": {
            "full_name": repo.full_name,
            "description": repo.description.unwrap_or_default(),
            "language": repo.language,
            "stargazers_count": repo.stargazers_count,
            "topics": repo.topics,
            "default_branch": branch_name,
            "html_url": repo.html_url,
        },
        "path": resolved_path.clone().unwrap_or_default(),
        "active_ref": content_reference,
        "active_ref_kind": if selected_commit_sha.is_some() { "commit" } else { "branch" },
        "target_kind": preview_target_kind,
        "selected_file": selected_file
            .map(|file| {
                github_selected_file_payload(
                    file,
                    selected_commit_file_html,
                    selected_commit_file_raw,
                )
            })
            .unwrap_or_else(|| json!({})),
        "entries": tree
            .into_iter()
            .take(40)
            .map(|item| {
                json!({
                    "name": item.name,
                    "path": item.path,
                    "sha": item.sha,
                    "size": item.size,
                    "kind": item.item_type,
                    "html_url": item.html_url,
                    "download_url": item.download_url,
                })
            })
            .collect::<Vec<_>>(),
        "readme": readme,
        "history": {
            "scope_mode": history_scope_mode,
            "scope_path": history_scope_path,
            "selected_commit_sha": selected_commit_sha.unwrap_or_default(),
            "selected_commit": selected_commit_payload,
            "compare_base_sha": selected_compare_base_sha.unwrap_or_default(),
            "compare_head_sha": selected_compare_head_sha.unwrap_or_default(),
            "commits": history_commit_payloads,
            "diff": selected_diff,
            "compare": compare_payload,
        }
    }))
}

#[tool]
impl GitHubTools {
    /// Search GitHub repositories for codebases, benchmarks, or dataset-like repos relevant to the user request.
    pub fn search_github_repositories(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let query = query.trim().to_string();
        if query.is_empty() {
            return Err("search_github_repositories: query is required.".to_string());
        }
        github_search_repositories(&query, limit.unwrap_or(8))
    }

    /// Search GitHub code for implementation fragments, configs, scripts, or dataset manifests relevant to the user request.
    pub fn search_github_code(&self, query: String, limit: Option<usize>) -> Result<Value, String> {
        let query = query.trim().to_string();
        if query.is_empty() {
            return Err("search_github_code: query is required.".to_string());
        }
        github_search_code(&query, limit.unwrap_or(8))
    }

    /// Search GitHub for dataset-oriented repositories, benchmark suites, and corpus repos that can complement public dataset search.
    pub fn search_github_datasets(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let query = query.trim().to_string();
        if query.is_empty() {
            return Err("search_github_datasets: query is required.".to_string());
        }
        let dataset_query = format!("{} dataset OR benchmark OR corpus OR \"task suite\"", query);
        github_search_repositories(&dataset_query, limit.unwrap_or(8))
    }

    /// Preview a public GitHub repository subtree plus README content for IDE-side inspection.
    pub fn search_github_preview(
        &self,
        repo_full_name: String,
        branch: Option<String>,
        path: Option<String>,
        commit_sha: Option<String>,
        compare_base_sha: Option<String>,
        compare_head_sha: Option<String>,
        history_scope_mode: Option<String>,
    ) -> Result<Value, String> {
        github_preview(
            &repo_full_name,
            branch,
            path,
            commit_sha,
            compare_base_sha,
            compare_head_sha,
            history_scope_mode,
        )
    }

    /// Search the official ONNX Model Zoo manifest and return model artifacts that match the query.
    pub fn search_onnx_models(&self, query: String, limit: Option<usize>) -> Result<Value, String> {
        let query = query.trim().to_string();
        if query.is_empty() {
            return Err("search_onnx_models: query is required.".to_string());
        }
        onnx_model_search(limit.unwrap_or(8), &query)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        github_commit_repo_diff_payload, github_history_scope_path, parse_github_hunk_positions,
        parse_github_patch_hunks, GitHubCommitDetailFile, GitHubCommitDetailResponse,
    };

    #[test]
    fn github_patch_parser_tracks_line_numbers() {
        let patch = "@@ -10,2 +10,3 @@\n context line\n-old value\n+new value\n+extra value\n";
        let hunks = parse_github_patch_hunks(patch);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].header, "@@ -10,2 +10,3 @@");
        assert_eq!(hunks[0].lines.len(), 4);
        assert_eq!(hunks[0].lines[0].kind, "context");
        assert_eq!(hunks[0].lines[0].old_number, Some(10));
        assert_eq!(hunks[0].lines[0].new_number, Some(10));
        assert_eq!(hunks[0].lines[1].kind, "removed");
        assert_eq!(hunks[0].lines[1].old_number, Some(11));
        assert_eq!(hunks[0].lines[1].new_number, None);
        assert_eq!(hunks[0].lines[2].kind, "added");
        assert_eq!(hunks[0].lines[2].old_number, None);
        assert_eq!(hunks[0].lines[2].new_number, Some(11));
        assert_eq!(hunks[0].lines[3].kind, "added");
        assert_eq!(hunks[0].lines[3].new_number, Some(12));
    }

    #[test]
    fn github_patch_parser_reads_hunk_positions() {
        assert_eq!(parse_github_hunk_positions("@@ -3,7 +8,9 @@"), (3, 8));
    }

    #[test]
    fn github_repo_diff_payload_collects_multi_file_commit_changes() {
        let detail = GitHubCommitDetailResponse {
            files: vec![
                GitHubCommitDetailFile {
                    filename: "src/lib.rs".to_string(),
                    status: "modified".to_string(),
                    additions: 2,
                    deletions: 1,
                    changes: 3,
                    patch: Some("@@ -1,2 +1,3 @@\n line\n-old\n+new\n+extra\n".to_string()),
                    ..GitHubCommitDetailFile::default()
                },
                GitHubCommitDetailFile {
                    filename: "README.md".to_string(),
                    status: "modified".to_string(),
                    additions: 1,
                    deletions: 0,
                    changes: 1,
                    patch: Some("@@ -3,1 +3,2 @@\n context\n+note\n".to_string()),
                    ..GitHubCommitDetailFile::default()
                },
            ],
            ..GitHubCommitDetailResponse::default()
        };

        let payload = github_commit_repo_diff_payload(&detail);
        assert_eq!(payload["available"], serde_json::json!(true));
        assert_eq!(payload["scope"], serde_json::json!("repository"));
        assert_eq!(payload["file_count"], serde_json::json!(2));
        assert_eq!(payload["text_diff_file_count"], serde_json::json!(2));
        assert_eq!(
            payload["files"].as_array().map(|items| items.len()),
            Some(2)
        );
    }

    #[test]
    fn github_history_scope_path_can_force_repository_history() {
        let scope =
            github_history_scope_path(Some("repository"), Some("src/main.rs"), Some("src/main.rs"));
        assert!(scope.is_empty());
    }

    #[test]
    fn github_history_scope_path_defaults_to_selected_file_then_resolved_path() {
        let file_scope =
            github_history_scope_path(Some("selection"), Some("src/main.rs"), Some("src"));
        assert_eq!(file_scope, "src/main.rs");

        let directory_scope = github_history_scope_path(Some("selection"), None, Some("src"));
        assert_eq!(directory_scope, "src");
    }
}
