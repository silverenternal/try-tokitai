# Atlas quick start

## Requirements

- Stable Rust and Cargo
- A supported model API or local Ollama
- Node.js for frontend wiring checks
- Desktop dependencies listed in [DESKTOP_PLATFORMS.md](DESKTOP_PLATFORMS.md)

## Configure

```sh
cp .env.example .env
```

Set at least `AI_API_URL`, `AI_MODEL`, and the relevant provider key. `.env` is ignored by Git.

## Run

Desktop IDE:

```sh
cargo run --features desktop-shell --bin desktop_wry
```

Other hosts:

```sh
cargo run --release
cargo run --release -- --tui
cargo run --release -- --web
cargo run --release -- --mcp
```

## Verify

```sh
cargo check --lib
cargo test --lib
node tools/test_chat_state_resilience.mjs
node tools/test_research_os_wiring.mjs
node tools/test_research_domains_wiring.mjs
```

Atlas creates local runtime data under `.atlas/`. Build output, provider credentials, sessions,
downloads, and local runtime state are excluded from Git.
