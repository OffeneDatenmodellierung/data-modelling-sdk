//! Error types for staging operations

#![allow(unexpected_cfgs)]

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during staging operations
#[derive(Error, Debug)]
pub enum StagingError {
    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Database not initialized
    #[error("Database not initialized. Run 'init' first.")]
    NotInitialized,

    /// Schema version mismatch
    #[error("Schema version mismatch: expected {expected}, found {found}")]
    SchemaVersionMismatch { expected: i32, found: i32 },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Query error
    #[error("Query error: {0}")]
    Query(String),
}

/// Errors that can occur during ingestion
#[derive(Error, Debug)]
pub enum IngestError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Invalid file format
    #[error("Invalid file format: {path} - {reason}")]
    InvalidFormat { path: PathBuf, reason: String },

    /// JSON parsing error for a specific file
    #[error("JSON parsing error in {path} at record {record}: {error}")]
    JsonParse {
        path: PathBuf,
        record: usize,
        error: String,
    },

    /// Database insert error
    #[error("Database insert error: {0}")]
    Insert(String),

    /// Batch not found for resume
    #[error("Batch not found: {0}")]
    BatchNotFound(String),

    /// Batch already completed
    #[error("Batch already completed: {0}")]
    BatchCompleted(String),

    /// Source not accessible
    #[error("Source not accessible: {path} - {reason}")]
    SourceNotAccessible { path: String, reason: String },

    /// Pattern matching error
    #[error("Invalid pattern: {0}")]
    InvalidPattern(String),

    /// Staging error wrapper
    #[error(transparent)]
    Staging(#[from] StagingError),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// S3 error (when s3 feature is enabled)
    #[cfg(feature = "s3")]
    #[error("S3 error: {0}")]
    S3(String),

    /// Databricks error (when databricks feature is enabled)
    #[cfg(feature = "databricks")]
    #[error("Databricks error: {0}")]
    Databricks(String),
}

impl StagingError {
    /// Get a user-friendly error message for CLI output
    pub fn user_message(&self) -> String {
        match self {
            StagingError::NotInitialized => {
                "Staging database not initialized.\n\nHint: Run 'odm staging init' first."
                    .to_string()
            }
            StagingError::SchemaVersionMismatch { expected, found } => {
                format!(
                    "Schema version mismatch (expected v{expected}, found v{found}).\n\n\
                    Hint: Run 'odm staging init --force' to reinitialize the database."
                )
            }
            StagingError::InvalidConfig(msg) => {
                format!("Invalid configuration: {msg}\n\nHint: Check your staging configuration.")
            }
            _ => self.to_string(),
        }
    }
}

impl IngestError {
    /// Get a user-friendly error message for CLI output
    pub fn user_message(&self) -> String {
        match self {
            IngestError::FileNotFound(path) => {
                format!(
                    "File not found: {}\n\nHint: Check that the file exists and the path is correct.",
                    path.display()
                )
            }
            IngestError::InvalidFormat { path, reason } => {
                format!(
                    "Invalid file format: {}\nReason: {reason}\n\nHint: Ensure the file contains valid JSON.",
                    path.display()
                )
            }
            IngestError::JsonParse {
                path,
                record,
                error,
            } => {
                format!(
                    "JSON parse error in {} at record {record}:\n{error}\n\n\
                    Hint: Check the JSON syntax around record {record}.",
                    path.display()
                )
            }
            IngestError::BatchNotFound(batch_id) => {
                format!(
                    "Batch not found: {batch_id}\n\nHint: Use 'odm staging batches' to list available batches."
                )
            }
            IngestError::InvalidPattern(pattern) => {
                format!(
                    "Invalid glob pattern: {pattern}\n\n\
                    Hint: Use standard glob syntax like '*.json' or '**/*.json'."
                )
            }
            IngestError::SourceNotAccessible { path, reason } => {
                format!(
                    "Cannot access source: {path}\nReason: {reason}\n\n\
                    Hint: Check permissions and network connectivity."
                )
            }
            _ => self.to_string(),
        }
    }
}

#[cfg(feature = "duckdb-backend")]
impl From<duckdb::Error> for StagingError {
    fn from(err: duckdb::Error) -> Self {
        StagingError::Database(err.to_string())
    }
}

#[cfg(feature = "duckdb-backend")]
impl From<duckdb::Error> for IngestError {
    fn from(err: duckdb::Error) -> Self {
        IngestError::Insert(err.to_string())
    }
}
