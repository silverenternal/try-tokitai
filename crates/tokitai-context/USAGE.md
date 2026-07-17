# Atlas-Context Usage Guide

**Version**: 0.1.0  
**License**: MIT OR Apache-2.0

---

## 📚 What is Atlas-Context?

Atlas-Context is a **Git-style parallel context management system** for AI agents. It introduces version control semantics to AI conversation management, enabling:

- **Branch-based context** - Fork, merge, and explore multiple conversation paths simultaneously (O(1) with COW)
- **High-performance storage** - LSM-Tree based FileKV with 54x write performance improvement (92ns vs 5μs)
- **AI-powered workflows** - Intelligent conflict resolution, purpose inference, and merge recommendations
- **Crash recovery** - WAL (Write-Ahead Log) ensures data durability

### Key Innovation

> **First application of Git version control principles to AI conversation context management**

| Traditional Approach | Atlas-Context |
|---------------------|-----------------|
| Linear context | Branch-based parallel contexts |
| No rollback | Full history tracking + time travel |
| Single session | Multi-branch concurrent exploration |
| Manual backup | COW automatic deduplication |

---

## 🚀 Quick Start

### Add to Cargo.toml

```toml
[dependencies]
tokitai-context = "0.1"
```

### Basic Usage (Simple KV Store)

```rust
use tokitai_context::facade::{Context, Layer};

fn main() -> anyhow::Result<()> {
    // Open context store
    let mut ctx = Context::open("./.context")?;
    
    // Store conversation turns with layer-based lifetime
    let hash1 = ctx.store("session-1", b"Hello!", Layer::ShortTerm)?;
    let hash2 = ctx.store("session-1", b"Hi there!", Layer::ShortTerm)?;
    
    // Retrieve by hash
    let item = ctx.retrieve("session-1", &hash1)?;
    println!("Content: {:?}", String::from_utf8_lossy(&item.content));
    
    // Semantic search (SimHash-based)
    let results = ctx.search("session-1", "greeting")?;
    for hit in results {
        println!("Found: {} (score: {:.2})", hit.hash, hit.score);
    }
    
    Ok(())
}
```

---

## 🌿 Git-Style Branch Management

### Core Operations

```rust
use tokitai_context::{ParallelContextManager, ParallelContextManagerConfig};
use tokitai_context::parallel::branch::MergeStrategy;

let config = ParallelContextManagerConfig {
    context_root: std::path::PathBuf::from("./.context"),
    default_merge_strategy: MergeStrategy::SelectiveMerge,
    ..Default::default()
};

let mut manager = ParallelContextManager::new(config)?;

// Create branch from main (O(1) with COW - Copy-on-Write)
manager.create_branch("feature-auth", "main")?;

// Switch to branch (O(1) pointer update)
manager.checkout("feature-auth")?;

// List all branches
let branches = manager.list_branches()?;

// Compare branches
let diff = manager.diff("main", "feature-auth")?;

// Merge with strategy
manager.merge("feature-auth", "main", Some(MergeStrategy::SelectiveMerge))?;

// Time travel to historical state
manager.time_travel("main", "abc123...")?;
```

### Performance Characteristics

| Operation | Complexity | Latency | Mechanism |
|-----------|------------|---------|-----------|
| Create branch | O(n) | ~6ms | COW + symlink |
| Checkout | O(1) | ~2ms | Pointer update |
| Merge | O(n) | ~45ms | diff3 + LCS |
| Storage overhead | - | ~18% | COW deduplication |

### Merge Strategies

| Strategy | Description | Use Case |
|----------|-------------|----------|
| `FastForward` | Direct fast-forward when no conflict | Simple append |
| `SelectiveMerge` | Selectively merge non-conflicting items | Default choice |
| `AIAssisted` | AI-powered conflict resolution | Complex merges |
| `ThreeWayMerge` | Classic 3-way merge with diff3 | Standard Git-style |
| `Ours` | Keep current branch content | Discard other |
| `Theirs` | Accept other branch content | Accept other |

---

## 🤖 AI-Powered Features

Enable the `ai` feature in Cargo.toml:

```toml
[dependencies]
tokitai-context = { version = "0.1", features = ["ai"] }
```

Set your API key:
```bash
export OPENAI_API_KEY="sk-..."
# or
export ANTHROPIC_API_KEY="sk-ant-..."
```

### AI Conflict Resolution

```rust
use tokitai_context::facade::{Context, AIContext};
use tokitai_context::ai::clients::OpenAIClient;
use std::sync::Arc;

let mut ctx = Context::open("./.context")?;
let llm = Arc::new(OpenAIClient::from_env());
let mut ai_ctx = AIContext::new(&mut ctx, llm);

// Resolve conflicts with AI semantic understanding
let response = ai_ctx.resolve_conflict(
    "conflict-id",
    "feature",
    "main",
    "Content from feature branch",
    "Content from main branch"
).await?;

println!("Decision: {:?}", response.decision);
println!("Reasoning: {}", response.reasoning);
println!("Confidence: {:.0}%", response.confidence * 100.0);
```

### Branch Purpose Inference

```rust
let purpose = ai_ctx.infer_branch_purpose("feature-auth").await?;
println!("Type: {:?}", purpose.branch_type);
println!("Purpose: {}", purpose.purpose);
println!("Confidence: {:.0}%", purpose.confidence * 100.0);
```

### Merge Recommendations

```rust
let rec = ai_ctx.get_merge_recommendation("feature", "main").await?;
println!("Should merge: {}", rec.recommend_merge);
println!("Recommended strategy: {:?}", rec.recommended_strategy);
println!("Risk level: {} ({:.0}%)", 
    rec.risk_assessment.risk_level, 
    rec.risk_assessment.risk_score * 100.0
);
```

### Branch Summarization

```rust
let summary = ai_ctx.summarize_branch("feature-auth").await?;
println!("Title: {}", summary.title);
println!("Completion: {:.0}%", summary.status_assessment.completion_ratio * 100.0);
println!("Quality score: {}/10", summary.status_assessment.quality_score);
println!("Next steps: {:?}", summary.next_steps);
```

---

## 📦 Storage Layers

Atlas-Context provides three storage layers with different lifetime semantics:

| Layer | Lifetime | Use Case |
|-------|----------|----------|
| `Transient` | Session lifetime | Temporary working data |
| `ShortTerm` | Last N rounds (default 10) | Recent conversation turns |
| `LongTerm` | Permanent | System prompts, rules, configs |

```rust
use tokitai_context::facade::Layer;

// Transient: deleted on session cleanup
ctx.store("session-1", b"Draft message", Layer::Transient)?;

// ShortTerm: auto-trimmed to N rounds
ctx.store("session-1", b"User query", Layer::ShortTerm)?;

// LongTerm: permanent storage
ctx.store("session-1", b"System prompt", Layer::LongTerm)?;
```

---

## ⚡ Performance Benchmarks

### Single Write Latency

| Backend | Latency | Improvement |
|---------|---------|-------------|
| FileKV (LSM-Tree) | 92ns | 54x faster than target |
| Default (file-based) | 5μs | Baseline |

### Merge Performance

| Algorithm | Latency | Improvement |
|-----------|---------|-------------|
| diff3 + LCS | <0.01s | 6000x vs naive |
| Three-way merge | ~0.1s | Baseline |

### Branch Operations

| Operation | Latency | Notes |
|-----------|---------|-------|
| Fork branch | ~6ms | O(1) with COW |
| Checkout | ~2ms | Pointer update |
| Storage overhead | ~18% | COW deduplication |

### Enable High-Performance Backend

```rust
use tokitai_context::facade::{Context, ContextConfig};

let config = ContextConfig {
    enable_filekv_backend: true,      // LSM-Tree backend
    memtable_flush_threshold_bytes: 4 * 1024 * 1024,  // 4MB
    block_cache_size_bytes: 64 * 1024 * 1024,         // 64MB cache
    ..Default::default()
};

let mut ctx = Context::open_with_config("./.context", config)?;
```

---

## 🔧 Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `default` | Enables WAL | - |
| `wal` | Write-Ahead Log for crash recovery | - |
| `ai` | AI-powered features | `reqwest`, `jsonschema` |
| `benchmarks` | Performance benchmarking suite | `criterion` |
| `distributed` | Distributed coordination with etcd | `etcd-client` |
| `fuse` | FUSE filesystem interface | `fuser` (requires system lib) |
| `metrics` | Prometheus metrics export | `prometheus` |
| `full` | All features enabled | All of the above |

---

## 📁 Directory Structure

After running, the following structure is created:

```
.context/
├── branches/           # Branch metadata
├── graph.json          # Context graph (DAG)
├── merge_logs/         # Merge history
├── checkpoints/        # Snapshots for recovery
├── cow_store/          # Copy-on-Write storage
├── filekv/
│   ├── segments/       # LSM-Tree segments
│   ├── wal/            # Write-Ahead Log
│   └── index/          # Index files
└── index/              # Semantic index
```

---

## 🛠️ Common Patterns

### Pattern 1: Multi-Session Management

```rust
let mut ctx = Context::open("./.context")?;

// Manage multiple users with isolated contexts
ctx.store("user-1", b"User 1 message", Layer::ShortTerm)?;
ctx.store("user-2", b"User 2 message", Layer::ShortTerm)?;

// Retrieve specific session's history
let history = ctx.retrieve_range("user-1", start, end)?;
```

### Pattern 2: Explore Multiple Solutions

```rust
let mut manager = ParallelContextManager::new(config)?;

// Main: baseline approach
manager.checkout("main")?;
// ... baseline conversation ...

// Branch 1: JWT auth
manager.create_branch("jwt-auth", "main")?;
manager.checkout("jwt-auth")?;
// ... JWT implementation discussion ...

// Branch 2: Session auth
manager.create_branch("session-auth", "main")?;
manager.checkout("session-auth")?;
// ... Session implementation discussion ...

// Compare and choose the best approach
manager.merge("jwt-auth", "main", None)?;
manager.delete_branch("session-auth")?;
```

### Pattern 3: Long-Running Project

```rust
let mut ctx = Context::open("./.context")?;

// Store project rules (permanent)
ctx.store("project", b"Written in Rust", Layer::LongTerm)?;
ctx.store("project", b"Follow Rust guidelines", Layer::LongTerm)?;

// Store current task (short-term)
ctx.store("task", b"Implement login feature", Layer::ShortTerm)?;

// Search related rules semantically
let rules = ctx.search("project", "guidelines")?;
```

---

## 🐛 Troubleshooting

### Issue: "Failed to open context store"

**Cause**: Directory permissions or disk space

**Solution**:
```bash
# Check permissions
ls -la ./.context

# Check disk space
df -h

# Remove and recreate
rm -rf ./.context
```

### Issue: High memory usage

**Cause**: BlockCache or MemTable too large

**Solution**:
```rust
let config = ContextConfig {
    block_cache_size_bytes: 32 * 1024 * 1024,  // Reduce to 32MB
    memtable_flush_threshold_bytes: 2 * 1024 * 1024,  // Reduce to 2MB
    ..Default::default()
};
```

### Issue: AI features not working

**Cause**: Missing `ai` feature or API key

**Solution**:
```toml
# Cargo.toml
tokitai-context = { version = "0.1", features = ["ai"] }
```

```bash
# Set API key
export OPENAI_API_KEY="sk-..."
```

---

## 📚 Examples

See the `examples/` directory for complete working examples:

| Example | Description |
|---------|-------------|
| `ai_workflow.rs` | Complete AI-powered workflow |
| `ai_conflict_resolution.rs` | AI conflict resolution demo |
| `ai_purpose_inference.rs` | Branch purpose inference |
| `aliyun_coding_plan.rs` | Alibaba Cloud Coding Plan integration |

Run examples:
```bash
# Basic example
cargo run --example ai_workflow --features ai

# With your API key
OPENAI_API_KEY=sk-... cargo run --example ai_workflow --features ai
```

---

## 📖 More Resources

- **GitHub**: https://github.com/chen-maker999/Atlas
- **API Docs**: https://docs.rs/tokitai-context
- **Issues**: https://github.com/chen-maker999/Atlas/issues

---

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

---

**Version**: 0.1.0  
**Published**: 2026
