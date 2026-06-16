//! Individual LLM Provider Implementations
//!
//! This module contains concrete implementations for each LLM provider.

use super::{ChatRequest, ChatResponse, LLMProvider, Message, ProviderType, StreamChunk, Usage};
use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// OpenAI Provider
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    api_url: String,
    default_model: String,
}

impl OpenAIProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            api_url: "https://api.openai.com/v1/chat/completions".to_string(),
            default_model: model.unwrap_or_else(|| "gpt-4o".to_string()),
        }
    }

    pub fn with_base_url(api_key: String, base_url: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            api_url: base_url,
            default_model: model.unwrap_or_else(|| "gpt-4o".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for OpenAIProvider {
    fn provider_type(&self) -> &ProviderType {
        &ProviderType::OpenAI
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        let payload = OpenAIRequest {
            model: request.model,
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop: request.stop,
            stream: Some(false),
            tools: request.tools,
        };

        let response: OpenAIResponse = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to OpenAI")?
            .error_for_status()
            .context("OpenAI API returned an error")?
            .json()
            .await
            .context("Failed to parse OpenAI response")?;

        let choice = response
            .choices
            .first()
            .context("No choices in OpenAI response")?;

        // Convert tool_calls to JSON Value format if present
        let tool_calls_json: Option<Vec<serde_json::Value>> =
            choice.message.tool_calls.as_ref().map(|tc| {
                tc.iter()
                    .map(|t| {
                        serde_json::json!({
                            "id": t.id,
                            "type": "function",
                            "function": {
                                "name": t.function.name,
                                "arguments": t.function.arguments,
                            }
                        })
                    })
                    .collect()
            });

        Ok(ChatResponse {
            content: tool_calls_json
                .as_ref()
                .map(|tc: &Vec<serde_json::Value>| {
                    serde_json::to_string(tc).unwrap_or_default()
                })
                .unwrap_or_else(|| choice.message.content.clone()),
            model: response.model,
            usage: response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>> {
        // Streaming implementation using SSE
        use futures::stream::StreamExt;
        use std::collections::BTreeMap;

        let payload = OpenAIRequest {
            model: request.model,
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop: request.stop,
            stream: Some(true),
            tools: request.tools,
        };

        let mut event_source = reqwest_eventsource::EventSource::new(
            self.client
                .post(&self.api_url)
                .header("Authorization", format!("Bearer {}", self.api_key))
                .header("Content-Type", "application/json")
                .json(&payload),
        )?;

        let stream = async_stream::stream! {
            // Map to accumulate tool call deltas across SSE events
            let mut tc_index: BTreeMap<usize, serde_json::Value> = BTreeMap::new();

            while let Some(event) = event_source.next().await {
                match event {
                    Ok(reqwest_eventsource::Event::Open) => {}
                    Ok(reqwest_eventsource::Event::Message(message)) => {
                        if message.data == "[DONE]" {
                            break;
                        }
                        if let Ok(response) = serde_json::from_str::<OpenAIStreamResponse>(&message.data) {
                            if let Some(choice) = response.choices.first() {
                                // Accumulate tool call deltas
                                if let Some(delta_tool_calls) = &choice.delta.tool_calls {
                                    for dtc in delta_tool_calls {
                                        let entry = tc_index.entry(dtc.index).or_insert_with(|| {
                                            serde_json::json!({
                                                "id": "",
                                                "type": "function",
                                                "function": {
                                                    "name": "",
                                                    "arguments": ""
                                                }
                                            })
                                        });
                                        if let Some(ref id) = dtc.id {
                                            entry["id"] = serde_json::json!(id);
                                        }
                                        if let Some(ref fn_def) = dtc.function {
                                            if let Some(ref name) = fn_def.name {
                                                entry["function"]["name"] = serde_json::json!(name);
                                            }
                                            if let Some(ref args) = fn_def.arguments {
                                                if let Some(ref existing) = entry["function"]["arguments"].as_str() {
                                                    entry["function"]["arguments"] = serde_json::json!(format!("{}{}", existing, args));
                                                } else {
                                                    entry["function"]["arguments"] = serde_json::json!(args);
                                                }
                                            }
                                        }
                                    }
                                }

                                let tool_calls = if tc_index.is_empty() {
                                    None
                                } else {
                                    Some(tc_index.values().cloned().collect())
                                };

                                let has_content = choice.delta.content.is_some() || choice.finish_reason.is_some();

                                if has_content || tool_calls.is_some() {
                                    yield Ok(StreamChunk {
                                        content: choice.delta.content.clone().unwrap_or_default(),
                                        finish_reason: choice.finish_reason.clone(),
                                        tool_calls: tool_calls.clone(),
                                        usage: None,
                                    });
                                }
                            }
                        }
                    }
                    Err(err) => {
                        yield Err(anyhow::anyhow!("SSE error: {}", err));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    async fn health_check(&self) -> bool {
        // Simple health check by calling models endpoint
        let url = self.api_url.replace("/chat/completions", "/models");
        self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

/// Gemini Provider (Google)
pub struct GeminiProvider {
    client: Client,
    api_key: String,
    default_model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: model.unwrap_or_else(|| "gemini-pro".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for GeminiProvider {
    fn provider_type(&self) -> &ProviderType {
        &ProviderType::Gemini
    }

    fn name(&self) -> &str {
        "Gemini"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Convert messages to Gemini format
        let gemini_request = GeminiRequest {
            contents: request
                .messages
                .into_iter()
                .filter(|m| m.role != "system") // Gemini doesn't support system messages directly
                .map(|m| GeminiContent {
                    role: if m.role == "user" { "user" } else { "model" }.to_string(),
                    parts: vec![GeminiPart { text: m.content }],
                })
                .collect(),
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(request.temperature),
                max_output_tokens: request.max_tokens,
                top_p: request.top_p,
                stop_sequences: request.stop,
            }),
        };

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            request.model, self.api_key
        );

        let response: GeminiResponse = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&gemini_request)
            .send()
            .await
            .context("Failed to send request to Gemini")?
            .error_for_status()
            .context("Gemini API returned an error")?
            .json()
            .await
            .context("Failed to parse Gemini response")?;

        let content = response
            .candidates
            .first()
            .and_then(|c| c.content.parts.first())
            .map(|p| p.text.clone())
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            model: request.model,
            usage: None, // Gemini doesn't provide token usage in all cases
            finish_reason: response
                .candidates
                .first()
                .and_then(|c| c.finish_reason.clone()),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>> {
        // Streaming not fully implemented for Gemini
        bail!("Streaming not yet implemented for Gemini provider");
    }

    async fn health_check(&self) -> bool {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models?key={}",
            self.api_key
        );
        self.client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

/// Anthropic Provider (Claude)
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    default_model: String,
}

impl AnthropicProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: model.unwrap_or_else(|| "claude-3-5-sonnet-20241022".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for AnthropicProvider {
    fn provider_type(&self) -> &ProviderType {
        &ProviderType::Anthropic
    }

    fn name(&self) -> &str {
        "Anthropic"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Separate system message from conversation
        let system_message = request
            .messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone());

        let messages: Vec<AnthropicMessage> = request
            .messages
            .into_iter()
            .filter(|m| m.role != "system")
            .map(|m| AnthropicMessage {
                role: if m.role == "user" {
                    "user"
                } else {
                    "assistant"
                }
                .to_string(),
                content: vec![AnthropicContent::Text { text: m.content }],
            })
            .collect();

        let anthropic_request = AnthropicRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens.unwrap_or(4096),
            system: system_message,
            temperature: Some(request.temperature),
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: false,
        };

        let response: AnthropicResponse = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&anthropic_request)
            .send()
            .await
            .context("Failed to send request to Anthropic")?
            .error_for_status()
            .context("Anthropic API returned an error")?
            .json()
            .await
            .context("Failed to parse Anthropic response")?;

        let content = response
            .content
            .first()
            .and_then(|c| match c {
                AnthropicContent::Text { text } => Some(text.clone()),
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            model: response.model,
            usage: response.usage.map(|u| Usage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
            }),
            finish_reason: response.stop_reason,
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>> {
        bail!("Streaming not yet implemented for Anthropic provider");
    }

    async fn health_check(&self) -> bool {
        // Anthropic doesn't have a dedicated health endpoint
        // Just try a minimal request
        true
    }
}

/// Zhipu Provider (智谱 AI)
pub struct ZhipuProvider {
    client: Client,
    api_key: String,
    default_model: String,
}

impl ZhipuProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: model.unwrap_or_else(|| "glm-4".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for ZhipuProvider {
    fn provider_type(&self) -> &ProviderType {
        &ProviderType::Zhipu
    }

    fn name(&self) -> &str {
        "Zhipu"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Zhipu uses OpenAI-compatible API
        let payload = OpenAIRequest {
            model: request.model,
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop: request.stop,
            stream: Some(false),
            tools: request.tools,
        };

        let response: OpenAIResponse = self
            .client
            .post("https://open.bigmodel.cn/api/paas/v4/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Zhipu")?
            .error_for_status()
            .context("Zhipu API returned an error")?
            .json()
            .await
            .context("Failed to parse Zhipu response")?;

        let choice = response
            .choices
            .first()
            .context("No choices in Zhipu response")?;

        Ok(ChatResponse {
            content: choice.message.content.clone(),
            model: response.model,
            usage: response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>> {
        bail!("Streaming not yet implemented for Zhipu provider");
    }

    async fn health_check(&self) -> bool {
        true
    }
}

/// Moonshot Provider (月之暗面)
pub struct MoonshotProvider {
    client: Client,
    api_key: String,
    default_model: String,
}

impl MoonshotProvider {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: Client::new(),
            api_key,
            default_model: model.unwrap_or_else(|| "moonshot-v1-8k".to_string()),
        }
    }
}

#[async_trait::async_trait]
impl LLMProvider for MoonshotProvider {
    fn provider_type(&self) -> &ProviderType {
        &ProviderType::Moonshot
    }

    fn name(&self) -> &str {
        "Moonshot"
    }

    fn default_model(&self) -> &str {
        &self.default_model
    }

    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse> {
        // Moonshot uses OpenAI-compatible API
        let payload = OpenAIRequest {
            model: request.model,
            messages: request.messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            stop: request.stop,
            stream: Some(false),
            tools: request.tools,
        };

        let response: OpenAIResponse = self
            .client
            .post("https://api.moonshot.cn/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send request to Moonshot")?
            .error_for_status()
            .context("Moonshot API returned an error")?
            .json()
            .await
            .context("Failed to parse Moonshot response")?;

        let choice = response
            .choices
            .first()
            .context("No choices in Moonshot response")?;

        Ok(ChatResponse {
            content: choice.message.content.clone(),
            model: response.model,
            usage: response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason.clone(),
        })
    }

    async fn chat_stream(
        &self,
        request: ChatRequest,
    ) -> Result<Pin<Box<dyn futures::Stream<Item = Result<StreamChunk>> + Send>>> {
        bail!("Streaming not yet implemented for Moonshot provider");
    }

    async fn health_check(&self) -> bool {
        true
    }
}

// ============================================================================
// Shared Request/Response Types (OpenAI-compatible)
// ============================================================================

#[derive(Debug, Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponse {
    model: String,
    choices: Vec<OpenAIChoice>,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    #[serde(default)]
    content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIDeltaToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIDeltaToolCall {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type", default)]
    call_type: Option<String>,
    #[serde(default)]
    function: Option<OpenAIDeltaFunction>,
}

#[derive(Debug, Clone, Deserialize)]
struct OpenAIDeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

// ============================================================================
// Gemini Types
// ============================================================================

#[derive(Debug, Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Debug, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

// ============================================================================
// Anthropic Types
// ============================================================================

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    model: String,
    content: Vec<AnthropicContent>,
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: usize,
    output_tokens: usize,
}

// Re-export Pin for stream return type
pub use std::pin::Pin;
