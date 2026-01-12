//! Output validation for LLM-refined schemas
//!
//! This module provides validation logic to ensure that LLM-refined schemas
//! maintain compatibility with the original inferred schema and only make
//! safe, additive changes.

use std::collections::HashSet;

use serde_json::Value;

use super::error::{LlmError, LlmResult};

/// Result of schema validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether the validation passed
    pub is_valid: bool,
    /// List of validation errors
    pub errors: Vec<ValidationError>,
    /// List of warnings (non-fatal issues)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation result
    pub fn success() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation result
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add multiple warnings
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings.extend(warnings);
        self
    }
}

/// Types of validation errors
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// A field was removed from the schema
    FieldRemoved { field: String },
    /// A field was renamed
    FieldRenamed { original: String, renamed: String },
    /// A field's type was changed incompatibly
    TypeChanged {
        field: String,
        original: String,
        refined: String,
    },
    /// The schema structure was fundamentally changed
    StructureChanged { description: String },
    /// Required status was changed incorrectly
    RequiredChanged { field: String, was_required: bool },
    /// Invalid JSON structure
    InvalidStructure { description: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationError::FieldRemoved { field } => {
                write!(f, "Field '{}' was removed from the schema", field)
            }
            ValidationError::FieldRenamed { original, renamed } => {
                write!(f, "Field '{}' was renamed to '{}'", original, renamed)
            }
            ValidationError::TypeChanged {
                field,
                original,
                refined,
            } => {
                write!(
                    f,
                    "Field '{}' type changed incompatibly from '{}' to '{}'",
                    field, original, refined
                )
            }
            ValidationError::StructureChanged { description } => {
                write!(f, "Schema structure changed: {}", description)
            }
            ValidationError::RequiredChanged {
                field,
                was_required,
            } => {
                write!(
                    f,
                    "Field '{}' required status changed (was_required: {})",
                    field, was_required
                )
            }
            ValidationError::InvalidStructure { description } => {
                write!(f, "Invalid schema structure: {}", description)
            }
        }
    }
}

/// Validate a refined schema against the original
///
/// Checks that:
/// 1. No fields were removed
/// 2. No fields were renamed
/// 3. Types were not changed incompatibly
/// 4. Required status was not changed incorrectly
/// 5. Only additive changes were made
pub fn validate_refinement(original: &Value, refined: &Value) -> ValidationResult {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Both must be objects
    let (orig_obj, refined_obj) = match (original.as_object(), refined.as_object()) {
        (Some(o), Some(r)) => (o, r),
        _ => {
            return ValidationResult::failure(vec![ValidationError::InvalidStructure {
                description: "Both schemas must be JSON objects".to_string(),
            }]);
        }
    };

    // Check for JSON Schema structure
    let orig_props = orig_obj.get("properties").and_then(|v| v.as_object());
    let refined_props = refined_obj.get("properties").and_then(|v| v.as_object());

    match (orig_props, refined_props) {
        (Some(orig_p), Some(refined_p)) => {
            // Validate properties
            validate_properties(orig_p, refined_p, "", &mut errors, &mut warnings);
        }
        (None, None) => {
            // Direct object comparison (not JSON Schema format)
            validate_object_fields(orig_obj, refined_obj, "", &mut errors, &mut warnings);
        }
        _ => {
            errors.push(ValidationError::StructureChanged {
                description: "Properties structure mismatch".to_string(),
            });
        }
    }

    // Check required fields
    let orig_required: HashSet<&str> = orig_obj
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let refined_required: HashSet<&str> = refined_obj
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    // Fields that were required but are no longer (dangerous)
    for field in orig_required.difference(&refined_required) {
        errors.push(ValidationError::RequiredChanged {
            field: field.to_string(),
            was_required: true,
        });
    }

    // Fields that are newly required (warning only)
    for field in refined_required.difference(&orig_required) {
        warnings.push(format!(
            "Field '{}' is now marked as required (was optional)",
            field
        ));
    }

    if errors.is_empty() {
        ValidationResult::success().with_warnings(warnings)
    } else {
        ValidationResult::failure(errors).with_warnings(warnings)
    }
}

/// Validate properties between original and refined schemas
fn validate_properties(
    original: &serde_json::Map<String, Value>,
    refined: &serde_json::Map<String, Value>,
    path_prefix: &str,
    errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<String>,
) {
    // Check that all original fields exist
    for (field_name, orig_value) in original {
        let full_path = if path_prefix.is_empty() {
            field_name.clone()
        } else {
            format!("{}.{}", path_prefix, field_name)
        };

        match refined.get(field_name) {
            Some(refined_value) => {
                // Field exists, validate it
                validate_field(
                    field_name,
                    orig_value,
                    refined_value,
                    &full_path,
                    errors,
                    warnings,
                );
            }
            None => {
                // Field was removed
                errors.push(ValidationError::FieldRemoved {
                    field: full_path.clone(),
                });
            }
        }
    }

    // Check for new fields (this is allowed, just log it)
    for field_name in refined.keys() {
        if !original.contains_key(field_name) {
            let full_path = if path_prefix.is_empty() {
                field_name.clone()
            } else {
                format!("{}.{}", path_prefix, field_name)
            };
            warnings.push(format!("New field added: {}", full_path));
        }
    }
}

/// Validate a single field
fn validate_field(
    _field_name: &str,
    original: &Value,
    refined: &Value,
    full_path: &str,
    errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<String>,
) {
    let orig_obj = original.as_object();
    let refined_obj = refined.as_object();

    match (orig_obj, refined_obj) {
        (Some(orig), Some(ref_obj)) => {
            // Check type compatibility
            if let (Some(orig_type), Some(ref_type)) = (orig.get("type"), ref_obj.get("type")) {
                if !is_type_compatible(orig_type, ref_type) {
                    errors.push(ValidationError::TypeChanged {
                        field: full_path.to_string(),
                        original: orig_type.to_string(),
                        refined: ref_type.to_string(),
                    });
                }
            }

            // Check nested properties
            if let (Some(orig_props), Some(ref_props)) = (
                orig.get("properties").and_then(|v| v.as_object()),
                ref_obj.get("properties").and_then(|v| v.as_object()),
            ) {
                validate_properties(orig_props, ref_props, full_path, errors, warnings);
            }

            // Check for added metadata (descriptions, formats, etc.)
            if ref_obj.get("description").is_some() && orig.get("description").is_none() {
                warnings.push(format!("Description added to field: {}", full_path));
            }

            if ref_obj.get("format").is_some() && orig.get("format").is_none() {
                warnings.push(format!("Format added to field: {}", full_path));
            }
        }
        _ => {
            // Type mismatch at field level
            if original != refined {
                warnings.push(format!("Field {} value changed", full_path));
            }
        }
    }
}

/// Validate object fields (non-JSON-Schema format)
fn validate_object_fields(
    original: &serde_json::Map<String, Value>,
    refined: &serde_json::Map<String, Value>,
    path_prefix: &str,
    errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<String>,
) {
    for (field_name, orig_value) in original {
        let full_path = if path_prefix.is_empty() {
            field_name.clone()
        } else {
            format!("{}.{}", path_prefix, field_name)
        };

        match refined.get(field_name) {
            Some(refined_value) => {
                // Recursively validate nested objects
                if let (Some(orig_obj), Some(refined_obj)) =
                    (orig_value.as_object(), refined_value.as_object())
                {
                    validate_object_fields(orig_obj, refined_obj, &full_path, errors, warnings);
                }
            }
            None => {
                errors.push(ValidationError::FieldRemoved {
                    field: full_path.clone(),
                });
            }
        }
    }

    // Check for new fields
    for field_name in refined.keys() {
        if !original.contains_key(field_name) {
            let full_path = if path_prefix.is_empty() {
                field_name.clone()
            } else {
                format!("{}.{}", path_prefix, field_name)
            };
            warnings.push(format!("New field added: {}", full_path));
        }
    }
}

/// Check if a type change is compatible (same or narrower)
fn is_type_compatible(original: &Value, refined: &Value) -> bool {
    match (original, refined) {
        // Same type
        (Value::String(o), Value::String(r)) if o == r => true,
        // Array of types - refined should be subset or same
        (Value::Array(orig_types), Value::Array(ref_types)) => {
            ref_types.iter().all(|rt| orig_types.contains(rt))
        }
        // Array narrowed to single type
        (Value::Array(orig_types), Value::String(_)) => orig_types.contains(refined),
        // Single type to array (widening - usually not allowed but check if compatible)
        (Value::String(o), Value::Array(ref_types)) => {
            ref_types.len() == 1 && ref_types.contains(&Value::String(o.clone()))
        }
        // Different single types
        (Value::String(o), Value::String(r)) => {
            // Allow narrowing: "string" can become "string" with format
            // "number" can become "integer"
            o == r || (o == "number" && r == "integer")
        }
        _ => false,
    }
}

/// Convert validation result to LlmResult
impl ValidationResult {
    /// Convert to Result, returning Err if validation failed
    pub fn to_result(&self) -> LlmResult<()> {
        if self.is_valid {
            Ok(())
        } else {
            let error_messages: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
            Err(LlmError::ValidationError(error_messages.join("; ")))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_identical_schemas() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });

        let result = validate_refinement(&schema, &schema);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_added_description() {
        let original = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let refined = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string", "description": "Customer name"}
            }
        });

        let result = validate_refinement(&original, &refined);
        assert!(result.is_valid);
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("Description added"))
        );
    }

    #[test]
    fn test_validate_field_removed() {
        let original = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });

        let refined = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let result = validate_refinement(&original, &refined);
        assert!(!result.is_valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| matches!(e, ValidationError::FieldRemoved { field } if field == "age"))
        );
    }

    #[test]
    fn test_validate_type_changed() {
        let original = json!({
            "type": "object",
            "properties": {
                "count": {"type": "string"}
            }
        });

        let refined = json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        });

        let result = validate_refinement(&original, &refined);
        assert!(!result.is_valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| matches!(e, ValidationError::TypeChanged { .. }))
        );
    }

    #[test]
    fn test_validate_number_to_integer_allowed() {
        let original = json!({
            "type": "object",
            "properties": {
                "count": {"type": "number"}
            }
        });

        let refined = json!({
            "type": "object",
            "properties": {
                "count": {"type": "integer"}
            }
        });

        let result = validate_refinement(&original, &refined);
        assert!(result.is_valid);
    }

    #[test]
    fn test_validate_required_removed() {
        let original = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let refined = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": []
        });

        let result = validate_refinement(&original, &refined);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| matches!(e, ValidationError::RequiredChanged { field, was_required: true } if field == "name")));
    }

    #[test]
    fn test_validate_new_field_warning() {
        let original = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let refined = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "new_field": {"type": "string"}
            }
        });

        let result = validate_refinement(&original, &refined);
        assert!(result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("new_field")));
    }

    #[test]
    fn test_validate_nested_properties() {
        let original = json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"},
                        "city": {"type": "string"}
                    }
                }
            }
        });

        let refined = json!({
            "type": "object",
            "properties": {
                "address": {
                    "type": "object",
                    "properties": {
                        "street": {"type": "string"}
                    }
                }
            }
        });

        let result = validate_refinement(&original, &refined);
        assert!(!result.is_valid);
        assert!(result.errors.iter().any(
            |e| matches!(e, ValidationError::FieldRemoved { field } if field.contains("city"))
        ));
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::FieldRemoved {
            field: "test_field".to_string(),
        };
        assert!(err.to_string().contains("test_field"));
        assert!(err.to_string().contains("removed"));

        let err = ValidationError::TypeChanged {
            field: "count".to_string(),
            original: "string".to_string(),
            refined: "integer".to_string(),
        };
        assert!(err.to_string().contains("count"));
        assert!(err.to_string().contains("string"));
        assert!(err.to_string().contains("integer"));
    }

    #[test]
    fn test_validation_result_to_result() {
        let success = ValidationResult::success();
        assert!(success.to_result().is_ok());

        let failure = ValidationResult::failure(vec![ValidationError::FieldRemoved {
            field: "test".to_string(),
        }]);
        assert!(failure.to_result().is_err());
    }

    #[test]
    fn test_is_type_compatible() {
        // Same type
        assert!(is_type_compatible(&json!("string"), &json!("string")));

        // number -> integer (narrowing)
        assert!(is_type_compatible(&json!("number"), &json!("integer")));

        // string -> integer (incompatible)
        assert!(!is_type_compatible(&json!("string"), &json!("integer")));

        // Array narrowed to single type
        assert!(is_type_compatible(
            &json!(["string", "null"]),
            &json!("string")
        ));
    }
}
