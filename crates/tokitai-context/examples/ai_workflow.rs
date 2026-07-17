//! Complete AI-powered workflow example
//!
//! This example demonstrates a complete workflow using AI features:
//! 1. Create branches for different features
//! 2. Store conversation context
//! 3. Get AI merge recommendations
//! 4. Infer branch purposes
//! 5. Resolve conflicts with AI
//! 6. Generate branch summaries
//!
//! # Usage
//!
//! ```bash
//! OPENAI_API_KEY=your-key cargo run --example ai_workflow --features ai
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

    println!("🚀 Complete AI-Powered Workflow Example\n");
    println!("═══════════════════════════════════════\n");

    // Step 1: Initialize
    println!("📦 Step 1: Initialize context and AI");
    println!("─────────────────────────────────────");

    let mut ctx = Context::open("./.context")?;
    println!("   ✅ Context store opened");

    use tokitai_context::ai::clients::OpenAIClient;
    let llm: Arc<dyn LLMClient> = Arc::new(OpenAIClient::from_env());
    let mut ai_ctx = AIContext::new(&mut ctx, Arc::clone(&llm));
    println!("   ✅ AI client initialized (OpenAI)\n");

    // Step 2: Create branches
    println!("🌿 Step 2: Create feature branches");
    println!("──────────────────────────────────");
    
    let config = ParallelContextManagerConfig {
        context_root: std::path::PathBuf::from("./.context"),
        default_merge_strategy: MergeStrategy::SelectiveMerge,
        ..Default::default()
    };
    let mut manager = ParallelContextManager::new(config)?;

    // Create feature-auth branch
    manager.create_branch("feature-auth", "main")?;
    manager.checkout("feature-auth")?;

    ai_ctx.inner_mut().store("auth-1", b"Implement JWT token generation", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("auth-2", b"Add password hashing with bcrypt", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("auth-3", b"Create login endpoint", Layer::ShortTerm)?;
    println!("   ✅ Created 'feature-auth' branch with 3 conversations");

    // Create feature-api branch
    manager.checkout("main")?;
    manager.create_branch("feature-api", "main")?;
    manager.checkout("feature-api")?;

    ai_ctx.inner_mut().store("api-1", b"Design REST API structure", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("api-2", b"Implement user CRUD endpoints", Layer::ShortTerm)?;
    ai_ctx.inner_mut().store("api-3", b"Add request validation", Layer::ShortTerm)?;
    println!("   ✅ Created 'feature-api' branch with 3 conversations\n");

    // Step 3: Infer purposes
    println!("🎯 Step 3: Infer branch purposes with AI");
    println!("────────────────────────────────────────");
    
    manager.checkout("main")?;
    
    let auth_purpose = ai_ctx.infer_branch_purpose("feature-auth").await?;
    println!("   feature-auth:");
    println!("      Type: {:?}", auth_purpose.branch_type);
    println!("      Purpose: {}", auth_purpose.purpose);
    println!("      Confidence: {:.0}%", auth_purpose.confidence * 100.0);

    let api_purpose = ai_ctx.infer_branch_purpose("feature-api").await?;
    println!("   feature-api:");
    println!("      Type: {:?}", api_purpose.branch_type);
    println!("      Purpose: {}", api_purpose.purpose);
    println!("      Confidence: {:.0}%\n", api_purpose.confidence * 100.0);

    // Step 4: Get merge recommendations
    println!("💡 Step 4: Get AI merge recommendations");
    println!("────────────────────────────────────────");
    
    let auth_rec = ai_ctx.get_merge_recommendation("feature-auth", "main").await?;
    println!("   feature-auth → main:");
    println!("      Recommend merge: {}", auth_rec.recommend_merge);
    println!("      Strategy: {:?}", auth_rec.recommended_strategy);
    println!("      Timing: {}", auth_rec.timing_recommendation);
    println!("      Risk: {} ({:.0}%)", auth_rec.risk_assessment.risk_level, auth_rec.risk_assessment.risk_score * 100.0);

    let api_rec = ai_ctx.get_merge_recommendation("feature-api", "main").await?;
    println!("   feature-api → main:");
    println!("      Recommend merge: {}", api_rec.recommend_merge);
    println!("      Strategy: {:?}", api_rec.recommended_strategy);
    println!("      Timing: {}", api_rec.timing_recommendation);
    println!("      Risk: {} ({:.0}%)\n", api_rec.risk_assessment.risk_level, api_rec.risk_assessment.risk_score * 100.0);

    // Step 5: Generate summaries
    println!("📝 Step 5: Generate branch summaries");
    println!("────────────────────────────────────");
    
    let auth_summary = ai_ctx.summarize_branch("feature-auth").await?;
    println!("   feature-auth summary:");
    println!("      Title: {}", auth_summary.title);
    println!("      Status: {:.0}% complete, Quality: {}/10", 
             auth_summary.status_assessment.completion_ratio * 100.0,
             auth_summary.status_assessment.quality_score);
    println!("      Next steps: {:?}", auth_summary.next_steps);

    let api_summary = ai_ctx.summarize_branch("feature-api").await?;
    println!("   feature-api summary:");
    println!("      Title: {}", api_summary.title);
    println!("      Status: {:.0}% complete, Quality: {}/10\n", 
             api_summary.status_assessment.completion_ratio * 100.0,
             api_summary.status_assessment.quality_score);

    // Step 6: Demonstrate conflict resolution
    println!("⚡ Step 6: Demonstrate AI conflict resolution");
    println!("─────────────────────────────────────────────");
    
    let conflict_response = ai_ctx.resolve_conflict(
        "demo-conflict",
        "feature-auth",
        "main",
        "Use JWT for authentication",
        "Use session-based auth",
    ).await?;
    
    println!("   Conflict: JWT vs Session-based auth");
    println!("   Decision: {:?}", conflict_response.decision);
    println!("   Confidence: {:.0}%", conflict_response.confidence * 100.0);
    println!("   Reasoning: {}", conflict_response.reasoning);

    // Final summary
    println!("\n═══════════════════════════════════════");
    println!("🎉 Workflow completed successfully!");
    println!("═══════════════════════════════════════\n");

    Ok(())
}
