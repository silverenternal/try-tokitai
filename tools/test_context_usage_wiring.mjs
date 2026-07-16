import fs from "node:fs";

const app = fs.readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const web = fs.readFileSync(new URL("../src/web.rs", import.meta.url), "utf8");

for (const marker of [
  "context_usage: WebContextUsage",
  "context_used_tokens: usize",
  "context_usage_estimated: bool",
  "context_model: String",
  "update_runtime_context_usage(",
  "estimate_text_tokens(&Value::Array(tools.clone()).to_string())",
]) {
  if (!web.includes(marker)) throw new Error(`missing backend context usage marker: ${marker}`);
}

for (const marker of [
  "applyContextUsageSnapshot(bootstrapData.context_usage)",
  "snapshot.context_used_tokens",
  'contextUsage.dataset.estimated = lastContextUsage.estimated ? "true" : "false"',
  'contextUsage.dataset.model = lastContextUsage.model || ""',
  'document.getElementById("context-usage-model")',
]) {
  if (!app.includes(marker)) throw new Error(`missing frontend context usage marker: ${marker}`);
}

console.log("context usage wiring is valid");
