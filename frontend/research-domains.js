(function initializeAtlasResearchDomains() {
  "use strict";

  const NS = "http://www.w3.org/2000/svg";
  const DOMAIN_ICON_MARKUP = Object.freeze({
    "ai-ml": '<circle cx="6" cy="7" r="2"></circle><circle cx="18" cy="6" r="2"></circle><circle cx="12" cy="17" r="2"></circle><path d="m7.8 7.5 2.9 7.7m5.5-8.1-3 8.2M8 6.8l8-.6"></path>',
    "computer-vision": '<path d="M3.5 12s3.2-5 8.5-5 8.5 5 8.5 5-3.2 5-8.5 5-8.5-5-8.5-5z"></path><circle cx="12" cy="12" r="2.5"></circle>',
    "nlp": '<path d="M5 6.5h14M5 11h10M5 15.5h7"></path><path d="m15 15 2 2 3-4"></path>',
    "computer-graphics": '<path d="m12 3.8 7 4v8.2l-7 4-7-4V7.8zM5 7.8l7 4 7-4M12 11.8V20"></path>',
    "cad": '<circle cx="12" cy="6" r="2"></circle><path d="m10.8 7.6-4.3 11m6.7-11 4.3 11M5 18.5h14M9.5 14.5h5"></path>',
    "robotics": '<rect x="5" y="7" width="14" height="11" rx="3"></rect><path d="M12 4v3M8 12h.01M16 12h.01M9 15.5h6"></path>',
    "computer-networks": '<circle cx="5" cy="12" r="2"></circle><circle cx="12" cy="5" r="2"></circle><circle cx="19" cy="9" r="2"></circle><circle cx="15" cy="18" r="2"></circle><path d="m6.5 10.6 4.1-4.2m3.2-.8 3.4 2.1m.7 3.2-2 5.2M13 17l-6.1-4"></path>',
    "operating-systems": '<rect x="6" y="6" width="12" height="12" rx="2"></rect><rect x="9.5" y="9.5" width="5" height="5" rx="1"></rect><path d="M9 3.5v2M15 3.5v2M9 18.5v2M15 18.5v2M3.5 9h2M18.5 9h2M3.5 15h2M18.5 15h2"></path>',
    "compiler": '<path d="m8.5 6-4 6 4 6M15.5 6l4 6-4 6M13.5 4l-3 16"></path>',
    "database": '<ellipse cx="12" cy="6" rx="7" ry="3"></ellipse><path d="M5 6v6c0 1.7 3.1 3 7 3s7-1.3 7-3V6M5 12v6c0 1.7 3.1 3 7 3s7-1.3 7-3v-6"></path>',
    "software-engineering": '<rect x="4" y="4" width="6" height="6" rx="1"></rect><rect x="14" y="4" width="6" height="6" rx="1"></rect><rect x="9" y="14" width="6" height="6" rx="1"></rect><path d="M10 7h4M7 10v2l5 2M17 10v2l-5 2"></path>',
    "program-analysis": '<circle cx="6" cy="5" r="2"></circle><circle cx="6" cy="19" r="2"></circle><circle cx="18" cy="8" r="2"></circle><circle cx="18" cy="16" r="2"></circle><path d="M6 7v10M8 6l8 2M8 18l8-2M18 10v4"></path>',
    "cyber-security": '<path d="M12 3.5 19 6v5c0 4.5-2.7 7.7-7 9.5C7.7 18.7 5 15.5 5 11V6z"></path><path d="m9 12 2 2 4-5"></path>',
    "hpc": '<path d="M4 6h5v5H4zM15 6h5v5h-5zM9.5 15h5v5h-5zM9 8.5h6M6.5 11v2.5L10 16M17.5 11v2.5L14 16"></path>',
    "distributed-systems": '<rect x="3.5" y="4" width="6" height="5" rx="1"></rect><rect x="14.5" y="4" width="6" height="5" rx="1"></rect><rect x="9" y="15" width="6" height="5" rx="1"></rect><path d="M9.5 6.5h5M6.5 9v3l3.5 3.5M17.5 9v3L14 15.5"></path>',
    "scientific-computing": '<path d="M4 19h16M5 19V5"></path><path d="M6 16c2-1 2.8-7 5-7s2.8 5 4.8 5 2.2-4 3.2-6"></path><circle cx="11" cy="9" r="1.3"></circle><circle cx="15.8" cy="14" r="1.3"></circle>',
  });
  const DOMAIN_TAB_LABELS = Object.freeze({
    overview: "Overview",
    resources: "Resources",
    visualization: "Visualization",
    artifacts: "Artifacts",
    history: "History",
    settings: "Settings",
    "agent-context": "Agent Context",
    preview: "Preview",
  });
  const elements = {
    app: document.querySelector(".app-shell"),
    sidebar: document.getElementById("research-domains-sidebar"),
    nav: document.getElementById("research-domains-nav"),
    workspace: document.getElementById("research-domain-workspace"),
    kicker: document.getElementById("research-domain-kicker"),
    title: document.getElementById("research-domain-title"),
    toolbarMeta: document.getElementById("research-domain-toolbar-meta"),
    refresh: document.getElementById("research-domain-refresh"),
    close: document.getElementById("research-domain-close"),
    assetCount: document.getElementById("research-domain-asset-count"),
    assetFilter: document.getElementById("research-domain-asset-filter"),
    assetFooter: document.getElementById("research-domain-asset-footer"),
    assets: document.getElementById("research-domain-assets"),
    resourcesLabel: document.getElementById("research-domain-resources-label"),
    objectTypes: document.getElementById("research-domain-object-types"),
    globalTabs: document.getElementById("research-domain-global-tabs"),
    primaryKicker: document.getElementById("research-domain-primary-kicker"),
    primaryLabel: document.getElementById("research-domain-primary-label"),
    openAsset: document.getElementById("research-domain-open-asset"),
    nativeActions: document.getElementById("research-domain-native-actions"),
    tabSurface: document.getElementById("research-domain-tab-surface"),
    nativeSurface: document.getElementById("research-domain-native-surface"),
    previewPanel: document.getElementById("research-domain-preview-panel"),
    viewerToolbar: document.getElementById("research-domain-viewer-toolbar"),
    viewerTabs: document.getElementById("research-domain-viewer-tabs"),
    openVisualization: document.getElementById("research-domain-open-visualization"),
    stage: document.getElementById("research-domain-preview-stage"),
    canvas3d: document.getElementById("research-domain-3d-canvas"),
    canvas: document.getElementById("research-domain-preview-canvas"),
    empty: document.getElementById("research-domain-empty"),
    inspector: document.getElementById("research-domain-inspector"),
    operationsLabel: document.getElementById("research-domain-operations-label"),
    operationsPanel: document.getElementById("research-domain-operations-panel"),
    runs: document.getElementById("research-domain-runs"),
    clearRuns: document.getElementById("research-domain-clear-runs"),
    agents: document.getElementById("research-domain-agents"),
    agentContextTitle: document.getElementById("research-domain-agent-context-title"),
    agentContextCopy: document.getElementById("research-domain-agent-context-copy"),
    adapters: document.getElementById("research-domain-adapters"),
    providerSection: document.getElementById("research-domain-provider-section"),
    liveContext: document.getElementById("research-domain-live-context"),
  };

  if (!elements.sidebar || !elements.nav || !elements.workspace || !elements.canvas) return;

  const state = {
    catalog: null,
    catalogPromise: null,
    active: false,
    domainId: "",
    workspace: null,
    assetId: "",
    visualizationId: "",
    activeTab: "overview",
    assetQuery: "",
    objectType: "all",
    selectedAgent: "",
    actions: [],
    actionsRequestId: 0,
    taskCatalog: null,
    tasks: [],
    tasksRequestId: 0,
    actionLog: "",
    runLedger: [],
    runningAction: null,
    document: null,
    pollTimer: null,
    requestId: 0,
    previewGeneration: 0,
    previewTimers: new WeakMap(),
    viewer3d: null,
    workbenchViewer3d: null,
    stateSyncTimer: null,
    pendingStatePatch: null,
    pendingStateDomain: "",
    highlightAssetId: "",
    locateAssetId: "",
  };

  function svg(tag, attributes = {}) {
    const node = document.createElementNS(NS, tag);
    Object.entries(attributes).forEach(([key, value]) => node.setAttribute(key, String(value)));
    return node;
  }

  async function requestJson(path, options = {}) {
    const response = await fetch(path, {
      ...options,
      headers: { Accept: "application/json", ...(options.headers || {}) },
    });
    if (!response.ok) throw new Error((await response.text()) || `HTTP ${response.status}`);
    const payload = await response.json();
    if (payload?.ok === false) throw new Error(payload?.error || "Research Domains request failed");
    return payload?.data ?? payload;
  }

  function pluginById(id) {
    return (state.catalog?.plugins || []).find((plugin) => plugin?.metadata?.id === id) || null;
  }

  function summaryById(id) {
    return (state.catalog?.workspaces || []).find((summary) => summary?.domain_id === id) || null;
  }

  function domainSpec(domainId = state.domainId) {
    return window.AtlasResearchDomainSpecs?.get?.(domainId) || {
      tabs: Object.keys(DOMAIN_TAB_LABELS),
      layout: "research-lab",
      studio: "RESEARCH LABORATORY",
      reference: "Atlas Research Domain",
      focus: "Evidence-backed research workflow",
      dataObjects: ["Asset", "Experiment", "Artifact"],
      toolbar: [],
      visualizations: [],
      workflowNouns: [],
      settings: [],
      zones: ["resources", "workflow", "artifacts", "activity"],
      interaction: "Select evidence, execute with an Agent, and verify generated artifacts.",
      preview: "research-result",
      agentContext: "Expose selected evidence, parameters, workflow and quality gates.",
      runtime: "Atlas research runtime",
      agentApi: "atlas.research.environment",
      selectionModel: "workspace -> object -> evidence",
      previewTarget: "research result",
      navigation: Object.entries(DOMAIN_TAB_LABELS).map(([tab, label], index) => ({
        tab, label, placement: index < 5 ? "primary" : "secondary",
      })),
    };
  }

  function domainTabDescriptor(tab, spec = domainSpec()) {
    return (spec.navigation || []).find((item) => item?.tab === tab)
      || { tab, label: DOMAIN_TAB_LABELS[tab] || formatRole(tab), placement: "primary" };
  }

  function workspaceState() {
    return state.workspace?.state || {};
  }

  async function persistWorkspaceState(patch, { quiet = true, domainId = state.domainId } = {}) {
    if (!domainId || !patch || typeof patch !== "object") return null;
    try {
      const next = await requestJson("/api/research-domains/state", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ domain_id: domainId, patch, updated_by: "ui" }),
      });
      if (state.domainId === domainId && state.workspace) state.workspace.state = next;
      renderLiveContext();
      return next;
    } catch (error) {
      if (!quiet) throw error;
      console.debug("Research domain workspace state sync failed", error);
      return null;
    }
  }

  function queueWorkspaceState(patch) {
    window.clearTimeout(state.stateSyncTimer);
    const previous = state.pendingStatePatch || {};
    state.pendingStatePatch = {
      ...previous,
      ...patch,
      ...(previous.ui || patch.ui ? { ui: { ...(previous.ui || {}), ...(patch.ui || {}) } } : {}),
      ...(previous.filters || patch.filters ? { filters: { ...(previous.filters || {}), ...(patch.filters || {}) } } : {}),
      ...(previous.parameters || patch.parameters ? { parameters: { ...(previous.parameters || {}), ...(patch.parameters || {}) } } : {}),
    };
    state.pendingStateDomain = state.domainId;
    window.dispatchEvent(new CustomEvent("atlas:research-environment-change", {
      detail: {
        domainId: state.domainId,
        assetId: state.assetId,
        revision: state.workspace?.revision || "",
        patch: state.pendingStatePatch,
      },
    }));
    state.stateSyncTimer = window.setTimeout(() => {
      const pending = state.pendingStatePatch;
      const domainId = state.pendingStateDomain;
      state.pendingStatePatch = null;
      state.pendingStateDomain = "";
      persistWorkspaceState(pending, { domainId });
    }, 180);
  }

  function domainIcon(domainId) {
    const icon = svg("svg", { viewBox: "0 0 24 24", "aria-hidden": "true" });
    icon.innerHTML = DOMAIN_ICON_MARKUP[domainId]
      || '<circle cx="12" cy="12" r="8"></circle><path d="M8 12h8M12 8v8"></path>';
    return icon;
  }

  async function ensureCatalog({ force = false } = {}) {
    if (state.catalog && !force) return state.catalog;
    if (!state.catalogPromise) {
      elements.nav.setAttribute("aria-busy", "true");
      state.catalogPromise = requestJson("/api/research-domains?compact=true")
        .then((catalog) => {
          state.catalog = catalog;
          if (!state.domainId) {
            state.domainId = catalog?.active_domain?.domain_id || catalog?.plugins?.[0]?.metadata?.id || "";
          }
          renderNav();
          return catalog;
        })
        .finally(() => {
          state.catalogPromise = null;
          elements.nav.removeAttribute("aria-busy");
        });
    }
    return state.catalogPromise;
  }

  function renderNav() {
    elements.nav.replaceChildren();
    const seenIds = new Set();
    for (const plugin of state.catalog?.plugins || []) {
      const id = plugin?.metadata?.id || "";
      if (!id || seenIds.has(id)) continue;
      seenIds.add(id);
      const summary = summaryById(id);
      const button = document.createElement("button");
      button.type = "button";
      button.className = `activity-rail-button research-domain-nav-item${id === state.domainId ? " is-active" : ""}`;
      button.dataset.domainId = id;
      const label = plugin.metadata.label || id;
      const assetSuffix = summary?.asset_count ? ` · ${summary.asset_count} assets` : "";
      button.setAttribute("aria-label", `${label}${assetSuffix}`);
      button.setAttribute("title", `${label}${assetSuffix}`);
      button.appendChild(domainIcon(id));
      const statusBadge = createRuntimeStatusBadge(plugin, summary);
      if (statusBadge) button.appendChild(statusBadge);
      button.addEventListener("click", () => {
        window.dispatchEvent(new CustomEvent("atlas:research-domain-open", {
          detail: { domainId: id },
        }));
      });
      elements.nav.appendChild(button);
    }
  }

  function createRuntimeStatusBadge(plugin, summary) {
    const readyCount = (plugin.native_actions || []).filter(a => a.ready).length;
    const totalCount = (plugin.native_actions || []).length;

    if (readyCount === 0 && totalCount > 0) {
      const badge = document.createElement("span");
      badge.className = "domain-status-badge is-missing";
      badge.setAttribute("title", "Requires SDK installation");
      return badge;
    } else if (readyCount < totalCount) {
      const badge = document.createElement("span");
      badge.className = "domain-status-badge is-needs-assets";
      badge.setAttribute("title", `${readyCount}/${totalCount} actions ready`);
      return badge;
    } else if (readyCount > 0) {
      const badge = document.createElement("span");
      badge.className = "domain-status-badge is-ready";
      badge.setAttribute("title", `${readyCount} actions ready`);
      return badge;
    }
    return null;
  }

  async function loadWorkspace(domainId, {
    preserveAsset = true,
    quiet = false,
    highlightAssetId = "",
    requestedTab = "",
  } = {}) {
    const requestId = ++state.requestId;
    const previousAsset = preserveAsset ? state.assetId : "";
    if (!quiet) {
      elements.toolbarMeta.textContent = "Reading real workspace assets…";
      elements.refresh.disabled = true;
    }
    try {
      const params = new URLSearchParams({ domain_id: domainId });
      const workspace = await requestJson(`/api/research-domains/workspace?${params}`);
      if (requestId !== state.requestId || state.domainId !== domainId) return;
      state.workspace = workspace;
      const summary = {
        domain_id: domainId,
        asset_count: workspace?.assets?.length || 0,
        visualization_count: (workspace?.assets || []).reduce((total, asset) => total + (asset.visualizations?.length || 0), 0),
        revision: workspace?.revision || "",
        latest_modified_at: workspace?.assets?.[0]?.modified_at || null,
      };
      const summaries = state.catalog?.workspaces || [];
      const summaryIndex = summaries.findIndex((item) => item.domain_id === domainId);
      if (summaryIndex >= 0) summaries[summaryIndex] = summary;
      else summaries.push(summary);
      const assets = workspace?.assets || [];
      const sharedAssetId = String(workspace?.state?.active_asset_id || "");
      const sharedTaskPath = String(workspace?.state?.active_task?.artifacts?.[0]?.path || workspace?.state?.ui?.highlight_output_path || workspace?.state?.focus || "");
      const taskAsset = assets.find((asset) => normalizePath(asset.path) === normalizePath(sharedTaskPath));
      state.assetId = assets.some((asset) => asset.id === previousAsset)
        ? previousAsset
        : assets.some((asset) => asset.id === sharedAssetId)
          ? sharedAssetId
          : taskAsset?.id || assets[0]?.id || "";
      const tabs = domainSpec(domainId).tabs || [];
      const sharedTab = String(workspace?.state?.active_tab || "");
      if (tabs.includes(requestedTab)) state.activeTab = requestedTab;
      else if (tabs.includes(sharedTab)) state.activeTab = sharedTab;
      const requestedHighlight = assets.some((asset) => asset.id === highlightAssetId) ? highlightAssetId : "";
      const sharedHighlight = String(workspace?.state?.ui?.highlight_asset_id || "");
      if (requestedHighlight) state.highlightAssetId = requestedHighlight;
      else if (assets.some((asset) => asset.id === sharedHighlight)) state.highlightAssetId = sharedHighlight;
      else if (taskAsset) state.highlightAssetId = taskAsset.id;
      state.selectedAgent = String(workspace?.state?.selected_agent || state.selectedAgent || "");
      const selected = selectedAsset();
      const sharedVisualizationId = String(workspace?.state?.active_visualization_id || "");
      state.visualizationId = selected?.visualizations?.some((item) => item.id === state.visualizationId)
        ? state.visualizationId
        : selected?.visualizations?.some((item) => item.id === sharedVisualizationId)
          ? sharedVisualizationId
        : selected?.visualizations?.[0]?.id || "";
      renderNav();
      renderWorkspace();
      await Promise.all([
        loadNativeActions({ quiet }),
        loadDomainTasks({ quiet }),
      ]);
      if (state.assetId && state.visualizationId) {
        await loadVisualization({ quiet });
      } else {
        state.document = null;
        renderPreview();
      }
      schedulePolling();
    } finally {
      elements.refresh.disabled = false;
    }
  }

  function selectedAsset() {
    return (state.workspace?.assets || []).find((asset) => asset.id === state.assetId) || null;
  }

  function selectedVisualization() {
    return selectedAsset()?.visualizations?.find((item) => item.id === state.visualizationId) || null;
  }

  async function loadNativeActions({ quiet = true } = {}) {
    if (!state.domainId) return [];
    const requestId = ++state.actionsRequestId;
    const params = new URLSearchParams({ domain_id: state.domainId });
    if (state.assetId) params.set("asset_id", state.assetId);
    try {
      const payload = await requestJson(`/api/research-domains/actions?${params}`);
      if (requestId !== state.actionsRequestId) return state.actions;
      state.actions = Array.isArray(payload?.actions) ? payload.actions : [];
      renderNativeToolbar();
      if (state.activeTab !== "visualization") renderTabSurface();
      return state.actions;
    } catch (error) {
      if (!quiet) throw error;
      state.actions = [];
      renderNativeToolbar();
      return [];
    }
  }

  async function loadDomainTasks({ quiet = true } = {}) {
    if (!state.domainId) return [];
    const requestId = ++state.tasksRequestId;
    const params = new URLSearchParams({ domain_id: state.domainId });
    if (state.assetId) params.set("asset_id", state.assetId);
    try {
      const payload = await requestJson(`/api/research-domains/tasks?${params}`);
      if (requestId !== state.tasksRequestId) return state.tasks;
      state.taskCatalog = payload?.catalog || null;
      state.tasks = Array.isArray(payload?.tasks) ? payload.tasks : [];
      renderNativeToolbar();
      renderLiveContext();
      renderRuns();
      if (state.activeTab !== "visualization") renderTabSurface();
      return state.tasks;
    } catch (error) {
      if (!quiet) throw error;
      state.taskCatalog = null;
      state.tasks = [];
      renderNativeToolbar();
      return [];
    }
  }

  function intentEntries() {
    return Array.isArray(state.taskCatalog?.intents) ? state.taskCatalog.intents : [];
  }

  function activeDomainTask() {
    const shared = workspaceState()?.active_task;
    const id = String(shared?.id || "");
    return state.tasks.find((task) => task?.id === id) || (shared && typeof shared === "object" ? shared : null);
  }

  function formatBytes(value) {
    const bytes = Number(value || 0);
    if (!Number.isFinite(bytes) || bytes <= 0) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    const exponent = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)));
    return `${(bytes / (1024 ** exponent)).toFixed(exponent ? 1 : 0)} ${units[exponent]}`;
  }

  function formatRole(value) {
    return String(value || "domain")
      .split(/[-_\s]+/)
      .filter(Boolean)
      .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
      .join(" ");
  }

  function adapterStatuses() {
    return Array.isArray(state.workspace?.execution?.adapter_status)
      ? state.workspace.execution.adapter_status
      : [];
  }

  function adapterStatusFor(sdk) {
    const target = String(sdk || "").trim().toLowerCase();
    if (!target) return null;
    return adapterStatuses().find((item) => {
      const name = String(item?.sdk || "").trim().toLowerCase();
      return name === target || name.includes(target) || target.includes(name);
    }) || null;
  }

  function chooseAgent(preferred = "") {
    const agents = state.workspace?.domain?.supported_agents || [];
    if (preferred && agents.includes(preferred)) return preferred;
    if (state.selectedAgent && agents.includes(state.selectedAgent)) return state.selectedAgent;
    return agents[0] || preferred || "domain";
  }

  function setActiveTab(tab, { persist = true } = {}) {
    const spec = domainSpec();
    state.activeTab = spec.tabs.includes(tab) ? tab : "overview";
    elements.workspace.dataset.activeTab = state.activeTab;
    renderGlobalTabs();
    renderTabSurface();
    if (persist) queueWorkspaceState({ active_tab: state.activeTab });
  }

  function renderGlobalTabs() {
    const spec = domainSpec();
    elements.globalTabs.replaceChildren();
    const navigation = (spec.navigation || spec.tabs.map((tab, index) => ({
      tab,
      label: DOMAIN_TAB_LABELS[tab] || formatRole(tab),
      placement: index < 5 ? "primary" : "secondary",
    }))).filter((item) => spec.tabs.includes(item.tab));
    const primaryTabs = navigation.filter((item) => item.placement !== "secondary");
    const secondaryTabs = navigation.filter((item) => item.placement === "secondary");
    const createTab = (descriptor, inMenu = false) => {
      const tab = descriptor.tab;
      const button = document.createElement("button");
      button.type = "button";
      button.className = `${inMenu ? "research-domain-more-tab" : "research-domain-global-tab"}${tab === state.activeTab ? " is-active" : ""}`;
      button.dataset.domainTab = tab;
      button.setAttribute("aria-selected", tab === state.activeTab ? "true" : "false");
      button.textContent = descriptor.label || DOMAIN_TAB_LABELS[tab] || formatRole(tab);
      button.title = `${spec.studio}: ${button.textContent}`;
      button.addEventListener("click", () => setActiveTab(tab));
      return button;
    };
    primaryTabs.forEach((descriptor) => elements.globalTabs.appendChild(createTab(descriptor)));
    if (secondaryTabs.length) {
      const menu = document.createElement("details");
      menu.className = "research-domain-more-tabs";
      const summary = document.createElement("summary");
      const activeSecondary = secondaryTabs.find((item) => item.tab === state.activeTab);
      summary.textContent = activeSecondary?.label || "Environment";
      menu.appendChild(summary);
      const panel = document.createElement("div");
      secondaryTabs.forEach((descriptor) => {
        const button = createTab(descriptor, true);
        button.addEventListener("click", () => { menu.open = false; });
        panel.appendChild(button);
      });
      menu.appendChild(panel);
      elements.globalTabs.appendChild(menu);
    }
  }

  function renderObjectTypes() {
    const spec = domainSpec();
    const assets = state.workspace?.assets || [];
    elements.objectTypes.replaceChildren();
    const types = ["all", ...spec.dataObjects];
    types.forEach((type, index) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `research-domain-object-type${type === state.objectType ? " is-active" : ""}`;
      button.textContent = type === "all" ? `All ${assets.length}` : type;
      button.addEventListener("click", () => {
        state.objectType = type;
        renderObjectTypes();
        renderAssets(assets);
        queueWorkspaceState({ filters: { object_type: type } });
      });
      if (index > 6) button.classList.add("is-secondary");
      elements.objectTypes.appendChild(button);
    });
  }

  function assetMatchesObjectType(asset, objectType) {
    if (!objectType || objectType === "all") return true;
    const target = objectType.toLowerCase().replace(/\s+/g, "-");
    const source = [asset.file_type, asset.name, asset.path, ...(asset.visualizations || []).map((item) => item.id)]
      .join(" ")
      .toLowerCase();
    const aliases = {
      experiment: ["csv", "jsonl", "run", "metric"], checkpoint: ["ckpt", "pt", "pth", "safetensors"],
      model: ["onnx", "model", "urdf", "sdf"], image: ["png", "jpg", "jpeg", "webp", "tif"],
      video: ["mp4", "avi", "mov"], annotation: ["json", "xml", "conll"], packet: ["pcap", "pcapng"],
      capture: ["pcap", "pcapng", "har"], query: ["sql", "plan"], table: ["csv", "parquet", "arrow", "db"],
      trace: ["trace", "etl", "perf", "log", "jsonl"], mesh: ["obj", "ply", "stl", "vtk", "vtu"],
      scene: ["gltf", "glb", "blend", "fbx"], binary: ["exe", "dll", "so", "wasm", "bc"],
    };
    return source.includes(target) || (aliases[target] || []).some((alias) => source.includes(alias));
  }

  function renderAssets(assets) {
    const query = state.assetQuery.trim().toLowerCase();
    const filtered = assets.filter((asset) => assetMatchesObjectType(asset, state.objectType)
      && (!query || [asset.name, asset.path, asset.file_type]
        .some((value) => String(value || "").toLowerCase().includes(query))));
    elements.assets.replaceChildren();
    for (const asset of filtered) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `research-domain-asset${asset.id === state.assetId ? " is-active" : ""}${asset.id === state.highlightAssetId ? " is-highlighted" : ""}`;
      button.dataset.assetType = String(asset.file_type || "file");
      const head = document.createElement("span");
      head.className = "research-domain-asset-head";
      const type = document.createElement("span");
      type.className = "research-domain-asset-type";
      type.textContent = String(asset.file_type || "file").slice(0, 5).toUpperCase();
      const name = document.createElement("span");
      name.className = "research-domain-asset-name";
      name.textContent = asset.name || asset.path;
      head.append(type, name);
      const path = document.createElement("span");
      path.className = "research-domain-asset-path";
      path.textContent = asset.path || "";
      const meta = document.createElement("span");
      meta.className = "research-domain-asset-meta";
      const size = document.createElement("span");
      size.textContent = formatBytes(asset.size_bytes);
      const views = document.createElement("span");
      views.textContent = `${asset.visualizations?.length || 0} evidence view${asset.visualizations?.length === 1 ? "" : "s"}`;
      meta.append(size, views);
      button.append(head, path, meta);
      button.addEventListener("click", async () => {
        if (state.assetId === asset.id) return;
        state.assetId = asset.id;
        state.visualizationId = asset.visualizations?.[0]?.id || "";
        state.highlightAssetId = asset.id;
        state.document = null;
        renderWorkspace();
        renderPreview();
        loadNativeActions();
        queueWorkspaceState({
          active_asset_id: state.assetId,
          active_visualization_id: state.visualizationId,
          focus: asset.path || asset.id,
        });
        if (state.visualizationId) await loadVisualization();
      });
      elements.assets.appendChild(button);
      if (asset.id === state.locateAssetId) {
        state.locateAssetId = "";
        window.requestAnimationFrame(() => button.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" }));
      }
    }
    if (!filtered.length) {
      const empty = document.createElement("div");
      empty.className = "research-domain-assets-empty";
      empty.textContent = assets.length ? "No assets match this filter." : "No domain-native artifact is present in the workspace.";
      elements.assets.appendChild(empty);
    }
    const totalBytes = assets.reduce((total, asset) => total + Number(asset.size_bytes || 0), 0);
    elements.assetFooter.textContent = `${filtered.length}/${assets.length} assets · ${formatBytes(totalBytes)}`;
  }

  function renderWorkflowMarkup(plugin) {
    const stages = plugin?.workbench?.workflow || [];
    const host = document.createElement("div");
    host.className = "research-domain-workflow";
    stages.forEach((stage, index) => {
      const card = document.createElement("article");
      card.className = "research-domain-stage";
      const rail = document.createElement("div");
      rail.className = "research-domain-stage-rail";
      const number = document.createElement("span");
      number.textContent = String(index + 1).padStart(2, "0");
      rail.appendChild(number);
      const body = document.createElement("div");
      body.className = "research-domain-stage-body";
      const head = document.createElement("div");
      head.className = "research-domain-stage-head";
      const title = document.createElement("strong");
      title.textContent = stage.label || stage.id;
      const agent = document.createElement("button");
      agent.type = "button";
      agent.className = "research-domain-stage-agent";
      agent.textContent = `${formatRole(stage.agent)} Agent`;
      agent.title = `Dispatch this stage to ${formatRole(stage.agent)} Agent`;
      agent.addEventListener("click", () => dispatchAgentAction({ stage }));
      head.append(title, agent);
      const description = document.createElement("p");
      description.textContent = stage.description || "";
      const io = document.createElement("div");
      io.className = "research-domain-stage-io";
      const inputs = document.createElement("span");
      inputs.textContent = `IN  ${(stage.inputs || []).join(" + ") || "workspace evidence"}`;
      const outputs = document.createElement("span");
      outputs.textContent = `OUT  ${(stage.outputs || []).join(" + ") || "verified artifacts"}`;
      io.append(inputs, outputs);
      const gate = document.createElement("div");
      gate.className = "research-domain-stage-gate";
      gate.innerHTML = '<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m7 12 3 3 7-7"></path></svg>';
      const gateText = document.createElement("span");
      gateText.textContent = stage.gate || "Evidence gate required";
      gate.appendChild(gateText);
      body.append(head, description, io, gate);
      card.append(rail, body);
      host.appendChild(card);
    });
    return host;
  }

  function renderNativeToolbar() {
    elements.nativeActions.replaceChildren();
    const intents = intentEntries();
    const activeTask = activeDomainTask();
    const launch = document.createElement("button");
    launch.type = "button";
    launch.className = "research-domain-task-launch";
    const launchCopy = document.createElement("span");
    const launchKicker = document.createElement("small");
    launchKicker.textContent = activeTask && !["completed", "failed", "cancelled"].includes(String(activeTask.status || ""))
      ? `${String(activeTask.status || "planning").toUpperCase()} · ${formatRole(activeTask.current_stage || "plan")}`
      : "AGENT WORKFLOW";
    const launchLabel = document.createElement("strong");
    launchLabel.textContent = activeTask && !["completed", "failed", "cancelled"].includes(String(activeTask.status || ""))
      ? activeTask.intent_label || "Continue domain task"
      : `Describe ${state.workspace?.domain?.metadata?.label || "domain"} task…`;
    launchCopy.append(launchKicker, launchLabel);
    launch.appendChild(launchCopy);
    launch.disabled = !intents.length;
    launch.title = intents.length
      ? "Create a persistent domain task with explicit inputs, SDK actions, artifacts and verification gates."
      : "Domain task contracts are loading.";
    launch.addEventListener("click", () => {
      if (activeTask && !["completed", "failed", "cancelled"].includes(String(activeTask.status || ""))) setActiveTab("history");
      else openDomainTaskDialog();
    });
    elements.nativeActions.appendChild(launch);

    const readyCount = state.actions.filter((action) => action.ready).length;
    const tools = document.createElement("details");
    tools.className = "research-domain-sdk-menu";
    const summary = document.createElement("summary");
    summary.textContent = `SDK tools ${readyCount}/${state.actions.length}`;
    tools.appendChild(summary);
    const menu = document.createElement("div");
    menu.className = "research-domain-sdk-menu-panel";
    state.actions.forEach((action) => {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "research-domain-native-action";
      button.dataset.ready = action.ready ? "true" : "false";
      button.disabled = !action.ready || Boolean(state.runningAction);
      button.append(
        Object.assign(document.createElement("strong"), { textContent: action.label }),
        Object.assign(document.createElement("span"), { textContent: action.ready ? `${action.sdk}${action.version ? ` · ${action.version}` : ""}` : action.reason }),
      );
      button.title = action.description || action.reason;
      button.addEventListener("click", () => { tools.open = false; openNativeAction(action); });
      menu.appendChild(button);
    });
    if (!state.actions.length) {
      const status = document.createElement("span");
      status.className = "research-domain-native-actions-empty";
      status.textContent = "No registered native action";
      menu.appendChild(status);
    }
    tools.appendChild(menu);
    elements.nativeActions.appendChild(tools);
  }

  function preferredIntentEntry({ intentId = "", stage = null } = {}) {
    const entries = intentEntries();
    const saved = String(workspaceState()?.ui?.intent_id || "");
    return entries.find((entry) => entry?.contract?.id === intentId)
      || entries.find((entry) => stage?.id && entry?.contract?.workflow_stages?.includes(stage.id))
      || entries.find((entry) => entry?.contract?.id === saved)
      || entries[0]
      || null;
  }

  function openDomainTaskDialog({ intentId = "", initialPrompt = "", stage = null } = {}) {
    const entries = intentEntries();
    if (!entries.length) return;
    let selected = preferredIntentEntry({ intentId, stage });
    const dialog = document.createElement("dialog");
    dialog.className = "research-domain-task-dialog";
    const form = document.createElement("form");
    form.method = "dialog";
    const header = document.createElement("header");
    const identity = document.createElement("div");
    const kicker = document.createElement("span");
    kicker.textContent = `${domainSpec().studio} · AGENT TASK`;
    const title = document.createElement("strong");
    title.textContent = "Describe the research outcome";
    identity.append(kicker, title);
    const close = document.createElement("button");
    close.type = "button";
    close.className = "research-domain-action-dialog-close";
    close.setAttribute("aria-label", "Close domain task");
    close.textContent = "×";
    header.append(identity, close);

    const body = document.createElement("div");
    body.className = "research-domain-task-dialog-body";
    const intentList = document.createElement("aside");
    intentList.className = "research-domain-task-intents";
    const editor = document.createElement("section");
    editor.className = "research-domain-task-editor";
    const brief = document.createElement("textarea");
    brief.name = "prompt";
    brief.required = true;
    brief.maxLength = 16000;
    brief.value = initialPrompt || (stage ? `${stage.label}: ${stage.description || ""}` : "");
    const contract = document.createElement("div");
    contract.className = "research-domain-task-contract";
    const foot = document.createElement("footer");
    const cancel = document.createElement("button");
    cancel.type = "button";
    cancel.textContent = "Cancel";
    const start = document.createElement("button");
    start.type = "submit";
    start.className = "is-primary";
    start.textContent = "Create task & run with Agent";
    foot.append(cancel, start);

    const renderSelection = () => {
      intentList.replaceChildren();
      entries.forEach((entry) => {
        const item = document.createElement("button");
        item.type = "button";
        item.className = `research-domain-task-intent${entry === selected ? " is-active" : ""}`;
        const top = document.createElement("span");
        top.append(
          Object.assign(document.createElement("strong"), { textContent: entry.contract?.label || entry.contract?.id || "Task" }),
          Object.assign(document.createElement("i"), { textContent: String(entry.toolchain_status || "workspace").toUpperCase() }),
        );
        item.append(top, Object.assign(document.createElement("small"), { textContent: entry.contract?.description || "" }));
        item.addEventListener("click", () => { selected = entry; renderSelection(); });
        intentList.appendChild(item);
      });
      const intent = selected?.contract || {};
      brief.placeholder = intent.input_contract || "Describe the objective, real inputs, constraints and acceptance gate…";
      contract.replaceChildren();
      const scope = selectedAsset();
      const rows = [
        ["INPUT", intent.input_contract || "Workspace evidence"],
        ["SCOPE", scope ? `${scope.path} · ${String(scope.content_revision || "").slice(0, 10)}` : "Workspace scope · Agent must resolve real inputs"],
        ["OUTPUT", (intent.expected_outputs || []).join(" · ") || "Verified artifacts"],
        ["WORKFLOW", (intent.workflow_stages || []).map(formatRole).join(" → ") || "Plan → Execute → Verify"],
        ["GATE", intent.gate || "Verification evidence required"],
      ];
      rows.forEach(([label, value]) => {
        const row = document.createElement("div");
        row.append(Object.assign(document.createElement("span"), { textContent: label }), Object.assign(document.createElement("p"), { textContent: value }));
        contract.appendChild(row);
      });
      const sdk = document.createElement("div");
      sdk.className = "research-domain-task-sdk-strip";
      (selected?.sdk_statuses || []).forEach((status) => {
        const badge = document.createElement("span");
        badge.className = status.available === true ? "is-ready" : status.available === false ? "is-missing" : "is-unknown";
        badge.textContent = `${status.sdk} · ${status.available === true ? (status.version || "ready") : status.available === false ? "missing" : "resolve at runtime"}`;
        badge.title = status.reason || status.sdk;
        sdk.appendChild(badge);
      });
      (selected?.native_actions || []).forEach((action) => {
        const badge = document.createElement("span");
        badge.className = action.ready ? "is-ready" : "is-unknown";
        badge.textContent = `ACTION · ${action.label || action.id} · ${action.ready ? "ready" : "preflight"}`;
        badge.title = action.reason || action.sdk || action.id;
        sdk.appendChild(badge);
      });
      contract.appendChild(sdk);
      start.disabled = Boolean(intent.asset_required && !scope);
      start.title = start.disabled ? "Select a compatible real asset before starting this task." : "";
      queueWorkspaceState({ ui: { intent_id: intent.id || "" } });
    };
    renderSelection();
    editor.append(brief, contract);
    body.append(intentList, editor);
    form.append(header, body, foot);
    dialog.appendChild(form);
    elements.workspace.appendChild(dialog);

    const finish = () => { dialog.close?.(); dialog.remove(); };
    close.addEventListener("click", finish);
    cancel.addEventListener("click", finish);
    dialog.addEventListener("cancel", (event) => { event.preventDefault(); finish(); });
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      if (!form.reportValidity() || !selected) return;
      start.disabled = true;
      start.textContent = "Creating task…";
      try {
        const payload = await requestJson("/api/research-domains/tasks/begin", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            domain_id: state.domainId,
            intent_id: selected.contract.id,
            prompt: brief.value.trim(),
            asset_id: selectedAsset()?.id || null,
            parameters: workspaceState()?.parameters || {},
          }),
        });
        if (state.workspace && payload?.workspace_state) state.workspace.state = payload.workspace_state;
        if (payload?.task) {
          state.tasks = [payload.task, ...state.tasks.filter((task) => task?.id !== payload.task.id)];
          dispatchAgentTask(payload.task, selected);
        }
        finish();
        renderWorkspace();
      } catch (error) {
        start.disabled = false;
        start.textContent = "Create task & run with Agent";
        start.title = error?.message || String(error);
      }
    });
    dialog.showModal?.();
    window.requestAnimationFrame(() => brief.focus());
  }

  function dispatchAgentTask(task, intentEntry) {
    const intent = intentEntry?.contract || {};
    const asset = selectedAsset();
    state.selectedAgent = task.agent || chooseAgent(intent.agent);
    state.runLedger.unshift({
      id: task.id,
      taskId: task.id,
      label: task.intent_label || intent.label || "Domain task",
      agent: task.agent || intent.agent || "domain",
      asset: asset?.path || "workspace scope",
      startedAt: Date.now(),
      status: task.status || "planning",
      kind: "agent-task",
    });
    renderRuns();
    const prompt = [
      `Execute Atlas Research Domain task ${task.id} for domain_id=${state.domainId}.`,
      `User objective: ${task.prompt}`,
      `Intent: ${task.intent_label} (${task.intent_id}); assigned role=${formatRole(task.agent)} Agent.`,
      `Input contract: ${task.input_contract}`,
      `Expected artifacts: ${(task.expected_outputs || []).join(", ")}.`,
      `Workflow stages: ${(task.workflow_stages || []).join(" -> ")}. Current stage=${task.current_stage}.`,
      `Verification gate: ${task.gate}`,
      asset ? `Selected real asset: ${asset.path}; revision=${asset.content_revision}; type=${asset.file_type}.` : "No input asset is pinned; resolve real inputs from the live workspace before executing.",
      "First call research_domain_task with operation=read for this task_id, then read research_domain_workspace, research_domain_execution_context and research_domain_action operation=list for the explicit domain.",
      "Update research_domain_task to status=running and advance current_stage as work progresses. Use only installed SDKs, registered native actions, real project entry points and real workspace files. If a required SDK/input is absent, set status=blocked with a concrete note; never fabricate an execution, metric, model, trace, mesh, drawing or preview.",
      "For generative work, materialize a reviewable domain-native source/configuration and execute the detected SDK or project pipeline. Record commands, versions and validation evidence. Keep research_domain_workspace_state synchronized with the focused artifact and visualization.",
      "Before completion, call research_domain_task operation=update with status=completed, the final workflow stage, real artifact paths and verification evidence. Completion will be rejected unless artifacts exist inside the workspace and evidence is present.",
    ].join("\n");
    window.dispatchEvent(new CustomEvent("atlas:domain-agent-dispatch", {
      detail: { domainId: state.domainId, assetId: asset?.id || "", agent: task.agent, actionId: task.intent_id, taskId: task.id, label: task.intent_label, prompt, preserveWorkspace: true },
    }));
  }

  function actionParameters(action) {
    const saved = workspaceState()?.parameters?.[action.id] || {};
    const schema = action.parameters || [];
    if (!schema.length) return Promise.resolve({});
    return new Promise((resolve) => {
      const dialog = document.createElement("dialog");
      dialog.className = "research-domain-action-dialog";
      const form = document.createElement("form");
      form.method = "dialog";
      const head = document.createElement("header");
      const identity = document.createElement("div");
      const kicker = document.createElement("span");
      kicker.textContent = `NATIVE SDK · ${action.sdk}`;
      const title = document.createElement("strong");
      title.textContent = action.label;
      identity.append(kicker, title);
      const close = document.createElement("button");
      close.type = "button";
      close.className = "research-domain-action-dialog-close";
      close.setAttribute("aria-label", "Close action parameters");
      close.textContent = "×";
      head.append(identity, close);
      const description = document.createElement("p");
      description.textContent = action.description;
      const fields = document.createElement("section");
      fields.className = "research-domain-action-fields";
      const inputs = new Map();
      schema.forEach((parameter) => {
        const label = document.createElement("label");
        const copy = document.createElement("span");
        const name = document.createElement("strong");
        name.textContent = parameter.label;
        const detail = document.createElement("small");
        detail.textContent = parameter.description || parameter.id;
        copy.append(name, detail);
        let input;
        if (parameter.value_type === "enum" && parameter.choices?.length) {
          input = document.createElement("select");
          parameter.choices.forEach((choice) => {
            const option = document.createElement("option");
            option.value = choice;
            option.textContent = choice;
            input.appendChild(option);
          });
        } else {
          input = document.createElement("input");
          input.type = parameter.value_type === "number" ? "number" : "text";
          if (parameter.minimum !== null && parameter.minimum !== undefined) input.min = String(parameter.minimum);
          if (parameter.maximum !== null && parameter.maximum !== undefined) input.max = String(parameter.maximum);
        }
        input.name = parameter.id;
        input.required = Boolean(parameter.required);
        input.value = String(saved[parameter.id] ?? parameter.default ?? "");
        inputs.set(parameter.id, { input, parameter });
        label.append(copy, input);
        fields.appendChild(label);
      });
      const foot = document.createElement("footer");
      const cancel = document.createElement("button");
      cancel.type = "button";
      cancel.textContent = "Cancel";
      const run = document.createElement("button");
      run.type = "submit";
      run.className = "is-primary";
      run.textContent = `Run with ${action.sdk}`;
      foot.append(cancel, run);
      form.append(head, description, fields, foot);
      dialog.appendChild(form);
      elements.workspace.appendChild(dialog);
      let settled = false;
      const finish = (value) => {
        if (settled) return;
        settled = true;
        dialog.close?.();
        dialog.remove();
        resolve(value);
      };
      close.addEventListener("click", () => finish(null));
      cancel.addEventListener("click", () => finish(null));
      dialog.addEventListener("cancel", (event) => { event.preventDefault(); finish(null); });
      form.addEventListener("submit", (event) => {
        event.preventDefault();
        if (!form.reportValidity()) return;
        const parameters = {};
        inputs.forEach(({ input, parameter }, id) => {
          parameters[id] = parameter.value_type === "number" ? Number(input.value) : input.value;
        });
        finish(parameters);
      });
      if (typeof dialog.showModal === "function") dialog.showModal();
      else dialog.setAttribute("open", "");
      inputs.values().next().value?.input?.focus();
    });
  }

  async function openNativeAction(action) {
    if (!action?.ready || state.runningAction) return;
    const parameters = await actionParameters(action);
    if (parameters === null) return;
    const asset = selectedAsset();
    const run = {
      id: `${Date.now()}-${action.id}`,
      actionId: action.id,
      taskId: "",
      label: action.label,
      agent: "native-sdk",
      sdk: action.sdk,
      asset: asset?.path || "workspace scope",
      startedAt: Date.now(),
      status: "queued",
      log: "",
    };
    state.runLedger.unshift(run);
    state.runningAction = run;
    renderRuns();
    renderNativeToolbar();
    renderTabSurface();
    try {
      const payload = await requestJson("/api/research-domains/actions/run", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          domain_id: state.domainId,
          action_id: action.id,
          asset_id: asset?.id || null,
          parameters,
        }),
      });
      run.taskId = payload?.task?.id || "";
      run.status = payload?.task?.status || "running";
      run.outputPath = payload?.output_path || "";
      if (state.workspace && payload?.workspace_state) state.workspace.state = payload.workspace_state;
      renderRuns();
      renderLiveContext();
      scheduleActionPolling(run);
    } catch (error) {
      run.status = "failed";
      run.log = error.message || String(error);
      state.runningAction = null;
      renderRuns();
      renderNativeToolbar();
    }
  }

  async function scheduleActionPolling(run) {
    if (!run?.taskId) return;
    try {
      const payload = await requestJson("/api/tasks/log", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ task_id: run.taskId }),
      });
      run.log = payload?.log || "";
      const tasks = await requestJson("/api/tasks");
      const task = (tasks?.tasks || []).find((item) => item.id === run.taskId);
      if (task) run.status = task.status;
      renderRuns();
      if (["queued", "running"].includes(run.status)) {
        window.setTimeout(() => scheduleActionPolling(run), 750);
      } else {
        state.runningAction = null;
        await persistWorkspaceState({
          focus: run.outputPath || run.asset,
          last_run: { task_id: run.taskId, action_id: run.actionId, status: run.status, output_path: run.outputPath || "" },
          ui: { last_action_status: run.status },
        });
        renderNativeToolbar();
        renderTabSurface();
        await loadWorkspace(state.domainId, { preserveAsset: false, quiet: true });
      }
    } catch (error) {
      run.log = error.message || String(error);
      renderRuns();
      window.setTimeout(() => scheduleActionPolling(run), 1200);
    }
  }

  function renderAgents(plugin) {
    const agents = plugin?.supported_agents || [];
    if (!state.selectedAgent || !agents.includes(state.selectedAgent)) state.selectedAgent = agents[0] || "";
    elements.agents.replaceChildren();
    elements.agentContextTitle.textContent = `${plugin?.metadata?.label || "Domain"} crew`;
    elements.agentContextCopy.textContent = "The selected role receives the same persisted task, asset revision, tool readiness and verification evidence as this workbench.";
    const control = document.createElement("label");
    control.className = "research-domain-agent-router";
    const avatar = document.createElement("span");
    avatar.className = "research-domain-agent-avatar";
    avatar.textContent = formatRole(state.selectedAgent || "AI").split(" ").map((word) => word[0]).join("").slice(0, 2);
    const copy = document.createElement("span");
    copy.append(
      Object.assign(document.createElement("small"), { textContent: "ROUTING ROLE" }),
      Object.assign(document.createElement("strong"), { textContent: `${formatRole(state.selectedAgent || "domain")} Agent` }),
    );
    const select = document.createElement("select");
    agents.forEach((agentName) => {
      const option = document.createElement("option");
      option.value = agentName;
      option.textContent = formatRole(agentName);
      option.selected = agentName === state.selectedAgent;
      select.appendChild(option);
    });
    select.addEventListener("change", () => {
      state.selectedAgent = select.value;
      renderAgents(plugin);
      queueWorkspaceState({ selected_agent: state.selectedAgent });
    });
    control.append(avatar, copy, select);
    elements.agents.appendChild(control);
  }

  function renderLiveContext() {
    const spec = domainSpec();
    const shared = workspaceState();
    const asset = selectedAsset();
    const task = activeDomainTask();
    elements.liveContext.replaceChildren();
    const header = document.createElement("div");
    header.className = "research-domain-live-context-head";
    header.innerHTML = '<span class="research-domain-surface-label">SHARED LIVE STATE</span>';
    const revision = document.createElement("code");
    revision.textContent = String(shared.revision || state.workspace?.revision || "").slice(0, 10);
    header.appendChild(revision);
    const rows = [
      ["Surface", domainTabDescriptor(state.activeTab).label],
      ["Focus", asset?.name || shared.focus || "Workspace"],
      ["Agent", formatRole(state.selectedAgent || "auto")],
      ["Task", task ? `${task.intent_label || task.intent_id} · ${task.status || "planning"}` : "No active task"],
      ["Stage", task?.current_stage ? formatRole(task.current_stage) : "—"],
      ["State by", shared.updated_by || "system"],
    ];
    const list = document.createElement("div");
    list.className = "research-domain-live-context-list";
    rows.forEach(([label, value]) => {
      const row = document.createElement("div");
      const key = document.createElement("span");
      key.textContent = label;
      const detail = document.createElement("strong");
      detail.textContent = value;
      row.append(key, detail);
      list.appendChild(row);
    });
    const description = document.createElement("p");
    description.textContent = spec.agentContext;
    elements.liveContext.append(header, list, description);
  }

  function surfaceHeader(kicker, title, meta = "") {
    const header = document.createElement("header");
    header.className = "research-domain-surface-head";
    const copy = document.createElement("div");
    const small = document.createElement("span");
    small.className = "research-domain-surface-label";
    small.textContent = kicker;
    const strong = document.createElement("strong");
    strong.textContent = title;
    copy.append(small, strong);
    const detail = document.createElement("span");
    detail.textContent = meta;
    header.append(copy, detail);
    return header;
  }

  function zoneTitle(zone) {
    return formatRole(zone.replace(/^(?:research-domain-)?/, ""));
  }

  function zoneKind(zone) {
    if (/viewport|canvas|map|graph|dag|topology|diagram|flow/i.test(zone)) return "diagram";
    if (/timeline|strip|history|train/i.test(zone)) return "timeline";
    if (/table|grid|queue|items|variable/i.test(zone)) return "table";
    if (/editor|source|assembly|disassembly|decompiler|equation|hex/i.test(zone)) return "editor";
    if (/tree|nav|outliner|bin|stack/i.test(zone)) return "tree";
    if (/metric|health|risk|stats|resource|counter|status|readout|overview/i.test(zone)) return "metrics";
    if (/workflow|pipeline|stage/i.test(zone)) return "workflow";
    if (/activity|event|console|diagnostic|expert|monitor/i.test(zone)) return "activity";
    return "inspector";
  }

  function appendMiniDiagram(host, assets, spec) {
    const diagram = svg("svg", { viewBox: "0 0 600 240", role: "img", "aria-label": `${spec.studio} workspace relationships` });
    diagram.classList.add("research-domain-mini-diagram");
    const candidates = assets.slice(0, 7);
    const points = candidates.map((asset, index) => {
      const angle = (Math.PI * 2 * index) / Math.max(1, candidates.length) - Math.PI / 2;
      return { asset, x: 300 + Math.cos(angle) * 190, y: 120 + Math.sin(angle) * 78 };
    });
    points.forEach((point, index) => {
      const next = points[(index + 1) % points.length];
      if (!next || points.length < 2) return;
      diagram.appendChild(svg("path", { class: "research-domain-mini-edge", d: `M${point.x},${point.y} L${next.x},${next.y}` }));
    });
    points.forEach(({ asset, x, y }) => {
      const group = svg("g", { class: `research-domain-mini-node${asset.id === state.assetId ? " is-active" : ""}`, transform: `translate(${x},${y})` });
      group.appendChild(svg("rect", { x: -55, y: -18, width: 110, height: 36, rx: 7 }));
      const label = svg("text", { x: 0, y: 4, "text-anchor": "middle" });
      label.textContent = truncate(asset.name || asset.path, 17);
      group.appendChild(label);
      group.addEventListener("click", () => {
        state.assetId = asset.id;
        state.highlightAssetId = asset.id;
        state.visualizationId = asset.visualizations?.[0]?.id || "";
        renderWorkspace();
        queueWorkspaceState({ active_asset_id: asset.id, focus: asset.path || asset.id });
      });
      diagram.appendChild(group);
    });
    if (!points.length) {
      const label = svg("text", { class: "research-domain-mini-empty", x: 300, y: 124, "text-anchor": "middle" });
      label.textContent = "No live workspace relationships yet";
      diagram.appendChild(label);
    }
    host.appendChild(diagram);
  }

  function appendTimeline(host, assets, plugin) {
    const list = document.createElement("div");
    list.className = "research-domain-native-timeline";
    const entries = assets.slice(0, 5).map((asset) => ({
      label: asset.name || asset.path,
      detail: `${String(asset.file_type || "asset").toUpperCase()} · ${asset.modified_at || "revision tracked"}`,
    }));
    if (!entries.length) {
      (plugin?.workbench?.workflow || []).forEach((stage) => entries.push({ label: stage.label, detail: stage.gate }));
    }
    entries.forEach((entry, index) => {
      const item = document.createElement("div");
      const marker = document.createElement("i");
      marker.textContent = String(index + 1).padStart(2, "0");
      const copy = document.createElement("span");
      const label = document.createElement("strong");
      label.textContent = entry.label;
      const detail = document.createElement("span");
      detail.textContent = entry.detail;
      copy.append(label, detail);
      item.append(marker, copy);
      list.appendChild(item);
    });
    host.appendChild(list);
  }

  function appendAssetRows(host, assets, limit = 7) {
    const list = document.createElement("div");
    list.className = "research-domain-native-rows";
    assets.slice(0, limit).forEach((asset) => {
      const row = document.createElement("button");
      row.type = "button";
      row.className = asset.id === state.assetId ? "is-active" : "";
      const type = document.createElement("code");
      type.textContent = String(asset.file_type || "file").toUpperCase();
      const name = document.createElement("strong");
      name.textContent = asset.name || asset.path;
      const meta = document.createElement("span");
      meta.textContent = `${formatBytes(asset.size_bytes)} · ${asset.visualizations?.length || 0} views`;
      row.append(type, name, meta);
      row.addEventListener("click", () => {
        state.assetId = asset.id;
        state.highlightAssetId = asset.id;
        state.visualizationId = asset.visualizations?.[0]?.id || "";
        renderWorkspace();
        queueWorkspaceState({ active_asset_id: asset.id, focus: asset.path || asset.id });
      });
      list.appendChild(row);
    });
    if (!assets.length) {
      const empty = document.createElement("div");
      empty.className = "research-domain-native-empty";
      empty.textContent = "No matching live object is present.";
      list.appendChild(empty);
    }
    host.appendChild(list);
  }

  function appendMetrics(host, assets, plugin, spec) {
    const metrics = [
      [spec.dataObjects[0] || "Objects", assets.length],
      ["Evidence views", assets.reduce((sum, asset) => sum + (asset.visualizations?.length || 0), 0)],
      ["Agents", plugin?.supported_agents?.length || 0],
      ["SDK ready", adapterStatuses().filter((item) => item?.available).length],
    ];
    const grid = document.createElement("div");
    grid.className = "research-domain-native-metrics";
    metrics.forEach(([label, value]) => {
      const card = document.createElement("div");
      const number = document.createElement("strong");
      number.textContent = String(value);
      const caption = document.createElement("span");
      caption.textContent = label;
      card.append(number, caption);
      grid.appendChild(card);
    });
    host.appendChild(grid);
  }

  function appendEditorEvidence(host, asset, spec) {
    const editor = document.createElement("div");
    editor.className = "research-domain-native-editor";
    const lines = asset ? [
      ["OBJECT", asset.name || asset.path],
      ["PATH", asset.path],
      ["TYPE", String(asset.file_type || "file").toUpperCase()],
      ["REVISION", String(asset.content_revision || "").slice(0, 20)],
      ["MODE", spec.interaction],
    ] : [["WORKSPACE", "Select or generate a domain-native object to inspect it here."]];
    lines.forEach(([label, value], index) => {
      const row = document.createElement("div");
      const line = document.createElement("span");
      line.textContent = String(index + 1).padStart(2, "0");
      const key = document.createElement("code");
      key.textContent = label;
      const detail = document.createElement("strong");
      detail.textContent = value || "—";
      row.append(line, key, detail);
      editor.appendChild(row);
    });
    host.appendChild(editor);
  }

  function appendActivity(host) {
    const list = document.createElement("div");
    list.className = "research-domain-native-activity";
    const runs = state.runLedger.slice(0, 6);
    runs.forEach((run) => {
      const row = document.createElement("div");
      row.className = `is-${run.status || "queued"}`;
      const dot = document.createElement("i");
      const copy = document.createElement("span");
      const label = document.createElement("strong");
      label.textContent = run.label;
      const meta = document.createElement("span");
      meta.textContent = `${formatRole(run.agent)} Agent · ${run.status}`;
      copy.append(label, meta);
      row.append(dot, copy);
      list.appendChild(row);
    });
    if (!runs.length) {
      const empty = document.createElement("div");
      empty.className = "research-domain-native-empty";
      empty.textContent = "No Agent operation has run in this workspace session.";
      list.appendChild(empty);
    }
    host.appendChild(list);
  }

  function renderNativeZone(zone, assets, plugin, spec) {
    const section = document.createElement("section");
    const kind = zoneKind(zone);
    section.className = `research-domain-zone is-${kind}`;
    section.dataset.zone = zone;
    section.appendChild(surfaceHeader(kind.toUpperCase(), zoneTitle(zone), kind === "diagram" ? spec.reference : ""));
    const body = document.createElement("div");
    body.className = "research-domain-zone-body";
    if (kind === "diagram") appendMiniDiagram(body, assets, spec);
    else if (kind === "timeline") appendTimeline(body, assets, plugin);
    else if (kind === "table" || kind === "tree") appendAssetRows(body, assets);
    else if (kind === "metrics") appendMetrics(body, assets, plugin, spec);
    else if (kind === "editor") appendEditorEvidence(body, selectedAsset(), spec);
    else if (kind === "workflow") body.appendChild(renderWorkflowMarkup(plugin));
    else if (kind === "activity") appendActivity(body);
    else {
      appendEditorEvidence(body, selectedAsset(), spec);
    }
    section.appendChild(body);
    return section;
  }

  function renderOverviewSurface() {
    const spec = domainSpec();
    const plugin = state.workspace?.domain || pluginById(state.domainId);
    const assets = state.workspace?.assets || [];
    const professional = window.AtlasResearchWorkbenches?.render?.({
      domainId: state.domainId,
      spec,
      plugin,
      assets,
      selectedAsset: selectedAsset(),
      selectedVisualization: selectedVisualization(),
      documentData: state.document,
      workspaceState: workspaceState(),
      actions: state.actions,
      tasks: state.tasks,
      activeTask: activeDomainTask(),
      runs: state.runLedger,
      runningAction: state.runningAction,
      actionLog: state.actionLog,
      runAction: openNativeAction,
      openTab(tab) { setActiveTab(tab); },
      openTask: openDomainTaskDialog,
      mountGeometry(canvas, geometry) {
        state.workbenchViewer3d?.dispose?.();
        state.workbenchViewer3d = window.AtlasDomain3D?.mount?.(canvas, geometry, {
          domainId: state.domainId,
          visualizationId: state.visualizationId,
        }) || null;
      },
      selectAsset(asset) {
        if (!asset?.id) return;
        state.assetId = asset.id;
        state.visualizationId = asset.visualizations?.[0]?.id || "";
        state.highlightAssetId = asset.id;
        state.document = null;
        renderWorkspace();
        loadNativeActions();
        queueWorkspaceState({ active_asset_id: asset.id, active_visualization_id: state.visualizationId, focus: asset.path || asset.id });
        if (state.visualizationId) loadVisualization();
      },
      updateUi(patch) { queueWorkspaceState({ ui: patch }); },
    });
    if (professional) {
      elements.nativeSurface.replaceChildren(professional);
      return;
    }
    const fragment = document.createDocumentFragment();
    const hero = document.createElement("section");
    hero.className = "research-domain-native-hero";
    const copy = document.createElement("div");
    const kicker = document.createElement("span");
    kicker.className = "research-domain-surface-label";
    kicker.textContent = spec.reference;
    const title = document.createElement("strong");
    title.textContent = spec.focus;
    const text = document.createElement("p");
    text.textContent = spec.interaction;
    copy.append(kicker, title, text);
    const types = document.createElement("div");
    types.className = "research-domain-native-object-chips";
    spec.dataObjects.slice(0, 8).forEach((item) => {
      const chip = document.createElement("span");
      chip.textContent = item;
      types.appendChild(chip);
    });
    hero.append(copy, types);
    fragment.appendChild(hero);
    const grid = document.createElement("div");
    grid.className = "research-domain-native-grid";
    spec.zones.forEach((zone) => grid.appendChild(renderNativeZone(zone, assets, plugin, spec)));
    fragment.appendChild(grid);
    elements.nativeSurface.replaceChildren(fragment);
  }

  function renderResourcesTab() {
    const spec = domainSpec();
    const assets = state.workspace?.assets || [];
    const shell = document.createElement("section");
    shell.className = "research-domain-tab-page research-domain-resources-page";
    shell.appendChild(surfaceHeader("DOMAIN OBJECT MODEL", `${spec.studio} resources`, `${assets.length} live assets`));
    const taxonomy = document.createElement("div");
    taxonomy.className = "research-domain-taxonomy";
    spec.dataObjects.forEach((type) => {
      const card = document.createElement("button");
      card.type = "button";
      card.className = type === state.objectType ? "is-active" : "";
      const title = document.createElement("strong");
      title.textContent = type;
      const meta = document.createElement("span");
      meta.textContent = `${assets.filter((asset) => assetMatchesObjectType(asset, type)).length} matched workspace objects`;
      card.append(title, meta);
      card.addEventListener("click", () => {
        state.objectType = type;
        renderObjectTypes();
        renderAssets(assets);
        renderResourcesTab();
        queueWorkspaceState({ filters: { object_type: type } });
      });
      taxonomy.appendChild(card);
    });
    shell.appendChild(taxonomy);
    appendAssetRows(shell, assets.filter((asset) => assetMatchesObjectType(asset, state.objectType)), 40);
    elements.nativeSurface.replaceChildren(shell);
  }

  function renderArtifactsTab() {
    const spec = domainSpec();
    const assets = state.workspace?.assets || [];
    const shell = document.createElement("section");
    shell.className = "research-domain-tab-page research-domain-artifacts-page";
    shell.appendChild(surfaceHeader("PROVENANCE & OUTPUTS", `${spec.dataObjects.slice(-2).join(" / ")} artifacts`, "Content-addressed workspace evidence"));
    const lineage = document.createElement("div");
    lineage.className = "research-domain-artifact-lineage";
    assets.slice(0, 24).forEach((asset, index) => {
      const card = document.createElement("button");
      card.type = "button";
      const indexLabel = document.createElement("i");
      indexLabel.textContent = String(index + 1).padStart(2, "0");
      const copy = document.createElement("span");
      const title = document.createElement("strong");
      title.textContent = asset.name || asset.path;
      const meta = document.createElement("span");
      meta.textContent = `${asset.path} · rev ${String(asset.content_revision || "").slice(0, 10)}`;
      copy.append(title, meta);
      const views = document.createElement("code");
      views.textContent = `${asset.visualizations?.length || 0} VIEWS`;
      card.append(indexLabel, copy, views);
      card.addEventListener("click", () => {
        state.assetId = asset.id;
        state.highlightAssetId = asset.id;
        setActiveTab("visualization");
        queueWorkspaceState({ active_asset_id: asset.id, focus: asset.path || asset.id });
      });
      lineage.appendChild(card);
    });
    if (!assets.length) appendAssetRows(lineage, []);
    shell.appendChild(lineage);
    elements.nativeSurface.replaceChildren(shell);
  }

  function renderHistoryTab() {
    const spec = domainSpec();
    const shell = document.createElement("section");
    shell.className = "research-domain-tab-page research-domain-history-page";
    shell.appendChild(surfaceHeader("RESEARCH HISTORY", `${spec.studio} event stream`, `State rev ${String(workspaceState().revision || "").slice(0, 10)}`));
    appendTimeline(shell, state.workspace?.assets || [], state.workspace?.domain);
    appendActivity(shell);
    if (state.actionLog) {
      const log = document.createElement("pre");
      log.className = "research-domain-action-log";
      log.textContent = state.actionLog;
      shell.appendChild(log);
    }
    elements.nativeSurface.replaceChildren(shell);
  }

  function renderSettingsTab() {
    const spec = domainSpec();
    const parameters = workspaceState().parameters || {};
    const shell = document.createElement("section");
    shell.className = "research-domain-tab-page research-domain-settings-page";
    shell.appendChild(surfaceHeader("DOMAIN PARAMETERS", `${spec.studio} settings`, "Changes synchronize to Agent Context"));
    const form = document.createElement("div");
    form.className = "research-domain-settings-grid";
    spec.settings.forEach((key) => {
      const label = document.createElement("label");
      const title = document.createElement("span");
      title.textContent = formatRole(key);
      const input = document.createElement("input");
      input.value = parameters[key] ?? "";
      input.placeholder = `Set ${formatRole(key).toLowerCase()}`;
      input.addEventListener("change", () => persistWorkspaceState({ parameters: { [key]: input.value } }));
      label.append(title, input);
      form.appendChild(label);
    });
    const notes = document.createElement("label");
    notes.className = "research-domain-settings-notes";
    const title = document.createElement("span");
    title.textContent = "Research Notes";
    const area = document.createElement("textarea");
    area.value = String(workspaceState().notes || "");
    area.placeholder = "Shared notes for the domain Agents…";
    area.addEventListener("change", () => persistWorkspaceState({ notes: area.value }));
    notes.append(title, area);
    form.appendChild(notes);
    shell.appendChild(form);
    elements.nativeSurface.replaceChildren(shell);
  }

  function renderAgentContextTab() {
    const spec = domainSpec();
    const plugin = state.workspace?.domain || pluginById(state.domainId);
    const shell = document.createElement("section");
    shell.className = "research-domain-tab-page research-domain-context-page";
    shell.appendChild(surfaceHeader("AGENT CONTEXT CONTRACT", `${spec.studio} shared state`, "UI ↔ Agent bidirectional synchronization"));
    const contract = document.createElement("div");
    contract.className = "research-domain-context-contract";
    const prose = document.createElement("section");
    const heading = document.createElement("strong");
    heading.textContent = "What the Agent receives";
    const description = document.createElement("p");
    description.textContent = spec.agentContext;
    const interaction = document.createElement("p");
    interaction.textContent = `Interaction contract: ${spec.interaction}`;
    prose.append(heading, description, interaction);
      const json = document.createElement("pre");
    json.textContent = JSON.stringify({
      domain_id: state.domainId,
      workspace_state: workspaceState(),
      selected_asset: selectedAsset()?.path || null,
      supported_agents: plugin?.supported_agents || [],
      ready_adapters: adapterStatuses().filter((item) => item?.available).map((item) => item.sdk),
      native_actions: state.actions.map((action) => ({ id: action.id, sdk: action.sdk, ready: action.ready, reason: action.reason })),
      domain_intents: intentEntries().map((entry) => ({ id: entry?.contract?.id, label: entry?.contract?.label, toolchain_status: entry?.toolchain_status })),
      active_task: activeDomainTask(),
      last_native_run: workspaceState().last_run || null,
    }, null, 2);
    contract.append(prose, json);
    shell.appendChild(contract);
    elements.nativeSurface.replaceChildren(shell);
  }

  function renderPreviewTab() {
    const spec = domainSpec();
    const asset = selectedAsset();
    const shell = document.createElement("section");
    shell.className = `research-domain-tab-page research-domain-preview-page preview-${spec.preview}`;
    shell.appendChild(surfaceHeader("RESEARCH PREVIEW", `${spec.studio} final output card`, asset?.name || "No generated artifact selected"));
    const card = buildDomainPreviewCard({ spec, asset, visualization: selectedVisualization(), documentData: state.document, interactive: true });
    shell.appendChild(card);
    elements.nativeSurface.replaceChildren(shell);
  }

  function professionalTabContext() {
    const spec = domainSpec();
    return {
      domainId: state.domainId,
      spec,
      plugin: state.workspace?.domain || pluginById(state.domainId),
      assets: state.workspace?.assets || [],
      selectedAsset: selectedAsset(),
      selectedVisualization: selectedVisualization(),
      documentData: state.document,
      workspaceState: workspaceState(),
      actions: state.actions,
      tasks: state.tasks,
      activeTask: activeDomainTask(),
      runs: state.runLedger,
      runningAction: state.runningAction,
      actionLog: state.actionLog,
      runAction: openNativeAction,
      openTask: openDomainTaskDialog,
      openTab(tab) { setActiveTab(tab); },
      mountGeometry(canvas, geometry) {
        state.workbenchViewer3d?.dispose?.();
        state.workbenchViewer3d = window.AtlasDomain3D?.mount?.(canvas, geometry, {
          domainId: state.domainId,
          visualizationId: state.visualizationId,
        }) || null;
      },
      assetMatches: assetMatchesObjectType,
      selectAsset(asset) {
        if (!asset?.id) return;
        state.assetId = asset.id;
        state.visualizationId = asset.visualizations?.[0]?.id || "";
        state.highlightAssetId = asset.id;
        state.document = null;
        renderWorkspace();
        loadNativeActions();
        queueWorkspaceState({ active_asset_id: asset.id, active_visualization_id: state.visualizationId, focus: asset.path || asset.id });
        if (state.visualizationId) loadVisualization();
      },
      filterType(type) {
        state.objectType = type;
        renderObjectTypes();
        renderAssets(state.workspace?.assets || []);
        queueWorkspaceState({ filters: { object_type: type } });
      },
      updateParameters(patch) { persistWorkspaceState({ parameters: patch }); },
      updateUi(patch) { queueWorkspaceState({ ui: patch }); },
    };
  }

  function renderTabSurface() {
    state.workbenchViewer3d?.dispose?.();
    state.workbenchViewer3d = null;
    const visualization = state.activeTab === "visualization";
    elements.tabSurface.hidden = visualization;
    elements.previewPanel.hidden = !visualization;
    elements.operationsPanel.hidden = !state.runningAction;
    elements.primaryLabel.textContent = domainTabDescriptor(state.activeTab).label;
    if (visualization) {
      window.requestAnimationFrame(() => {
        if (state.document) renderPreview();
        else state.viewer3d?.resize?.();
      });
      return;
    }
    if (!["preview"].includes(state.activeTab)) {
      const professional = window.AtlasResearchWorkbenches?.renderTab?.(professionalTabContext(), state.activeTab);
      if (professional) {
        elements.nativeSurface.replaceChildren(professional);
        return;
      }
    }
    if (state.activeTab === "overview") renderOverviewSurface();
    else if (state.activeTab === "resources") renderResourcesTab();
    else if (state.activeTab === "artifacts") renderArtifactsTab();
    else if (state.activeTab === "history") renderHistoryTab();
    else if (state.activeTab === "settings") renderSettingsTab();
    else if (state.activeTab === "agent-context") renderAgentContextTab();
    else if (state.activeTab === "preview") renderPreviewTab();
    else renderOverviewSurface();
  }

  function renderAdapters(plugin) {
    const statuses = adapterStatuses();
    elements.adapters.replaceChildren();
    for (const adapter of statuses) {
      const row = document.createElement("div");
      row.className = "research-domain-adapter";
      const stateMark = document.createElement("i");
      stateMark.className = adapter?.available ? "is-ready" : "";
      const copy = document.createElement("span");
      const name = document.createElement("strong");
      name.textContent = adapter?.sdk || "SDK";
      const detail = document.createElement("span");
      detail.textContent = adapter?.available
        ? String(adapter.executable || "Executable detected")
        : "Not detected · Agent may use project-local tooling";
      copy.append(name, detail);
      detail.textContent = adapter?.available
        ? `${adapter.version || "Ready"} · ${String(adapter.executable || "Executable detected")}`
        : String(adapter.reason || "Not detected");
      row.append(stateMark, copy);
      elements.adapters.appendChild(row);
    }
    if (!statuses.length) {
      const empty = document.createElement("div");
      empty.className = "research-domain-adapters-empty";
      empty.textContent = (plugin?.sdk_adapters || []).length ? "Checking adapters…" : "No SDK adapter declared.";
      elements.adapters.appendChild(empty);
    }
    const providers = [
      ["DATA", plugin?.data_provider?.id],
      ["EXEC", plugin?.execution_provider?.id],
      ["CONTEXT", plugin?.context_provider?.id],
    ];
    elements.providerSection.replaceChildren();
    for (const [label, id] of providers) {
      const row = document.createElement("div");
      const key = document.createElement("span");
      key.textContent = label;
      const value = document.createElement("code");
      value.textContent = id || "unregistered";
      row.append(key, value);
      elements.providerSection.appendChild(row);
    }
  }

  function renderRuns() {
    elements.runs.replaceChildren();
    elements.operationsLabel.textContent = state.workspace?.domain?.workbench?.bottom_panel_label || "Agent runs & evidence";
    const taskRows = state.tasks.map((task) => ({
      id: task.id,
      taskId: task.id,
      label: task.intent_label || task.intent_id || "Domain task",
      agent: task.agent || "domain",
      asset: task.artifacts?.[0]?.path || task.asset_path || "workspace scope",
      startedAt: Date.parse(task.created_at || task.updated_at || "") || Date.now(),
      status: task.status || "planning",
      log: task.note || "",
      kind: "agent-task",
    }));
    const rows = [...taskRows, ...state.runLedger.filter((run) => !taskRows.some((task) => task.id === run.id))];
    if (!rows.length) {
      const empty = document.createElement("div");
      empty.className = "research-domain-runs-empty";
      empty.textContent = "No domain operation has been dispatched. Select a native operation or a runbook stage to create an evidence-backed Agent run.";
      elements.runs.appendChild(empty);
      return;
    }
    rows.slice(0, 8).forEach((run) => {
      const row = document.createElement("div");
      row.className = `research-domain-run is-${run.status || "queued"}`;
      const status = document.createElement("i");
      const copy = document.createElement("span");
      copy.className = "research-domain-run-copy";
      const title = document.createElement("strong");
      title.textContent = run.label;
      const detail = document.createElement("span");
      detail.textContent = `${formatRole(run.agent)} Agent · ${run.asset || "workspace scope"}`;
      copy.append(title, detail);
      const meta = document.createElement("span");
      meta.className = "research-domain-run-meta";
      meta.textContent = `${run.status || "queued"} · ${new Date(run.startedAt).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}`;
      row.append(status, copy, meta);
      if (run.agent === "native-sdk") {
        detail.textContent = `${run.sdk || "Native SDK"} · ${run.asset || "workspace scope"}`;
      }
      if (run.log) {
        row.title = run.log.slice(-4000);
        row.addEventListener("click", () => {
          state.actionLog = run.log;
          renderTabSurface();
        });
      }
      elements.runs.appendChild(row);
    });
  }

  function dispatchAgentAction({ tool = null, stage = null, preferredAgent = "" } = {}) {
    const action = tool || stage;
    if (!action) return;
    const candidate = intentEntries().find((entry) => tool?.id && entry?.contract?.recommended_actions?.includes(tool.id))
      || intentEntries().find((entry) => preferredAgent && entry?.contract?.agent === preferredAgent)
      || preferredIntentEntry({ stage });
    openDomainTaskDialog({
      intentId: candidate?.contract?.id || "",
      initialPrompt: stage
        ? `${stage.label}: ${stage.description || ""}\nGate: ${stage.gate || "verification evidence required"}`
        : `${action.label || action.id}: ${action.description || ""}`,
      stage,
    });
  }

  function renderWorkspace() {
    const plugin = state.workspace?.domain || pluginById(state.domainId);
    const assets = state.workspace?.assets || [];
    const spec = domainSpec();
    elements.kicker.textContent = spec.studio;
    elements.title.textContent = plugin?.metadata?.label || "Research Domain";
    elements.workspace.dataset.domainLayout = spec.layout;
    elements.workspace.dataset.domainId = state.domainId;
    elements.workspace.dataset.activeTab = state.activeTab;
    elements.resourcesLabel.textContent = `${spec.dataObjects[0] || "Research"} Resources`;
    elements.primaryKicker.textContent = spec.studio;
    elements.primaryLabel.textContent = domainTabDescriptor(state.activeTab, spec).label;
    elements.toolbarMeta.textContent = `${spec.runtime} · ${spec.selectionModel} · ${assets.length} live objects · ${String(state.workspace?.revision || "").slice(0, 10)}`;
    elements.assetCount.textContent = assets.length ? String(assets.length) : "";
    elements.openAsset.disabled = !selectedAsset()?.path;
    state.objectType = String(workspaceState()?.filters?.object_type || state.objectType || "all");
    renderGlobalTabs();
    renderObjectTypes();
    renderAssets(assets);
    renderNativeToolbar();
    renderAgents(plugin);
    renderAdapters(plugin);
    renderLiveContext();
    renderRuns();
    renderViewerTabs();
    renderTabSurface();
  }

  function renderViewerTabs() {
    const asset = selectedAsset();
    const visualizations = asset?.visualizations || [];
    elements.viewerToolbar.hidden = !visualizations.length;
    elements.viewerTabs.replaceChildren();
    for (const visualization of visualizations) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = `research-domain-viewer-tab${visualization.id === state.visualizationId ? " is-active" : ""}`;
      button.setAttribute("role", "tab");
      button.setAttribute("aria-selected", visualization.id === state.visualizationId ? "true" : "false");
      button.textContent = visualization.label;
      button.addEventListener("click", async () => {
        if (visualization.id === state.visualizationId) return;
        state.visualizationId = visualization.id;
        state.document = null;
        renderViewerTabs();
        renderPreview();
        await loadVisualization();
      });
      elements.viewerTabs.appendChild(button);
    }
    elements.openVisualization.disabled = !state.document;
  }

  async function fetchVisualization(domainId = state.domainId, assetId = state.assetId, visualizationId = state.visualizationId) {
    const params = new URLSearchParams({ domain_id: domainId, asset_id: assetId });
    if (visualizationId) params.set("visualization_id", visualizationId);
    return requestJson(`/api/research-domains/visualization?${params}`);
  }

  async function loadVisualization({ quiet = false } = {}) {
    if (!state.domainId || !state.assetId || !state.visualizationId) return;
    const requestId = ++state.requestId;
    if (!quiet) elements.empty.textContent = "Parsing real domain data…";
    try {
      const documentData = await fetchVisualization();
      if (requestId !== state.requestId) return;
      state.document = documentData;
      renderPreview();
      if (state.activeTab === "preview") renderPreviewTab();
      else if (state.activeTab !== "visualization") renderTabSurface();
      elements.openVisualization.disabled = false;
    } catch (error) {
      if (requestId !== state.requestId) return;
      state.document = null;
      renderPreview(error?.message || "Unable to parse this domain asset.");
    }
  }

  function renderPreview(error = "") {
    state.viewer3d?.dispose?.();
    state.viewer3d = null;
    if (elements.canvas3d) elements.canvas3d.hidden = true;
    elements.canvas.hidden = false;
    elements.canvas.replaceChildren();
    elements.inspector.hidden = true;
    elements.inspector.replaceChildren();
    const documentData = state.document;
    if (!documentData) {
      elements.empty.textContent = error || (state.assetId ? "" : "No domain data to display.");
      return;
    }
    const nodes = (documentData.nodes || []).filter((node) => node?.id).slice(0, 80);
    const nodeIds = new Set(nodes.map((node) => node.id));
    const edges = (documentData.edges || []).filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)).slice(0, 140);
    const series = (documentData.series || []).filter((item) => (item.points || []).length).slice(0, 5);
    const geometry = documentData?.metadata?.geometry;
    if (Array.isArray(geometry?.points) && geometry.points.length) {
      if (elements.canvas3d && window.AtlasDomain3D) {
        elements.canvas.hidden = true;
        elements.canvas3d.hidden = false;
        state.viewer3d = window.AtlasDomain3D.mount(elements.canvas3d, geometry, {
          domainId: state.domainId,
          visualizationId: state.visualizationId,
        });
      } else {
        renderGeometry(elements.canvas, geometry, { width: 960, height: 600 });
      }
    } else {
      renderWorkbenchView(documentData, nodes, edges, series);
    }
    const hasData = nodes.length || series.length || (geometry?.points || []).length;
    elements.empty.textContent = hasData ? "" : error || "";
  }

  function renderWorkbenchView(documentData, nodes, edges, series) {
    const renderer = String(documentData?.metadata?.renderer || selectedVisualization()?.renderer || "graph");
    const visualizationId = String(documentData?.metadata?.visualization_id || state.visualizationId || "");
    elements.stage.dataset.renderer = renderer;
    elements.stage.dataset.visualizationId = visualizationId;
    if (["chart", "heatmap", "timeline", "trace", "tensor"].includes(renderer) && series.length) {
      renderSeries(elements.canvas, series, { width: 960, height: nodes.length ? 300 : 540, top: 28 });
      if (nodes.length && !["heatmap", "tensor"].includes(renderer)) {
        renderGraph(elements.canvas, nodes, edges, { width: 960, height: 270, top: 315, interactive: true });
      }
      return;
    }
    if (renderer === "table" || documentData?.metadata?.table) {
      renderTable(elements.canvas, documentData?.metadata?.table, nodes, { width: 960, height: 600 });
      return;
    }
    if (renderer === "code" || renderer === "equation") {
      renderCode(elements.canvas, documentData, nodes, { width: 960, height: 600, equation: renderer === "equation" });
      return;
    }
    if (series.length) renderSeries(elements.canvas, series, { width: 960, height: nodes.length ? 230 : 540, top: 28 });
    if (nodes.length) renderGraph(elements.canvas, nodes, edges, { width: 960, height: series.length ? 340 : 560, top: series.length ? 250 : 20, interactive: true });
  }

  function renderTable(target, table, nodes, options) {
    const rows = Array.isArray(table?.rows) ? table.rows.slice(0, 16) : [];
    const columns = Array.isArray(table?.columns) && table.columns.length
      ? table.columns.slice(0, 8)
      : Array.from(new Set(rows.flatMap((row) => Object.keys(row || {})))).slice(0, 8);
    if (!rows.length && nodes.length) {
      rows.push(...nodes.slice(0, 16).map((node) => ({ Name: node.label, Type: node.category, Status: node.status || "" })));
      columns.push(...["Name", "Type", "Status"].filter((column) => !columns.includes(column)));
    }
    if (!columns.length) return;
    const left = 26;
    const top = 34;
    const rowHeight = 30;
    const columnWidth = (options.width - left * 2) / columns.length;
    target.appendChild(svg("rect", { class: "research-domain-table-head", x: left, y: top, width: options.width - left * 2, height: rowHeight, rx: 6 }));
    columns.forEach((column, index) => {
      const label = svg("text", { class: "research-domain-table-label", x: left + index * columnWidth + 9, y: top + 20 });
      label.textContent = truncate(column, 18);
      target.appendChild(label);
    });
    rows.forEach((row, rowIndex) => {
      const y = top + rowHeight * (rowIndex + 1);
      target.appendChild(svg("rect", { class: "research-domain-table-row", x: left, y, width: options.width - left * 2, height: rowHeight }));
      columns.forEach((column, columnIndex) => {
        const value = svg("text", { class: "research-domain-table-value", x: left + columnIndex * columnWidth + 9, y: y + 20 });
        const raw = row?.[column];
        value.textContent = truncate(typeof raw === "object" ? JSON.stringify(raw) : raw ?? "", 22);
        target.appendChild(value);
      });
    });
  }

  function renderCode(target, documentData, nodes, options) {
    const source = String(documentData?.metadata?.source?.text || "");
    const equations = options.equation
      ? nodes.filter((node) => node.category === "equation").map((node) => `${node.label} = ${node.metadata?.expression || ""}`)
      : [];
    const lines = (equations.length ? equations : source.split(/\r?\n/)).slice(0, 34);
    const gutter = 54;
    target.appendChild(svg("rect", { class: "research-domain-code-surface", x: 20, y: 20, width: options.width - 40, height: options.height - 40, rx: 7 }));
    lines.forEach((line, index) => {
      const y = 48 + index * 15.5;
      const lineNumber = svg("text", { class: "research-domain-code-number", x: gutter, y, "text-anchor": "end" });
      lineNumber.textContent = String(index + 1);
      const code = svg("text", { class: "research-domain-code-line", x: gutter + 16, y });
      code.textContent = truncate(line, 112);
      target.append(lineNumber, code);
    });
  }

  function graphPositions(nodes, edges, width, height, top = 0) {
    const ids = new Set(nodes.map((node) => node.id));
    const incoming = new Map(nodes.map((node) => [node.id, 0]));
    const outgoing = new Map(nodes.map((node) => [node.id, []]));
    edges.forEach((edge) => {
      if (!ids.has(edge.source) || !ids.has(edge.target)) return;
      incoming.set(edge.target, (incoming.get(edge.target) || 0) + 1);
      outgoing.get(edge.source)?.push(edge.target);
    });
    const levels = new Map();
    const queue = nodes.filter((node) => !incoming.get(node.id)).map((node) => node.id);
    if (!queue.length && nodes.length) queue.push(nodes[0].id);
    queue.forEach((id) => levels.set(id, 0));
    for (let cursor = 0; cursor < queue.length; cursor += 1) {
      const id = queue[cursor];
      for (const target of outgoing.get(id) || []) {
        const next = Math.max(levels.get(target) || 0, (levels.get(id) || 0) + 1);
        levels.set(target, next);
        incoming.set(target, Math.max(0, (incoming.get(target) || 0) - 1));
        if (!incoming.get(target)) queue.push(target);
      }
    }
    nodes.forEach((node, index) => { if (!levels.has(node.id)) levels.set(node.id, index % 5); });
    const columns = new Map();
    nodes.forEach((node) => {
      const level = levels.get(node.id) || 0;
      if (!columns.has(level)) columns.set(level, []);
      columns.get(level).push(node);
    });
    const ordered = Array.from(columns.keys()).sort((a, b) => a - b);
    const positions = new Map();
    ordered.forEach((level, columnIndex) => {
      const column = columns.get(level);
      const x = ordered.length === 1 ? width / 2 : 80 + columnIndex * ((width - 160) / Math.max(1, ordered.length - 1));
      column.forEach((node, rowIndex) => {
        const y = top + 48 + ((rowIndex + 1) * (height - 88)) / (column.length + 1);
        positions.set(node.id, { x, y });
      });
    });
    return positions;
  }

  function renderGraph(target, nodes, edges, options) {
    const positions = graphPositions(nodes, edges, options.width, options.height, options.top || 0);
    edges.forEach((edge) => {
      const source = positions.get(edge.source);
      const destination = positions.get(edge.target);
      if (!source || !destination) return;
      const path = svg("path", {
        class: "research-domain-preview-edge",
        "data-preview-edge-id": edge.id || `${edge.source}->${edge.target}`,
        d: `M${source.x},${source.y} C${source.x + 54},${source.y} ${destination.x - 54},${destination.y} ${destination.x},${destination.y}`,
      });
      target.appendChild(path);
    });
    nodes.forEach((node) => {
      const position = positions.get(node.id);
      const group = svg("g", {
        class: "research-domain-preview-node",
        "data-preview-node-id": node.id,
        transform: `translate(${position.x},${position.y})`,
      });
      group.appendChild(svg("rect", { x: -68, y: -23, width: 136, height: 46, rx: 7 }));
      const label = svg("text", { x: 0, y: -2, "text-anchor": "middle" });
      label.textContent = truncate(node.label || node.id, 20);
      const meta = svg("text", { class: "research-domain-preview-node-meta", x: 0, y: 13, "text-anchor": "middle" });
      meta.textContent = truncate(node.category || "node", 24);
      group.append(label, meta);
      if (options.interactive) {
        group.style.cursor = "pointer";
        group.addEventListener("click", () => showInspector(node));
      }
      target.appendChild(group);
    });
    return positions;
  }

  function renderSeries(target, series, options) {
    const all = series.flatMap((item) => item.points || []);
    if (!all.length) return;
    const left = 54;
    const right = options.width - 30;
    const top = options.top || 20;
    const bottom = top + options.height - 28;
    const minTime = Math.min(...all.map((point) => Number(point.timestamp_ms) || 0));
    const maxTime = Math.max(...all.map((point) => Number(point.timestamp_ms) || 0));
    let minValue = Math.min(...all.map((point) => Number(point.value) || 0));
    let maxValue = Math.max(...all.map((point) => Number(point.value) || 0));
    if (minValue === maxValue) { minValue -= 1; maxValue += 1; }
    const x = (value) => left + ((Number(value) - minTime) / Math.max(1, maxTime - minTime)) * (right - left);
    const y = (value) => bottom - ((Number(value) - minValue) / Math.max(1e-9, maxValue - minValue)) * (bottom - top);
    for (let index = 0; index <= 4; index += 1) {
      const yy = top + ((bottom - top) * index) / 4;
      target.appendChild(svg("line", { class: "research-domain-preview-series-grid", x1: left, x2: right, y1: yy, y2: yy }));
    }
    series.forEach((item, index) => {
      const path = svg("path", {
        class: "research-domain-preview-series-line",
        d: (item.points || []).map((point, pointIndex) => `${pointIndex ? "L" : "M"}${x(point.timestamp_ms).toFixed(2)},${y(point.value).toFixed(2)}`).join(" "),
      });
      path.style.stroke = index % 2 ? "var(--green)" : "var(--accent)";
      target.appendChild(path);
    });
  }

  function renderGeometry(target, geometry, options) {
    const rawPoints = geometry.points.slice(0, 10000);
    const projected = rawPoints.map((point) => ({
      x: (Number(point?.[0]) || 0) - (Number(point?.[2]) || 0) * 0.42,
      y: (Number(point?.[1]) || 0) + (Number(point?.[2]) || 0) * 0.24,
    }));
    const minX = Math.min(...projected.map((point) => point.x));
    const maxX = Math.max(...projected.map((point) => point.x));
    const minY = Math.min(...projected.map((point) => point.y));
    const maxY = Math.max(...projected.map((point) => point.y));
    const scale = Math.max(0.001, Math.min((options.width - 100) / Math.max(1e-9, maxX - minX), (options.height - 80) / Math.max(1e-9, maxY - minY)));
    const points = projected.map((point) => ({ x: 50 + (point.x - minX) * scale, y: 40 + (maxY - point.y) * scale }));
    const faces = Array.isArray(geometry.faces) ? geometry.faces.slice(0, 4000) : [];
    if (faces.length) {
      faces.forEach((face) => {
        const coordinates = (face || []).map((index) => points[Number(index)]).filter(Boolean);
        if (coordinates.length < 2) return;
        target.appendChild(svg("path", {
          class: "research-domain-preview-edge",
          d: `${coordinates.map((point, index) => `${index ? "L" : "M"}${point.x},${point.y}`).join(" ")} Z`,
        }));
      });
    } else {
      points.slice(0, 5000).forEach((point) => target.appendChild(svg("circle", { cx: point.x, cy: point.y, r: 1.5, fill: "var(--accent)" })));
    }
  }

  function showInspector(node) {
    elements.inspector.replaceChildren();
    const title = document.createElement("strong");
    title.textContent = node.label || node.id;
    const list = document.createElement("dl");
    const values = { Category: node.category, Status: node.status, ...node.metrics, ...node.metadata };
    Object.entries(values).filter(([, value]) => value !== null && value !== undefined && value !== "").slice(0, 40).forEach(([key, value]) => {
      const term = document.createElement("dt");
      term.textContent = key;
      const detail = document.createElement("dd");
      detail.textContent = typeof value === "object" ? JSON.stringify(value) : String(value);
      list.append(term, detail);
    });
    elements.inspector.append(title, list);
    elements.inspector.hidden = false;
  }

  function truncate(value, length) {
    const text = String(value || "");
    return text.length > length ? `${text.slice(0, length - 1)}…` : text;
  }

  function schedulePolling() {
    stopPolling();
    if (!state.active || !state.domainId) return;
    state.pollTimer = window.setTimeout(async () => {
      try {
        const previousRevision = state.workspace?.revision || "";
        const params = new URLSearchParams({ domain_id: state.domainId });
        const workspace = await requestJson(`/api/research-domains/workspace?${params}`);
        if (workspace?.revision !== previousRevision) {
          const previousAsset = state.assetId;
          state.workspace = workspace;
          const shared = workspace?.state || {};
          const sharedAssetId = String(shared.active_asset_id || "");
          const agentUpdated = String(shared.updated_by || "") === "agent";
          const sharedTaskPath = String(shared?.active_task?.artifacts?.[0]?.path || shared?.ui?.highlight_output_path || shared?.focus || "");
          const taskAsset = (workspace?.assets || []).find((asset) => normalizePath(asset.path) === normalizePath(sharedTaskPath));
          state.assetId = agentUpdated && (workspace?.assets || []).some((asset) => asset.id === sharedAssetId)
            ? sharedAssetId
            : agentUpdated && taskAsset
              ? taskAsset.id
            : (workspace?.assets || []).some((asset) => asset.id === previousAsset)
              ? previousAsset
            : workspace?.assets?.[0]?.id || "";
          if (agentUpdated && domainSpec().tabs.includes(String(shared.active_tab || ""))) {
            state.activeTab = String(shared.active_tab);
          }
          if (agentUpdated && sharedAssetId) state.highlightAssetId = sharedAssetId;
          else if (agentUpdated && taskAsset) state.highlightAssetId = taskAsset.id;
          if (agentUpdated && shared.selected_agent) state.selectedAgent = String(shared.selected_agent);
          const selected = selectedAsset();
          const sharedVisualizationId = String(shared.active_visualization_id || "");
          state.visualizationId = agentUpdated && selected?.visualizations?.some((item) => item.id === sharedVisualizationId)
            ? sharedVisualizationId
            : selected?.visualizations?.some((item) => item.id === state.visualizationId)
              ? state.visualizationId
            : selected?.visualizations?.[0]?.id || "";
          await loadDomainTasks({ quiet: true });
          renderWorkspace();
          if (state.assetId && state.visualizationId) await loadVisualization({ quiet: true });
          else renderPreview();
        }
        schedulePolling();
      } catch (_error) {
        schedulePolling();
      }
    }, 1500);
  }

  function stopPolling() {
    if (state.pollTimer) window.clearTimeout(state.pollTimer);
    state.pollTimer = null;
  }

  async function open(domainId, assetId = "", tab = "", options = {}) {
    state.active = true;
    await ensureCatalog();
    if (!pluginById(domainId)) throw new Error("Research domain plugin is unavailable.");
    if (state.pendingStatePatch && state.pendingStateDomain && state.pendingStateDomain !== domainId) {
      window.clearTimeout(state.stateSyncTimer);
      const pending = state.pendingStatePatch;
      const pendingDomain = state.pendingStateDomain;
      state.pendingStatePatch = null;
      state.pendingStateDomain = "";
      await persistWorkspaceState(pending, { domainId: pendingDomain });
    }
    if (state.domainId && state.domainId !== domainId) {
      state.runLedger = [];
      state.runningAction = null;
      state.actions = [];
      state.taskCatalog = null;
      state.tasks = [];
      state.actionLog = "";
      state.selectedAgent = "";
      state.assetQuery = "";
      if (elements.assetFilter) elements.assetFilter.value = "";
    }
    state.domainId = domainId;
    if (domainSpec(domainId).tabs.includes(tab)) state.activeTab = tab;
    if (assetId) {
      state.assetId = assetId;
      if (options.highlight !== false) {
        state.highlightAssetId = assetId;
        state.locateAssetId = assetId;
      }
    }
    state.workspace = null;
    state.document = null;
    renderNav();
    await loadWorkspace(domainId, {
      preserveAsset: Boolean(assetId),
      highlightAssetId: options.highlight === false ? "" : assetId,
      requestedTab: tab,
    });
    const asset = selectedAsset();
    if (assetId && asset?.id === assetId) {
      await persistWorkspaceState({
        active_asset_id: asset.id,
        active_visualization_id: state.visualizationId,
        active_tab: state.activeTab,
        focus: asset.path || asset.id,
        ui: {
          highlight_asset_id: options.highlight === false ? "" : asset.id,
          highlight_output_path: asset.path || "",
        },
      });
    }
  }

  async function activate() {
    state.active = true;
    await ensureCatalog();
    if (!state.domainId) return;
    if (!state.workspace || state.workspace?.domain?.metadata?.id !== state.domainId) {
      await loadWorkspace(state.domainId, { preserveAsset: true });
    } else {
      renderWorkspace();
      renderPreview();
      schedulePolling();
    }
  }

  async function openArtifact(path, options = {}) {
    const artifactPath = String(path || "").trim();
    if (!artifactPath) return false;
    const normalized = normalizePath(artifactPath);
    const pathDomain = normalized.match(/^\.atlas\/domain-(?:actions|tasks)\/([^/]+)\//)?.[1] || "";
    const domainId = String(options.domainId || pathDomain).trim();
    const params = new URLSearchParams({
      query: [String(options.query || "").trim(), artifactPath].filter(Boolean).join(" ").slice(0, 4000),
    });
    if (domainId) params.set("domain_id", domainId);
    let contextSnapshot;
    try {
      contextSnapshot = await requestJson(`/api/research-domains/context?${params}`);
    } catch (_error) {
      return false;
    }
    const asset = (contextSnapshot?.assets || []).find((candidate) => {
      const candidatePath = normalizePath(candidate?.path);
      return normalized === candidatePath
        || normalized.endsWith(`/${candidatePath}`)
        || candidatePath.endsWith(`/${normalized}`);
    });
    if (!asset?.domain_id || !asset?.id) return false;
    const requestedTab = String(options.tab || "").trim();
    const tab = requestedTab || (asset.visualizations?.length ? "visualization" : "artifacts");
    await open(asset.domain_id, asset.id, tab, { highlight: options.highlight !== false });
    return true;
  }

  function deactivate() {
    state.active = false;
    state.viewer3d?.dispose?.();
    state.viewer3d = null;
    state.workbenchViewer3d?.dispose?.();
    state.workbenchViewer3d = null;
    stopPolling();
  }

  function normalizePath(value) {
    return String(value || "").replace(/\\/g, "/").replace(/^\.\//, "").toLowerCase();
  }

  function parsedToolPayload(value) {
    if (!value) return null;
    if (typeof value === "object") return value;
    if (typeof value !== "string") return null;
    try { return JSON.parse(value); } catch (_error) { return null; }
  }

  function appendArtifactPath(paths, value) {
    if (typeof value !== "string" || /^(?:data:|https?:\/\/)/i.test(value.trim())) return;
    const path = normalizePath(value);
    if (path) paths.push(path);
  }

  function appendPayloadArtifacts(paths, payload, { includeGenericPath = false } = {}) {
    if (!payload || typeof payload !== "object") return;
    if (includeGenericPath) [payload.path, payload.file_path].forEach((value) => appendArtifactPath(paths, value));
    [payload.output_path, payload.saved_path, payload.result_path]
      .forEach((value) => appendArtifactPath(paths, value));
    const containers = [
      payload.artifacts,
      payload.outputs,
      payload.task?.artifacts,
      payload.workspace_state?.active_task?.artifacts,
      payload.workspace_state?.artifacts,
    ];
    containers.forEach((items) => {
      if (!Array.isArray(items)) return;
      items.forEach((item) => appendArtifactPath(paths, typeof item === "string" ? item : item?.path));
    });
    appendArtifactPath(paths, payload.task?.result_path);
    appendArtifactPath(paths, payload.workspace_state?.last_run?.output_path);
  }

  function editedPaths(turn) {
    const paths = (turn?.diffs || []).map((diff) => normalizePath(diff?.path)).filter(Boolean);
    for (const tool of turn?.tools || []) {
      const toolName = String(tool?.name || "");
      const result = parsedToolPayload(tool?.result);
      const writesArtifact = /^(?:research_domain_action|research_domain_task|write_file|edit_file|apply_patch|create_file|generate_image|render|export|save)/i.test(toolName);
      appendPayloadArtifacts(paths, result, { includeGenericPath: writesArtifact });
      if (writesArtifact) {
        appendPayloadArtifacts(paths, parsedToolPayload(tool?.args), { includeGenericPath: true });
      }
    }
    return [...new Set(paths)];
  }

  function preservedInteractivePreviewKind(turn) {
    for (const tool of turn?.tools || []) {
      const args = parsedToolPayload(tool?.args);
      const result = parsedToolPayload(tool?.result);
      const candidates = [
        args?.kind,
        args?.preview_kind,
        args?.visualization_kind,
        result?.kind,
        result?.preview_kind,
        result?.visualization_kind,
        result?.task?.preview_kind,
        ...(Array.isArray(result?.task?.artifacts) ? result.task.artifacts.map((artifact) => artifact?.kind) : []),
      ];
      for (const candidate of candidates) {
        const kind = String(candidate || "").trim().toLowerCase().replace(/_/g, "-");
        if (kind === "paper" || kind === "multi-agent") return kind;
      }
    }
    return "";
  }

  function actionDomainFromTurn(turn) {
    for (const tool of turn?.tools || []) {
      if (!["research_domain_action", "research_domain_task"].includes(String(tool?.name || ""))) continue;
      const args = parsedToolPayload(tool?.args);
      const result = parsedToolPayload(tool?.result);
      const domainId = [
        args?.domain_id,
        result?.domain_id,
        result?.task?.domain_id,
        result?.workspace_state?.domain_id,
      ].map((value) => String(value || "").trim()).find(Boolean);
      if (domainId) return domainId;
    }
    return "";
  }

  function validationCompleted(turn) {
    const tools = (turn?.tools || []).filter((tool) => {
      const status = String(tool?.status || "").toLowerCase();
      return tool?.success !== false && !["failed", "error", "denied", "cancelled"].includes(status);
    });
    if ((turn?.diffs || []).length && tools.some((tool) => /^(?:write_file|edit_file|apply_patch|create_file)$/.test(String(tool?.name || "").toLowerCase()))) return true;
    if (tools.some((tool) => String(tool?.name || "") === "research_domain_task" && /"status"\s*:\s*"completed"/i.test(JSON.stringify(tool?.args || {})))) return true;
    return tools.some((tool) => /build|check|test|run|execute/i.test(`${tool?.name || ""} ${JSON.stringify(tool?.args || {})}`));
  }

  function removePreviewCards(host) {
    host.querySelectorAll("[data-research-preview-card]").forEach((card) => {
      [card, ...card.querySelectorAll(".research-domain-preview-card")].forEach((target) => {
        const timer = state.previewTimers.get(target);
        if (timer) window.clearInterval(timer);
        state.previewTimers.delete(target);
      });
      card.remove();
    });
  }

  function renderPreviewEvidence(svgHost, documentData, spec) {
    const nodes = (documentData?.nodes || []).slice(0, 20);
    const nodeIds = new Set(nodes.map((node) => node.id));
    const edges = (documentData?.edges || [])
      .filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target))
      .slice(0, 32);
    const series = (documentData?.series || []).filter((item) => (item.points || []).length).slice(0, 3);
    const geometry = documentData?.metadata?.geometry;
    const layout = spec.layout;
    if (Array.isArray(geometry?.points) && geometry.points.length) {
      renderGeometry(svgHost, geometry, { width: 960, height: 280 });
      return null;
    }
    if (series.length && /experiment|system|compute|scientific|network|distributed/i.test(layout)) {
      renderSeries(svgHost, series, { width: 960, height: 245, top: 18 });
      return null;
    }
    if ((documentData?.metadata?.table?.rows || []).length && /database|vision|security/i.test(layout)) {
      renderTable(svgHost, documentData.metadata.table, nodes, { width: 960, height: 280 });
      return null;
    }
    if (nodes.length) return renderGraph(svgHost, nodes, edges, { width: 960, height: 270, top: 0, interactive: false });
    if (series.length) {
      renderSeries(svgHost, series, { width: 960, height: 245, top: 18 });
      return null;
    }
    const empty = svg("text", { class: "research-domain-mini-empty", x: 480, y: 142, "text-anchor": "middle" });
    empty.textContent = "No renderable evidence is registered for this artifact";
    svgHost.appendChild(empty);
    return null;
  }

  function buildDomainPreviewCard({ spec, asset, visualization, documentData, interactive = false, domainId = state.domainId }) {
    const card = document.createElement(interactive ? "button" : "div");
    if (interactive) card.type = "button";
    card.className = `research-preview-card research-domain-preview-card layout-${spec.layout} preview-${spec.preview}`;
    card.dataset.previewDomain = domainId;
    card.dataset.cardRoute = "research-domain";
    if (asset?.id) card.dataset.previewAsset = asset.id;
    const head = document.createElement("span");
    head.className = "research-preview-card-head";
    const identity = document.createElement("span");
    identity.className = "research-domain-preview-identity";
    identity.appendChild(domainIcon(domainId));
    const title = document.createElement("span");
    title.className = "research-preview-card-title";
    const strong = document.createElement("strong");
    strong.textContent = documentData?.title || asset?.name || `${spec.studio} result`;
    const subtitle = document.createElement("span");
    subtitle.textContent = `${spec.studio} · ${visualization?.label || spec.preview.replace(/-/g, " ")}`;
    title.append(strong, subtitle);
    identity.appendChild(title);
    const status = document.createElement("span");
    status.className = "research-preview-card-status";
    status.textContent = documentData ? "VERIFIED DATA" : "AWAITING EVIDENCE";
    head.append(identity, status);
    const preview = svg("svg", { viewBox: "0 0 960 280", role: "img", "aria-label": `${asset?.name || spec.studio} research preview` });
    const positions = renderPreviewEvidence(preview, documentData, spec);
    const summary = document.createElement("span");
    summary.className = "research-domain-preview-summary";
    const chips = [
      spec.dataObjects[0],
      visualization?.label || spec.visualizations[0],
      asset?.file_type ? String(asset.file_type).toUpperCase() : "NO ASSET",
      asset?.content_revision ? `rev ${String(asset.content_revision).slice(0, 8)}` : "state linked",
    ].filter(Boolean);
    chips.forEach((value) => {
      const chip = document.createElement("span");
      chip.textContent = value;
      summary.appendChild(chip);
    });
    const foot = document.createElement("span");
    foot.className = "research-preview-card-foot";
    const meta = document.createElement("span");
    meta.className = "research-preview-card-meta";
    meta.textContent = `${spec.reference} · ${new Date(documentData?.generated_at || Date.now()).toLocaleTimeString()}`;
    const action = document.createElement("span");
    action.className = "research-preview-card-action";
    action.textContent = "Open · locate · highlight →";
    foot.append(meta, action);
    card.append(head, preview, summary, foot);
    if (interactive && asset?.id) {
      card.addEventListener("click", () => {
        window.dispatchEvent(new CustomEvent("atlas:research-domain-open", {
          detail: { domainId, assetId: asset.id, tab: "visualization", highlight: true },
        }));
      });
    }
    if (positions && (documentData?.frames || []).length > 1 && !window.matchMedia?.("(prefers-reduced-motion: reduce)").matches) {
      let index = 0;
      const apply = () => {
        const frame = documentData.frames[index % documentData.frames.length] || {};
        const activeNodes = new Set(frame.active_nodes || []);
        const activeEdges = new Set(frame.active_edges || []);
        preview.querySelectorAll("[data-preview-node-id]").forEach((node) => node.classList.toggle("is-active", activeNodes.has(node.dataset.previewNodeId)));
        preview.querySelectorAll("[data-preview-edge-id]").forEach((edge) => edge.classList.toggle("is-active", activeEdges.has(edge.dataset.previewEdgeId)));
        index += 1;
      };
      apply();
      state.previewTimers.set(card, window.setInterval(apply, 950));
    }
    return card;
  }

  function renderConversationPreview(host, documentData, contextSnapshot, asset, visualization) {
    const wasNearBottom = host.scrollHeight - host.clientHeight - host.scrollTop < 80;
    const wrapper = document.createElement("div");
    wrapper.className = "codex-turn-anchor research-preview-entry";
    wrapper.dataset.researchPreviewCard = "true";
    const spec = domainSpec(asset.domain_id);
    const button = buildDomainPreviewCard({
      spec,
      asset,
      visualization,
      documentData,
      interactive: true,
      domainId: asset.domain_id,
    });
    button.setAttribute("aria-label", `Open ${contextSnapshot.plugin?.metadata?.label || "research"} workspace and locate ${asset.name}`);
    wrapper.appendChild(button);
    host.appendChild(wrapper);
    if (wasNearBottom) host.scrollTop = host.scrollHeight;
  }

  async function syncPreview({ host, turn, running = false } = {}) {
    if (state.runningAction && state.runningAction.agent !== "native-sdk") {
      const nextStatus = running
        ? "running"
        : turn
          ? ((turn?.tools || []).some((tool) => tool?.success === false || ["failed", "error", "denied", "cancelled"].includes(String(tool?.status || "").toLowerCase())) ? "attention" : "complete")
          : state.runningAction.status;
      if (nextStatus !== state.runningAction.status) {
        state.runningAction.status = nextStatus;
        if (!running && turn) state.runningAction = null;
        renderRuns();
      }
    }
    if (!(host instanceof HTMLElement)) return false;
    const generation = ++state.previewGeneration;
    removePreviewCards(host);
    if (preservedInteractivePreviewKind(turn)) return false;
    const paths = editedPaths(turn);
    if (running || !paths.length || !validationCompleted(turn)) return false;
    const query = [turn?.text || "", ...paths].join(" ").slice(0, 4000);
    const params = new URLSearchParams({ query });
    const actionDomain = actionDomainFromTurn(turn);
    if (actionDomain) params.set("domain_id", actionDomain);
    let contextSnapshot;
    const attempts = actionDomain || paths.some((path) => path.startsWith(".atlas/domain-")) ? 5 : 1;
    for (let attempt = 0; attempt < attempts; attempt += 1) {
      try {
        contextSnapshot = await requestJson(`/api/research-domains/context?${params}`);
      } catch (_error) {
        return false;
      }
      const found = (contextSnapshot?.assets || []).some((candidate) => {
        const assetPath = normalizePath(candidate.path);
        return paths.some((path) => path === assetPath || path.endsWith(`/${assetPath}`) || assetPath.endsWith(`/${path}`));
      });
      if (found || attempt === attempts - 1) break;
      await new Promise((resolve) => window.setTimeout(resolve, 700));
    }
    if (generation !== state.previewGeneration) return false;
    const asset = (contextSnapshot?.assets || []).find((candidate) => {
      const assetPath = normalizePath(candidate.path);
      return paths.some((path) => path === assetPath || path.endsWith(`/${assetPath}`) || assetPath.endsWith(`/${path}`));
    });
    if (!asset?.visualizations?.length) return false;
    const visualization = asset.visualizations[0];
    const documentData = await fetchVisualization(asset.domain_id, asset.id, visualization.id).catch(() => null);
    if (generation !== state.previewGeneration || !documentData) return false;
    const hasRenderable = (documentData.nodes || []).length
      || (documentData.series || []).some((item) => (item.points || []).length)
      || (documentData?.metadata?.geometry?.points || []).length;
    if (!hasRenderable) return false;
    renderConversationPreview(host, documentData, contextSnapshot, asset, visualization);
    return true;
  }

  elements.refresh?.addEventListener("click", async () => {
    await ensureCatalog({ force: true });
    if (state.domainId) await loadWorkspace(state.domainId, { preserveAsset: true });
  });
  elements.assetFilter?.addEventListener("input", () => {
    state.assetQuery = elements.assetFilter.value || "";
    renderAssets(state.workspace?.assets || []);
    queueWorkspaceState({ filters: { query: state.assetQuery } });
  });
  elements.openAsset?.addEventListener("click", () => {
    const asset = selectedAsset();
    if (!asset?.path) return;
    window.dispatchEvent(new CustomEvent("atlas:domain-open-asset", { detail: { path: asset.path } }));
  });
  elements.clearRuns?.addEventListener("click", () => {
    state.runLedger = state.runningAction ? [state.runningAction] : [];
    renderRuns();
  });
  elements.close?.addEventListener("click", () => window.dispatchEvent(new CustomEvent("atlas:research-domain-close")));
  elements.openVisualization?.addEventListener("click", () => {
    if (!state.document) return;
    const plugin = state.workspace?.domain;
    const asset = selectedAsset();
    const visualization = selectedVisualization();
    const domainId = state.domainId;
    const assetId = state.assetId;
    const visualizationId = state.visualizationId;
    window.dispatchEvent(new CustomEvent("atlas:visualization-document-open", {
      detail: {
        document: state.document,
        presentation: {
          domainId,
          assetId,
          visualizationId,
          label: plugin?.metadata?.label || "Research Domain",
          reload: () => fetchVisualization(domainId, assetId, visualizationId),
          asset,
          visualization,
        },
      },
    }));
  });

  elements.app?.classList.add("has-research-domains");
  ensureCatalog().catch((error) => {
    elements.sidebar.setAttribute("title", error?.message || "Research Domains unavailable");
  });

  window.AtlasResearchDomains = Object.freeze({
    activate,
    deactivate,
    open,
    openArtifact,
    refresh: () => state.domainId ? loadWorkspace(state.domainId, { preserveAsset: true }) : ensureCatalog({ force: true }),
    syncPreview,
    getCatalog: () => state.catalog,
    getWorkspace: () => state.workspace,
  });
})();
