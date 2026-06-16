//! AI Scientist RAG — Scientific Literature Retrieval
//!
//! Pipeline: `PDF → Chunk → Embedding → VectorDB → Retriever → Context`
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────┐   ┌──────────┐   ┌───────────┐
//! │  PDF    │→  │  Text    │→  │  BLAKE3   │
//! │ Parser  │   │ Chunker  │   │  Hash ID  │
//! └─────────┘   └──────────┘   └───────────┘
//!                                    │
//!                                    ▼
//!                               ┌───────────┐
//!                               │ Embedding │
//!                               │ Generator │
//!                               └───────────┘
//!                                    │
//!                                    ▼
//!                               ┌───────────┐
//!                               │  Vector   │
//!                               │   Store   │
//!                               └───────────┘
//!                                    │
//!                                    ▼
//! ┌──────────┐   ┌───────────┐  ┌───────────┐
//! │ Hybrid   │←  │  Dense    │← │  Vector   │
//! │Retriever │   │ Retriever │  │  Search   │
//! └──────────┘   └───────────┘  └───────────┘
//!      │
//!      ▼
//! ┌──────────┐
//! │  Rerank  │ → Context for LLM
//! └──────────┘
//! ```

pub mod chunker;
pub mod embedding;
pub mod parser;
pub mod retriever;
pub mod vector;

pub use chunker::{Chunk, DocumentChunk, TextChunker};
pub use embedding::{EmbeddingGenerator, EmbeddingProvider, OpenAiEmbeddingProvider};
pub use parser::PdfParser;
pub use retriever::{HybridRetriever, RetrievalResult, RetrieverConfig};
pub use vector::{InMemoryVectorStore, VectorStore, VectorStoreError};
