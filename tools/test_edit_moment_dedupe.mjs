import { readFileSync } from "node:fs";
import vm from "node:vm";

const source = readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
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

if (turn.streamMoments.length !== 1) {
  throw new Error(`expected one semantic edit moment, got ${turn.streamMoments.length}`);
}
if (turn.streamMoments[0].state !== "done" || turn.streamMoments[0].added !== 4) {
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
if (lifecycleTurn.streamMoments.length !== 1) {
  throw new Error(`expected one current operation slot, got ${lifecycleTurn.streamMoments.length}`);
}
if (lifecycleTurn.streamMoments[0].kind !== "edit" || lifecycleTurn.streamMoments[0].state !== "run") {
  throw new Error("a late completion replaced the current operation");
}
context.pushTurnStreamMoment(lifecycleTurn, {
  kind: "edit",
  text: "Edit failed",
  state: "fail",
  filePath: "frontend/app.js",
  operationKey: "tool:edit-2",
});
if (lifecycleTurn.streamMoments[0].state !== "fail" || lifecycleTurn.streamMoments[0].id !== "current-operation") {
  throw new Error("the current operation did not update in place to its failure state");
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
