import { readFileSync } from "node:fs";
import vm from "node:vm";

const source = readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const storyStart = source.indexOf("function buildTurnStoryEntries");
const storyEnd = source.indexOf("function renderTurnStoryTextEntry", storyStart);
const storySource = source.slice(storyStart, storyEnd);
if (storyStart < 0 || storyEnd < 0 || storySource.includes("latestOperationIndex")) {
  throw new Error("story rendering still discards all but the latest operation capsule");
}
const turnMergeStart = source.indexOf("function mergeAssistantTurnData");
const turnMergeEnd = source.indexOf("function shouldSuppressInlineAssistantCode", turnMergeStart);
const turnMergeSource = source.slice(turnMergeStart, turnMergeEnd);
if (
  turnMergeStart < 0
  || turnMergeEnd < 0
  || !turnMergeSource.includes("streamMoments: mergeAssistantCollectionByKey(")
) {
  throw new Error("persisted and live operation capsules are still selected from only one side");
}
const start = source.indexOf("function pushTurnStreamMoment");
const end = source.indexOf("function pushAssistantStreamMoment", start);
if (start < 0 || end < 0) throw new Error("stream moment function not found");
const context = {
  cleanDisplayText: (value, fallback = "") => String(value || fallback),
  Date,
  Math,
};
vm.createContext(context);
vm.runInContext(source.slice(start, end), context);

const turn = { streamMoments: [] };
context.pushTurnStreamMoment(turn, {
  kind: "edit",
  text: "Editing",
  state: "run",
  operationKey: "tool:call-1",
});
context.pushTurnStreamMoment(turn, {
  kind: "edit",
  text: "Editing",
  state: "run",
  filePath: "D:\\Atlas\\frontend\\app.js",
  operationKey: "tool:call-2",
});
context.pushTurnStreamMoment(turn, {
  kind: "edit",
  text: "Editing",
  state: "run",
  filePath: "./frontend/app.js",
  operationKey: "edit:frontend/app.js",
});
context.pushTurnStreamMoment(turn, {
  kind: "edit",
  text: "Edit done",
  state: "done",
  filePath: "frontend/app.js",
  operationKey: "edit:frontend/app.js",
  added: 4,
  removed: 1,
});

if (turn.streamMoments.length !== 2) {
  throw new Error(`expected the generic and file-specific edits to remain distinct, got ${turn.streamMoments.length}`);
}
if (turn.streamMoments.at(-1).state !== "done" || turn.streamMoments.at(-1).added !== 4) {
  throw new Error("completed diff did not replace the running edit moment");
}

const lifecycleTurn = { streamMoments: [] };
context.pushTurnStreamMoment(lifecycleTurn, {
  kind: "inspection",
  text: "Inspecting",
  state: "run",
  operationKey: "tool:inspect-1",
});
context.pushTurnStreamMoment(lifecycleTurn, {
  kind: "edit",
  text: "Editing",
  state: "run",
  filePath: "frontend/app.js",
  operationKey: "tool:edit-2",
});
context.pushTurnStreamMoment(lifecycleTurn, {
  kind: "inspection",
  text: "Inspection done",
  state: "done",
  operationKey: "tool:inspect-1",
});
if (lifecycleTurn.streamMoments.length !== 2) {
  throw new Error(`expected completed inspection and current edit capsules, got ${lifecycleTurn.streamMoments.length}`);
}
if (lifecycleTurn.streamMoments[0].kind !== "inspection" || lifecycleTurn.streamMoments[0].state !== "done") {
  throw new Error("the completed inspection capsule was lost");
}
if (lifecycleTurn.streamMoments[1].kind !== "edit" || lifecycleTurn.streamMoments[1].state !== "run") {
  throw new Error("a late completion replaced the current operation");
}
context.pushTurnStreamMoment(lifecycleTurn, {
  kind: "edit",
  text: "Edit failed",
  state: "fail",
  filePath: "frontend/app.js",
  operationKey: "tool:edit-2",
});
if (lifecycleTurn.streamMoments[1].state !== "fail") {
  throw new Error("the current operation did not update in place to its failure state");
}

const multiFileTurn = { streamMoments: [] };
for (const [callId, filePath] of [["edit-a", "src/tracker.cpp"], ["edit-b", "src/detector.cpp"]]) {
  context.pushTurnStreamMoment(multiFileTurn, {
    kind: "edit",
    text: "Editing",
    state: "run",
    filePath,
    operationKey: `tool:${callId}`,
  });
  context.pushTurnStreamMoment(multiFileTurn, {
    kind: "edit",
    text: "Edit done",
    state: "done",
    filePath,
    operationKey: `edit:${filePath}`,
    added: 5,
  });
}
if (multiFileTurn.streamMoments.length !== 2 || multiFileTurn.streamMoments.some((item) => item.state !== "done")) {
  throw new Error("sequential file edits do not preserve both completed capsules");
}

console.log("single operation moment lifecycle is valid");

const css = readFileSync(new URL("../frontend/styles.css", import.meta.url), "utf8");
for (const selector of [
  ".codex-thinking-summary-label.is-streaming",
  ".codex-runtime-panel-title.is-streaming",
  ".codex-inline-moment-text.is-streaming",
  ".codex-inline-moment-prefix.is-streaming",
  ".codex-op-edit-prefix.is-streaming",
]) {
  if (!css.includes(selector)) throw new Error(`missing shared shimmer selector: ${selector}`);
}
if (!css.includes("animation-name: codex-live-text-shimmer !important")) {
  throw new Error("legacy status animations can still override the shared Codex shimmer");
}
