//! Integration tests for the full staging pipeline
//!
//! Tests the complete workflow: file discovery → ingestion → query

#![cfg(feature = "staging")]

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

use data_modelling_core::staging::{
    DedupStrategy, DiscoveredFile, InferenceProgress, IngestConfig, IngestProgress, IngestStats,
    ParsedFile, Spinner, StagingDb, compute_hashes_parallel, format_bytes, format_number,
    parse_files_parallel,
};

/// Helper to create test JSONL files (newline-delimited JSON)
fn create_test_json_files(dir: &TempDir, count: usize) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(count);
    for i in 0..count {
        // Use .jsonl extension since we're writing multiple JSON objects per line
        let path = dir.path().join(format!("test_{:04}.jsonl", i));
        let mut file = File::create(&path).expect("Failed to create test file");

        // Write JSONL format - one JSON object per line
        for j in 0..10 {
            writeln!(
                file,
                r#"{{"id": {}, "name": "test_{}", "value": {}, "active": {}}}"#,
                i * 10 + j,
                i,
                (i * 10 + j) as f64 * 1.5,
                j % 2 == 0
            )
            .expect("Failed to write JSON");
        }
        paths.push(path);
    }
    paths
}

/// Helper to create nested JSONL files
fn create_nested_json_files(dir: &TempDir, count: usize) -> Vec<PathBuf> {
    let mut paths = Vec::with_capacity(count);
    for i in 0..count {
        // Use .jsonl extension since we're writing multiple JSON objects per line
        let path = dir.path().join(format!("nested_{:04}.jsonl", i));
        let mut file = File::create(&path).expect("Failed to create test file");

        for j in 0..5 {
            writeln!(
                file,
                r#"{{"id": {}, "user": {{"name": "user_{}", "email": "user{}@test.com"}}, "tags": ["tag1", "tag2"], "metadata": {{"created_at": "2024-01-{:02}T00:00:00Z", "version": {}}}}}"#,
                i * 5 + j,
                j,
                j,
                (j % 28) + 1,
                i
            ).expect("Failed to write JSON");
        }
        paths.push(path);
    }
    paths
}

#[test]
fn test_full_pipeline_local_files() {
    // Setup: Create temporary directory with test files
    let _temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Create test JSON files
    let json_dir = TempDir::new().expect("Failed to create json dir");
    let _files = create_test_json_files(&json_dir, 10);

    // Step 1: Initialize staging database (in-memory for testing)
    let db = StagingDb::memory().expect("Failed to open staging db");
    db.init().expect("Failed to initialize db");

    // Verify initial state
    assert_eq!(db.record_count(None).expect("Failed to get count"), 0);

    // Step 2: Configure and run ingestion
    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("test-partition")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    let stats = db.ingest(&config).expect("Failed to ingest files");

    // Step 3: Verify ingestion results
    assert_eq!(stats.files_processed, 10, "Should process 10 files");
    assert_eq!(
        stats.records_ingested, 100,
        "Should ingest 100 records (10 per file)"
    );
    assert_eq!(stats.errors_count, 0, "Should have no errors");

    // Step 4: Verify data in database
    let record_count = db.record_count(None).expect("Failed to get count");
    assert_eq!(record_count, 100, "Database should contain 100 records");

    // Step 5: Query sample data
    let samples = db.get_sample(5, None).expect("Failed to get sample");
    assert_eq!(samples.len(), 5, "Should get 5 sample records");

    // Verify samples are valid JSON
    for sample in &samples {
        let parsed: serde_json::Value =
            serde_json::from_str(sample).expect("Sample should be valid JSON");
        assert!(parsed.get("id").is_some(), "Sample should have 'id' field");
        assert!(
            parsed.get("name").is_some(),
            "Sample should have 'name' field"
        );
    }

    // Step 6: Test deduplication (re-ingest same files)
    let stats2 = db.ingest(&config).expect("Failed to re-ingest files");
    assert_eq!(
        stats2.files_skipped, 10,
        "Should skip all 10 files on re-ingest"
    );
    assert_eq!(
        stats2.records_ingested, 0,
        "Should not ingest any new records"
    );

    // Record count should remain the same
    let record_count_after = db.record_count(None).expect("Failed to get count");
    assert_eq!(record_count_after, 100, "Record count should remain 100");
}

#[test]
fn test_pipeline_with_nested_json() {
    let json_dir = TempDir::new().expect("Failed to create json dir");
    let _files = create_nested_json_files(&json_dir, 5);

    // Initialize and ingest
    let db = StagingDb::memory().expect("Failed to open staging db");
    db.init().expect("Failed to initialize db");

    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("nested-partition")
        .dedup(DedupStrategy::ByContent) // Use content hash-based dedup
        .build()
        .expect("Failed to build config");

    let stats = db.ingest(&config).expect("Failed to ingest files");

    assert_eq!(stats.files_processed, 5);
    assert_eq!(stats.records_ingested, 25); // 5 records per file

    // Get samples and verify nested structure
    let samples = db.get_sample(5, None).expect("Failed to get sample");
    for sample in &samples {
        let parsed: serde_json::Value =
            serde_json::from_str(sample).expect("Sample should be valid JSON");

        // Check nested objects
        assert!(parsed.get("id").is_some(), "Should have 'id' field");
        assert!(parsed.get("user").is_some(), "Should have 'user' field");
        assert!(
            parsed.get("metadata").is_some(),
            "Should have 'metadata' field"
        );

        // Check nested user object
        let user = parsed.get("user").unwrap();
        assert!(user.get("name").is_some(), "User should have 'name'");
        assert!(user.get("email").is_some(), "User should have 'email'");
    }
}

#[test]
fn test_parallel_file_processing() {
    let json_dir = TempDir::new().expect("Failed to create json dir");

    // Create 20 files
    let file_paths = create_test_json_files(&json_dir, 20);

    // Convert PathBufs to DiscoveredFiles
    let mut discovered: Vec<DiscoveredFile> = file_paths
        .iter()
        .map(|p| {
            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            DiscoveredFile::new(p.clone(), size)
        })
        .collect();

    // Test parallel hash computation (modifies in place)
    compute_hashes_parallel(&mut discovered);

    // All hashes should be valid (64 chars for SHA-256)
    for file in &discovered {
        assert!(file.content_hash.is_some(), "Hash should be computed");
        assert_eq!(
            file.content_hash.as_ref().unwrap().len(),
            64,
            "SHA-256 hash should be 64 chars"
        );
    }

    // Re-create discovered files for parsing
    let discovered_for_parse: Vec<DiscoveredFile> = file_paths
        .iter()
        .map(|p| {
            let size = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
            DiscoveredFile::new(p.clone(), size)
        })
        .collect();

    // Test parallel file parsing
    let parsed: Vec<ParsedFile> = parse_files_parallel(discovered_for_parse);
    assert_eq!(parsed.len(), 20, "Should parse 20 files");

    // Each file should have 10 records
    for parsed_file in &parsed {
        assert!(parsed_file.records.is_ok(), "Parsing should succeed");
        assert_eq!(
            parsed_file.records.as_ref().unwrap().len(),
            10,
            "Each file should have 10 records"
        );
    }
}

#[test]
fn test_progress_reporting() {
    // Test format_number
    assert_eq!(format_number(0), "0");
    assert_eq!(format_number(999), "999");
    assert_eq!(format_number(1000), "1,000");
    assert_eq!(format_number(1234567), "1,234,567");

    // Test format_bytes
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(100), "100 B");
    assert_eq!(format_bytes(1024), "1.00 KB");
    assert_eq!(format_bytes(1536), "1.50 KB");
    assert_eq!(format_bytes(1048576), "1.00 MB");
    assert_eq!(format_bytes(1073741824), "1.00 GB");

    // Test IngestProgress creation
    let progress = IngestProgress::new(100, true);

    // Test updating progress
    progress.inc_files();
    progress.update_records(10);
    progress.update_bytes(1024);

    // Finish
    progress.finish_success("Done");

    // Test InferenceProgress
    let inf_progress = InferenceProgress::new(50);
    inf_progress.set_message("Processing field 1");
    inf_progress.inc();
    inf_progress.finish_success("Complete");

    // Test Spinner
    let spinner = Spinner::new("Loading...");
    spinner.set_message("Still loading...");
    spinner.finish_success("Done!");
}

#[test]
fn test_batch_tracking() {
    let json_dir = TempDir::new().expect("Failed to create json dir");
    let _files = create_test_json_files(&json_dir, 5);

    let db = StagingDb::memory().expect("Failed to open staging db");
    db.init().expect("Failed to initialize db");

    let source_path = json_dir.path().to_string_lossy().to_string();

    // First batch with partition "batch-1"
    let config1 = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("test_0000.jsonl") // Only first file
        .partition("batch-1")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    let stats1 = db.ingest(&config1).expect("Failed to ingest batch 1");
    assert_eq!(
        stats1.files_processed, 1,
        "Should process 1 file in batch 1"
    );

    // Second batch with different partition
    let config2 = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("test_0001.jsonl")
        .partition("batch-2")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    let stats2 = db.ingest(&config2).expect("Failed to ingest batch 2");
    assert_eq!(
        stats2.files_processed, 1,
        "Should process 1 file in batch 2"
    );

    // Total records from both batches
    let total_records = db.record_count(None).expect("Failed to get count");
    assert_eq!(
        total_records, 20,
        "Should have 20 records from both batches (10 each)"
    );

    // Verify partition-specific counts
    let batch1_count = db
        .record_count(Some("batch-1"))
        .expect("Failed to get batch-1 count");
    assert_eq!(batch1_count, 10, "Batch 1 should have 10 records");

    let batch2_count = db
        .record_count(Some("batch-2"))
        .expect("Failed to get batch-2 count");
    assert_eq!(batch2_count, 10, "Batch 2 should have 10 records");
}

#[test]
fn test_error_handling() {
    let json_dir = TempDir::new().expect("Failed to create json dir");

    // Create some valid files
    create_test_json_files(&json_dir, 3);

    // Create an invalid JSONL file
    let invalid_path = json_dir.path().join("invalid.jsonl");
    let mut invalid_file = File::create(&invalid_path).expect("Failed to create invalid file");
    writeln!(invalid_file, "{{invalid json").expect("Failed to write invalid JSON");
    drop(invalid_file);

    let db = StagingDb::memory().expect("Failed to open staging db");
    db.init().expect("Failed to initialize db");

    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("error-test")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    // Ingestion should still complete, but report errors
    let stats = db.ingest(&config).expect("Failed to ingest files");

    // Should process valid files
    assert!(
        stats.files_processed >= 3,
        "Should process at least 3 valid files"
    );

    // Should report error for invalid file
    assert!(
        stats.errors_count >= 1,
        "Should have at least 1 error for invalid JSON"
    );

    // Records from valid files should be ingested
    let record_count = db.record_count(None).expect("Failed to get count");
    assert_eq!(
        record_count, 30,
        "Should have 30 records from 3 valid files"
    );
}

#[test]
fn test_empty_directory() {
    let empty_dir = TempDir::new().expect("Failed to create empty dir");

    let db = StagingDb::memory().expect("Failed to open staging db");
    db.init().expect("Failed to initialize db");

    let source_path = empty_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("empty-test")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    let stats = db
        .ingest(&config)
        .expect("Failed to ingest empty directory");

    assert_eq!(stats.files_processed, 0, "Should process 0 files");
    assert_eq!(stats.records_ingested, 0, "Should ingest 0 records");
    assert_eq!(stats.errors_count, 0, "Should have no errors");
}

#[test]
fn test_large_file_handling() {
    let json_dir = TempDir::new().expect("Failed to create json dir");

    // Create a file with 10000 records
    let large_path = json_dir.path().join("large.jsonl");
    let mut large_file = File::create(&large_path).expect("Failed to create large file");

    for i in 0..10000 {
        writeln!(
            large_file,
            r#"{{"id": {}, "data": "record_{}", "value": {}}}"#,
            i,
            i,
            i as f64 * 0.123
        )
        .expect("Failed to write JSON");
    }
    drop(large_file);

    let db = StagingDb::memory().expect("Failed to open staging db");
    db.init().expect("Failed to initialize db");

    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("large-test")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    let stats = db.ingest(&config).expect("Failed to ingest large file");

    assert_eq!(stats.files_processed, 1, "Should process 1 file");
    assert_eq!(stats.records_ingested, 10000, "Should ingest 10000 records");
    assert_eq!(stats.errors_count, 0, "Should have no errors");

    // Verify all records are in database
    let record_count = db.record_count(None).expect("Failed to get count");
    assert_eq!(record_count, 10000, "Database should contain 10000 records");
}

#[test]
fn test_stats_throughput() {
    let mut stats = IngestStats::new();
    stats.records_ingested = 1000;
    stats.duration = std::time::Duration::from_secs(10);

    let throughput = stats.throughput();
    assert!(
        (throughput - 100.0).abs() < 0.001,
        "Throughput should be 100 records/sec"
    );

    // Test duration formatting
    assert_eq!(stats.duration_string(), "10s");

    stats.duration = std::time::Duration::from_secs(90);
    assert_eq!(stats.duration_string(), "1m 30s");

    stats.duration = std::time::Duration::from_secs(3661);
    assert_eq!(stats.duration_string(), "1h 1m 1s");
}

#[test]
fn test_sql_query() {
    let json_dir = TempDir::new().expect("Failed to create json dir");
    create_test_json_files(&json_dir, 3);

    let db = StagingDb::memory().expect("Failed to open staging db");
    db.init().expect("Failed to initialize db");

    let source_path = json_dir.path().to_string_lossy().to_string();
    let config = IngestConfig::builder()
        .source(&source_path)
        .expect("Failed to set source")
        .pattern("*.jsonl")
        .partition("query-test")
        .dedup(DedupStrategy::ByPath)
        .build()
        .expect("Failed to build config");

    db.ingest(&config).expect("Failed to ingest files");

    // Test SQL query
    let results = db
        .query("SELECT COUNT(*) as cnt FROM staged_json")
        .expect("Query failed");
    assert_eq!(results.len(), 1, "Should return 1 row");

    let count = results[0].get("cnt").expect("Should have 'cnt' field");
    assert_eq!(count.as_i64().unwrap(), 30, "Should count 30 records");
}

/// Tests for secret redaction (cross-cutting concern)
#[cfg(feature = "s3")]
mod secret_redaction_tests {
    use data_modelling_core::staging::{redact_secret, redact_secrets_in_string};

    #[test]
    fn test_redact_secret() {
        // Short secret
        assert_eq!(redact_secret("abc", 4), "[REDACTED]");

        // Normal secret
        assert_eq!(
            redact_secret("AKIAIOSFODNN7EXAMPLE", 4),
            "AKIA...[REDACTED]"
        );

        // Empty secret
        assert_eq!(redact_secret("", 4), "[REDACTED]");
    }

    #[test]
    fn test_redact_secrets_in_string() {
        // AWS Access Key
        let input = "Access key: AKIAIOSFODNN7EXAMPLE";
        let redacted = redact_secrets_in_string(input);
        assert!(
            redacted.contains("AKIA...[REDACTED]"),
            "Should redact AWS access key"
        );
        assert!(
            !redacted.contains("AKIAIOSFODNN7EXAMPLE"),
            "Should not contain full key"
        );

        // Bearer token
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let redacted = redact_secrets_in_string(input);
        assert!(
            redacted.contains("[REDACTED]"),
            "Should redact bearer token"
        );

        // URL with password
        let input = "postgres://user:secretpass@localhost/db";
        let redacted = redact_secrets_in_string(input);
        assert!(
            !redacted.contains("secretpass"),
            "Should redact URL password"
        );
    }

    #[test]
    fn test_no_redaction_needed() {
        let input = "This is a normal log message with no secrets";
        let redacted = redact_secrets_in_string(input);
        assert_eq!(input, redacted, "Should not modify normal text");
    }
}
