//! llama.cpp client for offline LLM inference
//!
//! This module provides an embedded llama.cpp inference engine for
//! offline schema refinement using local GGUF model files.
//!
//! # Example
//!
//! ```ignore
//! use data_modelling_core::llm::llamacpp::LlamaCppClient;
//!
//! let client = LlamaCppClient::new("/models/codellama-7b.gguf")?
//!     .with_gpu_layers(35)
//!     .with_context_size(4096);
//!
//! let response = client.complete("Analyze this schema...").await?;
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;

use super::client::LlmClient;
use super::error::{LlmError, LlmResult};

#[cfg(feature = "llm-offline")]
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{AddBos, LlamaModel, Special, params::LlamaModelParams},
    sampling::{LlamaSampler, LlamaSamplerChainParams},
    token::LlamaToken,
};

#[cfg(feature = "llm-offline")]
use std::sync::Mutex;

/// llama.cpp client for offline LLM inference
#[derive(Debug)]
pub struct LlamaCppClient {
    /// Path to the GGUF model file
    model_path: PathBuf,
    /// Model name (derived from filename)
    model_name: String,
    /// Number of GPU layers to offload
    gpu_layers: u32,
    /// Context size in tokens
    context_size: usize,
    /// Temperature for sampling
    temperature: f32,
    /// Top-p (nucleus) sampling
    top_p: f32,
    /// Top-k sampling
    top_k: i32,
    /// Repeat penalty
    repeat_penalty: f32,
    /// Maximum tokens to generate
    max_gen_tokens: usize,
    /// Loaded model (lazy initialization)
    #[cfg(feature = "llm-offline")]
    inner: Arc<Mutex<Option<LoadedModel>>>,
}

/// Holds the loaded model and backend
#[cfg(feature = "llm-offline")]
struct LoadedModel {
    backend: LlamaBackend,
    model: LlamaModel,
}

#[cfg(feature = "llm-offline")]
impl std::fmt::Debug for LoadedModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadedModel")
            .field("n_params", &self.model.n_params())
            .field("n_vocab", &self.model.n_vocab())
            .finish()
    }
}

impl LlamaCppClient {
    /// Create a new llama.cpp client
    ///
    /// # Arguments
    /// * `model_path` - Path to the GGUF model file
    ///
    /// # Returns
    /// A new client instance (model is loaded lazily on first use)
    pub fn new(model_path: impl Into<PathBuf>) -> LlmResult<Self> {
        let path: PathBuf = model_path.into();

        // Extract model name from filename
        let model_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Verify the file exists
        if !path.exists() {
            return Err(LlmError::ModelError(format!(
                "Model file not found: {}",
                path.display()
            )));
        }

        // Verify it's a GGUF file
        if path.extension().and_then(|s| s.to_str()) != Some("gguf") {
            return Err(LlmError::ModelError(format!(
                "Model file must be a GGUF file: {}",
                path.display()
            )));
        }

        Ok(Self {
            model_path: path,
            model_name,
            gpu_layers: 0,
            context_size: 4096,
            temperature: 0.1,
            top_p: 0.9,
            top_k: 40,
            repeat_penalty: 1.1,
            max_gen_tokens: 2048,
            #[cfg(feature = "llm-offline")]
            inner: Arc::new(Mutex::new(None)),
        })
    }

    /// Set the number of GPU layers to offload
    ///
    /// Set to 0 for CPU-only inference, or higher values to offload
    /// more layers to the GPU for faster inference.
    pub fn with_gpu_layers(mut self, layers: u32) -> Self {
        self.gpu_layers = layers;
        self
    }

    /// Set the context size in tokens
    pub fn with_context_size(mut self, size: usize) -> Self {
        self.context_size = size;
        self
    }

    /// Set the temperature for sampling
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Set the top-p (nucleus) sampling parameter
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = top_p.clamp(0.0, 1.0);
        self
    }

    /// Set the top-k sampling parameter
    pub fn with_top_k(mut self, top_k: i32) -> Self {
        self.top_k = top_k.max(1);
        self
    }

    /// Set the repeat penalty
    pub fn with_repeat_penalty(mut self, penalty: f32) -> Self {
        self.repeat_penalty = penalty.clamp(1.0, 2.0);
        self
    }

    /// Set the maximum tokens to generate
    pub fn with_max_gen_tokens(mut self, max_tokens: usize) -> Self {
        self.max_gen_tokens = max_tokens;
        self
    }

    /// Get the model path
    pub fn model_path(&self) -> &PathBuf {
        &self.model_path
    }

    /// Check if GPU acceleration is enabled
    pub fn uses_gpu(&self) -> bool {
        self.gpu_layers > 0
    }

    /// Load the model if not already loaded
    #[cfg(feature = "llm-offline")]
    fn ensure_loaded(&self) -> LlmResult<()> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| LlmError::ModelError(format!("Failed to acquire model lock: {}", e)))?;

        if inner.is_some() {
            return Ok(());
        }

        tracing::info!(
            model_path = %self.model_path.display(),
            gpu_layers = self.gpu_layers,
            context_size = self.context_size,
            "Loading llama.cpp model"
        );

        // Initialize backend
        let backend = LlamaBackend::init().map_err(|e| {
            LlmError::ModelError(format!("Failed to initialize llama.cpp backend: {}", e))
        })?;

        // Configure model parameters
        let model_params = LlamaModelParams::default().with_n_gpu_layers(self.gpu_layers);

        // Load model
        let model = LlamaModel::load_from_file(&backend, &self.model_path, &model_params)
            .map_err(|e| LlmError::ModelError(format!("Failed to load model: {}", e)))?;

        tracing::info!(
            n_params = model.n_params(),
            n_vocab = model.n_vocab(),
            n_embd = model.n_embd(),
            "Model loaded successfully"
        );

        *inner = Some(LoadedModel { backend, model });
        Ok(())
    }

    /// Generate a completion using the loaded model
    #[cfg(feature = "llm-offline")]
    fn generate_completion(&self, prompt: &str) -> LlmResult<String> {
        // Ensure model is loaded
        self.ensure_loaded()?;

        let inner = self
            .inner
            .lock()
            .map_err(|e| LlmError::ModelError(format!("Failed to acquire model lock: {}", e)))?;

        let loaded = inner
            .as_ref()
            .ok_or_else(|| LlmError::ModelError("Model not loaded".to_string()))?;

        // Create context
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(self.context_size as u32).unwrap());

        let mut ctx = loaded
            .model
            .new_context(&loaded.backend, ctx_params)
            .map_err(|e| LlmError::ModelError(format!("Failed to create context: {}", e)))?;

        // Tokenize prompt
        let tokens = loaded
            .model
            .str_to_token(prompt, AddBos::Always)
            .map_err(|e| LlmError::ModelError(format!("Tokenization failed: {}", e)))?;

        let prompt_len = tokens.len();
        tracing::debug!(prompt_len, "Prompt tokenized");

        if prompt_len > self.context_size {
            return Err(LlmError::ContextTooLarge {
                max: self.context_size,
                actual: prompt_len,
            });
        }

        // Create batch for prompt processing
        let mut batch = LlamaBatch::new(self.context_size, 1);

        // Add prompt tokens to batch
        for (i, token) in tokens.iter().enumerate() {
            let is_last = i == tokens.len() - 1;
            batch.add(*token, i as i32, &[0], is_last).map_err(|e| {
                LlmError::ModelError(format!("Failed to add token to batch: {}", e))
            })?;
        }

        // Decode prompt
        ctx.decode(&mut batch)
            .map_err(|e| LlmError::ModelError(format!("Failed to decode prompt: {}", e)))?;

        // Set up sampler chain with temperature, top-k, top-p
        let sampler_params = LlamaSamplerChainParams::default();
        let mut sampler = LlamaSampler::chain(sampler_params);

        // Add sampling stages
        sampler.add_temp(self.temperature);
        sampler.add_top_k(self.top_k);
        sampler.add_top_p(self.top_p, 1);
        sampler.add_dist(42); // Random seed

        // Generate tokens
        let mut output_tokens: Vec<LlamaToken> = Vec::new();
        let mut pos = prompt_len;
        let max_tokens = self.max_gen_tokens.min(self.context_size - prompt_len);

        tracing::debug!(max_tokens, "Starting generation");

        for _ in 0..max_tokens {
            // Sample next token
            let token = sampler.sample(&ctx, -1);
            sampler.accept(token);

            // Check for end of generation
            if loaded.model.is_eog_token(token) {
                tracing::debug!("End of generation token reached");
                break;
            }

            output_tokens.push(token);

            // Prepare batch for next token
            batch.clear();
            batch.add(token, pos as i32, &[0], true).map_err(|e| {
                LlmError::ModelError(format!("Failed to add generated token to batch: {}", e))
            })?;

            // Decode next token
            ctx.decode(&mut batch).map_err(|e| {
                LlmError::ModelError(format!("Failed to decode generated token: {}", e))
            })?;

            pos += 1;
        }

        // Convert output tokens to string
        let output = loaded
            .model
            .tokens_to_str(&output_tokens, Special::Tokenize)
            .map_err(|e| LlmError::ModelError(format!("Failed to decode output tokens: {}", e)))?;

        tracing::debug!(
            output_tokens = output_tokens.len(),
            output_len = output.len(),
            "Generation complete"
        );

        Ok(output)
    }
}

#[cfg(feature = "llm-offline")]
#[async_trait]
impl LlmClient for LlamaCppClient {
    async fn complete(&self, prompt: &str) -> LlmResult<String> {
        // llama.cpp inference is blocking, so we run it in a blocking task
        let client = self.clone();
        let prompt = prompt.to_string();

        tokio::task::spawn_blocking(move || client.generate_completion(&prompt))
            .await
            .map_err(|e| LlmError::ModelError(format!("Task join error: {}", e)))?
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn max_tokens(&self) -> usize {
        self.context_size
    }

    async fn is_ready(&self) -> bool {
        self.ensure_loaded().is_ok()
    }
}

#[cfg(feature = "llm-offline")]
impl Clone for LlamaCppClient {
    fn clone(&self) -> Self {
        Self {
            model_path: self.model_path.clone(),
            model_name: self.model_name.clone(),
            gpu_layers: self.gpu_layers,
            context_size: self.context_size,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            repeat_penalty: self.repeat_penalty,
            max_gen_tokens: self.max_gen_tokens,
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(not(feature = "llm-offline"))]
#[async_trait]
impl LlmClient for LlamaCppClient {
    async fn complete(&self, _prompt: &str) -> LlmResult<String> {
        Err(LlmError::FeatureNotAvailable(
            "Offline LLM".to_string(),
            "llm-offline".to_string(),
        ))
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn max_tokens(&self) -> usize {
        self.context_size
    }

    async fn is_ready(&self) -> bool {
        false
    }
}

#[cfg(not(feature = "llm-offline"))]
impl Clone for LlamaCppClient {
    fn clone(&self) -> Self {
        Self {
            model_path: self.model_path.clone(),
            model_name: self.model_name.clone(),
            gpu_layers: self.gpu_layers,
            context_size: self.context_size,
            temperature: self.temperature,
            top_p: self.top_p,
            top_k: self.top_k,
            repeat_penalty: self.repeat_penalty,
            max_gen_tokens: self.max_gen_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_llamacpp_client_new_nonexistent() {
        let result = LlamaCppClient::new("/nonexistent/model.gguf");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), LlmError::ModelError(_)));
    }

    #[test]
    fn test_llamacpp_client_new_wrong_extension() {
        let mut temp_file = NamedTempFile::with_suffix(".bin").unwrap();
        writeln!(temp_file, "fake model data").unwrap();

        let result = LlamaCppClient::new(temp_file.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, LlmError::ModelError(_)));
        if let LlmError::ModelError(msg) = err {
            assert!(msg.contains("GGUF"));
        }
    }

    #[test]
    fn test_llamacpp_client_new_exists() {
        // Create a temporary file to simulate a model
        let mut temp_file = NamedTempFile::with_suffix(".gguf").unwrap();
        writeln!(temp_file, "fake model data").unwrap();

        let client = LlamaCppClient::new(temp_file.path()).unwrap();
        assert!(client.model_path().exists());
        assert!(!client.model_name.is_empty());
        assert_eq!(client.gpu_layers, 0);
        assert_eq!(client.context_size, 4096);
    }

    #[test]
    fn test_llamacpp_client_builder() {
        let mut temp_file = NamedTempFile::with_suffix(".gguf").unwrap();
        writeln!(temp_file, "fake model data").unwrap();

        let client = LlamaCppClient::new(temp_file.path())
            .unwrap()
            .with_gpu_layers(35)
            .with_context_size(8192)
            .with_temperature(0.3)
            .with_top_p(0.95)
            .with_top_k(50)
            .with_repeat_penalty(1.2)
            .with_max_gen_tokens(4096);

        assert_eq!(client.gpu_layers, 35);
        assert_eq!(client.context_size, 8192);
        assert!((client.temperature - 0.3).abs() < f32::EPSILON);
        assert!((client.top_p - 0.95).abs() < f32::EPSILON);
        assert_eq!(client.top_k, 50);
        assert!((client.repeat_penalty - 1.2).abs() < f32::EPSILON);
        assert_eq!(client.max_gen_tokens, 4096);
        assert!(client.uses_gpu());
    }

    #[test]
    fn test_parameter_clamping() {
        let mut temp_file = NamedTempFile::with_suffix(".gguf").unwrap();
        writeln!(temp_file, "fake model data").unwrap();

        let client = LlamaCppClient::new(temp_file.path())
            .unwrap()
            .with_temperature(5.0)
            .with_top_p(1.5)
            .with_top_k(0)
            .with_repeat_penalty(0.5);

        assert!((client.temperature - 2.0).abs() < f32::EPSILON);
        assert!((client.top_p - 1.0).abs() < f32::EPSILON);
        assert_eq!(client.top_k, 1); // Minimum of 1
        assert!((client.repeat_penalty - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_model_name_extraction() {
        let mut temp_file = NamedTempFile::with_suffix(".gguf").unwrap();
        writeln!(temp_file, "fake model data").unwrap();

        let client = LlamaCppClient::new(temp_file.path()).unwrap();
        // The model name should be derived from the temp file name (without .gguf)
        assert!(!client.model_name.is_empty());
        assert!(!client.model_name.contains(".gguf"));
    }

    #[test]
    fn test_clone() {
        let mut temp_file = NamedTempFile::with_suffix(".gguf").unwrap();
        writeln!(temp_file, "fake model data").unwrap();

        let client = LlamaCppClient::new(temp_file.path())
            .unwrap()
            .with_gpu_layers(10)
            .with_context_size(2048);

        let cloned = client.clone();
        assert_eq!(cloned.model_path, client.model_path);
        assert_eq!(cloned.gpu_layers, client.gpu_layers);
        assert_eq!(cloned.context_size, client.context_size);
    }
}
