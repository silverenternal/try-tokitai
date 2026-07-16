use super::adapters::{probe_program_version, sdk_probe};
use super::model::DomainAsset;
use crate::task_queue::{BackgroundTask, TaskQueue};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const ACTION_RESULT_SCHEMA: &str = "atlas.domain-action-result.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DomainActionMode {
    Native,
    Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainActionParameter {
    pub id: String,
    pub label: String,
    pub value_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    #[serde(default)]
    pub choices: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DomainActionDescriptor {
    pub domain_id: String,
    pub id: String,
    pub label: String,
    pub description: String,
    pub mode: DomainActionMode,
    pub sdk: String,
    #[serde(default)]
    pub compatible_file_types: Vec<String>,
    pub asset_required: bool,
    #[serde(default)]
    pub parameters: Vec<DomainActionParameter>,
    pub output_kind: String,
    pub available: bool,
    pub ready: bool,
    pub readiness: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DomainActionRunRequest {
    pub domain_id: String,
    pub action_id: String,
    #[serde(default)]
    pub asset_id: Option<String>,
    #[serde(default)]
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct DomainActionRunResponse {
    pub descriptor: DomainActionDescriptor,
    pub task: BackgroundTask,
    pub output_path: String,
}

#[derive(Debug, Clone, Copy)]
enum InvocationKind {
    Python(&'static str),
    Program(&'static [&'static str]),
    CargoMetadata,
    NpmMetadata,
    CmakeProject,
}

#[derive(Debug, Clone, Copy)]
struct ActionTemplate {
    domain_id: &'static str,
    id: &'static str,
    label: &'static str,
    description: &'static str,
    sdk: &'static str,
    compatible: &'static [&'static str],
    asset_required: bool,
    parameters: &'static [ParameterTemplate],
    output_kind: &'static str,
    invocation: InvocationKind,
}

#[derive(Debug, Clone, Copy)]
struct ParameterTemplate {
    id: &'static str,
    label: &'static str,
    value_type: &'static str,
    required: bool,
    default: Option<&'static str>,
    choices: &'static [&'static str],
    minimum: Option<f64>,
    maximum: Option<f64>,
    description: &'static str,
}

const NO_PARAMETERS: &[ParameterTemplate] = &[];
const NLP_LANGUAGE: &[ParameterTemplate] = &[ParameterTemplate {
    id: "language",
    label: "Language code",
    value_type: "string",
    required: false,
    default: Some("xx"),
    choices: &[],
    minimum: None,
    maximum: None,
    description: "spaCy blank-pipeline language code.",
}];
const NLP_MODEL: &[ParameterTemplate] = &[ParameterTemplate {
    id: "model",
    label: "Pipeline model",
    value_type: "string",
    required: true,
    default: Some("en_core_web_sm"),
    choices: &[],
    minimum: None,
    maximum: None,
    description: "Installed spaCy model containing a parser.",
}];
const CAD_EXPORT: &[ParameterTemplate] = &[ParameterTemplate {
    id: "format",
    label: "Export format",
    value_type: "enum",
    required: true,
    default: Some("step"),
    choices: &["step", "stl"],
    minimum: None,
    maximum: None,
    description: "Geometry format written beside the action result.",
}];
const SQLITE_QUERY: &[ParameterTemplate] = &[ParameterTemplate {
    id: "query",
    label: "Read-only SQL",
    value_type: "string",
    required: true,
    default: Some("SELECT name, type FROM sqlite_schema ORDER BY type, name LIMIT 200"),
    choices: &[],
    minimum: None,
    maximum: None,
    description: "Only SELECT, WITH, EXPLAIN and PRAGMA statements are accepted.",
}];
const KUBE_NAMESPACE: &[ParameterTemplate] = &[ParameterTemplate {
    id: "namespace",
    label: "Namespace",
    value_type: "string",
    required: false,
    default: Some("all"),
    choices: &[],
    minimum: None,
    maximum: None,
    description: "Use 'all' for every namespace.",
}];

const ACTIONS: &[ActionTemplate] = &[
    action(
        "ai-ml",
        "inspect-onnx",
        "Inspect ONNX Model",
        "Read model I/O, providers and graph metadata with ONNX Runtime.",
        "ONNX Runtime",
        &["onnx"],
        true,
        NO_PARAMETERS,
        "model-inspection",
        InvocationKind::Python("inspect-onnx"),
    ),
    action(
        "ai-ml",
        "inspect-tensor",
        "Inspect Tensor",
        "Read real array shape, dtype, range and mean without loading pickle data.",
        "NumPy",
        &["npy", "npz"],
        true,
        NO_PARAMETERS,
        "tensor-profile",
        InvocationKind::Python("inspect-tensor"),
    ),
    action(
        "computer-vision",
        "inspect-image",
        "Inspect Image",
        "Decode selected media with OpenCV and report dimensions and channel statistics.",
        "OpenCV",
        &["png", "jpg", "jpeg", "bmp", "tif", "tiff", "webp"],
        true,
        NO_PARAMETERS,
        "image-profile",
        InvocationKind::Python("inspect-image"),
    ),
    action(
        "computer-vision",
        "inspect-geometry",
        "Inspect Reconstruction",
        "Validate point cloud or triangle-mesh topology using Open3D.",
        "Open3D",
        &["ply", "pcd", "xyz", "pts", "obj", "stl"],
        true,
        NO_PARAMETERS,
        "geometry-profile",
        InvocationKind::Python("inspect-geometry"),
    ),
    action(
        "nlp",
        "tokenize",
        "Tokenize Corpus",
        "Create tokens with exact source offsets using spaCy.",
        "spaCy",
        &["txt", "md", "json", "jsonl", "conll", "conllu"],
        true,
        NLP_LANGUAGE,
        "token-document",
        InvocationKind::Python("tokenize"),
    ),
    action(
        "nlp",
        "dependency-parse",
        "Dependency Parse",
        "Run an installed spaCy pipeline and persist tokens, dependency heads and entities.",
        "spaCy",
        &["txt", "md", "json", "jsonl"],
        true,
        NLP_MODEL,
        "dependency-document",
        InvocationKind::Python("dependency-parse"),
    ),
    action(
        "computer-graphics",
        "validate-mesh",
        "Validate Mesh",
        "Inspect mesh topology, bounds, watertightness and orientability.",
        "Open3D",
        &["obj", "ply", "stl", "off", "gltf", "glb"],
        true,
        NO_PARAMETERS,
        "mesh-quality",
        InvocationKind::Python("inspect-geometry"),
    ),
    action(
        "computer-graphics",
        "render-scene",
        "Render Scene",
        "Invoke Blender headlessly against the selected .blend scene.",
        "Blender Python API",
        &["blend"],
        true,
        NO_PARAMETERS,
        "render",
        InvocationKind::Program(&["-b", "{asset}", "-o", "{output_stem}", "-f", "1"]),
    ),
    action(
        "cad",
        "recompute",
        "Recompute Model",
        "Open and recompute the selected native FreeCAD document.",
        "FreeCAD",
        &["fcstd"],
        true,
        NO_PARAMETERS,
        "cad-recompute",
        InvocationKind::Python("freecad-recompute"),
    ),
    action(
        "cad",
        "export",
        "Export Geometry",
        "Export a FreeCAD document to STEP or STL with a non-interactive script.",
        "FreeCAD",
        &["fcstd"],
        true,
        CAD_EXPORT,
        "cad-export",
        InvocationKind::Python("freecad-export"),
    ),
    action(
        "robotics",
        "bag-info",
        "Inspect ROS Bag",
        "Read bag metadata through the installed ROS 2 runtime.",
        "ROS 2",
        &["bag", "db3", "mcap"],
        true,
        NO_PARAMETERS,
        "bag-metadata",
        InvocationKind::Program(&["bag", "info", "{asset}"]),
    ),
    action(
        "robotics",
        "validate-urdf",
        "Validate Robot Model",
        "Parse a URDF with the ROS check_urdf utility.",
        "check_urdf",
        &["urdf"],
        true,
        NO_PARAMETERS,
        "robot-model-validation",
        InvocationKind::Program(&["{asset}"]),
    ),
    action(
        "computer-networks",
        "decode-packets",
        "Decode Packets",
        "Decode packet records using tshark without mutating the capture.",
        "Wireshark",
        &["pcap", "pcapng"],
        true,
        NO_PARAMETERS,
        "packet-table",
        InvocationKind::Program(&["-r", "{asset}", "-T", "json"]),
    ),
    action(
        "computer-networks",
        "protocol-hierarchy",
        "Protocol Hierarchy",
        "Compute tshark protocol hierarchy statistics from the selected capture.",
        "Wireshark",
        &["pcap", "pcapng"],
        true,
        NO_PARAMETERS,
        "protocol-hierarchy",
        InvocationKind::Program(&["-r", "{asset}", "-q", "-z", "io,phs"]),
    ),
    action(
        "operating-systems",
        "trace-info",
        "Inspect ETW Trace",
        "Read trace metadata using Windows Performance Analyzer tooling.",
        "Windows ETW",
        &["etl"],
        true,
        NO_PARAMETERS,
        "trace-metadata",
        InvocationKind::Program(&["-i", "{asset}", "-a", "systeminfo"]),
    ),
    action(
        "operating-systems",
        "perf-report",
        "Inspect perf.data",
        "Read Linux perf report data in stdio mode.",
        "Linux perf",
        &["perf", "data"],
        true,
        NO_PARAMETERS,
        "perf-report",
        InvocationKind::Program(&["report", "--stdio", "-i", "{asset}"]),
    ),
    action(
        "compiler",
        "build-ast",
        "Build AST",
        "Dump the Clang AST for the selected translation unit.",
        "Clang",
        &["c", "cc", "cpp", "cxx", "h", "hpp", "m", "mm"],
        true,
        NO_PARAMETERS,
        "ast",
        InvocationKind::Program(&["-Xclang", "-ast-dump", "-fsyntax-only", "{asset}"]),
    ),
    action(
        "compiler",
        "emit-ir",
        "Emit LLVM IR",
        "Emit optimized-independent textual LLVM IR for the selected source.",
        "Clang",
        &["c", "cc", "cpp", "cxx", "m", "mm"],
        true,
        NO_PARAMETERS,
        "llvm-ir",
        InvocationKind::Program(&["-S", "-emit-llvm", "{asset}", "-o", "-"]),
    ),
    action(
        "database",
        "sqlite-schema",
        "Inspect SQLite Schema",
        "Read tables, views, indexes and triggers from a SQLite database.",
        "SQLite",
        &["sqlite", "sqlite3", "db"],
        true,
        NO_PARAMETERS,
        "database-schema",
        InvocationKind::Program(&["{asset}", ".schema"]),
    ),
    action(
        "database",
        "sqlite-query",
        "Run Read-only Query",
        "Execute a bounded read-only SQLite query against the selected database.",
        "SQLite",
        &["sqlite", "sqlite3", "db"],
        true,
        SQLITE_QUERY,
        "query-result",
        InvocationKind::Program(&["-json", "{asset}", "{param:query}"]),
    ),
    action(
        "software-engineering",
        "cargo-metadata",
        "Cargo Metadata",
        "Resolve repository packages, targets and dependencies without compiling.",
        "Cargo",
        &[],
        false,
        NO_PARAMETERS,
        "project-metadata",
        InvocationKind::CargoMetadata,
    ),
    action(
        "software-engineering",
        "npm-metadata",
        "npm Metadata",
        "Read the current Node package and dependency tree.",
        "npm",
        &[],
        false,
        NO_PARAMETERS,
        "project-metadata",
        InvocationKind::NpmMetadata,
    ),
    action(
        "program-analysis",
        "semgrep-scan",
        "Run Static Analysis",
        "Scan the selected source with Semgrep's maintained auto configuration.",
        "Semgrep",
        &[
            "py", "js", "jsx", "ts", "tsx", "java", "go", "rs", "c", "cpp", "cs", "rb", "php",
        ],
        true,
        NO_PARAMETERS,
        "findings",
        InvocationKind::Program(&["scan", "--config", "auto", "--json", "{asset}"]),
    ),
    action(
        "program-analysis",
        "clang-cfg",
        "Analyze Control Flow",
        "Run Clang static analyzer and retain diagnostics for the selected translation unit.",
        "Clang",
        &["c", "cc", "cpp", "cxx", "m", "mm"],
        true,
        NO_PARAMETERS,
        "analysis-diagnostics",
        InvocationKind::Program(&["--analyze", "{asset}"]),
    ),
    action(
        "cyber-security",
        "semgrep-scan",
        "Run Security Scan",
        "Run Semgrep security rules against a selected source artifact.",
        "Semgrep",
        &[
            "py", "js", "jsx", "ts", "tsx", "java", "go", "rs", "c", "cpp", "cs", "rb", "php",
        ],
        true,
        NO_PARAMETERS,
        "security-findings",
        InvocationKind::Program(&["scan", "--config", "auto", "--json", "{asset}"]),
    ),
    action(
        "cyber-security",
        "yara-scan",
        "Run YARA Scan",
        "Apply a selected workspace YARA rule file to a target artifact.",
        "YARA",
        &["yar", "yara"],
        true,
        NO_PARAMETERS,
        "malware-findings",
        InvocationKind::Program(&["{asset}", "."]),
    ),
    action(
        "hpc",
        "nsys-stats",
        "Nsight Systems Stats",
        "Export profiler summary statistics from an existing Nsight report.",
        "Nsight Systems",
        &["nsys-rep", "qdrep"],
        true,
        NO_PARAMETERS,
        "gpu-timeline-summary",
        InvocationKind::Program(&["stats", "{asset}"]),
    ),
    action(
        "hpc",
        "ncu-import",
        "Nsight Compute Report",
        "Import and print the selected Nsight Compute report.",
        "Nsight Compute",
        &["ncu-rep"],
        true,
        NO_PARAMETERS,
        "kernel-profile",
        InvocationKind::Program(&["--import", "{asset}", "--page", "details"]),
    ),
    action(
        "distributed-systems",
        "cluster-snapshot",
        "Cluster Snapshot",
        "Read live Kubernetes workloads, nodes and services from the current context.",
        "Kubernetes",
        &[],
        false,
        KUBE_NAMESPACE,
        "cluster-snapshot",
        InvocationKind::Program(&[
            "get",
            "nodes,pods,services,deployments,statefulsets",
            "{namespace_args}",
            "-o",
            "json",
        ]),
    ),
    action(
        "distributed-systems",
        "cluster-events",
        "Cluster Events",
        "Read live Kubernetes events ordered by last timestamp.",
        "Kubernetes",
        &[],
        false,
        KUBE_NAMESPACE,
        "cluster-events",
        InvocationKind::Program(&[
            "get",
            "events",
            "{namespace_args}",
            "--sort-by=.lastTimestamp",
            "-o",
            "json",
        ]),
    ),
    action(
        "scientific-computing",
        "inspect-array",
        "Inspect Array",
        "Read numerical array dimensions, dtype and finite-value range using NumPy.",
        "NumPy",
        &["npy", "npz"],
        true,
        NO_PARAMETERS,
        "array-profile",
        InvocationKind::Python("inspect-array"),
    ),
    action(
        "scientific-computing",
        "inspect-vtk",
        "Inspect VTK Dataset",
        "Read mesh topology, bounds and field arrays through VTK.",
        "VTK",
        &["vtk", "vtu", "vtp"],
        true,
        NO_PARAMETERS,
        "mesh-profile",
        InvocationKind::Python("inspect-vtk"),
    ),
    action(
        "scientific-computing",
        "cmake-project",
        "Inspect Solver Project",
        "Inspect the configured CMake project and its build targets.",
        "CMake",
        &[],
        false,
        NO_PARAMETERS,
        "solver-project",
        InvocationKind::CmakeProject,
    ),
];

const fn action(
    domain_id: &'static str,
    id: &'static str,
    label: &'static str,
    description: &'static str,
    sdk: &'static str,
    compatible: &'static [&'static str],
    asset_required: bool,
    parameters: &'static [ParameterTemplate],
    output_kind: &'static str,
    invocation: InvocationKind,
) -> ActionTemplate {
    ActionTemplate {
        domain_id,
        id,
        label,
        description,
        sdk,
        compatible,
        asset_required,
        parameters,
        output_kind,
        invocation,
    }
}

pub fn list_actions(
    workspace_root: &Path,
    domain_id: &str,
    assets: &[DomainAsset],
    asset_id: Option<&str>,
) -> Result<Vec<DomainActionDescriptor>> {
    validate_domain_id(domain_id)?;
    let selected = match asset_id.filter(|value| !value.trim().is_empty()) {
        Some(id) => Some(find_asset(assets, id)?),
        None => None,
    };
    Ok(ACTIONS
        .iter()
        .filter(|action| action.domain_id == domain_id)
        .map(|action| descriptor(workspace_root, action, selected))
        .collect())
}

pub fn run_action(
    workspace_root: &Path,
    queue: &TaskQueue,
    assets: &[DomainAsset],
    request: &DomainActionRunRequest,
) -> Result<DomainActionRunResponse> {
    validate_domain_id(&request.domain_id)?;
    let template = ACTIONS
        .iter()
        .find(|action| action.domain_id == request.domain_id && action.id == request.action_id)
        .ok_or_else(|| anyhow!("unknown or unauthorized domain action"))?;
    let asset = match request
        .asset_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        Some(id) => Some(find_asset(assets, id)?),
        None => None,
    };
    let descriptor = descriptor(workspace_root, template, asset);
    if !descriptor.ready {
        return Err(anyhow!(descriptor.reason.clone()));
    }
    let parameters = validate_parameters(template.parameters, &request.parameters)?;
    let run_id = uuid::Uuid::new_v4().to_string();
    let output_relative = format!(
        ".atlas/domain-actions/{}/{}/{}/result.json",
        request.domain_id, template.output_kind, run_id
    );
    let output_absolute = workspace_root.join(&output_relative);
    fs::create_dir_all(output_absolute.parent().unwrap())?;
    let (program, args) = build_invocation(
        workspace_root,
        template,
        asset,
        &parameters,
        &output_absolute,
    )?;
    let task = queue.enqueue_process(
        template.label,
        &format!("domain:{}:{}", request.domain_id, request.action_id),
        &program,
        &args,
        None,
        true,
        Some(output_relative.as_str()),
    )?;
    let persisted_output = output_relative.clone();
    let manifest = json!({
        "schema_version": ACTION_RESULT_SCHEMA,
        "domain_id": request.domain_id,
        "action_id": request.action_id,
        "task_id": task.id,
        "asset_id": asset.map(|value| value.id.as_str()).unwrap_or(""),
        "asset_path": asset.map(|value| value.path.as_str()).unwrap_or(""),
        "parameters": parameters,
        "sdk": descriptor.sdk,
        "sdk_version": descriptor.version,
        "created_at": Utc::now().to_rfc3339(),
        "result_path": persisted_output,
        "status": "running"
    });
    fs::write(
        output_absolute.with_file_name("run.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(DomainActionRunResponse {
        descriptor,
        task,
        output_path: persisted_output.replace('\\', "/"),
    })
}

fn descriptor(
    workspace_root: &Path,
    template: &ActionTemplate,
    asset: Option<&DomainAsset>,
) -> DomainActionDescriptor {
    let probe = action_probe(template);
    let mut ready = probe.available;
    let mut readiness = if probe.available {
        "ready"
    } else {
        "sdk_unavailable"
    }
    .to_string();
    let mut reason = probe.reason.clone();
    if template.asset_required && asset.is_none() {
        ready = false;
        readiness = "asset_required".into();
        reason = "Select a compatible real workspace asset before running this action.".into();
    } else if let Some(asset) = asset {
        if !template.compatible.is_empty()
            && !template
                .compatible
                .iter()
                .any(|kind| kind.eq_ignore_ascii_case(&asset.file_type))
        {
            ready = false;
            readiness = "incompatible_asset".into();
            reason = format!(
                "Selected .{} asset is incompatible; expected {}.",
                asset.file_type,
                template
                    .compatible
                    .iter()
                    .map(|value| format!(".{value}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        } else if checked_asset_path(workspace_root, asset).is_err() {
            ready = false;
            readiness = "asset_unavailable".into();
            reason =
                "Selected asset no longer resolves to a real file inside the workspace.".into();
        }
    }
    if probe.available && !template.asset_required && !workspace_ready(template, workspace_root) {
        ready = false;
        readiness = "project_not_detected".into();
        reason = project_reason(template).into();
    }
    DomainActionDescriptor {
        domain_id: template.domain_id.into(),
        id: template.id.into(),
        label: template.label.into(),
        description: template.description.into(),
        mode: DomainActionMode::Native,
        sdk: template.sdk.into(),
        compatible_file_types: template
            .compatible
            .iter()
            .map(|value| (*value).into())
            .collect(),
        asset_required: template.asset_required,
        parameters: template
            .parameters
            .iter()
            .map(parameter_descriptor)
            .collect(),
        output_kind: template.output_kind.into(),
        available: probe.available,
        ready,
        readiness,
        reason,
        executable: probe.executable,
        version: probe.version,
    }
}

fn action_probe(template: &ActionTemplate) -> super::adapters::SdkProbe {
    let required_programs: Option<&[&str]> = match (template.domain_id, template.id) {
        ("cad", _) => Some(&["FreeCADCmd", "freecadcmd"]),
        ("robotics", "validate-urdf") => Some(&["check_urdf"]),
        ("computer-networks", _) => Some(&["tshark"]),
        ("operating-systems", "trace-info") => Some(&["xperf"]),
        ("compiler", _) | ("program-analysis", "clang-cfg") => Some(&["clang"]),
        ("scientific-computing", "cmake-project") => Some(&["cmake"]),
        _ => None,
    };
    if let Some(candidates) = required_programs {
        return explicit_program_probe(template.sdk, candidates);
    }
    sdk_probe(template.sdk)
}

fn explicit_program_probe(sdk: &str, candidates: &[&str]) -> super::adapters::SdkProbe {
    let detected = candidates
        .iter()
        .find_map(|candidate| which::which(candidate).ok());
    match detected {
        Some(program) => super::adapters::SdkProbe {
            sdk: sdk.into(),
            status: "available".into(),
            available: true,
            executable: Some(program.to_string_lossy().to_string()),
            version: probe_program_version(&program),
            candidates: candidates.iter().map(|value| (*value).into()).collect(),
            execution_enabled: true,
            reason: "Executable detected on PATH".into(),
        },
        None => super::adapters::SdkProbe {
            sdk: sdk.into(),
            status: "unavailable".into(),
            available: false,
            executable: None,
            version: None,
            candidates: candidates.iter().map(|value| (*value).into()).collect(),
            execution_enabled: false,
            reason: format!(
                "No supported executable detected ({})",
                candidates.join(", ")
            ),
        },
    }
}

fn workspace_ready(template: &ActionTemplate, root: &Path) -> bool {
    match template.invocation {
        InvocationKind::CargoMetadata => root.join("Cargo.toml").is_file(),
        InvocationKind::NpmMetadata => root.join("package.json").is_file(),
        InvocationKind::CmakeProject => root.join("CMakeLists.txt").is_file(),
        _ => true,
    }
}

fn project_reason(template: &ActionTemplate) -> &'static str {
    match template.invocation {
        InvocationKind::CargoMetadata => "Cargo.toml was not found at the workspace root.",
        InvocationKind::NpmMetadata => "package.json was not found at the workspace root.",
        InvocationKind::CmakeProject => "CMakeLists.txt was not found at the workspace root.",
        _ => "Required project entry point was not found.",
    }
}

fn build_invocation(
    workspace_root: &Path,
    template: &ActionTemplate,
    asset: Option<&DomainAsset>,
    parameters: &Map<String, Value>,
    output: &Path,
) -> Result<(String, Vec<String>)> {
    let probe = action_probe(template);
    let program = probe.executable.ok_or_else(|| anyhow!(probe.reason))?;
    let asset_path = asset
        .map(|value| checked_asset_path(workspace_root, value))
        .transpose()?;
    match template.invocation {
        InvocationKind::Python(action) => {
            let script = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/research_domains/python_runtime.py");
            let mut runtime_parameters = parameters.clone();
            if action == "freecad-export" {
                let extension = parameters
                    .get("format")
                    .and_then(Value::as_str)
                    .unwrap_or("step");
                runtime_parameters.insert(
                    "_export_path".into(),
                    json!(output.with_file_name(format!("model.{extension}"))),
                );
            }
            Ok((
                program,
                vec![
                    script.to_string_lossy().to_string(),
                    action.into(),
                    "--asset".into(),
                    asset_path
                        .ok_or_else(|| anyhow!("action requires an asset"))?
                        .to_string_lossy()
                        .to_string(),
                    "--output".into(),
                    output.to_string_lossy().to_string(),
                    "--parameters".into(),
                    serde_json::to_string(&runtime_parameters)?,
                ],
            ))
        }
        InvocationKind::Program(arguments) => {
            let asset = asset_path
                .as_ref()
                .map(|value| value.to_string_lossy().to_string())
                .unwrap_or_default();
            let output_stem = output
                .with_file_name("frame-####")
                .to_string_lossy()
                .to_string();
            let mut args = Vec::new();
            for argument in arguments {
                if *argument == "{namespace_args}" {
                    let namespace = parameters
                        .get("namespace")
                        .and_then(Value::as_str)
                        .unwrap_or("all");
                    if namespace.eq_ignore_ascii_case("all") || namespace.trim().is_empty() {
                        args.push("--all-namespaces".into());
                    } else {
                        args.extend(["--namespace".into(), namespace.into()]);
                    }
                } else if let Some(key) = argument
                    .strip_prefix("{param:")
                    .and_then(|value| value.strip_suffix('}'))
                {
                    args.push(value_to_arg(
                        parameters
                            .get(key)
                            .ok_or_else(|| anyhow!("missing action parameter: {key}"))?,
                    )?);
                } else {
                    args.push(
                        argument
                            .replace("{asset}", &asset)
                            .replace("{output}", &output.to_string_lossy())
                            .replace("{output_stem}", &output_stem),
                    );
                }
            }
            Ok((program, args))
        }
        InvocationKind::CargoMetadata => Ok((
            program,
            vec!["metadata".into(), "--format-version".into(), "1".into()],
        )),
        InvocationKind::NpmMetadata => {
            Ok((program, vec!["ls".into(), "--all".into(), "--json".into()]))
        }
        InvocationKind::CmakeProject => Ok((
            program,
            vec![
                "-S".into(),
                ".".into(),
                "-B".into(),
                output
                    .with_file_name("cmake-build")
                    .to_string_lossy()
                    .to_string(),
                "-N".into(),
            ],
        )),
    }
}

fn validate_parameters(schema: &[ParameterTemplate], raw: &Value) -> Result<Map<String, Value>> {
    let supplied = if raw.is_null() {
        Map::new()
    } else {
        raw.as_object()
            .cloned()
            .ok_or_else(|| anyhow!("action parameters must be an object"))?
    };
    if let Some(key) = supplied
        .keys()
        .find(|key| !schema.iter().any(|parameter| parameter.id == key.as_str()))
    {
        return Err(anyhow!("unknown action parameter: {key}"));
    }
    let mut validated = Map::new();
    for parameter in schema {
        let value = supplied
            .get(parameter.id)
            .cloned()
            .or_else(|| parameter.default.map(Value::from));
        if parameter.required
            && value
                .as_ref()
                .is_none_or(|value| value.as_str().is_some_and(str::is_empty))
        {
            return Err(anyhow!(
                "missing required action parameter: {}",
                parameter.id
            ));
        }
        let Some(value) = value else { continue };
        match parameter.value_type {
            "string" | "enum" if !value.is_string() => {
                return Err(anyhow!("{} must be a string", parameter.id))
            }
            "number" if !value.is_number() => {
                return Err(anyhow!("{} must be a number", parameter.id))
            }
            "boolean" if !value.is_boolean() => {
                return Err(anyhow!("{} must be a boolean", parameter.id))
            }
            _ => {}
        }
        if !parameter.choices.is_empty()
            && !parameter
                .choices
                .iter()
                .any(|choice| value.as_str() == Some(choice))
        {
            return Err(anyhow!(
                "{} must be one of {}",
                parameter.id,
                parameter.choices.join(", ")
            ));
        }
        if parameter.id == "query" {
            validate_read_only_sql(value.as_str().unwrap_or_default())?;
        }
        validated.insert(parameter.id.into(), value);
    }
    Ok(validated)
}

fn validate_read_only_sql(query: &str) -> Result<()> {
    let normalized = query.trim().to_ascii_lowercase();
    if query.len() > 32_000
        || !["select", "with", "explain", "pragma"]
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
    {
        return Err(anyhow!("only bounded read-only SQL is accepted"));
    }
    if [
        " insert ",
        " update ",
        " delete ",
        " drop ",
        " alter ",
        " create ",
        " attach ",
        " detach ",
        " vacuum ",
        " replace ",
    ]
    .iter()
    .any(|token| format!(" {normalized} ").contains(token))
    {
        return Err(anyhow!("mutating SQL is not allowed by this action"));
    }
    Ok(())
}

fn value_to_arg(value: &Value) -> Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        Value::Bool(value) => Ok(value.to_string()),
        _ => Err(anyhow!(
            "action parameter cannot be converted to an argument"
        )),
    }
}

fn parameter_descriptor(parameter: &ParameterTemplate) -> DomainActionParameter {
    DomainActionParameter {
        id: parameter.id.into(),
        label: parameter.label.into(),
        value_type: parameter.value_type.into(),
        required: parameter.required,
        default: parameter.default.map(Value::from),
        choices: parameter
            .choices
            .iter()
            .map(|value| (*value).into())
            .collect(),
        minimum: parameter.minimum,
        maximum: parameter.maximum,
        description: parameter.description.into(),
    }
}

fn find_asset<'a>(assets: &'a [DomainAsset], id: &str) -> Result<&'a DomainAsset> {
    assets
        .iter()
        .find(|asset| asset.id == id || asset.source_id == id || asset.path == id)
        .ok_or_else(|| anyhow!("domain asset is no longer available: {id}"))
}

fn checked_asset_path(workspace_root: &Path, asset: &DomainAsset) -> Result<PathBuf> {
    let root = workspace_root
        .canonicalize()
        .context("workspace root is unavailable")?;
    let candidate = root
        .join(&asset.path)
        .canonicalize()
        .context("domain asset is unavailable")?;
    if !candidate.starts_with(&root) || !candidate.is_file() {
        return Err(anyhow!(
            "domain asset must be a real file inside the workspace"
        ));
    }
    Ok(candidate)
}

fn validate_domain_id(domain_id: &str) -> Result<()> {
    if domain_id.is_empty()
        || domain_id.len() > 80
        || !domain_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_'))
    {
        return Err(anyhow!("invalid research domain id"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_mutating_sql() {
        assert!(validate_read_only_sql("select * from runs").is_ok());
        assert!(validate_read_only_sql("delete from runs").is_err());
        assert!(validate_read_only_sql("with x as (select 1) delete from runs").is_err());
    }

    #[test]
    fn all_domains_have_native_actions() {
        let expected = [
            "ai-ml",
            "computer-vision",
            "nlp",
            "computer-graphics",
            "cad",
            "robotics",
            "computer-networks",
            "operating-systems",
            "compiler",
            "database",
            "software-engineering",
            "program-analysis",
            "cyber-security",
            "hpc",
            "distributed-systems",
            "scientific-computing",
        ];
        for domain in expected {
            assert!(
                ACTIONS.iter().any(|action| action.domain_id == domain),
                "{domain} has no native action"
            );
        }
    }
}
