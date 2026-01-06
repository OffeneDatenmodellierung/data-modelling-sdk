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
    /// Performs structural validation of OpenAPI 3.x specifications including:
    /// - JSON/YAML parsing validation
    /// - Required fields validation (openapi version, info, paths)
    /// - Schema validation against OpenAPI JSON Schema (when schema-validation feature is enabled)
    ///
    /// # Arguments
    ///
    /// * `content` - The OpenAPI content as a string.
    /// * `format` - The format of the content.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether validation succeeded.
    pub fn validate(&self, content: &str, format: OpenAPIFormat) -> Result<()> {
        // Parse content based on format
        let spec: serde_json::Value = match format {
            OpenAPIFormat::Json => {
                serde_json::from_str(content).context("Failed to parse OpenAPI JSON content")?
            }
            OpenAPIFormat::Yaml => {
                serde_yaml::from_str(content).context("Failed to parse OpenAPI YAML content")?
            }
        };

        // Validate required fields
        let obj = spec
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("OpenAPI spec must be an object"))?;

        // Check for openapi version field
        let openapi_version = obj
            .get("openapi")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'openapi' version field"))?;

        // Validate version format (should be 3.x.x)
        if !openapi_version.starts_with("3.") {
            return Err(anyhow::anyhow!(
                "Unsupported OpenAPI version '{}'. Only OpenAPI 3.x is supported.",
                openapi_version
            ));
        }

        // Check for info object
        let info = obj
            .get("info")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("Missing required 'info' object"))?;

        // Check for required info fields
        if !info.contains_key("title") {
            return Err(anyhow::anyhow!("Missing required 'info.title' field"));
        }

        if !info.contains_key("version") {
            return Err(anyhow::anyhow!("Missing required 'info.version' field"));
        }

        // Check for paths or webhooks (at least one should be present for valid API)
        let has_paths = obj.get("paths").is_some();
        let has_webhooks = obj.get("webhooks").is_some();
        let has_components = obj.get("components").is_some();

        if !has_paths && !has_webhooks && !has_components {
            tracing::warn!(
                "OpenAPI spec does not contain 'paths', 'webhooks', or 'components' - this may be incomplete"
            );
        }

        // Schema validation using jsonschema (if feature enabled)
        #[cfg(all(feature = "schema-validation", feature = "openapi"))]
        {
            use jsonschema::Validator;

            let schema_content = include_str!("../../schemas/openapi-3.1.1.json");
            let schema: serde_json::Value = serde_json::from_str(schema_content)
                .context("Failed to load OpenAPI JSON Schema")?;

            // Only validate if the schema is not a placeholder
            if schema
                .get("properties")
                .is_some_and(|p| p.as_object().is_some_and(|obj| obj.len() > 1))
            {
                let validator =
                    Validator::new(&schema).context("Failed to compile OpenAPI JSON Schema")?;

                if let Err(error) = validator.validate(&spec) {
                    return Err(anyhow::anyhow!(
                        "OpenAPI schema validation failed: {}",
                        error
                    ));
                }
            }
        }

        Ok(())
    }

    /// Extract metadata from OpenAPI content
    ///
    /// Extracts information including:
    /// - API title and version
    /// - Description and contact info
    /// - Server URLs
    /// - Path count and operation count
    /// - Security schemes
    /// - Component schemas count
    ///
    /// # Arguments
    ///
    /// * `content` - The OpenAPI content as a string.
    /// * `format` - The format of the content.
    ///
    /// # Returns
    ///
    /// A `HashMap` containing extracted metadata.
    pub fn extract_metadata(
        &self,
        content: &str,
        format: OpenAPIFormat,
    ) -> HashMap<String, serde_json::Value> {
        use serde_json::json;

        let mut metadata = HashMap::new();

        // Parse content
        let spec: serde_json::Value = match format {
            OpenAPIFormat::Json => match serde_json::from_str(content) {
                Ok(v) => v,
                Err(_) => return metadata,
            },
            OpenAPIFormat::Yaml => match serde_yaml::from_str(content) {
                Ok(v) => v,
                Err(_) => return metadata,
            },
        };

        let obj = match spec.as_object() {
            Some(o) => o,
            None => return metadata,
        };

        // Extract OpenAPI version
        if let Some(version) = obj.get("openapi").and_then(|v| v.as_str()) {
            metadata.insert("openapiVersion".to_string(), json!(version));
        }

        // Extract info section
        if let Some(info) = obj.get("info").and_then(|v| v.as_object()) {
            if let Some(title) = info.get("title").and_then(|v| v.as_str()) {
                metadata.insert("title".to_string(), json!(title));
            }

            if let Some(version) = info.get("version").and_then(|v| v.as_str()) {
                metadata.insert("apiVersion".to_string(), json!(version));
            }

            if let Some(description) = info.get("description").and_then(|v| v.as_str()) {
                metadata.insert("description".to_string(), json!(description));
            }

            if let Some(contact) = info.get("contact").and_then(|v| v.as_object()) {
                let mut contact_info = serde_json::Map::new();
                if let Some(name) = contact.get("name").and_then(|v| v.as_str()) {
                    contact_info.insert("name".to_string(), json!(name));
                }
                if let Some(email) = contact.get("email").and_then(|v| v.as_str()) {
                    contact_info.insert("email".to_string(), json!(email));
                }
                if let Some(url) = contact.get("url").and_then(|v| v.as_str()) {
                    contact_info.insert("url".to_string(), json!(url));
                }
                if !contact_info.is_empty() {
                    metadata.insert(
                        "contact".to_string(),
                        serde_json::Value::Object(contact_info),
                    );
                }
            }

            if let Some(license) = info.get("license").and_then(|v| v.as_object()) {
                let mut license_info = serde_json::Map::new();
                if let Some(name) = license.get("name").and_then(|v| v.as_str()) {
                    license_info.insert("name".to_string(), json!(name));
                }
                if let Some(url) = license.get("url").and_then(|v| v.as_str()) {
                    license_info.insert("url".to_string(), json!(url));
                }
                if !license_info.is_empty() {
                    metadata.insert(
                        "license".to_string(),
                        serde_json::Value::Object(license_info),
                    );
                }
            }
        }

        // Extract servers
        if let Some(servers) = obj.get("servers").and_then(|v| v.as_array()) {
            let server_urls: Vec<serde_json::Value> = servers
                .iter()
                .filter_map(|s| {
                    s.as_object().map(|server| {
                        let mut server_info = serde_json::Map::new();
                        if let Some(url) = server.get("url").and_then(|v| v.as_str()) {
                            server_info.insert("url".to_string(), json!(url));
                        }
                        if let Some(desc) = server.get("description").and_then(|v| v.as_str()) {
                            server_info.insert("description".to_string(), json!(desc));
                        }
                        serde_json::Value::Object(server_info)
                    })
                })
                .collect();

            if !server_urls.is_empty() {
                metadata.insert("servers".to_string(), json!(server_urls));
            }
        }

        // Count paths and operations
        let mut path_count = 0;
        let mut operation_count = 0;
        let mut operations_by_method: HashMap<String, i32> = HashMap::new();

        if let Some(paths) = obj.get("paths").and_then(|v| v.as_object()) {
            path_count = paths.len();

            for (_path, path_item) in paths {
                if let Some(item) = path_item.as_object() {
                    for method in &[
                        "get", "post", "put", "delete", "patch", "options", "head", "trace",
                    ] {
                        if item.contains_key(*method) {
                            operation_count += 1;
                            *operations_by_method.entry(method.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        metadata.insert("pathCount".to_string(), json!(path_count));
        metadata.insert("operationCount".to_string(), json!(operation_count));

        if !operations_by_method.is_empty() {
            metadata.insert(
                "operationsByMethod".to_string(),
                json!(operations_by_method),
            );
        }

        // Count components
        if let Some(components) = obj.get("components").and_then(|v| v.as_object()) {
            let mut component_counts = serde_json::Map::new();

            if let Some(schemas) = components.get("schemas").and_then(|v| v.as_object()) {
                component_counts.insert("schemas".to_string(), json!(schemas.len()));

                // Extract schema names
                let schema_names: Vec<String> = schemas.keys().cloned().collect();
                if !schema_names.is_empty() {
                    metadata.insert("schemaNames".to_string(), json!(schema_names));
                }
            }

            if let Some(responses) = components.get("responses").and_then(|v| v.as_object()) {
                component_counts.insert("responses".to_string(), json!(responses.len()));
            }

            if let Some(parameters) = components.get("parameters").and_then(|v| v.as_object()) {
                component_counts.insert("parameters".to_string(), json!(parameters.len()));
            }

            if let Some(request_bodies) =
                components.get("requestBodies").and_then(|v| v.as_object())
            {
                component_counts.insert("requestBodies".to_string(), json!(request_bodies.len()));
            }

            if let Some(security_schemes) = components
                .get("securitySchemes")
                .and_then(|v| v.as_object())
            {
                component_counts
                    .insert("securitySchemes".to_string(), json!(security_schemes.len()));

                // Extract security scheme details
                let schemes: Vec<serde_json::Value> = security_schemes
                    .iter()
                    .map(|(name, scheme)| {
                        let mut scheme_info = serde_json::Map::new();
                        scheme_info.insert("name".to_string(), json!(name));
                        if let Some(scheme_type) = scheme
                            .as_object()
                            .and_then(|s| s.get("type"))
                            .and_then(|v| v.as_str())
                        {
                            scheme_info.insert("type".to_string(), json!(scheme_type));
                        }
                        serde_json::Value::Object(scheme_info)
                    })
                    .collect();

                if !schemes.is_empty() {
                    metadata.insert("securitySchemes".to_string(), json!(schemes));
                }
            }

            if !component_counts.is_empty() {
                metadata.insert(
                    "componentCounts".to_string(),
                    serde_json::Value::Object(component_counts),
                );
            }
        }

        // Check for tags
        if let Some(tags) = obj.get("tags").and_then(|v| v.as_array()) {
            let tag_names: Vec<String> = tags
                .iter()
                .filter_map(|t| {
                    t.as_object()
                        .and_then(|o| o.get("name"))
                        .and_then(|v| v.as_str())
                })
                .map(|s| s.to_string())
                .collect();

            if !tag_names.is_empty() {
                metadata.insert("tags".to_string(), json!(tag_names));
            }
        }

        // Check for external docs
        if let Some(external_docs) = obj.get("externalDocs").and_then(|v| v.as_object())
            && let Some(url) = external_docs.get("url").and_then(|v| v.as_str())
        {
            metadata.insert("externalDocsUrl".to_string(), json!(url));
        }

        metadata
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
        let metadata = self.extract_metadata(content, format);

        // Determine API name from metadata or parameter
        let name = api_name
            .map(|s| s.to_string())
            .or_else(|| {
                metadata
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_openapi() {
        let openapi_yaml = r#"
openapi: "3.1.0"
info:
  title: Test API
  version: "1.0.0"
paths:
  /users:
    get:
      summary: Get users
      responses:
        "200":
          description: Success
"#;

        let importer = OpenAPIImporter::new();
        let format = importer.detect_format(openapi_yaml);
        assert!(importer.validate(openapi_yaml, format).is_ok());
    }

    #[test]
    fn test_validate_missing_openapi_version() {
        let openapi_yaml = r#"
info:
  title: Test API
  version: "1.0.0"
paths: {}
"#;

        let importer = OpenAPIImporter::new();
        let format = importer.detect_format(openapi_yaml);
        let result = importer.validate(openapi_yaml, format);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("openapi"));
    }

    #[test]
    fn test_validate_unsupported_version() {
        let openapi_yaml = r#"
openapi: "2.0"
info:
  title: Test API
  version: "1.0.0"
paths: {}
"#;

        let importer = OpenAPIImporter::new();
        let format = importer.detect_format(openapi_yaml);
        let result = importer.validate(openapi_yaml, format);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported OpenAPI version")
        );
    }

    #[test]
    fn test_extract_metadata() {
        let openapi_yaml = r#"
openapi: "3.1.0"
info:
  title: Pet Store API
  version: "2.0.0"
  description: A sample API for pet stores
  contact:
    name: API Support
    email: support@example.com
  license:
    name: MIT
servers:
  - url: https://api.example.com/v2
    description: Production server
paths:
  /pets:
    get:
      summary: List pets
    post:
      summary: Create pet
  /pets/{id}:
    get:
      summary: Get pet
    delete:
      summary: Delete pet
components:
  schemas:
    Pet:
      type: object
    Error:
      type: object
  securitySchemes:
    api_key:
      type: apiKey
tags:
  - name: pets
  - name: store
"#;

        let importer = OpenAPIImporter::new();
        let format = importer.detect_format(openapi_yaml);
        let metadata = importer.extract_metadata(openapi_yaml, format);

        assert_eq!(
            metadata.get("title").and_then(|v| v.as_str()),
            Some("Pet Store API")
        );
        assert_eq!(
            metadata.get("apiVersion").and_then(|v| v.as_str()),
            Some("2.0.0")
        );
        assert_eq!(
            metadata.get("openapiVersion").and_then(|v| v.as_str()),
            Some("3.1.0")
        );
        assert_eq!(metadata.get("pathCount").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(
            metadata.get("operationCount").and_then(|v| v.as_i64()),
            Some(4)
        );

        // Check component counts
        let component_counts = metadata.get("componentCounts").and_then(|v| v.as_object());
        assert!(component_counts.is_some());
        let counts = component_counts.unwrap();
        assert_eq!(counts.get("schemas").and_then(|v| v.as_i64()), Some(2));
        assert_eq!(
            counts.get("securitySchemes").and_then(|v| v.as_i64()),
            Some(1)
        );

        // Check tags
        let tags = metadata.get("tags").and_then(|v| v.as_array());
        assert!(tags.is_some());
        assert_eq!(tags.unwrap().len(), 2);
    }

    #[test]
    fn test_detect_format_json() {
        let json_content = r#"{"openapi": "3.1.0", "info": {"title": "Test", "version": "1.0"}}"#;
        let importer = OpenAPIImporter::new();
        assert!(matches!(
            importer.detect_format(json_content),
            OpenAPIFormat::Json
        ));
    }

    #[test]
    fn test_detect_format_yaml() {
        let yaml_content = r#"
openapi: "3.1.0"
info:
  title: Test
  version: "1.0"
"#;
        let importer = OpenAPIImporter::new();
        assert!(matches!(
            importer.detect_format(yaml_content),
            OpenAPIFormat::Yaml
        ));
    }
}
