//! Schema mapping module for matching source schemas to target schemas
//!
//! This module provides functionality to:
//! - Match fields between source and target JSON Schemas
//! - Detect type mismatches and suggest transformations
//! - Generate transformation scripts (SQL, JQ, Python, PySpark)
//! - Identify gaps and unmapped fields
//!
//! # Example
//!
//! ```rust,ignore
//! use data_modelling_core::mapping::{SchemaMatcher, MappingConfig, TransformFormat, generate_transform};
//! use serde_json::json;
//!
//! let source = json!({
//!     "type": "object",
//!     "properties": {
//!         "customer_name": {"type": "string"},
//!         "customer_email": {"type": "string"}
//!     }
//! });
//!
//! let target = json!({
//!     "type": "object",
//!     "properties": {
//!         "name": {"type": "string"},
//!         "email": {"type": "string"}
//!     }
//! });
//!
//! let matcher = SchemaMatcher::with_config(
//!     MappingConfig::new()
//!         .with_fuzzy_matching(true)
//!         .with_min_confidence(0.7)
//! );
//!
//! let mapping = matcher.match_schemas(&source, &target)?;
//! println!("Compatibility: {:.1}%", mapping.compatibility_score * 100.0);
//!
//! // Generate SQL transformation
//! let sql = generate_transform(&mapping, TransformFormat::Sql, "source_table", "target_table")?;
//! println!("{}", sql);
//! ```

mod config;
mod error;
mod generator;
#[cfg(feature = "llm")]
mod llm_matcher;
mod matcher;
mod types;

pub use config::{MappingConfig, TransformFormat};
pub use error::{MappingError, MappingResult};
pub use generator::generate_transform;
#[cfg(feature = "llm")]
pub use llm_matcher::{LlmFieldSuggestion, LlmMatchResponse, LlmMatcherConfig, LlmSchemaMatcher};
pub use matcher::SchemaMatcher;
pub use types::{
    FieldGap, FieldMapping, MappingStats, MatchMethod, SchemaMapping, TransformMapping,
    TransformType,
};

/// Map a source schema to a target schema with default configuration
///
/// This is a convenience function for simple mapping operations.
pub fn map_schemas(
    source: &serde_json::Value,
    target: &serde_json::Value,
) -> MappingResult<SchemaMapping> {
    let matcher = SchemaMatcher::new();
    matcher.match_schemas(source, target)
}

/// Map schemas and generate a transformation script
///
/// This is a convenience function that combines matching and script generation.
pub fn map_and_generate(
    source: &serde_json::Value,
    target: &serde_json::Value,
    config: MappingConfig,
    source_table: &str,
    target_table: &str,
) -> MappingResult<(SchemaMapping, String)> {
    let matcher = SchemaMatcher::with_config(config.clone());
    let mapping = matcher.match_schemas(source, target)?;
    let script = generate_transform(
        &mapping,
        config.transform_format,
        source_table,
        target_table,
    )?;
    Ok((mapping, script))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_map_schemas() {
        let source = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "email": {"type": "string"}
            }
        });

        let result = map_schemas(&source, &target).unwrap();
        assert_eq!(result.direct_mappings.len(), 2);
        assert_eq!(result.compatibility_score, 1.0);
    }

    #[test]
    fn test_map_and_generate() {
        let source = json!({
            "type": "object",
            "properties": {
                "user_name": {"type": "string"}
            }
        });

        let target = json!({
            "type": "object",
            "properties": {
                "username": {"type": "string"}
            }
        });

        let config = MappingConfig::new()
            .with_fuzzy_matching(true)
            .with_transform_format(TransformFormat::Sql);

        let (mapping, sql) = map_and_generate(&source, &target, config, "src", "tgt").unwrap();

        assert!(!mapping.direct_mappings.is_empty() || !mapping.gaps.is_empty());
        assert!(sql.contains("INSERT") || sql.contains("SELECT"));
    }
}
