//! File ingestion logic
//!
//! This module provides parallel file discovery, parsing, and ingestion
//! using rayon for CPU-bound operations.

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::config::DedupStrategy;
use super::error::IngestError;

/// Statistics from an ingestion run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestStats {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of files skipped (duplicates)
    pub files_skipped: usize,
    /// Number of records ingested
    pub records_ingested: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Number of errors encountered
    pub errors_count: usize,
    /// List of errors (limited to first 100)
    pub errors: Vec<String>,
    /// Duration of the ingestion
    #[serde(skip)]
    pub duration: Duration,
}

impl IngestStats {
    /// Create new empty stats
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an error (limited to 100)
    pub fn add_error(&mut self, error: String) {
        self.errors_count += 1;
        if self.errors.len() < 100 {
            self.errors.push(error);
        }
    }

    /// Get records per second throughput
    pub fn throughput(&self) -> f64 {
        let secs = self.duration.as_secs_f64();
        if secs == 0.0 {
            0.0
        } else {
            self.records_ingested as f64 / secs
        }
    }

    /// Format duration as human-readable string
    pub fn duration_string(&self) -> String {
        let secs = self.duration.as_secs();
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60)
        }
    }
}

/// A discovered file to ingest
#[derive(Debug, Clone)]
pub struct DiscoveredFile {
    /// Path to the file
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Content hash (if computed)
    pub content_hash: Option<String>,
}

impl DiscoveredFile {
    /// Create a new discovered file
    pub fn new(path: PathBuf, size: u64) -> Self {
        Self {
            path,
            size,
            content_hash: None,
        }
    }

    /// Compute and cache the content hash
    pub fn compute_hash(&mut self) -> Result<&str, IngestError> {
        if self.content_hash.is_none() {
            let content = fs::read(&self.path)?;
            let hash = Sha256::digest(&content);
            self.content_hash = Some(format!("{:x}", hash));
        }
        Ok(self.content_hash.as_ref().unwrap())
    }
}

/// Discover files matching a pattern in a local directory
pub fn discover_local_files(
    base_path: &Path,
    pattern: &str,
) -> Result<Vec<DiscoveredFile>, IngestError> {
    let mut files = Vec::new();

    // Build the glob pattern
    let full_pattern = if pattern.starts_with('/') || pattern.starts_with('.') {
        pattern.to_string()
    } else {
        format!("{}/{}", base_path.display(), pattern)
    };

    // Use glob to find matching files
    let entries = glob::glob(&full_pattern)
        .map_err(|e| IngestError::InvalidPattern(format!("{}: {}", pattern, e)))?;

    for entry in entries {
        match entry {
            Ok(path) => {
                if path.is_file() {
                    let metadata = fs::metadata(&path)?;
                    files.push(DiscoveredFile::new(path, metadata.len()));
                }
            }
            Err(e) => {
                // Log but continue
                tracing::warn!("Error accessing path: {}", e);
            }
        }
    }

    // Sort by path for consistent ordering
    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(files)
}

/// Parsed JSON record from a file
#[derive(Debug)]
pub struct ParsedRecord {
    /// The raw JSON string
    pub json: String,
    /// Record index within the file (0-based)
    pub index: usize,
}

/// Parse a JSON file (single object)
pub fn parse_json_file(path: &Path) -> Result<Vec<ParsedRecord>, IngestError> {
    let content = fs::read_to_string(path)?;

    // Validate it's valid JSON
    serde_json::from_str::<serde_json::Value>(&content).map_err(|e| IngestError::JsonParse {
        path: path.to_path_buf(),
        record: 0,
        error: e.to_string(),
    })?;

    Ok(vec![ParsedRecord {
        json: content.trim().to_string(),
        index: 0,
    }])
}

/// Parse a JSONL file (newline-delimited JSON)
pub fn parse_jsonl_file(path: &Path) -> Result<Vec<ParsedRecord>, IngestError> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut records = Vec::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Validate it's valid JSON
        serde_json::from_str::<serde_json::Value>(trimmed).map_err(|e| IngestError::JsonParse {
            path: path.to_path_buf(),
            record: index,
            error: e.to_string(),
        })?;

        records.push(ParsedRecord {
            json: trimmed.to_string(),
            index,
        });
    }

    Ok(records)
}

/// Parse a file based on its extension
pub fn parse_file(path: &Path) -> Result<Vec<ParsedRecord>, IngestError> {
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    match extension.to_lowercase().as_str() {
        "jsonl" | "ndjson" => parse_jsonl_file(path),
        "json" => parse_json_file(path),
        _ => {
            // Try to detect format from content
            let content = fs::read_to_string(path)?;
            let trimmed = content.trim();

            // If it starts with '[' or '{', treat as single JSON
            if trimmed.starts_with('[') || trimmed.starts_with('{') {
                parse_json_file(path)
            } else {
                // Try JSONL
                parse_jsonl_file(path)
            }
        }
    }
}

/// Check if a file should be skipped based on dedup strategy
pub fn should_skip_file(
    file: &DiscoveredFile,
    dedup: DedupStrategy,
    existing_paths: &std::collections::HashSet<String>,
    existing_hashes: &std::collections::HashSet<String>,
) -> bool {
    match dedup {
        DedupStrategy::None => false,
        DedupStrategy::ByPath => existing_paths.contains(&file.path.display().to_string()),
        DedupStrategy::ByContent => {
            if let Some(hash) = &file.content_hash {
                existing_hashes.contains(hash)
            } else {
                false
            }
        }
        DedupStrategy::Both => {
            let path_exists = existing_paths.contains(&file.path.display().to_string());
            let hash_exists = file
                .content_hash
                .as_ref()
                .map(|h| existing_hashes.contains(h))
                .unwrap_or(false);
            path_exists || hash_exists
        }
    }
}

/// Result of parsing a single file in parallel
#[derive(Debug)]
pub struct ParsedFile {
    /// The discovered file
    pub file: DiscoveredFile,
    /// Parsed records (if successful)
    pub records: Result<Vec<ParsedRecord>, IngestError>,
}

/// Parse multiple files in parallel using rayon
///
/// This function uses rayon's parallel iterator to parse files concurrently,
/// which significantly speeds up ingestion for large numbers of files.
///
/// # Arguments
/// * `files` - List of discovered files to parse
///
/// # Returns
/// A vector of ParsedFile results, one for each input file
pub fn parse_files_parallel(files: Vec<DiscoveredFile>) -> Vec<ParsedFile> {
    files
        .into_par_iter()
        .map(|file| {
            let records = parse_file(&file.path);
            ParsedFile { file, records }
        })
        .collect()
}

/// Compute content hashes for files in parallel
///
/// This function uses rayon to hash file contents concurrently.
///
/// # Arguments
/// * `files` - Mutable slice of discovered files to hash
pub fn compute_hashes_parallel(files: &mut [DiscoveredFile]) {
    files.par_iter_mut().for_each(|file| {
        if let Err(e) = file.compute_hash() {
            tracing::warn!("Failed to hash {}: {}", file.path.display(), e);
        }
    });
}

/// Streaming record iterator for memory-efficient processing
///
/// Instead of loading all records into memory, this iterator yields
/// records one at a time from a JSONL file.
pub struct StreamingJsonlReader {
    reader: BufReader<File>,
    path: PathBuf,
    line_number: usize,
}

impl StreamingJsonlReader {
    /// Create a new streaming reader for a JSONL file
    pub fn new(path: &Path) -> Result<Self, IngestError> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
            path: path.to_path_buf(),
            line_number: 0,
        })
    }
}

impl Iterator for StreamingJsonlReader {
    type Item = Result<ParsedRecord, IngestError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();

        loop {
            line.clear();
            match self.reader.read_line(&mut line) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let index = self.line_number;
                    self.line_number += 1;

                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue; // Skip empty lines
                    }

                    // Validate JSON
                    if let Err(e) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        return Some(Err(IngestError::JsonParse {
                            path: self.path.clone(),
                            record: index,
                            error: e.to_string(),
                        }));
                    }

                    return Some(Ok(ParsedRecord {
                        json: trimmed.to_string(),
                        index,
                    }));
                }
                Err(e) => {
                    return Some(Err(IngestError::Io(e)));
                }
            }
        }
    }
}

/// Parallel batch processor for processing parsed records
///
/// This struct provides a way to process records in parallel batches,
/// useful for CPU-intensive operations like schema inference.
pub struct ParallelBatchProcessor<T> {
    batch_size: usize,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Send> ParallelBatchProcessor<T> {
    /// Create a new parallel batch processor
    pub fn new(batch_size: usize) -> Self {
        Self {
            batch_size,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Process items in parallel batches
    ///
    /// # Arguments
    /// * `items` - Iterator of items to process
    /// * `processor` - Function to apply to each item
    ///
    /// # Returns
    /// Vector of results from processing
    pub fn process<I, F, R>(&self, items: I, processor: F) -> Vec<R>
    where
        I: Iterator<Item = T>,
        F: Fn(T) -> R + Sync + Send,
        R: Send,
    {
        let items: Vec<T> = items.collect();

        // Process in parallel batches
        items
            .into_par_iter()
            .with_min_len(self.batch_size.max(1))
            .map(processor)
            .collect()
    }
}

/// Convert a discovered file to RawJsonRecords for Iceberg storage
#[cfg(feature = "iceberg")]
pub fn to_raw_json_records(
    file: &DiscoveredFile,
    partition: Option<&str>,
) -> Result<Vec<super::iceberg_table::RawJsonRecord>, IngestError> {
    use super::iceberg_table::RawJsonRecord;
    use chrono::Utc;

    let records = parse_file(&file.path)?;
    let now = Utc::now();

    Ok(records
        .into_iter()
        .map(|r| RawJsonRecord {
            path: file.path.display().to_string(),
            content: r.json,
            size: file.size as usize,
            content_hash: file.content_hash.clone(),
            partition: partition.map(|s| s.to_string()),
            ingested_at: now,
        })
        .collect())
}

/// Configuration for Iceberg ingestion
#[cfg(feature = "iceberg")]
#[derive(Debug, Clone)]
pub struct IcebergIngestConfig {
    /// Base path for file discovery
    pub base_path: std::path::PathBuf,
    /// File pattern to match
    pub pattern: String,
    /// Partition key
    pub partition: Option<String>,
    /// Deduplication strategy
    pub dedup: DedupStrategy,
    /// Batch size for writes
    pub batch_size: usize,
    /// Resume a previous batch
    pub resume: bool,
    /// Batch ID for resume (auto-generated if not provided)
    pub batch_id: Option<String>,
}

#[cfg(feature = "iceberg")]
impl IcebergIngestConfig {
    /// Create a new config with defaults
    pub fn new(base_path: impl Into<std::path::PathBuf>, pattern: impl Into<String>) -> Self {
        Self {
            base_path: base_path.into(),
            pattern: pattern.into(),
            partition: None,
            dedup: DedupStrategy::ByPath,
            batch_size: 1000,
            resume: false,
            batch_id: None,
        }
    }

    /// Set partition key
    pub fn with_partition(mut self, partition: impl Into<String>) -> Self {
        self.partition = Some(partition.into());
        self
    }

    /// Set deduplication strategy
    pub fn with_dedup(mut self, dedup: DedupStrategy) -> Self {
        self.dedup = dedup;
        self
    }

    /// Set batch size
    pub fn with_batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Enable resume mode
    pub fn with_resume(mut self, batch_id: impl Into<String>) -> Self {
        self.resume = true;
        self.batch_id = Some(batch_id.into());
        self
    }
}

/// Ingest files to an Iceberg table
///
/// This function:
/// 1. Discovers files matching the pattern
/// 2. Optionally computes content hashes for deduplication
/// 3. Converts files to RawJsonRecords
/// 4. Writes records to the Iceberg table in batches
///
/// Returns ingestion statistics including records written and any errors.
#[cfg(feature = "iceberg")]
pub async fn ingest_to_iceberg(
    base_path: &Path,
    pattern: &str,
    table: &super::iceberg_table::IcebergTable,
    catalog: &super::catalog::IcebergCatalog,
    partition: Option<&str>,
    dedup: DedupStrategy,
    batch_size: usize,
) -> Result<IngestStats, IngestError> {
    let config = IcebergIngestConfig {
        base_path: base_path.to_path_buf(),
        pattern: pattern.to_string(),
        partition: partition.map(|s| s.to_string()),
        dedup,
        batch_size,
        resume: false,
        batch_id: None,
    };
    ingest_to_iceberg_with_config(table, catalog, &config).await
}

/// Ingest files to an Iceberg table with full configuration including resume support
///
/// This function supports resuming interrupted ingestions by tracking progress
/// in the Iceberg table properties. When `config.resume` is true and a `batch_id`
/// is provided, the function will resume from the last processed file.
#[cfg(feature = "iceberg")]
pub async fn ingest_to_iceberg_with_config(
    table: &super::iceberg_table::IcebergTable,
    catalog: &super::catalog::IcebergCatalog,
    config: &IcebergIngestConfig,
) -> Result<IngestStats, IngestError> {
    use super::iceberg_table::{BatchMetadata, BatchStatus};
    use std::collections::HashSet;
    use std::time::Instant;

    let start = Instant::now();
    let mut stats = IngestStats::new();

    // Generate or use provided batch ID
    let batch_id = config
        .batch_id
        .clone()
        .unwrap_or_else(BatchMetadata::generate_id);

    // Create or resume batch
    let mut batch = if config.resume {
        // Try to load existing batch from table properties
        match table.get_batch_metadata(&batch_id) {
            Some(b) if b.can_resume() => {
                tracing::info!(
                    "Resuming batch {} from file {:?}",
                    batch_id,
                    b.last_file_path
                );
                b
            }
            Some(_) => {
                return Err(IngestError::BatchCompleted(batch_id));
            }
            None => {
                return Err(IngestError::BatchNotFound(batch_id));
            }
        }
    } else {
        let source = format!("{}:{}", config.base_path.display(), config.pattern);
        BatchMetadata::new(batch_id.clone(), source, config.partition.clone())
    };

    // Store initial batch metadata
    if !config.resume {
        if let Err(e) = table.store_batch_metadata(&batch).await {
            tracing::warn!("Failed to store initial batch metadata: {}", e);
        }
    }

    // Discover files
    let mut files = discover_local_files(&config.base_path, &config.pattern)?;

    // Get existing paths/hashes if deduplication is enabled
    let existing_paths: HashSet<String> = HashSet::new(); // Could query from table if needed
    let existing_hashes: HashSet<String> = HashSet::new(); // Could query from table if needed

    // Compute hashes if needed for deduplication (in parallel)
    if matches!(config.dedup, DedupStrategy::ByContent | DedupStrategy::Both) {
        tracing::info!(
            file_count = files.len(),
            "Computing file hashes in parallel"
        );
        compute_hashes_parallel(&mut files);
    }

    // Determine resume point
    let resume_after = if config.resume {
        batch.last_file_path.clone()
    } else {
        None
    };
    let mut past_resume_point = resume_after.is_none();

    // Process files in batches
    let mut batch_records = Vec::new();
    let partition = config.partition.as_deref();

    for mut file in files {
        let file_path_str = file.path.display().to_string();

        // Skip files before resume point
        if !past_resume_point {
            if Some(&file_path_str) == resume_after.as_ref() {
                past_resume_point = true;
            }
            continue;
        }

        // Check deduplication
        if should_skip_file(&file, config.dedup, &existing_paths, &existing_hashes) {
            stats.files_skipped += 1;
            batch.files_skipped += 1;
            continue;
        }

        // Compute hash if not already done (for storage)
        if file.content_hash.is_none() {
            let _ = file.compute_hash();
        }

        // Convert to RawJsonRecords
        match to_raw_json_records(&file, partition) {
            Ok(records) => {
                stats.bytes_processed += file.size;
                batch.bytes_processed += file.size;
                batch_records.extend(records);
            }
            Err(e) => {
                stats.add_error(format!("Failed to parse {}: {}", file.path.display(), e));
                continue;
            }
        }

        stats.files_processed += 1;
        batch.files_processed += 1;
        batch.last_file_path = Some(file_path_str);

        // Write batch if size threshold reached
        if batch_records.len() >= config.batch_size {
            match table.append_records(&batch_records, catalog).await {
                Ok(result) => {
                    stats.records_ingested += result.records_written;
                    batch.record_count += result.records_written;
                    tracing::info!(
                        "Wrote batch of {} records ({} bytes)",
                        result.records_written,
                        result.bytes_written
                    );
                }
                Err(e) => {
                    let error = format!("Failed to write batch: {}", e);
                    stats.add_error(error.clone());
                    batch.fail(error);
                    // Store failed batch state for resume
                    let _ = table.store_batch_metadata(&batch).await;
                    return Err(IngestError::Insert(e.to_string()));
                }
            }
            batch_records.clear();

            // Update batch progress periodically
            if batch.files_processed % 100 == 0 {
                let _ = table.store_batch_metadata(&batch).await;
            }
        }
    }

    // Write remaining records
    if !batch_records.is_empty() {
        match table.append_records(&batch_records, catalog).await {
            Ok(result) => {
                stats.records_ingested += result.records_written;
                batch.record_count += result.records_written;
                tracing::info!(
                    "Wrote final batch of {} records ({} bytes)",
                    result.records_written,
                    result.bytes_written
                );
            }
            Err(e) => {
                let error = format!("Failed to write final batch: {}", e);
                stats.add_error(error.clone());
                batch.fail(error);
                let _ = table.store_batch_metadata(&batch).await;
                return Err(IngestError::Insert(e.to_string()));
            }
        }
    }

    // Mark batch as completed
    batch.complete();
    if let Err(e) = table.store_batch_metadata(&batch).await {
        tracing::warn!("Failed to store final batch metadata: {}", e);
    }

    stats.duration = start.elapsed();

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_ingest_stats() {
        let mut stats = IngestStats::new();
        stats.files_processed = 10;
        stats.records_ingested = 1000;
        stats.duration = Duration::from_secs(10);

        assert_eq!(stats.throughput(), 100.0);
        assert_eq!(stats.duration_string(), "10s");
    }

    #[test]
    fn test_ingest_stats_duration_formatting() {
        let mut stats = IngestStats::new();

        stats.duration = Duration::from_secs(90);
        assert_eq!(stats.duration_string(), "1m 30s");

        stats.duration = Duration::from_secs(3661);
        assert_eq!(stats.duration_string(), "1h 1m 1s");
    }

    #[test]
    fn test_parse_json_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.json");

        let mut file = File::create(&file_path).unwrap();
        writeln!(file, r#"{{"name": "test", "value": 42}}"#).unwrap();

        let records = parse_json_file(&file_path).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].index, 0);
    }

    #[test]
    fn test_parse_jsonl_file() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.jsonl");

        let mut file = File::create(&file_path).unwrap();
        writeln!(file, r#"{{"name": "row1"}}"#).unwrap();
        writeln!(file, r#"{{"name": "row2"}}"#).unwrap();
        writeln!(file, r#"{{"name": "row3"}}"#).unwrap();

        let records = parse_jsonl_file(&file_path).unwrap();
        assert_eq!(records.len(), 3);
        assert_eq!(records[0].index, 0);
        assert_eq!(records[1].index, 1);
        assert_eq!(records[2].index, 2);
    }

    #[test]
    fn test_discovered_file_hash() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("test.json");

        let mut file = File::create(&file_path).unwrap();
        writeln!(file, r#"{{"test": true}}"#).unwrap();

        let mut discovered = DiscoveredFile::new(file_path.clone(), 100);
        assert!(discovered.content_hash.is_none());

        let hash = discovered.compute_hash().unwrap();
        assert!(!hash.is_empty());
        assert!(discovered.content_hash.is_some());
    }
}
