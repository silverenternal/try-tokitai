use ai_assistant::scientist::tools::verification_center::VerificationCenterTools;
use serde_json::Value;

#[test]
fn verification_center_exposes_installation_status() {
    let center = VerificationCenterTools;
    let report = center
        .call_tool("verification_center_status", &serde_json::json!({}))
        .expect("status report");
    let value: Value = report;

    assert_eq!(value["status"], "ready");
    assert!(value["summary"]["total_tools"].as_u64().unwrap_or(0) >= 1);
    assert!(value["summary"]["total_platforms"].as_u64().unwrap_or(0) >= 1);
    assert!(value["available_bundles"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));
}

#[test]
fn verification_center_can_fetch_remote_papers() {
    let center = VerificationCenterTools;
    let report = center
        .call_tool(
            "verification_center_fetch_papers",
            &serde_json::json!({
                "query": "computer vision",
                "limit": 3
            }),
        )
        .expect("paper report");
    let value: Value = report;

    assert_eq!(
        value["paper_ids"].as_array().map(|a| a.len()).unwrap_or(0),
        3
    );
    assert!(value["search"]["status"]
        .as_str()
        .unwrap_or("")
        .contains("success"));
}

#[test]
fn verification_center_run_returns_profile_aware_bundle_runs() {
    let center = VerificationCenterTools;
    let report = center
        .call_tool(
            "verification_center_run",
            &serde_json::json!({
                "workspace_root": "D:\\try-tokitai",
                "target_profile": "security_analysis"
            }),
        )
        .expect("verification_center_run should succeed");
    let value: Value = report;

    assert!(
        value["verification_center"]["summary"]["total_tools"]
            .as_u64()
            .unwrap_or(0)
            >= 1
    );
    let bundle_runs = value["bundle_runs"].as_array().expect("bundle_runs array");
    assert!(!bundle_runs.is_empty());
    assert!(bundle_runs
        .iter()
        .any(|bundle| bundle["bundle_id"] == "workspace_hygiene"));
    assert!(bundle_runs
        .iter()
        .any(|bundle| bundle["bundle_id"] == "security_scan"));
}

#[test]
fn verification_center_report_exposes_bundle_runs() {
    let center = VerificationCenterTools;
    let run_report = center
        .call_tool(
            "verification_center_run",
            &serde_json::json!({
                "workspace_root": "D:\\try-tokitai",
                "target_profile": "theory"
            }),
        )
        .expect("verification_center_run should succeed");
    let summarized = center
        .call_tool(
            "verification_center_report",
            &serde_json::json!({
                "report": run_report
            }),
        )
        .expect("verification_center_report should succeed");
    let value: Value = summarized;

    assert!(value["bundle_runs"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));
    assert!(value["score"].as_u64().unwrap_or(0) <= 100);
}
