//! Benchmarks for schema inference operations
//!
//! Run with: cargo bench --features inference -p data-modelling-core

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use data_modelling_core::inference::{InferredType, SchemaInferrer, detect_format};

/// Generate sample JSON records for benchmarking
fn generate_sample_records(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            format!(
                r#"{{"id": "user-{}", "email": "user{}@example.com", "name": "User {}", "age": {}, "balance": {}, "is_active": {}, "created_at": "2024-01-15T10:30:00Z", "phone": "+1-555-{:04}-{:04}", "ip_address": "192.168.{}.{}", "website": "https://user{}.example.com"}}"#,
                i,
                i,
                i,
                20 + (i % 60),
                1000.0 + (i as f64 * 10.5),
                i % 2 == 0,
                i % 10000,
                (i * 7) % 10000,
                i % 256,
                (i * 3) % 256,
                i
            )
        })
        .collect()
}

/// Benchmark format detection for various string patterns
fn bench_format_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("format_detection");

    // Test different format patterns
    let test_cases = vec![
        ("email", "user@example.com"),
        ("uuid", "550e8400-e29b-41d4-a716-446655440000"),
        ("url", "https://example.com/path"),
        ("phone", "+1-555-123-4567"),
        ("ipv4", "192.168.1.1"),
        ("date", "2024-01-15"),
        ("datetime", "2024-01-15T10:30:00Z"),
        ("plain_string", "hello world"),
    ];

    for (name, value) in test_cases {
        group.bench_with_input(BenchmarkId::new("detect", name), &value, |b, value| {
            b.iter(|| black_box(detect_format(value)));
        });
    }

    group.finish();
}

/// Benchmark schema inference with varying record counts
fn bench_schema_inference(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_inference");

    for count in [10, 100, 500].iter() {
        let records = generate_sample_records(*count);
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(
            BenchmarkId::new("infer_schema", count),
            &records,
            |b, records| {
                b.iter(|| {
                    let mut inferrer = SchemaInferrer::new();
                    for record in records {
                        let _ = inferrer.add_json(record);
                    }
                    black_box(inferrer.finalize())
                });
            },
        );
    }

    group.finish();
}

/// Benchmark batch JSON addition
fn bench_batch_inference(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_inference");

    for count in [100, 500, 1000].iter() {
        let records = generate_sample_records(*count);
        group.throughput(Throughput::Elements(*count as u64));

        group.bench_with_input(
            BenchmarkId::new("add_batch", count),
            &records,
            |b, records| {
                b.iter(|| {
                    let mut inferrer = SchemaInferrer::new();
                    black_box(inferrer.add_json_batch(records))
                });
            },
        );
    }

    group.finish();
}

/// Benchmark type merging
fn bench_type_merging(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_merging");

    // Test compatible type merges
    group.bench_function("merge_strings", |b| {
        b.iter(|| {
            let t1 = InferredType::String { format: None };
            let t2 = InferredType::String { format: None };
            black_box(t1.merge_with(t2))
        });
    });

    group.bench_function("merge_numbers", |b| {
        b.iter(|| {
            let t1 = InferredType::Integer;
            let t2 = InferredType::Number;
            black_box(t1.merge_with(t2))
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_format_detection,
    bench_schema_inference,
    bench_batch_inference,
    bench_type_merging
);
criterion_main!(benches);
