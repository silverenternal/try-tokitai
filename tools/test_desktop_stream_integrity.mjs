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
assert.match(desktop, /__ATLAS_BRIDGE_STREAM_FINISH__/);
assert.match(desktop, /events: pending_events\.unwrap_or_default\(\)/);
assert.match(desktop, /Duration::from_millis\(12\)/);
assert.match(desktop, /serialized_chars < 192_000/);
assert.match(app, /if \(event\.type === "assistant_snapshot"\)/);
assert.match(app, /replaceAssistantBubbleSnapshot\(event\.delta \|\| ""\)/);
assert.match(app, /rawDelta: true/);
assert.ok(!app.includes('await window.AtlasScientificInfrastructure?.captureSnapshot?.("before_agent")'));

const paintWaitStart = app.indexOf("function waitForNextBrowserPaint");
const paintWaitEnd = app.indexOf("\nfunction isVisibleSessionRunning", paintWaitStart);
assert.ok(paintWaitStart >= 0 && paintWaitEnd > paintWaitStart, "paint wait helper is missing");
const paintWaitSource = app.slice(paintWaitStart, paintWaitEnd);
assert.match(paintWaitSource, /window\.setTimeout\(finish, \d+\)/);
assert.match(paintWaitSource, /window\.requestAnimationFrame\(finish\)/);
const paintWaitContext = {
  window: {
    setTimeout,
    clearTimeout,
    requestAnimationFrame: () => {},
  },
};
vm.createContext(paintWaitContext);
vm.runInContext(`${paintWaitSource}; this.waitForPaint = waitForNextBrowserPaint;`, paintWaitContext);
await Promise.race([
  paintWaitContext.waitForPaint(),
  new Promise((_, reject) => setTimeout(() => reject(new Error("paint fallback did not resolve")), 500)),
]);

const sendStart = app.indexOf("async function sendMessage()");
const sendEnd = app.indexOf("\nasync function optimizePromptBeforeSend", sendStart);
assert.ok(sendStart >= 0 && sendEnd > sendStart, "sendMessage source is missing");
const sendSource = app.slice(sendStart, sendEnd);
const streamOpenIndex = sendSource.indexOf("hostClient.chat.stream");
const beforeAgentSnapshotIndex = sendSource.indexOf('captureSnapshot?.("before_agent")');
assert.ok(streamOpenIndex >= 0, "desktop stream open call is missing");
assert.ok(beforeAgentSnapshotIndex > streamOpenIndex, "workspace snapshot still runs before the native stream opens");
assert.match(sendSource, /let streamTransportOpened = false/);
assert.match(sendSource, /streamTransportOpened = true/);
assert.match(sendSource, /streamTransportOpened && targetIsVisible && await reconcileStreamAfterTransportFailure/);

const scrollStart = app.indexOf("function scrollMessageStreamToBottom");
const scrollEnd = app.indexOf("\nfunction waitForNextBrowserPaint", scrollStart);
assert.ok(scrollStart >= 0 && scrollEnd > scrollStart, "message scroll helper is missing");
const scrollSource = app.slice(scrollStart, scrollEnd);
assert.doesNotMatch(scrollSource, /\btrimmed\b|\bcommandName\b/, "command parser code leaked into the message scroll helper");
const scrollContext = {
  messageStream: { scrollHeight: 640, scrollTop: 0, clientHeight: 240 },
  messageStreamFollowBlocked: false,
  messageStreamFollowFrame: null,
  messageStreamFollowTarget: 0,
  window: {
    matchMedia: () => ({ matches: true }),
    requestAnimationFrame: () => 1,
  },
};
vm.createContext(scrollContext);
vm.runInContext(`${app.slice(app.indexOf("function isNearMessageStreamBottom"), scrollEnd)}; this.scrollToBottom = scrollMessageStreamToBottom;`, scrollContext);
assert.doesNotThrow(() => scrollContext.scrollToBottom(true));
assert.equal(scrollContext.messageStream.scrollTop, 400);

const parserStart = app.indexOf("function parseAgentInputProtocol");
const parserEnd = app.indexOf("\nfunction formatSessionTime", parserStart);
assert.ok(parserStart >= 0 && parserEnd > parserStart, "agent input parser is missing");
const parserSource = app.slice(parserStart, parserEnd);
assert.match(parserSource, /const goalMatch = trimmed\.match/);
assert.match(parserSource, /commandName === "status"/);

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

const segmentStart = app.indexOf("function pushTurnTextSegment");
const segmentEnd = app.indexOf("\nfunction pushAssistantProgressWorklogText", segmentStart);
assert.ok(segmentStart >= 0 && segmentEnd > segmentStart, "stream segment helpers are missing");
const segmentContext = {
  sanitizeMessageContent: (value) => String(value || "").trimStart(),
};
vm.createContext(segmentContext);
vm.runInContext(
  `${app.slice(segmentStart, segmentEnd)};
   this.pushSegment = pushTurnTextSegment;
   this.replaceSegments = replaceTurnTextSegments;`,
  segmentContext,
);
const streamedTurn = { text: "", textSegments: [], lastStreamEventKind: "" };
segmentContext.pushSegment(streamedTurn, "ha", { rawDelta: true, appendToLast: true });
segmentContext.pushSegment(streamedTurn, "ha", { rawDelta: true, appendToLast: true });
segmentContext.pushSegment(streamedTurn, " ", { rawDelta: true, appendToLast: true });
segmentContext.pushSegment(streamedTurn, "world", { rawDelta: true, appendToLast: true });
assert.equal(streamedTurn.text, "haha world");
assert.equal(streamedTurn.textSegments.length, 1);
segmentContext.replaceSegments(streamedTurn, "new final answer");
assert.equal(streamedTurn.text, "new final answer");
assert.equal(streamedTurn.textSegments.length, 1);

console.log("Desktop native stream integrity checks passed");
