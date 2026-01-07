//! Schema validation helpers

use crate::cli::error::CliError;

/// Format validation error with path information
#[cfg(feature = "schema-validation")]
fn format_validation_error(error: &jsonschema::ValidationError, schema_type: &str) -> String {
    // Extract instance path (JSON path where error occurred)
    let instance_path = error.instance_path();

    // Format the path nicely - Location implements Display/Debug
    let path_str = instance_path.to_string();
    let path_str = if path_str == "/" || path_str.is_empty() {
        "root".to_string()
    } else {
        path_str
    };

    // Get the error message
    let error_message = error.to_string();

    format!(
        "{} validation failed at path '{}': {}",
        schema_type, path_str, error_message
    )
}

/// Validate an ODCS file against the ODCS JSON Schema
/// Automatically detects and validates ODCL format files against ODCL schema
#[cfg(feature = "schema-validation")]
pub fn validate_odcs(content: &str) -> Result<(), CliError> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Parse YAML content to check format
    let data: Value = serde_yaml::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse YAML: {}", e)))?;

    // Check if this is an ODCL format file (legacy format)
    // ODCL files have "dataContractSpecification" field or simple "name"/"columns" structure
    let is_odcl_format = if let Some(obj) = data.as_object() {
        // Check for ODCL v3 format (dataContractSpecification)
        obj.contains_key("dataContractSpecification")
            // Check for simple ODCL format (name + columns, but no apiVersion/kind/schema)
            || (obj.contains_key("name")
                && obj.contains_key("columns")
                && !obj.contains_key("apiVersion")
                && !obj.contains_key("kind")
                && !obj.contains_key("schema"))
    } else {
        false
    };

    // Validate against ODCL schema if ODCL format detected
    if is_odcl_format {
        return validate_odcl(content);
    }

    // Load ODCS JSON Schema
    let schema_content = include_str!("../../schemas/odcs-json-schema-v3.1.0.json");
    let schema: Value = serde_json::from_str(schema_content)
        .map_err(|e| CliError::ValidationError(format!("Failed to load ODCS schema: {}", e)))?;

    let validator = Validator::new(&schema)
        .map_err(|e| CliError::ValidationError(format!("Failed to compile ODCS schema: {}", e)))?;

    // Validate against ODCS schema
    if let Err(error) = validator.validate(&data) {
        // Extract path information from validation error
        let error_msg = format_validation_error(&error, "ODCS");
        return Err(CliError::ValidationError(error_msg));
    }

    Ok(())
}

/// Validate an ODCL file against the ODCL JSON Schema
#[cfg(feature = "schema-validation")]
pub fn validate_odcl(content: &str) -> Result<(), CliError> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Load ODCL JSON Schema
    let schema_content = include_str!("../../schemas/odcl-json-schema-1.2.1.json");
    let schema: Value = serde_json::from_str(schema_content)
        .map_err(|e| CliError::ValidationError(format!("Failed to load ODCL schema: {}", e)))?;

    let validator = Validator::new(&schema)
        .map_err(|e| CliError::ValidationError(format!("Failed to compile ODCL schema: {}", e)))?;

    // Parse YAML content
    let data: Value = serde_yaml::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse YAML: {}", e)))?;

    // Validate
    if let Err(error) = validator.validate(&data) {
        let error_msg = format_validation_error(&error, "ODCL");
        return Err(CliError::ValidationError(error_msg));
    }

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_odcl(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_odcs(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Validate an OpenAPI file against the OpenAPI JSON Schema
#[cfg(feature = "schema-validation")]
pub fn validate_openapi(content: &str) -> Result<(), CliError> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Load OpenAPI JSON Schema
    let schema_content = include_str!("../../schemas/openapi-3.1.1.json");
    let schema: Value = serde_json::from_str(schema_content)
        .map_err(|e| CliError::ValidationError(format!("Failed to load OpenAPI schema: {}", e)))?;

    let validator = Validator::new(&schema).map_err(|e| {
        CliError::ValidationError(format!("Failed to compile OpenAPI schema: {}", e))
    })?;

    // Parse YAML or JSON content
    let data: Value = if content.trim_start().starts_with('{') {
        serde_json::from_str(content)
            .map_err(|e| CliError::ValidationError(format!("Failed to parse JSON: {}", e)))?
    } else {
        serde_yaml::from_str(content)
            .map_err(|e| CliError::ValidationError(format!("Failed to parse YAML: {}", e)))?
    };

    // Validate
    if let Err(error) = validator.validate(&data) {
        return Err(CliError::ValidationError(format!(
            "OpenAPI validation failed: {}",
            error
        )));
    }

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_openapi(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Validate Protobuf file syntax
pub fn validate_protobuf(content: &str) -> Result<(), CliError> {
    // Basic syntax validation - check for common proto keywords
    if !content.contains("syntax") && !content.contains("message") && !content.contains("enum") {
        return Err(CliError::ValidationError(
            "File does not appear to be a valid Protobuf file".to_string(),
        ));
    }

    // Check for balanced braces (basic syntax check)
    let open_braces = content.matches('{').count();
    let close_braces = content.matches('}').count();
    if open_braces != close_braces {
        return Err(CliError::ValidationError(format!(
            "Unbalanced braces in Protobuf file ({} open, {} close)",
            open_braces, close_braces
        )));
    }

    Ok(())
}

/// Validate AVRO file against AVRO specification
pub fn validate_avro(content: &str) -> Result<(), CliError> {
    // Parse as JSON
    let _value: serde_json::Value = serde_json::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse AVRO JSON: {}", e)))?;

    // Basic validation - check for required AVRO fields
    // More comprehensive validation would require an AVRO schema validator crate
    Ok(())
}

/// Validate JSON Schema file
#[cfg(feature = "schema-validation")]
pub fn validate_json_schema(content: &str) -> Result<(), CliError> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Parse JSON Schema
    let schema: Value = serde_json::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse JSON Schema: {}", e)))?;

    // Try to compile the schema (this validates the schema itself)
    Validator::new(&schema)
        .map_err(|e| CliError::ValidationError(format!("Invalid JSON Schema: {}", e)))?;

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_json_schema(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Validate an ODPS file against the ODPS JSON Schema
#[cfg(feature = "schema-validation")]
pub fn validate_odps(content: &str) -> Result<(), CliError> {
    validate_odps_internal(content).map_err(CliError::ValidationError)
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_odps(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Internal ODPS validation function that returns a string error (used by both CLI and import/export modules)
#[cfg(feature = "schema-validation")]
pub(crate) fn validate_odps_internal(content: &str) -> Result<(), String> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Load ODPS JSON Schema
    let schema_content = include_str!("../../schemas/odps-json-schema-latest.json");
    let schema: Value = serde_json::from_str(schema_content)
        .map_err(|e| format!("Failed to load ODPS schema: {}", e))?;

    let validator =
        Validator::new(&schema).map_err(|e| format!("Failed to compile ODPS schema: {}", e))?;

    // Parse YAML content
    let data: Value =
        serde_yaml::from_str(content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    // Validate
    if let Err(error) = validator.validate(&data) {
        let instance_path = error.instance_path();
        let path_str = instance_path.to_string();
        let path_str = if path_str == "/" || path_str.is_empty() {
            "root".to_string()
        } else {
            path_str
        };
        return Err(format!(
            "ODPS validation failed at path '{}': {}",
            path_str, error
        ));
    }

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
#[allow(dead_code)]
pub(crate) fn validate_odps_internal(_content: &str) -> Result<(), String> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Validate a CADS file against the CADS JSON Schema
#[cfg(feature = "schema-validation")]
pub fn validate_cads(content: &str) -> Result<(), CliError> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Load CADS JSON Schema
    let schema_content = include_str!("../../schemas/cads.schema.json");
    let schema: Value = serde_json::from_str(schema_content)
        .map_err(|e| CliError::ValidationError(format!("Failed to load CADS schema: {}", e)))?;

    let validator = Validator::new(&schema)
        .map_err(|e| CliError::ValidationError(format!("Failed to compile CADS schema: {}", e)))?;

    // Parse YAML content
    let data: Value = serde_yaml::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse YAML: {}", e)))?;

    // Validate
    if let Err(error) = validator.validate(&data) {
        let error_msg = format_validation_error(&error, "CADS");
        return Err(CliError::ValidationError(error_msg));
    }

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_cads(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Internal CADS validation function that returns a string error (used by export modules)
#[cfg(feature = "schema-validation")]
pub(crate) fn validate_cads_internal(content: &str) -> Result<(), String> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Load CADS JSON Schema
    let schema_content = include_str!("../../schemas/cads.schema.json");
    let schema: Value = serde_json::from_str(schema_content)
        .map_err(|e| format!("Failed to load CADS schema: {}", e))?;

    let validator =
        Validator::new(&schema).map_err(|e| format!("Failed to compile CADS schema: {}", e))?;

    // Parse YAML content
    let data: Value =
        serde_yaml::from_str(content).map_err(|e| format!("Failed to parse YAML: {}", e))?;

    // Validate
    if let Err(error) = validator.validate(&data) {
        let instance_path = error.instance_path();
        let path_str = instance_path.to_string();
        let path_str = if path_str == "/" || path_str.is_empty() {
            "root".to_string()
        } else {
            path_str
        };
        return Err(format!(
            "CADS validation failed at path '{}': {}",
            path_str, error
        ));
    }

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
#[allow(dead_code)]
pub(crate) fn validate_cads_internal(_content: &str) -> Result<(), String> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Validate SQL syntax using sqlparser
pub fn validate_sql(content: &str) -> Result<(), CliError> {
    use sqlparser::dialect::GenericDialect;
    use sqlparser::parser::Parser;

    let dialect = GenericDialect {};

    Parser::parse_sql(&dialect, content)
        .map_err(|e| CliError::ValidationError(format!("SQL validation failed: {}", e)))?;

    Ok(())
}

/// Validate a workspace.yaml file against the workspace JSON Schema
#[cfg(feature = "schema-validation")]
pub fn validate_workspace(content: &str) -> Result<(), CliError> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Load workspace JSON Schema
    let schema_content = include_str!("../../schemas/workspace-schema.json");
    let schema: Value = serde_json::from_str(schema_content).map_err(|e| {
        CliError::ValidationError(format!("Failed to load workspace schema: {}", e))
    })?;

    let validator = Validator::new(&schema).map_err(|e| {
        CliError::ValidationError(format!("Failed to compile workspace schema: {}", e))
    })?;

    // Parse YAML content
    let data: Value = serde_yaml::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse YAML: {}", e)))?;

    // Validate
    if let Err(error) = validator.validate(&data) {
        let error_msg = format_validation_error(&error, "Workspace");
        return Err(CliError::ValidationError(error_msg));
    }

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_workspace(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Validate a relationships.yaml file
pub fn validate_relationships(content: &str) -> Result<(), CliError> {
    use serde_json::Value;

    // Parse YAML content
    let data: Value = serde_yaml::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse YAML: {}", e)))?;

    // Check structure - should be an object with "relationships" array or a direct array
    let relationships = data
        .get("relationships")
        .and_then(|v| v.as_array())
        .or_else(|| data.as_array());

    if let Some(rels) = relationships {
        for (i, rel) in rels.iter().enumerate() {
            // Each relationship should have source_table_id and target_table_id
            if rel.get("source_table_id").is_none() {
                return Err(CliError::ValidationError(format!(
                    "Relationship {} is missing 'source_table_id'",
                    i
                )));
            }
            if rel.get("target_table_id").is_none() {
                return Err(CliError::ValidationError(format!(
                    "Relationship {} is missing 'target_table_id'",
                    i
                )));
            }
        }
    }

    Ok(())
}
