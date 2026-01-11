//! File ingestion logic

use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;

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
