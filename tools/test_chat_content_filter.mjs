import fs from "node:fs";
import assert from "node:assert/strict";

const app = fs.readFileSync("frontend/app.js", "utf8");
const start = app.indexOf("function shouldSuppressInlineAssistantCode");
const end = app.indexOf("function turnHasRealWorkspaceChanges", start);
assert.ok(start >= 0 && end > start, "chat suppression filter missing");
const filter = app.slice(start, end);

assert.match(filter, /explicitProtocolPayload/);
assert.match(filter, /hasDiffs && \(looksPayloadLike \|\| looksJsonLike \|\| longCodeLike\)/);
assert.ok(!filter.includes("content.trim().length > 420))"), "long technical answers must not be hidden by length");
assert.match(app, /else if \(content\) \{\s*activeAssistantTurn\.suppressedInlineContent = false;/);
assert.match(app, /function visibleAssistantWorkspaceNotice[\s\S]*?if \(!turnHasRealWorkspaceChanges\(turn\)\) \{\s*return "";/);
assert.ok(!app.includes("\u5df2\u7701\u7565\u4e0d\u9002\u5408\u4f5c\u4e3a\u804a\u5929\u6b63\u6587\u5c55\u793a\u7684\u4ee3\u7801\u6216\u5de5\u5177\u8f7d\u8377"), "misleading omission notice must be removed");
console.log("Chat content filter regression checks passed");
