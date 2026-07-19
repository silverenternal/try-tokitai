import fs from "node:fs";
import assert from "node:assert/strict";

const read = (path) => fs.readFileSync(path, "utf8");
const app = read("frontend/app.js");
const html = read("frontend/index.html");
const css = read("frontend/professional-overrides.css");
const featureScripts = [
  "frontend/ssh-remote.js",
  "frontend/scientific-infrastructure.js",
  "frontend/notebook.js",
  "frontend/research-os.js",
].map(read).join("\n");

assert.match(html, /id="atlas-dialog"/);
assert.match(html, /id="atlas-context-menu"/);
assert.match(app, /window\.AtlasUI = Object\.freeze/);
assert.match(app, /document\.addEventListener\("contextmenu"/);
assert.match(app, /\.monaco-editor, \.monaco-menu-container/);
assert.match(css, /\.atlas-context-menu/);
assert.ok(!/\bwindow\.(?:alert|confirm|prompt)\s*\(/.test(featureScripts), "feature scripts must not use WebView dialogs");
assert.ok(!/(^|[^.\w])(?:alert|confirm|prompt)\s*\(/m.test(featureScripts), "feature scripts must use AtlasUI dialogs");
console.log("Atlas native UI wiring checks passed");
