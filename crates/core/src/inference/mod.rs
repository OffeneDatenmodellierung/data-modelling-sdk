//! Schema inference engine for JSON data
//!
//! This module provides automatic schema inference from raw JSON data,
//! detecting types, formats, and generating JSON Schema definitions.
//!
//! ## Features
//!
//! - **Type inference** - Detect JSON types (string, number, boolean, array, object)
//! - **Format detection** - Recognize common formats (date, uuid, email, uri, etc.)
//! - **Schema merging** - Combine schemas to find the minimum common schema
//! - **Nullability tracking** - Track optional vs required fields
//! - **Example collection** - Gather sample values for documentation
//!
//! ## Example
//!
//! ```rust,ignore
//! use data_modelling_core::inference::{SchemaInferrer, InferenceConfig};
//!
//! let mut inferrer = SchemaInferrer::new();
//!
//! // Add JSON records
//! inferrer.add_json(r#"{"name": "Alice", "age": 30}"#)?;
//! inferrer.add_json(r#"{"name": "Bob", "age": 25, "email": "bob@example.com"}"#)?;
//!
//! // Generate schema
//! let schema = inferrer.finalize()?;
//! println!("{}", serde_json::to_string_pretty(&schema)?);
//! ```

mod config;
mod error;
mod formats;
mod inferrer;
mod merge;
mod types;

pub use config::{InferenceConfig, InferenceConfigBuilder};
pub use error::InferenceError;
pub use formats::{Format, detect_format};
pub use inferrer::{InferenceStats, SchemaInferrer};
pub use merge::{group_similar_schemas, merge_schemas};
pub use types::{InferredField, InferredSchema, InferredType};
