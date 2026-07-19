import fs from "node:fs";
import assert from "node:assert/strict";
const js=fs.readFileSync("frontend/scientific-infrastructure.js","utf8");
for(const route of ["papers","datasets","models","github","web","tracking","benchmarks"]) assert.match(js,new RegExp(`route\\s*===\\s*"${route}"|${route}`));
for (const field of ["payload.url","payload.html_url","payload.download_url","payload.web_url","payload.link"])
  assert.ok(js.includes(field), `missing external URL fallback ${field}`);
assert.match(js,/Search unavailable/);assert.match(js,/No results for/);
console.log("Search route coverage checks passed");
