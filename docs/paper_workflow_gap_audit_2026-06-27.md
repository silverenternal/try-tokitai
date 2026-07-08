# Paper Workflow Gap Audit (2026-06-27)

## Scope

This audit covers the current IDE and paper-production chain in `D:\try-tokitai`, with emphasis on whether the workflow is actually closed rather than merely exposed by UI or tool entrypoints.

Reviewed files:

- `frontend/index.html`
- `frontend/app.js`
- `frontend/styles.css`
- `src/web.rs`
- `src/host.rs`
- `src/scientist/workflow/paper_workflow.rs`
- `src/scientist/agents/report.rs`
- `src/scientist/agents/verification.rs`
- `src/scientist/tools/literature.rs`
- `src/scientist/tools/data.rs`
- `tests/`

## Confirmed Closed Or Improved

### Reviewer revision execution artifact

- Reviewer-driven revision planning is now emitted as a first-class paper artifact:
  - `revision_execution_plan.json`
- The artifact includes:
  - `section_rewrite_queue`
  - `shared_repair_actions`
  - `open_verification_gaps`
  - `rebuttal_closure_records`
  - `execution_protocol`
- The web sidecar now exposes:
  - `revision_execution_plan_path`
  - `revision_queue_size`
  - `revision_queue_preview`
- The IDE paper artifact panel can now surface revision execution state directly instead of requiring manual JSON inspection.

Relevant files:

- `src/scientist/workflow/paper_workflow.rs`
- `src/web.rs`
- `frontend/app.js`
- `frontend/styles.css`
- `tests/scientist_paper_workflow_e2e_test.rs`

### Dataset retrieval policy

- `tokitai-search` has been fully removed from active code paths and tests.
- Public dataset discovery now resolves directly against:
  - OpenML
  - Hugging Face datasets API
  - Papers With Code dataset pages
  - Kaggle dataset pages
- Dataset retrieval entrypoint is now `official_dataset_databases`.

Relevant files:

- `src/scientist/tools/data.rs`
- `src/web.rs`
- `src/scientist/tools/verification_center.rs`
- `tests/public_dataset_entrypoint_live_test.rs`
- `tests/scientist_workflow_contract_test.rs`

### Paper-ready gate

- `paper_ready` now requires successful PDF compilation (`compiled`).
- `missing_toolchain` no longer counts as paper-ready.

Relevant files:

- `src/scientist/workflow/paper_workflow.rs`
- `tests/scientist_paper_workflow_e2e_test.rs`

### Paper evidence traces

- Paper outputs now include:
  - `reviewer_feedback_trace`
  - `evidence_trace`
- These are written into:
  - `review_response.json`
  - `paper_sections.json`

Relevant files:

- `src/scientist/agents/report.rs`
- `src/scientist/workflow/paper_workflow.rs`

### Paper search health now uses real upstream probes

- Paper search health is no longer hardcoded ready.
- The web sidecar now reports provider-level probe state for official paper APIs:
  - Semantic Scholar
  - OpenAlex
  - arXiv
  - Crossref
  - OpenReview
- The IDE search health strip renders both aggregate status and provider rows from the sidecar payload.

Relevant files:

- `src/scientist/tools/literature.rs`
- `src/web.rs`
- `frontend/app.js`
- `frontend/styles.css`

### Workflow checkpoint/resume now includes interruption-recovery coverage

- The workflow already had staged checkpoint/resume in code.
- The missing proof point was "resume after interruption" rather than only fresh-run success.
- E2E coverage now explicitly simulates interruption by:
  - rewinding `workflow_checkpoint.json` from a later stage back to `report_initial_ready`
  - clearing downstream paper workflow fields that should be recomputed
  - deleting downstream artifacts that should be rematerialized
  - deleting the local `papers_dir` before the second run
- The second run is then required to recover from checkpoint and finish again.

Relevant files:

- `tests/scientist_paper_workflow_e2e_test.rs`

### GitHub preview now supports full blob preview and syntax highlighting

- GitHub preview is no longer limited to snippets.
- The preview payload now returns:
  - full decoded file content
  - language hint
  - retained snippet
  - blob/raw URLs
- The IDE preview panel now renders:
  - full blob content
  - syntax-highlighted code block
  - README content when present
  - lightweight preview back/forward history inside the panel

Relevant files:

- `src/scientist/tools/github.rs`
- `frontend/app.js`
- `frontend/styles.css`

## 2026-06-27 Follow-up Update

This round closed three previously open gaps and tightened the manuscript-readiness contract.

### Newly closed in code

1. `paper_ready` is no longer only a status aggregation flag.
   - The workflow now emits a structured `paper_ready_gate` bundle.
   - The gate checks manuscript-level evidence coverage rather than only unresolved feedback / checklist / skipped tools / PDF status.
   - Current checks include:
     - required manuscript sections present
     - non-placeholder draft section substance
     - abstract / setup / results evidence-anchor coverage
     - artifact appendix consumption and lineage / reviewer / verification integration
     - reviewer feedback to rebuttal closure alignment
     - verification bundle consumption and final artifact coverage

2. IDE paper panel now consumes and visualizes review closure state.
   - `reviewer_feedback_trace`
   - `rebuttal_closure_records`
   - `revision_execution_trace`
   - `paper_ready_gate`
   are all exposed through the web sidecar and rendered in the paper workflow panel.

3. GitHub preview panel now supports recursive inspection.
   - click directory entries to drill down
   - click file entries to load file preview
   - open repo / blob / raw / README directly
   - show hit snippets from the originating GitHub search result in the preview pane instead of destabilizing result cards

### Files updated in this round

- `src/scientist/workflow/paper_workflow.rs`
- `src/web.rs`
- `frontend/app.js`
- `frontend/styles.css`
- `frontend/index.html`

### Validation completed

- `node --check frontend/app.js`
- `cargo check -q --all-targets`
- `cargo test --test scientist_paper_workflow_e2e_test -- --nocapture`
- `cargo test paper_workflow_sidecar_roundtrip -- --nocapture`
- `cargo test research_payload_includes_saved_paper_workflow_payload -- --nocapture`

### Remaining notable gaps after this round

1. The manuscript evidence gate is now real, but still heuristic.
   - It validates evidence coverage shape and consumption contracts, not semantic correctness of every prose claim against every result field.
   - A stronger next step would align per-section generated prose spans with concrete result bundle fields / lineage keys / verification items.

2. Reviewer-driven rewrite is now auto-executed and visualized, but section rewriting is still represented as workflow-driven structured closure rather than literal section-diff regeneration with before/after manuscript deltas surfaced in IDE.

3. GitHub preview is now recursive and actionable, but it still does not render inline file diffs, syntax-highlighted blobs, or multi-file breadcrumb history within the preview panel.

## 2026-06-27 Manuscript Diff Update

This round closed the remaining "structured closure only" gap for revision inspection.

### Newly closed in code

1. Paper workflow now emits manuscript-level before/after/diff artifacts derived from real `markdown_draft` sections.
   - New artifacts:
     - `paper_manuscript.sections.before.json`
     - `paper_manuscript.sections.after.json`
     - `paper_manuscript.diff.json`
   - The diff is section-aware and preserves:
     - `markdown_text`
     - `word_count`
     - `claim_anchors`
     - reviewer-linked changed section mapping

2. Web sidecar now exposes manuscript diff payload fields.
   - Added:
     - `manuscript_bundle_before_path`
     - `manuscript_bundle_after_path`
     - `manuscript_diff_path`
     - `manuscript_diff_preview`

3. IDE review flow now prefers manuscript-level before/after excerpts.
   - The review closure surface shows:
     - section rewrite status
     - changed field hints
     - before/after manuscript excerpts
     - before/after word counts
     - claim anchors beside the diff
   - The paper workspace also links manuscript before/after/diff artifacts directly.

### Files updated in this round

- `src/scientist/workflow/paper_workflow.rs`
- `src/web.rs`
- `frontend/app.js`
- `frontend/styles.css`
- `frontend/index.html`
- `tests/scientist_paper_workflow_e2e_test.rs`

### Validation completed

- `node --check frontend/app.js`
- `cargo check -q --all-targets`
- `cargo test --test scientist_paper_workflow_e2e_test -- --nocapture`

### Remaining notable gaps after this update

1. Manuscript diff is now based on real markdown sections, but not yet line-level or inline semantic diff.
   - The workflow emits excerpted before/after bodies and changed fields, not token/line patches inside the prose.

2. `paper_ready` is claim-aware, but claim coverage still validates claim-to-evidence structure rather than natural-language entailment over final prose spans.

3. Unified paper workspace is now linked around manuscript/review/gate/rebuttal artifacts, but it is still a launch surface rather than a fully embedded synchronized viewer.

## 2026-06-27 Recovery, GitHub Preview, And Browser Regression Update

This update supersedes the earlier "still open" claims about paper-search health, GitHub blob preview, and workflow partial recovery.

### Newly closed in code

1. Paper workflow interruption recovery is now covered by E2E.
   - Added `scientist_paper_workflow_recovers_from_checkpoint_after_interruption`.
   - The test rewinds a later-stage checkpoint, clears downstream workflow state, deletes rematerializable artifacts, removes the local paper source directory, and verifies that a second run resumes and closes the workflow again.
   - A shared mutex guard now serializes the two paper-workflow E2E tests so they do not trample the same environment.

2. GitHub preview now renders complete blob content in-panel with syntax highlighting.
   - `search.github_preview` returns full decoded blob text plus language metadata.
   - The IDE preview panel renders the full blob and README with highlighted code blocks.
   - The panel now tracks preview back/forward history across repo/path transitions.

3. Browser-level layout regression evidence now exists for search and paper workflow surfaces.
   - Added `tests/browser_layout_regression.mjs`.
   - The script launches headless Edge via CDP against `http://127.0.0.1:3001`.
   - It validates, in both `1440x1100` and `960x1180` viewports:
     - search health cards stay non-overlapping
     - GitHub preview sections stay non-overlapping under full blob content
     - syntax highlighting is present in blob preview
     - paper workspace summary/primary/band cards stay non-overlapping
     - review-flow cards and diff cards stay non-overlapping
     - reviewer-feedback list/form cards stay non-overlapping
   - Output artifacts:
     - `target/browser-regression/layout-regression-report.json`
     - `target/browser-regression/desktop-search-github-preview.png`
     - `target/browser-regression/narrow-search-github-preview.png`
     - `target/browser-regression/desktop-research-paper-workflow.png`
     - `target/browser-regression/narrow-research-paper-workflow.png`

### Validation completed

- `node --check frontend/app.js`
- `node --check tests/browser_layout_regression.mjs`
- `cargo check -q --all-targets`
- `cargo test --test scientist_paper_workflow_e2e_test -- --nocapture`
- `node tests/browser_layout_regression.mjs`
- Browser regression report confirmed:
  - real search-health probe text rendered from upstream provider status
  - GitHub preview rendered a full Rust blob with syntax highlighting and preview history
  - paper workspace / review flow / reviewer feedback cards stayed layout-stable in both tested viewports

### Current highest-priority remaining gaps

1. `paper_ready` is now stronger than localized span overlap, but still not full entailment-grade claim grounding.
   - The gate now emits `claim_sentence_alignments`, turning each claim into a claim-unit -> grounded-sentence -> evidence trace inside the best localized span.
   - Numeric mismatches, direction/polarity conflicts, and partial claim-unit grounding now feed `entailed` / `supported` / `mixed` / `contradicted` / `unsupported`.
   - This is still heuristic semantic grounding, not a trained NLI entailment/contradiction model over arbitrary prose.

2. GitHub preview now supports arbitrary commit-vs-commit compare inside the IDE panel.
   - The preview already had full blob, README, recursive tree drill-down, file-scoped commit history, and parent diff.
   - It now also supports:
     - Base / Head commit selection in-panel
     - multi-file compare payloads from the official GitHub compare API
     - panel-internal side-by-side diff rendering for compare files
   - Still missing:
     - repo-wide history exploration beyond the selected file scope
     - true side-by-side diff for the single-commit parent-diff surface

3. Paper workspace is now a synchronized section viewer, but still not a full embedded manuscript editor/viewer.
   - The workspace now keeps section pills, current manuscript excerpt, review/rebuttal closure cards, claim gate cards, and before/after diff panes synchronized on one selected section.
   - Review-flow diff cards can jump directly into the corresponding section in the workspace viewer.
   - It is still not a fully in-place manuscript authoring surface with synchronized cursor/file view.

## 2026-06-27 Localized Claim Grounding Tightening Update

This follow-up tightens the current `paper_ready` gate without overstating it as entailment or NLI.

### Newly tightened in code

1. Required evidence grounding is now item-aware inside a localized manuscript span.
   - Structured evidence refs with multiple items no longer pass merely because one field name appears in the same section.
   - For required refs, the gate now computes claim-relevant evidence items and requires those items to ground inside the same local paragraph/sentence window before the source counts as grounded.
   - Structured `field_name + field_value` items no longer count as grounded from field-name mention alone; the span must also ground the concrete value/text bundle.

2. Gate payload now exposes stronger debugging signals for IDE inspection.
   - Added per-claim fields:
     - `grounded_required_item_count`
     - `grounded_required_items`
     - `claim_relevant_required_item_count`
     - `required_item_grounding_target_count`
     - `grounded_section_span_excerpt`

3. The IDE paper-ready claim cards now surface same-span grounding counts.
   - Claim cards show:
     - same-span grounded required sources
     - same-span grounded evidence items
     - the best localized grounding span excerpt

### Files updated in this follow-up

- `src/scientist/workflow/paper_workflow.rs`
- `frontend/app.js`
- `tests/browser_layout_regression.mjs`

### Validation completed

- `cargo test --lib claim_grounding -- --nocapture`
- `cargo test --lib paper_ready_requires_compiled_pdf -- --nocapture`
- `node --check frontend/app.js`
- `node tests/browser_layout_regression.mjs`

### What still remains open

1. This is still not full entailment-grade semantic support.
   - The gate now runs heuristic claim-unit -> grounded-sentence semantic checks, but it does not use a dedicated NLI model.

2. GitHub preview now has arbitrary commit-vs-commit compare, but still stays file-scoped for commit picking.
   - The compare panel can render multi-file side-by-side diffs in-panel.
   - It still does not expose a repo-wide commit browser independent of the selected file path.

3. The paper workspace is now a synchronized review viewer, not only a launch surface.
   - The remaining gap is an embedded manuscript editing/viewing workbench rather than synchronization itself.

## 2026-06-27 GitHub Commit History And Diff Update

This follow-up closes the earlier in-panel history/diff gap for GitHub preview.

### Newly closed in code

1. `search.github_preview` now returns file-scoped commit history.
   - The preview payload can resolve a selected blob against either a branch head or a specific commit SHA.
   - For the selected file path, the payload now includes recent commit rows with:
     - `sha`
     - `short_sha`
     - `subject`
     - `author`
     - `date`
     - commit URL

2. The preview payload now includes single-commit inline diff data.
   - When a commit is selected, the backend fetches commit detail from the official GitHub API and extracts the matching file patch.
   - The payload exposes:
     - file change status
     - additions / deletions / changes
     - parent SHA
     - parsed diff hunks with per-line old/new numbering

3. The IDE GitHub preview panel now supports:
   - clicking a file-scoped commit history row
   - loading that file revision in-panel
   - rendering inline diff against the parent commit
   - returning from commit view to branch-head view
   - preserving commit selection in preview back/forward history and path navigation

### Files updated in this follow-up

- `src/scientist/tools/github.rs`
- `src/web.rs`
- `frontend/app.js`
- `frontend/styles.css`
- `tests/browser_layout_regression.mjs`

### Validation completed

- `cargo test --lib github_patch_parser -- --nocapture`
- `node --check frontend/app.js`
- `node --check tests/browser_layout_regression.mjs`
- `node tests/browser_layout_regression.mjs`

### What still remains open after this update

1. `paper_ready` still is not entailment-grade claim grounding.

2. GitHub preview still does not support arbitrary commit-vs-commit compare, repo-wide commit diff browsing, or side-by-side diff.

## 2026-06-27 Search Preview Layout Follow-up

This follow-up closes the browser-reported GitHub search preview layout regression where the preview pane was being pushed below the result list inside the search flyout.

### Newly closed in code

1. GitHub search preview now uses a widened search flyout on desktop-width viewports.
   - The root cause was container-level: the search surface still lived inside the left activity flyout, whose saved width could remain near the old `304px` default even when the overall app viewport was wide enough for side-by-side inspection.
   - The frontend now computes an effective flyout width for active GitHub search preview mode while preserving the user's base flyout preference for other panels.

2. The search workspace now keeps result cards and preview sections side-by-side at comment-sized desktop widths.
   - `#search-workspace.is-github-preview` now uses a wider dual-column track definition.
   - The existing single-column fallback remains for narrow viewports.

3. Browser regression coverage now includes the exact comment-class viewport.
   - Added a `1248x899` viewport run to `tests/browser_layout_regression.mjs`.
   - Added a geometry assertion that the GitHub preview panel must remain to the right of the result list rather than dropping below it on desktop-width layouts.

### Files updated in this round

- `frontend/app.js`
- `frontend/styles.css`
- `tests/browser_layout_regression.mjs`

### Validation completed

- `node --check frontend/app.js`
- `node --check tests/browser_layout_regression.mjs`
- `node tests/browser_layout_regression.mjs`

### Remaining notable gaps after this update

1. `paper_ready` now includes heuristic claim-unit -> grounded-sentence semantic grounding, but still is not full entailment/contradiction-grade NLI over final prose.

2. GitHub preview now supports arbitrary commit-vs-commit compare, multi-file compare diff browsing, and side-by-side compare rendering in-panel.
   - Remaining GitHub gap: broader repo-history browsing and richer diff modes for non-compare paths.

3. The paper workspace is now a synchronized manuscript-review viewer around one selected section.
   - Remaining workspace gap: in-place manuscript viewing/editing rather than synchronized read-only review panes.

## 2026-06-27 Sentence-Level Grounding, Compare, And Synchronized Workspace Update

This update closes the three remaining surface gaps without overstating the semantic gate as full NLI.

### Newly closed in code

1. `paper_ready` now exposes sentence-level claim grounding traces.
   - Per claim, the gate now emits `claim_sentence_alignments`.
   - Each alignment records:
     - the claim unit
     - the best grounded sentence inside the localized span
     - relation label
     - support score
     - matched / missing numbers
     - contradiction signals
   - Schema versions advanced to:
     - `paper_claim_evidence_gate_v5`
     - `paper_ready_gate_v7`
     - `paper_ready_gate_bundle_v7`

2. GitHub preview now supports in-panel arbitrary compare.
   - Added Base / Head actions to file-scoped commit history rows.
   - Compare requests now flow through `src/web.rs` into the official GitHub compare API backend.
   - The preview panel now renders:
     - compare summary
     - multi-file compare cards
     - side-by-side diff rows

3. Paper workspace now acts as a synchronized section viewer.
   - Section pills select one synchronized section.
   - The viewer updates manuscript excerpt, review/rebuttal closure cards, claim gate cards, and before/after diff panes together.
   - Review-flow diff cards can deep-link into the synchronized section selection.

4. Browser regression coverage now includes compare and synchronized workspace surfaces.
   - The regression script now requires:
     - compare files + side-by-side compare rows
     - paper workspace section pills
     - paper workspace claim cards

### Files updated in this round

- `src/scientist/workflow/paper_workflow.rs`
- `tests/scientist_paper_workflow_e2e_test.rs`
- `frontend/app.js`
- `frontend/styles.css`
- `tests/browser_layout_regression.mjs`

### Validation target for this update

- `cargo check -q --all-targets`
- `cargo test claim_grounding_fails_when_grounded_numbers_contradict_claim -- --nocapture`
- `cargo test --test scientist_paper_workflow_e2e_test -- --nocapture`
- `node --check frontend/app.js`
- `node --check tests/browser_layout_regression.mjs`
- `node tests/browser_layout_regression.mjs`

## 2026-06-27 PDF Toolchain Recovery Update

This follow-up closes the highest-priority workflow hole blocking a real end-to-end paper PDF from the existing paper workflow path.

### Newly closed in code

1. Paper workflow toolchain detection now discovers Codex-bundled `tectonic`.
   - The prior workflow only trusted `PATH` or explicitly passed toolchain values.
   - On this machine, `latex_doctor` proved that a bundled `tectonic.exe` was already present and smoke-testable under the Codex plugin cache, but `run_paper_workflow` still reported `missing_toolchain`.
   - `src/toolchain.rs` now falls back to bundled plugin-cache discovery for supported tools, including `tectonic`, so the web runtime, paper workflow, and other callers inherit the same detection path.

2. Failed PDF compiles no longer poison workflow checkpoint state.
   - `src/scientist/workflow/paper_workflow.rs` previously marked `pdf_compiled` even when `compile_paper_pdf(...)` returned `missing_toolchain` or `failed`.
   - The workflow now records `pdf_compiled` only when `pdf_compile_status == "compiled"` and `paper.pdf` actually exists.
   - When PDF compilation fails, the checkpoint explicitly drops any stale `pdf_compiled` stage and keeps the workflow resumable from the outputs stage.

3. Checkpoint-resume coverage now proves failed-PDF recovery, not only generic interruption recovery.
   - Added `scientist_paper_workflow_retries_pdf_after_failed_compile_checkpoint`.
   - The test runs the workflow once with intentionally missing `tectonic` / `pdflatex`, verifies:
     - `pdf_compile_status == "missing_toolchain"`
     - no `paper.pdf`
     - no `pdf_compiled` stage in `workflow_checkpoint.json`
   - It then reruns the same session with the discovered bundled `tectonic` path and requires:
     - `pdf_compile_status == "compiled"`
     - `paper.pdf` exists
     - `workflow_checkpoint.json` now includes `pdf_compiled`

### Files updated in this follow-up

- `src/toolchain.rs`
- `src/scientist/workflow/paper_workflow.rs`
- `tests/scientist_paper_workflow_e2e_test.rs`

### Validation completed

- `python C:/Users/Administrator/.codex/plugins/cache/openai-bundled/latex/0.2.3/scripts/latex_doctor.py --json`
- `cargo test --test scientist_paper_workflow_e2e_test scientist_paper_workflow_recovers_from_checkpoint_after_interruption -- --nocapture`
- `cargo test --test scientist_paper_workflow_e2e_test scientist_paper_workflow_retries_pdf_after_failed_compile_checkpoint -- --nocapture`
- manual `tectonic` compile against:
  - `D:/Project Testing/.tokitai/paper-workflows/18bceb0073f7b104/paper/paper.tex`
  - output confirmed at `D:/Project Testing/.tokitai/paper-workflows/18bceb0073f7b104/paper/paper.pdf`

### What still remains open

1. The workflow can now produce a real PDF through the existing paper workflow path, but the generated manuscript is not yet “high quality” in the stronger semantic sense the user asked for.
   - The current example still drifts into a systems-evaluation framing for a tiny iris topic.
   - `paper_ready` remains false because claim grounding is stronger localized span grounding, not entailment/contradiction-grade support.

2. The current LaTeX output is compilable, but not yet polished publication-quality typography.
   - The example compile emits overfull box warnings and an empty bibliography section because citations are still template-level rather than article-quality.

## 2026-06-28 Real Runtime Payload And Paper Runner Update

This follow-up adds one reusable non-mock path for driving the existing paper workflow from a real experiment workspace, and records the latest proof points that are now backed by code and tests.

### Newly closed in code

1. Real runtime payload consumption is now explicitly regression-tested.
   - `tests/scientist_paper_workflow_e2e_test.rs` includes:
     - `scientist_paper_workflow_consumes_supplied_runtime_payload_and_invalidates_checkpoint_on_runtime_change`
   - The test proves that:
     - `run_paper_workflow(...)` consumes caller-supplied `runtime_artifact_paths`, `runtime_result_bundle`, `runtime_run_comparison`, and `runtime_lineage`
     - verification binds to `source_workspace_root`
     - changing the runtime payload invalidates the saved checkpoint and rematerializes downstream paper artifacts

2. Revision-stage verification now stays attached to the source workspace.
   - `src/scientist/workflow/paper_workflow.rs` previously passed the paper-workflow workspace into revision verification.
   - It now prefers `request.source_workspace_root` and only falls back to `request.workspace_root` when no source workspace was supplied.

3. Official paper API-only mode is now proved to reject local paper fallback.
   - `src/scientist/tools/literature.rs` already respected `AI_SCIENTIST_DISABLE_LOCAL_PAPER_FALLBACK`.
   - The missing proof point is now covered by:
     - `test_fetch_paper_official_api_mode_does_not_fallback_to_local`
   - This keeps the paper workflow aligned with the "official paper APIs only" source policy when no explicit local paper workspace is requested.

4. A reusable real-experiment runner now exists for paper workflow execution.
   - Added:
     - `src/bin/paper_workflow_runner.rs`
   - The runner:
     - reads a real experiment workspace
     - reuses persisted script / metrics / summary / figure artifacts
     - materializes a dataset split manifest inside the source workspace
     - assembles `runtime_result_bundle`, `runtime_run_comparison`, and `runtime_lineage`
     - calls the existing `run_paper_workflow(...)` path with official paper API search and detected LaTeX toolchains

### Files updated in this follow-up

- `src/bin/paper_workflow_runner.rs`
- `src/scientist/workflow/paper_workflow.rs`
- `src/scientist/tools/literature.rs`
- `tests/scientist_paper_workflow_e2e_test.rs`

### Validation completed in this follow-up

- `cargo test --test scientist_paper_workflow_e2e_test scientist_paper_workflow_consumes_supplied_runtime_payload_and_invalidates_checkpoint_on_runtime_change -- --nocapture`
- `cargo test test_fetch_paper_official_api_mode_does_not_fallback_to_local -- --nocapture`

### What still remains open

1. `paper_ready` still is not entailment/contradiction-grade claim grounding.
   - The current gate is stronger localized claim-to-span-to-evidence grounding with heuristic sentence-level support traces.

2. The paper workspace still is not a fully embedded synchronized manuscript-review editor.
   - The workflow artifacts are linked and viewable, but the surface is still closer to a launch-and-inspect workspace than a full manuscript workbench.

3. GitHub preview still does not provide the full repo-history / diff experience one would expect from a dedicated code review surface.
   - Blob preview and compare coverage are stronger than before, but broader history traversal and richer diff modes remain an open product gap.

## 2026-06-29 Historical Runtime Backfill Update

This follow-up closes a real gap between "paper workflow can compile a PDF" and "paper workflow is fed by the strongest available runtime evidence" for older research sessions.

### Newly closed in code

1. Historical research sessions now backfill runtime evidence from existing workspace experiment artifacts.
   - `src/web.rs` previously derived research runtime evidence almost entirely from the current-turn tool trace.
   - For older sessions whose experiment already existed on disk, `bootstrap` could mark the workflow `complete` while still exposing a weak `result_bundle` such as:
     - `dataset acquisition pending`
     - `paper dataset hints pending`
     - `metric evidence observed`
   - The web runtime now scans the current workspace `experiments/` artifacts (`config.py`, `experiment.py`, `results.csv`, `metrics.csv`) and folds that evidence back into:
     - `artifact_paths`
     - metric facts
     - comparison facts
     - dataset hints
     - error-analysis text evidence

2. Real CSV-derived metric signals now outrank stale placeholder bundle fields.
   - For the current real workspace, `results.csv` now surfaces:
     - `primary_metric = 0.9793`
     - `baseline_delta = +0.0094 over RandomForest at noise_rate=0.0`
     - an explicit error-analysis summary tied to the observed robustness pattern
   - `bootstrap` also now carries `experiments/results.csv` into `result_bundle.artifact_paths`.

3. Regression coverage now proves the historical-session backfill behavior.
   - Added `research_payload_backfills_workspace_results_for_historical_session`.
   - The test creates a workspace with only persisted experiment files and no fresh tool-run trace, then requires:
     - `workflow_kind == classical_ml`
     - `overall_state == complete`
     - `artifact_paths` includes `experiments/results.csv`
     - `paper_dataset_hints == digits`
     - `primary_metric == 0.9793`
     - `baseline_delta` is recovered from the CSV comparison rows

### Files updated in this follow-up

- `src/web.rs`

### Validation completed in this follow-up

- `cargo test research_payload_backfills_workspace_results_for_historical_session --lib`
- `cargo check --features desktop-shell --bin desktop_shell`
- manual `http://127.0.0.1:3001/api/bootstrap` verification after restarting `target/debug/ai-assistant.exe --web`
  - confirmed `result_bundle.artifact_paths` now includes `experiments/results.csv`
  - confirmed `primary_metric = 0.9793`
  - confirmed `baseline_delta = +0.0094 over RandomForest at noise_rate=0.0`

### What still remains open

1. `dataset_manifest` is still not fully materialized from a direct official dataset manifest artifact in this historical session.
   - The runtime now recovers `paper_dataset_hints = digits`, but this session still lacks a persisted direct-database manifest artifact, so `dataset_manifest` remains pending.

2. `paper_ready` still is not entailment/contradiction-grade claim grounding.
   - The gate is stronger than a lexical heuristic, but it still stops short of a true semantic entailment verifier.

## 2026-06-29 Runtime Dataset Canonicalization Update

This follow-up closes a real manuscript-generation leak where stale literature dataset hints could still reappear in the final paper draft even after runtime evidence and the effective benchmark plan had already converged on the actual experiment dataset.

### Newly closed in code

1. Runtime-backed benchmark dataset hints now outrank stale literature-only hints inside the paper workflow.
   - `src/scientist/workflow/paper_workflow.rs` now treats runtime/result-bundle/workspace dataset signals as canonical when deriving `effective_benchmark_plan`.
   - Once runtime signals exist, older literature-only `paper_dataset_hints` are no longer merged back into the effective dataset hint list.

2. Dataset-hint drift now invalidates downstream paper-generation checkpoint stages.
   - When canonical dataset hints change, the workflow now clears:
     - `report_response_initial`
     - `final_report_response`
     - downstream paper-ready / PDF / artifact stages
   - This prevents an already-corrected runtime plan from reusing an older manuscript bundle.

3. Report generation now prefers `benchmark_plan.datasets` over stale free-form hint arrays.
   - `src/scientist/agents/report.rs` now treats structured `benchmark_plan.datasets` as the manuscript’s primary dataset source.
   - `paper_dataset_hints` is only used as a fallback when no structured benchmark dataset is available.
   - This closes the concrete manuscript leak where prose such as `iris; digits (...)` could still appear even though the effective benchmark dataset was already `digits`.

### Validation completed in this follow-up

- `cargo test dataset_mentions_prefer_benchmark_datasets_over_stale_hint_only_entries --lib -- --nocapture`
- `cargo test effective_benchmark_plan_prefers_runtime_profile_and_digits_dataset --lib -- --nocapture`
- `cargo test scientist_paper_workflow_runtime_dataset_overrides_stale_literature_hints_in_manuscript --test scientist_paper_workflow_e2e_test -- --nocapture`
- real rerun via `POST /api/research/paper-workflow` for session `18bd200ccf2ce404`

### Current real-session proof points

For `D:\Project Testing\.tokitai\paper-workflows\18bd200ccf2ce404\paper\paper.md`:

- the abstract now says `grounded in digits (...)`
- the introduction now previews `digits (...)`
- the experimental setup now enumerates only `digits (...)`
- the earlier mixed manuscript phrasing `iris; digits (...)` is no longer present

For the same session’s structured artifacts:

- `paper_bundle.json -> paper_blueprint.paper_dataset_hints`
  - now contains only `digits (...)`
- `workflow_checkpoint.json -> paper_dataset_hints`
  - now contains `["digits"]`
- `workflow_checkpoint.json -> effective_benchmark_plan.datasets[0].dataset_id`
  - remains `digits`

### Remaining notable gaps after this follow-up

1. `paper_ready` still is not entailment/contradiction-grade claim grounding.
   - The gate is stronger than localized span overlap and now carries sentence-level support traces, but it is still heuristic rather than NLI-grade semantic verification.

2. The current real session still reports `paper_ready=false`.
   - The blocker is no longer stale dataset leakage.
   - The remaining failing checks are manuscript claim-evidence semantic alignment and verification-bundle consumption / skipped-tool closure.

3. The paper workspace remains a synchronized manuscript-review viewer rather than a full in-place manuscript editor/workbench.

## 2026-06-29 Claim Grounding Fact-Anchor Update

This follow-up closes a concrete mismatch between what the paper-ready gate was checking and what the manuscript sections actually contain.

### Newly closed in code

1. Section claim anchors now carry a manuscript-facing fact anchor in addition to the instructional claim text.
   - `src/scientist/agents/report.rs` now emits `grounding_text` for each paper section claim anchor.
   - For `title_abstract`, `results.primary_outcome`, and `results.boundary_conditions`, that grounding text is derived from the actual draft/section seed rather than from imperative writing instructions such as `The abstract must summarize ...`.
   - Other sections now fall back to the section seed so the paper-ready gate is checking the closest available fact-shaped manuscript target instead of a prompt-shaped control sentence.

2. Several required evidence refs were rebalanced to match the section that actually carries them.
   - The introduction now requires literature plus benchmark scope, while artifact-path evidence is no longer treated as mandatory same-span support there.
   - The method now requires benchmark artifacts and dataset scope, with raw artifact paths downgraded to optional appendix-style support.
   - Limitations now requires surfaced verifier gaps and skipped tools when they exist, instead of using only a broad verifier summary as the primary required evidence source.

3. The paper-ready claim gate now consumes `grounding_text` when present.
   - `src/scientist/workflow/paper_workflow.rs` now tokenizes and semantically aligns against `claim_anchor.grounding_text` first, falling back to `claim_text` only when no fact anchor is available.
   - The gate output now also persists the consumed `grounding_text` in each claim check for auditability.

### Validation completed in this follow-up

- `cargo test title_abstract_claim_anchor_exposes_fact_grounding_text --lib -- --nocapture`
- `cargo test claim_grounding_prefers_fact_grounding_text_over_instructional_claim_text --lib -- --nocapture`
- `cargo test dataset_mentions_prefer_benchmark_datasets_over_stale_hint_only_entries --lib -- --nocapture`
- `cargo test scientist_paper_workflow_runtime_dataset_overrides_stale_literature_hints_in_manuscript --test scientist_paper_workflow_e2e_test -- --nocapture`
- real rerun via `POST /api/research/paper-workflow` for session `18bd200ccf2ce404`

### Current real-session proof points

For `D:\Project Testing\.tokitai\paper-workflows\18bd200ccf2ce404\workflow_checkpoint.json` after the rerun:

- `paper_ready` remains `false`
- `paper_ready_detail` still fails on:
  - `claim_evidence_semantic_alignment`
  - `verification_bundle_consumption`
- `paper_ready_gate.manuscript_evidence_coverage.claim_evidence_gate.claim_failure_count`
  - is now `9`

This confirms the update reduced prompt-shaped claim checking noise, but it did not finish the remaining manuscript-level evidence closure.

### Remaining notable gaps after this follow-up

1. `paper_ready` is still not entailment / contradiction-grade claim grounding.
   - The gate now checks a better fact anchor, but the remaining failures are still driven by localized heuristic support rather than NLI-grade semantic verification.

2. `verification_bundle_consumption` is still hard-blocked by skipped tools.
   - The current real session still reports `skipped_tools=4`, so paper readiness remains blocked even when PDF compilation and artifact generation succeed.

3. Several manuscript sections still need richer synchronized evidence phrasing if they are to satisfy the current same-span grounding standard.
   - The remaining failures are concentrated in introduction / related work / method / discussion / limitations / conclusion / appendix style sections rather than in runtime-result extraction alone.

## 2026-06-29 Verification Bundle Consumption Fix

This round closed the `verification_bundle_consumption` gate failure for the real session `18bd200ccf2ce404`.

### Root cause confirmed

`manuscript_evidence_coverage_gate` was reading `paper.materialized_artifacts.artifact_appendix_markdown` to check whether the appendix discloses skipped tools and verification gaps.
That field is never populated in production — it only existed in the unit test fixture.
Result: `appendix_discloses_skipped_tools=false` and `appendix_discloses_verification_gaps=false` always, so `verification_bundle_consumption` always failed in real runs.

Secondary issue: the disk-write path (`expected_appendix_markdown = build_appendix_markdown(plan)`) used the cached `final_report_response.paper.artifact_appendix_plan`, which pre-dated the `skipped_tools` key. So `artifact_appendix.md` on disk also lacked the "## Skipped Tools" section.

### Newly closed in code

1. Extracted `appendix_plan_with_vcr_skipped(plan, vcr)` helper.
   - Returns `plan` unchanged when it already has non-empty `skipped_tools`.
   - Otherwise merges `skipped_tools` from `verification_center_repair` (same string format as the rest of the gate).

2. Gate (`manuscript_evidence_coverage_gate`) now derives appendix content via `build_appendix_markdown(&appendix_plan_with_vcr_skipped(...))`.
   - No longer reads from `materialized_artifacts` (removed the stale code path).

3. Disk-write path now uses the same helper.
   - `artifact_appendix.md` now correctly includes "## Skipped Tools" for sessions with non-empty vcr skipped tools, even when the cached plan pre-dates the field.

### Files updated

- `src/scientist/workflow/paper_workflow.rs`

### Validation completed

- `cargo test verification_bundle_consumption --lib -- --nocapture` — 2 tests pass
- `cargo test --test scientist_paper_workflow_e2e_test -- --nocapture` — 7 tests pass
- Real re-run via `POST /api/research/paper-workflow` for session `18bd200ccf2ce404`:
  - `verification_bundle_consumption: pass`
  - `appendix_discloses_skipped_tools: true`
  - `appendix_discloses_verification_gaps: true`
  - `artifact_appendix.md` now contains "## Skipped Tools" with all 4 tools listed

### Remaining notable gaps after this fix

1. `paper_ready` is still `false` — the sole remaining gate failure is `claim_evidence_semantic_alignment`.
   - Heuristic localized span grounding, not NLI-grade entailment/contradiction.

2. `quality_checklist.verification_center_bundle_closure` is permanently `needs_attention` whenever any tools are skipped.
   - This contributes to `quality_items_needing_attention=2` in `compute_paper_ready_status`, which independently blocks `paper_ready=true` even when `verification_bundle_consumption` passes.
   - Closing this would require either recovering/replacing all skipped tools, or adding a "disclosed and acknowledged" satisfaction path to the checklist.

3. GitHub preview and paper workspace gaps remain unchanged (see earlier entries).
