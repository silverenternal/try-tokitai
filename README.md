# Atlas

Atlas is an agent-native desktop IDE for software engineering and computational research. It
combines chat, code navigation, project indexing, Git workflows, terminals, research workbenches,
scientific objects, evidence tracking, and interactive visualization in one Rust/Wry application.

Repository: `https://github.com/chen-maker999/Atlas`

## Highlights

- Agent-assisted development with streaming responses, tool calls, workspace edits, terminals,
  browser workflows, and Git integration.
- Incremental, parallel project indexing for large codebases.
- Research OS for hypotheses, experiments, evidence, negative results, decisions, and publications.
- Domain workbenches for AI/ML, systems, security, databases, graphics, robotics, scientific
  computing, and other technical fields.
- Native desktop host on Windows, macOS, and Linux, plus web, CLI, TUI, and MCP entry points.

## Quick start

Requirements:

- Stable Rust and Cargo
- A supported model API or a local Ollama installation
- Windows: Microsoft Edge WebView2 Runtime
- Linux: GTK 3 and WebKitGTK development libraries
- macOS: Xcode Command Line Tools

Copy the environment template and configure the providers you need:

```sh
cp .env.example .env
```

PowerShell equivalent:

```powershell
Copy-Item .env.example .env
```

Run the desktop IDE:

```sh
cargo run --bin desktop_wry --features desktop-shell
```

Other hosts:

```sh
cargo run --release
cargo run --release -- --tui
cargo run --release -- --mcp
cargo run --release -- --web
```

## Build and verify

```sh
cargo check --lib
cargo test --lib
cargo build --release --bin desktop_wry --features desktop-shell
```

Platform-specific desktop notes are in [docs/DESKTOP_PLATFORMS.md](docs/DESKTOP_PLATFORMS.md).
Extension authors should use the comprehensive [Atlas IDE SDK reference](docs/SDK.md).

## Repository layout

```text
frontend/                 Desktop and web frontend
src/                      Rust application and agent backend
src/atlas_core/           Versioned scientific object engine
src/research_domains/     Domain registry, workbenches, tasks, and actions
src/research_intelligence/ Planning, execution, query, and recommendation engine
src/research_os/          Research records, evidence, and lineage
crates/                   Internal workspace crates
tools/                    Regression and wiring checks
scripts/                  Build, packaging, and maintenance scripts
docs/                     Architecture and user documentation
```

## Compatibility note

Some Rust dependency names still contain `tokitai`. They are upstream crate API names used by the
tool macro, context engine, MCP integration, and embedded key-value store. These dependency
identifiers are not the Atlas product or repository name.

## Security and local data

`.env`, local Atlas state, logs, downloads, screenshots, and build output are ignored by Git.
Workspace tools validate paths and commands before execution. Never commit provider keys or GitHub
tokens.

## License

MIT OR Apache-2.0
