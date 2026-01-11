//! Staging database implementation

#![allow(unexpected_cfgs)]
#![allow(clippy::type_complexity)]
#![allow(clippy::collapsible_if)]

#[cfg(feature = "duckdb-backend")]
use std::collections::HashSet;
#[cfg(feature = "duckdb-backend")]
use std::time::Instant;

#[cfg(feature = "duckdb-backend")]
use chrono::Utc;

#[cfg(feature = "duckdb-backend")]
use super::batch::{BatchStatus, ProcessingBatch};
#[cfg(feature = "duckdb-backend")]
use super::config::{DedupStrategy, IngestConfig, SourceType};
#[cfg(feature = "duckdb-backend")]
use super::error::{IngestError, StagingError};
#[cfg(feature = "duckdb-backend")]
use super::ingest::{IngestStats, discover_local_files, parse_file, should_skip_file};
#[cfg(feature = "duckdb-backend")]
use super::schema::{SCHEMA_VERSION, StagingSchema};

/// Staging database for raw JSON ingestion
///
/// Supports both DuckDB (embedded) and PostgreSQL backends.
#[cfg(feature = "duckdb-backend")]
pub struct StagingDb {
    conn: duckdb::Connection,
    path: Option<String>,
}

#[cfg(feature = "duckdb-backend")]
impl StagingDb {
    /// Open or create a staging database at the given path
    pub fn open(path: &str) -> Result<Self, StagingError> {
        let conn = duckdb::Connection::open(path)?;
        Ok(Self {
            conn,
            path: Some(path.to_string()),
        })
    }

    /// Open an in-memory database (for testing)
    pub fn memory() -> Result<Self, StagingError> {
        let conn = duckdb::Connection::open_in_memory()?;
        Ok(Self { conn, path: None })
    }

    /// Get the database path (if not in-memory)
    pub fn path(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Initialize the database schema
    pub fn init(&self) -> Result<(), StagingError> {
        // Run the DDL statements
        let ddl = StagingSchema::create_tables_duckdb();
        self.conn.execute_batch(ddl)?;

        // Set the schema version
        self.conn.execute(
            "INSERT INTO schema_info (key, value) VALUES ('version', ?1)
             ON CONFLICT (key) DO UPDATE SET value = ?1",
            [SCHEMA_VERSION.to_string()],
        )?;

        Ok(())
    }

    /// Check if the database is initialized
    pub fn is_initialized(&self) -> Result<bool, StagingError> {
        let result: Result<i32, _> = self.conn.query_row(
            "SELECT 1 FROM information_schema.tables WHERE table_name = 'staged_json'",
            [],
            |row| row.get(0),
        );
        Ok(result.is_ok())
    }

    /// Get the schema version
    pub fn schema_version(&self) -> Result<i32, StagingError> {
        let version: String =
            self.conn
                .query_row(StagingSchema::select_schema_version(), [], |row| row.get(0))?;
        version
            .parse()
            .map_err(|_| StagingError::Database("Invalid schema version".to_string()))
    }

    /// Get the total record count
    pub fn record_count(&self, partition: Option<&str>) -> Result<i64, StagingError> {
        let count: i64 = if let Some(partition) = partition {
            self.conn.query_row(
                "SELECT COUNT(*) FROM staged_json WHERE partition_key = ?1",
                [partition],
                |row| row.get(0),
            )?
        } else {
            self.conn
                .query_row("SELECT COUNT(*) FROM staged_json", [], |row| row.get(0))?
        };
        Ok(count)
    }

    /// Get sample records for schema inference
    pub fn get_sample(
        &self,
        limit: usize,
        partition: Option<&str>,
    ) -> Result<Vec<String>, StagingError> {
        let mut samples = Vec::new();

        if let Some(partition) = partition {
            let mut stmt = self.conn.prepare(
                "SELECT raw_json FROM staged_json WHERE partition_key = ?1 ORDER BY RANDOM() LIMIT ?2"
            )?;
            let rows = stmt.query_map(duckdb::params![partition, limit as i64], |row| {
                row.get::<_, String>(0)
            })?;
            for row in rows {
                samples.push(row?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare("SELECT raw_json FROM staged_json ORDER BY RANDOM() LIMIT ?1")?;
            let rows =
                stmt.query_map(duckdb::params![limit as i64], |row| row.get::<_, String>(0))?;
            for row in rows {
                samples.push(row?);
            }
        }

        Ok(samples)
    }

    /// Execute a query and return results as JSON
    pub fn query(&self, sql: &str) -> Result<Vec<serde_json::Value>, StagingError> {
        let mut stmt = self.conn.prepare(sql)?;
        let mut rows = stmt.query([])?;

        // Get column names after query execution
        let column_count = rows.as_ref().map(|r| r.column_count()).unwrap_or(0);
        let column_names: Vec<String> = (0..column_count)
            .map(|i| {
                rows.as_ref()
                    .and_then(|r| r.column_name(i).ok())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("col{}", i))
            })
            .collect();

        let mut results = Vec::new();

        while let Some(row) = rows.next()? {
            let mut obj = serde_json::Map::new();
            for (i, name) in column_names.iter().enumerate() {
                let value: duckdb::types::Value = row.get(i)?;
                let json_value = match value {
                    duckdb::types::Value::Null => serde_json::Value::Null,
                    duckdb::types::Value::Boolean(b) => serde_json::Value::Bool(b),
                    duckdb::types::Value::TinyInt(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::Value::SmallInt(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::Value::Int(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::Value::BigInt(n) => serde_json::Value::Number(n.into()),
                    duckdb::types::Value::Float(f) => serde_json::Number::from_f64(f as f64)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    duckdb::types::Value::Double(f) => serde_json::Number::from_f64(f)
                        .map(serde_json::Value::Number)
                        .unwrap_or(serde_json::Value::Null),
                    duckdb::types::Value::Text(s) => serde_json::Value::String(s),
                    _ => serde_json::Value::String(format!("{:?}", value)),
                };
                obj.insert(name.clone(), json_value);
            }
            results.push(serde_json::Value::Object(obj));
        }

        Ok(results)
    }

    /// Get existing file paths for deduplication
    fn get_existing_paths(&self, partition: Option<&str>) -> Result<HashSet<String>, StagingError> {
        let mut paths = HashSet::new();

        if let Some(partition) = partition {
            let mut stmt = self
                .conn
                .prepare("SELECT DISTINCT file_path FROM staged_json WHERE partition_key = ?1")?;
            let rows = stmt.query_map([partition], |row| row.get::<_, String>(0))?;
            for row in rows {
                paths.insert(row?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare("SELECT DISTINCT file_path FROM staged_json")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                paths.insert(row?);
            }
        }

        Ok(paths)
    }

    /// Get existing content hashes for deduplication
    fn get_existing_hashes(
        &self,
        partition: Option<&str>,
    ) -> Result<HashSet<String>, StagingError> {
        let mut hashes = HashSet::new();

        if let Some(partition) = partition {
            let mut stmt = self.conn.prepare(
                "SELECT DISTINCT content_hash FROM staged_json WHERE partition_key = ?1 AND content_hash IS NOT NULL"
            )?;
            let rows = stmt.query_map([partition], |row| row.get::<_, String>(0))?;
            for row in rows {
                hashes.insert(row?);
            }
        } else {
            let mut stmt = self.conn.prepare(
                "SELECT DISTINCT content_hash FROM staged_json WHERE content_hash IS NOT NULL",
            )?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            for row in rows {
                hashes.insert(row?);
            }
        }

        Ok(hashes)
    }

    /// Get the next available ID for staged_json
    fn next_id(&self) -> Result<i64, StagingError> {
        let id: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(id), 0) + 1 FROM staged_json",
            [],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Insert a batch of records
    fn insert_records(
        &self,
        records: &[(String, String, usize, Option<String>, Option<String>, u64)],
        start_id: i64,
    ) -> Result<(), StagingError> {
        let mut stmt = self.conn.prepare(
            "INSERT INTO staged_json (id, file_path, record_index, partition_key, raw_json, content_hash, file_size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        for (i, (file_path, json, record_index, partition, hash, size)) in
            records.iter().enumerate()
        {
            stmt.execute(duckdb::params![
                start_id + i as i64,
                file_path,
                *record_index as i32,
                partition.as_deref(),
                json,
                hash.as_deref(),
                *size as i64,
            ])?;
        }

        Ok(())
    }

    /// Create a new processing batch
    pub fn create_batch(&self, batch: &ProcessingBatch) -> Result<(), StagingError> {
        self.conn.execute(
            "INSERT INTO processing_batches
             (id, source_path, source_type, partition_key, pattern, status,
              files_total, files_processed, files_skipped, records_ingested,
              bytes_processed, errors_count, started_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            duckdb::params![
                batch.id,
                batch.source_path,
                batch.source_type,
                batch.partition_key.as_deref(),
                batch.pattern,
                batch.status.to_string(),
                batch.files_total,
                batch.files_processed,
                batch.files_skipped,
                batch.records_ingested,
                batch.bytes_processed,
                batch.errors_count,
                batch.started_at.map(|t| t.to_rfc3339()),
                batch.updated_at.map(|t| t.to_rfc3339()),
            ],
        )?;
        Ok(())
    }

    /// Update a processing batch
    pub fn update_batch(&self, batch: &ProcessingBatch) -> Result<(), StagingError> {
        self.conn.execute(
            "UPDATE processing_batches SET
             status = ?2, files_total = ?3, files_processed = ?4, files_skipped = ?5,
             records_ingested = ?6, bytes_processed = ?7, errors_count = ?8,
             last_file_path = ?9, last_record_index = ?10,
             updated_at = ?11, completed_at = ?12, error_message = ?13
             WHERE id = ?1",
            duckdb::params![
                batch.id,
                batch.status.to_string(),
                batch.files_total,
                batch.files_processed,
                batch.files_skipped,
                batch.records_ingested,
                batch.bytes_processed,
                batch.errors_count,
                batch.last_file_path.as_deref(),
                batch.last_record_index,
                batch.updated_at.map(|t| t.to_rfc3339()),
                batch.completed_at.map(|t| t.to_rfc3339()),
                batch.error_message.as_deref(),
            ],
        )?;
        Ok(())
    }

    /// Get a batch by ID
    pub fn get_batch(&self, batch_id: &str) -> Result<Option<ProcessingBatch>, StagingError> {
        let result = self.conn.query_row(
            "SELECT id, source_path, source_type, partition_key, pattern, status,
                    files_total, files_processed, files_skipped, records_ingested,
                    bytes_processed, errors_count, last_file_path, last_record_index,
                    started_at, updated_at, completed_at, error_message
             FROM processing_batches WHERE id = ?1",
            [batch_id],
            |row| {
                Ok(ProcessingBatch {
                    id: row.get(0)?,
                    source_path: row.get(1)?,
                    source_type: row.get(2)?,
                    partition_key: row.get(3)?,
                    pattern: row.get(4)?,
                    status: row
                        .get::<_, String>(5)?
                        .parse()
                        .unwrap_or(BatchStatus::Running),
                    files_total: row.get(6)?,
                    files_processed: row.get(7)?,
                    files_skipped: row.get(8)?,
                    records_ingested: row.get(9)?,
                    bytes_processed: row.get(10)?,
                    errors_count: row.get(11)?,
                    last_file_path: row.get(12)?,
                    last_record_index: row.get(13)?,
                    started_at: row
                        .get::<_, Option<String>>(14)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    updated_at: row
                        .get::<_, Option<String>>(15)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    completed_at: row
                        .get::<_, Option<String>>(16)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    error_message: row.get(17)?,
                })
            },
        );

        match result {
            Ok(batch) => Ok(Some(batch)),
            Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List recent batches
    pub fn list_batches(&self, limit: usize) -> Result<Vec<ProcessingBatch>, StagingError> {
        let mut batches = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT id, source_path, source_type, partition_key, pattern, status,
                    files_total, files_processed, files_skipped, records_ingested,
                    bytes_processed, errors_count, last_file_path, last_record_index,
                    started_at, updated_at, completed_at, error_message
             FROM processing_batches
             ORDER BY started_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map([limit], |row| {
            Ok(ProcessingBatch {
                id: row.get(0)?,
                source_path: row.get(1)?,
                source_type: row.get(2)?,
                partition_key: row.get(3)?,
                pattern: row.get(4)?,
                status: row
                    .get::<_, String>(5)?
                    .parse()
                    .unwrap_or(BatchStatus::Running),
                files_total: row.get(6)?,
                files_processed: row.get(7)?,
                files_skipped: row.get(8)?,
                records_ingested: row.get(9)?,
                bytes_processed: row.get(10)?,
                errors_count: row.get(11)?,
                last_file_path: row.get(12)?,
                last_record_index: row.get(13)?,
                started_at: row
                    .get::<_, Option<String>>(14)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                updated_at: row
                    .get::<_, Option<String>>(15)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                completed_at: row
                    .get::<_, Option<String>>(16)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                error_message: row.get(17)?,
            })
        })?;

        for row in rows {
            batches.push(row?);
        }

        Ok(batches)
    }

    /// Get partition statistics
    pub fn partition_stats(&self) -> Result<Vec<(String, i64)>, StagingError> {
        let mut stats = Vec::new();
        let mut stmt = self.conn.prepare(
            "SELECT COALESCE(partition_key, '<none>'), COUNT(*)
             FROM staged_json
             GROUP BY partition_key
             ORDER BY COUNT(*) DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            stats.push(row?);
        }

        Ok(stats)
    }

    /// Ingest files from the configured source
    pub fn ingest(&self, config: &IngestConfig) -> Result<IngestStats, IngestError> {
        let start = Instant::now();
        let mut stats = IngestStats::new();

        // Check if database is initialized
        if !self.is_initialized()? {
            return Err(IngestError::Staging(StagingError::NotInitialized));
        }

        // Create or resume batch
        let batch_id = config
            .batch_id
            .clone()
            .unwrap_or_else(ProcessingBatch::generate_id);

        let source_type_str = match &config.source {
            SourceType::Local(_) => "local",
            #[cfg(feature = "s3")]
            SourceType::S3 { .. } => "s3",
            #[cfg(feature = "databricks")]
            SourceType::UnityVolume { .. } => "unity_volume",
        };

        let mut batch = if config.resume {
            match self.get_batch(&batch_id)? {
                Some(b) if b.can_resume() => b,
                Some(_) => return Err(IngestError::BatchCompleted(batch_id)),
                None => return Err(IngestError::BatchNotFound(batch_id)),
            }
        } else {
            let b = ProcessingBatch::new(
                batch_id,
                config.source.display(),
                source_type_str.to_string(),
                config.partition.clone(),
                config.pattern.clone(),
            );
            self.create_batch(&b)?;
            b
        };

        // Discover files based on source type
        let files = match &config.source {
            SourceType::Local(path) => discover_local_files(path, &config.pattern)?,
            #[cfg(feature = "s3")]
            SourceType::S3 { .. } => {
                // S3 discovery would go here
                return Err(IngestError::SourceNotAccessible {
                    path: config.source.display(),
                    reason: "S3 ingestion not yet implemented".to_string(),
                });
            }
            #[cfg(feature = "databricks")]
            SourceType::UnityVolume { .. } => {
                // Unity Catalog discovery would go here
                return Err(IngestError::SourceNotAccessible {
                    path: config.source.display(),
                    reason: "Unity Catalog ingestion not yet implemented".to_string(),
                });
            }
        };

        batch.files_total = files.len() as i32;

        // Get existing data for deduplication
        let existing_paths = if matches!(config.dedup, DedupStrategy::ByPath | DedupStrategy::Both)
        {
            self.get_existing_paths(config.partition.as_deref())?
        } else {
            HashSet::new()
        };

        let existing_hashes =
            if matches!(config.dedup, DedupStrategy::ByContent | DedupStrategy::Both) {
                self.get_existing_hashes(config.partition.as_deref())?
            } else {
                HashSet::new()
            };

        // Determine resume point
        let resume_after = if config.resume {
            batch.last_file_path.clone()
        } else {
            None
        };

        let mut next_id = self.next_id()?;
        let mut records_batch: Vec<(String, String, usize, Option<String>, Option<String>, u64)> =
            Vec::new();
        let mut past_resume_point = resume_after.is_none();

        for mut file in files {
            let file_path_str = file.path.display().to_string();

            // Skip files before resume point
            if !past_resume_point {
                if Some(&file_path_str) == resume_after.as_ref() {
                    past_resume_point = true;
                }
                continue;
            }

            // Compute hash if needed for dedup
            if matches!(config.dedup, DedupStrategy::ByContent | DedupStrategy::Both) {
                if let Err(e) = file.compute_hash() {
                    stats.add_error(format!("Error computing hash for {}: {}", file_path_str, e));
                    continue;
                }
            }

            // Check deduplication
            if should_skip_file(&file, config.dedup, &existing_paths, &existing_hashes) {
                stats.files_skipped += 1;
                batch.files_skipped += 1;
                continue;
            }

            // Parse the file
            let records = match parse_file(&file.path) {
                Ok(r) => r,
                Err(e) => {
                    stats.add_error(format!("Error parsing {}: {}", file_path_str, e));
                    batch.increment_errors();
                    continue;
                }
            };

            // Add records to batch
            for record in records {
                records_batch.push((
                    file_path_str.clone(),
                    record.json,
                    record.index,
                    config.partition.clone(),
                    file.content_hash.clone(),
                    file.size,
                ));

                // Insert batch when full
                if records_batch.len() >= config.batch_size {
                    self.insert_records(&records_batch, next_id)?;
                    stats.records_ingested += records_batch.len();
                    batch.records_ingested += records_batch.len() as i64;
                    next_id += records_batch.len() as i64;
                    records_batch.clear();
                }
            }

            stats.files_processed += 1;
            stats.bytes_processed += file.size;
            batch.files_processed += 1;
            batch.bytes_processed += file.size as i64;
            batch.last_file_path = Some(file_path_str);

            // Update batch progress periodically
            if batch.files_processed % 100 == 0 {
                self.update_batch(&batch)?;
            }
        }

        // Insert remaining records
        if !records_batch.is_empty() {
            self.insert_records(&records_batch, next_id)?;
            stats.records_ingested += records_batch.len();
            batch.records_ingested += records_batch.len() as i64;
        }

        // Complete batch
        batch.complete();
        self.update_batch(&batch)?;

        stats.duration = start.elapsed();
        Ok(stats)
    }
}

// ============================================================================
// PostgreSQL Implementation
// ============================================================================

#[cfg(feature = "postgres-backend")]
pub use postgres_impl::StagingDbPostgres;

#[cfg(feature = "postgres-backend")]
mod postgres_impl {
    use std::collections::HashSet;
    use std::time::Instant;

    use chrono::Utc;
    use tokio_postgres::{Client, NoTls};

    use crate::staging::batch::{BatchStatus, ProcessingBatch};
    use crate::staging::config::{DedupStrategy, IngestConfig, SourceType};
    use crate::staging::error::{IngestError, StagingError};
    use crate::staging::ingest::{IngestStats, discover_local_files, parse_file, should_skip_file};
    use crate::staging::schema::{SCHEMA_VERSION, StagingSchema};

    /// PostgreSQL staging database (async)
    pub struct StagingDbPostgres {
        client: Client,
        connection_string: String,
    }

    impl StagingDbPostgres {
        /// Connect to a PostgreSQL database
        pub async fn connect(connection_string: &str) -> Result<Self, StagingError> {
            let (client, connection) = tokio_postgres::connect(connection_string, NoTls)
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            // Spawn connection handler
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("PostgreSQL connection error: {}", e);
                }
            });

            Ok(Self {
                client,
                connection_string: connection_string.to_string(),
            })
        }

        /// Get the connection string
        pub fn connection_string(&self) -> &str {
            &self.connection_string
        }

        /// Initialize the database schema
        pub async fn init(&self) -> Result<(), StagingError> {
            let ddl = StagingSchema::create_tables_postgres();
            self.client
                .batch_execute(ddl)
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            // Set the schema version
            self.client
                .execute(
                    "INSERT INTO schema_info (key, value) VALUES ('version', $1)
                     ON CONFLICT (key) DO UPDATE SET value = $1",
                    &[&SCHEMA_VERSION.to_string()],
                )
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(())
        }

        /// Check if the database is initialized
        pub async fn is_initialized(&self) -> Result<bool, StagingError> {
            let result = self
                .client
                .query_opt(
                    "SELECT 1 FROM information_schema.tables WHERE table_name = 'staged_json'",
                    &[],
                )
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(result.is_some())
        }

        /// Get the schema version
        pub async fn schema_version(&self) -> Result<i32, StagingError> {
            let row = self
                .client
                .query_one(StagingSchema::select_schema_version(), &[])
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            let version: String = row.get(0);
            version
                .parse()
                .map_err(|_| StagingError::Database("Invalid schema version".to_string()))
        }

        /// Get the total record count
        pub async fn record_count(&self, partition: Option<&str>) -> Result<i64, StagingError> {
            let count: i64 = if let Some(partition) = partition {
                let row = self
                    .client
                    .query_one(
                        "SELECT COUNT(*) FROM staged_json WHERE partition_key = $1",
                        &[&partition],
                    )
                    .await
                    .map_err(|e| StagingError::Database(e.to_string()))?;
                row.get(0)
            } else {
                let row = self
                    .client
                    .query_one("SELECT COUNT(*) FROM staged_json", &[])
                    .await
                    .map_err(|e| StagingError::Database(e.to_string()))?;
                row.get(0)
            };
            Ok(count)
        }

        /// Get sample records for schema inference
        pub async fn get_sample(
            &self,
            limit: usize,
            partition: Option<&str>,
        ) -> Result<Vec<String>, StagingError> {
            let rows = if let Some(partition) = partition {
                self.client
                    .query(
                        "SELECT raw_json::text FROM staged_json WHERE partition_key = $1 ORDER BY RANDOM() LIMIT $2",
                        &[&partition, &(limit as i64)],
                    )
                    .await
            } else {
                self.client
                    .query(
                        "SELECT raw_json::text FROM staged_json ORDER BY RANDOM() LIMIT $1",
                        &[&(limit as i64)],
                    )
                    .await
            }
            .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
        }

        /// Get existing file paths for deduplication
        async fn get_existing_paths(
            &self,
            partition: Option<&str>,
        ) -> Result<HashSet<String>, StagingError> {
            let rows = if let Some(partition) = partition {
                self.client
                    .query(
                        "SELECT DISTINCT file_path FROM staged_json WHERE partition_key = $1",
                        &[&partition],
                    )
                    .await
            } else {
                self.client
                    .query("SELECT DISTINCT file_path FROM staged_json", &[])
                    .await
            }
            .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
        }

        /// Get existing content hashes for deduplication
        async fn get_existing_hashes(
            &self,
            partition: Option<&str>,
        ) -> Result<HashSet<String>, StagingError> {
            let rows = if let Some(partition) = partition {
                self.client
                    .query(
                        "SELECT DISTINCT content_hash FROM staged_json WHERE partition_key = $1 AND content_hash IS NOT NULL",
                        &[&partition],
                    )
                    .await
            } else {
                self.client
                    .query(
                        "SELECT DISTINCT content_hash FROM staged_json WHERE content_hash IS NOT NULL",
                        &[],
                    )
                    .await
            }
            .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(rows.iter().map(|r| r.get::<_, String>(0)).collect())
        }

        /// Insert a batch of records
        async fn insert_records(
            &self,
            records: &[(String, String, usize, Option<String>, Option<String>, u64)],
        ) -> Result<(), StagingError> {
            for (file_path, json, record_index, partition, hash, size) in records {
                self.client
                    .execute(
                        "INSERT INTO staged_json (file_path, record_index, partition_key, raw_json, content_hash, file_size_bytes)
                         VALUES ($1, $2, $3, $4::jsonb, $5, $6)",
                        &[
                            file_path,
                            &(*record_index as i32),
                            &partition.as_deref(),
                            json,
                            &hash.as_deref(),
                            &(*size as i64),
                        ],
                    )
                    .await
                    .map_err(|e| StagingError::Database(e.to_string()))?;
            }
            Ok(())
        }

        /// Create a new processing batch
        pub async fn create_batch(&self, batch: &ProcessingBatch) -> Result<(), StagingError> {
            self.client
                .execute(
                    "INSERT INTO processing_batches
                     (id, source_path, source_type, partition_key, pattern, status,
                      files_total, files_processed, files_skipped, records_ingested,
                      bytes_processed, errors_count, started_at, updated_at)
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
                    &[
                        &batch.id,
                        &batch.source_path,
                        &batch.source_type,
                        &batch.partition_key.as_deref(),
                        &batch.pattern,
                        &batch.status.to_string(),
                        &batch.files_total,
                        &batch.files_processed,
                        &batch.files_skipped,
                        &batch.records_ingested,
                        &batch.bytes_processed,
                        &batch.errors_count,
                        &batch.started_at.map(|t| t.to_rfc3339()),
                        &batch.updated_at.map(|t| t.to_rfc3339()),
                    ],
                )
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;
            Ok(())
        }

        /// Update a processing batch
        pub async fn update_batch(&self, batch: &ProcessingBatch) -> Result<(), StagingError> {
            self.client
                .execute(
                    "UPDATE processing_batches SET
                     status = $2, files_total = $3, files_processed = $4, files_skipped = $5,
                     records_ingested = $6, bytes_processed = $7, errors_count = $8,
                     last_file_path = $9, last_record_index = $10,
                     updated_at = $11, completed_at = $12, error_message = $13
                     WHERE id = $1",
                    &[
                        &batch.id,
                        &batch.status.to_string(),
                        &batch.files_total,
                        &batch.files_processed,
                        &batch.files_skipped,
                        &batch.records_ingested,
                        &batch.bytes_processed,
                        &batch.errors_count,
                        &batch.last_file_path.as_deref(),
                        &batch.last_record_index,
                        &batch.updated_at.map(|t| t.to_rfc3339()),
                        &batch.completed_at.map(|t| t.to_rfc3339()),
                        &batch.error_message.as_deref(),
                    ],
                )
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;
            Ok(())
        }

        /// Get a batch by ID
        pub async fn get_batch(
            &self,
            batch_id: &str,
        ) -> Result<Option<ProcessingBatch>, StagingError> {
            let row = self
                .client
                .query_opt(
                    "SELECT id, source_path, source_type, partition_key, pattern, status,
                            files_total, files_processed, files_skipped, records_ingested,
                            bytes_processed, errors_count, last_file_path, last_record_index,
                            started_at, updated_at, completed_at, error_message
                     FROM processing_batches WHERE id = $1",
                    &[&batch_id],
                )
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(row.map(|r| ProcessingBatch {
                id: r.get(0),
                source_path: r.get(1),
                source_type: r.get(2),
                partition_key: r.get(3),
                pattern: r.get(4),
                status: r
                    .get::<_, String>(5)
                    .parse()
                    .unwrap_or(BatchStatus::Running),
                files_total: r.get(6),
                files_processed: r.get(7),
                files_skipped: r.get(8),
                records_ingested: r.get(9),
                bytes_processed: r.get(10),
                errors_count: r.get(11),
                last_file_path: r.get(12),
                last_record_index: r.get(13),
                started_at: r
                    .get::<_, Option<String>>(14)
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                updated_at: r
                    .get::<_, Option<String>>(15)
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                completed_at: r
                    .get::<_, Option<String>>(16)
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
                error_message: r.get(17),
            }))
        }

        /// List recent batches
        pub async fn list_batches(
            &self,
            limit: usize,
        ) -> Result<Vec<ProcessingBatch>, StagingError> {
            let rows = self
                .client
                .query(
                    "SELECT id, source_path, source_type, partition_key, pattern, status,
                            files_total, files_processed, files_skipped, records_ingested,
                            bytes_processed, errors_count, last_file_path, last_record_index,
                            started_at, updated_at, completed_at, error_message
                     FROM processing_batches
                     ORDER BY started_at DESC
                     LIMIT $1",
                    &[&(limit as i64)],
                )
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(rows
                .iter()
                .map(|r| ProcessingBatch {
                    id: r.get(0),
                    source_path: r.get(1),
                    source_type: r.get(2),
                    partition_key: r.get(3),
                    pattern: r.get(4),
                    status: r
                        .get::<_, String>(5)
                        .parse()
                        .unwrap_or(BatchStatus::Running),
                    files_total: r.get(6),
                    files_processed: r.get(7),
                    files_skipped: r.get(8),
                    records_ingested: r.get(9),
                    bytes_processed: r.get(10),
                    errors_count: r.get(11),
                    last_file_path: r.get(12),
                    last_record_index: r.get(13),
                    started_at: r
                        .get::<_, Option<String>>(14)
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    updated_at: r
                        .get::<_, Option<String>>(15)
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    completed_at: r
                        .get::<_, Option<String>>(16)
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    error_message: r.get(17),
                })
                .collect())
        }

        /// Get partition statistics
        pub async fn partition_stats(&self) -> Result<Vec<(String, i64)>, StagingError> {
            let rows = self
                .client
                .query(
                    "SELECT COALESCE(partition_key, '<none>'), COUNT(*)
                     FROM staged_json
                     GROUP BY partition_key
                     ORDER BY COUNT(*) DESC",
                    &[],
                )
                .await
                .map_err(|e| StagingError::Database(e.to_string()))?;

            Ok(rows.iter().map(|r| (r.get(0), r.get(1))).collect())
        }

        /// Ingest files from the configured source
        pub async fn ingest(&self, config: &IngestConfig) -> Result<IngestStats, IngestError> {
            let start = Instant::now();
            let mut stats = IngestStats::new();

            // Check if database is initialized
            if !self.is_initialized().await? {
                return Err(IngestError::Staging(StagingError::NotInitialized));
            }

            // Create or resume batch
            let batch_id = config
                .batch_id
                .clone()
                .unwrap_or_else(ProcessingBatch::generate_id);

            let source_type_str = match &config.source {
                SourceType::Local(_) => "local",
                #[cfg(feature = "s3")]
                SourceType::S3 { .. } => "s3",
                #[cfg(feature = "databricks")]
                SourceType::UnityVolume { .. } => "unity_volume",
            };

            let mut batch = if config.resume {
                match self.get_batch(&batch_id).await? {
                    Some(b) if b.can_resume() => b,
                    Some(_) => return Err(IngestError::BatchCompleted(batch_id)),
                    None => return Err(IngestError::BatchNotFound(batch_id)),
                }
            } else {
                let b = ProcessingBatch::new(
                    batch_id,
                    config.source.display(),
                    source_type_str.to_string(),
                    config.partition.clone(),
                    config.pattern.clone(),
                );
                self.create_batch(&b).await?;
                b
            };

            // Discover files based on source type
            let files = match &config.source {
                SourceType::Local(path) => discover_local_files(path, &config.pattern)?,
                #[cfg(feature = "s3")]
                SourceType::S3 { .. } => {
                    return Err(IngestError::SourceNotAccessible {
                        path: config.source.display(),
                        reason: "S3 ingestion not yet implemented".to_string(),
                    });
                }
                #[cfg(feature = "databricks")]
                SourceType::UnityVolume { .. } => {
                    return Err(IngestError::SourceNotAccessible {
                        path: config.source.display(),
                        reason: "Unity Catalog ingestion not yet implemented".to_string(),
                    });
                }
            };

            batch.files_total = files.len() as i32;

            // Get existing data for deduplication
            let existing_paths =
                if matches!(config.dedup, DedupStrategy::ByPath | DedupStrategy::Both) {
                    self.get_existing_paths(config.partition.as_deref()).await?
                } else {
                    HashSet::new()
                };

            let existing_hashes =
                if matches!(config.dedup, DedupStrategy::ByContent | DedupStrategy::Both) {
                    self.get_existing_hashes(config.partition.as_deref())
                        .await?
                } else {
                    HashSet::new()
                };

            // Determine resume point
            let resume_after = if config.resume {
                batch.last_file_path.clone()
            } else {
                None
            };

            let mut records_batch: Vec<(
                String,
                String,
                usize,
                Option<String>,
                Option<String>,
                u64,
            )> = Vec::new();
            let mut past_resume_point = resume_after.is_none();

            for mut file in files {
                let file_path_str = file.path.display().to_string();

                // Skip files before resume point
                if !past_resume_point {
                    if Some(&file_path_str) == resume_after.as_ref() {
                        past_resume_point = true;
                    }
                    continue;
                }

                // Compute hash if needed for dedup
                if matches!(config.dedup, DedupStrategy::ByContent | DedupStrategy::Both) {
                    if let Err(e) = file.compute_hash() {
                        stats.add_error(format!(
                            "Error computing hash for {}: {}",
                            file_path_str, e
                        ));
                        continue;
                    }
                }

                // Check deduplication
                if should_skip_file(&file, config.dedup, &existing_paths, &existing_hashes) {
                    stats.files_skipped += 1;
                    batch.files_skipped += 1;
                    continue;
                }

                // Parse the file
                let records = match parse_file(&file.path) {
                    Ok(r) => r,
                    Err(e) => {
                        stats.add_error(format!("Error parsing {}: {}", file_path_str, e));
                        batch.increment_errors();
                        continue;
                    }
                };

                // Add records to batch
                for record in records {
                    records_batch.push((
                        file_path_str.clone(),
                        record.json,
                        record.index,
                        config.partition.clone(),
                        file.content_hash.clone(),
                        file.size,
                    ));

                    // Insert batch when full
                    if records_batch.len() >= config.batch_size {
                        self.insert_records(&records_batch).await?;
                        stats.records_ingested += records_batch.len();
                        batch.records_ingested += records_batch.len() as i64;
                        records_batch.clear();
                    }
                }

                stats.files_processed += 1;
                stats.bytes_processed += file.size;
                batch.files_processed += 1;
                batch.bytes_processed += file.size as i64;
                batch.last_file_path = Some(file_path_str);

                // Update batch progress periodically
                if batch.files_processed % 100 == 0 {
                    self.update_batch(&batch).await?;
                }
            }

            // Insert remaining records
            if !records_batch.is_empty() {
                self.insert_records(&records_batch).await?;
                stats.records_ingested += records_batch.len();
                batch.records_ingested += records_batch.len() as i64;
            }

            // Complete batch
            batch.complete();
            self.update_batch(&batch).await?;

            stats.duration = start.elapsed();
            Ok(stats)
        }
    }
}

#[cfg(test)]
#[cfg(feature = "duckdb-backend")]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_staging_db_init() {
        let db = StagingDb::memory().unwrap();
        assert!(!db.is_initialized().unwrap());

        db.init().unwrap();
        assert!(db.is_initialized().unwrap());
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
    }

    #[test]
    fn test_staging_db_record_count() {
        let db = StagingDb::memory().unwrap();
        db.init().unwrap();

        assert_eq!(db.record_count(None).unwrap(), 0);
    }

    #[test]
    fn test_staging_db_ingest_local() {
        let dir = TempDir::new().unwrap();

        // Create test files
        let file1 = dir.path().join("test1.json");
        let mut f1 = File::create(&file1).unwrap();
        writeln!(f1, r#"{{"name": "test1", "value": 1}}"#).unwrap();

        let file2 = dir.path().join("test2.json");
        let mut f2 = File::create(&file2).unwrap();
        writeln!(f2, r#"{{"name": "test2", "value": 2}}"#).unwrap();

        // Create database and ingest
        let db = StagingDb::memory().unwrap();
        db.init().unwrap();

        let config = IngestConfig::builder()
            .source_type(SourceType::Local(dir.path().to_path_buf()))
            .pattern("*.json")
            .build()
            .unwrap();

        let stats = db.ingest(&config).unwrap();
        assert_eq!(stats.files_processed, 2);
        assert_eq!(stats.records_ingested, 2);
        assert_eq!(db.record_count(None).unwrap(), 2);
    }

    #[test]
    fn test_staging_db_ingest_jsonl() {
        let dir = TempDir::new().unwrap();

        // Create JSONL file
        let file = dir.path().join("data.jsonl");
        let mut f = File::create(&file).unwrap();
        writeln!(f, r#"{{"row": 1}}"#).unwrap();
        writeln!(f, r#"{{"row": 2}}"#).unwrap();
        writeln!(f, r#"{{"row": 3}}"#).unwrap();

        let db = StagingDb::memory().unwrap();
        db.init().unwrap();

        let config = IngestConfig::builder()
            .source_type(SourceType::Local(dir.path().to_path_buf()))
            .pattern("*.jsonl")
            .build()
            .unwrap();

        let stats = db.ingest(&config).unwrap();
        assert_eq!(stats.files_processed, 1);
        assert_eq!(stats.records_ingested, 3);
    }

    #[test]
    fn test_staging_db_dedup_by_path() {
        let dir = TempDir::new().unwrap();

        let file = dir.path().join("test.json");
        let mut f = File::create(&file).unwrap();
        writeln!(f, r#"{{"name": "test"}}"#).unwrap();

        let db = StagingDb::memory().unwrap();
        db.init().unwrap();

        let config = IngestConfig::builder()
            .source_type(SourceType::Local(dir.path().to_path_buf()))
            .pattern("*.json")
            .dedup(DedupStrategy::ByPath)
            .build()
            .unwrap();

        // First ingest
        let stats1 = db.ingest(&config).unwrap();
        assert_eq!(stats1.files_processed, 1);
        assert_eq!(stats1.files_skipped, 0);

        // Second ingest - should skip
        let stats2 = db.ingest(&config).unwrap();
        assert_eq!(stats2.files_processed, 0);
        assert_eq!(stats2.files_skipped, 1);
    }

    #[test]
    fn test_staging_db_batch_tracking() {
        let db = StagingDb::memory().unwrap();
        db.init().unwrap();

        let batch = ProcessingBatch::new(
            "test-batch".to_string(),
            "./data".to_string(),
            "local".to_string(),
            Some("2024-01".to_string()),
            "*.json".to_string(),
        );

        db.create_batch(&batch).unwrap();

        let retrieved = db.get_batch("test-batch").unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.source_path, "./data");
        assert_eq!(retrieved.status, BatchStatus::Running);
    }

    #[test]
    fn test_staging_db_query() {
        let db = StagingDb::memory().unwrap();
        db.init().unwrap();

        let results = db.query("SELECT 1 as num, 'hello' as str").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["num"], 1);
        assert_eq!(results[0]["str"], "hello");
    }
}
