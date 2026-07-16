(function initializeAtlasResearchOS() {
  "use strict";

  const NS = "http://www.w3.org/2000/svg";
  const TYPE_LABELS = Object.freeze({
    hypothesis: "Hypotheses", experiment: "Experiments", evidence: "Evidence",
    failure: "Negative results", decision: "Decisions", memory: "Research memory",
    publication: "Publications", paper: "Papers", dataset: "Datasets", method: "Methods",
    concept: "Concepts", result: "Results",
  });
  const TYPE_ORDER = ["hypothesis", "experiment", "evidence", "failure", "decision", "memory", "publication", "paper", "dataset", "method", "concept", "result"];
  const elements = {};
  const state = {
    open: false,
    tab: "graph",
    snapshot: null,
    selectedId: "",
    query: "",
    timeIndex: -1,
    loading: false,
    pollTimer: null,
    graphTransform: { x: 0, y: 0, scale: 1 },
  };

  function el(tag, className = "", text = "") {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text) node.textContent = text;
    return node;
  }

  function svg(tag, attributes = {}) {
    const node = document.createElementNS(NS, tag);
    Object.entries(attributes).forEach(([key, value]) => node.setAttribute(key, String(value)));
    return node;
  }

  async function requestSnapshot({ quiet = false } = {}) {
    if (state.loading) return state.snapshot;
    state.loading = true;
    if (!quiet) setSyncState("syncing", "SYNCING");
    try {
      const response = await fetch("/api/research-os/snapshot", { headers: { Accept: "application/json" } });
      if (!response.ok) throw new Error((await response.text()) || `HTTP ${response.status}`);
      const payload = await response.json();
      if (payload?.ok === false) throw new Error(payload.error || "Research OS unavailable");
      const previousRevision = state.snapshot?.generated_at;
      state.snapshot = payload?.data || payload;
      if (!state.selectedId || !objectById(state.selectedId)) state.selectedId = defaultSelectionId();
      syncTimeMachine();
      if (state.open && previousRevision !== state.snapshot?.generated_at) render();
      setSyncState("live", "LIVE");
      return state.snapshot;
    } catch (error) {
      setSyncState("error", "OFFLINE");
      if (!quiet) renderError(error);
      return null;
    } finally {
      state.loading = false;
    }
  }

  function defaultSelectionId() {
    const graphNodes = state.snapshot?.graph?.nodes || [];
    return graphNodes.find((item) => item.object_type === "hypothesis")?.id
      || graphNodes[0]?.id
      || "";
  }

  function objectById(id) {
    return (state.snapshot?.graph?.nodes || []).find((item) => item.id === id) || null;
  }

  function setSyncState(kind, label) {
    if (!elements.sync) return;
    elements.sync.className = `research-os-sync-state is-${kind}`;
    elements.sync.replaceChildren(el("i"), document.createTextNode(` ${label}`));
  }

  function openPanel() {
    state.open = true;
    elements.panel.hidden = false;
    elements.toggle.classList.add("is-active");
    elements.toggle.setAttribute("aria-pressed", "true");
    document.querySelector(".app-shell")?.classList.add("has-research-os");
    requestSnapshot();
    schedulePolling();
  }

  function closePanel() {
    state.open = false;
    elements.panel.hidden = true;
    elements.toggle.classList.remove("is-active");
    elements.toggle.setAttribute("aria-pressed", "false");
    document.querySelector(".app-shell")?.classList.remove("has-research-os");
    window.clearTimeout(state.pollTimer);
    state.pollTimer = null;
  }

  function schedulePolling() {
    window.clearTimeout(state.pollTimer);
    if (!state.open) return;
    state.pollTimer = window.setTimeout(async () => {
      await requestSnapshot({ quiet: true });
      schedulePolling();
    }, 2500);
  }

  function selectTab(tab) {
    state.tab = tab;
    elements.tabs.forEach((button) => {
      const active = button.dataset.tab === tab;
      button.classList.toggle("is-active", active);
      button.setAttribute("aria-selected", active ? "true" : "false");
    });
    renderMain();
  }

  function selectObject(id, { switchTab = false } = {}) {
    if (!id || !objectById(id)) return;
    state.selectedId = id;
    if (switchTab) {
      const type = objectById(id)?.object_type;
      if (type === "hypothesis") state.tab = "hypotheses";
      else if (type === "evidence") state.tab = "evidence";
      else if (type === "experiment") state.tab = "lineage";
      else if (type === "decision") state.tab = "decisions";
      else if (type === "memory") state.tab = "memory";
      else if (type === "publication") state.tab = "publication";
      elements.tabs.forEach((button) => {
        const active = button.dataset.tab === state.tab;
        button.classList.toggle("is-active", active);
        button.setAttribute("aria-selected", active ? "true" : "false");
      });
    }
    renderTree();
    renderMain();
    renderInspector();
  }

  function visibleAtTime(node) {
    const events = state.snapshot?.timeline || [];
    if (state.timeIndex < 0 || state.timeIndex >= events.length - 1) return true;
    const cutoff = Date.parse(events[state.timeIndex]?.timestamp || "");
    const timestamp = Date.parse(node.timestamp || "");
    return !Number.isFinite(timestamp) || !Number.isFinite(cutoff) || timestamp <= cutoff;
  }

  function filteredGraphNodes() {
    const query = state.query.trim().toLowerCase();
    return (state.snapshot?.graph?.nodes || []).filter((node) => visibleAtTime(node)
      && (!query || [node.label, node.object_type, node.status, node.domain_id]
        .some((value) => String(value || "").toLowerCase().includes(query))));
  }

  function render() {
    renderTree();
    renderMain();
    renderInspector();
    renderWarnings();
  }

  function renderTree() {
    if (!elements.tree) return;
    elements.tree.replaceChildren();
    const nodes = filteredGraphNodes();
    const groups = new Map();
    nodes.forEach((node) => {
      const type = TYPE_LABELS[node.object_type] ? node.object_type : "result";
      if (!groups.has(type)) groups.set(type, []);
      groups.get(type).push(node);
    });
    TYPE_ORDER.forEach((type) => {
      const items = groups.get(type) || [];
      if (!items.length) return;
      const section = el("section", "research-os-tree-group");
      const header = el("button", "research-os-tree-group-head");
      header.type = "button";
      header.append(el("span", "", TYPE_LABELS[type] || type), el("code", "", String(items.length)));
      const body = el("div", "research-os-tree-items");
      items.slice(0, 100).forEach((item) => {
        const button = el("button", `research-os-tree-item type-${item.object_type}${item.id === state.selectedId ? " is-active" : ""}`);
        button.type = "button";
        button.dataset.objectId = item.id;
        button.append(el("i"), el("span", "", item.label || item.id), el("small", "", item.status || "recorded"));
        button.addEventListener("click", () => selectObject(item.id));
        body.appendChild(button);
      });
      header.addEventListener("click", () => body.toggleAttribute("hidden"));
      section.append(header, body);
      elements.tree.appendChild(section);
    });
    if (!elements.tree.children.length) elements.tree.appendChild(emptyState("No research objects match this view."));
  }

  function renderMain() {
    if (!elements.content) return;
    elements.content.replaceChildren();
    if (!state.snapshot) {
      elements.content.appendChild(el("div", "research-os-loading", "Synchronizing Research OS…"));
      return;
    }
    const renderers = {
      graph: renderKnowledgeGraph,
      hypotheses: renderHypothesisLifecycle,
      evidence: renderEvidenceEngine,
      lineage: renderExperimentLineage,
      diary: renderDiary,
      timeline: renderScientificTimeline,
      decisions: renderDecisionBoard,
      memory: renderMemory,
      publication: renderPublication,
    };
    elements.content.appendChild((renderers[state.tab] || renderKnowledgeGraph)());
  }

  function surfaceHeader(kicker, title, meta = "") {
    const header = el("header", "research-os-surface-head");
    const copy = el("div");
    copy.append(el("span", "", kicker), el("strong", "", title));
    header.append(copy, el("code", "", meta));
    return header;
  }

  function renderKnowledgeGraph() {
    const shell = el("section", "research-os-surface research-os-graph-surface");
    const nodes = filteredGraphNodes().slice(0, 120);
    const nodeIds = new Set(nodes.map((item) => item.id));
    const edges = (state.snapshot.graph?.edges || []).filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)).slice(0, 300);
    shell.appendChild(surfaceHeader("KNOWLEDGE GRAPH", "Connected research objects", `${nodes.length} nodes · ${edges.length} relations`));
    const viewport = el("div", "research-os-graph-viewport");
    const stage = el("div", "research-os-graph-stage");
    const lines = svg("svg", { viewBox: "0 0 1000 620", preserveAspectRatio: "none", "aria-label": "Research knowledge graph relations" });
    const positions = graphPositions(nodes);
    edges.forEach((edge) => {
      const source = positions.get(edge.source);
      const target = positions.get(edge.target);
      if (!source || !target) return;
      const line = svg("line", { x1: source.x, y1: source.y, x2: target.x, y2: target.y });
      line.dataset.relation = edge.relation || "related";
      lines.appendChild(line);
      if (Math.abs(source.x - target.x) + Math.abs(source.y - target.y) > 180) {
        const label = svg("text", { x: (source.x + target.x) / 2, y: (source.y + target.y) / 2 });
        label.textContent = edge.relation || "related";
        lines.appendChild(label);
      }
    });
    stage.appendChild(lines);
    nodes.forEach((node) => {
      const position = positions.get(node.id);
      const button = el("button", `research-os-graph-node type-${node.object_type}${node.id === state.selectedId ? " is-active" : ""}`);
      button.type = "button";
      button.style.left = `${(position.x / 1000) * 100}%`;
      button.style.top = `${(position.y / 620) * 100}%`;
      button.append(el("i"), el("strong", "", truncate(node.label, 42)), el("span", "", node.object_type));
      button.addEventListener("click", () => selectObject(node.id));
      stage.appendChild(button);
    });
    if (!nodes.length) stage.appendChild(emptyState("Research objects will appear here as Atlas creates hypotheses, runs experiments and records evidence."));
    viewport.appendChild(stage);
    bindGraphNavigation(viewport, stage);
    shell.appendChild(viewport);
    return shell;
  }

  function graphPositions(nodes) {
    const lanes = new Map(TYPE_ORDER.map((type, index) => [type, index]));
    const grouped = new Map();
    nodes.forEach((node) => {
      const lane = lanes.has(node.object_type) ? node.object_type : "result";
      if (!grouped.has(lane)) grouped.set(lane, []);
      grouped.get(lane).push(node);
    });
    const activeLanes = [...grouped.keys()].sort((a, b) => (lanes.get(a) || 99) - (lanes.get(b) || 99));
    const positions = new Map();
    activeLanes.forEach((type, laneIndex) => {
      const items = grouped.get(type);
      items.forEach((node, index) => {
        const seed = hashCode(node.id);
        const column = index % Math.max(1, Math.ceil(Math.sqrt(items.length)));
        const row = Math.floor(index / Math.max(1, Math.ceil(Math.sqrt(items.length))));
        positions.set(node.id, {
          x: 75 + laneIndex * (850 / Math.max(1, activeLanes.length - 1)) + ((seed % 29) - 14),
          y: 75 + row * 125 + column * 46 + ((seed % 31) - 15),
        });
      });
    });
    return positions;
  }

  function bindGraphNavigation(viewport, stage) {
    const apply = () => { stage.style.transform = `translate(${state.graphTransform.x}px, ${state.graphTransform.y}px) scale(${state.graphTransform.scale})`; };
    viewport.addEventListener("wheel", (event) => {
      event.preventDefault();
      state.graphTransform.scale = Math.max(.62, Math.min(1.8, state.graphTransform.scale * (event.deltaY > 0 ? .92 : 1.08)));
      apply();
    }, { passive: false });
    let drag = null;
    viewport.addEventListener("pointerdown", (event) => {
      if (event.target.closest("button")) return;
      drag = { x: event.clientX, y: event.clientY, tx: state.graphTransform.x, ty: state.graphTransform.y };
      viewport.setPointerCapture(event.pointerId);
    });
    viewport.addEventListener("pointermove", (event) => {
      if (!drag) return;
      state.graphTransform.x = drag.tx + event.clientX - drag.x;
      state.graphTransform.y = drag.ty + event.clientY - drag.y;
      apply();
    });
    viewport.addEventListener("pointerup", () => { drag = null; });
    apply();
  }

  function renderHypothesisLifecycle() {
    const shell = el("section", "research-os-surface research-os-hypothesis-surface");
    const hypotheses = (state.snapshot.hypotheses || []).filter((item) => visibleAtTime({ timestamp: item.updated_at }));
    shell.appendChild(surfaceHeader("HYPOTHESIS LIFECYCLE", "Ideas under test", `${hypotheses.length} hypotheses`));
    const body = el("div", "research-os-hypothesis-layout");
    const lifecycle = ["draft", "active", "validated", "refuted", "abandoned"];
    lifecycle.forEach((status) => {
      const lane = el("section", `research-os-hypothesis-lane status-${status}`);
      lane.appendChild(el("header", "", status.toUpperCase()));
      hypotheses.filter((item) => item.status === status).forEach((item) => {
        const confidence = Number(state.snapshot.hypothesis_confidence?.[item.id] || 0);
        const button = el("button", `research-os-hypothesis-card${item.id === state.selectedId ? " is-active" : ""}`);
        button.type = "button";
        button.append(el("strong", "", item.title), el("p", "", truncate(item.description, 150)));
        const evidence = el("span", "research-os-confidence");
        const fill = el("i");
        fill.style.width = `${Math.round(confidence * 100)}%`;
        evidence.append(fill, el("code", "", `${Math.round(confidence * 100)}% evidence confidence`));
        button.append(evidence, el("small", "", `${item.domain_id} · ${item.evidence_ids?.length || 0} linked evidence`));
        button.addEventListener("click", () => selectObject(item.id));
        lane.appendChild(button);
      });
      if (lane.children.length === 1) lane.appendChild(el("div", "research-os-lane-empty", "No objects"));
      body.appendChild(lane);
    });
    shell.appendChild(body);
    return shell;
  }

  function renderEvidenceEngine() {
    const shell = el("section", "research-os-surface research-os-evidence-surface");
    const hypotheses = state.snapshot.hypotheses || [];
    const evidence = state.snapshot.evidence || [];
    const experiments = state.snapshot.experiments || [];
    const experimentById = new Map(experiments.map((item) => [item.id, item]));
    shell.appendChild(surfaceHeader("EVIDENCE ENGINE", "Hypothesis → experiment → evidence chains", `${evidence.length} evidence records`));
    const body = el("div", "research-os-evidence-body");
    hypotheses.forEach((hypothesis) => {
      const linked = evidence.filter((item) => item.hypothesis_id === hypothesis.id
        || (item.experiment_id && experimentById.get(item.experiment_id)?.hypothesis_id === hypothesis.id));
      const chain = el("article", `research-os-evidence-chain${!linked.length ? " is-missing" : ""}`);
      const head = el("header");
      head.append(el("strong", "", hypothesis.title), el("code", "", `${linked.length} evidence`));
      chain.appendChild(head);
      if (!linked.length) {
        chain.appendChild(el("p", "research-os-evidence-warning", "No evidence linked yet. Do not mark this hypothesis validated or refuted."));
      } else {
        const matrix = el("div", "research-os-evidence-matrix");
        linked.forEach((item) => {
          const experiment = item.experiment_id ? experimentById.get(item.experiment_id) : null;
          const row = el("button", `research-os-evidence-row${item.supports ? " supports" : " rejects"}${item.id === state.selectedId ? " is-active" : ""}`);
          row.type = "button";
          row.append(
            el("span", "research-os-evidence-verdict", item.supports ? "SUPPORTS" : "REJECTS"),
            el("strong", "", truncate(item.summary, 90)),
            el("small", "", experiment ? experiment.title : (item.source_path || item.kind)),
          );
          const strength = el("span", "research-os-confidence");
          const fill = el("i");
          fill.style.width = `${Math.round(Number(item.strength || 0) * 100)}%`;
          strength.append(fill, el("code", "", `${Math.round(Number(item.strength || 0) * 100)}%`));
          row.appendChild(strength);
          row.addEventListener("click", () => selectObject(item.id));
          matrix.appendChild(row);
        });
        chain.appendChild(matrix);
      }
      chain.addEventListener("click", (event) => { if (event.target === chain || event.target === head) selectObject(hypothesis.id); });
      body.appendChild(chain);
    });
    const linkedIds = new Set();
    hypotheses.forEach((hypothesis) => evidence.forEach((item) => {
      if (item.hypothesis_id === hypothesis.id || (item.experiment_id && experimentById.get(item.experiment_id)?.hypothesis_id === hypothesis.id)) linkedIds.add(item.id);
    }));
    const orphaned = evidence.filter((item) => !linkedIds.has(item.id));
    if (orphaned.length) {
      const section = el("article", "research-os-evidence-chain research-os-evidence-orphaned");
      section.appendChild(el("header", "", `${orphaned.length} unlinked evidence records`));
      const matrix = el("div", "research-os-evidence-matrix");
      orphaned.forEach((item) => {
        const row = el("button", `research-os-evidence-row${item.supports ? " supports" : " rejects"}${item.id === state.selectedId ? " is-active" : ""}`);
        row.type = "button";
        row.append(el("span", "research-os-evidence-verdict", item.supports ? "SUPPORTS" : "REJECTS"), el("strong", "", truncate(item.summary, 90)), el("small", "", item.kind));
        row.addEventListener("click", () => selectObject(item.id));
        matrix.appendChild(row);
      });
      section.appendChild(matrix);
      body.appendChild(section);
    }
    if (!body.children.length) body.appendChild(emptyState("Evidence chains connect hypotheses, experiments and conclusions once Atlas records results."));
    shell.appendChild(body);
    return shell;
  }

  function renderExperimentLineage() {
    const shell = el("section", "research-os-surface research-os-lineage-surface");
    const experiments = (state.snapshot.experiments || []).filter((item) => visibleAtTime({ timestamp: item.updated_at }));
    shell.appendChild(surfaceHeader("EXPERIMENT LINEAGE", "Forks, parents and verified outputs", `${experiments.length} experiment nodes`));
    const canvas = el("div", "research-os-lineage-canvas");
    const byId = new Map(experiments.map((item) => [item.id, item]));
    const roots = experiments.filter((item) => !(item.parent_experiment_ids || []).some((id) => byId.has(id)));
    const visited = new Set();
    const renderBranch = (item, depth = 0) => {
      if (!item || visited.has(item.id)) return;
      visited.add(item.id);
      const row = el("div", `research-os-lineage-row status-${item.status}${item.id === state.selectedId ? " is-active" : ""}`);
      row.style.setProperty("--lineage-depth", depth);
      row.append(el("i"), el("strong", "", item.title), el("code", "", item.status), el("span", "", `${item.artifacts?.length || 0} artifacts`));
      row.addEventListener("click", () => selectObject(item.id));
      canvas.appendChild(row);
      experiments.filter((candidate) => (candidate.parent_experiment_ids || []).includes(item.id)).forEach((child) => renderBranch(child, depth + 1));
    };
    roots.forEach((root) => renderBranch(root));
    experiments.filter((item) => !visited.has(item.id)).forEach((item) => renderBranch(item));
    if (!experiments.length) canvas.appendChild(emptyState("Domain Agent tasks automatically become experiment lineage nodes."));
    shell.appendChild(canvas);
    return shell;
  }

  function renderDiary() {
    const shell = el("section", "research-os-surface research-os-diary-surface");
    const entries = (state.snapshot.diary || []).filter((item) => visibleAtTime({ timestamp: item.timestamp }));
    shell.appendChild(surfaceHeader("RESEARCH DIARY", "Automatic activity stream", `${entries.length} entries`));
    const body = el("div", "research-os-diary-body");
    const groups = new Map();
    entries.forEach((entry) => {
      const day = (entry.timestamp || "").slice(0, 10) || "undated";
      if (!groups.has(day)) groups.set(day, []);
      groups.get(day).push(entry);
    });
    [...groups.keys()].sort((a, b) => (a < b ? 1 : -1)).forEach((day) => {
      const dayEntries = groups.get(day);
      const group = el("section", "research-os-diary-day");
      const header = el("header");
      header.append(el("strong", "", day), el("code", "", `${dayEntries.length} events`));
      group.appendChild(header);
      const stream = el("div", "research-os-diary-stream");
      dayEntries.forEach((entry) => {
        const row = el("article", `research-os-diary-entry type-${entry.entry_type}${entry.id === state.selectedId ? " is-active" : ""}`);
        row.append(
          el("span", "research-os-diary-time", formatTime(entry.timestamp)),
          el("i"),
          el("div", "research-os-diary-copy"),
        );
        const copy = row.querySelector(".research-os-diary-copy");
        copy.append(el("p", "", entry.content), el("small", "", `${entry.author}${entry.domain_id ? ` · ${entry.domain_id}` : ""}`));
        row.addEventListener("click", () => {
          const related = entry.related_objects?.find((item) => objectById(item.id));
          if (related) selectObject(related.id);
        });
        stream.appendChild(row);
      });
      group.appendChild(stream);
      body.appendChild(group);
    });
    if (!body.children.length) body.appendChild(emptyState("Every domain task, agent turn and status transition writes a diary entry automatically."));
    shell.appendChild(body);
    return shell;
  }

  function renderScientificTimeline() {
    const shell = el("section", "research-os-surface research-os-timeline-surface");
    const events = (state.snapshot.timeline || []).slice().reverse();
    shell.appendChild(surfaceHeader("SCIENTIFIC TIMELINE", "Replay every research transition", `${events.length} recorded moments`));
    const lanes = el("div", "research-os-timeline-lanes");
    const types = ["hypothesis_created", "experiment_run", "evidence_added", "decision_made", "failure_recorded", "publication_drafted"];
    types.forEach((type) => {
      const laneEvents = events.filter((item) => item.event_type === type);
      if (!laneEvents.length) return;
      const lane = el("section", `research-os-event-lane event-${type}`);
      lane.appendChild(el("header", "", type.replaceAll("_", " ").toUpperCase()));
      const track = el("div");
      laneEvents.forEach((event) => {
        const button = el("button", "research-os-event");
        button.type = "button";
        button.append(el("strong", "", event.title), el("span", "", formatTime(event.timestamp)), el("p", "", truncate(event.description, 170)));
        button.addEventListener("click", () => {
          const related = event.related_objects?.find((item) => objectById(item.id));
          if (related) selectObject(related.id);
        });
        track.appendChild(button);
      });
      lane.appendChild(track);
      lanes.appendChild(lane);
    });
    if (!lanes.children.length) lanes.appendChild(emptyState("Timeline events are written automatically when research state changes."));
    shell.appendChild(lanes);
    return shell;
  }

  function renderDecisionBoard() {
    const shell = el("section", "research-os-surface research-os-decision-surface");
    const decisions = state.snapshot.decisions || [];
    shell.appendChild(surfaceHeader("DECISION ENGINE", "Evidence-backed choices", `${decisions.length} decision records`));
    const list = el("div", "research-os-decision-list");
    decisions.forEach((decision) => {
      const article = el("article", `research-os-decision${decision.id === state.selectedId ? " is-active" : ""}`);
      const head = el("header");
      head.append(el("strong", "", decision.title), el("code", "", `${Math.round(Number(decision.decision_score || 0) * 100)} score`));
      const options = el("div", "research-os-decision-options");
      (decision.options || []).forEach((option) => {
        const chosen = option.id === decision.chosen_option_id;
        const section = el("section", chosen ? "is-chosen" : "");
        section.append(el("span", "", chosen ? "SELECTED" : "OPTION"), el("strong", "", option.label), el("small", "", option.estimated_cost || "Cost not recorded"));
        const tradeoffs = el("div");
        tradeoffs.append(el("p", "is-pro", (option.pros || []).join(" · ") || "No supporting factors recorded"), el("p", "is-con", (option.cons || []).join(" · ") || "No risks recorded"));
        section.appendChild(tradeoffs);
        options.appendChild(section);
      });
      article.append(head, el("p", "research-os-decision-context", decision.context), options, el("footer", "", decision.rationale));
      article.addEventListener("click", () => selectObject(decision.id));
      list.appendChild(article);
    });
    if (!decisions.length) list.appendChild(emptyState("Decision records appear when Atlas compares research options with cost, risk and evidence."));
    shell.appendChild(list);
    return shell;
  }

  function renderMemory() {
    const shell = el("section", "research-os-surface research-os-memory-surface");
    const memory = state.snapshot.memory || [];
    shell.appendChild(surfaceHeader("RESEARCH MEMORY", "Reusable lessons and assumptions", `${memory.length} retained memories`));
    const field = el("div", "research-os-memory-field");
    memory.forEach((item, index) => {
      const button = el("button", `research-os-memory-node${item.id === state.selectedId ? " is-active" : ""}`);
      button.type = "button";
      button.style.setProperty("--memory-x", `${8 + ((index * 37) % 82)}%`);
      button.style.setProperty("--memory-y", `${10 + ((index * 53) % 72)}%`);
      button.style.setProperty("--memory-weight", String(.8 + Number(item.importance || 0) * .45));
      button.append(el("strong", "", truncate(item.content, 70)), el("span", "", `${Math.round(Number(item.importance || 0) * 100)} importance · ${item.accessed_count || 0} recalls`));
      button.addEventListener("click", () => selectObject(item.id));
      field.appendChild(button);
    });
    if (!memory.length) field.appendChild(emptyState("Research lessons, patterns and mistakes will persist here across sessions."));
    shell.appendChild(field);
    return shell;
  }

  function renderPublication() {
    const shell = el("section", "research-os-surface research-os-publication-surface");
    const publications = state.snapshot.publications || [];
    shell.appendChild(surfaceHeader("PUBLICATION PIPELINE", "Claims traced back to evidence", `${publications.length} publication drafts`));
    const pipeline = el("div", "research-os-publication-pipeline");
    ["draft", "review", "ready", "published"].forEach((status) => {
      const column = el("section", `research-os-publication-column status-${status}`);
      column.appendChild(el("header", "", status.toUpperCase()));
      publications.filter((item) => item.status === status).forEach((item) => {
        const button = el("button", item.id === state.selectedId ? "is-active" : "");
        button.type = "button";
        button.append(el("strong", "", item.title), el("span", "", `${item.sections?.length || 0} sections`), el("small", "", `${item.evidence_ids?.length || 0} evidence · ${item.experiment_ids?.length || 0} experiments`));
        button.addEventListener("click", () => selectObject(item.id));
        column.appendChild(button);
      });
      if (column.children.length === 1) column.appendChild(el("div", "research-os-lane-empty", "No objects"));
      pipeline.appendChild(column);
    });
    shell.appendChild(pipeline);
    return shell;
  }

  function renderInspector() {
    if (!elements.inspector) return;
    elements.inspector.replaceChildren();
    const node = objectById(state.selectedId);
    if (!node) {
      elements.inspector.appendChild(emptyState("Select a research object to inspect its evidence and provenance."));
      return;
    }
    const header = el("header", "research-os-inspector-head");
    header.append(el("span", "", String(node.object_type || "object").toUpperCase()), el("strong", "", node.label || node.id), el("code", "", String(node.id).slice(0, 12)));
    elements.inspector.appendChild(header);
    const facts = el("div", "research-os-inspector-facts");
    const factRows = {
      Status: node.status || "recorded",
      Domain: node.domain_id || "cross-domain",
      Confidence: node.confidence == null ? "—" : `${Math.round(Number(node.confidence) * 100)}%`,
      Updated: formatTime(node.timestamp),
    };
    Object.entries(factRows).forEach(([label, value]) => {
      const row = el("div");
      row.append(el("span", "", label), el("code", "", value));
      facts.appendChild(row);
    });
    elements.inspector.appendChild(facts);
    const relations = (state.snapshot.graph?.edges || []).filter((edge) => edge.source === node.id || edge.target === node.id);
    const related = el("section", "research-os-inspector-section");
    related.appendChild(el("header", "", "CONNECTED OBJECTS"));
    relations.slice(0, 24).forEach((edge) => {
      const otherId = edge.source === node.id ? edge.target : edge.source;
      const other = objectById(otherId);
      if (!other) return;
      const button = el("button");
      button.type = "button";
      button.append(el("span", "", edge.relation), el("strong", "", truncate(other.label, 58)), el("code", "", other.object_type));
      button.addEventListener("click", () => selectObject(other.id));
      related.appendChild(button);
    });
    if (related.children.length === 1) related.appendChild(el("p", "", "No graph relation has been recorded yet."));
    elements.inspector.appendChild(related);
    elements.inspector.appendChild(renderInspectorStructured(node));
    elements.inspector.appendChild(renderInspectorRawMetadata(node));
    const actions = el("footer", "research-os-inspector-actions");
    if (node.domain_id) {
      const open = el("button", "is-primary", "Open environment");
      open.type = "button";
      open.addEventListener("click", () => window.dispatchEvent(new CustomEvent("atlas:research-domain-open", { detail: { domainId: node.domain_id } })));
      actions.appendChild(open);
    }
    const ask = el("button", "", "Ask Atlas");
    ask.type = "button";
    ask.addEventListener("click", () => window.dispatchEvent(new CustomEvent("atlas:domain-agent-dispatch", {
      detail: { prompt: `Inspect Research OS ${node.object_type} ${node.id}: ${node.label}. Explain its evidence, contradictions, provenance, risks and next reproducible action.` },
    })));
    actions.appendChild(ask);
    elements.inspector.appendChild(actions);
  }

  const INSPECTOR_FIELD_GROUPS = Object.freeze({
    hypothesis: [
      ["EVIDENCE", ["evidence_ids", "experiment_ids", "current_confidence"]],
      ["NOTES", ["summary", "motivation", "problem", "novelty", "expected_result", "tags", "priority"]],
      ["LINKS", ["paper_ids", "dataset_ids", "model_ids", "task_ids", "visualization_ids", "publication_ids"]],
      ["PROVENANCE", ["created_by", "created_at", "owner", "version"]],
    ],
    experiment: [
      ["EVIDENCE", ["evidence_ids"]],
      ["ARTIFACTS", ["artifacts"]],
      ["RUNTIME", ["parameters"]],
      ["LINEAGE", ["hypothesis_id", "parent_experiment_ids", "child_experiment_ids"]],
      ["PROVENANCE", ["created_by", "created_at", "task_id"]],
    ],
    evidence: [
      ["EVIDENCE", ["kind", "verification_status", "verified_by", "verified_at"]],
      ["ARTIFACTS", ["attachment", "raw_data"]],
      ["RUNTIME", ["source_metadata"]],
      ["PROVENANCE", ["created_by", "source_path", "source_command", "hypothesis_id", "experiment_id"]],
    ],
    failure: [
      ["EVIDENCE", ["classification", "failure_score", "learned"]],
      ["RUNTIME", ["environment", "dataset", "checkpoint", "runtime_info", "gpu_info", "memory_info", "hyperparameters"]],
      ["ARTIFACTS", ["artifacts", "logs"]],
      ["PROVENANCE", ["created_by", "hypothesis_id", "experiment_id", "task_id"]],
    ],
    decision: [
      ["EVIDENCE", ["paper_support", "experiment_support", "failure_risk"]],
      ["RUNTIME", ["gpu_time", "cost", "risk"]],
      ["NOTES", ["rationale", "options", "expected_gain", "novelty"]],
      ["PROVENANCE", ["decided_by"]],
    ],
    publication: [
      ["EVIDENCE", ["evidence_ids", "hypothesis_ids", "experiment_ids"]],
      ["ARTIFACTS", ["artifact_paths", "sections"]],
      ["PROVENANCE", ["created_by", "created_at"]],
    ],
    memory: [
      ["EVIDENCE", ["accessed_count", "related_objects"]],
      ["PROVENANCE", ["created_at", "last_accessed_at"]],
    ],
  });

  function isEmptyMetaValue(value) {
    return value === null || value === undefined || value === ""
      || (Array.isArray(value) && value.length === 0)
      || (typeof value === "object" && !Array.isArray(value) && Object.keys(value).length === 0);
  }

  function formatMetaValue(value) {
    if (Array.isArray(value)) return value.length ? value.map((item) => (typeof item === "object" ? JSON.stringify(item) : String(item))).join(", ") : "—";
    if (typeof value === "object") return JSON.stringify(value);
    if (typeof value === "number") return String(value);
    return String(value);
  }

  function renderInspectorStructured(node) {
    const section = el("section", "research-os-inspector-section research-os-inspector-structured");
    const metadata = node.metadata || {};
    const groups = INSPECTOR_FIELD_GROUPS[node.object_type] || [];
    let rendered = 0;
    groups.forEach(([label, keys]) => {
      const rows = keys
        .map((key) => [key, metadata[key]])
        .filter(([, value]) => !isEmptyMetaValue(value));
      if (!rows.length) return;
      section.appendChild(el("header", "", label));
      rows.forEach(([key, value]) => {
        const row = el("div", "research-os-inspector-structured-row");
        row.append(el("span", "", key.replaceAll("_", " ")), el("code", "", truncate(formatMetaValue(value), 200)));
        section.appendChild(row);
        rendered += 1;
      });
    });
    if (!rendered) section.appendChild(el("p", "", "No structured evidence fields recorded yet."));
    return section;
  }

  function renderInspectorRawMetadata(node) {
    const details = el("details", "research-os-inspector-section research-os-metadata");
    details.appendChild(el("summary", "", "RAW METADATA (JSON)"));
    const pre = el("pre");
    pre.textContent = JSON.stringify(node.metadata || {}, null, 2);
    details.appendChild(pre);
    return details;
  }

  function renderWarnings() {
    if (!elements.warning) return;
    elements.warning.replaceChildren();
    const failures = state.snapshot?.warnings?.similar_failures || [];
    const unsupported = state.snapshot?.warnings?.unsupported_hypotheses || [];
    if (failures.length) {
      const button = el("button", "is-failure", `${failures.length} similar failure warning${failures.length === 1 ? "" : "s"}`);
      button.type = "button";
      button.addEventListener("click", () => selectObject(failures[0].id));
      elements.warning.appendChild(button);
    }
    if (unsupported.length) {
      const button = el("button", "is-evidence", `${unsupported.length} unsupported hypothes${unsupported.length === 1 ? "is" : "es"}`);
      button.type = "button";
      button.addEventListener("click", () => selectObject(unsupported[0].id, { switchTab: true }));
      elements.warning.appendChild(button);
    }
    if (!elements.warning.children.length) elements.warning.appendChild(el("span", "", "No unresolved research warnings"));
  }

  function syncTimeMachine() {
    const events = state.snapshot?.timeline || [];
    if (!events.length) {
      state.timeIndex = -1;
      if (elements.scrubber) { elements.scrubber.max = "0"; elements.scrubber.value = "0"; }
      if (elements.timeLabel) elements.timeLabel.textContent = "Current state";
      return;
    }
    if (state.timeIndex < 0 || state.timeIndex >= events.length) state.timeIndex = events.length - 1;
    if (elements.scrubber) { elements.scrubber.max = String(events.length - 1); elements.scrubber.value = String(state.timeIndex); }
    if (elements.timeLabel) elements.timeLabel.textContent = state.timeIndex === events.length - 1 ? "Current state" : `${formatTime(events[state.timeIndex].timestamp)} · ${events[state.timeIndex].title}`;
  }

  function setTimeIndex(index) {
    const events = state.snapshot?.timeline || [];
    if (!events.length) return;
    state.timeIndex = Math.max(0, Math.min(events.length - 1, Number(index)));
    syncTimeMachine();
    render();
  }

  function renderError(error) {
    if (!elements.content) return;
    const host = el("div", "research-os-error");
    host.append(el("strong", "", "Research OS could not synchronize"), el("p", "", error?.message || "Unknown error"));
    const retry = el("button", "", "Retry");
    retry.type = "button";
    retry.addEventListener("click", () => requestSnapshot());
    host.appendChild(retry);
    elements.content.replaceChildren(host);
  }

  function emptyState(text) { return el("div", "research-os-empty", text); }
  function truncate(value, limit) { const text = String(value || ""); return text.length > limit ? `${text.slice(0, limit - 1)}…` : text; }
  function hashCode(value) { return [...String(value || "")].reduce((hash, char) => ((hash << 5) - hash + char.charCodeAt(0)) | 0, 0) >>> 0; }
  function formatTime(value) {
    const date = new Date(value || "");
    if (Number.isNaN(date.getTime())) return "—";
    return date.toLocaleString([], { month: "short", day: "2-digit", hour: "2-digit", minute: "2-digit" });
  }

  function init() {
    elements.panel = document.getElementById("research-os-panel");
    elements.toggle = document.getElementById("research-os-toggle");
    elements.close = document.getElementById("research-os-close");
    elements.refresh = document.getElementById("research-os-refresh");
    elements.sync = document.getElementById("research-os-sync-state");
    elements.tabs = [...document.querySelectorAll(".research-os-tabs [data-tab]")];
    elements.content = document.getElementById("research-os-content");
    elements.tree = document.getElementById("research-os-tree");
    elements.inspector = document.getElementById("research-os-inspector");
    elements.search = document.getElementById("research-os-search");
    elements.warning = document.getElementById("research-os-warning");
    elements.scrubber = document.getElementById("research-os-scrubber");
    elements.timeLabel = document.getElementById("research-os-time-label");
    elements.prev = document.getElementById("research-os-prev");
    elements.next = document.getElementById("research-os-next");
    if (!elements.panel || !elements.toggle) return;
    elements.toggle.addEventListener("click", () => state.open ? closePanel() : openPanel());
    elements.close?.addEventListener("click", closePanel);
    elements.refresh?.addEventListener("click", () => requestSnapshot());
    elements.tabs.forEach((button) => button.addEventListener("click", () => selectTab(button.dataset.tab)));
    elements.search?.addEventListener("input", () => { state.query = elements.search.value || ""; renderTree(); if (state.tab === "graph") renderMain(); });
    elements.scrubber?.addEventListener("input", () => setTimeIndex(elements.scrubber.value));
    elements.prev?.addEventListener("click", () => setTimeIndex(state.timeIndex - 1));
    elements.next?.addEventListener("click", () => setTimeIndex(state.timeIndex + 1));
    window.addEventListener("atlas:research-environment-change", () => { if (state.open) window.setTimeout(() => requestSnapshot({ quiet: true }), 350); });
    window.addEventListener("atlas:research-os-open", (event) => {
      openPanel();
      if (event?.detail?.tab) selectTab(event.detail.tab);
      if (event?.detail?.objectId) window.setTimeout(() => selectObject(event.detail.objectId), 0);
    });
    window.AtlasResearchOS = Object.freeze({
      open: openPanel,
      close: closePanel,
      refresh: () => requestSnapshot(),
      selectObject,
      snapshot: () => state.snapshot,
    });
  }

  if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", init, { once: true });
  else init();
})();
