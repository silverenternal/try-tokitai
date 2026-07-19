//! ReportAgent - CS paper synthesis and manuscript authoring bundle

use ai_scientist_core::agent::{
    Agent, AgentContext, AgentError, AgentMessage, AgentResponse, AgentRole, Capability,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::BTreeSet;

const PAPER_SCHEMA_VERSION: &str = "cs_paper_blueprint_v1";
const MANUSCRIPT_BUNDLE_SCHEMA_VERSION: &str = "cs_manuscript_bundle_v1";
const DEFAULT_TARGET_VENUE: &str = "computer_science_conference";

#[derive(Clone, Copy)]
struct PaperSectionSpec {
    id: &'static str,
    title: &'static str,
    purpose: &'static str,
    required_inputs: &'static [&'static str],
    prompt_focus: &'static [&'static str],
    writing_skill: &'static str,
    output_contract: &'static [&'static str],
    target_words: usize,
}

const PAPER_SECTION_SPECS: &[PaperSectionSpec] = &[
    PaperSectionSpec {
        id: "title_abstract",
        title: "Title And Abstract",
        purpose: "Summarize the research question, method, strongest evidence, and calibrated takeaway.",
        required_inputs: &[
            "problem_formulation",
            "benchmark_profile",
            "result_bundle.summary_fields",
            "verification.summary",
        ],
        prompt_focus: &[
            "state the concrete computer science task",
            "name the method, workflow, or system under study",
            "report the strongest verified evidence first",
            "calibrate the takeaway to open verification gaps",
        ],
        writing_skill: "abstract_compression",
        output_contract: &["title", "abstract", "keywords"],
        target_words: 220,
    },
    PaperSectionSpec {
        id: "introduction",
        title: "Introduction",
        purpose: "Frame the problem, motivation, gap, and contributions for a CS audience.",
        required_inputs: &[
            "problem_formulation",
            "knowledge_summary",
            "paper_dataset_hints",
            "verification_center_repair",
        ],
        prompt_focus: &[
            "open with the operational problem and why it matters",
            "state the gap relative to retrieved prior work",
            "list contributions that match observed artifacts",
            "preview evaluation scope and known limitations",
        ],
        writing_skill: "research_gap_framing",
        output_contract: &["problem_context", "gap_statement", "contribution_bullets"],
        target_words: 650,
    },
    PaperSectionSpec {
        id: "related_work",
        title: "Related Work",
        purpose: "Position the work against retrieved literature, baselines, and adjacent methods.",
        required_inputs: &[
            "literature_evidence",
            "structured_paper_coverage",
            "paper_dataset_hints",
            "benchmark_plan.baselines",
        ],
        prompt_focus: &[
            "group prior work by method family or benchmark setting",
            "cite only retrieved or explicitly supplied literature evidence",
            "compare datasets, assumptions, and evaluation criteria",
            "identify the unresolved gap the current work targets",
        ],
        writing_skill: "citation_grounded_comparison",
        output_contract: &["comparison_axes", "positioning_claims", "citation_map"],
        target_words: 700,
    },
    PaperSectionSpec {
        id: "method",
        title: "Method",
        purpose: "Describe the algorithm, workflow, or system clearly enough to reproduce the core idea.",
        required_inputs: &[
            "experiment.design",
            "experiment.methodology",
            "benchmark_plan.execution_schema",
            "benchmark_plan.dataset_acquisition",
        ],
        prompt_focus: &[
            "define inputs, outputs, and execution stages",
            "separate method logic from evaluation procedure",
            "document implementation-critical choices",
            "make assumptions and fallback behavior explicit",
        ],
        writing_skill: "method_specification",
        output_contract: &["pipeline_description", "algorithm_or_system_steps", "reproducibility_notes"],
        target_words: 850,
    },
    PaperSectionSpec {
        id: "experimental_setup",
        title: "Experimental Setup",
        purpose: "Document datasets, baselines, metrics, environment, and validation conditions.",
        required_inputs: &[
            "benchmark_plan.datasets",
            "benchmark_plan.metrics",
            "benchmark_plan.baselines",
            "benchmark_plan.reproducibility",
            "verification_center.bundle_runs",
        ],
        prompt_focus: &[
            "state dataset acquisition boundaries and source policy",
            "document baselines and metric direction",
            "capture seed, split, environment, and tool availability",
            "preserve verifier-visible artifact expectations",
        ],
        writing_skill: "benchmark_protocol_design",
        output_contract: &["dataset_table", "baseline_table", "metric_table", "environment_summary"],
        target_words: 900,
    },
    PaperSectionSpec {
        id: "results",
        title: "Results",
        purpose: "Present main quantitative or qualitative outcomes directly from the result bundle.",
        required_inputs: &[
            "result_bundle.summary_fields",
            "run_comparison",
            "lineage",
            "verification.profile_check",
        ],
        prompt_focus: &[
            "report the strongest verified outcome first",
            "tie claims to current and prior runs when comparisons exist",
            "separate observation from interpretation",
            "include negative, null, or inconclusive findings when present",
        ],
        writing_skill: "results_reporting",
        output_contract: &["main_findings", "comparison_summary", "error_or_failure_summary"],
        target_words: 900,
    },
    PaperSectionSpec {
        id: "discussion",
        title: "Discussion",
        purpose: "Interpret the results, tradeoffs, and deployment implications without overstating the evidence.",
        required_inputs: &[
            "result_bundle.summary_fields",
            "verification_center_repair",
            "reviewer_feedback_summary",
            "specialized_profile_verification",
        ],
        prompt_focus: &[
            "explain why the observed results are plausible",
            "connect results to practical or scientific implications",
            "address tensions raised by reviewer feedback or skipped verifiers",
            "keep causal and statistical claims calibrated",
        ],
        writing_skill: "evidence_bounded_interpretation",
        output_contract: &["interpretation_points", "tradeoff_analysis", "practical_implications"],
        target_words: 700,
    },
    PaperSectionSpec {
        id: "limitations",
        title: "Limitations And Threats To Validity",
        purpose: "Make uncertainty, missing evidence, and verification gaps explicit.",
        required_inputs: &[
            "benchmark_verifier.missing_items",
            "runtime_result_verification.missing_items",
            "verification_center_repair",
            "skipped_tools",
        ],
        prompt_focus: &[
            "list threats grounded in missing artifacts or skipped tools",
            "differentiate internal, external, and reproducibility threats",
            "name what future reruns or evidence would close the gap",
            "preserve reviewer-visible honesty",
        ],
        writing_skill: "threat_modeling_for_papers",
        output_contract: &["threats_internal", "threats_external", "followup_requirements"],
        target_words: 500,
    },
    PaperSectionSpec {
        id: "conclusion",
        title: "Conclusion",
        purpose: "Close the paper with the supported claim, evidence recap, and next step.",
        required_inputs: &[
            "problem_formulation",
            "result_bundle.summary_fields",
            "verification.summary",
            "lineage",
        ],
        prompt_focus: &[
            "restate the supported claim in one sentence",
            "summarize the strongest defensible evidence",
            "name the next experimental or engineering step",
            "keep the conclusion aligned with verification outcomes",
        ],
        writing_skill: "claim_calibration",
        output_contract: &["closing_claim", "evidence_recap", "future_work"],
        target_words: 300,
    },
    PaperSectionSpec {
        id: "references_appendix",
        title: "References And Appendix",
        purpose: "Package citations, artifact lineage, and reproducibility appendices for a full paper bundle.",
        required_inputs: &[
            "literature_evidence",
            "artifact_paths",
            "lineage.history",
            "reviewer_feedback",
        ],
        prompt_focus: &[
            "cite only known literature inputs",
            "append artifact and lineage provenance",
            "include reviewer feedback closure notes when relevant",
            "keep appendices structured and audit-friendly",
        ],
        writing_skill: "artifact_appendix_packaging",
        output_contract: &["reference_inventory", "appendix_items", "artifact_lineage_table"],
        target_words: 350,
    },
];

pub struct ReportAgent {
    id: String,
}

impl ReportAgent {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn cleaned_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        Some(Value::Bool(flag)) => flag.to_string(),
        _ => String::new(),
    }
}

fn dedup_string_array(value: Option<&Value>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    value
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|item| {
                    let text = cleaned_string(Some(item));
                    if text.is_empty() || !seen.insert(text.clone()) {
                        None
                    } else {
                        Some(text)
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn join_non_empty(items: &[String], fallback: &str) -> String {
    let filtered = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        fallback.to_string()
    } else {
        filtered.join("; ")
    }
}

fn join_limited(items: &[String], limit: usize, fallback: &str) -> String {
    let filtered = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .take(limit)
        .collect::<Vec<_>>();
    if filtered.is_empty() {
        fallback.to_string()
    } else {
        filtered.join("; ")
    }
}

fn prose_join(items: &[String], fallback: &str) -> String {
    let filtered = items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>();
    match filtered.len() {
        0 => fallback.to_string(),
        1 => filtered[0].to_string(),
        2 => format!("{} and {}", filtered[0], filtered[1]),
        _ => {
            let head = filtered[..filtered.len() - 1].join(", ");
            format!("{head}, and {}", filtered[filtered.len() - 1])
        }
    }
}

fn prose_join_limited(items: &[String], limit: usize, fallback: &str) -> String {
    prose_join(&limited_items(items, limit), fallback)
}

fn limited_items(items: &[String], limit: usize) -> Vec<String> {
    items
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .take(limit)
        .map(|item| item.to_string())
        .collect()
}

fn latex_escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\\' => escaped.push_str("\\textbackslash{}"),
            '{' => escaped.push_str("\\{"),
            '}' => escaped.push_str("\\}"),
            '$' => escaped.push_str("\\$"),
            '&' => escaped.push_str("\\&"),
            '%' => escaped.push_str("\\%"),
            '#' => escaped.push_str("\\#"),
            '_' => escaped.push_str("\\_"),
            '^' => escaped.push_str("\\^{}"),
            '~' => escaped.push_str("\\~{}"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn get_path_value<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().fold(Some(value), |cursor, key| {
        cursor.and_then(|node| node.get(*key))
    })
}

fn collect_named_entries(
    value: Option<&Value>,
    primary_keys: &[&str],
    extra_keys: &[&str],
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let object = item.as_object()?;
                    let primary = primary_keys
                        .iter()
                        .find_map(|key| object.get(*key))
                        .map(|entry| cleaned_string(Some(entry)))
                        .unwrap_or_default();
                    if primary.is_empty() {
                        return None;
                    }
                    let extras = extra_keys
                        .iter()
                        .filter_map(|key| object.get(*key))
                        .map(|entry| cleaned_string(Some(entry)))
                        .filter(|entry| !entry.is_empty() && entry != &primary)
                        .collect::<Vec<_>>();
                    let text = if extras.is_empty() {
                        primary
                    } else {
                        format!("{} ({})", primary, extras.join(", "))
                    };
                    if seen.insert(text.clone()) {
                        Some(text)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn benchmark_dataset_mentions(payload: &Value) -> Vec<String> {
    collect_named_entries(
        get_path_value(payload, &["benchmark_plan", "datasets"]),
        &["dataset_id", "name", "path"],
        &["provider", "task_hint", "split_hint"],
    )
}

fn dataset_hint_mentions(payload: &Value) -> Vec<String> {
    dedup_string_array(payload.get("paper_dataset_hints"))
}

fn result_bundle_summary_entries(payload: &Value) -> Vec<(String, String)> {
    payload
        .get("result_bundle")
        .and_then(|bundle| bundle.get("summary_fields"))
        .and_then(Value::as_array)
        .map(|fields| {
            fields
                .iter()
                .filter_map(|field| {
                    let name = field
                        .get("name")
                        .or_else(|| field.get("field"))
                        .map(|value| cleaned_string(Some(value)))
                        .unwrap_or_default();
                    let value = field
                        .get("value")
                        .or_else(|| field.get("summary"))
                        .map(|entry| cleaned_string(Some(entry)))
                        .unwrap_or_default();
                    if name.is_empty() && value.is_empty() {
                        None
                    } else {
                        Some((name, value))
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn result_bundle_summary_fields(payload: &Value) -> Vec<String> {
    result_bundle_summary_entries(payload)
        .into_iter()
        .map(|(name, value)| match (name.is_empty(), value.is_empty()) {
            (true, true) => String::new(),
            (false, true) => name,
            (true, false) => value,
            (false, false) => format!("{name}: {value}"),
        })
        .filter(|item| !item.trim().is_empty())
        .collect()
}

fn result_field_value(payload: &Value, field_name: &str) -> String {
    result_bundle_summary_entries(payload)
        .into_iter()
        .find_map(|(name, value)| {
            let lowered = value.trim().to_ascii_lowercase();
            if name.eq_ignore_ascii_case(field_name)
                && !value.trim().is_empty()
                && !matches!(
                    lowered.as_str(),
                    "pending" | "tbd" | "todo" | "n/a" | "unknown"
                )
                && !lowered.contains(" pending")
                && !lowered.starts_with("pending ")
            {
                Some(value)
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn selected_result_entries(payload: &Value, field_names: &[&str]) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    field_names
        .iter()
        .filter_map(|field_name| {
            let value = result_field_value(payload, field_name);
            if value.is_empty() || !seen.insert(field_name.to_ascii_lowercase()) {
                None
            } else {
                Some(((*field_name).to_string(), value))
            }
        })
        .collect()
}

fn selected_result_fields(payload: &Value, field_names: &[&str]) -> Vec<String> {
    selected_result_entries(payload, field_names)
        .into_iter()
        .map(|(field_name, field_value)| format!("{field_name}: {field_value}"))
        .collect()
}

fn trimmed_sentence_fragment(text: &str) -> String {
    text.trim()
        .trim_end_matches(|ch: char| matches!(ch, '.' | ';' | ':' | ','))
        .trim()
        .to_string()
}

fn cleaned_error_analysis_summary(payload: &Value) -> String {
    trimmed_sentence_fragment(
        result_field_value(payload, "error_analysis_summary")
            .trim()
            .trim_start_matches("error analysis:")
            .trim(),
    )
}

fn result_audit_anchor(payload: &Value) -> String {
    let run_id = result_field_value(payload, "run_id");
    let dataset_acquisition = result_field_value(payload, "dataset_acquisition");
    let paper_dataset_hints = result_field_value(payload, "paper_dataset_hints");
    let mut parts = Vec::new();
    if !run_id.is_empty() {
        parts.push(format!("run_id: {run_id}"));
    }
    if !dataset_acquisition.is_empty() {
        parts.push(format!("dataset_acquisition: {dataset_acquisition}"));
    }
    if !paper_dataset_hints.is_empty()
        && !dataset_acquisition.to_ascii_lowercase().contains(&format!(
            "paper_dataset_hints={}",
            paper_dataset_hints.to_ascii_lowercase()
        ))
    {
        parts.push(format!("paper_dataset_hints={paper_dataset_hints}"));
    }
    join_non_empty(&parts, "the current result bundle")
}

fn verified_result_sentence(payload: &Value) -> String {
    let run_id = result_field_value(payload, "run_id");
    let primary_metric = result_field_value(payload, "primary_metric");
    let baseline_delta = result_field_value(payload, "baseline_delta");

    match (
        run_id.trim().is_empty(),
        primary_metric.trim().is_empty(),
        baseline_delta.trim().is_empty(),
    ) {
        (false, false, false) => format!(
            "In verified run {run_id}, the result bundle reports primary_metric {primary_metric} and baseline_delta {baseline_delta}."
        ),
        (false, false, true) => format!(
            "In verified run {run_id}, the result bundle reports primary_metric {primary_metric}."
        ),
        (false, true, false) => format!(
            "In verified run {run_id}, the result bundle records baseline_delta {baseline_delta}."
        ),
        (true, false, false) => format!(
            "The result bundle reports primary_metric {primary_metric} and baseline_delta {baseline_delta}."
        ),
        (true, false, true) => {
            format!("The result bundle reports primary_metric {primary_metric}.")
        }
        (true, true, false) => {
            format!("The result bundle records baseline_delta {baseline_delta}.")
        }
        (false, true, true) => format!("The current evidence is tied to run_id: {run_id}."),
        (true, true, true) => "Result evidence is still being assembled.".to_string(),
    }
}

fn verified_run_label(payload: &Value) -> String {
    let run_id = result_field_value(payload, "run_id");
    if run_id.is_empty() {
        "the current verified run".to_string()
    } else {
        format!("run {run_id}")
    }
}

fn reviewer_feedback_open_items(payload: &Value) -> Vec<String> {
    payload
        .get("reviewer_feedback")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter(|entry| {
                    !entry
                        .get("resolved")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .filter_map(|entry| {
                    let reviewer = cleaned_string(entry.get("reviewer"));
                    let comment = cleaned_string(entry.get("comment"));
                    match (reviewer.is_empty(), comment.is_empty()) {
                        (true, true) => None,
                        (false, true) => Some(format!("{reviewer}: feedback pending detail")),
                        (true, false) => Some(comment),
                        (false, false) => Some(format!("{reviewer}: {comment}")),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn verification_missing_items(payload: &Value) -> Vec<String> {
    let mut items = Vec::new();
    for path in [
        &["benchmark_verifier", "missing_items"][..],
        &["runtime_result_verification", "missing_items"][..],
        &["specialized_profile_verification", "missing_items"][..],
    ] {
        if let Some(array) = get_path_value(payload, path).and_then(Value::as_array) {
            items.extend(
                array
                    .iter()
                    .map(|item| cleaned_string(Some(item)))
                    .filter(|text| !text.is_empty()),
            );
        }
    }
    items.sort();
    items.dedup();
    items
}

fn benchmark_profile(payload: &Value) -> String {
    cleaned_string(
        payload
            .get("benchmark_plan")
            .and_then(|plan| plan.get("benchmark_profile"))
            .or_else(|| payload.get("benchmark_profile")),
    )
    .if_empty_then("general_cs")
}

fn profile_display_name(profile: &str) -> &'static str {
    match profile {
        "classical_ml" => "classical machine learning",
        "deep_learning" => "deep learning",
        "systems_evaluation" => "systems evaluation",
        "security_analysis" => "security analysis",
        "agent_evaluation" => "agent evaluation",
        "literature_review" => "literature review",
        "theory" => "theory",
        _ => "computer science",
    }
}

fn infer_target_venue(profile: &str) -> &'static str {
    match profile {
        "deep_learning" => "machine_learning_conference",
        "systems_evaluation" => "systems_conference",
        "security_analysis" => "security_conference",
        "agent_evaluation" => "evaluation_or_agents_track",
        "theory" => "theory_conference",
        "literature_review" => "survey_or_workshop_track",
        _ => DEFAULT_TARGET_VENUE,
    }
}

fn dataset_mentions(payload: &Value) -> Vec<String> {
    let benchmark_mentions = benchmark_dataset_mentions(payload);
    if !benchmark_mentions.is_empty() {
        return benchmark_mentions;
    }
    dataset_hint_mentions(payload)
}

fn metric_mentions(payload: &Value) -> Vec<String> {
    collect_named_entries(
        get_path_value(payload, &["benchmark_plan", "metrics"]),
        &["name"],
        &["direction"],
    )
}

fn baseline_mentions(payload: &Value) -> Vec<String> {
    collect_named_entries(
        get_path_value(payload, &["benchmark_plan", "baselines"]),
        &["name"],
        &["kind"],
    )
}

fn artifact_mentions(payload: &Value) -> Vec<String> {
    collect_named_entries(
        get_path_value(payload, &["benchmark_plan", "artifacts"]),
        &["name"],
        &["kind"],
    )
}

fn artifact_paths(payload: &Value) -> Vec<String> {
    dedup_string_array(payload.get("artifact_paths"))
}

fn skipped_tool_summaries(payload: &Value) -> Vec<String> {
    payload
        .get("verification_center_repair")
        .and_then(|value| value.get("skipped_tools"))
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    if let Some(text) = item.as_str() {
                        let text = text.trim();
                        return if text.is_empty() {
                            None
                        } else {
                            Some(text.to_string())
                        };
                    }
                    let tool = cleaned_string(item.get("tool"));
                    let reason = cleaned_string(item.get("reason"));
                    if tool.is_empty() && reason.is_empty() {
                        None
                    } else if reason.is_empty() {
                        Some(tool)
                    } else if tool.is_empty() {
                        Some(reason)
                    } else {
                        Some(format!("{tool}: {reason}"))
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn skipped_tool_names(payload: &Value) -> Vec<String> {
    let mut seen = BTreeSet::new();
    skipped_tool_summaries(payload)
        .into_iter()
        .filter_map(|item| {
            let name = item.split(':').next().unwrap_or("").trim();
            if name.is_empty() || !seen.insert(name.to_string()) {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

fn appendix_discloses_all(items: &[String], appendix_markdown: &str) -> bool {
    if items.is_empty() {
        return true;
    }
    let normalized_markdown = appendix_markdown.to_ascii_lowercase();
    items.iter().all(|item| {
        let trimmed = item.trim();
        !trimmed.is_empty() && normalized_markdown.contains(&trimmed.to_ascii_lowercase())
    })
}

fn repair_next_actions(payload: &Value) -> Vec<String> {
    dedup_string_array(
        payload
            .get("verification_center_repair")
            .and_then(|value| value.get("next_actions")),
    )
}

fn run_comparison_observations(payload: &Value) -> Vec<String> {
    dedup_string_array(
        payload
            .get("run_comparison")
            .and_then(|value| value.get("observations")),
    )
}

fn compare_keys(payload: &Value) -> Vec<String> {
    dedup_string_array(
        payload
            .get("run_comparison")
            .and_then(|value| value.get("compare_keys"))
            .or_else(|| {
                get_path_value(
                    payload,
                    &["benchmark_plan", "lineage_schema", "compare_keys"],
                )
            }),
    )
}

fn verification_status_summary(payload: &Value) -> String {
    let benchmark_status =
        cleaned_string(get_path_value(payload, &["benchmark_verifier", "status"]));
    let runtime_status = cleaned_string(get_path_value(
        payload,
        &["runtime_result_verification", "status"],
    ));
    let specialized_status = cleaned_string(get_path_value(
        payload,
        &["specialized_profile_verification", "status"],
    ));
    let center_summary = cleaned_string(get_path_value(
        payload,
        &["verification_center_repair", "summary"],
    ));
    let mut parts = Vec::new();
    if !benchmark_status.is_empty() {
        parts.push(format!("benchmark verification: {benchmark_status}"));
    }
    if !runtime_status.is_empty() {
        parts.push(format!("runtime structure verification: {runtime_status}"));
    }
    if !specialized_status.is_empty() {
        parts.push(format!(
            "specialized profile verification: {specialized_status}"
        ));
    }
    if !center_summary.is_empty() {
        parts.push(center_summary);
    }
    join_non_empty(
        &parts,
        "Verification outputs are partial; unresolved claims must be disclosed instead of invented.",
    )
}

fn sanitize_problem_formulation(problem_formulation: &str) -> String {
    let mut cleaned = problem_formulation
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(index) = cleaned.find("[generated") {
        cleaned.truncate(index);
    }
    cleaned = cleaned
        .trim()
        .trim_end_matches(':')
        .trim_end_matches('.')
        .trim()
        .to_string();
    if cleaned.to_ascii_lowercase().starts_with("based on ") {
        cleaned = cleaned[9..].trim().to_string();
    }
    cleaned
}

fn build_title_hint(problem_formulation: &str, profile: &str) -> String {
    let cleaned = sanitize_problem_formulation(problem_formulation);
    if cleaned.is_empty() {
        return "Evidence-Grounded Computer Science Study".to_string();
    }
    let lowered = cleaned.to_ascii_lowercase();
    if profile == "classical_ml" && lowered.contains("iris") {
        return "Lightweight Classification Baselines on the Iris Dataset".to_string();
    }
    if profile == "systems_evaluation"
        && lowered.contains("latency")
        && lowered.contains("throughput")
    {
        return "Reproducible Latency and Throughput Evaluation".to_string();
    }
    if cleaned.len() <= 88 {
        return cleaned;
    }
    let compact = cleaned
        .split(['.', ';', ':'])
        .map(str::trim)
        .find(|segment| !segment.is_empty())
        .unwrap_or(cleaned.as_str())
        .chars()
        .take(88)
        .collect::<String>()
        .trim()
        .trim_end_matches(',')
        .to_string();
    if compact.is_empty() {
        format!("Evidence-Grounded {} Study", profile_display_name(profile))
    } else {
        compact
    }
}

fn abstract_draft(payload: &Value) -> String {
    let profile = benchmark_profile(payload);
    let problem = sanitize_problem_formulation(&cleaned_string(payload.get("problem_formulation")))
        .if_empty_then("the current computer science research question");
    let datasets = dataset_mentions(payload);
    let metrics = metric_mentions(payload);
    let baselines = baseline_mentions(payload);
    let gaps = verification_missing_items(payload);
    let dataset_text = prose_join_limited(&datasets, 2, "configured benchmark inputs");
    let metric_text = prose_join_limited(&metrics, 3, "profile-appropriate metrics");
    let baseline_text = prose_join_limited(&baselines, 3, "documented comparison baselines");
    let error_analysis = cleaned_error_analysis_summary(payload);
    let run_label = verified_run_label(payload);
    let gap_text = if gaps.is_empty() {
        "Current verifier outputs do not surface a blocking evidence gap.".to_string()
    } else {
        format!(
            "One verification item remains open: {}.",
            prose_join_limited(&gaps, 2, "verification gaps")
        )
    };
    let mut sentences = vec![
        format!(
            "We study {} using {} in a reproducible {} evaluation.",
            problem,
            dataset_text,
            profile_display_name(&profile)
        ),
        format!(
            "The workflow compares {} and evaluates {} within a fixed, experiment-backed protocol.",
            baseline_text, metric_text
        ),
        verified_result_sentence(payload),
    ];
    if !error_analysis.is_empty() {
        sentences.push(format!(
            "The recorded error analysis suggests that {}.",
            error_analysis
        ));
    }
    sentences.push(format!(
        "Accordingly, the paper supports a narrow empirical claim tied to {} rather than a dataset-agnostic generalization.",
        run_label
    ));
    sentences.push(gap_text);
    sentences.join(" ")
}

fn section_seed_text(spec: &PaperSectionSpec, payload: &Value) -> String {
    let problem = cleaned_string(payload.get("problem_formulation"))
        .if_empty_then("the active computer science research problem");
    let datasets = dataset_mentions(payload);
    let metrics = metric_mentions(payload);
    let baselines = baseline_mentions(payload);
    let artifacts = artifact_mentions(payload);
    let gaps = verification_missing_items(payload);
    let repair_actions = repair_next_actions(payload);
    let run_observations = run_comparison_observations(payload);
    let skipped_tool_names_only = skipped_tool_names(payload);
    let literature = relevant_literature_titles(payload);
    let (survey_literature, direct_literature, adjacent_literature, peripheral_literature) =
        literature_title_buckets(payload);
    let artifact_locations = artifact_paths(payload);
    let lineage = lineage_mentions(payload);
    let verification_bundle_runs = verification_bundle_run_mentions(payload);
    let error_analysis = cleaned_error_analysis_summary(payload);
    let audit_anchor = result_audit_anchor(payload);
    let verification_summary = verification_status_summary(payload);

    match spec.id {
        "title_abstract" => abstract_draft(payload),
        "introduction" => {
            let mut paragraphs = vec![
                format!(
                    "The question of {} matters because a useful computer-science result must be both measurable and reproducible. This paper evaluates it on {} as a controlled {} study, with retrieved literature defining the context and the workflow artifacts defining the evidentiary boundary.",
                    problem,
                    prose_join_limited(&datasets, 2, "the configured benchmark inputs"),
                    profile_display_name(&benchmark_profile(payload))
                ),
                format!(
                    "The retrieved literature supplies both broad context and a closer methodological reference point. {} Against that background, the present workflow makes a deliberately narrow contribution: it compares {} under {} and anchors the main narrative to concrete run evidence rather than to general claims about all ensemble methods.",
                    if survey_literature.is_empty() && direct_literature.is_empty() {
                        "The official search did not return a directly relevant prior-work set, so this draft records a literature gap instead of presenting unrelated titles as evidence.".to_string()
                    } else {
                        format!(
                            "Broad context comes from {}, while the closest retrieved comparison is {}.",
                            prose_join_limited(&survey_literature, 2, "the retrieved surveys"),
                            prose_join_limited(&direct_literature, 1, "the most directly aligned study")
                        )
                    },
                    prose_join_limited(&baselines, 3, "documented comparison baselines"),
                    prose_join_limited(&metrics, 3, "the declared evaluation metrics")
                ),
            ];
            paragraphs.push(if gaps.is_empty() {
                format!(
                    "The study is therefore positioned as a reproducible benchmark note tied to {}.",
                    verified_run_label(payload)
                )
            } else {
                format!(
                    "The study is therefore positioned as a reproducible benchmark note tied to {}, with the claims calibrated to the remaining verification item {}.",
                    verified_run_label(payload),
                    prose_join_limited(&gaps, 2, "verification gaps")
                )
            });
            paragraphs.join("\n\n")
        }
        "related_work" => {
            let broad_context = if survey_literature.is_empty() {
                "The retrieved set does not include a dedicated survey paper, so broad ensemble context must be inferred from the remaining sources.".to_string()
            } else {
                format!(
                    "Broad context is supplied by {}, which situate the research question within the wider computer-science literature.",
                    prose_join_limited(&survey_literature, 2, "the retrieved surveys")
                )
            };
            let direct_context = if direct_literature.is_empty() {
                "No retrieved paper directly isolates the complete question posed here, which sharpens the need for a reproducible benchmark-driven comparison.".to_string()
            } else {
                format!(
                    "The most directly aligned retrieved study is {}, which provides the closest methodological reference point for the present evaluation.",
                    prose_join_limited(&direct_literature, 2, "the directly aligned prior work")
                )
            };
            let adjacent_context = if adjacent_literature.is_empty() {
                String::new()
            } else {
                format!(
                    "Adjacent methodological background comes from {}, which broadens the backdrop without serving as a one-to-one comparison.",
                    prose_join_limited(&adjacent_literature, 2, "adjacent ensemble references")
                )
            };
            let peripheral_context = if peripheral_literature.is_empty() {
                String::new()
            } else {
                "A small number of retrieved items were only loosely aligned with the research question and were treated as peripheral context rather than direct comparison evidence.".to_string()
            };
            [
                broad_context,
                direct_context,
                if literature.is_empty() {
                    "No retrieved title passed the topic-relevance filter; related-work coverage remains an explicit quality gap.".to_string()
                } else {
                    format!(
                        "For the claim anchor used in this paper, the key retrieved references are {}.",
                        prose_join(
                            &{
                                let mut titles = Vec::new();
                                titles.extend(limited_items(&survey_literature, 1));
                                titles.extend(limited_items(&direct_literature, 1));
                                if titles.is_empty() {
                                    titles.extend(limited_items(&literature, 2));
                                }
                                titles
                            },
                            "the retrieved references"
                        )
                    )
                },
                adjacent_context,
                format!(
                    "Against this literature, the current benchmark is intentionally narrower: it contrasts {} on {} using {}. The aim is not to out-survey prior work, but to produce a reproducible statement about how robustness behaves in the observed workflow.",
                    prose_join_limited(&baselines, 4, "the declared baselines"),
                    prose_join_limited(&datasets, 2, "the benchmark inputs"),
                    prose_join_limited(&metrics, 3, "the declared evaluation metrics")
                ),
                peripheral_context,
            ]
            .into_iter()
            .filter(|paragraph| !paragraph.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n\n")
        }
        "method" => format!(
            "The method is organized as a compact, script-based pipeline so that every substantive claim can be traced to an executable artifact. In concrete terms, the workflow for {} is materialized through {}.\n\nOperationally, the pipeline separates configuration, execution, and reporting. This keeps implementation-critical choices inspectable, while the recorded artifact paths {} provide a direct bridge from the manuscript to the runnable workspace state.",
            problem,
            prose_join_limited(&artifacts, 3, "the benchmark scripts and reports"),
            prose_join_limited(&artifact_locations, 3, "keep the main artifacts available in the workspace")
        ),
        "experimental_setup" => {
            let bundle_context = if verification_bundle_runs.is_empty() {
                verification_summary
            } else {
                format!(
                    "Verification-center bundle runs currently surface {}.",
                    prose_join_limited(&verification_bundle_runs, 2, "the recorded bundle state")
                )
            };
            format!(
                "The experimental setup favors a narrow and reproducible comparison over an unnecessarily broad search space. The dataset or workload anchor is {}, the evaluation metrics are {}, and the baseline family consists of {}.\n\nThis design isolates the declared comparison within a fixed benchmark configuration. {}",
                prose_join_limited(&datasets, 4, "dataset selection pending"),
                prose_join_limited(&metrics, 4, "metric definitions pending"),
                prose_join_limited(&baselines, 4, "baseline specification pending"),
                bundle_context
            )
        }
        "results" => {
            let mut text = format!("The main empirical signal is straightforward. {}", {
                let primary_metric = result_field_value(payload, "primary_metric");
                let baseline_delta = result_field_value(payload, "baseline_delta");
                if error_analysis.is_empty() {
                    verified_result_sentence(payload)
                } else {
                    format!(
                        "In verified run {}, the result bundle reports primary_metric {} and baseline_delta {}; the recorded error analysis indicates that {}.",
                        result_field_value(payload, "run_id").if_empty_then("current-run"),
                        primary_metric.if_empty_then("pending"),
                        baseline_delta.if_empty_then("pending"),
                        error_analysis
                    )
                }
            });
            if !run_observations.is_empty() {
                text.push_str(&format!(
                    " The run comparison log adds {}.",
                    prose_join_limited(&run_observations, 2, "limited comparison evidence")
                ));
            }
            text.push_str(&format!(
                "\n\nWithin the current benchmark, these observations should be read as evidence tied to {} rather than as a universal ranking beyond the recorded datasets, workloads, and conditions.",
                audit_anchor
            ));
            text
        }
        "discussion" => {
            let mut text = format!(
                "The discussion stays deliberately close to the observed evidence. {}",
                {
                    let run_id = result_field_value(payload, "run_id");
                    let primary_metric = result_field_value(payload, "primary_metric");
                    let baseline_delta = result_field_value(payload, "baseline_delta");
                    format!(
                        "It is anchored to run_id: {}, primary_metric: {}, and baseline_delta: {}.",
                        run_id.if_empty_then("current-run"),
                        primary_metric.if_empty_then("pending"),
                        baseline_delta.if_empty_then("pending")
                    )
                }
            );
            if !error_analysis.is_empty() {
                text.push_str(&format!(
                    " The accompanying error analysis suggests that {}.",
                    error_analysis
                ));
            }
            text.push_str(
                " The observed pattern is interpreted only within the recorded protocol; mechanisms not directly tested by the artifacts remain hypotheses for follow-up work.",
            );
            if !repair_actions.is_empty() {
                text.push_str(&format!(
                    "\n\nThe practical implication is that the workflow currently serves better as a calibrated screening study than as a final deployment recommendation. Follow-up work should focus on {} so that the empirical story and the verification bundle close at the same level of rigor.",
                    prose_join_limited(&repair_actions, 2, "the remaining verification-center repair actions")
                ));
            }
            text
        }
        "limitations" => format!(
            "The main internal-validity limitation is that unresolved verifier items still exist, namely {}, and that the verification center still skipped {}. As a result, the paper cannot yet claim completely closed reporting coverage.\n\nExternal validity is also intentionally narrow: the present evidence comes from {} and from a small family of baselines, so the manuscript should be read as a benchmark-specific study rather than as a universal result. The remaining repair path is {}.",
            prose_join_limited(&gaps, 4, "no verifier gap is currently surfaced"),
            prose_join_limited(
                &skipped_tool_names_only,
                4,
                "no skipped verification-center tool is currently surfaced"
            ),
            prose_join_limited(&datasets, 2, "the configured benchmark inputs"),
            prose_join_limited(&repair_actions, 3, "no additional repair action was surfaced")
        ),
        "conclusion" => {
            let gap_sentence = if gaps.is_empty() {
                "The current verification bundle is largely closed, so the conclusion can stay focused on the empirical finding itself.".to_string()
            } else {
                format!(
                    "The conclusion nevertheless remains calibrated to the open verification item {}.",
                    prose_join_limited(&gaps, 2, "verification gaps")
                )
            };
            format!(
                "Taken together, the current workflow supports a narrow, experiment-bound conclusion about {}. The closing recap is anchored to run_id: {}, primary_metric: {}, and baseline_delta: {}.{}\n\n{} The next step is to turn this benchmark-backed narrative into a fully closed paper bundle by addressing {}.",
                problem,
                result_field_value(payload, "run_id").if_empty_then("current-run"),
                result_field_value(payload, "primary_metric").if_empty_then("pending"),
                result_field_value(payload, "baseline_delta").if_empty_then("pending"),
                if error_analysis.is_empty() {
                    String::new()
                } else {
                    format!(" The error analysis indicates that {}.", error_analysis)
                },
                gap_sentence,
                prose_join_limited(&repair_actions, 2, "the remaining verification tasks")
            )
        }
        "references_appendix" => {
            let literature_sentence = if literature.is_empty() {
                "No retrieved references are currently attached to the manuscript bundle.".to_string()
            } else {
                format!(
                    "The reference inventory combines broad disciplinary context with the most relevant methodological comparison, including {}.",
                    prose_join_limited(&literature, 4, "the retrieved papers")
                )
            };
            format!(
                "{}\n\nThe appendix then serves as the audit layer for reproduction and review. It records artifact paths such as {}, lineage notes such as {}, and the disclosure state needed to interpret the current verification bundle.",
                literature_sentence,
                prose_join_limited(&artifact_locations, 3, "artifact paths are pending"),
                prose_join_limited(&lineage, 2, "lineage notes pending")
            )
        }
        _ => String::new(),
    }
}

fn section_revision_items(spec: &PaperSectionSpec, payload: &Value) -> Vec<Value> {
    reviewer_feedback_trace(payload)
        .into_iter()
        .filter(|entry| {
            entry
                .get("closure_state")
                .and_then(Value::as_str)
                .is_some_and(|state| state.eq_ignore_ascii_case("open"))
                && entry
                    .get("target_sections")
                    .and_then(Value::as_array)
                    .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(spec.id)))
        })
        .collect()
}

fn section_revision_summary(spec: &PaperSectionSpec, payload: &Value) -> String {
    let items = section_revision_items(spec, payload);
    if items.is_empty() {
        return "No open reviewer-driven revision is currently targeting this section.".to_string();
    }
    let mut parts = Vec::new();
    for entry in items {
        let reviewer = cleaned_string(entry.get("reviewer")).if_empty_then("reviewer");
        let comment = cleaned_string(entry.get("comment"));
        let reverification_required = entry
            .get("reverification_required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        parts.push(format!(
            "{} asks to address '{}'; reverification_required={}",
            reviewer,
            comment.if_empty_then("feedback detail pending"),
            reverification_required
        ));
    }
    parts.join(" | ")
}

fn section_reverification_scope(spec: &PaperSectionSpec, payload: &Value) -> Vec<String> {
    let mut scopes = BTreeSet::new();
    for entry in section_revision_items(spec, payload) {
        if let Some(items) = entry.get("reverification_scope").and_then(Value::as_array) {
            for item in items {
                if let Some(name) = item.as_str() {
                    let trimmed = name.trim();
                    if !trimmed.is_empty() {
                        scopes.insert(trimmed.to_string());
                    }
                }
            }
        }
    }
    scopes.into_iter().collect()
}

fn section_prompt(spec: &PaperSectionSpec, payload: &Value) -> String {
    let profile = benchmark_profile(payload);
    let problem_formulation = cleaned_string(payload.get("problem_formulation"))
        .if_empty_then("the current experiment-backed computer science study");
    let datasets = dataset_mentions(payload);
    let metrics = metric_mentions(payload);
    let results = result_bundle_summary_fields(payload);
    let gaps = verification_missing_items(payload);
    let feedback = reviewer_feedback_open_items(payload);
    let revision_summary = section_revision_summary(spec, payload);
    let reverification_scope = section_reverification_scope(spec, payload);

    format!(
        "Write the '{}' section for a {} paper targeting {}. Problem anchor: {}. Use only evidence already present in the workflow payload. Required evidence inputs: {}. Available dataset anchors: {}. Available metric anchors: {}. Strongest result anchors: {}. Open reviewer feedback: {}. Section-specific revision queue: {}. Reverification scope after editing: {}. Open verification gaps: {}. Writing focus: {}. Hard constraints: cite only retrieved literature; keep paper retrieval on official APIs; use datasets only from direct official dataset databases or provider APIs; never use dataset search as a paper source; disclose missing evidence instead of inventing details. Deliver polished prose plus structured notes that satisfy the section contract.",
        spec.title,
        profile.replace('_', " "),
        infer_target_venue(&profile),
        problem_formulation,
        spec.required_inputs.join(", "),
        join_limited(&datasets, 3, "dataset evidence pending"),
        join_limited(&metrics, 3, "metric evidence pending"),
        join_limited(&results, 3, "result evidence pending"),
        join_limited(&feedback, 2, "none"),
        revision_summary,
        join_limited(&reverification_scope, 4, "paper_ready_gate"),
        join_limited(&gaps, 3, "none surfaced"),
        spec.prompt_focus.join("; "),
    )
}

fn section_skill_contract(spec: &PaperSectionSpec) -> Value {
    json!({
        "skill_id": spec.writing_skill,
        "purpose": spec.purpose,
        "required_inputs": spec.required_inputs,
        "target_words": spec.target_words,
        "writing_constraints": [
            "Use only evidence already present in the workflow payload.",
            "Differentiate observation, interpretation, and future work.",
            "If evidence is missing, disclose the gap instead of inventing content.",
            "Preserve paper-source and dataset-source boundary policies."
        ],
        "quality_checks": [
            "Claims must trace to result_bundle, lineage, benchmark_plan, or verifier output.",
            "Literature claims may only cite retrieved or explicitly supplied papers.",
            "Reviewer feedback and verification gaps must be surfaced when unresolved."
        ],
        "output_contract": spec.output_contract,
    })
}

fn section_evidence_map(spec: &PaperSectionSpec, payload: &Value) -> Value {
    json!({
        "required_inputs": spec.required_inputs,
        "datasets": dataset_mentions(payload),
        "metrics": metric_mentions(payload),
        "baselines": baseline_mentions(payload),
        "artifacts": artifact_mentions(payload),
        "result_highlights": result_bundle_summary_fields(payload),
        "run_comparison_observations": run_comparison_observations(payload),
        "verification_gaps": verification_missing_items(payload),
        "open_reviewer_feedback": reviewer_feedback_open_items(payload),
        "section_revision_items": section_revision_items(spec, payload),
        "section_reverification_scope": section_reverification_scope(spec, payload),
        "repair_actions": repair_next_actions(payload),
    })
}

fn section_record(spec: &PaperSectionSpec, payload: &Value) -> Value {
    json!({
        "section_id": spec.id,
        "title": spec.title,
        "purpose": spec.purpose,
        "target_words": spec.target_words,
        "prompt": section_prompt(spec, payload),
        "skill_contract": section_skill_contract(spec),
        "evidence_map": section_evidence_map(spec, payload),
        "claim_anchors": section_claim_anchors(spec, payload),
        "draft_seed": section_seed_text(spec, payload),
        "output_contract": spec.output_contract,
    })
}

fn paper_sections(payload: &Value) -> Vec<Value> {
    PAPER_SECTION_SPECS
        .iter()
        .map(|spec| section_record(spec, payload))
        .collect()
}

fn lineage_mentions(payload: &Value) -> Vec<String> {
    let mut seen = BTreeSet::new();
    get_path_value(payload, &["lineage", "history"])
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let text = if let Some(text) = item.as_str() {
                        text.trim().to_string()
                    } else {
                        let run_id = cleaned_string(item.get("run_id").or_else(|| item.get("id")));
                        let parent = cleaned_string(item.get("parent_run_id"));
                        let summary = cleaned_string(
                            item.get("summary")
                                .or_else(|| item.get("change_summary"))
                                .or_else(|| item.get("note")),
                        );
                        if !run_id.is_empty() && !summary.is_empty() {
                            format!("{run_id}: {summary}")
                        } else if !run_id.is_empty() && !parent.is_empty() {
                            format!("{run_id} <- {parent}")
                        } else if !run_id.is_empty() {
                            run_id
                        } else {
                            summary.if_empty_then(&parent)
                        }
                    };
                    if text.is_empty() || !seen.insert(text.clone()) {
                        None
                    } else {
                        Some(text)
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn literature_titles(payload: &Value) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut items = Vec::new();
    for key in ["literature_evidence", "retrieved_papers", "papers"] {
        if let Some(entries) = payload.get(key).and_then(Value::as_array) {
            for entry in entries {
                let title = cleaned_string(entry.get("title"));
                if title.is_empty() || !seen.insert(title.clone()) {
                    continue;
                }
                items.push(title);
            }
        }
    }
    items
}

fn literature_relevance_tokens(payload: &Value) -> BTreeSet<String> {
    let mut source = cleaned_string(payload.get("problem_formulation"));
    for value in dataset_mentions(payload) {
        source.push(' ');
        source.push_str(&value);
    }
    for value in baseline_mentions(payload) {
        source.push(' ');
        source.push_str(&value);
    }
    let stopwords = [
        "about",
        "after",
        "against",
        "analysis",
        "benchmark",
        "comparison",
        "computer",
        "data",
        "dataset",
        "evaluation",
        "experiment",
        "for",
        "from",
        "into",
        "model",
        "models",
        "paper",
        "reproducible",
        "research",
        "results",
        "study",
        "systematic",
        "that",
        "the",
        "their",
        "this",
        "through",
        "using",
        "with",
    ];
    let mut tokens = source
        .split(|ch: char| !ch.is_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3 && !stopwords.contains(&token.as_str()))
        .collect::<BTreeSet<_>>();
    if tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "bagging" | "forest" | "randomforest" | "boosting"
        )
    }) {
        tokens.insert("ensemble".to_string());
    }
    tokens
}

fn literature_title_is_relevant(payload: &Value, title: &str) -> bool {
    let topic_tokens = literature_relevance_tokens(payload);
    if topic_tokens.is_empty() {
        return false;
    }
    let title_tokens = title
        .split(|ch: char| !ch.is_alphanumeric())
        .map(|token| token.trim().to_ascii_lowercase())
        .filter(|token| token.len() >= 3)
        .collect::<BTreeSet<_>>();
    let overlap = topic_tokens.intersection(&title_tokens).count();
    let exact_anchor = [
        "knn",
        "nearest",
        "decision",
        "forest",
        "classification",
        "classifier",
        "ensemble",
        "subsampling",
        "noise",
        "robustness",
    ]
    .iter()
    .any(|token| topic_tokens.contains(*token) && title_tokens.contains(*token));
    exact_anchor || overlap >= 2
}

fn relevant_literature_titles(payload: &Value) -> Vec<String> {
    literature_titles(payload)
        .into_iter()
        .filter(|title| literature_title_is_relevant(payload, title))
        .collect()
}

fn literature_title_buckets(
    payload: &Value,
) -> (Vec<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut survey = Vec::new();
    let mut direct = Vec::new();
    let mut adjacent = Vec::new();
    let mut peripheral = Vec::new();

    for title in relevant_literature_titles(payload) {
        let lowered = title.to_ascii_lowercase();
        if lowered.contains("survey") || lowered.contains("review") {
            survey.push(title);
        } else if lowered.contains("subsampling")
            || lowered.contains("random forest")
            || lowered.contains("tree depth")
            || lowered.contains("label noise")
            || lowered.contains("noisy label")
            || lowered.contains("nearest neighbor")
            || lowered.contains("decision tree")
            || lowered.contains("classification")
            || lowered.contains("classifier")
        {
            direct.push(title);
        } else if lowered.contains("boost")
            || lowered.contains("ensemble learning")
            || lowered.contains("tree ensemble")
            || lowered.contains("forest")
        {
            adjacent.push(title);
        } else {
            peripheral.push(title);
        }
    }

    (survey, direct, adjacent, peripheral)
}

fn related_work_anchor_titles(payload: &Value) -> Vec<String> {
    let (survey, direct, _, _) = literature_title_buckets(payload);
    let mut titles = Vec::new();
    titles.extend(limited_items(&survey, 1));
    titles.extend(limited_items(&direct, 1));
    titles
}

fn verification_bundle_run_mentions(payload: &Value) -> Vec<String> {
    let mut seen = BTreeSet::new();
    get_path_value(payload, &["verification_center_repair", "bundle_runs"])
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let text = if let Some(text) = item.as_str() {
                        text.trim().to_string()
                    } else {
                        let run_id = cleaned_string(item.get("run_id").or_else(|| item.get("id")));
                        let status = cleaned_string(item.get("status"));
                        let summary = cleaned_string(
                            item.get("summary")
                                .or_else(|| item.get("note"))
                                .or_else(|| item.get("bundle_kind")),
                        );
                        if !run_id.is_empty() && !status.is_empty() {
                            format!("{run_id}: {status}")
                        } else if !run_id.is_empty() && !summary.is_empty() {
                            format!("{run_id}: {summary}")
                        } else if !run_id.is_empty() {
                            run_id
                        } else {
                            status.if_empty_then(&summary)
                        }
                    };
                    if text.is_empty() || !seen.insert(text.clone()) {
                        None
                    } else {
                        Some(text)
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn claim_ref_from_strings(
    source_key: &str,
    required: bool,
    items: Vec<String>,
    detail: &str,
) -> Value {
    json!({
        "source_key": source_key,
        "required": required,
        "detail": detail,
        "items": items
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .take(4)
            .collect::<Vec<_>>()
    })
}

fn claim_ref_from_result_entries(
    source_key: &str,
    required: bool,
    entries: Vec<(String, String)>,
    detail: &str,
) -> Value {
    json!({
        "source_key": source_key,
        "required": required,
        "detail": detail,
        "items": entries
            .into_iter()
            .filter(|(field_name, field_value)| !field_name.trim().is_empty() || !field_value.trim().is_empty())
            .take(4)
            .map(|(field_name, field_value)| {
                json!({
                    "field_name": field_name,
                    "field_value": field_value
                })
            })
            .collect::<Vec<_>>()
    })
}

fn claim_anchor_grounding_text(spec: &PaperSectionSpec, claim_id: &str, payload: &Value) -> String {
    let results = result_bundle_summary_fields(payload);
    let result_entries = result_bundle_summary_entries(payload);
    let run_observations = run_comparison_observations(payload);
    let compare_keys = compare_keys(payload);
    let datasets = dataset_mentions(payload);
    let metrics = metric_mentions(payload);
    let baselines = baseline_mentions(payload);
    let artifacts = artifact_mentions(payload);
    let artifact_locations = artifact_paths(payload);
    let literature = relevant_literature_titles(payload);
    let related_titles = related_work_anchor_titles(payload);
    let gaps = verification_missing_items(payload);
    let skipped_tools = skipped_tool_summaries(payload);
    let feedback = reviewer_feedback_open_items(payload);
    let repair_actions = repair_next_actions(payload);
    let lineage = lineage_mentions(payload);
    let verification_summary = verification_status_summary(payload);
    let problem = sanitize_problem_formulation(&cleaned_string(payload.get("problem_formulation")))
        .if_empty_then("the current computer science study");
    let dataset_focus = limited_items(&datasets, 1);
    let metric_focus = limited_items(&metrics, 2);
    let baseline_focus = limited_items(&baselines, 2);
    let artifact_focus = limited_items(&artifacts, 2);
    let artifact_location_focus = limited_items(&artifact_locations, 3);
    let literature_focus = limited_items(&literature, 2);
    let gap_focus = limited_items(&gaps, 1);
    let skipped_tool_focus = limited_items(&skipped_tool_names(payload), 2);
    let run_observation_focus = limited_items(&run_observations, 1);
    let lineage_focus = limited_items(&lineage, 1);
    let result_entry_focus = result_entries
        .iter()
        .take(2)
        .map(|(k, v)| format!("{k}: {v}"))
        .collect::<Vec<_>>();

    match claim_id {
        "title_abstract.main_takeaway" => abstract_draft(payload),
        "introduction.problem_gap" => format!(
            "This paper studies {} on {}. The retrieved literature supplies broad context for this benchmark.",
            problem,
            join_limited(&dataset_focus, 1, "configured benchmark inputs")
        ),
        "related_work.positioning" => format!(
            "For the claim anchor used in this paper, the key retrieved references are {}.",
            prose_join(&related_titles, "retrieved papers pending")
        ),
        "method.reproducible_workflow" => format!(
            "The method implements a reproducible workflow for {}. Workflow artifacts include {}.",
            problem,
            join_limited(&artifact_focus, 2, "workflow artifacts pending")
        ),
        "experimental_setup.protocol" => format!(
            "Experimental setup enumerates datasets {}, metrics {}, and baselines {}.",
            join_limited(&dataset_focus, 1, "datasets pending"),
            join_limited(&metric_focus, 2, "metrics pending"),
            join_limited(&baseline_focus, 2, "baselines pending")
        ),
        "results.primary_outcome" => {
            let primary_metric = result_field_value(payload, "primary_metric");
            let baseline_delta = result_field_value(payload, "baseline_delta");
            let error_analysis = cleaned_error_analysis_summary(payload);
            if error_analysis.is_empty() {
                format!(
                    "In verified run {}, the result bundle reports primary_metric {} and baseline_delta {}.",
                    result_field_value(payload, "run_id").if_empty_then("current-run"),
                    primary_metric.if_empty_then("pending"),
                    baseline_delta.if_empty_then("pending")
                )
            } else {
                format!(
                    "In verified run {}, the result bundle reports primary_metric {} and baseline_delta {}; the recorded error analysis indicates that {}.",
                    result_field_value(payload, "run_id").if_empty_then("current-run"),
                    primary_metric.if_empty_then("pending"),
                    baseline_delta.if_empty_then("pending"),
                    error_analysis
                )
            }
        }
        "results.boundary_conditions" => format!(
            "Run comparison evidence includes {}.",
            join_limited(&run_observation_focus, 1, "comparison evidence pending")
        ),
        "discussion.interpretation_boundary" => format!(
            "The discussion is anchored to run_id: {}, primary_metric: {}, and baseline_delta: {}.",
            result_field_value(payload, "run_id").if_empty_then("current-run"),
            result_field_value(payload, "primary_metric").if_empty_then("pending"),
            result_field_value(payload, "baseline_delta").if_empty_then("pending")
        ),
        "limitations.disclosed_gaps" => format!(
            "Unresolved verifier items still include {}, and the verification center still skipped {}.",
            join_limited(&gap_focus, 1, "no surfaced verifier gap"),
            join_limited(&skipped_tool_focus, 2, "no skipped tool surfaced")
        ),
        "conclusion.supported_closing_claim" => format!(
            "The closing recap is anchored to run_id: {}, primary_metric: {}, and baseline_delta: {}.",
            result_field_value(payload, "run_id").if_empty_then("current-run"),
            result_field_value(payload, "primary_metric").if_empty_then("pending"),
            result_field_value(payload, "baseline_delta").if_empty_then("pending")
        ),
        "references_appendix.audit_trail" => format!(
            "The appendix records artifact paths {} and lineage notes {}.",
            join_limited(&artifact_location_focus, 3, "artifact paths pending"),
            join_limited(&lineage_focus, 1, "lineage notes pending")
        ),
        _ => section_seed_text(spec, payload),
    }
}

fn section_claim_anchors(spec: &PaperSectionSpec, payload: &Value) -> Vec<Value> {
    let problem = cleaned_string(payload.get("problem_formulation"))
        .if_empty_then("the current computer science study");
    let datasets = dataset_mentions(payload);
    let metrics = metric_mentions(payload);
    let baselines = baseline_mentions(payload);
    let artifacts = artifact_mentions(payload);
    let artifact_locations = artifact_paths(payload);
    let result_entries = result_bundle_summary_entries(payload);
    let result_fields = result_bundle_summary_fields(payload);
    let run_observations = run_comparison_observations(payload);
    let compare_keys = compare_keys(payload);
    let feedback = reviewer_feedback_open_items(payload);
    let gaps = verification_missing_items(payload);
    let repair_actions = repair_next_actions(payload);
    let verification_summary = verification_status_summary(payload);
    let skipped_tools = skipped_tool_summaries(payload);
    let lineage = lineage_mentions(payload);
    let literature = relevant_literature_titles(payload);
    let related_titles = related_work_anchor_titles(payload);
    let verification_bundle_runs = verification_bundle_run_mentions(payload);
    let mut anchors = match spec.id {
        "title_abstract" => {
            let claim_id = "title_abstract.main_takeaway";
            let mut evidence_refs = vec![
                claim_ref_from_result_entries(
                    "result_bundle.summary_fields",
                    true,
                    result_entries.clone(),
                    "Result-bundle summary fields that justify the headline abstract claim.",
                ),
                claim_ref_from_strings(
                    "benchmark_plan.datasets",
                    true,
                    datasets.clone(),
                    "Datasets that bound the abstract's study scope.",
                ),
            ];
            if !gaps.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "runtime_result_verification.missing_items",
                    true,
                    gaps.clone(),
                    "Verifier-visible gaps that the abstract explicitly discloses.",
                ));
            } else if !verification_summary.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification.summary",
                    false,
                    vec![verification_summary.clone()],
                    "Verifier-facing summary that calibrates the abstract takeaway.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "summary",
                "claim_text": format!(
                    "The abstract summarizes {} using the strongest verified result ({}) and keeps the takeaway calibrated to the visible verification state.",
                    problem,
                    join_limited(&result_fields, 1, "result evidence pending")
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "introduction" => {
            let claim_id = "introduction.problem_gap";
            let mut evidence_refs = vec![
                claim_ref_from_strings(
                    "literature_evidence",
                    false,
                    limited_items(&literature, 2),
                    "Retrieved literature titles that support the related gap statement.",
                ),
                claim_ref_from_strings(
                    "benchmark_plan.datasets",
                    true,
                    limited_items(&datasets, 1),
                    "Concrete benchmark scope surfaced in the introduction.",
                ),
            ];
            if !result_entries.is_empty() {
                evidence_refs.push(claim_ref_from_result_entries(
                    "result_bundle.summary_fields",
                    false,
                    result_entries.clone(),
                    "Observed result fields that bound the introduction's contribution preview.",
                ));
            }
            if !gaps.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "runtime_result_verification.missing_items",
                    false,
                    gaps.clone(),
                    "Open verification gaps that the introduction should not overclaim past.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "problem_framing",
                "claim_text": format!(
                    "The introduction identifies the benchmark scope ({}) for {} and notes that retrieved literature frames the study context.",
                    join_limited(&limited_items(&datasets, 1), 1, "benchmark scope pending"),
                    problem,
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "related_work" => {
            let claim_id = "related_work.positioning";
            let mut evidence_refs = vec![claim_ref_from_strings(
                "literature_evidence",
                true,
                related_titles.clone(),
                "Topic-relevant retrieved paper titles used in the related-work positioning; an empty set intentionally fails the paper quality gate.",
            )];
            if !datasets.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "benchmark_plan.datasets",
                    false,
                    limited_items(&datasets, 1),
                    "Benchmark datasets that define whether prior work is directly comparable.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "literature_positioning",
                "claim_text": format!(
                    "Related work identifies the key retrieved references ({}) that frame the comparison set for this study.",
                    join_limited(&related_titles, 2, "retrieved papers pending")
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "method" => {
            let claim_id = "method.reproducible_workflow";
            let mut evidence_refs = vec![claim_ref_from_strings(
                "benchmark_plan.artifacts",
                true,
                limited_items(&artifacts, 2),
                "Executable or report artifacts that make the method reproducible.",
            )];
            if !artifact_locations.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "artifact_paths",
                    false,
                    limited_items(&artifact_locations, 3),
                    "Concrete artifact paths that can be linked from the manuscript appendix.",
                ));
            }
            if !verification_bundle_runs.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification_center_repair.bundle_runs",
                    false,
                    verification_bundle_runs.clone(),
                    "Verification-center bundle runs that connect the method to replayable execution.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "method_specification",
                "claim_text": format!(
                    "The method section names the executable workflow artifacts ({}) that support reproducibility for {}.",
                    join_limited(&limited_items(&artifacts, 2), 2, "workflow artifacts pending"),
                    problem
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "experimental_setup" => {
            let claim_id = "experimental_setup.protocol";
            let mut evidence_refs = vec![
                claim_ref_from_strings(
                    "benchmark_plan.datasets",
                    true,
                    limited_items(&datasets, 1),
                    "Datasets named in the setup section.",
                ),
                claim_ref_from_strings(
                    "benchmark_plan.metrics",
                    true,
                    limited_items(&metrics, 2),
                    "Metrics named in the setup section.",
                ),
                claim_ref_from_strings(
                    "benchmark_plan.baselines",
                    true,
                    limited_items(&baselines, 2),
                    "Baselines named in the setup section.",
                ),
            ];
            if !verification_bundle_runs.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification_center_repair.bundle_runs",
                    false,
                    verification_bundle_runs.clone(),
                    "Verification-center bundle runs that confirm environment or bundle conditions.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "protocol",
                "claim_text": format!(
                    "Experimental setup enumerates datasets ({}), metrics ({}), baselines ({}), and reproducibility conditions for the run.",
                    join_limited(&limited_items(&datasets, 1), 1, "datasets pending"),
                    join_limited(&limited_items(&metrics, 2), 2, "metrics pending"),
                    join_limited(&limited_items(&baselines, 2), 2, "baselines pending")
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "results" => {
            let primary_claim_id = "results.primary_outcome";
            let mut primary_evidence_refs = vec![claim_ref_from_result_entries(
                "result_bundle.summary_fields",
                true,
                selected_result_entries(
                    payload,
                    &[
                        "run_id",
                        "primary_metric",
                        "baseline_delta",
                        "error_analysis_summary",
                    ],
                ),
                "Result fields that support the main empirical claim.",
            )];
            if !compare_keys.is_empty() {
                primary_evidence_refs.push(claim_ref_from_strings(
                    "run_comparison.compare_keys",
                    false,
                    compare_keys.clone(),
                    "Comparison axes that support any claimed delta versus prior runs.",
                ));
            }
            if !lineage.is_empty() {
                primary_evidence_refs.push(claim_ref_from_strings(
                    "lineage.history",
                    false,
                    lineage.clone(),
                    "Lineage-linked runs that contextualize the result.",
                ));
            }
            let mut anchors = vec![json!({
                "claim_id": primary_claim_id,
                "section_id": spec.id,
                "claim_kind": "empirical_observation",
                "claim_text": format!(
                    "The results section anchors the main empirical outcome to {}.",
                    join_limited(
                        &selected_result_fields(
                            payload,
                            &["run_id", "primary_metric", "baseline_delta"]
                        ),
                        3,
                        "result evidence pending"
                    )
                ),
                "grounding_text": claim_anchor_grounding_text(spec, primary_claim_id, payload),
                "evidence_refs": primary_evidence_refs
            })];
            if !run_observations.is_empty() {
                let boundary_claim_id = "results.boundary_conditions";
                anchors.push(json!({
                    "claim_id": boundary_claim_id,
                    "section_id": spec.id,
                    "claim_kind": "boundary_case",
                    "claim_text": format!(
                        "The results section records comparison evidence such as {}.",
                        join_limited(&limited_items(&run_observations, 1), 1, "comparison evidence pending")
                    ),
                    "grounding_text": claim_anchor_grounding_text(spec, boundary_claim_id, payload),
                    "evidence_refs": [claim_ref_from_strings(
                        "run_comparison.observations",
                        true,
                        limited_items(&run_observations, 1),
                        "Observed comparison notes that expose boundary conditions."
                    )]
                }));
            }
            anchors
        }
        "discussion" => {
            let claim_id = "discussion.interpretation_boundary";
            let evidence_refs = vec![claim_ref_from_result_entries(
                "result_bundle.summary_fields",
                true,
                selected_result_entries(payload, &["run_id", "primary_metric", "baseline_delta"]),
                "Result fields that the discussion is allowed to interpret.",
            )];
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "interpretation",
                "claim_text": format!(
                    "Discussion stays anchored to {}.",
                    join_limited(
                        &selected_result_fields(
                            payload,
                            &["run_id", "primary_metric", "baseline_delta"]
                        ),
                        3,
                        "the current result bundle"
                    )
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "limitations" => {
            let claim_id = "limitations.disclosed_gaps";
            let gap_focus = limited_items(&gaps, 1);
            let skipped_tool_focus = limited_items(&skipped_tool_names(payload), 2);
            let mut evidence_refs = Vec::new();
            if !gap_focus.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "runtime_result_verification.missing_items",
                    true,
                    gap_focus.clone(),
                    "Concrete missing items that must be disclosed when present.",
                ));
            }
            if !skipped_tool_focus.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification_center_repair.skipped_tools",
                    true,
                    skipped_tool_focus.clone(),
                    "Skipped tools that must be disclosed instead of hidden.",
                ));
            }
            if evidence_refs.is_empty() && !verification_summary.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification.summary",
                    true,
                    vec![verification_summary.clone()],
                    "Verification summary that motivates the limitation framing.",
                ));
            }
            if !repair_actions.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification_center_repair.next_actions",
                    false,
                    repair_actions.clone(),
                    "Follow-up actions that would close remaining limitations.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "limitation_disclosure",
                "claim_text": format!(
                    "Limitations disclose the verifier gap ({}) and the skipped tools ({}).",
                    join_limited(&gap_focus, 1, "no surfaced verifier gap"),
                    join_limited(&skipped_tool_focus, 2, "no skipped tool surfaced")
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "conclusion" => {
            let claim_id = "conclusion.supported_closing_claim";
            let mut evidence_refs = vec![claim_ref_from_result_entries(
                "result_bundle.summary_fields",
                true,
                selected_result_entries(payload, &["run_id", "primary_metric", "baseline_delta"]),
                "Result fields that support the closing claim.",
            )];
            if !gaps.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "runtime_result_verification.missing_items",
                    false,
                    gaps.clone(),
                    "Open verification gaps that bound the strength of the closing claim.",
                ));
            } else if !verification_summary.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification.summary",
                    false,
                    vec![verification_summary.clone()],
                    "Verification summary that calibrates the conclusion.",
                ));
            }
            if !repair_actions.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "verification_center_repair.next_actions",
                    false,
                    repair_actions.clone(),
                    "Next actions that define the conclusion's forward-looking scope.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "closing_claim",
                "claim_text": format!(
                    "The closing recap is anchored to {}.",
                    join_limited(
                        &selected_result_fields(
                            payload,
                            &["run_id", "primary_metric", "baseline_delta"]
                        ),
                        3,
                        "the current evidence bundle"
                    )
                ),
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        "references_appendix" => {
            let claim_id = "references_appendix.audit_trail";
            let mut evidence_refs = vec![claim_ref_from_strings(
                "artifact_paths",
                true,
                limited_items(&artifact_locations, 3),
                "Artifact paths surfaced in the appendix.",
            )];
            if !lineage.is_empty() {
                evidence_refs.push(claim_ref_from_strings(
                    "lineage.history",
                    false,
                    limited_items(&lineage, 1),
                    "Lineage notes that connect artifacts to prior runs.",
                ));
            }
            vec![json!({
                "claim_id": claim_id,
                "section_id": spec.id,
                "claim_kind": "audit_trail",
                "claim_text": "The appendix records artifact paths and lineage notes so the workflow can be audited.",
                "grounding_text": claim_anchor_grounding_text(spec, claim_id, payload),
                "evidence_refs": evidence_refs
            })]
        }
        _ => Vec::new(),
    };
    for anchor in &mut anchors {
        if anchor.get("section_title").is_none() {
            anchor["section_title"] = json!(spec.title);
        }
    }
    anchors
}

fn source_policy() -> Value {
    json!({
        "paper_retrieval": "official_api_only",
        "dataset_retrieval": "direct_official_dataset_databases",
        "dataset_results_must_not_be_used_as_paper_sources": true,
        "paper_fetch_limit_note": "fetch_papers remote fulltext should stay bounded by the configured per-run limit",
    })
}

fn manuscript_blueprint(payload: &Value) -> Value {
    let profile = benchmark_profile(payload);
    let problem_formulation = cleaned_string(payload.get("problem_formulation"));
    let title_hint = build_title_hint(&problem_formulation, &profile);
    let result_highlights = result_bundle_summary_fields(payload);
    let sections = paper_sections(payload);

    json!({
        "paper_schema_version": PAPER_SCHEMA_VERSION,
        "manuscript_bundle_schema_version": MANUSCRIPT_BUNDLE_SCHEMA_VERSION,
        "benchmark_profile": profile,
        "target_venue": infer_target_venue(&benchmark_profile(payload)),
        "problem_formulation": problem_formulation,
        "title_hint": title_hint,
        "abstract_focus": join_non_empty(
            &result_highlights,
            "Summarize the strongest verified result, experimental setting, and the main limitation."
        ),
        "paper_dataset_hints": dataset_mentions(payload),
        "metrics": metric_mentions(payload),
        "baselines": baseline_mentions(payload),
        "artifacts": artifact_mentions(payload),
        "artifact_paths": artifact_paths(payload),
        "result_highlights": result_highlights,
        "run_comparison_observations": run_comparison_observations(payload),
        "open_reviewer_feedback": reviewer_feedback_open_items(payload),
        "verification_gaps": verification_missing_items(payload),
        "verification_center": {
            "repair_directive": cleaned_string(get_path_value(payload, &["verification_center_repair", "repair_directive"])),
            "summary": cleaned_string(get_path_value(payload, &["verification_center_repair", "summary"])),
            "skipped_tools": skipped_tool_summaries(payload),
            "next_actions": repair_next_actions(payload),
            "bundle_focus": payload
                .get("verification_center_repair")
                .and_then(|value| value.get("bundle_focus"))
                .cloned()
                .unwrap_or_else(|| json!([]))
        },
        "source_policy": source_policy(),
        "sections": sections,
        "module_execution_order": PAPER_SECTION_SPECS
            .iter()
            .map(|spec| spec.id)
            .collect::<Vec<_>>(),
        "quality_gates": [
            "Every empirical claim must map to result_bundle, lineage, or verifier evidence.",
            "Related work may only cite retrieved or explicitly supplied literature evidence.",
            "Limitations must mention unresolved verifier gaps and skipped tools when they exist.",
            "Experimental setup must preserve official-paper-API and direct-dataset-database boundaries.",
            "Reviewer feedback must be answered or explicitly disclosed before the paper is marked complete."
        ],
        "delivery_contract": {
            "required_outputs": [
                "complete_manuscript_bundle",
                "section_prompt_pack",
                "section_skill_pack",
                "latex_manuscript_shell",
                "tables_figures_plan",
                "citation_inventory",
                "artifact_appendix_plan",
                "paper_quality_checklist"
            ],
            "format": "latex_plus_structured_json",
            "citation_style": "bibtex-ready",
            "must_be_experiment_grounded": true,
            "must_preserve_source_boundaries": true,
            "must_close_or_disclose_reviewer_feedback": true
        }
    })
}

fn section_skill_pack() -> Vec<Value> {
    PAPER_SECTION_SPECS
        .iter()
        .map(|spec| {
            json!({
                "skill_id": spec.writing_skill,
                "section_id": spec.id,
                "section_title": spec.title,
                "purpose": spec.purpose,
                "required_inputs": spec.required_inputs,
                "target_words": spec.target_words,
                "output_contract": spec.output_contract,
            })
        })
        .collect()
}

fn draft_sections(payload: &Value) -> Vec<Value> {
    PAPER_SECTION_SPECS
        .iter()
        .map(|spec| {
            json!({
                "section_id": spec.id,
                "title": spec.title,
                "target_words": spec.target_words,
                "draft_seed": section_seed_text(spec, payload),
                "claim_anchors": section_claim_anchors(spec, payload),
                "revision_directive": section_revision_summary(spec, payload),
                "reverification_scope": section_reverification_scope(spec, payload),
                "completion_checks": [
                    "Citations and claims are grounded in retrieved evidence.",
                    "The section satisfies the declared output contract.",
                    "Open gaps or reviewer comments are disclosed when relevant."
                ],
            })
        })
        .collect()
}

fn tables_figures_plan(payload: &Value) -> Vec<Value> {
    let profile = benchmark_profile(payload);
    let metrics = metric_mentions(payload);
    let baselines = baseline_mentions(payload);
    let result_highlights = result_bundle_summary_fields(payload);
    let mut items = vec![
        json!({
            "artifact_id": "evidence_flow_figure",
            "kind": "figure",
            "section": "Introduction",
            "purpose": "Show how source evidence, executable methods, verification, and research outputs are connected without introducing synthetic measurements.",
            "required_inputs": ["artifact_paths", "result_bundle.summary_fields", "runtime_result_verification"],
            "materialization": "deterministic_tikz"
        }),
        json!({
            "artifact_id": "main_results_table",
            "kind": "table",
            "section": "Results",
            "purpose": "Compare the main method or workflow against baselines using the declared primary metrics.",
            "required_inputs": ["result_bundle.summary_fields", "benchmark_plan.metrics", "benchmark_plan.baselines"],
            "suggested_columns": ["method_or_run", "metric", "value", "baseline_delta", "evidence_anchor"],
            "anchors": {
                "metrics": metrics,
                "baselines": baselines,
                "result_highlights": result_highlights
            }
        }),
        json!({
            "artifact_id": "reproducibility_table",
            "kind": "table",
            "section": "Experimental Setup",
            "purpose": "Summarize dataset source, split policy, environment capture, and artifact locations.",
            "required_inputs": ["benchmark_plan.datasets", "benchmark_plan.reproducibility", "artifact_paths"],
            "suggested_columns": ["component", "setting", "source", "artifact_or_note"]
        }),
    ];

    match profile.as_str() {
        "deep_learning" => {
            items.push(json!({
                "artifact_id": "training_resource_table",
                "kind": "table",
                "section": "Results",
                "purpose": "Summarize validation metric, checkpoint artifact, and resource usage.",
                "required_inputs": ["result_bundle.summary_fields", "verification_center_repair.runtime_summary"],
                "suggested_columns": ["run_id", "best_validation_metric", "checkpoint_path", "resource_summary"]
            }));
            items.push(json!({
                "artifact_id": "validation_curve_figure",
                "kind": "figure",
                "section": "Results",
                "purpose": "Visualize training or validation behavior across checkpoints or reruns.",
                "required_inputs": ["lineage.history", "run_comparison.compare_keys"],
            }));
        }
        "systems_evaluation" => {
            items.push(json!({
                "artifact_id": "latency_throughput_table",
                "kind": "table",
                "section": "Results",
                "purpose": "Report workload, latency, throughput, and resource summary together.",
                "required_inputs": ["result_bundle.summary_fields", "verification_center_repair.runtime_summary"],
                "suggested_columns": ["workload", "latency", "throughput", "resource_summary"]
            }));
            items.push(json!({
                "artifact_id": "tail_latency_figure",
                "kind": "figure",
                "section": "Results",
                "purpose": "Show latency behavior or tail-performance tradeoffs across runs.",
                "required_inputs": ["run_comparison.observations", "lineage.history"],
            }));
        }
        "security_analysis" => {
            items.push(json!({
                "artifact_id": "findings_table",
                "kind": "table",
                "section": "Results",
                "purpose": "List confirmed findings, false positives, coverage, and impact evidence.",
                "required_inputs": ["result_bundle.summary_fields"],
                "suggested_columns": ["finding_group", "confirmed_findings", "false_positive_count", "coverage_summary", "impact_summary"]
            }));
            items.push(json!({
                "artifact_id": "coverage_impact_matrix",
                "kind": "figure",
                "section": "Discussion",
                "purpose": "Map target coverage against impact or severity for reviewer-facing analysis.",
                "required_inputs": ["result_bundle.summary_fields", "reviewer_feedback"],
            }));
        }
        "agent_evaluation" => {
            items.push(json!({
                "artifact_id": "task_suite_table",
                "kind": "table",
                "section": "Results",
                "purpose": "Compare task success, tool error rate, and judge summary across runs.",
                "required_inputs": ["result_bundle.summary_fields", "run_comparison"],
                "suggested_columns": ["run_id", "task_success_rate", "tool_error_rate", "judge_summary"]
            }));
        }
        "theory" => {
            items.push(json!({
                "artifact_id": "proof_dependency_table",
                "kind": "table",
                "section": "Method",
                "purpose": "Track definitions, lemmas, theorem claims, and counterexample checks.",
                "required_inputs": ["result_bundle.summary_fields", "artifact_paths"],
                "suggested_columns": ["claim", "supporting_lemma", "proof_status", "counterexample_status"]
            }));
        }
        "literature_review" => {
            items.push(json!({
                "artifact_id": "screening_summary_table",
                "kind": "table",
                "section": "Experimental Setup",
                "purpose": "Record search scope, screening summary, remote fulltext coverage, and structured-paper coverage.",
                "required_inputs": ["result_bundle.summary_fields"],
                "suggested_columns": ["search_scope", "screening_summary", "remote_fulltext_coverage", "structured_paper_coverage"]
            }));
        }
        _ => {
            items.push(json!({
                "artifact_id": "error_analysis_table",
                "kind": "table",
                "section": "Results",
                "purpose": "Summarize primary metric, baseline delta, and error or failure analysis.",
                "required_inputs": ["result_bundle.summary_fields"],
                "suggested_columns": ["run_id", "primary_metric", "baseline_delta", "error_analysis_summary"]
            }));
        }
    }

    if !run_comparison_observations(payload).is_empty() {
        items.push(json!({
            "artifact_id": "run_comparison_table",
            "kind": "table",
            "section": "Results",
            "purpose": "Compare the current run against prior lineage-linked runs.",
            "required_inputs": ["run_comparison.compare_keys", "run_comparison.observations", "lineage.history"],
            "suggested_columns": ["compare_key", "current_run", "prior_run", "observation"]
        }));
    }

    items
}

fn citation_inventory(payload: &Value) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    let mut citations = Vec::new();
    for key in ["literature_evidence", "retrieved_papers", "papers"] {
        if let Some(items) = payload.get(key).and_then(Value::as_array) {
            for item in items {
                let title = cleaned_string(item.get("title"));
                if title.is_empty() || !literature_title_is_relevant(payload, &title) {
                    continue;
                }
                let paper_id = cleaned_string(item.get("paper_id").or_else(|| item.get("id")));
                let authors = item
                    .get("authors")
                    .and_then(Value::as_array)
                    .map(|entries| {
                        entries
                            .iter()
                            .filter_map(|entry| {
                                if let Some(text) = entry.as_str() {
                                    let text = text.trim();
                                    if text.is_empty() {
                                        None
                                    } else {
                                        Some(text.to_string())
                                    }
                                } else {
                                    let name = cleaned_string(entry.get("name"));
                                    if name.is_empty() {
                                        None
                                    } else {
                                        Some(name)
                                    }
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" and ")
                    })
                    .unwrap_or_default();
                let venue = cleaned_string(
                    item.get("venue")
                        .or_else(|| item.get("source"))
                        .or_else(|| item.get("provider")),
                );
                let year = cleaned_string(item.get("year"));
                let url = cleaned_string(
                    item.pointer("/urls/landing_page")
                        .or_else(|| item.pointer("/urls/pdf"))
                        .or_else(|| item.get("url")),
                );
                let dedup_key = if !paper_id.is_empty() {
                    paper_id.clone()
                } else {
                    title.clone()
                };
                if dedup_key.is_empty() || !seen.insert(dedup_key) {
                    continue;
                }
                citations.push(json!({
                    "paper_id": if paper_id.is_empty() { Value::Null } else { json!(paper_id) },
                    "authors": if authors.is_empty() { Value::Null } else { json!(authors) },
                    "title": if title.is_empty() { Value::Null } else { json!(title) },
                    "venue_or_source": if venue.is_empty() { Value::Null } else { json!(venue) },
                    "year": if year.is_empty() { Value::Null } else { json!(year) },
                    "url": if url.is_empty() { Value::Null } else { json!(url) },
                    "citation_status": "retrieved"
                }));
            }
        }
    }
    citations
}

fn artifact_appendix_plan(payload: &Value) -> Value {
    let paths = artifact_paths(payload);
    let feedback = reviewer_feedback_open_items(payload);
    let gaps = verification_missing_items(payload);
    let skipped_tools = skipped_tool_summaries(payload);
    json!({
        "artifact_paths": paths,
        "lineage_required": true,
        "reviewer_feedback_integration": true,
        "verification_center_integration": true,
        "appendix_sections": [
            {
                "section_id": "artifact_inventory",
                "purpose": "List primary artifacts, reports, manifests, and checkpoints used by the manuscript.",
                "required_inputs": ["artifact_paths", "benchmark_plan.artifacts"]
            },
            {
                "section_id": "lineage_trace",
                "purpose": "Link run identifiers, parent runs, change summaries, and artifact paths.",
                "required_inputs": ["result_bundle.run_id", "lineage.history"]
            },
            {
                "section_id": "review_response",
                "purpose": "Track unresolved and resolved reviewer feedback items tied to the current run.",
                "required_inputs": ["reviewer_feedback"]
            },
            {
                "section_id": "verification_gap_log",
                "purpose": "Disclose skipped tools and unresolved verification items for auditability.",
                "required_inputs": ["verification_center_repair.skipped_tools", "runtime_result_verification.missing_items"]
            }
        ],
        "open_feedback": feedback,
        "verification_gaps": gaps,
        "skipped_tools": skipped_tools
    })
}

fn paper_quality_checklist(payload: &Value) -> Vec<Value> {
    let missing_items = verification_missing_items(payload);
    let open_feedback = reviewer_feedback_open_items(payload);
    let skipped_tools = skipped_tool_summaries(payload);
    let appendix_plan = artifact_appendix_plan(payload);
    let appendix_markdown = build_appendix_markdown(&appendix_plan);
    let verification_gaps_disclosed = appendix_discloses_all(&missing_items, &appendix_markdown);
    let skipped_tools_disclosed = appendix_discloses_all(&skipped_tools, &appendix_markdown);
    let evidence_ready = !result_bundle_summary_fields(payload).is_empty();
    let reproducibility_ready = !dataset_mentions(payload).is_empty()
        && !metric_mentions(payload).is_empty()
        && !baseline_mentions(payload).is_empty()
        && !artifact_paths(payload).is_empty();
    vec![
        json!({
            "name": "evidence_grounding",
            "status": if evidence_ready { "satisfied" } else { "needs_attention" },
            "detail": if evidence_ready {
                "The result bundle supplies explicit claim anchors for manuscript grounding."
            } else {
                "No result-bundle claim anchors are available; empirical prose cannot be accepted."
            }
        }),
        json!({
            "name": "reproducibility_reporting",
            "status": if reproducibility_ready { "satisfied" } else { "needs_attention" },
            "detail": if reproducibility_ready {
                "Dataset/workload, metrics, baselines, and artifact locations are attached to the manuscript bundle."
            } else {
                "Reproducibility requires dataset/workload, metrics, baselines, environment/seed details, and artifact locations."
            }
        }),
        json!({
            "name": "source_policy_compliance",
            "status": "satisfied",
            "detail": "Keep paper retrieval on official APIs and dataset retrieval on direct official dataset databases or provider APIs."
        }),
        json!({
            "name": "verification_gap_disclosure",
            "status": if missing_items.is_empty() || verification_gaps_disclosed {
                "satisfied"
            } else {
                "needs_attention"
            },
            "detail": if missing_items.is_empty() {
                "Verification gaps are currently closed or not surfaced by the verifier.".to_string()
            } else if verification_gaps_disclosed {
                format!(
                    "Verification gaps are disclosed in the appendix: {}",
                    missing_items.join(", ")
                )
            } else {
                format!("Disclose unresolved verifier gaps in the paper: {}", missing_items.join(", "))
            }
        }),
        json!({
            "name": "reviewer_feedback_closure",
            "status": if open_feedback.is_empty() { "satisfied" } else { "needs_attention" },
            "detail": if open_feedback.is_empty() {
                "No unresolved reviewer feedback is currently attached to the run.".to_string()
            } else {
                format!("Address open reviewer feedback explicitly: {}", open_feedback.join(" | "))
            }
        }),
        json!({
            "name": "verification_center_bundle_closure",
            "status": if skipped_tools.is_empty() || skipped_tools_disclosed { "satisfied" } else { "needs_attention" },
            "detail": if skipped_tools.is_empty() {
                "No skipped verification-center tool is currently surfaced.".to_string()
            } else if skipped_tools_disclosed {
                format!(
                    "Skipped verification-center tools are disclosed in the appendix: {}",
                    skipped_tools.join(" | ")
                )
            } else {
                format!("Skipped verification-center tools must be recovered or disclosed: {}", skipped_tools.join(" | "))
            }
        }),
    ]
}

fn build_appendix_markdown(plan: &Value) -> String {
    let mut text = String::from("# Artifact Appendix\n\n");
    if let Some(paths) = plan.get("artifact_paths").and_then(Value::as_array) {
        text.push_str("## Artifact Paths\n\n");
        for path in paths {
            if let Some(path) = path.as_str() {
                text.push_str(&format!("- {}\n", path));
            }
        }
        text.push('\n');
    }
    if let Some(gaps) = plan.get("verification_gaps").and_then(Value::as_array) {
        text.push_str("## Verification Gaps\n\n");
        if gaps.is_empty() {
            text.push_str("- none\n\n");
        } else {
            for gap in gaps {
                if let Some(gap) = gap.as_str() {
                    text.push_str(&format!("- {}\n", gap));
                }
            }
            text.push('\n');
        }
    }
    if let Some(skipped_tools) = plan.get("skipped_tools").and_then(Value::as_array) {
        text.push_str("## Skipped Tools\n\n");
        if skipped_tools.is_empty() {
            text.push_str("- none\n\n");
        } else {
            for tool in skipped_tools {
                if let Some(tool) = tool.as_str() {
                    text.push_str(&format!("- {}\n", tool));
                }
            }
            text.push('\n');
        }
    }
    if let Some(sections) = plan.get("appendix_sections").and_then(Value::as_array) {
        text.push_str("## Appendix Sections\n\n");
        for section in sections {
            let section_id = section
                .get("section_id")
                .and_then(Value::as_str)
                .unwrap_or("section");
            let purpose = section.get("purpose").and_then(Value::as_str).unwrap_or("");
            text.push_str(&format!("### {}\n\n{}\n\n", section_id, purpose));
        }
    }
    text
}

fn manuscript_master_prompt(payload: &Value, blueprint: &Value) -> String {
    let profile = benchmark_profile(payload);
    let title_hint = cleaned_string(blueprint.get("title_hint"))
        .if_empty_then("Evidence-Grounded Computer Science Study");
    let sections = PAPER_SECTION_SPECS
        .iter()
        .map(|spec| format!("{} [{} words]", spec.title, spec.target_words))
        .collect::<Vec<_>>()
        .join(" -> ");
    let result_highlights = result_bundle_summary_fields(payload);
    let gaps = verification_missing_items(payload);
    let feedback = reviewer_feedback_open_items(payload);
    let repair_actions = repair_next_actions(payload);

    format!(
        "You are writing a complete, experiment-grounded academic paper for a {} workflow. Working title: {}. Produce a format-correct manuscript with the following ordered modules: {}. Use only evidence present in the workflow payload. Strongest result anchors: {}. Open verification gaps: {}. Open reviewer feedback: {}. Repair actions to reflect in discussion or limitations: {}. Hard constraints: (1) every empirical claim must map to result_bundle, lineage, benchmark_plan, or verifier evidence; (2) cite only retrieved or explicitly supplied literature; (3) keep paper retrieval on official APIs; (4) use datasets only from direct official dataset databases or provider APIs and never treat dataset search as a paper source; (5) if evidence is missing, disclose the gap in limitations instead of inventing support. Final deliverable: a high-quality CS paper with title, abstract, introduction, related work, method, experimental setup, results, discussion, limitations, conclusion, references, and reproducibility appendix.",
        profile.replace('_', " "),
        title_hint,
        sections,
        join_limited(&result_highlights, 4, "result evidence pending"),
        join_limited(&gaps, 4, "none surfaced"),
        join_limited(&feedback, 3, "none"),
        join_limited(&repair_actions, 3, "none")
    )
}

fn reviewer_feedback_trace(payload: &Value) -> Vec<Value> {
    payload
        .get("reviewer_feedback")
        .and_then(Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .enumerate()
                .map(|(index, entry)| {
                    let comment = cleaned_string(entry.get("comment"));
                    let normalized = comment.to_ascii_lowercase();
                    let mut target_sections = Vec::new();
                    if normalized.contains("abstract") || normalized.contains("claim") {
                        target_sections.push("title_abstract".to_string());
                    }
                    if normalized.contains("intro") || normalized.contains("motivation") {
                        target_sections.push("introduction".to_string());
                    }
                    if normalized.contains("related") || normalized.contains("citation") {
                        target_sections.push("related_work".to_string());
                    }
                    if normalized.contains("method") || normalized.contains("implementation") {
                        target_sections.push("method".to_string());
                    }
                    if normalized.contains("split")
                        || normalized.contains("dataset")
                        || normalized.contains("setup")
                        || normalized.contains("benchmark")
                    {
                        target_sections.push("experimental_setup".to_string());
                    }
                    if normalized.contains("result")
                        || normalized.contains("metric")
                        || normalized.contains("table")
                        || normalized.contains("figure")
                    {
                        target_sections.push("results".to_string());
                    }
                    if normalized.contains("limitation") || normalized.contains("threat") {
                        target_sections.push("limitations".to_string());
                    }
                    if normalized.contains("rebuttal") || normalized.contains("response") {
                        target_sections.push("references_appendix".to_string());
                    }
                    if target_sections.is_empty() {
                        target_sections.push("discussion".to_string());
                    }
                    let resolved = entry
                        .get("resolved")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    json!({
                        "feedback_index": index,
                        "reviewer": cleaned_string(entry.get("reviewer")),
                        "linked_run_id": cleaned_string(entry.get("linked_run_id")),
                        "score": entry.get("score").cloned().unwrap_or(Value::Null),
                        "comment": comment,
                        "target_sections": target_sections,
                        "reverification_required": !resolved,
                        "closure_state": if resolved { "resolved" } else { "open" },
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn evidence_trace(payload: &Value) -> Vec<Value> {
    let result_fields = result_bundle_summary_entries(payload);
    let profile = benchmark_profile(payload);
    let mut items = vec![
        json!({
            "claim_id": "results_primary_claim",
            "section_id": "results",
            "claim": join_limited(&result_bundle_summary_fields(payload), 2, "primary result evidence pending"),
            "evidence_sources": ["result_bundle.summary_fields", "run_comparison", "lineage"],
            "profile": profile,
        }),
        json!({
            "claim_id": "setup_reproducibility_claim",
            "section_id": "experimental_setup",
            "claim": join_limited(&dataset_mentions(payload), 2, "dataset acquisition pending"),
            "evidence_sources": ["benchmark_plan.datasets", "benchmark_plan.reproducibility", "artifact_paths"],
            "profile": profile,
        }),
    ];
    if let Some((name, value)) = result_fields.first() {
        items.push(json!({
            "claim_id": "abstract_anchor",
            "section_id": "title_abstract",
            "claim": format!("{}: {}", name, value),
            "evidence_sources": ["result_bundle.summary_fields"],
            "profile": profile,
        }));
    }
    items
}

fn revision_plan(payload: &Value) -> Value {
    let trace = reviewer_feedback_trace(payload);
    let repair_actions = repair_next_actions(payload);
    let verification_gaps = verification_missing_items(payload);
    let queue = trace
        .iter()
        .filter(|entry| {
            entry.get("closure_state")
                .and_then(Value::as_str)
                .is_some_and(|state| state.eq_ignore_ascii_case("open"))
        })
        .map(|entry| {
            let target_sections = entry
                .get("target_sections")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let touches_results = target_sections.iter().any(|section| {
                section
                    .as_str()
                    .is_some_and(|name| matches!(name, "results" | "experimental_setup" | "method" | "title_abstract"))
            });
            json!({
                "feedback_index": entry.get("feedback_index").cloned().unwrap_or(Value::Null),
                "reviewer": entry.get("reviewer").cloned().unwrap_or(Value::Null),
                "linked_run_id": entry.get("linked_run_id").cloned().unwrap_or(Value::Null),
                "comment": entry.get("comment").cloned().unwrap_or(Value::Null),
                "target_sections": target_sections,
                "rewrite_actions": [
                    "update section prose to answer the reviewer comment",
                    "preserve claim-to-evidence grounding in the edited sections",
                    "sync the rebuttal entry with the edited sections"
                ],
                "reverification_required": touches_results,
                "reverification_scope": if touches_results {
                    json!(["runtime_result_verification", "verification_center_repair", "paper_ready_gate"])
                } else {
                    json!(["paper_ready_gate"])
                }
            })
        })
        .collect::<Vec<_>>();
    json!({
        "mode": if queue.is_empty() { "fresh_draft_or_closed_feedback" } else { "reviewer_guided_revision" },
        "section_rewrite_queue": queue,
        "shared_repair_actions": repair_actions,
        "open_verification_gaps": verification_gaps,
    })
}

fn rebuttal_closure_records(payload: &Value) -> Vec<Value> {
    reviewer_feedback_trace(payload)
        .into_iter()
        .map(|entry| {
            let target_sections = entry
                .get("target_sections")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let open = entry
                .get("closure_state")
                .and_then(Value::as_str)
                .is_some_and(|state| state.eq_ignore_ascii_case("open"));
            json!({
                "feedback_index": entry.get("feedback_index").cloned().unwrap_or(Value::Null),
                "reviewer": entry.get("reviewer").cloned().unwrap_or(Value::Null),
                "comment": entry.get("comment").cloned().unwrap_or(Value::Null),
                "target_sections": target_sections,
                "response_status": if open { "pending_revision" } else { "resolved" },
                "required_followup": if open {
                    "Revise the targeted sections, rerun verification if empirical claims changed, then update the rebuttal item."
                } else {
                    "Keep the resolved response in the final rebuttal and appendix bundle."
                }
            })
        })
        .collect()
}

fn markdown_draft(payload: &Value, blueprint: &Value) -> String {
    let title = cleaned_string(blueprint.get("title_hint"))
        .if_empty_then("Evidence-Grounded Computer Science Study");
    let mut out = format!(
        "# {}\n\n## Abstract\n\n{}\n\n",
        title,
        abstract_draft(payload)
    );
    for spec in PAPER_SECTION_SPECS {
        if spec.id == "title_abstract" {
            continue;
        }
        out.push_str(&format!(
            "## {}\n\n{}\n\n",
            spec.title,
            section_seed_text(spec, payload)
        ));
    }
    out
}

fn latex_paragraphs(text: &str) -> String {
    text.split("\n\n")
        .map(str::trim)
        .filter(|paragraph| !paragraph.is_empty())
        .map(latex_escape)
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn latex_result_table(payload: &Value) -> String {
    let rows = result_bundle_summary_entries(payload)
        .into_iter()
        .filter(|(name, value)| {
            !name.trim().is_empty()
                && !value.trim().is_empty()
                && !value.to_ascii_lowercase().contains("pending")
        })
        .take(8)
        .map(|(name, value)| {
            format!(
                "{} & {} \\\\\n",
                latex_escape(&name.replace('_', " ")),
                latex_escape(&value)
            )
        })
        .collect::<String>();
    if rows.is_empty() {
        return String::new();
    }
    format!(
        "\\begin{{table}}[t]\n  \\centering\n  \\caption{{Auditable summary of the current result bundle.}}\n  \\label{{tab:result-bundle}}\n  \\begin{{tabularx}}{{\\linewidth}}{{@{{}}p{{0.29\\linewidth}}X@{{}}}}\n    \\toprule\n    \\textbf{{Field}} & \\textbf{{Recorded value}} \\\\\n    \\midrule\n{}    \\bottomrule\n  \\end{{tabularx}}\n\\end{{table}}\n",
        rows
    )
}

fn latex_reproducibility_table(payload: &Value) -> String {
    let datasets = prose_join_limited(&dataset_mentions(payload), 3, "not recorded");
    let metrics = prose_join_limited(&metric_mentions(payload), 4, "not recorded");
    let baselines = prose_join_limited(&baseline_mentions(payload), 4, "not recorded");
    let artifacts = prose_join_limited(&artifact_paths(payload), 3, "not recorded");
    let rows = [
        ("Dataset / workload", datasets),
        ("Metrics", metrics),
        ("Baselines", baselines),
        ("Artifact anchors", artifacts),
    ]
    .into_iter()
    .map(|(label, value)| format!("{} & {} \\\\\n", latex_escape(label), latex_escape(&value)))
    .collect::<String>();
    format!(
        "\\begin{{table}}[t]\n  \\centering\n  \\caption{{Reproducibility anchors fixed by the workflow.}}\n  \\label{{tab:reproducibility}}\n  \\begin{{tabularx}}{{\\linewidth}}{{@{{}}p{{0.25\\linewidth}}X@{{}}}}\n    \\toprule\n    \\textbf{{Component}} & \\textbf{{Recorded configuration or evidence}} \\\\\n    \\midrule\n{}    \\bottomrule\n  \\end{{tabularx}}\n\\end{{table}}\n",
        rows
    )
}

fn latex_evidence_figure(payload: &Value) -> String {
    let run_id = result_field_value(payload, "run_id").if_empty_then("current run");
    format!(
        "\\begin{{figure}}[t]\n  \\centering\n  \\begin{{tikzpicture}}[x=1cm,y=1cm,>=Latex,font=\\sffamily\\footnotesize]\n    \\tikzset{{stage/.style={{draw=AtlasRule,rounded corners=2pt,fill=AtlasSoft,minimum width=2.75cm,minimum height=1.0cm,align=center,inner sep=5pt}}}}\n    \\node[stage] (input) at (0,0) {{Evidence inputs\\\\dataset / literature}};\n    \\node[stage] (method) at (3.55,0) {{Executable method\\\\fixed configuration}};\n    \\node[stage] (verify) at (7.10,0) {{Verification\\\\tests and audit}};\n    \\node[stage] (report) at (10.65,0) {{Research output\\\\{}}};\n    \\draw[->,very thick,AtlasAccent] (input) -- (method);\n    \\draw[->,very thick,AtlasAccent] (method) -- (verify);\n    \\draw[->,very thick,AtlasAccent] (verify) -- (report);\n  \\end{{tikzpicture}}\n  \\caption{{Evidence flow used by the research workflow. The diagram documents provenance and does not introduce quantitative evidence.}}\n  \\label{{fig:evidence-flow}}\n\\end{{figure}}\n",
        latex_escape(&run_id)
    )
}

fn latex_outline(payload: &Value, blueprint: &Value) -> String {
    let title_hint = cleaned_string(blueprint.get("title_hint"))
        .if_empty_then("Evidence-Grounded Computer Science Study");
    let has_citations = !citation_inventory(payload).is_empty();
    let mut body = String::new();
    for spec in PAPER_SECTION_SPECS {
        if spec.id == "title_abstract" {
            continue;
        }
        let section_artifact = match spec.id {
            "experimental_setup" => latex_reproducibility_table(payload),
            "results" => latex_result_table(payload),
            _ => String::new(),
        };
        body.push_str(&format!(
            "\\section{{{}}}\n{}\n\n{}",
            latex_escape(spec.title),
            latex_paragraphs(&section_seed_text(spec, payload)),
            section_artifact
        ));
    }

    format!(
        "\\documentclass[10pt]{{article}}\n\\usepackage[a4paper,top=18mm,bottom=20mm,left=19mm,right=19mm]{{geometry}}\n\\usepackage[T1]{{fontenc}}\n\\usepackage{{lmodern}}\n\\usepackage{{booktabs}}\n\\usepackage{{tabularx}}\n\\usepackage{{array}}\n\\usepackage{{microtype}}\n\\usepackage{{graphicx}}\n\\usepackage{{tikz}}\n\\usetikzlibrary{{arrows.meta,positioning}}\n\\usepackage{{xcolor}}\n\\usepackage{{caption}}\n\\usepackage{{fancyhdr}}\n\\usepackage{{titlesec}}\n\\usepackage{{enumitem}}\n\\usepackage{{xurl}}\n\\definecolor{{AtlasInk}}{{HTML}}{{20242B}}\n\\definecolor{{AtlasAccent}}{{HTML}}{{B8521F}}\n\\definecolor{{AtlasRule}}{{HTML}}{{C8CDD5}}\n\\definecolor{{AtlasSoft}}{{HTML}}{{F2F4F7}}\n\\usepackage[colorlinks=true,linkcolor=AtlasAccent,citecolor=AtlasAccent,urlcolor=AtlasAccent]{{hyperref}}\n\\captionsetup{{font=small,labelfont={{bf,color=AtlasAccent}},skip=5pt}}\n\\titleformat{{\\section}}{{\\large\\bfseries\\color{{AtlasInk}}}}{{\\thesection}}{{0.65em}}{{}}[\\vspace{{-0.35em}}\\color{{AtlasRule}}\\titlerule]\n\\titleformat{{\\subsection}}{{\\normalsize\\bfseries\\color{{AtlasInk}}}}{{\\thesubsection}}{{0.55em}}{{}}\n\\titlespacing*{{\\section}}{{0pt}}{{1.2em}}{{0.65em}}\n\\setlength{{\\parindent}}{{0pt}}\n\\setlength{{\\parskip}}{{0.52em}}\n\\setlength{{\\emergencystretch}}{{2em}}\n\\setlist{{nosep,leftmargin=1.35em}}\n\\urlstyle{{same}}\n\\pagestyle{{fancy}}\n\\fancyhf{{}}\n\\fancyhead[L]{{\\small\\color{{AtlasAccent}} Evidence-Grounded Research}}\n\\fancyhead[R]{{\\small\\color{{AtlasInk}} Atlas AI Scientist}}\n\\fancyfoot[C]{{\\small\\thepage}}\n\\renewcommand{{\\headrulewidth}}{{0.3pt}}\n\\title{{\\vspace{{-1.4em}}\\bfseries\\color{{AtlasInk}} {}}}\n\\author{{Atlas AI Scientist \\quad | \\quad Reproducible Research Workflow}}\n\\date{{}}\n\\begin{{document}}\n\\maketitle\n\\vspace{{-1.1em}}\n\\begin{{abstract}}\n{}\n\\end{{abstract}}\n{}\n{}{}\\end{{document}}\n",
        latex_escape(&title_hint),
        latex_paragraphs(&abstract_draft(payload)),
        latex_evidence_figure(payload),
        body,
        if has_citations {
            "\\nocite{*}\n\\bibliographystyle{plain}\n\\bibliography{references}\n"
        } else {
            "\\section*{References}\nNo retrieved references were available for this draft.\n"
        }
    )
}

fn completion_protocol(payload: &Value) -> Value {
    json!({
        "stages": [
            {
                "stage_id": "assemble_blueprint",
                "objective": "Freeze section order, evidence inputs, and output contracts before prose polishing."
            },
            {
                "stage_id": "draft_modules",
                "objective": "Write each module with its section prompt and skill contract, keeping claims evidence-bounded."
            },
            {
                "stage_id": "review_tables_figures",
                "objective": "Ensure every figure and table is supported by result_bundle, run_comparison, or lineage data."
            },
            {
                "stage_id": "close_feedback",
                "objective": "Resolve reviewer feedback or explicitly disclose unresolved comments in discussion and appendix."
            },
            {
                "stage_id": "final_package",
                "objective": "Emit manuscript text, LaTeX shell, citation inventory, appendix plan, and quality checklist."
            }
        ],
        "final_artifacts": [
            "paper.tex",
            "paper.md",
            "references.bib",
            "artifact_appendix.md",
            "result_bundle.json",
            "review_response.json"
        ],
        "review_readiness": {
            "open_reviewer_feedback_count": reviewer_feedback_open_items(payload).len(),
            "verification_gap_count": verification_missing_items(payload).len(),
            "skipped_tool_count": skipped_tool_summaries(payload).len()
        }
    })
}

#[async_trait]
impl Agent for ReportAgent {
    fn id(&self) -> &str {
        &self.id
    }

    fn role(&self) -> AgentRole {
        AgentRole::Reporter
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![
            Capability {
                name: "technical_report_generation".into(),
                description: "Generate a complete CS paper bundle with manuscript prompts, section skills, LaTeX scaffolding, and evidence-grounded delivery contracts.".into(),
                required_tools: vec![
                    "generate_latex".into(),
                    "format_citations".into(),
                    "fetch_paper".into(),
                    "fetch_papers".into(),
                ],
            },
            Capability {
                name: "section_prompt_orchestration".into(),
                description: "Design section-specific prompts and skills for title, abstract, method, experiments, results, discussion, and appendix modules.".into(),
                required_tools: vec![
                    "summarize_text".into(),
                    "extract_entities".into(),
                    "format_citations".into(),
                ],
            },
            Capability {
                name: "manuscript_bundle_assembly".into(),
                description: "Assemble draft sections, tables and figures plans, citation inventory, appendix structure, and completion protocol for a final paper.".into(),
                required_tools: vec![
                    "generate_latex".into(),
                    "summarize_text".into(),
                    "format_citations".into(),
                ],
            },
        ]
    }

    async fn handle_message(
        &self,
        msg: AgentMessage,
        _ctx: &AgentContext,
    ) -> Result<AgentResponse, AgentError> {
        let blueprint = manuscript_blueprint(&msg.payload);
        let section_prompt_pack = blueprint
            .get("sections")
            .cloned()
            .unwrap_or_else(|| json!([]));
        let latex = latex_outline(&msg.payload, &blueprint);
        let markdown = markdown_draft(&msg.payload, &blueprint);
        let draft_sections = draft_sections(&msg.payload);
        let skill_pack = section_skill_pack();
        let tables_and_figures = tables_figures_plan(&msg.payload);
        let citations = citation_inventory(&msg.payload);
        let appendix_plan = artifact_appendix_plan(&msg.payload);
        let completion = completion_protocol(&msg.payload);
        let title = cleaned_string(blueprint.get("title_hint"));
        let paper_sections = PAPER_SECTION_SPECS
            .iter()
            .map(|spec| spec.title.to_string())
            .collect::<Vec<_>>();

        Ok(AgentResponse::ok(json!({
            "agent": self.id,
            "status": "Paper manuscript bundle generated",
            "paper": {
                "schema_version": PAPER_SCHEMA_VERSION,
                "manuscript_bundle_schema_version": MANUSCRIPT_BUNDLE_SCHEMA_VERSION,
                "title": title,
                "abstract": abstract_draft(&msg.payload),
                "sections": paper_sections,
                "format": "latex",
                "target_venue": blueprint["target_venue"].clone(),
                "manuscript_master_prompt": manuscript_master_prompt(&msg.payload, &blueprint),
                "section_prompt_pack": section_prompt_pack,
                "section_skill_pack": skill_pack,
                "draft_sections": draft_sections,
                "markdown_draft": markdown,
                "latex_outline": latex,
                "latex_manuscript_shell": latex,
                "tables_figures_plan": tables_and_figures,
                "citation_inventory": citations,
                "reviewer_feedback_trace": reviewer_feedback_trace(&msg.payload),
                "evidence_trace": evidence_trace(&msg.payload),
                "revision_plan": revision_plan(&msg.payload),
                "rebuttal_closure_records": rebuttal_closure_records(&msg.payload),
                "quality_checklist": paper_quality_checklist(&msg.payload),
                "artifact_appendix_plan": appendix_plan,
                "completion_protocol": completion
            },
            "paper_blueprint": blueprint
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dataset_mentions_prefer_benchmark_datasets_over_stale_hint_only_entries() {
        let payload = json!({
            "problem_formulation": "Subsampling robustness of tree ensembles under label noise",
            "paper_dataset_hints": ["iris"],
            "benchmark_plan": {
                "benchmark_profile": "classical_ml",
                "datasets": [
                    {
                        "dataset_id": "digits",
                        "provider": "sklearn",
                        "path": "sklearn.datasets.load_digits",
                        "task_hint": "classification",
                        "split_hint": "train_test_split, test_size=0.3, random_state=42, stratified"
                    }
                ],
                "metrics": [
                    { "name": "accuracy_mean", "direction": "maximize" }
                ]
            },
            "result_bundle": {
                "summary_fields": [
                    { "name": "run_id", "value": "classical_ml-run-13" }
                ]
            }
        });

        let mentions = dataset_mentions(&payload);
        assert_eq!(mentions.len(), 1);
        assert!(mentions[0].contains("digits"));
        assert!(!mentions
            .iter()
            .any(|entry| entry.eq_ignore_ascii_case("iris")));

        let abstract_text = abstract_draft(&payload);
        assert!(abstract_text.contains("digits"));
        assert!(!abstract_text.contains("iris;"));
        assert!(!abstract_text.contains(" grounded in iris"));
    }

    #[test]
    fn title_abstract_claim_anchor_exposes_fact_grounding_text() {
        let payload = json!({
            "problem_formulation": "Subsampling robustness of tree ensembles under label noise",
            "benchmark_plan": {
                "benchmark_profile": "classical_ml",
                "datasets": [
                    {
                        "dataset_id": "digits",
                        "provider": "sklearn",
                        "path": "sklearn.datasets.load_digits",
                        "task_hint": "classification",
                        "split_hint": "train_test_split, test_size=0.3, random_state=42, stratified"
                    }
                ],
                "metrics": [
                    { "name": "accuracy", "direction": "maximize" }
                ]
            },
            "result_bundle": {
                "summary_fields": [
                    { "name": "run_id", "value": "classical_ml-run-13" },
                    { "name": "primary_metric", "value": "0.9793" }
                ]
            },
            "runtime_result_verification": {
                "missing_items": ["metric_reports"]
            }
        });

        let spec = &PAPER_SECTION_SPECS[0];
        let anchors = section_claim_anchors(spec, &payload);
        assert_eq!(anchors.len(), 1);
        let grounding_text = anchors[0]
            .get("grounding_text")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(grounding_text.contains("We study Subsampling robustness"));
        assert!(grounding_text.contains("digits"));
        assert!(grounding_text.contains("metric_reports"));
    }

    #[test]
    fn paper_quality_checklist_marks_disclosed_skipped_tools_as_satisfied() {
        let payload = json!({
            "verification_center_repair": {
                "skipped_tools": [
                    { "tool": "pytest", "reason": "tool unavailable or not runnable for this workspace" }
                ]
            },
            "runtime_result_verification": {
                "missing_items": []
            },
            "reviewer_feedback": []
        });

        let checklist = paper_quality_checklist(&payload);
        let item = checklist
            .iter()
            .find(|entry| entry["name"] == "verification_center_bundle_closure")
            .expect("bundle closure item");
        assert_eq!(item["status"], json!("satisfied"));
        assert!(item["detail"]
            .as_str()
            .unwrap_or("")
            .contains("disclosed in the appendix"));
    }

    #[test]
    fn related_work_claim_anchor_limits_required_evidence_to_localizable_items() {
        let payload = json!({
            "benchmark_plan": {
                "datasets": [
                    {
                        "dataset_id": "digits",
                        "provider": "sklearn",
                        "path": "sklearn.datasets.load_digits",
                        "task_hint": "classification",
                        "split_hint": "train_test_split, test_size=0.3, random_state=42, stratified"
                    }
                ],
                "metrics": [
                    { "name": "f1", "direction": "maximize" },
                    { "name": "accuracy", "direction": "maximize" }
                ],
                "baselines": [
                    { "name": "RandomForest", "kind": "reproducible_baseline" },
                    { "name": "Bagging(s=0.3)", "kind": "subsample_or_ensemble_ablation" },
                    { "name": "Bagging(s=0.5)", "kind": "subsample_or_ensemble_ablation" }
                ]
            },
            "literature_evidence": [
                { "title": "A Survey of Ensemble Learning: Concepts, Algorithms, Applications, and Prospects" },
                { "title": "Ensemble Perception" },
                { "title": "Gradient boosting machines, a tutorial" }
            ]
        });

        let spec = PAPER_SECTION_SPECS
            .iter()
            .find(|spec| spec.id == "related_work")
            .expect("related_work spec");
        let anchors = section_claim_anchors(spec, &payload);
        assert_eq!(anchors.len(), 1);
        let refs = anchors[0]["evidence_refs"]
            .as_array()
            .expect("evidence refs");
        let required = refs
            .iter()
            .filter(|entry| entry["required"].as_bool().unwrap_or(false))
            .collect::<Vec<_>>();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0]["source_key"], json!("literature_evidence"));
        let item_count = required[0]["items"].as_array().map(|items| items.len());
        assert!(matches!(item_count, Some(count) if count >= 1 && count <= 2));
    }

    #[test]
    fn paper_quality_checklist_marks_disclosed_verification_gaps_as_satisfied() {
        let payload = json!({
            "verification_center_repair": {
                "skipped_tools": []
            },
            "runtime_result_verification": {
                "missing_items": ["metric_reports"]
            },
            "reviewer_feedback": []
        });

        let checklist = paper_quality_checklist(&payload);
        let item = checklist
            .iter()
            .find(|entry| entry["name"] == "verification_gap_disclosure")
            .expect("verification gap disclosure item");
        assert_eq!(item["status"], json!("satisfied"));
        assert!(item["detail"]
            .as_str()
            .unwrap_or("")
            .contains("disclosed in the appendix"));
    }
}
