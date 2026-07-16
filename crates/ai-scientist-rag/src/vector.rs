//! Vector Store — stores and indexes embedding vectors
//!
//! Provides a trait-based abstraction over vector databases.
//! Default implementation is in-memory (cosine similarity).
//! Production deployments can swap in Qdrant, Weaviate, or pgvector.

use async_trait::async_trait;
use std::collections::HashMap;

use super::chunker::DocumentChunk;

/// Vector store trait — multiple backend implementations
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Insert a batch of chunks with embeddings
    async fn insert(&mut self, chunks: &[DocumentChunk]) -> Result<usize, VectorStoreError>;

    /// Search for similar chunks by embedding vector
    async fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<(DocumentChunk, f64)>, VectorStoreError>;

    /// Delete chunks by document ID
    async fn delete_document(&mut self, document_id: &str) -> Result<usize, VectorStoreError>;

    /// Total number of chunks stored
    fn count(&self) -> usize;

    /// List all document IDs
    fn documents(&self) -> Vec<String>;
}

/// In-memory vector store using cosine similarity
pub struct InMemoryVectorStore {
    chunks: HashMap<String, DocumentChunk>,
    /// chunk_id -> embedding vector
    embeddings: HashMap<String, Vec<f32>>,
}

impl InMemoryVectorStore {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            embeddings: HashMap::new(),
        }
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
        let dot: f64 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (*x as f64) * (*y as f64))
            .sum();
        let norm_a: f64 = a
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            .sqrt();
        let norm_b: f64 = b
            .iter()
            .map(|x| (*x as f64) * (*x as f64))
            .sum::<f64>()
            .sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return 0.0;
        }

        dot / (norm_a * norm_b)
    }
}

impl Default for InMemoryVectorStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorStore for InMemoryVectorStore {
    async fn insert(&mut self, chunks: &[DocumentChunk]) -> Result<usize, VectorStoreError> {
        let mut count = 0;
        for chunk in chunks {
            if let Some(ref embedding) = chunk.embedding {
                self.chunks.insert(chunk.chunk.id.clone(), chunk.clone());
                self.embeddings
                    .insert(chunk.chunk.id.clone(), embedding.clone());
                count += 1;
            }
        }
        Ok(count)
    }

    async fn search(
        &self,
        query_embedding: &[f32],
        top_k: usize,
    ) -> Result<Vec<(DocumentChunk, f64)>, VectorStoreError> {
        let mut scored: Vec<(DocumentChunk, f64)> = self
            .embeddings
            .iter()
            .filter_map(|(id, emb)| {
                let chunk = self.chunks.get(id)?;
                let score = Self::cosine_similarity(query_embedding, emb);
                Some((chunk.clone(), score))
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored.into_iter().take(top_k).collect())
    }

    async fn delete_document(&mut self, document_id: &str) -> Result<usize, VectorStoreError> {
        let ids: Vec<String> = self
            .chunks
            .iter()
            .filter(|(_, c)| c.chunk.document_id == document_id)
            .map(|(id, _)| id.clone())
            .collect();

        let count = ids.len();
        for id in &ids {
            self.chunks.remove(id);
            self.embeddings.remove(id);
        }
        Ok(count)
    }

    fn count(&self) -> usize {
        self.chunks.len()
    }

    fn documents(&self) -> Vec<String> {
        let mut docs: Vec<String> = self
            .chunks
            .values()
            .map(|c| c.chunk.document_id.clone())
            .collect();
        docs.sort();
        docs.dedup();
        docs
    }
}

/// Vector store errors
#[derive(Debug, thiserror::Error)]
pub enum VectorStoreError {
    #[error("Chunk not found: {0}")]
    NotFound(String),

    #[error("Embedding missing for chunk: {0}")]
    MissingEmbedding(String),

    #[error("Dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },

    #[error("Store error: {0}")]
    Store(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunker::{Chunk, DocumentChunk};

    fn make_test_chunk(id: &str, doc_id: &str, embedding: Vec<f32>) -> DocumentChunk {
        DocumentChunk {
            chunk: Chunk {
                id: id.to_string(),
                content: format!("Content of {}", id),
                document_id: doc_id.to_string(),
                chunk_index: 0,
                total_chunks: 1,
                start_char: 0,
                end_char: 10,
                document_hash: "hash".to_string(),
            },
            embedding: Some(embedding),
        }
    }

    #[tokio::test]
    async fn test_insert_and_search() {
        let mut store = InMemoryVectorStore::new();

        let c1 = make_test_chunk("c1", "doc1", vec![1.0, 0.0, 0.0]);
        let c2 = make_test_chunk("c2", "doc2", vec![0.0, 1.0, 0.0]);

        store.insert(&[c1, c2]).await.unwrap();
        assert_eq!(store.count(), 2);

        // Search with query similar to c1
        let results = store.search(&[1.0, 0.1, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].1 > results[1].1); // c1 should be more similar
    }

    #[tokio::test]
    async fn test_delete_document() {
        let mut store = InMemoryVectorStore::new();
        let c1 = make_test_chunk("c1", "doc1", vec![1.0, 0.0]);
        let c2 = make_test_chunk("c2", "doc2", vec![0.0, 1.0]);

        store.insert(&[c1, c2]).await.unwrap();
        store.delete_document("doc1").await.unwrap();

        assert_eq!(store.count(), 1);
        assert_eq!(store.documents(), vec!["doc2"]);
    }
}
