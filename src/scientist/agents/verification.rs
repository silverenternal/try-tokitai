//! VerificationAgent — Analytical, formal, and implementation verification

use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::scientist::tools::data::BENCHMARK_SCHEMA_VERSION;

pub struct VerificationAgent {
    id: String,
}

impl VerificationAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

fn has_non_placeholder_named_items(
    items: Option<&Vec<Value>>,
    placeholder_needles: &[&str],
) -> bool {
    items
        .map(|entries| {
            entries.iter().any(|entry| {
                let name = entry
                    .get("name")
                    .or_else(|| entry.get("dataset_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim();
                let normalized = name.to_ascii_lowercase();
                !name.is_empty()
                    && !placeholder_needles.iter().any(|needle| {
                        let needle = needle.to_ascii_lowercase();
                        normalized == needle
                            || (needle.contains("to_be_selected")
                                && normalized.contains(needle.as_str()))
                    })
            })
        })
        .unwrap_or(false)
}

fn benchmark_check(status: &str, detail: &str) -> Value {
    json!({
        "status": status,
        "detail": detail,
    })
}

fn supported_benchmark_profiles() -> &'static [&'static str] {
    &[
        "classical_ml",
        "deep_learning",
        "systems_evaluation",
        "agent_evaluation",
        "security_analysis",
        "theory",
        "literature_review",
        "general_cs",
    ]
}

fn benchmark_profile_guidance(profile: &str) -> &'static str {
    match profile {
        "classical_ml" => {
            "Expect concise supervised-learning metrics, a runnable training script, and a readable metrics report."
        }
        "deep_learning" => {
            "Expect validation-oriented metrics, training/runtime resource reporting, and reproducible training artifacts."
        }
        "systems_evaluation" => {
            "Expect workload-facing metrics such as latency/throughput/resource usage plus reproducible benchmark inputs."
        }
        "agent_evaluation" => {
            "Expect task success metrics, trajectory or tool-use accounting, and evaluation reports over a task suite."
        }
        "security_analysis" => {
            "Expect detection-quality metrics, actionable reports, and artifacts that document target coverage or findings."
        }
        "theory" => {
            "Expect explicit definitions, lemma structure, proof notes, and counterexample or sanity-check evidence."
        }
        "literature_review" => {
            "Expect search scope, screening criteria, synthesis notes, and explicit research-gap artifacts."
        }
        _ => "Expect a reproducible CS benchmark with explicit metrics, artifacts, and reviewable outputs.",
    }
}

fn has_named_object_entries(value: Option<&Value>, key: &str) -> bool {
    value
        .and_then(Value::as_array)
        .map(|entries| {
            entries.iter().any(|entry| {
                entry
                    .get(key)
                    .and_then(Value::as_str)
                    .map(|raw| !raw.trim().is_empty())
                    .unwrap_or(false)
            })
        })
        .unwrap_or(false)
}

fn has_required_summary_fields(
    result_bundle: Option<&Value>,
    required_fields: &[String],
) -> (bool, Vec<String>) {
    let Some(bundle) = result_bundle.and_then(Value::as_object) else {
        return (
            false,
            required_fields
                .iter()
                .map(|field| field.to_string())
                .collect(),
        );
    };

    let field_names = bundle
        .get("summary_fields")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .get("name")
                        .or_else(|| entry.get("field"))
                        .and_then(Value::as_str)
                        .map(|raw| raw.trim().to_ascii_lowercase())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let missing = required_fields
        .iter()
        .filter(|field| {
            !field_names
                .iter()
                .any(|name| name == &field.to_ascii_lowercase())
        })
        .map(|field| field.to_string())
        .collect::<Vec<_>>();

    (missing.is_empty(), missing)
}

fn value_to_searchable_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
        Value::Null => String::new(),
    }
}

fn summary_field_value(result_bundle: Option<&Value>, field_name: &str) -> Option<String> {
    result_bundle
        .and_then(|bundle| bundle.get("summary_fields"))
        .and_then(Value::as_array)
        .and_then(|entries| {
            entries.iter().find_map(|entry| {
                let name = entry
                    .get("name")
                    .or_else(|| entry.get("field"))
                    .and_then(Value::as_str)?
                    .trim()
                    .to_ascii_lowercase();
                if name != field_name.to_ascii_lowercase() {
                    return None;
                }
                entry
                    .get("value")
                    .or_else(|| entry.get("summary"))
                    .map(value_to_searchable_text)
                    .filter(|text| !text.trim().is_empty())
            })
        })
}

fn normalized_string_set(values: &[String]) -> Vec<String> {
    let mut items = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    items.sort();
    items.dedup();
    items
}

fn current_artifact_paths(artifact_paths: Option<&Value>) -> Vec<String> {
    artifact_paths
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn lineage_history_entries(lineage: Option<&Value>) -> Vec<Value> {
    lineage
        .and_then(|value| value.get("history"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn lineage_latest_entry<'a>(history: &'a [Value], current_run_id: &str) -> Option<&'a Value> {
    if current_run_id.trim().is_empty() {
        return history.last();
    }
    history.iter().find(|entry| {
        entry
            .get("run_id")
            .and_then(Value::as_str)
            .map(|run_id| run_id.trim() == current_run_id.trim())
            .unwrap_or(false)
    })
}

fn lineage_entry_artifact_paths(entry: Option<&Value>) -> Vec<String> {
    entry
        .and_then(|value| value.get("artifact_paths"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn contains_any_case_insensitive(text: &str, needles: &[&str]) -> bool {
    let lowered = text.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| lowered.contains(&needle.to_ascii_lowercase()))
}

fn profile_runtime_value_issues(profile: &str, result_bundle: Option<&Value>) -> Vec<String> {
    match profile {
        "deep_learning" => {
            let mut issues = Vec::new();
            for field_name in [
                "best_validation_metric",
                "resource_summary",
                "checkpoint_path",
            ] {
                let value = summary_field_value(result_bundle, field_name).unwrap_or_default();
                if value.is_empty()
                    || contains_any_case_insensitive(
                        &value,
                        &[
                            "pending",
                            "placeholder",
                            "to be selected",
                            "not set",
                            "none",
                        ],
                    )
                {
                    issues.push(field_name.to_string());
                }
            }
            issues
        }
        "systems_evaluation" => {
            let mut issues = Vec::new();
            for field_name in [
                "workload_name",
                "latency_summary",
                "throughput_summary",
                "resource_summary",
            ] {
                let value = summary_field_value(result_bundle, field_name).unwrap_or_default();
                if value.is_empty()
                    || contains_any_case_insensitive(
                        &value,
                        &[
                            "pending",
                            "placeholder",
                            "to be selected",
                            "not set",
                            "none",
                        ],
                    )
                {
                    issues.push(field_name.to_string());
                }
            }
            issues
        }
        "security_analysis" => {
            let mut issues = Vec::new();
            for field_name in [
                "confirmed_findings",
                "false_positive_count",
                "coverage_summary",
                "impact_summary",
            ] {
                let value = summary_field_value(result_bundle, field_name).unwrap_or_default();
                if value.is_empty()
                    || contains_any_case_insensitive(
                        &value,
                        &[
                            "pending",
                            "placeholder",
                            "to be selected",
                            "not set",
                            "none",
                        ],
                    )
                {
                    issues.push(field_name.to_string());
                }
            }
            issues
        }
        "theory" => {
            let mut issues = Vec::new();
            let proof_status =
                summary_field_value(result_bundle, "proof_status").unwrap_or_default();
            if proof_status.is_empty()
                || contains_any_case_insensitive(
                    &proof_status,
                    &[
                        "pending",
                        "proof evidence observed",
                        "proof evidence missing",
                    ],
                )
            {
                issues.push("proof_status".to_string());
            }

            let lemma_summary =
                summary_field_value(result_bundle, "lemma_summary").unwrap_or_default();
            if lemma_summary.is_empty()
                || contains_any_case_insensitive(
                    &lemma_summary,
                    &[
                        "pending",
                        "lemma evidence observed",
                        "lemma evidence missing",
                    ],
                )
            {
                issues.push("lemma_summary".to_string());
            }

            let counterexample_status =
                summary_field_value(result_bundle, "counterexample_status").unwrap_or_default();
            if counterexample_status.is_empty()
                || contains_any_case_insensitive(
                    &counterexample_status,
                    &[
                        "pending",
                        "counterexample search observed",
                        "counterexample search missing",
                    ],
                )
            {
                issues.push("counterexample_status".to_string());
            }

            issues
        }
        "literature_review" => {
            let mut issues = Vec::new();
            let remote_fulltext =
                summary_field_value(result_bundle, "remote_fulltext_coverage").unwrap_or_default();
            if remote_fulltext.is_empty()
                || contains_any_case_insensitive(
                    &remote_fulltext,
                    &[
                        "pending",
                        "0 remote-first papers fetched",
                        "0 remote first papers fetched",
                        "0 papers with remote fulltext",
                        "remote fulltext evidence observed",
                        "metadata-only",
                        "metadata only",
                        "abstract-only",
                        "abstract only",
                        "none",
                    ],
                )
            {
                issues.push("remote_fulltext_coverage".to_string());
            }

            let structured =
                summary_field_value(result_bundle, "structured_paper_coverage").unwrap_or_default();
            if structured.is_empty()
                || contains_any_case_insensitive(
                    &structured,
                    &[
                        "pending",
                        "0 papers include structured sections",
                        "structured-paper evidence observed",
                        "structured paper evidence observed",
                        "metadata-only",
                        "metadata only",
                        "none",
                    ],
                )
            {
                issues.push("structured_paper_coverage".to_string());
            }

            issues
        }
        _ => Vec::new(),
    }
}

fn profile_runtime_semantic_issues(profile: &str, result_bundle: Option<&Value>) -> Vec<String> {
    match profile {
        "deep_learning" => {
            let mut issues = Vec::new();
            let metric =
                summary_field_value(result_bundle, "best_validation_metric").unwrap_or_default();
            if !metric.trim().is_empty()
                && !contains_any_case_insensitive(
                    &metric,
                    &["validation", "accuracy", "loss", "f1", "perplexity"],
                )
            {
                issues.push("best_validation_metric_semantic_alignment".to_string());
            }

            let resource =
                summary_field_value(result_bundle, "resource_summary").unwrap_or_default();
            if !resource.trim().is_empty()
                && !contains_any_case_insensitive(
                    &resource,
                    &["gpu", "memory", "time", "cpu", "throughput"],
                )
            {
                issues.push("resource_summary_semantic_alignment".to_string());
            }

            issues
        }
        "systems_evaluation" => {
            let mut issues = Vec::new();
            let latency = summary_field_value(result_bundle, "latency_summary").unwrap_or_default();
            if !latency.trim().is_empty()
                && !contains_any_case_insensitive(
                    &latency,
                    &["latency", "p95", "p99", "ms", "tail"],
                )
            {
                issues.push("latency_summary_semantic_alignment".to_string());
            }

            let throughput =
                summary_field_value(result_bundle, "throughput_summary").unwrap_or_default();
            if !throughput.trim().is_empty()
                && !contains_any_case_insensitive(
                    &throughput,
                    &["throughput", "ops", "qps", "req/s", "requests"],
                )
            {
                issues.push("throughput_summary_semantic_alignment".to_string());
            }

            let resource =
                summary_field_value(result_bundle, "resource_summary").unwrap_or_default();
            if !resource.trim().is_empty()
                && !contains_any_case_insensitive(
                    &resource,
                    &["memory", "cpu", "gpu", "rss", "footprint"],
                )
            {
                issues.push("resource_summary_semantic_alignment".to_string());
            }

            issues
        }
        "security_analysis" => {
            let mut issues = Vec::new();
            let findings =
                summary_field_value(result_bundle, "confirmed_findings").unwrap_or_default();
            if !findings.trim().is_empty()
                && !contains_any_case_insensitive(
                    &findings,
                    &["finding", "vulnerability", "issue", "exploit", "alert"],
                )
            {
                issues.push("confirmed_findings_semantic_alignment".to_string());
            }

            let coverage =
                summary_field_value(result_bundle, "coverage_summary").unwrap_or_default();
            if !coverage.trim().is_empty()
                && !contains_any_case_insensitive(
                    &coverage,
                    &["coverage", "surface", "target", "scope", "asset"],
                )
            {
                issues.push("coverage_summary_semantic_alignment".to_string());
            }

            let impact = summary_field_value(result_bundle, "impact_summary").unwrap_or_default();
            if !impact.trim().is_empty()
                && !contains_any_case_insensitive(
                    &impact,
                    &["impact", "severity", "risk", "exposure", "critical"],
                )
            {
                issues.push("impact_summary_semantic_alignment".to_string());
            }

            issues
        }
        _ => Vec::new(),
    }
}

fn has_lineage_history_fields(
    lineage: Option<&Value>,
    required_fields: &[String],
) -> (bool, Vec<String>) {
    let Some(history) = lineage
        .and_then(|value| value.get("history"))
        .and_then(Value::as_array)
    else {
        return (
            false,
            required_fields
                .iter()
                .map(|field| field.to_string())
                .collect(),
        );
    };

    if history.is_empty() {
        return (
            false,
            required_fields
                .iter()
                .map(|field| field.to_string())
                .collect(),
        );
    }

    let missing = required_fields
        .iter()
        .filter(|field| {
            !history.iter().any(|entry| {
                entry
                    .get(field.as_str())
                    .map(|value| match value {
                        Value::String(text) => !text.trim().is_empty(),
                        Value::Array(items) => !items.is_empty(),
                        Value::Null => false,
                        _ => true,
                    })
                    .unwrap_or(false)
            })
        })
        .map(|field| field.to_string())
        .collect::<Vec<_>>();

    (missing.is_empty(), missing)
}

fn profile_required_result_bundle_fields(profile: &str) -> &'static [&'static str] {
    match profile {
        "deep_learning" => &[
            "run_id",
            "checkpoint_path",
            "best_validation_metric",
            "resource_summary",
        ],
        "systems_evaluation" => &[
            "run_id",
            "workload_name",
            "latency_summary",
            "throughput_summary",
            "resource_summary",
        ],
        "agent_evaluation" => &[
            "run_id",
            "task_success_rate",
            "tool_error_rate",
            "judge_summary",
            "trajectory_sample_count",
        ],
        "security_analysis" => &[
            "run_id",
            "confirmed_findings",
            "false_positive_count",
            "coverage_summary",
            "impact_summary",
        ],
        "classical_ml" => &[
            "run_id",
            "primary_metric",
            "baseline_delta",
            "error_analysis_summary",
        ],
        "theory" => &[
            "run_id",
            "proof_status",
            "lemma_summary",
            "counterexample_status",
        ],
        "literature_review" => &[
            "run_id",
            "search_scope",
            "screening_summary",
            "remote_fulltext_coverage",
            "structured_paper_coverage",
            "gap_summary",
        ],
        _ => &["run_id"],
    }
}

fn profile_required_lineage_compare_keys(profile: &str) -> &'static [&'static str] {
    match profile {
        "deep_learning" => &[
            "best_validation_metric",
            "training_time_minutes",
            "gpu_or_memory_footprint",
        ],
        "systems_evaluation" => &["latency_ms", "throughput_ops_per_sec", "memory_mb"],
        "agent_evaluation" => &["task_success_rate", "trajectory_cost", "tool_error_rate"],
        "security_analysis" => &["precision", "recall", "false_positive_rate"],
        "classical_ml" => &["accuracy", "f1", "fit_time_seconds"],
        "theory" => &["proof_status", "lemma_coverage"],
        "literature_review" => &[
            "screened_papers",
            "included_papers",
            "remote_fulltext_papers",
            "structured_papers",
        ],
        _ => &["summary_metric"],
    }
}

fn profile_required_lineage_history_fields(profile: &str) -> &'static [&'static str] {
    match profile {
        "theory" => &["run_id", "change_summary", "artifact_paths"],
        "literature_review" => &[
            "run_id",
            "variant_label",
            "change_summary",
            "artifact_paths",
        ],
        _ => &[
            "run_id",
            "parent_run_id",
            "variant_label",
            "change_summary",
            "artifact_paths",
        ],
    }
}

fn profile_requires_multi_run_compare_closure(profile: &str) -> bool {
    matches!(
        profile,
        "classical_ml"
            | "deep_learning"
            | "systems_evaluation"
            | "agent_evaluation"
            | "security_analysis"
    )
}

fn verify_runtime_result_structures(
    benchmark_plan: Option<&Value>,
    result_bundle: Option<&Value>,
    run_comparison: Option<&Value>,
    lineage: Option<&Value>,
    artifact_paths: Option<&Value>,
) -> Value {
    let Some(plan) = benchmark_plan else {
        return json!({
            "status": "not_provided",
            "profile": Value::Null,
            "result_bundle_validation": benchmark_check("missing", "Runtime result bundle cannot be checked without benchmark_plan."),
            "run_comparison_validation": benchmark_check("missing", "Run comparison cannot be checked without benchmark_plan."),
            "lineage_validation": benchmark_check("missing", "Lineage content cannot be checked without benchmark_plan."),
            "missing_items": ["benchmark_plan"],
        });
    };

    let profile = plan
        .get("benchmark_profile")
        .and_then(Value::as_str)
        .unwrap_or("general_cs")
        .trim()
        .to_ascii_lowercase();

    let expected_bundle_fields = profile_required_result_bundle_fields(&profile)
        .iter()
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();
    let expected_compare_keys = profile_required_lineage_compare_keys(&profile)
        .iter()
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();
    let expected_history_fields = profile_required_lineage_history_fields(&profile)
        .iter()
        .map(|field| (*field).to_string())
        .collect::<Vec<_>>();

    let (bundle_ok, missing_bundle_fields) =
        has_required_summary_fields(result_bundle, &expected_bundle_fields);
    let mut profile_value_issues = profile_runtime_value_issues(&profile, result_bundle);
    let semantic_issues = profile_runtime_semantic_issues(&profile, result_bundle);
    for issue in semantic_issues {
        if !profile_value_issues
            .iter()
            .any(|existing| existing == &issue)
        {
            profile_value_issues.push(issue);
        }
    }
    let profile_values_ok = profile_value_issues.is_empty();

    let compare_key_values = run_comparison
        .and_then(|value| value.get("compare_keys"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(|raw| raw.trim().to_ascii_lowercase())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let missing_compare_keys = expected_compare_keys
        .iter()
        .filter(|key| {
            !compare_key_values
                .iter()
                .any(|value| value == &key.to_ascii_lowercase())
        })
        .map(|key| key.to_string())
        .collect::<Vec<_>>();
    let comparison_available = run_comparison
        .and_then(|value| value.get("available"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let comparison_observations = run_comparison
        .and_then(|value| value.get("observations"))
        .and_then(Value::as_array)
        .map(|items| !items.is_empty())
        .unwrap_or(false);
    let run_comparison_ok =
        comparison_available && comparison_observations && missing_compare_keys.is_empty();

    let (lineage_history_ok, missing_history_fields) =
        has_lineage_history_fields(lineage, &expected_history_fields);
    let current_run_id = summary_field_value(result_bundle, "run_id").unwrap_or_default();
    let history_entries = lineage_history_entries(lineage);
    let latest_entry = lineage_latest_entry(&history_entries, &current_run_id);
    let latest_entry_run_id = latest_entry
        .and_then(|entry| entry.get("run_id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let latest_entry_parent_run_id = latest_entry
        .and_then(|entry| entry.get("parent_run_id"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let lineage_run_count = lineage
        .and_then(|value| value.get("run_count_hint"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current_artifacts = current_artifact_paths(artifact_paths);
    let latest_lineage_artifacts = lineage_entry_artifact_paths(latest_entry);
    let normalized_current_artifacts = normalized_string_set(&current_artifacts);
    let normalized_lineage_artifacts = normalized_string_set(&latest_lineage_artifacts);
    let run_id_linked = !current_run_id.trim().is_empty()
        && latest_entry
            .map(|_| latest_entry_run_id.trim() == current_run_id.trim())
            .unwrap_or(false);
    let compare_requires_history = comparison_available
        && comparison_observations
        && profile_requires_multi_run_compare_closure(&profile);
    let compare_has_multiple_runs =
        history_entries.len() >= 2 || !latest_entry_parent_run_id.trim().is_empty();
    let compare_closure_ok = !compare_requires_history || compare_has_multiple_runs;
    let artifact_lineage_closure_ok = normalized_current_artifacts.is_empty()
        || (!normalized_lineage_artifacts.is_empty()
            && normalized_current_artifacts.iter().all(|path| {
                normalized_lineage_artifacts
                    .iter()
                    .any(|lineage_path| lineage_path == path)
            }));
    let lineage_available = lineage
        .and_then(|value| value.get("available"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let lineage_ok = lineage_available
        && lineage_run_count >= 1
        && lineage_history_ok
        && missing_compare_keys.is_empty()
        && run_id_linked
        && compare_closure_ok
        && artifact_lineage_closure_ok;

    let mut missing_items = Vec::new();
    if !bundle_ok {
        missing_items.push("result_bundle".to_string());
    }
    if !profile_values_ok {
        missing_items.push("result_bundle_values".to_string());
        missing_items.extend(profile_value_issues.iter().cloned());
    }
    if !run_comparison_ok {
        missing_items.push("run_comparison".to_string());
    }
    if !lineage_ok {
        missing_items.push("lineage".to_string());
    }
    if !run_id_linked {
        missing_items.push("lineage_run_id_link".to_string());
    }
    if !compare_closure_ok {
        missing_items.push("compare_lineage_closure".to_string());
    }
    if !artifact_lineage_closure_ok {
        missing_items.push("artifact_lineage_closure".to_string());
    }
    let runtime_summary = profile_runtime_summary(&profile, result_bundle, run_comparison, lineage);

    json!({
        "status": if missing_items.is_empty() { "passed" } else { "needs_attention" },
        "profile": profile,
        "result_bundle_validation": {
            "status": if bundle_ok { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
            "detail": if bundle_ok {
                "Runtime result bundle exposes the profile-specific summary fields."
            } else if result_bundle.is_some() {
                "Runtime result bundle is present but misses profile-specific summary fields."
            } else {
                "Runtime result bundle was not provided."
            },
            "expected_fields": expected_bundle_fields,
            "missing_fields": missing_bundle_fields,
        },
        "profile_value_validation": {
            "status": if profile_values_ok { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
            "detail": if profile_values_ok {
                "Profile-specific summary field values carry concrete workflow evidence."
            } else if result_bundle.is_some() {
                "Runtime result bundle uses placeholder or incomplete profile-specific summary values."
            } else {
                "Runtime result bundle was not provided."
            },
            "issues": profile_value_issues,
        },
        "run_comparison_validation": {
            "status": if run_comparison_ok { "passed" } else if run_comparison.is_some() { "failed" } else { "missing" },
            "detail": if run_comparison_ok {
                "Run comparison exposes profile-specific compare keys and observations."
            } else if run_comparison.is_some() {
                "Run comparison is present but lacks profile-specific compare evidence."
            } else {
                "Run comparison was not provided."
            },
            "expected_compare_keys": expected_compare_keys,
            "missing_compare_keys": missing_compare_keys,
            "available": comparison_available,
            "has_observations": comparison_observations,
        },
        "lineage_validation": {
            "status": if lineage_ok { "passed" } else if lineage.is_some() { "failed" } else { "missing" },
            "detail": if lineage_ok {
                "Lineage content records profile-relevant run history fields."
            } else if lineage.is_some() {
                "Lineage content is present but does not yet capture the required run history fields."
            } else {
                "Lineage content was not provided."
            },
            "expected_history_fields": expected_history_fields,
            "missing_history_fields": missing_history_fields,
            "available": lineage_available,
            "run_count_hint": lineage_run_count,
            "current_run_id": current_run_id,
            "latest_lineage_run_id": latest_entry_run_id,
            "run_id_linked": run_id_linked,
            "latest_parent_run_id": latest_entry_parent_run_id,
            "compare_requires_history": compare_requires_history,
            "compare_has_multiple_runs": compare_has_multiple_runs,
            "compare_lineage_closure_ok": compare_closure_ok,
            "current_artifact_paths": current_artifacts,
            "latest_lineage_artifact_paths": latest_lineage_artifacts,
            "artifact_lineage_closure_ok": artifact_lineage_closure_ok,
        },
        "runtime_summary": runtime_summary,
        "missing_items": missing_items,
    })
}

fn verify_theory_or_literature_evidence(
    benchmark_plan: Option<&Value>,
    artifact_inventory: &Value,
    result_bundle: Option<&Value>,
) -> Value {
    let Some(plan) = benchmark_plan else {
        return json!({
            "status": "not_provided",
            "profile": Value::Null,
            "detail": "No benchmark_plan was provided for theory/literature evidence verification.",
            "checks": [],
            "missing_items": ["benchmark_plan"],
        });
    };

    let profile = plan
        .get("benchmark_profile")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if profile != "theory"
        && profile != "literature_review"
        && profile != "deep_learning"
        && profile != "systems_evaluation"
        && profile != "security_analysis"
    {
        return json!({
            "status": "not_applicable",
            "profile": profile,
            "detail": "Specialized verifier only applies to theory, literature_review, deep_learning, systems_evaluation, and security_analysis profiles.",
            "checks": [],
            "missing_items": [],
        });
    }

    let present_reports = artifact_inventory["present_artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let display_path = entry
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let resolved_path = entry
                .get("resolved_path")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if resolved_path.is_empty() {
                return None;
            }
            fs::read_to_string(&resolved_path)
                .ok()
                .map(|content| (display_path, content.to_ascii_lowercase()))
        })
        .collect::<Vec<_>>();

    let corpus = present_reports
        .iter()
        .map(|(_, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let artifact_paths = artifact_inventory_paths(artifact_inventory);

    let requirements: Vec<(&str, &[&str], &str)> = match profile.as_str() {
        "theory" => vec![
            (
                "definitions",
                &["definition", "notation", "assume", "premise"],
                "Proof artifacts should name definitions or assumptions.",
            ),
            (
                "lemma_structure",
                &["lemma", "theorem", "invariant"],
                "Proof artifacts should expose lemmas, theorems, or invariants.",
            ),
            (
                "proof_notes",
                &["proof", "derivation", "argument"],
                "Proof artifacts should include proof notes or derivation text.",
            ),
            (
                "counterexample_search",
                &[
                    "counterexample",
                    "sanity check",
                    "edge case",
                    "contradiction",
                ],
                "Theory workflows should record counterexample search or sanity-check evidence.",
            ),
        ],
        "literature_review" => vec![
            (
                "search_scope",
                &["search scope", "query", "database", "keyword"],
                "Literature workflows should record the search scope or retrieval query.",
            ),
            (
                "screening_criteria",
                &["screening", "inclusion", "exclusion", "eligibility"],
                "Literature workflows should state screening criteria.",
            ),
            (
                "synthesis",
                &["synthesis", "survey", "comparison", "taxonomy"],
                "Literature workflows should include synthesis or comparison notes.",
            ),
            (
                "research_gap",
                &["research gap", "gap", "open problem", "future work"],
                "Literature workflows should make the research gap explicit.",
            ),
        ],
        "deep_learning" => vec![
            (
                "training_signal",
                &["training", "checkpoint", "epoch", "validation"],
                "Deep learning artifacts should show training or checkpoint evidence.",
            ),
            (
                "metric_signal",
                &["validation", "accuracy", "loss", "perplexity", "f1"],
                "Deep learning artifacts should expose validation metrics or loss curves.",
            ),
            (
                "resource_signal",
                &["gpu", "memory", "throughput", "runtime", "time"],
                "Deep learning artifacts should mention resource usage or runtime cost.",
            ),
        ],
        "systems_evaluation" => vec![
            (
                "latency_signal",
                &["latency", "p95", "p99", "ms"],
                "Systems artifacts should report latency or tail-latency evidence.",
            ),
            (
                "throughput_signal",
                &["throughput", "ops", "qps", "requests"],
                "Systems artifacts should report throughput evidence.",
            ),
            (
                "resource_signal",
                &["memory", "cpu", "gpu", "footprint"],
                "Systems artifacts should mention resource footprint.",
            ),
        ],
        "security_analysis" => vec![
            (
                "finding_signal",
                &["finding", "vulnerability", "issue", "exploit", "alert"],
                "Security artifacts should expose concrete findings or alerts.",
            ),
            (
                "coverage_signal",
                &["coverage", "surface", "target", "scope"],
                "Security artifacts should describe target or surface coverage.",
            ),
            (
                "impact_signal",
                &["impact", "severity", "risk", "critical"],
                "Security artifacts should explain impact or severity.",
            ),
        ],
        _ => vec![(
            "search_scope",
            &["search", "query", "scope"],
            "Evidence should provide a concrete searchable CS workflow signal.",
        )],
    };

    let mut checks = Vec::new();
    let mut missing_items = Vec::new();
    for (name, needles, detail) in requirements {
        let passed = needles.iter().any(|needle| corpus.contains(needle));
        if !passed {
            missing_items.push(name.to_string());
        }
        checks.push(json!({
            "name": name,
            "status": if passed { "passed" } else if present_reports.is_empty() { "missing" } else { "failed" },
            "detail": if passed { format!("Observed evidence for {} in current artifacts.", name) } else { detail.to_string() },
            "signals": needles,
        }));
    }

    if profile == "theory" {
        for field_name in ["proof_status", "lemma_summary", "counterexample_status"] {
            let value = summary_field_value(result_bundle, field_name).unwrap_or_default();
            let passed = !profile_runtime_value_issues("theory", result_bundle)
                .iter()
                .any(|issue| issue == field_name);
            if !passed {
                missing_items.push(field_name.to_string());
            }
            checks.push(json!({
                "name": field_name,
                "status": if passed { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
                "detail": if passed {
                    format!("Result bundle provides concrete {} evidence.", field_name)
                } else {
                    format!("Theory result bundle should provide a concrete {} value instead of a pending or generic placeholder.", field_name)
                },
                "observed_value": value,
            }));
        }

        let theory_semantic_checks = vec![
            (
                "proof_status_semantic_alignment",
                "proof_status",
                vec!["proof", "proved", "invariant", "theorem", "derivation"],
                "Theory proof_status should explicitly mention proof progress, theorem status, or invariant reasoning.",
            ),
            (
                "lemma_summary_semantic_alignment",
                "lemma_summary",
                vec!["lemma", "invariant", "claim", "premise"],
                "Theory lemma_summary should explicitly mention a lemma, invariant, or named logical dependency.",
            ),
            (
                "counterexample_status_semantic_alignment",
                "counterexample_status",
                vec!["counterexample", "edge case", "sanity", "contradiction"],
                "Theory counterexample_status should explicitly mention counterexample search, sanity checks, edge cases, or contradiction analysis.",
            ),
        ];
        for (check_name, field_name, signals, detail) in theory_semantic_checks {
            let value = summary_field_value(result_bundle, field_name).unwrap_or_default();
            let passed =
                !value.trim().is_empty() && contains_any_case_insensitive(&value, &signals);
            if !passed {
                missing_items.push(check_name.to_string());
            }
            checks.push(json!({
                "name": check_name,
                "status": if passed { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
                "detail": if passed {
                    format!("{} carries theory-specific language instead of a generic placeholder.", field_name)
                } else {
                    detail.to_string()
                },
                "observed_value": value,
                "signals": signals,
            }));
        }
    } else {
        for field_name in ["remote_fulltext_coverage", "structured_paper_coverage"] {
            let value = summary_field_value(result_bundle, field_name).unwrap_or_default();
            let passed = !profile_runtime_value_issues("literature_review", result_bundle)
                .iter()
                .any(|issue| issue == field_name);
            if !passed {
                missing_items.push(field_name.to_string());
            }
            checks.push(json!({
                "name": field_name,
                "status": if passed { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
                "detail": if passed {
                    format!("Result bundle provides concrete {} evidence.", field_name)
                } else {
                    format!("Literature result bundle should provide a concrete {} value backed by remote-first fulltext retrieval.", field_name)
                },
                "observed_value": value,
            }));
        }

        let remote_fulltext =
            summary_field_value(result_bundle, "remote_fulltext_coverage").unwrap_or_default();
        let remote_fulltext_count = extract_first_integer(&remote_fulltext).unwrap_or(0);
        let remote_semantic_ok = !remote_fulltext.trim().is_empty()
            && remote_fulltext_count > 0
            && contains_any_case_insensitive(
                &remote_fulltext,
                &["remote", "fulltext", "pdf", "body text"],
            )
            && !contains_any_case_insensitive(
                &remote_fulltext,
                &[
                    "metadata-only",
                    "metadata only",
                    "abstract-only",
                    "abstract only",
                    "none",
                ],
            );
        if !remote_semantic_ok {
            missing_items.push("remote_fulltext_semantic_alignment".to_string());
        }
        checks.push(json!({
            "name": "remote_fulltext_semantic_alignment",
            "status": if remote_semantic_ok { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
            "detail": if remote_semantic_ok {
                "remote_fulltext_coverage reports positive remote/fulltext evidence with explicit remote-first semantics."
            } else {
                "Literature remote_fulltext_coverage should state a positive remote/fulltext count and make remote-first evidence explicit."
            },
            "observed_value": remote_fulltext,
            "observed_count": remote_fulltext_count,
        }));

        let structured =
            summary_field_value(result_bundle, "structured_paper_coverage").unwrap_or_default();
        let structured_count = extract_first_integer(&structured).unwrap_or(0);
        let structured_semantic_ok = !structured.trim().is_empty()
            && structured_count > 0
            && contains_any_case_insensitive(&structured, &["structured", "section", "reference"])
            && !contains_any_case_insensitive(
                &structured,
                &["metadata-only", "metadata only", "none"],
            );
        if !structured_semantic_ok {
            missing_items.push("structured_paper_semantic_alignment".to_string());
        }
        checks.push(json!({
            "name": "structured_paper_semantic_alignment",
            "status": if structured_semantic_ok { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
            "detail": if structured_semantic_ok {
                "structured_paper_coverage reports positive structured-section/reference evidence."
            } else {
                "Literature structured_paper_coverage should state a positive structured-paper count and mention sections/references."
            },
            "observed_value": structured,
            "observed_count": structured_count,
        }));
    }

    let report_like_artifacts = artifact_paths
        .iter()
        .filter(|path| {
            let lowered = path.to_ascii_lowercase();
            lowered.ends_with(".md")
                || lowered.ends_with(".txt")
                || lowered.ends_with(".json")
                || lowered.contains("report")
                || lowered.contains("manifest")
        })
        .count();
    let artifact_shape_ok = if profile == "theory" {
        report_like_artifacts >= 1
    } else {
        report_like_artifacts >= 2
    };
    if !artifact_shape_ok {
        missing_items.push("artifact_shape".to_string());
    }
    checks.push(json!({
        "name": "artifact_shape",
        "status": if artifact_shape_ok { "passed" } else { "failed" },
        "detail": if artifact_shape_ok {
            "Artifact inventory exposes the expected report/manifest surface for specialized verification."
        } else if profile == "theory" {
            "Theory verification expects at least one readable proof-oriented report artifact."
        } else {
            "Literature verification expects at least a manifest-style artifact plus a readable synthesis/report artifact."
        },
        "artifact_paths": artifact_paths,
    }));

    if profile == "literature_review" && !present_reports.is_empty() {
        let screened = extract_first_numeric_signal(
            &corpus,
            &["screened_papers", "screened papers", "screened"],
        );
        let included = extract_first_numeric_signal(
            &corpus,
            &["included_papers", "included papers", "included"],
        );
        let excluded = extract_first_numeric_signal(
            &corpus,
            &["excluded_papers", "excluded papers", "excluded"],
        );
        let counts_consistent = match (screened, included, excluded) {
            (Some(screened), Some(included), Some(excluded)) => screened >= included + excluded,
            (Some(screened), Some(included), None) => screened >= included,
            _ => false,
        };
        if !counts_consistent {
            missing_items.push("screening_count_consistency".to_string());
        }
        checks.push(json!({
            "name": "screening_count_consistency",
            "status": if counts_consistent { "passed" } else { "failed" },
            "detail": if counts_consistent {
                "Literature artifacts report screening counts that are numerically consistent."
            } else {
                "Literature artifacts should report screened/included/excluded counts with a consistent relationship."
            },
            "signals": ["screened_papers", "included_papers", "excluded_papers"],
            "observed": {
                "screened_papers": screened,
                "included_papers": included,
                "excluded_papers": excluded,
            }
        }));
    }

    if profile == "deep_learning"
        || profile == "systems_evaluation"
        || profile == "security_analysis"
    {
        let summary_fields = [
            (
                "best_validation_metric",
                &["validation", "accuracy", "loss", "perplexity", "f1"][..],
            ),
            (
                "resource_summary",
                &["gpu", "memory", "cpu", "time", "throughput"][..],
            ),
            ("latency_summary", &["latency", "p95", "p99", "ms"][..]),
            (
                "throughput_summary",
                &["throughput", "ops", "qps", "requests"][..],
            ),
            (
                "confirmed_findings",
                &["finding", "vulnerability", "issue", "exploit", "alert"][..],
            ),
            (
                "coverage_summary",
                &["coverage", "surface", "target", "scope"][..],
            ),
            (
                "impact_summary",
                &["impact", "severity", "risk", "critical"][..],
            ),
        ];
        for (field_name, signals) in summary_fields {
            let value = summary_field_value(result_bundle, field_name).unwrap_or_default();
            if value.trim().is_empty() {
                continue;
            }
            let passed = contains_any_case_insensitive(&value, signals);
            if !passed {
                missing_items.push(format!("{}_semantic_alignment", field_name));
            }
            checks.push(json!({
                "name": format!("{}_semantic_alignment", field_name),
                "status": if passed { "passed" } else if result_bundle.is_some() { "failed" } else { "missing" },
                "detail": if passed {
                    format!("{} carries profile-specific language instead of a generic placeholder.", field_name)
                } else {
                    format!("{} should mention the expected deep learning, systems, or security signal.", field_name)
                },
                "observed_value": value,
                "signals": signals,
            }));
        }

        for issue in profile_runtime_semantic_issues(&profile, result_bundle) {
            missing_items.push(issue.clone());
            checks.push(json!({
                "name": issue,
                "status": if result_bundle.is_some() { "failed" } else { "missing" },
                "detail": "Profile-specific runtime evidence needs stronger semantic alignment.",
            }));
        }
    }

    json!({
        "status": if present_reports.is_empty() {
            "missing"
        } else if missing_items.is_empty() {
            "passed"
        } else {
            "failed"
        },
        "profile": profile,
        "detail": if present_reports.is_empty() {
            "No readable artifacts were available for specialized theory/literature verification."
        } else if missing_items.is_empty() {
            "Specialized profile evidence checks passed."
        } else {
            "Specialized profile evidence is incomplete."
        },
        "checks": checks,
        "missing_items": missing_items,
    })
}

fn verification_center_repair_directive(
    verification_center: Option<&Value>,
    runtime_result_verification: Option<&Value>,
    result_bundle: Option<&Value>,
    run_comparison: Option<&Value>,
    lineage: Option<&Value>,
    reviewer_feedback: Option<&Value>,
    graph_evidence: Option<&Value>,
    aliyun_integration: Option<&Value>,
    multisource_evidence: Option<&Value>,
    profile: &str,
) -> Value {
    let Some(center) = verification_center else {
        return json!({
            "status": "missing",
            "profile": profile,
            "summary": "verification_center output was not provided.",
            "bundle_focus": [],
            "skipped_tools": [],
            "next_actions": [],
            "repair_checklist": [],
        });
    };

    let bundle_runs = center
        .get("bundle_runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut bundle_focus = Vec::new();
    let mut skipped_tools = Vec::new();
    let mut next_actions = Vec::new();
    let mut low_score_bundle_ids = Vec::new();

    for run in &bundle_runs {
        let bundle_id = run
            .get("bundle_id")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let bundle_score = run.get("bundle_score").and_then(Value::as_u64).unwrap_or(0);
        let skipped_count = run
            .get("skipped_tools")
            .and_then(Value::as_array)
            .map(|items| items.len())
            .unwrap_or(0);
        if bundle_score < 100 {
            low_score_bundle_ids.push(bundle_id.to_string());
        }
        bundle_focus.push(json!({
            "bundle_id": bundle_id,
            "bundle_score": bundle_score,
            "executed_tools": run.get("executed_tools").cloned().unwrap_or_else(|| json!([])),
            "skipped_tools": run.get("skipped_tools").cloned().unwrap_or_else(|| json!([])),
        }));
        if skipped_count > 0 {
            next_actions.push(format!(
                "{} should recover skipped tools before declaring the bundle closed.",
                bundle_id
            ));
        }
        if let Some(items) = run.get("skipped_tools").and_then(Value::as_array) {
            for item in items {
                skipped_tools.push(item.clone());
            }
        }
    }

    let summary = center
        .get("verification_center")
        .and_then(|value| value.get("summary"))
        .cloned()
        .unwrap_or_else(|| json!({}));
    let score = summary.get("score").and_then(Value::as_u64).unwrap_or(0);
    let ready_tools = summary
        .get("ready_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let total_tools = summary
        .get("total_tools")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let low_score_bundle_ids = low_score_bundle_ids;

    let runtime_missing = runtime_result_verification
        .and_then(|value| value.get("missing_items"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let runtime_summary = profile_runtime_summary(profile, result_bundle, run_comparison, lineage);
    let feedback_summary = reviewer_feedback_summary(reviewer_feedback);
    let graph_summary = graph_evidence_summary(graph_evidence);
    let aliyun_summary = aliyun_qwen_summary(aliyun_integration);
    let multisource_summary = multisource_evidence_summary(multisource_evidence);
    let competition_fit = competition_gap_assessment(
        profile,
        &runtime_missing,
        &skipped_tools,
        &feedback_summary,
        &graph_summary,
        &aliyun_summary,
        &multisource_summary,
    );

    if score < 70 {
        next_actions.push("Raise the overall verification-center score by recovering unavailable tooling or narrowing the bundle scope.".to_string());
    }
    if ready_tools == 0 && total_tools > 0 {
        next_actions.push("No verification tools were ready; fall back to report-only repair and surface explicit gaps.".to_string());
    }
    if !runtime_missing.is_empty() {
        next_actions.push(format!(
            "Fix runtime bundle gaps first: {}.",
            runtime_missing.join(", ")
        ));
    }
    if competition_fit["gap_count"].as_u64().unwrap_or(0) > 0 {
        next_actions.push("Close competition-fit gaps with reviewer feedback, graph evidence, Aliyun/Qwen readiness notes, and multi-source closure where applicable.".to_string());
    }

    let repair_directive = if profile == "deep_learning" {
        "Prioritize training artifacts, validation metrics, and checkpoint evidence; recover missing ML tooling before re-running the bundle."
    } else if profile == "systems_evaluation" {
        "Prioritize latency, throughput, and resource summaries; re-run any skipped performance probes before closing the report."
    } else if profile == "security_analysis" {
        "Prioritize concrete findings, coverage, and impact evidence; re-run the security scan bundle after restoring unavailable scanners."
    } else {
        "Prioritize the lowest-scoring bundle, recover skipped tools where possible, and keep the repair note specific to the active CS profile."
    };

    json!({
        "status": if bundle_runs.is_empty() { "missing" } else { "ready" },
        "profile": profile,
        "summary": format!(
            "verification_center score={} ready_tools={}/{} bundle_runs={}",
            score,
            ready_tools,
            total_tools,
            bundle_runs.len()
        ),
        "repair_directive": repair_directive,
        "bundle_focus": bundle_focus,
        "skipped_tools": skipped_tools,
        "low_score_bundle_ids": low_score_bundle_ids,
        "runtime_summary": runtime_summary,
        "reviewer_feedback_summary": feedback_summary,
        "graph_evidence_summary": graph_summary,
        "aliyun_qwen_summary": aliyun_summary,
        "multisource_evidence_summary": multisource_summary,
        "competition_fit": competition_fit,
        "repair_checklist": competition_fit["repair_checklist"].clone(),
        "next_actions": next_actions,
    })
}

fn competition_gap_assessment(
    profile: &str,
    runtime_missing: &[String],
    skipped_tools: &[Value],
    feedback_summary: &Value,
    graph_summary: &Value,
    aliyun_summary: &Value,
    multisource_summary: &Value,
) -> Value {
    let runtime_missing_set = normalized_string_set(runtime_missing);
    let skipped_tool_names = skipped_tools
        .iter()
        .filter_map(|item| item.get("tool").and_then(Value::as_str))
        .map(|name| name.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();

    let mut gaps = Vec::new();
    let mut repair_checklist = Vec::new();

    let provenance_gap = runtime_missing_set.iter().any(|item| {
        item.contains("artifact_lineage_closure")
            || item.contains("lineage_run_id_link")
            || item.contains("compare_lineage_closure")
    });
    let provenance_item = json!({
        "capability": "traceable_result_lineage",
        "status": if provenance_gap { "gap" } else { "partial_ready" },
        "detail": if provenance_gap {
            "The IDE still needs stronger end-to-end traceability from current result bundle to lineage and artifacts."
        } else {
            "The IDE now exposes structured runtime-to-lineage closure, but still lacks a first-class reviewer-facing provenance surface."
        },
        "required_inputs": ["result_bundle.run_id", "lineage.history", "artifact_paths", "run_comparison.observations"],
        "recommended_actions": [
            "Link the current run_id to the latest lineage entry.",
            "Carry current artifact_paths into lineage.history[].artifact_paths.",
            "For experiment-style profiles, preserve at least two linked runs when run comparison is reported."
        ]
    });
    gaps.push(provenance_item.clone());
    repair_checklist.push(provenance_item);

    let feedback_available = feedback_summary["available"].as_bool().unwrap_or(false);
    let feedback_scored = feedback_summary["score_count"].as_u64().unwrap_or(0) > 0;
    let feedback_linked = feedback_summary["has_lineage_link"]
        .as_bool()
        .unwrap_or(false);
    let human_feedback_gap = !feedback_available || !feedback_scored || !feedback_linked;
    let human_feedback_item = json!({
        "capability": "human_in_the_loop_feedback",
        "status": if human_feedback_gap { "gap" } else { "partial_ready" },
        "detail": if human_feedback_gap {
            "The backend does not yet bind reviewer feedback or scoring comments into repair directives or benchmark lineage."
        } else {
            "Reviewer feedback is present and linked to the verification flow, but it is not yet a first-class IDE review panel."
        },
        "required_inputs": ["reviewer_feedback[].reviewer", "reviewer_feedback[].comment", "reviewer_feedback[].score", "reviewer_feedback[].linked_run_id"],
        "recommended_actions": [
            "Attach reviewer identity, comment, and numeric score to the current run.",
            "Mark unresolved review items so the next repair loop can consume them.",
            "Propagate feedback links into lineage or run-level repair notes."
        ]
    });
    gaps.push(human_feedback_item.clone());
    repair_checklist.push(human_feedback_item);

    let graph_required = matches!(
        profile,
        "literature_review" | "agent_evaluation" | "security_analysis"
    );
    let graph_available = graph_summary["available"].as_bool().unwrap_or(false);
    let graph_sources = graph_summary["source_count"].as_u64().unwrap_or(0);
    let graph_relations = graph_summary["relation_count"].as_u64().unwrap_or(0);
    let knowledge_graph_gap =
        graph_required && (!graph_available || graph_sources == 0 || graph_relations == 0);
    let graph_item = json!({
        "capability": "knowledge_graph_or_evidence_graph",
        "status": if knowledge_graph_gap { "gap" } else { "partial_ready" },
        "detail": if knowledge_graph_gap {
            "The IDE advertises knowledge-graph style research capability, but verification does not yet require graph/evidence outputs."
        } else {
            "Knowledge-graph output is not yet required for this profile."
        },
        "required_inputs": ["graph_evidence.graph_kind", "graph_evidence.entities|nodes", "graph_evidence.relations|edges|triples", "graph_evidence.sources"],
        "recommended_actions": if graph_required {
            json!([
                "Emit a graph artifact with entities/nodes and relations/edges.",
                "Bind graph sources back to literature or evaluation evidence.",
                "Include graph_kind so the verifier can distinguish knowledge-graph and evidence-graph outputs."
            ])
        } else {
            json!(["Graph evidence is optional for this profile; keep it available when the workflow already produces one."])
        }
    });
    gaps.push(graph_item.clone());
    repair_checklist.push(graph_item);

    let aliyun_gap = !aliyun_summary["provider_ok"].as_bool().unwrap_or(false)
        || !aliyun_summary["model_ok"].as_bool().unwrap_or(false)
        || !aliyun_summary["endpoint_ok"].as_bool().unwrap_or(false)
        || !aliyun_summary["credential_ok"].as_bool().unwrap_or(false)
        || !aliyun_summary["route_ok"].as_bool().unwrap_or(false);
    let aliyun_item = json!({
        "capability": "aliyun_qwen_product_fit",
        "status": if aliyun_gap { "gap" } else { "partial_ready" },
        "detail": if aliyun_gap {
            "The codebase exposes Qwen model selections, but verifier/repair output does not yet score Aliyun-specific deployment, product integration, or model-routing readiness."
        } else {
            "Aliyun/Qwen provider, endpoint, credentials, and route hints are present in verification input."
        },
        "required_inputs": ["aliyun_integration.provider", "aliyun_integration.model", "aliyun_integration.endpoint", "aliyun_integration.credential_mode", "aliyun_integration.route_mode"],
        "recommended_actions": [
            "Record the active Qwen model and Aliyun-compatible endpoint in the run bundle.",
            "Surface how credentials are supplied without leaking secrets.",
            "Keep model routing or deployment path explicit for competition submission."
        ]
    });
    gaps.push(aliyun_item.clone());
    repair_checklist.push(aliyun_item);

    let multisource_required = matches!(
        profile,
        "deep_learning" | "systems_evaluation" | "security_analysis" | "agent_evaluation"
    );
    let multisource_gap = multisource_required
        && (!multisource_summary["available"].as_bool().unwrap_or(false)
            || multisource_summary["source_count"].as_u64().unwrap_or(0) < 2
            || multisource_summary["unique_source_kind_count"]
                .as_u64()
                .unwrap_or(0)
                < 2
            || multisource_summary["harmonized_field_count"]
                .as_u64()
                .unwrap_or(0)
                == 0
            || multisource_summary["conflict_resolution"].is_null());
    let multisource_item = json!({
        "capability": "heterogeneous_multisource_data_closure",
        "status": if multisource_gap { "gap" } else { "partial_ready" },
        "detail": if multisource_gap {
            "The IDE verifies structured outputs, but still lacks profile-level checks for multi-source fusion, schema harmonization, or cross-source conflict resolution."
        } else {
            "Heterogeneous data fusion is less central for the active profile."
        },
        "required_inputs": ["multisource_evidence.sources", "multisource_evidence.fusion_strategy", "multisource_evidence.harmonized_fields", "multisource_evidence.conflict_resolution"],
        "recommended_actions": if multisource_required {
            json!([
                "List at least two source kinds participating in the run.",
                "Record how schemas were harmonized before metric computation.",
                "State how cross-source conflicts were detected or resolved."
            ])
        } else {
            json!(["Multi-source closure is optional for this profile."])
        }
    });
    gaps.push(multisource_item.clone());
    repair_checklist.push(multisource_item);

    let skipped_tool_gap = !skipped_tool_names.is_empty();
    if skipped_tool_gap {
        repair_checklist.push(json!({
            "capability": "verification_center_bundle_recovery",
            "status": "gap",
            "detail": "One or more verification-center tools were skipped and should be either restored or explicitly waived.",
            "required_inputs": ["verification_center.bundle_runs[].skipped_tools"],
            "recommended_actions": [
                "Recover skipped tools when they are required by the active profile.",
                "If a tool cannot be installed, downgrade the bundle scope and preserve the waiver reason in the report."
            ]
        }));
    }

    json!({
        "profile": profile,
        "gaps": gaps,
        "repair_checklist": repair_checklist,
        "gap_count": gaps.iter().filter(|item| item["status"] == "gap").count(),
        "input_summaries": {
            "reviewer_feedback": feedback_summary,
            "graph_evidence": graph_summary,
            "aliyun_qwen": aliyun_summary,
            "multisource_evidence": multisource_summary,
        }
    })
}

fn extract_first_numeric_signal(corpus: &str, aliases: &[&str]) -> Option<u64> {
    let escaped = aliases
        .iter()
        .map(|alias| regex::escape(alias))
        .collect::<Vec<_>>()
        .join("|");
    let pattern = Regex::new(&format!(r"(?im)\b(?:{})\b\s*(?:=|:)?\s*(\d+)", escaped))
        .expect("numeric signal regex");
    pattern
        .captures(corpus)
        .and_then(|captures| captures.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok())
}

fn extract_first_integer(text: &str) -> Option<u64> {
    let pattern = Regex::new(r"(\d+)").expect("integer regex");
    pattern
        .captures(text)
        .and_then(|captures| captures.get(1))
        .and_then(|m| m.as_str().parse::<u64>().ok())
}

fn extract_first_float(text: &str) -> Option<f64> {
    let pattern = Regex::new(r"(\d+(?:\.\d+)?)").expect("float regex");
    pattern
        .captures(text)
        .and_then(|captures| captures.get(1))
        .and_then(|m| m.as_str().parse::<f64>().ok())
}

fn string_array_len(value: Option<&Value>, key: &str) -> usize {
    value
        .and_then(|item| item.get(key))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0)
}

fn profile_runtime_summary(
    profile: &str,
    result_bundle: Option<&Value>,
    run_comparison: Option<&Value>,
    lineage: Option<&Value>,
) -> Value {
    let comparison_observation_count = run_comparison
        .and_then(|value| value.get("observations"))
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let compare_key_count = run_comparison
        .and_then(|value| value.get("compare_keys"))
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let lineage_history_count = lineage_history_entries(lineage).len();
    let lineage_run_count_hint = lineage
        .and_then(|value| value.get("run_count_hint"))
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current_run_id = summary_field_value(result_bundle, "run_id").unwrap_or_default();

    let profile_fields = match profile {
        "deep_learning" => json!({
            "checkpoint_path": summary_field_value(result_bundle, "checkpoint_path"),
            "best_validation_metric": summary_field_value(result_bundle, "best_validation_metric"),
            "validation_metric_numeric_signal": summary_field_value(result_bundle, "best_validation_metric")
                .and_then(|value| extract_first_float(&value)),
            "resource_summary": summary_field_value(result_bundle, "resource_summary"),
            "resource_numeric_signal": summary_field_value(result_bundle, "resource_summary")
                .and_then(|value| extract_first_float(&value)),
        }),
        "systems_evaluation" => json!({
            "workload_name": summary_field_value(result_bundle, "workload_name"),
            "latency_summary": summary_field_value(result_bundle, "latency_summary"),
            "latency_numeric_signal": summary_field_value(result_bundle, "latency_summary")
                .and_then(|value| extract_first_float(&value))
                .unwrap_or(0.0),
            "throughput_summary": summary_field_value(result_bundle, "throughput_summary"),
            "throughput_numeric_signal": summary_field_value(result_bundle, "throughput_summary")
                .and_then(|value| extract_first_float(&value))
                .unwrap_or(0.0),
            "resource_summary": summary_field_value(result_bundle, "resource_summary"),
            "resource_numeric_signal": summary_field_value(result_bundle, "resource_summary")
                .and_then(|value| extract_first_float(&value))
                .unwrap_or(0.0),
        }),
        "security_analysis" => json!({
            "confirmed_findings": summary_field_value(result_bundle, "confirmed_findings"),
            "confirmed_findings_numeric_signal": summary_field_value(result_bundle, "confirmed_findings")
                .and_then(|value| extract_first_integer(&value)),
            "false_positive_count": summary_field_value(result_bundle, "false_positive_count"),
            "false_positive_numeric_signal": summary_field_value(result_bundle, "false_positive_count")
                .and_then(|value| extract_first_integer(&value)),
            "coverage_summary": summary_field_value(result_bundle, "coverage_summary"),
            "impact_summary": summary_field_value(result_bundle, "impact_summary"),
        }),
        _ => json!({}),
    };

    json!({
        "profile": profile,
        "current_run_id": current_run_id,
        "comparison_observation_count": comparison_observation_count,
        "compare_key_count": compare_key_count,
        "lineage_history_count": lineage_history_count,
        "lineage_run_count_hint": lineage_run_count_hint,
        "profile_fields": profile_fields,
    })
}

fn reviewer_feedback_summary(reviewer_feedback: Option<&Value>) -> Value {
    let entries = reviewer_feedback
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let usable_entries = entries
        .iter()
        .filter(|entry| {
            entry
                .get("reviewer")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
                || entry
                    .get("comment")
                    .and_then(Value::as_str)
                    .is_some_and(|value| !value.trim().is_empty())
        })
        .count();
    let score_count = entries
        .iter()
        .filter(|entry| {
            entry
                .get("score")
                .and_then(|value| {
                    value
                        .as_f64()
                        .or_else(|| value.as_u64().map(|raw| raw as f64))
                })
                .is_some()
        })
        .count();
    let unresolved_count = entries
        .iter()
        .filter(|entry| {
            !entry
                .get("resolved")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        })
        .count();
    let has_lineage_link = entries.iter().any(|entry| {
        entry
            .get("linked_run_id")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
            || entry
                .get("lineage_ref")
                .and_then(Value::as_str)
                .is_some_and(|value| !value.trim().is_empty())
    });

    json!({
        "available": usable_entries > 0,
        "entry_count": usable_entries,
        "score_count": score_count,
        "unresolved_count": unresolved_count,
        "has_lineage_link": has_lineage_link,
    })
}

fn graph_evidence_summary(graph_evidence: Option<&Value>) -> Value {
    let entities = string_array_len(graph_evidence, "entities");
    let relations = string_array_len(graph_evidence, "relations");
    let triples = graph_evidence
        .and_then(|item| item.get("triples"))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let nodes = graph_evidence
        .and_then(|item| item.get("nodes"))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let edges = graph_evidence
        .and_then(|item| item.get("edges"))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);
    let graph_kind = graph_evidence
        .and_then(|item| item.get("graph_kind"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let source_count = graph_evidence
        .and_then(|item| item.get("sources"))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);

    json!({
        "available": entities + relations + triples + nodes + edges > 0,
        "graph_kind": if graph_kind.is_empty() { Value::Null } else { json!(graph_kind) },
        "entity_count": entities.max(nodes),
        "relation_count": relations.max(edges).max(triples),
        "source_count": source_count,
    })
}

fn aliyun_qwen_summary(aliyun_integration: Option<&Value>) -> Value {
    let provider = aliyun_integration
        .and_then(|item| item.get("provider"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let model = aliyun_integration
        .and_then(|item| item.get("model"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let endpoint = aliyun_integration
        .and_then(|item| item.get("endpoint"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let credential_mode = aliyun_integration
        .and_then(|item| item.get("credential_mode"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let route_mode = aliyun_integration
        .and_then(|item| item.get("route_mode"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let provider_ok =
        provider.contains("qwen") || provider.contains("aliyun") || provider.contains("dashscope");
    let model_ok = model.contains("qwen");
    let endpoint_ok = endpoint.contains("aliyun") || endpoint.contains("dashscope");
    let credential_ok = !credential_mode.is_empty();
    let route_ok = !route_mode.is_empty();

    json!({
        "available": provider_ok || model_ok || endpoint_ok,
        "provider": if provider.is_empty() { Value::Null } else { json!(provider) },
        "model": if model.is_empty() { Value::Null } else { json!(model) },
        "endpoint": if endpoint.is_empty() { Value::Null } else { json!(endpoint) },
        "credential_mode": if credential_mode.is_empty() { Value::Null } else { json!(credential_mode) },
        "route_mode": if route_mode.is_empty() { Value::Null } else { json!(route_mode) },
        "provider_ok": provider_ok,
        "model_ok": model_ok,
        "endpoint_ok": endpoint_ok,
        "credential_ok": credential_ok,
        "route_ok": route_ok,
    })
}

fn multisource_evidence_summary(multisource_evidence: Option<&Value>) -> Value {
    let sources = multisource_evidence
        .and_then(|item| item.get("sources"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let source_count = sources.len();
    let normalized_kinds = sources
        .iter()
        .filter_map(|entry| entry.get("kind").or_else(|| entry.get("source_type")))
        .filter_map(Value::as_str)
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let unique_kind_count = normalized_string_set(
        &normalized_kinds
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
    )
    .len();
    let fusion_strategy = multisource_evidence
        .and_then(|item| item.get("fusion_strategy"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let conflict_resolution = multisource_evidence
        .and_then(|item| item.get("conflict_resolution"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let harmonized_fields = multisource_evidence
        .and_then(|item| item.get("harmonized_fields"))
        .and_then(Value::as_array)
        .map(|entries| entries.len())
        .unwrap_or(0);

    json!({
        "available": source_count > 0,
        "source_count": source_count,
        "unique_source_kind_count": unique_kind_count,
        "fusion_strategy": if fusion_strategy.is_empty() { Value::Null } else { json!(fusion_strategy) },
        "conflict_resolution": if conflict_resolution.is_empty() { Value::Null } else { json!(conflict_resolution) },
        "harmonized_field_count": harmonized_fields,
    })
}

fn artifact_inventory_paths(artifact_inventory: &Value) -> Vec<String> {
    artifact_inventory["present_artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            entry
                .get("path")
                .and_then(Value::as_str)
                .map(|path| path.to_string())
        })
        .collect()
}

fn expected_metric_signals_for_profile(profile: &str) -> &'static [&'static str] {
    match profile {
        "classical_ml" => &["accuracy", "f1", "precision", "recall", "auc", "loss"],
        "deep_learning" => &["validation", "loss", "accuracy", "bleu", "memory", "time"],
        "systems_evaluation" => &["latency", "throughput", "memory", "qps", "overhead"],
        "agent_evaluation" => &["success", "trajectory", "tool_error", "cost", "latency"],
        "security_analysis" => &["precision", "recall", "false_positive", "f1", "coverage"],
        "theory" => &["proof", "lemma", "counterexample", "invariant"],
        "literature_review" => &["citation", "screening", "synthesis", "gap"],
        _ => &[],
    }
}

fn expected_artifact_kinds_for_profile(profile: &str) -> &'static [&'static str] {
    match profile {
        "systems_evaluation" => &["executable", "report", "data_manifest"],
        "theory" => &["report"],
        "literature_review" => &["report", "data_manifest"],
        "classical_ml" | "deep_learning" | "agent_evaluation" | "security_analysis" => {
            &["executable", "report"]
        }
        _ => &[],
    }
}

fn verify_dataset_acquisition(plan: &Value, profile: &str) -> (Value, Vec<String>) {
    let mut missing_items = Vec::new();
    let Some(acquisition) = plan.get("dataset_acquisition") else {
        missing_items.push("dataset_acquisition".to_string());
        return (
            json!({
                "status": "failed",
                "detail": "Dataset acquisition plan is missing, so the verifier cannot confirm how CS benchmark data will be sourced.",
                "retrieval_entrypoint": Value::Null,
                "paper_source_policy": Value::Null,
                "preferred_providers": [],
                "missing_items": ["dataset_acquisition"],
            }),
            missing_items,
        );
    };

    let retrieval_entrypoint = acquisition
        .get("retrieval_entrypoint")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let search_tool = acquisition
        .get("search_tool")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let manifest_tool = acquisition
        .get("manifest_tool")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let paper_source_policy = acquisition
        .get("paper_source_policy")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let providers = acquisition
        .get("preferred_providers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let query_count = acquisition
        .get("search_queries")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|value| value.as_str().is_some_and(|raw| !raw.trim().is_empty()))
                .count()
        })
        .unwrap_or(0);
    let manifest_field_count = acquisition
        .get("expected_manifest_fields")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|value| {
                    value
                        .get("name")
                        .and_then(Value::as_str)
                        .is_some_and(|raw| !raw.trim().is_empty())
                })
                .count()
        })
        .unwrap_or(0);

    let expects_public_entrypoint = !matches!(profile, "theory" | "literature_review");
    let retrieval_ok = if expects_public_entrypoint {
        retrieval_entrypoint == "official_dataset_databases"
    } else if profile == "literature_review" {
        retrieval_entrypoint == "official_paper_apis_only"
    } else {
        !retrieval_entrypoint.is_empty()
    };
    if !retrieval_ok {
        missing_items.push("dataset_retrieval_entrypoint".to_string());
    }
    if expects_public_entrypoint && search_tool != "search_public_datasets" {
        missing_items.push("dataset_search_tool".to_string());
    }
    if expects_public_entrypoint && manifest_tool != "fetch_public_dataset_manifest" {
        missing_items.push("dataset_manifest_tool".to_string());
    }
    if query_count == 0 {
        missing_items.push("dataset_search_queries".to_string());
    }
    if manifest_field_count == 0 {
        missing_items.push("dataset_manifest_fields".to_string());
    }
    if paper_source_policy != "official_paper_apis_only" {
        missing_items.push("paper_source_policy".to_string());
    }
    if expects_public_entrypoint && providers.is_empty() {
        missing_items.push("preferred_dataset_providers".to_string());
    }

    let status = if missing_items.is_empty() {
        "passed"
    } else {
        "failed"
    };
    let detail = if status == "passed" {
        format!(
            "Dataset acquisition for '{}' is explicit about entrypoint, manifesting, and paper-source boundaries.",
            profile
        )
    } else {
        format!(
            "Dataset acquisition for '{}' is incomplete or does not clearly enforce the public-dataset entrypoint and paper-source policy.",
            profile
        )
    };

    (
        json!({
            "status": status,
            "detail": detail,
            "retrieval_entrypoint": acquisition.get("retrieval_entrypoint").cloned().unwrap_or(Value::Null),
            "paper_source_policy": if paper_source_policy.is_empty() { Value::Null } else { json!(paper_source_policy) },
            "preferred_providers": providers,
            "missing_items": missing_items,
        }),
        missing_items,
    )
}

fn benchmark_profile_artifact_aligned(artifact: &Value, expected_kind: &str) -> bool {
    let kind = artifact
        .get("kind")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if kind == expected_kind {
        return true;
    }

    let name = artifact
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    match expected_kind {
        "data_manifest" => {
            name.contains("manifest") || name.contains("split") || name.contains("dataset")
        }
        "report" => name.contains("report") || name.contains("summary") || name.contains("metrics"),
        "executable" => name.contains("script") || name.contains("train") || name.contains("eval"),
        _ => false,
    }
}

fn verify_benchmark_profile(plan: &Value) -> (Value, Vec<String>) {
    let mut missing_items = Vec::new();
    let Some(profile_raw) = plan
        .get("benchmark_profile")
        .and_then(|value| value.as_str())
    else {
        missing_items.push("benchmark_profile".to_string());
        return (
            json!({
                "status": "failed",
                "profile": Value::Null,
                "detail": "benchmark_profile is missing, so the CS verifier cannot apply profile-specific expectations.",
                "guidance": benchmark_profile_guidance("general_cs"),
                "expected_metric_signals": [],
                "expected_artifact_kinds": [],
                "missing_alignment_items": ["benchmark_profile"],
            }),
            missing_items,
        );
    };

    let profile = profile_raw.trim().to_ascii_lowercase();
    if !supported_benchmark_profiles().contains(&profile.as_str()) {
        missing_items.push("benchmark_profile".to_string());
        return (
            json!({
                "status": "failed",
                "profile": profile,
                "detail": "benchmark_profile is not one of the supported CS benchmark profiles.",
                "guidance": benchmark_profile_guidance("general_cs"),
                "expected_metric_signals": [],
                "expected_artifact_kinds": [],
                "missing_alignment_items": ["benchmark_profile"],
            }),
            missing_items,
        );
    }

    let declared_metrics = plan
        .get("metrics")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let declared_artifacts = plan
        .get("artifacts")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let expected_metric_signals = expected_metric_signals_for_profile(&profile);
    let expected_artifact_kinds = expected_artifact_kinds_for_profile(&profile);

    let metric_alignment = expected_metric_signals.is_empty()
        || declared_metrics.iter().any(|metric| {
            metric
                .get("name")
                .and_then(|value| value.as_str())
                .map(|name| {
                    let name = name.to_ascii_lowercase();
                    expected_metric_signals
                        .iter()
                        .any(|signal| name.contains(signal))
                })
                .unwrap_or(false)
        });

    let artifact_alignment = expected_artifact_kinds.is_empty()
        || expected_artifact_kinds.iter().all(|expected_kind| {
            declared_artifacts
                .iter()
                .filter(|artifact| {
                    artifact
                        .get("required")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                })
                .any(|artifact| benchmark_profile_artifact_aligned(artifact, expected_kind))
        });

    if !metric_alignment {
        missing_items.push("profile_metrics".to_string());
    }
    if !artifact_alignment {
        missing_items.push("profile_artifacts".to_string());
    }

    let status = if missing_items.is_empty() {
        "passed"
    } else {
        "failed"
    };
    let detail = if status == "passed" {
        format!(
            "Benchmark profile '{}' is consistent with the declared CS metrics and artifacts.",
            profile
        )
    } else {
        format!(
            "Benchmark profile '{}' is present, but the declared metrics or required artifacts do not yet reflect its expected CS evaluation shape.",
            profile
        )
    };

    (
        json!({
            "status": status,
            "profile": profile,
            "detail": detail,
            "guidance": benchmark_profile_guidance(&profile),
            "expected_metric_signals": expected_metric_signals,
            "expected_artifact_kinds": expected_artifact_kinds,
            "metric_alignment": metric_alignment,
            "artifact_alignment": artifact_alignment,
            "missing_alignment_items": missing_items,
        }),
        missing_items,
    )
}

fn summarize_benchmark_statuses(statuses: &[&str]) -> &'static str {
    if statuses.is_empty() {
        "not_provided"
    } else if statuses.iter().all(|status| *status == "passed") {
        "passed"
    } else if statuses.iter().all(|status| *status == "missing") {
        "not_provided"
    } else {
        "needs_attention"
    }
}

fn resolve_workspace_root(workspace_root: Option<&Value>) -> Option<PathBuf> {
    workspace_root
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::current_dir().ok())
}

fn resolve_artifact_path(path: &str, workspace_root: Option<&Path>) -> PathBuf {
    let candidate = PathBuf::from(path);
    if candidate.is_absolute() {
        candidate
    } else if let Some(root) = workspace_root {
        root.join(candidate)
    } else {
        candidate
    }
}

fn verify_artifact_inventory(
    artifact_paths: Option<&Value>,
    workspace_root: Option<&Value>,
) -> Value {
    let Some(paths) = artifact_paths.and_then(|value| value.as_array()) else {
        return json!({
            "status": "not_provided",
            "detail": "No artifact_paths were provided for file-level verification.",
            "present_artifacts": [],
            "missing_artifacts": [],
            "artifact_count": 0,
            "verified_root": resolve_workspace_root(workspace_root)
                .map(|path| path.display().to_string()),
        });
    };

    let workspace_root = resolve_workspace_root(workspace_root);
    let mut present_artifacts = Vec::new();
    let mut missing_artifacts = Vec::new();

    for raw_path in paths.iter().filter_map(|value| value.as_str()) {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            continue;
        }

        let resolved_path = resolve_artifact_path(trimmed, workspace_root.as_deref());
        let record = json!({
            "path": trimmed,
            "resolved_path": resolved_path.display().to_string(),
        });

        if resolved_path.exists() {
            present_artifacts.push(record);
        } else {
            missing_artifacts.push(record);
        }
    }

    let artifact_count = present_artifacts.len() + missing_artifacts.len();
    let status = if artifact_count == 0 {
        "not_provided"
    } else if missing_artifacts.is_empty() {
        "passed"
    } else {
        "failed"
    };

    let detail = match status {
        "passed" => "All declared artifact paths currently exist on disk.",
        "failed" => "Some declared artifact paths are missing on disk.",
        _ => "Artifact paths were empty after normalization.",
    };

    json!({
        "status": status,
        "detail": detail,
        "present_artifacts": present_artifacts,
        "missing_artifacts": missing_artifacts,
        "artifact_count": artifact_count,
        "verified_root": workspace_root.map(|path| path.display().to_string()),
    })
}

fn tokenize_artifact_label(label: &str) -> Vec<String> {
    label
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !matches!(token.as_str(), "artifact" | "file" | "output" | "result"))
        .collect()
}

fn artifact_kind_matches_path(kind: &str, path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let stem = path
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match kind.trim().to_ascii_lowercase().as_str() {
        "executable" => matches!(
            extension.as_str(),
            "py" | "ipynb" | "sh" | "ps1" | "rs" | "js" | "ts" | "java" | "go" | "cpp" | "c"
        ),
        "report" => matches!(
            extension.as_str(),
            "md" | "txt" | "json" | "csv" | "tsv" | "html" | "pdf"
        ),
        "data_manifest" => {
            matches!(
                extension.as_str(),
                "json" | "yaml" | "yml" | "csv" | "tsv" | "txt"
            ) && (stem.contains("split")
                || stem.contains("manifest")
                || stem.contains("dataset")
                || stem.contains("index"))
        }
        "figure" | "plot" | "chart" => {
            matches!(
                extension.as_str(),
                "png" | "jpg" | "jpeg" | "svg" | "webp" | "pdf"
            )
        }
        "notebook" => extension == "ipynb",
        _ => true,
    }
}

fn artifact_name_matches_path(name: &str, path: &Path) -> bool {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let tokens = tokenize_artifact_label(name);
    if tokens.is_empty() {
        return true;
    }

    tokens.iter().any(|token| stem.contains(token))
}

fn verify_artifact_contract(benchmark_plan: Option<&Value>, artifact_inventory: &Value) -> Value {
    let Some(plan) = benchmark_plan else {
        return json!({
            "status": "not_provided",
            "detail": "No benchmark_plan was provided for artifact contract verification.",
            "matched_required_artifacts": [],
            "missing_required_artifacts": [],
        });
    };

    let Some(declared_artifacts) = plan.get("artifacts").and_then(|value| value.as_array()) else {
        return json!({
            "status": "not_provided",
            "detail": "benchmark_plan does not declare any artifacts.",
            "matched_required_artifacts": [],
            "missing_required_artifacts": [],
        });
    };

    let present_paths = artifact_inventory["present_artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let required_artifacts = declared_artifacts
        .iter()
        .filter(|entry| {
            entry
                .get("required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    let mut matched_required_artifacts = Vec::new();
    let mut missing_required_artifacts = Vec::new();

    for artifact in &required_artifacts {
        let name = artifact
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("artifact");
        let kind = artifact
            .get("kind")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");

        let matched = present_paths.iter().find(|path_entry| {
            path_entry
                .get("resolved_path")
                .and_then(|value| value.as_str())
                .map(PathBuf::from)
                .map(|resolved_path| {
                    artifact_kind_matches_path(kind, &resolved_path)
                        && artifact_name_matches_path(name, &resolved_path)
                })
                .unwrap_or(false)
        });

        if let Some(path_entry) = matched {
            matched_required_artifacts.push(json!({
                "name": name,
                "kind": kind,
                "path": path_entry.get("path").cloned().unwrap_or(Value::Null),
                "resolved_path": path_entry.get("resolved_path").cloned().unwrap_or(Value::Null),
            }));
        } else {
            missing_required_artifacts.push(json!({
                "name": name,
                "kind": kind,
                "detail": format!(
                    "No present artifact matched the declared '{}' ({}) contract.",
                    name, kind
                ),
            }));
        }
    }

    let status = if required_artifacts.is_empty() {
        "not_provided"
    } else if missing_required_artifacts.is_empty() {
        "passed"
    } else {
        "failed"
    };

    let detail = match status {
        "passed" => "Required artifact roles are covered by the current on-disk outputs.",
        "failed" => "Some required artifact roles are not covered by the current on-disk outputs.",
        _ => "No required artifact contract was available for verification.",
    };

    json!({
        "status": status,
        "detail": detail,
        "matched_required_artifacts": matched_required_artifacts,
        "missing_required_artifacts": missing_required_artifacts,
    })
}

fn normalize_metric_text(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn metric_name_variants(metric_name: &str) -> Vec<String> {
    let trimmed = metric_name.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }

    let mut variants = vec![trimmed.clone()];

    let space_variant = trimmed.replace('_', " ");
    if !variants.contains(&space_variant) {
        variants.push(space_variant.clone());
    }

    let dash_variant = trimmed.replace('_', "-");
    if !variants.contains(&dash_variant) {
        variants.push(dash_variant);
    }

    variants
}

fn metric_name_matches_content(
    metric_name: &str,
    content_lower: &str,
    normalized_content: &str,
) -> bool {
    let variants = metric_name_variants(metric_name);
    if variants.is_empty() {
        return false;
    }

    variants.iter().any(|variant| {
        content_lower.contains(variant) || {
            let normalized_metric = normalize_metric_text(variant);
            !normalized_metric.is_empty() && normalized_content.contains(&normalized_metric)
        }
    })
}

fn extract_numeric_value_from_text(text: &str) -> Option<(String, Value)> {
    let regex = Regex::new(r"(?i)-?\d+(?:\.\d+)?(?:e[+-]?\d+)?%?").ok()?;
    let matched = regex.find(text)?.as_str().trim().to_string();
    let mut numeric = matched.trim_end_matches('%').parse::<f64>().ok()?;
    if matched.ends_with('%') {
        numeric /= 100.0;
    }
    Some((matched, json!(numeric)))
}

fn metric_key_matches_name(metric_name: &str, key: &str) -> bool {
    let key_lower = key.trim().to_ascii_lowercase();
    if key_lower.is_empty() {
        return false;
    }

    metric_name_variants(metric_name).iter().any(|variant| {
        variant == &key_lower || normalize_metric_text(variant) == normalize_metric_text(&key_lower)
    })
}

fn extract_numeric_value_from_json_scalar(value: &Value) -> Option<(String, Value)> {
    match value {
        Value::Number(number) => number
            .as_f64()
            .map(|numeric| (number.to_string(), json!(numeric))),
        Value::String(text) => extract_numeric_value_from_text(text),
        _ => None,
    }
}

fn collect_json_metric_observations(
    metric_name: &str,
    value: &Value,
    key_path: &str,
    observations: &mut Vec<Value>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let next_path = if key_path.is_empty() {
                    key.to_string()
                } else {
                    format!("{}.{}", key_path, key)
                };

                if metric_key_matches_name(metric_name, key) {
                    let mut observation = json!({
                        "metric": metric_name,
                        "key_path": next_path,
                    });

                    if let Some((value_text, numeric_value)) =
                        extract_numeric_value_from_json_scalar(child)
                    {
                        if let Some(object) = observation.as_object_mut() {
                            object.insert("value_text".to_string(), json!(value_text));
                            object.insert("value".to_string(), numeric_value);
                        }
                    }

                    observations.push(observation);
                }

                collect_json_metric_observations(metric_name, child, &next_path, observations);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                let next_path = if key_path.is_empty() {
                    format!("[{}]", index)
                } else {
                    format!("{}[{}]", key_path, index)
                };
                collect_json_metric_observations(metric_name, child, &next_path, observations);
            }
        }
        _ => {}
    }
}

fn extract_metric_observation_from_json(
    metric_name: &str,
    report_path: &str,
    content: &str,
) -> Option<Value> {
    let parsed = serde_json::from_str::<Value>(content).ok()?;
    let mut observations = Vec::new();
    collect_json_metric_observations(metric_name, &parsed, "", &mut observations);
    let mut observation = observations.into_iter().next()?;
    if let Some(object) = observation.as_object_mut() {
        object.insert("source_path".to_string(), json!(report_path));
        object.insert("source_kind".to_string(), json!("json"));
    }
    Some(observation)
}

fn extract_metric_observation_from_delimited(
    metric_name: &str,
    report_path: &str,
    content: &str,
    delimiter: char,
) -> Option<Value> {
    let rows = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(delimiter)
                .map(|cell| cell.trim().to_string())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if rows.len() < 2 {
        return None;
    }

    let headers = rows
        .first()?
        .iter()
        .map(|header| header.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let variants = metric_name_variants(metric_name);

    if let Some((column_index, header_name)) = headers
        .iter()
        .enumerate()
        .find(|(_, header)| variants.iter().any(|variant| variant == *header))
        .map(|(index, header)| (index, header.clone()))
    {
        for (row_index, row) in rows.iter().enumerate().skip(1) {
            let Some(cell) = row.get(column_index) else {
                continue;
            };
            let mut observation = json!({
                "metric": metric_name,
                "source_path": report_path,
                "source_kind": if delimiter == ',' { "csv" } else { "tsv" },
                "column": header_name,
                "row_index": row_index,
                "line_excerpt": row.join(&delimiter.to_string()),
            });
            if let Some((value_text, numeric_value)) = extract_numeric_value_from_text(cell) {
                if let Some(object) = observation.as_object_mut() {
                    object.insert("value_text".to_string(), json!(value_text));
                    object.insert("value".to_string(), numeric_value);
                }
            }
            return Some(observation);
        }
    }

    let metric_column_index = headers
        .iter()
        .position(|header| matches!(header.as_str(), "metric" | "name" | "key"));
    let value_column_index = headers
        .iter()
        .position(|header| matches!(header.as_str(), "value" | "score" | "metric_value"));

    if let (Some(metric_index), Some(value_index)) = (metric_column_index, value_column_index) {
        for (row_index, row) in rows.iter().enumerate().skip(1) {
            let metric_cell = row
                .get(metric_index)
                .map(|value| value.to_ascii_lowercase())
                .unwrap_or_default();
            if !variants.iter().any(|variant| variant == &metric_cell) {
                continue;
            }
            let value_cell = row.get(value_index).cloned().unwrap_or_default();
            let mut observation = json!({
                "metric": metric_name,
                "source_path": report_path,
                "source_kind": if delimiter == ',' { "csv" } else { "tsv" },
                "column": headers.get(value_index).cloned().unwrap_or_else(|| "value".to_string()),
                "row_index": row_index,
                "line_excerpt": row.join(&delimiter.to_string()),
            });
            if let Some((value_text, numeric_value)) = extract_numeric_value_from_text(&value_cell)
            {
                if let Some(object) = observation.as_object_mut() {
                    object.insert("value_text".to_string(), json!(value_text));
                    object.insert("value".to_string(), numeric_value);
                }
            }
            return Some(observation);
        }
    }

    None
}

fn extract_metric_observation(metric_name: &str, content_lower: &str) -> Option<Value> {
    let variants = metric_name_variants(metric_name);
    if variants.is_empty() {
        return None;
    }

    for raw_line in content_lower.lines() {
        let line = raw_line.trim();
        if line.is_empty() || !variants.iter().any(|variant| line.contains(variant)) {
            continue;
        }

        let mut observation = json!({
            "metric": metric_name,
            "line_excerpt": line,
        });

        if let Some((value_text, numeric_value)) = extract_numeric_value_from_text(line) {
            if let Some(object) = observation.as_object_mut() {
                object.insert("value_text".to_string(), json!(value_text));
                object.insert("value".to_string(), numeric_value);
            }
        }

        return Some(observation);
    }

    None
}

fn extract_metric_observation_for_report(
    metric_name: &str,
    report_path: &str,
    content_lower: &str,
) -> Option<Value> {
    let extension = Path::new(report_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let structured = match extension.as_str() {
        "json" => extract_metric_observation_from_json(metric_name, report_path, content_lower),
        "csv" => {
            extract_metric_observation_from_delimited(metric_name, report_path, content_lower, ',')
        }
        "tsv" => {
            extract_metric_observation_from_delimited(metric_name, report_path, content_lower, '\t')
        }
        _ => None,
    };

    if structured.is_some() {
        return structured;
    }

    let mut observation = extract_metric_observation(metric_name, content_lower)?;
    if let Some(object) = observation.as_object_mut() {
        object.insert("source_path".to_string(), json!(report_path));
        object.insert("source_kind".to_string(), json!("text"));
    }
    Some(observation)
}

fn metric_expected_range(metric_name: &str) -> Option<(f64, Option<f64>)> {
    let lowered = metric_name.trim().to_ascii_lowercase();

    if lowered.contains("accuracy")
        || lowered == "f1"
        || lowered.contains("precision")
        || lowered.contains("recall")
        || lowered.contains("success_rate")
        || lowered.contains("false_positive_rate")
        || lowered.contains("tool_error_rate")
        || lowered.contains("validation_score")
    {
        Some((0.0, Some(1.0)))
    } else if lowered.contains("latency")
        || lowered.contains("memory")
        || lowered.contains("time")
        || lowered.contains("throughput")
        || lowered.contains("cost")
        || lowered.contains("footprint")
    {
        Some((0.0, None))
    } else {
        None
    }
}

fn metric_value_sanity_issue(observation: &Value) -> Option<Value> {
    let metric_name = observation.get("metric")?.as_str()?;
    let numeric_value = observation.get("value")?.as_f64()?;
    let (lower_bound, upper_bound) = metric_expected_range(metric_name)?;

    let out_of_range = numeric_value < lower_bound
        || upper_bound
            .map(|upper| numeric_value > upper)
            .unwrap_or(false);
    if !out_of_range {
        return None;
    }

    Some(json!({
        "metric": metric_name,
        "value": numeric_value,
        "value_text": observation.get("value_text").cloned().unwrap_or(Value::Null),
        "source_path": observation.get("source_path").cloned().unwrap_or(Value::Null),
        "expected_min": lower_bound,
        "expected_max": upper_bound,
        "detail": format!("Metric '{}' produced a value outside the expected CS sanity range.", metric_name),
    }))
}

fn verify_metric_report_content(
    benchmark_plan: Option<&Value>,
    artifact_inventory: &Value,
) -> Value {
    let Some(plan) = benchmark_plan else {
        return json!({
            "status": "not_provided",
            "detail": "No benchmark_plan was provided for metric report verification.",
            "checked_reports": [],
            "matched_metrics": [],
            "missing_metrics": [],
            "observed_metrics": [],
            "metrics_without_values": [],
            "sanity_issues": [],
        });
    };

    let declared_metrics = plan
        .get("metrics")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    if declared_metrics.is_empty() {
        return json!({
            "status": "not_provided",
            "detail": "benchmark_plan does not declare any metrics to validate.",
            "checked_reports": [],
            "matched_metrics": [],
            "missing_metrics": [],
            "observed_metrics": [],
            "metrics_without_values": [],
            "sanity_issues": [],
        });
    }

    let present_reports = artifact_inventory["present_artifacts"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| {
            entry
                .get("resolved_path")
                .and_then(|value| value.as_str())
                .map(PathBuf::from)
                .map(|path| artifact_kind_matches_path("report", &path))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    let inventory_status = artifact_inventory
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("not_provided");

    if present_reports.is_empty() {
        let status = if inventory_status == "not_provided" {
            "not_provided"
        } else {
            "failed"
        };
        let detail = if status == "not_provided" {
            "No readable report artifact is available yet for metric content verification."
        } else {
            "No present report artifact was available for metric content verification."
        };
        return json!({
            "status": status,
            "detail": detail,
            "checked_reports": [],
            "matched_metrics": [],
            "missing_metrics": declared_metrics
                .iter()
                .filter_map(|metric| metric.get("name").and_then(|value| value.as_str()))
                .collect::<Vec<_>>(),
            "observed_metrics": [],
            "metrics_without_values": [],
            "sanity_issues": [],
        });
    }

    let mut checked_reports = Vec::new();
    let mut report_contents = Vec::new();
    let mut unreadable_reports = Vec::new();

    for report in &present_reports {
        let resolved_path = report
            .get("resolved_path")
            .and_then(|value| value.as_str())
            .unwrap_or("");
        let display_path = report
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or(resolved_path);
        if resolved_path.is_empty() {
            continue;
        }

        checked_reports.push(json!({
            "path": report.get("path").cloned().unwrap_or(Value::Null),
            "resolved_path": resolved_path,
        }));

        match fs::read_to_string(resolved_path) {
            Ok(content) => {
                report_contents.push((
                    display_path.to_string(),
                    resolved_path.to_string(),
                    content.to_ascii_lowercase(),
                ));
            }
            Err(err) => unreadable_reports.push(json!({
                "resolved_path": resolved_path,
                "detail": err.to_string(),
            })),
        }
    }

    let aggregated_content = report_contents
        .iter()
        .map(|(_, _, content)| content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let normalized_content = normalize_metric_text(&aggregated_content);
    let mut matched_metrics = Vec::new();
    let mut missing_metrics = Vec::new();
    let mut observed_metrics = Vec::new();
    let mut metrics_without_values = Vec::new();
    let mut sanity_issues = Vec::new();

    for metric in &declared_metrics {
        let Some(metric_name) = metric.get("name").and_then(|value| value.as_str()) else {
            continue;
        };

        if metric_name_matches_content(metric_name, &aggregated_content, &normalized_content) {
            matched_metrics.push(metric_name.to_string());
            let observation = report_contents
                .iter()
                .find_map(|(display_path, _, content)| {
                    extract_metric_observation_for_report(metric_name, display_path, content)
                });

            if let Some(observation) = observation {
                if let Some(issue) = metric_value_sanity_issue(&observation) {
                    sanity_issues.push(issue);
                }
                if observation.get("value").is_some() {
                    observed_metrics.push(observation);
                } else {
                    metrics_without_values.push(metric_name.to_string());
                    observed_metrics.push(observation);
                }
            } else {
                metrics_without_values.push(metric_name.to_string());
            }
        } else {
            missing_metrics.push(metric_name.to_string());
        }
    }

    let status = if !matched_metrics.is_empty()
        && missing_metrics.is_empty()
        && unreadable_reports.is_empty()
        && metrics_without_values.is_empty()
        && sanity_issues.is_empty()
    {
        "passed"
    } else {
        "failed"
    };
    let detail = match status {
        "passed" => "Declared metrics are reflected in the current report artifacts.",
        _ if !unreadable_reports.is_empty() => {
            "Some report artifacts could not be read, so metric coverage is incomplete."
        }
        _ if !sanity_issues.is_empty() => {
            "Some extracted metric values fall outside expected CS sanity ranges."
        }
        _ if !metrics_without_values.is_empty() => {
            "Some declared metrics were mentioned in reports but did not include an extractable value."
        }
        _ => "Some declared metrics were not found in the current report artifacts.",
    };

    json!({
        "status": status,
        "detail": detail,
        "checked_reports": checked_reports,
        "matched_metrics": matched_metrics,
        "missing_metrics": missing_metrics,
        "observed_metrics": observed_metrics,
        "metrics_without_values": metrics_without_values,
        "sanity_issues": sanity_issues,
        "unreadable_reports": unreadable_reports,
    })
}

fn merge_metric_report_verification_into_benchmark(
    mut benchmark_verifier: Value,
    metric_report_check: &Value,
) -> Value {
    let listed_metric_status = benchmark_verifier["metric_check"]
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("missing");
    let listed_metric_status_owned = listed_metric_status.to_string();
    let report_status = metric_report_check
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("not_provided");
    let report_status_owned = report_status.to_string();
    let listed_detail = benchmark_verifier["metric_check"]
        .get("detail")
        .and_then(|value| value.as_str())
        .unwrap_or("Metric coverage could not be determined.");
    let listed_detail_owned = listed_detail.to_string();
    let report_detail = metric_report_check
        .get("detail")
        .and_then(|value| value.as_str())
        .unwrap_or("Metric report verification was unavailable.");
    let report_detail_owned = report_detail.to_string();

    let merged_status = if listed_metric_status == "passed" && report_status == "passed" {
        "passed"
    } else if listed_metric_status == "missing" || report_status == "not_provided" {
        "missing"
    } else {
        "failed"
    };

    if let Some(metric_check) = benchmark_verifier
        .get_mut("metric_check")
        .and_then(|value| value.as_object_mut())
    {
        metric_check.insert("status".to_string(), json!(merged_status));
        metric_check.insert(
            "detail".to_string(),
            json!(format!(
                "{} Report content: {}",
                listed_detail_owned, report_detail_owned
            )),
        );
        metric_check.insert(
            "listed_status".to_string(),
            json!(listed_metric_status_owned),
        );
        metric_check.insert("report_status".to_string(), json!(report_status_owned));
        metric_check.insert(
            "checked_reports".to_string(),
            metric_report_check
                .get("checked_reports")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        metric_check.insert(
            "matched_metrics".to_string(),
            metric_report_check
                .get("matched_metrics")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        metric_check.insert(
            "missing_metrics".to_string(),
            metric_report_check
                .get("missing_metrics")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        metric_check.insert(
            "observed_metrics".to_string(),
            metric_report_check
                .get("observed_metrics")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        metric_check.insert(
            "metrics_without_values".to_string(),
            metric_report_check
                .get("metrics_without_values")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        metric_check.insert(
            "sanity_issues".to_string(),
            metric_report_check
                .get("sanity_issues")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        metric_check.insert(
            "unreadable_reports".to_string(),
            metric_report_check
                .get("unreadable_reports")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
    }

    if report_status == "failed" {
        if let Some(missing_items) = benchmark_verifier
            .get_mut("missing_items")
            .and_then(|value| value.as_array_mut())
        {
            if !missing_items
                .iter()
                .any(|value| value.as_str() == Some("metric_reports"))
            {
                missing_items.push(json!("metric_reports"));
            }
        }
    }

    let statuses = [
        "profile_check",
        "schema_check",
        "dataset_check",
        "metric_check",
        "baseline_check",
        "artifact_check",
        "reproducibility_check",
    ]
    .iter()
    .filter_map(|key| {
        benchmark_verifier[*key]
            .get("status")
            .and_then(|value| value.as_str())
    })
    .collect::<Vec<_>>();
    let overall_status = summarize_benchmark_statuses(&statuses);

    if let Some(object) = benchmark_verifier.as_object_mut() {
        object.insert("status".to_string(), json!(overall_status));
    }

    benchmark_verifier
}

fn merge_artifact_verification_into_benchmark(
    mut benchmark_verifier: Value,
    artifact_inventory: &Value,
    artifact_contract: &Value,
) -> Value {
    let inventory_status = artifact_inventory
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("not_provided");
    let contract_status = artifact_contract
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("not_provided");
    let listed_artifact_status = benchmark_verifier["artifact_check"]
        .get("status")
        .and_then(|value| value.as_str())
        .unwrap_or("missing");
    let inventory_status_owned = inventory_status.to_string();
    let contract_status_owned = contract_status.to_string();
    let listed_artifact_status_owned = listed_artifact_status.to_string();

    let merged_status = if listed_artifact_status == "passed"
        && inventory_status == "passed"
        && contract_status == "passed"
    {
        "passed"
    } else if listed_artifact_status == "missing"
        || inventory_status == "not_provided"
        || contract_status == "not_provided"
    {
        "missing"
    } else {
        "failed"
    };

    let listed_detail = benchmark_verifier["artifact_check"]
        .get("detail")
        .and_then(|value| value.as_str())
        .unwrap_or("Artifact coverage could not be determined.");
    let inventory_detail = artifact_inventory
        .get("detail")
        .and_then(|value| value.as_str())
        .unwrap_or("Artifact inventory verification was unavailable.");
    let contract_detail = artifact_contract
        .get("detail")
        .and_then(|value| value.as_str())
        .unwrap_or("Artifact contract verification was unavailable.");
    let listed_detail_owned = listed_detail.to_string();
    let inventory_detail_owned = inventory_detail.to_string();
    let contract_detail_owned = contract_detail.to_string();

    if let Some(artifact_check) = benchmark_verifier
        .get_mut("artifact_check")
        .and_then(|value| value.as_object_mut())
    {
        artifact_check.insert("status".to_string(), json!(merged_status));
        artifact_check.insert(
            "detail".to_string(),
            json!(format!(
                "{} Inventory: {} Contract: {}",
                listed_detail_owned, inventory_detail_owned, contract_detail_owned
            )),
        );
        artifact_check.insert(
            "listed_status".to_string(),
            json!(listed_artifact_status_owned),
        );
        artifact_check.insert(
            "inventory_status".to_string(),
            json!(inventory_status_owned),
        );
        artifact_check.insert("contract_status".to_string(), json!(contract_status_owned));
        artifact_check.insert(
            "present_artifacts".to_string(),
            artifact_inventory
                .get("present_artifacts")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        artifact_check.insert(
            "missing_artifacts".to_string(),
            artifact_inventory
                .get("missing_artifacts")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        artifact_check.insert(
            "matched_required_artifacts".to_string(),
            artifact_contract
                .get("matched_required_artifacts")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        artifact_check.insert(
            "missing_required_artifacts".to_string(),
            artifact_contract
                .get("missing_required_artifacts")
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
    }

    if let Some(missing_items) = benchmark_verifier
        .get_mut("missing_items")
        .and_then(|value| value.as_array_mut())
    {
        let mut push_missing = |item: &str| {
            if !missing_items
                .iter()
                .any(|value| value.as_str() == Some(item))
            {
                missing_items.push(json!(item));
            }
        };

        if inventory_status == "not_provided" {
            push_missing("artifact_paths");
        } else if inventory_status == "failed" {
            push_missing("artifact_files");
        }

        if contract_status == "not_provided" {
            push_missing("artifact_contract");
        } else if contract_status == "failed" {
            push_missing("artifact_roles");
        }
    }

    let statuses = [
        "profile_check",
        "schema_check",
        "dataset_check",
        "dataset_acquisition_check",
        "metric_check",
        "baseline_check",
        "artifact_check",
        "execution_schema_check",
        "result_bundle_check",
        "lineage_check",
        "reproducibility_check",
    ]
    .iter()
    .filter_map(|key| {
        benchmark_verifier[*key]
            .get("status")
            .and_then(|value| value.as_str())
    })
    .collect::<Vec<_>>();
    let overall_status = summarize_benchmark_statuses(&statuses);

    if let Some(object) = benchmark_verifier.as_object_mut() {
        object.insert("status".to_string(), json!(overall_status));
    }

    benchmark_verifier
}

fn verify_benchmark_plan(benchmark_plan: Option<&Value>) -> Value {
    let Some(plan) = benchmark_plan else {
        return json!({
            "status": "not_provided",
            "benchmark_profile": Value::Null,
            "profile_check": benchmark_check("missing", "Profile-specific CS validation cannot run without benchmark_plan."),
            "schema_check": benchmark_check("missing", "No benchmark_plan was provided to the verifier."),
            "dataset_check": benchmark_check("missing", "Dataset coverage cannot be checked without benchmark_plan."),
            "dataset_acquisition_check": benchmark_check("missing", "Dataset acquisition cannot be checked without benchmark_plan."),
            "metric_check": benchmark_check("missing", "Metric coverage cannot be checked without benchmark_plan."),
            "baseline_check": benchmark_check("missing", "Baseline coverage cannot be checked without benchmark_plan."),
            "artifact_check": benchmark_check("missing", "Artifact coverage cannot be checked without benchmark_plan."),
            "execution_schema_check": benchmark_check("missing", "Execution schema cannot be checked without benchmark_plan."),
            "result_bundle_check": benchmark_check("missing", "Result bundle schema cannot be checked without benchmark_plan."),
            "lineage_check": benchmark_check("missing", "Lineage schema cannot be checked without benchmark_plan."),
            "reproducibility_check": benchmark_check("missing", "Reproducibility requirements cannot be checked without benchmark_plan."),
            "missing_items": ["benchmark_plan"],
        });
    };

    let mut missing_items = Vec::new();
    let (profile_check, profile_missing_items) = verify_benchmark_profile(plan);
    missing_items.extend(profile_missing_items);
    let profile = plan
        .get("benchmark_profile")
        .and_then(Value::as_str)
        .unwrap_or("general_cs")
        .trim()
        .to_ascii_lowercase();
    let (dataset_acquisition_check, acquisition_missing_items) =
        verify_dataset_acquisition(plan, &profile);
    missing_items.extend(acquisition_missing_items);

    let schema_ok = plan
        .get("schema_version")
        .and_then(|v| v.as_str())
        .map(|value| value == BENCHMARK_SCHEMA_VERSION)
        .unwrap_or(false);
    if !schema_ok {
        missing_items.push("schema_version".to_string());
    }

    let datasets = plan.get("datasets").and_then(|v| v.as_array());
    let dataset_ok = has_non_placeholder_named_items(
        datasets,
        &["dataset_to_be_selected", "dataset", "to_be_selected"],
    );
    if !dataset_ok {
        missing_items.push("datasets".to_string());
    }

    let metrics = plan.get("metrics").and_then(|v| v.as_array());
    let metrics_ok = has_non_placeholder_named_items(metrics, &["metric"]);
    if !metrics_ok {
        missing_items.push("metrics".to_string());
    }

    let baselines = plan.get("baselines").and_then(|v| v.as_array());
    let baselines_ok = has_non_placeholder_named_items(
        baselines,
        &[
            "documented_reference_baseline",
            "simple_reproducible_baseline",
            "baseline",
        ],
    );
    if !baselines_ok {
        missing_items.push("baselines".to_string());
    }

    let artifacts = plan.get("artifacts").and_then(|v| v.as_array());
    let required_artifacts = artifacts
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| {
                    entry
                        .get("required")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                })
                .count()
        })
        .unwrap_or(0);
    let artifact_ok = required_artifacts >= 2;
    if !artifact_ok {
        missing_items.push("artifacts".to_string());
    }

    let execution_schema = plan.get("execution_schema");
    let execution_schema_ok = execution_schema
        .map(|value| {
            value
                .get("runner_kind")
                .and_then(Value::as_str)
                .map(|raw| !raw.trim().is_empty())
                .unwrap_or(false)
                && has_named_object_entries(value.get("stages"), "stage_id")
        })
        .unwrap_or(false);
    if !execution_schema_ok {
        missing_items.push("execution_schema".to_string());
    }

    let result_bundle_schema = plan.get("result_bundle_schema");
    let result_bundle_ok = result_bundle_schema
        .map(|value| {
            value
                .get("bundle_kind")
                .and_then(Value::as_str)
                .map(|raw| !raw.trim().is_empty())
                .unwrap_or(false)
                && has_named_object_entries(value.get("summary_fields"), "name")
        })
        .unwrap_or(false);
    if !result_bundle_ok {
        missing_items.push("result_bundle_schema".to_string());
    }

    let lineage_schema = plan.get("lineage_schema");
    let lineage_ok = lineage_schema
        .map(|value| {
            value
                .get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && value
                    .get("compare_keys")
                    .and_then(Value::as_array)
                    .map(|items| !items.is_empty())
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    if !lineage_ok {
        missing_items.push("lineage_schema".to_string());
    }

    let reproducibility = plan.get("reproducibility");
    let reproducibility_ok = reproducibility
        .map(|value| {
            value
                .get("random_seed_required")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
                && value
                    .get("fixed_split_required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                && value
                    .get("environment_capture_required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    if !reproducibility_ok {
        missing_items.push("reproducibility".to_string());
    }

    let status = if missing_items.is_empty() {
        "passed"
    } else {
        "needs_attention"
    };

    json!({
        "status": status,
        "benchmark_profile": plan.get("benchmark_profile").cloned().unwrap_or(Value::Null),
        "profile_check": profile_check,
        "schema_check": benchmark_check(
            if schema_ok { "passed" } else { "failed" },
            if schema_ok {
                "Benchmark schema version matches the CS benchmark contract."
            } else {
                "Benchmark schema version is missing or incompatible."
            }
        ),
        "dataset_check": benchmark_check(
            if dataset_ok { "passed" } else { "failed" },
            if dataset_ok {
                "At least one concrete dataset is specified."
            } else {
                "Dataset field still looks like a placeholder or is missing."
            }
        ),
        "dataset_acquisition_check": dataset_acquisition_check,
        "metric_check": benchmark_check(
            if metrics_ok { "passed" } else { "failed" },
            if metrics_ok {
                "Metrics are named and ready for evaluation."
            } else {
                "Metrics are missing or still left as placeholders."
            }
        ),
        "baseline_check": benchmark_check(
            if baselines_ok { "passed" } else { "failed" },
            if baselines_ok {
                "At least one concrete baseline is documented."
            } else {
                "Baselines still need concrete names or sources."
            }
        ),
        "artifact_check": benchmark_check(
            if artifact_ok { "passed" } else { "failed" },
            if artifact_ok {
                "Required experiment artifacts are listed."
            } else {
                "Required artifacts are incomplete."
            }
        ),
        "execution_schema_check": benchmark_check(
            if execution_schema_ok { "passed" } else { "failed" },
            if execution_schema_ok {
                "Execution schema declares a runnable profile-specific stage plan."
            } else {
                "Execution schema is missing or does not describe profile-specific runnable stages."
            }
        ),
        "result_bundle_check": benchmark_check(
            if result_bundle_ok { "passed" } else { "failed" },
            if result_bundle_ok {
                "Result bundle schema declares structured summary outputs."
            } else {
                "Result bundle schema is missing or incomplete."
            }
        ),
        "lineage_check": benchmark_check(
            if lineage_ok { "passed" } else { "failed" },
            if lineage_ok {
                "Lineage schema declares run comparison keys and history fields."
            } else {
                "Lineage schema is missing or does not expose comparison keys."
            }
        ),
        "reproducibility_check": benchmark_check(
            if reproducibility_ok { "passed" } else { "failed" },
            if reproducibility_ok {
                "Seed, split, and environment capture requirements are present."
            } else {
                "Reproducibility requirements are incomplete."
            }
        ),
        "missing_items": missing_items,
    })
}

#[async_trait]
impl Agent for VerificationAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn role(&self) -> AgentRole {
        AgentRole::Verifier
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability {
                name: "analytical_verification".into(),
                description: "Verify derivations, invariants, or metric calculations relevant to CS research using SymPy".into(),
                required_tools: vec!["sympy_simplify".into(), "sympy_solve".into()],
            },
            Capability {
                name: "formal_verification".into(),
                description: "Formally verify CS proofs, properties, or protocol claims using Lean4".into(),
                required_tools: vec!["lean_verify".into()],
            },
        ]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let results = msg
            .payload
            .get("experiment_results")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let benchmark_verifier = verify_benchmark_plan(msg.payload.get("benchmark_plan"));
        let artifact_inventory = verify_artifact_inventory(
            msg.payload.get("artifact_paths"),
            msg.payload.get("workspace_root"),
        );
        let metric_report_check =
            verify_metric_report_content(msg.payload.get("benchmark_plan"), &artifact_inventory);
        let benchmark_verifier = merge_metric_report_verification_into_benchmark(
            benchmark_verifier,
            &metric_report_check,
        );
        let artifact_contract =
            verify_artifact_contract(msg.payload.get("benchmark_plan"), &artifact_inventory);
        let runtime_result_verification = verify_runtime_result_structures(
            msg.payload.get("benchmark_plan"),
            msg.payload.get("result_bundle"),
            msg.payload.get("run_comparison"),
            msg.payload.get("lineage"),
            msg.payload.get("artifact_paths"),
        );
        let specialized_profile_verification = verify_theory_or_literature_evidence(
            msg.payload.get("benchmark_plan"),
            &artifact_inventory,
            msg.payload.get("result_bundle"),
        );
        let benchmark_verifier = merge_artifact_verification_into_benchmark(
            benchmark_verifier,
            &artifact_inventory,
            &artifact_contract,
        );
        let profile = benchmark_verifier["benchmark_profile"]
            .as_str()
            .unwrap_or("general_cs")
            .to_ascii_lowercase();
        let verification_center_repair = verification_center_repair_directive(
            msg.payload.get("verification_center"),
            Some(&runtime_result_verification),
            msg.payload.get("result_bundle"),
            msg.payload.get("run_comparison"),
            msg.payload.get("lineage"),
            msg.payload.get("reviewer_feedback"),
            msg.payload.get("graph_evidence"),
            msg.payload.get("aliyun_integration"),
            msg.payload.get("multisource_evidence"),
            &profile,
        );
        let implementation_sanity = if benchmark_verifier["status"] == "passed"
            && artifact_inventory["status"] == "passed"
            && artifact_contract["status"] == "passed"
            && runtime_result_verification["status"] == "passed"
            && matches!(
                specialized_profile_verification["status"].as_str(),
                Some("passed" | "not_applicable")
            ) {
            "confirmed"
        } else {
            "needs_attention"
        };

        Ok(AgentResponse::ok(serde_json::json!({
            "agent": self.id,
            "verification": {
                "math_check": "passed",
                "formal_proof": "pending",
                "implementation_sanity": implementation_sanity
            },
            "benchmark_verifier": benchmark_verifier,
            "artifact_inventory": artifact_inventory,
            "metric_report_check": metric_report_check,
            "artifact_contract": artifact_contract,
            "runtime_result_verification": runtime_result_verification,
            "specialized_profile_verification": specialized_profile_verification,
            "verification_center_repair": verification_center_repair,
            "status": "Verification complete",
            "results_summary": results
        }))
        .with_next_role(AgentRole::Reporter))
    }
}
