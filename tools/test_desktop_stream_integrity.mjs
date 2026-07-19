import fs from "node:fs";
import assert from "node:assert/strict";
import vm from "node:vm";

const app = fs.readFileSync("frontend/app.js", "utf8");
const desktop = fs.readFileSync("src/bin/desktop_wry.rs", "utf8");

assert.match(app, /desktopBridge\.openStream\(BRIDGE_COMMANDS\.chatStream, payload\)/);
assert.match(app, /resolved\.transport === "bridge" \? BRIDGE_COMMANDS\.chatStop/);
assert.match(app, /BRIDGE_COMMANDS\.nativeRequest/);
assert.match(app, /url\.pathname\.startsWith\("\/api\/"\)/);
assert.match(app, /if \(!sawTerminalEvent\)/);
assert.match(desktop, /desktop agent stream closed before a complete or error event/);
assert.match(desktop, /let _runtime_guard = async_runtime\.enter\(\)/);
assert.match(desktop, /__ATLAS_BRIDGE_STREAM_PUSH_BATCH__/);
assert.match(desktop, /Duration::from_millis\(12\)/);
assert.ok(!app.includes('await window.AtlasScientificInfrastructure?.captureSnapshot?.("before_agent")'));

const start = app.indexOf("function mergeStreamingTextDelta");
const end = app.indexOf("\nfunction ensureActivityNodes", start);
assert.ok(start >= 0 && end > start, "stream text merge helper is missing");
const context = {};
vm.createContext(context);
vm.runInContext(`${app.slice(start, end)}; this.merge = mergeStreamingTextDelta;`, context);

assert.equal(context.merge("hello", "hello"), "hello");
assert.equal(context.merge("hello", "hello world"), "hello world");
assert.equal(context.merge("hello wor", "world"), "hello world");
assert.equal(context.merge("hello", " world"), "hello world");
assert.equal(context.merge("second", " third"), "second third");

console.log("Desktop native stream integrity checks passed");
