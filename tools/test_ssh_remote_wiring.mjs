import fs from "node:fs";
import assert from "node:assert/strict";

const read = (path) => fs.readFileSync(path, "utf8");
const html = read("frontend/index.html");
const js = read("frontend/ssh-remote.js");
const css = read("frontend/styles.css");
const overrides = read("frontend/professional-overrides.css");
const web = read("src/web.rs");
const core = read("src/remote_ssh.rs");

assert.match(html, /data-activity-panel="ssh"/);
assert.match(html, /data-activity-panel-id="ssh"/);
assert.match(html, /ssh-remote\.js/);
assert.match(css, /ssh-resource-grid/);
assert.match(overrides, /\.ssh-activity-panel\{display:none!important/);
assert.match(overrides, /\.ssh-activity-panel\.is-active\{display:grid!important/);

for (const route of [
  "/api/ssh", "/api/ssh/hosts/save", "/api/ssh/config/import", "/api/ssh/connect",
  "/api/ssh/disconnect", "/api/ssh/reconnect", "/api/ssh/heartbeat", "/api/ssh/execute",
  "/api/ssh/detect", "/api/ssh/files", "/api/ssh/transfer", "/api/ssh/terminals/create",
  "/api/ssh/forwards/start",
]) assert.ok(web.includes(route), `missing route ${route}`);

for (const tool of ["remote_ssh_context", "remote_ssh_connect", "remote_ssh_execute", "remote_ssh_transfer", "remote_ssh_environment", "remote_ssh_forward"])
  assert.ok(web.includes(tool), `missing Agent tool ${tool}`);

for (const objectType of ["remote-server", "remote-runtime", "python-environment", "gpu-device"])
  assert.ok(core.includes(`\"${objectType}\"`), `missing Scientific Object ${objectType}`);

assert.match(core, /ProxyJump|proxyjump/i);
assert.match(core, /auto_reconnect/);
assert.match(core, /known_hosts/);
assert.match(core, /Command::new\("sftp"\)/);
assert.match(core, /"remote-directory"/);
assert.match(core, /"remote-process"/);
assert.match(core, /"remote-container"/);
assert.match(core, /"remote-training-run"/);
assert.match(core, /"remote-port-forward"/);
assert.match(core, /Agent access was not authorized/);
assert.ok(!/iframe/i.test(js), "SSH workspace must not embed an external application");
const hostStruct = core.slice(core.indexOf("pub struct RemoteHostConfig"), core.indexOf("impl Default for RemoteHostConfig"));
assert.ok(!/password/i.test(hostStruct), "password must not be persisted in host profiles");

console.log("SSH Remote Development wiring checks passed");
