//! Performance benchmarks for staging pipeline
//!
//! Run with: cargo bench --features staging -p data-modelling-core
//!
//! For the 1M record test:
//!   cargo test --features staging -p data-modelling-core million_records -- --ignored --nocapture

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;
use tempfile::TempDir;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

use data_modelling_core::staging::{
    DedupStrategy, IngestConfig, StagingDb, compute_hashes_parallel, parse_files_parallel,
};

/// Generate test JSON files with specified total record count
fn generate_test_files(
    dir: &TempDir,
    total_records: usize,
    records_per_file: usize,
) -> Vec<std::path::PathBuf> {
    let file_count = (total_records + records_per_file - 1) / records_per_file;
    let mut paths = Vec::with_capacity(file_count);

    for file_idx in 0..file_count {
        let path = dir.path().join(format!("data_{:06}.jsonl", file_idx));
        let file = File::create(&path).expect("Failed to create file");
        let mut writer = BufWriter::new(file);

        let start_record = file_idx * records_per_file;
        let end_record = std::cmp::min(start_record + records_per_file, total_records);

        for record_idx in start_record..end_record {
            writeln!(
                writer,
                r#"{{"id": {}, "user_id": {}, "event_type": "type_{}", "timestamp": "2024-01-{:02}T{:02}:{:02}:{:02}Z", "value": {}, "metadata": {{"source": "benchmark", "version": {}}}}}"#,
                record_idx,
                record_idx % 10000,
                record_idx % 50,
                (record_idx % 28) + 1,
                record_idx % 24,
                record_idx % 60,
                record_idx % 60,
                record_idx as f64 * 0.123,
                record_idx / 1000
            ).expect("Failed to write record");
        }

        paths.push(path);
    }

    paths
}

fn bench_parallel_hashing(c: &mut Criterion) {
    let json_dir = TempDir::new().expect("Failed to create json dir");
    let files = generate_test_files(&json_dir, 10_000, 1_000);

    c.bench_function("parallel_hash_10k_records", |b| {
        b.iter(|| {
            let hashes = compute_hashes_parallel(&files);
            assert_eq!(hashes.len(), 10);
        });
    });
}

fn bench_parallel_parsing(c: &mut Criterion) {
    let json_dir = TempDir::new().expect("Failed to create json dir");
    let files = generate_test_files(&json_dir, 10_000, 1_000);

    c.bench_function("parallel_parse_10k_records", |b| {
        b.iter(|| {
            let parsed = parse_files_parallel(&files);
            assert_eq!(parsed.len(), 10);
        });
    });
}

fn bench_full_ingestion(c: &mut Criterion) {
    let mut group = c.benchmark_group("ingestion");
    group.sample_size(10); // Reduce sample size for slower benchmarks

    for record_count in [1_000, 10_000, 100_000].iter() {
        let json_dir = TempDir::new().expect("Failed to create json dir");
        generate_test_files(&json_dir, *record_count, 1_000);

        let source_path = json_dir.path().to_string_lossy().to_string();

        group.bench_with_input(
            BenchmarkId::new("records", record_count),
            record_count,
            |b, _| {
                b.iter(|| {
                    let db = StagingDb::memory().expect("Failed to open db");
                    db.init().expect("Failed to init db");

                    let config = IngestConfig::builder()
                        .source(&source_path)
                        .expect("Failed to set source")
                        .pattern("*.jsonl")
                        .partition("benchmark")
                        .dedup(DedupStrategy::ByPath)
                        .build()
                        .expect("Failed to build config");

                    let stats = db.ingest(&config).expect("Failed to ingest");
                    stats
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_parallel_hashing,
    bench_parallel_parsing,
    bench_full_ingestion,
);

criterion_main!(benches);
