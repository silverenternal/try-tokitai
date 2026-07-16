import fs from "node:fs";

const app = fs.readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const web = fs.readFileSync(new URL("../src/web.rs", import.meta.url), "utf8");
const prompt = fs.readFileSync(new URL("../src/domain_prompt.rs", import.meta.url), "utf8");

for (const marker of [
  "let turn_model = lock_runtime_settings",
  "runtime.model = non_empty_or(turn_model.trim(), &runtime.model)",
  "context_model: String",
  'Some(format!("{}|{}", context_window, request.model))',
  "session_manager.update_model_for(&session_id, &runtime.model)",
]) {
  if (!web.includes(marker)) throw new Error(`missing per-turn model binding marker: ${marker}`);
}

for (const marker of [
  "context-usage-model",
  "snapshot.context_model",
  "the active task stays on",
  "shouldRenderAssistantDecisionCard",
]) {
  if (!app.includes(marker)) throw new Error(`missing frontend model/decision marker: ${marker}`);
}

for (const marker of [
  "Research participation protocol",
  "consequential research branch points",
  "Normally ask at most once during initial scoping",
  "If uploaded materials or explicit user constraints already determine the answer",
]) {
  if (!prompt.includes(marker)) throw new Error(`missing research choice policy marker: ${marker}`);
}

if (!web.includes("total_choices < 2") || !web.includes("take(12)")) {
  throw new Error("research choice cards are missing cooldown/frequency limits");
}
if ((web.match(/turn_is_waiting_for_research_choice\(&persisted_blocks, turn_start\)/g) || []).length < 2) {
  throw new Error("research loops do not pause for a persisted user choice");
}

console.log("model binding and research decision wiring is valid");
