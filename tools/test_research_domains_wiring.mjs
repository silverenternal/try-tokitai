import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const read = (file) => fs.readFileSync(path.join(root, file), "utf8");

const index = read("frontend/index.html");
const app = read("frontend/app.js");
const domains = read("frontend/research-domains.js");
const domainSpecs = read("frontend/research-domain-specs.js");
const workbenches = read("frontend/research-workbenches.js");
const domain3d = read("frontend/domain-3d-viewer.js");
const visualization = read("frontend/visualization.js");
const styles = read("frontend/styles.css");
const web = read("src/web.rs");
const registry = read("src/research_domains/registry.rs");
const providers = read("src/research_domains/providers.rs");

const expect = (condition, message) => {
  if (!condition) throw new Error(message);
};

for (const id of [
  "research-domains-sidebar",
  "research-domains-nav",
  "research-domain-workspace",
  "research-domain-assets",
  "research-domain-global-tabs",
  "research-domain-native-actions",
  "research-domain-native-surface",
  "research-domain-object-types",
  "research-domain-agents",
  "research-domain-adapters",
  "research-domain-runs",
  "research-domain-viewer-tabs",
  "research-domain-preview-canvas",
  "research-domain-3d-canvas",
]) {
  expect(index.includes(`id="${id}"`), `missing Research Domains element #${id}`);
}

expect(index.includes("research-domains.js"), "Research Domains frontend module is not loaded");
expect(index.includes("research-domain-specs.js"), "domain-specific workspace specifications are not loaded");
expect(index.includes("research-workbenches.js"), "professional domain workbench renderers are not loaded");
expect(index.includes("domain-3d-viewer.js"), "shared domain 3D viewer is not loaded");
expect(index.indexOf("domain-3d-viewer.js") < index.indexOf("research-domains.js"), "shared domain 3D viewer must load before domain workbenches");
expect(app.includes('currentMainView === "domain"'), "domain workspace is not integrated with Atlas main-view routing");
expect(app.includes('window.addEventListener("atlas:research-domain-open"'), "Research Preview cannot open a domain workspace");
expect(app.includes('window.addEventListener("atlas:visualization-document-open"'), "domain documents are not routed into the shared visualization engine");
expect(visualization.includes("async function openDocument"), "shared visualization engine cannot accept domain documents");
expect(visualization.includes("renderGeometry"), "shared visualization engine has no real mesh/point projection path");
expect(domains.includes("syncPreview"), "task completion does not generate Research Preview cards");
expect(domains.includes("1500"), "domain workspace live synchronization cadence is not wired");
expect(domains.includes("DOMAIN_ICON_MARKUP"), "Research Domains rail does not register icon-only domain controls");
expect(domains.includes("renderWorkbenchView"), "domain preview still lacks renderer-specific workbench dispatch");
expect(domains.includes("openNativeAction"), "native domain operations are not connected to the Action Catalog");
expect(domains.includes("/api/research-domains/actions/run"), "native workbench actions do not use the server whitelist");
expect(domains.includes("openDomainTaskDialog"), "domain workbenches do not expose the structured text-to-task entrypoint");
expect(domains.includes("/api/research-domains/tasks/begin"), "Agent tasks are not persisted before dispatch");
expect(domains.includes("dispatchAgentTask"), "persistent domain tasks are not routed to the Agent runtime");
expect(domains.includes("dispatchAgentAction"), "generative Agent workflows are not separated from native actions");
expect(domains.includes("persistWorkspaceState"), "domain UI state is not synchronized with Agent context");
expect(domains.includes("buildDomainPreviewCard"), "domain-specific Research Preview Cards are not implemented");
expect(domains.includes("openArtifact"), "workspace-backed Agent cards cannot resolve their Research Domain");
expect(app.includes("data-open-research-artifact"), "workspace-backed Agent cards do not use the Research Domain artifact route");
expect(domains.includes('card.dataset.cardRoute = "research-domain"'), "Agent output cards do not declare their Research Domain route");
expect(domains.includes('detail: { domainId, assetId: asset.id, tab: "visualization", highlight: true }'), "Research Preview does not route to and highlight its domain artifact");
expect(domains.includes("preservedInteractivePreviewKind"), "Paper and Multi-Agent preview exceptions are not preserved");
expect(domains.includes('kind === "paper" || kind === "multi-agent"'), "Paper and Multi-Agent are not the explicit final-card routing exceptions");
expect(!app.includes("AtlasAlgorithmPreview"), "Agent completion still falls back to the removed Algorithm preview");
expect(app.includes('window.addEventListener("atlas:domain-agent-dispatch"'), "Agent dispatch is not connected to the main conversation runtime");
expect(app.includes('window.addEventListener("atlas:domain-open-asset"'), "domain assets cannot open in the workspace editor");
expect(domains.includes("AtlasDomain3D.mount"), "domain workbench does not reuse the shared 3D renderer");
expect(domain3d.includes("class Domain3DViewer"), "shared 3D renderer is missing its interactive viewer");
expect(domain3d.includes("rotation") && domain3d.includes("perspective"), "shared 3D renderer lacks depth-aware rotation");
expect(!domains.includes("COLLAPSE_KEY"), "Research Domains rail still carries the legacy expanded/collapsed sidebar state");
expect(styles.includes(".research-domains-sidebar"), "Research Domains sidebar does not inherit Atlas styling");
expect(styles.includes("padding-right: 56px"), "Research Domains rail width does not match the left activity rail");

for (const trait of [
  "IDomainPlugin",
  "IDataProvider",
  "IVisualizationProvider",
  "IAgentContextProvider",
  "IPreviewProvider",
  "IRenderProvider",
  "IExecutionProvider",
]) {
  expect(providers.includes(`trait ${trait}`), `missing provider contract ${trait}`);
}

for (const route of [
  "/api/research-domains",
  "/api/research-domains/workspace",
  "/api/research-domains/context",
  "/api/research-domains/visualization",
  "/api/research-domains/tasks",
]) {
  expect(web.includes(route), `missing Research Domains API route ${route}`);
}

for (const tool of [
  "research_domain_context",
  "research_domain_workspace",
  "research_domain_visualization",
  "research_domain_execution_context",
  "research_domain_task",
]) {
  expect(web.includes(tool), `Agent cannot access ${tool}`);
}

for (const domain of [
  "ai-ml",
  "computer-vision",
  "nlp",
  "computer-graphics",
  "cad",
  "robotics",
  "computer-networks",
  "operating-systems",
  "compiler",
  "database",
  "software-engineering",
  "program-analysis",
  "cyber-security",
  "hpc",
  "distributed-systems",
  "scientific-computing",
]) {
  expect(registry.includes(`"${domain}"`), `missing built-in domain plugin ${domain}`);
  expect(domainSpecs.includes(`"${domain}"`) || domain === "nlp" || domain === "hpc", `missing frontend workspace specification ${domain}`);
  expect(workbenches.includes(`"${domain}"`) || ["nlp", "hpc", "cad", "robotics", "compiler", "database"].includes(domain), `missing professional renderer ${domain}`);
}

expect(registry.includes('.join(".atlas").join("domains")'), "workspace plugin manifests are not dynamically loaded");
expect(registry.includes("content_revision"), "workspace-to-agent synchronization lacks content revisions");
expect(registry.includes("DomainWorkbenchStageDescriptor"), "domain workbenches do not expose workflow contracts");
expect(registry.includes("fn workflow_for"), "domain workbenches do not define domain-specific quality gates");
expect(registry.includes("fn intents_for"), "domain workbenches do not define domain-specific natural-language task contracts");
expect(registry.includes("text-to-parametric-model") && registry.includes("gpu-profile-study") && registry.includes("cluster-diagnosis"), "task contracts are not distributed across all research domains");
expect(web.includes("/api/research-domains/state"), "shared Research Domain workspace state API is missing");
expect(web.includes("research_domain_workspace_state"), "Agent cannot read or update live domain workspace state");
expect(web.includes("/api/research-domains/actions"), "Research Domain Action Catalog API is missing");
expect(web.includes("research_domain_action"), "Agent cannot list or run native Research Domain actions");
expect(web.includes("research_domain_task"), "Agent cannot read or advance persistent domain tasks");
expect(workbenches.includes("renderTab"), "common tabs are not routed through domain-native workbench IA");
expect(workbenches.includes("ml-experiment-console"), "ML workbench does not use an experiment-console composition");
expect(workbenches.includes("vision-annotation-console"), "Computer Vision workbench does not use an annotation-console composition");
expect(workbenches.includes("packet-analysis-console"), "Network workbench does not use packet/protocol/bytes composition");
expect(workbenches.includes("compiler-explorer-console"), "Compiler workbench does not correlate source, IR and assembly");
expect(workbenches.includes("database-query-console"), "Database workbench does not expose schema, SQL, results and plan composition");
expect(domainSpecs.includes("environmentContracts"), "domain environments do not own independent IA/runtime contracts");
for (const contractField of ["runtime", "agentApi", "selectionModel", "previewTarget", "navigation"]) {
  expect(domainSpecs.includes(`${contractField}:`), `domain environment contract is missing ${contractField}`);
}
for (const environmentApi of [
  "atlas.ml.experiment", "atlas.vision.annotation", "atlas.language.trace", "atlas.graphics.scene",
  "atlas.cad.document", "atlas.robotics.runtime", "atlas.network.capture", "atlas.system.trace",
  "atlas.compiler.pipeline", "atlas.database.session", "atlas.engineering.change", "atlas.analysis.target",
  "atlas.security.investigation", "atlas.hpc.profile", "atlas.distributed.cluster", "atlas.scientific.model",
]) {
  expect(domainSpecs.includes(environmentApi), `missing independent Agent API ${environmentApi}`);
}
expect(workbenches.includes("persistSelection"), "domain-native selections are not synchronized to Agent state");
expect(workbenches.includes("VIEWPORT_TOOLS"), "spatial environments do not expose professional tool modes");
expect(workbenches.includes("editableConsole"), "prompt/SQL/source environments do not expose editable runtime drafts");
expect(workbenches.includes("runMatrix"), "ML environment is missing experiment/run comparison semantics");
expect(styles.includes('data-domain="computer-networks"') && styles.includes('data-domain="compiler"') && styles.includes('data-domain="cad"'), "domain compositions do not have distinct visual grammar");
expect(domains.includes("atlas:research-environment-change"), "environment interaction changes are not broadcast to the live Agent bridge");
for (const tab of ["overview", "resources", "visualization", "artifacts", "history", "settings", "agent-context", "preview"]) {
  expect(domainSpecs.includes(`"${tab}"`), `missing common domain workspace tab ${tab}`);
}
for (const layout of [
  "experiment-lab", "vision-lab", "language-center", "graphics-studio", "engineering-studio",
  "robotics-lab", "network-operations", "system-monitor", "compiler-pipeline", "database-studio",
  "engineering-hub", "analysis-center", "security-operations", "compute-center",
  "distributed-cluster", "scientific-laboratory",
]) {
  expect(domainSpecs.includes(`layout: "${layout}"`), `missing distinct workspace layout ${layout}`);
}

console.log("Research Domains wiring checks passed");
