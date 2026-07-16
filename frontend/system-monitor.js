(function initializeSystemMonitorBar() {
  "use strict";

  const STORAGE_KEY = "atlas-system-monitor-enabled-v1";
  const POLL_INTERVAL_MS = 750;
  const bar = document.getElementById("system-metrics-bar");
  const toggle = document.getElementById("system-monitor-enabled");
  const appShell = document.querySelector(".app-shell");
  if (!bar || !toggle || !appShell) return;

  const state = {
    enabled: false,
    timer: null,
    controller: null,
    requestId: 0,
    lastSuccessAt: 0,
  };

  function finite(value) {
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
  }

  function formatPercent(value) {
    return `${Math.max(0, value).toFixed(value >= 10 ? 0 : 1)}%`;
  }

  function formatBytes(value) {
    const bytes = Math.max(0, Number(value) || 0);
    const units = ["B", "KB", "MB", "GB", "TB"];
    let scaled = bytes;
    let unit = 0;
    while (scaled >= 1024 && unit < units.length - 1) {
      scaled /= 1024;
      unit += 1;
    }
    return `${scaled.toFixed(unit >= 3 ? 1 : scaled >= 10 ? 0 : 1)} ${units[unit]}`;
  }

  function latestSeriesValue(documentData, nodeId) {
    return (documentData?.series || [])
      .filter((series) => series?.node_id === nodeId && (series?.points || []).length)
      .reduce((sum, series) => {
        const point = series.points[series.points.length - 1];
        return sum + (finite(point?.value) || 0);
      }, 0);
  }

  function metricForCategory(documentData, category) {
    const nodes = (documentData?.nodes || []).filter((node) => node?.category === category && finite(node?.metrics?.value) !== null);
    if (!nodes.length) return null;
    if (category === "cpu" || category === "gpu") {
      const value = Math.max(...nodes.map((node) => finite(node.metrics.value) || 0));
      return { label: category.toUpperCase(), value: formatPercent(value) };
    }
    if (category === "memory") {
      const node = nodes[0];
      const used = finite(node.metrics.value);
      const total = finite(node.metadata?.graph_max);
      if (used === null) return null;
      return { label: "MEM", value: total ? `${formatBytes(used)} / ${formatBytes(total)}` : formatBytes(used) };
    }
    if (category === "disk") {
      const value = Math.max(...nodes.map((node) => finite(node.metrics.value) || 0));
      return { label: "DISK", value: formatPercent(value) };
    }
    if (category === "network") {
      const throughput = nodes.reduce((sum, node) => sum + latestSeriesValue(documentData, node.id), 0);
      const fallback = nodes.reduce((sum, node) => sum + (finite(node.metrics.value) || 0), 0);
      return { label: "NET", value: `${formatBytes(throughput || fallback)}/s` };
    }
    return null;
  }

  function render(documentData) {
    const metrics = ["cpu", "gpu", "memory", "disk", "network"]
      .map((category) => metricForCategory(documentData, category))
      .filter(Boolean);
    bar.replaceChildren();
    metrics.forEach((metric) => {
      const item = document.createElement("span");
      item.className = "system-metrics-item";
      const label = document.createElement("span");
      label.className = "system-metrics-label";
      label.textContent = metric.label;
      const value = document.createElement("strong");
      value.className = "system-metrics-value";
      value.textContent = metric.value;
      item.append(label, value);
      bar.appendChild(item);
    });
    const visible = state.enabled && metrics.length > 0;
    bar.hidden = !visible;
    appShell.classList.toggle("has-system-metrics", visible);
  }

  async function poll() {
    if (!state.enabled) return;
    const requestId = ++state.requestId;
    state.controller?.abort();
    const controller = new AbortController();
    state.controller = controller;
    try {
      const params = new URLSearchParams({ kind: "system", source_id: "runtime:system" });
      const response = await fetch(`/api/visualizations/snapshot?${params}`, {
        signal: controller.signal,
        headers: { Accept: "application/json" },
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const payload = await response.json();
      if (requestId !== state.requestId) return;
      const documentData = payload?.data ?? payload;
      if (payload?.ok === false) throw new Error(payload?.error || "System monitoring unavailable");
      state.lastSuccessAt = Date.now();
      render(documentData);
    } catch (error) {
      if (error?.name !== "AbortError" && Date.now() - state.lastSuccessAt > 5000) render(null);
    } finally {
      if (requestId === state.requestId && state.enabled) {
        state.timer = window.setTimeout(poll, POLL_INTERVAL_MS);
      }
    }
  }

  function setEnabled(enabled) {
    state.enabled = Boolean(enabled);
    toggle.checked = state.enabled;
    window.clearTimeout(state.timer);
    state.timer = null;
    state.controller?.abort();
    state.controller = null;
    if (!state.enabled) {
      bar.hidden = true;
      bar.replaceChildren();
      appShell.classList.remove("has-system-metrics");
      return;
    }
    poll();
  }

  toggle.addEventListener("change", () => {
    try { localStorage.setItem(STORAGE_KEY, toggle.checked ? "true" : "false"); } catch (_error) {}
    setEnabled(toggle.checked);
  });

  let initiallyEnabled = false;
  try { initiallyEnabled = localStorage.getItem(STORAGE_KEY) === "true"; } catch (_error) {}
  setEnabled(initiallyEnabled);

  window.AtlasSystemMonitor = Object.freeze({ setEnabled });
})();
