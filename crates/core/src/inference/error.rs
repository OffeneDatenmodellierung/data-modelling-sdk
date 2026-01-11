//! Error types for schema inference

use thiserror::Error;

/// Errors that can occur during schema inference
#[derive(Error, Debug, Clone)]
pub enum InferenceError {
    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonParse(String),

    /// Invalid JSON structure (not an object at root)
    #[error("Invalid JSON structure: expected object at root, found {0}")]
    InvalidStructure(String),

    /// Maximum depth exceeded
    #[error("Maximum nesting depth exceeded: {depth} > {max}")]
    MaxDepthExceeded { depth: usize, max: usize },

    /// No records to infer from
    #[error("No records provided for inference")]
    NoRecords,

    /// IO error
    #[error("IO error: {0}")]
    Io(String),

    /// Staging database error
    #[error("Staging error: {0}")]
    Staging(String),
}

impl From<serde_json::Error> for InferenceError {
    fn from(e: serde_json::Error) -> Self {
        InferenceError::JsonParse(e.to_string())
    }
}

impl From<std::io::Error> for InferenceError {
    fn from(e: std::io::Error) -> Self {
        InferenceError::Io(e.to_string())
    }
}
