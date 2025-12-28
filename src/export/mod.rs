//! Export functionality
//! 
//! Provides exporters for various formats:
//! - SQL
//! - JSON Schema
//! - AVRO
//! - Protobuf
//! - ODCS (Open Data Contract Standard) v3.1.0
//! - PNG

pub mod sql;
pub mod json_schema;
pub mod avro;
pub mod protobuf;
pub mod odcs;
#[cfg(feature = "png-export")]
pub mod png;

// anyhow::Result not currently used in this module

/// Result of an export operation
#[derive(Debug)]
pub struct ExportResult {
    /// Exported content (as string - binary formats will be base64 encoded)
    pub content: String,
    /// Format identifier
    pub format: String,
}

/// Error during export
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Export error: {0}")]
    ExportError(String),
}

impl From<Box<dyn std::error::Error>> for ExportError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        ExportError::ExportError(err.to_string())
    }
}

// Re-export for convenience
pub use sql::SQLExporter;
pub use json_schema::JSONSchemaExporter;
pub use avro::AvroExporter;
pub use protobuf::ProtobufExporter;
pub use odcs::ODCSExporter;
#[cfg(feature = "png-export")]
pub use png::PNGExporter;
