use ai_assistant::scientist::workflow::{run_paper_workflow, PaperWorkflowRequest};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tempfile::tempdir;

fn paper_workflow_test_guard() -> std::sync::MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("paper workflow test guard")
}

fn bundled_tectonic_path() -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(codex_home) = std::env::var("CODEX_HOME") {
        roots.push(PathBuf::from(codex_home).join("plugins").join("cache"));
    }
    if let Some(home) = dirs::home_dir() {
        roots.push(home.join(".codex").join("plugins").join("cache"));
    }
    if let Some(data_local) = dirs::data_local_dir() {
        roots.push(data_local.join("Codex").join("plugins").join("cache"));
    }

    for root in roots {
        let base = root.join("openai-bundled").join("latex");
        if !base.exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&base) {
            for entry in entries.filter_map(|entry| entry.ok()) {
                let candidate = entry.path().join("bin").join(if cfg!(windows) {
                    "tectonic.exe"
                } else {
                    "tectonic"
                });
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn classical_ml_runtime_result_bundle(
    run_id: &str,
    primary_metric: &str,
    baseline_delta: &str,
    error_analysis_summary: &str,
    artifact_paths: &[String],
) -> Value {
    json!({
        "bundle_kind": "classical_ml_result_bundle",
        "summary_fields": [
            { "name": "run_id", "value": run_id },
            { "name": "primary_metric", "value": primary_metric },
            { "name": "baseline_delta", "value": baseline_delta },
            { "name": "error_analysis_summary", "value": error_analysis_summary }
        ],
        "artifact_paths": artifact_paths,
    })
}

fn classical_ml_runtime_run_comparison(observation: &str) -> Value {
    json!({
        "available": true,
        "compare_keys": ["accuracy", "f1", "fit_time_seconds"],
        "observations": [
            observation,
            "The current run preserves the same classical_ml evaluation schema."
        ]
    })
}

fn classical_ml_runtime_lineage(
    baseline_run_id: &str,
    current_run_id: &str,
    artifact_paths: &[String],
    current_change_summary: &str,
) -> Value {
    json!({
        "available": true,
        "run_count_hint": 2,
        "history": [
            {
                "run_id": baseline_run_id,
                "parent_run_id": "iris-root",
                "variant_label": "baseline",
                "change_summary": "Reference logistic-regression baseline before the current revision.",
                "artifact_paths": artifact_paths
            },
            {
                "run_id": current_run_id,
                "parent_run_id": baseline_run_id,
                "variant_label": "current",
                "change_summary": current_change_summary,
                "artifact_paths": artifact_paths
            }
        ]
    })
}

#[test]
fn scientist_paper_workflow_end_to_end_writes_manuscript_bundle() {
    let _guard = paper_workflow_test_guard();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let papers_dir = temp_dir.path().join("papers");
        fs::create_dir_all(&papers_dir).expect("create papers dir");

        let local_paper = papers_dir.join("tiny_ml_paper.md");
        let mut file = fs::File::create(&local_paper).expect("create local paper");
        writeln!(
            file,
            "# Tiny ML Benchmark Note\n\nAbstract: We study a compact image classification workflow.\n\n## Experimental Setup\nWe evaluate on CIFAR-10 with a fixed split and compare against a regularized linear baseline.\n\n## Results\nValidation accuracy reaches 0.91 with clear boundary-case failures.\n\n## References\n[1] Prior benchmark paper."
        )
        .expect("write local paper");

        let workspace = temp_dir.path().join("workflow");
        let result = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-e2e".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("paper workflow should succeed");

        assert!(result.paper_markdown_path.exists());
        assert!(result.paper_latex_path.exists());
        assert!(result.references_bib_path.exists());
        assert!(result.appendix_markdown_path.exists());
        assert!(result.result_bundle_path.exists());
        assert!(result.review_response_path.exists());
        assert!(result.revision_execution_plan_path.exists());
        assert!(result.payload_path.exists());
        assert!(result.rebuttal_markdown_path.exists());
        assert!(result.section_bundle_path.exists());
        assert!(result.section_bundle_before_path.exists());
        assert!(result.section_bundle_after_path.exists());
        assert!(result.section_diff_path.exists());
        assert!(result.manuscript_bundle_before_path.exists());
        assert!(result.manuscript_bundle_after_path.exists());
        assert!(result.manuscript_diff_path.exists());

        let paper_md = fs::read_to_string(&result.paper_markdown_path).expect("read paper md");
        let paper_tex = fs::read_to_string(&result.paper_latex_path).expect("read paper tex");
        let references_bib =
            fs::read_to_string(&result.references_bib_path).expect("read references bib");
        let appendix_md =
            fs::read_to_string(&result.appendix_markdown_path).expect("read appendix md");
        let rebuttal_md =
            fs::read_to_string(&result.rebuttal_markdown_path).expect("read rebuttal md");
        let section_bundle =
            fs::read_to_string(&result.section_bundle_path).expect("read section bundle");
        let section_bundle_before =
            fs::read_to_string(&result.section_bundle_before_path).expect("read before section bundle");
        let section_bundle_after =
            fs::read_to_string(&result.section_bundle_after_path).expect("read after section bundle");
        let section_diff =
            fs::read_to_string(&result.section_diff_path).expect("read section diff bundle");
        let manuscript_bundle_before =
            fs::read_to_string(&result.manuscript_bundle_before_path).expect("read manuscript before bundle");
        let manuscript_bundle_after =
            fs::read_to_string(&result.manuscript_bundle_after_path).expect("read manuscript after bundle");
        let manuscript_diff =
            fs::read_to_string(&result.manuscript_diff_path).expect("read manuscript diff bundle");
        let payload = fs::read_to_string(&result.payload_path).expect("read payload");
        let review_response =
            fs::read_to_string(&result.review_response_path).expect("read review response");
        let revision_execution_plan = fs::read_to_string(&result.revision_execution_plan_path)
            .expect("read revision execution plan");

        assert!(paper_md.contains("# "));
        assert!(paper_md.contains("## Abstract"));
        assert!(paper_md.contains("## Introduction"));
        assert!(paper_md.contains("## Method"));
        assert!(paper_md.contains("## Results"));
        assert!(paper_md.contains("## Conclusion"));

        assert!(paper_tex.contains("\\documentclass"));
        assert!(paper_tex.contains("\\begin{abstract}"));
        assert!(paper_tex.contains("\\section{Method}"));
        assert!(paper_tex.contains("\\bibliography{references}"));

        assert!(references_bib.contains("% References auto-generated by AI Scientist"));
        assert!(appendix_md.contains("# Artifact Appendix"));
        assert!(appendix_md.contains("## Artifact Paths"));
        assert!(rebuttal_md.contains("# Rebuttal And Review Response"));
        assert!(section_bundle.contains("\"section_prompt_pack\""));
        assert!(section_bundle.contains("\"reviewer_feedback_trace\""));
        assert!(section_bundle.contains("\"evidence_trace\""));
        assert!(section_bundle.contains("\"revision_plan\""));
        assert!(section_bundle.contains("\"rebuttal_closure_records\""));
        assert!(section_bundle.contains("\"claim_anchors\""));
        assert!(section_bundle_before.contains("\"claim_anchors\""));
        assert!(section_bundle_after.contains("\"claim_anchors\""));
        assert!(section_diff.contains("\"paper_section_diff_bundle_v2\""));
        assert!(section_diff.contains("\"changed_section_count\""));
        assert!(manuscript_bundle_before.contains("\"paper_manuscript_section_bundle_v1\""));
        assert!(manuscript_bundle_before.contains("\"markdown_text\""));
        assert!(manuscript_bundle_after.contains("\"paper_manuscript_section_bundle_v1\""));
        assert!(manuscript_diff.contains("\"paper_manuscript_diff_bundle_v1\""));
        assert!(manuscript_diff.contains("\"changed_section_count\""));
        assert!(review_response.contains("\"revision_plan\""));
        assert!(review_response.contains("\"rebuttal_closure_records\""));
        assert!(revision_execution_plan.contains("\"paper_revision_execution_plan_v1\""));
        assert!(revision_execution_plan.contains("\"section_rewrite_queue\""));
        assert!(revision_execution_plan.contains("\"execution_protocol\""));
        assert!(payload.contains("\"paper\""));
        assert!(payload.contains("\"markdown_draft\""));
        assert!(payload.contains("\"reviewer_feedback_trace\""));
        assert!(payload.contains("\"evidence_trace\""));
        assert!(payload.contains("\"revision_plan\""));
        assert!(payload.contains("\"rebuttal_closure_records\""));
        assert_eq!(
            result.paper_ready_gate["schema_version"],
            "paper_ready_gate_bundle_v7"
        );
        assert_eq!(
            result.paper_ready_gate["manuscript_evidence_coverage"]["schema_version"],
            "paper_ready_gate_v7"
        );
        assert_eq!(
            result.paper_ready_gate["manuscript_evidence_coverage"]["claim_evidence_semantics"]["schema_version"],
            "paper_claim_evidence_gate_v5"
        );
        assert!(result.paper_ready_gate["manuscript_evidence_coverage"]["claim_evidence_semantics"]["checks"]
            .as_array()
            .is_some());
        assert!(result
            .paper_ready_gate["manuscript_evidence_coverage"]["claim_evidence_semantics"]["checks"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .all(|item| {
                item.get("semantic_support_status").is_some()
                    && item.get("semantic_support_score").is_some()
                    && item.get("semantic_relation").is_some()
                    && item.get("semantic_relation_detail").is_some()
                    && item.get("claim_sentence_alignments").is_some()
                    && item.get("manuscript_excerpt").is_some()
                    && item.get("grounded_section_span_excerpt").is_some()
                    && item.get("grounded_required_source_count").is_some()
            }));
        assert!(result
            .paper_ready_gate["manuscript_evidence_coverage"]["claim_evidence_semantics"]["checks"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .all(|item| {
                item.get("claim_anchor_overlap")
                    .and_then(|value| value.get("ratio"))
                    .is_some()
                    && item.get("evidence_overlap")
                        .and_then(|value| value.get("ratio"))
                        .is_some()
            }));
        assert!(result.section_diff_preview.iter().all(|item| {
            item.get("before").is_some()
                && item.get("after").is_some()
                && item.pointer("/before/markdown_excerpt").is_some()
                && item.pointer("/after/markdown_excerpt").is_some()
        }));
        assert!(result.manuscript_diff_preview.iter().all(|item| {
            item.pointer("/before/markdown_excerpt").is_some() && item.pointer("/after/markdown_excerpt").is_some()
        }));

        assert_eq!(result.report_response.content["paper"]["format"], "latex");
        assert_eq!(
            result.report_response.content["paper"]["schema_version"],
            "cs_paper_blueprint_v1"
        );
        assert_eq!(
            result.report_response.content["paper"]["manuscript_bundle_schema_version"],
            "cs_manuscript_bundle_v1"
        );
        assert_eq!(
            result.verification_response.content["verification_center_repair"]["status"],
            "ready"
        );
        assert!(!result.revision_mode.is_empty());
        assert!(!result.source_run_id.is_empty());
        assert!(!result.paper_ready_detail.is_empty());
        assert!(
            result.report_response.content["paper"]["quality_checklist"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item["name"] == "source_policy_compliance")
        );
        assert!(result.report_response.content["paper"]["revision_plan"]["section_rewrite_queue"]
            .as_array()
            .is_some());
        assert!(result.report_response.content["paper"]["rebuttal_closure_records"]
            .as_array()
            .is_some());
        assert!(result.report_response.content["paper"]["draft_sections"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|item| item["section_id"] == "results" && item["claim_anchors"].as_array().is_some()));
    });
}

#[test]
fn scientist_paper_workflow_consumes_supplied_runtime_payload_and_invalidates_checkpoint_on_runtime_change(
) {
    let _guard = paper_workflow_test_guard();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let papers_dir = temp_dir.path().join("papers");
        fs::create_dir_all(&papers_dir).expect("create papers dir");

        let local_paper = papers_dir.join("iris_runtime_note.md");
        let mut paper_file = fs::File::create(&local_paper).expect("create local paper");
        writeln!(
            paper_file,
            "# Iris Runtime Note\n\nAbstract: We study lightweight iris baselines with explicit runtime provenance.\n\n## Experimental Setup\nWe compare small classical ML baselines under a fixed split.\n\n## Results\nRuntime payloads should flow into the manuscript bundle without internal stub substitution.\n\n## References\n[1] Iris benchmark note."
        )
        .expect("write local paper");

        let source_workspace = temp_dir.path().join("source_workspace");
        let results_dir = source_workspace.join("results");
        fs::create_dir_all(&results_dir).expect("create results dir");

        let dataset_manifest = results_dir.join("dataset_split_manifest.json");
        let train_script = results_dir.join("train_or_eval_script.py");
        let metrics_report = results_dir.join("metrics_report.md");

        fs::write(
            &dataset_manifest,
            r#"{"dataset_id":"iris","provider":"sklearn","split":"train_test_split","random_state":42}"#,
        )
        .expect("write dataset manifest");
        fs::write(
            &train_script,
            "from sklearn.datasets import load_iris\nprint('train/eval iris baseline')\n",
        )
        .expect("write train script");
        fs::write(
            &metrics_report,
            "# Iris Metrics\n\naccuracy: 0.9333\nf1: 0.9300\nfit_time_seconds: 0.0233\nerror analysis: versicolor versus virginica boundary cases dominate the residual mistakes.\n",
        )
        .expect("write initial metrics report");

        let artifact_paths = vec![
            "results/dataset_split_manifest.json".to_string(),
            "results/train_or_eval_script.py".to_string(),
            "results/metrics_report.md".to_string(),
        ];

        let workspace = temp_dir.path().join("workflow");
        let initial_run_id = "iris-real-run-a";
        let initial_primary_metric = "accuracy 0.9333";
        let initial_error_summary =
            "versicolor versus virginica boundary cases dominate the residual mistakes";
        let initial = run_paper_workflow(PaperWorkflowRequest {
            topic: "Use sklearn logistic regression on the iris dataset with cross validation"
                .to_string(),
            session_id: "paper-runtime-swap".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: Some(source_workspace.clone()),
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: Some(artifact_paths.clone()),
            runtime_result_bundle: Some(classical_ml_runtime_result_bundle(
                initial_run_id,
                initial_primary_metric,
                "+0.0222 over logistic baseline",
                initial_error_summary,
                &artifact_paths,
            )),
            runtime_run_comparison: Some(classical_ml_runtime_run_comparison(
                "Cross-validation rerun confirms the baseline remains stable at accuracy 0.9333 and f1 0.9300.",
            )),
            runtime_lineage: Some(classical_ml_runtime_lineage(
                "iris-baseline-run",
                initial_run_id,
                &artifact_paths,
                "Added report-grounded error analysis for the current iris run.",
            )),
        })
        .await
        .expect("workflow should consume initial runtime payload");

        assert_eq!(initial.source_run_id, initial_run_id);
        assert_eq!(
            initial.verification_response.content["artifact_inventory"]["verified_root"],
            Value::String(source_workspace.display().to_string())
        );
        let initial_bundle =
            fs::read_to_string(&initial.result_bundle_path).expect("read initial result bundle");
        assert!(initial_bundle.contains(initial_run_id));
        assert!(initial_bundle.contains(initial_primary_metric));
        assert!(initial_bundle.contains(initial_error_summary));

        let initial_paper_md =
            fs::read_to_string(&initial.paper_markdown_path).expect("read initial paper md");
        assert!(initial_paper_md.contains(initial_primary_metric));
        assert!(initial_paper_md.contains(initial_error_summary));
        assert!(initial_paper_md.contains("Cross-validation rerun confirms"));

        fs::write(
            &metrics_report,
            "# Iris Metrics\n\naccuracy: 0.9778\nf1: 0.9778\nfit_time_seconds: 0.0111\nerror analysis: one residual boundary-case miss remains after the tree-depth adjustment.\n",
        )
        .expect("rewrite metrics report");

        let resumed_run_id = "iris-real-run-b";
        let resumed_primary_metric = "accuracy 0.9778";
        let resumed_error_summary =
            "one residual boundary-case miss remains after the tree-depth adjustment";
        let resumed = run_paper_workflow(PaperWorkflowRequest {
            topic: "Use sklearn logistic regression on the iris dataset with cross validation"
                .to_string(),
            session_id: "paper-runtime-swap".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: Some(source_workspace.clone()),
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: Some(artifact_paths.clone()),
            runtime_result_bundle: Some(classical_ml_runtime_result_bundle(
                resumed_run_id,
                resumed_primary_metric,
                "+0.0667 over logistic baseline",
                resumed_error_summary,
                &artifact_paths,
            )),
            runtime_run_comparison: Some(classical_ml_runtime_run_comparison(
                "Depth-limited tree rerun improves weighted f1 while reducing fit_time_seconds to 0.0111.",
            )),
            runtime_lineage: Some(classical_ml_runtime_lineage(
                initial_run_id,
                resumed_run_id,
                &artifact_paths,
                "Promoted the faster tree-based variant after confirming the updated report artifacts.",
            )),
        })
        .await
        .expect("workflow should invalidate the old checkpoint when runtime payload changes");

        assert_eq!(resumed.source_run_id, resumed_run_id);
        assert_eq!(resumed.checkpoint_stage, "paper_ready_evaluated");
        let resumed_bundle =
            fs::read_to_string(&resumed.result_bundle_path).expect("read resumed result bundle");
        assert!(resumed_bundle.contains(resumed_run_id));
        assert!(resumed_bundle.contains(resumed_primary_metric));
        assert!(resumed_bundle.contains(resumed_error_summary));
        assert!(!resumed_bundle.contains(initial_primary_metric));
        assert!(!resumed_bundle.contains(initial_error_summary));

        let resumed_paper_md =
            fs::read_to_string(&resumed.paper_markdown_path).expect("read resumed paper md");
        assert!(resumed_paper_md.contains(resumed_primary_metric));
        assert!(resumed_paper_md.contains(resumed_error_summary));
        assert!(resumed_paper_md.contains("Depth-limited tree rerun improves weighted f1"));

        let checkpoint_raw = fs::read_to_string(workspace.join("workflow_checkpoint.json"))
            .expect("read checkpoint");
        let checkpoint: Value =
            serde_json::from_str(&checkpoint_raw).expect("parse workflow checkpoint");
        assert_eq!(
            checkpoint["result_bundle"]["summary_fields"][0]["value"],
            Value::String(resumed_run_id.to_string())
        );
        assert_eq!(
            checkpoint["verification_center"]["verification_center"]["workspace_root"],
            Value::String(source_workspace.display().to_string())
        );
    });
}

#[test]
fn scientist_paper_workflow_recovers_from_checkpoint_after_interruption() {
    let _guard = paper_workflow_test_guard();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let papers_dir = temp_dir.path().join("papers");
        fs::create_dir_all(&papers_dir).expect("create papers dir");

        let local_paper = papers_dir.join("tiny_ml_paper.md");
        let mut file = fs::File::create(&local_paper).expect("create local paper");
        writeln!(
            file,
            "# Tiny ML Benchmark Note\n\nAbstract: We study checkpointed paper workflow recovery.\n\n## Experimental Setup\nWe evaluate a compact benchmark and preserve restartable workflow state.\n\n## Results\nValidation accuracy reaches 0.90 with explicit failure recovery notes.\n\n## References\n[1] Prior benchmark paper."
        )
        .expect("write local paper");

        let workspace = temp_dir.path().join("workflow");
        let initial = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-resume".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("initial workflow should succeed");

        let checkpoint_path = workspace.join("workflow_checkpoint.json");
        let checkpoint_raw = fs::read_to_string(&checkpoint_path).expect("read checkpoint");
        let mut checkpoint: Value = serde_json::from_str(&checkpoint_raw).expect("parse checkpoint");
        checkpoint["current_stage"] = Value::String("report_initial_ready".to_string());
        checkpoint["stages_completed"] = Value::Array(
            checkpoint["stages_completed"]
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter(|item| {
                    item.as_str().is_some_and(|stage| {
                        !matches!(
                            stage,
                            "revision_closure_ready"
                                | "artifacts_materialized"
                                | "pdf_compiled"
                                | "paper_ready_evaluated"
                        )
                    })
                })
                .collect(),
        );
        checkpoint["final_reviewer_feedback"] = Value::Null;
        checkpoint["revision_execution_trace"] = Value::Null;
        checkpoint["final_verification_response"] = Value::Null;
        checkpoint["final_report_response"] = Value::Null;
        checkpoint["auto_revision_applied"] = Value::Bool(false);
        checkpoint["pdf_compile_status"] = Value::Null;
        checkpoint["pdf_compile_detail"] = Value::Null;
        checkpoint["paper_ready"] = Value::Null;
        checkpoint["paper_ready_detail"] = Value::Null;
        checkpoint["paper_ready_gate"] = Value::Null;
        fs::write(
            &checkpoint_path,
            serde_json::to_string_pretty(&checkpoint).expect("serialize checkpoint"),
        )
        .expect("rewrite checkpoint");

        for path in [
            &initial.review_response_path,
            &initial.revision_execution_plan_path,
            &initial.rebuttal_markdown_path,
            &initial.section_bundle_path,
            &initial.section_bundle_before_path,
            &initial.section_bundle_after_path,
            &initial.section_diff_path,
            &initial.manuscript_bundle_before_path,
            &initial.manuscript_bundle_after_path,
            &initial.manuscript_diff_path,
            &initial.payload_path,
        ] {
            if path.exists() {
                fs::remove_file(path).expect("remove resumed artifact");
            }
        }
        fs::remove_dir_all(&papers_dir).expect("remove local paper source");

        let resumed = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-resume".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("workflow should recover from checkpoint");

        assert!(resumed.review_response_path.exists());
        assert!(resumed.revision_execution_plan_path.exists());
        assert!(resumed.rebuttal_markdown_path.exists());
        assert!(resumed.payload_path.exists());
        assert_eq!(resumed.checkpoint_stage, "paper_ready_evaluated");
        assert!(!resumed.paper_ready_detail.is_empty());

        let resumed_checkpoint_raw = fs::read_to_string(&checkpoint_path).expect("read resumed checkpoint");
        let resumed_checkpoint: Value =
            serde_json::from_str(&resumed_checkpoint_raw).expect("parse resumed checkpoint");
        let stages = resumed_checkpoint["stages_completed"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(stages.iter().any(|item| item == "revision_closure_ready"));
        assert!(stages.iter().any(|item| item == "artifacts_materialized"));
        assert!(stages.iter().any(|item| item == "paper_ready_evaluated"));
    });
}

#[test]
fn scientist_paper_workflow_refreshes_stale_appendix_after_outputs_checkpoint_resume() {
    let _guard = paper_workflow_test_guard();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let papers_dir = temp_dir.path().join("papers");
        fs::create_dir_all(&papers_dir).expect("create papers dir");

        let local_paper = papers_dir.join("tiny_ml_paper.md");
        let mut file = fs::File::create(&local_paper).expect("create local paper");
        writeln!(
            file,
            "# Tiny ML Benchmark Note\n\nAbstract: We study appendix refresh recovery.\n\n## Experimental Setup\nWe evaluate a compact benchmark and preserve restartable workflow state.\n\n## Results\nValidation accuracy reaches 0.90 with explicit failure recovery notes.\n\n## References\n[1] Prior benchmark paper."
        )
        .expect("write local paper");

        let workspace = temp_dir.path().join("workflow");
        let initial = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-appendix-refresh".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("initial workflow should succeed");

        fs::write(
            &initial.appendix_markdown_path,
            "# Artifact Appendix\n\n## Artifact Paths\n\n- stale/path.txt\n\n## Appendix Sections\n\n### artifact_inventory\n\nlegacy appendix\n",
        )
        .expect("overwrite appendix with stale content");

        let resumed = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-appendix-refresh".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("workflow should refresh stale appendix after checkpoint resume");

        let refreshed_appendix =
            fs::read_to_string(&resumed.appendix_markdown_path).expect("read refreshed appendix");
        assert!(refreshed_appendix.contains("## Verification Gaps"));
        assert!(refreshed_appendix.contains("## Skipped Tools"));
        assert_eq!(resumed.checkpoint_stage, "paper_ready_evaluated");
    });
}

#[test]
fn scientist_paper_workflow_force_rewrite_invalidates_cached_report_outputs() {
    let _guard = paper_workflow_test_guard();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let papers_dir = temp_dir.path().join("papers");
        fs::create_dir_all(&papers_dir).expect("create papers dir");

        let local_paper = papers_dir.join("tiny_ml_paper.md");
        let mut file = fs::File::create(&local_paper).expect("create local paper");
        writeln!(
            file,
            "# Tiny ML Benchmark Note\n\nAbstract: We study forced rewrite recovery.\n\n## Experimental Setup\nWe evaluate a compact benchmark and preserve restartable workflow state.\n\n## Results\nValidation accuracy reaches 0.90 with explicit failure recovery notes.\n\n## References\n[1] Prior benchmark paper."
        )
        .expect("write local paper");

        let workspace = temp_dir.path().join("workflow");
        let initial = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-force-rewrite".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("initial workflow should succeed");

        let checkpoint_path = workspace.join("workflow_checkpoint.json");
        let checkpoint_raw = fs::read_to_string(&checkpoint_path).expect("read checkpoint");
        let mut checkpoint: Value = serde_json::from_str(&checkpoint_raw).expect("parse checkpoint");
        let mut stale_report_initial = checkpoint["report_response_initial"].clone();
        stale_report_initial["content"]["paper"]["markdown_draft"] = Value::String("# stale draft".to_string());
        stale_report_initial["content"]["paper"]["latex_manuscript_shell"] =
            Value::String("stale latex".to_string());
        stale_report_initial["content"]["paper"]["artifact_appendix_plan"] = serde_json::json!({
            "artifact_paths": ["stale/path.txt"],
            "appendix_sections": [],
            "verification_gaps": [],
            "skipped_tools": []
        });
        let stale_report_final = stale_report_initial.clone();
        checkpoint["report_response_initial"] = stale_report_initial;
        checkpoint["final_report_response"] = stale_report_final;
        fs::write(
            &checkpoint_path,
            serde_json::to_string_pretty(&checkpoint).expect("serialize checkpoint"),
        )
        .expect("rewrite checkpoint");
        fs::write(&initial.appendix_markdown_path, "# stale appendix\n").expect("write stale appendix");

        let resumed = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-force-rewrite".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: true,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("workflow should invalidate cached report outputs on force rewrite");

        let refreshed_appendix =
            fs::read_to_string(&resumed.appendix_markdown_path).expect("read refreshed appendix");
        assert!(refreshed_appendix.contains("## Artifact Paths"));
        assert_ne!(refreshed_appendix.trim(), "# stale appendix");

        let payload_raw = fs::read_to_string(&resumed.payload_path).expect("read payload");
        assert!(!payload_raw.contains("\"markdown_draft\": \"# stale draft\""));

        let resumed_checkpoint_raw =
            fs::read_to_string(&checkpoint_path).expect("read resumed checkpoint");
        let resumed_checkpoint: Value =
            serde_json::from_str(&resumed_checkpoint_raw).expect("parse resumed checkpoint");
        let stages = resumed_checkpoint["stages_completed"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(stages.iter().any(|item| item == "report_initial_ready"));
        assert!(stages.iter().any(|item| item == "revision_closure_ready"));
        assert!(stages.iter().any(|item| item == "artifacts_materialized"));
    });
}

#[test]
fn scientist_paper_workflow_retries_pdf_after_failed_compile_checkpoint() {
    let _guard = paper_workflow_test_guard();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let papers_dir = temp_dir.path().join("papers");
        fs::create_dir_all(&papers_dir).expect("create papers dir");

        let local_paper = papers_dir.join("tiny_ml_paper.md");
        let mut file = fs::File::create(&local_paper).expect("create local paper");
        writeln!(
            file,
            "# Tiny ML Benchmark Note\n\nAbstract: We study PDF resume recovery with a bounded local seed.\n\n## Experimental Setup\nWe keep the setup tiny so checkpointed PDF recovery can be validated deterministically.\n\n## Results\nValidation accuracy reaches 0.89 with explicit PDF recovery annotations.\n\n## References\n[1] Prior benchmark paper."
        )
        .expect("write local paper");

        let workspace = temp_dir.path().join("workflow");
        let mut missing_toolchains = BTreeMap::new();
        missing_toolchains.insert("tectonic".to_string(), "__missing_tectonic__".to_string());
        missing_toolchains.insert("pdflatex".to_string(), "__missing_pdflatex__".to_string());

        let first = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-pdf-retry".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: Some(missing_toolchains),
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("workflow should succeed even when pdf compile is unavailable");

        assert_eq!(first.pdf_compile_status, "missing_toolchain");
        assert!(first.paper_pdf_path.is_none());

        let checkpoint_path = workspace.join("workflow_checkpoint.json");
        let checkpoint_raw = fs::read_to_string(&checkpoint_path).expect("read checkpoint");
        let checkpoint: Value = serde_json::from_str(&checkpoint_raw).expect("parse checkpoint");
        let stages = checkpoint["stages_completed"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(!stages.iter().any(|item| item == "pdf_compiled"));

        let tectonic = bundled_tectonic_path().expect("bundled tectonic should be available for retry test");
        let mut working_toolchains = BTreeMap::new();
        working_toolchains.insert("tectonic".to_string(), tectonic.to_string_lossy().to_string());

        let resumed = run_paper_workflow(PaperWorkflowRequest {
            topic: "tiny machine learning benchmark".to_string(),
            session_id: "paper-pdf-retry".to_string(),
            workspace_root: workspace.clone(),
            source_workspace_root: None,
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: Some(working_toolchains),
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: None,
            runtime_result_bundle: None,
            runtime_run_comparison: None,
            runtime_lineage: None,
        })
        .await
        .expect("workflow should retry pdf compile once a compiler is available");

        assert_eq!(resumed.pdf_compile_status, "compiled");
        assert!(resumed.paper_pdf_path.as_ref().is_some_and(|path| path.exists()));

        let resumed_checkpoint_raw = fs::read_to_string(&checkpoint_path).expect("read resumed checkpoint");
        let resumed_checkpoint: Value =
            serde_json::from_str(&resumed_checkpoint_raw).expect("parse resumed checkpoint");
        let resumed_stages = resumed_checkpoint["stages_completed"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert!(resumed_stages.iter().any(|item| item == "pdf_compiled"));
        assert_eq!(resumed_checkpoint["pdf_compile_status"], Value::String("compiled".to_string()));
    });
}

#[test]
fn scientist_paper_workflow_runtime_dataset_overrides_stale_literature_hints_in_manuscript() {
    let _guard = paper_workflow_test_guard();
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let papers_dir = temp_dir.path().join("papers");
        fs::create_dir_all(&papers_dir).expect("create papers dir");

        let local_paper = papers_dir.join("subsampling_robustness_tree_ensembles_note.md");
        let mut paper_file = fs::File::create(&local_paper).expect("create local paper");
        writeln!(
            paper_file,
            "# Subsampling robustness of tree ensembles under label noise\n\nAbstract: We discuss prior iris baselines for subsampling robustness of tree ensembles under label noise.\n\n## Experimental Setup\nThe prior note mentions the iris dataset only.\n\n## Results\nThis seed should not override newer runtime evidence.\n\n## References\n[1] Iris benchmark note."
        )
        .expect("write local paper");

        let source_workspace = temp_dir.path().join("source_workspace");
        let experiments_dir = source_workspace.join("experiments");
        fs::create_dir_all(&experiments_dir).expect("create experiments dir");

        fs::write(
            experiments_dir.join("experiment.py"),
            "from sklearn.datasets import load_digits\nfrom sklearn.model_selection import train_test_split\n",
        )
        .expect("write experiment.py");
        fs::write(
            experiments_dir.join("results.csv"),
            "model,subsample,noise_rate,acc_mean,acc_std\nRandomForest,1.0,0.0,0.9698,0.0028\nExtraTrees,1.0,0.0,0.9793,0.0018\n",
        )
        .expect("write results.csv");

        let artifact_paths = vec![
            "experiments/experiment.py".to_string(),
            "experiments/results.csv".to_string(),
        ];
        let result = run_paper_workflow(PaperWorkflowRequest {
            topic: "Subsampling robustness of tree ensembles under label noise".to_string(),
            session_id: "paper-runtime-dataset-canonical".to_string(),
            workspace_root: temp_dir.path().join("workflow"),
            source_workspace_root: Some(source_workspace.clone()),
            local_paper_source: Some(papers_dir.clone()),
            search_limit: 3,
            toolchains: None,
            reviewer_feedback: None,
            force_rewrite: false,
            runtime_artifact_paths: Some(artifact_paths.clone()),
            runtime_result_bundle: Some(json!({
                "bundle_kind": "classical_ml_result_bundle",
                "summary_fields": [
                    { "name": "run_id", "value": "classical_ml-run-13" },
                    { "name": "paper_dataset_hints", "value": "digits" },
                    { "name": "baseline_delta", "value": "+0.0094 over RandomForest at noise_rate=0.0" },
                    { "name": "primary_metric", "value": "0.9793" }
                ],
                "artifact_paths": artifact_paths,
            })),
            runtime_run_comparison: Some(json!({
                "available": true,
                "compare_keys": ["accuracy", "f1", "fit_time_seconds"],
                "observations": [
                    "Comparison evidence: compare vs baseline: ExtraTrees 0.9793 versus RandomForest 0.9698"
                ]
            })),
            runtime_lineage: None,
        })
        .await
        .expect("workflow should canonicalize runtime dataset hints");

        let paper_md = fs::read_to_string(&result.paper_markdown_path).expect("read paper md");
        assert!(paper_md.contains("digits"));
        assert!(!paper_md.contains("iris; digits"));
        assert!(!paper_md.contains("grounded in iris;"));

        let payload_raw = fs::read_to_string(&result.payload_path).expect("read payload");
        let payload: Value = serde_json::from_str(&payload_raw).expect("parse payload");
        let blueprint_hints = payload["paper_blueprint"]["paper_dataset_hints"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(blueprint_hints.len(), 1);
        let blueprint_hint = blueprint_hints[0].as_str().unwrap_or("");
        assert!(blueprint_hint.contains("digits"));
        assert!(blueprint_hint.contains("sklearn"));
        assert!(!blueprint_hint.contains("iris"));

        let checkpoint_raw = fs::read_to_string(&result.workflow_checkpoint_path).expect("read checkpoint");
        let checkpoint: Value = serde_json::from_str(&checkpoint_raw).expect("parse checkpoint");
        assert_eq!(
            checkpoint["paper_dataset_hints"],
            json!(["digits"])
        );
    });
}
