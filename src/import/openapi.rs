//! OpenAPI importer
//!
//! Provides functionality to import OpenAPI 3.1.1 YAML or JSON files with validation.

use anyhow::{Context, Result};
use std::collections::HashMap;
use uuid::Uuid;

use crate::models::openapi::{OpenAPIFormat, OpenAPIModel};

/// OpenAPI Importer
///
/// Imports OpenAPI 3.1.1 YAML or JSON content into an OpenAPIModel struct.
#[derive(Debug, Default)]
pub struct OpenAPIImporter {
    /// List of errors encountered during parsing
    pub errors: Vec<String>,
}

impl OpenAPIImporter {
    /// Create a new OpenAPIImporter
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Detect format (YAML or JSON) from content
    ///
    /// # Arguments
    ///
    /// * `content` - The OpenAPI content as a string.
    ///
    /// # Returns
    ///
    /// The detected format.
    pub fn detect_format(&self, content: &str) -> OpenAPIFormat {
        // Try to parse as JSON first (more strict)
        if serde_json::from_str::<serde_json::Value>(content).is_ok() {
            OpenAPIFormat::Json
        } else {
            OpenAPIFormat::Yaml
        }
    }

    /// Validate OpenAPI content against JSON Schema
    ///
    /// # Arguments
    ///
    /// * `content` - The OpenAPI content as a string.
    /// * `format` - The format of the content.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether validation succeeded.
    pub fn validate(&self, _content: &str, _format: OpenAPIFormat) -> Result<()> {
        // TODO: Implement JSON Schema validation using schemas/openapi-3.1.1.json
        Ok(())
    }

    /// Extract metadata from OpenAPI content
    ///
    /// # Arguments
    ///
    /// * `content` - The OpenAPI content as a string.
    /// * `format` - The format of the content.
    ///
    /// # Returns
    ///
    /// A `HashMap` containing extracted metadata (info.title, info.version, etc.).
    pub fn extract_metadata(
        &self,
        _content: &str,
        _format: OpenAPIFormat,
    ) -> HashMap<String, serde_json::Value> {
        // TODO: Implement metadata extraction from OpenAPI spec
        HashMap::new()
    }

    /// Import OpenAPI content into an OpenAPIModel struct.
    ///
    /// # Arguments
    ///
    /// * `content` - The OpenAPI content as a string.
    /// * `domain_id` - The domain ID this model belongs to.
    /// * `api_name` - The name for the API (extracted from info.title if not provided).
    ///
    /// # Returns
    ///
    /// A `Result` containing the `OpenAPIModel` if successful, or an error if parsing fails.
    pub fn import(
        &mut self,
        content: &str,
        domain_id: Uuid,
        api_name: Option<&str>,
    ) -> Result<OpenAPIModel> {
        // Detect format
        let format = self.detect_format(content);

        // Validate content
        self.validate(content, format)
            .context("OpenAPI validation failed")?;

        // Extract metadata
        let _metadata = self.extract_metadata(content, format);

        // Determine API name
        let name = api_name
            .map(|s| s.to_string())
            .unwrap_or_else(|| "openapi_spec".to_string());

        // Create file path with appropriate extension
        let extension = match format {
            OpenAPIFormat::Yaml => "yaml",
            OpenAPIFormat::Json => "json",
        };
        let file_path = format!("{}/{}.openapi.{}", domain_id, name, extension);

        // Calculate file size
        let file_size = content.len() as u64;

        Ok(OpenAPIModel::new(
            domain_id, name, file_path, format, file_size,
        ))
    }
}
