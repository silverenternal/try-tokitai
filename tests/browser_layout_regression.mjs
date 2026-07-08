import { spawn } from "node:child_process";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";

const EDGE_CANDIDATES = [
  process.env.EDGE_PATH || "",
  "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
  "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
].filter(Boolean);
const BASE_URL = process.env.TOKITAI_BROWSER_REGRESSION_URL || "http://127.0.0.1:3001";
const OUT_DIR = path.resolve("target", "browser-regression");
const VIEWPORTS = [
  { name: "desktop", width: 1440, height: 1100, mobile: false },
  { name: "comment", width: 1248, height: 899, mobile: false },
  { name: "narrow", width: 960, height: 1180, mobile: false },
];

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function waitFor(label, fn, timeoutMs = 20_000, intervalMs = 200) {
  const started = Date.now();
  let lastError = null;
  while (Date.now() - started < timeoutMs) {
    try {
      const value = await fn();
      if (value) return value;
    } catch (error) {
      lastError = error;
    }
    await sleep(intervalMs);
  }
  const detail = lastError ? `: ${lastError.message}` : "";
  throw new Error(`Timed out waiting for ${label}${detail}`);
}

class CdpClient {
  constructor(ws) {
    this.ws = ws;
    this.nextId = 1;
    this.pending = new Map();
    this.handlers = new Map();
    ws.addEventListener("message", (event) => {
      const message = JSON.parse(String(event.data));
      if (message.id) {
        const pending = this.pending.get(message.id);
        if (!pending) return;
        this.pending.delete(message.id);
        if (message.error) {
          pending.reject(new Error(message.error.message || JSON.stringify(message.error)));
          return;
        }
        pending.resolve(message.result || {});
        return;
      }
      const listeners = this.handlers.get(message.method) || [];
      listeners.forEach((listener) => listener(message.params || {}));
    });
    ws.addEventListener("close", () => {
      for (const pending of this.pending.values()) {
        pending.reject(new Error("CDP socket closed"));
      }
      this.pending.clear();
    });
  }

  on(method, handler) {
    const listeners = this.handlers.get(method) || [];
    listeners.push(handler);
    this.handlers.set(method, listeners);
  }

  once(method) {
    return new Promise((resolve) => {
      const handler = (params) => {
        const listeners = this.handlers.get(method) || [];
        this.handlers.set(
          method,
          listeners.filter((entry) => entry !== handler),
        );
        resolve(params);
      };
      this.on(method, handler);
    });
  }

  send(method, params = {}) {
    const id = this.nextId++;
    const payload = { id, method, params };
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.ws.send(JSON.stringify(payload));
    });
  }

  async evaluate(expression) {
    const result = await this.send("Runtime.evaluate", {
      expression,
      returnByValue: true,
      awaitPromise: true,
    });
    if (result.exceptionDetails) {
      throw new Error(result.exceptionDetails.text || "Runtime.evaluate failed");
    }
    return result.result?.value;
  }
}

async function connectCdp(wsUrl) {
  const ws = new WebSocket(wsUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener("open", resolve, { once: true });
    ws.addEventListener("error", reject, { once: true });
  });
  return new CdpClient(ws);
}

async function fetchJson(url) {
  const response = await fetch(url);
  if (!response.ok) {
    throw new Error(`${url} returned HTTP ${response.status}`);
  }
  return response.json();
}

async function detectEdgePath() {
  for (const candidate of EDGE_CANDIDATES) {
    if (!candidate) continue;
    try {
      await import("node:fs/promises").then(({ access }) => access(candidate));
      return candidate;
    } catch (_error) {
      // Try the next path.
    }
  }
  throw new Error("Microsoft Edge not found; set EDGE_PATH to the browser executable.");
}

async function startEdge(port) {
  const edgePath = await detectEdgePath();
  const profileDir = await mkdtemp(path.join(os.tmpdir(), "tokitai-edge-regression-"));
  const child = spawn(
    edgePath,
    [
      "--headless=new",
      "--disable-gpu",
      "--no-first-run",
      "--no-default-browser-check",
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profileDir}`,
      BASE_URL,
    ],
    {
      stdio: "ignore",
      windowsHide: true,
    },
  );
  return { child, profileDir };
}

async function stopEdge(child) {
  if (!child || child.exitCode != null) return;
  if (process.platform === "win32" && child.pid) {
    await new Promise((resolve) => {
      const killer = spawn("taskkill", ["/pid", String(child.pid), "/t", "/f"], {
        stdio: "ignore",
        windowsHide: true,
      });
      killer.once("exit", () => resolve());
      killer.once("error", () => resolve());
    });
    return;
  }
  child.kill("SIGTERM");
  await new Promise((resolve) => {
    child.once("exit", () => resolve());
    setTimeout(resolve, 1500);
  });
}

async function cleanupProfile(profileDir) {
  for (let attempt = 0; attempt < 6; attempt += 1) {
    try {
      await rm(profileDir, { recursive: true, force: true });
      return;
    } catch (error) {
      if (error?.code !== "EBUSY") throw error;
      await sleep(400 * (attempt + 1));
    }
  }
  await rm(profileDir, { recursive: true, force: true });
}

async function setupPage(client, viewport) {
  await client.send("Page.enable");
  await client.send("Runtime.enable");
  await client.send("DOM.enable");
  await client.send("Network.enable");
  await client.send("Emulation.setDeviceMetricsOverride", {
    width: viewport.width,
    height: viewport.height,
    deviceScaleFactor: 1,
    mobile: Boolean(viewport.mobile),
    screenWidth: viewport.width,
    screenHeight: viewport.height,
  });
  await client.send("Page.navigate", { url: BASE_URL });
  await waitFor(`page shell (${viewport.name})`, async () => {
    const ready = await client.evaluate(
      "document.readyState === 'complete' && !!document.querySelector('.app-shell')",
    );
    return ready;
  });
}

async function click(client, selector) {
  const ok = await client.evaluate(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return false;
    el.click();
    return true;
  })()`);
  if (!ok) {
    throw new Error(`Missing element for click: ${selector}`);
  }
}

async function setInputValue(client, selector, value) {
  const ok = await client.evaluate(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return false;
    el.focus();
    el.value = ${JSON.stringify(value)};
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.dispatchEvent(new Event('change', { bubbles: true }));
    return true;
  })()`);
  if (!ok) {
    throw new Error(`Missing input: ${selector}`);
  }
}

async function takeScreenshot(client, fileName) {
  const result = await client.send("Page.captureScreenshot", {
    format: "png",
    captureBeyondViewport: true,
    fromSurface: true,
  });
  await writeFile(path.join(OUT_DIR, fileName), Buffer.from(result.data, "base64"));
}

async function openSearchGitHubPreview(client) {
  await click(client, '[data-activity-panel="search"]');
  await waitFor("search panel active", async () => {
    const active = await client.evaluate(
      "document.querySelector('#activity-panel-search')?.classList.contains('is-active') === true",
    );
    return active;
  });
  await click(client, '#search-mode-switch [data-value="github"]');
  await waitFor("github mode active", async () => {
    const active = await client.evaluate(`(() => {
      const button = document.querySelector('#search-mode-switch [data-value="github"]');
      const workspace = document.querySelector('#search-workspace');
      return button?.classList.contains('is-active') === true
        && workspace?.classList.contains('is-github-preview') === true;
    })()`);
    return active;
  }, 20_000);
  await waitFor("search health cards", async () => {
    const count = await client.evaluate(
      "document.querySelectorAll('#search-health-strip .search-health-card').length",
    );
    return Number(count) >= 4;
  }, 30_000);
  await client.evaluate("loadSearchHealth({ force: true })");
  await waitFor("resolved search health state", async () => {
    const ready = await client.evaluate(`(() => {
      return [...document.querySelectorAll('#search-health-strip .search-health-card .search-health-detail')]
        .some((el) => (el.textContent || '').trim() && (el.textContent || '').trim() !== '-');
    })()`);
    return ready;
  }, 45_000);
  await client.evaluate(`(() => {
    searchState.githubPreviewSourceIndex = 0;
    return loadGitHubPreview('tokio-rs/mini-redis', 'master', 'src/lib.rs', { sourceIndex: 0 });
  })()`);
  await waitFor("blob preview code block", async () => {
    const ready = await client.evaluate(`(() => {
      const code = document.querySelector('#search-preview-panel .search-preview-code-block .syntax-code-block');
      return !!code && code.textContent.trim().length > 0;
    })()`);
    return ready;
  }, 45_000);
  await waitFor("github preview shell", async () => {
    const ready = await client.evaluate(
      "document.querySelector('#search-workspace')?.classList.contains('is-github-preview') === true",
    );
    return ready;
  }, 45_000);
  await waitFor("github preview commit history", async () => {
    const ready = await client.evaluate(
      "document.querySelectorAll('#search-preview-panel [data-search-github-commit]').length >= 1",
    );
    return ready;
  }, 45_000);
  await click(client, '#search-preview-panel [data-search-github-commit]');
  await waitFor("github preview commit diff", async () => {
    const ready = await client.evaluate(`(() => {
      const rows = document.querySelectorAll('#search-preview-panel .search-preview-diff-shell .review-code-row');
      return rows.length >= 1;
    })()`);
    return ready;
  }, 45_000);
  await client.evaluate(`(() => {
    const baseButtons = [...document.querySelectorAll('#search-preview-panel [data-search-github-compare-base]')];
    const headButtons = [...document.querySelectorAll('#search-preview-panel [data-search-github-compare-head]')];
    const byCommit = new Map();
    for (const button of baseButtons) {
      const commitSha = (button.getAttribute('data-search-github-compare-base') || '').trim();
      if (!commitSha) continue;
      byCommit.set(commitSha, { base: button, head: null });
    }
    for (const button of headButtons) {
      const commitSha = (button.getAttribute('data-search-github-compare-head') || '').trim();
      if (!commitSha) continue;
      const entry = byCommit.get(commitSha) || { base: null, head: null };
      entry.head = button;
      byCommit.set(commitSha, entry);
    }
    const commits = [...document.querySelectorAll('#search-preview-panel [data-search-github-commit]')].map((button) => ({
      sha: (button.getAttribute('data-search-github-commit') || '').trim(),
      selected: button.classList.contains('is-selected'),
    })).filter((entry) => entry.sha);
    const selected = commits.find((entry) => entry.selected) || commits[0] || null;
    const baseCandidate = commits.find((entry) => entry.sha && selected && entry.sha !== selected.sha) || commits[1] || commits[0] || null;
    if (!baseCandidate || !selected) return false;
    const baseButton = byCommit.get(baseCandidate.sha)?.base || null;
    const headButton = byCommit.get(selected.sha)?.head || null;
    if (!baseButton || !headButton) return false;
    baseButton.click();
    return true;
  })()`);
  await waitFor("github preview commit compare", async () => {
    const ready = await client.evaluate(`(() => {
      const activeBase = document.querySelector('#search-preview-panel [data-search-github-compare-base].is-active');
      const activeHead = document.querySelector('#search-preview-panel [data-search-github-compare-head].is-active');
      const files = document.querySelectorAll('#search-preview-panel .search-preview-compare-file');
      const rows = document.querySelectorAll('#search-preview-panel .search-preview-diff-side-row');
      return !!activeBase && !!activeHead && files.length >= 1 && rows.length >= 1;
    })()`);
    return ready;
  }, 45_000);
  await client.evaluate(`(() => {
    searchState.githubPreviewSourceIndex = 0;
    return loadGitHubPreview('tokio-rs/mini-redis', 'master', null, { sourceIndex: 0 });
  })()`);
  await waitFor("github preview repo commit history", async () => {
    const ready = await client.evaluate(`(() => {
      const titles = [...document.querySelectorAll('#search-preview-panel .search-preview-section .search-preview-title')].map((el) => (el.textContent || '').trim());
      const commits = document.querySelectorAll('#search-preview-panel [data-search-github-commit]');
      return titles.some((text) => /repository commit history|全仓库提交历史/i.test(text))
        && commits.length >= 1;
    })()`);
    return ready;
  }, 45_000);
  await click(client, '#search-preview-panel [data-search-github-commit]');
  await waitFor("github preview repo commit diff", async () => {
    const ready = await client.evaluate(`(() => {
      const files = document.querySelectorAll('#search-preview-panel .search-preview-compare-file');
      const rows = document.querySelectorAll('#search-preview-panel .search-preview-diff-side-row');
      return files.length >= 1 && rows.length >= 1;
    })()`);
    return ready;
  }, 45_000);
}

async function seedResearchFixture(client) {
  const fixture = {
    active: true,
    topic: "Layout stress fixture",
    phase: "deliver",
    phase_index: 6,
    phase_total: 6,
    next_phase: "done",
    overall_state: "active",
    review: [
      "Review cards should wrap rather than overlap when long text appears in closure notes.",
      "The workspace must keep summary cards readable while diff excerpts stretch vertically.",
    ],
    reviewer_feedback: {
      current_run_id: "fixture-run-2",
      unresolved_count: 1,
      entries: [
        {
          reviewer: "Program Chair",
          comment: "Clarify how the final claim in discussion is grounded in the verifier bundle and link the revised span back to the rebuttal item.",
          score: 84,
          linked_run_id: "fixture-run-2",
          resolved: false,
        },
        {
          reviewer: "Area Chair",
          comment: "The paper workspace should still surface the before/after manuscript bundle and the checkpoint link without card collisions.",
          score: 91,
          linked_run_id: "fixture-run-1",
          resolved: true,
        },
      ],
    },
    paper_ready: {
      ready: false,
      workflow_present: true,
      reason: "fixture review still has one unresolved feedback item",
      current_run_id: "fixture-run-2",
      unresolved_reviewer_feedback: 1,
      auto_triggered: false,
    },
    paper_workflow: {
      summary: "Fixture workflow with long cards, diff excerpts, and claim anchors for browser layout regression.",
      workflow_profile: "classical_ml",
      workflow_checkpoint_path: ".tokitai/paper-workflows/fixture/workflow_checkpoint.json",
      checkpoint_stage: "paper_ready_evaluated",
      paper_ready: false,
      paper_ready_detail: "Claim coverage failed for one discussion span.",
      paper_pdf_path: ".tokitai/paper-workflows/fixture/paper/paper.pdf",
      paper_latex_path: ".tokitai/paper-workflows/fixture/paper/paper.tex",
      paper_markdown_path: ".tokitai/paper-workflows/fixture/paper/paper.md",
      references_bib_path: ".tokitai/paper-workflows/fixture/paper/references.bib",
      review_response_path: ".tokitai/paper-workflows/fixture/paper/review_response.json",
      rebuttal_markdown_path: ".tokitai/paper-workflows/fixture/paper/rebuttal.md",
      section_diff_path: ".tokitai/paper-workflows/fixture/paper/section_diff.json",
      manuscript_diff_path: ".tokitai/paper-workflows/fixture/paper/manuscript_diff.json",
      manuscript_bundle_before_path: ".tokitai/paper-workflows/fixture/paper/manuscript_before.json",
      manuscript_bundle_after_path: ".tokitai/paper-workflows/fixture/paper/manuscript_after.json",
      section_bundle_before_path: ".tokitai/paper-workflows/fixture/paper/sections_before.json",
      section_bundle_after_path: ".tokitai/paper-workflows/fixture/paper/sections_after.json",
      payload_path: ".tokitai/paper-workflows/fixture/paper/paper_bundle.json",
      result_bundle_path: ".tokitai/paper-workflows/fixture/paper/result_bundle.json",
      revision_execution_plan_path: ".tokitai/paper-workflows/fixture/paper/revision_execution_plan.json",
      revision_execution_trace: {
        executed_sections: [
          {
            feedback_index: 0,
            reverification_scope: ["paper_ready_gate", "discussion_claim_3"],
            rewrite_actions: [
              "Expand the verifier evidence paragraph with explicit metric lineage.",
              "Add a short rebuttal closure note referencing the updated discussion span.",
            ],
            closure_note: "The revised discussion paragraph now cites the verifier bundle and the updated rebuttal entry.",
          },
        ],
      },
      revision_queue_size: 1,
      revision_queue_preview: [
        "discussion -> reinforce final claim with verifier-backed metric lineage and rebuttal pointer",
      ],
      revision_mode: "reviewer_guided_revision",
      auto_revision_applied: true,
      pdf_compile_status: "compiled",
      pdf_compile_detail: "tectonic compiled the fixture PDF successfully.",
      unresolved_reviewer_feedback: 1,
      reviewer_feedback_trace: [
        {
          feedback_index: 0,
          reviewer: "Program Chair",
          comment: "Clarify how the final claim in discussion is grounded in the verifier bundle and link the revised span back to the rebuttal item.",
          closure_state: "open",
          target_sections: ["discussion"],
          reverification_required: true,
        },
      ],
      rebuttal_closure_records: [
        {
          feedback_index: 0,
          response_status: "pending",
          required_followup: "Resolve the last discussion claim grounding mismatch before marking the rebuttal closed.",
        },
      ],
      section_diff_preview: [
        {
          section_id: "discussion",
          title: "Discussion",
          changed: true,
          changed_fields: ["markdown_excerpt", "claim_anchors", "reverification_scope"],
          before: {
            markdown_excerpt: "The prior draft stated the conclusion without citing which verifier artifact proved the final claim.",
            draft_seed: "Discussion seed before rewrite.",
            word_count: 74,
            revision_directive: "Need clearer evidence lineage.",
            reverification_scope: ["paper_ready_gate"],
            claim_anchors: [
              {
                claim_text: "The final discussion claim was under-supported.",
                evidence_refs: [{ source_key: "verification.metrics.top1", required: true }],
              },
            ],
          },
          after: {
            markdown_excerpt: "The revised discussion now binds the conclusion to verifier bundle fields and cites the rebuttal closure note for the same claim span.",
            draft_seed: "Discussion seed after rewrite.",
            word_count: 108,
            revision_directive: "Integrated verifier bundle lineage and rebuttal response mapping.",
            reverification_scope: ["paper_ready_gate", "discussion_claim_3"],
            claim_anchors: [
              {
                claim_text: "The conclusion is now grounded by the verifier bundle and rebuttal response.",
                evidence_refs: [
                  { source_key: "verification.metrics.top1", required: true },
                  { source_key: "review_response.entries.0", required: true },
                ],
              },
            ],
          },
        },
      ],
      manuscript_diff_preview: [
        {
          section_id: "discussion",
          title: "Discussion",
          changed: true,
          changed_fields: ["markdown_excerpt", "word_count"],
          before: {
            markdown_excerpt: "Previous manuscript excerpt missing explicit verifier linkage for the final paragraph.",
            word_count: 74,
            claim_anchors: [
              {
                claim_text: "The previous conclusion lacked explicit evidence.",
                evidence_refs: [{ source_key: "verification.metrics.top1", required: true }],
              },
            ],
          },
          after: {
            markdown_excerpt: "Updated manuscript excerpt explicitly ties the final paragraph to the verifier metrics and the rebuttal closure record, producing a longer but grounded discussion block.",
            word_count: 108,
            claim_anchors: [
              {
                claim_text: "The updated conclusion is grounded by named verifier metrics and rebuttal records.",
                evidence_refs: [
                  { source_key: "verification.metrics.top1", required: true },
                  { source_key: "review_response.entries.0", required: true },
                ],
              },
            ],
          },
        },
      ],
      paper_ready_gate: {
        manuscript_evidence_coverage: {
          checks: [
            {
              check_id: "required_sections_present",
              status: "pass",
              detail: "All required manuscript sections are present in the fixture draft.",
            },
            {
              check_id: "claim_evidence_semantic_alignment",
              status: "fail",
              detail: "One discussion claim still needs tighter grounding to the verifier bundle.",
              evidence: {
                claim_evidence_gate: {
                  checks: [
                    {
                      claim_id: "discussion_claim_3",
                      status: "fail",
                      section_title: "Discussion",
                      claim_text: "The revised conclusion is fully grounded in verifier evidence.",
                      semantic_support_status: "weak",
                      semantic_support_score: 0.58,
                      required_source_count: 2,
                      grounded_required_source_count: 1,
                      claim_relevant_required_item_count: 2,
                      required_item_grounding_target_count: 2,
                      grounded_required_item_count: 1,
                      claim_anchor_overlap: { matched: 1, total: 2 },
                      evidence_overlap: { matched: 1, total: 2 },
                      matched_result_bundle_fields: ["verification.metrics.top1"],
                      matched_result_bundle_values: ["top1=0.884"],
                      grounded_section_span_excerpt: "The revised conclusion now names top1=0.884 but still leaves the rebuttal-linked evidence in a separate sentence.",
                      semantic_failure_reasons: [
                        "rebuttal linkage is present but not yet fully reflected in the final discussion sentence",
                      ],
                      failure_sources: ["review_response.entries.0", "paper_sections.discussion"],
                      manuscript_excerpt: "The final discussion sentence still needs one more explicit evidence pointer.",
                    },
                  ],
                },
              },
            },
          ],
        },
      },
      artifacts: [
        { kind: "pdf", label: "Paper PDF", path: ".tokitai/paper-workflows/fixture/paper/paper.pdf" },
        { kind: "latex", label: "Paper LaTeX", path: ".tokitai/paper-workflows/fixture/paper/paper.tex" },
        { kind: "json", label: "Review Response", path: ".tokitai/paper-workflows/fixture/paper/review_response.json" },
        { kind: "json", label: "Revision Execution Plan", path: ".tokitai/paper-workflows/fixture/paper/revision_execution_plan.json" },
      ],
    },
  };
  await click(client, '[data-activity-panel="nav"]');
  await waitFor("nav panel active", async () => {
    const active = await client.evaluate(
      "document.querySelector('#activity-panel-nav')?.classList.contains('is-active') === true",
    );
    return active;
  });
  await client.evaluate(`(() => {
    const fixture = ${JSON.stringify(fixture)};
    bootstrapData = {
      ...(bootstrapData || {}),
      research: fixture,
    };
    applyWorkspaceMode('research');
    renderResearch(fixture);
    document.querySelector('.sidebar-section-research')?.scrollIntoView({ block: 'start', behavior: 'instant' });
    return true;
  })()`);
  await waitFor("paper workspace rendered", async () => {
    const ok = await client.evaluate(
      "!!document.querySelector('#research-panel .paper-workspace') && !!document.querySelector('#research-panel .paper-review-flow') && !!document.querySelector('#research-panel .reviewer-feedback-panel')",
    );
    return ok;
  });
}

async function collectLayout(client, containerSelector, itemSelector) {
  return client.evaluate(`(() => {
    const container = document.querySelector(${JSON.stringify(containerSelector)});
    if (!container) {
      return { containerSelector: ${JSON.stringify(containerSelector)}, itemSelector: ${JSON.stringify(itemSelector)}, exists: false };
    }
    const crect = container.getBoundingClientRect();
    const items = [...container.querySelectorAll(${JSON.stringify(itemSelector)})].map((el, index) => {
      const rect = el.getBoundingClientRect();
      return {
        index,
        text: (el.textContent || '').trim().slice(0, 120),
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
        width: rect.width,
        height: rect.height,
        scrollWidth: el.scrollWidth,
        clientWidth: el.clientWidth,
        scrollHeight: el.scrollHeight,
        clientHeight: el.clientHeight,
      };
    });
    return {
      containerSelector: ${JSON.stringify(containerSelector)},
      itemSelector: ${JSON.stringify(itemSelector)},
      exists: true,
      containerRect: {
        left: crect.left,
        top: crect.top,
        right: crect.right,
        bottom: crect.bottom,
        width: crect.width,
        height: crect.height,
      },
      items,
    };
  })()`);
}

function assertLayout(group) {
  if (!group.exists) {
    throw new Error(`Missing container: ${group.containerSelector}`);
  }
  const { containerRect, items } = group;
  if (!items.length) {
    throw new Error(`No items found for ${group.itemSelector}`);
  }
  for (const item of items) {
    if (item.width <= 0 || item.height <= 0) {
      throw new Error(`Collapsed card in ${group.itemSelector}: ${item.text}`);
    }
    if (item.left < containerRect.left - 2 || item.right > containerRect.right + 2) {
      throw new Error(`Horizontal overflow in ${group.itemSelector}: ${item.text}`);
    }
    if (item.scrollWidth > item.clientWidth + 4) {
      throw new Error(`Horizontal scroll leak in ${group.itemSelector}: ${item.text}`);
    }
  }
  for (let index = 0; index < items.length; index += 1) {
    for (let other = index + 1; other < items.length; other += 1) {
      const a = items[index];
      const b = items[other];
      const overlapX = Math.min(a.right, b.right) - Math.max(a.left, b.left);
      const overlapY = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (overlapX > 4 && overlapY > 4) {
        throw new Error(`Card overlap in ${group.itemSelector}: "${a.text}" vs "${b.text}"`);
      }
    }
  }
}

async function collectSummary(client) {
  return client.evaluate(`(() => ({
    healthCards: [...document.querySelectorAll('#search-health-strip .search-health-card')].map((card) => card.textContent.trim()),
    previewMeta: document.querySelector('#search-preview-panel .search-preview-meta')?.textContent?.trim() || '',
    previewHasSyntaxSpans: !!document.querySelector('#search-preview-panel .syntax-highlight span'),
    previewHistoryVisible: !!document.querySelector('#search-preview-panel [data-search-github-history=\"back\"]'),
    previewCommitButtons: document.querySelectorAll('#search-preview-panel [data-search-github-commit]').length,
    previewDiffRows: document.querySelectorAll('#search-preview-panel .search-preview-diff-shell .review-code-row').length,
    previewCompareFiles: document.querySelectorAll('#search-preview-panel .search-preview-compare-file').length,
    previewCompareSideRows: document.querySelectorAll('#search-preview-panel .search-preview-diff-side-row').length,
    reviewFlowItems: document.querySelectorAll('#research-panel .paper-review-flow-item').length,
    reviewerFeedbackItems: document.querySelectorAll('#research-panel .reviewer-feedback-item').length,
    workspaceTiles: document.querySelectorAll('#research-panel .paper-workspace-tile').length,
    workspaceViewerSections: document.querySelectorAll('#research-panel .paper-workspace-section-pill').length,
    workspaceViewerClaimCards: document.querySelectorAll('#research-panel .paper-workspace-claim-card').length,
  }))()`);
}

async function collectGitHubWorkspaceGeometry(client) {
  return client.evaluate(`(() => {
    const workspace = document.querySelector('#search-workspace');
    const results = document.querySelector('#search-results');
    const preview = document.querySelector('#search-preview-panel');
    if (!workspace || !results || !preview) return null;
    const style = window.getComputedStyle(workspace);
    const workspaceRect = workspace.getBoundingClientRect();
    const resultsRect = results.getBoundingClientRect();
    const previewRect = preview.getBoundingClientRect();
    return {
      gridTemplateColumns: style.gridTemplateColumns,
      workspace: {
        width: workspaceRect.width,
        height: workspaceRect.height,
      },
      results: {
        left: resultsRect.left,
        right: resultsRect.right,
        top: resultsRect.top,
        bottom: resultsRect.bottom,
        width: resultsRect.width,
        height: resultsRect.height,
      },
      preview: {
        left: previewRect.left,
        right: previewRect.right,
        top: previewRect.top,
        bottom: previewRect.bottom,
        width: previewRect.width,
        height: previewRect.height,
      },
    };
  })()`);
}

async function runViewport(client, viewport) {
  await setupPage(client, viewport);
  await openSearchGitHubPreview(client);
  const searchGroups = [
    await collectLayout(client, "#search-health-strip", ".search-health-card"),
    await collectLayout(client, "#search-preview-panel", ".search-preview-section"),
    await collectLayout(client, "#search-preview-panel .search-preview-commit-list", ".search-preview-commit-item"),
    await collectLayout(client, "#search-preview-panel .search-preview-compare-file-list", ".search-preview-compare-file"),
  ];
  searchGroups.forEach(assertLayout);
  const searchSummary = await collectSummary(client);
  if (!searchSummary.previewHasSyntaxSpans) {
    throw new Error(`Syntax highlighting was not detected in ${viewport.name} GitHub blob preview`);
  }
  if (
    searchSummary.previewCommitButtons < 1
    || searchSummary.previewDiffRows < 1
    || searchSummary.previewCompareFiles < 1
    || searchSummary.previewCompareSideRows < 1
  ) {
    throw new Error(`Commit history, inline diff, or commit compare did not render in ${viewport.name} GitHub preview`);
  }
  const githubWorkspace = await collectGitHubWorkspaceGeometry(client);
  if (!githubWorkspace) {
    throw new Error(`GitHub workspace geometry missing for ${viewport.name}`);
  }
  if (viewport.width >= 1200) {
    const previewDroppedBelowResults = githubWorkspace.preview.top >= githubWorkspace.results.bottom - 4;
    const previewNotToRight = githubWorkspace.preview.left <= githubWorkspace.results.left + 40;
    if (previewDroppedBelowResults || previewNotToRight) {
      throw new Error(
        `GitHub preview dropped below results in ${viewport.name}: ${JSON.stringify(githubWorkspace)}`,
      );
    }
  }
  await takeScreenshot(client, `${viewport.name}-search-github-preview.png`);

  await seedResearchFixture(client);
  await client.evaluate(`(() => {
    document.querySelector('#research-panel .paper-workspace')?.scrollIntoView({ block: 'start', behavior: 'instant' });
    return true;
  })()`);
  await takeScreenshot(client, `${viewport.name}-research-paper-workflow.png`);

  const researchGroups = [
    await collectLayout(client, "#research-panel .paper-workspace-summary-grid", ".paper-workspace-summary-card"),
    await collectLayout(client, "#research-panel .paper-workspace-primary-grid", ".paper-workspace-primary-tile"),
    await collectLayout(client, "#research-panel .paper-workspace-band-grid", ".paper-workspace-tile"),
    await collectLayout(client, "#research-panel .paper-review-flow-list", ".paper-review-flow-item"),
    await collectLayout(client, "#research-panel .paper-review-flow-diff-list", ".paper-review-flow-diff-card"),
    await collectLayout(client, "#research-panel .reviewer-feedback-list", ".reviewer-feedback-item"),
    await collectLayout(client, "#research-panel .reviewer-feedback-form-grid", ".reviewer-feedback-field"),
  ];
  researchGroups.forEach(assertLayout);
  const researchSummary = await collectSummary(client);
  if (
    researchSummary.reviewFlowItems < 1
    || researchSummary.reviewerFeedbackItems < 1
    || researchSummary.workspaceTiles < 3
    || researchSummary.workspaceViewerSections < 1
    || researchSummary.workspaceViewerClaimCards < 1
  ) {
    throw new Error(`Paper workflow fixture did not render expected cards for ${viewport.name}`);
  }
  return {
    viewport,
    searchSummary,
    researchSummary,
  };
}

async function main() {
  await mkdir(OUT_DIR, { recursive: true });
  const port = 9229;
  const { child, profileDir } = await startEdge(port);
  const debugBase = `http://127.0.0.1:${port}`;
  try {
    await waitFor("edge remote debugging", async () => {
      const version = await fetchJson(`${debugBase}/json/version`);
      return version.webSocketDebuggerUrl || null;
    }, 20_000);
    const target = await waitFor("page target", async () => {
      const list = await fetchJson(`${debugBase}/json/list`);
      return list.find((entry) => String(entry.url || "").startsWith(BASE_URL));
    }, 20_000);
    const client = await connectCdp(target.webSocketDebuggerUrl);
    const results = [];
    for (const viewport of VIEWPORTS) {
      results.push(await runViewport(client, viewport));
    }
    const report = {
      baseUrl: BASE_URL,
      outDir: OUT_DIR,
      viewports: results,
    };
    const reportPath = path.join(OUT_DIR, "layout-regression-report.json");
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`);
    console.log(JSON.stringify(report, null, 2));
  } finally {
    await stopEdge(child);
    await cleanupProfile(profileDir);
  }
}

main().catch((error) => {
  console.error(error.stack || error.message || String(error));
  process.exitCode = 1;
});
