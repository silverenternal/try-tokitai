import fs from "node:fs";

const read = (path) => fs.readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
const html = read("frontend/index.html");
const app = read("frontend/app.js");
const notebook = read("frontend/notebook.js");
const web = read("src/web.rs");
const core = read("src/notebook.rs");

const checks = [
  ["Notebook rail entry", html.includes('data-activity-panel="notebook" id="notebook-rail-button"')],
  ["Notebook workspace", html.includes('id="atlas-notebook-workspace"')],
  ["Notebook frontend loaded", html.includes("./notebook.js")],
  ["Notebook Navigation-sized panel", html.includes('data-activity-panel-id="notebook"')],
  ["Notebook list API", web.includes('route("/api/notebooks"')],
  ["Notebook create API", web.includes('route("/api/notebooks/create"')],
  ["Notebook save API", web.includes('route("/api/notebooks/save"')],
  ["Notebook execute API", web.includes('route("/api/notebooks/execute"')],
  ["Scientific Object sync", /ScientificObject::new\(\s*"research-notebook"/.test(core)],
  ["Atlas toolchain execution", /runtime\s*\.\s*toolchains\s*\.\s*get\("python"\)/.test(web)],
  ["No embedded Jupyter UI", !notebook.includes("iframe") && !html.includes("jupyter.org")],
];

for (const [label, passed] of checks) {
  if (!passed) throw new Error(`Notebook wiring failed: ${label}`);
  console.log(`ok - ${label}`);
}
