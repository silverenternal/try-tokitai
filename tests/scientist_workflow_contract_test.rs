use ai_assistant::scientist::{
    workflow::{ResearchStage, AI_SCIENTIST_WORKFLOW_TOML},
    ExperimentAgent, HypothesisAgent, ReportAgent, ResearchAgent, VerificationAgent,
};
use ai_scientist_core::agent::MessageType;
use ai_scientist_core::{Agent, AgentContext, AgentMessage, AgentRole};
use serde::Deserialize;
use serde_json::json;

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

    assert_eq!(workflow.workflow.id, "ai-scientist-minimal");
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
    assert!(step_ids.contains(&"generate_hypothesis"));
    assert!(step_ids.contains(&"design_experiment"));
    assert!(step_ids.contains(&"math_simplify"));
    assert!(step_ids.contains(&"summarize_results"));
    assert!(step_ids.contains(&"generate_output"));

    assert!(tools.contains(&"search_paper"));
    assert!(tools.contains(&"generate_hypothesis"));
    assert!(tools.contains(&"design_experiment"));
    assert!(tools.contains(&"sympy_simplify"));
    assert!(tools.contains(&"run_python"));

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
                        "query": "workflow contract testing"
                    }),
                ),
                &context,
            )
            .await
            .expect("research response");
        assert_eq!(research_response.next_role, Some(AgentRole::Hypothesizer));

        let hypothesis_response = hypothesis
            .handle_message(
                AgentMessage::new(
                    AgentRole::Researcher,
                    Some(AgentRole::Hypothesizer),
                    MessageType::Request,
                    json!({
                        "knowledge_summary": "workflow stages are aligned"
                    }),
                ),
                &context,
            )
            .await
            .expect("hypothesis response");
        assert_eq!(hypothesis_response.next_role, Some(AgentRole::Experimenter));
        assert_eq!(hypothesis_response.content["testable"], true);

        let experiment_response = experiment
            .handle_message(
                AgentMessage::new(
                    AgentRole::Hypothesizer,
                    Some(AgentRole::Experimenter),
                    MessageType::Request,
                    json!({
                        "hypothesis": "Aligned workflow contracts reduce regressions"
                    }),
                ),
                &context,
            )
            .await
            .expect("experiment response");
        assert_eq!(experiment_response.next_role, Some(AgentRole::Verifier));
        assert_eq!(experiment_response.content["status"], "Experiment designed");

        let verification_response = verification
            .handle_message(
                AgentMessage::new(
                    AgentRole::Experimenter,
                    Some(AgentRole::Verifier),
                    MessageType::Request,
                    json!({
                        "experiment_results": "contract checks passed"
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

        let report_response = report
            .handle_message(
                AgentMessage::new(
                    AgentRole::Verifier,
                    Some(AgentRole::Reporter),
                    MessageType::Request,
                    json!({
                        "all_results": "contract checks passed"
                    }),
                ),
                &context,
            )
            .await
            .expect("report response");
        assert!(report_response.success);
        assert_eq!(report_response.next_role, None);
        assert_eq!(report_response.content["paper"]["format"], "latex");
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
    assert!(
        research_capabilities
            .iter()
            .any(|cap| cap.required_tools.contains(&"search_paper".to_string()))
    );

    let hypothesis_capabilities = hypothesis.capabilities();
    assert_eq!(hypothesis_capabilities.len(), 1);
    assert_eq!(hypothesis_capabilities[0].name, "hypothesis_generation");

    let experiment_capabilities = experiment.capabilities();
    assert!(
        experiment_capabilities[0]
            .required_tools
            .contains(&"design_experiment".to_string())
    );

    let verification_capabilities = verification.capabilities();
    assert!(
        verification_capabilities
            .iter()
            .any(|cap| cap.required_tools.contains(&"lean_verify".to_string()))
    );

    let report_capabilities = report.capabilities();
    assert!(
        report_capabilities[0]
            .required_tools
            .contains(&"generate_latex".to_string())
    );
}
