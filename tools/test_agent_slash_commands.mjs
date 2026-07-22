import fs from "node:fs";
import assert from "node:assert/strict";

const app = fs.readFileSync("frontend/app.js", "utf8");
const html = fs.readFileSync("frontend/index.html", "utf8");
const css = fs.readFileSync("frontend/professional-overrides.css", "utf8");
const web = fs.readFileSync("src/web.rs", "utf8");

assert.match(html, /id="slash-command-menu"/);
assert.match(html, /id="slash-command-list"/);
assert.match(app, /const AGENT_SLASH_COMMANDS/);
for (const command of ["goal", "plan", "review", "status", "compact", "resume", "spec", "schedule", "model", "permissions", "new", "help"]) {
  assert.match(app, new RegExp(`name: "${command}"`));
}
assert.match(app, /currentWorkspaceMode !== "research"/);
assert.match(app, /mode: "goal"/);
assert.match(app, /renderSlashCommandMenu/);
assert.match(css, /\.slash-command-menu/);
assert.match(web, /goal_execution_contract_prompt/);
assert.match(web, /repeated_tool_call_guard/);
assert.match(web, /per-turn budget of 3 high-risk calls/);
assert.match(web, /TurnExecutionPolicy::Strict/);

console.log("Agent slash command and persistent goal wiring is valid");
