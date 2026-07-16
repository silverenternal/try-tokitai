use ai_assistant::scientist::{
    workflow::{ResearchStage, AI_SCIENTIST_WORKFLOW_TOML},
    ExperimentAgent, HypothesisAgent, ReportAgent, ResearchAgent, VerificationAgent,
};
use ai_scientist_core::agent::MessageType;
use ai_scientist_core::{Agent, AgentContext, AgentMessage, AgentRole};
use serde::Deserialize;
use serde_json::json;
use std::fs;
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct WorkflowFile {
    workflow: WorkflowDefinition,
}

#[derive(Debug, Deserialize)]
struct WorkflowDefinition {
    id: String,
    name: String,
    description: String,
    stages: Vec<WorkflowStage>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStage {
    id: String,
    name: String,
    steps: Vec<WorkflowStep>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStep {
    id: String,
    description: String,
    tool: String,
    #[serde(default)]
    depends_on: Vec<String>,
    role: String,
}

#[test]
fn scientist_workflow_toml_matches_stage_contract() {
    let workflow: WorkflowFile =
        toml::from_str(AI_SCIENTIST_WORKFLOW_TOML).expect("workflow TOML should parse");

    assert_eq!(workflow.workflow.id, "ai-scientist-cs-minimal");
    assert!(workflow.workflow.name.contains("AI Scientist"));
    assert!(workflow.workflow.description.contains("research pipeline"));

    let stage_ids: Vec<&str> = workflow
        .workflow
        .stages
        .iter()
        .map(|stage| stage.id.as_str())
        .collect();
    let stages = ResearchStage::all();
    let expected_stage_ids: Vec<&str> = stages.iter().map(ResearchStage::id).collect();

    assert_eq!(stage_ids, expected_stage_ids);

    for stage in &workflow.workflow.stages {
        assert!(!stage.name.trim().is_empty());
        assert!(
            !stage.steps.is_empty(),
            "stage '{}' should declare at least one step",
            stage.id
        );
    }
}

#[test]
fn scientist_workflow_steps_cover_expected_tools_and_roles() {
    let workflow: WorkflowFile =
        toml::from_str(AI_SCIENTIST_WORKFLOW_TOML).expect("workflow TOML should parse");

    let step_ids: Vec<&str> = workflow
        .workflow
        .stages
        .iter()
        .flat_map(|stage| stage.steps.iter().map(|step| step.id.as_str()))
        .collect();
    let tools: Vec<&str> = workflow
        .workflow
        .stages
        .iter()
        .flat_map(|stage| stage.steps.iter().map(|step| step.tool.as_str()))
        .collect();
    let roles: Vec<&str> = workflow
        .workflow
        .stages
        .iter()
        .flat_map(|stage| stage.steps.iter().map(|step| step.role.as_str()))
        .collect();

    assert!(step_ids.contains(&"search_literature"));
    assert!(step_ids.contains(&"formulate_problem"));
    assert!(step_ids.contains(&"design_pipeline"));
    assert!(step_ids.contains(&"math_simplify"));
    assert!(step_ids.contains(&"summarize_results"));
    assert!(step_ids.contains(&"plan_paper_blueprint"));
    assert!(step_ids.contains(&"draft_paper_sections"));
    assert!(step_ids.contains(&"generate_output"));

    assert!(tools.contains(&"search_paper"));
    assert!(tools.contains(&"run_python"));
    assert!(tools.contains(&"sympy_simplify"));

    assert!(roles.contains(&"researcher"));
    assert!(roles.contains(&"hypothesizer"));
    assert!(roles.contains(&"experimenter"));
    assert!(roles.contains(&"verifier"));
    assert!(roles.contains(&"reporter"));

    let summarize_step = workflow
        .workflow
        .stages
        .iter()
        .flat_map(|stage| stage.steps.iter())
        .find(|step| step.id == "summarize_results")
        .expect("summarize_results step should exist");
    assert_eq!(summarize_step.depends_on, vec!["math_integrate"]);
    assert!(summarize_step.description.contains("Summarize"));

    let plan_paper_step = workflow
        .workflow
        .stages
        .iter()
        .flat_map(|stage| stage.steps.iter())
        .find(|step| step.id == "plan_paper_blueprint")
        .expect("plan_paper_blueprint step should exist");
    assert_eq!(plan_paper_step.depends_on, vec!["summarize_results"]);

    let draft_sections_step = workflow
        .workflow
        .stages
        .iter()
        .flat_map(|stage| stage.steps.iter())
        .find(|step| step.id == "draft_paper_sections")
        .expect("draft_paper_sections step should exist");
    assert_eq!(draft_sections_step.depends_on, vec!["plan_paper_blueprint"]);

    let generate_output_step = workflow
        .workflow
        .stages
        .iter()
        .flat_map(|stage| stage.steps.iter())
        .find(|step| step.id == "generate_output")
        .expect("generate_output step should exist");
    assert_eq!(
        generate_output_step.depends_on,
        vec!["draft_paper_sections"]
    );
}

#[test]
fn scientist_agents_form_expected_handoff_chain() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let context = AgentContext::new("scientist-contract")
            .with_goal("Validate the AI Scientist workflow contract");

        let research = ResearchAgent::new("research-1");
        let hypothesis = HypothesisAgent::new("hypothesis-1");
        let experiment = ExperimentAgent::new("experiment-1");
        let verification = VerificationAgent::new("verification-1");
        let report = ReportAgent::new("report-1");

        let research_response = research
            .handle_message(
                AgentMessage::new(
                    AgentRole::Orchestrator,
                    Some(AgentRole::Researcher),
                    MessageType::Request,
                    json!({
                        "action": "search",
                        "query": "workflow contract testing",
                        "paper_dataset_hints": ["CIFAR-10"]
                    }),
                ),
                &context,
            )
            .await
            .expect("research response");
        assert_eq!(research_response.next_role, Some(AgentRole::Hypothesizer));
        assert_eq!(research_response.content["paper_dataset_hints"][0], "CIFAR-10");

        let hypothesis_response = hypothesis
            .handle_message(
                AgentMessage::new(
                    AgentRole::Researcher,
                    Some(AgentRole::Hypothesizer),
                    MessageType::Request,
                    json!({
                        "knowledge_summary": "workflow stages are aligned",
                        "paper_dataset_hints": research_response.content["paper_dataset_hints"].clone()
                    }),
                ),
                &context,
            )
            .await
            .expect("hypothesis response");
        assert_eq!(hypothesis_response.next_role, Some(AgentRole::Experimenter));
        assert_eq!(hypothesis_response.content["testable"], true);
        assert_eq!(hypothesis_response.content["paper_dataset_hints"][0], "CIFAR-10");

        let experiment_response = experiment
            .handle_message(
                AgentMessage::new(
                    AgentRole::Hypothesizer,
                    Some(AgentRole::Experimenter),
                    MessageType::Request,
                    json!({
                        "problem_formulation": "Aligned workflow contracts reduce regressions",
                        "paper_dataset_hints": hypothesis_response.content["paper_dataset_hints"].clone()
                    }),
                ),
                &context,
            )
            .await
            .expect("experiment response");
        assert_eq!(experiment_response.next_role, Some(AgentRole::Verifier));
        assert_eq!(experiment_response.content["status"], "Benchmark plan designed");
        assert_eq!(
            experiment_response.content["benchmark_profile"],
            "general_cs"
        );
        assert_eq!(
            experiment_response.content["benchmark_plan"]["schema_version"],
            "cs_benchmark_v1"
        );
        assert_eq!(
            experiment_response.content["benchmark_plan"]["benchmark_profile"],
            "general_cs"
        );
        assert_eq!(
            experiment_response.content["benchmark_plan"]["dataset_acquisition"]["retrieval_entrypoint"],
            "official_dataset_databases"
        );
        assert_eq!(
            experiment_response.content["benchmark_plan"]["dataset_acquisition"]["paper_dataset_hints"][0],
            "CIFAR-10"
        );
        assert!(
            experiment_response.content["benchmark_plan"]["dataset_acquisition"]["search_queries"][0]
                .as_str()
                .unwrap_or("")
                .contains("CIFAR-10")
        );
        assert_eq!(
            experiment_response.content["experiment"]["dataset_acquisition"]["search_tool"],
            "search_public_datasets"
        );
        assert_eq!(
            experiment_response.content["benchmark_plan"]["reproducibility"]["random_seed_required"],
            true
        );
        assert!(experiment_response.content["benchmark_plan"]["execution_schema"]["stages"]
            .as_array()
            .unwrap_or(&Vec::new())
            .len()
            >= 1);
        assert!(experiment_response.content["benchmark_plan"]["result_bundle_schema"]["summary_fields"]
            .as_array()
            .unwrap_or(&Vec::new())
            .len()
            >= 1);
        assert_eq!(
            experiment_response.content["benchmark_plan"]["lineage_schema"]["required"],
            true
        );

        let verification_response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "contract checks passed",
                        "benchmark_plan": experiment_response.content["benchmark_plan"].clone()
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");
        assert_eq!(verification_response.next_role, Some(AgentRole::Reporter));
        assert_eq!(
            verification_response.content["verification"]["math_check"],
            "passed"
        );
        assert_eq!(
            verification_response.content["benchmark_verifier"]["status"],
            "needs_attention"
        );
        assert_eq!(
            verification_response.content["benchmark_verifier"]["benchmark_profile"],
            "general_cs"
        );
        assert_eq!(
            verification_response.content["benchmark_verifier"]["profile_check"]["status"],
            "passed"
        );
        assert_eq!(
            verification_response.content["verification"]["implementation_sanity"],
            "needs_attention"
        );
        assert!(
            verification_response.content["benchmark_verifier"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "datasets")
        );

        let report_response = report
            .handle_message(
                AgentMessage::new(
                    AgentRole::Verifier,
                    Some(AgentRole::Reporter),
                    MessageType::Request,
                    json!({
                        "all_results": "contract checks passed",
                        "problem_formulation": "Aligned workflow contracts reduce regressions",
                        "paper_dataset_hints": ["CIFAR-10"],
                        "artifact_paths": ["results/report.md", "runs/run-001.json"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "run-001"},
                                {"name": "primary_metric", "value": "accuracy 0.91"},
                                {"name": "baseline_delta", "value": "+0.04 over baseline"},
                                {"name": "error_analysis_summary", "value": "Most errors occur on boundary cases."}
                            ]
                        },
                        "benchmark_plan": experiment_response.content["benchmark_plan"].clone(),
                        "benchmark_verifier": verification_response.content["benchmark_verifier"].clone(),
                        "runtime_result_verification": verification_response
                            .content["runtime_result_verification"]
                            .clone(),
                        "reviewer_feedback": [{
                            "reviewer": "committee-a",
                            "comment": "Clarify how the benchmark split was fixed.",
                            "score": 88,
                            "resolved": false,
                            "linked_run_id": "run-001"
                        }]
                    }),
                ),
                &context,
            )
            .await
            .expect("report response");
        assert!(report_response.success);
        assert_eq!(report_response.next_role, None);
        assert_eq!(report_response.content["paper"]["format"], "latex");
        assert_eq!(
            report_response.content["paper"]["schema_version"],
            "cs_paper_blueprint_v1"
        );
        assert_eq!(
            report_response.content["paper"]["manuscript_bundle_schema_version"],
            "cs_manuscript_bundle_v1"
        );
        assert_eq!(
            report_response.content["paper_blueprint"]["delivery_contract"]["must_be_experiment_grounded"],
            true
        );
        assert_eq!(
            report_response.content["paper_blueprint"]["delivery_contract"]["must_preserve_source_boundaries"],
            true
        );
        assert!(
            report_response.content["paper"]["section_prompt_pack"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 8
        );
        assert!(
            report_response.content["paper"]["section_skill_pack"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 8
        );
        assert!(
            report_response.content["paper"]["draft_sections"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 8
        );
        assert!(
            report_response.content["paper"]["section_prompt_pack"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| {
                    item["section_id"] == "experimental_setup"
                        && item["prompt"]
                            .as_str()
                            .unwrap_or("")
                            .contains("Section-specific revision queue")
                })
        );
        assert!(
            report_response.content["paper"]["draft_sections"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| {
                    item["section_id"] == "experimental_setup"
                        && item.get("claim_anchors").and_then(|value| value.as_array()).is_some()
                        && item.get("revision_directive").and_then(|value| value.as_str()).is_some()
                        && item.get("reverification_scope").and_then(|value| value.as_array()).is_some()
                })
        );
        assert!(
            report_response.content["paper"]["latex_outline"]
                .as_str()
                .unwrap_or("")
                .contains("\\section{Method}")
        );
        assert!(
            report_response.content["paper"]["manuscript_master_prompt"]
                .as_str()
                .unwrap_or("")
                .contains("official APIs")
        );
        assert!(
            report_response.content["paper"]["manuscript_master_prompt"]
                .as_str()
                .unwrap_or("")
                .contains("official dataset databases")
        );
        assert!(
            report_response.content["paper"]["markdown_draft"]
                .as_str()
                .unwrap_or("")
                .contains("## Method")
        );
        assert!(
            report_response.content["paper"]["tables_figures_plan"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item["artifact_id"] == "main_results_table")
        );
        assert!(
            report_response.content["paper"]["citation_inventory"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len()
                >= 1
        );
        assert_eq!(
            report_response.content["paper"]["artifact_appendix_plan"]["lineage_required"],
            true
        );
        assert!(
            report_response.content["paper"]["completion_protocol"]["final_artifacts"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "paper.tex")
        );
        assert!(
            report_response.content["paper"]["quality_checklist"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item["name"] == "verification_gap_disclosure")
        );
    });
}

#[test]
fn scientist_agents_advertise_expected_capabilities() {
    let research = ResearchAgent::new("research-1");
    let hypothesis = HypothesisAgent::new("hypothesis-1");
    let experiment = ExperimentAgent::new("experiment-1");
    let verification = VerificationAgent::new("verification-1");
    let report = ReportAgent::new("report-1");

    assert_eq!(research.role(), AgentRole::Researcher);
    assert_eq!(hypothesis.role(), AgentRole::Hypothesizer);
    assert_eq!(experiment.role(), AgentRole::Experimenter);
    assert_eq!(verification.role(), AgentRole::Verifier);
    assert_eq!(report.role(), AgentRole::Reporter);

    let research_capabilities = research.capabilities();
    assert!(research_capabilities
        .iter()
        .any(|cap| cap.required_tools.contains(&"search_paper".to_string())));

    let hypothesis_capabilities = hypothesis.capabilities();
    assert_eq!(hypothesis_capabilities.len(), 1);
    assert_eq!(hypothesis_capabilities[0].name, "problem_formulation");

    let experiment_capabilities = experiment.capabilities();
    assert_eq!(experiment_capabilities[0].name, "benchmark_design");
    assert!(experiment_capabilities[0]
        .required_tools
        .contains(&"run_python".to_string()));

    let verification_capabilities = verification.capabilities();
    assert!(verification_capabilities
        .iter()
        .any(|cap| cap.required_tools.contains(&"lean_verify".to_string())));

    let report_capabilities = report.capabilities();
    assert!(report_capabilities
        .iter()
        .any(|cap| cap.required_tools.contains(&"generate_latex".to_string())));
    assert!(report_capabilities
        .iter()
        .any(|cap| cap.name == "section_prompt_orchestration"));
    assert!(report_capabilities
        .iter()
        .any(|cap| cap.name == "manuscript_bundle_assembly"));
}

#[test]
fn verification_agent_accepts_theory_profile_with_report_artifact() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let report = temp_dir.path().join("proof_report.md");
        fs::write(
            &report,
            "# proof\nDefinition: graph cut.\nLemma 1.\nProof sketch.\nCounterexample search: none found.\n",
        )
        .expect("write theory report");
        let context = AgentContext::new("scientist-theory-profile")
            .with_goal("Verify theory profile coverage");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "proof notes prepared",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "theory",
                            "datasets": [{
                                "dataset_id": "formal_problem_instance",
                                "provider": "local",
                                "path": "proof_notes.md",
                                "format": "markdown"
                            }],
                            "metrics": [{
                                "name": "proof_status",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "documented_reference_baseline",
                                "kind": "prior_work_or_existing_system"
                            }],
                            "artifacts": [{
                                "name": "proof_report",
                                "kind": "report",
                                "required": true
                            }, {
                                "name": "lemma_notes",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "formal_reasoning_workflow",
                                "stages": [{"stage_id": "prove"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "theory_result_bundle",
                                "summary_fields": [{"name": "proof_status"}]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["proof_status"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "proof_report.md"
                        ],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "theory-run-1"},
                                {"name": "proof_status", "value": "proof sketch completed for the min-cut invariant"},
                                {"name": "lemma_summary", "value": "Lemma 1 establishes the residual-capacity invariant used by the proof."},
                                {"name": "counterexample_status", "value": "Checked small edge-case graphs and found no counterexample."}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["proof_status", "lemma_coverage"],
                            "observations": ["Compared the latest proof attempt against the baseline sketch."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "theory-run-1",
                                "change_summary": "Refined the invariant statement.",
                                "artifact_paths": ["proof_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["benchmark_verifier"]["benchmark_profile"],
            "theory"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["execution_schema_check"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["result_bundle_check"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["lineage_check"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "passed"
        );
    });
}

#[test]
fn verification_agent_accepts_literature_profile_with_manifest_and_report() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let manifest = temp_dir.path().join("search_manifest.json");
        let report = temp_dir.path().join("synthesis_report.md");
        fs::write(
            &manifest,
            "{\"query\":\"agent evaluation\",\"search_scope\":\"benchmark databases and arXiv\",\"screening\":\"include empirical agent evaluations\",\"screened_papers\":12,\"included_papers\":5,\"excluded_papers\":7}",
        )
        .expect("write manifest");
        fs::write(
            &report,
            "# synthesis\nScreening and inclusion criteria documented.\nscreened_papers: 12\nincluded_papers: 5\nexcluded_papers: 7\nComparison across tool-use agents.\nResearch gap: weak multi-turn repair evidence.\n",
        )
        .expect("write report");
        let context = AgentContext::new("scientist-literature-profile")
            .with_goal("Verify literature review profile coverage");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "screening notes prepared",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "literature_review",
                            "datasets": [{
                                "dataset_id": "paper_corpus",
                                "provider": "local",
                                "path": "papers.csv",
                                "format": "csv"
                            }],
                            "metrics": [{
                                "name": "screening_summary",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "documented_reference_baseline",
                                "kind": "prior_work_or_existing_system"
                            }],
                            "artifacts": [{
                                "name": "search_manifest",
                                "kind": "data_manifest",
                                "required": true
                            }, {
                                "name": "synthesis_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "evidence_synthesis_workflow",
                                "stages": [{"stage_id": "screen"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "literature_review_result_bundle",
                                "summary_fields": [{"name": "gap_summary"}]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["included_papers"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "search_manifest.json",
                            "synthesis_report.md"
                        ],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "lit-run-2"},
                                {"name": "search_scope", "value": "Queried OpenAlex, Semantic Scholar, and arXiv for agent evaluation and repair terms."},
                                {"name": "screening_summary", "value": "Screened 12 papers, included 5 empirical evaluations, excluded 7 off-scope or tutorial papers."},
                                {"name": "remote_fulltext_coverage", "value": "5 papers with remote-first fulltext and 4 with direct PDF-backed structured sections."},
                                {"name": "structured_paper_coverage", "value": "4 papers include structured sections with references and section headings."},
                                {"name": "gap_summary", "value": "The remaining gap is weak evidence on multi-turn repair robustness under tool failure."}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["screened_papers", "included_papers", "remote_fulltext_papers", "structured_papers"],
                            "observations": ["Compared inclusion counts before and after query refinement."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 2,
                            "history": [{
                                "run_id": "lit-run-1",
                                "variant_label": "initial-search",
                                "change_summary": "Initial broad search over agent evaluation papers.",
                                "artifact_paths": ["search_manifest.json"]
                            },{
                                "run_id": "lit-run-2",
                                "variant_label": "refined-search",
                                "change_summary": "Refined queries and exclusion criteria.",
                                "artifact_paths": ["search_manifest.json","synthesis_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["benchmark_verifier"]["benchmark_profile"],
            "literature_review"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["artifact_check"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["profile_value_validation"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "passed"
        );
    });
}

#[test]
fn verification_agent_flags_missing_agent_eval_runtime_bundle_fields() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let context = AgentContext::new("scientist-agent-eval-runtime")
            .with_goal("Verify agent evaluation runtime bundle coverage");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "agent_evaluation",
                            "datasets": [{"dataset_id": "task_suite_v1"}],
                            "metrics": [{"name": "task_success_rate"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "trajectory_bundle",
                                "kind": "report",
                                "required": true
                            },{
                                "name": "evaluation_script",
                                "kind": "executable",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "evaluation_orchestrator",
                                "stages": [{"stage_id": "evaluate"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "agent_evaluation_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "task_success_rate"},
                                    {"name": "tool_error_rate"},
                                    {"name": "judge_summary"},
                                    {"name": "trajectory_sample_count"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["task_success_rate", "trajectory_cost", "tool_error_rate"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id"},
                                {"name": "task_success_rate"}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["task_success_rate"],
                            "observations": []
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "eval-run-1",
                                "variant_label": "baseline"
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["result_bundle_validation"]["status"],
            "failed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["run_comparison_validation"]["status"],
            "failed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["lineage_validation"]["status"],
            "failed"
        );
    });
}

#[test]
fn verification_agent_flags_inconsistent_literature_screening_counts() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let manifest = temp_dir.path().join("search_manifest.json");
        let report = temp_dir.path().join("synthesis_report.md");
        fs::write(
            &manifest,
            "{\"query\":\"agent evaluation\",\"search_scope\":\"benchmark databases\",\"screened_papers\":10,\"included_papers\":8,\"excluded_papers\":5}",
        )
        .expect("write manifest");
        fs::write(
            &report,
            "# synthesis\nScreening inclusion exclusion criteria documented.\nSynthesis compares agent evaluation papers.\nResearch gap: weak repair evidence.\n",
        )
        .expect("write report");

        let context = AgentContext::new("scientist-literature-counts")
            .with_goal("Verify literature screening count consistency");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "literature_review",
                            "datasets": [{"dataset_id": "paper_corpus"}],
                            "metrics": [{"name": "screening_summary"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "search_manifest",
                                "kind": "data_manifest",
                                "required": true
                            }, {
                                "name": "synthesis_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "evidence_synthesis_workflow",
                                "stages": [{"stage_id": "screen"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "literature_review_result_bundle",
                                "summary_fields": [{"name": "search_scope"}]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["screened_papers", "included_papers"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["search_manifest.json", "synthesis_report.md"]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "failed"
        );
        assert!(
            response.content["specialized_profile_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "screening_count_consistency")
        );
    });
}

#[test]
fn verification_agent_flags_placeholder_literature_runtime_values() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let manifest = temp_dir.path().join("search_manifest.json");
        let report = temp_dir.path().join("synthesis_report.md");
        fs::write(
            &manifest,
            "{\"query\":\"deep learning systems\",\"search_scope\":\"openalex and arxiv\",\"screened_papers\":6,\"included_papers\":3,\"excluded_papers\":3}",
        )
        .expect("write manifest");
        fs::write(
            &report,
            "# synthesis\nScreening and synthesis documented.\nResearch gap: limited deployment evidence.\n",
        )
        .expect("write report");

        let context = AgentContext::new("scientist-literature-placeholder-values")
            .with_goal("Reject placeholder literature runtime values");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "literature_review",
                            "datasets": [{"dataset_id": "paper_corpus"}],
                            "metrics": [{"name": "screening_summary"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "search_manifest",
                                "kind": "data_manifest",
                                "required": true
                            }, {
                                "name": "synthesis_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "evidence_synthesis_workflow",
                                "stages": [{"stage_id": "screen"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "literature_review_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "search_scope"},
                                    {"name": "screening_summary"},
                                    {"name": "remote_fulltext_coverage"},
                                    {"name": "structured_paper_coverage"},
                                    {"name": "gap_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["screened_papers", "included_papers", "remote_fulltext_papers", "structured_papers"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["search_manifest.json", "synthesis_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "lit-run-1"},
                                {"name": "search_scope", "value": "OpenAlex and arXiv query over deep learning systems papers."},
                                {"name": "screening_summary", "value": "Screened 6 papers and included 3."},
                                {"name": "remote_fulltext_coverage", "value": "remote fulltext coverage pending"},
                                {"name": "structured_paper_coverage", "value": "structured paper evidence pending"},
                                {"name": "gap_summary", "value": "Need stronger deployment evidence."}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["screened_papers", "included_papers", "remote_fulltext_papers", "structured_papers"],
                            "observations": ["Compared counts after query refinement."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "lit-run-1",
                                "variant_label": "initial-search",
                                "change_summary": "Initial search scope.",
                                "artifact_paths": ["search_manifest.json", "synthesis_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["profile_value_validation"]["status"],
            "failed"
        );
        assert!(
            response.content["runtime_result_verification"]["profile_value_validation"]["issues"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "remote_fulltext_coverage")
        );
        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "failed"
        );
    });
}

#[test]
fn verification_agent_flags_placeholder_theory_runtime_values() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let report = temp_dir.path().join("proof_report.md");
        fs::write(
            &report,
            "# proof\nDefinition: graph cut.\nLemma 1.\nProof sketch.\nCounterexample search documented.\n",
        )
        .expect("write theory report");

        let context = AgentContext::new("scientist-theory-placeholder-values")
            .with_goal("Reject placeholder theory runtime values");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "theory",
                            "datasets": [{"dataset_id": "formal_problem_instance"}],
                            "metrics": [{"name": "proof_status"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "proof_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "formal_reasoning_workflow",
                                "stages": [{"stage_id": "prove"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "theory_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "proof_status"},
                                    {"name": "lemma_summary"},
                                    {"name": "counterexample_status"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["proof_status", "lemma_coverage"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["proof_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "theory-run-2"},
                                {"name": "proof_status", "value": "proof evidence observed"},
                                {"name": "lemma_summary", "value": "lemma evidence pending"},
                                {"name": "counterexample_status", "value": "counterexample search pending"}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["proof_status", "lemma_coverage"],
                            "observations": ["Compared the latest proof attempt against the baseline sketch."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "theory-run-2",
                                "change_summary": "Refined proof notes.",
                                "artifact_paths": ["proof_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["profile_value_validation"]["status"],
            "failed"
        );
        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "failed"
        );
        assert!(
            response.content["specialized_profile_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "proof_status")
        );
    });
}

#[test]
fn verification_agent_flags_theory_runtime_values_without_theory_semantics() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let report = temp_dir.path().join("proof_report.md");
        fs::write(
            &report,
            "# proof\nDefinition: graph cut.\nLemma 1.\nProof sketch.\nCounterexample search documented.\n",
        )
        .expect("write theory report");

        let context = AgentContext::new("scientist-theory-semantic-alignment")
            .with_goal("Reject theory runtime values without theory semantics");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "theory",
                            "datasets": [{"dataset_id": "formal_problem_instance"}],
                            "metrics": [{"name": "proof_status"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "proof_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "formal_reasoning_workflow",
                                "stages": [{"stage_id": "prove"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "theory_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "proof_status"},
                                    {"name": "lemma_summary"},
                                    {"name": "counterexample_status"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["proof_status", "lemma_coverage"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["proof_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "theory-run-3"},
                                {"name": "proof_status", "value": "status updated after review"},
                                {"name": "lemma_summary", "value": "summary updated for this run"},
                                {"name": "counterexample_status", "value": "notes updated for this run"}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["proof_status", "lemma_coverage"],
                            "observations": ["Compared the latest proof attempt against the baseline sketch."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "theory-run-3",
                                "change_summary": "Refined proof notes.",
                                "artifact_paths": ["proof_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "failed"
        );
        assert!(
            response.content["specialized_profile_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "proof_status_semantic_alignment")
        );
    });
}

#[test]
fn verification_agent_flags_literature_runtime_values_without_remote_or_structured_semantics() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let manifest = temp_dir.path().join("search_manifest.json");
        let report = temp_dir.path().join("synthesis_report.md");
        fs::write(
            &manifest,
            "{\"query\":\"deep learning systems\",\"search_scope\":\"openalex and arxiv\",\"screened_papers\":6,\"included_papers\":3,\"excluded_papers\":3}",
        )
        .expect("write manifest");
        fs::write(
            &report,
            "# synthesis\nScreening and synthesis documented.\nResearch gap: limited deployment evidence.\n",
        )
        .expect("write report");

        let context = AgentContext::new("scientist-literature-semantic-alignment")
            .with_goal("Reject literature runtime values without remote/structured semantics");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "literature_review",
                            "datasets": [{"dataset_id": "paper_corpus"}],
                            "metrics": [{"name": "screening_summary"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "search_manifest",
                                "kind": "data_manifest",
                                "required": true
                            }, {
                                "name": "synthesis_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "evidence_synthesis_workflow",
                                "stages": [{"stage_id": "screen"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "literature_review_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "search_scope"},
                                    {"name": "screening_summary"},
                                    {"name": "remote_fulltext_coverage"},
                                    {"name": "structured_paper_coverage"},
                                    {"name": "gap_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["screened_papers", "included_papers", "remote_fulltext_papers", "structured_papers"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["search_manifest.json", "synthesis_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "lit-run-3"},
                                {"name": "search_scope", "value": "OpenAlex and arXiv query over deep learning systems papers."},
                                {"name": "screening_summary", "value": "Screened 6 papers and included 3."},
                                {"name": "remote_fulltext_coverage", "value": "3 documents processed in this pass"},
                                {"name": "structured_paper_coverage", "value": "2 documents processed in this pass"},
                                {"name": "gap_summary", "value": "Need stronger deployment evidence."}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["screened_papers", "included_papers", "remote_fulltext_papers", "structured_papers"],
                            "observations": ["Compared counts after query refinement."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "lit-run-3",
                                "variant_label": "initial-search",
                                "change_summary": "Initial search scope.",
                                "artifact_paths": ["search_manifest.json", "synthesis_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "failed"
        );
        assert!(
            response.content["specialized_profile_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "remote_fulltext_semantic_alignment")
        );
        assert!(
            response.content["specialized_profile_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "structured_paper_semantic_alignment")
        );
    });
}

#[test]
fn verification_agent_reports_artifact_inventory_from_workspace_root() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        let existing_artifact = temp_dir.path().join("metrics.md");
        fs::write(&existing_artifact, "# metrics\naccuracy: 0.97\n").expect("write artifact");

        let context = AgentContext::new("scientist-artifact-verifier")
            .with_goal("Verify CS experiment artifacts are present on disk");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "lightweight run finished",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{
                                "dataset_id": "iris",
                                "provider": "sklearn",
                                "path": "builtin://iris",
                                "format": "tabular"
                            }],
                            "dataset_acquisition": {
                                "retrieval_mode": "public_dataset_entrypoint",
                                "retrieval_entrypoint": "official_dataset_databases",
                                "search_tool": "search_public_datasets",
                                "manifest_tool": "fetch_public_dataset_manifest",
                                "search_queries": ["iris classification dataset"],
                                "preferred_providers": ["openml", "huggingface"],
                                "expected_manifest_fields": [
                                    {"name": "dataset_id", "required": true},
                                    {"name": "provider", "required": true},
                                    {"name": "path", "required": true},
                                    {"name": "format", "required": true}
                                ],
                                "selection_guidance": "Freeze the dataset manifest before training.",
                                "paper_source_policy": "official_paper_apis_only"
                            },
                            "metrics": [{
                                "name": "accuracy",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "majority_class_baseline",
                                "kind": "sanity_check"
                            }],
                            "artifacts": [{
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "training_pipeline",
                                "stages": [{"stage_id": "train_eval"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "classical_ml_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "primary_metric"},
                                    {"name": "baseline_delta"},
                                    {"name": "error_analysis_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["accuracy", "f1", "fit_time_seconds"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "metrics.md",
                            "confusion_matrix.png"
                        ]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(response.content["benchmark_verifier"]["status"], "needs_attention");
        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["status"],
            "passed"
        );
        assert_eq!(response.content["artifact_inventory"]["status"], "failed");
        assert_eq!(response.content["artifact_contract"]["status"], "failed");
        assert_eq!(
            response.content["verification"]["implementation_sanity"],
            "needs_attention"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["artifact_check"]["inventory_status"],
            "failed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["artifact_check"]["contract_status"],
            "failed"
        );

        let present = response.content["artifact_inventory"]["present_artifacts"]
            .as_array()
            .expect("present artifacts array");
        assert_eq!(present.len(), 1);
        assert_eq!(present[0]["path"], "metrics.md");

        let missing = response.content["artifact_inventory"]["missing_artifacts"]
            .as_array()
            .expect("missing artifacts array");
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0]["path"], "confusion_matrix.png");

        let contract_missing = response.content["artifact_contract"]["missing_required_artifacts"]
            .as_array()
            .expect("missing required artifacts array");
        assert_eq!(contract_missing.len(), 1);
        assert_eq!(contract_missing[0]["name"], "train_script");
    });
}

#[test]
fn verification_agent_reports_passing_artifact_contract_when_roles_are_covered() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(
            temp_dir.path().join("train_and_eval.py"),
            "print('ok')\n",
        )
        .expect("write train script");
        fs::write(
            temp_dir.path().join("metrics_report.md"),
            "# metrics\naccuracy: 0.97\n",
        )
        .expect("write metrics report");

        let context = AgentContext::new("scientist-artifact-contract")
            .with_goal("Verify CS artifact roles are covered on disk");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "lightweight run finished",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{
                                "dataset_id": "iris",
                                "provider": "sklearn",
                                "path": "builtin://iris",
                                "format": "tabular"
                            }],
                            "dataset_acquisition": {
                                "retrieval_mode": "public_dataset_entrypoint",
                                "retrieval_entrypoint": "official_dataset_databases",
                                "search_tool": "search_public_datasets",
                                "manifest_tool": "fetch_public_dataset_manifest",
                                "search_queries": ["iris classification dataset"],
                                "preferred_providers": ["openml", "huggingface"],
                                "expected_manifest_fields": [
                                    {"name": "dataset_id", "required": true},
                                    {"name": "provider", "required": true},
                                    {"name": "path", "required": true},
                                    {"name": "format", "required": true}
                                ],
                                "selection_guidance": "Freeze the dataset manifest before training.",
                                "paper_source_policy": "official_paper_apis_only"
                            },
                            "metrics": [{
                                "name": "accuracy",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "majority_class_baseline",
                                "kind": "sanity_check"
                            }],
                            "artifacts": [{
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "training_pipeline",
                                "stages": [{"stage_id": "train_eval"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "classical_ml_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "primary_metric"},
                                    {"name": "baseline_delta"},
                                    {"name": "error_analysis_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["accuracy", "f1", "fit_time_seconds"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "train_and_eval.py",
                            "metrics_report.md"
                        ],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "ml-run-1"},
                                {"name": "primary_metric", "value": "accuracy 0.97"},
                                {"name": "baseline_delta", "value": "+0.09 over majority baseline"},
                                {"name": "error_analysis_summary", "value": "Remaining mistakes concentrate on the versicolor-versginica boundary."}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["accuracy", "f1", "fit_time_seconds"],
                            "observations": ["Compared the latest run against the majority baseline."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "ml-run-1",
                                "parent_run_id": "baseline",
                                "variant_label": "logreg-v1",
                                "change_summary": "Added standardized features and tuned C.",
                                "artifact_paths": ["train_and_eval.py", "metrics_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(response.content["benchmark_verifier"]["status"], "passed");
        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["status"],
            "passed"
        );
        assert_eq!(response.content["artifact_inventory"]["status"], "passed");
        assert_eq!(response.content["artifact_contract"]["status"], "passed");
        assert_eq!(
            response.content["verification"]["implementation_sanity"],
            "confirmed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["artifact_check"]["inventory_status"],
            "passed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["artifact_check"]["contract_status"],
            "passed"
        );

        let matched = response.content["artifact_contract"]["matched_required_artifacts"]
            .as_array()
            .expect("matched required artifacts array");
        assert_eq!(matched.len(), 2);
        assert_eq!(response.content["metric_report_check"]["status"], "passed");
        assert_eq!(
            response.content["benchmark_verifier"]["metric_check"]["report_status"],
            "passed"
        );
        let matched_metrics = response.content["metric_report_check"]["matched_metrics"]
            .as_array()
            .expect("matched metrics array");
        assert_eq!(matched_metrics.len(), 1);
        assert_eq!(matched_metrics[0], "accuracy");
        let observed_metrics = response.content["metric_report_check"]["observed_metrics"]
            .as_array()
            .expect("observed metrics array");
        assert_eq!(observed_metrics.len(), 1);
        assert_eq!(observed_metrics[0]["metric"], "accuracy");
        assert_eq!(observed_metrics[0]["value_text"], "0.97");
        assert_eq!(observed_metrics[0]["value"], 0.97);
    });
}

#[test]
fn verification_agent_flags_missing_metric_mentions_in_report_content() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(temp_dir.path().join("train_and_eval.py"), "print('ok')\n")
            .expect("write train script");
        fs::write(
            temp_dir.path().join("metrics_report.md"),
            "# metrics\naccuracy: 0.97\n",
        )
        .expect("write metrics report");

        let context = AgentContext::new("scientist-metric-report-check")
            .with_goal("Verify declared metrics are reflected in CS experiment reports");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "lightweight run finished",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{
                                "dataset_id": "iris",
                                "provider": "sklearn",
                                "path": "builtin://iris",
                                "format": "tabular"
                            }],
                            "metrics": [{
                                "name": "accuracy",
                                "direction": "maximize"
                            }, {
                                "name": "f1",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "majority_class_baseline",
                                "kind": "sanity_check"
                            }],
                            "artifacts": [{
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "train_and_eval.py",
                            "metrics_report.md"
                        ]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(response.content["metric_report_check"]["status"], "failed");
        assert_eq!(
            response.content["benchmark_verifier"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["metric_check"]["report_status"],
            "failed"
        );
        assert_eq!(
            response.content["verification"]["implementation_sanity"],
            "needs_attention"
        );
        let missing_metrics = response.content["metric_report_check"]["missing_metrics"]
            .as_array()
            .expect("missing metrics array");
        assert_eq!(missing_metrics.len(), 1);
        assert_eq!(missing_metrics[0], "f1");
        assert!(response.content["benchmark_verifier"]["missing_items"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .any(|item| item == "metric_reports"));
    });
}

#[test]
fn verification_agent_flags_metrics_without_extractable_values() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(temp_dir.path().join("train_and_eval.py"), "print('ok')\n")
            .expect("write train script");
        fs::write(
            temp_dir.path().join("metrics_report.md"),
            "# metrics\naccuracy is strong on this split\n",
        )
        .expect("write metrics report");

        let context = AgentContext::new("scientist-metric-value-check")
            .with_goal("Verify metric mentions also include concrete values");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "lightweight run finished",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{
                                "dataset_id": "iris",
                                "provider": "sklearn",
                                "path": "builtin://iris",
                                "format": "tabular"
                            }],
                            "metrics": [{
                                "name": "accuracy",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "majority_class_baseline",
                                "kind": "sanity_check"
                            }],
                            "artifacts": [{
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "train_and_eval.py",
                            "metrics_report.md"
                        ]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(response.content["metric_report_check"]["status"], "failed");
        assert_eq!(
            response.content["benchmark_verifier"]["status"],
            "needs_attention"
        );
        let metrics_without_values = response.content["metric_report_check"]
            ["metrics_without_values"]
            .as_array()
            .expect("metrics_without_values array");
        assert_eq!(metrics_without_values.len(), 1);
        assert_eq!(metrics_without_values[0], "accuracy");
        let observed_metrics = response.content["metric_report_check"]["observed_metrics"]
            .as_array()
            .expect("observed metrics array");
        assert_eq!(observed_metrics.len(), 1);
        assert_eq!(observed_metrics[0]["metric"], "accuracy");
        assert!(observed_metrics[0].get("value").is_none());
    });
}

#[test]
fn verification_agent_extracts_metric_values_from_json_report() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(temp_dir.path().join("train_and_eval.py"), "print('ok')\n")
            .expect("write train script");
        fs::write(
            temp_dir.path().join("metrics_report.json"),
            r#"{"accuracy": 0.961, "f1": 0.944}"#,
        )
        .expect("write metrics report");

        let context = AgentContext::new("scientist-json-metric-check")
            .with_goal("Verify structured JSON metric reports can be read");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "json metrics emitted",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{
                                "dataset_id": "iris",
                                "provider": "sklearn",
                                "path": "builtin://iris",
                                "format": "tabular"
                            }],
                            "metrics": [{
                                "name": "accuracy",
                                "direction": "maximize"
                            }, {
                                "name": "f1",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "majority_class_baseline",
                                "kind": "sanity_check"
                            }],
                            "artifacts": [{
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "train_and_eval.py",
                            "metrics_report.json"
                        ]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(response.content["metric_report_check"]["status"], "passed");
        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["status"],
            "passed"
        );
        let observed_metrics = response.content["metric_report_check"]["observed_metrics"]
            .as_array()
            .expect("observed metrics array");
        assert_eq!(observed_metrics.len(), 2);
        assert_eq!(observed_metrics[0]["source_kind"], "json");
        assert_eq!(observed_metrics[0]["source_path"], "metrics_report.json");
    });
}

#[test]
fn verification_agent_extracts_metric_values_from_csv_report() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(temp_dir.path().join("train_and_eval.py"), "print('ok')\n")
            .expect("write train script");
        fs::write(
            temp_dir.path().join("metrics_report.csv"),
            "accuracy,f1\n0.97,0.95\n",
        )
        .expect("write metrics report");

        let context = AgentContext::new("scientist-csv-metric-check")
            .with_goal("Verify structured CSV metric reports can be read");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "csv metrics emitted",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{
                                "dataset_id": "iris",
                                "provider": "sklearn",
                                "path": "builtin://iris",
                                "format": "tabular"
                            }],
                            "metrics": [{
                                "name": "accuracy",
                                "direction": "maximize"
                            }, {
                                "name": "f1",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "majority_class_baseline",
                                "kind": "sanity_check"
                            }],
                            "artifacts": [{
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "train_and_eval.py",
                            "metrics_report.csv"
                        ]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(response.content["metric_report_check"]["status"], "passed");
        let observed_metrics = response.content["metric_report_check"]["observed_metrics"]
            .as_array()
            .expect("observed metrics array");
        assert_eq!(observed_metrics.len(), 2);
        assert_eq!(observed_metrics[0]["source_kind"], "csv");
    });
}

#[test]
fn verification_agent_flags_metric_values_outside_cs_sanity_range() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(temp_dir.path().join("train_and_eval.py"), "print('ok')\n")
            .expect("write train script");
        fs::write(
            temp_dir.path().join("metrics_report.json"),
            r#"{"accuracy": 1.7}"#,
        )
        .expect("write metrics report");

        let context = AgentContext::new("scientist-sanity-metric-check")
            .with_goal("Verify impossible CS metric values are flagged");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "sanity edge case",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{
                                "dataset_id": "iris",
                                "provider": "sklearn",
                                "path": "builtin://iris",
                                "format": "tabular"
                            }],
                            "metrics": [{
                                "name": "accuracy",
                                "direction": "maximize"
                            }],
                            "baselines": [{
                                "name": "majority_class_baseline",
                                "kind": "sanity_check"
                            }],
                            "artifacts": [{
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "train_and_eval.py",
                            "metrics_report.json"
                        ]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(response.content["metric_report_check"]["status"], "failed");
        let sanity_issues = response.content["metric_report_check"]["sanity_issues"]
            .as_array()
            .expect("sanity_issues array");
        assert_eq!(sanity_issues.len(), 1);
        assert_eq!(sanity_issues[0]["metric"], "accuracy");
        assert_eq!(sanity_issues[0]["value"], 1.7);
    });
}

#[test]
fn verification_agent_flags_profile_artifact_mismatch_for_systems_plan() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(temp_dir.path().join("benchmark_runner.py"), "print('ok')\n")
            .expect("write benchmark script");
        fs::write(
            temp_dir.path().join("metrics_report.md"),
            "# metrics\nlatency_ms: 12\nthroughput_ops_per_sec: 200\nmemory_mb: 128\n",
        )
        .expect("write metrics report");

        let context = AgentContext::new("scientist-profile-alignment-check")
            .with_goal("Verify profile-specific CS artifact expectations are enforced");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "systems benchmark emitted without manifest",
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "systems_evaluation",
                            "datasets": [{
                                "dataset_id": "runtime_trace_suite",
                                "provider": "local",
                                "path": "workloads/",
                                "format": "mixed"
                            }],
                            "metrics": [{
                                "name": "latency_ms",
                                "direction": "minimize"
                            }, {
                                "name": "throughput_ops_per_sec",
                                "direction": "maximize"
                            }, {
                                "name": "memory_mb",
                                "direction": "minimize"
                            }],
                            "baselines": [{
                                "name": "single_thread_baseline",
                                "kind": "reference"
                            }],
                            "artifacts": [{
                                "name": "benchmark_runner",
                                "kind": "executable",
                                "required": true
                            }, {
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }],
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": [
                            "benchmark_runner.py",
                            "metrics_report.md"
                        ]
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["status"],
            "failed"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["profile"],
            "systems_evaluation"
        );
        assert_eq!(
            response.content["benchmark_verifier"]["profile_check"]["artifact_alignment"],
            false
        );
        assert_eq!(
            response.content["benchmark_verifier"]["dataset_acquisition_check"]["status"],
            "failed"
        );
        assert!(
            response.content["benchmark_verifier"]["profile_check"]["missing_alignment_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "profile_artifacts")
        );
        assert!(
            response.content["benchmark_verifier"]["dataset_acquisition_check"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "dataset_acquisition")
        );
    });
}

#[test]
fn verification_agent_builds_profile_aware_repair_directive_from_verification_center() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(
            temp_dir.path().join("systems_report.md"),
            "# systems\nLatency p95: 12 ms\nThroughput: 200 ops/s\nMemory: 128 MB\n",
        )
        .expect("write systems report");

        let context = AgentContext::new("scientist-verification-center-repair")
            .with_goal("Verify repair directives consume verification center bundles");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "systems_evaluation",
                            "datasets": [{"dataset_id": "trace_suite"}],
                            "metrics": [{"name": "latency_ms"}],
                            "baselines": [{"name": "single_thread_baseline"}],
                            "artifacts": [{
                                "name": "systems_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "benchmark_pipeline",
                                "stages": [{"stage_id": "measure"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "systems_evaluation_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "workload_name"},
                                    {"name": "latency_summary"},
                                    {"name": "throughput_summary"},
                                    {"name": "resource_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["latency_ms", "throughput_ops_per_sec", "memory_mb"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["systems_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "sys-run-1"},
                                {"name": "workload_name", "value": "benchmark trial"},
                                {"name": "latency_summary", "value": "summary updated"},
                                {"name": "throughput_summary", "value": "throughput updated"},
                                {"name": "resource_summary", "value": "resource updated"}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["latency_ms", "throughput_ops_per_sec", "memory_mb"],
                            "observations": ["Compared the latest run against the previous baseline."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "sys-run-1",
                                "parent_run_id": "baseline",
                                "variant_label": "fast-path",
                                "change_summary": "Reduced queue depth.",
                                "artifact_paths": ["systems_report.md"]
                            }]
                        },
                        "verification_center": {
                            "verification_center": {
                                "summary": {
                                    "score": 48,
                                    "ready_tools": 3,
                                    "total_tools": 8
                                }
                            },
                            "bundle_runs": [{
                                "bundle_id": "systems_perf",
                                "bundle_score": 40,
                                "executed_tools": ["git"],
                                "skipped_tools": [{"tool": "hyperfine", "reason": "tool unavailable"}],
                                "runs": []
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["verification_center_repair"]["status"],
            "ready"
        );
        assert!(
            response.content["verification_center_repair"]["repair_directive"]
                .as_str()
                .unwrap_or("")
                .contains("latency")
        );
        assert_eq!(
            response.content["verification_center_repair"]["low_score_bundle_ids"][0],
            "systems_perf"
        );
        assert!(
            response.content["verification_center_repair"]["repair_checklist"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item["capability"] == "human_in_the_loop_feedback")
        );
        assert!(
            response.content["verification_center_repair"]["repair_checklist"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item["capability"] == "aliyun_qwen_product_fit")
        );
        assert!(
            response.content["verification_center_repair"]["competition_fit"]["gaps"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|gap| gap["capability"] == "traceable_result_lineage")
        );
        assert_eq!(
            response.content["verification_center_repair"]["runtime_summary"]["profile_fields"]["latency_numeric_signal"],
            0.0
        );
        assert_eq!(
            response.content["verification_center_repair"]["skipped_tools"]
                .as_array()
                .unwrap_or(&Vec::new())
                .len(),
            1
        );
        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert!(
            response.content["runtime_result_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "latency_summary_semantic_alignment")
        );
    });
}

#[test]
fn verification_agent_flags_deep_learning_runtime_values_without_training_semantics() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(
            temp_dir.path().join("deep_learning_report.md"),
            "# training\nCheckpoint saved after validation.\nGPU memory footprint recorded.\n",
        )
        .expect("write report");

        let context = AgentContext::new("scientist-deep-learning-semantics")
            .with_goal("Verify deep learning runtime values carry training semantics");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "deep_learning",
                            "datasets": [{"dataset_id": "vision_set"}],
                            "metrics": [{"name": "validation_accuracy"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "deep_learning_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "training_pipeline",
                                "stages": [{"stage_id": "train"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "deep_learning_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "checkpoint_path"},
                                    {"name": "best_validation_metric"},
                                    {"name": "resource_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["best_validation_metric", "training_time_minutes", "gpu_or_memory_footprint"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["deep_learning_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "dl-run-1"},
                                {"name": "checkpoint_path", "value": "/tmp/checkpoint.pt"},
                                {"name": "best_validation_metric", "value": "embedding vector"},
                                {"name": "resource_summary", "value": "status updated"}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["best_validation_metric", "training_time_minutes", "gpu_or_memory_footprint"],
                            "observations": ["Compared the latest checkpoint against the previous run."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "dl-run-1",
                                "parent_run_id": "baseline",
                                "variant_label": "resnet-variant",
                                "change_summary": "Adjusted augmentation policy.",
                                "artifact_paths": ["deep_learning_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["profile_value_validation"]["status"],
            "failed"
        );
        assert!(
            response.content["runtime_result_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "best_validation_metric_semantic_alignment")
        );
        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "failed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["runtime_summary"]["profile_fields"]["validation_metric_numeric_signal"],
            serde_json::Value::Null
        );
    });
}

#[test]
fn verification_agent_flags_security_runtime_values_without_finding_semantics() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(
            temp_dir.path().join("security_report.md"),
            "# security\nFinding coverage and impact recorded.\nCritical surface documented.\n",
        )
        .expect("write report");

        let context = AgentContext::new("scientist-security-semantics")
            .with_goal("Verify security runtime values carry finding semantics");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "security_analysis",
                            "datasets": [{"dataset_id": "target_surface"}],
                            "metrics": [{"name": "precision"}],
                            "baselines": [{"name": "documented_reference_baseline"}],
                            "artifacts": [{
                                "name": "security_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "security_pipeline",
                                "stages": [{"stage_id": "scan"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "security_analysis_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "confirmed_findings"},
                                    {"name": "false_positive_count"},
                                    {"name": "coverage_summary"},
                                    {"name": "impact_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["precision", "recall", "false_positive_rate"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["security_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "sec-run-1"},
                                {"name": "confirmed_findings", "value": "status updated"},
                                {"name": "false_positive_count", "value": "2"},
                                {"name": "coverage_summary", "value": "summary updated"},
                                {"name": "impact_summary", "value": "notes updated"}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["precision", "recall", "false_positive_rate"],
                            "observations": ["Compared the current scan against the previous baseline."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "sec-run-1",
                                "parent_run_id": "baseline",
                                "variant_label": "narrow-scan",
                                "change_summary": "Expanded target list.",
                                "artifact_paths": ["security_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["profile_value_validation"]["status"],
            "failed"
        );
        assert!(
            response.content["runtime_result_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "confirmed_findings_semantic_alignment")
        );
        assert_eq!(
            response.content["specialized_profile_verification"]["status"],
            "failed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["runtime_summary"]["profile_fields"]["confirmed_findings_numeric_signal"],
            serde_json::Value::Null
        );
    });
}

#[test]
fn verification_agent_flags_run_comparison_without_multi_run_lineage_closure() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(
            temp_dir.path().join("metrics_report.md"),
            "# metrics\naccuracy: 0.94\nf1: 0.91\n",
        )
        .expect("write report");

        let context = AgentContext::new("scientist-lineage-compare-closure")
            .with_goal("Verify run comparison requires multi-run lineage closure");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{"dataset_id": "iris"}],
                            "metrics": [{"name": "accuracy"}],
                            "baselines": [{"name": "majority_class_baseline"}],
                            "artifacts": [{
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }, {
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "training_pipeline",
                                "stages": [{"stage_id": "train_eval"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "classical_ml_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "primary_metric"},
                                    {"name": "baseline_delta"},
                                    {"name": "error_analysis_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["accuracy", "f1", "fit_time_seconds"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["metrics_report.md"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "ml-run-2"},
                                {"name": "primary_metric", "value": "accuracy 0.94"},
                                {"name": "baseline_delta", "value": "+0.07 over majority baseline"},
                                {"name": "error_analysis_summary", "value": "Errors cluster on versicolor boundary cases."}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["accuracy", "f1", "fit_time_seconds"],
                            "observations": ["Compared the latest run against the previous baseline."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 1,
                            "history": [{
                                "run_id": "ml-run-2",
                                "variant_label": "logreg-v2",
                                "change_summary": "Tuned regularization.",
                                "artifact_paths": ["metrics_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["run_comparison_validation"]["status"],
            "passed"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["lineage_validation"]["compare_lineage_closure_ok"],
            false
        );
        assert!(
            response.content["runtime_result_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "compare_lineage_closure")
        );
    });
}

#[test]
fn verification_agent_flags_artifact_lineage_mismatch_for_current_run() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(
            temp_dir.path().join("metrics_report.md"),
            "# metrics\naccuracy: 0.95\nf1: 0.92\n",
        )
        .expect("write report");
        fs::write(
            temp_dir.path().join("train_and_eval.py"),
            "print('ok')\n",
        )
        .expect("write train script");

        let context = AgentContext::new("scientist-artifact-lineage-closure")
            .with_goal("Verify current artifacts are represented in lineage closure");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "classical_ml",
                            "datasets": [{"dataset_id": "iris"}],
                            "metrics": [{"name": "accuracy"}],
                            "baselines": [{"name": "majority_class_baseline"}],
                            "artifacts": [{
                                "name": "metrics_report",
                                "kind": "report",
                                "required": true
                            }, {
                                "name": "train_script",
                                "kind": "executable",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "training_pipeline",
                                "stages": [{"stage_id": "train_eval"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "classical_ml_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "primary_metric"},
                                    {"name": "baseline_delta"},
                                    {"name": "error_analysis_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["accuracy", "f1", "fit_time_seconds"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["metrics_report.md", "train_and_eval.py"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "ml-run-3"},
                                {"name": "primary_metric", "value": "accuracy 0.95"},
                                {"name": "baseline_delta", "value": "+0.08 over majority baseline"},
                                {"name": "error_analysis_summary", "value": "Fewer errors on the minority class after feature scaling."}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["accuracy", "f1", "fit_time_seconds"],
                            "observations": ["Compared the latest run against ml-run-2."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 2,
                            "history": [{
                                "run_id": "ml-run-2",
                                "parent_run_id": "baseline",
                                "variant_label": "logreg-v2",
                                "change_summary": "Tuned regularization.",
                                "artifact_paths": ["metrics_report.md"]
                            }, {
                                "run_id": "ml-run-3",
                                "parent_run_id": "ml-run-2",
                                "variant_label": "logreg-v3",
                                "change_summary": "Added feature scaling.",
                                "artifact_paths": ["metrics_report.md"]
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["runtime_result_verification"]["status"],
            "needs_attention"
        );
        assert_eq!(
            response.content["runtime_result_verification"]["lineage_validation"]["artifact_lineage_closure_ok"],
            false
        );
        assert!(
            response.content["runtime_result_verification"]["missing_items"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .any(|item| item == "artifact_lineage_closure")
        );
    });
}

#[test]
fn verification_agent_closes_competition_fit_gaps_when_structured_inputs_are_present() {
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let temp_dir = tempdir().expect("tempdir");
        fs::write(
            temp_dir.path().join("agent_eval_report.md"),
            "# agent eval\nTask success: 0.81\nTrajectory coverage: 14 tasks\n",
        )
        .expect("write report");
        fs::write(
            temp_dir.path().join("agent_eval_manifest.json"),
            "{\"run\":\"agent-run-2\"}\n",
        )
        .expect("write manifest");

        let context = AgentContext::new("scientist-competition-fit-closure")
            .with_goal("Verify competition-fit gaps close when structured evidence is present");
        let verification = VerificationAgent::new("verification-1");

        let response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "benchmark_plan": {
                            "schema_version": "cs_benchmark_v1",
                            "benchmark_profile": "agent_evaluation",
                            "datasets": [{"dataset_id": "task_suite"}],
                            "metrics": [{"name": "task_success_rate"}],
                            "baselines": [{"name": "documented_reference_agent"}],
                            "artifacts": [{
                                "name": "agent_eval_report",
                                "kind": "report",
                                "required": true
                            }],
                            "execution_schema": {
                                "runner_kind": "agent_eval_pipeline",
                                "stages": [{"stage_id": "evaluate"}]
                            },
                            "result_bundle_schema": {
                                "bundle_kind": "agent_evaluation_result_bundle",
                                "summary_fields": [
                                    {"name": "run_id"},
                                    {"name": "task_success_summary"},
                                    {"name": "trajectory_summary"},
                                    {"name": "tool_use_summary"}
                                ]
                            },
                            "lineage_schema": {
                                "required": true,
                                "compare_keys": ["task_success_rate", "tool_call_count", "latency_ms"]
                            },
                            "reproducibility": {
                                "random_seed_required": true,
                                "fixed_split_required": true,
                                "environment_capture_required": true
                            }
                        },
                        "workspace_root": temp_dir.path().display().to_string(),
                        "artifact_paths": ["agent_eval_report.md", "agent_eval_manifest.json"],
                        "result_bundle": {
                            "summary_fields": [
                                {"name": "run_id", "value": "agent-run-2"},
                                {"name": "task_success_summary", "value": "task success rate 0.81 across 14 tasks"},
                                {"name": "trajectory_summary", "value": "trajectory comparison across the latest and prior runs"},
                                {"name": "tool_use_summary", "value": "tool use summary with 42 calls"}
                            ]
                        },
                        "run_comparison": {
                            "available": true,
                            "compare_keys": ["task_success_rate", "tool_call_count", "latency_ms"],
                            "observations": ["Compared the latest agent run against agent-run-1."]
                        },
                        "lineage": {
                            "available": true,
                            "run_count_hint": 2,
                            "history": [{
                                "run_id": "agent-run-1",
                                "parent_run_id": "baseline",
                                "variant_label": "agent-v1",
                                "change_summary": "Initial tool routing.",
                                "artifact_paths": ["agent_eval_report.md"]
                            }, {
                                "run_id": "agent-run-2",
                                "parent_run_id": "agent-run-1",
                                "variant_label": "agent-v2",
                                "change_summary": "Improved retrieval routing.",
                                "artifact_paths": ["agent_eval_report.md", "agent_eval_manifest.json"]
                            }]
                        },
                        "reviewer_feedback": [{
                            "reviewer": "panel-a",
                            "comment": "Need stronger trajectory attribution for failed tasks.",
                            "score": 87,
                            "resolved": false,
                            "linked_run_id": "agent-run-2"
                        }],
                        "graph_evidence": {
                            "graph_kind": "evidence_graph",
                            "nodes": [{"id": "paper_1"}, {"id": "task_1"}],
                            "edges": [{"from": "paper_1", "to": "task_1", "relation": "supports"}],
                            "sources": [{"kind": "paper"}, {"kind": "benchmark"}]
                        },
                        "aliyun_integration": {
                            "provider": "qwen",
                            "model": "qwen-plus",
                            "endpoint": "https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions",
                            "credential_mode": "env",
                            "route_mode": "bailian-compatible"
                        },
                        "multisource_evidence": {
                            "sources": [
                                {"kind": "benchmark", "name": "task_suite"},
                                {"kind": "trajectory", "name": "agent_trace_log"}
                            ],
                            "fusion_strategy": "join task outcomes with trajectory events by task_id",
                            "harmonized_fields": ["task_id", "status", "latency_ms"],
                            "conflict_resolution": "prefer benchmark status and preserve trajectory notes"
                        },
                        "verification_center": {
                            "verification_center": {
                                "summary": {
                                    "score": 86,
                                    "ready_tools": 6,
                                    "total_tools": 8
                                }
                            },
                            "bundle_runs": [{
                                "bundle_id": "agent_eval",
                                "bundle_score": 100,
                                "executed_tools": ["pytest", "git"],
                                "skipped_tools": [],
                                "runs": []
                            }]
                        }
                    }),
                ),
                &context,
            )
            .await
            .expect("verification response");

        assert_eq!(
            response.content["verification_center_repair"]["competition_fit"]["gap_count"],
            0
        );
        assert_eq!(
            response.content["verification_center_repair"]["reviewer_feedback_summary"]["available"],
            true
        );
        assert_eq!(
            response.content["verification_center_repair"]["graph_evidence_summary"]["available"],
            true
        );
        assert_eq!(
            response.content["verification_center_repair"]["aliyun_qwen_summary"]["provider_ok"],
            true
        );
        assert_eq!(
            response.content["verification_center_repair"]["multisource_evidence_summary"]["unique_source_kind_count"],
            2
        );
    });
}
