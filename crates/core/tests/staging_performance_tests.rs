//! Performance tests for staging pipeline
//!
//! These are integration tests that measure performance with large datasets.
//! They are marked #[ignore] to avoid running in normal test suites.
//!
//! Run with:
//!   cargo test --features staging -p data-modelling-core --test staging_performance_tests -- --ignored --nocapture

#![cfg(feature = "staging")]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;
use tempfile::TempDir;

use data_modelling_core::staging::{
    DedupStrategy, DiscoveredFile, IngestConfig, StagingDb, compute_hashes_parallel,
    parse_files_parallel,
};

/// Generate test JSONL files with specified total record count
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

/// Convert PathBufs to DiscoveredFiles
fn paths_to_discovered(paths: &[std::path::PathBuf]) -> Vec<DiscoveredFile> {
    paths
        .iter()
        .map(|p| {
            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            DiscoveredFile::new(p.clone(), size)
        })
        .collect()
}

/// Benchmark parallel hash computation
fn bench_parallel_hashing(paths: &[std::path::PathBuf]) {
    println!("\n=== Parallel Hash Computation ===");
    println!("Files: {}", paths.len());

    let mut discovered = paths_to_discovered(paths);

    let start = Instant::now();
    compute_hashes_parallel(&mut discovered);
    let duration = start.elapsed();

    let success_count = discovered
        .iter()
        .filter(|f| f.content_hash.is_some())
        .count();
    let total_bytes: u64 = paths
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum();

    println!("Duration: {:?}", duration);
    println!("Successful hashes: {}/{}", success_count, paths.len());
    println!("Total bytes: {} MB", total_bytes / 1024 / 1024);
    println!(
        "Throughput: {:.2} MB/s",
        total_bytes as f64 / 1024.0 / 1024.0 / duration.as_secs_f64()
    );
}

/// Benchmark parallel file parsing
fn bench_parallel_parsing(paths: &[std::path::PathBuf]) {
    println!("\n=== Parallel File Parsing ===");
    println!("Files: {}", paths.len());

    let discovered = paths_to_discovered(paths);

    let start = Instant::now();
    let parsed = parse_files_parallel(discovered);
    let duration = start.elapsed();

    let total_records: usize = parsed
        .iter()
        .filter_map(|p| p.records.as_ref().ok())
        .map(|r| r.len())
        .sum();

    println!("Duration: {:?}", duration);
    println!("Total records parsed: {}", total_records);
    println!(
        "Throughput: {:.2} records/s",
        total_records as f64 / duration.as_secs_f64()
    );
}

/// Benchmark full ingestion pipeline
fn bench_full_ingestion(json_dir: &TempDir, expected_records: usize) {
    println!("\n=== Full Ingestion Pipeline ===");

    let start = Instant::now();

    let db = StagingDb::memory().expect("Failed to open db");
    db.init().expect("Failed to init db");

    let init_duration = start.elapsed();
    println!("DB initialization: {:?}", init_duration);

    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("benchmark")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    let ingest_start = Instant::now();
    let stats = db.ingest(&config).expect("Failed to ingest");
    let ingest_duration = ingest_start.elapsed();

    let total_duration = start.elapsed();

    println!("Files processed: {}", stats.files_processed);
    println!("Records ingested: {}", stats.records_ingested);
    println!(
        "Bytes processed: {} MB",
        stats.bytes_processed / 1024 / 1024
    );
    println!("Errors: {}", stats.errors_count);
    println!("Ingestion duration: {:?}", ingest_duration);
    println!("Total duration: {:?}", total_duration);
    println!(
        "Throughput: {:.2} records/s",
        stats.records_ingested as f64 / ingest_duration.as_secs_f64()
    );

    assert_eq!(
        stats.records_ingested, expected_records,
        "Expected {} records, got {}",
        expected_records, stats.records_ingested
    );
}

/// Benchmark query performance
fn bench_queries(json_dir: &TempDir) {
    println!("\n=== Query Performance ===");

    let db = StagingDb::memory().expect("Failed to open db");
    db.init().expect("Failed to init db");

    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("benchmark")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    db.ingest(&config).expect("Failed to ingest");

    // Count query
    let start = Instant::now();
    let results = db
        .query("SELECT COUNT(*) as cnt FROM staged_json")
        .expect("Query failed");
    let count_duration = start.elapsed();
    println!("COUNT(*) query: {:?}", count_duration);
    println!("  Result: {} records", results[0].get("cnt").unwrap());

    // Sample query
    let start = Instant::now();
    let samples = db.get_sample(100, None).expect("Failed to get sample");
    let sample_duration = start.elapsed();
    println!("Sample 100 records: {:?}", sample_duration);
    println!("  Retrieved: {} records", samples.len());

    // Partition count query
    let start = Instant::now();
    let count = db.record_count(Some("benchmark")).expect("Failed to count");
    let partition_duration = start.elapsed();
    println!("Partition count: {:?}", partition_duration);
    println!("  Result: {} records", count);
}

/// Main benchmark function - 1M records
#[test]
#[ignore] // Run with: cargo test --features staging -p data-modelling-core --test staging_performance_tests million_records -- --ignored --nocapture
fn test_million_records_performance() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║          1,000,000 Records Performance Test                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    let json_dir = TempDir::new().expect("Failed to create json dir");

    // Generate 1M records across 100 files (10k records each)
    println!("\n=== Generating Test Data ===");
    let gen_start = Instant::now();
    let files = generate_test_files(&json_dir, 1_000_000, 10_000);
    let gen_duration = gen_start.elapsed();

    let total_bytes: u64 = files
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum();

    println!("Generated {} files with 1M records", files.len());
    println!("Total size: {} MB", total_bytes / 1024 / 1024);
    println!("Generation time: {:?}", gen_duration);

    // Run benchmarks
    bench_parallel_hashing(&files);
    bench_parallel_parsing(&files);
    bench_full_ingestion(&json_dir, 1_000_000);
    bench_queries(&json_dir);

    println!("\n=== Performance Test Complete ===\n");
}

/// Quick benchmark with 100k records for CI
#[test]
#[ignore] // Run with: cargo test --features staging -p data-modelling-core --test staging_performance_tests hundred_k_records -- --ignored --nocapture
fn test_hundred_k_records_performance() {
    println!("\n");
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║            100,000 Records Performance Test                  ║");
    println!("╚══════════════════════════════════════════════════════════════╝");

    let json_dir = TempDir::new().expect("Failed to create json dir");

    // Generate 100k records across 10 files (10k records each)
    println!("\n=== Generating Test Data ===");
    let gen_start = Instant::now();
    let files = generate_test_files(&json_dir, 100_000, 10_000);
    let gen_duration = gen_start.elapsed();

    let total_bytes: u64 = files
        .iter()
        .filter_map(|p| std::fs::metadata(p).ok())
        .map(|m| m.len())
        .sum();

    println!("Generated {} files with 100k records", files.len());
    println!("Total size: {} MB", total_bytes / 1024 / 1024);
    println!("Generation time: {:?}", gen_duration);

    // Run benchmarks
    bench_parallel_hashing(&files);
    bench_parallel_parsing(&files);
    bench_full_ingestion(&json_dir, 100_000);
    bench_queries(&json_dir);

    println!("\n=== Performance Test Complete ===\n");
}

/// Small benchmark for quick validation (not ignored - runs with regular tests)
#[test]
fn test_ten_k_records_quick() {
    let json_dir = TempDir::new().expect("Failed to create json dir");

    // Generate 10k records across 10 files
    let files = generate_test_files(&json_dir, 10_000, 1_000);

    // Quick validation of parallel operations
    let mut discovered = paths_to_discovered(&files);
    compute_hashes_parallel(&mut discovered);
    assert_eq!(discovered.len(), 10);
    assert!(discovered.iter().all(|f| f.content_hash.is_some()));

    let discovered_for_parse = paths_to_discovered(&files);
    let parsed = parse_files_parallel(discovered_for_parse);
    assert_eq!(parsed.len(), 10);
    let total_records: usize = parsed
        .iter()
        .filter_map(|p| p.records.as_ref().ok())
        .map(|r| r.len())
        .sum();
    assert_eq!(total_records, 10_000);

    // Full pipeline
    let db = StagingDb::memory().expect("Failed to open db");
    db.init().expect("Failed to init db");

    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("quick-test")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    let stats = db.ingest(&config).expect("Failed to ingest");
    assert_eq!(stats.records_ingested, 10_000);
    assert_eq!(stats.errors_count, 0);

    // Verify record count
    let count = db.record_count(None).expect("Failed to count");
    assert_eq!(count, 10_000);
}
