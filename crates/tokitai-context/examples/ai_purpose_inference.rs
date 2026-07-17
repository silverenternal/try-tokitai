//! AI-powered branch purpose inference example
//!
//! This example demonstrates how to use AI to automatically infer
//! the purpose of a branch based on its content and conversation history.
//!
//! # Usage
//!
//! ```bash
//! # With OpenAI
//! OPENAI_API_KEY=your-key cargo run --example ai_purpose_inference --features ai
//!
//! # With Anthropic
//! ANTHROPIC_API_KEY=your-key cargo run --example ai_purpose_inference --features ai --anthropic
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

    println!("🎯 AI Branch Purpose Inference Example\n");

    // Open context
    let mut ctx = Context::open("./.context")?;
    println!("✅ Opened context store");

    // Choose LLM provider
    let args: Vec<String> = std::env::args().collect();
    let use_anthropic = args.iter().any(|a| a == "--anthropic");

    // Create AI client
    let llm: Arc<dyn LLMClient> = if use_anthropic {
        println!("🤖 Using Anthropic Claude");
        use tokitai_context::ai::clients::AnthropicClient;
        Arc::new(AnthropicClient::from_env())
    } else {
        println!("🤖 Using OpenAI GPT");
        use tokitai_context::ai::clients::OpenAIClient;
        Arc::new(OpenAIClient::from_env())
    };

    // Wrap with AI capabilities
    let mut ai_ctx = AIContext::new(&mut ctx, llm);
    println!("✅ AI context initialized\n");

    // Create parallel manager
    let config = ParallelContextManagerConfig {
        context_root: std::path::PathBuf::from("./.context"),
        default_merge_strategy: MergeStrategy::SelectiveMerge,
        ..Default::default()
    };
    let mut manager = ParallelContextManager::new(config)?;

    // Create a feature branch
    println!("📝 Creating a feature branch...");
    manager.create_branch("feature-user-auth", "main")?;
    manager.checkout("feature-user-auth")?;

    // Simulate conversation history using ai_ctx.inner_mut()
    ai_ctx.inner_mut().store("conv-1", b"User asked: How do I implement JWT authentication?", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("conv-2", b"Assistant explained JWT token structure", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("conv-3", b"User asked about password hashing with bcrypt", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("conv-4", b"Assistant provided bcrypt implementation example", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("conv-5", b"User implemented login endpoint", Layer::ShortTerm)?;

    println!("✅ Branch created with conversation history\n");

    // Infer purpose using AI
    println!("🤖 Inferring branch purpose...\n");

    let result = ai_ctx.infer_branch_purpose("feature-user-auth").await?;

    println!("✅ Purpose inferred!");
    println!("   Type: {:?}", result.branch_type);
    println!("   Purpose: {}", result.purpose);
    println!("   Confidence: {:.1}%", result.confidence * 100.0);
    println!("   Tags: {:?}", result.suggested_tags);
    println!("   Auto-merge recommended: {}", result.suggest_auto_merge);
    println!("   Suggested strategy: {}", result.suggested_merge_strategy);
    println!();
    println!("   Reasoning: {}", result.reasoning);

    println!("\n🎉 Example completed successfully!");

    Ok(())
}
