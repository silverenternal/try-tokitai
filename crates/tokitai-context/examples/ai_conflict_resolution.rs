//! AI-powered conflict resolution example
//!
//! This example demonstrates how to use AI to automatically resolve
//! conflicts when merging branches.
//!
//! # Usage
//!
//! ```bash
//! # With OpenAI
//! OPENAI_API_KEY=your-key cargo run --example ai_conflict_resolution --features ai
//!
//! # With Anthropic
//! ANTHROPIC_API_KEY=your-key cargo run --example ai_conflict_resolution --features ai --anthropic
//!
//! # With Ollama (local)
//! cargo run --example ai_conflict_resolution --features ai --ollama
//! ```

use std::sync::Arc;
use anyhow::Result;
use tokitai_context::facade::{Context, Layer, AIContext};
use tokitai_context::parallel::{ParallelContextManager, ParallelContextManagerConfig};
use tokitai_context::parallel::branch::MergeStrategy;
use tokitai_context::ai::client::LLMClient;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    let _ = tokitai_context::tracing_config::init_tracing_minimal();

    println!("🚀 AI Conflict Resolution Example\n");

    // Open context
    let mut ctx = Context::open("./.context")?;
    println!("✅ Opened context store");

    // Choose LLM provider based on args
    let args: Vec<String> = std::env::args().collect();
    let use_anthropic = args.iter().any(|a| a == "--anthropic");
    let use_ollama = args.iter().any(|a| a == "--ollama");

    // Create AI client
    let llm: Arc<dyn LLMClient> = if use_anthropic {
        println!("🤖 Using Anthropic Claude");
        use tokitai_context::ai::clients::AnthropicClient;
        let client = AnthropicClient::from_env();
        Arc::new(client)
    } else if use_ollama {
        println!("🤖 Using Ollama (local)");
        use tokitai_context::ai::clients::OllamaClient;
        let client = OllamaClient::new("http://localhost:11434", "llama3.1");
        Arc::new(client)
    } else {
        println!("🤖 Using OpenAI GPT");
        use tokitai_context::ai::clients::OpenAIClient;
        let client = OpenAIClient::from_env();
        Arc::new(client)
    };

    // Wrap with AI capabilities
    let mut ai_ctx = AIContext::new(&mut ctx, llm);
    println!("✅ AI context initialized\n");

    // Create parallel manager for branch operations
    let config = ParallelContextManagerConfig {
        context_root: std::path::PathBuf::from("./.context"),
        default_merge_strategy: MergeStrategy::SelectiveMerge,
        ..Default::default()
    };
    let mut manager = ParallelContextManager::new(config)?;

    // Create two branches with conflicting content
    println!("📝 Creating branches with conflicting content...");

    // Store some initial content in main using ai_ctx.inner_mut()
    ai_ctx.inner_mut().store("session-1", b"Initial content", Layer::ShortTerm)?;

    // Create feature branch
    manager.create_branch("feature-conflict", "main")?;
    manager.checkout("feature-conflict")?;

    // Add conflicting content in feature branch
    ai_ctx.inner_mut().store("session-1", b"Feature branch version", Layer::ShortTerm)?;

    // Switch back to main
    manager.checkout("main")?;

    // Add different content in main
    ai_ctx.inner_mut().store("session-1", b"Main branch version", Layer::ShortTerm)?;

    println!("✅ Branches created with conflicts\n");

    // Show the conflict
    println!("🔍 Conflict scenario:");
    println!("   Main branch:   'Main branch version'");
    println!("   Feature branch: 'Feature branch version'");
    println!();

    // Resolve conflict using AI
    println!("🤖 Resolving conflict with AI...\n");
    
    let response = ai_ctx.resolve_conflict(
        "conflict-1",
        "feature-conflict",
        "main",
        "Feature branch version",
        "Main branch version",
    ).await?;

    println!("✅ Conflict resolved!");
    println!("   Decision: {:?}", response.decision);
    println!("   Confidence: {:.1}%", response.confidence * 100.0);
    println!("   Reasoning: {}", response.reasoning);
    
    if let Some(combined) = &response.combined_content {
        println!("   Combined: {}", combined);
    }

    println!("\n🎉 Example completed successfully!");
    
    Ok(())
}
