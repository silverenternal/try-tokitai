use super::{
    AtlasEvent, AtlasEventKind, EventBus, FileObjectStore, LifecycleState, ObjectId,
    ObjectRevision, ObjectStore, Relationship, RelationshipDirection, RelationshipKind,
    RelationshipRef, ScientificObject,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObjectComparison {
    pub object_id: ObjectId,
    pub left_version: u64,
    pub right_version: u64,
    pub changed_fields: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ObjectGraph {
    pub objects: Vec<ScientificObject>,
    pub relationships: Vec<Relationship>,
}

pub struct AtlasCore {
    store: Arc<dyn ObjectStore>,
    events: Arc<EventBus>,
}

impl std::fmt::Debug for AtlasCore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("AtlasCore").finish_non_exhaustive()
    }
}

impl AtlasCore {
    pub fn open(workspace_root: &Path) -> Result<Self> {
        Ok(Self::new(Arc::new(FileObjectStore::open(workspace_root)?)))
    }

    pub fn new(store: Arc<dyn ObjectStore>) -> Self {
        Self {
            store,
            events: Arc::new(EventBus::default()),
        }
    }

    pub fn event_bus(&self) -> Arc<EventBus> {
        self.events.clone()
    }

    pub fn create(&self, mut object: ScientificObject, actor: &str) -> Result<ScientificObject> {
        if self.store.read_head(&object.id).is_ok() {
            return Err(anyhow!("scientific object already exists"));
        }
        object.version = 1;
        let now = Utc::now().to_rfc3339();
        object.created_at = now.clone();
        object.updated_at = now;
        object.rebuild_search_index();
        self.commit_head(object, actor, "create", AtlasEventKind::ObjectCreated, None)
    }

    /// Import or synchronize an object owned by a compatibility adapter.
    /// Stable source IDs make this idempotent; changed source data creates a
    /// new immutable Atlas revision instead of overwriting history.
    pub fn sync_external(
        &self,
        mut incoming: ScientificObject,
        actor: &str,
    ) -> Result<ScientificObject> {
        let Ok(current) = self.get(&incoming.id) else {
            return self.create(incoming, actor);
        };
        incoming.created_at = current.created_at.clone();
        incoming.updated_at = current.updated_at.clone();
        incoming.version = current.version;
        incoming.relationships = current.relationships.clone();
        if comparable_object(&incoming) == comparable_object(&current) {
            return Ok(current);
        }
        incoming.version = current.version + 1;
        incoming.updated_at = Utc::now().to_rfc3339();
        self.commit_head(
            incoming,
            actor,
            "external source synchronization",
            AtlasEventKind::ObjectUpdated,
            Some(current.version),
        )
    }

    pub fn get(&self, id: &str) -> Result<ScientificObject> {
        self.store.read_head(id)
    }

    pub fn list(&self) -> Result<Vec<ScientificObject>> {
        self.store.list_heads()
    }

    pub fn update(&self, id: &str, patch: &Value, actor: &str) -> Result<ScientificObject> {
        let mut object = self.get(id)?;
        if !object.can_write(actor) {
            return Err(anyhow!(
                "actor is not allowed to update this scientific object"
            ));
        }
        let parent = object.version;
        object.patch(patch)?;
        object.version += 1;
        object.updated_at = Utc::now().to_rfc3339();
        self.commit_head(
            object,
            actor,
            "update",
            AtlasEventKind::ObjectUpdated,
            Some(parent),
        )
    }

    pub fn archive(&self, id: &str, actor: &str) -> Result<ScientificObject> {
        self.update(id, &json!({"lifecycle": "archived"}), actor)
    }

    pub fn delete(&self, id: &str, actor: &str) -> Result<ScientificObject> {
        let object = self.update(id, &json!({"lifecycle": "deleted"}), actor)?;
        self.emit(AtlasEvent::new(
            AtlasEventKind::ObjectDeleted,
            actor,
            vec![id.to_string()],
        ))?;
        Ok(object)
    }

    pub fn clone_object(&self, id: &str, actor: &str) -> Result<ScientificObject> {
        self.fork(id, actor, "clone")
    }

    pub fn fork(&self, id: &str, actor: &str, reason: &str) -> Result<ScientificObject> {
        let source = self.get(id)?;
        if !source.can_read(actor) {
            return Err(anyhow!(
                "actor is not allowed to fork this scientific object"
            ));
        }
        let mut fork = source.clone();
        fork.id = Uuid::new_v4().to_string();
        fork.version = 1;
        fork.display_name = format!("{} (fork)", fork.display_name);
        fork.relationships.clear();
        fork.lifecycle = LifecycleState::Draft;
        let fork = self.create(fork, actor)?;
        self.relate(
            &fork.id,
            id,
            RelationshipKind::ForkOf,
            actor,
            BTreeMap::new(),
        )?;
        let mut event = AtlasEvent::new(
            AtlasEventKind::ObjectForked,
            actor,
            vec![id.to_string(), fork.id.clone()],
        );
        event.data = json!({"reason": reason});
        self.emit(event)?;
        Ok(fork)
    }

    pub fn merge(&self, target_id: &str, source_id: &str, actor: &str) -> Result<ScientificObject> {
        let source = self.get(source_id)?;
        let mut target = self.get(target_id)?;
        if !source.can_read(actor) || !target.can_write(actor) {
            return Err(anyhow!(
                "actor is not allowed to merge these scientific objects"
            ));
        }
        let parent = target.version;
        target.metadata.extend(source.metadata.clone());
        target.tags.extend(source.tags.clone());
        for artifact in source.artifacts {
            if !target
                .artifacts
                .iter()
                .any(|existing| existing.id == artifact.id)
            {
                target.artifacts.push(artifact);
            }
        }
        for evidence in source.evidence {
            if !target
                .evidence
                .iter()
                .any(|existing| existing.evidence_object_id == evidence.evidence_object_id)
            {
                target.evidence.push(evidence);
            }
        }
        target.version += 1;
        target.updated_at = Utc::now().to_rfc3339();
        let target = self.commit_head(
            target,
            actor,
            "merge",
            AtlasEventKind::ObjectMerged,
            Some(parent),
        )?;
        self.relate(
            &target.id,
            source_id,
            RelationshipKind::DerivedFrom,
            actor,
            BTreeMap::new(),
        )?;
        Ok(target)
    }

    pub fn export(&self, id: &str) -> Result<Value> {
        Ok(serde_json::to_value(self.get(id)?)?)
    }

    pub fn serialize(&self, id: &str) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(&self.get(id)?)?)
    }

    pub fn deserialize(&self, bytes: &[u8], actor: &str) -> Result<ScientificObject> {
        self.create(serde_json::from_slice(bytes)?, actor)
    }

    pub fn preview(&self, id: &str) -> Result<Value> {
        let object = self.get(id)?;
        Ok(json!({
            "object_id": object.id,
            "object_type": object.object_type,
            "display_name": object.display_name,
            "provider": object.preview.provider,
            "kind": object.preview.kind,
            "payload": object.preview.payload,
            "artifacts": object.artifacts,
        }))
    }

    pub fn visualize(&self, id: &str) -> Result<Value> {
        let object = self.get(id)?;
        Ok(json!({"object_id": object.id, "mappings": object.visualizations}))
    }

    pub fn rollback(&self, id: &str, version: u64, actor: &str) -> Result<ScientificObject> {
        let snapshot = self
            .store
            .read_revisions(id)?
            .into_iter()
            .find(|revision| revision.version == version)
            .ok_or_else(|| anyhow!("unknown scientific object version"))?
            .snapshot;
        let current = self.get(id)?;
        let mut next = snapshot;
        next.version = current.version + 1;
        next.updated_at = Utc::now().to_rfc3339();
        self.commit_head(
            next,
            actor,
            &format!("rollback to version {version}"),
            AtlasEventKind::ObjectUpdated,
            Some(current.version),
        )
    }

    pub fn compare(&self, id: &str, left: u64, right: u64) -> Result<ObjectComparison> {
        let revisions = self.store.read_revisions(id)?;
        let find = |version| {
            revisions
                .iter()
                .find(|revision| revision.version == version)
                .map(|revision| serde_json::to_value(&revision.snapshot))
                .transpose()
        };
        let left_value = find(left)?.ok_or_else(|| anyhow!("left version not found"))?;
        let right_value = find(right)?.ok_or_else(|| anyhow!("right version not found"))?;
        let mut changed_fields = BTreeMap::new();
        let keys = left_value
            .as_object()
            .into_iter()
            .flat_map(|value| value.keys())
            .chain(
                right_value
                    .as_object()
                    .into_iter()
                    .flat_map(|value| value.keys()),
            )
            .cloned()
            .collect::<BTreeSet<_>>();
        for key in keys {
            if left_value.get(&key) != right_value.get(&key) {
                changed_fields.insert(
                    key.clone(),
                    json!({"left": left_value.get(&key), "right": right_value.get(&key)}),
                );
            }
        }
        Ok(ObjectComparison {
            object_id: id.to_string(),
            left_version: left,
            right_version: right,
            changed_fields,
        })
    }

    pub fn relate(
        &self,
        source_id: &str,
        target_id: &str,
        kind: RelationshipKind,
        actor: &str,
        metadata: BTreeMap<String, Value>,
    ) -> Result<Relationship> {
        if source_id == target_id {
            return Err(anyhow!("scientific object cannot relate to itself"));
        }
        let mut source = self.get(source_id)?;
        let mut target = self.get(target_id)?;
        if !source.can_write(actor) || !target.can_write(actor) {
            return Err(anyhow!(
                "actor is not allowed to relate these scientific objects"
            ));
        }
        if let Some(existing) = self
            .store
            .list_relationships()?
            .into_iter()
            .find(|relationship| {
                relationship.source_id == source_id
                    && relationship.target_id == target_id
                    && relationship.kind == kind
            })
        {
            return Ok(existing);
        }
        let relationship = Relationship {
            id: Uuid::new_v4().to_string(),
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            kind: kind.clone(),
            created_at: Utc::now().to_rfc3339(),
            created_by: actor.to_string(),
            metadata,
        };
        self.store.write_relationship(&relationship)?;
        source.relationships.push(RelationshipRef {
            relationship_id: relationship.id.clone(),
            object_id: target_id.to_string(),
            kind: kind.clone(),
            direction: RelationshipDirection::Outgoing,
        });
        target.relationships.push(RelationshipRef {
            relationship_id: relationship.id.clone(),
            object_id: source_id.to_string(),
            kind: kind.inverse(),
            direction: RelationshipDirection::Incoming,
        });
        self.replace_without_event(source, actor, "relationship created")?;
        self.replace_without_event(target, actor, "relationship created")?;
        let mut event = AtlasEvent::new(
            AtlasEventKind::RelationshipCreated,
            actor,
            vec![source_id.to_string(), target_id.to_string()],
        );
        event.data = serde_json::to_value(&relationship)?;
        self.emit(event)?;
        Ok(relationship)
    }

    pub fn graph(&self) -> Result<ObjectGraph> {
        Ok(ObjectGraph {
            objects: self.store.list_heads()?,
            relationships: self.store.list_relationships()?,
        })
    }

    pub fn timeline(&self, object_id: Option<&str>) -> Result<Vec<AtlasEvent>> {
        let mut events = self.store.list_events()?;
        if let Some(object_id) = object_id {
            events.retain(|event| event.object_ids.iter().any(|id| id == object_id));
        }
        events.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
        Ok(events)
    }

    pub fn record_event(&self, event: AtlasEvent) -> Result<()> {
        self.emit(event)
    }

    pub fn search(&self, query: &str) -> Result<Vec<ScientificObject>> {
        let terms = query
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        let mut objects = self.store.list_heads()?;
        objects.retain(|object| {
            object.lifecycle != LifecycleState::Deleted
                && terms
                    .iter()
                    .all(|term| object.search_index.iter().any(|token| token.contains(term)))
        });
        objects.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(objects)
    }

    fn replace_without_event(
        &self,
        mut object: ScientificObject,
        actor: &str,
        reason: &str,
    ) -> Result<()> {
        let parent = object.version;
        object.version += 1;
        object.updated_at = Utc::now().to_rfc3339();
        self.store.write_head(&object)?;
        self.store.append_revision(&ObjectRevision {
            object_id: object.id.clone(),
            version: object.version,
            parent_version: Some(parent),
            created_at: object.updated_at.clone(),
            created_by: actor.to_string(),
            reason: reason.to_string(),
            snapshot: object,
        })
    }

    fn commit_head(
        &self,
        object: ScientificObject,
        actor: &str,
        reason: &str,
        event_kind: AtlasEventKind,
        parent_version: Option<u64>,
    ) -> Result<ScientificObject> {
        self.store.write_head(&object)?;
        self.store.append_revision(&ObjectRevision {
            object_id: object.id.clone(),
            version: object.version,
            parent_version,
            created_at: object.updated_at.clone(),
            created_by: actor.to_string(),
            reason: reason.to_string(),
            snapshot: object.clone(),
        })?;
        self.emit(AtlasEvent::new(event_kind, actor, vec![object.id.clone()]))?;
        Ok(object)
    }

    fn emit(&self, event: AtlasEvent) -> Result<()> {
        self.store.append_event(&event)?;
        self.events.publish(&event)
    }
}

fn comparable_object(object: &ScientificObject) -> Value {
    let mut value = serde_json::to_value(object).unwrap_or(Value::Null);
    if let Some(map) = value.as_object_mut() {
        for key in ["version", "created_at", "updated_at", "relationships"] {
            map.remove(key);
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn object_lifecycle_versions_and_relationships_are_persistent() {
        let directory = tempdir().unwrap();
        let core = AtlasCore::open(directory.path()).unwrap();
        let first = core
            .create(
                ScientificObject::new("experiment", "Run A", "tester"),
                "tester",
            )
            .unwrap();
        let second = core
            .create(
                ScientificObject::new("dataset", "Data A", "tester"),
                "tester",
            )
            .unwrap();
        let updated = core
            .update(
                &first.id,
                &json!({"description": "baseline", "metadata": {"accuracy": 0.9}}),
                "tester",
            )
            .unwrap();
        assert_eq!(updated.version, 2);
        core.relate(
            &first.id,
            &second.id,
            RelationshipKind::Uses,
            "tester",
            BTreeMap::new(),
        )
        .unwrap();
        let graph = core.graph().unwrap();
        assert_eq!(graph.relationships.len(), 1);
        assert_eq!(core.get(&first.id).unwrap().relationships.len(), 1);
        assert_eq!(core.get(&second.id).unwrap().relationships.len(), 1);
        assert!(!core.timeline(Some(&first.id)).unwrap().is_empty());
        let comparison = core.compare(&first.id, 1, 2).unwrap();
        assert!(comparison.changed_fields.contains_key("description"));
    }
}
