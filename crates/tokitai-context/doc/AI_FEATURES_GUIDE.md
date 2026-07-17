# AI Features Guide

**Last updated**: 2026-04-04
**Version**: 1.1.0
**Feature flag**: `ai`

---

## 🎯 Overview

Atlas-Context provides AI-powered features to enhance context management with intelligent automation:

- **AI Conflict Resolution**: Automatically resolve merge conflicts using LLMs
- **Branch Purpose Inference**: Auto-detect and label branch purposes
- **Smart Merge Recommendations**: Get AI-powered merge timing and strategy advice
- **Branch Summarization**: Generate human-readable branch summaries

---

## 📦 Installation

Enable the `ai` feature in your `Cargo.toml`:

```toml
[dependencies]
tokitai-context = { version = "0.1.0", features = ["ai"] }
```

This adds `reqwest` and `jsonschema` as dependencies for HTTP communication with LLM providers.

---

## 🔑 Configuration

### Using `.env` File (Recommended)

```bash
# Copy the template
cp .env.example .env

# Edit .env and add your API keys
```

**Example `.env` file**:
```bash
# Alibaba Cloud Coding Plan (阿里云百炼 AI 编码套餐)
ALIYUN_CODING_PLAN_API_KEY="your-coding-plan-api-key"
# or
DASHSCOPE_API_KEY="your-dashscope-api-key"

# OpenAI
OPENAI_API_KEY="sk-xxxxxxxxxxxxxxxx"

# Anthropic
ANTHROPIC_API_KEY="sk-ant-xxxxxxxxxxxxxxxx"

# Ollama (Self-hosted)
OLLAMA_BASE_URL="http://localhost:11434"
OLLAMA_MODEL="llama3.1"
```

### Using Environment Variables

```bash
# Alibaba Cloud Coding Plan
export ALIYUN_CODING_PLAN_API_KEY="your-api-key"

# OpenAI
export OPENAI_API_KEY="sk-xxxxxxxxxxxxxxxx"

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-xxxxxxxxxxxxxxxx"

# Ollama
export OLLAMA_BASE_URL="http://localhost:11434"
export OLLAMA_MODEL="llama3.1"
```

---

## 🤖 Supported LLM Providers

Atlas-Context includes built-in clients for popular LLM providers:

### Alibaba Cloud Coding Plan (阿里云百炼)

```rust
use tokitai_context::ai::clients::AliyunCodingPlanClient;

// From .env file or environment variable
let client = AliyunCodingPlanClient::from_env();

// Custom configuration
let client = AliyunCodingPlanClient::with_config(
    AliyunCodingPlanConfig::default()
        .with_model("qwen-plus")
        .with_api_key("your-coding-plan-api-key")
);
```

**Supported models**: qwen-plus, qwen-max, qwen-turbo, qwen-long

**⚠️ Important**: You MUST use the Coding Plan exclusive API Key and Base URL. Using standard API keys will be charged via pay-as-you-go instead of deducting from the plan quota.

**Get API Key**: https://bailian.console.aliyun.com

### OpenAI

```rust
use tokitai_context::ai::clients::OpenAIClient;

// From .env file or environment variable (OPENAI_API_KEY)
let client = OpenAIClient::from_env();

// Or with explicit API key
let client = OpenAIClient::new("your-api-key");

// Custom configuration
let client = OpenAIClient::with_config(
    OpenAIConfig::default()
        .with_model("gpt-4o-mini")
        .with_api_key("your-api-key")
);
```

**Supported models**: GPT-4, GPT-4-turbo, GPT-3.5-turbo, gpt-4o, gpt-4o-mini

**Get API Key**: https://platform.openai.com

### Anthropic

```rust
use tokitai_context::ai::clients::AnthropicClient;

// From .env file or environment variable (ANTHROPIC_API_KEY)
let client = AnthropicClient::from_env();

// Or with explicit API key
let client = AnthropicClient::new("your-api-key");

// Custom model
let client = AnthropicClient::with_config(
    AnthropicConfig::default()
        .with_model("claude-3-5-sonnet-20241022")
);
```

**Supported models**: Claude 3 family (Opus, Sonnet, Haiku)

**Get API Key**: https://console.anthropic.com

### Ollama (Self-hosted)

```rust
use tokitai_context::ai::clients::OllamaClient;

// From .env file or environment variables
let client = OllamaClient::from_env();

// Or explicit configuration
let client = OllamaClient::new("http://localhost:11434", "llama3.1");

// Custom configuration
let client = OllamaClient::with_config(
    OllamaConfig::default()
        .with_base_url("http://127.0.0.1:11434")
        .with_model("mistral")
);
```

**Supported models**: Any model available in Ollama (Llama, Mistral, Qwen, etc.)

**Get Ollama**: https://ollama.com

---

## 🚀 Quick Start

### Basic AI-Powered Merge

```rust
use tokitai_context::facade::{Context, AIContext};
use tokitai_context::ai::clients::OpenAIClient;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Open context
    let mut ctx = Context::open("./.context")?;
    
    // Initialize AI client
    let llm = Arc::new(OpenAIClient::from_env());
    
    // Wrap with AI capabilities
    let mut ai_ctx = AIContext::new(&mut ctx, llm);
    
    // Merge with AI conflict resolution
    let result = ai_ctx.merge_with_ai("feature", "main").await?;
    
    println!("Merge completed: {:?}", result);
    
    Ok(())
}
```

### Conflict Resolution

```rust
let response = ai_ctx.resolve_conflict(
    "conflict-1",
    "feature-branch",
    "main",
    "Source branch content",
    "Target branch content",
).await?;

println!("Decision: {:?}", response.decision);
println!("Confidence: {:.1}%", response.confidence * 100.0);
println!("Reasoning: {}", response.reasoning);

if let Some(combined) = &response.combined_content {
    println!("Combined: {}", combined);
}
```

### Branch Purpose Inference

```rust
let result = ai_ctx.infer_branch_purpose("feature-auth").await?;

println!("Type: {:?}", result.branch_type);
println!("Purpose: {}", result.purpose);
println!("Confidence: {:.0}%", result.confidence * 100.0);
println!("Tags: {:?}", result.suggested_tags);
println!("Auto-merge: {}", result.suggest_auto_merge);
```

### Merge Recommendations

```rust
let rec = ai_ctx.get_merge_recommendation("feature", "main").await?;

println!("Should merge: {}", rec.recommend_merge);
println!("Strategy: {:?}", rec.recommended_strategy);
println!("Timing: {}", rec.timing_recommendation);
println!("Risk: {} ({:.0}%)", rec.risk_assessment.risk_level, rec.risk_assessment.risk_score * 100.0);
```

### Branch Summarization

```rust
let summary = ai_ctx.summarize_branch("feature-auth").await?;

println!("Title: {}", summary.title);
println!("Summary: {}", summary.summary);
println!("Status: {:.0}% complete", summary.status_assessment.completion_ratio * 100.0);
println!("Next steps: {:?}", summary.next_steps);
```

---

## 📚 Complete Examples

Run the included examples:

```bash
# AI conflict resolution
OPENAI_API_KEY=your-key cargo run --example ai_conflict_resolution --features ai

# Branch purpose inference
OPENAI_API_KEY=your-key cargo run --example ai_purpose_inference --features ai

# Complete workflow
OPENAI_API_KEY=your-key cargo run --example ai_workflow --features ai

# Use Anthropic instead
ANTHROPIC_API_KEY=your-key cargo run --example ai_workflow --features ai --anthropic

# Use local Ollama
cargo run --example ai_workflow --features ai --ollama
```

---

## 🔧 Configuration

### Environment Variables

| Variable | Provider | Description |
|----------|----------|-------------|
| `ALIYUN_CODING_PLAN_API_KEY` | Alibaba Cloud | API key for Coding Plan |
| `DASHSCOPE_API_KEY` | Alibaba Cloud | Alternative variable for Coding Plan |
| `OPENAI_API_KEY` | OpenAI | API key for OpenAI |
| `ANTHROPIC_API_KEY` | Anthropic | API key for Anthropic |
| `OLLAMA_BASE_URL` | Ollama | Ollama server URL (default: http://localhost:11434) |
| `OLLAMA_MODEL` | Ollama | Default model to use |
| `OPENAI_BASE_URL` | OpenAI | Custom API endpoint (optional) |

### Advanced Configuration

```rust
use tokitai_context::ai::clients::{OpenAIConfig, OpenAIClient};

let config = OpenAIConfig {
    api_key: "your-api-key".to_string(),
    base_url: "https://api.openai.com/v1".to_string(),
    model: "gpt-4o-mini".to_string(),
    timeout_ms: 30000,
    max_retries: 3,
};

let client = OpenAIClient::with_config(config);
```

---

## 🏗️ Architecture

### LLMClient Trait

All AI features are built on the `LLMClient` trait:

```rust
#[async_trait]
pub trait LLMClient: Send + Sync {
    async fn chat(&self, prompt: &str) -> Result<String>;
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String>;
}
```

You can implement this trait to use custom LLM providers:

```rust
use tokitai_context::ai::resolver::LLMClient;

struct MyCustomLLM {
    // Your configuration
}

#[async_trait]
impl LLMClient for MyCustomLLM {
    async fn chat(&self, prompt: &str) -> Result<String> {
        // Your implementation
    }
    
    async fn chat_with_schema(&self, prompt: &str, schema: &serde_json::Value) -> Result<String> {
        // Your implementation
    }
}
```

### AIContext Wrapper

The `AIContext` wrapper provides a high-level API for AI features:

```rust
pub struct AIContext<'a, T: LLMClient> {
    inner: &'a mut Context,
    llm_client: Arc<T>,
    conflict_resolver: AIConflictResolver,
    purpose_inference: AIPurposeInference,
}
```

**Methods**:
- `merge_with_ai()` - Merge with automatic conflict resolution
- `infer_branch_purpose()` - Auto-detect branch purpose
- `get_merge_recommendation()` - Get merge timing/strategy advice
- `summarize_branch()` - Generate branch summary
- `resolve_conflict()` - Resolve specific conflict

---

## 🎯 Use Cases

### 1. Multi-Branch Development

```rust
// Create multiple feature branches
manager.create_branch("feature-auth", "main")?;
manager.create_branch("feature-api", "main")?;

// Get recommendations for each
let auth_rec = ai_ctx.get_merge_recommendation("feature-auth", "main").await?;
let api_rec = ai_ctx.get_merge_recommendation("feature-api", "main").await?;

// Merge in optimal order based on AI advice
if auth_rec.recommend_merge {
    ai_ctx.merge_with_ai("feature-auth", "main").await?;
}
```

### 2. Automated Code Review

```rust
// Infer what each branch does
let purpose = ai_ctx.infer_branch_purpose("feature-x").await?;

// Check if purpose matches expectations
assert_eq!(purpose.branch_type, BranchType::Feature);
println!("This branch implements: {}", purpose.purpose);
```

### 3. Conflict Resolution at Scale

```rust
// Batch resolve conflicts
let conflicts = vec![
    ("conflict-1", "source1", "target1", "content1", "content2"),
    ("conflict-2", "source2", "target2", "content3", "content4"),
];

for (id, src_branch, tgt_branch, src, tgt) in conflicts {
    let response = ai_ctx.resolve_conflict(id, src_branch, tgt_branch, src, tgt).await?;
    
    match response.decision {
        MergeDecision::KeepSource => println!("Keep source"),
        MergeDecision::KeepTarget => println!("Keep target"),
        MergeDecision::Combine => println!("Combined: {}", response.combined_content.unwrap()),
        _ => {}
    }
}
```

---

## 📊 Performance Considerations

### Latency

| Operation | Typical Latency |
|-----------|----------------|
| Conflict Resolution | 1-3 seconds |
| Purpose Inference | 1-2 seconds |
| Merge Recommendation | 2-4 seconds |
| Branch Summary | 2-5 seconds |

### Cost Optimization

1. **Use smaller models** for simple tasks (GPT-3.5-turbo, Claude Haiku)
2. **Cache results** for repeated queries
3. **Batch operations** when possible
4. **Use Ollama** for development/testing (free, local)

---

## 🔒 Security

### API Key Management

**Never** hardcode API keys in your code. Use environment variables:

```bash
export OPENAI_API_KEY="your-key-here"
```

Or use a secrets manager in production.

### Data Privacy

Be aware that:
- OpenAI and Anthropic may log API requests
- For sensitive data, use local models (Ollama)
- Consider enabling enterprise privacy features

---

## 🧪 Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tokitai_context::ai::clients::OpenAIClient;
    
    #[test]
    fn test_ai_context_creation() {
        let mut ctx = Context::open("./.context").unwrap();
        let llm = Arc::new(OpenAIClient::new("test-key"));
        let ai_ctx = AIContext::new(&mut ctx, llm);
        
        assert!(ai_ctx.inner().is_ok());
    }
}
```

### Integration Tests

See `examples/` directory for complete integration test examples.

---

## 🐛 Troubleshooting

### Common Issues

**"Failed to parse LLM response"**
- Check that your API key is valid
- Verify network connectivity
- Try a different model

**"AI purpose inference failed"**
- Ensure branch has sufficient content
- Check LLM API status
- Increase timeout in config

**"Request timeout"**
- Increase `timeout_ms` in client config
- Check network latency
- Use a closer API endpoint

---

## 📖 Related Documentation

- [Architecture](ARCHITECTURE.md) - System architecture
- [Quick Start](QUICKSTART.md) - Basic context management
- [Parallel Context](PARALLEL_CONTEXT_IMPLEMENTATION.md) - Branch management
- [API Reference](https://docs.rs/tokitai-context) - Rust API docs

---

## 🎓 Best Practices

1. **Start with Ollama** for development (free, fast iteration)
2. **Use GPT-4o-mini or Claude Haiku** for production (cost-effective)
3. **Cache AI results** to avoid redundant API calls
4. **Log AI decisions** for debugging and auditing
5. **Set reasonable timeouts** to avoid hanging
6. **Implement fallback strategies** for API failures

---

**Last updated**: 2026-04-04  
**Version**: 1.0.0  
**Maintainer**: Atlas Team
