//! Verification Center
//!
//! Unified detection, execution, scoring, and reporting for CS verification tools
//! and paper/research platforms.

use serde::Serialize;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tokitai::tool;

use crate::scientist::tools::literature::LiteratureTools;
use crate::toolchain::{command_is_available, detect_toolchain_executable};

pub struct VerificationCenterTools;

const HF_TRENDING_PAPERS_URL: &str = "https://huggingface.co/papers";
const MLPERF_BENCHMARKS_URL: &str = "https://mlcommons.org/benchmarks/";
const CODESOTA_URL: &str = "https://paperswithcode.com/sota";
const WANDB_REPORTS_URL: &str = "https://wandb.ai/site/reports";
const MLFLOW_DOCS_URL: &str = "https://mlflow.org/docs/latest/index.html";
const DVC_DOCS_URL: &str = "https://dvc.org/doc";

#[derive(Debug, Clone, Serialize)]
struct ProbeResult {
    name: String,
    kind: String,
    available: bool,
    command: Option<String>,
    probe: String,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PlatformStatus {
    name: String,
    kind: String,
    available: bool,
    endpoint: Option<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VerificationCenterSummary {
    score: u32,
    ready_tools: usize,
    total_tools: usize,
    ready_platforms: usize,
    total_platforms: usize,
}

#[derive(Debug, Clone, Serialize)]
struct VerificationBundleSpec {
    id: String,
    label: String,
    target_profile: String,
    goals: Vec<String>,
    preferred_tools: Vec<String>,
    fallback_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct VerificationBundleRun {
    bundle_id: String,
    label: String,
    target_profile: String,
    goals: Vec<String>,
    executed_tools: Vec<String>,
    skipped_tools: Vec<Value>,
    runs: Vec<Value>,
    bundle_score: u32,
}

#[derive(Debug, Clone, Serialize)]
struct ResearchTrackingRecord {
    title: String,
    url: String,
    provider: String,
    snippet: String,
    kind: String,
    rank: usize,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkTrackingRecord {
    title: String,
    url: String,
    provider: String,
    snippet: String,
    benchmark_family: String,
    rank: usize,
}

fn trim_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_matches_query(haystack: &str, query: &str) -> bool {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return true;
    }
    let h = haystack.to_ascii_lowercase();
    q.split_whitespace().all(|token| h.contains(token))
}

fn search_huggingface_trending_papers(
    query: &str,
    limit: usize,
) -> Result<Vec<ResearchTrackingRecord>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|err| format!("search_research_tracking: failed to build client: {}", err))?;
    let response = client
        .get(HF_TRENDING_PAPERS_URL)
        .header("User-Agent", "tokitai-ai-scientist/1.0")
        .send()
        .map_err(|err| {
            format!(
                "search_research_tracking: failed to reach Hugging Face papers: {}",
                err
            )
        })?;
    if !response.status().is_success() {
        return Err(format!(
            "search_research_tracking: Hugging Face papers returned HTTP {}",
            response.status()
        ));
    }
    let body = response
        .text()
        .map_err(|err| format!("search_research_tracking: invalid HTML body: {}", err))?;
    let document = scraper::Html::parse_document(&body);
    let article_selector = scraper::Selector::parse("article").expect("valid article selector");
    let link_selector = scraper::Selector::parse("a[href]").expect("valid link selector");
    let text_selector = scraper::Selector::parse("h1, h2, h3, h4, p").expect("valid text selector");

    let mut out = Vec::new();
    for article in document.select(&article_selector) {
        let mut href = String::new();
        let mut title = String::new();
        for link in article.select(&link_selector) {
            let candidate_href = link.value().attr("href").unwrap_or("").trim();
            let candidate_text = trim_text(&link.text().collect::<Vec<_>>().join(" "));
            if candidate_href.starts_with("/papers/") && !candidate_text.is_empty() {
                href = format!("https://huggingface.co{}", candidate_href);
                title = candidate_text;
                break;
            }
        }
        if href.is_empty() || title.is_empty() {
            continue;
        }
        let snippet = article
            .select(&text_selector)
            .map(|node| trim_text(&node.text().collect::<Vec<_>>().join(" ")))
            .find(|text| !text.is_empty() && text != &title)
            .unwrap_or_else(|| "Trending paper surfaced by Hugging Face papers.".to_string());
        let combined = format!("{} {}", title, snippet);
        if !text_matches_query(&combined, query) {
            continue;
        }
        out.push(ResearchTrackingRecord {
            title,
            url: href,
            provider: "huggingface_trending_papers".to_string(),
            snippet,
            kind: "paper_trend".to_string(),
            rank: out.len() + 1,
        });
        if out.len() >= limit {
            break;
        }
    }

    if out.is_empty() {
        out.push(ResearchTrackingRecord {
            title: "Hugging Face Trending Papers".to_string(),
            url: HF_TRENDING_PAPERS_URL.to_string(),
            provider: "huggingface_trending_papers".to_string(),
            snippet: if query.trim().is_empty() {
                "Official Hugging Face papers surface for daily paper trend tracking.".to_string()
            } else {
                format!(
                    "No direct trending-paper match was parsed for '{}'; open the official Hugging Face papers surface.",
                    query.trim()
                )
            },
            kind: "paper_trend_directory".to_string(),
            rank: 1,
        });
    }

    Ok(out)
}

fn push_tracking_directory_record(
    out: &mut Vec<ResearchTrackingRecord>,
    title: &str,
    url: &str,
    provider: &str,
    snippet: String,
    kind: &str,
    limit: usize,
) {
    if out.len() >= limit {
        return;
    }
    if out.iter().any(|item| item.url == url) {
        return;
    }
    out.push(ResearchTrackingRecord {
        title: title.to_string(),
        url: url.to_string(),
        provider: provider.to_string(),
        snippet,
        kind: kind.to_string(),
        rank: out.len() + 1,
    });
}

fn search_codesota_tracking(query: &str, limit: usize) -> Vec<ResearchTrackingRecord> {
    let mut out = Vec::new();
    let q = query.trim();
    let matches = q.is_empty()
        || text_matches_query("codesota sota benchmark leaderboard paperswithcode", q);
    if matches {
        push_tracking_directory_record(
            &mut out,
            "CodeSOTA",
            CODESOTA_URL,
            "codesota",
            if q.is_empty() {
                "Official Papers with Code SOTA leaderboard surface for code and benchmark tracking."
                    .to_string()
            } else {
                format!(
                    "Search '{}' on the CodeSOTA / Papers with Code leaderboard surface.",
                    q
                )
            },
            "sota_directory",
            limit,
        );
    }
    out
}

fn search_wandb_tracking(query: &str, limit: usize) -> Vec<ResearchTrackingRecord> {
    let mut out = Vec::new();
    let q = query.trim();
    let matches = q.is_empty()
        || text_matches_query("wandb weights biases experiment tracking reports", q);
    if matches {
        push_tracking_directory_record(
            &mut out,
            "Weights & Biases Reports",
            WANDB_REPORTS_URL,
            "wandb",
            if q.is_empty() {
                "Official Weights & Biases reports surface for experiment tracking and reproducibility dashboards."
                    .to_string()
            } else {
                format!(
                    "Open the official W&B reports surface to track experiment runs related to '{}'.",
                    q
                )
            },
            "experiment_tracking_surface",
            limit,
        );
    }
    out
}

fn search_mlflow_tracking(query: &str, limit: usize) -> Vec<ResearchTrackingRecord> {
    let mut out = Vec::new();
    let q = query.trim();
    let matches = q.is_empty()
        || text_matches_query("mlflow experiment tracking model registry runs docs", q);
    if matches {
        push_tracking_directory_record(
            &mut out,
            "MLflow Tracking Docs",
            MLFLOW_DOCS_URL,
            "mlflow",
            if q.is_empty() {
                "Official MLflow tracking and model-registry documentation entrypoint.".to_string()
            } else {
                format!(
                    "Open official MLflow tracking documentation for workflows related to '{}'.",
                    q
                )
            },
            "experiment_tracking_docs",
            limit,
        );
    }
    out
}

fn search_dvc_tracking(query: &str, limit: usize) -> Vec<ResearchTrackingRecord> {
    let mut out = Vec::new();
    let q = query.trim();
    let matches = q.is_empty()
        || text_matches_query("dvc data version control experiment pipeline doc", q);
    if matches {
        push_tracking_directory_record(
            &mut out,
            "DVC Documentation",
            DVC_DOCS_URL,
            "dvc",
            if q.is_empty() {
                "Official DVC documentation for data lineage, pipelines, and experiment versioning."
                    .to_string()
            } else {
                format!(
                    "Open official DVC documentation for lineage and experiment management related to '{}'.",
                    q
                )
            },
            "lineage_docs",
            limit,
        );
    }
    out
}

fn search_mlperf_benchmarks(query: &str, limit: usize) -> Result<Vec<BenchmarkTrackingRecord>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(|err| format!("search_mlperf_benchmarks: failed to build client: {}", err))?;
    let response = client
        .get(MLPERF_BENCHMARKS_URL)
        .header("User-Agent", "tokitai-ai-scientist/1.0")
        .send()
        .map_err(|err| format!("search_mlperf_benchmarks: failed to reach MLPerf: {}", err))?;
    if !response.status().is_success() {
        return Err(format!(
            "search_mlperf_benchmarks: MLPerf returned HTTP {}",
            response.status()
        ));
    }
    let body = response
        .text()
        .map_err(|err| format!("search_mlperf_benchmarks: invalid HTML body: {}", err))?;
    let document = scraper::Html::parse_document(&body);
    let link_selector = scraper::Selector::parse("a[href]").expect("valid link selector");

    let mut out = Vec::new();
    for link in document.select(&link_selector) {
        let href = link.value().attr("href").unwrap_or("").trim();
        let title = trim_text(&link.text().collect::<Vec<_>>().join(" "));
        if title.is_empty() {
            continue;
        }
        let lower_href = href.to_ascii_lowercase();
        let lower_title = title.to_ascii_lowercase();
        let benchmark_like = lower_href.contains("mlperf")
            || lower_href.contains("benchmark")
            || lower_title.contains("mlperf")
            || lower_title.contains("benchmark")
            || lower_title.contains("training")
            || lower_title.contains("inference")
            || lower_title.contains("storage");
        if !benchmark_like {
            continue;
        }
        let resolved_url = if href.starts_with("http://") || href.starts_with("https://") {
            href.to_string()
        } else if href.starts_with('/') {
            format!("https://mlcommons.org{}", href)
        } else {
            continue;
        };
        let snippet = format!("Official MLPerf benchmark surface: {}", title);
        if !text_matches_query(&format!("{} {}", title, snippet), query) {
            continue;
        }
        let benchmark_family = if lower_title.contains("training") {
            "training"
        } else if lower_title.contains("inference") {
            "inference"
        } else if lower_title.contains("storage") {
            "storage"
        } else {
            "benchmark"
        };
        if out.iter().any(|item: &BenchmarkTrackingRecord| item.url == resolved_url) {
            continue;
        }
        out.push(BenchmarkTrackingRecord {
            title,
            url: resolved_url,
            provider: "mlperf".to_string(),
            snippet,
            benchmark_family: benchmark_family.to_string(),
            rank: out.len() + 1,
        });
        if out.len() >= limit {
            break;
        }
    }

    if out.is_empty() && query.trim().is_empty() {
        out.push(BenchmarkTrackingRecord {
            title: "MLPerf Benchmarks".to_string(),
            url: MLPERF_BENCHMARKS_URL.to_string(),
            provider: "mlperf".to_string(),
            snippet: "Official MLCommons benchmark index for training, inference, and systems evaluation."
                .to_string(),
            benchmark_family: "benchmark_directory".to_string(),
            rank: 1,
        });
    }

    Ok(out)
}

impl VerificationCenterTools {
    fn probe_command(name: &str, command: &str, args: &[&str], kind: &str) -> ProbeResult {
        let output = Command::new(command).args(args).output();
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                ProbeResult {
                    name: name.to_string(),
                    kind: kind.to_string(),
                    available: output.status.success(),
                    command: Some(command.to_string()),
                    probe: format!("{} {}", command, args.join(" ")),
                    notes: vec![stdout, stderr]
                        .into_iter()
                        .filter(|text| !text.trim().is_empty())
                        .collect(),
                }
            }
            Err(err) => ProbeResult {
                name: name.to_string(),
                kind: kind.to_string(),
                available: false,
                command: Some(command.to_string()),
                probe: format!("{} {}", command, args.join(" ")),
                notes: vec![err.to_string()],
            },
        }
    }

    fn probe_python_module(module: &str, kind: &str) -> ProbeResult {
        let python = detect_toolchain_executable("python").or_else(|| {
            if command_is_available("python") {
                Some("python".to_string())
            } else {
                None
            }
        });
        let Some(python) = python else {
            return ProbeResult {
                name: module.to_string(),
                kind: kind.to_string(),
                available: false,
                command: None,
                probe: format!("python -c import {}", module),
                notes: vec!["python unavailable".to_string()],
            };
        };

        let code = format!("import {}; print('ok')", module);
        Self::probe_command(module, &python, &["-c", &code], kind)
    }

    fn tool_probes() -> Vec<ProbeResult> {
        vec![
            Self::probe_command("pytest", "pytest", &["--version"], "code_correctness"),
            Self::probe_command("jupyter", "jupyter", &["--version"], "code_correctness"),
            Self::probe_command("ruff", "ruff", &["--version"], "code_quality"),
            Self::probe_python_module("mypy", "code_quality"),
            Self::probe_command("semgrep", "semgrep", &["--version"], "security"),
            Self::probe_command("z3", "z3", &["-h"], "theory"),
            Self::probe_command("mlflow", "mlflow", &["--version"], "experiment_tracking"),
            Self::probe_command("wandb", "wandb", &["--version"], "experiment_tracking"),
            Self::probe_command("git", "git", &["--version"], "versioning"),
            Self::probe_command("dvc", "dvc", &["version"], "versioning"),
            Self::probe_command("hyperfine", "hyperfine", &["--version"], "performance"),
            Self::probe_python_module("memory_profiler", "performance"),
            Self::probe_python_module("cProfile", "performance"),
            Self::probe_command("python", "python", &["-m", "pip", "--version"], "runtime"),
        ]
    }

    fn platform_status() -> Vec<PlatformStatus> {
        vec![
            PlatformStatus {
                name: "Papers With Code".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://paperswithcode.com".to_string()),
                notes: vec!["web discovery supported through research workflows".to_string()],
            },
            PlatformStatus {
                name: "Hugging Face".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://huggingface.co".to_string()),
                notes: vec![
                    "datasets and models can be resolved by research workflows".to_string(),
                    "trending papers are tracked through Hugging Face paper surfaces when available".to_string(),
                ],
            },
            PlatformStatus {
                name: "Hugging Face Trending Papers".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://huggingface.co/papers".to_string()),
                notes: vec!["paper trend tracking is exposed as a web-native research surface".to_string()],
            },
            PlatformStatus {
                name: "Official dataset databases".to_string(),
                kind: "dataset_platform".to_string(),
                available: true,
                endpoint: Some("https://www.openml.org".to_string()),
                notes: vec![
                    "dataset discovery is resolved directly against OpenML / Hugging Face / Google Dataset Search / torchvision / Papers With Code / Kaggle".to_string(),
                    "paper retrieval remains constrained to official paper APIs".to_string(),
                ],
            },
            PlatformStatus {
                name: "Google Dataset Search".to_string(),
                kind: "dataset_platform".to_string(),
                available: true,
                endpoint: Some("https://datasetsearch.research.google.com".to_string()),
                notes: vec!["official dataset directory entrypoint for broader dataset discovery".to_string()],
            },
            PlatformStatus {
                name: "torchvision Datasets".to_string(),
                kind: "dataset_platform".to_string(),
                available: true,
                endpoint: Some("https://pytorch.org/vision/stable/datasets.html".to_string()),
                notes: vec!["official torchvision dataset registry for computer vision benchmarks".to_string()],
            },
            PlatformStatus {
                name: "OpenAlex".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://api.openalex.org".to_string()),
                notes: vec!["remote-first literature retrieval already wired".to_string()],
            },
            PlatformStatus {
                name: "arXiv".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://export.arxiv.org/api/query".to_string()),
                notes: vec!["remote-first literature retrieval already wired".to_string()],
            },
            PlatformStatus {
                name: "OpenReview".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://api2.openreview.net".to_string()),
                notes: vec!["remote-first literature retrieval already wired".to_string()],
            },
            PlatformStatus {
                name: "ACL Anthology".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://aclanthology.org".to_string()),
                notes: vec!["ACL/NLP literature retrieval is wired through anthology landing pages".to_string()],
            },
            PlatformStatus {
                name: "CodeSOTA".to_string(),
                kind: "paper_platform".to_string(),
                available: true,
                endpoint: Some("https://paperswithcode.com/sota".to_string()),
                notes: vec!["SOTA/code tracking is surfaced through Papers With Code ranking pages".to_string()],
            },
            PlatformStatus {
                name: "Weights & Biases".to_string(),
                kind: "experiment_platform".to_string(),
                available: true,
                endpoint: Some(WANDB_REPORTS_URL.to_string()),
                notes: vec!["experiment tracking visibility is surfaced through official W&B report pages".to_string()],
            },
            PlatformStatus {
                name: "MLflow".to_string(),
                kind: "experiment_platform".to_string(),
                available: true,
                endpoint: Some(MLFLOW_DOCS_URL.to_string()),
                notes: vec!["tracking and registry workflows are surfaced through official MLflow documentation entrypoints".to_string()],
            },
            PlatformStatus {
                name: "DVC".to_string(),
                kind: "versioning_platform".to_string(),
                available: true,
                endpoint: Some(DVC_DOCS_URL.to_string()),
                notes: vec!["dataset lineage and experiment versioning workflows are surfaced through official DVC docs".to_string()],
            },
            PlatformStatus {
                name: "ONNX Model Zoo".to_string(),
                kind: "model_platform".to_string(),
                available: true,
                endpoint: Some("https://onnx.ai/supported-tools.html".to_string()),
                notes: vec!["official ONNX model ecosystem entrypoint for interoperable model artifacts".to_string()],
            },
            PlatformStatus {
                name: "MLPerf".to_string(),
                kind: "benchmark_platform".to_string(),
                available: true,
                endpoint: Some("https://mlcommons.org/benchmarks/".to_string()),
                notes: vec!["official benchmark suite reference for standardized ML systems evaluation".to_string()],
            },
        ]
    }

    fn tool_available(probes: &[ProbeResult], name: &str) -> bool {
        probes
            .iter()
            .any(|probe| probe.name == name && probe.available)
    }

    fn verification_bundles(target_profile: Option<&str>) -> Vec<VerificationBundleSpec> {
        let profile = target_profile.unwrap_or("general_cs");
        let mut bundles = vec![
            VerificationBundleSpec {
                id: "workspace_hygiene".to_string(),
                label: "Workspace hygiene".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "check unit/integration regressions".to_string(),
                    "check lint and static typing readiness".to_string(),
                    "record workspace version-control state".to_string(),
                ],
                preferred_tools: vec![
                    "pytest".to_string(),
                    "ruff".to_string(),
                    "mypy".to_string(),
                    "git".to_string(),
                ],
                fallback_notes: vec![
                    "Skip test/lint/type stages when tools are unavailable, but keep the skip reason explicit."
                        .to_string(),
                ],
            },
            VerificationBundleSpec {
                id: "dataset_lineage".to_string(),
                label: "Dataset and lineage closure".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "check dataset or benchmark source tracking".to_string(),
                    "record data/version lineage readiness".to_string(),
                ],
                preferred_tools: vec!["git".to_string(), "dvc".to_string()],
                fallback_notes: vec![
                    "If DVC is unavailable, keep git lineage and manifest-oriented notes in the report."
                        .to_string(),
                ],
            },
        ];

        match profile {
            "classical_ml" | "deep_learning" => bundles.push(VerificationBundleSpec {
                id: "ml_runtime".to_string(),
                label: "ML runtime verification".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "capture notebook or script execution readiness".to_string(),
                    "capture experiment tracking availability".to_string(),
                ],
                preferred_tools: vec![
                    "jupyter".to_string(),
                    "mlflow".to_string(),
                    "wandb".to_string(),
                ],
                fallback_notes: vec![
                    "Experiment tracking may stay optional, but the report should say whether MLflow/W&B were detected."
                        .to_string(),
                ],
            }),
            "systems_evaluation" => bundles.push(VerificationBundleSpec {
                id: "systems_perf".to_string(),
                label: "Systems performance verification".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "capture benchmark timing readiness".to_string(),
                    "capture profiling readiness".to_string(),
                ],
                preferred_tools: vec![
                    "hyperfine".to_string(),
                    "memory_profiler".to_string(),
                    "cProfile".to_string(),
                ],
                fallback_notes: vec![
                    "If dedicated benchmark tools are missing, keep the performance verification gap explicit."
                        .to_string(),
                ],
            }),
            "agent_evaluation" => bundles.push(VerificationBundleSpec {
                id: "agent_eval".to_string(),
                label: "Agent evaluation verification".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "capture task-suite execution readiness".to_string(),
                    "capture trajectory and notebook-style evaluation support".to_string(),
                ],
                preferred_tools: vec!["pytest".to_string(), "jupyter".to_string(), "git".to_string()],
                fallback_notes: vec![
                    "Agent-evaluation workflows should still expose trajectory/test execution readiness even without notebooks."
                        .to_string(),
                ],
            }),
            "security_analysis" => bundles.push(VerificationBundleSpec {
                id: "security_scan".to_string(),
                label: "Security verification".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "capture static security scan readiness".to_string(),
                    "record source-control state for remediation tracking".to_string(),
                ],
                preferred_tools: vec!["semgrep".to_string(), "git".to_string()],
                fallback_notes: vec![
                    "If Semgrep is unavailable, the report should keep the security verification gap visible."
                        .to_string(),
                ],
            }),
            "theory" => bundles.push(VerificationBundleSpec {
                id: "formal_checks".to_string(),
                label: "Formal verification".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "capture SMT/solver availability".to_string(),
                    "record proof-oriented execution support".to_string(),
                ],
                preferred_tools: vec!["z3".to_string(), "python".to_string()],
                fallback_notes: vec![
                    "If Z3 is unavailable, leave a hard note that theory claims still need external solver validation."
                        .to_string(),
                ],
            }),
            "literature_review" => bundles.push(VerificationBundleSpec {
                id: "literature_remote".to_string(),
                label: "Remote literature verification".to_string(),
                target_profile: profile.to_string(),
                goals: vec![
                    "verify remote-first paper retrieval readiness".to_string(),
                    "keep official paper API provenance explicit".to_string(),
                ],
                preferred_tools: vec![],
                fallback_notes: vec![
                    "Literature verification is mediated by official remote paper APIs rather than local CLI tools."
                        .to_string(),
                ],
            }),
            _ => {}
        }

        bundles
    }

    fn summarize(
        probes: &[ProbeResult],
        platforms: &[PlatformStatus],
    ) -> VerificationCenterSummary {
        let ready_tools = probes.iter().filter(|probe| probe.available).count();
        let ready_platforms = platforms
            .iter()
            .filter(|platform| platform.available)
            .count();
        let total_tools = probes.len();
        let total_platforms = platforms.len();
        let tool_score = if total_tools == 0 {
            0
        } else {
            (ready_tools as u32 * 70) / total_tools as u32
        };
        let platform_score = if total_platforms == 0 {
            0
        } else {
            (ready_platforms as u32 * 30) / total_platforms as u32
        };
        VerificationCenterSummary {
            score: tool_score + platform_score,
            ready_tools,
            total_tools,
            ready_platforms,
            total_platforms,
        }
    }

    fn report_to_value(
        probes: Vec<ProbeResult>,
        platforms: Vec<PlatformStatus>,
        workspace_root: Option<&str>,
        target_profile: Option<&str>,
        execution_notes: Vec<String>,
    ) -> Value {
        let summary = Self::summarize(&probes, &platforms);
        json!({
            "status": if summary.ready_tools > 0 || summary.ready_platforms > 0 { "ready" } else { "degraded" },
            "workspace_root": workspace_root,
            "target_profile": target_profile,
            "summary": summary,
            "tool_probes": probes,
            "paper_platforms": platforms,
            "execution_notes": execution_notes,
        })
    }

    fn run_workspace_command(workspace_root: &Path, command: &str, args: &[&str]) -> Value {
        let output = Command::new(command)
            .args(args)
            .current_dir(workspace_root)
            .output();
        match output {
            Ok(output) => json!({
                "command": command,
                "args": args,
                "status": if output.status.success() { "passed" } else { "failed" },
                "exit_code": output.status.code().unwrap_or(-1),
                "stdout": String::from_utf8_lossy(&output.stdout).trim(),
                "stderr": String::from_utf8_lossy(&output.stderr).trim(),
            }),
            Err(err) => json!({
                "command": command,
                "args": args,
                "status": "missing",
                "exit_code": -1,
                "stderr": err.to_string(),
            }),
        }
    }

    fn summarize_bundle_run(run: &VerificationBundleRun) -> u32 {
        let executed = run.executed_tools.len() as u32;
        let skipped = run.skipped_tools.len() as u32;
        if executed == 0 && skipped == 0 {
            0
        } else if skipped == 0 {
            100
        } else {
            (executed * 100) / (executed + skipped)
        }
    }

    fn run_bundle(
        workspace: &Path,
        probes: &[ProbeResult],
        bundle: &VerificationBundleSpec,
    ) -> VerificationBundleRun {
        let mut runs = Vec::new();
        let mut executed_tools = Vec::new();
        let mut skipped_tools = Vec::new();

        for tool_name in &bundle.preferred_tools {
            match tool_name.as_str() {
                "pytest" if Self::tool_available(probes, "pytest") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(workspace, "pytest", &["-q"]));
                }
                "ruff" if Self::tool_available(probes, "ruff") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "ruff",
                        &["check", "."],
                    ));
                }
                "mypy" if Self::tool_available(probes, "mypy") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(workspace, "mypy", &["."]));
                }
                "semgrep" if Self::tool_available(probes, "semgrep") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "semgrep",
                        &["scan", "--config", "auto", "."],
                    ));
                }
                "git" if Self::tool_available(probes, "git") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "git",
                        &["status", "--short"],
                    ));
                }
                "dvc" if Self::tool_available(probes, "dvc") => {
                    if workspace.join(".dvc").exists() || workspace.join("dvc.yaml").exists() {
                        executed_tools.push(tool_name.clone());
                        runs.push(Self::run_workspace_command(workspace, "dvc", &["status"]));
                    } else {
                        skipped_tools.push(json!({
                            "tool": tool_name,
                            "reason": "dvc project not detected in workspace",
                        }));
                    }
                }
                "jupyter" if Self::tool_available(probes, "jupyter") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "jupyter",
                        &["--paths"],
                    ));
                }
                "mlflow" if Self::tool_available(probes, "mlflow") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "mlflow",
                        &["--version"],
                    ));
                }
                "wandb" if Self::tool_available(probes, "wandb") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "wandb",
                        &["--version"],
                    ));
                }
                "hyperfine" if Self::tool_available(probes, "hyperfine") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "hyperfine",
                        &["--version"],
                    ));
                }
                "memory_profiler" if Self::tool_available(probes, "memory_profiler") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "python",
                        &["-c", "import memory_profiler; print('memory_profiler ok')"],
                    ));
                }
                "cProfile" if Self::tool_available(probes, "cProfile") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "python",
                        &["-c", "import cProfile; print('cProfile ok')"],
                    ));
                }
                "z3" if Self::tool_available(probes, "z3") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(workspace, "z3", &["-h"]));
                }
                "python" if Self::tool_available(probes, "python") => {
                    executed_tools.push(tool_name.clone());
                    runs.push(Self::run_workspace_command(
                        workspace,
                        "python",
                        &["-c", "print('python runtime ok')"],
                    ));
                }
                other => skipped_tools.push(json!({
                    "tool": other,
                    "reason": "tool unavailable or not runnable for this workspace",
                })),
            }
        }

        let mut bundle_run = VerificationBundleRun {
            bundle_id: bundle.id.clone(),
            label: bundle.label.clone(),
            target_profile: bundle.target_profile.clone(),
            goals: bundle.goals.clone(),
            executed_tools,
            skipped_tools,
            runs,
            bundle_score: 0,
        };
        bundle_run.bundle_score = Self::summarize_bundle_run(&bundle_run);
        bundle_run
    }
}

#[tool]
impl VerificationCenterTools {
    /// Detect installed verification tools and supported research platforms.
    pub fn verification_center_status(&self) -> Result<Value, String> {
        let probes = Self::tool_probes();
        let platforms = Self::platform_status();
        let bundles = Self::verification_bundles(None);
        let mut report = Self::report_to_value(
            probes,
            platforms,
            None,
            None,
            vec![
                "Auto-detected installed verification tools.".to_string(),
                "Paper platforms are exposed through remote literature workflows.".to_string(),
                format!(
                    "{} verification bundles are available for orchestration.",
                    bundles.len()
                ),
            ],
        );
        if let Some(object) = report.as_object_mut() {
            object.insert("available_bundles".to_string(), json!(bundles));
        }
        Ok(report)
    }

    /// Run a verification bundle over the workspace with installed tools.
    pub fn verification_center_run(
        &self,
        workspace_root: Option<String>,
        target_profile: Option<String>,
    ) -> Result<Value, String> {
        let workspace = workspace_root
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let mut execution_notes = Vec::new();
        let probes = Self::tool_probes();
        let platforms = Self::platform_status();
        let bundles = Self::verification_bundles(target_profile.as_deref());
        let bundle_runs = bundles
            .iter()
            .map(|bundle| Self::run_bundle(&workspace, &probes, bundle))
            .collect::<Vec<_>>();
        let mut runs = Vec::new();
        for bundle_run in &bundle_runs {
            execution_notes.push(format!(
                "{}: executed {} tool(s), skipped {} tool(s)",
                bundle_run.bundle_id,
                bundle_run.executed_tools.len(),
                bundle_run.skipped_tools.len()
            ));
            runs.extend(bundle_run.runs.iter().cloned());
        }

        let report = Self::report_to_value(
            probes,
            platforms,
            Some(workspace.to_string_lossy().as_ref()),
            target_profile.as_deref(),
            execution_notes,
        );
        Ok(json!({
            "verification_center": report,
            "bundle_runs": bundle_runs,
            "runs": runs,
        }))
    }

    /// Build a readable summary from a verification-center run result.
    pub fn verification_center_report(&self, report: Value) -> Result<Value, String> {
        let summary = report
            .get("verification_center")
            .and_then(|value| value.get("summary"))
            .cloned()
            .unwrap_or_else(|| json!({}));
        Ok(json!({
            "summary": summary,
            "tool_count": report["verification_center"]["summary"]["total_tools"].clone(),
            "ready_tools": report["verification_center"]["summary"]["ready_tools"].clone(),
            "platform_count": report["verification_center"]["summary"]["total_platforms"].clone(),
            "ready_platforms": report["verification_center"]["summary"]["ready_platforms"].clone(),
            "score": report["verification_center"]["summary"]["score"].clone(),
            "bundle_runs": report["bundle_runs"].clone(),
            "runs": report["runs"].clone(),
        }))
    }

    /// Fetch up to three remote-first papers and expose their structured content.
    pub fn verification_center_fetch_papers(
        &self,
        query: String,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let literature = LiteratureTools;
        let search = literature.search_paper(query.clone(), None, limit.or(Some(3)))?;
        let paper_ids = search["results"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|paper| paper.get("paper_id").and_then(Value::as_str))
                    .take(3)
                    .map(|paper_id| paper_id.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let fetched = if paper_ids.is_empty() {
            json!([])
        } else {
            literature.fetch_papers(paper_ids.clone(), Some(3))?
        };
        Ok(json!({
            "query": query,
            "search": search,
            "fetched": fetched,
            "paper_ids": paper_ids,
        }))
    }

    /// Search research-tracking surfaces such as Hugging Face Trending Papers.
    pub fn search_research_tracking(
        &self,
        query: String,
        source: Option<String>,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let source = source
            .unwrap_or_else(|| "huggingface_trending_papers".to_string())
            .trim()
            .to_ascii_lowercase();
        let limit = limit.unwrap_or(8).clamp(1, 20);
        match source.as_str() {
            "huggingface_trending_papers" | "huggingface_papers" | "hf_papers" => {
                let items = search_huggingface_trending_papers(&query, limit)?;
                Ok(json!({
                    "status": "success",
                    "operation": "search_research_tracking",
                    "provider": "huggingface_trending_papers",
                    "query": query,
                    "total": items.len(),
                    "results": items,
                }))
            }
            "codesota" => {
                let items = search_codesota_tracking(&query, limit);
                Ok(json!({
                    "status": "success",
                    "operation": "search_research_tracking",
                    "provider": "codesota",
                    "query": query,
                    "total": items.len(),
                    "results": items,
                }))
            }
            "wandb" => {
                let items = search_wandb_tracking(&query, limit);
                Ok(json!({
                    "status": "success",
                    "operation": "search_research_tracking",
                    "provider": "wandb",
                    "query": query,
                    "total": items.len(),
                    "results": items,
                }))
            }
            "mlflow" => {
                let items = search_mlflow_tracking(&query, limit);
                Ok(json!({
                    "status": "success",
                    "operation": "search_research_tracking",
                    "provider": "mlflow",
                    "query": query,
                    "total": items.len(),
                    "results": items,
                }))
            }
            "dvc" => {
                let items = search_dvc_tracking(&query, limit);
                Ok(json!({
                    "status": "success",
                    "operation": "search_research_tracking",
                    "provider": "dvc",
                    "query": query,
                    "total": items.len(),
                    "results": items,
                }))
            }
            "auto" => {
                let mut items = Vec::new();
                items.extend(search_huggingface_trending_papers(&query, limit)?);
                items.extend(search_codesota_tracking(&query, limit));
                items.extend(search_wandb_tracking(&query, limit));
                items.extend(search_mlflow_tracking(&query, limit));
                items.extend(search_dvc_tracking(&query, limit));
                items.truncate(limit);
                Ok(json!({
                    "status": "success",
                    "operation": "search_research_tracking",
                    "provider": "tracking_aggregate",
                    "query": query,
                    "total": items.len(),
                    "results": items,
                }))
            }
            other => Err(format!(
                "search_research_tracking: unsupported source '{}'",
                other
            )),
        }
    }

    /// Search official MLPerf benchmark surfaces.
    pub fn search_benchmark_platforms(
        &self,
        query: String,
        source: Option<String>,
        limit: Option<usize>,
    ) -> Result<Value, String> {
        let source = source
            .unwrap_or_else(|| "mlperf".to_string())
            .trim()
            .to_ascii_lowercase();
        let limit = limit.unwrap_or(8).clamp(1, 20);
        match source.as_str() {
            "mlperf" | "auto" => {
                let items = search_mlperf_benchmarks(&query, limit)?;
                Ok(json!({
                    "status": "success",
                    "operation": "search_benchmark_platforms",
                    "provider": "mlperf",
                    "query": query,
                    "total": items.len(),
                    "results": items,
                }))
            }
            other => Err(format!(
                "search_benchmark_platforms: unsupported source '{}'",
                other
            )),
        }
    }
}
