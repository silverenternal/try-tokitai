//! Research Memory Module
//!
//! Research-specific memory with importance scoring and access tracking.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectId, ResearchObjectType,
};
use super::timeline::{create_timeline_event, EventType};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchMemoryEntry {
    pub schema_version: String,
    pub id: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    #[serde(default)]
    pub related_objects: Vec<ResearchObjectId>,
    pub importance: f64,
    pub accessed_count: usize,
    pub created_at: String,
    pub last_accessed_at: String,
}

const MEMORY_SCHEMA: &str = "atlas.research-os.memory.v1";

pub fn create_memory_entry(
    workspace_root: &Path,
    content: &str,
    importance: f64,
    related_objects: Vec<ResearchObjectId>,
    embedding: Option<Vec<f32>>,
) -> Result<ResearchMemoryEntry> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}", content, now));

    let entry = ResearchMemoryEntry {
        schema_version: MEMORY_SCHEMA.to_string(),
        id: id.clone(),
        content: content.to_string(),
        embedding,
        related_objects,
        importance: importance.clamp(0.0, 1.0),
        accessed_count: 0,
        created_at: now.clone(),
        last_accessed_at: now,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::Memory,
        &id,
        &entry,
    )?;

    create_timeline_event(
        workspace_root,
        EventType::MemoryCaptured,
        "Research memory captured",
        content,
        None,
        vec![ResearchObjectId {
            object_type: "memory".to_string(),
            id: entry.id.clone(),
            created_at: entry.created_at.clone(),
        }],
    )?;

    Ok(entry)
}

pub fn get_memory_entry(workspace_root: &Path, id: &str) -> Result<ResearchMemoryEntry> {
    let mut entry: ResearchMemoryEntry =
        read_research_object(workspace_root, ResearchObjectType::Memory, id)?;

    // Update access tracking
    entry.accessed_count += 1;
    entry.last_accessed_at = Utc::now().to_rfc3339();

    write_research_object(
        workspace_root,
        ResearchObjectType::Memory,
        id,
        &entry,
    )?;

    Ok(entry)
}

pub fn list_memory_entries(workspace_root: &Path) -> Result<Vec<ResearchMemoryEntry>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::Memory)?;
    let mut entries = Vec::new();

    for id in ids {
        // Use read_research_object directly to avoid incrementing access count
        if let Ok(entry) =
            read_research_object::<ResearchMemoryEntry>(workspace_root, ResearchObjectType::Memory, &id)
        {
            entries.push(entry);
        }
    }

    // Sort by importance descending
    entries.sort_by(|a, b| {
        b.importance
            .partial_cmp(&a.importance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(entries)
}

pub fn search_memory(workspace_root: &Path, query: &str) -> Result<Vec<ResearchMemoryEntry>> {
    let all_entries = list_memory_entries(workspace_root)?;
    let query_lower = query.to_lowercase();

    let mut matches: Vec<_> = all_entries
        .into_iter()
        .filter_map(|entry| {
            let content_lower = entry.content.to_lowercase();
            if content_lower.contains(&query_lower) {
                Some((entry, compute_relevance_score(&content_lower, &query_lower)))
            } else {
                None
            }
        })
        .collect();

    // Sort by relevance score (importance * text match score)
    matches.sort_by(|a, b| {
        let score_a = a.0.importance * a.1;
        let score_b = b.0.importance * b.1;
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(matches.into_iter().map(|(entry, _)| entry).collect())
}

fn compute_relevance_score(content: &str, query: &str) -> f64 {
    let query_words: std::collections::HashSet<&str> = query.split_whitespace().collect();
    let content_words: Vec<&str> = content.split_whitespace().collect();

    if query_words.is_empty() || content_words.is_empty() {
        return 0.0;
    }

    let matches = content_words
        .iter()
        .filter(|word| query_words.contains(*word))
        .count();

    matches as f64 / content_words.len() as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_memory_entry() {
        let dir = tempdir().unwrap();
        let entry = create_memory_entry(
            dir.path(),
            "Important finding: model performs better with batch size 32",
            0.9,
            vec![],
            None,
        )
        .unwrap();

        assert_eq!(entry.importance, 0.9);
        assert_eq!(entry.accessed_count, 0);

        let retrieved = get_memory_entry(dir.path(), &entry.id).unwrap();
        assert_eq!(retrieved.accessed_count, 1); // Incremented on access
        assert_ne!(retrieved.last_accessed_at, entry.last_accessed_at);
    }

    #[test]
    fn tracks_access_count() {
        let dir = tempdir().unwrap();
        let entry = create_memory_entry(dir.path(), "Test memory", 0.5, vec![], None).unwrap();

        get_memory_entry(dir.path(), &entry.id).unwrap();
        let accessed_twice = get_memory_entry(dir.path(), &entry.id).unwrap();

        assert_eq!(accessed_twice.accessed_count, 2);
    }

    #[test]
    fn searches_memory_by_content() {
        let dir = tempdir().unwrap();
        create_memory_entry(
            dir.path(),
            "learning rate should be 0.001 for stable training",
            0.8,
            vec![],
            None,
        )
        .unwrap();
        create_memory_entry(
            dir.path(),
            "batch size affects convergence speed",
            0.7,
            vec![],
            None,
        )
        .unwrap();

        let results = search_memory(dir.path(), "learning rate").unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].content.contains("learning rate"));
    }

    #[test]
    fn sorts_memory_by_importance() {
        let dir = tempdir().unwrap();
        create_memory_entry(dir.path(), "Low importance", 0.3, vec![], None).unwrap();
        create_memory_entry(dir.path(), "High importance", 0.9, vec![], None).unwrap();

        let list = list_memory_entries(dir.path()).unwrap();
        assert_eq!(list[0].importance, 0.9); // Highest first
        assert_eq!(list[1].importance, 0.3);
    }
}
