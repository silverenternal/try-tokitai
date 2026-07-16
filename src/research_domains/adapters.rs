use super::model::{DomainAsset, DomainPluginDescriptor, DomainVisualizationDescriptor};
use crate::process_window::CommandWindowExt;
use crate::visualization::model::{
    VisualizationDiagnostic, VisualizationDocument, VisualizationEdge, VisualizationEvent,
    VisualizationFrame, VisualizationNode, VisualizationPoint, VisualizationSeries,
    VisualizationSource,
};
use crate::visualization::{
    AlgorithmAdapter, NetworkAdapter, VisualizationAdapter, VisualizationContext,
};
use anyhow::{anyhow, Result};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

const MAX_ADAPTER_TEXT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_DOMAIN_NODES: usize = 2_000;
const MAX_DOMAIN_RECORDS: usize = 10_000;

/// Detects declared SDKs without attempting to execute them. The result is used by
/// the workbench to disable SDK-backed actions when no real executable is present.
#[derive(Debug, Clone, serde::Serialize)]
pub(super) struct SdkProbe {
    pub sdk: String,
    pub status: String,
    pub available: bool,
    pub executable: Option<String>,
    pub version: Option<String>,
    pub candidates: Vec<String>,
    pub execution_enabled: bool,
    pub reason: String,
}

static SDK_PROBE_CACHE: OnceLock<Mutex<HashMap<String, SdkProbe>>> = OnceLock::new();

pub(super) fn sdk_probe(sdk: &str) -> SdkProbe {
    let key = sdk.trim().to_ascii_lowercase();
    let cache = SDK_PROBE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(probe) = cache
        .lock()
        .ok()
        .and_then(|values| values.get(&key).cloned())
    {
        return probe;
    }
    let candidates = sdk_executables(sdk);
    let python_requirement = python_sdk_requirement(sdk);
    let detected = candidates
        .iter()
        .find_map(|candidate| which::which(candidate).ok());
    let (available, executable, version, reason) = if let Some((module, distribution)) =
        python_requirement
    {
        match detected.as_ref() {
            Some(python) => match probe_python_module(python, module, distribution) {
                Some(version) => (
                    true,
                    Some(python.to_string_lossy().to_string()),
                    version,
                    format!("Python module '{module}' is importable"),
                ),
                None => (
                    false,
                    Some(python.to_string_lossy().to_string()),
                    None,
                    format!("Python is present but required module '{module}' is not installed"),
                ),
            },
            None => (
                false,
                None,
                None,
                "Python executable was not detected".to_string(),
            ),
        }
    } else if let Some(program) = detected {
        let version = probe_program_version(&program);
        (
            true,
            Some(program.to_string_lossy().to_string()),
            version,
            "Executable detected on PATH".to_string(),
        )
    } else {
        (
            false,
            None,
            None,
            format!(
                "No supported executable detected ({})",
                candidates.join(", ")
            ),
        )
    };
    let probe = SdkProbe {
        sdk: sdk.to_string(),
        status: if available {
            "available"
        } else {
            "unavailable"
        }
        .to_string(),
        available,
        executable,
        version,
        candidates,
        execution_enabled: available,
        reason,
    };
    if let Ok(mut values) = cache.lock() {
        values.insert(key, probe.clone());
    }
    probe
}

pub(super) fn sdk_statuses(sdks: &[String]) -> Value {
    Value::Array(sdks.iter().map(|sdk| json!(sdk_probe(sdk))).collect())
}

fn python_sdk_requirement(sdk: &str) -> Option<(&'static str, &'static str)> {
    let normalized = sdk.trim().to_ascii_lowercase();
    if normalized.contains("pytorch vision") {
        Some(("torchvision", "torchvision"))
    } else if normalized.contains("pytorch") {
        Some(("torch", "torch"))
    } else if normalized.contains("tensorflow") {
        Some(("tensorflow", "tensorflow"))
    } else if normalized == "jax" {
        Some(("jax", "jax"))
    } else if normalized.contains("onnx") {
        Some(("onnxruntime", "onnxruntime"))
    } else if normalized.contains("hugging face") {
        Some(("transformers", "transformers"))
    } else if normalized.contains("sentence") {
        Some(("sentence_transformers", "sentence-transformers"))
    } else if normalized.contains("spacy") {
        Some(("spacy", "spacy"))
    } else if normalized.contains("nltk") {
        Some(("nltk", "nltk"))
    } else if normalized.contains("opencv") {
        Some(("cv2", "opencv-python"))
    } else if normalized.contains("open3d") {
        Some(("open3d", "open3d"))
    } else if normalized.contains("cadquery") {
        Some(("cadquery", "cadquery"))
    } else if normalized.contains("mujoco") {
        Some(("mujoco", "mujoco"))
    } else if normalized.contains("scapy") {
        Some(("scapy", "scapy"))
    } else if normalized.contains("numpy") {
        Some(("numpy", "numpy"))
    } else if normalized.contains("scipy") {
        Some(("scipy", "scipy"))
    } else if normalized.contains("arrow") {
        Some(("pyarrow", "pyarrow"))
    } else if normalized.contains("datafusion") {
        Some(("datafusion", "datafusion"))
    } else if normalized == "vtk" {
        Some(("vtk", "vtk"))
    } else {
        None
    }
}

fn probe_python_module(python: &Path, module: &str, distribution: &str) -> Option<Option<String>> {
    let script = "import importlib.util,importlib.metadata,sys\nmodule,dist=sys.argv[1:3]\nif importlib.util.find_spec(module) is None: raise SystemExit(3)\ntry: print(importlib.metadata.version(dist))\nexcept Exception: print('')";
    let mut command = Command::new(python);
    command
        .args(["-I", "-c", script, module, distribution])
        .hide_window();
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some((!version.is_empty()).then_some(version))
}

pub(super) fn probe_program_version(program: &Path) -> Option<String> {
    for argument in ["--version", "-version", "-v"] {
        let mut command = Command::new(program);
        command.arg(argument).hide_window();
        let Ok(output) = command.output() else {
            continue;
        };
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if let Some(line) = text.lines().map(str::trim).find(|line| !line.is_empty()) {
            return Some(line.chars().take(160).collect());
        }
    }
    None
}

/// Capability gating is deliberately evidence based. A tab is exposed only when
/// its semantics can be derived from the selected artifact; file extension alone
/// is not enough for optional structures such as CAD feature trees or annotations.
pub(super) fn supports_visualization(
    domain_id: &str,
    visualization: &DomainVisualizationDescriptor,
    path: &Path,
    file_type: &str,
    raw_preview: Option<&str>,
) -> bool {
    let id = visualization.id.as_str();
    let lower = raw_preview.map(str::to_ascii_lowercase);
    let text = lower.as_deref().unwrap_or_default();

    match (domain_id, id) {
        ("cad", "feature-tree") => match file_type {
            "fcstd" => fcstd_document_xml(path)
                .map(|raw| {
                    contains_any_ci(&raw, &["<Object", "Property name=", "PartDesign::Feature"])
                })
                .unwrap_or(false),
            "scad" => contains_any_ci(text, &["module ", "difference(", "union(", "intersection("]),
            _ => false,
        },
        ("cad", "constraint-graph") => match file_type {
            "fcstd" => fcstd_document_xml(path)
                .map(|raw| contains_any_ci(&raw, &["Constraint", "Sketcher::SketchObject"]))
                .unwrap_or(false),
            "scad" => contains_any_ci(text, &["module ", "=", "translate(", "rotate(", "scale("]),
            "dxf" => contains_any_ci(text, &["DIMENSION", "CONSTRAINT", "ACAD_CONSTRAINT"]),
            _ => false,
        },
        ("cad", "exploded-view") => match file_type {
            "fcstd" => fcstd_document_xml(path)
                .map(|raw| count_occurrences_ci(&raw, "<Object") > 1)
                .unwrap_or(false),
            "step" | "stp" => contains_any_ci(
                text,
                &[
                    "NEXT_ASSEMBLY_USAGE_OCCURRENCE",
                    "CONTEXT_DEPENDENT_SHAPE_REPRESENTATION",
                ],
            ),
            _ => false,
        },
        ("computer-vision", "image-overlay") => {
            is_image_type(file_type)
                || contains_any_ci(text, &["bounding_box", "bbox", "detections", "annotations"])
        }
        ("computer-vision", "segmentation") => {
            contains_any_ci(
                text,
                &["segmentation", "mask", "polygon", "class_map", "label_map"],
            ) || matches!(file_type, "npy" | "npz")
        }
        ("computer-vision", "feature-map") => {
            matches!(file_type, "npy" | "npz")
                || contains_any_ci(text, &["feature_map", "activations", "channels"])
        }
        ("nlp", "dependency-tree") => {
            matches!(file_type, "conll" | "conllu")
                || contains_any_ci(text, &["\"head\"", "\"deprel\"", "dependencies"])
        }
        ("nlp", "attention-flow") => {
            matches!(file_type, "npy" | "npz")
                || contains_any_ci(text, &["attention", "attention_weights"])
        }
        ("compiler", "syntax-tree") => {
            matches!(file_type, "ast")
                || contains_any_ci(text, &["\"ast\"", "translationunitdecl", "syntax tree"])
                || is_source_type(file_type)
        }
        ("compiler", "control-flow") => {
            matches!(file_type, "cfg" | "dot" | "graphml")
                || contains_any_ci(text, &["br label", " cf.br ", "basicblock", "digraph"])
        }
        ("compiler", "ssa") => {
            matches!(file_type, "ll" | "mlir" | "cfg" | "dot")
                && contains_any_ci(text, &[" phi ", "phi ", "%", "block argument"])
        }
        ("compiler", "optimization") => {
            matches!(file_type, "bc")
                || contains_any_ci(
                    text,
                    &["pass", "optimization", "before", "after", "remarks"],
                )
        }
        ("database", "schema") => {
            matches!(file_type, "sql" | "json")
                && contains_any_ci(
                    text,
                    &["create table", "create view", "create index", "\"tables\""],
                )
        }
        ("database", "query-plan") => {
            matches!(file_type, "plan")
                || contains_any_ci(
                    text,
                    &[
                        "query plan",
                        "seq scan",
                        "index scan",
                        "table scan",
                        "explain",
                    ],
                )
        }
        ("database", "lineage") => contains_any_ci(
            text,
            &[
                "insert into",
                "create table",
                "create view",
                "select",
                " from ",
                " join ",
            ],
        ),
        ("database", "table") => matches!(file_type, "csv" | "json" | "jsonl"),
        ("program-analysis", "call-graph") => {
            matches!(file_type, "dot" | "graphml" | "gexf")
                || contains_any_ci(text, &["calls", "callgraph", " call ", "invoke "])
        }
        ("program-analysis", "data-flow") => {
            matches!(file_type, "dfg")
                || contains_any_ci(
                    text,
                    &["dataflow", "data flow", "def-use", "reaching definitions"],
                )
        }
        ("program-analysis", "taint-flow") | ("cyber-security", "taint-flow") => contains_any_ci(
            text,
            &["taint", "source", "sink", "codeflows", "threadflows"],
        ),
        ("cyber-security", "attack-graph") => contains_any_ci(
            text,
            &[
                "attack",
                "vulnerability",
                "cve-",
                "codeflows",
                "relatedlocations",
            ],
        ),
        ("cyber-security", "findings") => {
            matches!(file_type, "sarif" | "nessus")
                || contains_any_ci(text, &["results", "findings", "vulnerability", "severity"])
        }
        ("robotics", "robot-model") | ("robotics", "coordinate-frame") => {
            matches!(file_type, "urdf" | "xacro" | "sdf")
                && contains_any_ci(text, &["<robot", "<model", "<link", "<joint"])
        }
        ("robotics", "trajectory") | ("robotics", "joint-state") => {
            matches!(file_type, "csv" | "json")
                && contains_any_ci(
                    text,
                    &["joint", "position", "trajectory", "timestamp", "time"],
                )
        }
        ("computer-networks", "tcp-state") => {
            matches!(file_type, "pcap" | "pcapng")
                || contains_any_ci(text, &["tcp", "syn", "fin", "established"])
        }
        ("computer-networks", "bandwidth") => {
            matches!(file_type, "pcap" | "pcapng")
                || contains_any_ci(text, &["bytes", "bandwidth", "throughput"])
        }
        ("computer-networks", "latency") => {
            matches!(file_type, "pcap" | "pcapng" | "har")
                || contains_any_ci(text, &["latency", "duration", "elapsed", "time_ms"])
        }
        ("operating-systems", "process-tree") => {
            contains_any_ci(text, &["pid", "ppid", "process", "parent_pid", "processid"])
        }
        ("operating-systems", "thread-timeline") => contains_any_ci(text, &["thread", "tid"]),
        ("operating-systems", "cpu-timeline") => contains_any_ci(
            text,
            &["cpu", "scheduler", "sched_switch", "duration", "timestamp"],
        ),
        ("operating-systems", "memory-layout") => {
            matches!(file_type, "dmp" | "core")
                || contains_any_ci(text, &["address", "memory", "region", "heap", "stack"])
        }
        ("operating-systems", "file-system-tree") => {
            contains_any_ci(text, &["openat(", "stat(", "read(", "write(", "path"])
        }
        ("hpc", "gpu-timeline") => {
            matches!(file_type, "trace" | "csv" | "jsonl")
                && contains_any_ci(text, &["kernel", "gpu", "duration", "timestamp"])
        }
        ("hpc", "mpi-communication") => {
            matches!(file_type, "trace" | "json" | "jsonl")
                && contains_any_ci(
                    text,
                    &["mpi_send", "mpi_recv", "rank", "source", "destination"],
                )
        }
        ("hpc", "memory-bandwidth") => contains_any_ci(text, &["bandwidth", "bytes", "dram"]),
        ("hpc", "occupancy") => contains_any_ci(text, &["occupancy", "warps", "blocks", "sm_"]),
        ("distributed-systems", "request-trace") => {
            matches!(file_type, "har" | "trace")
                || contains_any_ci(
                    text,
                    &["trace_id", "traceid", "span_id", "spanid", "duration"],
                )
        }
        ("distributed-systems", "state-lifecycle") => contains_any_ci(
            text,
            &["state", "status", "term", "leader", "replica", "transition"],
        ),
        ("scientific-computing", "equation") => contains_equation(text, file_type),
        ("scientific-computing", "field") => matches!(
            file_type,
            "vtk" | "vtu" | "npy" | "npz" | "mat" | "h5" | "hdf5" | "nc"
        ),
        ("scientific-computing", "mesh") => matches!(file_type, "vtk" | "vtu"),
        _ => true,
    }
}

/// Uses mature shared adapters where their input contract already matches a
/// research domain, then handles domain-native formats locally.
pub(super) fn parse_registered_adapter(
    workspace_root: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let path = checked_asset_path(workspace_root, asset)?;
    let extension = asset.file_type.as_str();

    if asset
        .metadata
        .get("action_result")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return parse_action_result(&path, descriptor, asset).map(Some);
    }

    if descriptor.metadata.id == "ai-ml"
        && matches!(
            extension,
            "py" | "rs"
                | "js"
                | "ts"
                | "tsx"
                | "jsx"
                | "java"
                | "cpp"
                | "c"
                | "go"
                | "json"
                | "csv"
                | "jsonl"
                | "onnx"
                | "safetensors"
        )
    {
        return parse_shared_adapter(AlgorithmAdapter, workspace_root, asset).map(Some);
    }
    if descriptor.metadata.id == "computer-networks"
        && matches!(
            extension,
            "pcap" | "pcapng" | "har" | "log" | "json" | "jsonl" | "csv" | "txt"
        )
    {
        return parse_shared_adapter(NetworkAdapter, workspace_root, asset).map(Some);
    }

    let document = match descriptor.metadata.id.as_str() {
        "computer-vision" => parse_computer_vision(&path, descriptor, asset)?,
        "nlp" => parse_nlp(&path, descriptor, asset)?,
        "computer-graphics" | "cad" => parse_geometry_domain(&path, descriptor, asset)?,
        "robotics" => parse_robotics(&path, descriptor, asset)?,
        "operating-systems" => parse_trace_domain(&path, descriptor, asset, "operating-system")?,
        "compiler" => parse_compiler(&path, descriptor, asset)?,
        "database" => parse_database(&path, descriptor, asset)?,
        "software-engineering" => parse_software(&path, descriptor, asset)?,
        "program-analysis" => parse_program_analysis(&path, descriptor, asset)?,
        "cyber-security" => parse_security(&path, descriptor, asset)?,
        "hpc" => parse_trace_domain(&path, descriptor, asset, "parallel-computing")?,
        "distributed-systems" => parse_distributed(&path, descriptor, asset)?,
        "scientific-computing" => parse_scientific(&path, descriptor, asset)?,
        _ => None,
    };
    Ok(document)
}

fn parse_action_result(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<VisualizationDocument> {
    let raw = read_text(path)?;
    let value = serde_json::from_str::<Value>(&raw)
        .map_err(|error| anyhow!("invalid domain action result: {error}"))?;
    if value.get("schema_version").and_then(Value::as_str) != Some("atlas.domain-action-result.v1")
    {
        return Err(anyhow!("unsupported domain action result schema"));
    }

    let mut document = empty_document(descriptor, asset);
    let action_id = value
        .get("action")
        .or_else(|| value.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("domain-action");
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("completed");
    let mut root = VisualizationNode::new("action-result", action_id, "action-result");
    root.status = status.to_string();
    for key in [
        "task_id", "asset", "title", "program", "command", "log_path",
    ] {
        if let Some(field) = value.get(key) {
            root.metadata.insert(key.to_string(), field.clone());
        }
    }
    if let Some(exit_code) = value.get("exit_code").and_then(Value::as_f64) {
        root.metrics.insert("exit_code".to_string(), exit_code);
    }
    document.nodes.push(root);

    let result = value.get("result").unwrap_or(&value);
    let mut table_rows = Vec::<Value>::new();
    append_action_result_value(
        result,
        "result",
        "action-result",
        &mut document,
        &mut table_rows,
        0,
    );
    for key in [
        "status",
        "exit_code",
        "started_at",
        "completed_at",
        "output_truncated",
    ] {
        if let Some(field) = value.get(key) {
            table_rows.push(json!({"Field": key, "Value": display_json_value(field)}));
        }
    }
    if let Some(output) = value.get("output").and_then(Value::as_str) {
        document
            .metadata
            .insert("action_output".to_string(), json!(output));
        table_rows.push(json!({
            "Field": "output",
            "Value": output.chars().take(4_000).collect::<String>()
        }));
    }
    document.metadata.insert(
        "table".to_string(),
        json!({"columns":["Field","Value"],"rows":table_rows}),
    );
    document.metadata.insert(
        "domain_action".to_string(),
        json!({
            "id": action_id,
            "status": status,
            "task_id": value.get("task_id"),
            "sdk": value.get("result").and_then(|item| item.get("sdk")),
            "synthetic_data": false
        }),
    );
    Ok(document)
}

fn append_action_result_value(
    value: &Value,
    label: &str,
    parent: &str,
    document: &mut VisualizationDocument,
    table_rows: &mut Vec<Value>,
    depth: usize,
) {
    if document.nodes.len() >= MAX_DOMAIN_NODES || depth > 5 {
        return;
    }
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if document.nodes.len() >= MAX_DOMAIN_NODES {
                    break;
                }
                if child.is_array() || child.is_object() {
                    let id = format!("action:{}:{}", depth, stable_value_id(parent, key));
                    let mut node = VisualizationNode::new(&id, key, "action-result-group");
                    node.parent_id = Some(parent.to_string());
                    document.nodes.push(node);
                    document.edges.push(VisualizationEdge::new(
                        format!("action-edge:{}", stable_value_id(parent, key)),
                        parent,
                        &id,
                        "contains",
                        "action-result",
                    ));
                    append_action_result_value(child, key, &id, document, table_rows, depth + 1);
                } else {
                    table_rows.push(json!({
                        "Field": if label == "result" { key.clone() } else { format!("{label}.{key}") },
                        "Value": display_json_value(child)
                    }));
                    if let Some(number) = child.as_f64() {
                        if let Some(node) = document.nodes.iter_mut().find(|node| node.id == parent)
                        {
                            node.metrics.insert(key.clone(), number);
                        }
                    } else if let Some(node) =
                        document.nodes.iter_mut().find(|node| node.id == parent)
                    {
                        node.metadata.insert(key.clone(), child.clone());
                    }
                }
            }
        }
        Value::Array(values) => {
            for (index, child) in values
                .iter()
                .take(MAX_DOMAIN_NODES.saturating_sub(document.nodes.len()))
                .enumerate()
            {
                let child_label = child
                    .get("name")
                    .or_else(|| child.get("text"))
                    .or_else(|| child.get("id"))
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| format!("{label} {index}"));
                let id = format!(
                    "action-item:{}:{}",
                    depth,
                    stable_value_id(parent, &index.to_string())
                );
                let mut node = VisualizationNode::new(&id, child_label, label);
                node.parent_id = Some(parent.to_string());
                if let Some(object) = child.as_object() {
                    for (key, field) in object {
                        if let Some(number) = field.as_f64() {
                            node.metrics.insert(key.clone(), number);
                        } else if !field.is_array() && !field.is_object() {
                            node.metadata.insert(key.clone(), field.clone());
                        }
                    }
                } else {
                    node.metadata.insert("value".to_string(), child.clone());
                }
                document.nodes.push(node);
                document.edges.push(VisualizationEdge::new(
                    format!(
                        "action-edge:{}",
                        stable_value_id(parent, &index.to_string())
                    ),
                    parent,
                    &id,
                    "contains",
                    "action-result",
                ));
                if child.is_array() || child.is_object() {
                    append_action_result_value(child, label, &id, document, table_rows, depth + 1);
                }
            }
        }
        _ => table_rows.push(json!({"Field": label, "Value": display_json_value(value)})),
    }
}

fn stable_value_id(parent: &str, key: &str) -> String {
    blake3::hash(format!("{parent}:{key}").as_bytes()).to_hex()[..12].to_string()
}

fn display_json_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".to_string(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

/// Adds a stable, renderer-independent workbench payload and prunes generic
/// structures that do not belong to the selected professional view.
pub(super) fn adapt_document(
    workspace_root: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
    visualization: &DomainVisualizationDescriptor,
    document: &mut VisualizationDocument,
) -> Result<()> {
    let path = checked_asset_path(workspace_root, asset)?;
    let sdk_status = sdk_statuses(&descriptor.sdk_adapters);
    let required_sdk = if visualization.requires_sdk.is_empty() {
        Value::Null
    } else {
        sdk_statuses(&visualization.requires_sdk)
    };
    document.metadata.insert(
        "workbench".to_string(),
        json!({
            "layout": descriptor.workbench.layout,
            "explorer_label": descriptor.workbench.explorer_label,
            "primary_label": descriptor.workbench.primary_label,
            "inspector_label": descriptor.workbench.inspector_label,
            "bottom_panel_label": descriptor.workbench.bottom_panel_label,
            "view_id": visualization.id,
            "renderer": visualization.renderer,
            "region": visualization.workbench_region,
            "adapter": visualization.adapter,
            "source_file_type": asset.file_type,
            "source_path": asset.path,
            "sdk_status": sdk_status,
            "required_sdk_status": required_sdk,
            "data_provenance": "workspace-artifact",
            "synthetic_data": false,
        }),
    );

    match visualization.id.as_str() {
        "loss-curve" | "bandwidth" | "latency" | "memory-bandwidth" | "result-chart" => {
            document.series.retain(|series| !series.points.is_empty());
        }
        "feature-tree" => retain_categories(
            document,
            &["feature", "body", "part", "operation", "parameter"],
        ),
        "constraint-graph" => {
            retain_categories(document, &["constraint", "parameter", "feature", "sketch"])
        }
        "dependency-tree" => retain_categories(document, &["token", "sentence", "dependency"]),
        "findings" => retain_categories(document, &["finding", "rule", "location", "evidence"]),
        "schema" => retain_categories(
            document,
            &["database", "schema", "table", "view", "column", "index"],
        ),
        "query-plan" => retain_categories(document, &["query-plan", "plan-operator"]),
        "syntax-tree" => retain_categories(document, &["ast", "syntax", "symbol", "source"]),
        "control-flow" | "ssa" => {
            retain_categories(document, &["function", "basic-block", "instruction", "cfg"])
        }
        _ => {}
    }

    if document.nodes.is_empty()
        && document.series.is_empty()
        && document.metadata.get("geometry").is_none()
        && document.metadata.get("scene").is_none()
        && document.metadata.get("table").is_none()
    {
        document.diagnostics.push(VisualizationDiagnostic {
            level: "info".to_string(),
            message: format!(
                "No '{}' structure could be derived from {}.",
                visualization.label, asset.path
            ),
            metadata: BTreeMap::from([
                ("visualization_id".to_string(), json!(visualization.id)),
                ("source_path".to_string(), json!(asset.path)),
            ]),
        });
    }

    document.metadata.insert(
        "source_capabilities".to_string(),
        json!(derive_source_capabilities(&path, descriptor, asset)),
    );
    Ok(())
}

fn parse_shared_adapter<A: VisualizationAdapter>(
    adapter: A,
    workspace_root: &Path,
    asset: &DomainAsset,
) -> Result<VisualizationDocument> {
    let runtime = json!({});
    let source_id = format!("workspace:{}", asset.path);
    adapter.parse(&VisualizationContext {
        workspace_root,
        source_id: Some(&source_id),
        runtime: &runtime,
    })
}

fn empty_document(
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> VisualizationDocument {
    let source = VisualizationSource {
        id: asset.id.clone(),
        kind: "research-domain".to_string(),
        label: asset.path.clone(),
        source_type: asset.file_type.clone(),
        live: false,
        metadata: BTreeMap::from([
            ("path".to_string(), json!(asset.path)),
            ("domain_id".to_string(), json!(descriptor.metadata.id)),
        ]),
    };
    VisualizationDocument::empty(
        "research-domain",
        format!("{} · {}", descriptor.metadata.label, asset.name),
        source,
    )
}

fn checked_asset_path(workspace_root: &Path, asset: &DomainAsset) -> Result<PathBuf> {
    let root = workspace_root
        .canonicalize()
        .unwrap_or_else(|_| workspace_root.to_path_buf());
    let path = root.join(&asset.path);
    let canonical = path
        .canonicalize()
        .map_err(|error| anyhow!("domain asset is unavailable: {error}"))?;
    if !canonical.starts_with(&root) {
        return Err(anyhow!("domain asset is outside the active workspace"));
    }
    Ok(canonical)
}

fn read_text(path: &Path) -> Result<String> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > MAX_ADAPTER_TEXT_BYTES {
        return Err(anyhow!("domain adapter source exceeds 16 MiB"));
    }
    Ok(String::from_utf8_lossy(&fs::read(path)?).into_owned())
}

fn read_optional_text(path: &Path) -> Option<String> {
    read_text(path).ok()
}

fn parse_computer_vision(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let mut document = empty_document(descriptor, asset);
    if is_image_type(&asset.file_type) {
        let bytes = fs::read(path)?;
        if let Some((width, height, channels)) = image_dimensions(&bytes, &asset.file_type) {
            let mut node = VisualizationNode::new("image", asset.name.clone(), "image");
            node.metrics.insert("width".to_string(), width as f64);
            node.metrics.insert("height".to_string(), height as f64);
            node.metrics.insert("channels".to_string(), channels as f64);
            node.metadata.insert("path".to_string(), json!(asset.path));
            document.nodes.push(node);
            document.metadata.insert(
                "media".to_string(),
                json!({"kind":"image","path":asset.path,"width":width,"height":height,"channels":channels}),
            );
        }
        return Ok(Some(document));
    }
    if asset.file_type == "json" || asset.file_type == "jsonl" {
        let raw = read_text(path)?;
        parse_vision_annotations(&raw, &mut document);
        return Ok(Some(document));
    }
    Ok(None)
}

fn parse_vision_annotations(raw: &str, document: &mut VisualizationDocument) {
    let values = json_records(raw);
    for (index, value) in values.iter().take(MAX_DOMAIN_RECORDS).enumerate() {
        let object = value
            .as_object()
            .or_else(|| value.get("annotation")?.as_object());
        let Some(object) = object else { continue };
        let bbox = object.get("bbox").or_else(|| object.get("bounding_box"));
        let category = object
            .get("category")
            .or_else(|| object.get("label"))
            .or_else(|| object.get("class"))
            .and_then(value_label)
            .unwrap_or_else(|| format!("annotation {}", index + 1));
        if let Some(bbox) = bbox.and_then(Value::as_array) {
            let mut node =
                VisualizationNode::new(format!("detection:{index}"), &category, "detection");
            node.metadata
                .insert("bbox".to_string(), Value::Array(bbox.clone()));
            if let Some(score) = object.get("score").and_then(number) {
                node.metrics.insert("score".to_string(), score);
            }
            document.nodes.push(node);
        }
        if let Some(segmentation) = object.get("segmentation").or_else(|| object.get("mask")) {
            let mut node =
                VisualizationNode::new(format!("segmentation:{index}"), &category, "segmentation");
            node.metadata
                .insert("segmentation".to_string(), segmentation.clone());
            document.nodes.push(node);
        }
    }
    document
        .metadata
        .insert("annotation_count".to_string(), json!(document.nodes.len()));
}

fn parse_nlp(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let raw = match read_optional_text(path) {
        Some(raw) => raw,
        None => return Ok(None),
    };
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "conll" | "conllu" => parse_conllu(&raw, &mut document),
        "txt" | "md" => parse_plain_tokens(&raw, &mut document),
        "json" | "jsonl" => parse_nlp_json(&raw, &mut document),
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_conllu(raw: &str, document: &mut VisualizationDocument) {
    let mut sentence = 0usize;
    let mut sentence_nodes = HashMap::<usize, String>::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            sentence += 1;
            sentence_nodes.clear();
            continue;
        }
        if line.starts_with('#') {
            if let Some(text) = line.strip_prefix("# text =") {
                document
                    .metadata
                    .insert(format!("sentence_text:{sentence}"), json!(text.trim()));
            }
            continue;
        }
        let columns = line.split('\t').collect::<Vec<_>>();
        if columns.len() < 8 || columns[0].contains(['-', '.']) {
            continue;
        }
        let Ok(token_index) = columns[0].parse::<usize>() else {
            continue;
        };
        let id = format!("token:{sentence}:{token_index}");
        let mut node = VisualizationNode::new(&id, columns[1], "token");
        node.metadata.insert("lemma".to_string(), json!(columns[2]));
        node.metadata.insert("upos".to_string(), json!(columns[3]));
        node.metadata
            .insert("features".to_string(), json!(columns[5]));
        node.parent_id = Some(format!("sentence:{sentence}"));
        document.nodes.push(node);
        sentence_nodes.insert(token_index, id.clone());
        if columns[6] != "0" {
            if let Ok(head) = columns[6].parse::<usize>() {
                let target = format!("token:{sentence}:{head}");
                document.edges.push(VisualizationEdge::new(
                    format!("dependency:{sentence}:{token_index}"),
                    target,
                    id,
                    columns[7],
                    "dependency",
                ));
            }
        }
    }
    let sentence_count = sentence + usize::from(!raw.trim().is_empty());
    for index in 0..sentence_count {
        document.nodes.insert(
            index,
            VisualizationNode::new(
                format!("sentence:{index}"),
                format!("Sentence {}", index + 1),
                "sentence",
            ),
        );
    }
    document
        .metadata
        .insert("sentence_count".to_string(), json!(sentence_count));
}

fn parse_plain_tokens(raw: &str, document: &mut VisualizationDocument) {
    let token = Regex::new(r"(?u)\b[\p{L}\p{N}_'-]+\b").unwrap();
    for (index, matched) in token.find_iter(raw).take(MAX_DOMAIN_NODES).enumerate() {
        let mut node = VisualizationNode::new(format!("token:{index}"), matched.as_str(), "token");
        node.metrics
            .insert("offset".to_string(), matched.start() as f64);
        document.nodes.push(node);
        if index > 0 {
            document.edges.push(VisualizationEdge::new(
                format!("token-flow:{index}"),
                format!("token:{}", index - 1),
                format!("token:{index}"),
                "next",
                "token-flow",
            ));
        }
    }
}

fn parse_nlp_json(raw: &str, document: &mut VisualizationDocument) {
    let values = json_records(raw);
    for (sentence_index, value) in values.iter().take(500).enumerate() {
        let tokens = value
            .get("tokens")
            .and_then(Value::as_array)
            .or_else(|| value.as_array());
        let Some(tokens) = tokens else { continue };
        for (index, token) in tokens.iter().take(MAX_DOMAIN_NODES).enumerate() {
            let label = value_label(token).unwrap_or_else(|| format!("token {}", index + 1));
            let id = format!("token:{sentence_index}:{index}");
            let mut node = VisualizationNode::new(&id, label, "token");
            if let Some(object) = token.as_object() {
                node.metadata.extend(object.clone());
                if let Some(head) = object.get("head").and_then(Value::as_u64) {
                    document.edges.push(VisualizationEdge::new(
                        format!("dependency:{sentence_index}:{index}"),
                        format!("token:{sentence_index}:{head}"),
                        &id,
                        object
                            .get("deprel")
                            .and_then(Value::as_str)
                            .unwrap_or("depends"),
                        "dependency",
                    ));
                }
            }
            document.nodes.push(node);
        }
    }
}

fn parse_geometry_domain(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "obj" => parse_obj(&read_text(path)?, &mut document),
        "stl" => parse_stl(&fs::read(path)?, &mut document),
        "ply" => parse_ply(&read_text(path)?, &mut document),
        "gltf" => parse_gltf_json(&read_text(path)?, &mut document),
        "glb" => parse_glb(&fs::read(path)?, &mut document),
        "step" | "stp" | "iges" | "igs" | "brep" => {
            parse_cad_exchange(&read_text(path)?, asset.file_type.as_str(), &mut document)
        }
        "fcstd" => {
            let raw = fcstd_document_xml(path)
                .ok_or_else(|| anyhow!("FreeCAD Document.xml could not be read"))?;
            parse_freecad_document(&raw, &mut document);
        }
        "scad" => parse_openscad(&read_text(path)?, &mut document),
        "dxf" => parse_dxf(&read_text(path)?, &mut document),
        "mtl" | "vert" | "frag" | "glsl" | "hlsl" | "wgsl" => {
            parse_graphics_source(&read_text(path)?, asset.file_type.as_str(), &mut document)
        }
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_obj(raw: &str, document: &mut VisualizationDocument) {
    let mut points = Vec::<[f64; 3]>::new();
    let mut faces = Vec::<Vec<usize>>::new();
    let mut current_object = "mesh".to_string();
    let mut object_ids = HashSet::new();
    for line in raw.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        match fields.first().copied() {
            Some("v") if fields.len() >= 4 && points.len() < MAX_DOMAIN_RECORDS => {
                if let (Ok(x), Ok(y), Ok(z)) =
                    (fields[1].parse(), fields[2].parse(), fields[3].parse())
                {
                    points.push([x, y, z]);
                }
            }
            Some("f") if fields.len() >= 4 && faces.len() < MAX_DOMAIN_RECORDS => {
                let face = fields[1..]
                    .iter()
                    .filter_map(|value| value.split('/').next()?.parse::<usize>().ok())
                    .map(|index| index.saturating_sub(1))
                    .collect::<Vec<_>>();
                if face.len() >= 3 {
                    faces.push(face);
                }
            }
            Some("o" | "g") if fields.len() >= 2 => {
                current_object = fields[1..].join(" ");
                let id = stable_id("object", &current_object);
                if object_ids.insert(id.clone()) {
                    document.nodes.push(VisualizationNode::new(
                        id,
                        current_object.clone(),
                        "object",
                    ));
                }
            }
            Some("usemtl") if fields.len() >= 2 => {
                let material = fields[1..].join(" ");
                let id = stable_id("material", &material);
                if !document.nodes.iter().any(|node| node.id == id) {
                    document
                        .nodes
                        .push(VisualizationNode::new(&id, &material, "material"));
                }
                let object_id = stable_id("object", &current_object);
                if !document.nodes.iter().any(|node| node.id == object_id) {
                    document.nodes.push(VisualizationNode::new(
                        &object_id,
                        &current_object,
                        "object",
                    ));
                }
                document.edges.push(VisualizationEdge::new(
                    stable_id("material-edge", &format!("{current_object}:{material}")),
                    object_id,
                    id,
                    "uses",
                    "material-binding",
                ));
            }
            _ => {}
        }
    }
    add_geometry(document, points, faces);
}

fn parse_stl(bytes: &[u8], document: &mut VisualizationDocument) {
    let looks_ascii = bytes.starts_with(b"solid")
        && std::str::from_utf8(bytes.get(..bytes.len().min(512)).unwrap_or(bytes)).is_ok();
    let mut points = Vec::<[f64; 3]>::new();
    let mut faces = Vec::<Vec<usize>>::new();
    if looks_ascii {
        let raw = String::from_utf8_lossy(bytes);
        for line in raw.lines() {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields.first() == Some(&"vertex") && fields.len() >= 4 {
                if let (Ok(x), Ok(y), Ok(z)) =
                    (fields[1].parse(), fields[2].parse(), fields[3].parse())
                {
                    points.push([x, y, z]);
                }
            }
            if points.len() >= MAX_DOMAIN_RECORDS * 3 {
                break;
            }
        }
    } else if bytes.len() >= 84 {
        let declared = u32::from_le_bytes(bytes[80..84].try_into().unwrap_or_default()) as usize;
        let count = declared
            .min((bytes.len() - 84) / 50)
            .min(MAX_DOMAIN_RECORDS);
        for triangle in 0..count {
            let offset = 84 + triangle * 50 + 12;
            for vertex in 0..3 {
                let base = offset + vertex * 12;
                points.push([
                    f32::from_le_bytes(bytes[base..base + 4].try_into().unwrap_or_default()) as f64,
                    f32::from_le_bytes(bytes[base + 4..base + 8].try_into().unwrap_or_default())
                        as f64,
                    f32::from_le_bytes(bytes[base + 8..base + 12].try_into().unwrap_or_default())
                        as f64,
                ]);
            }
        }
    }
    for start in (0..points.len()).step_by(3) {
        if start + 2 < points.len() {
            faces.push(vec![start, start + 1, start + 2]);
        }
    }
    add_geometry(document, points, faces);
}

fn parse_ply(raw: &str, document: &mut VisualizationDocument) {
    let mut lines = raw.lines();
    let mut vertex_count = 0usize;
    let mut face_count = 0usize;
    for line in lines.by_ref() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.get(0) == Some(&"element") && fields.get(1) == Some(&"vertex") {
            vertex_count = fields
                .get(2)
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        } else if fields.get(0) == Some(&"element") && fields.get(1) == Some(&"face") {
            face_count = fields
                .get(2)
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
        } else if line.trim() == "end_header" {
            break;
        }
    }
    let mut points = Vec::new();
    for line in lines.by_ref().take(vertex_count.min(MAX_DOMAIN_RECORDS)) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() >= 3 {
            if let (Ok(x), Ok(y), Ok(z)) = (fields[0].parse(), fields[1].parse(), fields[2].parse())
            {
                points.push([x, y, z]);
            }
        }
    }
    let mut faces = Vec::new();
    for line in lines.take(face_count.min(MAX_DOMAIN_RECORDS)) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let count = fields
            .first()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        if count >= 3 && fields.len() > count {
            faces.push(
                fields[1..=count]
                    .iter()
                    .filter_map(|value| value.parse().ok())
                    .collect(),
            );
        }
    }
    add_geometry(document, points, faces);
}

fn parse_gltf_json(raw: &str, document: &mut VisualizationDocument) {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return;
    };
    parse_gltf_value(&value, document);
}

fn parse_glb(bytes: &[u8], document: &mut VisualizationDocument) {
    if bytes.len() < 20 || &bytes[0..4] != b"glTF" {
        return;
    }
    let json_length = u32::from_le_bytes(bytes[12..16].try_into().unwrap_or_default()) as usize;
    let chunk_type = u32::from_le_bytes(bytes[16..20].try_into().unwrap_or_default());
    if chunk_type == 0x4E4F534A && bytes.len() >= 20 + json_length {
        let raw = String::from_utf8_lossy(&bytes[20..20 + json_length]);
        parse_gltf_json(raw.trim_end_matches('\0'), document);
    }
}

fn parse_gltf_value(value: &Value, document: &mut VisualizationDocument) {
    let nodes = value
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (index, value) in nodes.iter().take(MAX_DOMAIN_NODES).enumerate() {
        let id = format!("scene-node:{index}");
        let mut node = VisualizationNode::new(
            &id,
            value
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Scene node"),
            if value.get("mesh").is_some() {
                "mesh-instance"
            } else {
                "scene-node"
            },
        );
        for key in [
            "translation",
            "rotation",
            "scale",
            "matrix",
            "mesh",
            "camera",
            "skin",
        ] {
            if let Some(field) = value.get(key) {
                node.metadata.insert(key.to_string(), field.clone());
            }
        }
        document.nodes.push(node);
    }
    for (index, value) in nodes.iter().enumerate() {
        if let Some(children) = value.get("children").and_then(Value::as_array) {
            for child in children.iter().filter_map(Value::as_u64) {
                document.edges.push(VisualizationEdge::new(
                    format!("scene-edge:{index}:{child}"),
                    format!("scene-node:{index}"),
                    format!("scene-node:{child}"),
                    "child",
                    "scene-hierarchy",
                ));
            }
        }
    }
    document.metadata.insert(
        "scene".to_string(),
        json!({
            "format":"gltf",
            "scene_count":value.get("scenes").and_then(Value::as_array).map_or(0, Vec::len),
            "node_count":nodes.len(),
            "mesh_count":value.get("meshes").and_then(Value::as_array).map_or(0, Vec::len),
            "material_count":value.get("materials").and_then(Value::as_array).map_or(0, Vec::len),
            "asset":value.get("asset").cloned().unwrap_or(Value::Null),
        }),
    );
}

fn parse_cad_exchange(raw: &str, file_type: &str, document: &mut VisualizationDocument) {
    if matches!(file_type, "step" | "stp") {
        let entity = Regex::new(r"(?mi)^#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*)\);\s*$").unwrap();
        for capture in entity.captures_iter(raw).take(MAX_DOMAIN_NODES) {
            let id = format!("step:{}", &capture[1]);
            let kind = capture[2].to_string();
            if !is_cad_semantic_entity(&kind) {
                continue;
            }
            let mut node = VisualizationNode::new(&id, &kind, cad_entity_category(&kind));
            node.metadata
                .insert("entity_id".to_string(), json!(&capture[1]));
            node.metadata
                .insert("arguments".to_string(), json!(&capture[3]));
            document.nodes.push(node);
            for reference in Regex::new(r"#(\d+)").unwrap().captures_iter(&capture[3]) {
                document.edges.push(VisualizationEdge::new(
                    format!("step-ref:{}:{}", &capture[1], &reference[1]),
                    &id,
                    format!("step:{}", &reference[1]),
                    "references",
                    "cad-reference",
                ));
            }
        }
        document.metadata.insert(
            "cad".to_string(),
            json!({"format":"STEP","entity_count":document.nodes.len()}),
        );
    } else {
        let entity = Regex::new(r"(?mi)^\s*([A-Z][A-Z0-9_]{2,})\s*[:(]").unwrap();
        for (index, capture) in entity.captures_iter(raw).take(MAX_DOMAIN_NODES).enumerate() {
            document.nodes.push(VisualizationNode::new(
                format!("cad-entity:{index}"),
                &capture[1],
                "geometry-entity",
            ));
        }
        document.metadata.insert(
            "cad".to_string(),
            json!({"format":file_type.to_ascii_uppercase(),"entity_count":document.nodes.len()}),
        );
    }
}

fn parse_freecad_document(raw: &str, document: &mut VisualizationDocument) {
    let object =
        Regex::new(r#"(?s)<Object\s+type="([^"]+)"\s+name="([^"]+)"[^>]*>(.*?)</Object>"#).unwrap();
    let label =
        Regex::new(r#"<Property\s+name="Label"[^>]*>.*?<String\s+value="([^"]*)""#).unwrap();
    let link = Regex::new(r#"<Link\s+value="([^"]+)""#).unwrap();
    for capture in object.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let name = capture[2].to_string();
        let kind = capture[1].to_string();
        let body = capture[3].to_string();
        let display = label
            .captures(&body)
            .and_then(|capture| capture.get(1))
            .map(|value| value.as_str())
            .unwrap_or(&name);
        let mut node =
            VisualizationNode::new(format!("freecad:{name}"), display, freecad_category(&kind));
        node.metadata.insert("object_type".to_string(), json!(kind));
        document.nodes.push(node);
        for target in link.captures_iter(&body) {
            document.edges.push(VisualizationEdge::new(
                format!("freecad-link:{name}:{}", &target[1]),
                format!("freecad:{name}"),
                format!("freecad:{}", &target[1]),
                "links",
                "feature-reference",
            ));
        }
    }
    document.metadata.insert(
        "cad".to_string(),
        json!({"format":"FreeCAD","object_count":document.nodes.len()}),
    );
}

fn parse_openscad(raw: &str, document: &mut VisualizationDocument) {
    let pattern =
        Regex::new(r"(?m)^\s*(module|function)\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(([^)]*)\)").unwrap();
    for capture in pattern.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let id = stable_id("scad", &capture[2]);
        let mut node = VisualizationNode::new(
            &id,
            &capture[2],
            if &capture[1] == "module" {
                "feature"
            } else {
                "parameter"
            },
        );
        node.metadata
            .insert("parameters".to_string(), json!(&capture[3]));
        document.nodes.push(node);
    }
    let operation = Regex::new(
        r"(?m)^\s*(union|difference|intersection|hull|minkowski|translate|rotate|scale)\s*\(",
    )
    .unwrap();
    for (index, capture) in operation
        .captures_iter(raw)
        .take(MAX_DOMAIN_NODES)
        .enumerate()
    {
        document.nodes.push(VisualizationNode::new(
            format!("operation:{index}"),
            &capture[1],
            "operation",
        ));
    }
    document.metadata.insert(
        "cad".to_string(),
        json!({"format":"OpenSCAD","construct_count":document.nodes.len(),"source":raw}),
    );
}

fn parse_dxf(raw: &str, document: &mut VisualizationDocument) {
    let lines = raw.lines().map(str::trim).collect::<Vec<_>>();
    let mut index = 0usize;
    let mut entity_index = 0usize;
    while index + 1 < lines.len() && entity_index < MAX_DOMAIN_NODES {
        if lines[index] == "0" {
            let entity = lines[index + 1];
            if matches!(
                entity,
                "LINE"
                    | "CIRCLE"
                    | "ARC"
                    | "LWPOLYLINE"
                    | "POLYLINE"
                    | "SPLINE"
                    | "DIMENSION"
                    | "INSERT"
            ) {
                document.nodes.push(VisualizationNode::new(
                    format!("dxf:{entity_index}"),
                    entity,
                    if entity == "DIMENSION" {
                        "constraint"
                    } else {
                        "geometry-entity"
                    },
                ));
                entity_index += 1;
            }
        }
        index += 2;
    }
    document.metadata.insert(
        "cad".to_string(),
        json!({"format":"DXF","entity_count":entity_index}),
    );
}

fn parse_graphics_source(raw: &str, file_type: &str, document: &mut VisualizationDocument) {
    if file_type == "mtl" {
        let material = Regex::new(r"(?m)^newmtl\s+(.+?)\s*$").unwrap();
        for capture in material.captures_iter(raw).take(MAX_DOMAIN_NODES) {
            document.nodes.push(VisualizationNode::new(
                stable_id("material", &capture[1]),
                capture[1].trim(),
                "material",
            ));
        }
    } else {
        let declaration = Regex::new(r"(?m)^\s*(?:layout\s*\([^)]*\)\s*)?(?:uniform|in|out|var|let|const|struct|fn|void|float|vec[234]|mat[234])\s+([A-Za-z_][A-Za-z0-9_]*)").unwrap();
        for capture in declaration.captures_iter(raw).take(MAX_DOMAIN_NODES) {
            document.nodes.push(VisualizationNode::new(
                stable_id("shader-symbol", &capture[1]),
                &capture[1],
                "shader-symbol",
            ));
        }
    }
    document.metadata.insert(
        "source".to_string(),
        json!({"language":file_type,"text":raw}),
    );
}

fn parse_robotics(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "urdf" | "xacro" | "sdf" => parse_robot_xml(&read_text(path)?, &mut document),
        "csv" | "json" | "jsonl" => parse_record_text(
            &read_text(path)?,
            &asset.file_type,
            &mut document,
            "robot-state",
        ),
        "yaml" | "yml" => parse_map_yaml(&read_text(path)?, &mut document),
        "bag" | "db3" | "mcap" => {
            document.metadata.insert(
                "binary_trace".to_string(),
                json!({"format":asset.file_type,"bytes":asset.size_bytes,"requires_sdk":true}),
            );
            document
                .diagnostics
                .push(binary_sdk_diagnostic(&asset.file_type));
        }
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_robot_xml(raw: &str, document: &mut VisualizationDocument) {
    let link = Regex::new(
        r#"(?s)<link\s+name=["']([^"']+)["'][^>]*>(.*?)</link>|<link\s+name=["']([^"']+)["']\s*/>"#,
    )
    .unwrap();
    for capture in link.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let name = capture.get(1).or_else(|| capture.get(3)).unwrap().as_str();
        let mut node = VisualizationNode::new(stable_id("link", name), name, "robot-link");
        let body = capture
            .get(2)
            .map(|value| value.as_str())
            .unwrap_or_default();
        if let Some(mesh) = Regex::new(r#"<mesh\s+filename=["']([^"']+)["']"#)
            .unwrap()
            .captures(body)
        {
            node.metadata.insert("mesh".to_string(), json!(&mesh[1]));
        }
        document.nodes.push(node);
    }
    let joint = Regex::new(
        r#"(?s)<joint\s+name=["']([^"']+)["']\s+type=["']([^"']+)["'][^>]*>(.*?)</joint>"#,
    )
    .unwrap();
    let parent = Regex::new(r#"<parent\s+link=["']([^"']+)["']"#).unwrap();
    let child = Regex::new(r#"<child\s+link=["']([^"']+)["']"#).unwrap();
    let origin =
        Regex::new(r#"<origin[^>]*(?:xyz=["']([^"']*)["'])?[^>]*(?:rpy=["']([^"']*)["'])?"#)
            .unwrap();
    for capture in joint.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let name = &capture[1];
        let kind = &capture[2];
        let body = &capture[3];
        let id = stable_id("joint", name);
        let mut node = VisualizationNode::new(&id, name, "robot-joint");
        node.metadata.insert("joint_type".to_string(), json!(kind));
        if let Some(origin) = origin.captures(body) {
            node.metadata.insert(
                "origin".to_string(),
                json!({
                    "xyz":origin.get(1).map(|value| value.as_str()),
                    "rpy":origin.get(2).map(|value| value.as_str()),
                }),
            );
        }
        document.nodes.push(node);
        let parent_name = parent.captures(body).map(|value| value[1].to_string());
        let child_name = child.captures(body).map(|value| value[1].to_string());
        if let Some(parent_name) = parent_name.as_deref() {
            document.edges.push(VisualizationEdge::new(
                stable_id("joint-parent", name),
                stable_id("link", parent_name),
                &id,
                "parent",
                "kinematic-chain",
            ));
        }
        if let Some(child_name) = child_name.as_deref() {
            document.edges.push(VisualizationEdge::new(
                stable_id("joint-child", name),
                &id,
                stable_id("link", child_name),
                "child",
                "kinematic-chain",
            ));
        }
    }
    document.metadata.insert(
        "scene".to_string(),
        json!({"format":"robot-description","node_count":document.nodes.len()}),
    );
}

fn parse_map_yaml(raw: &str, document: &mut VisualizationDocument) {
    let mut map = serde_json::Map::new();
    for line in raw.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        map.insert(
            key.trim().to_string(),
            Value::String(value.trim().trim_matches(['\'', '"']).to_string()),
        );
    }
    document
        .metadata
        .insert("map".to_string(), Value::Object(map));
}

fn parse_compiler(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let raw = match read_optional_text(path) {
        Some(raw) => raw,
        None => return Ok(None),
    };
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "ll" | "mlir" | "wat" => parse_ir(&raw, asset.file_type.as_str(), &mut document),
        "dot" | "cfg" => parse_dot_graph(&raw, &mut document, "basic-block", "cfg"),
        "graphml" => parse_graphml(&raw, &mut document, "basic-block", "cfg"),
        "ast" | "json" => parse_ast_value(&raw, &mut document),
        file_type if is_source_type(file_type) => {
            parse_source_symbols(&raw, file_type, &mut document)
        }
        _ => return Ok(None),
    }
    document.metadata.insert(
        "source".to_string(),
        json!({"language":asset.file_type,"text":raw}),
    );
    Ok(Some(document))
}

fn parse_ir(raw: &str, file_type: &str, document: &mut VisualizationDocument) {
    let function = Regex::new(
        r"(?m)^\s*(?:define\s+[^@]*@|func(?:\.func)?\s+@|\(func\s+\$)([A-Za-z_$][A-Za-z0-9_.$-]*)",
    )
    .unwrap();
    let block = Regex::new(r"(?m)^\s*([A-Za-z$._][A-Za-z0-9$._-]*):\s*(?:;.*)?$").unwrap();
    let instruction =
        Regex::new(r"(?m)^\s*(%[A-Za-z0-9$._-]+)\s*=\s*([A-Za-z][A-Za-z0-9_.-]*)").unwrap();
    let branch = Regex::new(
        r"\b(?:br|cf\.br|cf\.cond_br)\b[^\n]*?(?:label\s+)?%?([A-Za-z$._][A-Za-z0-9$._-]*)",
    )
    .unwrap();
    let mut current_function: Option<String> = None;
    let mut current_block: Option<String> = None;
    for line in raw.lines() {
        if let Some(capture) = function.captures(line) {
            let name = capture[1].to_string();
            let id = stable_id("function", &name);
            document
                .nodes
                .push(VisualizationNode::new(&id, &name, "function"));
            current_function = Some(id);
            current_block = None;
            continue;
        }
        if let Some(capture) = block.captures(line) {
            let name = capture[1].to_string();
            let id = stable_id(
                "block",
                &format!("{}:{name}", current_function.as_deref().unwrap_or("module")),
            );
            let mut node = VisualizationNode::new(&id, &name, "basic-block");
            node.parent_id = current_function.clone();
            document.nodes.push(node);
            if let Some(function) = current_function.as_deref() {
                document.edges.push(VisualizationEdge::new(
                    stable_id("contains", &format!("{function}:{id}")),
                    function,
                    &id,
                    "contains",
                    "ir-structure",
                ));
            }
            current_block = Some(id);
            continue;
        }
        if let Some(capture) = instruction.captures(line) {
            let name = capture[1].to_string();
            let operation = capture[2].to_string();
            let id = stable_id(
                "instruction",
                &format!("{}:{name}", current_block.as_deref().unwrap_or("module")),
            );
            let mut node = VisualizationNode::new(&id, &name, "instruction");
            node.metadata.insert("opcode".to_string(), json!(operation));
            node.metadata.insert("text".to_string(), json!(line.trim()));
            node.parent_id = current_block.clone().or_else(|| current_function.clone());
            document.nodes.push(node);
            if let Some(parent) = current_block.as_deref().or(current_function.as_deref()) {
                document.edges.push(VisualizationEdge::new(
                    stable_id("ir-parent", &format!("{parent}:{id}")),
                    parent,
                    &id,
                    "contains",
                    "ir-structure",
                ));
            }
        }
        if let (Some(source), Some(capture)) = (current_block.as_deref(), branch.captures(line)) {
            let target_label = capture[1].to_string();
            let target = stable_id(
                "block",
                &format!(
                    "{}:{target_label}",
                    current_function.as_deref().unwrap_or("module")
                ),
            );
            document.edges.push(VisualizationEdge::new(
                stable_id("cfg-edge", &format!("{source}:{target}")),
                source,
                target,
                "branch",
                "cfg",
            ));
        }
    }
    document.metadata.insert(
        "ir".to_string(),
        json!({"format":file_type,"line_count":raw.lines().count()}),
    );
}

fn parse_ast_value(raw: &str, document: &mut VisualizationDocument) {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        fn visit(
            value: &Value,
            parent: Option<&str>,
            path: &str,
            document: &mut VisualizationDocument,
            count: &mut usize,
        ) {
            if *count >= MAX_DOMAIN_NODES {
                return;
            }
            let kind = value
                .get("kind")
                .or_else(|| value.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("AST node");
            let label = value
                .get("name")
                .or_else(|| value.get("spelling"))
                .or_else(|| value.get("value"))
                .and_then(value_label)
                .unwrap_or_else(|| kind.to_string());
            let id = stable_id("ast", path);
            let mut node = VisualizationNode::new(&id, label, "ast");
            node.metadata.insert("kind".to_string(), json!(kind));
            document.nodes.push(node);
            if let Some(parent) = parent {
                document.edges.push(VisualizationEdge::new(
                    stable_id("ast-edge", path),
                    parent,
                    &id,
                    "child",
                    "syntax",
                ));
            }
            *count += 1;
            let children = value
                .get("inner")
                .or_else(|| value.get("children"))
                .or_else(|| value.get("body"));
            if let Some(children) = children.and_then(Value::as_array) {
                for (index, child) in children.iter().enumerate() {
                    visit(
                        child,
                        Some(&id),
                        &format!("{path}/{index}"),
                        document,
                        count,
                    );
                }
            }
        }
        let mut count = 0;
        if let Some(array) = value.as_array() {
            for (index, child) in array.iter().enumerate() {
                visit(child, None, &format!("$/{index}"), document, &mut count);
            }
        } else {
            visit(&value, None, "$", document, &mut count);
        }
    } else {
        let line =
            Regex::new(r"(?m)^(\s*)([A-Za-z][A-Za-z0-9_]*(?:Decl|Stmt|Expr|Type)?)(?:\s+(.+))?$")
                .unwrap();
        let mut parents = Vec::<(usize, String)>::new();
        for (index, capture) in line.captures_iter(raw).take(MAX_DOMAIN_NODES).enumerate() {
            let depth = capture[1].chars().count();
            let id = format!("ast:{index}");
            let label = capture
                .get(3)
                .map(|value| value.as_str())
                .unwrap_or(&capture[2]);
            let mut node = VisualizationNode::new(&id, label, "ast");
            node.metadata.insert("kind".to_string(), json!(&capture[2]));
            while parents
                .last()
                .is_some_and(|(parent_depth, _)| *parent_depth >= depth)
            {
                parents.pop();
            }
            if let Some((_, parent)) = parents.last() {
                document.edges.push(VisualizationEdge::new(
                    format!("ast-edge:{index}"),
                    parent,
                    &id,
                    "child",
                    "syntax",
                ));
            }
            document.nodes.push(node);
            parents.push((depth, id));
        }
    }
}

fn parse_source_symbols(raw: &str, file_type: &str, document: &mut VisualizationDocument) {
    let declaration = Regex::new(
        r"(?m)^\s*(?:pub\s+|export\s+|async\s+|static\s+|final\s+)*(?:fn|def|function|class|struct|enum|interface|trait)\s+([A-Za-z_$][A-Za-z0-9_$]*)",
    )
    .unwrap();
    let mut symbols = HashMap::new();
    for capture in declaration.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let name = capture[1].to_string();
        let id = stable_id("symbol", &name);
        symbols.insert(name.clone(), id.clone());
        document
            .nodes
            .push(VisualizationNode::new(&id, &name, "symbol"));
    }
    let calls = Regex::new(r"\b([A-Za-z_$][A-Za-z0-9_$]*)\s*\(").unwrap();
    for (source_name, source_id) in symbols.clone() {
        let Some(start) = raw.find(&source_name) else {
            continue;
        };
        let body = &raw[start..raw.len().min(start + 16_384)];
        for capture in calls.captures_iter(body).take(200) {
            let target_name = capture[1].to_string();
            if let Some(target_id) = symbols.get(&target_name) {
                if target_id != &source_id {
                    document.edges.push(VisualizationEdge::new(
                        stable_id("call", &format!("{source_name}:{target_name}")),
                        &source_id,
                        target_id,
                        "calls",
                        "call",
                    ));
                }
            }
        }
    }
    document.metadata.insert(
        "source".to_string(),
        json!({"language":file_type,"text":raw}),
    );
}

fn parse_database(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "sql" => parse_sql(&read_text(path)?, &mut document),
        "plan" => parse_query_plan(&read_text(path)?, &mut document),
        "csv" | "json" | "jsonl" => {
            parse_record_text(&read_text(path)?, &asset.file_type, &mut document, "row")
        }
        "db" | "sqlite" | "sqlite3" | "duckdb" => {
            let bytes = fs::read(path)?;
            if bytes.starts_with(b"SQLite format 3\0") {
                parse_sqlite_pages(&bytes, &mut document);
            } else {
                document.metadata.insert(
                    "database".to_string(),
                    json!({"format":asset.file_type,"bytes":asset.size_bytes,"schema_requires_sdk":true}),
                );
                document
                    .diagnostics
                    .push(binary_sdk_diagnostic(&asset.file_type));
            }
        }
        "parquet" | "arrow" => {
            document.metadata.insert(
                "table".to_string(),
                json!({"format":asset.file_type,"bytes":asset.size_bytes,"preview_requires_sdk":true}),
            );
            document
                .diagnostics
                .push(binary_sdk_diagnostic(&asset.file_type));
        }
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_sql(raw: &str, document: &mut VisualizationDocument) {
    let create = Regex::new(
        r#"(?is)CREATE\s+(TABLE|VIEW)\s+(?:IF\s+NOT\s+EXISTS\s+)?(?:[\["`]?([A-Za-z_][A-Za-z0-9_.$-]*)[\]"`]?)\s*\((.*?)\)\s*;"#,
    )
    .unwrap();
    for capture in create.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let object_kind = capture[1].to_ascii_lowercase();
        let name = capture[2].to_string();
        let table_id = stable_id(&object_kind, &name);
        document.nodes.push(VisualizationNode::new(
            &table_id,
            &name,
            object_kind.clone(),
        ));
        for (index, definition) in split_sql_list(&capture[3]).into_iter().enumerate() {
            let trimmed = definition.trim();
            if trimmed.is_empty() {
                continue;
            }
            let upper = trimmed.to_ascii_uppercase();
            if upper.starts_with("PRIMARY KEY")
                || upper.starts_with("FOREIGN KEY")
                || upper.starts_with("UNIQUE")
                || upper.starts_with("CONSTRAINT")
                || upper.starts_with("CHECK")
            {
                continue;
            }
            let fields = trimmed.split_whitespace().collect::<Vec<_>>();
            let Some(column_name) = fields.first() else {
                continue;
            };
            let column_name = column_name.trim_matches(['[', ']', '`', '"']);
            let column_id = stable_id("column", &format!("{name}.{column_name}"));
            let mut node = VisualizationNode::new(&column_id, column_name, "column");
            node.parent_id = Some(table_id.clone());
            node.metadata.insert(
                "data_type".to_string(),
                json!(fields.get(1).copied().unwrap_or("")),
            );
            node.metadata
                .insert("definition".to_string(), json!(trimmed));
            document.nodes.push(node);
            document.edges.push(VisualizationEdge::new(
                format!("column-edge:{index}:{column_id}"),
                &table_id,
                &column_id,
                "column",
                "schema",
            ));
        }
    }
    let index = Regex::new(r"(?is)CREATE\s+(?:UNIQUE\s+)?INDEX\s+(?:IF\s+NOT\s+EXISTS\s+)?([A-Za-z_][A-Za-z0-9_.$-]*)\s+ON\s+([A-Za-z_][A-Za-z0-9_.$-]*)\s*\(([^)]*)\)").unwrap();
    for capture in index.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let id = stable_id("index", &capture[1]);
        let mut node = VisualizationNode::new(&id, &capture[1], "index");
        node.metadata
            .insert("columns".to_string(), json!(&capture[3]));
        document.nodes.push(node);
        document.edges.push(VisualizationEdge::new(
            stable_id("index-edge", &capture[1]),
            stable_id("table", &capture[2]),
            &id,
            "index",
            "schema",
        ));
    }
    parse_sql_lineage(raw, document);
    document
        .metadata
        .insert("source".to_string(), json!({"language":"sql","text":raw}));
}

fn parse_sql_lineage(raw: &str, document: &mut VisualizationDocument) {
    let statement = Regex::new(r"(?is)(?:INSERT\s+INTO|CREATE\s+(?:TABLE|VIEW))\s+([A-Za-z_][A-Za-z0-9_.$-]*).*?\bSELECT\b.*?\bFROM\s+([A-Za-z_][A-Za-z0-9_.$-]*)").unwrap();
    for capture in statement.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let target = capture[1].to_string();
        let source = capture[2].to_string();
        for name in [&target, &source] {
            let id = stable_id("table", name);
            if !document.nodes.iter().any(|node| node.id == id) {
                document
                    .nodes
                    .push(VisualizationNode::new(&id, name, "table"));
            }
        }
        document.edges.push(VisualizationEdge::new(
            stable_id("lineage", &format!("{source}:{target}")),
            stable_id("table", &source),
            stable_id("table", &target),
            "feeds",
            "lineage",
        ));
    }
}

fn parse_query_plan(raw: &str, document: &mut VisualizationDocument) {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        let root = value
            .as_array()
            .and_then(|values| values.first())
            .and_then(|value| value.get("Plan"))
            .or_else(|| value.get("Plan"))
            .unwrap_or(&value);
        fn visit(
            value: &Value,
            parent: Option<&str>,
            path: &str,
            document: &mut VisualizationDocument,
        ) {
            let id = stable_id("plan", path);
            let label = value
                .get("Node Type")
                .or_else(|| value.get("name"))
                .or_else(|| value.get("operator"))
                .and_then(value_label)
                .unwrap_or_else(|| "Plan operator".to_string());
            let mut node = VisualizationNode::new(&id, label, "plan-operator");
            if let Some(object) = value.as_object() {
                for (key, value) in object {
                    if value.is_number() || value.is_string() || value.is_boolean() {
                        node.metadata.insert(key.clone(), value.clone());
                    }
                }
            }
            document.nodes.push(node);
            if let Some(parent) = parent {
                document.edges.push(VisualizationEdge::new(
                    stable_id("plan-edge", path),
                    parent,
                    &id,
                    "input",
                    "query-plan",
                ));
            }
            if let Some(children) = value
                .get("Plans")
                .or_else(|| value.get("children"))
                .and_then(Value::as_array)
            {
                for (index, child) in children.iter().enumerate() {
                    visit(child, Some(&id), &format!("{path}/{index}"), document);
                }
            }
        }
        visit(root, None, "$", document);
    } else {
        let mut parents = Vec::<(usize, String)>::new();
        for (index, line) in raw
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(MAX_DOMAIN_NODES)
            .enumerate()
        {
            let indent = line
                .chars()
                .take_while(|ch| ch.is_whitespace() || *ch == '-' || *ch == '>')
                .count();
            let id = format!("plan:{index}");
            while parents.last().is_some_and(|(depth, _)| *depth >= indent) {
                parents.pop();
            }
            document.nodes.push(VisualizationNode::new(
                &id,
                line.trim_start_matches([' ', '-', '>']),
                "plan-operator",
            ));
            if let Some((_, parent)) = parents.last() {
                document.edges.push(VisualizationEdge::new(
                    format!("plan-edge:{index}"),
                    parent,
                    &id,
                    "input",
                    "query-plan",
                ));
            }
            parents.push((indent, id));
        }
    }
}

fn parse_sqlite_pages(bytes: &[u8], document: &mut VisualizationDocument) {
    let page_size = if bytes.len() >= 18 {
        let value = u16::from_be_bytes([bytes[16], bytes[17]]) as usize;
        if value == 1 {
            65_536
        } else {
            value
        }
    } else {
        0
    };
    let page_count = if page_size > 0 {
        bytes.len() / page_size
    } else {
        0
    };
    let mut node = VisualizationNode::new("database", "SQLite database", "database");
    node.metrics
        .insert("page_size".to_string(), page_size as f64);
    node.metrics
        .insert("page_count".to_string(), page_count as f64);
    node.metrics.insert("bytes".to_string(), bytes.len() as f64);
    document.nodes.push(node);
    document.metadata.insert(
        "database".to_string(),
        json!({"format":"sqlite","page_size":page_size,"page_count":page_count,"schema_requires_sdk":true}),
    );
    document.diagnostics.push(VisualizationDiagnostic {
        level: "info".to_string(),
        message: "SQLite container metadata is available; table/schema introspection is disabled until a SQLite adapter is available.".to_string(),
        metadata: BTreeMap::new(),
    });
}

fn parse_software(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let raw = match read_optional_text(path) {
        Some(raw) => raw,
        None => return Ok(None),
    };
    let mut document = empty_document(descriptor, asset);
    parse_source_symbols(&raw, asset.file_type.as_str(), &mut document);
    parse_declared_dependencies(&raw, asset, &mut document);
    Ok(Some(document))
}

fn parse_declared_dependencies(
    raw: &str,
    asset: &DomainAsset,
    document: &mut VisualizationDocument,
) {
    let mut dependencies = Vec::<String>::new();
    if asset.name.eq_ignore_ascii_case("Cargo.toml") {
        let mut in_dependencies = false;
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                in_dependencies = trimmed.contains("dependencies");
                continue;
            }
            if in_dependencies {
                if let Some((name, _)) = trimmed.split_once('=') {
                    dependencies.push(name.trim().to_string());
                }
            }
        }
    } else if asset.name.eq_ignore_ascii_case("package.json") {
        if let Ok(value) = serde_json::from_str::<Value>(raw) {
            for key in ["dependencies", "devDependencies", "peerDependencies"] {
                if let Some(object) = value.get(key).and_then(Value::as_object) {
                    dependencies.extend(object.keys().cloned());
                }
            }
        }
    } else {
        let import =
            Regex::new(r#"(?m)^\s*(?:use|import|from|require\s*\()\s*["']?([A-Za-z0-9_.@/-]+)"#)
                .unwrap();
        dependencies.extend(
            import
                .captures_iter(raw)
                .take(MAX_DOMAIN_NODES)
                .map(|capture| capture[1].to_string()),
        );
    }
    let root = "artifact";
    document
        .nodes
        .push(VisualizationNode::new(root, &asset.name, "source"));
    dependencies.sort();
    dependencies.dedup();
    for dependency in dependencies.into_iter().take(MAX_DOMAIN_NODES) {
        let id = stable_id("dependency", &dependency);
        document
            .nodes
            .push(VisualizationNode::new(&id, &dependency, "dependency"));
        document.edges.push(VisualizationEdge::new(
            stable_id("dependency-edge", &dependency),
            root,
            id,
            "depends on",
            "dependency",
        ));
    }
}

fn parse_program_analysis(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let raw = match read_optional_text(path) {
        Some(raw) => raw,
        None => return Ok(None),
    };
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "dot" | "cfg" | "dfg" => parse_dot_graph(&raw, &mut document, "fact", "analysis"),
        "graphml" | "gexf" => parse_graphml(&raw, &mut document, "fact", "analysis"),
        "sarif" => parse_sarif(&raw, &mut document),
        "json" | "jsonl" | "trace" | "log" => parse_analysis_records(&raw, &mut document),
        file_type if is_source_type(file_type) => {
            parse_source_symbols(&raw, file_type, &mut document)
        }
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_security(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "sarif" => parse_sarif(&read_text(path)?, &mut document),
        "json" | "jsonl" | "csv" | "log" | "nessus" | "yara" => {
            parse_security_records(&read_text(path)?, &mut document)
        }
        "har" => parse_har_trace(&read_text(path)?, &mut document),
        "pcap" | "pcapng" => {
            document.metadata.insert(
                "packet_capture".to_string(),
                json!({"format":asset.file_type,"bytes":asset.size_bytes,"decode_requires_sdk":true}),
            );
            document
                .diagnostics
                .push(binary_sdk_diagnostic(&asset.file_type));
        }
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_sarif(raw: &str, document: &mut VisualizationDocument) {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return;
    };
    let mut sequence = 0usize;
    for run in value
        .get("runs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let rule_labels = run
            .pointer("/tool/driver/rules")
            .and_then(Value::as_array)
            .map(|rules| {
                rules
                    .iter()
                    .filter_map(|rule| {
                        let id = rule.get("id")?.as_str()?.to_string();
                        let label = rule
                            .get("name")
                            .or_else(|| rule.pointer("/shortDescription/text"))
                            .and_then(Value::as_str)
                            .unwrap_or(&id)
                            .to_string();
                        Some((id, label))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default();
        for result in run
            .get("results")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .take(MAX_DOMAIN_RECORDS)
        {
            let rule_id = result
                .get("ruleId")
                .and_then(Value::as_str)
                .unwrap_or("finding");
            let message = result
                .pointer("/message/text")
                .and_then(Value::as_str)
                .unwrap_or_else(|| {
                    rule_labels
                        .get(rule_id)
                        .map(String::as_str)
                        .unwrap_or(rule_id)
                });
            let finding_id = format!("finding:{sequence}");
            let mut finding = VisualizationNode::new(&finding_id, message, "finding");
            finding.status = result
                .get("level")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            finding
                .metadata
                .insert("rule_id".to_string(), json!(rule_id));
            if let Some(properties) = result.get("properties") {
                finding
                    .metadata
                    .insert("properties".to_string(), properties.clone());
            }
            document.nodes.push(finding);
            if let Some(locations) = result.get("locations").and_then(Value::as_array) {
                for (location_index, location) in locations.iter().enumerate() {
                    let uri = location
                        .pointer("/physicalLocation/artifactLocation/uri")
                        .and_then(Value::as_str)
                        .unwrap_or("location");
                    let line = location
                        .pointer("/physicalLocation/region/startLine")
                        .and_then(Value::as_u64);
                    let id = format!("location:{sequence}:{location_index}");
                    let mut node = VisualizationNode::new(
                        &id,
                        line.map_or_else(|| uri.to_string(), |line| format!("{uri}:{line}")),
                        "location",
                    );
                    node.metadata
                        .insert("location".to_string(), location.clone());
                    document.nodes.push(node);
                    document.edges.push(VisualizationEdge::new(
                        format!("finding-location:{sequence}:{location_index}"),
                        &finding_id,
                        &id,
                        "evidence",
                        "finding-evidence",
                    ));
                }
            }
            if let Some(flows) = result.get("codeFlows").and_then(Value::as_array) {
                for (flow_index, flow) in flows.iter().enumerate() {
                    let locations = flow
                        .get("threadFlows")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .flat_map(|thread| {
                            thread
                                .get("locations")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                        });
                    let mut previous = finding_id.clone();
                    for (step, location) in locations.enumerate() {
                        let id = format!("flow:{sequence}:{flow_index}:{step}");
                        let label = location
                            .pointer("/location/message/text")
                            .or_else(|| {
                                location.pointer("/location/physicalLocation/artifactLocation/uri")
                            })
                            .and_then(Value::as_str)
                            .unwrap_or("flow step");
                        document
                            .nodes
                            .push(VisualizationNode::new(&id, label, "evidence"));
                        document.edges.push(VisualizationEdge::new(
                            format!("flow-edge:{sequence}:{flow_index}:{step}"),
                            &previous,
                            &id,
                            "flows to",
                            "taint-flow",
                        ));
                        previous = id;
                    }
                }
            }
            sequence += 1;
        }
    }
    document
        .metadata
        .insert("finding_count".to_string(), json!(sequence));
}

fn parse_analysis_records(raw: &str, document: &mut VisualizationDocument) {
    let records = json_records(raw);
    for (index, record) in records.iter().take(MAX_DOMAIN_RECORDS).enumerate() {
        let Some(object) = record.as_object() else {
            continue;
        };
        let source = first_string(object, &["source", "caller", "from", "src"]);
        let target = first_string(object, &["target", "callee", "to", "dst", "sink"]);
        if let (Some(source), Some(target)) = (source, target) {
            let source_id = stable_id("fact", &source);
            let target_id = stable_id("fact", &target);
            ensure_node(document, &source_id, &source, "fact");
            ensure_node(document, &target_id, &target, "fact");
            document.edges.push(VisualizationEdge::new(
                format!("analysis-edge:{index}"),
                source_id,
                target_id,
                first_string(object, &["kind", "type", "label"])
                    .unwrap_or_else(|| "relates".to_string()),
                if object
                    .keys()
                    .any(|key| key.to_ascii_lowercase().contains("taint"))
                {
                    "taint-flow"
                } else {
                    "analysis"
                },
            ));
        }
    }
    if document.nodes.is_empty() {
        parse_explicit_edges(raw, document, "fact", "analysis");
    }
}

fn parse_security_records(raw: &str, document: &mut VisualizationDocument) {
    if raw.trim_start().starts_with('{') || raw.trim_start().starts_with('[') {
        let records = json_records(raw);
        for (index, record) in records.iter().take(MAX_DOMAIN_RECORDS).enumerate() {
            let Some(object) = record.as_object() else {
                continue;
            };
            let label = first_string(
                object,
                &["message", "title", "name", "rule", "vulnerability", "cve"],
            )
            .unwrap_or_else(|| format!("Finding {}", index + 1));
            let mut node = VisualizationNode::new(format!("finding:{index}"), label, "finding");
            node.status =
                first_string(object, &["severity", "level", "status"]).unwrap_or_default();
            node.metadata.extend(object.clone());
            document.nodes.push(node);
        }
    } else {
        let finding = Regex::new(
            r"(?mi)^.*(?:CVE-\d{4}-\d+|critical|high|medium|low|vulnerability|finding|alert).*$",
        )
        .unwrap();
        for (index, matched) in finding.find_iter(raw).take(MAX_DOMAIN_RECORDS).enumerate() {
            document.nodes.push(VisualizationNode::new(
                format!("finding:{index}"),
                matched.as_str().trim(),
                "finding",
            ));
        }
    }
    document
        .metadata
        .insert("finding_count".to_string(), json!(document.nodes.len()));
}

fn parse_trace_domain(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
    category: &str,
) -> Result<Option<VisualizationDocument>> {
    let mut document = empty_document(descriptor, asset);
    if matches!(
        asset.file_type.as_str(),
        "nsys-rep" | "ncu-rep" | "otf2" | "etl" | "dmp" | "core"
    ) {
        document.metadata.insert(
            "binary_trace".to_string(),
            json!({"format":asset.file_type,"bytes":asset.size_bytes,"requires_sdk":true}),
        );
        document
            .diagnostics
            .push(binary_sdk_diagnostic(&asset.file_type));
        return Ok(Some(document));
    }
    let raw = match read_optional_text(path) {
        Some(raw) => raw,
        None => return Ok(None),
    };
    match asset.file_type.as_str() {
        "csv" | "json" | "jsonl" => {
            parse_record_text(&raw, &asset.file_type, &mut document, category)
        }
        "cu" | "cuh" | "ptx" | "hip" | "cl" => parse_kernel_source(&raw, &mut document),
        _ => parse_log_events(&raw, &mut document, category),
    }
    Ok(Some(document))
}

fn parse_kernel_source(raw: &str, document: &mut VisualizationDocument) {
    let kernel =
        Regex::new(r"(?m)(?:__global__\s+(?:void\s+)?|\.entry\s+)([A-Za-z_][A-Za-z0-9_$]*)")
            .unwrap();
    for capture in kernel.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        document.nodes.push(VisualizationNode::new(
            stable_id("kernel", &capture[1]),
            &capture[1],
            "gpu-kernel",
        ));
    }
    document.metadata.insert(
        "source".to_string(),
        json!({"language":"gpu-kernel","text":raw}),
    );
}

fn parse_distributed(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let raw = match read_optional_text(path) {
        Some(raw) => raw,
        None => return Ok(None),
    };
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "proto" => parse_proto(&raw, &mut document),
        "dot" => parse_dot_graph(&raw, &mut document, "service", "communication"),
        "graphml" => parse_graphml(&raw, &mut document, "service", "communication"),
        "har" => parse_har_trace(&raw, &mut document),
        "json" | "jsonl" | "trace" | "log" => parse_distributed_records(&raw, &mut document),
        "yaml" | "yml" => parse_kubernetes_yaml(&raw, &mut document),
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_proto(raw: &str, document: &mut VisualizationDocument) {
    let service = Regex::new(r"(?m)^\s*service\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{").unwrap();
    let rpc = Regex::new(r"(?m)^\s*rpc\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(\s*([A-Za-z0-9_.]+)\s*\)\s+returns\s*\(\s*([A-Za-z0-9_.]+)\s*\)").unwrap();
    let mut current_service = None::<String>;
    for line in raw.lines() {
        if let Some(capture) = service.captures(line) {
            let id = stable_id("service", &capture[1]);
            document
                .nodes
                .push(VisualizationNode::new(&id, &capture[1], "service"));
            current_service = Some(id);
        }
        if let Some(capture) = rpc.captures(line) {
            let name = capture[1].to_string();
            let id = stable_id("rpc", &name);
            let mut node = VisualizationNode::new(&id, &name, "rpc");
            node.metadata
                .insert("request".to_string(), json!(&capture[2]));
            node.metadata
                .insert("response".to_string(), json!(&capture[3]));
            document.nodes.push(node);
            if let Some(service) = current_service.as_deref() {
                document.edges.push(VisualizationEdge::new(
                    stable_id("rpc-edge", &name),
                    service,
                    id,
                    "exposes",
                    "service-api",
                ));
            }
        }
    }
    document.metadata.insert(
        "source".to_string(),
        json!({"language":"protobuf","text":raw}),
    );
}

fn parse_distributed_records(raw: &str, document: &mut VisualizationDocument) {
    let records = json_records(raw);
    for (index, record) in records.iter().take(MAX_DOMAIN_RECORDS).enumerate() {
        let Some(object) = record.as_object() else {
            continue;
        };
        let source = first_string(
            object,
            &["service", "source", "client", "peer.service", "from"],
        )
        .unwrap_or_else(|| "service".to_string());
        let target = first_string(object, &["target", "server", "destination", "peer", "to"])
            .unwrap_or_else(|| source.clone());
        let source_id = stable_id("service", &source);
        let target_id = stable_id("service", &target);
        ensure_node(document, &source_id, &source, "service");
        ensure_node(document, &target_id, &target, "service");
        let category = first_string(object, &["kind", "type", "protocol"])
            .unwrap_or_else(|| "rpc".to_string());
        document.edges.push(VisualizationEdge::new(
            format!("rpc:{index}"),
            &source_id,
            &target_id,
            &category,
            "communication",
        ));
        let timestamp = first_string(object, &["timestamp", "time", "start_time"]);
        let mut event = VisualizationEvent {
            id: format!("span:{index}"),
            sequence: index,
            label: first_string(object, &["name", "operation", "span_name"]).unwrap_or(category),
            category: "span".to_string(),
            status: first_string(object, &["status", "state"]).unwrap_or_default(),
            timestamp,
            source_id: Some(source_id),
            target_id: Some(target_id),
            metadata: object.clone().into_iter().collect(),
        };
        if let Some(duration) = first_number(
            object,
            &["duration", "duration_ms", "latency_ms", "elapsed_ms"],
        ) {
            event
                .metadata
                .insert("duration_ms".to_string(), json!(duration));
        }
        document.events.push(event);
    }
    frames_from_events(document);
}

fn parse_kubernetes_yaml(raw: &str, document: &mut VisualizationDocument) {
    let kind = Regex::new(r"(?m)^kind:\s*([A-Za-z0-9_.-]+)\s*$").unwrap();
    let name = Regex::new(r"(?m)^\s{0,4}name:\s*([A-Za-z0-9_.-]+)\s*$").unwrap();
    for (index, part) in raw.split("\n---").enumerate() {
        let kind = kind.captures(part).map(|capture| capture[1].to_string());
        let name = name.captures(part).map(|capture| capture[1].to_string());
        if let (Some(kind), Some(name)) = (kind, name) {
            let mut node = VisualizationNode::new(format!("resource:{index}"), name, "service");
            node.metadata.insert("kind".to_string(), json!(kind));
            document.nodes.push(node);
        }
    }
    document
        .metadata
        .insert("source".to_string(), json!({"language":"yaml","text":raw}));
}

fn parse_scientific(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Result<Option<VisualizationDocument>> {
    let mut document = empty_document(descriptor, asset);
    match asset.file_type.as_str() {
        "csv" | "json" | "jsonl" => parse_record_text(
            &read_text(path)?,
            &asset.file_type,
            &mut document,
            "measurement",
        ),
        "vtk" => parse_legacy_vtk(&read_text(path)?, &mut document),
        "vtu" => parse_vtu(&read_text(path)?, &mut document),
        "m" | "jl" | "py" | "md" => parse_equations(&read_text(path)?, &mut document),
        "npy" => parse_npy_header(&fs::read(path)?, &mut document),
        "npz" => parse_npz_entries(path, &mut document)?,
        "mat" | "h5" | "hdf5" | "nc" => {
            document.metadata.insert(
                "array_container".to_string(),
                json!({"format":asset.file_type,"bytes":asset.size_bytes,"inspection_requires_sdk":true}),
            );
            document
                .diagnostics
                .push(binary_sdk_diagnostic(&asset.file_type));
        }
        _ => return Ok(None),
    }
    Ok(Some(document))
}

fn parse_legacy_vtk(raw: &str, document: &mut VisualizationDocument) {
    let tokens = raw.split_whitespace().collect::<Vec<_>>();
    let mut points = Vec::<[f64; 3]>::new();
    let mut faces = Vec::<Vec<usize>>::new();
    let mut index = 0usize;
    while index < tokens.len() {
        if tokens[index].eq_ignore_ascii_case("POINTS") && index + 2 < tokens.len() {
            let count = tokens[index + 1]
                .parse::<usize>()
                .unwrap_or(0)
                .min(MAX_DOMAIN_RECORDS);
            index += 3;
            for _ in 0..count {
                if index + 2 >= tokens.len() {
                    break;
                }
                if let (Ok(x), Ok(y), Ok(z)) = (
                    tokens[index].parse(),
                    tokens[index + 1].parse(),
                    tokens[index + 2].parse(),
                ) {
                    points.push([x, y, z]);
                }
                index += 3;
            }
            continue;
        }
        if (tokens[index].eq_ignore_ascii_case("POLYGONS")
            || tokens[index].eq_ignore_ascii_case("CELLS"))
            && index + 2 < tokens.len()
        {
            let count = tokens[index + 1]
                .parse::<usize>()
                .unwrap_or(0)
                .min(MAX_DOMAIN_RECORDS);
            index += 3;
            for _ in 0..count {
                if index >= tokens.len() {
                    break;
                }
                let size = tokens[index].parse::<usize>().unwrap_or(0);
                index += 1;
                let mut cell = Vec::new();
                for _ in 0..size {
                    if let Some(value) = tokens.get(index).and_then(|value| value.parse().ok()) {
                        cell.push(value);
                    }
                    index += 1;
                }
                if cell.len() >= 3 {
                    faces.push(cell);
                }
            }
            continue;
        }
        index += 1;
    }
    add_geometry(document, points, faces);
    document
        .metadata
        .insert("scientific_mesh".to_string(), json!({"format":"vtk"}));
}

fn parse_vtu(raw: &str, document: &mut VisualizationDocument) {
    let points = extract_xml_data_array(raw, Some("Points"), None)
        .chunks(3)
        .filter_map(|values| (values.len() == 3).then(|| [values[0], values[1], values[2]]))
        .take(MAX_DOMAIN_RECORDS)
        .collect::<Vec<_>>();
    let connectivity = extract_xml_data_array(raw, Some("Cells"), Some("connectivity"))
        .into_iter()
        .map(|value| value.max(0.0) as usize)
        .collect::<Vec<_>>();
    let offsets = extract_xml_data_array(raw, Some("Cells"), Some("offsets"))
        .into_iter()
        .map(|value| value.max(0.0) as usize)
        .collect::<Vec<_>>();
    let mut faces = Vec::new();
    let mut start = 0usize;
    for offset in offsets.into_iter().take(MAX_DOMAIN_RECORDS) {
        if offset <= connectivity.len() && offset > start {
            faces.push(connectivity[start..offset].to_vec());
            start = offset;
        }
    }
    add_geometry(document, points, faces);
    document
        .metadata
        .insert("scientific_mesh".to_string(), json!({"format":"vtu"}));
}

fn parse_equations(raw: &str, document: &mut VisualizationDocument) {
    let equation = Regex::new(r"(?m)^\s*(?:\$\$?|\\\[)?\s*([A-Za-z][A-Za-z0-9_]*(?:\([^)]*\))?)\s*=\s*([^=\n]{2,})(?:\$\$?|\\\])?\s*$").unwrap();
    for (index, capture) in equation
        .captures_iter(raw)
        .take(MAX_DOMAIN_NODES)
        .enumerate()
    {
        let mut node = VisualizationNode::new(format!("equation:{index}"), &capture[1], "equation");
        node.metadata
            .insert("expression".to_string(), json!(&capture[2]));
        document.nodes.push(node);
    }
    document.metadata.insert(
        "source".to_string(),
        json!({"language":"equation-source","text":raw}),
    );
}

fn parse_npy_header(bytes: &[u8], document: &mut VisualizationDocument) {
    let Some(header) = npy_header(bytes) else {
        return;
    };
    let shape = Regex::new(r"'shape'\s*:\s*\(([^)]*)\)")
        .unwrap()
        .captures(&header)
        .map(|capture| capture[1].to_string());
    let dtype = Regex::new(r"'descr'\s*:\s*'([^']+)'")
        .unwrap()
        .captures(&header)
        .map(|capture| capture[1].to_string());
    let mut node = VisualizationNode::new("array", "NumPy array", "array");
    node.metadata.insert("shape".to_string(), json!(shape));
    node.metadata.insert("dtype".to_string(), json!(dtype));
    node.metrics.insert("bytes".to_string(), bytes.len() as f64);
    document.nodes.push(node);
    document
        .metadata
        .insert("array".to_string(), json!({"format":"npy","header":header}));
}

fn parse_npz_entries(path: &Path, document: &mut VisualizationDocument) -> Result<()> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;
    for index in 0..archive.len().min(MAX_DOMAIN_NODES) {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_string();
        if !name.ends_with(".npy") {
            continue;
        }
        let mut bytes = Vec::new();
        entry.by_ref().take(64 * 1024).read_to_end(&mut bytes)?;
        let mut node = VisualizationNode::new(format!("array:{index}"), &name, "array");
        if let Some(header) = npy_header(&bytes) {
            node.metadata.insert("header".to_string(), json!(header));
        }
        node.metrics.insert(
            "compressed_bytes".to_string(),
            entry.compressed_size() as f64,
        );
        node.metrics
            .insert("bytes".to_string(), entry.size() as f64);
        document.nodes.push(node);
    }
    document.metadata.insert(
        "array".to_string(),
        json!({"format":"npz","array_count":document.nodes.len()}),
    );
    Ok(())
}

pub(super) fn sdk_executables(sdk: &str) -> Vec<String> {
    let normalized = sdk.trim().to_ascii_lowercase();
    let candidates: &[&str] = if normalized == "python" || normalized.starts_with("python ") {
        &["python", "python3"]
    } else if normalized.contains("pytorch") {
        &["python", "python3"]
    } else if normalized.contains("tensorflow") || normalized == "jax" {
        &["python", "python3"]
    } else if normalized.contains("onnx") {
        &["python", "python3", "onnxruntime_perf_test"]
    } else if normalized.contains("hugging face")
        || normalized.contains("spacy")
        || normalized.contains("nltk")
        || normalized.contains("sentence")
        || normalized.contains("opencv")
        || normalized.contains("open3d")
        || normalized.contains("numpy")
        || normalized.contains("scipy")
    {
        &["python", "python3"]
    } else if normalized.contains("blender") {
        &["blender"]
    } else if normalized.contains("freecad") {
        &["FreeCADCmd", "freecadcmd", "freecad"]
    } else if normalized.contains("cadquery") {
        &["cq-cli", "python", "python3"]
    } else if normalized.contains("openscad") {
        &["openscad"]
    } else if normalized.contains("open cascade") {
        &["DRAWEXE", "drawexe"]
    } else if normalized == "ros 2" {
        &["ros2"]
    } else if normalized.contains("gazebo") {
        &["gz", "gazebo"]
    } else if normalized.contains("moveit") {
        &["ros2"]
    } else if normalized.contains("mujoco") {
        &["python", "python3"]
    } else if normalized.contains("wireshark") {
        &["tshark", "wireshark"]
    } else if normalized.contains("libpcap") {
        &["dumpcap", "tcpdump"]
    } else if normalized.contains("scapy") {
        &["scapy", "python", "python3"]
    } else if normalized.contains("mininet") {
        &["mn"]
    } else if normalized == "ns-3" {
        &["ns3", "waf"]
    } else if normalized.contains("etw") {
        &["wpr", "xperf"]
    } else if normalized.contains("linux perf") {
        &["perf"]
    } else if normalized.contains("ebpf") {
        &["bpftrace", "bpftool"]
    } else if normalized.contains("strace") {
        &["strace"]
    } else if normalized == "llvm" {
        &["llvm-config", "opt", "clang"]
    } else if normalized == "mlir" {
        &["mlir-opt"]
    } else if normalized == "clang" {
        &["clang", "clang-cl"]
    } else if normalized == "gcc" {
        &["gcc"]
    } else if normalized == "rustc" {
        &["rustc"]
    } else if normalized == "sqlite" {
        &["sqlite3"]
    } else if normalized == "duckdb" {
        &["duckdb"]
    } else if normalized.contains("postgres") {
        &["psql"]
    } else if normalized.contains("arrow") || normalized.contains("datafusion") {
        &["python", "python3"]
    } else if normalized == "cargo" {
        &["cargo"]
    } else if normalized == "npm" {
        &["npm"]
    } else if normalized == "maven" {
        &["mvn"]
    } else if normalized == "gradle" {
        &["gradle"]
    } else if normalized == ".net" {
        &["dotnet"]
    } else if normalized == "cmake" {
        &["cmake"]
    } else if normalized == "codeql" {
        &["codeql"]
    } else if normalized == "semgrep" {
        &["semgrep"]
    } else if normalized == "joern" {
        &["joern", "joern-parse"]
    } else if normalized == "valgrind" {
        &["valgrind"]
    } else if normalized == "yara" {
        &["yara"]
    } else if normalized.contains("owasp zap") {
        &["zap-cli", "zap.sh", "zap.bat"]
    } else if normalized == "cuda" {
        &["nvcc"]
    } else if normalized == "rocm" {
        &["hipcc", "rocminfo"]
    } else if normalized == "opencl" {
        &["clinfo"]
    } else if normalized == "mpi" {
        &["mpiexec", "mpirun"]
    } else if normalized.contains("nsight systems") {
        &["nsys"]
    } else if normalized.contains("nsight compute") {
        &["ncu"]
    } else if normalized.contains("opentelemetry") {
        &["otel-cli"]
    } else if normalized == "grpc" {
        &["grpcurl"]
    } else if normalized == "kubernetes" {
        &["kubectl"]
    } else if normalized == "etcd" {
        &["etcdctl"]
    } else if normalized == "julia" {
        &["julia"]
    } else if normalized == "matlab" {
        &["matlab"]
    } else if normalized == "petsc" {
        &["petsc-config", "mpiexec"]
    } else if normalized == "vtk" {
        &["pvpython", "python", "python3"]
    } else if normalized == "opengl" {
        &["glxinfo"]
    } else if normalized == "vulkan" {
        &["vulkaninfo"]
    } else if normalized == "directx" {
        &["dxc"]
    } else if normalized == "webgpu" {
        &["node"]
    } else {
        &[]
    };
    candidates
        .iter()
        .map(|value| (*value).to_string())
        .collect()
}

fn contains_any_ci(raw: &str, needles: &[&str]) -> bool {
    let lower = raw.to_ascii_lowercase();
    needles
        .iter()
        .any(|needle| lower.contains(&needle.to_ascii_lowercase()))
}

fn count_occurrences_ci(raw: &str, needle: &str) -> usize {
    raw.to_ascii_lowercase()
        .match_indices(&needle.to_ascii_lowercase())
        .count()
}

fn contains_equation(raw: &str, file_type: &str) -> bool {
    matches!(file_type, "m" | "jl" | "py" | "ipynb" | "md")
        && Regex::new(r"(?m)^\s*[A-Za-z][A-Za-z0-9_]*(?:\([^)]*\))?\s*=\s*[^=\n]{2,}$")
            .unwrap()
            .is_match(raw)
}

fn is_source_type(file_type: &str) -> bool {
    matches!(
        file_type,
        "c" | "cc"
            | "cpp"
            | "h"
            | "hpp"
            | "rs"
            | "go"
            | "java"
            | "kt"
            | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "cs"
    )
}

fn is_image_type(file_type: &str) -> bool {
    matches!(
        file_type,
        "png" | "jpg" | "jpeg" | "bmp" | "tif" | "tiff" | "webp"
    )
}

fn stable_id(prefix: &str, value: &str) -> String {
    let normalized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let compact = normalized
        .split('-')
        .filter(|part| !part.is_empty())
        .take(12)
        .collect::<Vec<_>>()
        .join("-");
    if compact.is_empty() {
        format!(
            "{prefix}:{}",
            &blake3::hash(value.as_bytes()).to_hex()[..16]
        )
    } else {
        format!("{prefix}:{}", &compact[..compact.len().min(72)])
    }
}

fn number(value: &Value) -> Option<f64> {
    value
        .as_f64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn value_label(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
        .or_else(|| value.as_u64().map(|value| value.to_string()))
        .or_else(|| value.as_f64().map(|value| value.to_string()))
        .or_else(|| {
            value.as_object().and_then(|object| {
                ["text", "token", "word", "name", "label", "id"]
                    .iter()
                    .find_map(|key| object.get(*key).and_then(value_label))
            })
        })
}

fn json_records(raw: &str) -> Vec<Value> {
    if let Ok(value) = serde_json::from_str::<Value>(raw) {
        if let Some(values) = value.as_array() {
            return values.clone();
        }
        if let Some(object) = value.as_object() {
            for key in [
                "records",
                "rows",
                "data",
                "results",
                "annotations",
                "detections",
                "events",
                "spans",
                "tokens",
            ] {
                if let Some(values) = object.get(key).and_then(Value::as_array) {
                    return values.clone();
                }
            }
        }
        return vec![value];
    }
    raw.lines()
        .take(MAX_DOMAIN_RECORDS)
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn image_dimensions(bytes: &[u8], file_type: &str) -> Option<(u32, u32, u8)> {
    match file_type {
        "png" if bytes.len() >= 26 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") => Some((
            u32::from_be_bytes(bytes[16..20].try_into().ok()?),
            u32::from_be_bytes(bytes[20..24].try_into().ok()?),
            match bytes[25] {
                0 => 1,
                2 => 3,
                3 => 1,
                4 => 2,
                6 => 4,
                _ => 0,
            },
        )),
        "bmp" if bytes.len() >= 30 && bytes.starts_with(b"BM") => Some((
            u32::from_le_bytes(bytes[18..22].try_into().ok()?),
            i32::from_le_bytes(bytes[22..26].try_into().ok()?).unsigned_abs(),
            (u16::from_le_bytes(bytes[28..30].try_into().ok()?) / 8) as u8,
        )),
        "jpg" | "jpeg" => jpeg_dimensions(bytes),
        "webp" if bytes.len() >= 30 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" => {
            if &bytes[12..16] == b"VP8X" {
                let width = 1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]);
                let height = 1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]);
                Some((width, height, 4))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn jpeg_dimensions(bytes: &[u8]) -> Option<(u32, u32, u8)> {
    if bytes.len() < 4 || bytes[0..2] != [0xff, 0xd8] {
        return None;
    }
    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        if bytes[offset] != 0xff {
            offset += 1;
            continue;
        }
        let marker = bytes[offset + 1];
        offset += 2;
        if matches!(marker, 0xd8 | 0xd9) {
            continue;
        }
        if offset + 2 > bytes.len() {
            return None;
        }
        let length = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        if length < 2 || offset + length > bytes.len() {
            return None;
        }
        if matches!(marker, 0xc0..=0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf) && length >= 8 {
            return Some((
                u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]]) as u32,
                u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]]) as u32,
                bytes[offset + 7],
            ));
        }
        offset += length;
    }
    None
}

fn fcstd_document_xml(path: &Path) -> Option<String> {
    let file = fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;
    let mut entry = archive.by_name("Document.xml").ok()?;
    if entry.size() > MAX_ADAPTER_TEXT_BYTES {
        return None;
    }
    let mut raw = String::new();
    entry.read_to_string(&mut raw).ok()?;
    Some(raw)
}

fn add_geometry(
    document: &mut VisualizationDocument,
    points: Vec<[f64; 3]>,
    faces: Vec<Vec<usize>>,
) {
    if points.is_empty() {
        return;
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for point in &points {
        for axis in 0..3 {
            min[axis] = min[axis].min(point[axis]);
            max[axis] = max[axis].max(point[axis]);
        }
    }
    let mut mesh = VisualizationNode::new("mesh", "Geometry", "mesh");
    mesh.metrics
        .insert("vertices".to_string(), points.len() as f64);
    mesh.metrics.insert("faces".to_string(), faces.len() as f64);
    mesh.metadata.insert("bounds_min".to_string(), json!(min));
    mesh.metadata.insert("bounds_max".to_string(), json!(max));
    document.nodes.push(mesh);
    document.metadata.insert(
        "geometry".to_string(),
        json!({"points":points,"faces":faces,"bounds":{"min":min,"max":max}}),
    );
}

fn is_cad_semantic_entity(kind: &str) -> bool {
    [
        "PRODUCT",
        "PRODUCT_DEFINITION",
        "SHAPE_REPRESENTATION",
        "MANIFOLD_SOLID_BREP",
        "ADVANCED_BREP_SHAPE_REPRESENTATION",
        "NEXT_ASSEMBLY_USAGE_OCCURRENCE",
        "CARTESIAN_POINT",
        "AXIS2_PLACEMENT_3D",
        "PLANE",
        "CYLINDRICAL_SURFACE",
        "CONICAL_SURFACE",
        "SPHERICAL_SURFACE",
        "TOROIDAL_SURFACE",
        "ADVANCED_FACE",
        "CLOSED_SHELL",
        "OPEN_SHELL",
    ]
    .iter()
    .any(|candidate| kind.contains(candidate))
}

fn cad_entity_category(kind: &str) -> &'static str {
    if kind.contains("PRODUCT") || kind.contains("ASSEMBLY") {
        "part"
    } else if kind.contains("REPRESENTATION") || kind.contains("BREP") || kind.contains("SHELL") {
        "body"
    } else if kind.contains("SURFACE") || kind.contains("FACE") {
        "geometry-entity"
    } else {
        "parameter"
    }
}

fn freecad_category(kind: &str) -> &'static str {
    let lower = kind.to_ascii_lowercase();
    if lower.contains("body") {
        "body"
    } else if lower.contains("part") {
        "part"
    } else if lower.contains("sketch") {
        "sketch"
    } else if lower.contains("constraint") {
        "constraint"
    } else if lower.contains("feature")
        || lower.contains("pad")
        || lower.contains("pocket")
        || lower.contains("fillet")
        || lower.contains("chamfer")
    {
        "feature"
    } else {
        "object"
    }
}

fn binary_sdk_diagnostic(file_type: &str) -> VisualizationDiagnostic {
    VisualizationDiagnostic {
        level: "info".to_string(),
        message: format!(
            "{} is a binary container. Atlas exposes verified container metadata only; semantic decoding remains disabled until a compatible SDK is detected.",
            file_type.to_ascii_uppercase()
        ),
        metadata: BTreeMap::from([
            ("file_type".to_string(), json!(file_type)),
            ("execution_enabled".to_string(), json!(false)),
        ]),
    }
}

fn parse_record_text(
    raw: &str,
    file_type: &str,
    document: &mut VisualizationDocument,
    category: &str,
) {
    let records = if file_type == "csv" {
        delimited_records(raw, ',')
    } else if file_type == "tsv" {
        delimited_records(raw, '\t')
    } else {
        json_records(raw)
    };
    let mut numeric = BTreeMap::<String, Vec<VisualizationPoint>>::new();
    for (index, record) in records.iter().take(MAX_DOMAIN_RECORDS).enumerate() {
        let Some(object) = record.as_object() else {
            continue;
        };
        let label = first_string(
            object,
            &[
                "name",
                "label",
                "stage",
                "operation",
                "event",
                "kernel",
                "process",
                "thread",
            ],
        )
        .unwrap_or_else(|| format!("Record {}", index + 1));
        let node_id = format!("record:{index}");
        let mut node = VisualizationNode::new(&node_id, &label, category);
        node.metadata.extend(object.clone());
        for (key, value) in object {
            if let Some(number) = number(value) {
                node.metrics.insert(key.clone(), number);
                numeric
                    .entry(key.clone())
                    .or_default()
                    .push(VisualizationPoint {
                        timestamp_ms: record_timestamp(object).unwrap_or(index as i64),
                        value: number,
                    });
            }
        }
        document.nodes.push(node);
        let source = first_string(object, &["source", "from", "parent", "caller"]);
        let target = first_string(object, &["target", "to", "child", "callee"]);
        if let (Some(source), Some(target)) = (source, target) {
            let source_id = stable_id(category, &source);
            let target_id = stable_id(category, &target);
            ensure_node(document, &source_id, &source, category);
            ensure_node(document, &target_id, &target, category);
            document.edges.push(VisualizationEdge::new(
                format!("record-edge:{index}"),
                source_id,
                target_id,
                first_string(object, &["kind", "type", "protocol"])
                    .unwrap_or_else(|| "relates".to_string()),
                category,
            ));
        }
        document.events.push(VisualizationEvent {
            id: format!("event:{index}"),
            sequence: index,
            label,
            category: category.to_string(),
            status: first_string(object, &["status", "state", "level"]).unwrap_or_default(),
            timestamp: first_string(object, &["timestamp", "time", "start_time"]),
            source_id: Some(node_id),
            target_id: None,
            metadata: object.clone().into_iter().collect(),
        });
    }
    for (index, (label, points)) in numeric.into_iter().take(24).enumerate() {
        document.series.push(VisualizationSeries {
            id: format!("series:{index}"),
            label,
            unit: String::new(),
            node_id: None,
            category: category.to_string(),
            points,
        });
    }
    frames_from_events(document);
    document.metadata.insert(
        "table".to_string(),
        json!({"columns":record_columns(&records),"rows":records.into_iter().take(500).collect::<Vec<_>>() }),
    );
}

fn delimited_records(raw: &str, delimiter: char) -> Vec<Value> {
    let mut lines = raw.lines();
    let headers = lines
        .next()
        .unwrap_or_default()
        .split(delimiter)
        .map(|value| value.trim().trim_matches('"').to_string())
        .collect::<Vec<_>>();
    lines
        .take(MAX_DOMAIN_RECORDS)
        .map(|line| {
            let fields = line
                .split(delimiter)
                .map(|value| value.trim().trim_matches('"'))
                .collect::<Vec<_>>();
            let mut object = serde_json::Map::new();
            for (index, header) in headers.iter().enumerate() {
                if let Some(value) = fields.get(index) {
                    object.insert(
                        header.clone(),
                        value
                            .parse::<f64>()
                            .map(Value::from)
                            .unwrap_or_else(|_| Value::String((*value).to_string())),
                    );
                }
            }
            Value::Object(object)
        })
        .collect()
}

fn record_columns(records: &[Value]) -> Vec<String> {
    let mut columns = records
        .iter()
        .filter_map(Value::as_object)
        .flat_map(|object| object.keys().cloned())
        .collect::<Vec<_>>();
    columns.sort();
    columns.dedup();
    columns
}

fn record_timestamp(object: &serde_json::Map<String, Value>) -> Option<i64> {
    [
        "timestamp_ms",
        "time_ms",
        "step",
        "epoch",
        "sequence",
        "index",
    ]
    .iter()
    .find_map(|key| object.get(*key).and_then(number).map(|value| value as i64))
}

fn parse_log_events(raw: &str, document: &mut VisualizationDocument, category: &str) {
    let timestamp =
        Regex::new(r"(?x)(?P<ts>\d{4}-\d{2}-\d{2}[T ][0-9:.+-]+|\d+(?:\.\d+)?(?:ms|us|s)?)")
            .unwrap();
    let pid = Regex::new(r"(?i)\b(?:pid|process)[=: ]+(\d+)").unwrap();
    let tid = Regex::new(r"(?i)\b(?:tid|thread)[=: ]+(\d+)").unwrap();
    for (index, line) in raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(MAX_DOMAIN_RECORDS)
        .enumerate()
    {
        let label = line.trim().to_string();
        let mut metadata = BTreeMap::new();
        if let Some(capture) = pid.captures(line) {
            metadata.insert("pid".to_string(), json!(&capture[1]));
        }
        if let Some(capture) = tid.captures(line) {
            metadata.insert("tid".to_string(), json!(&capture[1]));
        }
        document.events.push(VisualizationEvent {
            id: format!("event:{index}"),
            sequence: index,
            label: label.clone(),
            category: category.to_string(),
            status: String::new(),
            timestamp: timestamp
                .captures(line)
                .and_then(|capture| capture.name("ts"))
                .map(|value| value.as_str().to_string()),
            source_id: None,
            target_id: None,
            metadata,
        });
        document.nodes.push(VisualizationNode::new(
            format!("log:{index}"),
            label,
            category,
        ));
    }
    frames_from_events(document);
}

fn frames_from_events(document: &mut VisualizationDocument) {
    if !document.frames.is_empty() {
        return;
    }
    document.frames = document
        .events
        .iter()
        .map(|event| VisualizationFrame {
            id: format!("frame:{}", event.sequence),
            sequence: event.sequence,
            label: event.label.clone(),
            active_nodes: [event.source_id.clone(), event.target_id.clone()]
                .into_iter()
                .flatten()
                .collect(),
            active_edges: Vec::new(),
            metrics: event
                .metadata
                .iter()
                .filter_map(|(key, value)| number(value).map(|value| (key.clone(), value)))
                .collect(),
        })
        .collect();
}

fn parse_dot_graph(
    raw: &str,
    document: &mut VisualizationDocument,
    node_category: &str,
    edge_category: &str,
) {
    let edge = Regex::new(
        r#"(?m)(?:"([^"]+)"|([A-Za-z0-9_.$:-]+))\s*(->|--)\s*(?:"([^"]+)"|([A-Za-z0-9_.$:-]+))(?:\s*\[([^]]*)\])?"#,
    )
    .unwrap();
    let label = Regex::new(r#"label\s*=\s*(?:"([^"]*)"|([^,\]]+))"#).unwrap();
    for (index, capture) in edge.captures_iter(raw).take(MAX_DOMAIN_RECORDS).enumerate() {
        let source = capture.get(1).or_else(|| capture.get(2)).unwrap().as_str();
        let target = capture.get(4).or_else(|| capture.get(5)).unwrap().as_str();
        let source_id = stable_id(node_category, source);
        let target_id = stable_id(node_category, target);
        ensure_node(document, &source_id, source, node_category);
        ensure_node(document, &target_id, target, node_category);
        let edge_label = capture
            .get(6)
            .and_then(|attributes| label.captures(attributes.as_str()))
            .and_then(|capture| capture.get(1).or_else(|| capture.get(2)))
            .map(|value| value.as_str().trim().to_string())
            .unwrap_or_default();
        document.edges.push(VisualizationEdge::new(
            format!("dot-edge:{index}"),
            source_id,
            target_id,
            edge_label,
            edge_category,
        ));
    }
    if document.nodes.is_empty() {
        let node =
            Regex::new(r#"(?m)^\s*(?:"([^"]+)"|([A-Za-z0-9_.$:-]+))\s*\[([^]]*)\]\s*;?"#).unwrap();
        for capture in node.captures_iter(raw).take(MAX_DOMAIN_NODES) {
            let name = capture.get(1).or_else(|| capture.get(2)).unwrap().as_str();
            ensure_node(
                document,
                &stable_id(node_category, name),
                name,
                node_category,
            );
        }
    }
}

fn parse_graphml(
    raw: &str,
    document: &mut VisualizationDocument,
    node_category: &str,
    edge_category: &str,
) {
    let node = Regex::new(r#"(?s)<node\b[^>]*\bid=["']([^"']+)["'][^>]*>(.*?)</node>|<node\b[^>]*\bid=["']([^"']+)["'][^>]*/>"#).unwrap();
    let data = Regex::new(r#"(?s)<data\b[^>]*>(.*?)</data>"#).unwrap();
    for capture in node.captures_iter(raw).take(MAX_DOMAIN_NODES) {
        let id = capture.get(1).or_else(|| capture.get(3)).unwrap().as_str();
        let label = capture
            .get(2)
            .and_then(|body| data.captures(body.as_str()))
            .and_then(|capture| capture.get(1))
            .map(|value| strip_xml(value.as_str()))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| id.to_string());
        document.nodes.push(VisualizationNode::new(
            stable_id(node_category, id),
            label,
            node_category,
        ));
    }
    let edge =
        Regex::new(r#"<edge\b[^>]*\bsource=["']([^"']+)["'][^>]*\btarget=["']([^"']+)["'][^>]*>"#)
            .unwrap();
    for (index, capture) in edge.captures_iter(raw).take(MAX_DOMAIN_RECORDS).enumerate() {
        document.edges.push(VisualizationEdge::new(
            format!("xml-edge:{index}"),
            stable_id(node_category, &capture[1]),
            stable_id(node_category, &capture[2]),
            "relates",
            edge_category,
        ));
    }
}

fn parse_explicit_edges(
    raw: &str,
    document: &mut VisualizationDocument,
    node_category: &str,
    edge_category: &str,
) {
    let edge =
        Regex::new(r"(?m)([A-Za-z_][A-Za-z0-9_.:$-]*)\s*(?:->|=>)\s*([A-Za-z_][A-Za-z0-9_.:$-]*)")
            .unwrap();
    for (index, capture) in edge.captures_iter(raw).take(MAX_DOMAIN_RECORDS).enumerate() {
        let source_id = stable_id(node_category, &capture[1]);
        let target_id = stable_id(node_category, &capture[2]);
        ensure_node(document, &source_id, &capture[1], node_category);
        ensure_node(document, &target_id, &capture[2], node_category);
        document.edges.push(VisualizationEdge::new(
            format!("explicit-edge:{index}"),
            source_id,
            target_id,
            "flows",
            edge_category,
        ));
    }
}

fn ensure_node(document: &mut VisualizationDocument, id: &str, label: &str, category: &str) {
    if !document.nodes.iter().any(|node| node.id == id) {
        document
            .nodes
            .push(VisualizationNode::new(id, label, category));
    }
}

fn first_string(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(value_label))
}

fn first_number(object: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<f64> {
    keys.iter()
        .find_map(|key| object.get(*key).and_then(number))
}

fn parse_har_trace(raw: &str, document: &mut VisualizationDocument) {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return;
    };
    let entries = value
        .pointer("/log/entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for (index, entry) in entries.iter().take(MAX_DOMAIN_RECORDS).enumerate() {
        let url = entry
            .pointer("/request/url")
            .and_then(Value::as_str)
            .unwrap_or("request");
        let host = url::Url::parse(url)
            .ok()
            .and_then(|url| url.host_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| url.to_string());
        let client_id = "endpoint:client";
        let server_id = stable_id("endpoint", &host);
        ensure_node(document, client_id, "client", "endpoint");
        ensure_node(document, &server_id, &host, "endpoint");
        document.edges.push(VisualizationEdge::new(
            format!("har:{index}"),
            client_id,
            &server_id,
            entry
                .pointer("/request/method")
                .and_then(Value::as_str)
                .unwrap_or("HTTP"),
            "request",
        ));
        let mut metadata = BTreeMap::new();
        if let Some(time) = entry.get("time").and_then(number) {
            metadata.insert("duration_ms".to_string(), json!(time));
        }
        document.events.push(VisualizationEvent {
            id: format!("har-event:{index}"),
            sequence: index,
            label: url.to_string(),
            category: "request".to_string(),
            status: entry
                .pointer("/response/status")
                .and_then(Value::as_u64)
                .map(|value| value.to_string())
                .unwrap_or_default(),
            timestamp: entry
                .get("startedDateTime")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            source_id: Some(client_id.to_string()),
            target_id: Some(server_id),
            metadata,
        });
    }
    frames_from_events(document);
}

fn split_sql_list(raw: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut quote = None::<char>;
    for (index, character) in raw.char_indices() {
        if let Some(active) = quote {
            if character == active {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' | '`' => quote = Some(character),
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                values.push(raw[start..index].to_string());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if start < raw.len() {
        values.push(raw[start..].to_string());
    }
    values
}

fn extract_xml_data_array(raw: &str, section: Option<&str>, name: Option<&str>) -> Vec<f64> {
    let scope = section
        .and_then(|section| {
            let pattern = Regex::new(&format!(r"(?s)<{section}\b[^>]*>(.*?)</{section}>")).ok()?;
            pattern
                .captures(raw)
                .and_then(|capture| capture.get(1))
                .map(|value| value.as_str())
        })
        .unwrap_or(raw);
    let array = Regex::new(r#"(?s)<DataArray\b([^>]*)>(.*?)</DataArray>"#).unwrap();
    for capture in array.captures_iter(scope) {
        let attributes = capture.get(1).unwrap().as_str();
        if let Some(name) = name {
            let expected_a = format!("Name=\"{name}\"");
            let expected_b = format!("Name='{name}'");
            if !attributes.contains(&expected_a) && !attributes.contains(&expected_b) {
                continue;
            }
        }
        if attributes.contains("format=\"binary\"")
            || attributes.contains("format='binary'")
            || attributes.contains("format=\"appended\"")
        {
            continue;
        }
        return capture[2]
            .split_whitespace()
            .filter_map(|value| value.parse().ok())
            .collect();
    }
    Vec::new()
}

fn npy_header(bytes: &[u8]) -> Option<String> {
    if bytes.len() < 10 || !bytes.starts_with(b"\x93NUMPY") {
        return None;
    }
    let major = bytes[6];
    let (header_start, header_length) = if major <= 1 {
        (
            10usize,
            u16::from_le_bytes(bytes[8..10].try_into().ok()?) as usize,
        )
    } else {
        if bytes.len() < 12 {
            return None;
        }
        (
            12usize,
            u32::from_le_bytes(bytes[8..12].try_into().ok()?) as usize,
        )
    };
    let end = header_start.checked_add(header_length)?;
    let header = bytes.get(header_start..end)?;
    Some(String::from_utf8_lossy(header).trim().to_string())
}

fn strip_xml(raw: &str) -> String {
    Regex::new(r"(?s)<[^>]+>")
        .unwrap()
        .replace_all(raw, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn retain_categories(document: &mut VisualizationDocument, categories: &[&str]) {
    let retained = document
        .nodes
        .iter()
        .filter(|node| categories.contains(&node.category.as_str()))
        .map(|node| node.id.clone())
        .collect::<HashSet<_>>();
    if retained.is_empty() {
        document.nodes.clear();
        document.edges.clear();
        return;
    }
    document.nodes.retain(|node| retained.contains(&node.id));
    document
        .edges
        .retain(|edge| retained.contains(&edge.source) && retained.contains(&edge.target));
}

fn derive_source_capabilities(
    path: &Path,
    descriptor: &DomainPluginDescriptor,
    asset: &DomainAsset,
) -> Vec<String> {
    descriptor
        .supported_visualizations
        .iter()
        .filter(|visualization| {
            supports_visualization(
                &descriptor.metadata.id,
                visualization,
                path,
                &asset.file_type,
                read_optional_text(path).as_deref(),
            )
        })
        .map(|visualization| visualization.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::research_domains::model::{
        DomainLifecycleDescriptor, DomainMetadata, DomainProviderDescriptor,
        DomainWorkbenchDescriptor,
    };
    use tempfile::tempdir;

    fn descriptor(domain_id: &str) -> DomainPluginDescriptor {
        let provider = DomainProviderDescriptor {
            id: "test".to_string(),
            api_version: "1".to_string(),
            provider_type: "test".to_string(),
        };
        DomainPluginDescriptor {
            metadata: DomainMetadata {
                id: domain_id.to_string(),
                label: domain_id.to_string(),
                description: String::new(),
                version: "1".to_string(),
                category: "test".to_string(),
            },
            capabilities: Vec::new(),
            supported_file_types: Vec::new(),
            supported_visualizations: Vec::new(),
            supported_agents: Vec::new(),
            context_provider: provider.clone(),
            preview_provider: provider.clone(),
            execution_provider: provider.clone(),
            data_provider: provider.clone(),
            visualization_provider: provider.clone(),
            render_provider: provider,
            lifecycle: DomainLifecycleDescriptor {
                states: Vec::new(),
                supports_hot_reload: false,
                supports_workspace_sync: false,
            },
            sdk_adapters: Vec::new(),
            plugin_api_version: "1".to_string(),
            workbench: DomainWorkbenchDescriptor::default(),
        }
    }

    fn asset(path: &str, file_type: &str) -> DomainAsset {
        DomainAsset {
            id: "asset".to_string(),
            source_id: format!("workspace:{path}"),
            domain_id: String::new(),
            path: path.to_string(),
            name: path.to_string(),
            file_type: file_type.to_string(),
            size_bytes: 1,
            modified_at: String::new(),
            content_revision: "revision".to_string(),
            visualizations: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    fn visualization(id: &str) -> DomainVisualizationDescriptor {
        DomainVisualizationDescriptor {
            id: id.to_string(),
            label: id.to_string(),
            renderer: "test".to_string(),
            compatible_file_types: Vec::new(),
            adapter: "test".to_string(),
            workbench_region: "primary".to_string(),
            requires_sdk: Vec::new(),
        }
    }

    #[test]
    fn stl_does_not_claim_a_feature_tree() {
        let root = tempdir().unwrap();
        let path = root.path().join("triangle.stl");
        fs::write(
            &path,
            "solid t\n facet normal 0 0 1\n  outer loop\n   vertex 0 0 0\n   vertex 1 0 0\n   vertex 0 1 0\n  endloop\n endfacet\nendsolid t\n",
        )
        .unwrap();
        assert!(!supports_visualization(
            "cad",
            &visualization("feature-tree"),
            &path,
            "stl",
            fs::read_to_string(&path).ok().as_deref(),
        ));
        assert!(supports_visualization(
            "cad",
            &visualization("parametric-model"),
            &path,
            "stl",
            fs::read_to_string(&path).ok().as_deref(),
        ));
    }

    #[test]
    fn conllu_produces_real_tokens_and_dependencies() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("sample.conllu"),
            "# text = Atlas parses data.\n1\tAtlas\tAtlas\tPROPN\t_\t_\t2\tnsubj\t_\t_\n2\tparses\tparse\tVERB\t_\t_\t0\troot\t_\t_\n3\tdata\tdata\tNOUN\t_\t_\t2\tobj\t_\t_\n4\t.\t.\tPUNCT\t_\t_\t2\tpunct\t_\t_\n",
        )
        .unwrap();
        let document = parse_registered_adapter(
            root.path(),
            &descriptor("nlp"),
            &asset("sample.conllu", "conllu"),
        )
        .unwrap()
        .unwrap();
        assert!(document.nodes.iter().any(|node| node.label == "Atlas"));
        assert!(document
            .edges
            .iter()
            .any(|edge| edge.label == "nsubj" && edge.category == "dependency"));
    }

    #[test]
    fn sql_schema_uses_declared_tables_and_columns() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("schema.sql"),
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT); CREATE INDEX users_name ON users(name);",
        )
        .unwrap();
        let document = parse_registered_adapter(
            root.path(),
            &descriptor("database"),
            &asset("schema.sql", "sql"),
        )
        .unwrap()
        .unwrap();
        assert!(document
            .nodes
            .iter()
            .any(|node| node.category == "table" && node.label == "users"));
        assert!(document
            .nodes
            .iter()
            .any(|node| node.category == "column" && node.label == "name"));
    }

    #[test]
    fn urdf_produces_kinematic_chain() {
        let root = tempdir().unwrap();
        fs::write(
            root.path().join("robot.urdf"),
            r#"<robot name="arm"><link name="base"/><link name="tool"/><joint name="wrist" type="revolute"><parent link="base"/><child link="tool"/></joint></robot>"#,
        )
        .unwrap();
        let document = parse_registered_adapter(
            root.path(),
            &descriptor("robotics"),
            &asset("robot.urdf", "urdf"),
        )
        .unwrap()
        .unwrap();
        assert!(document
            .nodes
            .iter()
            .any(|node| node.category == "robot-joint" && node.label == "wrist"));
        assert_eq!(
            document
                .edges
                .iter()
                .filter(|edge| edge.category == "kinematic-chain")
                .count(),
            2
        );
    }
}
