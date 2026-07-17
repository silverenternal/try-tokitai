//! Alibaba Cloud Coding Plan Example
//!
//! This example demonstrates how to use the AliyunCodingPlanClient
//! with the unified LLMClient trait.
//!
//! # Setup
//!
//! 1. Subscribe to Alibaba Cloud Model Studio Coding Plan
//! 2. Get your Coding Plan exclusive API Key and Base URL
//! 3. Configure your API key using one of these methods:
//!
//!    **Method A: Using .env file (Recommended)**
//!    ```bash
//!    cp .env.example .env
//!    # Edit .env and add:
//!    # ALIYUN_CODING_PLAN_API_KEY=your-api-key
//!    ```
//!
//!    **Method B: Using environment variable**
//!    ```bash
//!    export ALIYUN_CODING_PLAN_API_KEY=your-api-key
//!    ```
//!
//! 4. Run the example:
//!    cargo run --features ai --example aliyun_coding_plan

#![cfg(feature = "ai")]

use std::sync::Arc;
use tokitai_context::ai::client::LLMClient;
use tokitai_context::ai::clients::{AliyunCodingPlanClient, AliyunCodingPlanConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("=== Alibaba Cloud Coding Plan Example ===\n");

    // Method 1: Create from environment variable
    println!("1. Creating client from environment variable...");
    let client = AliyunCodingPlanClient::from_env();
    let llm: Arc<dyn LLMClient> = Arc::new(client);
    
    println!("   Model: {}", llm.model_name());
    println!("   Stats: {:?}\n", llm.get_stats());

    // Method 2: Create with custom config
    println!("2. Creating client with custom config...");
    let config = AliyunCodingPlanConfig::default()
        .with_model("qwen-plus")
        .with_api_key(&std::env::var("ALIYUN_CODING_PLAN_API_KEY").unwrap_or_else(|_| "your-api-key".to_string()));
    
    let custom_client = AliyunCodingPlanClient::with_config(config);
    println!("   Model: {}", custom_client.model_name());
    println!("   Base URL: {}", custom_client.config().base_url);

    // Example: Chat with the model
    println!("\n3. Example chat (will fail without valid API key):");
    match llm.chat("Hello, who are you?").await {
        Ok(response) => {
            println!("   Response: {}", response);
        }
        Err(e) => {
            println!("   Expected error (no valid API key): {}", e);
        }
    }

    // Example: Chat with timeout
    println!("\n4. Chat with custom timeout:");
    match llm.chat_with_timeout("Tell me a short story", 5000).await {
        Ok(response) => {
            println!("   Response: {}", response);
        }
        Err(e) => {
            println!("   Expected error (no valid API key): {}", e);
        }
    }

    // Example: Chat with JSON schema
    println!("\n5. Chat with JSON schema constraint:");
    let schema = serde_json::json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "age": { "type": "integer", "minimum": 0 },
            "city": { "type": "string" }
        },
        "required": ["name", "age"]
    });

    match llm.chat_with_schema(
        "Generate a person profile with name Alice, age 30, living in Beijing",
        &schema
    ).await {
        Ok(response) => {
            println!("   Response: {}", response);
        }
        Err(e) => {
            println!("   Expected error (no valid API key): {}", e);
        }
    }

    // Show final stats
    println!("\n6. Final client statistics:");
    println!("   {:?}", llm.get_stats());

    println!("\n=== Example Complete ===");
    println!("\nNote: To use this example with real API calls:");
    println!("1. Subscribe to Alibaba Cloud Model Studio Coding Plan");
    println!("2. Get your Coding Plan exclusive API Key from the console");
    println!("3. export ALIYUN_CODING_PLAN_API_KEY=your-api-key");
    println!("4. Run this example again");

    Ok(())
}
