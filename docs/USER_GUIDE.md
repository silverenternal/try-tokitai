# Atlas user guide

Atlas combines an agent conversation, workspace explorer, Monaco editor, terminal, Git tools,
research workbenches, and interactive visualizations.

## Workspace

Open a project from the desktop workspace picker or configure `workspace_root`. The explorer can
open, edit, search, review, and save workspace files. Project indexing runs incrementally and skips
generated dependency and build directories.

## Agent workflows

- Chat mode answers questions and performs focused workspace operations.
- Research mode plans, executes, verifies, and records evidence-backed research work.
- Tool calls that modify files or run commands remain scoped to the active workspace and security
  policy.
- Long-running operations surface progress and can be stopped from the activity panel.

## Git

The Git workspace shows status, branches, commits, diffs, and graph history. Fetch, pull, push,
stage, commit, branch, and restore actions use the repository's configured Git credentials.

## Terminal and run/debug

Atlas uses PowerShell on Windows and `$SHELL` (falling back to `/bin/sh`) on macOS and Linux.
Run/debug configurations write logs beneath `.atlas/run-debug/`.

## Research OS

Research OS stores hypotheses, experiments, evidence, negative results, decisions, memory,
timeline events, and publications. See [RESEARCH_OS_USER_GUIDE.md](../RESEARCH_OS_USER_GUIDE.md).

## Local data

Atlas stores workspace state beneath `.atlas/` and application state in the OS-specific data
directory. These paths and `.env` are ignored by Git. Back up a workspace before manually deleting
its local state.

## Troubleshooting

```sh
cargo check --lib
cargo test --lib
```

If the desktop shell fails to start, verify the platform webview dependencies documented in
[DESKTOP_PLATFORMS.md](DESKTOP_PLATFORMS.md). If a model request fails, confirm the provider URL,
model name, and API key in `.env` or the settings screen.
