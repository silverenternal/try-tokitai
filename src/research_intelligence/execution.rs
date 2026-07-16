use super::{
    ExecutionObservation, ExecutionStatus, ExecutionTaskSpec, FailureAnalysis, FailureCategory,
    RuntimeRequest, RuntimeResult,
};
use crate::atlas_core::{
    AtlasCore, AtlasEvent, AtlasEventKind, LifecycleState, RelationshipKind, ScientificObject,
};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

pub trait RuntimeAdapter: Send + Sync {
    fn runtime_object_id(&self) -> &str;
    fn capabilities(&self) -> BTreeSet<String>;
    fn available(&self) -> bool;
    fn execute(&self, request: &RuntimeRequest) -> Result<RuntimeResult>;
}

#[derive(Default)]
pub struct RuntimeRegistry {
    adapters: RwLock<Vec<Arc<dyn RuntimeAdapter>>>,
}

impl std::fmt::Debug for RuntimeRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("RuntimeRegistry")
            .field(
                "adapter_count",
                &self.adapters.read().map(|value| value.len()).unwrap_or(0),
            )
            .finish()
    }
}

impl RuntimeRegistry {
    pub fn register(&self, runtime: Arc<dyn RuntimeAdapter>) {
        if let Ok(mut adapters) = self.adapters.write() {
            adapters.retain(|existing| existing.runtime_object_id() != runtime.runtime_object_id());
            adapters.push(runtime);
        }
    }

    pub fn select(&self, capabilities: &BTreeSet<String>) -> Result<Arc<dyn RuntimeAdapter>> {
        self.adapters
            .read()
            .map_err(|_| anyhow!("runtime registry lock poisoned"))?
            .iter()
            .filter(|runtime| runtime.available())
            .filter(|runtime| capabilities.is_subset(&runtime.capabilities()))
            .min_by_key(|runtime| runtime.capabilities().len())
            .cloned()
            .ok_or_else(|| anyhow!("no available runtime satisfies the execution task"))
    }
}

#[derive(Debug, Clone)]
pub struct ExecutionEngine {
    core: Arc<AtlasCore>,
    runtimes: Arc<RuntimeRegistry>,
}

impl ExecutionEngine {
    pub fn new(core: Arc<AtlasCore>, runtimes: Arc<RuntimeRegistry>) -> Self {
        Self { core, runtimes }
    }

    pub fn runtimes(&self) -> Arc<RuntimeRegistry> {
        self.runtimes.clone()
    }

    pub fn register_runtime(
        &self,
        runtime: Arc<dyn RuntimeAdapter>,
        actor: &str,
    ) -> Result<ScientificObject> {
        let mut object = ScientificObject::new("runtime", runtime.runtime_object_id(), actor);
        object.id = runtime.runtime_object_id().to_string();
        object.description = "Scientific Runtime adapter".into();
        object.lifecycle = if runtime.available() {
            LifecycleState::Active
        } else {
            LifecycleState::Blocked
        };
        object.metadata.insert(
            "capabilities".into(),
            serde_json::to_value(runtime.capabilities())?,
        );
        object
            .metadata
            .insert("available".into(), json!(runtime.available()));
        object.rebuild_search_index();
        let object = self.core.sync_external(object, actor)?;
        self.runtimes.register(runtime);
        Ok(object)
    }

    pub fn create_task(&self, spec: &ExecutionTaskSpec, actor: &str) -> Result<ScientificObject> {
        for dependency in &spec.dependencies {
            self.core.get(dependency)?;
        }
        let mut object = ScientificObject::new("execution-task", &spec.title, actor);
        object.description = spec.goal.clone();
        object
            .metadata
            .insert("spec".into(), serde_json::to_value(spec)?);
        object
            .metadata
            .insert("status".into(), json!(ExecutionStatus::Planned));
        object.runtime.requested_capabilities =
            spec.required_capabilities.iter().cloned().collect();
        object.rebuild_search_index();
        let object = self.core.create(object, actor)?;
        for dependency in &spec.dependencies {
            self.core.relate(
                &object.id,
                dependency,
                RelationshipKind::DependsOn,
                actor,
                BTreeMap::new(),
            )?;
        }
        for scientific_object in &spec.scientific_object_ids {
            self.core.relate(
                &object.id,
                scientific_object,
                RelationshipKind::Uses,
                actor,
                BTreeMap::new(),
            )?;
        }
        Ok(object)
    }

    pub fn execute(&self, task_object_id: &str, actor: &str) -> Result<ExecutionObservation> {
        let task = self.core.get(task_object_id)?;
        if !task.can_execute(actor) {
            return Err(anyhow!(
                "actor is not allowed to execute this scientific object"
            ));
        }
        ensure_dependencies_completed(&self.core, &task)?;
        let spec: ExecutionTaskSpec = serde_json::from_value(
            task.metadata
                .get("spec")
                .cloned()
                .ok_or_else(|| anyhow!("execution task has no spec"))?,
        )?;
        let runtime = self.runtimes.select(&spec.required_capabilities)?;
        self.core.update(
            task_object_id,
            &json!({
                "lifecycle": "running",
                "metadata": {"status": ExecutionStatus::Running},
                "runtime": {
                    "runtime_object_id": runtime.runtime_object_id(),
                    "requested_capabilities": spec.required_capabilities,
                    "configuration": spec.parameters,
                }
            }),
            actor,
        )?;
        self.core.record_event(AtlasEvent::new(
            AtlasEventKind::ExecutionStarted,
            actor,
            vec![task_object_id.to_string()],
        ))?;
        let request = RuntimeRequest {
            task_object_id: task_object_id.to_string(),
            required_capabilities: spec.required_capabilities,
            parameters: spec.parameters,
        };
        let result = runtime.execute(&request);
        let observation = match result {
            Ok(result) if result.success => ExecutionObservation {
                task_object_id: task_object_id.to_string(),
                status: ExecutionStatus::Completed,
                metrics: result.metrics,
                evidence_object_ids: vec![],
                artifact_paths: result.artifact_paths,
                failure: None,
            },
            Ok(result) => ExecutionObservation {
                task_object_id: task_object_id.to_string(),
                status: ExecutionStatus::Failed,
                metrics: result.metrics,
                evidence_object_ids: vec![],
                artifact_paths: result.artifact_paths,
                failure: Some(analyze_failure(&result.summary)),
            },
            Err(error) => ExecutionObservation {
                task_object_id: task_object_id.to_string(),
                status: ExecutionStatus::Failed,
                metrics: BTreeMap::new(),
                evidence_object_ids: vec![],
                artifact_paths: vec![],
                failure: Some(analyze_failure(&error.to_string())),
            },
        };
        self.record_observation(&observation, actor)?;
        Ok(observation)
    }

    pub fn record_observation(
        &self,
        observation: &ExecutionObservation,
        actor: &str,
    ) -> Result<()> {
        let lifecycle = match observation.status {
            ExecutionStatus::Completed => LifecycleState::Completed,
            ExecutionStatus::Failed => LifecycleState::Failed,
            ExecutionStatus::Blocked => LifecycleState::Blocked,
            ExecutionStatus::Running => LifecycleState::Running,
            _ => LifecycleState::Active,
        };
        self.core.update(
            &observation.task_object_id,
            &json!({
                "lifecycle": lifecycle,
                "metadata": {"status": observation.status, "observation": observation}
            }),
            actor,
        )?;
        self.core.record_event(AtlasEvent::new(
            AtlasEventKind::ExecutionFinished,
            actor,
            vec![observation.task_object_id.clone()],
        ))?;
        Ok(())
    }

    pub fn pause(&self, id: &str, actor: &str) -> Result<ScientificObject> {
        self.core
            .update(id, &json!({"metadata": {"status": "paused"}}), actor)
    }

    pub fn resume(&self, id: &str, actor: &str) -> Result<ExecutionObservation> {
        self.execute(id, actor)
    }

    pub fn retry(&self, id: &str, actor: &str) -> Result<ExecutionObservation> {
        self.execute(id, actor)
    }

    pub fn rollback(&self, id: &str, version: u64, actor: &str) -> Result<ScientificObject> {
        self.core.rollback(id, version, actor)
    }

    pub fn fork(&self, id: &str, actor: &str) -> Result<ScientificObject> {
        self.core.fork(id, actor, "execution branch")
    }

    pub fn clone_task(&self, id: &str, actor: &str) -> Result<ScientificObject> {
        self.core.clone_object(id, actor)
    }

    pub fn checkpoint(&self, id: &str, label: &str, actor: &str) -> Result<ScientificObject> {
        self.core.update(
            id,
            &json!({"metadata": {"checkpoint": {
                "label": label,
                "created_at": chrono::Utc::now().to_rfc3339()
            }}}),
            actor,
        )
    }

    pub fn execute_parallel(
        &self,
        task_ids: &[String],
        actor: &str,
    ) -> Vec<Result<ExecutionObservation>> {
        // Runtime adapters decide their own concurrency. The engine preserves
        // independent task/object histories and returns every branch result.
        task_ids
            .iter()
            .map(|task_id| self.execute(task_id, actor))
            .collect()
    }
}

fn ensure_dependencies_completed(core: &AtlasCore, task: &ScientificObject) -> Result<()> {
    for relationship in task.relationships.iter().filter(|relationship| {
        relationship.direction == crate::atlas_core::RelationshipDirection::Outgoing
            && relationship.kind == RelationshipKind::DependsOn
    }) {
        let dependency = core.get(&relationship.object_id)?;
        if dependency.lifecycle != LifecycleState::Completed {
            return Err(anyhow!(
                "execution dependency is not completed: {}",
                dependency.display_name
            ));
        }
    }
    Ok(())
}

fn analyze_failure(message: &str) -> FailureAnalysis {
    let lower = message.to_ascii_lowercase();
    let (category, strategy, retryable) =
        if lower.contains("out of memory") || lower.contains("oom") {
            (
                FailureCategory::MemoryOverflow,
                "reduce memory pressure or move to a larger runtime",
                true,
            )
        } else if lower.contains("timeout") || lower.contains("timed out") {
            (
                FailureCategory::Timeout,
                "checkpoint, reduce task scope, or increase the runtime limit",
                true,
            )
        } else if lower.contains("converg") || lower.contains("nan") {
            (
                FailureCategory::BadConvergence,
                "adjust optimizer, initialization, scale, or numerical tolerances",
                true,
            )
        } else if lower.contains("accuracy") || lower.contains("metric gate") {
            (
                FailureCategory::LowAccuracy,
                "inspect errors and run a controlled ablation",
                true,
            )
        } else if lower.contains("dataset") || lower.contains("data format") {
            (
                FailureCategory::InvalidDataset,
                "validate the immutable dataset manifest and split",
                false,
            )
        } else if lower.contains("busy") || lower.contains("unavailable") {
            (
                FailureCategory::RuntimeBusy,
                "select a compatible available runtime",
                true,
            )
        } else if lower.contains("not found") || lower.contains("missing") {
            (
                FailureCategory::MissingDependency,
                "install or configure the required capability",
                false,
            )
        } else {
            (
                FailureCategory::RuntimeFailure,
                "inspect runtime evidence and revise the execution strategy",
                true,
            )
        };
    FailureAnalysis {
        category,
        summary: message.to_string(),
        retryable,
        recommended_strategy: strategy.into(),
        parameter_patch: Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    struct TestRuntime;

    impl RuntimeAdapter for TestRuntime {
        fn runtime_object_id(&self) -> &str {
            "test-runtime"
        }
        fn capabilities(&self) -> BTreeSet<String> {
            BTreeSet::from(["python".into()])
        }
        fn available(&self) -> bool {
            true
        }
        fn execute(&self, request: &RuntimeRequest) -> Result<RuntimeResult> {
            Ok(RuntimeResult {
                runtime_object_id: self.runtime_object_id().into(),
                success: true,
                summary: "complete".into(),
                metrics: BTreeMap::from([("accuracy".into(), json!(0.96))]),
                artifact_paths: vec![format!("{}.json", request.task_object_id)],
                raw: Value::Null,
            })
        }
    }

    #[test]
    fn selects_runtime_and_records_execution_lifecycle() {
        let directory = tempdir().unwrap();
        let core = Arc::new(AtlasCore::open(directory.path()).unwrap());
        let engine = ExecutionEngine::new(core.clone(), Arc::new(RuntimeRegistry::default()));
        engine
            .register_runtime(Arc::new(TestRuntime), "agent")
            .unwrap();
        let task = engine
            .create_task(
                &ExecutionTaskSpec {
                    title: "Evaluate model".into(),
                    goal: "Measure accuracy".into(),
                    priority: 1,
                    dependencies: vec![],
                    scientific_object_ids: vec![],
                    required_capabilities: BTreeSet::from(["python".into()]),
                    expected_output_types: vec![],
                    metrics: BTreeMap::new(),
                    parameters: json!({}),
                },
                "agent",
            )
            .unwrap();
        let result = engine.execute(&task.id, "agent").unwrap();
        assert_eq!(result.status, ExecutionStatus::Completed);
        assert_eq!(
            core.get(&task.id).unwrap().lifecycle,
            LifecycleState::Completed
        );
        assert!(core
            .list()
            .unwrap()
            .iter()
            .any(|object| object.object_type.0 == "runtime"));
    }
}
