//! # Zstd 压缩实现
//!
//! 本模块支持两种压缩模式：
//! 1. **Plain zstd 压缩**: 直接使用 zstd 压缩（当前默认）
//! 2. **字典训练压缩**: 使用样本数据训练字典，然后用字典进行压缩/解压
//!
//! ## 字典训练流程
//! 1. 收集样本数据（如前 N 个 value）
//! 2. 使用 `DictionaryTrainer` 训练字典
//! 3. 持久化字典到磁盘
//! 4. 加载字典，使用 `DictionaryCompressor` 进行字典压缩/解压

use std::path::Path;
use std::sync::Arc;

use crate::core::types::{BlockCompressionConfig, BlockCompressionMode};

/// 字典文件格式标识
const DICT_MAGIC: [u8; 4] = [0xF1, 0x4E, 0x4B, 0x56]; // "NKV" + 0xF1

/// # Zstd 压缩配置
///
/// 支持两种模式：
/// - plain zstd 压缩（当前默认）
/// - 字典训练压缩（需手动启用）
#[derive(Debug, Clone)]
pub struct DictionaryCompressionConfig {
    /// 启用 zstd 压缩
    pub enable_dictionary: bool,
    /// 字典训练时的目标字典大小（字节）
    pub dictionary_size: usize,
    /// 未使用 - 预留给未来字典训练的最小词长度
    pub min_word_length: usize,
}

impl Default for DictionaryCompressionConfig {
    fn default() -> Self {
        Self {
            enable_dictionary: false,
            dictionary_size: 10000,
            min_word_length: 3,
        }
    }
}

/// Dictionary compression statistics
#[derive(Debug, Clone, Default)]
pub struct DictionaryStats {
    pub dictionary_size: usize,
    pub compression_ratio: f64,
    pub words_learned: usize,
}

/// # 字典压缩器 (Zstd 实现)
///
/// 支持 plain zstd 压缩和字典训练压缩两种模式。
///
/// ## 使用方式
/// - **Plain 模式**: 直接创建 `DictionaryCompressor::new(config)`，调用 `compress`/`decompress`
/// - **字典模式**: 先使用 `DictionaryTrainer` 训练字典，然后通过 `load_dictionary_data()` 加载
pub struct DictionaryCompressor {
    config: DictionaryCompressionConfig,
    stats: DictionaryStats,
    /// 已训练的字典数据（原始字节）
    dictionary: Option<Vec<u8>>,
    /// 预创建的压缩字典（CDict），用于高效复用
    cdict: Option<Arc<zstd::dict::EncoderDictionary<'static>>>,
    /// 预创建的解压字典（DDict），用于高效复用
    ddict: Option<Arc<zstd::dict::DecoderDictionary<'static>>>,
}

impl DictionaryCompressor {
    pub fn new(config: DictionaryCompressionConfig) -> Self {
        Self {
            stats: DictionaryStats::default(),
            config,
            dictionary: None,
            cdict: None,
            ddict: None,
        }
    }

    /// 加载已训练的字典数据，用于后续的压缩/解压
    ///
    /// 此方法会预创建 CDict 和 DDict，后续压缩/解压操作将复用这些预创建对象，
    /// 避免每次操作都重新创建压缩器/解压器，显著提升性能。
    pub fn load_dictionary_data(&mut self, dict: Vec<u8>) {
        // 预创建 CDict（压缩级别使用默认值 3）
        let cdict = Arc::new(zstd::dict::EncoderDictionary::copy(&dict, 3));
        // 预创建 DDict
        let ddict = Arc::new(zstd::dict::DecoderDictionary::copy(&dict));

        self.stats.dictionary_size = dict.len();
        self.dictionary = Some(dict);
        self.cdict = Some(cdict);
        self.ddict = Some(ddict);
    }

    /// 检查是否已加载字典
    pub fn has_dictionary(&self) -> bool {
        self.dictionary.is_some()
    }

    /// Compress data using dictionary compression
    /// - 如果有字典，使用预创建的 CDict 进行压缩
    /// - 否则使用 plain zstd 压缩
    pub fn compress(&self, input: &[u8]) -> anyhow::Result<Vec<u8>> {
        if !self.config.enable_dictionary {
            return Ok(input.to_vec());
        }

        // 如果有预创建的 CDict，使用它进行高效压缩
        if let Some(ref cdict) = self.cdict {
            let mut compressor = zstd::bulk::Compressor::with_prepared_dictionary(cdict.as_ref())
                .map_err(|e| anyhow::anyhow!("zstd dictionary compressor creation failed: {}", e))?;
            let compressed = compressor
                .compress(input)
                .map_err(|e| anyhow::anyhow!("zstd dictionary compression failed: {}", e))?;
            return Ok(compressed);
        }

        // 无字典，使用 plain zstd
        let compressed = zstd::encode_all(input, 3).map_err(|e| anyhow::anyhow!("zstd compression failed: {}", e))?;

        Ok(compressed)
    }

    /// Decompress data
    pub fn decompress(&self, input: &[u8]) -> anyhow::Result<Vec<u8>> {
        if !self.config.enable_dictionary {
            return Ok(input.to_vec());
        }

        // 如果有预创建的 DDict，使用它进行高效解压
        if let Some(ref ddict) = self.ddict {
            let mut decompressor = zstd::bulk::Decompressor::with_prepared_dictionary(ddict.as_ref())
                .map_err(|e| anyhow::anyhow!("zstd dictionary decompressor creation failed: {}", e))?;
            // decompress 需要预估大小，先尝试用 10x 压缩大小作为上限
            let capacity = input.len().saturating_mul(10).max(1024);
            let decompressed = decompressor
                .decompress(input, capacity)
                .map_err(|e| anyhow::anyhow!("zstd dictionary decompression failed: {}", e))?;
            return Ok(decompressed);
        }

        // 无字典，使用 plain zstd
        let decompressed = zstd::decode_all(input).map_err(|e| anyhow::anyhow!("zstd decompression failed: {}", e))?;

        Ok(decompressed)
    }

    /// Get configuration
    pub fn config(&self) -> &DictionaryCompressionConfig {
        &self.config
    }

    /// Get statistics
    pub fn stats(&self) -> &DictionaryStats {
        &self.stats
    }
}

// ============================================================
// DictionaryTrainer - 字典训练器
// ============================================================

/// 字典训练器
///
/// 收集样本数据并使用 zstd 的字典训练 API 生成字典。
///
/// ## 使用示例
/// ```ignore
/// let mut trainer = DictionaryTrainer::new(1000, 10000);
/// trainer.add_sample(sample1);
/// trainer.add_sample(sample2);
/// if trainer.is_ready() {
///     let dict = trainer.train()?;
///     DictionaryTrainer::save_dictionary(&dict, path)?;
/// }
/// ```
pub struct DictionaryTrainer {
    samples: Vec<Vec<u8>>,
    max_samples: usize,
    dictionary_size: usize,
}

impl DictionaryTrainer {
    /// 创建新的字典训练器
    ///
    /// - `max_samples`: 最大样本数量
    /// - `dictionary_size`: 目标字典大小（字节）
    pub fn new(max_samples: usize, dictionary_size: usize) -> Self {
        Self {
            samples: Vec::with_capacity(max_samples),
            max_samples,
            dictionary_size,
        }
    }

    /// 添加一个样本数据
    ///
    /// 如果已达到最大样本数，则忽略新样本
    pub fn add_sample(&mut self, sample: &[u8]) {
        if self.samples.len() < self.max_samples && !sample.is_empty() {
            self.samples.push(sample.to_vec());
        }
    }

    /// 批量添加样本
    pub fn add_samples(&mut self, samples: impl IntoIterator<Item = Vec<u8>>) {
        for sample in samples {
            self.add_sample(&sample);
        }
    }

    /// 检查是否已收集足够的样本（至少 1 个）
    pub fn is_ready(&self) -> bool {
        !self.samples.is_empty()
    }

    /// 获取已收集的样本数量
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }

    /// 训练字典
    ///
    /// 使用 zstd::dict::from_samples API 训练字典
    /// 返回原始字典字节，可用于创建 CDict/DDict 或持久化
    pub fn train(&self) -> anyhow::Result<Vec<u8>> {
        if !self.is_ready() {
            return Err(anyhow::anyhow!(
                "Not enough samples for dictionary training. Need at least 1, got {}",
                self.samples.len()
            ));
        }

        // 转换为 & [&[u8]] 格式
        let sample_refs: Vec<&[u8]> = self.samples.iter().map(|s| s.as_slice()).collect();

        let dict = zstd::dict::from_samples(&sample_refs, self.dictionary_size)
            .map_err(|e| anyhow::anyhow!("Dictionary training failed: {}", e))?;

        Ok(dict)
    }

    /// 保存字典到文件
    ///
    /// 格式: [magic 4B][size 4B][dict data][crc32 4B]
    pub fn save_dictionary(dict: &[u8], path: &Path) -> anyhow::Result<()> {
        use crc32c::crc32c;

        let mut buffer = Vec::with_capacity(4 + 4 + dict.len() + 4);

        // Magic bytes
        buffer.extend_from_slice(&DICT_MAGIC);

        // Dictionary size (little-endian)
        buffer.extend_from_slice(&(dict.len() as u32).to_le_bytes());

        // Dictionary data
        buffer.extend_from_slice(dict);

        // CRC32 of dict data + size
        let mut crc_data = Vec::with_capacity(4 + dict.len());
        crc_data.extend_from_slice(&(dict.len() as u32).to_le_bytes());
        crc_data.extend_from_slice(dict);
        let crc = crc32c(&crc_data);
        buffer.extend_from_slice(&crc.to_le_bytes());

        std::fs::write(path, &buffer).map_err(|e| anyhow::anyhow!("Failed to write dictionary file: {}", e))?;

        Ok(())
    }

    /// 从文件加载字典并验证校验和
    ///
    /// 格式: [magic 4B][size 4B][dict data][crc32 4B]
    pub fn load_dictionary(path: &Path) -> anyhow::Result<Vec<u8>> {
        use crc32c::crc32c;

        let data = std::fs::read(path).map_err(|e| anyhow::anyhow!("Failed to read dictionary file: {}", e))?;

        if data.len() < 12 {
            return Err(anyhow::anyhow!("Dictionary file too small"));
        }

        // Check magic
        if data[0..4] != DICT_MAGIC {
            return Err(anyhow::anyhow!(
                "Invalid dictionary file magic: expected {:?}, got {:?}",
                DICT_MAGIC,
                &data[0..4]
            ));
        }

        // Read size
        let dict_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;

        if data.len() < 12 + dict_size {
            return Err(anyhow::anyhow!(
                "Dictionary file truncated: expected {} bytes of dict data",
                dict_size
            ));
        }

        let dict_data = data[8..8 + dict_size].to_vec();

        // Verify CRC32
        let stored_crc = u32::from_le_bytes([
            data[8 + dict_size],
            data[8 + dict_size + 1],
            data[8 + dict_size + 2],
            data[8 + dict_size + 3],
        ]);

        let mut crc_data = Vec::with_capacity(4 + dict_size);
        crc_data.extend_from_slice(&(dict_size as u32).to_le_bytes());
        crc_data.extend_from_slice(&dict_data);
        let computed_crc = crc32c(&crc_data);

        if stored_crc != computed_crc {
            return Err(anyhow::anyhow!(
                "Dictionary checksum mismatch: stored {:08x}, computed {:08x}",
                stored_crc,
                computed_crc
            ));
        }

        Ok(dict_data)
    }
}

/// Compress a block of data using the configured compression mode.
/// Returns (compressed_data, original_size, is_compressed, algorithm_id).
/// If the data is too small or compression increases size, returns original data.
pub fn compress_block(data: &[u8], config: &BlockCompressionConfig) -> anyhow::Result<(Vec<u8>, u32, bool, u8)> {
    match config.mode {
        BlockCompressionMode::None => Ok((data.to_vec(), data.len() as u32, false, 0)),
        BlockCompressionMode::Zstd => {
            if data.len() < config.min_compress_size as usize {
                // Skip compression for small blocks
                return Ok((data.to_vec(), data.len() as u32, false, 0));
            }
            let compressed = zstd::encode_all(data, config.compression_level)
                .map_err(|e| anyhow::anyhow!("zstd block compression failed: {}", e))?;
            // If compressed data is larger, return original
            if compressed.len() >= data.len() {
                Ok((data.to_vec(), data.len() as u32, false, 0))
            } else {
                Ok((compressed, data.len() as u32, true, config.mode.algorithm_id()))
            }
        }
        BlockCompressionMode::Snappy => {
            if data.len() < config.min_compress_size as usize {
                return Ok((data.to_vec(), data.len() as u32, false, 0));
            }
            let max_compressed = snap::raw::max_compress_len(data.len());
            let mut output = vec![0u8; max_compressed];
            let written = snap::raw::Encoder::new()
                .compress(data, &mut output)
                .map_err(|e| anyhow::anyhow!("snappy block compression failed: {}", e))?;
            output.truncate(written);
            if output.len() >= data.len() {
                Ok((data.to_vec(), data.len() as u32, false, 0))
            } else {
                Ok((output, data.len() as u32, true, config.mode.algorithm_id()))
            }
        }
        BlockCompressionMode::Lz4 => {
            if data.len() < config.min_compress_size as usize {
                return Ok((data.to_vec(), data.len() as u32, false, 0));
            }
            let mode = if config.compression_level <= 0 {
                lz4::block::CompressionMode::FAST(0)
            } else {
                lz4::block::CompressionMode::HIGHCOMPRESSION(config.compression_level.min(12))
            };
            // Use prepend_size=true to store original size in output for decompression
            let compressed = lz4::block::compress(data, Some(mode), true)
                .map_err(|e| anyhow::anyhow!("lz4 block compression failed: {}", e))?;
            if compressed.len() >= data.len() {
                Ok((data.to_vec(), data.len() as u32, false, 0))
            } else {
                Ok((compressed, data.len() as u32, true, config.mode.algorithm_id()))
            }
        }
    }
}

/// Decompress a block of data with the specified algorithm.
/// If is_compressed is false, returns the data as-is.
pub fn decompress_block(data: &[u8], is_compressed: bool, algorithm_id: u8) -> anyhow::Result<Vec<u8>> {
    if !is_compressed {
        return Ok(data.to_vec());
    }
    match algorithm_id {
        1 => {
            // Zstd
            let decompressed =
                zstd::decode_all(data).map_err(|e| anyhow::anyhow!("zstd block decompression failed: {}", e))?;
            Ok(decompressed)
        }
        2 => {
            // Snappy
            let len =
                snap::raw::decompress_len(data).map_err(|e| anyhow::anyhow!("snappy decompress_len failed: {}", e))?;
            let mut output = vec![0u8; len];
            snap::raw::Decoder::new()
                .decompress(data, &mut output)
                .map_err(|e| anyhow::anyhow!("snappy block decompression failed: {}", e))?;
            Ok(output)
        }
        3 => {
            // LZ4 - uses prepend_size=true, so decompress can use None for size
            let decompressed = lz4::block::decompress(data, None)
                .map_err(|e| anyhow::anyhow!("lz4 block decompression failed: {}", e))?;
            Ok(decompressed)
        }
        _ => Err(anyhow::anyhow!("unknown compression algorithm id: {}", algorithm_id)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dictionary_compression_roundtrip() {
        // S2-1: Test dictionary compression end-to-end
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        // Test with repetitive data (should compress well)
        let original = b"Hello World! This is a test value with some repetition. Hello again!";
        let compressed = compressor.compress(original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original.as_slice(), decompressed.as_slice());
        // With zstd level 3, even small data should compress or stay similar size
        assert!(compressed.len() <= original.len() + 32); // Allow small overhead
    }

    #[test]
    fn test_compression_disabled() {
        let config = DictionaryCompressionConfig {
            enable_dictionary: false,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        let original = b"test data";
        let compressed = compressor.compress(original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original.as_slice(), decompressed.as_slice());
        assert_eq!(compressed.len(), original.len()); // No compression
    }

    #[test]
    fn test_compression_with_repeated_patterns() {
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        // Create data with repeated patterns (should compress very well)
        let mut original = Vec::new();
        for _ in 0..100 {
            original.extend_from_slice(b"The quick brown fox jumps over the lazy dog. ");
        }

        let compressed = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);
        // With repeated patterns, compression ratio should be significant
        let ratio = compressed.len() as f64 / original.len() as f64;
        assert!(
            ratio < 0.5,
            "Compression ratio should be < 0.5 for repeated data, got {:.2}",
            ratio
        );
    }

    #[test]
    fn test_block_compress_decompress_roundtrip() {
        let config = BlockCompressionConfig::default();
        let original = b"The quick brown fox jumps over the lazy dog. ";
        let (compressed, orig_size, is_compressed, algo_id) = compress_block(original, &config).unwrap();
        if is_compressed {
            let decompressed = decompress_block(&compressed, is_compressed, algo_id).unwrap();
            assert_eq!(original.as_slice(), decompressed.as_slice());
            assert_eq!(orig_size, original.len() as u32);
        }
    }

    #[test]
    fn test_block_compress_repeated_data() {
        let config = BlockCompressionConfig::default();
        let mut original = Vec::new();
        for _ in 0..100 {
            original.extend_from_slice(b"test_value_");
        }
        let (compressed, orig_size, is_compressed, algo_id) = compress_block(&original, &config).unwrap();
        assert!(is_compressed, "Repeated data should be compressed");
        assert_eq!(orig_size, original.len() as u32);
        let decompressed = decompress_block(&compressed, is_compressed, algo_id).unwrap();
        assert_eq!(original, decompressed);
        let ratio = compressed.len() as f64 / original.len() as f64;
        assert!(
            ratio < 0.5,
            "Compression ratio should be < 0.5 for repeated data, got {:.2}",
            ratio
        );
    }

    #[test]
    fn test_block_compress_small_data() {
        let config = BlockCompressionConfig {
            min_compress_size: 64,
            ..BlockCompressionConfig::default()
        };
        let small_data = b"tiny";
        let (compressed, orig_size, is_compressed, _algo_id) = compress_block(small_data, &config).unwrap();
        assert!(!is_compressed, "Small data should skip compression");
        assert_eq!(orig_size, small_data.len() as u32);
        assert_eq!(compressed, small_data.to_vec());
    }

    #[test]
    fn test_block_compress_no_compression_mode() {
        let config = BlockCompressionConfig {
            mode: BlockCompressionMode::None,
            ..BlockCompressionConfig::default()
        };
        let original = b"some data that would benefit from compression";
        let (compressed, orig_size, is_compressed, _algo_id) = compress_block(original, &config).unwrap();
        assert!(!is_compressed);
        assert_eq!(orig_size, original.len() as u32);
        assert_eq!(compressed, original.to_vec());
    }

    #[test]
    fn test_block_compress_snappy_roundtrip() {
        let config = BlockCompressionConfig {
            mode: BlockCompressionMode::Snappy,
            ..BlockCompressionConfig::default()
        };
        let mut original = Vec::new();
        for _ in 0..100 {
            original.extend_from_slice(b"test_value_for_snappy_");
        }
        let (compressed, orig_size, is_compressed, algo_id) = compress_block(&original, &config).unwrap();
        assert!(is_compressed, "Repeated data should be compressed with Snappy");
        assert_eq!(orig_size, original.len() as u32);
        assert_eq!(algo_id, BlockCompressionMode::Snappy.algorithm_id());
        let decompressed = decompress_block(&compressed, is_compressed, algo_id).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_block_compress_lz4_roundtrip() {
        let config = BlockCompressionConfig {
            mode: BlockCompressionMode::Lz4,
            ..BlockCompressionConfig::default()
        };
        let mut original = Vec::new();
        for _ in 0..100 {
            original.extend_from_slice(b"test_value_for_lz4_");
        }
        let (compressed, orig_size, is_compressed, algo_id) = compress_block(&original, &config).unwrap();
        assert!(is_compressed, "Repeated data should be compressed with LZ4");
        assert_eq!(orig_size, original.len() as u32);
        assert_eq!(algo_id, BlockCompressionMode::Lz4.algorithm_id());
        let decompressed = decompress_block(&compressed, is_compressed, algo_id).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_block_compress_algorithm_id_persistence() {
        // Verify that each algorithm produces a unique algorithm_id
        let data = b"test data for algorithm id verification";

        let config_none = BlockCompressionConfig {
            mode: BlockCompressionMode::None,
            ..BlockCompressionConfig::default()
        };
        let config_zstd = BlockCompressionConfig {
            mode: BlockCompressionMode::Zstd,
            ..BlockCompressionConfig::default()
        };
        let config_snappy = BlockCompressionConfig {
            mode: BlockCompressionMode::Snappy,
            ..BlockCompressionConfig::default()
        };
        let config_lz4 = BlockCompressionConfig {
            mode: BlockCompressionMode::Lz4,
            ..BlockCompressionConfig::default()
        };

        let (_, _, _, id_none) = compress_block(data, &config_none).unwrap();
        let (_, _, _, _id_zstd) = compress_block(data, &config_zstd).unwrap();
        let (_, _, _, _id_snappy) = compress_block(data, &config_snappy).unwrap();
        let (_, _, _, _id_lz4) = compress_block(data, &config_lz4).unwrap();

        assert_eq!(id_none, 0);
        // Zstd/Snappy/LZ4 may skip compression if data is too small, so id could be 0
        // But the algorithm_id() function should still return correct values
        assert_eq!(BlockCompressionMode::None.algorithm_id(), 0);
        assert_eq!(BlockCompressionMode::Zstd.algorithm_id(), 1);
        assert_eq!(BlockCompressionMode::Snappy.algorithm_id(), 2);
        assert_eq!(BlockCompressionMode::Lz4.algorithm_id(), 3);
    }

    // ─── Dictionary Training Tests ───

    #[test]
    fn test_dictionary_trainer_basic() {
        let mut trainer = DictionaryTrainer::new(100, 10000);
        assert!(!trainer.is_ready());
        assert_eq!(trainer.sample_count(), 0);

        // Add samples
        for i in 0..10 {
            let sample = format!("This is sample number {} with some repeated text patterns", i);
            trainer.add_sample(sample.as_bytes());
        }

        assert!(trainer.is_ready());
        assert_eq!(trainer.sample_count(), 10);

        // Train dictionary
        let dict = trainer.train().unwrap();
        assert!(!dict.is_empty());
        assert!(dict.len() <= 10000);
    }

    #[test]
    fn test_dictionary_trainer_max_samples() {
        let mut trainer = DictionaryTrainer::new(5, 10000);

        // Add more samples than max
        for i in 0..10 {
            let sample = format!("Sample {}", i);
            trainer.add_sample(sample.as_bytes());
        }

        assert_eq!(trainer.sample_count(), 5); // Capped at max
    }

    #[test]
    fn test_dictionary_trainer_empty_samples_ignored() {
        let mut trainer = DictionaryTrainer::new(100, 10000);
        trainer.add_sample(b"");
        trainer.add_sample(b"");
        assert!(!trainer.is_ready());

        trainer.add_sample(b"valid sample");
        assert!(trainer.is_ready());
        assert_eq!(trainer.sample_count(), 1);
    }

    #[test]
    fn test_dictionary_save_load_roundtrip() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dict_path = temp_dir.path().join("test.dict");

        // Train a dictionary
        let mut trainer = DictionaryTrainer::new(100, 10000);
        for i in 0..20 {
            let sample = format!("JSON data structure with key {} and value pattern", i);
            trainer.add_sample(sample.as_bytes());
        }
        let dict = trainer.train().unwrap();

        // Save
        DictionaryTrainer::save_dictionary(&dict, &dict_path).unwrap();

        // Load
        let loaded = DictionaryTrainer::load_dictionary(&dict_path).unwrap();

        assert_eq!(dict, loaded);
    }

    #[test]
    fn test_dictionary_save_load_invalid_magic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let dict_path = temp_dir.path().join("invalid.dict");

        // Write invalid data
        std::fs::write(&dict_path, b"not a dictionary").unwrap();

        let result = DictionaryTrainer::load_dictionary(&dict_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_compression_with_dictionary() {
        // Train a dictionary on sample data
        let mut trainer = DictionaryTrainer::new(100, 10000);
        for i in 0..50 {
            let sample = format!(
                r#"{{"key_{}": {{"name": "item_{}", "value": 12345, "type": "test"}}}}"#,
                i, i
            );
            trainer.add_sample(sample.as_bytes());
        }
        let dict = trainer.train().unwrap();

        // Create compressor with dictionary
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let mut compressor = DictionaryCompressor::new(config.clone());
        compressor.load_dictionary_data(dict);

        assert!(compressor.has_dictionary());

        // Compress and decompress similar data
        let test_data = br#"{"key_99": {"name": "item_99", "value": 12345, "type": "test"}}"#;
        let compressed = compressor.compress(test_data).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(test_data.as_slice(), decompressed.as_slice());

        // Also test plain compression without dictionary (should still work)
        let compressor_no_dict = DictionaryCompressor::new(config);
        let compressed2 = compressor_no_dict.compress(test_data).unwrap();
        let decompressed2 = compressor_no_dict.decompress(&compressed2).unwrap();
        assert_eq!(test_data.as_slice(), decompressed2.as_slice());
    }

    #[test]
    fn test_dictionary_vs_plain_compression() {
        // Train a dictionary on JSON-like data
        let mut trainer = DictionaryTrainer::new(500, 20000);
        for i in 0..200 {
            let sample = format!(
                r#"{{"id": {}, "name": "user_{}", "email": "user_{}@example.com", "data": "some repeated pattern data for compression"}}"#,
                i, i, i
            );
            trainer.add_sample(sample.as_bytes());
        }
        let dict = trainer.train().unwrap();

        // Test data (similar but not identical to training data)
        let test_data = br#"{"id": 999, "name": "user_999", "email": "user_999@example.com", "data": "some repeated pattern data for compression"}"#;

        // Compression with dictionary
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 20000,
            min_word_length: 3,
        };
        let mut compressor_with_dict = DictionaryCompressor::new(config.clone());
        compressor_with_dict.load_dictionary_data(dict);

        let compressed_dict = compressor_with_dict.compress(test_data).unwrap();
        let decompressed = compressor_with_dict.decompress(&compressed_dict).unwrap();
        assert_eq!(test_data.as_slice(), decompressed.as_slice());

        // Compression without dictionary (plain zstd)
        let compressor_plain = DictionaryCompressor::new(config);
        let compressed_plain = compressor_plain.compress(test_data).unwrap();

        // Dictionary compression should generally be better for similar data
        println!("Dictionary compressed size: {}", compressed_dict.len());
        println!("Plain compressed size: {}", compressed_plain.len());
        println!("Original size: {}", test_data.len());
    }

    #[test]
    fn test_dictionary_training_roundtrip() {
        // Full roundtrip: train -> save -> load -> compress -> decompress
        let temp_dir = tempfile::tempdir().unwrap();
        let dict_path = temp_dir.path().join("roundtrip.dict");

        // Train
        let mut trainer = DictionaryTrainer::new(100, 10000);
        for i in 0..30 {
            let sample = format!("Repeated pattern data for dictionary training sample {}", i);
            trainer.add_sample(sample.as_bytes());
        }
        let dict = trainer.train().unwrap();

        // Save
        DictionaryTrainer::save_dictionary(&dict, &dict_path).unwrap();

        // Load
        let loaded_dict = DictionaryTrainer::load_dictionary(&dict_path).unwrap();

        // Compress with dictionary
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let mut compressor = DictionaryCompressor::new(config);
        compressor.load_dictionary_data(loaded_dict);

        let original = b"Repeated pattern data for dictionary training sample 999";
        let compressed = compressor.compress(original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original.as_slice(), decompressed.as_slice());
    }

    // ─── COMPRESS-002: Dictionary Compression Boundary Tests ───

    #[test]
    fn test_compress_empty_value() {
        // Empty value should compress/decompress correctly
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        let original = b"";
        let compressed = compressor.compress(original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_compress_empty_value_block_mode() {
        // Empty value in block compression mode
        let config = BlockCompressionConfig::default();
        let original = b"";
        let (compressed, orig_size, is_compressed, algo_id) = compress_block(original, &config).unwrap();
        let decompressed = decompress_block(&compressed, is_compressed, algo_id).unwrap();

        assert_eq!(original.as_slice(), decompressed.as_slice());
        assert_eq!(orig_size, 0);
    }

    #[test]
    fn test_compress_large_value() {
        // 1MB+ value should compress and decompress correctly
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        // Create 1MB+ of repetitive data (compresses well)
        let mut original = Vec::with_capacity(1_200_000);
        let pattern = b"The quick brown fox jumps over the lazy dog. ";
        while original.len() < 1_200_000 {
            original.extend_from_slice(pattern);
        }

        let compressed = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);

        // With repetitive patterns, 1MB should compress significantly
        let ratio = compressed.len() as f64 / original.len() as f64;
        assert!(
            ratio < 0.1,
            "Large repetitive data should compress well, ratio: {:.4}",
            ratio
        );
    }

    #[test]
    fn test_compress_large_value_block_mode() {
        // 1MB+ value in block compression mode
        let config = BlockCompressionConfig::default();

        // Create 1MB+ of repetitive data
        let mut original = Vec::with_capacity(1_200_000);
        let pattern = b"test_value_repeated_many_times_";
        while original.len() < 1_200_000 {
            original.extend_from_slice(pattern);
        }

        let (compressed, orig_size, is_compressed, algo_id) = compress_block(&original, &config).unwrap();
        assert!(is_compressed, "Large repetitive data should be compressed");
        assert_eq!(orig_size, original.len() as u32);

        let decompressed = decompress_block(&compressed, is_compressed, algo_id).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compress_incompressible() {
        // Random data should not compress significantly (may even grow slightly)
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        // Generate 10KB of random data
        let mut original = vec![0u8; 10_000];
        rng.fill(&mut original[..]);

        let compressed = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);

        // Incompressible data may grow slightly due to compression overhead
        // but should not be dramatically smaller
        assert!(
            compressed.len() >= original.len() / 2,
            "Incompressible data should not compress dramatically"
        );
    }

    #[test]
    fn test_compress_incompressible_block_mode() {
        // Random data in block compression mode should skip compression if larger
        use rand::{Rng, SeedableRng};
        let mut rng = rand::rngs::StdRng::seed_from_u64(42);

        let config = BlockCompressionConfig::default();

        // Generate random data
        let mut original = vec![0u8; 10_000];
        rng.fill(&mut original[..]);

        let (compressed, orig_size, is_compressed, algo_id) = compress_block(&original, &config).unwrap();
        assert_eq!(orig_size, original.len() as u32);

        let decompressed = decompress_block(&compressed, is_compressed, algo_id).unwrap();
        assert_eq!(original, decompressed);
    }

    #[test]
    fn test_compress_single_byte_value() {
        // Single byte edge case
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        let original = b"x";
        let compressed = compressor.compress(original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original.as_slice(), decompressed.as_slice());
    }

    #[test]
    fn test_compress_binary_data() {
        // Binary data (non-UTF8-safe, no patterns)
        let config = DictionaryCompressionConfig {
            enable_dictionary: true,
            dictionary_size: 10000,
            min_word_length: 3,
        };
        let compressor = DictionaryCompressor::new(config);

        // Binary data with some structure but not text
        let mut original = Vec::new();
        for i in 0..1000 {
            original.push((i % 256) as u8);
            original.push((i / 256 % 256) as u8);
        }

        let compressed = compressor.compress(&original).unwrap();
        let decompressed = compressor.decompress(&compressed).unwrap();

        assert_eq!(original, decompressed);
    }
}
