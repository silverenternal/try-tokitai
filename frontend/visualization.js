(function initializeAtlasVisualization() {
  "use strict";

  const NS = "http://www.w3.org/2000/svg";
  const elements = {
    workspace: document.getElementById("visualization-workspace"),
    tabs: document.getElementById("visualization-type-tabs"),
    source: document.getElementById("visualization-source-select"),
    refresh: document.getElementById("visualization-refresh"),
    fit: document.getElementById("visualization-fit"),
    close: document.getElementById("visualization-close"),
    live: document.getElementById("visualization-live-state"),
    status: document.getElementById("visualization-status"),
    stage: document.getElementById("visualization-stage"),
    canvas3d: document.getElementById("visualization-3d-canvas"),
    canvas: document.getElementById("visualization-canvas"),
    performance: document.getElementById("visualization-performance"),
    playback: document.getElementById("visualization-playback"),
    viewport: document.getElementById("visualization-viewport"),
    series: document.getElementById("visualization-series-layer"),
    edges: document.getElementById("visualization-edge-layer"),
    nodes: document.getElementById("visualization-node-layer"),
    inspector: document.getElementById("visualization-inspector"),
    empty: document.getElementById("visualization-empty"),
    previous: document.getElementById("visualization-previous"),
    play: document.getElementById("visualization-play"),
    next: document.getElementById("visualization-next"),
    timeline: document.getElementById("visualization-timeline"),
    frameLabel: document.getElementById("visualization-frame-label"),
  };

  if (!elements.workspace || !elements.canvas || !elements.viewport) return;

  const state = {
    active: false,
    catalog: null,
    kind: "",
    sourceId: "",
    document: null,
    loading: false,
    transform: { x: 0, y: 0, scale: 1 },
    positions: new Map(),
    nodeElements: new Map(),
    edgeElements: new Map(),
    frame: 0,
    playTimer: null,
    pollTimer: null,
    requestId: 0,
    drag: null,
    panning: null,
    history: new Map(),
    followLive: true,
    renderExtensions: new Set(),
    resizeObserver: null,
    snapshotController: null,
    documentCache: new Map(),
    catalogPromise: null,
    lastPresentedKind: "",
    externalLoader: null,
    externalKind: "",
    viewer3d: null,
  };

  function svgElement(tag, attributes = {}) {
    const element = document.createElementNS(NS, tag);
    Object.entries(attributes).forEach(([key, value]) => {
      if (value !== undefined && value !== null) element.setAttribute(key, String(value));
    });
    return element;
  }

  async function requestJson(path, options = {}) {
    const response = await fetch(path, { ...options, headers: { Accept: "application/json", ...(options.headers || {}) } });
    if (!response.ok) throw new Error((await response.text()) || `HTTP ${response.status}`);
    const payload = await response.json();
    if (payload?.ok === false) throw new Error(payload?.error || "Visualization request failed");
    return payload?.data ?? payload;
  }

  function sourcesForKind(kind) {
    return (state.catalog?.sources || []).filter((source) => source.kind === kind);
  }

  function typeForKind(kind) {
    return visualizationTypes().find((type) => type.kind === kind) || null;
  }

  function visualizationTypes() {
    return (state.catalog?.types || []).filter((type) => type.kind !== "system");
  }

  async function loadCatalog({ preserveSelection = true } = {}) {
    const previousKind = preserveSelection ? state.kind : "";
    const previousSource = preserveSelection ? state.sourceId : "";
    setStatus("Discovering real visualization sources…");
    state.catalog = await requestJson("/api/visualizations");
    const types = visualizationTypes();
    state.kind = types.some((type) => type.kind === previousKind) ? previousKind : types[0]?.kind || "";
    renderTypeTabs();
    renderSourceOptions(previousSource);
    updatePresentationChrome();
  }

  async function ensureCatalog() {
    if (state.catalog) return state.catalog;
    if (!state.catalogPromise) {
      state.catalogPromise = loadCatalog().then(() => state.catalog).finally(() => {
        state.catalogPromise = null;
      });
    }
    return state.catalogPromise;
  }

  function renderTypeTabs() {
    elements.tabs.replaceChildren();
    for (const type of visualizationTypes()) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `visualization-type-tab${type.kind === state.kind ? " is-active" : ""}`;
      button.dataset.kind = type.kind;
      button.setAttribute("role", "tab");
      button.setAttribute("aria-selected", type.kind === state.kind ? "true" : "false");
      button.textContent = type.label;
      button.addEventListener("click", () => selectKind(type.kind));
      elements.tabs.appendChild(button);
    }
  }

  function renderSourceOptions(preferred = "") {
    const sources = sourcesForKind(state.kind);
    elements.source.replaceChildren();
    for (const source of sources) {
      const option = document.createElement("option");
      option.value = source.id;
      option.textContent = source.label;
      option.dataset.live = source.live ? "true" : "false";
      elements.source.appendChild(option);
    }
    state.sourceId = sources.some((source) => source.id === preferred) ? preferred : sources[0]?.id || "";
    elements.source.value = state.sourceId;
    elements.source.disabled = sources.length === 0;
    updateLiveState();
  }

  async function selectKind(kind) {
    if (!kind || kind === state.kind) return;
    cancelSnapshotRequest();
    stopPlayback();
    stopPolling();
    state.kind = kind;
    if (kind !== state.externalKind) state.externalLoader = null;
    state.positions.clear();
    renderTypeTabs();
    renderSourceOptions();
    updatePresentationChrome();
    const cached = state.documentCache.get(snapshotCacheKey());
    state.document = cached || null;
    renderLoadingState();
    await waitForPresentationLayout(kind);
    if (state.kind !== kind) return;
    if (cached) renderDocument();
    await loadSnapshot({ fit: true });
  }

  function waitForPresentationLayout(kind = state.kind) {
    const frames = kind === "system" ? 2 : 1;
    return new Promise((resolve) => {
      const next = (remaining) => window.requestAnimationFrame(() => remaining > 1 ? next(remaining - 1) : resolve());
      next(frames);
    });
  }

  function snapshotCacheKey(kind = state.kind, sourceId = state.sourceId) {
    return `${kind}:${sourceId}`;
  }

  function cancelSnapshotRequest() {
    state.requestId += 1;
    state.snapshotController?.abort();
    state.snapshotController = null;
    state.loading = false;
    elements.workspace.removeAttribute("aria-busy");
  }

  function updatePresentationChrome() {
    const systemPerformance = state.kind === "system";
    if (elements.playback) elements.playback.hidden = systemPerformance;
    elements.status.hidden = systemPerformance;
    elements.fit.hidden = systemPerformance;
    if (state.lastPresentedKind !== state.kind) {
      state.lastPresentedKind = state.kind;
      window.dispatchEvent(new CustomEvent("atlas:visualization-kind-change", {
        detail: { kind: state.kind },
      }));
    }
  }

  function visibleGraph(documentData = state.document || {}) {
    const allNodes = (documentData.nodes || []).filter(hasRenderableNodeData);
    const ids = new Set(allNodes.map((node) => node.id));
    return {
      nodes: allNodes,
      edges: (documentData.edges || []).filter((edge) => ids.has(edge.source) && ids.has(edge.target)),
    };
  }

  function visibleFrames() {
    return state.document?.frames || [];
  }

  function renderLoadingState() {
    state.viewer3d?.dispose?.();
    state.viewer3d = null;
    if (elements.canvas3d) elements.canvas3d.hidden = true;
    elements.series.replaceChildren();
    elements.edges.replaceChildren();
    elements.nodes.replaceChildren();
    elements.performance?.replaceChildren();
    if (elements.performance) elements.performance.hidden = true;
    elements.canvas.hidden = false;
    elements.stage.classList.remove("is-performance");
    renderEmpty("Loading real data…");
  }

  function currentSource() {
    return sourcesForKind(state.kind).find((source) => source.id === state.sourceId) || null;
  }

  function updateLiveState() {
    const source = currentSource();
    elements.live.classList.toggle("is-live", Boolean(source?.live));
    elements.live.textContent = source?.live ? "Live" : source ? "Artifact" : "";
  }

  async function loadSnapshot({ fit = false, quiet = false } = {}) {
    if (!state.kind || !state.sourceId) {
      if (!state.sourceId) renderEmpty("No real data source was discovered for this visualization type.");
      return;
    }
    const requestId = ++state.requestId;
    state.snapshotController?.abort();
    const controller = new AbortController();
    state.snapshotController = controller;
    state.loading = true;
    elements.refresh.disabled = true;
    elements.workspace.setAttribute("aria-busy", "true");
    if (!quiet) setStatus("Parsing source…");
    try {
      const params = new URLSearchParams({ kind: state.kind, source_id: state.sourceId });
      const document = await requestJson(`/api/visualizations/snapshot?${params}`, { signal: controller.signal });
      if (requestId !== state.requestId) return;
      mergeLiveSeries(document);
      deriveLiveSeriesFrames(document);
      state.document = document;
      state.documentCache.set(snapshotCacheKey(), document);
      renderDocument();
      if (fit) requestAnimationFrame(fitView);
      schedulePolling();
    } catch (error) {
      if (requestId !== state.requestId) return;
      if (error?.name === "AbortError") return;
      setStatus(error?.message || "Unable to render this source.", true);
      renderEmpty(error?.message || "Unable to render this source.");
    } finally {
      if (requestId === state.requestId) {
        state.loading = false;
        state.snapshotController = null;
        elements.refresh.disabled = false;
        elements.workspace.removeAttribute("aria-busy");
      }
    }
  }

  function mergeLiveSeries(document) {
    if (!document?.source?.live) return;
    for (const series of document.series || []) {
      const key = `${document.source.id}:${series.id}`;
      const previous = state.history.get(key) || [];
      const incoming = Array.isArray(series.points) ? series.points : [];
      const byTimestamp = new Map(previous.map((point) => [point.timestamp_ms, point]));
      incoming.forEach((point) => byTimestamp.set(point.timestamp_ms, point));
      const merged = Array.from(byTimestamp.values()).sort((a, b) => a.timestamp_ms - b.timestamp_ms).slice(-180);
      state.history.set(key, merged);
      series.points = merged;
    }
  }

  function deriveLiveSeriesFrames(document) {
    if (document?.kind === "system") return;
    if (!document?.source?.live || (document.frames || []).length) return;
    const series = (document.series || []).filter((item) => (item.points || []).length);
    if (!series.length) return;
    const timestamps = Array.from(new Set(series.flatMap((item) => item.points.map((point) => point.timestamp_ms))))
      .sort((left, right) => left - right);
    document.frames = timestamps.map((timestamp, sequence) => {
      const metrics = {};
      const activeNodes = [];
      for (const item of series) {
        const point = item.points.find((candidate) => candidate.timestamp_ms === timestamp);
        if (!point) continue;
        metrics[item.id] = point.value;
        if (item.node_id) activeNodes.push(item.node_id);
      }
      return {
        id: `live-series-frame:${timestamp}`,
        sequence,
        label: new Date(Number(timestamp)).toLocaleTimeString(),
        active_nodes: activeNodes,
        active_edges: [],
        metrics,
      };
    });
  }

  function renderDocument() {
    state.viewer3d?.dispose?.();
    state.viewer3d = null;
    if (elements.canvas3d) elements.canvas3d.hidden = true;
    elements.series.replaceChildren();
    elements.edges.replaceChildren();
    elements.nodes.replaceChildren();
    state.nodeElements.clear();
    state.edgeElements.clear();
    closeInspector();
    elements.performance?.replaceChildren();
    if (elements.performance) elements.performance.hidden = true;
    elements.canvas.hidden = false;
    updatePresentationChrome();
    elements.stage.classList.remove("is-performance");

    const document = state.document || {};
    const { nodes, edges } = visibleGraph(document);
    const series = (document.series || []).filter((item) => (item.points || []).length > 0);
    const hasGeometry = renderGeometry(document, Math.max(elements.stage.clientWidth, 640));
    if (!nodes.length && !series.length && !hasGeometry) {
      const diagnostics = (document.diagnostics || []).map((item) => item.message).filter(Boolean);
      renderEmpty(diagnostics[0] || "This source contains no renderable graph or metric series.");
    } else {
      elements.empty.hidden = true;
    }

    const stageWidth = Math.max(elements.stage.clientWidth, 640);
    const seriesHeight = series.length ? Math.min(220, Math.max(150, elements.stage.clientHeight * 0.32)) : 0;
    if (!hasGeometry) {
      renderSeries(series, stageWidth, seriesHeight);
      layoutNodes(nodes, edges, stageWidth, seriesHeight ? seriesHeight + 34 : 34);
      renderEdges(edges);
      renderNodes(nodes);
      updateEdges();
    }
    configurePlayback();
    updateStatusSummary();
    updateTransform();
    for (const extension of state.renderExtensions) {
      try { extension({ document, elements, state }); } catch (error) { console.error("Visualization extension failed", error); }
    }
    if (elements.performance && !elements.performance.hidden) {
      elements.canvas.hidden = true;
      updatePresentationChrome();
      elements.stage.classList.add("is-performance");
    }
  }

  function renderGeometry(document, width) {
    const geometry = document?.metadata?.geometry;
    const points = Array.isArray(geometry?.points) ? geometry.points : [];
    if (!points.length) return false;
    if (elements.canvas3d && window.AtlasDomain3D) {
      elements.canvas.hidden = true;
      elements.canvas3d.hidden = false;
      state.viewer3d = window.AtlasDomain3D.mount(elements.canvas3d, geometry, {
        domainId: document?.metadata?.domain_id,
        visualizationId: document?.metadata?.visualization_id,
      });
      return true;
    }
    const faces = Array.isArray(geometry?.faces) ? geometry.faces : [];
    const projected = points.slice(0, 10000).map((point) => {
      const x = Number(point?.[0]) || 0;
      const y = Number(point?.[1]) || 0;
      const z = Number(point?.[2]) || 0;
      return { x: x - z * 0.42, y: y + z * 0.24 };
    });
    const minX = Math.min(...projected.map((point) => point.x));
    const maxX = Math.max(...projected.map((point) => point.x));
    const minY = Math.min(...projected.map((point) => point.y));
    const maxY = Math.max(...projected.map((point) => point.y));
    const height = Math.max(260, elements.stage.clientHeight - 56);
    const scale = Math.max(0.001, Math.min((width - 100) / Math.max(1e-9, maxX - minX), (height - 80) / Math.max(1e-9, maxY - minY)));
    const screen = projected.map((point) => ({
      x: 50 + (point.x - minX) * scale,
      y: 40 + (maxY - point.y) * scale,
    }));
    const group = svgElement("g", { class: "visualization-domain-geometry" });
    const visibleFaces = faces.slice(0, 4000);
    for (const face of visibleFaces) {
      if (!Array.isArray(face) || face.length < 2) continue;
      const coordinates = face.map((index) => screen[Number(index)]).filter(Boolean);
      if (coordinates.length < 2) continue;
      group.appendChild(svgElement("path", {
        class: "visualization-domain-geometry-face",
        d: `${coordinates.map((point, index) => `${index ? "L" : "M"}${point.x.toFixed(2)},${point.y.toFixed(2)}`).join(" ")} Z`,
      }));
    }
    if (!visibleFaces.length) {
      for (const point of screen.slice(0, 5000)) {
        group.appendChild(svgElement("circle", {
          class: "visualization-domain-geometry-point",
          cx: point.x,
          cy: point.y,
          r: 1.35,
        }));
      }
    }
    elements.series.appendChild(group);
    return true;
  }

  function hasRenderableNodeData(node) {
    if (!node || !String(node.id || "").trim()) return false;
    if (node.metadata?.presentation === "task-manager-performance") {
      return Object.values(node.metrics || {}).some((value) => Number.isFinite(Number(value)));
    }
    return Boolean(
      String(node.label || "").trim()
      || String(node.category || "").trim()
      || Object.keys(node.metrics || {}).length
      || Object.keys(node.metadata || {}).length,
    );
  }

  function renderSeries(seriesList, width, height) {
    if (!seriesList.length) return;
    const left = 58;
    const right = Math.max(left + 120, width - 34);
    const top = 26;
    const bottom = Math.max(top + 80, height - 24);
    const allPoints = seriesList.flatMap((series) => series.points || []);
    const minTime = Math.min(...allPoints.map((point) => Number(point.timestamp_ms) || 0));
    const maxTime = Math.max(...allPoints.map((point) => Number(point.timestamp_ms) || 0));
    let minValue = Math.min(...allPoints.map((point) => Number(point.value) || 0));
    let maxValue = Math.max(...allPoints.map((point) => Number(point.value) || 0));
    if (minValue === maxValue) { minValue -= 1; maxValue += 1; }
    const x = (timestamp) => left + ((Number(timestamp) - minTime) / Math.max(1, maxTime - minTime)) * (right - left);
    const y = (value) => bottom - ((Number(value) - minValue) / (maxValue - minValue)) * (bottom - top);

    for (let step = 0; step <= 4; step += 1) {
      const yy = top + ((bottom - top) * step) / 4;
      elements.series.appendChild(svgElement("line", { class: "visualization-series-grid", x1: left, x2: right, y1: yy, y2: yy }));
      const label = svgElement("text", { class: "visualization-series-tick", x: left - 7, y: yy + 3, "text-anchor": "end" });
      label.textContent = formatNumber(maxValue - ((maxValue - minValue) * step) / 4);
      elements.series.appendChild(label);
    }
    elements.series.appendChild(svgElement("line", { class: "visualization-series-axis", x1: left, x2: left, y1: top, y2: bottom }));
    elements.series.appendChild(svgElement("line", { class: "visualization-series-axis", x1: left, x2: right, y1: bottom, y2: bottom }));

    seriesList.forEach((series, index) => {
      const points = series.points || [];
      const path = svgElement("path", {
        class: "visualization-series-path",
        d: points.map((point, pointIndex) => `${pointIndex ? "L" : "M"}${x(point.timestamp_ms).toFixed(2)},${y(point.value).toFixed(2)}`).join(" "),
        "data-series-id": series.id,
      });
      path.style.stroke = `var(--${index % 2 ? "green" : "accent"})`;
      elements.series.appendChild(path);
      const label = svgElement("text", { class: "visualization-series-label", x: left + index * 150, y: 14 });
      const latest = points[points.length - 1];
      label.textContent = `${series.label}: ${latest ? formatNumber(latest.value) : "—"}${series.unit || ""}`;
      elements.series.appendChild(label);
    });
  }

  function layoutNodes(nodes, edges, width, offsetY) {
    const validIds = new Set(nodes.map((node) => node.id));
    const incoming = new Map(nodes.map((node) => [node.id, 0]));
    const outgoing = new Map(nodes.map((node) => [node.id, []]));
    for (const edge of edges) {
      if (!validIds.has(edge.source) || !validIds.has(edge.target)) continue;
      incoming.set(edge.target, (incoming.get(edge.target) || 0) + 1);
      outgoing.get(edge.source).push(edge.target);
    }
    const levels = new Map();
    const queue = nodes.filter((node) => !incoming.get(node.id)).map((node) => node.id);
    queue.forEach((id) => levels.set(id, 0));
    for (let cursor = 0; cursor < queue.length; cursor += 1) {
      const id = queue[cursor];
      for (const target of outgoing.get(id) || []) {
        const nextLevel = Math.max(levels.get(target) || 0, (levels.get(id) || 0) + 1);
        levels.set(target, nextLevel);
        incoming.set(target, Math.max(0, (incoming.get(target) || 0) - 1));
        if (incoming.get(target) === 0) queue.push(target);
      }
    }
    const unresolved = nodes.filter((node) => !levels.has(node.id));
    unresolved.forEach((node, index) => levels.set(node.id, index % Math.max(1, Math.ceil(Math.sqrt(unresolved.length)))));
    const columns = new Map();
    for (const node of nodes) {
      const level = levels.get(node.id) || 0;
      if (!columns.has(level)) columns.set(level, []);
      columns.get(level).push(node);
    }
    const sortedLevels = Array.from(columns.keys()).sort((a, b) => a - b);
    const usableWidth = Math.max(360, width - 120);
    const xStep = sortedLevels.length > 1 ? Math.max(170, usableWidth / (sortedLevels.length - 1)) : 0;
    sortedLevels.forEach((level, columnIndex) => {
      const column = columns.get(level);
      const x = sortedLevels.length === 1 ? width / 2 : 70 + columnIndex * xStep;
      column.forEach((node, rowIndex) => {
        const existing = state.positions.get(node.id);
        if (existing?.manual) return;
        const y = offsetY + 44 + rowIndex * 92;
        state.positions.set(node.id, { x, y, manual: false });
      });
    });
  }

  function renderEdges(edges) {
    for (const edge of edges) {
      const group = svgElement("g", { "data-edge-id": edge.id });
      const path = svgElement("path", { class: "visualization-edge" });
      const label = svgElement("text", { class: "visualization-edge-label", "text-anchor": "middle" });
      label.textContent = edge.label || "";
      group.append(path, label);
      elements.edges.appendChild(group);
      state.edgeElements.set(edge.id, { group, path, label, edge });
    }
  }

  function renderNodes(nodes) {
    for (const node of nodes) {
      const position = state.positions.get(node.id) || { x: 0, y: 0 };
      const width = Math.max(106, Math.min(220, 52 + String(node.label || node.id).length * 6.2));
      const height = 54;
      const group = svgElement("g", {
        class: "visualization-node",
        transform: `translate(${position.x},${position.y})`,
        "data-node-id": node.id,
        "data-status": String(node.status || "").toLowerCase(),
        "data-presentation": String(node.metadata?.presentation || ""),
        role: "button",
        tabindex: "0",
        "aria-label": `${node.label || node.id}, ${node.category || "node"}`,
      });
      const shape = svgElement("rect", { class: "visualization-node-shape", x: -width / 2, y: -height / 2, width, height, rx: 7 });
      const status = svgElement("circle", { class: "visualization-node-status", cx: -width / 2 + 12, cy: -height / 2 + 12, r: 3.5 });
      const label = svgElement("text", { class: "visualization-node-label", x: -width / 2 + 20, y: -2 });
      label.textContent = truncate(node.label || node.id, 28);
      const meta = svgElement("text", { class: "visualization-node-meta", x: -width / 2 + 20, y: 15 });
      meta.textContent = truncate(node.category || node.status || "node", 34);
      group.append(shape, status, label, meta);
      if (node.metadata?.presentation === "neural-layer") {
        const graphEdges = visibleGraph().edges;
        if (graphEdges.some((edge) => edge.target === node.id)) {
          group.appendChild(svgElement("circle", { class: "visualization-neural-port visualization-neural-port-input", cx: -width / 2, cy: 0, r: 4 }));
        }
        if (graphEdges.some((edge) => edge.source === node.id)) {
          group.appendChild(svgElement("circle", { class: "visualization-neural-port visualization-neural-port-output", cx: width / 2, cy: 0, r: 4 }));
        }
      }
      group.addEventListener("pointerdown", (event) => startNodeDrag(event, node));
      group.addEventListener("click", (event) => { event.stopPropagation(); showInspector(node, group); });
      group.addEventListener("keydown", (event) => {
        if (event.key === "Enter" || event.key === " ") { event.preventDefault(); showInspector(node, group); }
      });
      elements.nodes.appendChild(group);
      state.nodeElements.set(node.id, { group, node, width, height });
    }
  }

  function updateEdges() {
    for (const { path, label, edge } of state.edgeElements.values()) {
      const source = state.positions.get(edge.source);
      const target = state.positions.get(edge.target);
      if (!source || !target) {
        path.setAttribute("d", "");
        label.textContent = "";
        continue;
      }
      const dx = target.x - source.x;
      const dy = target.y - source.y;
      const bend = Math.min(70, Math.abs(dx) * 0.25 + Math.abs(dy) * 0.05);
      const direction = dx >= 0 ? 1 : -1;
      path.setAttribute("d", `M${source.x},${source.y} C${source.x + bend * direction},${source.y} ${target.x - bend * direction},${target.y} ${target.x},${target.y}`);
      label.setAttribute("x", (source.x + target.x) / 2);
      label.setAttribute("y", (source.y + target.y) / 2 - 5);
    }
  }

  function startNodeDrag(event, node) {
    if (event.button !== 0) return;
    event.stopPropagation();
    const position = state.positions.get(node.id);
    state.drag = { id: node.id, pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, x: position.x, y: position.y };
    event.currentTarget.setPointerCapture(event.pointerId);
  }

  function showInspector(node, group) {
    state.nodeElements.forEach(({ group: candidate }) => candidate.classList.toggle("is-selected", candidate === group));
    const values = {
      Category: node.category,
      Status: node.status,
      Parent: node.parent_id,
      ...node.metrics,
      ...node.metadata,
    };
    const head = document.createElement("div");
    head.className = "visualization-inspector-head";
    const title = document.createElement("strong");
    title.textContent = node.label || node.id;
    const close = document.createElement("button");
    close.type = "button";
    close.setAttribute("aria-label", "Close details");
    close.textContent = "×";
    close.addEventListener("click", closeInspector);
    head.append(title, close);
    const list = document.createElement("dl");
    list.className = "visualization-inspector-grid";
    Object.entries(values).filter(([, value]) => value !== "" && value !== null && value !== undefined).slice(0, 40).forEach(([key, value]) => {
      const term = document.createElement("dt");
      term.textContent = key;
      const detail = document.createElement("dd");
      detail.textContent = formatMetadata(value);
      list.append(term, detail);
    });
    elements.inspector.replaceChildren(head, list);
    elements.inspector.hidden = false;
  }

  function closeInspector() {
    elements.inspector.hidden = true;
    state.nodeElements.forEach(({ group }) => group.classList.remove("is-selected"));
  }

  function configurePlayback() {
    const frames = visibleFrames();
    if (state.document?.source?.live && state.followLive && frames.length) state.frame = frames.length - 1;
    state.frame = Math.max(0, Math.min(state.frame, Math.max(0, frames.length - 1)));
    elements.timeline.min = "0";
    elements.timeline.max = String(Math.max(0, frames.length - 1));
    elements.timeline.value = String(state.frame);
    elements.timeline.disabled = frames.length < 2;
    elements.previous.disabled = frames.length < 2;
    elements.next.disabled = frames.length < 2;
    elements.play.disabled = frames.length < 2;
    applyFrame(state.frame);
  }

  function applyFrame(index) {
    const frames = visibleFrames();
    if (!frames.length) {
      elements.frameLabel.textContent = "No timeline";
      state.nodeElements.forEach(({ group }) => group.classList.remove("is-active", "is-dimmed"));
      state.edgeElements.forEach(({ path }) => path.classList.remove("is-active", "is-dimmed"));
      return;
    }
    state.frame = Math.max(0, Math.min(Number(index) || 0, frames.length - 1));
    elements.timeline.value = String(state.frame);
    const frame = frames[state.frame];
    const activeNodes = new Set(frame.active_nodes || []);
    const activeEdges = new Set(frame.active_edges || []);
    state.nodeElements.forEach(({ group }, id) => {
      group.classList.toggle("is-active", activeNodes.has(id));
      group.classList.toggle("is-dimmed", activeNodes.size > 0 && !activeNodes.has(id));
    });
    state.edgeElements.forEach(({ path }, id) => {
      path.classList.toggle("is-active", activeEdges.has(id));
      path.classList.toggle("is-dimmed", activeEdges.size > 0 && !activeEdges.has(id));
    });
    const metricLabel = Object.entries(frame.metrics || {}).slice(0, 3)
      .map(([key, value]) => `${key} ${formatNumber(value)}`)
      .join(" · ");
    elements.frameLabel.textContent = `${state.frame + 1} / ${frames.length} · ${frame.label || frame.id}${metricLabel ? ` · ${metricLabel}` : ""}`;
  }

  function togglePlayback() {
    if (state.playTimer) { stopPlayback(); return; }
    const frames = visibleFrames();
    if (frames.length < 2) return;
    elements.play.textContent = "Pause";
    elements.play.setAttribute("aria-label", "Pause animation");
    state.playTimer = window.setInterval(() => {
      const next = state.frame + 1;
      if (next >= frames.length) { stopPlayback(); return; }
      applyFrame(next);
    }, window.matchMedia("(prefers-reduced-motion: reduce)").matches ? 1500 : 850);
  }

  function stopPlayback() {
    if (state.playTimer) window.clearInterval(state.playTimer);
    state.playTimer = null;
    elements.play.textContent = "Play";
    elements.play.setAttribute("aria-label", "Play animation");
  }

  function schedulePolling() {
    stopPolling();
    if (!state.active || !currentSource()?.live) return;
    state.pollTimer = window.setTimeout(async () => {
      await loadSnapshot({ quiet: true });
    }, state.kind === "system" ? 750 : 2500);
  }

  function stopPolling() {
    if (state.pollTimer) window.clearTimeout(state.pollTimer);
    state.pollTimer = null;
  }

  function fitView() {
    if (state.viewer3d && elements.canvas3d && !elements.canvas3d.hidden) {
      state.viewer3d.fit();
      return;
    }
    if (!elements.viewport.children.length) return;
    let box;
    try { box = elements.viewport.getBBox(); } catch (_error) { return; }
    if (!box.width || !box.height) return;
    const width = elements.stage.clientWidth;
    const height = elements.stage.clientHeight;
    const scale = Math.max(0.18, Math.min(2.2, Math.min((width - 70) / box.width, (height - 70) / box.height)));
    state.transform.scale = scale;
    state.transform.x = width / 2 - (box.x + box.width / 2) * scale;
    state.transform.y = height / 2 - (box.y + box.height / 2) * scale;
    updateTransform();
  }

  function updateTransform() {
    const { x, y, scale } = state.transform;
    elements.viewport.setAttribute("transform", `translate(${x},${y}) scale(${scale})`);
  }

  function stagePointerMove(event) {
    if (state.drag && state.drag.pointerId === event.pointerId) {
      const position = state.positions.get(state.drag.id);
      position.x = state.drag.x + (event.clientX - state.drag.startX) / state.transform.scale;
      position.y = state.drag.y + (event.clientY - state.drag.startY) / state.transform.scale;
      position.manual = true;
      const rendered = state.nodeElements.get(state.drag.id);
      rendered?.group.setAttribute("transform", `translate(${position.x},${position.y})`);
      updateEdges();
      return;
    }
    if (state.panning && state.panning.pointerId === event.pointerId) {
      state.transform.x = state.panning.x + event.clientX - state.panning.startX;
      state.transform.y = state.panning.y + event.clientY - state.panning.startY;
      updateTransform();
    }
  }

  function stagePointerUp(event) {
    if (state.drag?.pointerId === event.pointerId) state.drag = null;
    if (state.panning?.pointerId === event.pointerId) {
      state.panning = null;
      elements.stage.classList.remove("is-panning");
    }
  }

  function formatNumber(value) {
    const number = Number(value);
    if (!Number.isFinite(number)) return String(value ?? "");
    if (Math.abs(number) >= 1e9) return `${(number / 1e9).toFixed(1)}G`;
    if (Math.abs(number) >= 1e6) return `${(number / 1e6).toFixed(1)}M`;
    if (Math.abs(number) >= 1e3) return `${(number / 1e3).toFixed(1)}k`;
    return Number.isInteger(number) ? String(number) : number.toFixed(2);
  }

  function formatMetadata(value) {
    if (typeof value === "number") return formatNumber(value);
    if (typeof value === "string") return value;
    try { return JSON.stringify(value); } catch (_error) { return String(value); }
  }

  function truncate(value, length) {
    const text = String(value || "");
    return text.length > length ? `${text.slice(0, length - 1)}…` : text;
  }

  function setStatus(message, error = false) {
    elements.status.textContent = message || "";
    elements.status.classList.toggle("is-error", error);
  }

  function updateStatusSummary() {
    const document = state.document || {};
    const diagnostic = (document.diagnostics || [])[0]?.message;
    const visibleNodes = (document.nodes || []).filter(hasRenderableNodeData);
    const visibleIds = new Set(visibleNodes.map((node) => node.id));
    const edgeCount = (document.edges || []).filter((edge) => visibleIds.has(edge.source) && visibleIds.has(edge.target)).length;
    const seriesCount = (document.series || []).filter((series) => (series.points || []).length > 0).length;
    const summary = `${visibleNodes.length} nodes · ${edgeCount} edges · ${seriesCount} series · ${(document.frames || []).length} frames`;
    setStatus(diagnostic ? `${summary} · ${diagnostic}` : summary);
  }

  function renderEmpty(message) {
    elements.empty.textContent = message;
    elements.empty.hidden = false;
  }

  async function activate() {
    if (state.active && state.document) {
      schedulePolling();
      return;
    }
    state.active = true;
    try {
      await ensureCatalog();
      await loadSnapshot({ fit: true });
    } catch (error) {
      setStatus(error?.message || "Unable to initialize visualization.", true);
      renderEmpty(error?.message || "Unable to initialize visualization.");
    }
  }

  async function open(kind, sourceId) {
    state.active = true;
    state.externalLoader = null;
    state.externalKind = "";
    await ensureCatalog();
    let type = typeForKind(kind);
    let source = (state.catalog?.sources || []).find((item) => item.kind === kind && item.id === sourceId);
    if (!type || !source) {
      await loadCatalog({ preserveSelection: true });
      type = typeForKind(kind);
      source = (state.catalog?.sources || []).find((item) => item.kind === kind && item.id === sourceId);
    }
    if (!type || !source) throw new Error("The requested visualization source is no longer available.");
    cancelSnapshotRequest();
    stopPlayback();
    stopPolling();
    state.kind = kind;
    state.sourceId = sourceId;
    state.positions.clear();
    state.followLive = true;
    renderTypeTabs();
    renderSourceOptions(sourceId);
    updatePresentationChrome();
    const cached = state.documentCache.get(snapshotCacheKey());
    state.document = cached || null;
    renderLoadingState();
    await waitForPresentationLayout(kind);
    if (state.kind !== kind || state.sourceId !== sourceId) return;
    if (cached) renderDocument();
    await loadSnapshot({ fit: true });
  }

  async function openDocument(documentData, presentation = {}) {
    if (!documentData || typeof documentData !== "object") throw new TypeError("Visualization document is required");
    state.active = true;
    await ensureCatalog();
    cancelSnapshotRequest();
    stopPlayback();
    stopPolling();
    const domainId = String(presentation.domainId || documentData?.metadata?.domain_id || "research").trim();
    const visualizationId = String(presentation.visualizationId || documentData?.metadata?.visualization_id || "visualization").trim();
    const kind = `research-domain:${domainId}:${visualizationId}`;
    const sourceId = String(documentData?.source?.id || presentation.assetId || kind);
    state.catalog.types = (state.catalog.types || []).filter((type) => !type?.metadata?.external_domain);
    state.catalog.sources = (state.catalog.sources || []).filter((source) => !source?.metadata?.external_domain);
    state.catalog.types.push({
      kind,
      label: String(presentation.label || documentData?.metadata?.domain_label || "Research Domain"),
      description: String(documentData.title || "Domain visualization"),
      adapter_id: "atlas.research-domain.external",
      plugin_api_version: "1",
      metadata: { external_domain: true },
    });
    state.catalog.sources.push({
      id: sourceId,
      kind,
      label: String(documentData?.source?.label || documentData.title || sourceId),
      source_type: String(documentData?.source?.source_type || "domain-asset"),
      live: false,
      metadata: { external_domain: true, domain_id: domainId },
    });
    state.kind = kind;
    state.sourceId = sourceId;
    state.externalKind = kind;
    state.externalLoader = typeof presentation.reload === "function" ? presentation.reload : null;
    state.positions.clear();
    state.frame = 0;
    state.followLive = false;
    state.document = documentData;
    state.documentCache.set(snapshotCacheKey(), documentData);
    renderTypeTabs();
    renderSourceOptions(sourceId);
    updatePresentationChrome();
    renderDocument();
    requestAnimationFrame(fitView);
  }

  function deactivate() {
    state.active = false;
    state.viewer3d?.dispose?.();
    state.viewer3d = null;
    cancelSnapshotRequest();
    stopPolling();
    stopPlayback();
  }

  elements.source.addEventListener("change", async () => {
    cancelSnapshotRequest();
    state.sourceId = elements.source.value;
    state.positions.clear();
    state.followLive = true;
    stopPolling();
    updateLiveState();
    const cached = state.documentCache.get(snapshotCacheKey());
    state.document = cached || null;
    if (cached) renderDocument(); else renderLoadingState();
    await loadSnapshot({ fit: true });
  });
  elements.refresh.addEventListener("click", async () => {
    try {
      if (state.externalLoader) {
        const documentData = await state.externalLoader();
        if (documentData) {
          state.document = documentData;
          state.documentCache.set(snapshotCacheKey(), documentData);
          state.positions.clear();
          renderDocument();
          requestAnimationFrame(fitView);
        }
      } else {
        await loadCatalog();
        await loadSnapshot({ fit: false });
      }
    } catch (error) { setStatus(error?.message || "Refresh failed.", true); }
  });
  elements.fit.addEventListener("click", fitView);
  elements.close.addEventListener("click", () => window.dispatchEvent(new CustomEvent("atlas:visualization-close")));
  elements.previous.addEventListener("click", () => { state.followLive = false; stopPlayback(); applyFrame(state.frame - 1); });
  elements.next.addEventListener("click", () => { stopPlayback(); applyFrame(state.frame + 1); state.followLive = state.frame >= visibleFrames().length - 1; });
  elements.play.addEventListener("click", () => { state.followLive = false; togglePlayback(); });
  elements.timeline.addEventListener("input", () => { state.followLive = false; stopPlayback(); applyFrame(elements.timeline.value); });
  elements.stage.addEventListener("pointerdown", (event) => {
    if (event.button !== 0 || elements.stage.classList.contains("is-performance") || event.target.closest(".domain-3d-canvas, .visualization-node, .visualization-inspector, [data-visualization-control]")) return;
    closeInspector();
    state.panning = { pointerId: event.pointerId, startX: event.clientX, startY: event.clientY, x: state.transform.x, y: state.transform.y };
    elements.stage.setPointerCapture(event.pointerId);
    elements.stage.classList.add("is-panning");
  });
  elements.stage.addEventListener("pointermove", stagePointerMove);
  elements.stage.addEventListener("pointerup", stagePointerUp);
  elements.stage.addEventListener("pointercancel", stagePointerUp);
  elements.stage.addEventListener("wheel", (event) => {
    if (elements.stage.classList.contains("is-performance") || event.target.closest(".domain-3d-canvas")) return;
    event.preventDefault();
    const bounds = elements.stage.getBoundingClientRect();
    const cursorX = event.clientX - bounds.left;
    const cursorY = event.clientY - bounds.top;
    const worldX = (cursorX - state.transform.x) / state.transform.scale;
    const worldY = (cursorY - state.transform.y) / state.transform.scale;
    const nextScale = Math.max(0.15, Math.min(4, state.transform.scale * Math.exp(-event.deltaY * 0.0012)));
    state.transform.x = cursorX - worldX * nextScale;
    state.transform.y = cursorY - worldY * nextScale;
    state.transform.scale = nextScale;
    updateTransform();
  }, { passive: false });

  state.resizeObserver = new ResizeObserver(() => {
    if (!state.active || !state.document) return;
    window.requestAnimationFrame(() => {
      if (state.kind === "system" && !elements.performance?.hidden) {
        window.dispatchEvent(new CustomEvent("atlas:visualization-performance-resize"));
        return;
      }
      renderDocument();
    });
  });
  state.resizeObserver.observe(elements.stage);

  window.AtlasVisualization = Object.freeze({
    activate,
    deactivate,
    open,
    openDocument,
    refresh: () => loadSnapshot({ fit: false }),
    fit: fitView,
    registerRenderExtension(extension) {
      if (typeof extension !== "function") throw new TypeError("Visualization render extension must be a function");
      state.renderExtensions.add(extension);
      return () => state.renderExtensions.delete(extension);
    },
    getDocument: () => state.document,
    schemaVersion: "atlas.visualization.v1",
  });
})();
