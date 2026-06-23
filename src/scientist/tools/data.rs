//! Data tools focused on real experiment setup rather than mock preprocessing.

use serde_json::{json, Value};
use std::fs;
use std::path::Path;
use tokitai::tool;

pub struct DataTools;

fn detect_format(path: &str, requested: Option<&str>) -> String {
    let requested = requested.unwrap_or("auto").trim().to_ascii_lowercase();
    if requested != "auto" && !requested.is_empty() {
        return requested;
    }

    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("unknown")
        .to_ascii_lowercase()
}

fn inspect_delimited(content: &str, delimiter: char, preview_rows: usize) -> Value {
    let rows: Vec<Vec<String>> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .take(preview_rows + 1)
        .map(|line| line.split(delimiter).map(|cell| cell.trim().to_string()).collect())
        .collect();

    if rows.is_empty() {
        return json!({
            "format": "table",
            "rows_previewed": 0,
            "column_count": 0,
            "columns": [],
            "preview": [],
        });
    }

    let columns = rows.first().cloned().unwrap_or_default();
    let preview = rows.iter().skip(1).cloned().collect::<Vec<_>>();
    json!({
        "format": "table",
        "rows_previewed": preview.len(),
        "column_count": columns.len(),
        "columns": columns,
        "preview": preview,
    })
}

fn inspect_json_value(value: &Value) -> Value {
    match value {
        Value::Array(items) => {
            let columns = items
                .iter()
                .find_map(|item| item.as_object())
                .map(|map| map.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            json!({
                "format": "json",
                "shape": "array",
                "size_hint": items.len(),
                "column_count": columns.len(),
                "columns": columns,
                "preview": items.iter().take(5).cloned().collect::<Vec<_>>(),
            })
        }
        Value::Object(map) => json!({
            "format": "json",
            "shape": "object",
            "size_hint": map.len(),
            "column_count": map.len(),
            "columns": map.keys().cloned().collect::<Vec<_>>(),
            "preview": value,
        }),
        _ => json!({
            "format": "json",
            "shape": "scalar",
            "size_hint": 1,
            "column_count": 0,
            "columns": [],
            "preview": value,
        }),
    }
}

#[tool]
impl DataTools {
    /// Inspect a local dataset and return a lightweight structural summary.
    ///
    /// Supported formats: csv, tsv, json, jsonl.
    pub fn inspect_dataset(
        &self,
        path: String,
        format: Option<String>,
        preview_rows: Option<usize>,
    ) -> Result<Value, String> {
        let preview_rows = preview_rows.unwrap_or(5).clamp(1, 20);
        let dataset_path = Path::new(&path);
        if !dataset_path.exists() {
            return Err(format!("inspect_dataset: file does not exist: {}", path));
        }
        if !dataset_path.is_file() {
            return Err(format!("inspect_dataset: path is not a file: {}", path));
        }

        let content = fs::read_to_string(dataset_path)
            .map_err(|err| format!("inspect_dataset: failed to read '{}': {}", path, err))?;
        let format_name = detect_format(&path, format.as_deref());

        let summary = match format_name.as_str() {
            "csv" => inspect_delimited(&content, ',', preview_rows),
            "tsv" => inspect_delimited(&content, '\t', preview_rows),
            "json" => {
                let value: Value = serde_json::from_str(&content)
                    .map_err(|err| format!("inspect_dataset: invalid json in '{}': {}", path, err))?;
                inspect_json_value(&value)
            }
            "jsonl" => {
                let rows = content
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .take(preview_rows)
                    .map(|line| {
                        serde_json::from_str::<Value>(line)
                            .unwrap_or_else(|_| json!({ "raw": line }))
                    })
                    .collect::<Vec<_>>();
                let columns = rows
                    .iter()
                    .find_map(|item| item.as_object())
                    .map(|map| map.keys().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                json!({
                    "format": "jsonl",
                    "rows_previewed": rows.len(),
                    "column_count": columns.len(),
                    "columns": columns,
                    "preview": rows,
                })
            }
            other => {
                return Err(format!(
                    "inspect_dataset: unsupported dataset format '{}' for '{}'. Use csv, tsv, json, or jsonl.",
                    other, path
                ));
            }
        };

        Ok(json!({
            "status": "success",
            "operation": "inspect_dataset",
            "path": path,
            "summary": summary
        }))
    }
}
