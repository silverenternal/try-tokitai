import fs from "node:fs";

const html = fs.readFileSync(new URL("../frontend/index.html", import.meta.url), "utf8");
const app = fs.readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const renderer = fs.readFileSync(new URL("../frontend/visualization.js", import.meta.url), "utf8");
const domain3d = fs.readFileSync(new URL("../frontend/domain-3d-viewer.js", import.meta.url), "utf8");
const systemMonitor = fs.readFileSync(new URL("../frontend/system-monitor.js", import.meta.url), "utf8");
const web = fs.readFileSync(new URL("../src/web.rs", import.meta.url), "utf8");
const model = fs.readFileSync(new URL("../src/visualization/model.rs", import.meta.url), "utf8");
const registry = fs.readFileSync(new URL("../src/visualization/mod.rs", import.meta.url), "utf8");

for (const id of [
  "visualization-rail-button",
  "visualization-workspace",
  "visualization-canvas",
  "visualization-3d-canvas",
  "visualization-source-select",
  "visualization-play",
  "visualization-timeline",
  "visualization-chat-toggle",
  "visualization-floating-chat-head",
  "visualization-floating-chat-resize",
  "system-monitor-enabled",
  "system-metrics-bar",
]) {
  if (!html.includes(`id="${id}"`)) throw new Error(`missing visualization UI element: ${id}`);
}

for (const marker of [
  'nextView === "visualization"',
  '(currentMainView === "visualization" || currentMainView === "domain") || Boolean(dockLayout.hidden[panelId])',
  'appShell?.classList.toggle("is-visualization-fullscreen", isVisualization)',
  'syncVisualizationConversationLayout()',
  'startVisualizationChatResize',
  'window.addEventListener("atlas:visualization-open"',
  'window.AtlasVisualization?.activate()',
  'window.addEventListener("atlas:visualization-close"',
  'classList.toggle("is-system-performance", isSystemPerformance)',
]) {
  if (!app.includes(marker)) throw new Error(`missing IDE visualization wiring: ${marker}`);
}

if (!app.includes("syncResearchDomainFinalOutputCard")) {
  throw new Error("Agent final-output cards are not delegated to Research Domains");
}
if (app.includes("AtlasAlgorithmPreview") || html.includes("visualization-preview.js")) {
  throw new Error("legacy Algorithm final-output preview is still loaded");
}

for (const marker of [
  'kind: "system", source_id: "runtime:system"',
  '["cpu", "gpu", "memory", "disk", "network"]',
  "node?.metrics?.value",
]) {
  if (!systemMonitor.includes(marker)) throw new Error(`missing real system monitor behavior: ${marker}`);
}
if (html.includes('id="workspace-code-rename"')) {
  throw new Error("code panel must not expose rename controls");
}
if (!html.includes('<section class="git-workspace" id="git-workspace">')) {
  throw new Error("Git workspace must live in the Source Control flyout");
}
if (html.includes('<section class="git-workspace" id="git-workspace" hidden>') || app.includes('setMainView("git")')) {
  throw new Error("Git workspace must not use a standalone main page");
}

for (const marker of [
  '"/api/visualizations"',
  '"/api/visualizations/snapshot"',
  "VisualizationRegistry::default()",
  "visualization_runtime_payload",
]) {
  if (!web.includes(marker)) throw new Error(`missing visualization API wiring: ${marker}`);
}

for (const marker of [
  "pub nodes: Vec<VisualizationNode>",
  "pub edges: Vec<VisualizationEdge>",
  "pub series: Vec<VisualizationSeries>",
  "pub events: Vec<VisualizationEvent>",
  "pub frames: Vec<VisualizationFrame>",
]) {
  if (!model.includes(marker)) throw new Error(`missing shared schema field: ${marker}`);
}
if (!model.includes("pub node_id: Option<String>")) {
  throw new Error("metric series cannot associate with shared visualization nodes");
}

for (const marker of [
  "pub trait VisualizationAdapter",
  "fn discover(&self",
  "fn parse(&self",
  "pub fn register<A>",
]) {
  if (!registry.includes(marker)) throw new Error(`missing parser plugin contract: ${marker}`);
}
if (registry.includes("registry.register(AlgorithmAdapter)") || registry.includes("registry.register(NetworkAdapter)")) {
  throw new Error("Algorithm or Network is still registered in Interactive Visualization");
}
for (const marker of ["registry.register(SystemAdapter::default())", "registry.register(PaperAdapter)", "registry.register(MultiAgentAdapter)"]) {
  if (!registry.includes(marker)) throw new Error(`missing retained visualization adapter: ${marker}`);
}

for (const marker of [
  "state.document?.frames",
  "state.document || {}",
  "document.nodes || []",
  "document.edges || []",
  "document.series || []",
  "registerRenderExtension",
  "deriveLiveSeriesFrames(document)",
  'node.metadata?.presentation === "neural-layer"',
  "AtlasDomain3D.mount",
]) {
  if (!renderer.includes(marker)) throw new Error(`renderer is not bound to the shared document: ${marker}`);
}

if (!domain3d.includes("pointerdown") || !domain3d.includes("wheel") || !domain3d.includes("fit()")) {
  throw new Error("shared 3D renderer lacks rotate, zoom, or fit interaction");
}

if (/demoData|mockData|sampleNodes|fixedNodes|sortingExample/i.test(renderer)) {
  throw new Error("renderer contains a demo or fixed-data implementation");
}
if (/algorithmView|atlas:algorithm-visualization-ready|__ATLAS_PENDING_ALGORITHM_VISUALIZATION__/.test(renderer)) {
  throw new Error("legacy Algorithm-specific renderer state is still present");
}
if (html.includes('id="visualization-algorithm-view"')) {
  throw new Error("legacy Algorithm toolbar is still present");
}

if (!renderer.includes('filter((type) => type.kind !== "system")')) {
  throw new Error("System Performance detail page must be removed from visualization tabs");
}
if (html.includes('id="visualization-performance"') || html.includes("visualization-performance.js")) {
  throw new Error("System Performance detail renderer must not be loaded");
}

console.log("interactive visualization wiring is valid");
