use super::util::{ext, read_source, selected_path, source_for_path, stable_id, workspace_files};
use crate::visualization::model::{
    VisualizationDiagnostic, VisualizationDocument, VisualizationEdge, VisualizationEvent,
    VisualizationFrame, VisualizationNode, VisualizationPoint, VisualizationSeries,
    VisualizationSource, VisualizationTypeDescriptor,
};
use crate::visualization::{type_descriptor, VisualizationAdapter, VisualizationContext};
use anyhow::Result;
use regex::Regex;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

pub struct AlgorithmAdapter;

impl VisualizationAdapter for AlgorithmAdapter {
    fn descriptor(&self) -> VisualizationTypeDescriptor {
        type_descriptor(
            "algorithm",
            "Algorithm",
            "Code flow, model structure, and training metrics parsed from workspace artifacts.",
            "atlas.algorithm.workspace",
        )
    }

    fn discover(&self, context: &VisualizationContext<'_>) -> Result<Vec<VisualizationSource>> {
        let mut paths = workspace_files(
            context.workspace_root,
            |path| {
                matches!(
                    ext(path).as_str(),
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
                ) && looks_algorithmic(path)
            },
            2_000,
        );
        paths.sort_by_key(|path| std::cmp::Reverse(algorithm_source_priority(path)));
        paths.truncate(120);
        Ok(paths
            .iter()
            .map(|path| {
                source_for_path(
                    context.workspace_root,
                    path,
                    "algorithm",
                    algorithm_source_type(path),
                )
            })
            .collect())
    }

    fn parse(&self, context: &VisualizationContext<'_>) -> Result<VisualizationDocument> {
        let path = selected_path(context.workspace_root, context.source_id)?;
        let source = source_for_path(
            context.workspace_root,
            &path,
            "algorithm",
            algorithm_source_type(&path),
        );
        let mut document = VisualizationDocument::empty(
            "algorithm",
            path.file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("Algorithm"),
            source,
        );
        let extension = ext(&path);
        if extension == "csv" || extension == "jsonl" {
            parse_training_metrics(&path, &mut document)?;
        } else if extension == "json" {
            let raw = read_source(&path)?;
            parse_algorithm_json(&raw, &mut document)?;
        } else if extension == "onnx" {
            parse_onnx_graph(&path, &mut document)?;
        } else if extension == "safetensors" {
            parse_safetensors_metadata(&path, &mut document)?;
        } else {
            parse_code(&read_source(&path)?, extension.as_str(), &mut document);
        }
        finalize_algorithm_frames(&mut document);
        if document.nodes.is_empty() && document.series.is_empty() {
            document.diagnostics.push(VisualizationDiagnostic {
                level: "info".to_string(),
                message:
                    "No structural symbols or numeric training series were found in this source."
                        .to_string(),
                metadata: BTreeMap::new(),
            });
        }
        Ok(document)
    }
}

fn algorithm_source_priority(path: &Path) -> u8 {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match ext(path).as_str() {
        "onnx" => 100,
        "safetensors" => 95,
        "csv" | "jsonl" if name.contains("train") || name.contains("metric") => 90,
        "json" if name.contains("model") || name.contains("graph") => 85,
        _ if [
            "model",
            "train",
            "algorithm",
            "tensor",
            "cnn",
            "transformer",
        ]
        .iter()
        .any(|needle| name.contains(needle)) =>
        {
            80
        }
        _ => 10,
    }
}

fn looks_algorithmic(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let extension = ext(path);
    matches!(extension.as_str(), "onnx" | "safetensors")
        || [
            "model",
            "train",
            "algorithm",
            "network",
            "layer",
            "metric",
            "loss",
            "trace",
            "graph",
        ]
        .iter()
        .any(|needle| name.contains(needle))
        || matches!(
            extension.as_str(),
            "py" | "rs" | "js" | "ts" | "tsx" | "jsx" | "java" | "cpp" | "c" | "go"
        )
}

fn algorithm_source_type(path: &Path) -> &'static str {
    match ext(path).as_str() {
        "csv" | "jsonl" => "training-log",
        "onnx" | "safetensors" => "model-metadata",
        "json" => "model-or-trace",
        _ => "source-code",
    }
}

fn parse_code(raw: &str, language: &str, document: &mut VisualizationDocument) {
    let symbol_pattern = Regex::new(
        r"(?m)^\s*(?:pub\s+|async\s+|export\s+|static\s+|final\s+)*(?:fn|def|function|class|struct|enum|interface|trait)\s+([A-Za-z_$][\w$]*)",
    )
    .unwrap();
    let arrow_pattern = Regex::new(
        r"(?m)^\s*(?:const|let|var)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:async\s*)?\([^)]*\)\s*=>",
    )
    .unwrap();
    let call_pattern = Regex::new(r"\b([A-Za-z_$][\w$]*)\s*\(").unwrap();
    let keywords: HashSet<&str> = [
        "if", "for", "while", "match", "switch", "return", "Some", "Ok", "Err", "print", "println",
        "format", "new", "catch",
    ]
    .into_iter()
    .collect();
    let mut symbols = Vec::<(String, usize)>::new();
    for (line_index, line) in raw.lines().enumerate() {
        if let Some(capture) = symbol_pattern
            .captures(line)
            .or_else(|| arrow_pattern.captures(line))
        {
            if let Some(name) = capture.get(1) {
                symbols.push((name.as_str().to_string(), line_index + 1));
            }
        }
    }
    symbols.truncate(240);
    let ids = symbols
        .iter()
        .map(|(name, _)| (name.clone(), stable_id("symbol", name)))
        .collect::<HashMap<_, _>>();
    for (name, line) in &symbols {
        let mut node = VisualizationNode::new(ids[name].clone(), name, "symbol");
        node.metadata.insert("line".to_string(), Value::from(*line));
        node.metadata
            .insert("language".to_string(), Value::String(language.to_string()));
        document.nodes.push(node);
    }
    let lines = raw.lines().collect::<Vec<_>>();
    for (index, (name, start_line)) in symbols.iter().enumerate() {
        let end_line = symbols
            .get(index + 1)
            .map(|(_, line)| *line)
            .unwrap_or(lines.len() + 1);
        let body = lines[start_line.saturating_sub(1)..end_line.saturating_sub(1).min(lines.len())]
            .join("\n");
        let mut seen = HashSet::new();
        for capture in call_pattern.captures_iter(&body) {
            let called = capture
                .get(1)
                .map(|value| value.as_str())
                .unwrap_or_default();
            if called == name || keywords.contains(called) || !seen.insert(called.to_string()) {
                continue;
            }
            if let Some(target_id) = ids.get(called) {
                let edge_id = format!("call:{}:{}", ids[name], target_id);
                document.edges.push(VisualizationEdge::new(
                    edge_id,
                    ids[name].clone(),
                    target_id.clone(),
                    "calls",
                    "call",
                ));
            }
        }
    }
    for (sequence, (name, _)) in symbols.iter().enumerate() {
        document.frames.push(VisualizationFrame {
            id: format!("symbol-frame-{sequence}"),
            sequence,
            label: name.clone(),
            active_nodes: vec![ids[name].clone()],
            active_edges: document
                .edges
                .iter()
                .filter(|edge| edge.source == ids[name])
                .map(|edge| edge.id.clone())
                .collect(),
            metrics: BTreeMap::new(),
        });
    }
    parse_model_layers_from_code(raw, document);
}

fn parse_model_layers_from_code(raw: &str, document: &mut VisualizationDocument) {
    let assignment = Regex::new(
        r"(?m)^\s*(?:self\.|this\.)([A-Za-z_$][\w$]*)\s*=\s*([A-Za-z_$][\w$]*(?:(?:\.|::)[A-Za-z_$][\w$]*)*)\s*\(([^\n]*)",
    )
    .unwrap();
    let forward_call = Regex::new(r"(?:self\.|this\.)([A-Za-z_$][\w$]*)\s*\(").unwrap();
    let mut layer_ids = HashMap::new();
    for (index, capture) in assignment.captures_iter(raw).take(500).enumerate() {
        let name = capture.get(1).unwrap().as_str();
        let constructor = capture.get(2).unwrap().as_str();
        let arguments = capture
            .get(3)
            .map(|value| value.as_str().trim())
            .unwrap_or_default();
        let id = format!("model-layer:{index}:{name}");
        let mut node = VisualizationNode::new(&id, name, constructor);
        node.metadata.insert(
            "constructor".to_string(),
            Value::String(constructor.to_string()),
        );
        node.metadata.insert(
            "presentation".to_string(),
            Value::String("neural-layer".to_string()),
        );
        node.metadata.insert(
            "arguments".to_string(),
            Value::String(arguments.chars().take(400).collect()),
        );
        document.nodes.push(node);
        layer_ids.insert(name.to_string(), id);
    }
    let mut previous: Option<String> = None;
    let mut sequence = 0usize;
    for capture in forward_call.captures_iter(raw) {
        let Some(current) = capture
            .get(1)
            .and_then(|name| layer_ids.get(name.as_str()))
            .cloned()
        else {
            continue;
        };
        if previous.as_deref() != Some(current.as_str()) {
            if let Some(previous_id) = previous.as_ref() {
                let edge_id = format!("model-forward:{sequence}");
                document.edges.push(VisualizationEdge::new(
                    &edge_id,
                    previous_id,
                    &current,
                    "forward",
                    "model-flow",
                ));
                document.frames.push(VisualizationFrame {
                    id: format!("model-propagation:{sequence}"),
                    sequence: document.frames.len(),
                    label: format!("{} 鈫?{}", previous_id, current),
                    active_nodes: vec![previous_id.clone(), current.clone()],
                    active_edges: vec![edge_id],
                    metrics: BTreeMap::new(),
                });
                sequence += 1;
            } else {
                document.frames.push(VisualizationFrame {
                    id: "model-propagation:input".to_string(),
                    sequence: document.frames.len(),
                    label: format!("Input 鈫?{current}"),
                    active_nodes: vec![current.clone()],
                    active_edges: Vec::new(),
                    metrics: BTreeMap::new(),
                });
            }
            previous = Some(current);
        }
    }
}

fn parse_algorithm_json(raw: &str, document: &mut VisualizationDocument) -> Result<()> {
    let value: Value = serde_json::from_str(raw)?;
    if parse_model_value(&value, document).is_ok() {
        return Ok(());
    }
    let records = value
        .as_array()
        .cloned()
        .or_else(|| value.get("events").and_then(Value::as_array).cloned())
        .or_else(|| value.get("steps").and_then(Value::as_array).cloned())
        .or_else(|| value.get("trace").and_then(Value::as_array).cloned())
        .or_else(|| value.get("history").and_then(Value::as_array).cloned())
        .ok_or_else(|| anyhow::anyhow!("JSON has no model graph or execution record list"))?;
    parse_algorithm_records(&records, document);
    document
        .metadata
        .insert("raw_trace_metadata".to_string(), value);
    Ok(())
}

fn parse_model_value(value: &Value, document: &mut VisualizationDocument) -> Result<()> {
    let layers = value
        .get("layers")
        .and_then(Value::as_array)
        .or_else(|| value.pointer("/model/layers").and_then(Value::as_array))
        .or_else(|| value.pointer("/config/layers").and_then(Value::as_array))
        .or_else(|| value.get("nodes").and_then(Value::as_array))
        .or_else(|| value.pointer("/graph/nodes").and_then(Value::as_array))
        .or_else(|| value.pointer("/graph/node").and_then(Value::as_array));
    let Some(layers) = layers else {
        anyhow::bail!("JSON has no model layer list")
    };
    let mut aliases = HashMap::<String, String>::new();
    for (index, layer) in layers.iter().enumerate() {
        let label = layer
            .get("name")
            .or_else(|| layer.get("type"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("Layer {}", index + 1));
        let id = layer
            .get("id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("layer:{index}"));
        let category = layer.get("type").and_then(Value::as_str).unwrap_or("layer");
        let mut node = VisualizationNode::new(id.clone(), label, category);
        node.metadata.insert(
            "presentation".to_string(),
            Value::String("neural-layer".to_string()),
        );
        node.metadata
            .insert("index".to_string(), Value::from(index));
        node.metadata.insert("raw".to_string(), layer.clone());
        document.nodes.push(node);
        aliases.insert(id.clone(), id.clone());
        if let Some(name) = layer.get("name").and_then(Value::as_str) {
            aliases.insert(name.to_string(), id.clone());
        }
    }
    let mut explicit_edges = 0usize;
    for (index, layer) in layers.iter().enumerate() {
        let target = &document.nodes[index].id;
        let mut references = Vec::new();
        for key in [
            "input",
            "inputs",
            "source",
            "sources",
            "from",
            "inbound_nodes",
        ] {
            if let Some(value) = layer.get(key) {
                collect_reference_strings(value, &mut references);
            }
        }
        references.sort();
        references.dedup();
        for reference in references {
            let Some(source) = aliases.get(&reference) else {
                continue;
            };
            if source == target {
                continue;
            }
            document.edges.push(VisualizationEdge::new(
                format!("model-input:{explicit_edges}"),
                source,
                target,
                "input",
                "model-flow",
            ));
            explicit_edges += 1;
        }
    }
    if let Some(edges) = value
        .get("edges")
        .and_then(Value::as_array)
        .or_else(|| value.pointer("/graph/edges").and_then(Value::as_array))
        .or_else(|| value.get("connections").and_then(Value::as_array))
    {
        for edge in edges {
            let source = edge
                .get("source")
                .or_else(|| edge.get("from"))
                .and_then(Value::as_str)
                .and_then(|value| aliases.get(value));
            let target = edge
                .get("target")
                .or_else(|| edge.get("to"))
                .and_then(Value::as_str)
                .and_then(|value| aliases.get(value));
            let (Some(source), Some(target)) = (source, target) else {
                continue;
            };
            document.edges.push(VisualizationEdge::new(
                format!("model-edge:{explicit_edges}"),
                source,
                target,
                edge.get("label").and_then(Value::as_str).unwrap_or("flow"),
                "model-flow",
            ));
            explicit_edges += 1;
        }
    }
    if explicit_edges == 0 {
        for index in 1..document.nodes.len() {
            document.edges.push(VisualizationEdge::new(
                format!("metadata-order:{index}"),
                &document.nodes[index - 1].id,
                &document.nodes[index].id,
                "declared after",
                "metadata-order",
            ));
        }
        document.diagnostics.push(VisualizationDiagnostic {
            level: "info".to_string(),
            message: "The model metadata declares layer order but no explicit tensor connections; edges show declaration order only."
                .to_string(),
            metadata: BTreeMap::new(),
        });
    }
    document
        .metadata
        .insert("raw_model_metadata".to_string(), value.clone());
    Ok(())
}

fn collect_reference_strings(value: &Value, output: &mut Vec<String>) {
    match value {
        Value::String(value) => output.push(value.clone()),
        Value::Array(values) => {
            for value in values {
                collect_reference_strings(value, output);
            }
        }
        Value::Object(object) => {
            for key in ["name", "id", "source", "from", "layer"] {
                if let Some(value) = object.get(key) {
                    collect_reference_strings(value, output);
                }
            }
        }
        _ => {}
    }
}

fn parse_training_metrics(path: &Path, document: &mut VisualizationDocument) -> Result<()> {
    let raw = read_source(path)?;
    let mut records = Vec::<Value>::new();
    if ext(path) == "jsonl" {
        for line in raw.lines().take(20_000) {
            if let Ok(value) = serde_json::from_str(line) {
                records.push(value);
            }
        }
    } else {
        let mut lines = raw.lines();
        let headers = lines
            .next()
            .unwrap_or_default()
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        for line in lines.take(20_000) {
            let values = parse_csv_row(line);
            let mut object = serde_json::Map::new();
            for (index, header) in headers.iter().enumerate() {
                if let Some(value) = values.get(index) {
                    object.insert(
                        (*header).to_string(),
                        value
                            .parse::<f64>()
                            .map(Value::from)
                            .unwrap_or_else(|_| Value::String(value.to_string())),
                    );
                }
            }
            records.push(Value::Object(object));
        }
    }
    parse_algorithm_records(&records, document);
    Ok(())
}

fn parse_csv_row(line: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                values.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    values.push(current.trim().to_string());
    values
}

fn parse_algorithm_records(records: &[Value], document: &mut VisualizationDocument) {
    let mut numeric = BTreeMap::<String, Vec<VisualizationPoint>>::new();
    let mut stage_ids = HashMap::<String, String>::new();
    let mut previous_stage: Option<String> = None;
    let mut execution_edge = 0usize;
    for (index, record) in records.iter().enumerate() {
        let timestamp = record
            .get("timestamp_ms")
            .and_then(Value::as_i64)
            .unwrap_or(index as i64);
        if let Some(object) = record.as_object() {
            for (key, value) in object {
                if matches!(
                    key.as_str(),
                    "step" | "epoch" | "timestamp" | "timestamp_ms"
                ) {
                    continue;
                }
                if let Some(value) = value.as_f64().or_else(|| value.as_str()?.parse().ok()) {
                    numeric
                        .entry(key.clone())
                        .or_default()
                        .push(VisualizationPoint {
                            timestamp_ms: timestamp,
                            value,
                        });
                }
            }
        }
        let stage = first_record_string(
            record,
            &[
                "operation",
                "op",
                "function",
                "layer",
                "node",
                "stage",
                "phase",
                "event",
                "name",
            ],
        );
        let source_id = stage.as_ref().map(|stage| {
            stage_ids
                .entry(stage.clone())
                .or_insert_with(|| {
                    let id = stable_id("execution-stage", stage);
                    document
                        .nodes
                        .push(VisualizationNode::new(&id, stage, "execution-stage"));
                    id
                })
                .clone()
        });
        if let Some(current) = source_id.as_ref() {
            if let Some(previous) = previous_stage.as_ref().filter(|value| *value != current) {
                document.edges.push(VisualizationEdge::new(
                    format!("execution-order:{execution_edge}"),
                    previous,
                    current,
                    "observed next",
                    "execution",
                ));
                execution_edge += 1;
            }
            previous_stage = Some(current.clone());
        }
        document.events.push(VisualizationEvent {
            id: format!("metric-event:{index}"),
            sequence: index,
            label: record
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or("training step")
                .to_string(),
            category: "training".to_string(),
            status: record
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            timestamp: record
                .get("timestamp")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            source_id,
            target_id: None,
            metadata: record
                .as_object()
                .map(|object| {
                    object
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    document.series = numeric
        .into_iter()
        .map(|(id, points)| VisualizationSeries {
            label: id.clone(),
            id,
            unit: String::new(),
            node_id: None,
            category: "training".to_string(),
            points,
        })
        .collect();
}

fn first_record_string(record: &Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        record.get(*key).and_then(|value| {
            value
                .as_str()
                .map(ToOwned::to_owned)
                .or_else(|| value.as_i64().map(|value| value.to_string()))
        })
    })
}

fn finalize_algorithm_frames(document: &mut VisualizationDocument) {
    if !document.events.is_empty() {
        document.frames = document
            .events
            .iter()
            .map(|event| VisualizationFrame {
                id: format!("algorithm-event-frame:{}", event.sequence),
                sequence: event.sequence,
                label: event_label(event),
                active_nodes: event.source_id.clone().into_iter().collect(),
                active_edges: event
                    .source_id
                    .as_ref()
                    .map(|source| {
                        document
                            .edges
                            .iter()
                            .filter(|edge| edge.source == *source || edge.target == *source)
                            .map(|edge| edge.id.clone())
                            .collect()
                    })
                    .unwrap_or_default(),
                metrics: event
                    .metadata
                    .iter()
                    .filter_map(|(key, value)| value.as_f64().map(|value| (key.clone(), value)))
                    .collect(),
            })
            .collect();
        return;
    }
    let already_framed = document
        .frames
        .iter()
        .flat_map(|frame| frame.active_nodes.iter().cloned())
        .collect::<HashSet<_>>();
    let mut sequence = document.frames.len();
    for node in &document.nodes {
        if already_framed.contains(&node.id) {
            continue;
        }
        document.frames.push(VisualizationFrame {
            id: format!("algorithm-node-frame:{sequence}"),
            sequence,
            label: node.label.clone(),
            active_nodes: vec![node.id.clone()],
            active_edges: document
                .edges
                .iter()
                .filter(|edge| edge.source == node.id || edge.target == node.id)
                .map(|edge| edge.id.clone())
                .collect(),
            metrics: node.metrics.clone(),
        });
        sequence += 1;
    }
}

fn event_label(event: &VisualizationEvent) -> String {
    let coordinate = ["epoch", "step", "iteration", "batch"]
        .iter()
        .find_map(|key| {
            event
                .metadata
                .get(*key)
                .map(|value| format!("{key} {value}"))
        });
    match coordinate {
        Some(coordinate) => format!("{} 路 {coordinate}", event.label),
        None => event.label.clone(),
    }
}

fn parse_safetensors_metadata(path: &Path, document: &mut VisualizationDocument) -> Result<()> {
    let bytes = std::fs::read(path)?;
    if bytes.len() < 8 {
        anyhow::bail!("safetensors header is truncated");
    }
    let header_len = u64::from_le_bytes(bytes[..8].try_into().unwrap()) as usize;
    if header_len == 0 || 8 + header_len > bytes.len() {
        anyhow::bail!("safetensors header length is invalid");
    }
    let header: Value = serde_json::from_slice(&bytes[8..8 + header_len])?;
    let Some(tensors) = header.as_object() else {
        anyhow::bail!("safetensors metadata is not an object");
    };
    let mut previous = None;
    for (index, (name, metadata)) in tensors
        .iter()
        .filter(|(name, _)| name.as_str() != "__metadata__")
        .take(2_000)
        .enumerate()
    {
        let id = format!("tensor:{index}");
        let mut node = VisualizationNode::new(&id, name, "tensor");
        node.metadata.insert(
            "dtype".to_string(),
            metadata.get("dtype").cloned().unwrap_or(Value::Null),
        );
        node.metadata.insert(
            "shape".to_string(),
            metadata.get("shape").cloned().unwrap_or(Value::Null),
        );
        if let Some(shape) = metadata.get("shape").and_then(Value::as_array) {
            let elements = shape
                .iter()
                .filter_map(Value::as_u64)
                .fold(1u64, |total, value| total.saturating_mul(value));
            node.metrics.insert("elements".to_string(), elements as f64);
        }
        document.nodes.push(node);
        if let Some(previous_id) = previous.replace(id.clone()) {
            document.edges.push(VisualizationEdge::new(
                format!("tensor-order:{index}"),
                previous_id,
                id,
                "stored after",
                "tensor-order",
            ));
        }
    }
    document.metadata.insert(
        "model_metadata".to_string(),
        tensors.get("__metadata__").cloned().unwrap_or(Value::Null),
    );
    document
        .metadata
        .insert("artifact_bytes".to_string(), Value::from(bytes.len()));
    Ok(())
}

fn parse_onnx_graph(path: &Path, document: &mut VisualizationDocument) -> Result<()> {
    let bytes = std::fs::read(path)?;
    let graph = protobuf_length_fields(&bytes, 7)
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("ONNX ModelProto has no graph field"))?;
    let nodes = protobuf_length_fields(graph, 1);
    let mut output_producers = HashMap::<String, String>::new();
    let mut pending_inputs = Vec::<(String, Vec<String>)>::new();
    for (index, raw_node) in nodes.into_iter().take(10_000).enumerate() {
        let inputs = protobuf_strings(raw_node, 1);
        let outputs = protobuf_strings(raw_node, 2);
        let name = protobuf_strings(raw_node, 3)
            .into_iter()
            .next()
            .unwrap_or_default();
        let op_type = protobuf_strings(raw_node, 4)
            .into_iter()
            .next()
            .unwrap_or_else(|| "ONNX op".to_string());
        let id = format!("onnx-node:{index}");
        let label = if name.is_empty() {
            format!("{op_type} {index}")
        } else {
            name
        };
        let mut node = VisualizationNode::new(&id, label, &op_type);
        node.metadata
            .insert("inputs".to_string(), Value::from(inputs.clone()));
        node.metadata
            .insert("outputs".to_string(), Value::from(outputs.clone()));
        document.nodes.push(node);
        pending_inputs.push((id.clone(), inputs));
        for output in outputs {
            output_producers.insert(output, id.clone());
        }
    }
    let mut edge_index = 0usize;
    for (target, inputs) in pending_inputs {
        for input in inputs {
            let Some(source) = output_producers.get(&input) else {
                continue;
            };
            document.edges.push(VisualizationEdge::new(
                format!("onnx-edge:{edge_index}"),
                source,
                &target,
                input,
                "tensor-flow",
            ));
            edge_index += 1;
        }
    }
    document
        .metadata
        .insert("artifact_bytes".to_string(), Value::from(bytes.len()));
    Ok(())
}

fn protobuf_length_fields<'a>(bytes: &'a [u8], wanted_field: u64) -> Vec<&'a [u8]> {
    let mut fields = Vec::new();
    let mut offset = 0usize;
    while offset < bytes.len() {
        let Some((key, key_bytes)) = protobuf_varint(&bytes[offset..]) else {
            break;
        };
        offset += key_bytes;
        let field = key >> 3;
        let wire = key & 7;
        match wire {
            0 => {
                let Some((_, consumed)) = protobuf_varint(&bytes[offset..]) else {
                    break;
                };
                offset += consumed;
            }
            1 => offset = offset.saturating_add(8),
            2 => {
                let Some((length, consumed)) = protobuf_varint(&bytes[offset..]) else {
                    break;
                };
                offset += consumed;
                let end = offset.saturating_add(length as usize);
                if end > bytes.len() {
                    break;
                }
                if field == wanted_field {
                    fields.push(&bytes[offset..end]);
                }
                offset = end;
            }
            5 => offset = offset.saturating_add(4),
            _ => break,
        }
    }
    fields
}

fn protobuf_strings(bytes: &[u8], field: u64) -> Vec<String> {
    protobuf_length_fields(bytes, field)
        .into_iter()
        .filter_map(|value| std::str::from_utf8(value).ok().map(ToOwned::to_owned))
        .collect()
}

fn protobuf_varint(bytes: &[u8]) -> Option<(u64, usize)> {
    let mut value = 0u64;
    for (index, byte) in bytes.iter().copied().take(10).enumerate() {
        value |= ((byte & 0x7f) as u64) << (index * 7);
        if byte & 0x80 == 0 {
            return Some((value, index + 1));
        }
    }
    None
}
