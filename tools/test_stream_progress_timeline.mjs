import fs from "node:fs";

const source = fs.readFileSync(new URL("../frontend/app.js", import.meta.url), "utf8");
const backend = fs.readFileSync(new URL("../src/web.rs", import.meta.url), "utf8");

const pushMoment = source.match(/function pushAssistantStreamMoment\(moment\) \{[\s\S]*?\n\}/)?.[0] || "";
if (!pushMoment.includes("pendingAssistantStoryDirty = true")) {
  throw new Error("stream moments do not invalidate the inline story timeline");
}

const operationStart = source.match(/function markAssistantOperationStarted\(\) \{[\s\S]*?\n\}/)?.[0] || "";
if (!operationStart.includes("activeAssistantTurn.isThinkingPhase = true")) {
  throw new Error("operation transitions can still unmount and restart the Thinking shimmer");
}

if ((backend.match(/runtime_ready_rx\.await/g) || []).length < 2 || (backend.match(/runtime_ready_tx\.send/g) || []).length < 2) {
  throw new Error("stream workers can still emit before their runtime sessions are registered");
}

const operations = source.match(/function syncPendingAssistantOperations\(\) \{[\s\S]*?\n\}/)?.[0] || "";
if (!operations.includes("pendingAssistantOperationsNode.hidden = true")) {
  throw new Error("legacy detached operations area is no longer kept hidden");
}

for (const marker of [
  "streamPresentationQueues",
  "enqueueStreamPresentationEvent(event, sessionId)",
  "await drainStreamPresentationQueue(sessionId)",
  "completeRunningStreamMoments(activeAssistantTurn)",
  "activityStageMoment(event.activity || {})",
  'kind: "thinking"',
]) {
  if (!source.includes(marker)) {
    throw new Error(`missing Codex-like streaming timeline marker: ${marker}`);
  }
}

const contextUsage = source.match(/function updateContextUsage\(usedTokens, contextWindow\) \{[\s\S]*?\n\}/)?.[0] || "";
if (!contextUsage.includes('percent > 0 && percent < 1 ? "<1%"')) {
  throw new Error("small non-zero context usage is still displayed as 0%");
}

for (const eventType of ["assistant_progress", "activity", "subagent", "verifier", "tool", "edited_files"]) {
  const marker = `if (event.type === "${eventType}")`;
  const start = source.indexOf(marker);
  const nextEvent = source.indexOf("\n  if (event.type ===", start + marker.length);
  const branch = source.slice(start, nextEvent > start ? nextEvent : start + 5000);
  if (start < 0 || !branch.includes("markAssistantOperationStarted()")) {
    throw new Error(`${eventType} does not transition from thinking to visible operation progress`);
  }
}

console.log("stream progress timeline wiring is valid");
