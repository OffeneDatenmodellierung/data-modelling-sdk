//! Schema refinement pipeline
//!
//! This module provides the main refinement pipeline that combines
//! LLM inference with validation to produce refined schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::LlmClient;
use super::config::RefinementConfig;
use super::docs::load_documentation;
use super::error::{LlmError, LlmResult};
use super::prompt::{PromptContext, parse_llm_response};
use super::validation::{ValidationResult, validate_refinement};

/// Result of a schema refinement operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinementResult {
    /// The refined schema
    pub schema: Value,
    /// Whether refinement was performed (false if LLM disabled)
    pub was_refined: bool,
    /// The LLM model used (if any)
    pub model_used: Option<String>,
    /// Number of retries needed
    pub retries: usize,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Time taken for refinement in milliseconds
    pub duration_ms: Option<u64>,
}

impl RefinementResult {
    /// Create a result for when no refinement was performed
    pub fn unchanged(schema: Value) -> Self {
        Self {
            schema,
            was_refined: false,
            model_used: None,
            retries: 0,
            warnings: Vec::new(),
            duration_ms: None,
        }
    }

    /// Create a result for successful refinement
    pub fn refined(
        schema: Value,
        model: impl Into<String>,
        retries: usize,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            schema,
            was_refined: true,
            model_used: Some(model.into()),
            retries,
            warnings,
            duration_ms: None,
        }
    }

    /// Set the duration
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }
}

/// Schema refiner that uses LLM to enhance inferred schemas
pub struct SchemaRefiner<C: LlmClient> {
    client: C,
    config: RefinementConfig,
}

impl<C: LlmClient> SchemaRefiner<C> {
    /// Create a new schema refiner
    pub fn new(client: C, config: RefinementConfig) -> Self {
        Self { client, config }
    }

    /// Refine a schema using the LLM
    ///
    /// # Arguments
    /// * `schema` - The original inferred schema as JSON
    /// * `samples` - Optional sample records for context
    ///
    /// # Returns
    /// The refined schema with validation
    pub async fn refine(
        &self,
        schema: &Value,
        samples: Option<Vec<String>>,
    ) -> LlmResult<RefinementResult> {
        let start = std::time::Instant::now();

        // Load documentation if configured
        let documentation = self.load_documentation_context()?;

        // Build the prompt
        let schema_json = serde_json::to_string_pretty(schema)
            .map_err(|e| LlmError::ParseError(e.to_string()))?;

        let mut context =
            PromptContext::new(&schema_json).with_max_tokens(self.config.max_context_tokens);

        if let Some(doc) = documentation {
            context = context.with_documentation(doc);
        }

        if let Some(s) = samples {
            if self.config.include_samples {
                let limited_samples: Vec<String> =
                    s.into_iter().take(self.config.max_samples).collect();
                context = context.with_samples(limited_samples);
            }
        }

        let prompt = context.build_prompt();

        if self.config.verbose {
            tracing::debug!("Refinement prompt:\n{}", prompt);
        }

        // Try refinement with retries
        let mut last_error = None;
        let mut retries = 0;

        while retries <= self.config.max_retries {
            match self.try_refinement(&prompt, schema).await {
                Ok((refined_schema, validation)) => {
                    let duration = start.elapsed().as_millis() as u64;
                    return Ok(RefinementResult::refined(
                        refined_schema,
                        self.client.model_name(),
                        retries,
                        validation.warnings,
                    )
                    .with_duration(duration));
                }
                Err(e) => {
                    last_error = Some(e);
                    retries += 1;

                    if retries <= self.config.max_retries {
                        tracing::warn!("Refinement attempt {} failed, retrying...", retries);
                        // Small delay before retry
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or(LlmError::MaxRetriesExceeded(self.config.max_retries)))
    }

    /// Try a single refinement attempt
    async fn try_refinement(
        &self,
        prompt: &str,
        original_schema: &Value,
    ) -> LlmResult<(Value, ValidationResult)> {
        // Call the LLM
        let response = self.client.complete(prompt).await?;

        if self.config.verbose {
            tracing::debug!("LLM response:\n{}", response);
        }

        // Parse the response
        let parsed = parse_llm_response(&response).map_err(|e| LlmError::ParseError(e))?;

        // Validate the refinement
        let validation = validate_refinement(original_schema, &parsed.schema);

        if !validation.is_valid {
            let error_messages: Vec<String> =
                validation.errors.iter().map(|e| e.to_string()).collect();
            return Err(LlmError::ValidationError(error_messages.join("; ")));
        }

        Ok((parsed.schema, validation))
    }

    /// Load documentation from configured sources
    fn load_documentation_context(&self) -> LlmResult<Option<String>> {
        // Prefer direct text if provided
        if let Some(text) = &self.config.documentation_text {
            return Ok(Some(text.clone()));
        }

        // Load from file if configured
        if let Some(path) = &self.config.documentation_path {
            let content = load_documentation(path)?;
            return Ok(Some(content));
        }

        Ok(None)
    }
}

/// Refine a schema without creating a full refiner instance
///
/// This is a convenience function for simple refinement operations.
pub async fn refine_schema<C: LlmClient>(
    client: &C,
    schema: &Value,
    config: &RefinementConfig,
    samples: Option<Vec<String>>,
) -> LlmResult<RefinementResult> {
    if !config.is_enabled() {
        return Ok(RefinementResult::unchanged(schema.clone()));
    }

    // Load documentation
    let documentation = if let Some(text) = &config.documentation_text {
        Some(text.clone())
    } else if let Some(path) = &config.documentation_path {
        Some(load_documentation(path)?)
    } else {
        None
    };

    // Build prompt
    let schema_json =
        serde_json::to_string_pretty(schema).map_err(|e| LlmError::ParseError(e.to_string()))?;

    let mut context = PromptContext::new(&schema_json).with_max_tokens(config.max_context_tokens);

    if let Some(doc) = documentation {
        context = context.with_documentation(doc);
    }

    if let Some(s) = samples {
        if config.include_samples {
            let limited_samples: Vec<String> = s.into_iter().take(config.max_samples).collect();
            context = context.with_samples(limited_samples);
        }
    }

    let prompt = context.build_prompt();

    // Try refinement with retries
    let start = std::time::Instant::now();
    let mut last_error = None;
    let mut retries = 0;

    while retries <= config.max_retries {
        let response = match client.complete(&prompt).await {
            Ok(r) => r,
            Err(e) => {
                last_error = Some(e);
                retries += 1;
                continue;
            }
        };

        let parsed = match parse_llm_response(&response) {
            Ok(p) => p,
            Err(e) => {
                last_error = Some(LlmError::ParseError(e));
                retries += 1;
                continue;
            }
        };

        let validation = validate_refinement(schema, &parsed.schema);
        if !validation.is_valid {
            let error_messages: Vec<String> =
                validation.errors.iter().map(|e| e.to_string()).collect();
            last_error = Some(LlmError::ValidationError(error_messages.join("; ")));
            retries += 1;
            continue;
        }

        let duration = start.elapsed().as_millis() as u64;
        return Ok(RefinementResult::refined(
            parsed.schema,
            client.model_name(),
            retries,
            validation.warnings,
        )
        .with_duration(duration));
    }

    Err(last_error.unwrap_or(LlmError::MaxRetriesExceeded(config.max_retries)))
}

/// Builder for creating refinement configurations
pub struct RefinementBuilder {
    config: RefinementConfig,
}

impl RefinementBuilder {
    /// Create a new refinement builder
    pub fn new() -> Self {
        Self {
            config: RefinementConfig::default(),
        }
    }

    /// Set the LLM mode to online (Ollama)
    pub fn with_ollama(mut self, url: impl Into<String>, model: impl Into<String>) -> Self {
        self.config.llm_mode = super::config::LlmMode::Online {
            url: url.into(),
            model: model.into(),
        };
        self
    }

    /// Set the LLM mode to offline (llama.cpp)
    pub fn with_local_model(mut self, model_path: impl Into<std::path::PathBuf>) -> Self {
        self.config.llm_mode = super::config::LlmMode::Offline {
            model_path: model_path.into(),
            gpu_layers: 0,
        };
        self
    }

    /// Set documentation from file
    pub fn with_documentation_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.config.documentation_path = Some(path.into());
        self
    }

    /// Set documentation from text
    pub fn with_documentation_text(mut self, text: impl Into<String>) -> Self {
        self.config.documentation_text = Some(text.into());
        self
    }

    /// Set the temperature
    pub fn with_temperature(mut self, temp: f32) -> Self {
        self.config.temperature = temp.clamp(0.0, 2.0);
        self
    }

    /// Set the maximum context tokens
    pub fn with_max_context(mut self, tokens: usize) -> Self {
        self.config.max_context_tokens = tokens;
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.config.timeout_seconds = seconds;
        self
    }

    /// Set the maximum retries
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.config.max_retries = retries;
        self
    }

    /// Enable verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.config.verbose = verbose;
        self
    }

    /// Build the configuration
    pub fn build(self) -> RefinementConfig {
        self.config
    }
}

impl Default for RefinementBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::client::MockLlmClient;
    use serde_json::json;

    #[test]
    fn test_refinement_result_unchanged() {
        let schema = json!({"type": "object"});
        let result = RefinementResult::unchanged(schema.clone());

        assert!(!result.was_refined);
        assert!(result.model_used.is_none());
        assert_eq!(result.retries, 0);
    }

    #[test]
    fn test_refinement_result_refined() {
        let schema = json!({"type": "object"});
        let result =
            RefinementResult::refined(schema, "llama3.2", 1, vec!["Added description".to_string()])
                .with_duration(1500);

        assert!(result.was_refined);
        assert_eq!(result.model_used, Some("llama3.2".to_string()));
        assert_eq!(result.retries, 1);
        assert_eq!(result.duration_ms, Some(1500));
    }

    #[test]
    fn test_refinement_builder() {
        let config = RefinementBuilder::new()
            .with_ollama("http://localhost:11434", "llama3.2")
            .with_documentation_text("Test docs")
            .with_temperature(0.5)
            .with_max_context(8192)
            .with_timeout(60)
            .with_max_retries(5)
            .with_verbose(true)
            .build();

        assert!(config.is_enabled());
        assert!(config.has_documentation());
        assert_eq!(config.max_context_tokens, 8192);
        assert_eq!(config.timeout_seconds, 60);
        assert_eq!(config.max_retries, 5);
        assert!(config.verbose);
    }

    #[tokio::test]
    async fn test_refine_schema_disabled() {
        let client = MockLlmClient::new("{}");
        let schema = json!({"type": "object"});
        let config = RefinementConfig::default(); // LLM disabled

        let result = refine_schema(&client, &schema, &config, None)
            .await
            .unwrap();

        assert!(!result.was_refined);
        assert_eq!(result.schema, schema);
    }

    #[tokio::test]
    async fn test_schema_refiner_with_mock() {
        let refined_response = r#"{"type": "object", "properties": {"name": {"type": "string", "description": "Customer name"}}}"#;
        let client = MockLlmClient::new(refined_response);

        let config = RefinementConfig::with_ollama("llama3.2");
        let refiner = SchemaRefiner::new(client, config);

        let original = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let result = refiner.refine(&original, None).await.unwrap();

        assert!(result.was_refined);
        assert!(result.schema.get("properties").is_some());
    }

    #[tokio::test]
    async fn test_schema_refiner_validation_failure() {
        // Response that removes a field (should fail validation)
        let bad_response = r#"{"type": "object", "properties": {}}"#;
        let client = MockLlmClient::new(bad_response);

        let config = RefinementConfig::with_ollama("llama3.2").with_max_retries(0); // No retries

        let refiner = SchemaRefiner::new(client, config);

        let original = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let result = refiner.refine(&original, None).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LlmError::ValidationError(_)));
    }
}
