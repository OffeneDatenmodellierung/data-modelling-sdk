//! OpenAPI to ODCS converter
//!
//! Provides functionality to convert OpenAPI schema components to ODCS table definitions.

use crate::convert::ConversionError;
use crate::models::{Column, Table};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Strategy for handling nested objects in OpenAPI schemas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum NestedObjectStrategy {
    /// Create separate tables for nested objects
    SeparateTables,
    /// Flatten nested objects into parent table
    Flatten,
    /// Hybrid: flatten simple objects, separate complex ones
    Hybrid,
}

/// Type mapping rule for OpenAPI to ODCS conversion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TypeMappingRule {
    /// OpenAPI type
    pub openapi_type: String,
    /// OpenAPI format (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openapi_format: Option<String>,
    /// ODCS type
    pub odcs_type: String,
    /// Quality rules to apply
    #[serde(default)]
    pub quality_rules: Vec<serde_json::Value>,
    /// Field name pattern (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_name: Option<String>,
}

/// Conversion report for OpenAPI to ODCS conversion
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConversionReport {
    /// Component name in OpenAPI
    pub component_name: String,
    /// Generated table name in ODCS
    pub table_name: String,
    /// Field mappings
    #[serde(default)]
    pub mappings: Vec<TypeMappingRule>,
    /// Warnings during conversion
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Fields that were skipped
    #[serde(default)]
    pub skipped_fields: Vec<String>,
    /// Estimated structure (for nested objects)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_structure: Option<HashMap<String, serde_json::Value>>,
}

/// OpenAPI to ODCS Converter
///
/// Converts OpenAPI schema components to ODCS table definitions.
#[derive(Debug)]
pub struct OpenAPIToODCSConverter {
    /// Strategy for handling nested objects
    pub nested_object_strategy: NestedObjectStrategy,
    /// Whether to flatten simple nested objects
    pub flatten_simple_objects: bool,
}

impl Default for OpenAPIToODCSConverter {
    fn default() -> Self {
        Self {
            nested_object_strategy: NestedObjectStrategy::Hybrid,
            flatten_simple_objects: true,
        }
    }
}

impl OpenAPIToODCSConverter {
    /// Create a new OpenAPI to ODCS converter with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new OpenAPI to ODCS converter with custom strategy
    pub fn with_strategy(nested_object_strategy: NestedObjectStrategy) -> Self {
        OpenAPIToODCSConverter {
            nested_object_strategy,
            flatten_simple_objects: matches!(
                nested_object_strategy,
                NestedObjectStrategy::Flatten | NestedObjectStrategy::Hybrid
            ),
        }
    }

    /// Convert an OpenAPI component to an ODCS table
    ///
    /// # Arguments
    ///
    /// * `openapi_content` - The OpenAPI YAML or JSON content.
    /// * `component_name` - The name of the OpenAPI component to convert.
    /// * `table_name` - Optional desired ODCS table name (uses component_name if None).
    ///
    /// # Returns
    ///
    /// A `Result` containing the converted ODCS Table.
    pub fn convert_component(
        &self,
        openapi_content: &str,
        component_name: &str,
        table_name: Option<&str>,
    ) -> Result<Table, ConversionError> {
        // Parse OpenAPI content
        let openapi_value: JsonValue = if openapi_content.trim_start().starts_with('{') {
            serde_json::from_str(openapi_content).map_err(|e| {
                ConversionError::OpenAPISchemaInvalid(format!("Invalid JSON: {}", e))
            })?
        } else {
            serde_yaml::from_str(openapi_content).map_err(|e| {
                ConversionError::OpenAPISchemaInvalid(format!("Invalid YAML: {}", e))
            })?
        };

        // Extract components section
        let components = openapi_value
            .get("components")
            .and_then(|v| v.get("schemas"))
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                ConversionError::OpenAPIComponentNotFound(
                    "components.schemas section not found".to_string(),
                )
            })?;

        // Get the component schema
        let component_schema = components
            .get(component_name)
            .ok_or_else(|| {
                ConversionError::OpenAPIComponentNotFound(format!(
                    "Component '{}' not found in schemas",
                    component_name
                ))
            })?
            .as_object()
            .ok_or_else(|| {
                ConversionError::OpenAPISchemaInvalid(format!(
                    "Component '{}' is not an object",
                    component_name
                ))
            })?;

        // Determine table name
        let target_table_name = table_name.unwrap_or(component_name);

        // Convert schema to table
        self.convert_schema_to_table(component_schema, target_table_name, component_name)
    }

    /// Convert an OpenAPI schema object to an ODCS table
    fn convert_schema_to_table(
        &self,
        schema: &serde_json::Map<String, JsonValue>,
        table_name: &str,
        _component_name: &str,
    ) -> Result<Table, ConversionError> {
        let mut columns = Vec::new();
        let mut warnings = Vec::new();

        // Get properties
        if let Some(properties) = schema.get("properties").and_then(|v| v.as_object()) {
            // Get required fields
            let required_fields: Vec<&str> = schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();

            // Convert each property to a column
            for (field_name, field_schema) in properties {
                match self.convert_field_to_column(field_name, field_schema, &required_fields) {
                    Ok(column) => columns.push(column),
                    Err(e) => {
                        warnings.push(format!("Failed to convert field '{}': {}", field_name, e));
                    }
                }
            }
        } else {
            return Err(ConversionError::OpenAPISchemaInvalid(
                "Schema has no properties".to_string(),
            ));
        }

        // Create table
        let table = Table::new(table_name.to_string(), columns);
        Ok(table)
    }

    /// Convert an OpenAPI field schema to an ODCS column
    fn convert_field_to_column(
        &self,
        field_name: &str,
        field_schema: &JsonValue,
        required_fields: &[&str],
    ) -> Result<Column, ConversionError> {
        let schema_obj = field_schema.as_object().ok_or_else(|| {
            ConversionError::OpenAPISchemaInvalid("Field schema is not an object".to_string())
        })?;

        // Determine if field is required
        let nullable = !required_fields.contains(&field_name);

        // Get type
        let openapi_type = schema_obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ConversionError::OpenAPISchemaInvalid(format!("Field '{}' has no type", field_name))
            })?;

        // Get format
        let format = schema_obj.get("format").and_then(|v| v.as_str());

        // Map OpenAPI type to ODCS type
        let odcs_type = self.map_openapi_type_to_odcs(openapi_type, format)?;

        // Create column
        let mut column = Column::new(field_name.to_string(), odcs_type.clone());
        column.nullable = nullable;

        // Get description
        if let Some(desc) = schema_obj.get("description").and_then(|v| v.as_str()) {
            column.description = desc.to_string();
        }

        // Add quality rules for constraints
        self.add_constraints_to_column(&mut column, schema_obj, openapi_type, format)?;

        Ok(column)
    }

    /// Map OpenAPI type to ODCS type
    fn map_openapi_type_to_odcs(
        &self,
        openapi_type: &str,
        format: Option<&str>,
    ) -> Result<String, ConversionError> {
        match (openapi_type, format) {
            ("string", Some("date")) => Ok("date".to_string()),
            ("string", Some("date-time")) => Ok("timestamp".to_string()),
            ("string", Some("email")) => Ok("text".to_string()),
            ("string", Some("uri")) => Ok("text".to_string()),
            ("string", Some("uuid")) => Ok("text".to_string()),
            ("string", Some("password")) => Ok("text".to_string()),
            ("string", _) => Ok("text".to_string()),
            ("integer", _) => Ok("long".to_string()),
            ("number", _) => Ok("double".to_string()),
            ("boolean", _) => Ok("boolean".to_string()),
            ("array", _) => Err(ConversionError::NestedObjectConversionFailed(
                "Arrays require special handling - not yet implemented".to_string(),
            )),
            ("object", _) => Err(ConversionError::NestedObjectConversionFailed(
                "Nested objects require special handling - not yet implemented".to_string(),
            )),
            _ => Err(ConversionError::UnsupportedFormat(format!(
                "Unsupported OpenAPI type: {}",
                openapi_type
            ))),
        }
    }

    /// Add constraints from OpenAPI schema to column quality rules
    fn add_constraints_to_column(
        &self,
        column: &mut Column,
        schema_obj: &serde_json::Map<String, JsonValue>,
        openapi_type: &str,
        format: Option<&str>,
    ) -> Result<(), ConversionError> {
        let mut quality_rules = Vec::new();

        // Add format constraint
        if let Some(fmt) = format {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), JsonValue::String("text".to_string()));
            rule.insert(
                "description".to_string(),
                JsonValue::String(format!("Format: {}", fmt)),
            );
            rule.insert("format".to_string(), JsonValue::String(fmt.to_string()));
            quality_rules.push(rule);
        }

        // Add minLength/maxLength for strings
        if openapi_type == "string" {
            if let Some(min_len) = schema_obj.get("minLength").and_then(|v| v.as_u64()) {
                let mut rule = HashMap::new();
                rule.insert("type".to_string(), JsonValue::String("text".to_string()));
                rule.insert("minLength".to_string(), JsonValue::Number(min_len.into()));
                quality_rules.push(rule);
            }
            if let Some(max_len) = schema_obj.get("maxLength").and_then(|v| v.as_u64()) {
                let mut rule = HashMap::new();
                rule.insert("type".to_string(), JsonValue::String("text".to_string()));
                rule.insert("maxLength".to_string(), JsonValue::Number(max_len.into()));
                quality_rules.push(rule);
            }
            if let Some(pattern) = schema_obj.get("pattern").and_then(|v| v.as_str()) {
                let mut rule = HashMap::new();
                rule.insert("type".to_string(), JsonValue::String("text".to_string()));
                rule.insert(
                    "pattern".to_string(),
                    JsonValue::String(pattern.to_string()),
                );
                quality_rules.push(rule);
            }
        }

        // Add minimum/maximum for numbers
        if openapi_type == "integer" || openapi_type == "number" {
            if let Some(min_val) = schema_obj.get("minimum")
                && let Some(min_num) = min_val.as_number()
            {
                let mut rule = HashMap::new();
                rule.insert("type".to_string(), JsonValue::String("sql".to_string()));
                rule.insert(
                    "mustBeGreaterThan".to_string(),
                    JsonValue::Number(min_num.clone()),
                );
                quality_rules.push(rule);
            }
            if let Some(max_val) = schema_obj.get("maximum")
                && let Some(max_num) = max_val.as_number()
            {
                let mut rule = HashMap::new();
                rule.insert("type".to_string(), JsonValue::String("sql".to_string()));
                rule.insert(
                    "mustBeLessThan".to_string(),
                    JsonValue::Number(max_num.clone()),
                );
                quality_rules.push(rule);
            }
        }

        // Add enum values
        if let Some(enum_values) = schema_obj.get("enum").and_then(|v| v.as_array()) {
            let enum_strings: Vec<String> = enum_values
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !enum_strings.is_empty() {
                column.enum_values = enum_strings;
            }
        }

        column.quality = quality_rules;
        Ok(())
    }

    /// Convert multiple OpenAPI components to ODCS tables
    ///
    /// # Arguments
    ///
    /// * `openapi_content` - The OpenAPI YAML or JSON content.
    /// * `component_names` - Names of components to convert.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of converted ODCS Tables.
    pub fn convert_components(
        &self,
        openapi_content: &str,
        component_names: &[&str],
    ) -> Result<Vec<Table>, ConversionError> {
        let mut tables = Vec::new();
        for component_name in component_names {
            match self.convert_component(openapi_content, component_name, None) {
                Ok(table) => tables.push(table),
                Err(e) => {
                    return Err(ConversionError::OpenAPIToODCSError(format!(
                        "Failed to convert component '{}': {}",
                        component_name, e
                    )));
                }
            }
        }
        Ok(tables)
    }

    /// Analyze an OpenAPI component for conversion feasibility
    ///
    /// # Arguments
    ///
    /// * `openapi_content` - The OpenAPI YAML or JSON content.
    /// * `component_name` - The name of the OpenAPI component to analyze.
    ///
    /// # Returns
    ///
    /// A `Result` containing a conversion report with analysis.
    pub fn analyze_conversion(
        &self,
        openapi_content: &str,
        component_name: &str,
    ) -> Result<ConversionReport, ConversionError> {
        // Parse OpenAPI content
        let openapi_value: JsonValue = if openapi_content.trim_start().starts_with('{') {
            serde_json::from_str(openapi_content).map_err(|e| {
                ConversionError::OpenAPISchemaInvalid(format!("Invalid JSON: {}", e))
            })?
        } else {
            serde_yaml::from_str(openapi_content).map_err(|e| {
                ConversionError::OpenAPISchemaInvalid(format!("Invalid YAML: {}", e))
            })?
        };

        // Extract components section
        let components = openapi_value
            .get("components")
            .and_then(|v| v.get("schemas"))
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                ConversionError::OpenAPIComponentNotFound(
                    "components.schemas section not found".to_string(),
                )
            })?;

        // Get the component schema
        let component_schema = components
            .get(component_name)
            .ok_or_else(|| {
                ConversionError::OpenAPIComponentNotFound(format!(
                    "Component '{}' not found in schemas",
                    component_name
                ))
            })?
            .as_object()
            .ok_or_else(|| {
                ConversionError::OpenAPISchemaInvalid(format!(
                    "Component '{}' is not an object",
                    component_name
                ))
            })?;

        // Analyze the schema
        let mut mappings = Vec::new();
        let mut warnings = Vec::new();
        let mut skipped_fields = Vec::new();

        if let Some(properties) = component_schema
            .get("properties")
            .and_then(|v| v.as_object())
        {
            for (field_name, field_schema) in properties {
                if let Some(schema_obj) = field_schema.as_object() {
                    let openapi_type = schema_obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    let format = schema_obj.get("format").and_then(|v| v.as_str());

                    match self.map_openapi_type_to_odcs(openapi_type, format) {
                        Ok(odcs_type) => {
                            mappings.push(TypeMappingRule {
                                openapi_type: openapi_type.to_string(),
                                openapi_format: format.map(|s| s.to_string()),
                                odcs_type: odcs_type.clone(),
                                quality_rules: Vec::new(), // Simplified for analysis
                                field_name: Some(field_name.clone()),
                            });
                        }
                        Err(e) => {
                            warnings.push(format!("Field '{}': {}", field_name, e));
                            skipped_fields.push(field_name.clone());
                        }
                    }
                }
            }
        }

        Ok(ConversionReport {
            component_name: component_name.to_string(),
            table_name: component_name.to_string(),
            mappings,
            warnings,
            skipped_fields,
            estimated_structure: None,
        })
    }
}
