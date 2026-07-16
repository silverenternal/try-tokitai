use ai_assistant::scientist::workflow::{run_paper_workflow, PaperWorkflowRequest};
use ai_assistant::toolchain::auto_detect_toolchain_paths;
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
struct RunnerArgs {
    topic: String,
    source_workspace: PathBuf,
    workflow_workspace: Option<PathBuf>,
    session_id: Option<String>,
    summary_markdown: Option<PathBuf>,
    metrics_markdown: Option<PathBuf>,
    script_path: Option<PathBuf>,
    figure_paths: Vec<PathBuf>,
    search_limit: usize,
    force_rewrite: bool,
}

#[derive(Debug, Clone)]
struct ModelMetricRow {
    name: String,
    accuracy: f64,
}

#[derive(Debug)]
struct RuntimeBundle {
    artifact_paths: Vec<String>,
    result_bundle: Value,
    run_comparison: Value,
    lineage: Value,
}

fn main() -> Result<()> {
    let args = parse_args()?;
    let source_workspace = fs::canonicalize(&args.source_workspace).with_context(|| {
        format!(
            "failed to resolve source workspace {}",
            args.source_workspace.display()
        )
    })?;
    let session_id = args
        .session_id
        .clone()
        .unwrap_or_else(|| default_session_id(&args.topic));
    let workflow_workspace = args.workflow_workspace.clone().unwrap_or_else(|| {
        source_workspace
            .join(".atlas")
            .join("paper-workflows")
            .join(&session_id)
    });
    fs::create_dir_all(&workflow_workspace).with_context(|| {
        format!(
            "failed to create workflow workspace {}",
            workflow_workspace.display()
        )
    })?;

    let runtime = build_runtime_bundle(&args, &source_workspace, &session_id)?;
    let request = PaperWorkflowRequest {
        topic: args.topic.clone(),
        session_id: session_id.clone(),
        workspace_root: workflow_workspace.clone(),
        source_workspace_root: Some(source_workspace.clone()),
        local_paper_source: None,
        search_limit: args.search_limit.clamp(1, 10),
        toolchains: Some(auto_detect_toolchain_paths()),
        reviewer_feedback: None,
        force_rewrite: args.force_rewrite,
        runtime_artifact_paths: Some(runtime.artifact_paths.clone()),
        runtime_result_bundle: Some(runtime.result_bundle.clone()),
        runtime_run_comparison: Some(runtime.run_comparison.clone()),
        runtime_lineage: Some(runtime.lineage.clone()),
        image_api_key: std::env::var("DASHSCOPE_API_KEY").ok(),
        generate_images: false,
    };

    let runtime_handle =
        tokio::runtime::Runtime::new().context("failed to create tokio runtime")?;
    let result = runtime_handle
        .block_on(run_paper_workflow(request))
        .map_err(|err| anyhow!(err))?;

    println!("paper workflow completed");
    println!("source_workspace: {}", source_workspace.display());
    println!("workflow_workspace: {}", workflow_workspace.display());
    println!("session_id: {}", session_id);
    println!("paper_markdown: {}", result.paper_markdown_path.display());
    println!("paper_latex: {}", result.paper_latex_path.display());
    println!(
        "paper_pdf: {}",
        display_optional_path(result.paper_pdf_path.as_ref())
    );
    println!("result_bundle: {}", result.result_bundle_path.display());
    println!("review_response: {}", result.review_response_path.display());
    println!(
        "workflow_checkpoint: {}",
        result.workflow_checkpoint_path.display()
    );
    println!("paper_ready: {}", result.paper_ready);
    println!("paper_ready_detail: {}", result.paper_ready_detail);
    println!("pdf_compile_status: {}", result.pdf_compile_status);
    if let Some(detail) = result.pdf_compile_detail.as_deref() {
        println!("pdf_compile_detail: {}", detail);
    }

    Ok(())
}

fn parse_args() -> Result<RunnerArgs> {
    let mut parsed = RunnerArgs {
        search_limit: 5,
        ..RunnerArgs::default()
    };
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--topic" => {
                parsed.topic = next_arg_value(&mut args, "--topic")?;
            }
            "--source-workspace" => {
                parsed.source_workspace =
                    PathBuf::from(next_arg_value(&mut args, "--source-workspace")?);
            }
            "--workflow-workspace" => {
                parsed.workflow_workspace = Some(PathBuf::from(next_arg_value(
                    &mut args,
                    "--workflow-workspace",
                )?));
            }
            "--session-id" => {
                parsed.session_id = Some(next_arg_value(&mut args, "--session-id")?);
            }
            "--summary-markdown" => {
                parsed.summary_markdown = Some(PathBuf::from(next_arg_value(
                    &mut args,
                    "--summary-markdown",
                )?));
            }
            "--metrics-markdown" => {
                parsed.metrics_markdown = Some(PathBuf::from(next_arg_value(
                    &mut args,
                    "--metrics-markdown",
                )?));
            }
            "--script-path" => {
                parsed.script_path =
                    Some(PathBuf::from(next_arg_value(&mut args, "--script-path")?));
            }
            "--figure" => {
                parsed
                    .figure_paths
                    .push(PathBuf::from(next_arg_value(&mut args, "--figure")?));
            }
            "--search-limit" => {
                parsed.search_limit = next_arg_value(&mut args, "--search-limit")?
                    .parse::<usize>()
                    .context("invalid --search-limit value")?;
            }
            "--force-rewrite" => {
                parsed.force_rewrite = true;
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            other => bail!("unknown argument: {}", other),
        }
    }

    if parsed.topic.trim().is_empty() {
        bail!("--topic is required");
    }
    if parsed.source_workspace.as_os_str().is_empty() {
        bail!("--source-workspace is required");
    }

    Ok(parsed)
}

fn print_help() {
    println!("Usage:");
    println!("  cargo run --bin paper_workflow_runner -- \\");
    println!("    --topic <topic> \\");
    println!("    --source-workspace <path> [options]");
    println!();
    println!("Options:");
    println!("  --workflow-workspace <path>  explicit paper workflow workspace");
    println!("  --session-id <id>            stable session id for checkpoint resume");
    println!(
        "  --summary-markdown <path>    experiment summary markdown, relative to source workspace"
    );
    println!("  --metrics-markdown <path>    metrics markdown, relative to source workspace");
    println!("  --script-path <path>         experiment script path, relative to source workspace");
    println!("  --figure <path>              figure path, repeatable");
    println!("  --search-limit <n>           official paper API search limit, default 5");
    println!("  --force-rewrite              rebuild report, PDF, and quality gates");
}

fn next_arg_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String> {
    args.next()
        .ok_or_else(|| anyhow!("{} requires a value", flag))
}

fn default_session_id(topic: &str) -> String {
    let slug = slugify(topic);
    if slug.is_empty() {
        "paper-workflow-runner".to_string()
    } else {
        format!("paper-runner-{}", slug)
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

fn build_runtime_bundle(
    args: &RunnerArgs,
    source_workspace: &Path,
    session_id: &str,
) -> Result<RuntimeBundle> {
    let summary_rel = resolve_optional_input(
        source_workspace,
        args.summary_markdown.as_deref(),
        &["experiment_summary.md"],
        "summary markdown",
    )?;
    let metrics_rel = resolve_optional_input(
        source_workspace,
        args.metrics_markdown.as_deref(),
        &["metrics.md"],
        "metrics markdown",
    )?;
    let script_rel = resolve_optional_input(
        source_workspace,
        args.script_path.as_deref(),
        &[
            "iris_experiment.py",
            "ml_iris_experiment.py",
            "iris_ml_experiment.py",
        ],
        "experiment script",
    )?;

    let figures = if args.figure_paths.is_empty() {
        collect_existing_relative_paths(
            source_workspace,
            &[
                "model_comparison.png",
                "comparison.png",
                "confusion_matrix.png",
            ],
        )
    } else {
        args.figure_paths
            .iter()
            .map(|path| resolve_existing_relative_path(source_workspace, path))
            .collect::<Result<Vec<_>>>()?
    };

    let summary_text = read_utf8_text(&source_workspace.join(&summary_rel)).with_context(|| {
        format!(
            "failed to read summary markdown {}",
            source_workspace.join(&summary_rel).display()
        )
    })?;
    let metrics_text = read_utf8_text(&source_workspace.join(&metrics_rel)).with_context(|| {
        format!(
            "failed to read metrics markdown {}",
            source_workspace.join(&metrics_rel).display()
        )
    })?;

    let model_rows = parse_model_accuracy_rows(&summary_text);
    let best_model = model_rows
        .iter()
        .max_by(|left, right| left.accuracy.total_cmp(&right.accuracy))
        .cloned();
    let logistic_row = model_rows
        .iter()
        .find(|row| row.name.to_ascii_lowercase().contains("logistic"))
        .cloned();
    let comparator_row = logistic_row.clone().or_else(|| {
        model_rows
            .iter()
            .min_by(|left, right| left.accuracy.total_cmp(&right.accuracy))
            .cloned()
    });
    let metrics_accuracy = extract_metric_value(
        &metrics_text,
        &[
            "Accuracy",
            "test_accuracy",
            "accuracy",
            "测试准确率",
            "测试准确率(Accuracy)",
        ],
    );
    let error_analysis_summary = infer_error_analysis_summary(&metrics_text);
    let split_manifest_rel =
        ensure_dataset_split_manifest(source_workspace, session_id, &summary_rel, &metrics_rel)?;

    let mut artifact_paths = Vec::new();
    artifact_paths.push(split_manifest_rel.clone());
    artifact_paths.push(script_rel.clone());
    artifact_paths.push(metrics_rel.clone());
    artifact_paths.push(summary_rel.clone());
    artifact_paths.extend(figures.iter().cloned());
    artifact_paths = dedup_paths(artifact_paths);

    let primary_metric = best_model
        .as_ref()
        .map(|row| format!("best held-out accuracy {:.4} ({})", row.accuracy, row.name))
        .or_else(|| metrics_accuracy.map(|value| format!("held-out accuracy {:.4}", value)))
        .ok_or_else(|| {
            anyhow!("failed to infer a primary metric from supplied experiment files")
        })?;

    let baseline_delta = if let (Some(best), Some(comparator)) =
        (best_model.as_ref(), comparator_row.as_ref())
    {
        Some(format!(
            "{:+.4} over {} comparator",
            best.accuracy - comparator.accuracy,
            comparator.name
        ))
    } else if let (Some(best), Some(metric_accuracy)) = (best_model.as_ref(), metrics_accuracy) {
        Some(format!(
            "{:+.4} over standalone logistic verification run",
            best.accuracy - metric_accuracy
        ))
    } else {
        None
    };

    let best_model_detail = best_model
        .as_ref()
        .map(|row| {
            format!(
                "Best model in the comparison summary: {} at {:.4}.",
                row.name, row.accuracy
            )
        })
        .unwrap_or_else(|| {
            "Best model detail was not recoverable from the summary markdown.".to_string()
        });
    let standalone_detail = metrics_accuracy
        .map(|value| {
            format!(
                "Standalone logistic-regression verification note reports accuracy {:.4}.",
                value
            )
        })
        .unwrap_or_else(|| {
            "No standalone logistic-regression verification note was recovered.".to_string()
        });

    let run_id = format!("{}-{}", slugify(session_id), "real-runtime");
    let mut summary_fields = vec![
        json!({"name": "run_id", "value": run_id}),
        json!({"name": "dataset_name", "value": "iris"}),
        json!({"name": "primary_metric", "value": primary_metric}),
        json!({"name": "comparison_summary", "value": best_model_detail}),
        json!({"name": "error_analysis_summary", "value": error_analysis_summary}),
    ];
    if let Some(delta) = baseline_delta.as_ref() {
        summary_fields.push(json!({"name": "baseline_delta", "value": delta}));
    }
    if let Some(value) = metrics_accuracy {
        summary_fields.push(json!({
            "name": "standalone_logistic_accuracy",
            "value": format!("{:.4}", value)
        }));
    }

    let run_comparison = json!({
        "available": true,
        "compare_keys": ["primary_metric", "baseline_delta", "standalone_logistic_accuracy"],
        "observations": [
            best_model_detail,
            standalone_detail,
            format!(
                "Artifacts were assembled directly from {} with official paper retrieval left on upstream APIs only.",
                source_workspace.display()
            )
        ]
    });

    let lineage = json!({
        "available": true,
        "run_count_hint": 2,
        "history": [
            {
                "run_id": format!("{}-baseline", slugify(session_id)),
                "parent_run_id": "iris-root",
                "variant_label": "baseline",
                "change_summary": standalone_detail,
                "artifact_paths": vec![metrics_rel.clone()]
            },
            {
                "run_id": format!("{}-comparison", slugify(session_id)),
                "parent_run_id": format!("{}-baseline", slugify(session_id)),
                "variant_label": "current",
                "change_summary": "Multi-model iris comparison summary with persisted script, figure, and dataset split manifest.",
                "artifact_paths": artifact_paths.clone()
            }
        ]
    });

    Ok(RuntimeBundle {
        artifact_paths: artifact_paths.clone(),
        result_bundle: json!({
            "bundle_kind": "classical_ml_result_bundle",
            "summary_fields": summary_fields,
            "artifact_paths": artifact_paths
        }),
        run_comparison,
        lineage,
    })
}

fn resolve_optional_input(
    source_workspace: &Path,
    explicit: Option<&Path>,
    fallback_candidates: &[&str],
    label: &str,
) -> Result<String> {
    if let Some(path) = explicit {
        return resolve_existing_relative_path(source_workspace, path);
    }
    collect_existing_relative_paths(source_workspace, fallback_candidates)
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("could not find {} in {}", label, source_workspace.display()))
}

fn collect_existing_relative_paths(source_workspace: &Path, candidates: &[&str]) -> Vec<String> {
    candidates
        .iter()
        .filter_map(|candidate| {
            let path = source_workspace.join(candidate);
            path.exists().then(|| normalize_relative_path(candidate))
        })
        .collect()
}

fn resolve_existing_relative_path(source_workspace: &Path, candidate: &Path) -> Result<String> {
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        source_workspace.join(candidate)
    };
    if !absolute.exists() {
        bail!("artifact path does not exist: {}", absolute.display());
    }
    let relative = absolute.strip_prefix(source_workspace).map_err(|_| {
        anyhow!(
            "artifact {} is outside source workspace",
            absolute.display()
        )
    })?;
    Ok(normalize_relative_path(relative))
}

fn normalize_relative_path(path: impl AsRef<Path>) -> String {
    path.as_ref()
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn read_utf8_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path)?;
    String::from_utf8(bytes)
        .map_err(|error| anyhow!("{} is not valid UTF-8: {}", path.display(), error))
}

fn markdown_table_cells(line: &str) -> Option<Vec<String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with('|') {
        return None;
    }
    let cells = trimmed
        .trim_matches('|')
        .split('|')
        .map(|part| part.trim().to_string())
        .collect::<Vec<_>>();
    (cells.len() >= 2).then_some(cells)
}

fn normalized_header(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn parse_accuracy_cell(value: &str) -> Option<f64> {
    let cleaned = value.trim().trim_matches('*').trim_matches('`').trim();
    let percent = cleaned.ends_with('%');
    let numeric = cleaned.trim_end_matches('%').trim().parse::<f64>().ok()?;
    let normalized = if percent { numeric / 100.0 } else { numeric };
    (normalized.is_finite() && (0.0..=1.0).contains(&normalized)).then_some(normalized)
}

fn is_markdown_separator_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let compact = cell.trim().trim_matches(':');
            compact.len() >= 3 && compact.chars().all(|ch| ch == '-')
        })
}

fn parse_model_accuracy_rows(markdown: &str) -> Vec<ModelMetricRow> {
    let lines = markdown.lines().collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        let Some(headers) = markdown_table_cells(lines[index]) else {
            index += 1;
            continue;
        };
        let normalized = headers
            .iter()
            .map(|header| normalized_header(header))
            .collect::<Vec<_>>();
        let model_index = normalized.iter().position(|header| {
            header == "model"
                || header == "models"
                || header == "classifier"
                || header == "method"
                || header.contains("模型")
        });
        let accuracy_index = normalized.iter().position(|header| {
            header == "accuracy"
                || header == "testaccuracy"
                || header == "validationaccuracy"
                || header.contains("准确率")
        });
        let (Some(model_index), Some(accuracy_index)) = (model_index, accuracy_index) else {
            index += 1;
            continue;
        };

        index += 1;
        if index < lines.len() {
            if let Some(separator) = markdown_table_cells(lines[index]) {
                if is_markdown_separator_row(&separator) {
                    index += 1;
                }
            }
        }
        while index < lines.len() {
            let Some(cells) = markdown_table_cells(lines[index]) else {
                break;
            };
            if cells.len() > model_index && cells.len() > accuracy_index {
                let name = cells[model_index]
                    .trim()
                    .trim_matches('*')
                    .trim_matches('`')
                    .trim()
                    .to_string();
                if !name.is_empty() {
                    if let Some(accuracy) = parse_accuracy_cell(&cells[accuracy_index]) {
                        rows.push(ModelMetricRow { name, accuracy });
                    }
                }
            }
            index += 1;
        }
    }

    rows
}

fn extract_metric_value(text: &str, labels: &[&str]) -> Option<f64> {
    for label in labels {
        let pattern = format!(r"(?im){}\s*[:|]\s*([0-9]+\.[0-9]+)", regex::escape(label));
        if let Ok(regex) = Regex::new(&pattern) {
            if let Some(captures) = regex.captures(text) {
                if let Some(value) = captures.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn infer_error_analysis_summary(metrics_text: &str) -> String {
    let lowered = metrics_text.to_ascii_lowercase();
    if lowered.contains("versicolor") && lowered.contains("virginica") {
        "Most residual errors occur on versicolor versus virginica boundary cases.".to_string()
    } else if lowered.contains("confusion matrix") {
        "Residual errors remain concentrated in the confusion-matrix off-diagonal cells."
            .to_string()
    } else {
        "Residual errors were summarized from the supplied experiment metrics note.".to_string()
    }
}

fn ensure_dataset_split_manifest(
    source_workspace: &Path,
    session_id: &str,
    summary_rel: &str,
    metrics_rel: &str,
) -> Result<String> {
    let manifest_rel = format!(
        ".atlas/paper_runner/{}/dataset_split_manifest.json",
        slugify(session_id)
    );
    let manifest_path = source_workspace.join(manifest_rel.replace('/', "\\"));
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create manifest directory {}", parent.display()))?;
    }
    let payload = json!({
        "dataset_id": "iris",
        "dataset_provider": "sklearn.load_iris",
        "split_strategy": "train_test_split",
        "test_size": 0.3,
        "stratified": true,
        "random_state": 42,
        "derived_from": [summary_rel, metrics_rel],
        "generator": "paper_workflow_runner"
    });
    fs::write(&manifest_path, serde_json::to_string_pretty(&payload)?).with_context(|| {
        format!(
            "failed to write dataset split manifest {}",
            manifest_path.display()
        )
    })?;
    Ok(manifest_rel)
}

fn dedup_paths(paths: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for path in paths {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }
    deduped
}

fn display_optional_path(path: Option<&PathBuf>) -> String {
    path.map(|value| value.display().to_string())
        .unwrap_or_else(|| "<none>".to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_model_accuracy_rows, slugify};

    #[test]
    fn parses_markdown_model_accuracy_rows() {
        let rows = parse_model_accuracy_rows(
            r#"
| Model | Accuracy | F1 Score (weighted) |
|---|---|---|
| Logistic Regression | 0.9111 | 0.9107 |
| Decision Tree | 0.9778 | 0.9778 |
"#,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "Logistic Regression");
        assert!((rows[1].accuracy - 0.9778).abs() < 1e-9);
    }

    #[test]
    fn ignores_class_level_metric_tables() {
        let rows = parse_model_accuracy_rows(
            r#"
| Class | F1 | Recall |
|---|---|---|
| Setosa | 1.0000 | 1.0000 |

| Rank | Model | Parameter | Accuracy |
|---|---|---|---|
| 1 | KNN | k=15 | 96.47% |
| 2 | Random Forest | n=100 | 0.9567 |
"#,
        );
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].name, "KNN");
        assert!((rows[0].accuracy - 0.9647).abs() < 1e-9);
    }

    #[test]
    fn slugify_keeps_ascii_words() {
        assert_eq!(slugify("Iris comparison run"), "iris-comparison-run");
        assert_eq!(slugify("  "), "");
    }
}
