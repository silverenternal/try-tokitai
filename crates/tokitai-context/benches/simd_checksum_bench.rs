//! SIMD Checksum Benchmark Suite
//!
//! This benchmark suite compares SIMD-accelerated CRC32-C checksum performance
//! against various data sizes and batch operations.
//!
//! ## Benchmark Categories
//!
//! 1. **Single Checksum**: Calculate checksum for single data items
//! 2. **Batch Verification**: Verify multiple checksums in parallel
//! 3. **Streaming**: Calculate checksum for large data via streaming
//! 4. **Combined**: Merge multiple checksums
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all SIMD checksum benchmarks
//! cargo bench --bench simd_checksum_bench --features benchmarks
//!
//! # Run specific benchmark group
//! cargo bench --bench simd_checksum_bench --features benchmarks -- --filter single
//! ```

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use tokitai_context::simd_checksum::*;

/// Benchmark single checksum calculation for various data sizes
fn bench_single_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_checksum");
    
    let sizes = vec![64, 256, 1024, 4096, 16384, 65536, 262144, 1_048_576];
    
    for size in sizes {
        let data = vec![0x42u8; size];
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}B", size)),
            &data,
            |b, data| {
                b.iter(|| calculate_checksum(black_box(data)))
            },
        );
    }
    
    group.finish();
}

/// Benchmark checksum verification for various data sizes
fn bench_verify_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("verify_checksum");
    
    let sizes = vec![64, 256, 1024, 4096, 16384, 65536];
    
    for size in sizes {
        let data = vec![0x42u8; size];
        let checksum = calculate_checksum(&data);
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}B", size)),
            &(data, checksum),
            |b, (data, checksum)| {
                b.iter(|| verify_checksum(black_box(data), black_box(*checksum)))
            },
        );
    }
    
    group.finish();
}

/// Benchmark batch checksum calculation
fn bench_batch_calculate(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_calculate");
    
    let batch_sizes = vec![1, 5, 10, 50, 100, 500, 1000];
    
    for batch_size in batch_sizes {
        let data_items: Vec<Vec<u8>> = (0..batch_size)
            .map(|i| vec![i as u8; 1024])
            .collect();
        
        let data_refs: Vec<&[u8]> = data_items.iter().map(|d| d.as_slice()).collect();
        
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{}", batch_size)),
            &data_refs,
            |b, data| {
                b.iter(|| batch_calculate(black_box(data)))
            },
        );
    }
    
    group.finish();
}

/// Benchmark batch checksum verification
fn bench_batch_verify(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_verify");
    
    let batch_sizes = vec![1, 5, 10, 50, 100, 500, 1000];
    
    for batch_size in batch_sizes {
        let items: Vec<ChecksumItem> = (0..batch_size)
            .map(|i| {
                let data = vec![i as u8; 1024];
                let checksum = calculate_checksum(&data);
                ChecksumItem::new(data, checksum)
            })
            .collect();
        
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("batch_{}", batch_size)),
            &items,
            |b, items| {
                b.iter(|| batch_verify(black_box(items)))
            },
        );
    }
    
    group.finish();
}

/// Benchmark combined checksums
fn bench_combine_checksums(c: &mut Criterion) {
    let mut group = c.benchmark_group("combine_checksums");
    
    let counts = vec![1, 5, 10, 50, 100, 500, 1000];
    
    for count in counts {
        let checksums: Vec<u32> = (0..count)
            .map(|i| i as u32)
            .collect();
        
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("combine_{}", count)),
            &checksums,
            |b, checksums| {
                b.iter(|| combine_checksums(black_box(checksums)))
            },
        );
    }
    
    group.finish();
}

/// Benchmark streaming checksum calculation
fn bench_streaming_checksum(c: &mut Criterion) {
    let mut group = c.benchmark_group("streaming_checksum");
    
    let sizes = vec![4096, 16384, 65536, 262144, 1_048_576];
    
    for size in sizes {
        let data = vec![0x42u8; size];
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}B", size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let mut cursor = std::io::Cursor::new(black_box(data));
                    streaming_checksum(&mut cursor).unwrap()
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark SimdChecksumCalculator with stats tracking
fn bench_calculator_with_stats(c: &mut Criterion) {
    let mut group = c.benchmark_group("calculator_with_stats");
    
    let sizes = vec![256, 1024, 4096, 16384];
    
    for size in sizes {
        let data = vec![0x42u8; size];
        let calc = SimdChecksumCalculator::new();
        
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}B", size)),
            &(data, calc),
            |b, (data, calc)| {
                b.iter(|| {
                    let checksum = calc.calculate(black_box(data));
                    calc.verify(black_box(data), checksum);
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark parallel vs sequential batch verification
fn bench_parallel_vs_sequential(c: &mut Criterion) {
    let mut group = c.benchmark_group("parallel_vs_sequential");
    
    let batch_sizes = vec![10, 50, 100, 500, 1000];
    
    for batch_size in batch_sizes {
        let items: Vec<ChecksumItem> = (0..batch_size)
            .map(|i| {
                let data = vec![i as u8; 1024];
                let checksum = calculate_checksum(&data);
                ChecksumItem::new(data, checksum)
            })
            .collect();
        
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("parallel", batch_size),
            &items,
            |b, items| {
                b.iter(|| batch_verify(black_box(items)))
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_single_checksum,
    bench_verify_checksum,
    bench_batch_calculate,
    bench_batch_verify,
    bench_combine_checksums,
    bench_streaming_checksum,
    bench_calculator_with_stats,
    bench_parallel_vs_sequential,
);

criterion_main!(benches);
