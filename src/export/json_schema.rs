//! JSON Schema exporter for generating JSON Schema from data models.

use super::{ExportError, ExportResult};
use crate::models::{Column, DataModel, Table};
use serde_json::{Value, json};

/// Extract $ref path from column relationships.
/// Returns the first foreignKey relationship as a $ref path.
fn get_ref_path_from_relationships(column: &Column) -> Option<String> {
    column.relationships.iter().find_map(|rel| {
        if rel.relationship_type == "foreignKey" {
            // Convert back to $ref format
            Some(format!("#/{}", rel.to))
        } else {
            None
        }
    })
}

/// Exporter for JSON Schema format.
pub struct JSONSchemaExporter;

impl JSONSchemaExporter {
    /// Export tables to JSON Schema format (SDK interface).
    ///
    /// # Arguments
    ///
    /// * `tables` - Slice of tables to export
    ///
    /// # Returns
    ///
    /// An `ExportResult` containing JSON Schema with all tables in the `definitions` section.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::json_schema::JSONSchemaExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let tables = vec![
    ///     Table::new("User".to_string(), vec![Column::new("id".to_string(), "INTEGER".to_string())]),
    /// ];
    ///
    /// let exporter = JSONSchemaExporter;
    /// let result = exporter.export(&tables).unwrap();
    /// assert_eq!(result.format, "json_schema");
    /// assert!(result.content.contains("\"definitions\""));
    /// ```
    pub fn export(&self, tables: &[Table]) -> Result<ExportResult, ExportError> {
        let schema = Self::export_model_from_tables(tables);
        let content = serde_json::to_string_pretty(&schema)
            .map_err(|e| ExportError::SerializationError(e.to_string()))?;

        // Validate exported JSON Schema (if feature enabled)
        #[cfg(feature = "schema-validation")]
        {
            #[cfg(feature = "cli")]
            {
                use crate::cli::validation::validate_json_schema;
                validate_json_schema(&content).map_err(|e| {
                    ExportError::ValidationError(format!("JSON Schema validation failed: {}", e))
                })?;
            }
            #[cfg(not(feature = "cli"))]
            {
                // Inline validation when CLI feature is not enabled
                use jsonschema::Validator;
                use serde_json::Value;

                let schema_value: Value = serde_json::from_str(&content).map_err(|e| {
                    ExportError::ValidationError(format!("Failed to parse JSON Schema: {}", e))
                })?;

                // Try to compile the schema (this validates the schema itself)
                Validator::new(&schema_value).map_err(|e| {
                    ExportError::ValidationError(format!("Invalid JSON Schema: {}", e))
                })?;
            }
        }

        Ok(ExportResult {
            content,
            format: "json_schema".to_string(),
        })
    }

    fn export_model_from_tables(tables: &[Table]) -> serde_json::Value {
        let mut definitions = serde_json::Map::new();
        for table in tables {
            let schema = Self::export_table(table);
            definitions.insert(table.name.clone(), schema);
        }
        let mut root = serde_json::Map::new();
        root.insert(
            "$schema".to_string(),
            serde_json::json!("http://json-schema.org/draft-07/schema#"),
        );
        root.insert("type".to_string(), serde_json::json!("object"));
        root.insert("definitions".to_string(), serde_json::json!(definitions));
        serde_json::json!(root)
    }

    /// Export a table to JSON Schema format.
    ///
    /// # Arguments
    ///
    /// * `table` - The table to export
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` representing the JSON Schema for the table.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::json_schema::JSONSchemaExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let table = Table::new(
    ///     "User".to_string(),
    ///     vec![Column::new("id".to_string(), "INTEGER".to_string())],
    /// );
    ///
    /// let schema = JSONSchemaExporter::export_table(&table);
    /// assert_eq!(schema["title"], "User");
    /// assert_eq!(schema["type"], "object");
    /// ```
    pub fn export_table(table: &Table) -> Value {
        let mut properties = serde_json::Map::new();

        for column in &table.columns {
            let mut property = serde_json::Map::new();

            // Map data types to JSON Schema types
            let (json_type, format) = Self::map_data_type_to_json_schema(&column.data_type);
            property.insert("type".to_string(), json!(json_type));

            if let Some(fmt) = format {
                property.insert("format".to_string(), json!(fmt));
            }

            if !column.nullable {
                // Note: JSON Schema uses "required" array at schema level
            }

            if !column.description.is_empty() {
                property.insert("description".to_string(), json!(column.description));
            }

            // Export $ref if present (from relationships)
            if let Some(ref_path) = get_ref_path_from_relationships(column) {
                property.insert("$ref".to_string(), json!(ref_path));
            }

            // Export enum values
            if !column.enum_values.is_empty() {
                let enum_vals: Vec<Value> = column
                    .enum_values
                    .iter()
                    .map(|v| {
                        // Try to parse as number or boolean, otherwise use as string
                        if let Ok(num) = v.parse::<i64>() {
                            json!(num)
                        } else if let Ok(num) = v.parse::<f64>() {
                            json!(num)
                        } else if let Ok(b) = v.parse::<bool>() {
                            json!(b)
                        } else if v == "null" {
                            json!(null)
                        } else {
                            json!(v)
                        }
                    })
                    .collect();
                property.insert("enum".to_string(), json!(enum_vals));
            }

            // Export validation keywords from quality rules
            Self::export_validation_keywords(&mut property, column);

            properties.insert(column.name.clone(), json!(property));
        }

        let mut schema = serde_json::Map::new();
        schema.insert(
            "$schema".to_string(),
            json!("http://json-schema.org/draft-07/schema#"),
        );
        schema.insert("type".to_string(), json!("object"));
        schema.insert("title".to_string(), json!(table.name));
        schema.insert("properties".to_string(), json!(properties));

        // Add required fields (non-nullable columns)
        let required: Vec<String> = table
            .columns
            .iter()
            .filter(|c| !c.nullable)
            .map(|c| c.name.clone())
            .collect();

        if !required.is_empty() {
            schema.insert("required".to_string(), json!(required));
        }

        // Add tags if present
        if !table.tags.is_empty() {
            let tags_array: Vec<String> = table.tags.iter().map(|t| t.to_string()).collect();
            schema.insert("tags".to_string(), json!(tags_array));
        }

        json!(schema)
    }

    /// Export a data model to JSON Schema format (legacy method for compatibility).
    pub fn export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> Value {
        let mut definitions = serde_json::Map::new();

        let tables_to_export: Vec<&Table> = if let Some(ids) = table_ids {
            model
                .tables
                .iter()
                .filter(|t| ids.contains(&t.id))
                .collect()
        } else {
            model.tables.iter().collect()
        };

        for table in tables_to_export {
            let schema = Self::export_table(table);
            definitions.insert(table.name.clone(), schema);
        }

        let mut root = serde_json::Map::new();
        root.insert(
            "$schema".to_string(),
            json!("http://json-schema.org/draft-07/schema#"),
        );
        root.insert("title".to_string(), json!(model.name));
        root.insert("type".to_string(), json!("object"));
        root.insert("definitions".to_string(), json!(definitions));

        json!(root)
    }

    /// Map SQL/ODCL data types to JSON Schema types and formats.
    fn map_data_type_to_json_schema(data_type: &str) -> (String, Option<String>) {
        let dt_lower = data_type.to_lowercase();

        match dt_lower.as_str() {
            "int" | "integer" | "bigint" | "smallint" | "tinyint" => ("integer".to_string(), None),
            "float" | "double" | "real" | "decimal" | "numeric" => ("number".to_string(), None),
            "boolean" | "bool" => ("boolean".to_string(), None),
            "date" => ("string".to_string(), Some("date".to_string())),
            "time" => ("string".to_string(), Some("time".to_string())),
            "timestamp" | "datetime" => ("string".to_string(), Some("date-time".to_string())),
            "uuid" => ("string".to_string(), Some("uuid".to_string())),
            "uri" | "url" => ("string".to_string(), Some("uri".to_string())),
            "email" => ("string".to_string(), Some("email".to_string())),
            _ => {
                // Default to string for VARCHAR, TEXT, CHAR, etc.
                ("string".to_string(), None)
            }
        }
    }

    /// Export validation keywords from quality rules to JSON Schema property.
    fn export_validation_keywords(
        property: &mut serde_json::Map<String, Value>,
        column: &crate::models::Column,
    ) {
        for rule in &column.quality {
            // Only process rules that came from JSON Schema (have source="json_schema")
            // or don't have a source field (for backward compatibility)
            let source = rule.get("source").and_then(|v| v.as_str());
            if source.is_some() && source != Some("json_schema") {
                continue;
            }

            if let Some(rule_type) = rule.get("type").and_then(|v| v.as_str()) {
                match rule_type {
                    "pattern" => {
                        if let Some(pattern) = rule.get("pattern").or_else(|| rule.get("value")) {
                            property.insert("pattern".to_string(), pattern.clone());
                        }
                    }
                    "minimum" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("minimum".to_string(), value.clone());
                            if let Some(exclusive) = rule.get("exclusive")
                                && exclusive.as_bool() == Some(true)
                            {
                                property.insert("exclusiveMinimum".to_string(), json!(true));
                            }
                        }
                    }
                    "maximum" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("maximum".to_string(), value.clone());
                            if let Some(exclusive) = rule.get("exclusive")
                                && exclusive.as_bool() == Some(true)
                            {
                                property.insert("exclusiveMaximum".to_string(), json!(true));
                            }
                        }
                    }
                    "minLength" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("minLength".to_string(), value.clone());
                        }
                    }
                    "maxLength" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("maxLength".to_string(), value.clone());
                        }
                    }
                    "multipleOf" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("multipleOf".to_string(), value.clone());
                        }
                    }
                    "const" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("const".to_string(), value.clone());
                        }
                    }
                    "minItems" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("minItems".to_string(), value.clone());
                        }
                    }
                    "maxItems" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("maxItems".to_string(), value.clone());
                        }
                    }
                    "uniqueItems" => {
                        if let Some(value) = rule.get("value")
                            && value.as_bool() == Some(true)
                        {
                            property.insert("uniqueItems".to_string(), json!(true));
                        }
                    }
                    "minProperties" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("minProperties".to_string(), value.clone());
                        }
                    }
                    "maxProperties" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("maxProperties".to_string(), value.clone());
                        }
                    }
                    "additionalProperties" => {
                        if let Some(value) = rule.get("value") {
                            property.insert("additionalProperties".to_string(), value.clone());
                        }
                    }
                    "format" => {
                        // Format is already handled in map_data_type_to_json_schema,
                        // but if it's in quality rules, use it
                        if let Some(value) = rule.get("value").and_then(|v| v.as_str()) {
                            // Only set if not already set
                            if !property.contains_key("format") {
                                property.insert("format".to_string(), json!(value));
                            }
                        }
                    }
                    "allOf" | "anyOf" | "oneOf" | "not" => {
                        // Complex validation keywords
                        if let Some(value) = rule.get("value") {
                            property.insert(rule_type.to_string(), value.clone());
                        }
                    }
                    _ => {
                        // Unknown rule type - preserve as custom property or skip
                        // Could add to a customProperties field if needed
                    }
                }
            }
        }

        // Also handle constraints that might map to JSON Schema
        for constraint in &column.constraints {
            // Try to parse common constraint patterns
            let constraint_upper = constraint.to_uppercase();
            if constraint_upper.contains("UNIQUE") {
                // For string types, uniqueItems doesn't apply, but we could add a custom property
                // For now, skip as JSON Schema doesn't have a direct unique constraint
            } else if constraint_upper.starts_with("CHECK") {
                // CHECK constraints could be preserved as a custom property
                // For now, we'll skip as JSON Schema doesn't have CHECK
            }
        }
    }
}
