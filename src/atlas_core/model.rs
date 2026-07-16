use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub type ObjectId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ObjectType(pub String);

impl ObjectType {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into().trim().to_ascii_lowercase().replace('_', "-");
        if value.is_empty()
            || value.len() > 96
            || !value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
        {
            return Err(anyhow!("invalid scientific object type"));
        }
        Ok(Self(value))
    }
}

impl From<&str> for ObjectType {
    fn from(value: &str) -> Self {
        Self(value.trim().to_ascii_lowercase().replace('_', "-"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    #[default]
    Draft,
    Active,
    Running,
    Blocked,
    Completed,
    Failed,
    Archived,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Artifact {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub revision: String,
    #[serde(default)]
    pub media_type: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

impl Artifact {
    pub fn new(
        path: impl Into<String>,
        kind: impl Into<String>,
        revision: impl Into<String>,
    ) -> Self {
        let path = path.into().replace('\\', "/");
        let kind = kind.into();
        let revision = revision.into();
        let id = blake3::hash(format!("{path}:{kind}").as_bytes()).to_hex()[..24].to_string();
        Self {
            id,
            path,
            kind,
            revision,
            media_type: String::new(),
            metadata: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    GeneratedBy,
    DerivedFrom,
    Supports,
    Rejects,
    DependsOn,
    Uses,
    Contains,
    Produces,
    Consumes,
    BelongsTo,
    RelatedTo,
    VersionOf,
    ForkOf,
    Parent,
    Child,
    Custom(String),
}

impl RelationshipKind {
    pub fn inverse(&self) -> Self {
        match self {
            Self::GeneratedBy => Self::Produces,
            Self::DerivedFrom => Self::Parent,
            Self::Supports => Self::Supports,
            Self::Rejects => Self::Rejects,
            Self::DependsOn => Self::Uses,
            Self::Uses => Self::DependsOn,
            Self::Contains => Self::BelongsTo,
            Self::Produces => Self::GeneratedBy,
            Self::Consumes => Self::Uses,
            Self::BelongsTo => Self::Contains,
            Self::RelatedTo => Self::RelatedTo,
            Self::VersionOf => Self::VersionOf,
            Self::ForkOf => Self::Parent,
            Self::Parent => Self::Child,
            Self::Child => Self::Parent,
            Self::Custom(value) => Self::Custom(value.clone()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelationshipRef {
    pub relationship_id: String,
    pub object_id: ObjectId,
    pub kind: RelationshipKind,
    pub direction: RelationshipDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipDirection {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    pub id: String,
    pub source_id: ObjectId,
    pub target_id: ObjectId,
    pub kind: RelationshipKind,
    pub created_at: String,
    pub created_by: String,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PermissionPolicy {
    #[serde(default)]
    pub readers: BTreeSet<String>,
    #[serde(default)]
    pub writers: BTreeSet<String>,
    #[serde(default)]
    pub executors: BTreeSet<String>,
    #[serde(default)]
    pub public_read: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RuntimeBinding {
    pub runtime_object_id: Option<ObjectId>,
    #[serde(default)]
    pub requested_capabilities: Vec<String>,
    #[serde(default)]
    pub configuration: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct PreviewDescriptor {
    pub provider: String,
    pub kind: String,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct VisualizationMapping {
    pub provider: String,
    pub kind: String,
    #[serde(default)]
    pub source_artifact_ids: Vec<String>,
    #[serde(default)]
    pub options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceLink {
    pub evidence_object_id: ObjectId,
    pub supports: bool,
    pub strength: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AgentContext {
    pub summary: String,
    #[serde(default)]
    pub instructions: Vec<String>,
    #[serde(default)]
    pub relevant_object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub data: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScientificObject {
    pub schema_version: String,
    pub id: ObjectId,
    pub object_type: ObjectType,
    pub display_name: String,
    pub description: String,
    pub version: u64,
    pub owner: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub tags: BTreeSet<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, Value>,
    pub lifecycle: LifecycleState,
    #[serde(default)]
    pub relationships: Vec<RelationshipRef>,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    #[serde(default)]
    pub runtime: RuntimeBinding,
    #[serde(default)]
    pub preview: PreviewDescriptor,
    #[serde(default)]
    pub visualizations: Vec<VisualizationMapping>,
    #[serde(default)]
    pub evidence: Vec<EvidenceLink>,
    #[serde(default)]
    pub ai_context: AgentContext,
    #[serde(default)]
    pub permissions: PermissionPolicy,
    #[serde(default)]
    pub search_index: BTreeSet<String>,
}

impl ScientificObject {
    pub fn new(
        object_type: impl Into<ObjectType>,
        display_name: impl Into<String>,
        owner: impl Into<String>,
    ) -> Self {
        let now = Utc::now().to_rfc3339();
        let mut object = Self {
            schema_version: "atlas.scientific-object.v1".into(),
            id: Uuid::new_v4().to_string(),
            object_type: object_type.into(),
            display_name: display_name.into(),
            description: String::new(),
            version: 1,
            owner: owner.into(),
            created_at: now.clone(),
            updated_at: now,
            tags: BTreeSet::new(),
            metadata: BTreeMap::new(),
            lifecycle: LifecycleState::Draft,
            relationships: Vec::new(),
            artifacts: Vec::new(),
            runtime: RuntimeBinding::default(),
            preview: PreviewDescriptor::default(),
            visualizations: Vec::new(),
            evidence: Vec::new(),
            ai_context: AgentContext::default(),
            permissions: PermissionPolicy::default(),
            search_index: BTreeSet::new(),
        };
        object.rebuild_search_index();
        object
    }

    pub fn rebuild_search_index(&mut self) {
        self.search_index = format!(
            "{} {} {} {}",
            self.object_type.0,
            self.display_name,
            self.description,
            self.tags.iter().cloned().collect::<Vec<_>>().join(" ")
        )
        .split(|character: char| !character.is_alphanumeric() && character != '-')
        .filter(|token| !token.is_empty())
        .map(|token| token.to_ascii_lowercase())
        .collect();
    }

    pub fn can_read(&self, actor: &str) -> bool {
        self.owner == actor
            || self.permissions.public_read
            || self.permissions.readers.is_empty()
            || self.permissions.readers.contains(actor)
    }

    pub fn can_write(&self, actor: &str) -> bool {
        self.owner == actor
            || (self.permissions.writers.is_empty() && self.permissions.readers.is_empty())
            || self.permissions.writers.contains(actor)
    }

    pub fn can_execute(&self, actor: &str) -> bool {
        self.owner == actor
            || self.permissions.executors.is_empty()
            || self.permissions.executors.contains(actor)
    }

    pub fn patch(&mut self, patch: &Value) -> Result<()> {
        let patch = patch
            .as_object()
            .ok_or_else(|| anyhow!("scientific object patch must be an object"))?;
        if let Some(value) = patch.get("display_name").and_then(Value::as_str) {
            self.display_name = value.trim().to_string();
        }
        if let Some(value) = patch.get("description").and_then(Value::as_str) {
            self.description = value.to_string();
        }
        if let Some(value) = patch.get("owner").and_then(Value::as_str) {
            self.owner = value.trim().to_string();
        }
        if let Some(value) = patch.get("lifecycle") {
            self.lifecycle = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("tags") {
            self.tags = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("metadata") {
            merge_metadata(&mut self.metadata, value)?;
        }
        if let Some(value) = patch.get("artifacts") {
            self.artifacts = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("runtime") {
            self.runtime = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("preview") {
            self.preview = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("visualizations") {
            self.visualizations = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("evidence") {
            self.evidence = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("ai_context") {
            self.ai_context = serde_json::from_value(value.clone())?;
        }
        if let Some(value) = patch.get("permissions") {
            self.permissions = serde_json::from_value(value.clone())?;
        }
        self.rebuild_search_index();
        Ok(())
    }
}

fn merge_metadata(target: &mut BTreeMap<String, Value>, patch: &Value) -> Result<()> {
    let patch = patch
        .as_object()
        .ok_or_else(|| anyhow!("metadata patch must be an object"))?;
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
        } else {
            target.insert(key.clone(), value.clone());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectRevision {
    pub object_id: ObjectId,
    pub version: u64,
    pub parent_version: Option<u64>,
    pub created_at: String,
    pub created_by: String,
    pub reason: String,
    pub snapshot: ScientificObject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AtlasEventKind {
    ObjectCreated,
    ObjectUpdated,
    ObjectArchived,
    ObjectDeleted,
    ObjectForked,
    ObjectMerged,
    RelationshipCreated,
    RelationshipDeleted,
    ExecutionStarted,
    ExecutionFinished,
    EvidenceAdded,
    WorkspaceOpened,
    VisualizationGenerated,
    RecommendationGenerated,
    TimelineUpdated,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasEvent {
    pub id: String,
    pub kind: AtlasEventKind,
    pub timestamp: String,
    pub actor: String,
    pub object_ids: Vec<ObjectId>,
    #[serde(default)]
    pub data: Value,
}

impl AtlasEvent {
    pub fn new(kind: AtlasEventKind, actor: impl Into<String>, object_ids: Vec<ObjectId>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            kind,
            timestamp: Utc::now().to_rfc3339(),
            actor: actor.into(),
            object_ids,
            data: Value::Object(Map::new()),
        }
    }
}
