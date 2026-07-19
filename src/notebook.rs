use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use uuid::Uuid;

use crate::atlas_core::{AtlasCore, LifecycleState, ScientificObject};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotebookCellKind {
    Markdown,
    Python,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookCell {
    pub id: String,
    pub kind: NotebookCellKind,
    pub source: String,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub execution_count: u64,
    #[serde(default)]
    pub duration_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasNotebook {
    pub schema: String,
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub kernel: String,
    pub cells: Vec<NotebookCell>,
    #[serde(default)]
    pub tags: Vec<String>,
}
pub struct NotebookCore {
    workspace: PathBuf,
    dir: PathBuf,
}
impl NotebookCore {
    pub fn open(workspace: &Path) -> Result<Self> {
        let dir = workspace.join(".atlas").join("notebooks");
        fs::create_dir_all(&dir)?;
        Ok(Self {
            workspace: workspace.into(),
            dir,
        })
    }
    pub fn list(&self) -> Result<Vec<AtlasNotebook>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(&self.dir)? {
            let path = entry?.path();
            if path.extension().and_then(|v| v.to_str()) == Some("json") {
                if let Ok(value) = serde_json::from_slice(&fs::read(path)?) {
                    out.push(value)
                }
            }
        }
        out.sort_by(|a: &AtlasNotebook, b| b.updated_at.cmp(&a.updated_at));
        Ok(out)
    }
    pub fn create(&self, title: Option<&str>) -> Result<AtlasNotebook> {
        let now = Utc::now().to_rfc3339();
        let notebook=AtlasNotebook{schema:"atlas.notebook.v1".into(),id:Uuid::new_v4().to_string(),title:title.unwrap_or("Untitled Research Notebook").into(),created_at:now.clone(),updated_at:now,kernel:"python".into(),cells:vec![NotebookCell{id:Uuid::new_v4().to_string(),kind:NotebookCellKind::Markdown,source:"# Research Notebook\n\nDescribe the question, assumptions and expected evidence.".into(),output:String::new(),status:"idle".into(),execution_count:0,duration_ms:0},NotebookCell{id:Uuid::new_v4().to_string(),kind:NotebookCellKind::Python,source:"print(\"Atlas kernel ready\")".into(),output:String::new(),status:"idle".into(),execution_count:0,duration_ms:0}],tags:vec![]};
        self.save(notebook)
    }
    pub fn get(&self, id: &str) -> Result<AtlasNotebook> {
        validate_id(id)?;
        Ok(serde_json::from_slice(&fs::read(
            self.dir.join(format!("{id}.json")),
        )?)?)
    }
    pub fn save(&self, mut notebook: AtlasNotebook) -> Result<AtlasNotebook> {
        validate_id(&notebook.id)?;
        if notebook.cells.len() > 500 {
            return Err(anyhow!("notebook has too many cells"));
        }
        for cell in &notebook.cells {
            if cell.source.len() > 1024 * 1024 {
                return Err(anyhow!("notebook cell is too large"));
            }
        }
        notebook.updated_at = Utc::now().to_rfc3339();
        let path = self.dir.join(format!("{}.json", notebook.id));
        let temp = path.with_extension("tmp");
        fs::write(&temp, serde_json::to_vec_pretty(&notebook)?)?;
        if path.exists() {
            fs::remove_file(&path)?;
        }
        fs::rename(temp, path)?;
        self.sync_object(&notebook)?;
        Ok(notebook)
    }
    pub fn execute_cell(&self, id: &str, cell_id: &str, python: &str) -> Result<AtlasNotebook> {
        let mut notebook = self.get(id)?;
        let cell = notebook
            .cells
            .iter_mut()
            .find(|c| c.id == cell_id)
            .ok_or_else(|| anyhow!("notebook cell not found"))?;
        if !matches!(cell.kind, NotebookCellKind::Python) {
            return Err(anyhow!("only Python cells can execute"));
        }
        let script = tempfile::NamedTempFile::new_in(&self.dir)?;
        fs::write(script.path(), &cell.source)?;
        let started = Instant::now();
        let output = Command::new(python)
            .arg(script.path())
            .current_dir(&self.workspace)
            .output()
            .with_context(|| format!("failed to start Python kernel {python}"))?;
        cell.duration_ms = started.elapsed().as_millis() as u64;
        cell.execution_count += 1;
        cell.status = if output.status.success() {
            "completed"
        } else {
            "failed"
        }
        .into();
        cell.output = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if cell.output.len() > 512 * 1024 {
            cell.output.truncate(512 * 1024);
        }
        self.save(notebook)
    }
    fn sync_object(&self, notebook: &AtlasNotebook) -> Result<()> {
        let core = AtlasCore::open(&self.workspace)?;
        let mut object = ScientificObject::new(
            "research-notebook",
            notebook.title.clone(),
            "atlas-notebook",
        );
        object.id = format!("notebook-{}", notebook.id);
        object.lifecycle = LifecycleState::Active;
        object.description = format!(
            "{} cells · {} kernel",
            notebook.cells.len(),
            notebook.kernel
        );
        object
            .metadata
            .insert("notebook_id".into(), json!(notebook.id));
        object
            .metadata
            .insert("cell_count".into(), json!(notebook.cells.len()));
        object.preview.provider = "atlas.notebook".into();
        object.preview.kind = "computational-notebook".into();
        object.preview.payload = json!({"cells":notebook.cells.iter().map(|c|json!({"kind":c.kind,"status":c.status,"execution_count":c.execution_count})).collect::<Vec<_>>()});
        core.sync_external(object, "atlas-notebook")?;
        Ok(())
    }
}
fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(anyhow!("invalid notebook id"));
    }
    Ok(())
}
