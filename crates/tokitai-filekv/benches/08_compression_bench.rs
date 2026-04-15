//! Compression algorithm benchmark suite
//!
//! Compares Zstd, Snappy, and LZ4 across:
//! - Compression latency
//! - Decompression latency
//! - Compression ratio
//!
//! Run with: `cargo bench --bench 08_compression --features benchmarks`

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use tokitai_filekv::compression::{
    CompressionStrategy, Lz4Compressor, SnappyCompressor, ZstdCompressor,
};

/// Generate test data with the given size
fn generate_data(size: usize, pattern: &str) -> Vec<u8> {
    let pattern_bytes = pattern.as_bytes();
    let mut data = Vec::with_capacity(size);
    while data.len() < size {
        let remaining = size - data.len();
        let chunk_len = pattern_bytes.len().min(remaining);
        data.extend_from_slice(&pattern_bytes[..chunk_len]);
    }
    data
}

/// Benchmark compression speed for a given compressor and data size
fn bench_compression_speed(c: &mut Criterion, compressor: &dyn CompressionStrategy, data: &[u8], name: &str) {
    let mut group = c.benchmark_group(format!("compress_{}", name));
    group.throughput(Throughput::Bytes(data.len() as u64));
    group.bench_function(format!("{}/{}_bytes", compressor.name(), data.len()), |b| {
        b.iter(|| compressor.compress(data).unwrap())
    });
    group.finish();
}

/// Benchmark decompression speed for a given compressor
fn bench_decompression_speed(c: &mut Criterion, compressor: &dyn CompressionStrategy, data: &[u8], name: &str) {
    let compressed = compressor.compress(data).unwrap();
    let mut group = c.benchmark_group(format!("decompress_{}", name));
    group.throughput(Throughput::Bytes(compressed.len() as u64));
    group.bench_function(format!("{}/{}_bytes_compressed", compressor.name(), compressed.len()), |b| {
        b.iter(|| compressor.decompress(&compressed).unwrap())
    });
    group.finish();
}

/// Run compression benchmarks for all algorithms at a given size
fn compression_benchmarks(c: &mut Criterion, size: usize, label: &str, pattern: &str) {
    let data = generate_data(size, pattern);

    let zstd = ZstdCompressor::new(3);
    let snappy = SnappyCompressor;
    let lz4 = Lz4Compressor::new(0);

    // Compression speed
    bench_compression_speed(c, &zstd, &data, label);
    bench_compression_speed(c, &snappy, &data, label);
    bench_compression_speed(c, &lz4, &data, label);

    // Decompression speed
    bench_decompression_speed(c, &zstd, &data, label);
    bench_decompression_speed(c, &snappy, &data, label);
    bench_decompression_speed(c, &lz4, &data, label);

    // Compression ratio comparison
    let zstd_size = zstd.compress(&data).unwrap().len();
    let snappy_size = snappy.compress(&data).unwrap().len();
    let lz4_size = lz4.compress(&data).unwrap().len();

    let mut ratio_group = c.benchmark_group(format!("ratio_{}", label));
    ratio_group.bench_function("compression_ratio", |b| {
        b.iter(|| {
            format!(
                "original={} zstd={} snappy={} lz4={} zstd_ratio={:.2}% snappy_ratio={:.2}% lz4_ratio={:.2}%",
                data.len(),
                zstd_size,
                snappy_size,
                lz4_size,
                zstd_size as f64 / data.len() as f64 * 100.0,
                snappy_size as f64 / data.len() as f64 * 100.0,
                lz4_size as f64 / data.len() as f64 * 100.0,
            )
        })
    });
    ratio_group.finish();
}

fn bench_compress_100_bytes(c: &mut Criterion) {
    compression_benchmarks(c, 100, "100B", "test_data_");
}

fn bench_compress_1kb(c: &mut Criterion) {
    compression_benchmarks(c, 1024, "1KB", "test_data_");
}

fn bench_compress_10kb(c: &mut Criterion) {
    compression_benchmarks(c, 10 * 1024, "10KB", "test_data_");
}

fn bench_compress_100kb(c: &mut Criterion) {
    compression_benchmarks(c, 100 * 1024, "100KB", "test_data_");
}

// Repetitive data benchmarks (better compression)
fn bench_compress_repetitive_10kb(c: &mut Criterion) {
    compression_benchmarks(
        c,
        10 * 1024,
        "10KB_repetitive",
        "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. ",
    );
}

fn bench_compress_repetitive_100kb(c: &mut Criterion) {
    compression_benchmarks(
        c,
        100 * 1024,
        "100KB_repetitive",
        "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. ",
    );
}

// JSON-like data benchmarks
fn bench_compress_json_10kb(c: &mut Criterion) {
    compression_benchmarks(
        c,
        10 * 1024,
        "10KB_json",
        r#"{"id": 12345, "name": "user_example", "email": "user@example.com", "data": "some_value"}"#,
    );
}

fn bench_compress_json_100kb(c: &mut Criterion) {
    compression_benchmarks(
        c,
        100 * 1024,
        "100KB_json",
        r#"{"id": 12345, "name": "user_example", "email": "user@example.com", "data": "some_value"}"#,
    );
}

criterion_group!(
    compression_benches,
    bench_compress_100_bytes,
    bench_compress_1kb,
    bench_compress_10kb,
    bench_compress_100kb,
    bench_compress_repetitive_10kb,
    bench_compress_repetitive_100kb,
    bench_compress_json_10kb,
    bench_compress_json_100kb,
);
criterion_main!(compression_benches);
