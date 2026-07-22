//! Runtime tool governance: execution boundaries, examples, efficiency hints and concurrency.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolConcurrency {
    ReadOnly,
    Exclusive,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolBoundary {
    pub tool: String,
    pub concurrency: ToolConcurrency,
    pub boundary: String,
    pub example: String,
    pub efficiency: String,
}

pub fn concurrency(tool: &str) -> ToolConcurrency {
    if is_read_only(tool) {
        ToolConcurrency::ReadOnly
    } else {
        ToolConcurrency::Exclusive
    }
}

pub fn is_read_only(tool: &str) -> bool {
    matches!(
        tool,
        "workspace_overview"
            | "gather_context"
            | "inspect_path"
            | "list_dir"
            | "find_files"
            | "count_file_types"
            | "find_large_files"
            | "tree_dir"
            | "get_file_info"
            | "read_file"
            | "read_file_head"
            | "read_file_range"
            | "grep"
            | "search_content"
            | "search_files"
            | "search_workspace_text"
            | "search_workspace_index"
            | "search_knowledge_base"
            | "symbol_search"
            | "document_symbols"
            | "workspace_symbols"
            | "references_search"
            | "diagnostics"
            | "diagnostic_summary"
            | "file_complexity"
            | "import_map"
            | "api_surface"
            | "project_dependency_graph"
            | "git_status"
            | "git_log"
            | "git_diff"
            | "git_diff_file"
            | "git_branch"
            | "git_remote"
            | "research_os_snapshot"
            | "research_domain_context"
            | "research_domain_workspace"
            | "remote_ssh_context"
    )
}

/// Tools in this subset do not acquire the fallback CLI assistant lock and do
/// not change workspace or external state, so a single model response may run
/// them concurrently. Other read-only tools remain serial by default.
pub fn parallel_safe_readonly(tool: &str) -> bool {
    matches!(
        tool,
        "workspace_overview"
            | "search_workspace_index"
            | "search_knowledge_base"
            | "research_os_snapshot"
            | "research_domain_context"
            | "research_domain_workspace"
    )
}

pub fn boundary(tool: &str) -> ToolBoundary {
    let (boundary, example, efficiency) = match tool {
        "workspace_overview" => (
            "Read-only and bounded to the selected workspace; excludes build, dependency and VCS internals.",
            r#"{"max_depth":3,"max_files":300,"preview_files":6}"#,
            "Use once before individual file reads; do not combine with shell directory enumeration.",
        ),
        "search_knowledge_base" => (
            "Read-only; returns active/stale workspace knowledge only and excludes archived/expired versions.",
            r#"{"query":"refund policy for US customers","limit":6}"#,
            "Use for evidence retrieval; do not read every uploaded source file.",
        ),
        "search_workspace_index" => (
            "Read-only and workspace-scoped; snippets are size-bounded.",
            r#"{"query":"StreamSessionRuntime cancellation","limit":10,"kind":"code"}"#,
            "Use before recursive scanning or repeated grep calls.",
        ),
        "terminal_run" | "terminal_run_structured" => (
            "Workspace-scoped process execution. Destructive commands, privilege escalation, broad deletion and secret access require refusal or manual approval.",
            r#"{"command":"cargo test knowledge_base --lib","timeout_secs":120}"#,
            "Run a focused command with a timeout and bounded output; avoid full environment dumps.",
        ),
        "delete_file" | "delete_dir" | "remove_file" => (
            "Only explicit workspace-relative targets are allowed. Never accept a workspace root, home directory, unresolved variable or broad glob.",
            r#"{"path":"tmp/generated-report.json"}"#,
            "Prefer archive/trash semantics and verify the resolved target first.",
        ),
        "remote_ssh_execute" => (
            "Requires a connected host with explicit per-connection Agent authorization. Never receives passwords or bypasses host policy.",
            r#"{"host_id":"gpu-lab","operation":"gpu","command":"nvidia-smi --query-gpu=name,memory.used --format=csv"}"#,
            "Reuse an authorized connection and request bounded output; use background jobs for long training.",
        ),
        "atlas_workspace_snapshot" => (
            "Create/list/diff are reversible; restore changes workspace state and must remain approval-gated.",
            r#"{"operation":"create","name":"before refactor","snapshot_type":"before_agent"}"#,
            "Create one snapshot before a high-impact phase, not before every small edit.",
        ),
        "browser_computer" => (
            "Interactive browser actions only; external submissions, purchases, publishing and credential entry require explicit user intent.",
            r#"{"action":"screenshot"}"#,
            "Prefer APIs/search tools for semantic retrieval; use browser control for visible interactive state.",
        ),
        _ if is_read_only(tool) => (
            "Read-only and bounded to the selected workspace or explicitly named remote resource.",
            r#"{"query":"target symbol","limit":10}"#,
            "Use the narrowest query and reuse prior results.",
        ),
        _ => (
            "Mutating or externally observable action. Keep targets explicit, validate scope, and require approval above the configured risk boundary.",
            r#"{"path":"src/module.rs"}"#,
            "Batch related changes when safe, but never parallelize conflicting mutations.",
        ),
    };
    ToolBoundary {
        tool: tool.to_string(),
        concurrency: concurrency(tool),
        boundary: boundary.to_string(),
        example: example.to_string(),
        efficiency: efficiency.to_string(),
    }
}

pub fn enrich_definitions(definitions: &mut [Value]) {
    for definition in definitions {
        let Some(function) = definition.get_mut("function").and_then(Value::as_object_mut) else {
            continue;
        };
        let name = function
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let governance = boundary(&name);
        let existing = function
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default();
        function.insert(
            "description".to_string(),
            Value::String(format!(
                "{}\nBoundary: {}\nEfficiency: {}\nExample arguments: {}",
                existing, governance.boundary, governance.efficiency, governance.example
            )),
        );
        function.insert(
            "x-atlas-governance".to_string(),
            serde_json::to_value(governance).unwrap_or(Value::Null),
        );
    }
}

pub fn validate_arguments(tool: &str, args: &Value) -> Result<(), String> {
    if matches!(tool, "delete_file" | "delete_dir" | "remove_file") {
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
        let unsafe_target = path.is_empty()
            || matches!(path, "." | "./" | "/" | "\\" | "~")
            || path.contains('*')
            || path.contains('$')
            || path.contains('%')
            || path.split(['/', '\\']).any(|part| part == "..");
        if unsafe_target {
            return Err("destructive tool requires one explicit workspace-relative file or directory target".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn destructive_targets_are_explicit() {
        assert!(validate_arguments("delete_dir", &json!({"path":"."})).is_err());
        assert!(validate_arguments("delete_file", &json!({"path":"tmp/a.txt"})).is_ok());
    }

    #[test]
    fn tool_definitions_receive_boundary_and_example() {
        let mut tools = vec![json!({"type":"function","function":{"name":"workspace_overview","description":"Overview","parameters":{}}})];
        enrich_definitions(&mut tools);
        let description = tools[0]["function"]["description"].as_str().unwrap();
        assert!(description.contains("Boundary:"));
        assert!(description.contains("Example arguments:"));
    }
}
