//! Configuration types for staging and ingestion

#![allow(unexpected_cfgs)]

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Source type for ingestion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    /// Local filesystem path
    Local(PathBuf),
    /// S3 bucket and prefix
    #[cfg(feature = "s3")]
    S3 { bucket: String, prefix: String },
    /// Unity Catalog Volume path
    #[cfg(feature = "databricks")]
    UnityVolume {
        catalog: String,
        schema: String,
        volume: String,
        path: String,
    },
}

impl SourceType {
    /// Parse a source string into a SourceType
    ///
    /// Supported formats:
    /// - Local: `./path`, `/path`, `path`
    /// - S3: `s3://bucket/prefix`
    /// - Unity Catalog: `/Volumes/catalog/schema/volume/path`
    pub fn parse(source: &str) -> Result<Self, String> {
        if source.starts_with("s3://") {
            #[cfg(feature = "s3")]
            {
                let rest = source.strip_prefix("s3://").unwrap();
                let parts: Vec<&str> = rest.splitn(2, '/').collect();
                if parts.is_empty() || parts[0].is_empty() {
                    return Err("Invalid S3 URL: missing bucket name".to_string());
                }
                Ok(SourceType::S3 {
                    bucket: parts[0].to_string(),
                    prefix: parts.get(1).unwrap_or(&"").to_string(),
                })
            }
            #[cfg(not(feature = "s3"))]
            {
                Err("S3 support not enabled. Build with --features s3".to_string())
            }
        } else if source.starts_with("/Volumes/") {
            #[cfg(feature = "databricks")]
            {
                let rest = source.strip_prefix("/Volumes/").unwrap();
                let parts: Vec<&str> = rest.splitn(4, '/').collect();
                if parts.len() < 3 {
                    return Err(
                        "Invalid Unity Catalog path. Expected: /Volumes/<catalog>/<schema>/<volume>[/<path>]"
                            .to_string(),
                    );
                }
                Ok(SourceType::UnityVolume {
                    catalog: parts[0].to_string(),
                    schema: parts[1].to_string(),
                    volume: parts[2].to_string(),
                    path: parts.get(3).unwrap_or(&"").to_string(),
                })
            }
            #[cfg(not(feature = "databricks"))]
            {
                Err("Databricks support not enabled. Build with --features databricks".to_string())
            }
        } else {
            Ok(SourceType::Local(PathBuf::from(source)))
        }
    }

    /// Get a display string for this source
    pub fn display(&self) -> String {
        match self {
            SourceType::Local(path) => path.display().to_string(),
            #[cfg(feature = "s3")]
            SourceType::S3 { bucket, prefix } => format!("s3://{}/{}", bucket, prefix),
            #[cfg(feature = "databricks")]
            SourceType::UnityVolume {
                catalog,
                schema,
                volume,
                path,
            } => {
                if path.is_empty() {
                    format!("/Volumes/{}/{}/{}", catalog, schema, volume)
                } else {
                    format!("/Volumes/{}/{}/{}/{}", catalog, schema, volume, path)
                }
            }
        }
    }
}

/// Deduplication strategy for ingestion
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DedupStrategy {
    /// No deduplication - ingest all files
    None,
    /// Skip files with the same path (default)
    #[default]
    ByPath,
    /// Skip files with the same content hash
    ByContent,
    /// Skip files with the same path AND content hash
    Both,
}

impl std::str::FromStr for DedupStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(DedupStrategy::None),
            "path" | "bypath" => Ok(DedupStrategy::ByPath),
            "content" | "bycontent" => Ok(DedupStrategy::ByContent),
            "both" => Ok(DedupStrategy::Both),
            _ => Err(format!(
                "Invalid dedup strategy: {}. Expected: none, path, content, both",
                s
            )),
        }
    }
}

/// Configuration for data ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestConfig {
    /// Source type and location
    pub source: SourceType,
    /// File pattern to match (e.g., "*.json", "**/*.jsonl")
    pub pattern: String,
    /// Partition key for this batch (optional)
    pub partition: Option<String>,
    /// Number of parallel workers for local file processing
    pub workers: usize,
    /// Batch size for database inserts
    pub batch_size: usize,
    /// Deduplication strategy
    pub dedup: DedupStrategy,
    /// Resume a previous interrupted batch
    pub resume: bool,
    /// Batch ID for resume (auto-generated if not provided)
    pub batch_id: Option<String>,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            source: SourceType::Local(PathBuf::from(".")),
            pattern: "*.json".to_string(),
            partition: None,
            workers: 4,
            batch_size: 1000,
            dedup: DedupStrategy::ByPath,
            resume: false,
            batch_id: None,
        }
    }
}

impl IngestConfig {
    /// Create a new builder for IngestConfig
    pub fn builder() -> IngestConfigBuilder {
        IngestConfigBuilder::default()
    }
}

/// Builder for IngestConfig
#[derive(Debug, Default)]
pub struct IngestConfigBuilder {
    source: Option<SourceType>,
    pattern: Option<String>,
    partition: Option<String>,
    workers: Option<usize>,
    batch_size: Option<usize>,
    dedup: Option<DedupStrategy>,
    resume: bool,
    batch_id: Option<String>,
}

impl IngestConfigBuilder {
    /// Set the source path (will be parsed into SourceType)
    pub fn source(mut self, source: &str) -> Result<Self, String> {
        self.source = Some(SourceType::parse(source)?);
        Ok(self)
    }

    /// Set the source type directly
    pub fn source_type(mut self, source: SourceType) -> Self {
        self.source = Some(source);
        self
    }

    /// Set the file pattern
    pub fn pattern(mut self, pattern: &str) -> Self {
        self.pattern = Some(pattern.to_string());
        self
    }

    /// Set the partition key
    pub fn partition(mut self, partition: &str) -> Self {
        self.partition = Some(partition.to_string());
        self
    }

    /// Set the number of parallel workers
    pub fn workers(mut self, workers: usize) -> Self {
        self.workers = Some(workers);
        self
    }

    /// Set the batch size for database inserts
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = Some(batch_size);
        self
    }

    /// Set the deduplication strategy
    pub fn dedup(mut self, dedup: DedupStrategy) -> Self {
        self.dedup = Some(dedup);
        self
    }

    /// Enable resume mode
    pub fn resume(mut self, resume: bool) -> Self {
        self.resume = resume;
        self
    }

    /// Set the batch ID for resume
    pub fn batch_id(mut self, batch_id: &str) -> Self {
        self.batch_id = Some(batch_id.to_string());
        self
    }

    /// Build the IngestConfig
    pub fn build(self) -> Result<IngestConfig, String> {
        let source = self.source.ok_or("Source is required")?;

        Ok(IngestConfig {
            source,
            pattern: self.pattern.unwrap_or_else(|| "*.json".to_string()),
            partition: self.partition,
            workers: self.workers.unwrap_or(4),
            batch_size: self.batch_size.unwrap_or(1000),
            dedup: self.dedup.unwrap_or_default(),
            resume: self.resume,
            batch_id: self.batch_id,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_type_parse_local() {
        let source = SourceType::parse("./data").unwrap();
        assert!(matches!(source, SourceType::Local(_)));

        let source = SourceType::parse("/absolute/path").unwrap();
        assert!(matches!(source, SourceType::Local(_)));
    }

    #[test]
    fn test_dedup_strategy_from_str() {
        assert_eq!(
            "none".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::None
        );
        assert_eq!(
            "path".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::ByPath
        );
        assert_eq!(
            "content".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::ByContent
        );
        assert_eq!(
            "both".parse::<DedupStrategy>().unwrap(),
            DedupStrategy::Both
        );
    }

    #[test]
    fn test_ingest_config_builder() {
        let config = IngestConfig::builder()
            .source_type(SourceType::Local(PathBuf::from("./data")))
            .pattern("*.jsonl")
            .partition("2024-01")
            .workers(8)
            .batch_size(500)
            .dedup(DedupStrategy::Both)
            .build()
            .unwrap();

        assert_eq!(config.pattern, "*.jsonl");
        assert_eq!(config.partition, Some("2024-01".to_string()));
        assert_eq!(config.workers, 8);
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.dedup, DedupStrategy::Both);
    }
}
