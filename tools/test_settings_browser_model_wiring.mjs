import fs from "node:fs";

const app = fs.readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const css = fs.readFileSync(new URL("../frontend/styles.css", import.meta.url), "utf8");
const web = fs.readFileSync(new URL("../src/web.rs", import.meta.url), "utf8");
const desktop = fs.readFileSync(new URL("../src/bin/desktop_wry.rs", import.meta.url), "utf8");

for (const marker of [
  'String(config.max_tool_calls_per_minute ?? 0)',
  '["10", "30", "0"]',
  'String(config.burst_limit ?? 0)',
  '["1", "5", "0"]',
  "positionSettingsPanel()",
  'settingsPanel.style.setProperty("--settings-left"',
]) {
  if (!app.includes(marker)) throw new Error(`missing settings persistence/position marker: ${marker}`);
}

if (!css.includes("left: var(--settings-left") || !css.includes("transform-origin: left bottom")) {
  throw new Error("settings card is not anchored beside the left settings button");
}

if (!web.includes('normalized.contains("qwen3.7")') || !web.includes("1_000_000")) {
  throw new Error("Qwen long-context metadata is not wired");
}

for (const marker of [
  ".with_new_window_req_handler",
  'action: "open".to_string()',
  "Always consume the request",
  "false",
]) {
  if (!desktop.includes(marker)) throw new Error(`missing native browser popup interception: ${marker}`);
}

console.log("settings, browser popup, and model context wiring is valid");
