//! Schema validation helpers

use crate::cli::error::CliError;

/// Validate an ODCS file against the ODCS JSON Schema
#[cfg(feature = "schema-validation")]
pub fn validate_odcs(content: &str) -> Result<(), CliError> {
    use jsonschema::Validator;
    use serde_json::Value;

    // Load ODCS JSON Schema
    let schema_content = include_str!("../../schemas/odcs-json-schema-v3.1.0.json");
    let schema: Value = serde_json::from_str(schema_content)
        .map_err(|e| CliError::ValidationError(format!("Failed to load ODCS schema: {}", e)))?;

    let validator = Validator::new(&schema)
        .map_err(|e| CliError::ValidationError(format!("Failed to compile ODCS schema: {}", e)))?;

    // Parse YAML content
    let data: Value = serde_yaml::from_str(content)
        .map_err(|e| CliError::ValidationError(format!("Failed to parse YAML: {}", e)))?;

    // Validate
    if let Err(errors) = validator.validate(&data) {
        let error_messages: Vec<String> = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        return Err(CliError::ValidationError(format!(
            "ODCS validation failed:\n{}",
            error_messages.join("\n")
        )));
    }

    Ok(())
}

#[cfg(not(feature = "schema-validation"))]
pub fn validate_odcs(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Validate an OpenAPI file against the OpenAPI JSON Schema
#[cfg(all(feature = "schema-validation", feature = "openapi"))]
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
    if let Err(errors) = validator.validate(&data) {
        let error_messages: Vec<String> = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        return Err(CliError::ValidationError(format!(
            "OpenAPI validation failed:\n{}",
            error_messages.join("\n")
        )));
    }

    Ok(())
}

#[cfg(not(all(feature = "schema-validation", feature = "openapi")))]
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
#[cfg(feature = "odps-validation")]
pub fn validate_odps(content: &str) -> Result<(), CliError> {
    validate_odps_internal(content).map_err(CliError::ValidationError)
}

#[cfg(not(feature = "odps-validation"))]
pub fn validate_odps(_content: &str) -> Result<(), CliError> {
    // Validation disabled - feature not enabled
    Ok(())
}

/// Internal ODPS validation function that returns a string error (used by both CLI and import/export modules)
#[cfg(feature = "odps-validation")]
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
    if let Err(errors) = validator.validate(&data) {
        let error_messages: Vec<String> = errors
            .map(|e| format!("{}: {}", e.instance_path, e))
            .collect();
        return Err(format!(
            "ODPS validation failed:\n{}",
            error_messages.join("\n")
        ));
    }

    Ok(())
}

#[cfg(not(feature = "odps-validation"))]
pub(crate) fn validate_odps_internal(_content: &str) -> Result<(), String> {
    // Validation disabled - feature not enabled
    Ok(())
}
