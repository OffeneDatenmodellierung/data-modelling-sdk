//! Configuration types for LLM-based schema refinement
//!
//! This module provides configuration types for controlling how LLM refinement
//! is performed, including mode selection (online/offline), model parameters,
//! and documentation settings.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// LLM operation mode
///
/// Determines how the LLM is accessed for schema refinement:
/// - `None`: No LLM refinement (inference only)
/// - `Online`: Connect to an Ollama API server
/// - `Offline`: Use embedded llama.cpp with local model file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum LlmMode {
    /// No LLM refinement - use inference only
    None,

    /// Online mode using Ollama API
    Online {
        /// Ollama API URL (default: http://localhost:11434)
        url: String,
        /// Model name (e.g., "llama3.2", "mistral", "codellama")
        model: String,
    },

    /// Offline mode using embedded llama.cpp
    Offline {
        /// Path to GGUF model file
        model_path: PathBuf,
        /// Number of GPU layers to offload (0 = CPU only)
        #[serde(default)]
        gpu_layers: u32,
    },
}

impl Default for LlmMode {
    fn default() -> Self {
        LlmMode::None
    }
}

impl LlmMode {
    /// Create an online mode configuration with default Ollama URL
    pub fn online(model: impl Into<String>) -> Self {
        LlmMode::Online {
            url: "http://localhost:11434".to_string(),
            model: model.into(),
        }
    }

    /// Create an online mode configuration with custom URL
    pub fn online_with_url(url: impl Into<String>, model: impl Into<String>) -> Self {
        LlmMode::Online {
            url: url.into(),
            model: model.into(),
        }
    }

    /// Create an offline mode configuration
    pub fn offline(model_path: impl Into<PathBuf>) -> Self {
        LlmMode::Offline {
            model_path: model_path.into(),
            gpu_layers: 0,
        }
    }

    /// Create an offline mode configuration with GPU acceleration
    pub fn offline_with_gpu(model_path: impl Into<PathBuf>, gpu_layers: u32) -> Self {
        LlmMode::Offline {
            model_path: model_path.into(),
            gpu_layers,
        }
    }

    /// Check if LLM refinement is enabled
    pub fn is_enabled(&self) -> bool {
        !matches!(self, LlmMode::None)
    }

    /// Check if using online mode
    pub fn is_online(&self) -> bool {
        matches!(self, LlmMode::Online { .. })
    }

    /// Check if using offline mode
    pub fn is_offline(&self) -> bool {
        matches!(self, LlmMode::Offline { .. })
    }
}

/// Configuration for schema refinement using LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefinementConfig {
    /// LLM mode (none, online, offline)
    pub llm_mode: LlmMode,

    /// Path to documentation file for context
    pub documentation_path: Option<PathBuf>,

    /// Documentation text provided directly
    pub documentation_text: Option<String>,

    /// Maximum context tokens for LLM
    #[serde(default = "default_max_context_tokens")]
    pub max_context_tokens: usize,

    /// Request timeout in seconds
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,

    /// Maximum retries on failure
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,

    /// Temperature for LLM sampling (0.0 = deterministic, 1.0 = creative)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Include sample records in prompt
    #[serde(default = "default_include_samples")]
    pub include_samples: bool,

    /// Maximum number of sample records to include
    #[serde(default = "default_max_samples")]
    pub max_samples: usize,

    /// Enable verbose logging of prompts and responses
    #[serde(default)]
    pub verbose: bool,
}

fn default_max_context_tokens() -> usize {
    4096
}

fn default_timeout_seconds() -> u64 {
    120
}

fn default_max_retries() -> usize {
    3
}

fn default_temperature() -> f32 {
    0.1
}

fn default_include_samples() -> bool {
    true
}

fn default_max_samples() -> usize {
    5
}

impl Default for RefinementConfig {
    fn default() -> Self {
        Self {
            llm_mode: LlmMode::None,
            documentation_path: None,
            documentation_text: None,
            max_context_tokens: default_max_context_tokens(),
            timeout_seconds: default_timeout_seconds(),
            max_retries: default_max_retries(),
            temperature: default_temperature(),
            include_samples: default_include_samples(),
            max_samples: default_max_samples(),
            verbose: false,
        }
    }
}

impl RefinementConfig {
    /// Create a new refinement config with online LLM
    pub fn with_ollama(model: impl Into<String>) -> Self {
        Self {
            llm_mode: LlmMode::online(model),
            ..Default::default()
        }
    }

    /// Create a new refinement config with offline LLM
    pub fn with_local_model(model_path: impl Into<PathBuf>) -> Self {
        Self {
            llm_mode: LlmMode::offline(model_path),
            ..Default::default()
        }
    }

    /// Set documentation from file path
    pub fn with_documentation_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.documentation_path = Some(path.into());
        self
    }

    /// Set documentation from text
    pub fn with_documentation_text(mut self, text: impl Into<String>) -> Self {
        self.documentation_text = Some(text.into());
        self
    }

    /// Set maximum context tokens
    pub fn with_max_context_tokens(mut self, tokens: usize) -> Self {
        self.max_context_tokens = tokens;
        self
    }

    /// Set timeout in seconds
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }

    /// Set maximum retries
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature.clamp(0.0, 2.0);
        self
    }

    /// Enable verbose logging
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Check if LLM refinement is enabled
    pub fn is_enabled(&self) -> bool {
        self.llm_mode.is_enabled()
    }

    /// Check if documentation is available
    pub fn has_documentation(&self) -> bool {
        self.documentation_path.is_some() || self.documentation_text.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_mode_default() {
        let mode = LlmMode::default();
        assert!(matches!(mode, LlmMode::None));
        assert!(!mode.is_enabled());
    }

    #[test]
    fn test_llm_mode_online() {
        let mode = LlmMode::online("llama3.2");
        assert!(mode.is_enabled());
        assert!(mode.is_online());
        assert!(!mode.is_offline());

        match mode {
            LlmMode::Online { url, model } => {
                assert_eq!(url, "http://localhost:11434");
                assert_eq!(model, "llama3.2");
            }
            _ => panic!("Expected Online mode"),
        }
    }

    #[test]
    fn test_llm_mode_online_custom_url() {
        let mode = LlmMode::online_with_url("http://remote:11434", "mistral");
        match mode {
            LlmMode::Online { url, model } => {
                assert_eq!(url, "http://remote:11434");
                assert_eq!(model, "mistral");
            }
            _ => panic!("Expected Online mode"),
        }
    }

    #[test]
    fn test_llm_mode_offline() {
        let mode = LlmMode::offline("/models/llama.gguf");
        assert!(mode.is_enabled());
        assert!(!mode.is_online());
        assert!(mode.is_offline());

        match mode {
            LlmMode::Offline {
                model_path,
                gpu_layers,
            } => {
                assert_eq!(model_path, PathBuf::from("/models/llama.gguf"));
                assert_eq!(gpu_layers, 0);
            }
            _ => panic!("Expected Offline mode"),
        }
    }

    #[test]
    fn test_llm_mode_offline_with_gpu() {
        let mode = LlmMode::offline_with_gpu("/models/llama.gguf", 35);
        match mode {
            LlmMode::Offline { gpu_layers, .. } => {
                assert_eq!(gpu_layers, 35);
            }
            _ => panic!("Expected Offline mode"),
        }
    }

    #[test]
    fn test_llm_mode_serialize() {
        let mode = LlmMode::online("llama3.2");
        let json = serde_json::to_string(&mode).unwrap();
        assert!(json.contains("online"));
        assert!(json.contains("llama3.2"));

        let parsed: LlmMode = serde_json::from_str(&json).unwrap();
        assert_eq!(mode, parsed);
    }

    #[test]
    fn test_refinement_config_default() {
        let config = RefinementConfig::default();
        assert!(!config.is_enabled());
        assert!(!config.has_documentation());
        assert_eq!(config.max_context_tokens, 4096);
        assert_eq!(config.timeout_seconds, 120);
        assert_eq!(config.max_retries, 3);
        assert!((config.temperature - 0.1).abs() < f32::EPSILON);
    }

    #[test]
    fn test_refinement_config_with_ollama() {
        let config = RefinementConfig::with_ollama("llama3.2")
            .with_documentation_text("This is a customer database")
            .with_max_context_tokens(8192)
            .with_timeout(60)
            .with_temperature(0.2);

        assert!(config.is_enabled());
        assert!(config.has_documentation());
        assert_eq!(config.max_context_tokens, 8192);
        assert_eq!(config.timeout_seconds, 60);
        assert!((config.temperature - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_refinement_config_with_local_model() {
        let config = RefinementConfig::with_local_model("/models/codellama.gguf")
            .with_documentation_file("/docs/schema.md");

        assert!(config.is_enabled());
        assert!(config.has_documentation());
        assert!(config.llm_mode.is_offline());
    }

    #[test]
    fn test_refinement_config_temperature_clamp() {
        let config = RefinementConfig::default().with_temperature(5.0);
        assert!((config.temperature - 2.0).abs() < f32::EPSILON);

        let config = RefinementConfig::default().with_temperature(-1.0);
        assert!(config.temperature.abs() < f32::EPSILON);
    }

    #[test]
    fn test_refinement_config_serialize() {
        let config = RefinementConfig::with_ollama("mistral").with_documentation_text("Test docs");

        let json = serde_json::to_string(&config).unwrap();
        let parsed: RefinementConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.max_context_tokens, parsed.max_context_tokens);
        assert!(parsed.is_enabled());
    }
}
