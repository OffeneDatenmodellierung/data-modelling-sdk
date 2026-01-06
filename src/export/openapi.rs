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
    /// use data_modelling_sdk::export::openapi::OpenAPIExporter;
    /// use data_modelling_sdk::models::openapi::OpenAPIFormat;
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
                #[cfg(feature = "cli")]
                {
                    use crate::cli::validation::validate_openapi;
                    validate_openapi(content).map_err(|e| {
                        ExportError::ValidationError(format!("OpenAPI validation failed: {}", e))
                    })?;
                }
                #[cfg(not(feature = "cli"))]
                {
                    // Inline validation when CLI feature is not enabled
                    use jsonschema::Validator;
                    use serde_json::Value;

                    let schema_content = include_str!("../../schemas/openapi-3.1.1.json");
                    let schema: Value = serde_json::from_str(schema_content).map_err(|e| {
                        ExportError::ValidationError(format!(
                            "Failed to load OpenAPI schema: {}",
                            e
                        ))
                    })?;

                    let validator = Validator::new(&schema).map_err(|e| {
                        ExportError::ValidationError(format!(
                            "Failed to compile OpenAPI schema: {}",
                            e
                        ))
                    })?;

                    let data: Value = if content.trim_start().starts_with('{') {
                        serde_json::from_str(content).map_err(|e| {
                            ExportError::ValidationError(format!("Failed to parse JSON: {}", e))
                        })?
                    } else {
                        serde_yaml::from_str(content).map_err(|e| {
                            ExportError::ValidationError(format!("Failed to parse YAML: {}", e))
                        })?
                    };

                    if let Err(error) = validator.validate(&data) {
                        return Err(ExportError::ValidationError(format!(
                            "OpenAPI validation failed: {}",
                            error
                        )));
                    }
                }
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
            #[cfg(feature = "cli")]
            {
                use crate::cli::validation::validate_openapi;
                validate_openapi(&result).map_err(|e| {
                    ExportError::ValidationError(format!("OpenAPI validation failed: {}", e))
                })?;
            }
            #[cfg(not(feature = "cli"))]
            {
                // Inline validation when CLI feature is not enabled
                use jsonschema::Validator;
                use serde_json::Value;

                let schema_content = include_str!("../../schemas/openapi-3.1.1.json");
                let schema: Value = serde_json::from_str(schema_content).map_err(|e| {
                    ExportError::ValidationError(format!("Failed to load OpenAPI schema: {}", e))
                })?;

                let validator = Validator::new(&schema).map_err(|e| {
                    ExportError::ValidationError(format!("Failed to compile OpenAPI schema: {}", e))
                })?;

                let data: Value = if result.trim_start().starts_with('{') {
                    serde_json::from_str(&result).map_err(|e| {
                        ExportError::ValidationError(format!("Failed to parse JSON: {}", e))
                    })?
                } else {
                    serde_yaml::from_str(&result).map_err(|e| {
                        ExportError::ValidationError(format!("Failed to parse YAML: {}", e))
                    })?
                };

                if let Err(error) = validator.validate(&data) {
                    return Err(ExportError::ValidationError(format!(
                        "OpenAPI validation failed: {}",
                        error
                    )));
                }
            }
        }

        Ok(result)
    }
}
