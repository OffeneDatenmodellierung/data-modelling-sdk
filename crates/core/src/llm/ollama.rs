//! Ollama API client for online LLM inference
//!
//! This module provides an HTTP client for the Ollama API, enabling
//! online schema refinement using locally-hosted LLM models.
//!
//! # Example
//!
//! ```ignore
//! use data_modelling_core::llm::ollama::OllamaClient;
//!
//! let client = OllamaClient::new("http://localhost:11434", "llama3.2")
//!     .with_timeout(60);
//!
//! let response = client.complete("Analyze this schema...").await?;
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::client::LlmClient;
use super::error::{LlmError, LlmResult};

/// Ollama API client for online LLM inference
#[derive(Debug, Clone)]
pub struct OllamaClient {
    /// Base URL of the Ollama API
    base_url: String,
    /// Model name to use
    model: String,
    /// Request timeout in seconds
    timeout_seconds: u64,
    /// Maximum context tokens
    max_context_tokens: usize,
    /// Temperature for sampling
    temperature: f32,
    /// HTTP client
    #[cfg(feature = "llm-online")]
    client: reqwest::Client,
}

/// Request body for Ollama generate endpoint
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<GenerateOptions>,
}

/// Options for generation
#[derive(Debug, Serialize)]
#[allow(dead_code)]
struct GenerateOptions {
    temperature: f32,
    num_ctx: usize,
}

/// Response from Ollama generate endpoint
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GenerateResponse {
    response: String,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    total_duration: Option<u64>,
    #[serde(default)]
    prompt_eval_count: Option<usize>,
    #[serde(default)]
    eval_count: Option<usize>,
}

/// Response from Ollama tags endpoint (list models)
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TagsResponse {
    models: Vec<ModelInfo>,
}

/// Model information from Ollama
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ModelInfo {
    name: String,
    #[serde(default)]
    size: u64,
}

impl OllamaClient {
    /// Create a new Ollama client
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the Ollama API (e.g., "http://localhost:11434")
    /// * `model` - Model name to use (e.g., "llama3.2", "mistral", "codellama")
    #[cfg(feature = "llm-online")]
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            timeout_seconds: 120,
            max_context_tokens: 4096,
            temperature: 0.1,
            client: reqwest::Client::new(),
        }
    }

    /// Create a new Ollama client (stub for when feature is disabled)
    #[cfg(not(feature = "llm-online"))]
    pub fn new(base_url: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            model: model.into(),
            timeout_seconds: 120,
            max_context_tokens: 4096,
            temperature: 0.1,
        }
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set the maximum context tokens
    pub fn with_max_context(mut self, tokens: usize) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    /// Set the temperature for sampling
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// List available models on the Ollama server
    #[cfg(feature = "llm-online")]
    pub async fn list_models(&self) -> LlmResult<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| LlmError::ConnectionError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(LlmError::ConnectionError(format!(
                "Failed to list models: HTTP {}",
                response.status()
            )));
        }

        let tags: TagsResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    /// List available models (stub for when feature is disabled)
    #[cfg(not(feature = "llm-online"))]
    pub async fn list_models(&self) -> LlmResult<Vec<String>> {
        Err(LlmError::FeatureNotAvailable(
            "Online LLM".to_string(),
            "llm-online".to_string(),
        ))
    }

    /// Check if the specified model is available
    #[cfg(feature = "llm-online")]
    pub async fn model_available(&self) -> LlmResult<bool> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|m| m.starts_with(&self.model)))
    }

    /// Check if the specified model is available (stub)
    #[cfg(not(feature = "llm-online"))]
    pub async fn model_available(&self) -> LlmResult<bool> {
        Err(LlmError::FeatureNotAvailable(
            "Online LLM".to_string(),
            "llm-online".to_string(),
        ))
    }
}

#[cfg(feature = "llm-online")]
#[async_trait]
impl LlmClient for OllamaClient {
    async fn complete(&self, prompt: &str) -> LlmResult<String> {
        let url = format!("{}/api/generate", self.base_url);

        let request = GenerateRequest {
            model: &self.model,
            prompt,
            stream: false,
            options: Some(GenerateOptions {
                temperature: self.temperature,
                num_ctx: self.max_context_tokens,
            }),
        };

        tracing::debug!("Sending request to Ollama: {}", url);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(std::time::Duration::from_secs(self.timeout_seconds))
            .send()
            .await
            .map_err(|e| {
                if e.is_timeout() {
                    LlmError::Timeout(self.timeout_seconds)
                } else if e.is_connect() {
                    LlmError::ConnectionError(format!(
                        "Failed to connect to Ollama at {}: {}",
                        self.base_url, e
                    ))
                } else {
                    LlmError::ConnectionError(e.to_string())
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(LlmError::RateLimited(60));
            }
            return Err(LlmError::ConnectionError(format!(
                "Ollama API error (HTTP {}): {}",
                status, error_text
            )));
        }

        let gen_response: GenerateResponse = response
            .json()
            .await
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        if let Some(duration) = gen_response.total_duration {
            tracing::debug!(
                "Ollama completion took {} ms, {} prompt tokens, {} completion tokens",
                duration / 1_000_000,
                gen_response.prompt_eval_count.unwrap_or(0),
                gen_response.eval_count.unwrap_or(0)
            );
        }

        Ok(gen_response.response)
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn max_tokens(&self) -> usize {
        self.max_context_tokens
    }

    async fn is_ready(&self) -> bool {
        self.list_models().await.is_ok()
    }
}

#[cfg(not(feature = "llm-online"))]
#[async_trait]
impl LlmClient for OllamaClient {
    async fn complete(&self, _prompt: &str) -> LlmResult<String> {
        Err(LlmError::FeatureNotAvailable(
            "Online LLM".to_string(),
            "llm-online".to_string(),
        ))
    }

    fn model_name(&self) -> &str {
        &self.model
    }

    fn max_tokens(&self) -> usize {
        self.max_context_tokens
    }

    async fn is_ready(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_client_new() {
        let client = OllamaClient::new("http://localhost:11434", "llama3.2");
        assert_eq!(client.base_url(), "http://localhost:11434");
        assert_eq!(client.model_name(), "llama3.2");
        assert_eq!(client.max_tokens(), 4096);
    }

    #[test]
    fn test_ollama_client_builder() {
        let client = OllamaClient::new("http://remote:11434", "mistral")
            .with_timeout(60)
            .with_max_context(8192)
            .with_temperature(0.5);

        assert_eq!(client.timeout_seconds, 60);
        assert_eq!(client.max_context_tokens, 8192);
        assert!((client.temperature - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_temperature_clamp() {
        let client = OllamaClient::new("http://localhost:11434", "llama3.2").with_temperature(5.0);
        assert!((client.temperature - 2.0).abs() < f32::EPSILON);

        let client = OllamaClient::new("http://localhost:11434", "llama3.2").with_temperature(-1.0);
        assert!(client.temperature.abs() < f32::EPSILON);
    }

    #[test]
    fn test_generate_request_serialize() {
        let request = GenerateRequest {
            model: "llama3.2",
            prompt: "Test prompt",
            stream: false,
            options: Some(GenerateOptions {
                temperature: 0.1,
                num_ctx: 4096,
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("llama3.2"));
        assert!(json.contains("Test prompt"));
        assert!(json.contains("temperature"));
    }

    #[test]
    fn test_generate_response_deserialize() {
        let json = r#"{
            "response": "Generated text",
            "done": true,
            "total_duration": 1500000000,
            "prompt_eval_count": 50,
            "eval_count": 100
        }"#;

        let response: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.response, "Generated text");
        assert!(response.done);
        assert_eq!(response.total_duration, Some(1500000000));
        assert_eq!(response.prompt_eval_count, Some(50));
        assert_eq!(response.eval_count, Some(100));
    }

    #[test]
    fn test_generate_response_minimal() {
        let json = r#"{"response": "Text", "done": true}"#;
        let response: GenerateResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.response, "Text");
        assert!(response.total_duration.is_none());
    }
}
