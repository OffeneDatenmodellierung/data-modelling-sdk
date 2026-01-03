//! Export functionality
//!
//! Provides exporters for various formats:
//! - SQL
//! - JSON Schema
//! - AVRO
//! - Protobuf
//! - ODCS (Open Data Contract Standard) v3.1.0
//! - PNG

pub mod avro;
pub mod cads;
pub mod json_schema;
pub mod odcl;
pub mod odcs;
pub mod odps;
#[cfg(feature = "png-export")]
pub mod png;
pub mod protobuf;
pub mod sql;

// anyhow::Result not currently used in this module

/// Result of an export operation.
///
/// Contains the exported content and format identifier.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[must_use = "export results contain the exported content and should be used"]
pub struct ExportResult {
    /// Exported content (as string - binary formats will be base64 encoded)
    pub content: String,
    /// Format identifier
    pub format: String,
}

/// Error during export
#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
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
pub use avro::AvroExporter;
pub use cads::CADSExporter;
pub use json_schema::JSONSchemaExporter;
pub use odcl::ODCLExporter;
pub use odcs::ODCSExporter;
pub use odps::ODPSExporter;
#[cfg(feature = "png-export")]
pub use png::PNGExporter;
pub use protobuf::ProtobufExporter;
pub use sql::SQLExporter;
