//! Data tools focused on real experiment setup rather than mock preprocessing.

use reqwest::blocking::Client;
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tokitai::tool;

pub struct DataTools;

pub(crate) const BENCHMARK_SCHEMA_VERSION: &str = "cs_benchmark_v1";
const DATASET_SOURCE_FILTERS: &[&str] = &[
    "huggingface.co",
    "openml.org",
    "datasetsearch.research.google.com",
    "pytorch.org/vision/stable/datasets.html",
    "paperswithcode.com",
    "kaggle.com",
];

#[derive(Debug, Clone, Serialize)]
struct DatasetDescriptor {
    dataset_id: String,
    provider: String,
    path: String,
    format: String,
    row_count_hint: Option<usize>,
    column_count_hint: Option<usize>,
    columns: Vec<String>,
    split_hint: Option<String>,
    task_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MetricDescriptor {
    name: String,
    direction: String,
    notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BaselineDescriptor {
    name: String,
    kind: String,
    source: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ArtifactDescriptor {
    name: String,
    kind: String,
    required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutionStageDescriptor {
    stage_id: String,
    title: String,
    purpose: String,
    required_outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ExecutionSchema {
    runner_kind: String,
    primary_entrypoint_kind: String,
    required_runtime_signals: Vec<String>,
    stages: Vec<ExecutionStageDescriptor>,
}

#[derive(Debug, Clone, Serialize)]
struct ResultBundleField {
    name: String,
    kind: String,
    required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DatasetManifestField {
    name: String,
    required: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DatasetAcquisitionPlan {
    retrieval_mode: String,
    retrieval_entrypoint: String,
    search_tool: String,
    manifest_tool: String,
    search_queries: Vec<String>,
    paper_dataset_hints: Vec<String>,
    preferred_providers: Vec<String>,
    expected_manifest_fields: Vec<DatasetManifestField>,
    selection_guidance: String,
    paper_source_policy: String,
}

#[derive(Debug, Clone, Serialize)]
struct ResultBundleSchema {
    bundle_kind: String,
    summary_fields: Vec<ResultBundleField>,
    required_artifact_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct LineageSchema {
    required: bool,
    compare_keys: Vec<String>,
    history_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ReproducibilityDescriptor {
    random_seed_required: bool,
    fixed_split_required: bool,
    environment_capture_required: bool,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkPlan {
    schema_version: &'static str,
    benchmark_profile: String,
    task: String,
    datasets: Vec<DatasetDescriptor>,
    dataset_acquisition: DatasetAcquisitionPlan,
    metrics: Vec<MetricDescriptor>,
    baselines: Vec<BaselineDescriptor>,
    artifacts: Vec<ArtifactDescriptor>,
    execution_schema: ExecutionSchema,
    result_bundle_schema: ResultBundleSchema,
    lineage_schema: LineageSchema,
    reproducibility: ReproducibilityDescriptor,
}

#[derive(Debug, Clone, Serialize)]
struct PublicDatasetRecord {
    dataset_id: String,
    title: String,
    url: String,
    provider: String,
    snippet: String,
    source_kind: String,
    official_source: bool,
    source_tier: String,
    format_hint: Option<String>,
    task_hint: Option<String>,
}

pub(crate) fn infer_benchmark_profile(problem_formulation: &str) -> &'static str {
    let lowered = problem_formulation.trim().to_ascii_lowercase();

    if lowered.is_empty() {
        return "general_cs";
    }

    if contains_any(
        &lowered,
        &[
            "deep learning",
            "neural",
            "transformer",
            "checkpoint",
            "fine-tun",
            "finetun",
            "epoch",
            "cnn",
            "lstm",
            "diffusion",
        ],
    ) {
        return "deep_learning";
    }

    if contains_any(
        &lowered,
        &[
            "security",
            "vulnerability",
            "vuln",
            "exploit",
            "fuzz",
            "fuzzer",
            "static analysis",
            "dynamic analysis",
            "cve",
            "hardening",
            "malware",
            "taint",
        ],
    ) {
        return "security_analysis";
    }

    if contains_any(
        &lowered,
        &[
            "classification",
            "classifier",
            "regression",
            "linear regression",
            "logistic regression",
            "random forest",
            "decision tree",
            "xgboost",
            "lightgbm",
            "svm",
            "cross validation",
            "cross-validation",
            "train test split",
            "train/test split",
            "accuracy",
            "f1",
            "precision",
            "recall",
            "tabular",
            "sklearn",
            "scikit-learn",
            "iris",
            "feature engineering",
        ],
    ) {
        return "classical_ml";
    }

    if contains_any(
        &lowered,
        &[
            "multi-agent",
            "agent evaluation",
            "agentic",
            "tool use",
            "tool-use",
            "planner",
            "planning agent",
            "autonomous agent",
            "assistant benchmark",
            "trajectory",
        ],
    ) {
        return "agent_evaluation";
    }

    if contains_any(
        &lowered,
        &[
            "latency",
            "throughput",
            "qps",
            "tail latency",
            "memory",
            "profiling",
            "benchmark",
            "overhead",
            "concurrency",
            "distributed",
            "service",
            "runtime",
            "systems",
            "system evaluation",
            "compiler",
            "database",
        ],
    ) {
        return "systems_evaluation";
    }

    "general_cs"
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn classify_dataset_provider(url: &str, provider: Option<&str>) -> String {
    let lowered_url = url.to_ascii_lowercase();
    let lowered_provider = provider.unwrap_or("").to_ascii_lowercase();
    if lowered_url.contains("huggingface.co") || lowered_provider.contains("hugging face") {
        "huggingface".to_string()
    } else if lowered_url.contains("paperswithcode.com")
        || lowered_provider.contains("papers with code")
    {
        "paperswithcode".to_string()
    } else if lowered_url.contains("openml.org") || lowered_provider.contains("openml") {
        "openml".to_string()
    } else if lowered_url.contains("datasetsearch.research.google.com")
        || lowered_provider.contains("google dataset search")
    {
        "google_dataset_search".to_string()
    } else if lowered_url.contains("pytorch.org/vision") || lowered_provider.contains("torchvision")
    {
        "torchvision_datasets".to_string()
    } else if lowered_url.contains("kaggle.com") || lowered_provider.contains("kaggle") {
        "kaggle".to_string()
    } else {
        "web".to_string()
    }
}

fn infer_format_hint_from_url(url: &str) -> Option<String> {
    let lowered = url.to_ascii_lowercase();
    if lowered.contains("/datasets/") || lowered.contains("dataset") {
        Some("dataset_hub".to_string())
    } else if lowered.ends_with(".csv") {
        Some("csv".to_string())
    } else if lowered.ends_with(".json") || lowered.ends_with(".jsonl") {
        Some("json".to_string())
    } else if lowered.ends_with(".parquet") {
        Some("parquet".to_string())
    } else {
        None
    }
}

fn infer_public_dataset_task_hint(title: &str, snippet: &str) -> Option<String> {
    let text = format!("{} {}", title, snippet).to_ascii_lowercase();
    if contains_any(
        &text,
        &[
            "image",
            "vision",
            "classification",
            "detection",
            "segmentation",
        ],
    ) {
        Some("vision_or_multimedia".to_string())
    } else if contains_any(
        &text,
        &[
            "text",
            "llm",
            "language",
            "qa",
            "question answering",
            "translation",
        ],
    ) {
        Some("nlp_or_text_processing".to_string())
    } else if contains_any(
        &text,
        &["tabular", "regression", "forecast", "structured data"],
    ) {
        Some("supervised_learning".to_string())
    } else if contains_any(&text, &["graph", "network"]) {
        Some("graph_or_network_analysis".to_string())
    } else if contains_any(
        &text,
        &["agent", "trajectory", "tool use", "benchmark suite"],
    ) {
        Some("agent_evaluation".to_string())
    } else {
        None
    }
}

fn is_public_dataset_provider_domain(url: &str) -> bool {
    let lowered = url.to_ascii_lowercase();
    lowered.contains("huggingface.co")
        || lowered.contains("openml.org")
        || lowered.contains("datasetsearch.research.google.com")
        || lowered.contains("pytorch.org/vision/stable/datasets")
        || lowered.contains("paperswithcode.com")
        || lowered.contains("kaggle.com")
}

fn fallback_dataset_title_from_url(url: &str) -> Option<String> {
    let lowered = url.to_ascii_lowercase();
    if lowered.contains("huggingface.co/datasets") {
        Some("Hugging Face Datasets".to_string())
    } else if lowered.contains("openml.org") {
        Some("OpenML Datasets".to_string())
    } else if lowered.contains("datasetsearch.research.google.com") {
        Some("Google Dataset Search".to_string())
    } else if lowered.contains("pytorch.org/vision/stable/datasets") {
        Some("torchvision Datasets".to_string())
    } else if lowered.contains("paperswithcode.com") {
        Some("Papers With Code Datasets".to_string())
    } else if lowered.contains("kaggle.com/datasets") {
        Some("Kaggle Datasets".to_string())
    } else {
        None
    }
}

fn recovery_dataset_url(url: &str, query: &str) -> String {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return url.trim().to_string();
    }

    let encoded = urlencoding::encode(trimmed_query);
    let lowered = url.to_ascii_lowercase();
    if lowered.contains("huggingface.co/datasets") {
        format!("https://huggingface.co/datasets?search={}", encoded)
    } else if lowered.contains("openml.org") {
        format!(
            "https://www.openml.org/search?type=data&sort=runs&id=0&status=active&q={}",
            encoded
        )
    } else if lowered.contains("datasetsearch.research.google.com") {
        format!(
            "https://datasetsearch.research.google.com/search?query={}",
            encoded
        )
    } else if lowered.contains("pytorch.org/vision/stable/datasets") {
        "https://pytorch.org/vision/stable/datasets.html".to_string()
    } else if lowered.contains("paperswithcode.com") {
        format!("https://paperswithcode.com/datasets?q={}", encoded)
    } else if lowered.contains("kaggle.com/datasets") {
        format!("https://www.kaggle.com/datasets?search={}", encoded)
    } else {
        url.trim().to_string()
    }
}

fn public_dataset_record_from_recovery_candidate(
    candidate: &Value,
    query: &str,
) -> Option<PublicDatasetRecord> {
    let url = candidate
        .get("url")
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    if url.is_empty() || !is_public_dataset_provider_domain(&url) {
        return None;
    }

    let title = fallback_dataset_title_from_url(&url)?;
    let provider = classify_dataset_provider(&url, None);
    let resolved_url = recovery_dataset_url(&url, query);
    Some(PublicDatasetRecord {
        dataset_id: slugify_dataset_title(&title, &provider),
        title,
        url: resolved_url.clone(),
        provider,
        snippet: format!(
            "Direct provider discovery fallback for dataset query '{}'; opening the provider search results page.",
            query.trim()
        ),
        source_kind: "direct_provider_search_fallback".to_string(),
        official_source: true,
        source_tier: "provider_search_fallback".to_string(),
        format_hint: infer_format_hint_from_url(&resolved_url).or(Some("dataset_hub".to_string())),
        task_hint: infer_public_dataset_task_hint(query, &resolved_url),
    })
}

fn build_huggingface_dataset_record(entry: &Value) -> Option<PublicDatasetRecord> {
    let dataset_id = entry.get("id").and_then(Value::as_str)?.trim().to_string();
    if dataset_id.is_empty() {
        return None;
    }
    let title = dataset_id.clone();
    let url = format!("https://huggingface.co/datasets/{}", dataset_id);
    let description = entry
        .get("description")
        .and_then(Value::as_str)
        .map(collapse_dataset_snippet)
        .unwrap_or_else(|| "Official Hugging Face dataset page.".to_string());
    Some(PublicDatasetRecord {
        dataset_id: slugify_dataset_title(&dataset_id, "huggingface"),
        title,
        url: url.clone(),
        provider: "huggingface".to_string(),
        snippet: description,
        source_kind: "official_provider_dataset_page".to_string(),
        official_source: true,
        source_tier: "official_provider_page".to_string(),
        format_hint: infer_format_hint_from_url(&url).or(Some("dataset_hub".to_string())),
        task_hint: infer_public_dataset_task_hint(&dataset_id, &url),
    })
}

fn build_openml_dataset_record(entry: &Value) -> Option<PublicDatasetRecord> {
    let did = entry.get("did").and_then(Value::as_i64)?;
    let name = entry
        .get("name")
        .and_then(Value::as_str)?
        .trim()
        .to_string();
    if name.is_empty() {
        return None;
    }
    let version = entry.get("version").and_then(Value::as_i64);
    let format_name = entry
        .get("format")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("ARFF");
    let instance_count = entry
        .get("quality")
        .and_then(Value::as_array)
        .and_then(|items| {
            items.iter().find_map(|item| {
                let key = item.get("name").and_then(Value::as_str)?;
                if key == "NumberOfInstances" {
                    item.get("value")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                } else {
                    None
                }
            })
        });
    let url = format!("https://www.openml.org/search?type=data&id={}", did);
    let version_suffix = version
        .map(|value| format!(" v{}", value))
        .unwrap_or_default();
    let count_suffix = instance_count
        .map(|value| format!(" / {} rows", value))
        .unwrap_or_default();
    Some(PublicDatasetRecord {
        dataset_id: format!("openml-{}", did),
        title: format!("{}{}", name, version_suffix),
        url: url.clone(),
        provider: "openml".to_string(),
        snippet: format!(
            "Official OpenML dataset page / did={} / format={}{}",
            did, format_name, count_suffix
        ),
        source_kind: "official_provider_dataset_page".to_string(),
        official_source: true,
        source_tier: "official_provider_page".to_string(),
        format_hint: Some(format_name.to_ascii_lowercase()),
        task_hint: infer_public_dataset_task_hint(&name, &url),
    })
}

fn collapse_dataset_snippet(value: &str) -> String {
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

fn extract_dataset_name_candidates(query: &str) -> Vec<String> {
    let lowered = query.trim().to_ascii_lowercase();
    if lowered.is_empty() {
        return Vec::new();
    }

    let stop_words = [
        "dataset",
        "datasets",
        "classification",
        "regression",
        "benchmark",
        "baseline",
        "public",
        "open",
        "small",
        "machine",
        "learning",
        "with",
        "for",
        "and",
        "tabular",
        "image",
        "text",
        "vision",
        "nlp",
        "sklearn",
        "suite",
        "task",
    ];

    let mut candidates = Vec::new();
    for token in lowered
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_')
        .filter(|part| part.len() >= 3)
    {
        if stop_words.contains(&token) {
            continue;
        }
        if !candidates.iter().any(|item| item == token) {
            candidates.push(token.to_string());
        }
    }
    candidates.truncate(4);
    candidates
}

fn search_huggingface_datasets(
    query: &str,
    limit: usize,
) -> Result<Vec<PublicDatasetRecord>, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| {
            format!(
                "search_public_datasets: failed to build Hugging Face client: {}",
                err
            )
        })?;
    let response = client
        .get("https://huggingface.co/api/datasets")
        .query(&[
            ("search", query.trim()),
            ("limit", &limit.min(5).to_string()),
        ])
        .send()
        .map_err(|err| {
            format!(
                "search_public_datasets: failed to reach Hugging Face API: {}",
                err
            )
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "search_public_datasets: Hugging Face API returned HTTP {}",
            response.status()
        ));
    }
    let payload = response.json::<Value>().map_err(|err| {
        format!(
            "search_public_datasets: invalid Hugging Face response JSON: {}",
            err
        )
    })?;
    Ok(payload
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(build_huggingface_dataset_record)
                .take(limit)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default())
}

fn search_openml_datasets(query: &str, limit: usize) -> Result<Vec<PublicDatasetRecord>, String> {
    let candidates = extract_dataset_name_candidates(query);
    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|err| {
            format!(
                "search_public_datasets: failed to build OpenML client: {}",
                err
            )
        })?;

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for candidate in candidates {
        let url = format!(
            "https://www.openml.org/api/v1/json/data/list/data_name/{}/limit/{}",
            urlencoding::encode(&candidate),
            limit.min(5)
        );
        let response = client.get(&url).send().map_err(|err| {
            format!(
                "search_public_datasets: failed to reach OpenML API: {}",
                err
            )
        })?;

        if response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::BAD_REQUEST
        {
            continue;
        }
        if !response.status().is_success() {
            return Err(format!(
                "search_public_datasets: OpenML API returned HTTP {}",
                response.status()
            ));
        }
        let payload = response.json::<Value>().map_err(|err| {
            format!(
                "search_public_datasets: invalid OpenML response JSON: {}",
                err
            )
        })?;
        if let Some(items) = payload["data"]["dataset"].as_array() {
            for item in items {
                if let Some(record) = build_openml_dataset_record(item) {
                    if seen.insert(record.url.clone()) {
                        out.push(record);
                    }
                }
                if out.len() >= limit {
                    return Ok(out);
                }
            }
        }
    }
    Ok(out)
}

fn search_google_dataset_search(query: &str, limit: usize) -> Vec<PublicDatasetRecord> {
    let resolved_url = recovery_dataset_url("https://datasetsearch.research.google.com", query);
    vec![PublicDatasetRecord {
        dataset_id: slugify_dataset_title("Google Dataset Search", "google_dataset_search"),
        title: "Google Dataset Search".to_string(),
        url: resolved_url,
        provider: "google_dataset_search".to_string(),
        snippet: format!(
            "Official Google Dataset Search directory entry for dataset discovery related to '{}'.",
            query.trim()
        ),
        source_kind: "official_dataset_directory".to_string(),
        official_source: true,
        source_tier: "official_provider_page".to_string(),
        format_hint: Some("dataset_directory".to_string()),
        task_hint: infer_public_dataset_task_hint(query, "google dataset search"),
    }]
    .into_iter()
    .take(limit.min(1))
    .collect()
}

fn search_torchvision_datasets(query: &str, limit: usize) -> Vec<PublicDatasetRecord> {
    let lower = query.to_ascii_lowercase();
    if !contains_any(
        &lower,
        &[
            "image",
            "vision",
            "cnn",
            "resnet",
            "classification",
            "detection",
            "segmentation",
            "torchvision",
        ],
    ) {
        return Vec::new();
    }
    vec![PublicDatasetRecord {
        dataset_id: slugify_dataset_title("torchvision Datasets", "torchvision_datasets"),
        title: "torchvision Datasets".to_string(),
        url: "https://pytorch.org/vision/stable/datasets.html".to_string(),
        provider: "torchvision_datasets".to_string(),
        snippet: "Official torchvision dataset registry for image and vision benchmarks."
            .to_string(),
        source_kind: "official_provider_dataset_page".to_string(),
        official_source: true,
        source_tier: "official_provider_page".to_string(),
        format_hint: Some("dataset_registry".to_string()),
        task_hint: Some("vision_or_multimedia".to_string()),
    }]
    .into_iter()
    .take(limit.min(1))
    .collect()
}

fn merge_public_dataset_records(
    primary: Vec<PublicDatasetRecord>,
    secondary: Vec<PublicDatasetRecord>,
    limit: usize,
) -> Vec<PublicDatasetRecord> {
    let mut merged = Vec::new();
    let mut seen = HashSet::new();
    for record in primary.into_iter().chain(secondary.into_iter()) {
        if seen.insert(record.url.clone()) {
            merged.push(record);
        }
        if merged.len() >= limit {
            break;
        }
    }
    merged
}

fn slugify_dataset_title(title: &str, provider: &str) -> String {
    let slug = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        format!("{}-dataset", provider)
    } else {
        format!("{}-{}", provider, slug)
    }
}

fn build_default_dataset_placeholder(profile: &str) -> DatasetDescriptor {
    let (dataset_id, split_hint, task_hint) = match profile {
        "deep_learning" => (
            "training_corpus_or_dataset",
            Some("train/validation/test split with checkpoint-compatible sampling".to_string()),
            Some("representation_learning_or_high_capacity_prediction".to_string()),
        ),
        "systems_evaluation" => (
            "workload_trace_or_benchmark_suite",
            Some("calibration workload plus held-out benchmark scenarios".to_string()),
            Some("systems_or_runtime_evaluation".to_string()),
        ),
        "agent_evaluation" => (
            "task_suite_or_judge_set",
            Some("development tasks plus held-out evaluation trajectories".to_string()),
            Some("agent_capability_evaluation".to_string()),
        ),
        "security_analysis" => (
            "target_corpus_or_vulnerability_suite",
            Some("labeled findings or benign/adversarial split".to_string()),
            Some("security_detection_or_analysis".to_string()),
        ),
        "classical_ml" => (
            "tabular_or_labeled_dataset",
            Some("fixed train/validation/test or cross-validation protocol".to_string()),
            Some("supervised_learning".to_string()),
        ),
        _ => (
            "dataset_to_be_selected",
            Some("train/validation/test or benchmark corpus split".to_string()),
            None,
        ),
    };

    DatasetDescriptor {
        dataset_id: dataset_id.to_string(),
        provider: "local_or_configured".to_string(),
        path: "".to_string(),
        format: "unknown".to_string(),
        row_count_hint: None,
        column_count_hint: None,
        columns: Vec::new(),
        split_hint,
        task_hint,
    }
}

fn dataset_search_queries_for_profile(profile: &str, task: &str) -> Vec<String> {
    let concise_task = task.trim();
    match profile {
        "classical_ml" => vec![
            format!("{concise_task} tabular classification dataset"),
            "openml classification benchmark".to_string(),
            "huggingface tabular dataset".to_string(),
        ],
        "deep_learning" => vec![
            format!("{concise_task} training dataset"),
            "huggingface deep learning dataset".to_string(),
            "papers with code dataset benchmark".to_string(),
        ],
        "systems_evaluation" => vec![
            format!("{concise_task} benchmark suite"),
            "systems workload trace dataset".to_string(),
            "benchmark trace dataset".to_string(),
        ],
        "agent_evaluation" => vec![
            format!("{concise_task} agent benchmark task suite"),
            "tool use benchmark dataset".to_string(),
            "trajectory evaluation task suite".to_string(),
        ],
        "security_analysis" => vec![
            format!("{concise_task} vulnerability benchmark dataset"),
            "security benchmark suite".to_string(),
            "fuzzing corpus dataset".to_string(),
        ],
        "literature_review" => vec!["paper corpus handled by official paper APIs".to_string()],
        "theory" => vec!["formal problem instances or counterexample corpus".to_string()],
        _ => vec![
            format!("{concise_task} public dataset"),
            "public benchmark dataset".to_string(),
        ],
    }
}

fn normalize_dataset_hint(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim()
        .trim_matches(|ch: char| matches!(ch, '"' | '\'' | '`'));
    if trimmed.len() < 2 || trimmed.len() > 80 {
        return None;
    }
    let lowered = trimmed.to_ascii_lowercase();
    let banned = [
        "dataset",
        "datasets",
        "benchmark",
        "benchmarks",
        "training set",
        "test set",
        "validation set",
        "public dataset",
        "official api",
        "paper",
        "papers",
    ];
    if banned.contains(&lowered.as_str()) {
        return None;
    }
    if !trimmed.chars().any(|ch| ch.is_ascii_alphanumeric()) {
        return None;
    }
    Some(trimmed.to_string())
}

fn push_dataset_hint(out: &mut Vec<String>, raw: &str, limit: usize) {
    if out.len() >= limit {
        return;
    }
    let Some(candidate) = normalize_dataset_hint(raw) else {
        return;
    };
    if out.iter().any(|item| item.eq_ignore_ascii_case(&candidate)) {
        return;
    }
    out.push(candidate);
}

fn extract_paper_dataset_hints_from_text(text: &str, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit {
        return;
    }

    let patterns = [
        "dataset:",
        "datasets:",
        "dataset used:",
        "datasets used:",
        "benchmark dataset:",
        "benchmarks:",
        "evaluated on",
        "trained on",
        "test on",
        "experiment on",
    ];
    let separators = [',', ';', '/', '|'];

    for line in text.lines() {
        if out.len() >= limit {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lowered = trimmed.to_ascii_lowercase();
        for pattern in patterns {
            if let Some(index) = lowered.find(pattern) {
                let segment = trimmed[index + pattern.len()..].trim();
                if segment.is_empty() {
                    continue;
                }
                let compact = segment
                    .trim_matches(|ch: char| matches!(ch, '.' | ':' | '-' | '(' | ')' | '[' | ']'))
                    .trim();
                if compact.is_empty() {
                    continue;
                }
                let mut sliced = compact
                    .split(|ch| separators.contains(&ch))
                    .map(str::trim)
                    .filter(|part| !part.is_empty())
                    .collect::<Vec<_>>();
                if sliced.is_empty() {
                    sliced.push(compact);
                }
                for item in sliced {
                    push_dataset_hint(out, item, limit);
                    if out.len() >= limit {
                        break;
                    }
                }
            }
        }
    }
}

fn collect_string_values(value: &Value, out: &mut Vec<String>, limit: usize) {
    if out.len() >= limit {
        return;
    }
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.push(trimmed.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                if out.len() >= limit {
                    break;
                }
                collect_string_values(item, out, limit);
            }
        }
        Value::Object(map) => {
            for child in map.values() {
                if out.len() >= limit {
                    break;
                }
                collect_string_values(child, out, limit);
            }
        }
        _ => {}
    }
}

pub(crate) fn extract_paper_dataset_hints_from_value(value: &Value) -> Vec<String> {
    let mut hints = Vec::new();

    for pointer in [
        "/paper/title",
        "/paper/abstract_text",
        "/paper/snippet",
        "/summary",
        "/fulltext/body_text",
        "/structured_document/body_text",
    ] {
        if let Some(text) = value.pointer(pointer).and_then(Value::as_str) {
            extract_paper_dataset_hints_from_text(text, &mut hints, 8);
        }
    }

    for pointer in [
        "/paper/datasets",
        "/structured_document/datasets",
        "/structured_document/sections",
        "/results",
    ] {
        if let Some(node) = value.pointer(pointer) {
            let mut strings = Vec::new();
            collect_string_values(node, &mut strings, 48);
            for text in strings {
                extract_paper_dataset_hints_from_text(&text, &mut hints, 8);
                if out_len_reached(&hints, 8) {
                    break;
                }
                push_dataset_hint(&mut hints, &text, 8);
                if out_len_reached(&hints, 8) {
                    break;
                }
            }
        }
    }

    hints
}

fn out_len_reached(items: &[String], limit: usize) -> bool {
    items.len() >= limit
}

fn merge_dataset_search_queries(
    base_queries: Vec<String>,
    paper_dataset_hints: &[String],
) -> Vec<String> {
    let mut queries = Vec::new();

    for hint in paper_dataset_hints.iter().take(4) {
        let trimmed = hint.trim();
        if trimmed.is_empty() {
            continue;
        }
        queries.push(format!("{trimmed} dataset"));
        queries.push(format!("{trimmed} official dataset"));
    }

    for query in base_queries {
        if !queries.iter().any(|item| item.eq_ignore_ascii_case(&query)) {
            queries.push(query);
        }
    }

    queries.truncate(8);
    queries
}

fn preferred_dataset_providers_for_profile(profile: &str) -> Vec<String> {
    match profile {
        "classical_ml" => vec![
            "openml".to_string(),
            "huggingface".to_string(),
            "paperswithcode".to_string(),
        ],
        "deep_learning" => vec![
            "huggingface".to_string(),
            "paperswithcode".to_string(),
            "kaggle".to_string(),
        ],
        "systems_evaluation" => vec![
            "huggingface".to_string(),
            "paperswithcode".to_string(),
            "openml".to_string(),
        ],
        "agent_evaluation" => vec!["huggingface".to_string(), "paperswithcode".to_string()],
        "security_analysis" => vec![
            "huggingface".to_string(),
            "paperswithcode".to_string(),
            "kaggle".to_string(),
        ],
        _ => vec![
            "huggingface".to_string(),
            "openml".to_string(),
            "paperswithcode".to_string(),
        ],
    }
}

fn build_dataset_acquisition_plan(
    profile: &str,
    task: &str,
    paper_dataset_hints: &[String],
) -> DatasetAcquisitionPlan {
    let expected_manifest_fields = vec![
        DatasetManifestField {
            name: "dataset_id".to_string(),
            required: true,
        },
        DatasetManifestField {
            name: "title".to_string(),
            required: true,
        },
        DatasetManifestField {
            name: "provider".to_string(),
            required: true,
        },
        DatasetManifestField {
            name: "path".to_string(),
            required: true,
        },
        DatasetManifestField {
            name: "format".to_string(),
            required: true,
        },
        DatasetManifestField {
            name: "task_hint".to_string(),
            required: false,
        },
    ];

    match profile {
        "theory" => DatasetAcquisitionPlan {
            retrieval_mode: "formal_problem_instances".to_string(),
            retrieval_entrypoint: "local_or_constructed".to_string(),
            search_tool: "search_public_datasets".to_string(),
            manifest_tool: "fetch_public_dataset_manifest".to_string(),
            search_queries: merge_dataset_search_queries(
                dataset_search_queries_for_profile(profile, task),
                paper_dataset_hints,
            ),
            paper_dataset_hints: paper_dataset_hints.to_vec(),
            preferred_providers: Vec::new(),
            expected_manifest_fields,
            selection_guidance: "Prefer formal problem instances, counterexample corpora, or machine-checkable specifications over generic public datasets.".to_string(),
            paper_source_policy: "official_paper_apis_only".to_string(),
        },
        "literature_review" => DatasetAcquisitionPlan {
            retrieval_mode: "paper_corpus".to_string(),
            retrieval_entrypoint: "official_paper_apis_only".to_string(),
            search_tool: "direct_provider_dataset_search".to_string(),
            manifest_tool: "fetch_direct_dataset_manifest".to_string(),
            search_queries: merge_dataset_search_queries(
                dataset_search_queries_for_profile(profile, task),
                paper_dataset_hints,
            ),
            paper_dataset_hints: paper_dataset_hints.to_vec(),
            preferred_providers: Vec::new(),
            expected_manifest_fields,
            selection_guidance: "Use official paper APIs for literature retrieval. Public dataset entrypoints are secondary and only relevant when a benchmark corpus is explicitly required.".to_string(),
            paper_source_policy: "official_paper_apis_only".to_string(),
        },
        _ => DatasetAcquisitionPlan {
            retrieval_mode: "direct_provider_database_search".to_string(),
            retrieval_entrypoint: "official_dataset_databases".to_string(),
            search_tool: "search_public_datasets".to_string(),
            manifest_tool: "fetch_public_dataset_manifest".to_string(),
            search_queries: merge_dataset_search_queries(
                dataset_search_queries_for_profile(profile, task),
                paper_dataset_hints,
            ),
            paper_dataset_hints: paper_dataset_hints.to_vec(),
            preferred_providers: preferred_dataset_providers_for_profile(profile),
            expected_manifest_fields,
            selection_guidance: if paper_dataset_hints.is_empty() {
                "Start from direct provider or database discovery, then materialize a dataset manifest before fixing splits, baselines, and runnable evaluation artifacts.".to_string()
            } else {
                "Prioritize dataset names recovered from official paper APIs, then use direct provider or database discovery to resolve official dataset pages and materialize a fixed dataset manifest before running evaluation.".to_string()
            },
            paper_source_policy: "official_paper_apis_only".to_string(),
        },
    }
}

fn build_metrics_for_profile(profile: &str) -> Vec<MetricDescriptor> {
    match profile {
        "deep_learning" => vec![
            MetricDescriptor {
                name: "validation_score".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Use task-specific validation quality such as accuracy, F1, BLEU, or loss reduction.".to_string()),
            },
            MetricDescriptor {
                name: "training_time_minutes".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Track wall-clock training time to compare efficiency.".to_string()),
            },
            MetricDescriptor {
                name: "gpu_or_memory_footprint".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Capture peak accelerator or RAM usage during training/inference.".to_string()),
            },
        ],
        "systems_evaluation" => vec![
            MetricDescriptor {
                name: "latency_ms".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Measure median and tail latency under representative load.".to_string()),
            },
            MetricDescriptor {
                name: "throughput_ops_per_sec".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Track steady-state throughput or request handling capacity.".to_string()),
            },
            MetricDescriptor {
                name: "memory_mb".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Record peak memory or resident set size.".to_string()),
            },
        ],
        "agent_evaluation" => vec![
            MetricDescriptor {
                name: "task_success_rate".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Measure end-to-end completion on the held-out task suite.".to_string()),
            },
            MetricDescriptor {
                name: "trajectory_cost".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Track tokens, steps, or API/tool cost per solved task.".to_string()),
            },
            MetricDescriptor {
                name: "tool_error_rate".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Count failed tool calls, retries, or invalid action sequences.".to_string()),
            },
        ],
        "security_analysis" => vec![
            MetricDescriptor {
                name: "precision".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Prioritize actionable findings with low false-positive burden.".to_string()),
            },
            MetricDescriptor {
                name: "recall".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Measure coverage over known vulnerabilities or seeded cases.".to_string()),
            },
            MetricDescriptor {
                name: "false_positive_rate".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Keep analyst review effort manageable.".to_string()),
            },
        ],
        "classical_ml" => vec![
            MetricDescriptor {
                name: "accuracy".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Use when the task is balanced classification.".to_string()),
            },
            MetricDescriptor {
                name: "f1".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Useful when class balance or per-class behavior matters.".to_string()),
            },
            MetricDescriptor {
                name: "fit_time_seconds".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Track training cost for quick iteration.".to_string()),
            },
        ],
        _ => vec![
            MetricDescriptor {
                name: "accuracy".to_string(),
                direction: "maximize".to_string(),
                notes: Some("Use when the task is balanced classification.".to_string()),
            },
            MetricDescriptor {
                name: "latency_ms".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Measure end-to-end inference or execution latency.".to_string()),
            },
            MetricDescriptor {
                name: "memory_mb".to_string(),
                direction: "minimize".to_string(),
                notes: Some("Track peak memory or model footprint.".to_string()),
            },
        ],
    }
}

fn build_baselines_for_profile(profile: &str) -> Vec<BaselineDescriptor> {
    match profile {
        "deep_learning" => vec![
            BaselineDescriptor {
                name: "lightweight_reference_model".to_string(),
                kind: "reproducible_baseline".to_string(),
                source: None,
            },
            BaselineDescriptor {
                name: "documented_previous_run".to_string(),
                kind: "prior_work_or_existing_checkpoint".to_string(),
                source: None,
            },
        ],
        "systems_evaluation" => vec![
            BaselineDescriptor {
                name: "current_system_baseline".to_string(),
                kind: "existing_system".to_string(),
                source: None,
            },
            BaselineDescriptor {
                name: "instrumented_reference_configuration".to_string(),
                kind: "sanity_check".to_string(),
                source: None,
            },
        ],
        "agent_evaluation" => vec![
            BaselineDescriptor {
                name: "single_pass_agent".to_string(),
                kind: "behavioral_baseline".to_string(),
                source: None,
            },
            BaselineDescriptor {
                name: "documented_reference_prompt".to_string(),
                kind: "prompt_or_policy_baseline".to_string(),
                source: None,
            },
        ],
        "security_analysis" => vec![
            BaselineDescriptor {
                name: "documented_rule_based_baseline".to_string(),
                kind: "static_or_manual_baseline".to_string(),
                source: None,
            },
            BaselineDescriptor {
                name: "known_safe_control_sample".to_string(),
                kind: "sanity_check".to_string(),
                source: None,
            },
        ],
        "classical_ml" => vec![
            BaselineDescriptor {
                name: "majority_class_baseline".to_string(),
                kind: "sanity_check".to_string(),
                source: None,
            },
            BaselineDescriptor {
                name: "regularized_linear_model".to_string(),
                kind: "reproducible_baseline".to_string(),
                source: None,
            },
        ],
        _ => vec![
            BaselineDescriptor {
                name: "documented_reference_baseline".to_string(),
                kind: "prior_work_or_existing_system".to_string(),
                source: None,
            },
            BaselineDescriptor {
                name: "simple_reproducible_baseline".to_string(),
                kind: "sanity_check".to_string(),
                source: None,
            },
        ],
    }
}

fn build_artifacts_for_profile(profile: &str) -> Vec<ArtifactDescriptor> {
    match profile {
        "deep_learning" => vec![
            ArtifactDescriptor {
                name: "dataset_split".to_string(),
                kind: "data_manifest".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "training_script".to_string(),
                kind: "executable".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "evaluation_report".to_string(),
                kind: "report".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "training_curve_or_checkpoint_log".to_string(),
                kind: "figure".to_string(),
                required: false,
            },
        ],
        "systems_evaluation" => vec![
            ArtifactDescriptor {
                name: "benchmark_configuration".to_string(),
                kind: "data_manifest".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "benchmark_runner".to_string(),
                kind: "executable".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "performance_report".to_string(),
                kind: "report".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "profiling_output".to_string(),
                kind: "data_manifest".to_string(),
                required: false,
            },
        ],
        "agent_evaluation" => vec![
            ArtifactDescriptor {
                name: "task_suite_manifest".to_string(),
                kind: "data_manifest".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "evaluation_orchestration_script".to_string(),
                kind: "executable".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "trajectory_or_metrics_report".to_string(),
                kind: "report".to_string(),
                required: true,
            },
        ],
        "security_analysis" => vec![
            ArtifactDescriptor {
                name: "target_manifest".to_string(),
                kind: "data_manifest".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "analysis_or_detection_script".to_string(),
                kind: "executable".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "findings_report".to_string(),
                kind: "report".to_string(),
                required: true,
            },
        ],
        "classical_ml" => vec![
            ArtifactDescriptor {
                name: "dataset_split".to_string(),
                kind: "data_manifest".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "train_or_eval_script".to_string(),
                kind: "executable".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "metrics_report".to_string(),
                kind: "report".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "confusion_matrix_or_summary_figure".to_string(),
                kind: "figure".to_string(),
                required: false,
            },
        ],
        _ => vec![
            ArtifactDescriptor {
                name: "dataset_split".to_string(),
                kind: "data_manifest".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "train_or_eval_script".to_string(),
                kind: "executable".to_string(),
                required: true,
            },
            ArtifactDescriptor {
                name: "metrics_report".to_string(),
                kind: "report".to_string(),
                required: true,
            },
        ],
    }
}

fn build_execution_schema_for_profile(profile: &str) -> ExecutionSchema {
    match profile {
        "deep_learning" => ExecutionSchema {
            runner_kind: "training_pipeline".to_string(),
            primary_entrypoint_kind: "training_script".to_string(),
            required_runtime_signals: vec![
                "training_log".to_string(),
                "validation_metrics".to_string(),
                "checkpoint_written".to_string(),
                "resource_usage".to_string(),
            ],
            stages: vec![
                ExecutionStageDescriptor {
                    stage_id: "prepare".to_string(),
                    title: "Prepare training inputs".to_string(),
                    purpose: "Resolve dataset split, config, and seed before long-running training.".to_string(),
                    required_outputs: vec!["dataset_split".to_string(), "config_snapshot".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "train".to_string(),
                    title: "Run training".to_string(),
                    purpose: "Execute training with checkpointing and monitored validation.".to_string(),
                    required_outputs: vec!["training_log".to_string(), "checkpoint".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "evaluate".to_string(),
                    title: "Evaluate checkpoint".to_string(),
                    purpose: "Report validation/test metrics plus resource summary.".to_string(),
                    required_outputs: vec!["evaluation_report".to_string(), "validation_metrics".to_string()],
                },
            ],
        },
        "systems_evaluation" => ExecutionSchema {
            runner_kind: "benchmark_harness".to_string(),
            primary_entrypoint_kind: "benchmark_runner".to_string(),
            required_runtime_signals: vec![
                "benchmark_log".to_string(),
                "latency_summary".to_string(),
                "throughput_summary".to_string(),
                "profiling_capture".to_string(),
            ],
            stages: vec![
                ExecutionStageDescriptor {
                    stage_id: "configure".to_string(),
                    title: "Configure workload".to_string(),
                    purpose: "Fix workload shape, system flags, and observation settings.".to_string(),
                    required_outputs: vec!["benchmark_configuration".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "run".to_string(),
                    title: "Run benchmark".to_string(),
                    purpose: "Collect latency, throughput, and resource evidence under representative load.".to_string(),
                    required_outputs: vec!["benchmark_log".to_string(), "performance_report".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "analyze".to_string(),
                    title: "Analyze bottlenecks".to_string(),
                    purpose: "Link profiler evidence to observed bottlenecks and trade-offs.".to_string(),
                    required_outputs: vec!["profiling_output".to_string(), "bottleneck_summary".to_string()],
                },
            ],
        },
        "agent_evaluation" => ExecutionSchema {
            runner_kind: "evaluation_orchestrator".to_string(),
            primary_entrypoint_kind: "evaluation_script".to_string(),
            required_runtime_signals: vec![
                "task_level_outcomes".to_string(),
                "trajectory_capture".to_string(),
                "tool_error_log".to_string(),
                "judge_summary".to_string(),
            ],
            stages: vec![
                ExecutionStageDescriptor {
                    stage_id: "suite".to_string(),
                    title: "Materialize task suite".to_string(),
                    purpose: "Pin the task suite, judge criteria, and tool boundary before evaluation.".to_string(),
                    required_outputs: vec!["task_suite_manifest".to_string(), "judge_spec".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "evaluate".to_string(),
                    title: "Run agent evaluation".to_string(),
                    purpose: "Capture task success, trajectories, and tool errors across the suite.".to_string(),
                    required_outputs: vec!["trajectory_bundle".to_string(), "metrics_report".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "retest".to_string(),
                    title: "Compare repair deltas".to_string(),
                    purpose: "Re-run targeted failures and report before/after deltas.".to_string(),
                    required_outputs: vec!["repair_delta_summary".to_string()],
                },
            ],
        },
        "security_analysis" => ExecutionSchema {
            runner_kind: "analysis_pipeline".to_string(),
            primary_entrypoint_kind: "analysis_script".to_string(),
            required_runtime_signals: vec![
                "target_inventory".to_string(),
                "finding_log".to_string(),
                "false_positive_triage".to_string(),
                "remediation_notes".to_string(),
            ],
            stages: vec![
                ExecutionStageDescriptor {
                    stage_id: "scope".to_string(),
                    title: "Scope targets".to_string(),
                    purpose: "Fix the target set, analysis mode, and threat assumptions.".to_string(),
                    required_outputs: vec!["target_manifest".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "scan".to_string(),
                    title: "Run analysis".to_string(),
                    purpose: "Execute the security analysis and preserve raw findings.".to_string(),
                    required_outputs: vec!["finding_log".to_string(), "findings_report".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "triage".to_string(),
                    title: "Triage findings".to_string(),
                    purpose: "Separate real findings from false positives and record remediation guidance.".to_string(),
                    required_outputs: vec!["triage_report".to_string(), "remediation_notes".to_string()],
                },
            ],
        },
        "classical_ml" => ExecutionSchema {
            runner_kind: "training_pipeline".to_string(),
            primary_entrypoint_kind: "train_eval_script".to_string(),
            required_runtime_signals: vec![
                "dataset_split".to_string(),
                "metrics_report".to_string(),
                "baseline_comparison".to_string(),
                "error_analysis".to_string(),
            ],
            stages: vec![
                ExecutionStageDescriptor {
                    stage_id: "prepare".to_string(),
                    title: "Prepare data split".to_string(),
                    purpose: "Pin the dataset split or cross-validation protocol.".to_string(),
                    required_outputs: vec!["dataset_split".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "train_eval".to_string(),
                    title: "Train and evaluate".to_string(),
                    purpose: "Run the baseline and capture concise metrics.".to_string(),
                    required_outputs: vec!["metrics_report".to_string(), "baseline_comparison".to_string()],
                },
                ExecutionStageDescriptor {
                    stage_id: "analyze".to_string(),
                    title: "Review errors".to_string(),
                    purpose: "Capture confusion patterns or failure slices for the next iteration.".to_string(),
                    required_outputs: vec!["error_analysis".to_string()],
                },
            ],
        },
        _ => ExecutionSchema {
            runner_kind: "cs_experiment_runner".to_string(),
            primary_entrypoint_kind: "script_or_notebook".to_string(),
            required_runtime_signals: vec![
                "runtime_log".to_string(),
                "metrics_report".to_string(),
                "artifact_write".to_string(),
            ],
            stages: vec![ExecutionStageDescriptor {
                stage_id: "run".to_string(),
                title: "Run experiment".to_string(),
                purpose: "Execute the main CS workflow and capture reproducible outputs.".to_string(),
                required_outputs: vec!["metrics_report".to_string()],
            }],
        },
    }
}

fn build_result_bundle_schema_for_profile(profile: &str) -> ResultBundleSchema {
    match profile {
        "deep_learning" => ResultBundleSchema {
            bundle_kind: "deep_learning_result_bundle".to_string(),
            summary_fields: vec![
                ResultBundleField {
                    name: "run_id".to_string(),
                    kind: "string".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "checkpoint_path".to_string(),
                    kind: "path".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "best_validation_metric".to_string(),
                    kind: "metric".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "resource_summary".to_string(),
                    kind: "summary".to_string(),
                    required: true,
                },
            ],
            required_artifact_refs: vec![
                "training_script".to_string(),
                "evaluation_report".to_string(),
                "training_curve_or_checkpoint_log".to_string(),
            ],
        },
        "systems_evaluation" => ResultBundleSchema {
            bundle_kind: "systems_evaluation_result_bundle".to_string(),
            summary_fields: vec![
                ResultBundleField {
                    name: "run_id".to_string(),
                    kind: "string".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "workload_name".to_string(),
                    kind: "string".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "latency_summary".to_string(),
                    kind: "metric_group".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "throughput_summary".to_string(),
                    kind: "metric_group".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "resource_summary".to_string(),
                    kind: "metric_group".to_string(),
                    required: true,
                },
            ],
            required_artifact_refs: vec![
                "benchmark_configuration".to_string(),
                "benchmark_runner".to_string(),
                "performance_report".to_string(),
            ],
        },
        "agent_evaluation" => ResultBundleSchema {
            bundle_kind: "agent_evaluation_result_bundle".to_string(),
            summary_fields: vec![
                ResultBundleField {
                    name: "run_id".to_string(),
                    kind: "string".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "task_success_rate".to_string(),
                    kind: "metric".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "tool_error_rate".to_string(),
                    kind: "metric".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "judge_summary".to_string(),
                    kind: "summary".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "trajectory_sample_count".to_string(),
                    kind: "integer".to_string(),
                    required: true,
                },
            ],
            required_artifact_refs: vec![
                "task_suite_manifest".to_string(),
                "evaluation_orchestration_script".to_string(),
                "trajectory_or_metrics_report".to_string(),
            ],
        },
        "security_analysis" => ResultBundleSchema {
            bundle_kind: "security_analysis_result_bundle".to_string(),
            summary_fields: vec![
                ResultBundleField {
                    name: "run_id".to_string(),
                    kind: "string".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "confirmed_findings".to_string(),
                    kind: "integer".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "false_positive_count".to_string(),
                    kind: "integer".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "coverage_summary".to_string(),
                    kind: "summary".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "impact_summary".to_string(),
                    kind: "summary".to_string(),
                    required: true,
                },
            ],
            required_artifact_refs: vec![
                "target_manifest".to_string(),
                "analysis_or_detection_script".to_string(),
                "findings_report".to_string(),
            ],
        },
        "classical_ml" => ResultBundleSchema {
            bundle_kind: "classical_ml_result_bundle".to_string(),
            summary_fields: vec![
                ResultBundleField {
                    name: "run_id".to_string(),
                    kind: "string".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "primary_metric".to_string(),
                    kind: "metric".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "baseline_delta".to_string(),
                    kind: "metric_delta".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "error_analysis_summary".to_string(),
                    kind: "summary".to_string(),
                    required: true,
                },
            ],
            required_artifact_refs: vec![
                "dataset_split".to_string(),
                "train_or_eval_script".to_string(),
                "metrics_report".to_string(),
            ],
        },
        _ => ResultBundleSchema {
            bundle_kind: "general_cs_result_bundle".to_string(),
            summary_fields: vec![
                ResultBundleField {
                    name: "run_id".to_string(),
                    kind: "string".to_string(),
                    required: true,
                },
                ResultBundleField {
                    name: "summary_metric".to_string(),
                    kind: "metric".to_string(),
                    required: true,
                },
            ],
            required_artifact_refs: vec!["metrics_report".to_string()],
        },
    }
}

fn build_lineage_schema_for_profile(profile: &str) -> LineageSchema {
    let compare_keys = match profile {
        "deep_learning" => vec![
            "best_validation_metric".to_string(),
            "training_time_minutes".to_string(),
            "gpu_or_memory_footprint".to_string(),
        ],
        "systems_evaluation" => vec![
            "latency_ms".to_string(),
            "throughput_ops_per_sec".to_string(),
            "memory_mb".to_string(),
        ],
        "agent_evaluation" => vec![
            "task_success_rate".to_string(),
            "trajectory_cost".to_string(),
            "tool_error_rate".to_string(),
        ],
        "security_analysis" => vec![
            "precision".to_string(),
            "recall".to_string(),
            "false_positive_rate".to_string(),
        ],
        "classical_ml" => vec![
            "accuracy".to_string(),
            "f1".to_string(),
            "fit_time_seconds".to_string(),
        ],
        _ => vec!["summary_metric".to_string()],
    };

    LineageSchema {
        required: true,
        compare_keys,
        history_fields: vec![
            "run_id".to_string(),
            "parent_run_id".to_string(),
            "variant_label".to_string(),
            "change_summary".to_string(),
            "artifact_paths".to_string(),
        ],
    }
}

fn build_reproducibility_for_profile(profile: &str) -> ReproducibilityDescriptor {
    let notes = match profile {
        "deep_learning" => vec![
            "Record random seed, dependency versions, hardware type, and training command.".to_string(),
            "Persist checkpoint policy and validation selection rule with the experiment artifacts.".to_string(),
        ],
        "systems_evaluation" => vec![
            "Record hardware, OS, runtime flags, benchmark load shape, and warmup policy.".to_string(),
            "Keep raw benchmark logs or profiler outputs next to the summary report.".to_string(),
        ],
        "agent_evaluation" => vec![
            "Record prompt/policy version, tool availability, and task-suite revision.".to_string(),
            "Keep task-level trajectories or failure traces for later review.".to_string(),
        ],
        "security_analysis" => vec![
            "Record target version, rules/configuration, and whether seeded vulnerabilities were used.".to_string(),
            "Separate confirmed findings from suspected findings in the final report.".to_string(),
        ],
        "classical_ml" => vec![
            "Record the seed, dependency versions, and command line.".to_string(),
            "Keep dataset split definitions and preprocessing settings with the experiment artifacts.".to_string(),
        ],
        _ => vec![
            "Record the seed, dependency versions, and command line.".to_string(),
            "Keep dataset split definitions with the experiment artifacts.".to_string(),
        ],
    };

    ReproducibilityDescriptor {
        random_seed_required: true,
        fixed_split_required: true,
        environment_capture_required: true,
        notes,
    }
}

fn detect_format(path: &str, requested: Option<&str>) -> String {
    let requested = requested.unwrap_or("auto").trim().to_ascii_lowercase();
    if requested != "auto" && !requested.is_empty() {
        return requested;
    }

    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase()
}

fn inspect_delimited(content: &str, delimiter: char, preview_rows: usize) -> Value {
    let non_empty_lines = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    let rows: Vec<Vec<String>> = non_empty_lines
        .iter()
        .take(preview_rows + 1)
        .map(|line| {
            line.split(delimiter)
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect();

    if rows.is_empty() {
        return json!({
            "format": "table",
            "rows_previewed": 0,
            "row_count_hint": 0,
            "column_count": 0,
            "columns": [],
            "preview": [],
        });
    }

    let columns = rows.first().cloned().unwrap_or_default();
    let preview = rows.iter().skip(1).cloned().collect::<Vec<_>>();
    json!({
        "format": "table",
        "rows_previewed": preview.len(),
        "row_count_hint": non_empty_lines.len().saturating_sub(1),
        "column_count": columns.len(),
        "columns": columns,
        "preview": preview,
    })
}

fn inspect_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => {
            let columns = items
                .iter()
                .find_map(|item| item.as_object())
                .map(|map| map.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            json!({
                "format": "json",
                "shape": "array",
                "size_hint": items.len(),
                "row_count_hint": items.len(),
                "column_count": columns.len(),
                "columns": columns,
                "preview": items.iter().take(5).cloned().collect::<Vec<_>>(),
            })
        }
        Value::Object(map) => json!({
            "format": "json",
            "shape": "object",
            "size_hint": map.len(),
            "row_count_hint": 1,
            "column_count": map.len(),
            "columns": map.keys().cloned().collect::<Vec<_>>(),
            "preview": value,
        }),
        _ => json!({
            "format": "json",
            "shape": "scalar",
            "size_hint": 1,
            "row_count_hint": 1,
            "column_count": 0,
            "columns": [],
            "preview": value,
        }),
    }
}

fn infer_task_hint(columns: &[String]) -> Option<String> {
    let lowered = columns
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();

    if lowered
        .iter()
        .any(|name| name.contains("label") || name.contains("target") || name.contains("class"))
    {
        Some("supervised_learning".to_string())
    } else if lowered
        .iter()
        .any(|name| name.contains("text") || name.contains("prompt") || name.contains("sentence"))
    {
        Some("nlp_or_text_processing".to_string())
    } else if lowered
        .iter()
        .any(|name| name.contains("image") || name.contains("pixel") || name.contains("path"))
    {
        Some("vision_or_multimedia".to_string())
    } else {
        None
    }
}

fn build_dataset_descriptor(path: &str, format_name: &str, summary: &Value) -> DatasetDescriptor {
    let columns = summary["columns"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    DatasetDescriptor {
        dataset_id: Path::new(path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("dataset")
            .to_string(),
        provider: "local".to_string(),
        path: path.to_string(),
        format: format_name.to_string(),
        row_count_hint: summary["row_count_hint"]
            .as_u64()
            .map(|value| value as usize)
            .or_else(|| summary["size_hint"].as_u64().map(|value| value as usize)),
        column_count_hint: summary["column_count"].as_u64().map(|value| value as usize),
        columns: columns.clone(),
        split_hint: None,
        task_hint: infer_task_hint(&columns),
    }
}

pub(crate) fn build_default_benchmark_plan(problem_formulation: &str) -> Value {
    build_default_benchmark_plan_with_paper_hints(problem_formulation, &[])
}

pub(crate) fn build_default_benchmark_plan_with_paper_hints(
    problem_formulation: &str,
    paper_dataset_hints: &[String],
) -> Value {
    let profile = infer_benchmark_profile(problem_formulation);
    let task = if problem_formulation.trim().is_empty() {
        "Document the benchmark task, target metric, and success criteria.".to_string()
    } else {
        problem_formulation.to_string()
    };
    let plan = BenchmarkPlan {
        schema_version: BENCHMARK_SCHEMA_VERSION,
        benchmark_profile: profile.to_string(),
        task: task.clone(),
        datasets: vec![build_default_dataset_placeholder(profile)],
        dataset_acquisition: build_dataset_acquisition_plan(profile, &task, paper_dataset_hints),
        metrics: build_metrics_for_profile(profile),
        baselines: build_baselines_for_profile(profile),
        artifacts: build_artifacts_for_profile(profile),
        execution_schema: build_execution_schema_for_profile(profile),
        result_bundle_schema: build_result_bundle_schema_for_profile(profile),
        lineage_schema: build_lineage_schema_for_profile(profile),
        reproducibility: build_reproducibility_for_profile(profile),
    };

    serde_json::to_value(plan)
        .unwrap_or_else(|_| json!({ "schema_version": BENCHMARK_SCHEMA_VERSION }))
}

#[tool]
impl DataTools {
    /// Inspect a local dataset and return a lightweight structural summary.
    ///
    /// Supported formats: csv, tsv, json, jsonl.
    pub fn inspect_dataset(
        &self,
        path: String,
        format: Option<String>,
        preview_rows: Option<usize>,
    ) -> Result<Value, String> {
        let preview_rows = preview_rows.unwrap_or(5).clamp(1, 20);
        let dataset_path = Path::new(&path);
        if !dataset_path.exists() {
            return Err(format!("inspect_dataset: file does not exist: {}", path));
        }
        if !dataset_path.is_file() {
            return Err(format!("inspect_dataset: path is not a file: {}", path));
        }

        let content = fs::read_to_string(dataset_path)
            .map_err(|err| format!("inspect_dataset: failed to read '{}': {}", path, err))?;
        let format_name = detect_format(&path, format.as_deref());

        let summary = match format_name.as_str() {
            "csv" => inspect_delimited(&content, ',', preview_rows),
            "tsv" => inspect_delimited(&content, '\t', preview_rows),
            "json" => {
                let value: Value = serde_json::from_str(&content).map_err(|err| {
                    format!("inspect_dataset: invalid json in '{}': {}", path, err)
                })?;
                inspect_json_value(&value)
            }
            "jsonl" => {
                let rows = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .take(preview_rows)
                    .map(|line| {
                        serde_json::from_str::<Value>(line)
                            .unwrap_or_else(|_| json!({ "raw": line }))
                    })
                    .collect::<Vec<_>>();
                let columns = rows
                    .iter()
                    .find_map(|item| item.as_object())
                    .map(|map| map.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                json!({
                    "format": "jsonl",
                    "rows_previewed": rows.len(),
                    "row_count_hint": content.lines().filter(|line| !line.trim().is_empty()).count(),
                    "column_count": columns.len(),
                    "columns": columns,
                    "preview": rows,
                })
            }
            other => {
                return Err(format!(
                    "inspect_dataset: unsupported dataset format '{}' for '{}'. Use csv, tsv, json, or jsonl.",
                    other, path
                ));
            }
        };
        let dataset = build_dataset_descriptor(&path, &format_name, &summary);

        Ok(json!({
            "status": "success",
            "operation": "inspect_dataset",
            "path": path,
            "benchmark_schema_version": BENCHMARK_SCHEMA_VERSION,
            "dataset": dataset,
            "summary": summary
        }))
    }

    /// Search public dataset candidates directly against official dataset databases and provider APIs.
    pub fn search_public_datasets(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let limit = limit.unwrap_or(5).clamp(1, 10);
        let mut direct_datasets = Vec::new();
        let mut notes = Vec::new();
        let mut errors = Vec::new();

        match search_openml_datasets(&query, limit) {
            Ok(items) => {
                notes.push(format!("openml returned {} candidate(s)", items.len()));
                direct_datasets.extend(items);
            }
            Err(err) => errors.push(format!("openml: {}", err)),
        }
        match search_huggingface_datasets(&query, limit) {
            Ok(items) => {
                notes.push(format!("huggingface returned {} candidate(s)", items.len()));
                direct_datasets.extend(items);
            }
            Err(err) => errors.push(format!("huggingface: {}", err)),
        }
        let google_items = search_google_dataset_search(&query, limit);
        if !google_items.is_empty() {
            notes.push(format!(
                "google dataset search returned {} directory entrie(s)",
                google_items.len()
            ));
            direct_datasets.extend(google_items);
        }
        let torchvision_items = search_torchvision_datasets(&query, limit);
        if !torchvision_items.is_empty() {
            notes.push(format!(
                "torchvision datasets returned {} registry entrie(s)",
                torchvision_items.len()
            ));
            direct_datasets.extend(torchvision_items);
        }

        let fallback_candidates = DATASET_SOURCE_FILTERS
            .iter()
            .filter_map(|domain| {
                public_dataset_record_from_recovery_candidate(
                    &json!({ "url": format!("https://{}", domain) }),
                    &query,
                )
            })
            .collect::<Vec<_>>();
        let datasets = merge_public_dataset_records(direct_datasets, fallback_candidates, limit);
        let dataset_resolution_mode = if datasets.iter().any(|entry| {
            entry.source_tier == "official_provider_page"
                || entry.source_tier == "direct_provider_result"
        }) {
            "direct_provider_results"
        } else if datasets.is_empty() {
            "empty"
        } else {
            "provider_search_fallback"
        };

        Ok(json!({
            "status": "success",
            "operation": "search_public_datasets",
            "query": query,
            "provider": "direct-official-dataset-databases",
            "dataset_resolution_mode": dataset_resolution_mode,
            "raw_result_count": datasets.len(),
            "filtered_non_dataset_hits": 0,
            "recovery_scheduled_count": 0,
            "total": datasets.len(),
            "datasets": datasets,
            "dataset_source_policy": "direct_official_databases_only",
            "dataset_direct_sources": ["openml", "huggingface", "google_dataset_search", "torchvision_datasets", "paperswithcode", "kaggle"],
            "notes": notes,
            "errors": errors,
            "paper_source_policy": "official_paper_apis_only",
        }))
    }

    /// Fetch a lightweight manifest for a selected public dataset candidate.
    pub fn fetch_public_dataset_manifest(
        &self,
        dataset_url: String,
        title: Option<String>,
    ) -> Result<Value, String> {
        if dataset_url.trim().is_empty() {
            return Err("fetch_public_dataset_manifest: dataset_url is required.".to_string());
        }
        let provider = classify_dataset_provider(&dataset_url, None);
        let title = title
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| dataset_url.clone());
        Ok(json!({
            "status": "success",
            "operation": "fetch_public_dataset_manifest",
            "dataset": {
                "dataset_id": slugify_dataset_title(&title, &provider),
                "title": title,
                "provider": provider,
                "path": dataset_url,
                "format": infer_format_hint_from_url(&dataset_url).unwrap_or_else(|| "dataset_hub".to_string()),
                "task_hint": infer_public_dataset_task_hint(&dataset_url, ""),
            },
            "manifest": {
                "source_url": dataset_url,
                "source_kind": "official_dataset_database_or_provider_page",
                "retrieval_entrypoint": "official_dataset_databases",
                "paper_source_policy": "official_paper_apis_only",
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_inspect_dataset_returns_benchmark_descriptor() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dataset_path = temp_dir.path().join("iris_subset.csv");
        let mut file = fs::File::create(&dataset_path).unwrap();
        writeln!(file, "sepal_length,sepal_width,label").unwrap();
        writeln!(file, "5.1,3.5,setosa").unwrap();
        writeln!(file, "4.9,3.0,setosa").unwrap();

        let payload = DataTools
            .inspect_dataset(dataset_path.to_string_lossy().to_string(), None, Some(5))
            .unwrap();

        assert_eq!(
            payload["benchmark_schema_version"],
            BENCHMARK_SCHEMA_VERSION
        );
        assert_eq!(payload["dataset"]["dataset_id"], "iris_subset");
        assert_eq!(payload["dataset"]["provider"], "local");
        assert_eq!(payload["dataset"]["format"], "csv");
        assert_eq!(payload["dataset"]["row_count_hint"], 2);
        assert_eq!(payload["dataset"]["column_count_hint"], 3);
        assert_eq!(payload["dataset"]["task_hint"], "supervised_learning");
    }

    #[test]
    fn test_build_default_benchmark_plan_has_stable_schema() {
        let plan = build_default_benchmark_plan("Evaluate a baseline on a local dataset");
        assert_eq!(plan["schema_version"], BENCHMARK_SCHEMA_VERSION);
        assert_eq!(plan["benchmark_profile"], "general_cs");
        assert_eq!(plan["task"], "Evaluate a baseline on a local dataset");
        assert!(plan["datasets"].as_array().unwrap_or(&Vec::new()).len() >= 1);
        assert!(plan["metrics"].as_array().unwrap_or(&Vec::new()).len() >= 2);
        assert!(
            plan["execution_schema"]["stages"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 1
        );
        assert!(
            plan["result_bundle_schema"]["summary_fields"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 1
        );
        assert_eq!(plan["lineage_schema"]["required"], true);
        assert_eq!(plan["reproducibility"]["random_seed_required"], true);
    }

    #[test]
    fn test_infer_benchmark_profile_for_classical_ml() {
        assert_eq!(
            infer_benchmark_profile(
                "Use sklearn logistic regression on the iris dataset with cross validation"
            ),
            "classical_ml"
        );
    }

    #[test]
    fn test_infer_benchmark_profile_prefers_classical_ml_over_generic_benchmark_wording() {
        assert_eq!(
            infer_benchmark_profile(
                "Benchmark a tiny iris classifier comparison with accuracy, F1, and cross validation"
            ),
            "classical_ml"
        );
    }

    #[test]
    fn test_infer_benchmark_profile_for_deep_learning() {
        assert_eq!(
            infer_benchmark_profile(
                "Train a transformer with checkpoint monitoring over several epochs"
            ),
            "deep_learning"
        );
    }

    #[test]
    fn test_infer_benchmark_profile_for_systems_evaluation() {
        assert_eq!(
            infer_benchmark_profile(
                "Measure latency, throughput, and memory overhead of the runtime service"
            ),
            "systems_evaluation"
        );
    }

    #[test]
    fn test_infer_benchmark_profile_for_agent_evaluation() {
        assert_eq!(
            infer_benchmark_profile(
                "Run a multi-agent tool-use benchmark and compare task success trajectories"
            ),
            "agent_evaluation"
        );
    }

    #[test]
    fn test_infer_benchmark_profile_for_security_analysis() {
        assert_eq!(
            infer_benchmark_profile(
                "Evaluate static analysis recall on a vulnerability benchmark with fuzzing traces"
            ),
            "security_analysis"
        );
    }

    #[test]
    fn test_build_default_benchmark_plan_for_classical_ml_profile() {
        let plan =
            build_default_benchmark_plan("Use sklearn logistic regression on the iris dataset");
        assert_eq!(plan["benchmark_profile"], "classical_ml");
        assert_eq!(
            plan["datasets"][0]["dataset_id"],
            "tabular_or_labeled_dataset"
        );
        assert_eq!(
            plan["dataset_acquisition"]["retrieval_entrypoint"],
            "official_dataset_databases"
        );
        assert_eq!(
            plan["dataset_acquisition"]["search_tool"],
            "search_public_datasets"
        );
        assert_eq!(plan["metrics"][0]["name"], "accuracy");
        assert_eq!(plan["baselines"][0]["name"], "majority_class_baseline");
        assert_eq!(plan["execution_schema"]["runner_kind"], "training_pipeline");
        assert_eq!(
            plan["result_bundle_schema"]["bundle_kind"],
            "classical_ml_result_bundle"
        );
        assert!(plan["lineage_schema"]["compare_keys"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|value| value == "accuracy"));
    }

    #[test]
    fn test_build_default_benchmark_plan_for_systems_profile() {
        let plan = build_default_benchmark_plan(
            "Benchmark service latency and throughput under concurrent load",
        );
        assert_eq!(plan["benchmark_profile"], "systems_evaluation");
        assert_eq!(
            plan["datasets"][0]["dataset_id"],
            "workload_trace_or_benchmark_suite"
        );
        assert_eq!(
            plan["dataset_acquisition"]["retrieval_entrypoint"],
            "official_dataset_databases"
        );
        assert!(plan["dataset_acquisition"]["search_queries"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|value| value.as_str().unwrap_or("").contains("benchmark suite")));
        assert_eq!(plan["artifacts"][1]["name"], "benchmark_runner");
        assert_eq!(plan["metrics"][1]["name"], "throughput_ops_per_sec");
        assert_eq!(plan["execution_schema"]["runner_kind"], "benchmark_harness");
        assert_eq!(
            plan["result_bundle_schema"]["bundle_kind"],
            "systems_evaluation_result_bundle"
        );
        assert!(plan["lineage_schema"]["compare_keys"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|value| value == "latency_ms"));
    }

    #[test]
    fn test_extract_paper_dataset_hints_from_structured_value() {
        let payload = json!({
            "paper": {
                "title": "Benchmarking on CIFAR-10 and ImageNet",
                "abstract_text": "We evaluate our model on CIFAR-10, ImageNet, and SVHN."
            },
            "structured_document": {
                "sections": [
                    {"title": "Datasets", "content": "Datasets: CIFAR-10; ImageNet-1K; SVHN"}
                ]
            }
        });
        let hints = extract_paper_dataset_hints_from_value(&payload);
        assert!(hints
            .iter()
            .any(|item| item.eq_ignore_ascii_case("CIFAR-10")));
        assert!(hints
            .iter()
            .any(|item| item.eq_ignore_ascii_case("ImageNet-1K")
                || item.eq_ignore_ascii_case("ImageNet")));
    }

    #[test]
    fn test_build_default_benchmark_plan_prioritizes_paper_dataset_hints() {
        let plan = build_default_benchmark_plan_with_paper_hints(
            "Train a lightweight image classifier from recent papers",
            &["CIFAR-10".to_string(), "SVHN".to_string()],
        );
        let empty = Vec::new();
        let queries = plan["dataset_acquisition"]["search_queries"]
            .as_array()
            .unwrap_or(&empty)
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        assert!(queries
            .first()
            .is_some_and(|value| value.contains("CIFAR-10")));
        assert!(plan["dataset_acquisition"]["paper_dataset_hints"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|value| value == "CIFAR-10"));
    }

    #[test]
    fn test_fetch_public_dataset_manifest_keeps_dataset_metadata() {
        let payload = DataTools
            .fetch_public_dataset_manifest(
                "https://huggingface.co/datasets/allenai/c4".to_string(),
                Some("C4".to_string()),
            )
            .unwrap();
        assert_eq!(payload["dataset"]["provider"], "huggingface");
        assert_eq!(
            payload["manifest"]["retrieval_entrypoint"],
            "official_dataset_databases"
        );
        assert_eq!(
            payload["manifest"]["paper_source_policy"],
            "official_paper_apis_only"
        );
    }

    #[test]
    fn test_build_openml_dataset_record_marks_official_source() {
        let record = build_openml_dataset_record(&json!({
            "did": 61,
            "name": "iris",
            "version": 1,
            "format": "ARFF",
            "quality": [
                { "name": "NumberOfInstances", "value": "150.0" }
            ]
        }))
        .expect("openml record");

        assert_eq!(record.provider, "openml");
        assert!(record.official_source);
        assert_eq!(record.source_tier, "official_provider_page");
        assert!(record.url.contains("openml.org"));
    }

    #[test]
    fn test_merge_public_dataset_records_prefers_official_records_first() {
        let official = PublicDatasetRecord {
            dataset_id: "openml-61".to_string(),
            title: "iris v1".to_string(),
            url: "https://www.openml.org/search?type=data&id=61".to_string(),
            provider: "openml".to_string(),
            snippet: "Official OpenML dataset page".to_string(),
            source_kind: "official_provider_dataset_page".to_string(),
            official_source: true,
            source_tier: "official_provider_page".to_string(),
            format_hint: Some("arff".to_string()),
            task_hint: Some("supervised_learning".to_string()),
        };
        let retrieval_base = PublicDatasetRecord {
            dataset_id: "openml-openml-datasets".to_string(),
            title: "OpenML Datasets".to_string(),
            url: "https://www.openml.org/search?type=data&sort=runs&id=0&status=active&q=iris"
                .to_string(),
            provider: "openml".to_string(),
            snippet: "Retrieval-base fallback".to_string(),
            source_kind: "direct_provider_search_fallback".to_string(),
            official_source: false,
            source_tier: "provider_search_fallback".to_string(),
            format_hint: Some("dataset_hub".to_string()),
            task_hint: Some("supervised_learning".to_string()),
        };

        let merged = merge_public_dataset_records(vec![official.clone()], vec![retrieval_base], 5);
        assert_eq!(
            merged.first().map(|item| item.url.as_str()),
            Some(official.url.as_str())
        );
        assert!(merged
            .first()
            .map(|item| item.official_source)
            .unwrap_or(false));
    }
}
