//! Database schema definitions for staging tables

/// Current schema version
pub const SCHEMA_VERSION: i32 = 1;

/// Schema for staging database tables
pub struct StagingSchema;

impl StagingSchema {
    /// Get the DDL for creating all staging tables (DuckDB syntax)
    #[cfg(feature = "duckdb-backend")]
    pub fn create_tables_duckdb() -> &'static str {
        r#"
-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_info (
    key VARCHAR PRIMARY KEY,
    value VARCHAR NOT NULL
);

-- Core staging table for raw JSON records
CREATE TABLE IF NOT EXISTS staged_json (
    id BIGINT PRIMARY KEY,
    file_path VARCHAR NOT NULL,
    record_index INTEGER NOT NULL,
    partition_key VARCHAR,
    raw_json JSON NOT NULL,
    content_hash VARCHAR,
    file_size_bytes BIGINT,
    ingested_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(file_path, record_index)
);

-- Batch tracking for resume support
CREATE TABLE IF NOT EXISTS processing_batches (
    id VARCHAR PRIMARY KEY,
    source_path VARCHAR NOT NULL,
    source_type VARCHAR NOT NULL,
    partition_key VARCHAR,
    pattern VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
    files_total INTEGER DEFAULT 0,
    files_processed INTEGER DEFAULT 0,
    files_skipped INTEGER DEFAULT 0,
    records_ingested BIGINT DEFAULT 0,
    bytes_processed BIGINT DEFAULT 0,
    errors_count INTEGER DEFAULT 0,
    last_file_path VARCHAR,
    last_record_index INTEGER,
    started_at VARCHAR,
    updated_at VARCHAR,
    completed_at VARCHAR,
    error_message VARCHAR
);

-- Inferred schemas storage
CREATE TABLE IF NOT EXISTS inferred_schemas (
    id VARCHAR PRIMARY KEY,
    schema_name VARCHAR NOT NULL,
    partition_key VARCHAR,
    schema_json JSON NOT NULL,
    sample_count INTEGER,
    version INTEGER DEFAULT 1,
    parent_id VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_staged_partition ON staged_json(partition_key);
CREATE INDEX IF NOT EXISTS idx_staged_file ON staged_json(file_path);
CREATE INDEX IF NOT EXISTS idx_staged_hash ON staged_json(content_hash);
CREATE INDEX IF NOT EXISTS idx_batches_status ON processing_batches(status);
CREATE INDEX IF NOT EXISTS idx_schemas_partition ON inferred_schemas(partition_key);

-- Create sequence for staged_json IDs
CREATE SEQUENCE IF NOT EXISTS staged_json_id_seq START 1;
"#
    }

    /// Get the DDL for creating all staging tables (PostgreSQL syntax)
    #[cfg(feature = "postgres-backend")]
    pub fn create_tables_postgres() -> &'static str {
        r#"
-- Schema version tracking
CREATE TABLE IF NOT EXISTS schema_info (
    key VARCHAR PRIMARY KEY,
    value VARCHAR NOT NULL
);

-- Core staging table for raw JSON records
CREATE TABLE IF NOT EXISTS staged_json (
    id BIGSERIAL PRIMARY KEY,
    file_path VARCHAR NOT NULL,
    record_index INTEGER NOT NULL,
    partition_key VARCHAR,
    raw_json JSONB NOT NULL,
    content_hash VARCHAR,
    file_size_bytes BIGINT,
    ingested_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(file_path, record_index)
);

-- Batch tracking for resume support
CREATE TABLE IF NOT EXISTS processing_batches (
    id VARCHAR PRIMARY KEY,
    source_path VARCHAR NOT NULL,
    source_type VARCHAR NOT NULL,
    partition_key VARCHAR,
    pattern VARCHAR NOT NULL,
    status VARCHAR NOT NULL,
    files_total INTEGER DEFAULT 0,
    files_processed INTEGER DEFAULT 0,
    files_skipped INTEGER DEFAULT 0,
    records_ingested BIGINT DEFAULT 0,
    bytes_processed BIGINT DEFAULT 0,
    errors_count INTEGER DEFAULT 0,
    last_file_path VARCHAR,
    last_record_index INTEGER,
    started_at VARCHAR,
    updated_at VARCHAR,
    completed_at VARCHAR,
    error_message VARCHAR
);

-- Inferred schemas storage
CREATE TABLE IF NOT EXISTS inferred_schemas (
    id VARCHAR PRIMARY KEY,
    schema_name VARCHAR NOT NULL,
    partition_key VARCHAR,
    schema_json JSONB NOT NULL,
    sample_count INTEGER,
    version INTEGER DEFAULT 1,
    parent_id VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes for performance
CREATE INDEX IF NOT EXISTS idx_staged_partition ON staged_json(partition_key);
CREATE INDEX IF NOT EXISTS idx_staged_file ON staged_json(file_path);
CREATE INDEX IF NOT EXISTS idx_staged_hash ON staged_json(content_hash);
CREATE INDEX IF NOT EXISTS idx_batches_status ON processing_batches(status);
CREATE INDEX IF NOT EXISTS idx_schemas_partition ON inferred_schemas(partition_key);
"#
    }

    /// Get the INSERT statement for schema version
    pub fn insert_schema_version() -> &'static str {
        "INSERT INTO schema_info (key, value) VALUES ('version', $1) ON CONFLICT (key) DO UPDATE SET value = $1"
    }

    /// Get the SELECT statement for schema version
    pub fn select_schema_version() -> &'static str {
        "SELECT value FROM schema_info WHERE key = 'version'"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_version() {
        assert!(SCHEMA_VERSION >= 1);
    }

    #[cfg(feature = "duckdb-backend")]
    #[test]
    fn test_duckdb_schema_contains_tables() {
        let ddl = StagingSchema::create_tables_duckdb();
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS staged_json"));
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS processing_batches"));
        assert!(ddl.contains("CREATE TABLE IF NOT EXISTS inferred_schemas"));
        assert!(ddl.contains("CREATE INDEX IF NOT EXISTS idx_staged_partition"));
    }
}
