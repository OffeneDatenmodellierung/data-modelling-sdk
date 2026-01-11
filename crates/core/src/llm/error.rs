//! Error types for LLM operations
//!
//! This module defines error types for LLM-based schema refinement operations,
//! including connection errors, model loading errors, and validation failures.

use thiserror::Error;

/// Errors that can occur during LLM operations
#[derive(Error, Debug)]
pub enum LlmError {
    /// Failed to connect to LLM service
    #[error("Failed to connect to LLM service: {0}")]
    ConnectionError(String),

    /// Request timeout
    #[error("LLM request timed out after {0} seconds")]
    Timeout(u64),

    /// Model not found or failed to load
    #[error("Model error: {0}")]
    ModelError(String),

    /// Invalid response from LLM
    #[error("Invalid LLM response: {0}")]
    InvalidResponse(String),

    /// Failed to parse LLM output as JSON
    #[error("Failed to parse LLM output as JSON: {0}")]
    ParseError(String),

    /// Validation failed for LLM output
    #[error("LLM output validation failed: {0}")]
    ValidationError(String),

    /// Schema refinement produced invalid changes
    #[error("Invalid schema refinement: {0}")]
    RefinementError(String),

    /// Maximum retries exceeded
    #[error("Maximum retries ({0}) exceeded")]
    MaxRetriesExceeded(usize),

    /// Documentation loading error
    #[error("Failed to load documentation: {0}")]
    DocumentationError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(String),

    /// Rate limiting
    #[error("Rate limited by LLM service, retry after {0} seconds")]
    RateLimited(u64),

    /// Context too large
    #[error("Context exceeds maximum tokens ({max}): {actual} tokens")]
    ContextTooLarge { max: usize, actual: usize },

    /// Feature not available
    #[error("LLM feature not available: {0}. Enable with --features {1}")]
    FeatureNotAvailable(String, String),
}

impl From<std::io::Error> for LlmError {
    fn from(err: std::io::Error) -> Self {
        LlmError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for LlmError {
    fn from(err: serde_json::Error) -> Self {
        LlmError::ParseError(err.to_string())
    }
}

/// Result type for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

impl LlmError {
    /// Get a user-friendly error message for CLI output
    pub fn user_message(&self) -> String {
        match self {
            LlmError::ConnectionError(msg) => {
                format!(
                    "Failed to connect to LLM service: {msg}\n\n\
                    Hints:\n\
                    - Check your internet connection\n\
                    - Verify the API endpoint is correct\n\
                    - For Ollama: ensure 'ollama serve' is running"
                )
            }
            LlmError::Timeout(secs) => {
                format!(
                    "LLM request timed out after {secs} seconds.\n\n\
                    Hints:\n\
                    - The model may be overloaded, try again later\n\
                    - Consider using a smaller/faster model\n\
                    - Increase timeout with --timeout flag"
                )
            }
            LlmError::RateLimited(secs) => {
                format!(
                    "Rate limited by LLM service. Retry after {secs} seconds.\n\n\
                    Hint: Wait and try again, or use a different API key."
                )
            }
            LlmError::ContextTooLarge { max, actual } => {
                format!(
                    "Schema too large for LLM context ({actual} tokens, max {max}).\n\n\
                    Hints:\n\
                    - Use --sample-size to reduce the number of records\n\
                    - Process a subset of fields\n\
                    - Use a model with larger context window"
                )
            }
            LlmError::ConfigError(msg) => {
                format!(
                    "LLM configuration error: {msg}\n\n\
                    Hints:\n\
                    - Set OPENAI_API_KEY for OpenAI\n\
                    - Set ANTHROPIC_API_KEY for Anthropic\n\
                    - Use --provider ollama for local models"
                )
            }
            LlmError::FeatureNotAvailable(feature, flag) => {
                format!(
                    "LLM feature '{feature}' not available.\n\n\
                    Hint: Rebuild with --features {flag}"
                )
            }
            LlmError::MaxRetriesExceeded(retries) => {
                format!(
                    "Failed after {retries} retries.\n\n\
                    Hints:\n\
                    - Check your network connection\n\
                    - The LLM service may be experiencing issues\n\
                    - Try again later"
                )
            }
            _ => self.to_string(),
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            LlmError::ConnectionError(_) | LlmError::Timeout(_) | LlmError::RateLimited(_)
        )
    }

    /// Get suggested wait time before retry (in seconds)
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            LlmError::RateLimited(secs) => Some(*secs),
            LlmError::Timeout(_) => Some(5),
            LlmError::ConnectionError(_) => Some(2),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = LlmError::ConnectionError("Connection refused".to_string());
        assert_eq!(
            err.to_string(),
            "Failed to connect to LLM service: Connection refused"
        );

        let err = LlmError::Timeout(30);
        assert_eq!(err.to_string(), "LLM request timed out after 30 seconds");

        let err = LlmError::MaxRetriesExceeded(3);
        assert_eq!(err.to_string(), "Maximum retries (3) exceeded");

        let err = LlmError::ContextTooLarge {
            max: 4096,
            actual: 8000,
        };
        assert_eq!(
            err.to_string(),
            "Context exceeds maximum tokens (4096): 8000 tokens"
        );
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let llm_err: LlmError = io_err.into();
        assert!(matches!(llm_err, LlmError::IoError(_)));
    }

    #[test]
    fn test_error_from_serde() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let llm_err: LlmError = json_err.into();
        assert!(matches!(llm_err, LlmError::ParseError(_)));
    }
}
