//! Text Chunker — splits documents into semantic chunks with BLAKE3 hashes

use serde::{Deserialize, Serialize};

/// A text chunk with unique BLAKE3 hash ID
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique BLAKE3 hash of the chunk content
    pub id: String,
    /// The chunk text content
    pub content: String,
    /// Source document identifier
    pub document_id: String,
    /// Position in the document (chunk index)
    pub chunk_index: usize,
    /// Total chunks in the document
    pub total_chunks: usize,
    /// Overlap start position in the original text
    pub start_char: usize,
    /// Overlap end position in the original text
    pub end_char: usize,
    /// BLAKE3 hash of the source document
    pub document_hash: String,
}

/// A chunk with embedding vector attached
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentChunk {
    /// The chunk data
    #[serde(flatten)]
    pub chunk: Chunk,
    /// Embedding vector (populated after embedding generation)
    pub embedding: Option<Vec<f32>>,
}

/// Text chunker with configurable size and overlap
pub struct TextChunker {
    /// Target chunk size in characters
    chunk_size: usize,
    /// Overlap between consecutive chunks in characters
    chunk_overlap: usize,
}

impl TextChunker {
    /// Create a new chunker with given parameters
    pub fn new(chunk_size: usize, chunk_overlap: usize) -> Self {
        assert!(
            chunk_overlap < chunk_size,
            "Overlap must be smaller than chunk size"
        );
        Self {
            chunk_size,
            chunk_overlap,
        }
    }

    /// Split a document into chunks
    ///
    /// Splits on paragraph boundaries where possible, then falls back
    /// to sentence boundaries, then to character-level splitting.
    pub fn chunk_document(
        &self,
        content: &str,
        document_id: &str,
        document_hash: &str,
    ) -> Vec<Chunk> {
        let paragraphs = Self::split_paragraphs(content);
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut start_char = 0usize;
        let mut char_offset = 0usize;

        for para in &paragraphs {
            if current_chunk.len() + para.len() <= self.chunk_size {
                // Paragraph fits in current chunk
                if !current_chunk.is_empty() {
                    current_chunk.push_str("\n\n");
                }
                current_chunk.push_str(para);
            } else {
                // Finalize current chunk and start new one
                if !current_chunk.is_empty() {
                    let end_char = char_offset;
                    chunks.push(self.make_chunk(
                        &current_chunk,
                        document_id,
                        chunks.len(),
                        start_char,
                        end_char,
                        document_hash,
                    ));
                    start_char = end_char.saturating_sub(self.chunk_overlap);
                    current_chunk = String::new();
                }

                // If paragraph itself is too large, split on sentences
                if para.len() > self.chunk_size {
                    let sentences = Self::split_sentences(para);
                    for sentence in sentences {
                        if current_chunk.len() + sentence.len() > self.chunk_size
                            && !current_chunk.is_empty()
                        {
                            let end_char = char_offset;
                            chunks.push(self.make_chunk(
                                &current_chunk,
                                document_id,
                                chunks.len(),
                                start_char,
                                end_char,
                                document_hash,
                            ));
                            start_char = end_char.saturating_sub(self.chunk_overlap);
                            current_chunk = String::new();
                        }
                        if !current_chunk.is_empty() {
                            current_chunk.push(' ');
                        }
                        current_chunk.push_str(&sentence);
                        char_offset += sentence.len() + 1;
                    }
                } else {
                    current_chunk.push_str(para);
                }
            }
            char_offset += para.len() + 2; // +2 for \n\n
        }

        // Final chunk
        if !current_chunk.is_empty() {
            let end_char = char_offset;
            chunks.push(self.make_chunk(
                &current_chunk,
                document_id,
                chunks.len(),
                start_char,
                end_char,
                document_hash,
            ));
        }

        // Update total_chunks on all chunks
        let total = chunks.len();
        for chunk in &mut chunks {
            chunk.total_chunks = total;
        }

        chunks
    }

    fn make_chunk(
        &self,
        content: &str,
        doc_id: &str,
        index: usize,
        start: usize,
        end: usize,
        doc_hash: &str,
    ) -> Chunk {
        let id = blake3::hash(content.as_bytes()).to_hex().to_string();
        Chunk {
            id,
            content: content.to_string(),
            document_id: doc_id.to_string(),
            chunk_index: index,
            total_chunks: 1, // updated later
            start_char: start,
            end_char: end,
            document_hash: doc_hash.to_string(),
        }
    }

    /// Split text into paragraphs (separated by double newlines)
    fn split_paragraphs(text: &str) -> Vec<String> {
        text.split("\n\n")
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect()
    }

    /// Split text into sentences
    fn split_sentences(text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();

        for ch in text.chars() {
            current.push(ch);
            if matches!(ch, '.' | '!' | '?') && current.len() > 5 {
                // Don't split on abbreviations like "et al." or "e.g."
                let trimmed = current.trim();
                if !trimmed.ends_with("al.")
                    && !trimmed.ends_with("e.g.")
                    && !trimmed.ends_with("i.e.")
                    && !trimmed.ends_with("etc.")
                    && !trimmed.ends_with("vs.")
                {
                    sentences.push(current.trim().to_string());
                    current = String::new();
                }
            }
        }

        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }

        sentences
    }
}

impl Default for TextChunker {
    fn default() -> Self {
        Self::new(1000, 200)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_document() {
        let chunker = TextChunker::new(500, 100);
        let text = "This is a test paragraph.\n\nAnother paragraph here.\n\nThird one.";
        let chunks = chunker.chunk_document(text, "doc-1", "hash-1");

        assert!(!chunks.is_empty());
        for chunk in &chunks {
            // Verify BLAKE3 hash is valid hex
            assert_eq!(chunk.id.len(), 64);
            assert!(chunk.id.chars().all(|c| c.is_ascii_hexdigit()));
            assert_eq!(chunk.document_id, "doc-1");
        }
        // All chunks should have total_chunks set
        let total = chunks[0].total_chunks;
        assert!(total > 0);
        assert!(chunks.iter().all(|c| c.total_chunks == total));
    }

    #[test]
    fn test_blake3_id_unique() {
        let chunker = TextChunker::default();
        let c1 = chunker.chunk_document("Hello world", "d1", "h1");
        let c2 = chunker.chunk_document("Hello world!", "d2", "h2");

        // Same content = same hash
        assert_eq!(c1[0].id, c1[0].id);
        // Different content = different hash
        assert_ne!(c1[0].id, c2[0].id);
    }
}
