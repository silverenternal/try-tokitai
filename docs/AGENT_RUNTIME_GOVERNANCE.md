# Agent runtime, knowledge base, and tool governance

## Knowledge base and RAG

The workspace knowledge base stores its manifest under `.atlas/knowledge-base`. It accepts PDF, DOCX, PPTX, XLSX, CSV/TSV, Markdown/text, HTML/XML, JSON/YAML/TOML, TeX/BibTeX/SQL, and common source files. Uploading a file with the same logical name creates a new version and archives the old version.

Documents are split at headings, paragraphs, and sentence boundaries, with bounded overlap. Each chunk stores its source location, heading path, extracted entities, token estimate, and a deterministic semantic vector. Retrieval combines BM25 lexical ranking, semantic similarity, reciprocal-rank fusion, and a freshness score. Archived and expired documents are never retrieved; stale documents remain available but are freshness-penalized. Documents become stale after 90 days without verification.

RAG is opt-in per chat turn through the composer `RAG` control. When enabled, only relevant active/stale evidence is injected and the model is instructed to cite the document and location. The same hybrid scorer ranks explicit user memory and Research OS memory so irrelevant memories are not injected.

Governance operations are intentionally limited to:

- `archive`: remove a version from retrieval without deleting it;
- `restore`: return a non-expired version to retrieval;
- `verify`: refresh the review timestamp;
- `metadata`: update owner, tags, and validity dates.

There is no agent-accessible physical delete operation for knowledge sources.

## Sub-agent context contracts

Every delegated call receives an `atlas.subagent-context.v1` object.

- `minimal`: task/call arguments only. No transcript, tool result, or diff is shared.
- `manual`: task plus the exact manually entered facts. Use it for a curated policy or business summary.
- `automatic`: task, the configured recent dialogue window, and recent tool results/diffs. Sensitive values are redacted.
- `llm_generated`: creates the automatic redacted fallback first, then calls an LLM once to compress it into the same schema. The LLM cannot receive raw history; privacy rules are included in the request.

All modes redact credentials, authorization data, payment/card data, private keys, and credential-like long tokens. Planner, reviewer, and specialist sub-agents consume this scoped object rather than the raw conversation. The UI retains only the mode and a bounded task summary in sub-agent activity cards.

## Tool boundaries and examples

Tool definitions are enriched with an execution boundary, efficiency hint, example arguments, and a concurrency class. Examples:

```json
{"query":"refund policy for US customers","limit":6}
```

Use this with `search_knowledge_base`; it is workspace-scoped and returns only active/stale sources.

```json
{"query":"StreamSessionRuntime cancellation","kind":"code","limit":10}
```

Use this with `search_workspace_index` before broad recursive scans.

```json
{"command":"cargo test knowledge_base --lib","timeout_secs":120}
```

Use this for a focused terminal verification. Terminal commands, external actions, destructive changes, remote execution, and writes follow the configured approval boundary.

Deletion tools reject workspace roots, home directories, paths containing parent traversal, globs, and unresolved environment-variable paths. Prefer archive/reversible operations wherever possible.

## Streaming, parallelism, and cancellation

Model output is streamed through both HTTP NDJSON and the desktop bridge. A stop request first cancels any non-streaming synchronous model request for the session, then marks a stream cancelled, denies pending approvals, persists partial reasoning/progress/tool state, and aborts the parent worker. The non-streaming API executes its model work inside a session-registered Tokio task, so interruption stops waiting for the result and prevents a late result from being written back. Spawned sub-agent `JoinSet` futures are dropped with the streaming worker, producing cascade cancellation.

Only independent native read-only tools run concurrently: workspace overview, workspace-index search, knowledge-base search, Research OS snapshot, and read-only research-domain context/workspace queries. Results are reassembled in the model call order. All mutations, terminal commands, external actions, and tools that use the fallback CLI assistant remain serial to preserve ordering, avoid lock contention, and keep approval semantics unambiguous.

## Long tasks and research closure

- Long-task mode persists message, tool, diff, and verifier evidence after every meaningful round. After an application restart, the session can reuse completed evidence instead of replaying the task from zero.
- The autonomous round ceiling is configurable from 16 to 360; research defaults to 180. Stagnation handling switches strategies after 3, 6, and 9 no-progress rounds and exits safely when no new evidence is possible.
- Research completion requires itemized reviewer and hard-verifier approval. Only then may Atlas automatically start the paper workflow.
- The resumable paper workflow checkpoints literature, drafting, revision, verification, output, and PDF stages, and emits the manuscript, appendix, result bundle, review/rebuttal trace, and compilation status.

## Agent slash commands and `/goal`

Slash commands are enabled only in Agent mode. Typing `/` opens an inline command palette with usage and safety descriptions. Common commands include `/goal`, `/plan`, `/review`, `/status`, `/compact`, `/resume`, `/spec`, `/schedule`, `/model`, `/permissions`, `/new`, and `/help`.

`/goal <objective>` uses strict planning, multi-round execution, deterministic verification, and the configured long-task ceiling. It does not broaden authority or disable termination controls. The runtime applies these additional boundaries:

- identical successful read-only calls reuse their prior result instead of executing again;
- an identical failed call is blocked from its next retry until the tool, arguments, or evidence path changes;
- identical high-risk calls cannot repeat in goal mode, and goal mode allows at most three high-risk attempts per turn;
- high-risk actions continue to require the configured approval, path, and rate-limit checks;
- a user stop always cascades to the current worker and child work;
- stagnation changes strategy before exiting with a concrete blocker, so persistence never becomes an infinite no-progress loop.
