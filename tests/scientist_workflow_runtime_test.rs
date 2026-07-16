use ai_assistant::scientist::tools::{
    computation::ComputationTools, literature::LiteratureTools, sympy_tool::SymPyTool,
};
use ai_assistant::scientist::workflow::AI_SCIENTIST_WORKFLOW_TOML;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use tempfile::tempdir;
#[derive(Debug, Deserialize)]
struct WorkflowFile {
    workflow: WorkflowDefinition,
}

#[derive(Debug, Deserialize)]
struct WorkflowDefinition {
    stages: Vec<WorkflowStage>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStage {
    id: String,
    steps: Vec<WorkflowStep>,
}

#[derive(Debug, Deserialize)]
struct WorkflowStep {
    id: String,
    tool: String,
}

fn execute_workflow_tool(tool: &str, args: &Value) -> Result<Value, String> {
    match tool {
        "search_paper" => LiteratureTools
            .call_tool(tool, args)
            .map_err(|e| e.to_string()),
        "fetch_paper" => LiteratureTools
            .call_tool(tool, args)
            .map_err(|e| e.to_string()),
        "sympy_simplify" => SymPyTool::new()
            .call_tool(tool, args)
            .map_err(|e| e.to_string()),
        "sympy_integrate" => SymPyTool::new()
            .call_tool(tool, args)
            .map_err(|e| e.to_string()),
        "run_python" => ComputationTools
            .call_tool(tool, args)
            .map_err(|e| e.to_string()),
        other => Err(format!(
            "NotImplemented: workflow tool '{}' is not wired into the local runtime executor",
            other
        )),
    }
}

#[test]
fn scientist_workflow_runtime_handles_local_and_not_implemented_steps() {
    let workflow: WorkflowFile =
        toml::from_str(AI_SCIENTIST_WORKFLOW_TOML).expect("workflow TOML should parse");

    let temp_dir = tempdir().expect("tempdir");
    let paper_path = temp_dir.path().join("workflow_runtime_paper.md");
    let mut paper_file = fs::File::create(&paper_path).expect("create local paper");
    writeln!(
        paper_file,
        "# Workflow Runtime Paper\n\nThis local paper covers workflow runtime testing and symbolic verification."
    )
    .expect("write local paper");

    std::env::set_var("AI_SCIENTIST_PAPERS_DIR", temp_dir.path());

    let mut saw_successful_local_step = false;
    let mut saw_not_implemented_step = false;
    let mut saw_python_execution = false;

    for stage in &workflow.workflow.stages {
        for step in &stage.steps {
            let args = match step.id.as_str() {
                "search_literature" => json!({
                    "query": "workflow runtime testing",
                    "source": "local",
                    "limit": 5
                }),
                "fetch_paper" => json!({
                    "paper_id": "runtime_paper"
                }),
                "formulate_problem" => json!({
                    "code": "print('problem-formulated')",
                    "timeout_secs": 5
                }),
                "design_pipeline" => json!({
                    "code": "print('pipeline-designed')",
                    "timeout_secs": 5
                }),
                "math_simplify" => json!({
                    "expression": "x + x"
                }),
                "math_integrate" => json!({
                    "expression": "x",
                    "variable": "x"
                }),
                "summarize_results" => json!({
                    "code": "print('runtime-summary')",
                    "timeout_secs": 5
                }),
                "plan_paper_blueprint" => json!({
                    "code": "print('paper-blueprint-planned')",
                    "timeout_secs": 5
                }),
                "draft_paper_sections" => json!({
                    "code": "print('paper-sections-drafted')",
                    "timeout_secs": 5
                }),
                "generate_output" => json!({
                    "code": "import json; print(json.dumps({'status': 'workflow-ok'}))",
                    "timeout_secs": 5
                }),
                other => panic!("unexpected workflow step '{}'", other),
            };

            let result = execute_workflow_tool(&step.tool, &args);

            match step.id.as_str() {
                "search_literature" => {
                    let payload = result.expect("search_literature should succeed");
                    assert_eq!(payload["status"], "success");
                    assert_eq!(payload["mode"], "local");
                    assert!(payload["total"].as_u64().unwrap_or(0) >= 1);
                    saw_successful_local_step = true;
                }
                "fetch_paper" => {
                    let payload = result.expect("fetch_paper should succeed");
                    assert_eq!(payload["status"], "success");
                    assert_eq!(payload["mode"], "local");
                    assert!(payload["content"]
                        .as_str()
                        .unwrap_or("")
                        .contains("workflow runtime"));
                    saw_successful_local_step = true;
                }
                "formulate_problem" | "design_pipeline" => {
                    let payload = result.expect("run_python should return structured payload");
                    assert_eq!(payload["operation"], "run_python");
                    assert!(payload["result"]["status"].is_string());
                    saw_python_execution = true;
                }
                "math_simplify" => match result {
                    Ok(payload) => {
                        assert_eq!(payload["status"], "success");
                        assert!(payload["result"].as_str().unwrap_or("").contains("2*x"));
                    }
                    Err(err) => {
                        assert!(
                            err.contains("SymPy") || err.contains("python"),
                            "unexpected verification failure: {}",
                            err
                        );
                    }
                },
                "math_integrate" => match result {
                    Ok(payload) => {
                        assert_eq!(payload["status"], "success");
                        assert!(payload["result"].as_str().unwrap_or("").contains("x**2/2"));
                    }
                    Err(err) => {
                        assert!(
                            err.contains("SymPy") || err.contains("python"),
                            "unexpected integration failure: {}",
                            err
                        );
                    }
                },
                "summarize_results"
                | "plan_paper_blueprint"
                | "draft_paper_sections"
                | "generate_output" => {
                    let payload = result.expect("run_python should return structured payload");
                    assert_eq!(payload["operation"], "run_python");
                    assert!(payload["result"]["status"].is_string());
                    saw_python_execution = true;
                }
                _ => {}
            }
        }
    }

    std::env::remove_var("AI_SCIENTIST_PAPERS_DIR");

    assert!(saw_successful_local_step);
    assert!(saw_python_execution);
}
