//! Error types for pipeline operations
//!
//! This module provides error types for the data modeling pipeline.
//! Errors are designed to chain properly for debugging while providing
//! user-friendly messages for CLI output.

use std::path::PathBuf;
use thiserror::Error;

#[cfg(feature = "inference")]
use crate::inference::InferenceError;
#[cfg(feature = "mapping")]
use crate::mapping::MappingError;
#[cfg(feature = "staging")]
use crate::staging::StagingError;

/// Errors that can occur during pipeline execution
#[derive(Error, Debug)]
pub enum PipelineError {
    /// Pipeline configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Stage execution failed with context
    #[error("Stage '{stage}' failed: {message}")]
    StageError { stage: String, message: String },

    /// Stage failed with underlying cause
    #[error("Stage '{stage}' failed")]
    StageFailure {
        stage: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Missing required input
    #[error("Missing required input: {0}")]
    MissingInput(String),

    /// Invalid stage specification
    #[error("Invalid stage: {0}")]
    InvalidStage(String),

    /// Checkpoint error
    #[error("Checkpoint error: {0}")]
    CheckpointError(String),

    /// Resume error
    #[error("Cannot resume from checkpoint: {0}")]
    ResumeError(String),

    /// IO error with path context
    #[error("IO error with {path}: {message}")]
    IoErrorWithPath {
        path: PathBuf,
        message: String,
        #[source]
        source: std::io::Error,
    },

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Staging error (wrapped)
    #[error("Staging error: {0}")]
    StagingError(String),

    /// Inference error (wrapped)
    #[error("Inference error: {0}")]
    InferenceError(String),

    /// Mapping error (wrapped)
    #[error("Mapping error: {0}")]
    MappingError(String),

    /// Export error
    #[error("Export error: {0}")]
    ExportError(String),

    /// LLM error
    #[error("LLM error: {0}")]
    LlmError(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Pipeline cancelled
    #[error("Pipeline cancelled by user")]
    Cancelled,

    /// Multiple errors occurred
    #[error("Multiple errors occurred: {}", .0.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("; "))]
    Multiple(Vec<PipelineError>),
}

/// Result type for pipeline operations
pub type PipelineResult<T> = Result<T, PipelineError>;

impl PipelineError {
    /// Create a stage error with message
    pub fn stage(stage: impl Into<String>, message: impl Into<String>) -> Self {
        Self::StageError {
            stage: stage.into(),
            message: message.into(),
        }
    }

    /// Create a stage failure with underlying error
    pub fn stage_failure<E>(stage: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::StageFailure {
            stage: stage.into(),
            source: Box::new(source),
        }
    }

    /// Create an IO error with path context
    pub fn io_with_path(
        path: impl Into<PathBuf>,
        message: impl Into<String>,
        source: std::io::Error,
    ) -> Self {
        Self::IoErrorWithPath {
            path: path.into(),
            message: message.into(),
            source,
        }
    }

    /// Check if this error is recoverable (can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            PipelineError::IoError(_) | PipelineError::IoErrorWithPath { .. }
        )
    }

    /// Get the stage name if this is a stage error
    pub fn stage_name(&self) -> Option<&str> {
        match self {
            PipelineError::StageError { stage, .. } => Some(stage),
            PipelineError::StageFailure { stage, .. } => Some(stage),
            _ => None,
        }
    }

    /// Get a user-friendly error message for CLI output
    pub fn user_message(&self) -> String {
        match self {
            PipelineError::ConfigError(msg) => {
                format!(
                    "Configuration error: {msg}\n\nHint: Check your pipeline configuration file."
                )
            }
            PipelineError::StageError { stage, message } => {
                format!("Stage '{stage}' failed: {message}")
            }
            PipelineError::StageFailure { stage, source } => {
                format!("Stage '{stage}' failed: {source}")
            }
            PipelineError::MissingInput(input) => {
                format!(
                    "Missing required input: {input}\n\nHint: Ensure all required files exist and paths are correct."
                )
            }
            PipelineError::FileNotFound(path) => {
                format!(
                    "File not found: {}\n\nHint: Check that the file exists and the path is correct.",
                    path.display()
                )
            }
            PipelineError::CheckpointError(msg) => {
                format!(
                    "Checkpoint error: {msg}\n\nHint: Try running with --force to ignore checkpoints."
                )
            }
            PipelineError::ResumeError(msg) => {
                format!(
                    "Cannot resume: {msg}\n\nHint: Run the pipeline from the beginning with --force."
                )
            }
            PipelineError::Cancelled => "Pipeline cancelled by user.".to_string(),
            _ => self.to_string(),
        }
    }
}

// Feature-gated From implementations for sub-module errors
#[cfg(feature = "staging")]
impl From<StagingError> for PipelineError {
    fn from(err: StagingError) -> Self {
        PipelineError::StagingError(err.to_string())
    }
}

#[cfg(feature = "inference")]
impl From<InferenceError> for PipelineError {
    fn from(err: InferenceError) -> Self {
        PipelineError::InferenceError(err.to_string())
    }
}

#[cfg(feature = "mapping")]
impl From<MappingError> for PipelineError {
    fn from(err: MappingError) -> Self {
        PipelineError::MappingError(err.to_string())
    }
}

#[cfg(feature = "staging")]
impl From<crate::staging::IngestError> for PipelineError {
    fn from(err: crate::staging::IngestError) -> Self {
        PipelineError::StagingError(err.to_string())
    }
}

#[cfg(feature = "llm")]
impl From<crate::llm::LlmError> for PipelineError {
    fn from(err: crate::llm::LlmError) -> Self {
        PipelineError::LlmError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = PipelineError::stage("ingest", "Database connection failed");
        assert!(err.to_string().contains("ingest"));
        assert!(err.to_string().contains("Database connection failed"));

        let err = PipelineError::MissingInput("source path".to_string());
        assert!(err.to_string().contains("source path"));
    }

    #[test]
    fn test_stage_failure() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = PipelineError::stage_failure("ingest", io_err);
        assert!(err.to_string().contains("ingest"));
        assert!(err.stage_name() == Some("ingest"));
    }

    #[test]
    fn test_io_with_path() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = PipelineError::io_with_path("/path/to/file", "reading config", io_err);
        let display = err.to_string();
        assert!(display.contains("/path/to/file"));
        assert!(display.contains("reading config"));
    }

    #[test]
    fn test_is_recoverable() {
        let io_err =
            PipelineError::IoError(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"));
        assert!(io_err.is_recoverable());

        let config_err = PipelineError::ConfigError("bad config".to_string());
        assert!(!config_err.is_recoverable());
    }

    #[test]
    fn test_user_message() {
        let err = PipelineError::MissingInput("source.json".to_string());
        let msg = err.user_message();
        assert!(msg.contains("source.json"));
        assert!(msg.contains("Hint:"));

        let err = PipelineError::FileNotFound(PathBuf::from("/data/input.json"));
        let msg = err.user_message();
        assert!(msg.contains("/data/input.json"));
        assert!(msg.contains("Hint:"));
    }

    #[test]
    fn test_multiple_errors() {
        let errors = vec![
            PipelineError::ConfigError("error 1".to_string()),
            PipelineError::ConfigError("error 2".to_string()),
        ];
        let err = PipelineError::Multiple(errors);
        let display = err.to_string();
        assert!(display.contains("error 1"));
        assert!(display.contains("error 2"));
    }
}
