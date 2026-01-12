//! LLM client trait and implementations
//!
//! This module defines the `LlmClient` trait for interacting with LLMs,
//! along with implementations for Ollama (online) and llama.cpp (offline).

use async_trait::async_trait;

#[cfg(test)]
use super::error::LlmError;
use super::error::LlmResult;

/// Trait for LLM client implementations
///
/// This trait provides a unified interface for different LLM backends,
/// allowing schema refinement to work with both online (Ollama) and
/// offline (llama.cpp) models.
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Generate a completion for the given prompt
    ///
    /// # Arguments
    /// * `prompt` - The input prompt for the LLM
    ///
    /// # Returns
    /// The generated text response
    async fn complete(&self, prompt: &str) -> LlmResult<String>;

    /// Get the model name being used
    fn model_name(&self) -> &str;

    /// Get the maximum context size in tokens
    fn max_tokens(&self) -> usize;

    /// Check if the client is ready and connected
    async fn is_ready(&self) -> bool;
}

/// Response from an LLM completion request
#[derive(Debug, Clone)]
pub struct CompletionResponse {
    /// The generated text
    pub text: String,
    /// Number of tokens in the prompt
    pub prompt_tokens: Option<usize>,
    /// Number of tokens generated
    pub completion_tokens: Option<usize>,
    /// Time taken for completion in milliseconds
    pub duration_ms: Option<u64>,
}

impl CompletionResponse {
    /// Create a new completion response
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            prompt_tokens: None,
            completion_tokens: None,
            duration_ms: None,
        }
    }

    /// Set token counts
    pub fn with_tokens(mut self, prompt: usize, completion: usize) -> Self {
        self.prompt_tokens = Some(prompt);
        self.completion_tokens = Some(completion);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }
}

/// A mock LLM client for testing
#[cfg(test)]
pub struct MockLlmClient {
    response: String,
    model: String,
    max_tokens: usize,
    should_fail: bool,
}

#[cfg(test)]
impl MockLlmClient {
    /// Create a new mock client that returns the given response
    pub fn new(response: impl Into<String>) -> Self {
        Self {
            response: response.into(),
            model: "mock-model".to_string(),
            max_tokens: 4096,
            should_fail: false,
        }
    }

    /// Create a mock client that fails
    pub fn failing() -> Self {
        Self {
            response: String::new(),
            model: "mock-model".to_string(),
            max_tokens: 4096,
            should_fail: true,
        }
    }
}

#[cfg(test)]
#[async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _prompt: &str) -> LlmResult<String> {
        if self.should_fail {
            Err(LlmError::ConnectionError("Mock failure".to_string()))
        } else {
            Ok(self.response.clone())
        }
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn max_tokens(&self) -> usize {
        self.max_tokens
    }

    async fn is_ready(&self) -> bool {
        !self.should_fail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_success() {
        let client = MockLlmClient::new("Test response");
        assert!(client.is_ready().await);
        assert_eq!(client.model_name(), "mock-model");
        assert_eq!(client.max_tokens(), 4096);

        let response = client.complete("Test prompt").await.unwrap();
        assert_eq!(response, "Test response");
    }

    #[tokio::test]
    async fn test_mock_client_failure() {
        let client = MockLlmClient::failing();
        assert!(!client.is_ready().await);

        let result = client.complete("Test prompt").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_completion_response() {
        let response = CompletionResponse::new("Generated text")
            .with_tokens(100, 50)
            .with_duration(1500);

        assert_eq!(response.text, "Generated text");
        assert_eq!(response.prompt_tokens, Some(100));
        assert_eq!(response.completion_tokens, Some(50));
        assert_eq!(response.duration_ms, Some(1500));
    }
}
