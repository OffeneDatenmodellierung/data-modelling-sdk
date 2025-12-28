//! Import functionality
//! 
//! Provides parsers for importing data models from various formats:
//! - SQL (CREATE TABLE statements)
//! - ODCS (Open Data Contract Standard) v3.1.0 YAML format (legacy ODCL formats supported for import)
//! - JSON Schema
//! - AVRO
//! - Protobuf

pub mod sql;
pub mod odcs;
pub mod json_schema;
pub mod avro;
pub mod protobuf;

// anyhow::Result not currently used in this module

/// Result of an import operation
#[derive(Debug)]
pub struct ImportResult {
    /// Tables extracted from the import
    pub tables: Vec<TableData>,
    /// Tables that require name input (for SQL imports with unnamed tables)
    pub tables_requiring_name: Vec<TableRequiringName>,
    /// Parse errors/warnings
    pub errors: Vec<ImportError>,
    /// Whether AI suggestions are available
    pub ai_suggestions: Option<Vec<serde_json::Value>>,
}

/// Error during import
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
}

/// Table data from import
#[derive(Debug, Clone)]
pub struct TableData {
    pub table_index: usize,
    pub name: Option<String>,
    pub columns: Vec<ColumnData>,
    // Additional fields can be added as needed
}

/// Column data from import
#[derive(Debug, Clone)]
pub struct ColumnData {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub primary_key: bool,
}

// Re-export for convenience
pub use sql::SQLImporter;
pub use odcs::ODCSImporter;
pub use json_schema::JSONSchemaImporter;
pub use avro::AvroImporter;
pub use protobuf::ProtobufImporter;

/// Table requiring name input (for SQL imports)
#[derive(Debug, Clone)]
pub struct TableRequiringName {
    pub table_index: usize,
    pub suggested_name: Option<String>,
}
