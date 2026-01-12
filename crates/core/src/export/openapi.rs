//! OpenAPI exporter
//!
//! Provides functionality to export OpenAPI models in their native YAML or JSON format.

use crate::export::ExportError;
use crate::models::openapi::OpenAPIFormat;
use serde_json::Value as JsonValue;

/// OpenAPI Exporter
///
/// Exports OpenAPI models in their native YAML or JSON format.
#[derive(Debug, Default)]
pub struct OpenAPIExporter;

impl OpenAPIExporter {
    /// Create a new OpenAPIExporter
    pub fn new() -> Self {
        Self
    }

    /// Export OpenAPI model content
    ///
    /// # Arguments
    ///
    /// * `content` - The OpenAPI content as a string (YAML or JSON).
    /// * `source_format` - The format of the source content.
    /// * `target_format` - Optional target format (if conversion needed).
    ///
    /// # Returns
    ///
    /// The content in the requested format.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::export::openapi::OpenAPIExporter;
    /// use data_modelling_core::models::openapi::OpenAPIFormat;
    ///
    /// let exporter = OpenAPIExporter::new();
    /// let yaml_content = r#"openapi: 3.1.0
    /// info:
    ///   title: Test API
    ///   version: 1.0.0"#;
    /// let exported = exporter.export(yaml_content, OpenAPIFormat::Yaml, Some(OpenAPIFormat::Json)).unwrap();
    /// ```
    pub fn export(
        &self,
        content: &str,
        source_format: OpenAPIFormat,
        target_format: Option<OpenAPIFormat>,
    ) -> Result<String, ExportError> {
        // If no target format specified, return content as-is
        let target = target_format.unwrap_or(source_format);

        // If formats match, validate and return content as-is
        if source_format == target {
            // Validate content before returning (if feature enabled)
            #[cfg(all(feature = "schema-validation", feature = "openapi"))]
            {
                use crate::validation::schema::validate_openapi_internal;
                validate_openapi_internal(content).map_err(|e| {
                    ExportError::ValidationError(format!("OpenAPI validation failed: {}", e))
                })?;
            }
            return Ok(content.to_string());
        }

        // Parse source content
        let json_value: JsonValue = match source_format {
            OpenAPIFormat::Yaml => serde_yaml::from_str(content).map_err(|e| {
                ExportError::SerializationError(format!("Failed to parse YAML: {}", e))
            })?,
            OpenAPIFormat::Json => serde_json::from_str(content).map_err(|e| {
                ExportError::SerializationError(format!("Failed to parse JSON: {}", e))
            })?,
        };

        // Convert to target format
        let result = match target {
            OpenAPIFormat::Yaml => serde_yaml::to_string(&json_value).map_err(|e| {
                ExportError::SerializationError(format!("Failed to serialize to YAML: {}", e))
            })?,
            OpenAPIFormat::Json => serde_json::to_string_pretty(&json_value).map_err(|e| {
                ExportError::SerializationError(format!("Failed to serialize to JSON: {}", e))
            })?,
        };

        // Validate exported content against OpenAPI schema (if feature enabled)
        #[cfg(all(feature = "schema-validation", feature = "openapi"))]
        {
            use crate::validation::schema::validate_openapi_internal;
            validate_openapi_internal(&result).map_err(|e| {
                ExportError::ValidationError(format!("OpenAPI validation failed: {}", e))
            })?;
        }

        Ok(result)
    }
}
