# Atlas Core and Research Intelligence Engine

Atlas keeps its existing desktop UI, workspaces, chat, explorer, editor and terminal. The internal data path is now:

```text
Atlas IDE
  -> compatibility APIs
  -> Atlas Core
  -> Scientific Object Engine
  -> Research Intelligence Engine
  -> Runtime adapters
  -> object storage / legacy storage adapters
```

## Atlas Core

`src/atlas_core` defines the stable internal abstraction:

- `ScientificObject`: identity, type, display data, lifecycle, artifacts, runtime binding, preview, visualization, evidence, Agent context, permissions and search index.
- `AtlasCore`: create, update, delete, archive, clone, fork, compare, merge, export, preview, visualize, rollback, relationships, search, graph and timeline.
- `ObjectStore`: storage-independent persistence contract.
- `FileObjectStore`: current workspace-backed implementation under `.atlas/core`.
- `EventBus`: synchronous observable event contract for Core, RIE and plugins.
- adapters: stable mapping from existing domain workspaces, assets, tasks and Research OS records.

Object heads are mutable pointers to the latest revision. Every accepted mutation first creates an immutable numbered snapshot. Rollback creates a new head revision and never deletes later history.

Artifacts represent files owned by an object. Domain assets retain their familiar file paths in Explorer, while Atlas Core uses stable object IDs and artifact revisions internally.

Relationships are stored once and projected bidirectionally into both endpoint objects. The Core knowledge graph is generated from those relationships; it has no separately maintained graph data model.

## Research Intelligence Engine

`src/research_intelligence` is a headless layer over Atlas Core:

- `PlanningEngine` converts a research goal into versioned planning objects and an execution DAG.
- `ExecutionEngine` creates execution-task objects, selects runtime adapters by capabilities, records observations, analyzes failures, and supports pause, resume, retry, rollback, clone, fork, checkpoint and parallel dispatch.
- `RecommendationEngine` evaluates current objects and produces ranked, evidence-linked recommendation objects.
- `ObjectQueryEngine` executes structured object filters, natural-language object search and AOQL statements.
- `PluginRegistry` manages install, enable, disable, hot reload, unload and remove lifecycles for object, runtime, visualization, workspace, execution, recommendation, query, context and event contributions.

Example AOQL:

```text
FIND Experiment WHERE metadata.accuracy >= 0.95 AND text CONTAINS transformer LIMIT 20
```

## Compatibility migration

Existing UI endpoints still return their original JSON contracts. Before returning, compatibility adapters synchronize the same domain workspace, asset, task and Research OS records into Atlas Core using deterministic IDs. Re-reading an unchanged source is idempotent; changed source data becomes a new object revision.

The Agent keeps its legacy tools for backward compatibility and additionally receives:

- `atlas_object`
- `atlas_object_query`
- `atlas_research_plan`
- `atlas_recommend`

New features should use these object tools. Direct file tools remain only as compatibility and artifact implementation mechanisms.

## Extension rules

New scientific domains register new object types and providers without modifying Atlas Core. Core behavior must remain type-agnostic. Domain-specific validation belongs in plugins or RIE strategies; persistence, relationships, history, events, permissions, search and Agent context remain inherited Core behavior.
