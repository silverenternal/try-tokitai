//! Research Diary Module
//!
//! Captures observations, decisions, and insights during research.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectId, ResearchObjectType,
};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DiaryEntryType {
    #[serde(rename = "observation")]
    Observation,
    #[serde(rename = "decision")]
    Decision,
    #[serde(rename = "question")]
    Question,
    #[serde(rename = "insight")]
    Insight,
}

impl DiaryEntryType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Observation => "observation",
            Self::Decision => "decision",
            Self::Question => "question",
            Self::Insight => "insight",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiaryEntry {
    pub schema_version: String,
    pub id: String,
    pub timestamp: String,
    pub author: String,
    pub entry_type: String,
    pub content: String,
    #[serde(default)]
    pub related_objects: Vec<ResearchObjectId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub daily_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weekly_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monthly_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub milestone: Option<String>,
}

const DIARY_SCHEMA: &str = "atlas.research-os.diary.v1";

pub fn create_diary_entry(
    workspace_root: &Path,
    entry_type: DiaryEntryType,
    content: &str,
    author: &str,
    domain_id: Option<String>,
    related_objects: Vec<ResearchObjectId>,
) -> Result<DiaryEntry> {
    create_diary_entry_with_source(
        workspace_root,
        entry_type,
        content,
        author,
        domain_id,
        related_objects,
        None,
    )
}

pub fn create_diary_entry_with_source(
    workspace_root: &Path,
    entry_type: DiaryEntryType,
    content: &str,
    author: &str,
    domain_id: Option<String>,
    related_objects: Vec<ResearchObjectId>,
    source_id: Option<String>,
) -> Result<DiaryEntry> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}:{}", author, content, now));

    let entry = DiaryEntry {
        schema_version: DIARY_SCHEMA.to_string(),
        id: id.clone(),
        timestamp: now,
        author: author.to_string(),
        entry_type: entry_type.as_str().to_string(),
        content: content.to_string(),
        related_objects,
        domain_id,
        source_id,
        daily_summary: None,
        weekly_summary: None,
        monthly_summary: None,
        milestone: None,
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::DiaryEntry,
        &id,
        &entry,
    )?;

    Ok(entry)
}

pub fn get_diary_entry(workspace_root: &Path, id: &str) -> Result<DiaryEntry> {
    read_research_object(workspace_root, ResearchObjectType::DiaryEntry, id)
}

pub fn list_diary_entries(workspace_root: &Path) -> Result<Vec<DiaryEntry>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::DiaryEntry)?;
    let mut entries = Vec::new();

    for id in ids {
        if let Ok(entry) = get_diary_entry(workspace_root, &id) {
            entries.push(entry);
        }
    }

    // Sort by timestamp descending (newest first)
    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(entries)
}

pub fn list_diary_entries_by_type(
    workspace_root: &Path,
    entry_type: DiaryEntryType,
) -> Result<Vec<DiaryEntry>> {
    let all_entries = list_diary_entries(workspace_root)?;
    Ok(all_entries
        .into_iter()
        .filter(|e| e.entry_type == entry_type.as_str())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_diary_entry() {
        let dir = tempdir().unwrap();
        let entry = create_diary_entry(
            dir.path(),
            DiaryEntryType::Observation,
            "Model shows promising results on validation set",
            "agent",
            Some("ai-ml".to_string()),
            vec![],
        )
        .unwrap();

        assert_eq!(entry.entry_type, "observation");
        assert_eq!(entry.author, "agent");
        assert!(entry.content.contains("validation set"));

        let retrieved = get_diary_entry(dir.path(), &entry.id).unwrap();
        assert_eq!(retrieved.content, entry.content);
    }

    #[test]
    fn filters_diary_entries_by_type() {
        let dir = tempdir().unwrap();

        create_diary_entry(
            dir.path(),
            DiaryEntryType::Observation,
            "Observation 1",
            "agent",
            None,
            vec![],
        )
        .unwrap();

        create_diary_entry(
            dir.path(),
            DiaryEntryType::Decision,
            "Decision 1",
            "user",
            None,
            vec![],
        )
        .unwrap();

        let observations = list_diary_entries_by_type(dir.path(), DiaryEntryType::Observation).unwrap();
        assert_eq!(observations.len(), 1);
        assert_eq!(observations[0].content, "Observation 1");
    }

    #[test]
    fn diary_entries_sorted_newest_first() {
        let dir = tempdir().unwrap();
        let entry1 = create_diary_entry(
            dir.path(),
            DiaryEntryType::Insight,
            "First",
            "agent",
            None,
            vec![],
        )
        .unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let entry2 = create_diary_entry(
            dir.path(),
            DiaryEntryType::Insight,
            "Second",
            "agent",
            None,
            vec![],
        )
        .unwrap();

        let entries = list_diary_entries(dir.path()).unwrap();
        assert_eq!(entries[0].id, entry2.id); // Newest first
        assert_eq!(entries[1].id, entry1.id);
    }
}
