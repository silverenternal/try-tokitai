use super::{ObjectQuery, QueryFilter, QueryOperator, QueryView};
use crate::atlas_core::{AtlasCore, ObjectGraph, ObjectType, ScientificObject};
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueryResult {
    pub query: ObjectQuery,
    pub objects: Vec<ScientificObject>,
    pub graph: ObjectGraph,
    pub suggested_view: QueryView,
}

#[derive(Debug, Clone)]
pub struct ObjectQueryEngine {
    core: Arc<AtlasCore>,
}

impl ObjectQueryEngine {
    pub fn new(core: Arc<AtlasCore>) -> Self {
        Self { core }
    }

    pub fn execute(&self, query: ObjectQuery) -> Result<QueryResult> {
        let mut objects = self.core.list()?;
        objects.retain(|object| {
            (query.object_types.is_empty() || query.object_types.contains(&object.object_type))
                && text_matches(object, &query.text)
                && query.filters.iter().all(|filter| matches_filter(object, filter))
        });
        objects.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        objects.truncate(query.limit.clamp(1, 1000));
        let selected_ids = objects.iter().map(|object| object.id.clone()).collect::<BTreeSet<_>>();
        let full_graph = self.core.graph()?;
        let graph = ObjectGraph {
            objects: objects.clone(),
            relationships: full_graph
                .relationships
                .into_iter()
                .filter(|relationship| {
                    selected_ids.contains(&relationship.source_id)
                        && selected_ids.contains(&relationship.target_id)
                })
                .collect(),
        };
        let suggested_view = if !graph.relationships.is_empty() {
            QueryView::Graph
        } else if query.filters.iter().any(|filter| filter.field.ends_with("_at")) {
            QueryView::Timeline
        } else {
            QueryView::Table
        };
        Ok(QueryResult {
            query,
            objects,
            graph,
            suggested_view,
        })
    }

    pub fn natural_language(&self, input: &str) -> Result<QueryResult> {
        self.execute(if input.trim_start().to_ascii_lowercase().starts_with("find ") {
            parse_aoql(input)?
        } else {
            parse_natural_language(input)?
        })
    }
}

/// Parse the compact Atlas Object Query Language:
/// `FIND Experiment WHERE metadata.accuracy >= 0.95 AND text CONTAINS transformer LIMIT 20`.
fn parse_aoql(input: &str) -> Result<ObjectQuery> {
    let statement = Regex::new(
        r"(?i)^\s*FIND\s+([a-zA-Z0-9_-]+)(?:\s+WHERE\s+(.+?))?(?:\s+LIMIT\s+(\d+))?\s*$",
    )?;
    let captures = statement
        .captures(input)
        .ok_or_else(|| anyhow::anyhow!("invalid Atlas Object Query Language statement"))?;
    let mut query = ObjectQuery {
        object_types: BTreeSet::from([ObjectType::from(&captures[1])]),
        text: String::new(),
        filters: Vec::new(),
        limit: captures
            .get(3)
            .and_then(|value| value.as_str().parse::<usize>().ok())
            .unwrap_or(100),
    };
    let Some(expression) = captures.get(2).map(|value| value.as_str()) else {
        return Ok(query);
    };
    let condition = Regex::new(
        r#"(?i)^\s*([a-zA-Z0-9_.-]+)\s*(>=|<=|!=|=|>|<|CONTAINS)\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s]+))\s*$"#,
    )?;
    for clause in Regex::new(r"(?i)\s+AND\s+")?.split(expression) {
        let capture = condition
            .captures(clause)
            .ok_or_else(|| anyhow::anyhow!("invalid AOQL condition: {clause}"))?;
        let field = capture[1].to_string();
        let operator = match capture[2].to_ascii_uppercase().as_str() {
            ">" => QueryOperator::Gt,
            ">=" => QueryOperator::Gte,
            "<" => QueryOperator::Lt,
            "<=" => QueryOperator::Lte,
            "!=" => QueryOperator::NotEq,
            "CONTAINS" => QueryOperator::Contains,
            _ => QueryOperator::Eq,
        };
        let raw = capture
            .get(3)
            .or_else(|| capture.get(4))
            .or_else(|| capture.get(5))
            .map(|value| value.as_str())
            .unwrap_or_default();
        let value = raw
            .parse::<f64>()
            .map(Value::from)
            .unwrap_or_else(|_| Value::String(raw.to_string()));
        if field.eq_ignore_ascii_case("text") && operator == QueryOperator::Contains {
            query.text = raw.to_string();
        } else {
            query.filters.push(QueryFilter { field, operator, value });
        }
    }
    Ok(query)
}

fn parse_natural_language(input: &str) -> Result<ObjectQuery> {
    let lower = input.to_ascii_lowercase();
    let known_types = [
        ("experiments", "experiment"),
        ("experiment", "experiment"),
        ("papers", "paper"),
        ("paper", "paper"),
        ("datasets", "dataset"),
        ("dataset", "dataset"),
        ("models", "model"),
        ("model", "model"),
        ("hypotheses", "hypothesis"),
        ("hypothesis", "hypothesis"),
        ("robots", "robot"),
        ("robot", "robot"),
        ("simulations", "simulation"),
        ("simulation", "simulation"),
        ("visualizations", "visualization"),
        ("evidence", "evidence"),
        ("runtimes", "runtime"),
        ("runtime", "runtime"),
        ("publications", "publication"),
    ];
    let object_types = known_types
        .iter()
        .filter(|(word, _)| lower.contains(word))
        .map(|(_, object_type)| (*object_type).into())
        .collect::<BTreeSet<_>>();
    let comparison = Regex::new(r"([a-zA-Z0-9_.-]+)\s*(>=|<=|>|<|=)\s*([0-9]+(?:\.[0-9]+)?)")?;
    let mut filters = Vec::new();
    for capture in comparison.captures_iter(input) {
        let operator = match capture.get(2).map(|value| value.as_str()) {
            Some(">") => QueryOperator::Gt,
            Some(">=") => QueryOperator::Gte,
            Some("<") => QueryOperator::Lt,
            Some("<=") => QueryOperator::Lte,
            _ => QueryOperator::Eq,
        };
        filters.push(QueryFilter {
            field: capture[1].to_string(),
            operator,
            value: json!(capture[3].parse::<f64>()?),
        });
    }
    let mut text = input.to_string();
    for (word, _) in known_types {
        text = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(word)))?
            .replace_all(&text, "")
            .into_owned();
    }
    text = comparison.replace_all(&text, "").into_owned();
    text = Regex::new(r"(?i)\b(find|search|show|all|using|with|where|after|before)\b")?
        .replace_all(&text, "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Ok(ObjectQuery {
        object_types,
        text,
        filters,
        limit: 100,
    })
}

fn text_matches(object: &ScientificObject, text: &str) -> bool {
    text.split_whitespace().all(|term| {
        let term = term.to_ascii_lowercase();
        object.search_index.iter().any(|token| token.contains(&term))
            || object
                .metadata
                .values()
                .any(|value| value.to_string().to_ascii_lowercase().contains(&term))
    })
}

fn matches_filter(object: &ScientificObject, filter: &QueryFilter) -> bool {
    let value = object_field(object, &filter.field);
    match (&filter.operator, value) {
        (_, None) => false,
        (QueryOperator::Eq, Some(value)) => value == filter.value,
        (QueryOperator::NotEq, Some(value)) => value != filter.value,
        (QueryOperator::Contains, Some(value)) => value
            .to_string()
            .to_ascii_lowercase()
            .contains(&filter.value.to_string().trim_matches('"').to_ascii_lowercase()),
        (QueryOperator::In, Some(value)) => filter
            .value
            .as_array()
            .is_some_and(|values| values.contains(&value)),
        (operator, Some(value)) => compare_numbers(&value, &filter.value, operator),
    }
}

fn object_field(object: &ScientificObject, field: &str) -> Option<Value> {
    match field {
        "id" => Some(json!(object.id)),
        "type" | "object_type" => Some(json!(object.object_type.0)),
        "name" | "display_name" => Some(json!(object.display_name)),
        "version" => Some(json!(object.version)),
        "owner" => Some(json!(object.owner)),
        "created_at" => Some(json!(object.created_at)),
        "updated_at" => Some(json!(object.updated_at)),
        "lifecycle" | "status" => Some(json!(object.lifecycle)),
        path if path.starts_with("metadata.") => object
            .metadata
            .get(path.trim_start_matches("metadata."))
            .cloned(),
        key => object.metadata.get(key).cloned(),
    }
}

fn compare_numbers(left: &Value, right: &Value, operator: &QueryOperator) -> bool {
    let Some(left) = left.as_f64() else { return false };
    let Some(right) = right.as_f64() else { return false };
    match operator {
        QueryOperator::Gt => left > right,
        QueryOperator::Gte => left >= right,
        QueryOperator::Lt => left < right,
        QueryOperator::Lte => left <= right,
        QueryOperator::Eq => (left - right).abs() < f64::EPSILON,
        QueryOperator::NotEq => (left - right).abs() >= f64::EPSILON,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas_core::ScientificObject;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn aoql_filters_object_metadata() {
        let directory = tempdir().unwrap();
        let core = Arc::new(AtlasCore::open(directory.path()).unwrap());
        let mut experiment = ScientificObject::new("experiment", "Transformer baseline", "agent");
        experiment.metadata.insert("accuracy".into(), json!(0.97));
        experiment.rebuild_search_index();
        core.create(experiment, "agent").unwrap();
        let result = ObjectQueryEngine::new(core)
            .natural_language(
                "FIND Experiment WHERE metadata.accuracy >= 0.95 AND text CONTAINS transformer LIMIT 20",
            )
            .unwrap();
        assert_eq!(result.objects.len(), 1);
        assert_eq!(result.objects[0].display_name, "Transformer baseline");
    }
}
