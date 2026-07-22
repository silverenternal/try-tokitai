import fs from "node:fs";
import assert from "node:assert/strict";
import vm from "node:vm";

const app = fs.readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const html = fs.readFileSync(new URL("../frontend/index.html", import.meta.url), "utf8");
const web = fs.readFileSync(new URL("../src/web.rs", import.meta.url), "utf8");

const required = [
  "detachVisibleConversationRuntime({ preserveInputFocus: true })",
  "composerSendSessionId === targetSessionId",
  "reconcileStreamAfterTransportFailure(targetSessionId)",
  "bootstrapHasLiveSession(data, targetSessionId)",
  "snapshot.partial_thinking",
  'if (event.type === "assistant_snapshot")',
  "replaceAssistantBubbleSnapshot",
  "rawDelta: true",
  "hostClient.chat.optimize(content, currentLanguage)",
];
for (const marker of required) {
  if (!app.includes(marker)) throw new Error(`missing chat resilience marker: ${marker}`);
}
if (!html.includes('id="prompt-optimize-button"')) throw new Error("prompt optimizer button is missing");
if (!web.includes('"/api/prompt/optimize"') || !web.includes("bridge_prompt_optimize")) {
  throw new Error("prompt optimizer backend route is missing");
}
if (!web.includes("messages_to_stream_web(&finalized_blocks)")) {
  throw new Error("terminal stream payload still includes unbounded tool results");
}
for (const marker of [
  "MAX_STREAM_ATTEMPTS: usize = 4",
  "model stream was idle for 300 seconds",
  "is_retryable_stream_error(&error)",
  "emit_stream_retry_activity",
  "stream_retry_delay(attempt)",
]) {
  if (!web.includes(marker)) throw new Error(`missing backend stream recovery marker: ${marker}`);
}
const successFinalize = web.slice(
  web.indexOf("fn finalize_stream_success("),
  web.indexOf("fn recover_stream_finalize_context("),
);
if (successFinalize.indexOf("mark_stream_terminal_emitted") > successFinalize.indexOf("save_messages_for")) {
  // expected: terminal is marked only after persistence succeeds
} else {
  throw new Error("success finalization still marks terminal before persistence completes");
}
if (!app.includes("const deadline = Date.now() + (15 * 60 * 1000)")) {
  throw new Error("desktop transport recovery does not wait for long-running background turns");
}
if (/if structured_workflow\s*&& !needs_tools\s*&& final_turn_ends_with_unfinished_progress/.test(web)) {
  throw new Error("plain Agent turns can still finalize with unfinished progress narration");
}
if (!web.includes('"\\u{8ba9}\\u{6211}\\u{68c0}\\u{67e5}"')) {
  throw new Error("unfinished Chinese progress narration is not detected independently of source encoding");
}
for (const marker of [
  "Missing-final-answer repair directive",
  "final_turn_has_acceptable_assistant_completion",
  "missing_final_answer_message",
  "force_final_answer_mode",
  "final_answer_only_instruction",
  "readonly_workspace_evidence_is_sufficient",
  "synthesize_readonly_workspace_analysis",
]) {
  if (!web.includes(marker)) throw new Error(`missing final-answer completion gate marker: ${marker}`);
}
if (!app.includes("hasAssistantCompletionAfterLastTool(rawVisibleMessages)")) {
  throw new Error("complete events still accept assistant narration that occurred only before tool execution");
}
if (!app.includes("shouldReplaceAssistantMessageWithStructuredToolResult(nextContent)")) {
  throw new Error("assistant/tool-result precedence guard is missing");
}
const errorBranch = app.slice(
  app.indexOf('if (event.type === "error")'),
  app.indexOf("\n  }\n}", app.indexOf('if (event.type === "error")')),
);
if (!errorBranch.includes("finalizeVisibleAssistantBubble") || errorBranch.includes("throw new Error")) {
  throw new Error("stream error terminal still bypasses visible assistant finalization");
}
if (!app.includes('role: "error", content: failureText')) {
  throw new Error("stream failures are not materialized as structured visible error messages");
}
if (!app.includes("const choicesAreExplicit = isExplicitAssistantDecisionPayload(message.assistant_choices);")) {
  throw new Error("persisted assistant choices are not validated before rendering");
}
if (!app.includes("assistantChoicesAsMarkdown(message.assistant_choices)")) {
  throw new Error("legacy malformed assistant choices are not recovered as visible markdown");
}
for (const marker of [
  "looksLikeAssistantReportLayout(raw)",
  "visibleMessagesCoverAssistantCompletion(incomingVisibleMessages, previousMessages)",
  "completionGuardActive && !incomingCoversVisibleCompletion",
]) {
  if (!app.includes(marker)) throw new Error(`missing monotonic completion redraw marker: ${marker}`);
}

const functionSource = (name, nextName) => {
  const start = app.indexOf(`function ${name}`);
  const end = app.indexOf(`\nfunction ${nextName}`, start);
  assert.ok(start >= 0 && end > start, `${name} source is missing`);
  return app.slice(start, end);
};
const terminalMergeContext = {
  sanitizeMessageContent: (value) => String(value || ""),
  cleanDisplayText: (value) => String(value || ""),
  normalizedAssistantSubstantiveContent: (value) => String(value || "").trim(),
  assistantTextLooksLikeProcessNarration: (value) => /^(?:让我|接下来|现在让我)/.test(String(value || "").trim()),
  isAssistantPrimaryReplyText: (value) => String(value || "").length >= 120,
  isAssistantVerificationAppendixText: () => false,
  assistantPrimaryReplyCore: (value) => String(value || "").trim(),
  combineAssistantSegments: (left, right) => `${left}\n\n${right}`.trim(),
  looksLikeStructuredAssistantReport: (value) => String(value || "").length >= 120,
  looksLikeOperationalContentDump: () => false,
  shouldSuppressInlineAssistantCode: () => false,
  assistantChoicesAsMarkdown: (choices) => (choices?.options || []).map((item) => `- ${item}`).join("\n"),
  activeAssistantTurn: null,
};
vm.createContext(terminalMergeContext);
vm.runInContext(
  `${functionSource("isAssistantFailureSummaryText", "isAssistantVerificationAppendixText")}
   ${functionSource("preferAssistantMessageContent", "cleanDisplayMarkdown")}
   ${functionSource("assistantDecisionOptionHasExplicitMarker", "isExplicitAssistantDecisionPayload")}
   ${functionSource("isExplicitAssistantDecisionPayload", "assistantDecisionDisplayTitle")}
   ${functionSource("completionFallbackAssistantContent", "mergeAssistantCompletionText")}
   ${functionSource("mergeAssistantCompletionText", "latestAssistantCompletionMessageIndex")}
   ${functionSource("latestAssistantCompletionMessageIndex", "hasRenderableAssistantText")}
   ${functionSource("ensureVisibleAssistantCompletionMessage", "hasAssistantCompletionAfterLastTool")}
   ${functionSource("shouldReplaceAssistantMessageWithStructuredToolResult", "createMessageRow")}
   this.isFailure = isAssistantFailureSummaryText;
   this.prefer = preferAssistantMessageContent;
   this.completionText = completionFallbackAssistantContent;
   this.mergeCompletion = mergeAssistantCompletionText;
   this.ensureCompletion = ensureVisibleAssistantCompletionMessage;
   this.isExplicitDecision = isExplicitAssistantDecisionPayload;
   this.shouldReplace = shouldReplaceAssistantMessageWithStructuredToolResult;`,
  terminalMergeContext,
);
const realTraceFailure = "[completion-gate] 工具执行已经结束，但模型在自动续写后仍未给出面向用户的完整答复。工具结果和当前进度已保留；本轮不会被伪装成成功。";
const structuredToolResult = "## Workspace evidence\n\n" + "tool-derived evidence ".repeat(12);
assert.equal(terminalMergeContext.isFailure(realTraceFailure), true);
assert.equal(terminalMergeContext.shouldReplace(realTraceFailure), false);
const mergedTerminalText = terminalMergeContext.prefer(structuredToolResult, realTraceFailure);
assert.ok(mergedTerminalText.includes(structuredToolResult.trim()));
assert.ok(mergedTerminalText.includes(realTraceFailure));
assert.ok(mergedTerminalText.length >= structuredToolResult.trim().length);
const streamedLongAnswer = `## Stable final answer\n\n${"complete streamed paragraph ".repeat(24)}`;
const terminalShortAnswer = "## Stable final answer";
assert.equal(
  terminalMergeContext.mergeCompletion(streamedLongAnswer, terminalShortAnswer).trim(),
  streamedLongAnswer.trim(),
);
assert.equal(
  terminalMergeContext.completionText({
    text: streamedLongAnswer,
    textSegments: [{ text: terminalShortAnswer }],
    diffs: [],
  }).trim(),
  streamedLongAnswer.trim(),
);
const shortTerminalMessages = [
  { kind: "message", role: "user", content: "analyze" },
  { kind: "tool_result", role: "assistant", content: "tool complete" },
  { kind: "message", role: "assistant", content: terminalShortAnswer },
];
const repairedShortTerminal = terminalMergeContext.ensureCompletion(
  shortTerminalMessages,
  { text: streamedLongAnswer, textSegments: [], assistantChoices: null },
);
assert.equal(repairedShortTerminal.length, shortTerminalMessages.length);
assert.equal(repairedShortTerminal.at(-1).content.trim(), streamedLongAnswer.trim());
assert.ok(repairedShortTerminal.at(-1).content.trim().length >= streamedLongAnswer.trim().length);
const toolOnlyTerminal = terminalMergeContext.ensureCompletion(
  [
    { kind: "message", role: "user", content: "analyze" },
    { kind: "tool_result", role: "assistant", content: "tool complete" },
  ],
  { text: streamedLongAnswer, textSegments: [], assistantChoices: null },
);
assert.equal(toolOnlyTerminal.at(-1).role, "assistant");
assert.equal(toolOnlyTerminal.at(-1).content.trim(), streamedLongAnswer.trim());
const errorTerminal = terminalMergeContext.ensureCompletion(
  [
    { kind: "message", role: "user", content: "analyze" },
    { kind: "tool_result", role: "assistant", content: "tool complete" },
    { kind: "message", role: "error", content: "transport failed" },
  ],
  { text: streamedLongAnswer, textSegments: [], assistantChoices: null },
);
assert.equal(errorTerminal.at(-2).role, "assistant");
assert.equal(errorTerminal.at(-2).content.trim(), streamedLongAnswer.trim());
assert.equal(errorTerminal.at(-1).role, "error");
const analysisReport = [
  "## Workspace code analysis",
  "",
  "This project uses a staged processing pipeline. The following findings explain the important ownership and execution boundaries in detail.",
  "",
  "### Core modules",
  "",
  "| File | Responsibility |",
  "|---|---|",
  "| `src/tracker/pipeline.cpp` | Pipeline orchestration |",
  "| `src/tracker/data_association.cpp` | Track association |",
  "| `src/preprocessor/cuda_resize.cu` | GPU preprocessing |",
  "| `include/tracker/pipeline.hpp` | Public interfaces |",
  "",
  "### Findings",
  "",
  "The implementation has clear module boundaries, but resource ownership and build portability should be strengthened before production deployment.",
].join("\n");
const reportClassifierContext = {
  sanitizeMessageContent: (value) => String(value || ""),
  isAssistantFailureSummaryText: () => false,
  isAssistantCompletionSummaryText: () => false,
};
vm.createContext(reportClassifierContext);
vm.runInContext(
  `${functionSource("looksLikeToolPayloadDump", "looksLikeAssistantReportLayout")}
   ${functionSource("looksLikeAssistantReportLayout", "looksLikeDirectoryTreeDump")}
   ${functionSource("looksLikeDirectoryTreeDump", "looksLikeOperationalContentDump")}
   ${functionSource("looksLikeOperationalContentDump", "looksLikeStructuredAssistantReport")}
   this.isReportLayout = looksLikeAssistantReportLayout;
   this.isDirectoryDump = looksLikeDirectoryTreeDump;
   this.isOperationalDump = looksLikeOperationalContentDump;`,
  reportClassifierContext,
);
assert.equal(reportClassifierContext.isReportLayout(analysisReport), true);
assert.equal(reportClassifierContext.isDirectoryDump(analysisReport), false);
assert.equal(reportClassifierContext.isOperationalDump(analysisReport), false);
assert.equal(reportClassifierContext.isReportLayout('{"data":{"path":"src/main.rs","content":"raw"}}'), false);
assert.equal(reportClassifierContext.isOperationalDump('{"data":{"path":"src/main.rs","content":"raw"},"operation":"read_file"}'), true);
terminalMergeContext.looksLikeOperationalContentDump = reportClassifierContext.isOperationalDump;
assert.equal(terminalMergeContext.completionText({
  text: analysisReport,
  textSegments: [{ text: analysisReport }],
  diffs: [],
}).trim(), analysisReport.trim());

const coverageContext = {
  sanitizeMessageContent: (value) => String(value || ""),
  latestAssistantCompletionMessageIndex: terminalMergeContext.latestAssistantCompletionMessageIndex,
  mergeAssistantCompletionText: terminalMergeContext.mergeCompletion,
  isExplicitAssistantDecisionPayload: terminalMergeContext.isExplicitDecision,
};
vm.createContext(coverageContext);
vm.runInContext(
  `${functionSource("visibleMessagesCoverAssistantCompletion", "displayMarkdownText")}
   this.coversCompletion = visibleMessagesCoverAssistantCompletion;`,
  coverageContext,
);
const completeReference = [
  { kind: "message", role: "user", content: "analyze" },
  { kind: "tool_result", role: "assistant", content: "read complete" },
  { kind: "message", role: "assistant", content: streamedLongAnswer },
];
assert.equal(coverageContext.coversCompletion(shortTerminalMessages, completeReference), false);
assert.equal(coverageContext.coversCompletion(completeReference, shortTerminalMessages), true);
assert.equal(coverageContext.coversCompletion(completeReference, completeReference), true);
assert.equal(terminalMergeContext.isExplicitDecision({
  title: "explicit_decision",
  options: [
    "硬件感知优化：已有代码亮点",
    "依赖查找路径硬编码了 Linux 目录",
    "修复 CMake 跨平台支持：建议项",
  ],
}), false);
assert.equal(terminalMergeContext.isExplicitDecision({
  title: "explicit_decision",
  options: [
    "方向 A：保持现有架构",
    "方向 B：迁移到新架构",
    "方向 C：采用混合架构",
  ],
}), true);
const completeBranch = app.slice(
  app.indexOf('if (event.type === "complete")'),
  app.indexOf("const completionPreference", app.indexOf('if (event.type === "complete")')),
);
assert.ok(completeBranch.includes("ensureVisibleAssistantCompletionMessage("));
assert.ok(
  completeBranch.indexOf("finalizeVisibleAssistantBubble")
    < completeBranch.indexOf("finalizeActiveAssistantTurn"),
  "complete cleanup still destroys live text before the visible bubble is finalized",
);
if (/async function switchSession[\s\S]{0,400}resetConversationRuntimeState/.test(app)) {
  throw new Error("session switching still cancels the previous runtime");
}
if (/async function createSession[\s\S]{0,400}resetConversationRuntimeState/.test(app)) {
  throw new Error("session creation still cancels the previous runtime");
}
console.log("chat state resilience wiring is valid");
