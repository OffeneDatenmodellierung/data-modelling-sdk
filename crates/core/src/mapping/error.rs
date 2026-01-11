//! Error types for schema mapping operations

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during schema mapping
#[derive(Error, Debug)]
pub enum MappingError {
    /// Failed to read schema file
    #[error("Failed to read schema file: {path}")]
    SchemaReadError {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse schema
    #[error("Failed to parse schema: {0}")]
    SchemaParseError(String),

    /// Invalid schema structure
    #[error("Invalid schema structure: {0}")]
    InvalidSchema(String),

    /// No mappings found
    #[error("No field mappings could be determined between source and target schemas")]
    NoMappingsFound,

    /// Incompatible schemas
    #[error("Schemas are incompatible: {0}")]
    IncompatibleSchemas(String),

    /// Transform generation failed
    #[error("Failed to generate transformation: {0}")]
    TransformGenerationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// LLM error
    #[error("LLM error: {0}")]
    LlmError(String),
}

/// Result type for mapping operations
pub type MappingResult<T> = Result<T, MappingError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MappingError::NoMappingsFound;
        assert!(err.to_string().contains("No field mappings"));

        let err = MappingError::InvalidSchema("missing properties".to_string());
        assert!(err.to_string().contains("missing properties"));
    }
}
