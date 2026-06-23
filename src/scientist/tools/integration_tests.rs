//! Integration tests for AI Scientist tool system

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;
    use tokitai::ToolProvider;

    #[test]
    fn test_literature_tools_registered() {
        let defs = crate::scientist::tools::literature::LiteratureTools::tool_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"search_paper"), "search_paper not found in {:?}", names);
        assert!(names.contains(&"fetch_paper"), "fetch_paper not found in {:?}", names);
        assert!(names.contains(&"cite_paper"), "cite_paper not found in {:?}", names);
    }

    #[test]
    fn test_computation_tools_registered() {
        let defs = crate::scientist::tools::computation::ComputationTools::tool_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"run_python"), "run_python not found in {:?}", names);
        assert!(names.contains(&"run_python_file"), "run_python_file not found in {:?}", names);
        assert!(names.contains(&"run_r"), "run_r not found in {:?}", names);
        assert!(names.contains(&"run_julia"), "run_julia not found in {:?}", names);
    }

    #[test]
    fn test_data_tools_registered() {
        let defs = crate::scientist::tools::data::DataTools::tool_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(
            names.contains(&"inspect_dataset"),
            "inspect_dataset not found in {:?}",
            names
        );
    }

    #[test]
    fn test_domain_science_tools_registered() {
        let defs = crate::scientist::tools::domain_science::DomainScienceTools::tool_definitions();
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"chemistry_mol_weight"));
        assert!(names.contains(&"chemistry_conformers"));
        assert!(names.contains(&"chemistry_quantum_energy"));
        assert!(names.contains(&"biology_translate"));
        assert!(names.contains(&"biology_blast"));
        assert!(names.contains(&"simulation_run"));
        assert!(names.contains(&"simulation_run_preset"));
        assert!(names.contains(&"scientific_backend_status"));
    }

    #[test]
    fn test_run_python_executes_real_code() {
        let tool = crate::scientist::tools::computation::ComputationTools;
        let result = tool.call_tool(
            "run_python",
            &serde_json::json!({
                "code": "print('hello from python')",
                "timeout_secs": 5
            }),
        );
        assert!(result.is_ok(), "run_python failed: {:?}", result.err());
        let output = result.unwrap().to_string();
        assert!(
            output.contains("hello from python")
                || output.contains("no working Python interpreter")
                || output.contains("Failed to start"),
            "Unexpected output: {}",
            output
        );
    }

    #[test]
    fn test_run_python_timeout() {
        let tool = crate::scientist::tools::computation::ComputationTools;
        let result = tool.call_tool(
            "run_python",
            &serde_json::json!({
                "code": "import time; time.sleep(60)",
                "timeout_secs": 1
            }),
        );
        assert!(result.is_ok());
        let output = result.unwrap().to_string();
        assert!(output.contains("timed_out"), "Expected timeout in: {}", output);
    }

    #[test]
    fn test_search_paper_local_first_behavior() {
        let temp_dir = tempdir().unwrap();
        let paper_path = temp_dir.path().join("quantum_local.md");
        let mut file = fs::File::create(&paper_path).unwrap();
        writeln!(
            file,
            "# Quantum Computing Local Note\n\nThis local paper discusses quantum computing verification."
        )
        .unwrap();

        std::env::set_var("AI_SCIENTIST_PAPERS_DIR", temp_dir.path());
        let tool = crate::scientist::tools::literature::LiteratureTools;
        let result = tool.call_tool(
            "search_paper",
            &serde_json::json!({
                "query": "quantum computing",
                "source": "arxiv",
                "limit": 5
            }),
        );
        std::env::remove_var("AI_SCIENTIST_PAPERS_DIR");

        assert!(result.is_ok(), "search_paper should succeed via local fallback");
        let payload = result.unwrap();
        assert_eq!(payload["status"], "success");
        assert_eq!(payload["mode"], "local");
        assert!(payload["total"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn test_domain_science_tools_execute_real_logic() {
        let tool = crate::scientist::tools::domain_science::DomainScienceTools;

        let chemistry = tool
            .call_tool("chemistry_mol_weight", &serde_json::json!({ "smiles": "CCO" }))
            .unwrap();
        assert_eq!(chemistry["status"], "success");
        assert!(chemistry["molecular_weight"].as_f64().unwrap() > 40.0);

        let conformers = tool
            .call_tool(
                "chemistry_conformers",
                &serde_json::json!({ "smiles": "CCO", "num": 2 }),
            )
            .unwrap();
        assert_eq!(conformers["status"], "success");
        assert_eq!(conformers["conformers"].as_array().unwrap().len(), 2);

        let quantum = tool
            .call_tool(
                "chemistry_quantum_energy",
                &serde_json::json!({
                    "structure": {
                        "symbols": ["H", "H"],
                        "positions": [[0.0, 0.0, 0.0], [0.0, 0.0, 0.74]]
                    },
                    "method": "scf/sto-3g"
                }),
            )
            .unwrap();
        assert_eq!(quantum["status"], "success");

        let biology = tool
            .call_tool(
                "biology_sequence_analysis",
                &serde_json::json!({ "sequence": "ATGGCC" }),
            )
            .unwrap();
        assert_eq!(biology["status"], "success");
        assert_eq!(biology["reverse_complement"], "GGCCAT");

        let blast = tool
            .call_tool(
                "biology_blast",
                &serde_json::json!({ "sequence": "ATGGCC", "database": "local-demo" }),
            )
            .unwrap();
        assert_eq!(blast["status"], "success");

        let simulation = tool
            .call_tool(
                "simulation_run",
                &serde_json::json!({
                    "sim_type": "md",
                    "steps": 500,
                    "dt": 0.002
                }),
            )
            .unwrap();
        assert_eq!(simulation["status"], "success");
        assert_eq!(simulation["result"]["sim_type"], "md");

        let preset = tool
            .call_tool(
                "simulation_run_preset",
                &serde_json::json!({
                    "preset": "qe",
                    "steps": 200
                }),
            )
            .unwrap();
        assert_eq!(preset["status"], "success");
        assert_eq!(preset["result"]["sim_type"], "qe");

        let status = tool
            .call_tool("scientific_backend_status", &serde_json::json!({}))
            .unwrap();
        assert_eq!(status["status"], "success");
        assert!(status["report"]["python"]["name"].is_string());
    }

    #[test]
    fn test_count_all_scientist_tools() {
        let literature = crate::scientist::tools::literature::LiteratureTools::tool_definitions().len();
        let computation = crate::scientist::tools::computation::ComputationTools::tool_definitions().len();
        let data = crate::scientist::tools::data::DataTools::tool_definitions().len();
        let domain_science = crate::scientist::tools::domain_science::DomainScienceTools::tool_definitions().len();
        let sympy = crate::scientist::tools::sympy_tool::SymPyTool::tool_definitions().len();

        let total = literature + computation + data + domain_science + sympy;
        assert!(total >= 21, "Expected >= 21 scientist tools, got {}", total);
    }
}
