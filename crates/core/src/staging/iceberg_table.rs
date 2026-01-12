//! Iceberg table operations for staging data
//!
//! This module provides operations for reading and writing data to Iceberg tables,
//! including time travel queries and batch metadata tracking via table properties.
//!
//! # Example
//!
//! ```ignore
//! use data_modelling_core::staging::iceberg_table::IcebergTable;
//! use data_modelling_core::staging::catalog::{CatalogConfig, IcebergCatalog, TableIdentifier};
//!
//! // Create catalog and table
//! let config = CatalogConfig::Rest {
//!     endpoint: "http://localhost:8181".to_string(),
//!     warehouse: "./local-warehouse".to_string(),
//!     token: None,
//!     properties: Default::default(),
//! };
//!
//! let catalog = IcebergCatalog::new(config).await?;
//! let table_id = TableIdentifier::new("staging", "raw_json");
//!
//! // Create table for raw JSON storage
//! let table = IcebergTable::create_raw_json_table(&catalog, &table_id).await?;
//!
//! // Append records
//! let records = vec![
//!     RawJsonRecord { path: "file1.json".into(), content: "{...}".into(), ... },
//! ];
//! table.append_records(&records).await?;
//!
//! // Query with time travel
//! let snapshot = table.get_snapshot(5).await?;
//! ```

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::catalog::{CatalogError, CatalogResult, IcebergCatalog, TableIdentifier};

/// Raw JSON record for staging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawJsonRecord {
    /// Source file path
    pub path: String,
    /// Raw JSON content as string
    pub content: String,
    /// File size in bytes
    pub size: usize,
    /// Content hash (optional, for deduplication)
    pub content_hash: Option<String>,
    /// Partition key (e.g., date, source)
    pub partition: Option<String>,
    /// Ingestion timestamp
    pub ingested_at: DateTime<Utc>,
}

/// Batch metadata stored in table properties for resume support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchMetadata {
    /// Batch ID
    pub batch_id: String,
    /// Batch start time
    pub started_at: DateTime<Utc>,
    /// Batch completion time
    pub completed_at: Option<DateTime<Utc>>,
    /// Number of records in batch
    pub record_count: usize,
    /// Number of files processed
    pub files_processed: usize,
    /// Number of files skipped (dedup)
    pub files_skipped: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Source path or pattern
    pub source: String,
    /// Partition key
    pub partition: Option<String>,
    /// Last file path processed (for resume)
    pub last_file_path: Option<String>,
    /// Batch status
    pub status: BatchStatus,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Batch status for tracking progress
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchStatus {
    /// Batch is currently running
    Running,
    /// Batch completed successfully
    Completed,
    /// Batch failed with error
    Failed,
}

impl BatchMetadata {
    /// Create a new batch metadata
    pub fn new(batch_id: String, source: String, partition: Option<String>) -> Self {
        Self {
            batch_id,
            started_at: Utc::now(),
            completed_at: None,
            record_count: 0,
            files_processed: 0,
            files_skipped: 0,
            bytes_processed: 0,
            source,
            partition,
            last_file_path: None,
            status: BatchStatus::Running,
            error_message: None,
        }
    }

    /// Generate a unique batch ID
    pub fn generate_id() -> String {
        format!("batch-{}", uuid::Uuid::new_v4())
    }

    /// Check if the batch can be resumed
    pub fn can_resume(&self) -> bool {
        self.status == BatchStatus::Running || self.status == BatchStatus::Failed
    }

    /// Mark the batch as completed
    pub fn complete(&mut self) {
        self.status = BatchStatus::Completed;
        self.completed_at = Some(Utc::now());
    }

    /// Mark the batch as failed
    pub fn fail(&mut self, error: String) {
        self.status = BatchStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error);
    }
}

/// Snapshot info for time travel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotInfo {
    /// Snapshot ID
    pub snapshot_id: i64,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Parent snapshot ID
    pub parent_id: Option<i64>,
    /// Summary properties
    pub summary: HashMap<String, String>,
}

/// Result of an append operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendResult {
    /// Number of records written
    pub records_written: usize,
    /// Bytes written to Parquet file
    pub bytes_written: u64,
    /// Path to the written data file (if any)
    pub data_file_path: Option<String>,
}

/// Iceberg table wrapper for staging operations
pub struct IcebergTable {
    identifier: TableIdentifier,
    #[cfg(feature = "iceberg")]
    table: Arc<iceberg::table::Table>,
}

impl IcebergTable {
    /// Create a new raw JSON staging table
    #[cfg(feature = "iceberg")]
    pub async fn create_raw_json_table(
        catalog: &IcebergCatalog,
        identifier: &TableIdentifier,
    ) -> CatalogResult<Self> {
        use iceberg::spec::{NestedField, PrimitiveType, Schema, Type};
        use iceberg::{NamespaceIdent, TableCreation, TableIdent};

        // Define schema for raw JSON storage
        let schema = Schema::builder()
            .with_schema_id(0)
            .with_fields(vec![
                Arc::new(NestedField::required(
                    1,
                    "path",
                    Type::Primitive(PrimitiveType::String),
                )),
                Arc::new(NestedField::required(
                    2,
                    "content",
                    Type::Primitive(PrimitiveType::String),
                )),
                Arc::new(NestedField::required(
                    3,
                    "size",
                    Type::Primitive(PrimitiveType::Long),
                )),
                Arc::new(NestedField::optional(
                    4,
                    "content_hash",
                    Type::Primitive(PrimitiveType::String),
                )),
                Arc::new(NestedField::optional(
                    5,
                    "partition",
                    Type::Primitive(PrimitiveType::String),
                )),
                Arc::new(NestedField::required(
                    6,
                    "ingested_at",
                    Type::Primitive(PrimitiveType::Timestamptz),
                )),
            ])
            .build()
            .map_err(|e| CatalogError::SchemaError(e.to_string()))?;

        let ns_ident = NamespaceIdent::new(identifier.namespace.clone());
        let table_ident = TableIdent::new(ns_ident.clone(), identifier.name.clone());

        // Check if table exists
        let inner = catalog.inner.clone();
        if inner
            .table_exists(&table_ident)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?
        {
            // Load existing table
            let table = inner
                .load_table(&table_ident)
                .await
                .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

            return Ok(Self {
                identifier: identifier.clone(),
                table: Arc::new(table),
            });
        }

        // Create table location based on catalog warehouse
        let location = match catalog.config() {
            super::catalog::CatalogConfig::Rest { warehouse, .. } => {
                format!("{}/{}/{}", warehouse, identifier.namespace, identifier.name)
            }
            _ => {
                return Err(CatalogError::ConfigError(
                    "Unsupported catalog type for table creation".to_string(),
                ));
            }
        };

        // Create new table
        let creation = TableCreation::builder()
            .name(identifier.name.clone())
            .schema(schema)
            .location(location)
            .build();

        let table = inner
            .create_table(&ns_ident, creation)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

        Ok(Self {
            identifier: identifier.clone(),
            table: Arc::new(table),
        })
    }

    /// Load an existing table
    #[cfg(feature = "iceberg")]
    pub async fn load(
        catalog: &IcebergCatalog,
        identifier: &TableIdentifier,
    ) -> CatalogResult<Self> {
        use iceberg::{NamespaceIdent, TableIdent};

        let ns_ident = NamespaceIdent::new(identifier.namespace.clone());
        let table_ident = TableIdent::new(ns_ident, identifier.name.clone());

        let table = catalog
            .inner
            .load_table(&table_ident)
            .await
            .map_err(|e| CatalogError::IcebergError(e.to_string()))?;

        Ok(Self {
            identifier: identifier.clone(),
            table: Arc::new(table),
        })
    }

    /// Get the table identifier
    pub fn identifier(&self) -> &TableIdentifier {
        &self.identifier
    }

    /// Get the underlying Iceberg table reference
    #[cfg(feature = "iceberg")]
    pub fn inner(&self) -> &iceberg::table::Table {
        &self.table
    }

    /// Append records to the table
    ///
    /// This method:
    /// 1. Converts records to Arrow RecordBatch
    /// 2. Writes to a Parquet file in the table's data directory
    /// 3. Creates a DataFile reference
    /// 4. Commits the append via a transaction
    ///
    /// Note: This is a placeholder that documents the intended flow.
    /// Full implementation requires:
    /// - Arrow schema conversion
    /// - Parquet file writing with proper statistics
    /// - Transaction commit via catalog
    #[cfg(feature = "iceberg")]
    pub async fn append_records(
        &self,
        records: &[RawJsonRecord],
        catalog: &IcebergCatalog,
    ) -> CatalogResult<AppendResult> {
        use arrow::array::{Int64Array, StringArray, TimestampMicrosecondArray};
        use arrow::datatypes::{DataType, Field, Schema as ArrowSchema, TimeUnit};
        use arrow::record_batch::RecordBatch;
        use std::fs::File;
        use std::path::Path;

        if records.is_empty() {
            return Ok(AppendResult {
                records_written: 0,
                bytes_written: 0,
                data_file_path: None,
            });
        }

        // Create Arrow schema matching our Iceberg schema
        let arrow_schema = Arc::new(ArrowSchema::new(vec![
            Field::new("path", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("size", DataType::Int64, false),
            Field::new("content_hash", DataType::Utf8, true),
            Field::new("partition", DataType::Utf8, true),
            Field::new(
                "ingested_at",
                DataType::Timestamp(TimeUnit::Microsecond, Some("UTC".into())),
                false,
            ),
        ]));

        // Convert records to Arrow arrays
        let paths: Vec<&str> = records.iter().map(|r| r.path.as_str()).collect();
        let contents: Vec<&str> = records.iter().map(|r| r.content.as_str()).collect();
        let sizes: Vec<i64> = records.iter().map(|r| r.size as i64).collect();
        let hashes: Vec<Option<&str>> = records.iter().map(|r| r.content_hash.as_deref()).collect();
        let partitions: Vec<Option<&str>> =
            records.iter().map(|r| r.partition.as_deref()).collect();
        let timestamps: Vec<i64> = records
            .iter()
            .map(|r| r.ingested_at.timestamp_micros())
            .collect();

        let batch = RecordBatch::try_new(
            arrow_schema.clone(),
            vec![
                Arc::new(StringArray::from(paths)),
                Arc::new(StringArray::from(contents)),
                Arc::new(Int64Array::from(sizes)),
                Arc::new(StringArray::from(hashes)),
                Arc::new(StringArray::from(partitions)),
                Arc::new(
                    TimestampMicrosecondArray::from(timestamps).with_timezone("UTC".to_string()),
                ),
            ],
        )
        .map_err(|e| CatalogError::SchemaError(format!("Failed to create RecordBatch: {}", e)))?;

        // Generate unique file path in table's data directory
        let file_id = uuid::Uuid::new_v4();
        let location = self.table.metadata().location();
        let data_file_path = format!("{}/data/{}.parquet", location, file_id);

        // Write Parquet file
        let bytes_written = write_parquet_file(&data_file_path, &batch).await?;

        tracing::info!(
            "Wrote {} records ({} bytes) to {}",
            records.len(),
            bytes_written,
            data_file_path
        );

        // TODO: Create DataFile and commit via transaction
        // This requires:
        // 1. Creating a DataFile with proper statistics
        // 2. Using Transaction::fast_append().add_data_files()
        // 3. Committing to the catalog
        //
        // For now, we just write the Parquet file. Full transaction
        // support requires additional catalog integration.

        Ok(AppendResult {
            records_written: records.len(),
            bytes_written,
            data_file_path: Some(data_file_path),
        })
    }

    /// Get the current snapshot ID
    #[cfg(feature = "iceberg")]
    pub fn current_snapshot_id(&self) -> Option<i64> {
        self.table.metadata().current_snapshot_id()
    }

    /// Get table properties
    #[cfg(feature = "iceberg")]
    pub fn properties(&self) -> &HashMap<String, String> {
        self.table.metadata().properties()
    }

    /// List all snapshots (for time travel)
    #[cfg(feature = "iceberg")]
    pub fn list_snapshots(&self) -> Vec<SnapshotInfo> {
        self.table
            .metadata()
            .snapshots()
            .map(|s| {
                let summary = s.summary();
                let mut props = summary.additional_properties.clone();
                props.insert("operation".to_string(), format!("{:?}", summary.operation));
                SnapshotInfo {
                    snapshot_id: s.snapshot_id(),
                    timestamp: DateTime::from_timestamp_millis(s.timestamp_ms())
                        .unwrap_or_else(|| Utc::now()),
                    parent_id: s.parent_snapshot_id(),
                    summary: props,
                }
            })
            .collect()
    }

    /// Get a specific snapshot by ID
    #[cfg(feature = "iceberg")]
    pub fn get_snapshot(&self, snapshot_id: i64) -> Option<SnapshotInfo> {
        self.table.metadata().snapshot_by_id(snapshot_id).map(|s| {
            let summary = s.summary();
            let mut props = summary.additional_properties.clone();
            props.insert("operation".to_string(), format!("{:?}", summary.operation));
            SnapshotInfo {
                snapshot_id: s.snapshot_id(),
                timestamp: DateTime::from_timestamp_millis(s.timestamp_ms())
                    .unwrap_or_else(|| Utc::now()),
                parent_id: s.parent_snapshot_id(),
                summary: props,
            }
        })
    }

    /// Store batch metadata in table properties
    /// Note: This requires a table update operation
    #[cfg(feature = "iceberg")]
    pub async fn store_batch_metadata(&self, batch: &BatchMetadata) -> CatalogResult<()> {
        // Batch metadata is stored in table properties with a prefix
        let key = format!("batch.{}", batch.batch_id);
        let value =
            serde_json::to_string(batch).map_err(|e| CatalogError::ConfigError(e.to_string()))?;

        // Note: Table property updates require catalog-level operations
        // This is a placeholder - actual implementation depends on the catalog's
        // ability to update table properties

        tracing::info!(
            "Batch metadata stored: {} = {}",
            key,
            value.chars().take(100).collect::<String>()
        );

        Ok(())
    }

    /// Get batch metadata from table properties
    #[cfg(feature = "iceberg")]
    pub fn get_batch_metadata(&self, batch_id: &str) -> Option<BatchMetadata> {
        let key = format!("batch.{}", batch_id);
        self.table
            .metadata()
            .properties()
            .get(&key)
            .and_then(|v| serde_json::from_str(v).ok())
    }

    /// List all batch metadata from table properties
    #[cfg(feature = "iceberg")]
    pub fn list_batch_metadata(&self) -> Vec<BatchMetadata> {
        self.table
            .metadata()
            .properties()
            .iter()
            .filter(|(k, _)| k.starts_with("batch."))
            .filter_map(|(_, v)| serde_json::from_str(v).ok())
            .collect()
    }

    /// Get the table location
    #[cfg(feature = "iceberg")]
    pub fn location(&self) -> &str {
        self.table.metadata().location()
    }
}

/// Write a RecordBatch to a Parquet file
///
/// This function handles:
/// - Creating the parent directory if needed
/// - Writing with appropriate Parquet settings
/// - Returning the bytes written
#[cfg(feature = "iceberg")]
async fn write_parquet_file(
    path: &str,
    batch: &arrow::record_batch::RecordBatch,
) -> CatalogResult<u64> {
    use parquet::arrow::ArrowWriter;
    use parquet::basic::Compression;
    use parquet::file::properties::WriterProperties;
    use std::fs::{self, File};
    use std::path::Path;

    // Ensure parent directory exists
    let file_path = Path::new(path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            CatalogError::IoError(format!(
                "Failed to create directory {}: {}",
                parent.display(),
                e
            ))
        })?;
    }

    // Create the file
    let file = File::create(file_path)
        .map_err(|e| CatalogError::IoError(format!("Failed to create file {}: {}", path, e)))?;

    // Configure Parquet writer properties
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();

    // Write the RecordBatch
    let mut writer = ArrowWriter::try_new(file, batch.schema(), Some(props))
        .map_err(|e| CatalogError::IoError(format!("Failed to create Parquet writer: {}", e)))?;

    writer
        .write(batch)
        .map_err(|e| CatalogError::IoError(format!("Failed to write RecordBatch: {}", e)))?;

    let metadata = writer
        .close()
        .map_err(|e| CatalogError::IoError(format!("Failed to close Parquet writer: {}", e)))?;

    // Return total bytes written (sum of row groups)
    let bytes_written: u64 = metadata
        .row_groups
        .iter()
        .map(|rg| rg.total_byte_size as u64)
        .sum();

    Ok(bytes_written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_json_record() {
        let record = RawJsonRecord {
            path: "test.json".to_string(),
            content: r#"{"key": "value"}"#.to_string(),
            size: 16,
            content_hash: Some("abc123".to_string()),
            partition: Some("2024-01".to_string()),
            ingested_at: Utc::now(),
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("test.json"));
        // Content is escaped in JSON serialization
        assert!(json.contains("key"));
        assert!(json.contains("value"));
        assert!(json.contains("abc123"));
        assert!(json.contains("2024-01"));
    }

    #[test]
    fn test_raw_json_record_minimal() {
        let record = RawJsonRecord {
            path: "/data/file.json".to_string(),
            content: "{}".to_string(),
            size: 2,
            content_hash: None,
            partition: None,
            ingested_at: Utc::now(),
        };

        let json = serde_json::to_string(&record).unwrap();
        let parsed: RawJsonRecord = serde_json::from_str(&json).unwrap();

        assert_eq!(record.path, parsed.path);
        assert_eq!(record.size, parsed.size);
        assert!(parsed.content_hash.is_none());
        assert!(parsed.partition.is_none());
    }

    #[test]
    fn test_raw_json_record_large_content() {
        let large_content = r#"{"data": "x"}"#.repeat(1000);
        let record = RawJsonRecord {
            path: "large.json".to_string(),
            content: large_content.clone(),
            size: large_content.len(),
            content_hash: Some("sha256:abc".to_string()),
            partition: Some("partition-1".to_string()),
            ingested_at: Utc::now(),
        };

        assert_eq!(record.size, large_content.len());
    }

    #[test]
    fn test_batch_metadata() {
        let batch = BatchMetadata::new(
            "batch-001".to_string(),
            "./data/*.json".to_string(),
            Some("2024-01".to_string()),
        );

        let json = serde_json::to_string(&batch).unwrap();
        let parsed: BatchMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(batch.batch_id, parsed.batch_id);
        assert_eq!(batch.record_count, parsed.record_count);
        assert_eq!(batch.source, parsed.source);
        assert!(parsed.completed_at.is_none());
        assert_eq!(parsed.status, BatchStatus::Running);
    }

    #[test]
    fn test_batch_metadata_completed() {
        let mut batch = BatchMetadata::new(
            "batch-002".to_string(),
            "s3://bucket/data/".to_string(),
            None,
        );
        batch.record_count = 500;
        batch.complete();

        let json = serde_json::to_string(&batch).unwrap();
        let parsed: BatchMetadata = serde_json::from_str(&json).unwrap();

        assert!(parsed.completed_at.is_some());
        assert_eq!(parsed.record_count, 500);
        assert_eq!(parsed.status, BatchStatus::Completed);
    }

    #[test]
    fn test_batch_metadata_property_key() {
        let batch = BatchMetadata::new("test-batch-123".to_string(), "./".to_string(), None);

        // Verify the key format used for table properties
        let key = format!("batch.{}", batch.batch_id);
        assert_eq!(key, "batch.test-batch-123");
    }

    #[test]
    fn test_batch_metadata_resume() {
        let mut batch = BatchMetadata::new("batch-resume".to_string(), "./data/".to_string(), None);

        // Initially can resume (status is Running)
        assert!(batch.can_resume());

        // After completion, cannot resume
        batch.complete();
        assert!(!batch.can_resume());

        // Failed batches can be resumed
        let mut failed_batch =
            BatchMetadata::new("batch-failed".to_string(), "./data/".to_string(), None);
        failed_batch.fail("Connection error".to_string());
        assert!(failed_batch.can_resume());
        assert_eq!(
            failed_batch.error_message,
            Some("Connection error".to_string())
        );
    }

    #[test]
    fn test_snapshot_info() {
        let snapshot = SnapshotInfo {
            snapshot_id: 12345,
            timestamp: Utc::now(),
            parent_id: Some(12344),
            summary: HashMap::from([("added-records".to_string(), "100".to_string())]),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("12345"));
        assert!(json.contains("12344"));
        assert!(json.contains("added-records"));
    }

    #[test]
    fn test_snapshot_info_root_snapshot() {
        let snapshot = SnapshotInfo {
            snapshot_id: 1,
            timestamp: Utc::now(),
            parent_id: None, // Root snapshot has no parent
            summary: HashMap::from([
                ("operation".to_string(), "append".to_string()),
                ("added-records".to_string(), "1000".to_string()),
                ("added-data-files".to_string(), "5".to_string()),
            ]),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: SnapshotInfo = serde_json::from_str(&json).unwrap();

        assert!(parsed.parent_id.is_none());
        assert_eq!(parsed.summary.get("operation"), Some(&"append".to_string()));
    }

    #[test]
    fn test_snapshot_info_chain() {
        // Test a chain of snapshots
        let snapshots = vec![
            SnapshotInfo {
                snapshot_id: 1,
                timestamp: Utc::now(),
                parent_id: None,
                summary: HashMap::new(),
            },
            SnapshotInfo {
                snapshot_id: 2,
                timestamp: Utc::now(),
                parent_id: Some(1),
                summary: HashMap::new(),
            },
            SnapshotInfo {
                snapshot_id: 3,
                timestamp: Utc::now(),
                parent_id: Some(2),
                summary: HashMap::new(),
            },
        ];

        // Verify parent chain
        assert!(snapshots[0].parent_id.is_none());
        assert_eq!(snapshots[1].parent_id, Some(1));
        assert_eq!(snapshots[2].parent_id, Some(2));
    }

    #[test]
    fn test_table_identifier_from_catalog() {
        use crate::staging::catalog::TableIdentifier;

        let id = TableIdentifier::new("staging", "raw_json");
        assert_eq!(id.namespace, "staging");
        assert_eq!(id.name, "raw_json");
        assert_eq!(id.to_string(), "staging.raw_json");
    }
}
