use super::{AtlasEvent, ObjectRevision, Relationship, ScientificObject};
use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub trait ObjectStore: Send + Sync {
    fn write_head(&self, object: &ScientificObject) -> Result<()>;
    fn read_head(&self, id: &str) -> Result<ScientificObject>;
    fn list_heads(&self) -> Result<Vec<ScientificObject>>;
    fn append_revision(&self, revision: &ObjectRevision) -> Result<()>;
    fn read_revisions(&self, id: &str) -> Result<Vec<ObjectRevision>>;
    fn write_relationship(&self, relationship: &Relationship) -> Result<()>;
    fn delete_relationship(&self, id: &str) -> Result<()>;
    fn list_relationships(&self) -> Result<Vec<Relationship>>;
    fn append_event(&self, event: &AtlasEvent) -> Result<()>;
    fn list_events(&self) -> Result<Vec<AtlasEvent>>;
}

#[derive(Debug, Clone)]
pub struct FileObjectStore {
    root: PathBuf,
}

impl FileObjectStore {
    pub fn open(workspace_root: &Path) -> Result<Self> {
        let root = workspace_root.join(".atlas").join("core");
        for directory in ["objects", "history", "relationships", "events"] {
            fs::create_dir_all(root.join(directory))?;
        }
        Ok(Self { root })
    }

    fn object_path(&self, id: &str) -> Result<PathBuf> {
        validate_id(id)?;
        Ok(self.root.join("objects").join(format!("{id}.json")))
    }

    fn history_dir(&self, id: &str) -> Result<PathBuf> {
        validate_id(id)?;
        Ok(self.root.join("history").join(id))
    }

    fn relationship_path(&self, id: &str) -> Result<PathBuf> {
        validate_id(id)?;
        Ok(self.root.join("relationships").join(format!("{id}.json")))
    }

    fn event_path(&self, event: &AtlasEvent) -> Result<PathBuf> {
        validate_id(&event.id)?;
        let timestamp = event
            .timestamp
            .chars()
            .filter(|character| character.is_ascii_digit())
            .collect::<String>();
        Ok(self
            .root
            .join("events")
            .join(format!("{timestamp}-{}.json", event.id)))
    }
}

impl ObjectStore for FileObjectStore {
    fn write_head(&self, object: &ScientificObject) -> Result<()> {
        atomic_json(&self.object_path(&object.id)?, object)
    }

    fn read_head(&self, id: &str) -> Result<ScientificObject> {
        read_json(&self.object_path(id)?)
    }

    fn list_heads(&self) -> Result<Vec<ScientificObject>> {
        read_directory_json(&self.root.join("objects"))
    }

    fn append_revision(&self, revision: &ObjectRevision) -> Result<()> {
        let directory = self.history_dir(&revision.object_id)?;
        fs::create_dir_all(&directory)?;
        let path = directory.join(format!("{:020}.json", revision.version));
        if path.exists() {
            return Err(anyhow!("object revision already exists"));
        }
        atomic_json(&path, revision)
    }

    fn read_revisions(&self, id: &str) -> Result<Vec<ObjectRevision>> {
        let directory = self.history_dir(id)?;
        if !directory.exists() {
            return Ok(Vec::new());
        }
        read_directory_json(&directory)
    }

    fn write_relationship(&self, relationship: &Relationship) -> Result<()> {
        atomic_json(&self.relationship_path(&relationship.id)?, relationship)
    }

    fn delete_relationship(&self, id: &str) -> Result<()> {
        let path = self.relationship_path(id)?;
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn list_relationships(&self) -> Result<Vec<Relationship>> {
        read_directory_json(&self.root.join("relationships"))
    }

    fn append_event(&self, event: &AtlasEvent) -> Result<()> {
        atomic_json(&self.event_path(event)?, event)
    }

    fn list_events(&self) -> Result<Vec<AtlasEvent>> {
        read_directory_json(&self.root.join("events"))
    }
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(anyhow!("invalid Atlas object identifier"));
    }
    Ok(())
}

fn atomic_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(anyhow!("Atlas object record exceeds size limit"));
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, bytes)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary, path)?;
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(anyhow!("Atlas object record exceeds size limit"));
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn read_directory_json<T: serde::de::DeserializeOwned>(directory: &Path) -> Result<Vec<T>> {
    if !directory.exists() {
        return Ok(Vec::new());
    }
    let mut paths = fs::read_dir(directory)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    paths.sort();
    paths
        .iter()
        .map(|path| read_json(path))
        .collect::<Result<Vec<_>>>()
}
