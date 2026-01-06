//! Import functionality
//!
//! Provides parsers for importing data models from various formats:
//! - SQL (CREATE TABLE statements)
//! - ODCS (Open Data Contract Standard) v3.1.0 YAML format (legacy ODCL formats supported for import)
//! - JSON Schema
//! - AVRO
//! - Protobuf

pub mod avro;
#[cfg(feature = "bpmn")]
pub mod bpmn;
pub mod cads;
#[cfg(feature = "dmn")]
pub mod dmn;
pub mod json_schema;
pub mod odcl;
pub mod odcs;
pub mod odcs_shared;
pub mod odps;
#[cfg(feature = "openapi")]
pub mod openapi;
pub mod protobuf;
pub mod sql;

// anyhow::Result not currently used in this module

/// Result of an import operation.
///
/// Contains extracted tables and any errors/warnings from the import process.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[must_use = "import results should be processed or errors checked"]
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
#[derive(Debug, thiserror::Error, serde::Serialize, serde::Deserialize)]
pub enum ImportError {
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("BPMN validation error: {0}")]
    BPMNValidationError(String),
    #[error("DMN validation error: {0}")]
    DMNValidationError(String),
    #[error("OpenAPI validation error: {0}")]
    OpenAPIValidationError(String),
    #[error("BPMN parse error: {0}")]
    BPMNParseError(String),
    #[error("DMN parse error: {0}")]
    DMNParseError(String),
    #[error("OpenAPI parse error: {0}")]
    OpenAPIParseError(String),
}

/// Table data from import
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TableData {
    pub table_index: usize,
    pub name: Option<String>,
    pub columns: Vec<ColumnData>,
    // Additional fields can be added as needed
}

/// Column data from import
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ColumnData {
    pub name: String,
    pub data_type: String,
    /// Physical type - the actual database type (e.g., "DOUBLE", "VARCHAR(100)")
    /// For ODCS this maps to physicalType. Optional as not all formats distinguish logical/physical types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_type: Option<String>,
    pub nullable: bool,
    pub primary_key: bool,
    /// Column description/documentation (from ODCS/ODCL description field)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Quality rules and validation checks (from ODCS/ODCL quality array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Vec<std::collections::HashMap<String, serde_json::Value>>>,
    /// ODCS v3.1.0 relationships (property-level references)
    /// All $ref values are converted to relationships on import
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub relationships: Vec<crate::models::PropertyRelationship>,
    /// Enum values if this column is an enumeration type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

// Re-export for convenience
pub use avro::AvroImporter;
pub use cads::CADSImporter;
pub use json_schema::JSONSchemaImporter;
pub use odcl::ODCLImporter;
pub use odcs::ODCSImporter;
pub use odcs_shared::ParserError;
pub use odps::ODPSImporter;
pub use protobuf::ProtobufImporter;
pub use sql::SQLImporter;

/// Table requiring name input (for SQL imports)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TableRequiringName {
    pub table_index: usize,
    pub suggested_name: Option<String>,
}
