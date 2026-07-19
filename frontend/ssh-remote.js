(() => {
  "use strict";
  const panel = document.getElementById("activity-panel-ssh");
  if (!panel) return;
  const hostList = document.getElementById("ssh-host-list");
  const workspace = document.getElementById("ssh-remote-workspace");
  const transport = document.getElementById("ssh-transport-state");
  let snapshot = { hosts: [], connections: [], terminals: [], forwards: [], environments: [] };
  let selectedHostId = null;
  let selectedTerminalId = null;
  let files = null;
  let consoleOutput = "";
  let pollTimer = null;

  const esc = (value) => String(value ?? "").replace(/[&<>"']/g, (ch) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[ch]));
  async function api(path, payload) {
    const response = await fetch(path, payload === undefined ? {} : { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(payload) });
    let body; try { body = await response.json(); } catch (_) { body = { data: await response.text() }; }
    if (!response.ok || body?.ok === false) throw new Error(body?.data?.message || body?.message || body?.data || `SSH request failed (${response.status})`);
    return body?.data ?? body;
  }
  function connection(hostId) { return snapshot.connections.find((item) => item.host_id === hostId); }
  function selectedHost() { return snapshot.hosts.find((item) => item.id === selectedHostId); }
  function environment(hostId) { return snapshot.environments.find((item) => item.host_id === hostId); }
  function isVisible() { return panel.classList.contains("is-active"); }
  function notify(error) { consoleOutput += `\n[Atlas SSH] ${error?.message || error}`; renderWorkspace(); }

  async function refresh({ quiet = false } = {}) {
    try {
      snapshot = await api("/api/ssh");
      if (!selectedHostId && snapshot.hosts.length) selectedHostId = snapshot.hosts[0].id;
      if (selectedHostId && !snapshot.hosts.some((host) => host.id === selectedHostId)) selectedHostId = snapshot.hosts[0]?.id || null;
      render();
    } catch (error) { if (!quiet) notify(error); }
  }
  async function monitorConnections() {
    if (!isVisible()) return;
    const active = snapshot.connections.filter((item) => ["connected", "reconnecting"].includes(item.state));
    await Promise.allSettled(active.map((item) => api("/api/ssh/heartbeat", { host_id: item.host_id })));
    await refresh({ quiet: true });
  }

  function render() {
    transport.textContent = snapshot.transport_available ? "OpenSSH transport ready" : "OpenSSH unavailable";
    hostList.innerHTML = snapshot.hosts.length ? snapshot.hosts.map((host) => {
      const conn = connection(host.id); const state = conn?.state || "disconnected";
      return `<button class="ssh-host-row state-${esc(state)} ${host.id === selectedHostId ? "is-active" : ""}" type="button" data-ssh-host="${esc(host.id)}"><i></i><span><strong>${esc(host.label)}</strong><small>${esc(host.user ? `${host.user}@` : "")}${esc(host.host)}:${host.port}</small></span><code>${esc(state)}</code></button>`;
    }).join("") : `<div class="ssh-empty-state">No remote hosts. Import SSH Config or add a server.</div>`;
    renderWorkspace();
  }

  function renderWorkspace() {
    const host = selectedHost(); if (!host) { workspace.innerHTML = `<div class="ssh-empty-state">Select or add a remote research server.</div>`; return; }
    const conn = connection(host.id); const connected = conn?.state === "connected"; const env = environment(host.id);
    const terminals = snapshot.terminals.filter((item) => item.host_id === host.id);
    if (!selectedTerminalId || !terminals.some((item) => item.id === selectedTerminalId)) selectedTerminalId = terminals[0]?.id || null;
    const terminal = terminals.find((item) => item.id === selectedTerminalId);
    const fileRows = files?.entries?.length ? files.entries.map((entry) => `<div class="ssh-file"><span>${entry.kind === "d" ? "▸" : "·"}</span><span>${esc(entry.name)}</span><code>${entry.size ?? ""}</code></div>`).join("") : `<div class="ssh-empty-state">${connected ? "Open a remote directory." : "Connect to browse remote files."}</div>`;
    const envHtml = env ? `<div class="ssh-env-section"><strong>System runtime</strong><code>${esc(env.os)}\n${esc(env.kernel)} · ${esc(env.arch)}\n${esc(env.shell)}</code></div><div class="ssh-env-section"><strong>Research environments</strong><code>Python: ${esc(env.python.join(", ") || "not detected")}\nManagers: ${esc(env.managers.join(", ") || "none")}\nGit: ${esc(env.git || "not detected")}\nDocker: ${esc(env.docker || "not detected")}</code></div><div class="ssh-env-section"><strong>Accelerators & schedulers</strong><code>${esc(env.gpu_summary.join("\n") || "No NVIDIA GPU detected")}\nSchedulers: ${esc(env.schedulers.join(", ") || "none")}</code></div><div class="ssh-env-section"><strong>Processes & containers</strong><code>${esc((env.processes || []).slice(0, 12).join("\n") || "No process snapshot")}\n\n${esc((env.containers || []).join("\n") || "No running containers")}</code></div>` : `<div class="ssh-empty-state">Run environment detection to discover Python, containers and GPUs.</div>`;
    workspace.innerHTML = `<div class="ssh-connection-strip"><strong>${esc(host.label)}</strong><code>${esc(conn?.state || "disconnected")}${conn?.latency_ms != null ? ` · ${conn.latency_ms}ms` : ""}</code><label class="ssh-agent-consent"><input id="ssh-agent-authorized" type="checkbox" ${conn?.agent_authorized ? "checked" : ""} ${connected ? "disabled" : ""}>Authorize Agent</label><button type="button" data-ssh-action="${connected ? "disconnect" : conn ? "reconnect" : "connect"}">${connected ? "Disconnect" : conn ? "Reconnect" : "Connect"}</button><button type="button" data-ssh-action="detect" ${connected ? "" : "disabled"}>Detect</button><button type="button" data-ssh-action="edit">Edit</button><button type="button" data-ssh-action="delete" ${conn ? "disabled" : ""}>Delete</button></div>
      <div class="ssh-resource-grid"><section class="ssh-pane"><header><span>REMOTE FILE SYSTEM</span><button type="button" data-ssh-action="transfer" ${connected ? "" : "disabled"}>Transfer</button></header><div class="ssh-path-bar"><input id="ssh-path" value="${esc(files?.path || host.remote_root || "~")}" spellcheck="false"><button type="button" data-ssh-action="browse" ${connected ? "" : "disabled"}>Open</button></div><div class="ssh-file-list">${fileRows}</div></section><section class="ssh-pane"><header><span>ENVIRONMENT INSPECTOR</span><span>${env ? esc(env.detected_at.slice(11, 19)) : "not scanned"}</span></header><div class="ssh-env-list">${envHtml}</div></section></div>
      <section class="ssh-runtime-console"><div class="ssh-runtime-tabs">${terminals.map((item) => `<button type="button" data-ssh-terminal="${esc(item.id)}" class="${item.id === selectedTerminalId ? "is-active" : ""}">${esc(item.title)}</button>`).join("")}<button type="button" data-ssh-action="new-terminal" ${connected ? "" : "disabled"}>+ Session</button><button type="button" data-ssh-action="close-terminal" ${selectedTerminalId ? "" : "disabled"}>Close</button><button type="button" data-ssh-action="forward" ${connected ? "" : "disabled"}>Forward</button></div><pre class="ssh-terminal-output">${esc(consoleOutput || terminal?.output || "Atlas Remote Runtime ready.")}</pre><div class="ssh-command-line"><input id="ssh-command" placeholder="Remote command, training task, Git/Docker/GPU query…" spellcheck="false" ${connected ? "" : "disabled"}><button type="button" data-ssh-action="execute" ${connected ? "" : "disabled"}>Run</button></div>${renderForwards(host.id)}</section>`;
  }

  function renderForwards(hostId) {
    const forwards = snapshot.forwards.filter((item) => item.host_id === hostId); if (!forwards.length) return "";
    return `<div class="ssh-forward-table">${forwards.map((item) => `<div class="ssh-forward-row"><span>${esc(item.kind)}</span><code>${esc(item.bind)}${item.target ? ` → ${esc(item.target)}` : ""}</code><button type="button" data-ssh-forward-stop="${esc(item.id)}">Stop</button></div>`).join("")}</div>`;
  }

  function showHostDialog(host = {}) {
    const backdrop = document.createElement("div"); backdrop.className = "ssh-dialog-backdrop";
    backdrop.innerHTML = `<form class="ssh-dialog"><header><strong>${host.id ? "Edit remote host" : "Add remote host"}</strong><button type="button" data-close>×</button></header><div class="ssh-form-grid"><label>Label<input name="label" required value="${esc(host.label || "")}"></label><label>Host<input name="host" required value="${esc(host.host || "")}"></label><label>User<input name="user" value="${esc(host.user || "")}"></label><label>Port<input name="port" type="number" min="1" max="65535" value="${host.port || 22}"></label><label>Authentication<select name="auth_method"><option value="ssh_config">SSH Config / Agent</option><option value="key">Private key</option><option value="password">Password</option><option value="agent">SSH Agent</option></select></label><label>Identity file<input name="identity_file" value="${esc(host.identity_file || "")}"></label><label>Jump Host<input name="jump_host" value="${esc(host.jump_host || "")}"></label><label>Remote root<input name="remote_root" value="${esc(host.remote_root || "~")}"></label><label class="is-wide">SSH Config alias<input name="ssh_config_alias" value="${esc(host.ssh_config_alias || "")}"></label></div><footer><button type="button" data-close>Cancel</button><button class="is-primary" type="submit">Save host</button></footer></form>`;
    backdrop.querySelector("select").value = host.auth_method || "ssh_config";
    const close = () => backdrop.remove(); backdrop.querySelectorAll("[data-close]").forEach((button) => button.addEventListener("click", close));
    backdrop.querySelector("form").addEventListener("submit", async (event) => { event.preventDefault(); const data = Object.fromEntries(new FormData(event.currentTarget)); const payload = { ...host, ...data, port: Number(data.port), connect_timeout_secs: host.connect_timeout_secs || 10, keepalive_secs: host.keepalive_secs || 15, auto_reconnect: host.auto_reconnect ?? true, max_reconnect_attempts: host.max_reconnect_attempts || 5, tags: host.tags || [], identity_file: data.identity_file || null, jump_host: data.jump_host || null, ssh_config_alias: data.ssh_config_alias || null, ssh_config_file: host.ssh_config_file || null }; try { const saved = await api("/api/ssh/hosts/save", payload); selectedHostId = saved.id; close(); await refresh(); } catch (error) { notify(error); } }); document.body.append(backdrop);
  }

  async function connectHost(host) {
    const consent = Boolean(document.getElementById("ssh-agent-authorized")?.checked); let password = null;
    if (host.auth_method === "password") password = await window.AtlasUI.prompt("SSH password", "", { message: `Password for ${host.label}. It is kept only for this connection session.`, placeholder: "Password", sensitive: true }); if (host.auth_method === "password" && password == null) return;
    await api("/api/ssh/connect", { host_id: host.id, password, agent_authorized: consent }); consoleOutput += `\nConnected to ${host.label}.`; await refresh();
  }
  async function runAction(action) {
    const host = selectedHost(); if (!host) return;
    if (["execute", "transfer", "new-terminal", "forward"].includes(action)) await window.AtlasScientificInfrastructure?.captureSnapshot?.("auto").catch(() => null);
    if (action === "connect") return connectHost(host);
    if (action === "reconnect") { const consent = Boolean(document.getElementById("ssh-agent-authorized")?.checked); let password = null; if (host.auth_method === "password") password = await window.AtlasUI.prompt("SSH password", "", { message: `Password for ${host.label}. It is kept only for this connection session.`, placeholder: "Password", sensitive: true }); if (host.auth_method === "password" && password == null) return; await api("/api/ssh/reconnect", { host_id: host.id, password, agent_authorized: consent }); return refresh(); }
    if (action === "disconnect") { await api("/api/ssh/disconnect", { host_id: host.id }); files = null; consoleOutput += `\nDisconnected from ${host.label}.`; return refresh(); }
    if (action === "detect") { await api("/api/ssh/detect", { host_id: host.id }); return refresh(); }
    if (action === "edit") return showHostDialog(host);
    if (action === "delete") { if (!await window.AtlasUI.confirm(`Delete remote host ${host.label}?`, { title: "Delete remote host", confirmLabel: "Delete", danger: true })) return; await api("/api/ssh/hosts/delete", { host_id: host.id }); selectedHostId = null; return refresh(); }
    if (action === "browse") { const path = document.getElementById("ssh-path")?.value || host.remote_root; files = await api("/api/ssh/files", { host_id: host.id, path }); return renderWorkspace(); }
    if (action === "execute") { const input = document.getElementById("ssh-command"); const command = input?.value.trim(); if (!command) return; if (selectedTerminalId) { await api("/api/ssh/terminals/input", { terminal_id: selectedTerminalId, input: `${command}\n` }); input.value = ""; await refresh({ quiet: true }); } else { consoleOutput += `\n$ ${command}\n`; const result = await api("/api/ssh/execute", { host_id: host.id, command }); consoleOutput += result.output; renderWorkspace(); } requestAnimationFrame(() => { const pre = panel.querySelector(".ssh-terminal-output"); if (pre) pre.scrollTop = pre.scrollHeight; }); return; }
    if (action === "new-terminal") { const created = await api("/api/ssh/terminals/create", { host_id: host.id, title: `Session ${snapshot.terminals.length + 1}` }); selectedTerminalId = created.id; return refresh(); }
    if (action === "close-terminal" && selectedTerminalId) { await api("/api/ssh/terminals/close", { id: selectedTerminalId }); selectedTerminalId = null; return refresh(); }
    if (action === "transfer") { const direction = await window.AtlasUI.prompt("Transfer direction", "sync", { message: "Enter upload, download or sync." }); if (!direction) return; const local_path = await window.AtlasUI.prompt("Local path", "", { message: "Workspace-relative local path" }); if (!local_path) return; const remote_path = await window.AtlasUI.prompt("Remote path"); if (!remote_path) return; const result = await api("/api/ssh/transfer", { host_id: host.id, direction, local_path, remote_path }); consoleOutput += `\n${result.direction}: ${result.local_path} ↔ ${result.remote_path}`; return renderWorkspace(); }
    if (action === "forward") { const kind = await window.AtlasUI.prompt("Forward kind", "local", { message: "Enter local, remote or dynamic." }); if (!kind) return; const bind = await window.AtlasUI.prompt("Bind endpoint", "127.0.0.1:8888"); if (!bind) return; const target = kind === "dynamic" ? null : await window.AtlasUI.prompt("Target endpoint", "127.0.0.1:8888"); if (kind !== "dynamic" && !target) return; await api("/api/ssh/forwards/start", { host_id: host.id, kind, bind, target }); return refresh(); }
  }

  panel.addEventListener("click", async (event) => { try { const hostButton = event.target.closest("[data-ssh-host]"); if (hostButton) { selectedHostId = hostButton.dataset.sshHost; files = null; render(); return; } const terminalButton = event.target.closest("[data-ssh-terminal]"); if (terminalButton) { selectedTerminalId = terminalButton.dataset.sshTerminal; renderWorkspace(); return; } const stop = event.target.closest("[data-ssh-forward-stop]"); if (stop) { await api("/api/ssh/forwards/stop", { id: stop.dataset.sshForwardStop }); await refresh(); return; } const button = event.target.closest("[data-ssh-action]"); if (button && !button.disabled) await runAction(button.dataset.sshAction); } catch (error) { notify(error); } });
  panel.addEventListener("keydown", (event) => { if (event.key === "Enter" && event.target.id === "ssh-command") { event.preventDefault(); runAction("execute").catch(notify); } });
  document.getElementById("ssh-add-host")?.addEventListener("click", () => showHostDialog());
  document.getElementById("ssh-refresh")?.addEventListener("click", () => refresh());
  document.getElementById("ssh-import-config")?.addEventListener("click", async () => { try { await api("/api/ssh/config/import", {}); await refresh(); } catch (error) { notify(error); } });
  document.getElementById("ssh-rail-button")?.addEventListener("click", () => { refresh(); clearInterval(pollTimer); pollTimer = setInterval(monitorConnections, 10000); });
  window.addEventListener("beforeunload", () => clearInterval(pollTimer));
  refresh({ quiet: true });
})();
