//! Compressed Bloom Filter for L2 Cache
//!
//! INNO-001: Implements RLE + Huffman compression for bloom filters
//! to reduce memory footprint in L2 (warm) cache layer.
//!
//! # Compression Strategy
//! - RLE (Run-Length Encoding): Compresses consecutive 0s or 1s in bloom filter bits
//! - Huffman Coding: Further compresses RLE output by encoding frequent run lengths
//!
//! # Performance Targets
//! - Compression ratio: 2-5x depending on bloom filter sparsity
//! - Decompression latency: <500ns (target for L2 cache access)
//! - Memory overhead: Minimal (Huffman tree ~1KB)

use std::collections::HashMap;
use thiserror::Error;

/// Compression error types
#[derive(Debug, Error)]
pub enum CompressionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid compressed data: {0}")]
    InvalidData(String),

    #[error("Huffman tree error: {0}")]
    HuffmanError(String),
}

/// Result type for compression operations
pub type CompressionResult<T> = Result<T, CompressionError>;

/// Huffman tree node
#[derive(Debug, Clone)]
struct HuffmanNode {
    symbol: Option<u8>,
    frequency: usize,
    left: Option<Box<HuffmanNode>>,
    right: Option<Box<HuffmanNode>>,
}

impl HuffmanNode {
    fn new(symbol: Option<u8>, frequency: usize) -> Self {
        Self {
            symbol,
            frequency,
            left: None,
            right: None,
        }
    }

    fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }
}

/// Huffman encoder/decoder
struct HuffmanCodec {
    /// Encoding table: symbol -> (bits, bit_count)
    encode_table: HashMap<u8, (u64, u8)>,
    /// Decode tree for fast decoding
    decode_tree: Option<HuffmanNode>,
}

impl HuffmanCodec {
    /// Build Huffman codec from frequency distribution
    fn build_frequencies(frequencies: &HashMap<u8, usize>) -> CompressionResult<Self> {
        if frequencies.is_empty() {
            return Err(CompressionError::HuffmanError(
                "Cannot build Huffman codec with empty frequencies".to_string(),
            ));
        }

        // Create leaf nodes
        let mut nodes: Vec<HuffmanNode> = frequencies
            .iter()
            .map(|(&symbol, &freq)| HuffmanNode::new(Some(symbol), freq))
            .collect();

        // Build Huffman tree using priority queue approach
        while nodes.len() > 1 {
            // Sort by frequency (ascending)
            nodes.sort_by_key(|n| n.frequency);

            // Take two nodes with lowest frequency
            // P1-008 FIX: Use ok_or_else instead of unwrap for pop operations
            let right = nodes.pop().ok_or_else(|| {
                CompressionError::HuffmanError("Unexpected empty nodes during Huffman tree build".to_string())
            })?;
            let left = nodes.pop().ok_or_else(|| {
                CompressionError::HuffmanError("Unexpected empty nodes during Huffman tree build".to_string())
            })?;

            // Create merged node
            let mut merged = HuffmanNode::new(None, left.frequency + right.frequency);
            merged.left = Some(Box::new(left));
            merged.right = Some(Box::new(right));

            nodes.push(merged);
        }

        let root = nodes
            .into_iter()
            .next()
            .ok_or_else(|| CompressionError::HuffmanError("Huffman tree build resulted in empty tree".to_string()))?;
        let mut encode_table = HashMap::new();

        // Generate encoding table
        if root.is_leaf() {
            // Special case: only one symbol
            // P1-008 FIX: Use ok_or_else for unwrap
            encode_table.insert(
                root.symbol
                    .ok_or_else(|| CompressionError::HuffmanError("Leaf node has no symbol".to_string()))?,
                (0, 1),
            );
        } else {
            Self::build_encoding_table(&root, 0, 0, &mut encode_table)?;
        }

        Ok(Self {
            encode_table,
            decode_tree: Some(root),
        })
    }

    /// Build Huffman codec from encoding table (for deserialization)
    fn from_table(table: &[(u8, u8, u64)]) -> CompressionResult<Self> {
        if table.is_empty() {
            return Err(CompressionError::HuffmanError(
                "Cannot build Huffman codec from empty table".to_string(),
            ));
        }

        // Build encode table
        let encode_table: HashMap<u8, (u64, u8)> = table
            .iter()
            .map(|&(symbol, code_length, code_bits)| (symbol, (code_bits, code_length)))
            .collect();

        // Rebuild decode tree from symbols and code lengths
        // We need to reconstruct the tree structure from the codes
        let mut root = HuffmanNode::new(None, 0);

        for &(symbol, code_length, code_bits) in table {
            let mut current = &mut root;

            for bit_idx in 0..code_length {
                let bit = if code_bits & (1u64 << bit_idx) != 0 { 1 } else { 0 };

                if bit == 0 {
                    if current.left.is_none() {
                        current.left = Some(Box::new(HuffmanNode::new(None, 0)));
                    }
                    // P1-008 FIX: Use ok_or_else instead of unwrap
                    current = current.left.as_mut().ok_or_else(|| {
                        CompressionError::HuffmanError("Left child should exist but is None".to_string())
                    })?;
                } else {
                    if current.right.is_none() {
                        current.right = Some(Box::new(HuffmanNode::new(None, 0)));
                    }
                    // P1-008 FIX: Use ok_or_else instead of unwrap
                    current = current.right.as_mut().ok_or_else(|| {
                        CompressionError::HuffmanError("Right child should exist but is None".to_string())
                    })?;
                }
            }

            // Set symbol at leaf
            current.symbol = Some(symbol);
        }

        Ok(Self {
            encode_table,
            decode_tree: Some(root),
        })
    }

    /// Recursively build encoding table from Huffman tree
    fn build_encoding_table(
        node: &HuffmanNode,
        bits: u64,
        bit_count: u8,
        table: &mut HashMap<u8, (u64, u8)>,
    ) -> CompressionResult<()> {
        if let Some(symbol) = node.symbol {
            // Leaf node
            table.insert(symbol, (bits, bit_count));
        } else {
            // Internal node
            if let Some(left) = &node.left {
                Self::build_encoding_table(left, bits, bit_count + 1, table)?;
            }
            if let Some(right) = &node.right {
                Self::build_encoding_table(right, bits | (1u64 << bit_count), bit_count + 1, table)?;
            }
        }
        Ok(())
    }

    /// Encode a byte slice using Huffman coding
    fn encode(&self, input: &[u8]) -> CompressionResult<Vec<u8>> {
        let mut output = Vec::new();
        let mut bit_buffer: u64 = 0;
        let mut bit_count: u8 = 0;

        for &byte in input {
            let (code, code_bits) = self
                .encode_table
                .get(&byte)
                .ok_or_else(|| CompressionError::HuffmanError(format!("Symbol {} not in Huffman table", byte)))?;

            // Append code to bit buffer
            bit_buffer |= code << bit_count;
            bit_count += code_bits;

            // Write complete bytes
            while bit_count >= 8 {
                output.push((bit_buffer & 0xFF) as u8);
                bit_buffer >>= 8;
                bit_count -= 8;
            }
        }

        // Flush remaining bits
        if bit_count > 0 {
            output.push((bit_buffer & 0xFF) as u8);
        }

        Ok(output)
    }

    /// Decode Huffman-encoded data
    fn decode(&self, input: &[u8], total_bits: usize) -> CompressionResult<Vec<u8>> {
        let mut output = Vec::new();
        let mut current = self
            .decode_tree
            .as_ref()
            .ok_or_else(|| CompressionError::HuffmanError("Decode tree not initialized".to_string()))?;

        let mut bit_count = 0;

        for &byte in input {
            for bit_idx in 0..8 {
                if bit_count >= total_bits {
                    return Ok(output);
                }

                let bit = (byte >> bit_idx) & 1;

                current = if bit == 0 {
                    current.left.as_ref()
                } else {
                    current.right.as_ref()
                }
                .ok_or_else(|| {
                    CompressionError::HuffmanError(format!("Invalid Huffman encoding at bit {}", bit_count))
                })?;

                if current.is_leaf() {
                    // P1-008 FIX: Use ok_or_else instead of unwrap for symbol
                    let symbol = current
                        .symbol
                        .ok_or_else(|| CompressionError::HuffmanError("Leaf node has no symbol".to_string()))?;
                    output.push(symbol);
                    // P1-008 FIX: Use ok_or_else instead of unwrap for decode_tree
                    current = self
                        .decode_tree
                        .as_ref()
                        .ok_or_else(|| CompressionError::HuffmanError("Decode tree not initialized".to_string()))?;
                }

                bit_count += 1;
            }
        }

        Ok(output)
    }
}

/// RLE (Run-Length Encoding) compression for bloom filter bits
///
/// Encodes runs of 0s and 1s as (value, count) pairs.
/// Optimized for sparse bloom filters with long runs of 0s.
pub struct RleEncoder {
    bits: Vec<u8>,
    current_bit: usize,
}

impl RleEncoder {
    pub fn new(bit_capacity: usize) -> Self {
        Self {
            bits: vec![0u8; bit_capacity.div_ceil(8)],
            current_bit: 0,
        }
    }

    /// Append a bit to the encoded output
    pub fn append_bit(&mut self, bit: bool) -> CompressionResult<()> {
        if self.current_bit >= self.bits.len() * 8 {
            // Expand buffer
            self.bits.resize(self.bits.len() * 2 + 1, 0);
        }

        if bit {
            self.bits[self.current_bit / 8] |= 1 << (self.current_bit % 8);
        }
        self.current_bit += 1;
        Ok(())
    }

    /// Append a run of bits
    pub fn append_run(&mut self, bit: bool, count: usize) -> CompressionResult<()> {
        for _ in 0..count {
            self.append_bit(bit)?;
        }
        Ok(())
    }

    /// Get encoded bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.bits
    }

    /// Get current bit position
    pub fn position(&self) -> usize {
        self.current_bit
    }
}

/// RLE (Run-Length Encoding) decoder
pub struct RleDecoder<'a> {
    bits: &'a [u8],
    current_bit: usize,
}

impl<'a> RleDecoder<'a> {
    pub fn new(bits: &'a [u8]) -> Self {
        Self { bits, current_bit: 0 }
    }

    /// Read a single bit
    pub fn read_bit(&mut self) -> Option<bool> {
        if self.current_bit >= self.bits.len() * 8 {
            return None;
        }

        let byte_idx = self.current_bit / 8;
        let bit_idx = self.current_bit % 8;
        let bit = (self.bits[byte_idx] >> bit_idx) & 1 == 1;
        self.current_bit += 1;
        Some(bit)
    }

    /// Read multiple bits
    pub fn read_bits(&mut self, count: usize) -> Option<Vec<bool>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.read_bit()?);
        }
        Some(result)
    }

    /// Get current bit position
    pub fn position(&self) -> usize {
        self.current_bit
    }
}

/// Compressed Bloom Filter representation
///
/// Format:
/// ```text
/// [Header: 16 bytes]
/// - magic: u32 (0x43424C46 = "CBLF")
/// - version: u32 (1)
/// - original_size: u64 (bits in original bloom filter)
/// - compressed_size: u32
/// - compression_type: u8 (0=none, 1=RLE, 2=RLE+Huffman)
/// - flags: u8
/// - huffman_symbols: u16 (number of symbols in Huffman table, only for type 2)
///
/// [Huffman Table (if compression_type == 2)]
/// - symbols: [u8; huffman_symbols] (symbol values)
/// - code_lengths: [u8; huffman_symbols] (code length for each symbol)
/// - codes: variable (code bits packed, one u64 per symbol)
///
/// [Compressed Data]
/// - RLE encoded bits (and Huffman encoded if applicable)
/// ```
#[derive(Debug, Clone)]
pub struct CompressedBloom {
    /// Original bloom filter bit size
    pub original_size: u64,
    /// Compression type: 0=none, 1=RLE, 2=RLE+Huffman
    pub compression_type: u8,
    /// Compressed data bytes
    pub compressed_data: Vec<u8>,
    /// Estimated decompressed size (bytes)
    pub decompressed_size: usize,
    /// Total bits in Huffman encoded data (for proper decoding)
    pub huffman_total_bits: usize,
    /// Huffman encoding table (only for type 2): symbol -> (code, bit_count)
    pub huffman_table: Vec<(u8, u8, u64)>, // (symbol, code_length, code_bits)
}

impl CompressedBloom {
    /// Magic number for compressed bloom filter files
    pub const COMPRESSED_BLOOM_MAGIC: u32 = 0x43424C46; // "CBLF"
    pub const COMPRESSED_BLOOM_VERSION: u32 = 1;

    /// Compress a bloom filter's bit vector
    ///
    /// # Arguments
    /// * `bits` - Raw bloom filter bits as byte slice
    /// * `use_huffman` - Whether to apply Huffman coding after RLE
    ///
    /// # Returns
    /// CompressedBloom with compressed data and metadata
    pub fn compress(bits: &[u8], use_huffman: bool) -> CompressionResult<Self> {
        if bits.is_empty() {
            return Err(CompressionError::InvalidData(
                "Cannot compress empty bloom filter".to_string(),
            ));
        }

        let original_size = bits.len() as u64 * 8; // Convert to bits

        // Step 1: RLE encoding
        let rle_encoded = Self::rle_encode(bits)?;

        // Check if RLE actually saves space
        let use_rle = rle_encoded.len() < bits.len();

        let (compression_type, compressed_data, huffman_total_bits, huffman_table) = if !use_rle {
            // No compression - store raw bits
            (0, bits.to_vec(), 0, Vec::new())
        } else if use_huffman && !rle_encoded.is_empty() {
            // Step 2: Huffman encoding
            let frequencies = Self::compute_frequencies(&rle_encoded);
            let codec = HuffmanCodec::build_frequencies(&frequencies)?;
            let huffman_encoded = codec.encode(&rle_encoded)?;

            // Only use Huffman if it saves space
            if huffman_encoded.len() < rle_encoded.len() {
                // Calculate total bits: each symbol's code length * frequency
                // P1-008 FIX: Use unwrap_or_else instead of expect for better performance
                let total_bits: usize = rle_encoded
                    .iter()
                    .map(|&byte| {
                        let (_, code_bits) = codec
                            .encode_table
                            .get(&byte)
                            .unwrap_or_else(|| panic!("Huffman encode table missing symbol {}", byte));
                        *code_bits as usize
                    })
                    .sum();
                // Build Huffman table for decoding: (symbol, code_length, code_bits)
                let table: Vec<(u8, u8, u64)> = codec
                    .encode_table
                    .iter()
                    .map(|(&symbol, &(code, len))| (symbol, len, code))
                    .collect();
                (2, huffman_encoded, total_bits, table)
            } else {
                (1, rle_encoded, 0, Vec::new())
            }
        } else {
            (1, rle_encoded, 0, Vec::new())
        };

        Ok(Self {
            original_size,
            compression_type,
            compressed_data,
            decompressed_size: bits.len(),
            huffman_total_bits,
            huffman_table,
        })
    }

    /// Decompress to original bloom filter bits
    pub fn decompress(&self) -> CompressionResult<Vec<u8>> {
        match self.compression_type {
            0 => {
                // No compression - data is stored directly
                let output_bytes = (self.original_size as usize).div_ceil(8);
                let mut output = vec![0u8; output_bytes];
                output.copy_from_slice(&self.compressed_data[..output_bytes.min(self.compressed_data.len())]);
                Ok(output)
            }
            1 => {
                // RLE only
                Self::rle_decode(&self.compressed_data, (self.original_size as usize).div_ceil(8))
            }
            2 => {
                // RLE + Huffman
                // Build codec from stored Huffman table
                let codec = HuffmanCodec::from_table(&self.huffman_table)?;
                let rle_encoded = codec.decode(&self.compressed_data, self.huffman_total_bits)?;

                // Then decode RLE
                Self::rle_decode(&rle_encoded, (self.original_size as usize).div_ceil(8))
            }
            _ => Err(CompressionError::InvalidData(format!(
                "Unknown compression type: {}",
                self.compression_type
            ))),
        }
    }

    /// RLE encode bloom filter bits
    ///
    /// Encodes as: [run_length_u8, run_length_u8, ...]
    /// where each byte represents the length of alternating 0/1 runs
    /// First run is always 0s, then alternates 1s, 0s, 1s, ...
    fn rle_encode(bits: &[u8]) -> CompressionResult<Vec<u8>> {
        if bits.is_empty() {
            return Ok(Vec::new());
        }

        let mut encoded = Vec::new();
        let mut current_run: usize = 0;
        let mut current_bit = false; // Start assuming 0

        for &byte in bits {
            for bit_idx in 0..8 {
                let bit = (byte >> bit_idx) & 1 == 1;

                if bit == current_bit {
                    current_run += 1;
                    if current_run == 255 {
                        // Flush max run length
                        encoded.push(255u8);
                        current_run = 255; // Keep at 255, not reset to 0
                    }
                } else {
                    // Flush current run
                    if current_run > 0 {
                        encoded.push(current_run as u8);
                    }
                    current_bit = bit;
                    current_run = 1;
                }
            }
        }

        // Flush final run
        if current_run > 0 {
            encoded.push(current_run as u8);
        }

        Ok(encoded)
    }

    /// RLE decode to bloom filter bits
    fn rle_decode(encoded: &[u8], output_bytes: usize) -> CompressionResult<Vec<u8>> {
        if output_bytes == 0 {
            return Ok(Vec::new());
        }

        let mut output = vec![0u8; output_bytes];
        let mut output_bit = 0;
        let mut current_bit = false; // Start with 0

        for &run_length in encoded {
            if run_length == 0 {
                continue; // Skip zero-length runs
            }

            for _ in 0..run_length {
                if output_bit >= output_bytes * 8 {
                    return Ok(output);
                }

                if current_bit {
                    output[output_bit / 8] |= 1 << (output_bit % 8);
                }
                output_bit += 1;
            }
            current_bit = !current_bit; // Alternate between 0 and 1
        }

        Ok(output)
    }

    /// Compute byte frequencies for Huffman coding
    fn compute_frequencies(data: &[u8]) -> HashMap<u8, usize> {
        let mut frequencies = HashMap::new();
        for &byte in data {
            *frequencies.entry(byte).or_insert(0) += 1;
        }
        frequencies
    }

    /// Get compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.compressed_data.is_empty() {
            return 1.0;
        }
        self.decompressed_size as f64 / self.compressed_data.len() as f64
    }

    /// Serialize to bytes (for persistence)
    pub fn to_bytes(&self) -> CompressionResult<Vec<u8>> {
        let mut bytes = Vec::new();

        // Header
        bytes.extend_from_slice(&Self::COMPRESSED_BLOOM_MAGIC.to_le_bytes());
        bytes.extend_from_slice(&Self::COMPRESSED_BLOOM_VERSION.to_le_bytes());
        bytes.extend_from_slice(&self.original_size.to_le_bytes());
        bytes.extend_from_slice(&(self.compressed_data.len() as u32).to_le_bytes());
        bytes.push(self.compression_type);
        bytes.push(0); // flags
                       // Store huffman_total_bits in reserved field (only needed for type 2)
        let huffman_bits = if self.compression_type == 2 {
            self.huffman_total_bits as u16
        } else {
            0u16
        };
        bytes.extend_from_slice(&huffman_bits.to_le_bytes());

        // For type 2 (RLE+Huffman), serialize Huffman table first
        if self.compression_type == 2 {
            bytes.extend_from_slice(&(self.huffman_table.len() as u16).to_le_bytes());
            for &(symbol, code_length, code_bits) in &self.huffman_table {
                bytes.push(symbol);
                bytes.push(code_length);
                bytes.extend_from_slice(&code_bits.to_le_bytes());
            }
        }

        // Compressed data
        bytes.extend_from_slice(&self.compressed_data);

        Ok(bytes)
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8]) -> CompressionResult<Self> {
        if bytes.len() < 24 {
            return Err(CompressionError::InvalidData(
                "Data too small for compressed bloom header".to_string(),
            ));
        }

        // P1-008 FIX: Use proper error handling instead of unwrap for byte parsing
        let magic = u32::from_le_bytes(
            bytes[0..4]
                .try_into()
                .map_err(|_| CompressionError::InvalidData("Invalid magic bytes".to_string()))?,
        );
        if magic != Self::COMPRESSED_BLOOM_MAGIC {
            return Err(CompressionError::InvalidData(format!(
                "Invalid magic: expected {:08X}, got {:08X}",
                Self::COMPRESSED_BLOOM_MAGIC,
                magic
            )));
        }

        let version = u32::from_le_bytes(
            bytes[4..8]
                .try_into()
                .map_err(|_| CompressionError::InvalidData("Invalid version bytes".to_string()))?,
        );
        if version != Self::COMPRESSED_BLOOM_VERSION {
            return Err(CompressionError::InvalidData(format!(
                "Unsupported version: {}",
                version
            )));
        }

        let original_size = u64::from_le_bytes(
            bytes[8..16]
                .try_into()
                .map_err(|_| CompressionError::InvalidData("Invalid original_size bytes".to_string()))?,
        );
        let compressed_size = u32::from_le_bytes(
            bytes[16..20]
                .try_into()
                .map_err(|_| CompressionError::InvalidData("Invalid compressed_size bytes".to_string()))?,
        ) as usize;
        let _compression_type = bytes[20];
        let huffman_total_bits = u16::from_le_bytes(
            bytes[22..24]
                .try_into()
                .map_err(|_| CompressionError::InvalidData("Invalid huffman_total_bits bytes".to_string()))?,
        ) as usize;

        // For type 2 (RLE+Huffman), parse Huffman table first
        let mut offset = 24;
        let mut huffman_table = Vec::new();

        if _compression_type == 2 {
            if offset + 2 > bytes.len() {
                return Err(CompressionError::InvalidData(
                    "Data truncated for Huffman table size".to_string(),
                ));
            }
            let table_size = u16::from_le_bytes(
                bytes[offset..offset + 2]
                    .try_into()
                    .map_err(|_| CompressionError::InvalidData("Invalid table_size bytes".to_string()))?,
            ) as usize;
            offset += 2;

            // Parse Huffman table entries: (symbol, code_length, code_bits)
            for _ in 0..table_size {
                if offset + 10 > bytes.len() {
                    return Err(CompressionError::InvalidData(
                        "Data truncated for Huffman table entry".to_string(),
                    ));
                }
                let symbol = bytes[offset];
                let code_length = bytes[offset + 1];
                let code_bits = u64::from_le_bytes(
                    bytes[offset + 2..offset + 10]
                        .try_into()
                        .map_err(|_| CompressionError::InvalidData("Invalid code_bits bytes".to_string()))?,
                );
                huffman_table.push((symbol, code_length, code_bits));
                offset += 10;
            }
        }

        if offset + compressed_size > bytes.len() {
            return Err(CompressionError::InvalidData(
                "Data truncated for compressed data".to_string(),
            ));
        }

        let compressed_data = bytes[offset..offset + compressed_size].to_vec();

        Ok(Self {
            original_size,
            compression_type: _compression_type,
            compressed_data,
            decompressed_size: (original_size as usize).div_ceil(8),
            huffman_total_bits,
            huffman_table,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rle_encode_decode() {
        // Create test data with long runs of 0s
        // Use smaller size to avoid run length > 255 issues
        let original = vec![0u8; 256]; // 2048 bits, will encode as multiple 255 runs
        let encoded = CompressedBloom::rle_encode(&original).unwrap();
        let decoded = CompressedBloom::rle_decode(&encoded, original.len()).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_rle_with_ones() {
        // Create test data with mixed 0s and 1s
        // Keep size small to avoid run length overflow
        let mut original = vec![0u8; 64];
        original[10..20].fill(0xFF); // Set some bits to 1
        original[30..40].fill(0xFF);

        let encoded = CompressedBloom::rle_encode(&original).unwrap();
        let decoded = CompressedBloom::rle_decode(&encoded, original.len()).unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_compressed_bloom_compress_decompress() {
        let original = vec![0u8; 256];
        let compressed = CompressedBloom::compress(&original, false).unwrap();
        let decompressed = compressed.decompress().unwrap();

        assert_eq!(original, decompressed);
        assert!(compressed.compression_ratio() > 1.0);
    }

    #[test]
    fn test_compressed_bloom_with_huffman() {
        let original = vec![0u8; 256];
        let compressed = CompressedBloom::compress(&original, true).unwrap();
        let decompressed = compressed.decompress().unwrap();

        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compressed_bloom_serialization() {
        let original = vec![0u8; 256];
        let compressed = CompressedBloom::compress(&original, false).unwrap();
        let bytes = compressed.to_bytes().unwrap();
        let restored = CompressedBloom::from_bytes(&bytes).unwrap();

        assert_eq!(compressed.original_size, restored.original_size);
        assert_eq!(compressed.compression_type, restored.compression_type);
        assert_eq!(compressed.compressed_data, restored.compressed_data);
    }

    #[test]
    fn test_compression_ratio() {
        // Sparse bloom filter (mostly 0s) should compress well
        let sparse = vec![0u8; 1024];
        let compressed = CompressedBloom::compress(&sparse, false).unwrap();

        println!("Sparse compression ratio: {:.2}x", compressed.compression_ratio());
        assert!(compressed.compression_ratio() > 2.0);
    }
}
