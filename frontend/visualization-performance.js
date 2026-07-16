(function initializeAtlasPerformanceVisualization() {
  "use strict";

  const root = document.getElementById("visualization-performance");
  const renderer = window.AtlasVisualization;
  if (!root || !renderer) return;

  const WINDOW_MS = 60_000;
  const colors = ["var(--performance-blue)", "var(--performance-purple)", "var(--performance-green)", "var(--performance-magenta)", "var(--performance-cyan)"];
  let selectedNodeId = "";
  let lastDocument = null;
  let resizeFrame = 0;

  function render({ document, elements }) {
    if (document?.kind !== "system") {
      lastDocument = null;
      root.replaceChildren();
      root.hidden = true;
      return;
    }
    const seriesByNode = new Map();
    for (const series of document.series || []) {
      if (!series?.node_id || !(series.points || []).some(validPoint)) continue;
      if (!seriesByNode.has(series.node_id)) seriesByNode.set(series.node_id, []);
      seriesByNode.get(series.node_id).push(series);
    }
    const resources = (document.nodes || []).filter((node) =>
      node?.metadata?.presentation === "task-manager-performance"
      && seriesByNode.has(node.id)
      && Object.values(node.metrics || {}).some(finite),
    );
    if (!resources.length) return;

    lastDocument = document;
    if (!resources.some((node) => node.id === selectedNodeId)) selectedNodeId = resources[0].id;
    const selected = resources.find((node) => node.id === selectedNodeId) || resources[0];

    const sidebar = el("nav", "visualization-performance-sidebar", { "aria-label": "Performance resources" });
    for (const [index, node] of resources.entries()) {
      sidebar.appendChild(resourceButton(node, seriesByNode.get(node.id), index, node.id === selected.id));
    }

    const detail = el("div", "visualization-performance-detail");
    detail.appendChild(resourceHeader(selected));
    detail.appendChild(graphArea(selected, seriesByNode.get(selected.id), resources.indexOf(selected)));
    const facts = detailsArea(selected);
    if (facts) detail.appendChild(facts);

    root.replaceChildren(sidebar, detail);
    root.hidden = false;
    elements.empty.hidden = true;
  }

  function resourceButton(node, seriesList, index, selected) {
    const button = el("button", `visualization-performance-resource${selected ? " is-selected" : ""}`, {
      type: "button",
      "aria-pressed": selected ? "true" : "false",
      "aria-label": `${node.label}: ${resourceSummary(node, seriesList)}`,
    });
    button.style.setProperty("--performance-series", colors[index % colors.length]);
    const chart = el("div", "visualization-performance-mini");
    chart.appendChild(chartSvg(seriesList, node, 160, 52, true));
    const copy = el("span", "visualization-performance-resource-copy");
    copy.append(
      textEl("strong", "visualization-performance-resource-label", node.label || node.id),
      textEl("span", "visualization-performance-resource-summary", resourceSummary(node, seriesList)),
      textEl("span", "visualization-performance-resource-subtitle", node.metadata?.subtitle || ""),
    );
    button.append(chart, copy);
    button.addEventListener("click", () => {
      selectedNodeId = node.id;
      rerender();
    });
    return button;
  }

  function resourceHeader(node) {
    const header = el("header", "visualization-performance-heading");
    header.append(
      textEl("h2", "visualization-performance-title", node.label || node.id),
      textEl("span", "visualization-performance-device", node.metadata?.subtitle || ""),
    );
    return header;
  }

  function graphArea(node, seriesList, index) {
    const area = el("section", "visualization-performance-graphs");
    area.style.setProperty("--performance-series", colors[index % colors.length]);
    const graphMax = positive(node.metadata?.graph_max) || inferredMax(seriesList);
    const graphLabel = node.metadata?.graph_label || seriesList[0]?.label || "Usage";
    const topRow = el("div", "visualization-performance-graph-labels");
    topRow.append(
      textEl("span", "", graphLabel),
      textEl("span", "", formatValue(graphMax, primaryUnit(node, seriesList))),
    );
    area.appendChild(topRow);

    const useSmallMultiples = seriesList.length > 2;
    const graphs = el("div", useSmallMultiples ? "visualization-performance-graph-grid" : "visualization-performance-graph-single");
    if (useSmallMultiples) {
      for (const series of seriesList) {
        const panel = el("div", "visualization-performance-graph-panel");
        panel.append(
          textEl("span", "visualization-performance-graph-name", series.label || series.id),
          chartSvg([series], node, 600, 180, false),
        );
        graphs.appendChild(panel);
      }
    } else {
      graphs.appendChild(chartSvg(seriesList, node, 900, 360, false));
    }
    area.appendChild(graphs);
    const times = el("div", "visualization-performance-time-labels");
    times.append(textEl("span", "", "60 seconds"), textEl("span", "", "0"));
    area.appendChild(times);
    return area;
  }

  function chartSvg(seriesList, node, width, height, mini) {
    const svg = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    svg.setAttribute("viewBox", `0 0 ${width} ${height}`);
    svg.setAttribute("preserveAspectRatio", "none");
    svg.setAttribute("role", "img");
    svg.setAttribute("aria-label", `${node.label || node.id} ${mini ? "preview" : "60 second performance history"}`);
    svg.classList.add("visualization-performance-chart");
    const allPoints = seriesList.flatMap((series) => series.points || []).filter(validPoint);
    if (!allPoints.length) return svg;
    const newest = Math.max(...allPoints.map((point) => Number(point.timestamp_ms)));
    const oldest = newest - WINDOW_MS;
    const max = positive(node.metadata?.graph_max) || inferredMax(seriesList);

    for (let step = 1; step < (mini ? 4 : 8); step += 1) {
      const x = width * step / (mini ? 4 : 8);
      svg.appendChild(svgNode("line", { class: "visualization-performance-grid-line", x1: x, x2: x, y1: 0, y2: height }));
    }
    for (let step = 1; step < (mini ? 3 : 8); step += 1) {
      const y = height * step / (mini ? 3 : 8);
      svg.appendChild(svgNode("line", { class: "visualization-performance-grid-line", x1: 0, x2: width, y1: y, y2: y }));
    }

    for (const [seriesIndex, series] of seriesList.entries()) {
      const points = (series.points || []).filter(validPoint).filter((point) => Number(point.timestamp_ms) >= oldest);
      if (!points.length) continue;
      const coords = points.map((point) => ({
        x: clamp((Number(point.timestamp_ms) - oldest) / WINDOW_MS, 0, 1) * width,
        y: height - clamp(Number(point.value) / max, 0, 1) * height,
      }));
      const path = svgNode("path", {
        class: "visualization-performance-chart-path",
        d: coords.map((point, pointIndex) => `${pointIndex ? "L" : "M"}${point.x.toFixed(2)},${point.y.toFixed(2)}`).join(" "),
      });
      path.style.setProperty("--performance-path-opacity", String(Math.max(0.45, 1 - seriesIndex * 0.18)));
      svg.appendChild(path);
      if (coords.length === 1) {
        svg.appendChild(svgNode("circle", { class: "visualization-performance-chart-point", cx: coords[0].x, cy: coords[0].y, r: mini ? 2 : 3 }));
      }
    }
    return svg;
  }

  function detailsArea(node) {
    const details = (node.metadata?.details || []).filter((detail) => detail && detail.value !== null && detail.value !== undefined && detail.value !== "");
    if (!details.length) return null;
    const section = el("section", "visualization-performance-facts", { "aria-label": `${node.label} details` });
    for (const detail of details) {
      const item = el("div", "visualization-performance-fact");
      item.append(
        textEl("span", "visualization-performance-fact-label", detail.label || ""),
        textEl("strong", "visualization-performance-fact-value", formatDetail(detail)),
      );
      section.appendChild(item);
    }
    return section;
  }

  function resourceSummary(node, seriesList) {
    const value = Number(node.metrics?.value);
    const unit = node.metadata?.unit || primaryUnit(node, seriesList);
    const details = node.metadata?.details || [];
    if (node.category === "cpu") {
      const speed = details.find((item) => item?.label === "Speed");
      return `${formatValue(value, unit)}${speed ? `  ${formatDetail(speed)}` : ""}`;
    }
    if (node.category === "memory") {
      const available = details.find((item) => item?.label === "Available");
      const total = available && Number.isFinite(Number(available.value)) ? value + Number(available.value) : null;
      return total ? `${formatValue(value, "B")} / ${formatValue(total, "B")} (${Math.round(value / total * 100)}%)` : formatValue(value, unit);
    }
    if (node.category === "network") {
      const send = latestForLabel(seriesList, "send");
      const receive = latestForLabel(seriesList, "receive");
      return [send !== null ? `S: ${formatValue(send, "B/s")}` : "", receive !== null ? `R: ${formatValue(receive, "B/s")}` : ""].filter(Boolean).join("  ");
    }
    return formatValue(value, unit);
  }

  function latestForLabel(seriesList, label) {
    const series = seriesList.find((item) => String(item.label || "").toLowerCase().includes(label));
    const point = series?.points?.[series.points.length - 1];
    return validPoint(point) ? Number(point.value) : null;
  }

  function formatDetail(detail) {
    const first = formatValue(detail.value, detail.unit || "");
    if (detail.secondary_value === null || detail.secondary_value === undefined) return first;
    return `${first} / ${formatValue(detail.secondary_value, detail.unit || "")}`;
  }

  function formatValue(value, unit) {
    const number = Number(value);
    if (!Number.isFinite(number)) return String(value ?? "");
    if (unit === "%") return `${Math.round(number)}%`;
    if (unit === "B") return formatBytes(number);
    if (unit === "B/s") return `${formatBytes(number)}/s`;
    if (unit === "b/s") return `${formatBits(number)}/s`;
    if (unit === "GHz") return `${number.toFixed(2)} GHz`;
    if (unit === "MHz") return `${Math.round(number).toLocaleString()} MHz`;
    if (unit === "ms") return `${number < 10 ? number.toFixed(1) : Math.round(number)} ms`;
    if (unit === "uptime") return formatUptime(number);
    return `${number.toLocaleString(undefined, { maximumFractionDigits: 1 })}${unit ? ` ${unit}` : ""}`;
  }

  function formatBytes(value) {
    const units = ["B", "KB", "MB", "GB", "TB"];
    let index = 0;
    let scaled = Math.max(0, Number(value));
    while (scaled >= 1024 && index < units.length - 1) { scaled /= 1024; index += 1; }
    return `${scaled >= 100 || index === 0 ? Math.round(scaled) : scaled.toFixed(1)} ${units[index]}`;
  }

  function formatBits(value) {
    const units = ["bps", "Kbps", "Mbps", "Gbps"];
    let index = 0;
    let scaled = Math.max(0, Number(value));
    while (scaled >= 1000 && index < units.length - 1) { scaled /= 1000; index += 1; }
    return `${scaled >= 100 || index === 0 ? Math.round(scaled) : scaled.toFixed(1)} ${units[index]}`;
  }

  function formatUptime(seconds) {
    const total = Math.max(0, Math.floor(seconds));
    const days = Math.floor(total / 86400);
    const hours = Math.floor(total % 86400 / 3600);
    const minutes = Math.floor(total % 3600 / 60);
    const secs = total % 60;
    return `${days}:${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}:${String(secs).padStart(2, "0")}`;
  }

  function primaryUnit(node, seriesList) {
    return node.metadata?.unit || seriesList.find((series) => series.unit)?.unit || "";
  }

  function inferredMax(seriesList) {
    const values = seriesList.flatMap((series) => series.points || []).filter(validPoint).map((point) => Number(point.value));
    const maximum = Math.max(1, ...values);
    return maximum <= 100 && seriesList.every((series) => series.unit === "%") ? 100 : maximum * 1.2;
  }

  function validPoint(point) {
    return point && finite(point.value) && finite(point.timestamp_ms);
  }

  function finite(value) { return Number.isFinite(Number(value)); }
  function positive(value) { const number = Number(value); return Number.isFinite(number) && number > 0 ? number : null; }
  function clamp(value, min, max) { return Math.max(min, Math.min(max, value)); }

  function el(tag, className = "", attributes = {}) {
    const element = document.createElement(tag);
    if (className) element.className = className;
    for (const [name, value] of Object.entries(attributes)) element.setAttribute(name, value);
    return element;
  }

  function textEl(tag, className, value) {
    const element = el(tag, className);
    element.textContent = value == null ? "" : String(value);
    if (!element.textContent) element.hidden = true;
    return element;
  }

  function svgNode(tag, attributes) {
    const node = document.createElementNS("http://www.w3.org/2000/svg", tag);
    for (const [name, value] of Object.entries(attributes)) node.setAttribute(name, String(value));
    return node;
  }

  function rerender() {
    const currentDocument = renderer.getDocument?.() || null;
    if (currentDocument?.kind !== "system") {
      lastDocument = null;
      root.replaceChildren();
      root.hidden = true;
      return;
    }
    lastDocument = currentDocument;
    const elements = {
      empty: document.getElementById("visualization-empty"),
    };
    render({ document: currentDocument, elements });
  }

  function scheduleRerender() {
    window.cancelAnimationFrame(resizeFrame);
    resizeFrame = window.requestAnimationFrame(() => {
      resizeFrame = window.requestAnimationFrame(rerender);
    });
  }

  renderer.registerRenderExtension(render);
  window.addEventListener("atlas:visualization-performance-resize", scheduleRerender);
  new ResizeObserver(() => {
    if (!root.hidden) scheduleRerender();
  }).observe(root);
})();
