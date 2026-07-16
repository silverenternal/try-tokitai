import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");
const expect = (condition, message) => { if (!condition) throw new Error(message); };

const index = read("frontend/index.html");
const styles = read("frontend/styles.css");
const frontend = read("frontend/research-os.js");
const web = read("src/web.rs");
const ingestion = read("src/research_os/ingestion.rs");
const mod = read("src/research_os/mod.rs");

for (const id of [
  "research-os-panel", "research-os-toggle", "research-os-content", "research-os-tree",
  "research-os-inspector", "research-os-search", "research-os-warning", "research-os-scrubber",
]) expect(index.includes(`id="${id}"`), `missing Research OS element #${id}`);

for (const tab of ["graph", "hypotheses", "evidence", "lineage", "diary", "timeline", "decisions", "memory", "publication"])
  expect(index.includes(`data-tab="${tab}"`), `missing Research OS surface ${tab}`);

expect(frontend.includes("/api/research-os/snapshot"), "frontend does not use the unified Research OS snapshot");
expect(frontend.includes("renderKnowledgeGraph"), "knowledge graph renderer missing");
expect(frontend.includes("renderHypothesisLifecycle"), "hypothesis lifecycle renderer missing");
expect(frontend.includes("renderEvidenceEngine"), "evidence engine renderer missing");
expect(frontend.includes("renderExperimentLineage"), "experiment lineage renderer missing");
expect(frontend.includes("renderDiary"), "research diary renderer missing");
expect(frontend.includes("renderScientificTimeline"), "scientific timeline renderer missing");
expect(frontend.includes("renderDecisionBoard"), "decision engine renderer missing");
expect(frontend.includes("renderMemory"), "research memory renderer missing");
expect(frontend.includes("renderPublication"), "publication pipeline renderer missing");
expect(frontend.includes("renderInspectorStructured"), "structured inspector fields renderer missing");
expect(frontend.includes("renderInspectorRawMetadata"), "collapsible raw metadata renderer missing");
expect(frontend.includes("atlas:research-environment-change"), "Research OS does not synchronize domain changes");
expect(frontend.includes("atlas:domain-agent-dispatch"), "Research OS inspector cannot delegate to Atlas Agent");
expect(styles.includes("Research OS v2: graph-driven workbench"), "Atlas-native Research OS styling missing");
expect(styles.includes("grid-template-columns:210px minmax(390px,1fr) 250px"), "Research OS knowledge tree / canvas / inspector composition missing");
expect(styles.includes("research-os-evidence-chain"), "evidence chain styling missing");
expect(styles.includes("research-os-diary-entry"), "diary stream styling missing");

for (const route of ["/api/research-os/snapshot", "/api/research-os/graph", "/api/research-os/decisions", "/api/research-os/memory"])
  expect(web.includes(route), `missing Research OS API ${route}`);
expect(web.includes("research_os_snapshot_value"), "unified Research OS snapshot builder missing");
expect(web.includes("append_research_os_context_prompt"), "Research OS is not retrieved before Agent planning");
expect(web.includes('"research_os_snapshot"'), "Agent Research OS snapshot tool missing");
expect(web.includes('"research_os_mutate"'), "Agent Research OS mutate tool missing");
expect(web.includes("execute_research_os_mutate_tool"), "Research OS mutate dispatcher missing");
expect(web.includes("ingest_agent_turn(&workspace"), "chat turns are not ingested into Research OS");
expect(mod.includes("list_memory_entries"), "memory list API is not exported");
expect(mod.includes("execute_mutation"), "mutation dispatcher is not exported");
expect(ingestion.includes("existing_experiment"), "domain task ingestion is not idempotent");
expect(ingestion.includes("item.task_id.as_deref() == Some(task_id)"), "negative-result deduplication is missing");

const mutation = read("src/research_os/mutation.rs");
expect(mutation.includes("fn execute_mutation"), "mutation entry point missing");
for (const op of [
  "create_hypothesis", "update_hypothesis", "create_evidence", "create_experiment",
  "update_experiment", "create_negative_result", "create_decision", "create_memory",
  "create_publication", "update_publication", "link_objects",
]) expect(mutation.includes(`"${op}"`), `mutation dispatcher missing operation ${op}`);
expect(mutation.includes("without linked evidence"), "mutation module does not enforce evidence-before-validation guardrail");

console.log("Research OS wiring checks passed");
