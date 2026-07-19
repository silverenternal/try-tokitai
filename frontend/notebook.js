(() => {
  "use strict";

  const host = document.getElementById("atlas-notebook-workspace");
  const rail = document.getElementById("notebook-rail-button");
  if (!host || !rail) return;

  let notebooks = [];
  let active = null;
  let saving = false;
  const esc = (value) => String(value ?? "").replace(/[&<>"']/g, (character) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[character]);

  async function api(path, body) {
    const response = await fetch(path, body === undefined ? {} : {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const payload = await response.json();
    if (!response.ok || payload?.ok === false) throw new Error(payload?.data?.message || payload?.message || `Notebook request failed (${response.status})`);
    return payload?.data ?? payload;
  }

  async function load() {
    notebooks = await api("/api/notebooks");
    if (!active) active = notebooks[0] || null;
    if (active) active = notebooks.find((notebook) => notebook.id === active.id) || active;
    render();
  }

  function markdown(source) {
    return String(source || "").split("\n").map((line) => {
      if (line.startsWith("# ")) return `<h1>${esc(line.slice(2))}</h1>`;
      if (line.startsWith("## ")) return `<h2>${esc(line.slice(3))}</h2>`;
      return line.trim() ? `<p>${esc(line)}</p>` : "<br>";
    }).join("");
  }

  function cellHtml(cell) {
    const isMarkdown = cell.kind === "markdown";
    return `<article class="notebook-cell kind-${cell.kind}" data-cell-id="${esc(cell.id)}">
      <div class="notebook-cell-gutter"><span>${isMarkdown ? "M" : `[${cell.execution_count || " "}]`}</span><button data-cell-run="${esc(cell.id)}" ${isMarkdown ? "disabled" : ""} aria-label="Run cell">▶</button></div>
      <div class="notebook-cell-body"><header><span>${isMarkdown ? "Markdown" : "Python"}</span><code>${cell.duration_ms ? `${cell.duration_ms} ms` : cell.status || "idle"}</code><button data-cell-delete="${esc(cell.id)}">Delete</button></header>
      ${isMarkdown ? `<div class="notebook-markdown-preview">${markdown(cell.source)}</div>` : ""}
      <textarea data-cell-source="${esc(cell.id)}" spellcheck="false">${esc(cell.source)}</textarea>
      ${cell.output ? `<pre class="notebook-output status-${esc(cell.status)}">${esc(cell.output)}</pre>` : ""}</div>
    </article>`;
  }

  function render() {
    host.innerHTML = `<header class="notebook-toolbar">
      <div><small>ATLAS COMPUTATIONAL DOCUMENT</small><input data-notebook-title value="${esc(active?.title || "Research Notebook")}" ${active ? "" : "disabled"}></div>
      <span class="notebook-kernel"><i></i>${esc(active?.kernel || "python")} kernel</span>
      <button data-notebook-new>New</button><button data-notebook-save ${active ? "" : "disabled"}>${saving ? "Saving…" : "Save"}</button><button data-notebook-run-all ${active ? "" : "disabled"}>Run all</button><button data-notebook-close aria-label="Close notebook">×</button>
    </header><div class="notebook-layout">
      <aside class="notebook-library"><header>NOTEBOOKS <code>${notebooks.length}</code></header>${notebooks.map((notebook) => `<button data-notebook-open="${esc(notebook.id)}" class="${active?.id === notebook.id ? "is-active" : ""}"><strong>${esc(notebook.title)}</strong><small>${new Date(notebook.updated_at).toLocaleString()}</small><code>${notebook.cells.length} cells</code></button>`).join("") || "<p>No notebooks yet.</p>"}</aside>
      <main class="notebook-canvas">${active ? active.cells.map(cellHtml).join("") + '<div class="notebook-add-row"><button data-cell-add="python">+ Python</button><button data-cell-add="markdown">+ Markdown</button></div>' : '<div class="notebook-welcome"><strong>Atlas Notebook</strong><p>Create an executable scientific document with Markdown and Python cells.</p><button data-notebook-new>Create notebook</button></div>'}</main>
      <aside class="notebook-outline"><header>DOCUMENT MAP</header>${active?.cells.map((cell, index) => `<button data-cell-focus="${esc(cell.id)}"><span>${index + 1}</span><strong>${cell.kind}</strong><code>${cell.status || "idle"}</code></button>`).join("") || ""}</aside>
    </div>`;
  }

  function collect() {
    if (!active) return;
    active.title = host.querySelector("[data-notebook-title]")?.value || active.title;
    host.querySelectorAll("[data-cell-source]").forEach((input) => {
      const cell = active.cells.find((item) => item.id === input.dataset.cellSource);
      if (cell) cell.source = input.value;
    });
  }

  async function save() {
    if (!active) return;
    collect();
    saving = true;
    active = await api("/api/notebooks/save", active);
    saving = false;
    await load();
  }

  async function execute(id) {
    collect();
    await save();
    active = await api("/api/notebooks/execute", { notebook_id: active.id, cell_id: id });
    await load();
  }

  host.addEventListener("click", async (event) => {
    try {
      const open = event.target.closest("[data-notebook-open]")?.dataset.notebookOpen;
      if (open) { collect(); active = notebooks.find((notebook) => notebook.id === open); render(); return; }
      if (event.target.closest("[data-notebook-new]")) { active = await api("/api/notebooks/create", { title: "Untitled Research Notebook" }); await load(); return; }
      if (event.target.closest("[data-notebook-save]")) { await save(); return; }
      if (event.target.closest("[data-notebook-close]")) { window.AtlasWorkspaceBridge?.openPanel?.(null); return; }
      const run = event.target.closest("[data-cell-run]")?.dataset.cellRun;
      if (run) { await execute(run); return; }
      const add = event.target.closest("[data-cell-add]")?.dataset.cellAdd;
      if (add && active) { collect(); active.cells.push({ id: crypto.randomUUID(), kind: add, source: add === "markdown" ? "## New section\n\nAdd research notes here." : "# New computation\n", output: "", status: "idle", execution_count: 0, duration_ms: 0 }); render(); return; }
      const remove = event.target.closest("[data-cell-delete]")?.dataset.cellDelete;
      if (remove && active) { active.cells = active.cells.filter((cell) => cell.id !== remove); render(); return; }
      const focus = event.target.closest("[data-cell-focus]")?.dataset.cellFocus;
      if (focus) host.querySelector(`[data-cell-id="${CSS.escape(focus)}"]`)?.scrollIntoView({ behavior: "smooth", block: "center" });
      if (event.target.closest("[data-notebook-run-all]") && active) for (const cell of active.cells.filter((item) => item.kind === "python")) await execute(cell.id);
    } catch (error) { console.error(error); }
  });

  rail.addEventListener("click", () => load().catch(console.error));
  window.AtlasNotebook = { activate: load };
})();
