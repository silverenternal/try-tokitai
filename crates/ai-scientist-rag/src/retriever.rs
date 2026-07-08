//! Hybrid Retriever — combines dense + sparse retrieval with RRF reranking

use super::chunker::DocumentChunk;
use super::embedding::EmbeddingProvider;
use super::vector::{InMemoryVectorStore, VectorStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalResult {
    pub chunk: DocumentChunk,
    pub score: f64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrieverConfig {
    pub dense_candidates: usize,
    pub sparse_candidates: usize,
    pub top_k: usize,
    pub dense_weight: f64,
    pub rrf_k: f64,
}

impl Default for RetrieverConfig {
    fn default() -> Self {
        Self {
            dense_candidates: 50,
            sparse_candidates: 50,
            top_k: 10,
            dense_weight: 0.7,
            rrf_k: 60.0,
        }
    }
}

pub struct HybridRetriever {
    config: RetrieverConfig,
    vector_store: Arc<tokio::sync::RwLock<InMemoryVectorStore>>,
    embedding_provider: Arc<dyn EmbeddingProvider>,
    inverted_index: tokio::sync::RwLock<HashMap<String, HashMap<String, u32>>>,
    doc_lengths: tokio::sync::RwLock<HashMap<String, usize>>,
    total_docs: tokio::sync::RwLock<usize>,
    avg_doc_len: tokio::sync::RwLock<f64>,
}

impl HybridRetriever {
    pub fn new(
        config: RetrieverConfig,
        vector_store: Arc<tokio::sync::RwLock<InMemoryVectorStore>>,
        embedding_provider: Arc<dyn EmbeddingProvider>,
    ) -> Self {
        Self {
            config,
            vector_store,
            embedding_provider,
            inverted_index: tokio::sync::RwLock::new(HashMap::new()),
            doc_lengths: tokio::sync::RwLock::new(HashMap::new()),
            total_docs: tokio::sync::RwLock::new(0),
            avg_doc_len: tokio::sync::RwLock::new(0.0),
        }
    }

    pub async fn index_bm25(&self, chunks: &[DocumentChunk]) {
        let mut index = self.inverted_index.write().await;
        let mut lengths = self.doc_lengths.write().await;
        for chunk in chunks {
            let terms = Self::tokenize(&chunk.chunk.content);
            lengths.insert(chunk.chunk.id.clone(), terms.len());
            let mut term_freqs: HashMap<String, u32> = HashMap::new();
            for term in &terms {
                *term_freqs.entry(term.clone()).or_insert(0) += 1;
            }
            for (term, tf) in term_freqs {
                index
                    .entry(term)
                    .or_default()
                    .insert(chunk.chunk.id.clone(), tf);
            }
        }
        let mut total = self.total_docs.write().await;
        *total += chunks.len();
        let sum: usize = lengths.values().sum();
        let mut avg = self.avg_doc_len.write().await;
        *avg = sum as f64 / (*total as f64).max(1.0);
    }

    pub async fn retrieve(&self, query: &str) -> Result<Vec<RetrievalResult>, String> {
        let query_emb = self
            .embedding_provider
            .embed(query)
            .await
            .map_err(|e| format!("Embedding error: {}", e))?;

        let dense_results = {
            let store = self.vector_store.read().await;
            store
                .search(&query_emb, self.config.dense_candidates)
                .await
                .map_err(|e| format!("Vector search error: {}", e))?
        };

        let sparse_results = self.bm25_search(query).await;
        let fused = Self::reciprocal_rank_fusion(&dense_results, &sparse_results, &self.config);

        Ok(fused
            .into_iter()
            .take(self.config.top_k)
            .map(|(chunk, score)| RetrievalResult {
                chunk,
                score,
                source: "hybrid".to_string(),
            })
            .collect())
    }

    async fn bm25_search(&self, query: &str) -> Vec<(DocumentChunk, f64)> {
        let index = self.inverted_index.read().await;
        let lengths = self.doc_lengths.read().await;
        let total = *self.total_docs.read().await;
        let avg_len = *self.avg_doc_len.read().await;
        let query_terms = Self::tokenize(query);
        let k1 = 1.5_f64;
        let b = 0.75_f64;

        let mut scores: HashMap<String, f64> = HashMap::new();
        for term in &query_terms {
            if let Some(postings) = index.get(term) {
                let df = postings.len() as f64;
                if df == 0.0 {
                    continue;
                }
                let idf = ((total as f64 - df + 0.5) / (df + 0.5) + 1.0).ln();
                for (doc_id, tf) in postings {
                    let doc_len = *lengths.get(doc_id).unwrap_or(&1) as f64;
                    let tf_norm = (*tf as f64 * (k1 + 1.0))
                        / (*tf as f64 + k1 * (1.0 - b + b * doc_len / avg_len.max(1.0)));
                    *scores.entry(doc_id.clone()).or_insert(0.0) += idf * tf_norm;
                }
            }
        }

        // Convert scores to results using the vector store for chunk lookup
        let store = self.vector_store.read().await;
        let mut results = Vec::new();
        for (doc_id, score) in scores {
            // Use a zero-vec dummy search to get the chunk back
            // In production, use a direct chunk lookup
            if let Ok(mut hits) = store.search(&vec![0.0_f32; 128], 100).await {
                if let Some(pos) = hits.iter().position(|(c, _)| c.chunk.id == doc_id) {
                    let (chunk, _) = hits.remove(pos);
                    results.push((chunk, score));
                }
            }
        }

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(self.config.sparse_candidates);
        results
    }

    fn reciprocal_rank_fusion(
        dense: &[(DocumentChunk, f64)],
        sparse: &[(DocumentChunk, f64)],
        config: &RetrieverConfig,
    ) -> Vec<(DocumentChunk, f64)> {
        let mut scores: HashMap<String, (f64, DocumentChunk)> = HashMap::new();
        let k = config.rrf_k;

        for (rank, (chunk, _)) in dense.iter().enumerate() {
            let entry = scores
                .entry(chunk.chunk.id.clone())
                .or_insert_with(|| (0.0, chunk.clone()));
            entry.0 += config.dense_weight / (k + (rank + 1) as f64);
        }
        for (rank, (chunk, _)) in sparse.iter().enumerate() {
            let entry = scores
                .entry(chunk.chunk.id.clone())
                .or_insert_with(|| (0.0, chunk.clone()));
            entry.0 += (1.0 - config.dense_weight) / (k + (rank + 1) as f64);
        }

        let mut fused: Vec<_> = scores.into_values().collect();
        fused.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        fused.into_iter().map(|(s, c)| (c, s)).collect()
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 2)
            .map(|s| s.to_string())
            .collect()
    }
}
