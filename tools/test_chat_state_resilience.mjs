import fs from "node:fs";

const app = fs.readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const html = fs.readFileSync(new URL("../frontend/index.html", import.meta.url), "utf8");
const web = fs.readFileSync(new URL("../src/web.rs", import.meta.url), "utf8");

const required = [
  "detachVisibleConversationRuntime({ preserveInputFocus: true })",
  "composerSendSessionId === targetSessionId",
  "reconcileSuccessfulStreamAfterError(targetSessionId)",
  "snapshot.partial_thinking",
  "hostClient.chat.optimize(content, currentLanguage)",
];
for (const marker of required) {
  if (!app.includes(marker)) throw new Error(`missing chat resilience marker: ${marker}`);
}
if (!html.includes('id="prompt-optimize-button"')) throw new Error("prompt optimizer button is missing");
if (!web.includes('"/api/prompt/optimize"') || !web.includes("bridge_prompt_optimize")) {
  throw new Error("prompt optimizer backend route is missing");
}
if (/async function switchSession[\s\S]{0,400}resetConversationRuntimeState/.test(app)) {
  throw new Error("session switching still cancels the previous runtime");
}
if (/async function createSession[\s\S]{0,400}resetConversationRuntimeState/.test(app)) {
  throw new Error("session creation still cancels the previous runtime");
}
console.log("chat state resilience wiring is valid");
