//! LLM-enhanced schema refinement
//!
//! This module provides LLM-based schema refinement capabilities for enhancing
//! automatically inferred schemas with descriptions, formats, and constraints.
//!
//! # Features
//!
//! - **Online Mode**: Connect to Ollama API for LLM inference (requires `llm-online` feature)
//! - **Offline Mode**: Use embedded llama.cpp for local inference (requires `llm-offline` feature)
//! - **Documentation Context**: Load documentation to provide context for refinement
//! - **Validation**: Ensure refined schemas maintain compatibility with originals
//!
//! # Example
//!
//! ```ignore
//! use data_modelling_core::llm::{
//!     RefinementConfig, OllamaClient, SchemaRefiner,
//! };
//!
//! // Configure online refinement with Ollama
//! let config = RefinementConfig::with_ollama("llama3.2")
//!     .with_documentation_text("Customer database schema")
//!     .with_timeout(60);
//!
//! // Create the client and refiner
//! let client = OllamaClient::new("http://localhost:11434", "llama3.2");
//! let refiner = SchemaRefiner::new(client, config);
//!
//! // Refine a schema
//! let original_schema = serde_json::json!({
//!     "type": "object",
//!     "properties": {
//!         "customer_id": {"type": "string"},
//!         "email": {"type": "string"}
//!     }
//! });
//!
//! let result = refiner.refine(&original_schema, None).await?;
//! println!("Refined schema: {}", result.schema);
//! ```
//!
//! # Feature Flags
//!
//! - `llm-online`: Enable Ollama client for online inference
//! - `llm-offline`: Enable llama.cpp client for offline inference
//!
//! Without either feature, the LLM module provides configuration types and
//! validation, but actual inference will return feature-not-available errors.

pub mod client;
pub mod config;
pub mod docs;
pub mod error;
pub mod llamacpp;
pub mod ollama;
pub mod prompt;
pub mod refine;
pub mod validation;

// Re-export main types
pub use client::{CompletionResponse, LlmClient};
pub use config::{LlmMode, RefinementConfig};
pub use docs::{DocFormat, load_documentation};
pub use error::{LlmError, LlmResult};
pub use llamacpp::LlamaCppClient;
pub use ollama::OllamaClient;
pub use prompt::{PromptContext, estimate_tokens, parse_llm_response};
pub use refine::{RefinementBuilder, RefinementResult, SchemaRefiner, refine_schema};
pub use validation::{ValidationError, ValidationResult, validate_refinement};

#[cfg(test)]
pub use client::MockLlmClient;
