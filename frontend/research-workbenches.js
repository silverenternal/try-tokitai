(function initializeAtlasResearchWorkbenches() {
  "use strict";

  const WORKBENCHES = Object.freeze({
    "ai-ml": {
      id: "ml-experiment-console", signature: "EXPERIMENT / RUN / REGISTRY", mode: "metrics",
      primary: "Experiment & run matrix", secondary: "Metrics / version comparison", inspector: "Run / checkpoint / registry", dock: "Experiment → deployment lineage",
      columns: "260px minmax(390px, 1fr) 250px", areas: '"primary secondary inspector" "dock dock inspector"',
      commands: ["Runs", "Metric board", "Checkpoints", "Registry versions", "Artifacts"],
    },
    "computer-vision": {
      id: "vision-annotation-console", signature: "MEDIA / ANNOTATION / FRAMES", mode: "image",
      primary: "Media bin", secondary: "Annotation canvas", inspector: "Classes & instances", dock: "Frame filmstrip",
      columns: "190px minmax(440px, 1fr) 230px", areas: '"primary secondary inspector" "dock dock inspector"',
      commands: ["Annotation canvas", "Media", "Mask overlay", "Tracks", "Compare GT"],
    },
    nlp: {
      id: "language-reasoning-console", signature: "PROMPT / TOKEN / RETRIEVAL", mode: "tokens",
      primary: "Prompt pipeline", secondary: "Token & retrieval trace", inspector: "Knowledge / memory", dock: "Reasoning evidence chain",
      columns: "minmax(270px,.75fr) minmax(390px,1.25fr) 240px", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["Token flow", "Prompt", "Retrieval", "Knowledge", "Memory"],
    },
    "computer-graphics": {
      id: "graphics-scene-console", signature: "SCENE / VIEWPORT / SHADER", mode: "viewport",
      primary: "Scene outliner", secondary: "Realtime 3D viewport", inspector: "Object / material properties", dock: "Shader graph & animation",
      columns: "190px minmax(450px,1fr) 250px", areas: '"primary secondary inspector" "primary dock dock"',
      commands: ["Viewport", "Wireframe", "Material", "Shader", "Animation"],
    },
    cad: {
      id: "parametric-cad-console", signature: "FEATURES / CONSTRAINTS / MODEL", mode: "cad",
      primary: "Feature history", secondary: "Parametric model viewport", inspector: "Dimensions & constraints", dock: "Sketch / assembly history",
      columns: "210px minmax(440px,1fr) 245px", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["Model", "Sketch", "Constraints", "Section", "Assembly"],
    },
    robotics: {
      id: "robot-physics-console", signature: "WORLD / PHYSICS / JOINT / SENSOR", mode: "robot",
      primary: "World & bodies", secondary: "Interactive physics viewport", inspector: "Joint / collision / sensor", dock: "Simulation & trajectory timeline",
      columns: "205px minmax(440px,1fr) 250px", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["Simulation", "Robot model", "Joints", "Sensors", "Trajectory"],
    },
    "computer-networks": {
      id: "packet-analysis-console", signature: "PACKETS / PROTOCOL / BYTES", mode: "packets",
      primary: "Packet list", secondary: "Protocol dissection", inspector: "Packet bytes", dock: "Conversations / expert info",
      columns: "minmax(370px,1.4fr) minmax(260px,1fr) 260px", areas: '"primary primary primary" "secondary inspector inspector" "dock dock dock"',
      commands: ["Protocol tree", "Packets", "Bytes", "Flows", "Topology"],
    },
    "operating-systems": {
      id: "system-performance-console", signature: "PROCESS / CPU / MEMORY / TRACE", mode: "timeline",
      primary: "Process & thread tree", secondary: "CPU / scheduler lanes", inspector: "Stack & memory details", dock: "System calls / I/O events",
      columns: "240px minmax(430px,1fr) 240px", areas: '"primary secondary inspector" "dock dock inspector"',
      commands: ["CPU lanes", "Processes", "Memory", "Syscalls", "I/O"],
    },
    compiler: {
      id: "compiler-explorer-console", signature: "SOURCE → AST → IR → ASM", mode: "compiler",
      primary: "Source & AST", secondary: "LLVM IR / CFG", inspector: "Assembly & diagnostics", dock: "Pass pipeline correlation",
      columns: "minmax(280px,1fr) minmax(280px,1fr) minmax(260px,.9fr)", areas: '"primary primary primary" "dock secondary inspector"',
      commands: ["AST", "Source", "LLVM IR", "Assembly", "Diagnostics"],
    },
    database: {
      id: "database-query-console", signature: "SCHEMA / SQL / RESULTS / PLAN", mode: "database",
      primary: "Connections & schema", secondary: "SQL editor / result grid", inspector: "Actual execution plan", dock: "Transactions / query history",
      columns: "220px minmax(440px,1fr) 265px", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["SQL / results", "Actual plan", "Schema", "Locks", "Storage"],
    },
    "software-engineering": {
      id: "repository-history-console", signature: "REFERENCE / COMMIT / DIFF / MERGE", mode: "repository",
      primary: "References & repository tree", secondary: "Commit DAG / revision diff", inspector: "Commit / hunk / conflict", dock: "Branch history & merge state",
      columns: "215px minmax(450px,1fr) 255px", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["Commit graph", "Revision diff", "Branches", "Merge state", "History"],
    },
    "program-analysis": {
      id: "semantic-query-console", signature: "CODE DATABASE / QUERY / PATH", mode: "semantic",
      primary: "Queries & code databases", secondary: "Semantic query / source", inspector: "Path / predicate inspector", dock: "Call graph & data-flow steps",
      columns: "210px minmax(390px,1fr) minmax(270px,.75fr)", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["Query console", "Results", "Call graph", "Data flow", "Security paths"],
    },
    "cyber-security": {
      id: "reverse-engineering-console", signature: "SYMBOL / DISASSEMBLY / DECOMPILER", mode: "reverse",
      primary: "Programs, symbols & functions", secondary: "Decompiler / disassembly", inspector: "Function / reference inspector", dock: "Function graph / memory map",
      columns: "225px minmax(440px,1fr) 255px", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["Decompiler", "Disassembly", "Function graph", "Xrefs", "Memory map"],
    },
    hpc: {
      id: "parallel-profile-console", signature: "JOB / RANK / GPU / COUNTERS", mode: "gpu",
      primary: "Jobs, nodes & ranks", secondary: "Synchronized CPU / GPU timeline", inspector: "Kernel & hardware counters", dock: "MPI communication / scaling",
      columns: "220px minmax(450px,1fr) 255px", areas: '"primary secondary inspector" "dock dock inspector"',
      commands: ["GPU timeline", "Jobs", "Ranks", "Kernels", "Counters"],
    },
    "distributed-systems": {
      id: "container-runtime-console", signature: "COMPOSE / CONTAINER / IMAGE / LOG", mode: "containers",
      primary: "Compose, containers & images", secondary: "Runtime topology / lifecycle", inspector: "Container / mount / health", dock: "Log streams & runtime events",
      columns: "215px minmax(430px,1fr) 250px", areas: '"primary secondary inspector" "primary dock dock"',
      commands: ["Runtime", "Containers", "Images", "Volumes", "Logs"],
    },
    "scientific-computing": {
      id: "scientific-visualization-console", signature: "PIPELINE / DATASET / FIELD / VIEW", mode: "scientific",
      primary: "Visualization pipeline & arrays", secondary: "3D volume / surface / slice", inspector: "Field / cell / transfer function", dock: "Simulation time & pipeline state",
      columns: "220px minmax(440px,1fr) 255px", areas: '"primary secondary inspector" "primary dock inspector"',
      commands: ["Volume", "Surface", "Slice", "Contour", "Streamline"],
    },
  });

  function el(tag, className = "", text = "") {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text) node.textContent = text;
    return node;
  }

  function assetRows(context, limit = 10) {
    const host = el("div", "atlas-wb-rows");
    const assets = context.assets.slice(0, limit);
    assets.forEach((asset, index) => {
      const row = el("button", `atlas-wb-row${asset.id === context.selectedAsset?.id ? " is-active" : ""}`);
      row.type = "button";
      row.append(el("code", "", String(index + 1).padStart(3, "0")));
      const copy = el("span");
      copy.append(el("strong", "", asset.name || asset.path), el("small", "", `${String(asset.file_type || "file").toUpperCase()} · ${asset.path}`));
      row.append(copy, el("small", "atlas-wb-row-rev", String(asset.content_revision || "").slice(0, 7)));
      row.addEventListener("click", () => context.selectAsset(asset));
      host.appendChild(row);
    });
    if (!assets.length) host.appendChild(el("div", "atlas-wb-empty", "No compatible research object is present in this workspace."));
    return host;
  }

  function objectTree(context, treeKind = "object") {
    const host = el("div", `atlas-wb-object-tree is-${treeKind}`);
    const nodes = (context.documentData?.nodes || []).slice(0, 180);
    const source = nodes.length
      ? nodes.map((node) => ({
        id: node.id,
        label: node.label || node.name || node.id,
        type: node.category || treeKind,
        parent: node.parent_id || "",
        node,
      }))
      : context.assets.slice(0, 80).map((asset) => ({
        id: asset.id,
        label: asset.name || asset.path,
        type: asset.file_type || treeKind,
        parent: "",
        asset,
      }));
    source.forEach((item, index) => {
      const row = el("button", item.id === context.selectedAsset?.id ? "is-active" : "");
      row.type = "button";
      const depth = item.parent ? Math.min(5, String(item.parent).split(/[/.:]/).filter(Boolean).length) : 0;
      row.style.setProperty("--tree-depth", depth);
      row.append(el("i", "", depth ? "└" : "▾"), el("strong", "", item.label), el("code", "", String(item.type).toUpperCase()));
      row.addEventListener("click", () => {
        if (item.asset) context.selectAsset(item.asset);
        else persistSelection(context, `${treeKind}-node`, item.id || index, { object_type: item.type });
      });
      host.appendChild(row);
    });
    if (!source.length) host.appendChild(el("div", "atlas-wb-empty", `No ${treeKind} hierarchy is available in the selected artifact.`));
    return host;
  }

  function persistSelection(context, kind, value, extra = {}) {
    context.updateUi?.({
      selection_kind: kind,
      selection_id: String(value ?? ""),
      selection_revision: context.selectedAsset?.content_revision || "",
      ...extra,
    });
  }

  const VIEWPORT_TOOLS = Object.freeze({
    image: [["select", "Select"], ["box", "Box"], ["polygon", "Polygon"], ["mask", "Mask"], ["track", "Track"]],
    viewport: [["select", "Select"], ["move", "Move"], ["rotate", "Rotate"], ["scale", "Scale"], ["camera", "Camera"]],
    wireframe: [["select", "Select"], ["vertex", "Vertex"], ["edge", "Edge"], ["face", "Face"], ["normal", "Normals"]],
    cad: [["select", "Select"], ["sketch", "Sketch"], ["dimension", "Dimension"], ["constraint", "Constraint"], ["section", "Section"]],
    robot: [["select", "Select"], ["pose", "2D Pose"], ["goal", "Motion Goal"], ["measure", "Measure"], ["collision", "Collision"]],
    scientific: [["probe", "Probe"], ["slice", "Slice"], ["contour", "Contour"], ["streamline", "Streamline"], ["mesh", "Mesh"]],
  });

  function viewportToolRail(context, mode, stage) {
    const tools = VIEWPORT_TOOLS[mode] || VIEWPORT_TOOLS.viewport;
    const rail = el("div", "atlas-wb-toolrail");
    rail.setAttribute("aria-label", `${mode} tools`);
    const stored = context.workspaceState?.ui?.active_tool;
    const initial = tools.some(([id]) => id === stored) ? stored : tools[0][0];
    stage.dataset.activeTool = initial;
    tools.forEach(([id, label], index) => {
      const button = el("button", id === initial ? "is-active" : "", String(index + 1));
      button.type = "button";
      button.title = label;
      button.setAttribute("aria-label", label);
      button.addEventListener("click", () => {
        rail.querySelectorAll("button").forEach((item) => item.classList.toggle("is-active", item === button));
        stage.dataset.activeTool = id;
        context.updateUi?.({ active_tool: id, interaction_mode: mode });
      });
      rail.appendChild(button);
    });
    return rail;
  }

  function metricBoard(context) {
    const host = el("div", "atlas-wb-metric-board");
    const series = context.documentData?.series || [];
    if (!series.length) {
      host.appendChild(el("div", "atlas-wb-empty", "Select a real metric artifact. Atlas will not synthesize run values."));
      return host;
    }
    series.slice(0, 6).forEach((item, seriesIndex) => {
      const card = el("article");
      card.tabIndex = 0;
      card.append(el("strong", "", item.label || item.id || "Metric"));
      const values = item.points || item.values || [];
      const value = values.at?.(-1)?.y ?? values.at?.(-1)?.value ?? "—";
      card.append(el("code", "", String(value)));
      const numericValues = values.slice(-80).map((point) => Number(point?.y ?? point?.value ?? point)).filter(Number.isFinite);
      const spark = document.createElementNS("http://www.w3.org/2000/svg", "svg");
      spark.setAttribute("class", "atlas-wb-sparkline");
      spark.setAttribute("viewBox", "0 0 160 48");
      if (numericValues.length) {
        const minimum = Math.min(...numericValues);
        const maximum = Math.max(...numericValues);
        const range = maximum - minimum || 1;
        const points = numericValues.map((numeric, index) => {
          const x = numericValues.length > 1 ? (index / (numericValues.length - 1)) * 158 + 1 : 80;
          const y = 45 - ((numeric - minimum) / range) * 41;
          return `${x.toFixed(2)},${y.toFixed(2)}`;
        }).join(" ");
        const line = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
        line.setAttribute("points", points);
        spark.appendChild(line);
      }
      card.appendChild(spark);
      card.addEventListener("click", () => persistSelection(context, "metric-series", item.id || item.label || seriesIndex, {
        metric_range: numericValues.length ? [0, numericValues.length - 1] : [],
      }));
      host.appendChild(card);
    });
    return host;
  }

  function runMatrix(context) {
    const host = el("div", "atlas-wb-run-matrix");
    const header = el("div", "atlas-wb-run-head");
    ["Compare", "Run / experiment", "State", "Revision", "Output"].forEach((label) => header.appendChild(el("span", "", label)));
    host.appendChild(header);
    const persisted = new Set(context.workspaceState?.ui?.comparison_set || []);
    const taskRows = (context.tasks || []).map((task) => ({
      id: task.id,
      label: task.intent_label || task.intent_id || task.id,
      status: task.status || "planning",
      revision: String(task.updated_at || task.created_at || "").slice(0, 19),
      output: task.artifacts?.[0]?.path || task.current_stage || "",
    }));
    const assetRowsData = context.assets.slice(0, 60).map((asset) => ({
      id: asset.id,
      label: asset.name || asset.path,
      status: asset.id === context.selectedAsset?.id ? "active" : "recorded",
      revision: String(asset.content_revision || "").slice(0, 9),
      output: asset.path,
      asset,
    }));
    [...taskRows, ...assetRowsData.filter((row) => !taskRows.some((task) => task.output === row.output))].forEach((run) => {
      const row = el("button", `atlas-wb-run-row${persisted.has(run.id) ? " is-compared" : ""}`);
      row.type = "button";
      const marker = el("i");
      marker.setAttribute("aria-hidden", "true");
      row.append(marker, el("strong", "", run.label), el("span", `is-${run.status}`, run.status), el("code", "", run.revision || "—"), el("small", "", run.output || "—"));
      row.addEventListener("click", (event) => {
        if (event.shiftKey || !run.asset) {
          if (persisted.has(run.id)) persisted.delete(run.id); else persisted.add(run.id);
          row.classList.toggle("is-compared", persisted.has(run.id));
          context.updateUi?.({ comparison_set: [...persisted], selection_kind: "run-comparison" });
          return;
        }
        context.selectAsset(run.asset);
      });
      host.appendChild(row);
    });
    if (host.children.length === 1) host.appendChild(el("div", "atlas-wb-empty", "No experiment runs are registered. Create an experiment or import a real run artifact."));
    return host;
  }

  function viewport(context, mode) {
    const stage = el("div", `atlas-wb-viewport is-${mode}`);
    stage.appendChild(viewportToolRail(context, mode, stage));
    const reticle = el("div", "atlas-wb-reticle");
    reticle.innerHTML = '<i></i><i></i><i></i><i></i>';
    stage.appendChild(reticle);
    if (mode === "image" && context.selectedAsset && /^(png|jpe?g|bmp|tiff?|webp)$/i.test(context.selectedAsset.file_type || "")) {
      const image = el("img");
      image.alt = context.selectedAsset.name || "Selected image";
      image.src = `/api/workspace/file/raw?path=${encodeURIComponent(context.selectedAsset.path)}`;
      stage.appendChild(image);
      const boxes = (context.documentData?.nodes || []).filter((node) => Array.isArray(node?.metadata?.bbox));
      boxes.slice(0, 80).forEach((node, index) => {
        const [x, y, width, height] = node.metadata.bbox.map(Number);
        const media = context.documentData?.metadata?.media || {};
        if (![x, y, width, height, Number(media.width), Number(media.height)].every(Number.isFinite) || !media.width || !media.height) return;
        const box = el("div", "atlas-wb-bounding-box", node.label || "detection");
        box.style.left = `${(x / media.width) * 100}%`;
        box.style.top = `${(y / media.height) * 100}%`;
        box.style.width = `${(width / media.width) * 100}%`;
        box.style.height = `${(height / media.height) * 100}%`;
        box.tabIndex = 0;
        box.addEventListener("click", () => persistSelection(context, "annotation", node.id || index, {
          frame_id: context.selectedAsset?.id || "",
        }));
        stage.appendChild(box);
      });
    } else if (Array.isArray(context.documentData?.metadata?.geometry?.points) && context.documentData.metadata.geometry.points.length) {
      const canvas = el("canvas", "atlas-wb-geometry-canvas");
      stage.appendChild(canvas);
      window.requestAnimationFrame(() => {
        if (context.mountGeometry) context.mountGeometry(canvas, context.documentData.metadata.geometry);
        else window.AtlasDomain3D?.mount?.(canvas, context.documentData.metadata.geometry, {
          accent: getComputedStyle(stage).getPropertyValue("--domain-accent") || "#67aadb",
        });
      });
    } else {
      stage.append(el("div", "atlas-wb-perspective-grid"), el("div", "atlas-wb-empty atlas-wb-viewport-empty", "No parsed geometry is available. Select a compatible real asset or run its native inspection action."));
    }
    const overlay = el("div", "atlas-wb-viewport-overlay");
    overlay.append(el("code", "", context.selectedAsset?.name || "NO ACTIVE GEOMETRY"), el("span", "", context.selectedAsset ? `rev ${String(context.selectedAsset.content_revision || "").slice(0, 8)}` : "Awaiting real workspace asset"));
    stage.appendChild(overlay);
    const coordinates = el("div", "atlas-wb-coordinate-hud");
    const coordinateLabels = {
      image: "IMAGE · PIXEL / INSTANCE", viewport: "WORLD · OBJECT / MATERIAL", wireframe: "MESH · VERTEX / FACE",
      cad: "XY · PARAMETRIC / CONSTRAINED", robot: "MAP · TF / COLLISION", scientific: "DOMAIN · FIELD / SAMPLE",
    };
    coordinates.textContent = coordinateLabels[mode] || mode.toUpperCase();
    stage.appendChild(coordinates);
    return stage;
  }

  function codePane(context, label) {
    const host = el("div", "atlas-wb-code");
    const asset = context.selectedAsset;
    const nodes = context.documentData?.nodes || [];
    const sourceText = context.documentData?.metadata?.source?.text
      || context.documentData?.metadata?.action_output
      || "";
    const rows = sourceText
      ? String(sourceText).split(/\r?\n/).slice(0, 600)
      : nodes.slice(0, 80).map((node) => node.metadata?.text || node.label || node.id || JSON.stringify(node));
    if (!rows.length && asset) rows.push(`${asset.name}`, `source: ${asset.path}`, `content_revision: ${asset.content_revision}`, "", "Open the asset to inspect its exact source text.");
    if (!rows.length) rows.push("No source or IR artifact selected.");
    rows.forEach((line, index) => {
      const row = el("div");
      row.append(el("code", "", String(index + 1).padStart(3, " ")), el("span", "", String(line)));
      row.tabIndex = 0;
      row.addEventListener("click", () => persistSelection(context, `${label}-line`, index + 1, {
        source_path: asset?.path || "",
      }));
      host.appendChild(row);
    });
    host.dataset.language = label;
    return host;
  }

  function matchingAction(context, terms) {
    const needles = terms.map((term) => String(term).toLowerCase());
    return (context.actions || []).find((action) => {
      const source = `${action.id || ""} ${action.label || ""}`.toLowerCase();
      return needles.some((term) => source.includes(term));
    }) || null;
  }

  function editableConsole(context, kind, actionGroups) {
    const host = el("div", `atlas-wb-editable-console is-${kind}`);
    const toolbar = el("header");
    toolbar.append(el("span", "", `${kind.toUpperCase()} DRAFT`));
    const actions = el("nav");
    actionGroups.forEach(([label, ...terms]) => {
      const action = matchingAction(context, terms);
      const button = el("button", action?.ready ? "is-ready" : "", label);
      button.type = "button";
      button.title = action ? `${action.sdk || "Native runtime"}: ${action.ready ? "ready" : action.reason || "unavailable"}` : `Open ${kind} Agent task`;
      button.addEventListener("click", () => {
        if (action?.ready) context.runAction?.(action);
        else context.openTask?.({ initialPrompt: `${label} the active ${kind} draft against ${context.selectedAsset?.path || "the current workspace"}.` });
      });
      actions.appendChild(button);
    });
    toolbar.appendChild(actions);
    const editor = el("textarea");
    const sourceText = context.documentData?.metadata?.source?.text
      || context.documentData?.metadata?.action_output
      || "";
    const draftKey = `${kind}_draft`;
    editor.value = context.workspaceState?.ui?.[draftKey] ?? sourceText;
    editor.placeholder = kind === "prompt"
      ? "Write a prompt or select a prompt artifact…"
      : "Enter a statement or open a source artifact…";
    editor.spellcheck = false;
    editor.addEventListener("input", () => context.updateUi?.({
      [draftKey]: editor.value,
      selection_kind: `${kind}-draft`,
      draft_dirty: editor.value !== sourceText,
    }));
    const status = el("footer");
    status.append(el("span", "", context.selectedAsset?.name || "Workspace draft"), el("code", "", `${editor.value.length} chars · ${context.spec.runtime}`));
    host.append(toolbar, editor, status);
    return host;
  }

  function packetTable(context) {
    const host = el("div", "atlas-wb-packets");
    const header = el("div", "atlas-wb-packet-head");
    ["No.", "Time", "Source", "Destination", "Protocol", "Length", "Info"].forEach((value) => header.appendChild(el("span", "", value)));
    host.appendChild(header);
    const nodes = context.documentData?.nodes || [];
    nodes.slice(0, 80).forEach((node, index) => {
      const metadata = node.metadata || {};
      const row = el("button", "atlas-wb-packet-row");
      [index + 1, metadata.time || metadata.timestamp || "—", metadata.source || "—", metadata.destination || "—", metadata.protocol || node.category || "—", metadata.length || "—", node.label || node.id].forEach((value) => row.appendChild(el("span", "", String(value ?? "—"))));
      row.addEventListener("click", () => {
        host.querySelectorAll(".atlas-wb-packet-row").forEach((item) => item.classList.toggle("is-active", item === row));
        persistSelection(context, "packet", node.id || index + 1, { packet_number: index + 1 });
      });
      host.appendChild(row);
    });
    if (!nodes.length) host.appendChild(el("div", "atlas-wb-empty", "Decode a PCAP with tshark to populate the packet table."));
    return host;
  }

  function dataGrid(context) {
    const host = el("div", "atlas-wb-data-grid");
    const table = context.documentData?.metadata?.table || {};
    let rows = Array.isArray(table.rows) ? table.rows.slice(0, 200) : [];
    let columns = Array.isArray(table.columns) ? table.columns.slice(0, 14) : [];
    if (!rows.length) {
      rows = (context.documentData?.nodes || []).slice(0, 120).map((node) => ({
        Name: node.label || node.id,
        Type: node.category || "object",
        Status: node.status || "",
        ...node.metrics,
      }));
    }
    if (!columns.length) columns = [...new Set(rows.flatMap((row) => Object.keys(row || {})))].slice(0, 14);
    if (!columns.length) {
      host.appendChild(el("div", "atlas-wb-empty", "No query rows or actual-plan records are available."));
      return host;
    }
    const grid = el("div", "atlas-wb-data-grid-table");
    grid.style.setProperty("--data-columns", `repeat(${columns.length}, minmax(105px, 1fr))`);
    const head = el("div", "atlas-wb-data-grid-head");
    columns.forEach((column) => head.appendChild(el("span", "", column)));
    grid.appendChild(head);
    rows.forEach((row) => {
      const record = el("div", "atlas-wb-data-grid-row");
      columns.forEach((column) => {
        const value = row?.[column];
        record.appendChild(el("span", "", typeof value === "object" ? JSON.stringify(value) : String(value ?? "")));
      });
      grid.appendChild(record);
    });
    host.appendChild(grid);
    return host;
  }

  function tokenFlow(context) {
    const host = el("div", "atlas-wb-token-flow");
    const nodes = context.documentData?.nodes || [];
    nodes.slice(0, 120).forEach((node, index) => {
      const token = el("button", `atlas-wb-token t${index % 5}`, node.label || node.id || "token");
      token.type = "button";
      token.title = JSON.stringify(node.metadata || {});
      token.addEventListener("click", () => {
        host.querySelectorAll(".atlas-wb-token").forEach((item) => item.classList.toggle("is-active", item === token));
        persistSelection(context, "token", node.id || index, {
          token_index: index,
          source_span: node.metadata?.span || node.metadata?.offset || null,
        });
      });
      host.appendChild(token);
    });
    if (!nodes.length) host.appendChild(el("div", "atlas-wb-empty", "Run Tokenize on a corpus to inspect exact source offsets."));
    return host;
  }

  function mediaFilmstrip(context) {
    const host = el("div", "atlas-wb-filmstrip");
    const media = context.assets.filter((asset) => /^(png|jpe?g|bmp|tiff?|webp|mp4|avi|mov)$/i.test(asset.file_type || ""));
    media.slice(0, 60).forEach((asset, index) => {
      const item = el("button", asset.id === context.selectedAsset?.id ? "is-active" : "");
      item.type = "button";
      if (/^(png|jpe?g|bmp|tiff?|webp)$/i.test(asset.file_type || "")) {
        const image = el("img");
        image.src = `/api/workspace/file/raw?path=${encodeURIComponent(asset.path)}`;
        image.alt = asset.name || `Frame ${index + 1}`;
        item.appendChild(image);
      } else {
        item.appendChild(el("span", "atlas-wb-video-frame", "VIDEO"));
      }
      item.append(el("code", "", String(index + 1).padStart(3, "0")), el("strong", "", asset.name || asset.path));
      item.addEventListener("click", () => context.selectAsset(asset));
      host.appendChild(item);
    });
    if (!media.length) host.appendChild(el("div", "atlas-wb-empty", "No image or video media is registered in this workspace."));
    return host;
  }

  function protocolTree(context) {
    const host = el("div", "atlas-wb-protocol-tree");
    const nodes = context.documentData?.nodes || [];
    nodes.slice(0, 180).forEach((node) => {
      const depth = String(node.parent_id || "").split(/[/.:]/).filter(Boolean).length;
      const row = el("button");
      row.type = "button";
      row.style.paddingLeft = `${8 + Math.min(depth, 6) * 10}px`;
      row.append(el("i", "", node.parent_id ? "⌞" : "▾"), el("strong", "", node.label || node.id), el("code", "", node.category || "field"));
      row.title = JSON.stringify({ ...node.metrics, ...node.metadata });
      row.addEventListener("click", () => {
        host.querySelectorAll("button").forEach((item) => item.classList.toggle("is-active", item === row));
        persistSelection(context, "protocol-field", node.id || node.label, { protocol_path: node.parent_id || "" });
      });
      host.appendChild(row);
    });
    if (!nodes.length) host.appendChild(el("div", "atlas-wb-empty", "Decode a capture to inspect its real protocol field hierarchy."));
    return host;
  }

  function compilerCorrelation(context) {
    const host = el("div", "atlas-wb-compiler-correlation");
    const source = editableConsole(context, "source", [["Compile", "compile"], ["Emit IR", "emit-ir", "ir"]]);
    const astContext = { ...context, documentData: { ...context.documentData, metadata: {}, nodes: (context.documentData?.nodes || []).filter((node) => /ast|symbol/i.test(node.category || "")) } };
    const irContext = { ...context, documentData: { ...context.documentData, metadata: {}, nodes: (context.documentData?.nodes || []).filter((node) => /instruction|basic-block|function/i.test(node.category || "")) } };
    const columns = [["SOURCE", source], ["AST / SYMBOLS", codePane(astContext, "ast")], ["IR / ASSEMBLY", codePane(irContext, "ir")]];
    columns.forEach(([label, content]) => {
      const section = el("section");
      section.append(el("header", "", label), content);
      host.appendChild(section);
    });
    return host;
  }

  function databaseConsole(context) {
    const host = el("div", "atlas-wb-database-console");
    const editor = el("section", "atlas-wb-db-editor");
    editor.append(el("header", "", "SQL EDITOR"), editableConsole(context, "sql", [["Execute", "execute"], ["Explain", "explain"]]));
    const results = el("section", "atlas-wb-db-results");
    results.append(el("header", "", "RESULTS / ACTUAL PLAN"), dataGrid(context));
    host.append(editor, results);
    return host;
  }

  function simulationConsole(context) {
    const host = el("div", "atlas-wb-simulation-console");
    const controls = el("header");
    const running = context.workspaceState?.ui?.simulation_state === "running";
    const run = el("button", running ? "is-running" : "", running ? "Pause" : "Run");
    const step = el("button", "", "Step");
    const reset = el("button", "", "Reset");
    const clock = el("code", "", `STEP ${context.workspaceState?.ui?.simulation_step || 0}`);
    run.type = step.type = reset.type = "button";
    run.addEventListener("click", () => context.updateUi?.({ simulation_state: running ? "paused" : "running", selection_kind: "physics-clock" }));
    step.addEventListener("click", () => context.updateUi?.({ simulation_state: "paused", simulation_step: Number(context.workspaceState?.ui?.simulation_step || 0) + 1, selection_kind: "physics-step" }));
    reset.addEventListener("click", () => context.updateUi?.({ simulation_state: "paused", simulation_step: 0, time_index: 0, selection_kind: "physics-reset" }));
    controls.append(run, step, reset, clock);
    host.append(controls, viewport(context, "robot"));
    return host;
  }

  function linkedCodeWorkbench(context, kind, labels) {
    const host = el("div", `atlas-wb-linked-code is-${kind}`);
    labels.forEach(([label, matcher]) => {
      const section = el("section");
      const narrowed = (context.documentData?.nodes || []).filter((node) => matcher.test(`${node.category || ""} ${node.label || ""}`));
      section.append(el("header", "", label), codePane({ ...context, documentData: { ...context.documentData, metadata: {}, nodes: narrowed.length ? narrowed : context.documentData?.nodes || [] } }, label.toLowerCase()));
      host.appendChild(section);
    });
    return host;
  }

  function repositoryWorkbench(context) {
    const host = el("div", "atlas-wb-repository-workbench");
    const graphPane = el("section");
    graphPane.append(el("header", "", "COMMIT DAG"), graph(context, "commit-dag"));
    const diffPane = el("section");
    diffPane.append(el("header", "", "REVISION DIFF"), codePane(context, "diff"));
    host.append(graphPane, diffPane);
    return host;
  }

  function containerRuntime(context) {
    const host = el("div", "atlas-wb-container-runtime");
    const lifecycle = el("header");
    ["Create", "Start", "Pause", "Restart", "Stop"].forEach((label) => {
      const button = el("button", "", label);
      button.type = "button";
      button.addEventListener("click", () => context.updateUi?.({ container_operation: label.toLowerCase(), selection_kind: "container-lifecycle" }));
      lifecycle.appendChild(button);
    });
    host.append(lifecycle, graph(context, "container-topology"));
    return host;
  }

  function timeline(context, mode) {
    const host = el("div", `atlas-wb-timeline is-${mode}`);
    const events = context.documentData?.events || context.documentData?.frames || [];
    const items = events.slice(0, 80);
    const timestamps = items.map((item) => Date.parse(item.timestamp || "")).filter(Number.isFinite);
    const minimumTime = timestamps.length ? Math.min(...timestamps) : 0;
    const maximumTime = timestamps.length ? Math.max(...timestamps) : 0;
    if (items.length) {
      const ruler = el("div", "atlas-wb-time-ruler");
      ruler.append(el("code", "", mode.toUpperCase()));
      const cursor = el("input");
      cursor.type = "range";
      cursor.min = "0";
      cursor.max = String(Math.max(0, items.length - 1));
      cursor.value = String(Math.max(0, Number(context.workspaceState?.ui?.time_index || 0)));
      cursor.setAttribute("aria-label", `${mode} time cursor`);
      const label = el("span", "", items[Number(cursor.value)]?.timestamp || `event ${Number(cursor.value) + 1}`);
      cursor.addEventListener("input", () => {
        const index = Number(cursor.value);
        label.textContent = items[index]?.timestamp || `event ${index + 1}`;
        context.updateUi?.({ time_index: index, time_cursor: items[index]?.timestamp || index, selection_kind: `${mode}-time` });
      });
      ruler.append(cursor, label);
      host.appendChild(ruler);
    }
    items.forEach((item, index) => {
      const lane = el("div", "atlas-wb-lane");
      lane.tabIndex = 0;
      lane.append(el("code", "", item.label || item.name || `Lane ${index + 1}`));
      const track = el("span");
      const bar = el("i");
      const timestamp = Date.parse(item.timestamp || "");
      const left = Number.isFinite(timestamp) && maximumTime > minimumTime
        ? ((timestamp - minimumTime) / (maximumTime - minimumTime)) * 92
        : (items.length > 1 ? (index / (items.length - 1)) * 92 : 0);
      const duration = Number(item.metadata?.duration_ms ?? item.metrics?.duration_ms ?? 0);
      bar.style.left = `${left}%`;
      bar.style.width = duration > 0 && maximumTime > minimumTime
        ? `${Math.max(1, Math.min(100 - left, (duration / (maximumTime - minimumTime)) * 100))}%`
        : "2px";
      track.appendChild(bar);
      lane.appendChild(track);
      lane.addEventListener("click", () => {
        host.querySelectorAll(".atlas-wb-lane").forEach((entry) => entry.classList.toggle("is-active", entry === lane));
        persistSelection(context, `${mode}-event`, item.id || item.label || index, {
          time_cursor: item.timestamp || index,
          duration_ms: duration,
        });
      });
      host.appendChild(lane);
    });
    if (!items.length) host.appendChild(el("div", "atlas-wb-empty", "No trace or timeline evidence is loaded. Atlas does not generate placeholder intervals."));
    return host;
  }

  function graph(context, mode) {
    const host = el("div", `atlas-wb-graph is-${mode}`);
    const nodes = (context.documentData?.nodes || []).slice(0, 24);
    const nodeIds = new Set(nodes.map((node) => node.id));
    const edges = (context.documentData?.edges || []).filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)).slice(0, 60);
    const positions = new Map();
    nodes.forEach((node, index) => positions.set(node.id, {
      x: 10 + ((index * 31) % 76),
      y: 12 + ((index * 47) % 70),
    }));
    if (edges.length) {
      const lines = document.createElementNS("http://www.w3.org/2000/svg", "svg");
      lines.setAttribute("viewBox", "0 0 100 100");
      lines.setAttribute("preserveAspectRatio", "none");
      edges.forEach((edge) => {
        const source = positions.get(edge.source);
        const target = positions.get(edge.target);
        if (!source || !target) return;
        const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
        line.setAttribute("x1", source.x); line.setAttribute("y1", source.y);
        line.setAttribute("x2", target.x); line.setAttribute("y2", target.y);
        line.dataset.category = edge.category || "edge";
        lines.appendChild(line);
      });
      host.appendChild(lines);
    }
    nodes.forEach((node, index) => {
      const button = el("button", "atlas-wb-graph-node", node.label || node.name || node.id || `Node ${index + 1}`);
      const position = positions.get(node.id);
      button.style.setProperty("--x", `${position.x}%`);
      button.style.setProperty("--y", `${position.y}%`);
      button.title = JSON.stringify({ ...node.metrics, ...node.metadata });
      button.addEventListener("click", () => {
        host.querySelectorAll(".atlas-wb-graph-node").forEach((item) => item.classList.toggle("is-active", item === button));
        persistSelection(context, `${mode}-node`, node.id || index, { object_type: node.category || "node" });
      });
      host.appendChild(button);
    });
    if (!nodes.length) host.appendChild(el("div", "atlas-wb-empty", "No parsed node/edge evidence is available. Atlas does not infer placeholder relationships."));
    return host;
  }

  function inspector(context, definition) {
    const host = el("div", "atlas-wb-inspector-grid");
    const asset = context.selectedAsset;
    const lastRun = context.workspaceState?.last_run || {};
    const metadata = context.documentData?.metadata || {};
    const ui = context.workspaceState?.ui || {};
    const parameters = context.workspaceState?.parameters || {};
    const base = {
      "Active object": asset?.name || "Not selected",
      "Revision": asset?.content_revision || "—",
      "Selection": ui.selection_id || "—",
      "Runtime": context.spec.runtime,
    };
    const domainValues = {
      "ai-ml": { "Run set": (ui.comparison_set || []).join(", ") || "single", "Step": ui.time_cursor ?? ui.time_index ?? "—", "Device": parameters.device || "—", "Checkpoint": metadata.checkpoint || asset?.file_type || "—" },
      "computer-vision": { "Frame": ui.frame_id || asset?.name || "—", "Region": ui.selection_id || "—", "Tool": ui.active_tool || "select", "Class schema": parameters.class_schema || "—" },
      nlp: { "Span / token": ui.selection_id || "—", "Tokenizer": parameters.tokenizer || "—", "Top K": parameters.top_k || "—", "Index revision": metadata.index_revision || "—" },
      "computer-graphics": { "Scene object": ui.selection_id || asset?.name || "—", "Component": ui.selection_kind || "—", "Render engine": parameters.render_engine || "—", "Frame": ui.time_index ?? "—" },
      cad: { "Feature / entity": ui.selection_id || asset?.name || "—", "Tool": ui.active_tool || "select", "Units": parameters.unit_system || "—", "Recompute": parameters.recompute_mode || lastRun.status || "—" },
      robotics: { "Fixed frame": parameters.fixed_frame || "—", "Link / joint": ui.selection_id || "—", "Time cursor": ui.time_cursor ?? "—", "Planning group": parameters.planning_group || "—" },
      "computer-networks": { "Packet": ui.packet_number || "—", "Protocol field": ui.protocol_path || ui.selection_id || "—", "Interface": parameters.capture_interface || "—", "Display filter": parameters.capture_filter || "—" },
      "operating-systems": { "Process / thread": ui.selection_id || "—", "Time range": ui.time_cursor ?? "—", "Symbol status": parameters.symbol_path ? "configured" : "not configured", "Trace profile": parameters.trace_profile || "—" },
      compiler: { "Source selection": ui.selection_id || "—", "IR stage": ui.workbench_view || "—", "Target": parameters.target_triple || "—", "Pass pipeline": parameters.pass_pipeline || "—" },
      database: { "Connection": parameters.connection || "—", "Schema": parameters.schema || "—", "Plan node / row": ui.selection_id || "—", "Transaction": parameters.transaction_mode || "—" },
      "software-engineering": { "Reference / commit": ui.selection_id || asset?.name || "—", "Diff algorithm": parameters.diff_algorithm || "—", "Rename detection": parameters.rename_detection || "—", "Merge state": ui.merge_state || lastRun.status || "clean" },
      "program-analysis": { "Query / result": ui.selection_id || "—", "Selection type": ui.selection_kind || "—", "Code database": parameters.fact_database || "—", "Confidence": parameters.confidence_threshold || "—" },
      "cyber-security": { "Address / function": ui.selection_id || asset?.name || "—", "Selection type": ui.selection_kind || "—", "Language / compiler": parameters.scanner_profile || "auto", "Analysis state": lastRun.status || "—" },
      hpc: { "Rank / kernel": ui.selection_id || "—", "Time cursor": ui.time_cursor ?? "—", "Node / ranks": `${parameters.node_count || "—"} / ${parameters.rank_count || "—"}`, "Affinity": parameters.affinity || "—" },
      "distributed-systems": { "Engine / project": parameters.cluster_context || "local", "Container / image": ui.selection_id || "—", "Lifecycle op": ui.container_operation || "inspect", "Health": lastRun.status || "—" },
      "scientific-computing": { "Dataset / array": ui.selection_id || "—", "Representation": ui.workbench_view || "volume", "Field association": parameters.solver || "point / cell", "Time step": ui.time_index ?? "—" },
    };
    const values = { ...base, ...(domainValues[context.domainId] || {}), "Agent API": context.spec.agentApi };
    Object.entries(values).forEach(([key, value]) => {
      const row = el("div");
      row.append(el("span", "", key), el("code", "", String(value)));
      host.appendChild(row);
    });
    return host;
  }

  function primaryContent(context, definition) {
    if (context.domainId === "ai-ml") return runMatrix(context);
    if (context.domainId === "compiler") return compilerCorrelation(context);
    if (context.domainId === "program-analysis") return editableConsole(context, "query", [["Run Query", "query", "scan"], ["Build Database", "database", "index"]]);
    if (context.domainId === "cyber-security") return objectTree(context, "symbol-function");
    if (context.domainId === "computer-networks") return packetTable(context);
    if (context.domainId === "operating-systems" || context.domainId === "cyber-security") return dataGrid(context);
    if (context.domainId === "hpc") return objectTree(context, "job-rank");
    if (context.domainId === "nlp") return editableConsole(context, "prompt", [["Run", "prompt", "run"], ["Tokenize", "token"], ["Retrieve", "retrieve"]]);
    const treeKinds = {
      "computer-graphics": "scene", cad: "feature", robotics: "display", database: "schema",
      "software-engineering": "reference", "distributed-systems": "container", "scientific-computing": "pipeline",
    };
    if (treeKinds[context.domainId]) return objectTree(context, treeKinds[context.domainId]);
    return assetRows(context, 20);
  }

  function secondaryContent(context, definition) {
    if (definition.mode === "metrics") return metricBoard(context);
    if (definition.mode === "robot") return simulationConsole(context);
    if (["viewport", "image", "cad", "scientific"].includes(definition.mode)) return viewport(context, definition.mode);
    if (["timeline", "gpu"].includes(definition.mode)) return timeline(context, definition.mode);
    if (definition.mode === "database") return databaseConsole(context);
    if (definition.mode === "repository") return repositoryWorkbench(context);
    if (definition.mode === "semantic") return linkedCodeWorkbench(context, "semantic", [["QUERY / SOURCE", /query|source|predicate/i], ["PATH STEPS", /flow|path|call|result/i]]);
    if (definition.mode === "reverse") return linkedCodeWorkbench(context, "reverse", [["DECOMPILER", /decomp|function|variable/i], ["DISASSEMBLY", /instruction|block|assembly/i]]);
    if (definition.mode === "containers") return containerRuntime(context);
    if (definition.mode === "compiler") return codePane(context, definition.mode);
    if (definition.mode === "packets") return protocolTree(context);
    if (definition.mode === "tokens") return tokenFlow(context);
    return graph(context, definition.mode);
  }

  function viewContent(context, definition, view) {
    const key = String(view || "").toLowerCase();
    if (context.domainId === "ai-ml") {
      if (key.includes("metric")) return metricBoard(context);
      if (key.includes("checkpoint")) return assetRows({ ...context, assets: context.assets.filter((asset) => /^(ckpt|pt|pth|safetensors)$/i.test(asset.file_type || "")) }, 80);
      if (key.includes("hyper")) return dataGrid(context);
      return assetRows(context, 80);
    }
    if (context.domainId === "computer-vision") {
      if (key === "media") return mediaFilmstrip(context);
      if (key.includes("track")) return graph(context, "tracks");
      return viewport(context, "image");
    }
    if (context.domainId === "nlp") {
      if (key.includes("token")) return tokenFlow(context);
      if (key.includes("prompt")) return codePane(context, "prompt");
      if (key.includes("retrieval") || key.includes("knowledge")) return graph(context, key);
      return dataGrid(context);
    }
    if (context.domainId === "computer-graphics") {
      if (key.includes("shader")) return codePane(context, "shader");
      if (key.includes("animation")) return timeline(context, "animation");
      if (key.includes("material")) return dataGrid(context);
      return viewport(context, key.includes("wire") ? "wireframe" : "viewport");
    }
    if (context.domainId === "cad") {
      if (key.includes("constraint") || key.includes("assembly")) return graph(context, key);
      if (key.includes("sketch")) return codePane(context, "sketch");
      return viewport(context, "cad");
    }
    if (context.domainId === "robotics") {
      if (key.includes("tf")) return graph(context, "tf");
      if (key.includes("sensor")) return dataGrid(context);
      if (key.includes("trajectory")) return timeline(context, "trajectory");
      if (key === "map") return dataGrid(context);
      return viewport(context, "robot");
    }
    if (context.domainId === "computer-networks") {
      if (key.includes("protocol")) return protocolTree(context);
      if (key.includes("packet")) return packetTable(context);
      if (key.includes("byte")) return codePane(context, "packet-bytes");
      return graph(context, key);
    }
    if (context.domainId === "operating-systems") {
      if (key.includes("cpu") || key.includes("syscall") || key === "i/o") return timeline(context, key);
      if (key.includes("memory")) return graph(context, "memory");
      return dataGrid(context);
    }
    if (context.domainId === "compiler") {
      if (key === "ast") return graph(context, "ast");
      if (key.includes("diagnostic")) return dataGrid(context);
      return codePane(context, key);
    }
    if (context.domainId === "database") {
      if (key.includes("sql")) return databaseConsole(context);
      if (key.includes("plan") || key.includes("schema") || key.includes("lock")) return graph(context, key);
      return dataGrid(context);
    }
    if (context.domainId === "software-engineering") {
      if (key.includes("diff")) return codePane(context, "revision-diff");
      if (key.includes("history")) return timeline(context, "commit-history");
      if (key.includes("merge")) return dataGrid(context);
      return repositoryWorkbench(context);
    }
    if (context.domainId === "program-analysis") {
      if (key.includes("graph") || key.includes("flow") || key.includes("path")) return graph(context, key);
      if (key.includes("result")) return dataGrid(context);
      return editableConsole(context, "query", [["Run Query", "query", "scan"], ["Build Database", "database"]]);
    }
    if (context.domainId === "cyber-security") {
      if (key.includes("graph") || key.includes("xref") || key.includes("memory")) return graph(context, key);
      return linkedCodeWorkbench(context, "reverse", [["DECOMPILER", /decomp|function|variable/i], ["DISASSEMBLY", /instruction|block|assembly/i]]);
    }
    if (context.domainId === "hpc") {
      if (key.includes("timeline")) return timeline(context, "gpu");
      return dataGrid(context);
    }
    if (context.domainId === "distributed-systems") {
      if (key.includes("log") || key.includes("runtime")) return timeline(context, "container-events");
      if (key.includes("image") || key.includes("volume")) return objectTree(context, key);
      return containerRuntime(context);
    }
    if (context.domainId === "scientific-computing") {
      if (/volume|surface|slice|contour|streamline/.test(key)) return viewport(context, "scientific");
      return graph(context, "visualization-pipeline");
    }
    return secondaryContent(context, definition);
  }

  function dockContent(context, definition) {
    if (context.domainId === "computer-vision") return mediaFilmstrip(context);
    if (context.domainId === "nlp") return graph(context, "knowledge");
    if (context.domainId === "computer-networks") return graph(context, "flow");
    if (context.domainId === "compiler") return graph(context, "cfg");
    if (context.domainId === "database") return graph(context, "plan");
    if (context.domainId === "software-engineering") return timeline(context, "branch-history");
    if (context.domainId === "program-analysis") return graph(context, "data-flow-path");
    if (context.domainId === "cyber-security") return graph(context, "function-graph");
    if (context.domainId === "distributed-systems") return timeline(context, "container-events");
    if (context.domainId === "scientific-computing") return timeline(context, "simulation-time");
    if (["timeline", "gpu", "metrics", "cluster", "security", "architecture", "robot", "cad", "viewport"].includes(definition.mode)) return timeline(context, definition.mode);
    return graph(context, definition.mode);
  }

  function pane(area, label, content, meta = "") {
    const section = el("section", `atlas-wb-pane area-${area}`);
    const header = el("header");
    const copy = el("div");
    copy.append(el("span", "", area.toUpperCase()), el("strong", "", label));
    header.append(copy, el("code", "", meta));
    section.append(header, content);
    return section;
  }

  function render(context) {
    const definition = WORKBENCHES[context.domainId];
    if (!definition) return null;
    const shell = el("section", `atlas-professional-workbench wb-${definition.mode}`);
    shell.dataset.workbench = definition.id;
    shell.dataset.domain = context.domainId;
    shell.dataset.runtime = context.spec.runtime;
    shell.style.setProperty("--wb-columns", definition.columns);
    shell.style.setProperty("--wb-areas", definition.areas);

    const commandBar = el("header", "atlas-wb-commandbar");
    const identity = el("div", "atlas-wb-identity");
    identity.append(
      el("span", "", definition.signature),
      el("strong", "", context.activeTask
        ? `${context.activeTask.intent_label || context.activeTask.intent_id} · ${context.activeTask.status || "planning"} / ${context.activeTask.current_stage || "plan"}`
        : context.spec.focus),
    );
    const commandGroups = el("div", "atlas-wb-command-groups");
    const commands = el("nav", "atlas-wb-view-commands");
    const activeView = context.workspaceState?.ui?.workbench_view || definition.commands[0];
    definition.commands.forEach((label, index) => {
      const active = activeView === label;
      const button = el("button", active ? "is-primary" : "", label);
      button.type = "button";
      button.addEventListener("click", () => {
        commands.querySelectorAll("button").forEach((item) => item.classList.toggle("is-primary", item === button));
        context.updateUi({ workbench_view: label, workbench_id: definition.id });
        secondaryPane.querySelector("header strong").textContent = label;
        secondaryPane.lastElementChild?.remove();
        secondaryPane.appendChild(viewContent(context, definition, label));
      });
      commands.appendChild(button);
    });
    const runtimeActions = el("nav", "atlas-wb-runtime-actions");
    (context.actions || []).filter((action) => action.ready).slice(0, 2).forEach((action) => {
      const button = el("button", "is-native", action.label || action.id);
      button.type = "button";
      button.title = `${action.sdk || context.spec.runtime} · native action`;
      button.addEventListener("click", () => context.runAction?.(action));
      runtimeActions.appendChild(button);
    });
    const agentButton = el("button", "is-agent", context.activeTask ? "Open active task" : "Delegate to Agent");
    agentButton.type = "button";
    agentButton.title = `${context.spec.agentApi} receives the live ${context.spec.selectionModel} selection`;
    agentButton.addEventListener("click", () => context.openTask?.({
      initialPrompt: `Operate on the active ${context.spec.selectionModel} selection in ${context.spec.studio}.`,
    }));
    runtimeActions.appendChild(agentButton);
    commandGroups.append(commands, runtimeActions);
    commandBar.append(identity, commandGroups);

    const grid = el("div", "atlas-wb-grid");
    const secondaryPane = pane("secondary", activeView, viewContent(context, definition, activeView), context.selectedAsset?.name || context.spec.previewTarget);
    grid.append(
      pane("primary", definition.primary, primaryContent(context, definition), `${context.assets.length} objects`),
      secondaryPane,
      pane("inspector", definition.inspector, inspector(context, definition), context.spec.agentApi),
      pane("dock", definition.dock, dockContent(context, definition), `${context.spec.runtime} · ${context.workspaceState?.last_run?.status || "idle"}`),
    );
    shell.append(commandBar, grid);
    return shell;
  }

  function renderTab(context, tab) {
    const definition = WORKBENCHES[context.domainId];
    if (!definition) return null;
    if (tab === "overview") return render(context);
    const shell = el("section", `atlas-domain-tab-workbench wb-${definition.mode} tab-${tab}`);
    shell.dataset.workbench = definition.id;
    shell.dataset.domain = context.domainId;
    const header = el("header", "atlas-domain-tab-header");
    const identity = el("div");
    const tabLabel = context.spec.navigation?.find((item) => item.tab === tab)?.label || tab.toUpperCase();
    identity.append(el("span", "", `${definition.signature} / ${tabLabel.toUpperCase()}`), el("strong", "", context.spec.focus));
    header.append(identity, el("code", "", `${context.spec.runtime} · ${context.selectedAsset?.name || `${context.assets.length} workspace objects`}`));
    shell.appendChild(header);

    if (tab === "resources") {
      const body = el("div", "atlas-domain-resource-layout");
      const taxonomy = el("aside", "atlas-domain-object-palette");
      context.spec.dataObjects.forEach((type, index) => {
        const button = el("button", index === 0 ? "is-active" : "");
        button.type = "button";
        button.append(el("strong", "", type), el("span", "", context.assets.filter((asset) => context.assetMatches(asset, type)).length.toString()));
        button.addEventListener("click", () => context.filterType(type));
        taxonomy.appendChild(button);
      });
      body.append(taxonomy, pane("secondary", definition.primary, assetRows(context, 80), `${context.assets.length} discovered`), pane("inspector", definition.inspector, inspector(context, definition), "PROVENANCE"));
      shell.appendChild(body);
    } else if (tab === "artifacts") {
      const body = el("div", "atlas-domain-artifact-workflow");
      body.append(graph(context, definition.mode));
      const ledger = el("div", "atlas-domain-artifact-ledger");
      context.assets.slice(0, 40).forEach((asset) => {
        const row = el("button");
        row.type = "button";
        row.append(el("span", "", String(asset.file_type || "artifact").toUpperCase()), el("strong", "", asset.name || asset.path), el("code", "", String(asset.content_revision || "").slice(0, 10)));
        row.addEventListener("click", () => context.selectAsset(asset));
        ledger.appendChild(row);
      });
      body.appendChild(ledger);
      shell.appendChild(body);
    } else if (tab === "history") {
      const body = el("div", "atlas-domain-history-layout");
      body.append(timeline(context, definition.mode));
      const runs = el("section", "atlas-domain-run-ledger");
      const taskRows = (context.tasks || []).map((task) => ({
        id: task.id,
        label: task.intent_label || task.intent_id,
        status: task.status,
        outputPath: task.artifacts?.[0]?.path || task.current_stage || "",
      }));
      [...taskRows, ...context.runs.filter((run) => !taskRows.some((task) => task.id === run.id)), ...(context.workspaceState?.last_run ? [{ label: context.workspaceState.last_run.action_id, status: context.workspaceState.last_run.status, outputPath: context.workspaceState.last_run.output_path }] : [])].forEach((run) => {
        const row = el("div");
        row.append(el("i", `is-${run.status || "idle"}`), el("strong", "", run.label || run.action_id || "Domain action"), el("code", "", run.status || "recorded"), el("span", "", run.outputPath || run.output_path || run.asset || ""));
        runs.appendChild(row);
      });
      if (!runs.children.length) runs.appendChild(el("div", "atlas-wb-empty", "No SDK execution or Agent workflow is recorded."));
      body.appendChild(runs);
      if (context.actionLog) body.appendChild(el("pre", "research-domain-action-log", context.actionLog));
      shell.appendChild(body);
    } else if (tab === "settings") {
      const form = el("div", "atlas-domain-settings-console");
      const actionSettings = el("section");
      actionSettings.appendChild(el("strong", "", `${definition.id} parameters`));
      context.spec.settings.forEach((key) => {
        const label = el("label");
        label.appendChild(el("span", "", key.replaceAll("_", " ")));
        const input = el("input");
        input.value = context.workspaceState?.parameters?.[key] ?? "";
        input.placeholder = "Not configured";
        input.addEventListener("change", () => context.updateParameters({ [key]: input.value }));
        label.appendChild(input);
        actionSettings.appendChild(label);
      });
      const sdk = el("section", "atlas-domain-sdk-matrix");
      sdk.appendChild(el("strong", "", "Native action readiness"));
      context.actions.forEach((action) => {
        const row = el("div", action.ready ? "is-ready" : "is-disabled");
        row.append(el("i"), el("span", "", action.label), el("code", "", action.sdk), el("small", "", action.ready ? (action.version || "ready") : action.reason));
        sdk.appendChild(row);
      });
      form.append(actionSettings, sdk);
      shell.appendChild(form);
    } else if (tab === "agent-context") {
      const body = el("div", "atlas-domain-agent-contract");
      const contract = el("section");
      contract.append(el("span", "", definition.signature), el("strong", "", "Domain-specific Agent Context"), el("p", "", context.spec.agentContext), el("p", "", context.spec.interaction));
      const payload = el("pre");
      payload.textContent = JSON.stringify({
        domain_id: context.domainId, workbench_id: definition.id, active_object: context.selectedAsset || null,
        workspace_state: context.workspaceState, native_actions: context.actions.map(({ id, sdk, ready, reason }) => ({ id, sdk, ready, reason })),
        interaction_contract: context.spec.interaction,
      }, null, 2);
      body.append(contract, payload);
      shell.appendChild(body);
    }
    return shell;
  }

  window.AtlasResearchWorkbenches = Object.freeze({
    get(domainId) { return WORKBENCHES[domainId] || null; },
    all() { return { ...WORKBENCHES }; },
    render,
    renderTab,
  });
})();
