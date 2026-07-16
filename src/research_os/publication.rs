//! Publication Pipeline Module
//!
//! Manages publication drafts with evidence linking.

use super::object_graph::{
    generate_object_id, list_research_objects, read_research_object, write_research_object,
    ResearchObjectId, ResearchObjectType,
};
use super::timeline::{create_timeline_event, EventType};
use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PublicationStatus {
    #[serde(rename = "draft")]
    Draft,
    #[serde(rename = "review")]
    Review,
    #[serde(rename = "ready")]
    Ready,
    #[serde(rename = "published")]
    Published,
}

impl PublicationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Ready => "ready",
            Self::Published => "published",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationSection {
    pub section_id: String,
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicationDraft {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub status: String,
    #[serde(default)]
    pub sections: Vec<PublicationSection>,
    #[serde(default)]
    pub hypothesis_ids: Vec<String>,
    #[serde(default)]
    pub evidence_ids: Vec<String>,
    #[serde(default)]
    pub experiment_ids: Vec<String>,
    #[serde(default)]
    pub artifact_paths: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: String,
}

const PUBLICATION_SCHEMA: &str = "atlas.research-os.publication.v1";

pub fn create_publication(
    workspace_root: &Path,
    title: &str,
    created_by: &str,
) -> Result<PublicationDraft> {
    let now = Utc::now().to_rfc3339();
    let id = generate_object_id(&format!("{}:{}", title, now));

    let publication = PublicationDraft {
        schema_version: PUBLICATION_SCHEMA.to_string(),
        id: id.clone(),
        title: title.to_string(),
        status: PublicationStatus::Draft.as_str().to_string(),
        sections: Vec::new(),
        hypothesis_ids: Vec::new(),
        evidence_ids: Vec::new(),
        experiment_ids: Vec::new(),
        artifact_paths: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
        created_by: created_by.to_string(),
    };

    write_research_object(
        workspace_root,
        ResearchObjectType::Publication,
        &id,
        &publication,
    )?;

    create_timeline_event(
        workspace_root,
        EventType::PublicationDrafted,
        &format!("Publication drafted: {}", publication.title),
        "Publication pipeline initialized from Research OS objects.",
        None,
        vec![ResearchObjectId {
            object_type: "publication".to_string(),
            id: publication.id.clone(),
            created_at: publication.created_at.clone(),
        }],
    )?;

    Ok(publication)
}

pub fn update_publication(
    workspace_root: &Path,
    id: &str,
    updates: PublicationUpdate,
) -> Result<PublicationDraft> {
    let mut publication: PublicationDraft =
        read_research_object(workspace_root, ResearchObjectType::Publication, id)?;
    let previous_status = publication.status.clone();

    if let Some(title) = updates.title {
        publication.title = title;
    }
    if let Some(status) = updates.status {
        publication.status = status;
    }
    if let Some(sections) = updates.sections {
        publication.sections = sections;
    }
    if let Some(hypothesis_ids) = updates.hypothesis_ids {
        publication.hypothesis_ids = hypothesis_ids;
    }
    if let Some(evidence_ids) = updates.evidence_ids {
        publication.evidence_ids = evidence_ids;
    }
    if let Some(experiment_ids) = updates.experiment_ids {
        publication.experiment_ids = experiment_ids;
    }
    if let Some(artifact_paths) = updates.artifact_paths {
        publication.artifact_paths = artifact_paths;
    }

    publication.updated_at = Utc::now().to_rfc3339();

    write_research_object(
        workspace_root,
        ResearchObjectType::Publication,
        id,
        &publication,
    )?;

    create_timeline_event(
        workspace_root,
        EventType::PublicationUpdated,
        &format!("Publication updated: {}", publication.title),
        &format!(
            "Pipeline state {} -> {}; {} sections, {} linked evidence objects.",
            previous_status,
            publication.status,
            publication.sections.len(),
            publication.evidence_ids.len()
        ),
        None,
        vec![ResearchObjectId {
            object_type: "publication".to_string(),
            id: publication.id.clone(),
            created_at: publication.updated_at.clone(),
        }],
    )?;

    Ok(publication)
}

pub fn get_publication(workspace_root: &Path, id: &str) -> Result<PublicationDraft> {
    read_research_object(workspace_root, ResearchObjectType::Publication, id)
}

pub fn list_publications(workspace_root: &Path) -> Result<Vec<PublicationDraft>> {
    let ids = list_research_objects(workspace_root, ResearchObjectType::Publication)?;
    let mut publications = Vec::new();

    for id in ids {
        if let Ok(publication) = get_publication(workspace_root, &id) {
            publications.push(publication);
        }
    }

    // Sort by updated_at descending (most recently updated first)
    publications.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

    Ok(publications)
}

#[derive(Debug, Clone, Default)]
pub struct PublicationUpdate {
    pub title: Option<String>,
    pub status: Option<String>,
    pub sections: Option<Vec<PublicationSection>>,
    pub hypothesis_ids: Option<Vec<String>>,
    pub evidence_ids: Option<Vec<String>>,
    pub experiment_ids: Option<Vec<String>>,
    pub artifact_paths: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn creates_and_retrieves_publication() {
        let dir = tempdir().unwrap();
        let pub_draft = create_publication(
            dir.path(),
            "Scaling Laws for Neural Language Models",
            "agent",
        )
        .unwrap();

        assert_eq!(pub_draft.title, "Scaling Laws for Neural Language Models");
        assert_eq!(pub_draft.status, "draft");
        assert_eq!(pub_draft.sections.len(), 0);

        let retrieved = get_publication(dir.path(), &pub_draft.id).unwrap();
        assert_eq!(retrieved.title, pub_draft.title);
    }

    #[test]
    fn updates_publication_with_sections() {
        let dir = tempdir().unwrap();
        let pub_draft = create_publication(dir.path(), "Test Paper", "agent").unwrap();

        let sections = vec![
            PublicationSection {
                section_id: "intro".to_string(),
                title: "Introduction".to_string(),
                content: "This paper presents...".to_string(),
                evidence_ids: vec![],
            },
            PublicationSection {
                section_id: "methods".to_string(),
                title: "Methods".to_string(),
                content: "We used...".to_string(),
                evidence_ids: vec!["ev-123".to_string()],
            },
        ];

        let updates = PublicationUpdate {
            sections: Some(sections.clone()),
            status: Some("review".to_string()),
            evidence_ids: Some(vec!["ev-123".to_string()]),
            ..Default::default()
        };

        let updated = update_publication(dir.path(), &pub_draft.id, updates).unwrap();
        assert_eq!(updated.status, "review");
        assert_eq!(updated.sections.len(), 2);
        assert_eq!(updated.sections[1].evidence_ids[0], "ev-123");
    }

    #[test]
    fn links_publication_to_research_objects() {
        let dir = tempdir().unwrap();
        let pub_draft = create_publication(dir.path(), "Research Paper", "agent").unwrap();

        let updates = PublicationUpdate {
            hypothesis_ids: Some(vec!["hyp-1".to_string(), "hyp-2".to_string()]),
            experiment_ids: Some(vec!["exp-1".to_string()]),
            evidence_ids: Some(vec!["ev-1".to_string(), "ev-2".to_string()]),
            artifact_paths: Some(vec!["results/model.pth".to_string()]),
            ..Default::default()
        };

        let updated = update_publication(dir.path(), &pub_draft.id, updates).unwrap();
        assert_eq!(updated.hypothesis_ids.len(), 2);
        assert_eq!(updated.experiment_ids.len(), 1);
        assert_eq!(updated.evidence_ids.len(), 2);
        assert_eq!(updated.artifact_paths.len(), 1);
    }

    #[test]
    fn lists_publications_most_recent_first() {
        let dir = tempdir().unwrap();
        let pub1 = create_publication(dir.path(), "Paper 1", "agent").unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));

        let pub2 = create_publication(dir.path(), "Paper 2", "agent").unwrap();

        // Update pub1 to make it most recently updated
        std::thread::sleep(std::time::Duration::from_millis(10));
        update_publication(
            dir.path(),
            &pub1.id,
            PublicationUpdate {
                status: Some("review".to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let list = list_publications(dir.path()).unwrap();
        assert_eq!(list[0].id, pub1.id); // Most recently updated first
        assert_eq!(list[1].id, pub2.id);
    }
}
