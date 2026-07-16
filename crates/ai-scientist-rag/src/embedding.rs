//! Embedding Generator — converts text chunks to dense vectors
//!
//! Supports multiple embedding providers:
//! - OpenAI text-embedding-3-small/large
//! - Local models (future: via candle or ort)
//! - Custom API endpoints

use super::chunker::DocumentChunk;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Embedding provider trait — swap implementations at runtime
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// Generate embeddings for a batch of texts
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let results = self.embed_batch(&[text.to_string()]).await?;
        results
            .into_iter()
            .next()
            .ok_or(EmbeddingError::EmptyResponse)
    }

    /// Dimensionality of the embedding vectors
    fn dimension(&self) -> usize;

    /// Model name
    fn model_name(&self) -> &str;
}

/// Embedding generator that enriches chunks with vectors
pub struct EmbeddingGenerator {
    provider: Box<dyn EmbeddingProvider>,
}

impl EmbeddingGenerator {
    pub fn new(provider: Box<dyn EmbeddingProvider>) -> Self {
        Self { provider }
    }

    /// Generate embeddings for a set of chunks
    pub async fn embed_chunks(
        &self,
        chunks: &[DocumentChunk],
    ) -> Result<Vec<DocumentChunk>, EmbeddingError> {
        let texts: Vec<String> = chunks.iter().map(|c| c.chunk.content.clone()).collect();
        let embeddings = self.provider.embed_batch(&texts).await?;

        if embeddings.len() != chunks.len() {
            return Err(EmbeddingError::MismatchedLength {
                expected: chunks.len(),
                got: embeddings.len(),
            });
        }

        let mut result = chunks.to_vec();
        for (i, embedding) in embeddings.into_iter().enumerate() {
            result[i].embedding = Some(embedding);
        }

        Ok(result)
    }

    pub fn dimension(&self) -> usize {
        self.provider.dimension()
    }
}

/// OpenAI embedding provider
pub struct OpenAiEmbeddingProvider {
    api_key: String,
    api_url: String,
    model: String,
    client: reqwest::Client,
}

impl OpenAiEmbeddingProvider {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_url: "https://api.openai.com/v1/embeddings".into(),
            model: model.into(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.api_url = url.into();
        self
    }
}

#[async_trait]
impl EmbeddingProvider for OpenAiEmbeddingProvider {
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        #[derive(Serialize)]
        struct EmbeddingRequest {
            model: String,
            input: Vec<String>,
        }

        #[derive(Deserialize)]
        struct EmbeddingResponse {
            data: Vec<EmbeddingData>,
        }

        #[derive(Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
        }

        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
        };

        let response = self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| EmbeddingError::ApiError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(EmbeddingError::ApiError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let body: EmbeddingResponse = response
            .json()
            .await
            .map_err(|e| EmbeddingError::ApiError(format!("Failed to parse response: {}", e)))?;

        Ok(body.data.into_iter().map(|d| d.embedding).collect())
    }

    fn dimension(&self) -> usize {
        match self.model.as_str() {
            "text-embedding-3-small" => 1536,
            "text-embedding-3-large" => 3072,
            "text-embedding-ada-002" => 1536,
            _ => 1536, // default
        }
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// A simple mock embedding provider for testing
pub struct MockEmbeddingProvider {
    dimension: usize,
}

impl MockEmbeddingProvider {
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        // Generate deterministic pseudo-embeddings from text hash
        Ok(texts
            .iter()
            .map(|t| {
                let hash = blake3::hash(t.as_bytes());
                let bytes = hash.as_bytes();
                (0..self.dimension)
                    .map(|i| (bytes[i % 32] as f32) / 255.0)
                    .collect()
            })
            .collect())
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn model_name(&self) -> &str {
        "mock-embedding"
    }
}

/// Errors that can occur during embedding generation
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("API error: {0}")]
    ApiError(String),

    #[error("Empty response from embedding provider")]
    EmptyResponse,

    #[error("Mismatched lengths: expected {expected}, got {got}")]
    MismatchedLength { expected: usize, got: usize },

    #[error("Provider error: {0}")]
    Provider(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_provider() {
        let provider = MockEmbeddingProvider::new(128);
        let result = provider.embed("test text").await.unwrap();
        assert_eq!(result.len(), 128);
        // Values should be in [0, 1] range
        assert!(result.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }
}
