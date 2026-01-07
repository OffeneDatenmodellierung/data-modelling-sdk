//! JSON Schema parser for importing JSON Schema into data models.
//!
//! # Validation
//!
//! All imported table and column names are validated for:
//! - Valid identifier format
//! - Maximum length limits

use super::odcs_shared::column_to_column_data;
use super::{ImportError, ImportResult, TableData};
use crate::models::{Column, PropertyRelationship, Table, Tag};
use crate::validation::input::{validate_column_name, validate_data_type, validate_table_name};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, warn};

/// Convert a $ref path to a PropertyRelationship.
fn ref_to_relationships(ref_path: &Option<String>) -> Vec<PropertyRelationship> {
    match ref_path {
        Some(ref_str) => {
            let to = if ref_str.starts_with("#/definitions/") {
                let def_path = ref_str.strip_prefix("#/definitions/").unwrap_or(ref_str);
                format!("definitions/{}", def_path)
            } else if ref_str.starts_with("#/") {
                ref_str.strip_prefix("#/").unwrap_or(ref_str).to_string()
            } else {
                ref_str.clone()
            };
            vec![PropertyRelationship {
                relationship_type: "foreignKey".to_string(),
                to,
            }]
        }
        None => Vec::new(),
    }
}

/// Parser for JSON Schema format.
pub struct JSONSchemaImporter;

impl Default for JSONSchemaImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl JSONSchemaImporter {
    /// Create a new JSON Schema parser instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::json_schema::JSONSchemaImporter;
    ///
    /// let importer = JSONSchemaImporter::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Import JSON Schema content and create Table(s) (SDK interface).
    ///
    /// # Arguments
    ///
    /// * `json_content` - JSON Schema string (can be a single schema or schema with definitions)
    ///
    /// # Returns
    ///
    /// An `ImportResult` containing extracted tables and any parse errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::json_schema::JSONSchemaImporter;
    ///
    /// let importer = JSONSchemaImporter::new();
    /// let schema = r#"
    /// {
    ///   "type": "object",
    ///   "properties": {
    ///     "id": {"type": "integer"},
    ///     "name": {"type": "string"}
    ///   },
    ///   "required": ["id"]
    /// }
    /// "#;
    /// let result = importer.import(schema).unwrap();
    /// ```
    pub fn import(&self, json_content: &str) -> Result<ImportResult, ImportError> {
        match self.parse(json_content) {
            Ok((tables, errors)) => {
                let mut sdk_tables = Vec::new();
                for (idx, table) in tables.iter().enumerate() {
                    sdk_tables.push(TableData {
                        table_index: idx,
                        name: Some(table.name.clone()),
                        columns: table
                            .columns
                            .iter()
                            .map(column_to_column_data)
                            .collect(),
                    });
                }
                let sdk_errors: Vec<ImportError> = errors
                    .iter()
                    .map(|e| ImportError::ParseError(e.message.clone()))
                    .collect();
                Ok(ImportResult {
                    tables: sdk_tables,
                    tables_requiring_name: Vec::new(),
                    errors: sdk_errors,
                    ai_suggestions: None,
                })
            }
            Err(e) => Err(ImportError::ParseError(e.to_string())),
        }
    }

    /// Parse JSON Schema content and create Table(s) (internal method).
    ///
    /// # Returns
    ///
    /// Returns a tuple of (Tables, list of errors/warnings).
    fn parse(&self, json_content: &str) -> Result<(Vec<Table>, Vec<ParserError>)> {
        let mut errors = Vec::new();

        // Parse JSON
        let schema: Value =
            serde_json::from_str(json_content).context("Failed to parse JSON Schema")?;

        let mut tables = Vec::new();

        // Check if it's a schema with definitions (multiple tables)
        if let Some(definitions) = schema.get("definitions").and_then(|v| v.as_object()) {
            // Multiple schemas in definitions
            for (name, def_schema) in definitions {
                match self.parse_schema(def_schema, Some(name), &mut errors) {
                    Ok(table) => tables.push(table),
                    Err(e) => {
                        errors.push(ParserError {
                            error_type: "parse_error".to_string(),
                            field: Some(format!("definitions.{}", name)),
                            message: format!("Failed to parse schema: {}", e),
                        });
                    }
                }
            }
        } else {
            // Single schema
            match self.parse_schema(&schema, None, &mut errors) {
                Ok(table) => tables.push(table),
                Err(e) => {
                    errors.push(ParserError {
                        error_type: "parse_error".to_string(),
                        field: None,
                        message: format!("Failed to parse schema: {}", e),
                    });
                }
            }
        }

        Ok((tables, errors))
    }

    /// Parse a single JSON Schema object.
    fn parse_schema(
        &self,
        schema: &Value,
        name_override: Option<&str>,
        errors: &mut Vec<ParserError>,
    ) -> Result<Table> {
        let schema_obj = schema
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Schema must be an object"))?;

        // Extract name/title
        let name = name_override
            .map(|s| s.to_string())
            .or_else(|| {
                schema_obj
                    .get("title")
                    .or_else(|| schema_obj.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| anyhow::anyhow!("Missing required field: title or name"))?;

        // Validate table name
        if let Err(e) = validate_table_name(&name) {
            warn!("Table name validation warning for '{}': {}", name, e);
        }

        // Extract description
        let description = schema_obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Extract properties
        let properties = schema_obj
            .get("properties")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: properties"))?;

        // Extract required fields
        let required_fields: Vec<String> = schema_obj
            .get("required")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let mut columns = Vec::new();
        for (prop_name, prop_schema) in properties {
            let nullable = !required_fields.contains(prop_name);
            match self.parse_property(prop_name, prop_schema, nullable, errors) {
                Ok(mut cols) => columns.append(&mut cols),
                Err(e) => {
                    errors.push(ParserError {
                        error_type: "parse_error".to_string(),
                        field: Some(format!("properties.{}", prop_name)),
                        message: format!("Failed to parse property: {}", e),
                    });
                }
            }
        }

        // Extract tags from JSON Schema (can be in root or in customProperties)
        let mut tags: Vec<Tag> = Vec::new();
        if let Some(tags_arr) = schema_obj.get("tags").and_then(|v| v.as_array()) {
            for item in tags_arr {
                if let Some(s) = item.as_str() {
                    if let Ok(tag) = Tag::from_str(s) {
                        tags.push(tag);
                    } else {
                        tags.push(Tag::Simple(s.to_string()));
                    }
                }
            }
        }
        // Also check customProperties for tags
        if let Some(custom_props) = schema_obj
            .get("customProperties")
            .and_then(|v| v.as_object())
            && let Some(tags_val) = custom_props.get("tags")
            && let Some(tags_arr) = tags_val.as_array()
        {
            for item in tags_arr {
                if let Some(s) = item.as_str() {
                    if let Ok(tag) = Tag::from_str(s) {
                        if !tags.contains(&tag) {
                            tags.push(tag);
                        }
                    } else {
                        let simple_tag = Tag::Simple(s.to_string());
                        if !tags.contains(&simple_tag) {
                            tags.push(simple_tag);
                        }
                    }
                }
            }
        }

        // Build table metadata
        let mut odcl_metadata = HashMap::new();
        if !description.is_empty() {
            odcl_metadata.insert("description".to_string(), json!(description));
        }

        let table = Table {
            id: crate::models::table::Table::generate_id(&name, None, None, None),
            name: name.clone(),
            columns,
            database_type: None,
            catalog_name: None,
            schema_name: None,
            medallion_layers: Vec::new(),
            scd_pattern: None,
            data_vault_classification: None,
            modeling_level: None,
            tags,
            odcl_metadata,
            owner: None,
            sla: None,
            contact_details: None,
            infrastructure_type: None,
            notes: None,
            position: None,
            yaml_file_path: None,
            drawio_cell_id: None,
            quality: Vec::new(),
            errors: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        info!(
            "Parsed JSON Schema: {} with {} columns",
            name,
            table.columns.len()
        );
        Ok(table)
    }

    /// Parse a JSON Schema property (which can be a simple property or nested object).
    fn parse_property(
        &self,
        prop_name: &str,
        prop_schema: &Value,
        nullable: bool,
        errors: &mut Vec<ParserError>,
    ) -> Result<Vec<Column>> {
        // Validate column name
        if let Err(e) = validate_column_name(prop_name) {
            warn!("Column name validation warning for '{}': {}", prop_name, e);
        }

        let prop_obj = prop_schema
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Property schema must be an object"))?;

        // Handle $ref references
        if let Some(ref_path) = prop_obj.get("$ref").and_then(|v| v.as_str()) {
            // Create column with reference
            let description = prop_obj
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();

            let quality_rules = self.extract_validation_keywords(prop_obj, prop_name);

            return Ok(vec![Column {
                name: prop_name.to_string(),
                data_type: "STRING".to_string(), // Default for $ref, will be resolved later
                nullable,
                description,
                quality: quality_rules,
                relationships: ref_to_relationships(&Some(ref_path.to_string())),
                ..Default::default()
            }]);
        }

        let prop_type = prop_obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Property missing type"))?;

        // Validate data type
        let mapped_type = self.map_json_type_to_sql(prop_type);
        if let Err(e) = validate_data_type(&mapped_type) {
            warn!("Data type validation warning for '{}': {}", mapped_type, e);
        }

        let description = prop_obj
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Extract validation keywords and enum values
        let quality_rules = self.extract_validation_keywords(prop_obj, prop_name);
        let enum_values = self.extract_enum_values(prop_obj);

        let mut columns = Vec::new();

        match prop_type {
            "object" => {
                // Nested object - create nested columns with dot notation
                if let Some(nested_props) = prop_obj.get("properties").and_then(|v| v.as_object()) {
                    let nested_required: Vec<String> = prop_obj
                        .get("required")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    for (nested_name, nested_schema) in nested_props {
                        let nested_nullable = !nested_required.contains(nested_name);
                        match self.parse_property(
                            nested_name,
                            nested_schema,
                            nested_nullable,
                            errors,
                        ) {
                            Ok(mut nested_cols) => {
                                // Prefix nested columns with parent property name
                                for col in nested_cols.iter_mut() {
                                    col.name = format!("{}.{}", prop_name, col.name);
                                }
                                columns.append(&mut nested_cols);
                            }
                            Err(e) => {
                                errors.push(ParserError {
                                    error_type: "parse_error".to_string(),
                                    field: Some(format!("{}.{}", prop_name, nested_name)),
                                    message: format!("Failed to parse nested property: {}", e),
                                });
                            }
                        }
                    }
                    // Extract object-level validation keywords (minProperties, maxProperties, etc.)
                    // and add them to the first nested column or create a parent column
                    let object_quality = self.extract_validation_keywords(prop_obj, prop_name);
                    if !object_quality.is_empty() && !columns.is_empty() {
                        // Add object-level validation to the first column
                        columns[0].quality.extend(object_quality);
                    }
                } else {
                    // Object without properties - treat as STRUCT
                    let struct_quality = self.extract_validation_keywords(prop_obj, prop_name);
                    columns.push(Column {
                        name: prop_name.to_string(),
                        data_type: "STRUCT".to_string(),
                        nullable,
                        description,
                        quality: struct_quality,
                        ..Default::default()
                    });
                }
            }
            "array" => {
                // Array type
                let items = prop_obj
                    .get("items")
                    .ok_or_else(|| anyhow::anyhow!("Array property missing items"))?;

                let data_type = if let Some(items_str) = items.get("type").and_then(|v| v.as_str())
                {
                    if items_str == "object" {
                        // Array of objects - create nested columns
                        if let Some(nested_props) =
                            items.get("properties").and_then(|v| v.as_object())
                        {
                            let nested_required: Vec<String> = items
                                .get("required")
                                .and_then(|v| v.as_array())
                                .map(|arr| {
                                    arr.iter()
                                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                        .collect()
                                })
                                .unwrap_or_default();

                            for (nested_name, nested_schema) in nested_props {
                                let nested_nullable = !nested_required.contains(nested_name);
                                match self.parse_property(
                                    nested_name,
                                    nested_schema,
                                    nested_nullable,
                                    errors,
                                ) {
                                    Ok(mut nested_cols) => {
                                        for col in nested_cols.iter_mut() {
                                            col.name = format!("{}.{}", prop_name, col.name);
                                        }
                                        columns.append(&mut nested_cols);
                                    }
                                    Err(e) => {
                                        errors.push(ParserError {
                                            error_type: "parse_error".to_string(),
                                            field: Some(format!("{}.{}", prop_name, nested_name)),
                                            message: format!(
                                                "Failed to parse array item property: {}",
                                                e
                                            ),
                                        });
                                    }
                                }
                            }
                            return Ok(columns);
                        } else {
                            "ARRAY<STRUCT>".to_string()
                        }
                    } else {
                        format!("ARRAY<{}>", self.map_json_type_to_sql(items_str))
                    }
                } else {
                    "ARRAY<STRING>".to_string()
                };

                // Extract array-specific validation keywords
                let mut array_quality = self.extract_validation_keywords(prop_obj, prop_name);
                // Also extract validation from items if it's a simple array
                if let Some(items_obj) = items.as_object() {
                    let items_quality = self.extract_validation_keywords(items_obj, prop_name);
                    array_quality.extend(items_quality);
                }

                columns.push(Column {
                    name: prop_name.to_string(),
                    data_type,
                    nullable,
                    description,
                    quality: array_quality,
                    ..Default::default()
                });
            }
            _ => {
                // Simple type
                let data_type = self.map_json_type_to_sql(prop_type);
                columns.push(Column {
                    name: prop_name.to_string(),
                    data_type,
                    nullable,
                    description,
                    quality: quality_rules,
                    enum_values: enum_values.clone(),
                    ..Default::default()
                });
            }
        }

        Ok(columns)
    }

    /// Map JSON Schema type to SQL/ODCL data type.
    fn map_json_type_to_sql(&self, json_type: &str) -> String {
        match json_type {
            "integer" => "INTEGER".to_string(),
            "number" => "DOUBLE".to_string(),
            "boolean" => "BOOLEAN".to_string(),
            "string" => "STRING".to_string(),
            "null" => "NULL".to_string(),
            _ => "STRING".to_string(), // Default fallback
        }
    }

    /// Extract validation keywords from JSON Schema property and convert to quality rules.
    fn extract_validation_keywords(
        &self,
        prop_obj: &serde_json::Map<String, Value>,
        _prop_name: &str,
    ) -> Vec<HashMap<String, serde_json::Value>> {
        let mut quality_rules = Vec::new();

        // Pattern (regex) validation
        if let Some(pattern) = prop_obj.get("pattern").and_then(|v| v.as_str()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("pattern"));
            rule.insert("pattern".to_string(), json!(pattern));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // Minimum value (for numbers)
        if let Some(minimum) = prop_obj.get("minimum") {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("minimum"));
            rule.insert("value".to_string(), minimum.clone());
            rule.insert("source".to_string(), json!("json_schema"));
            if let Some(exclusive_minimum) = prop_obj.get("exclusiveMinimum") {
                rule.insert("exclusive".to_string(), exclusive_minimum.clone());
            }
            quality_rules.push(rule);
        }

        // Maximum value (for numbers)
        if let Some(maximum) = prop_obj.get("maximum") {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("maximum"));
            rule.insert("value".to_string(), maximum.clone());
            rule.insert("source".to_string(), json!("json_schema"));
            if let Some(exclusive_maximum) = prop_obj.get("exclusiveMaximum") {
                rule.insert("exclusive".to_string(), exclusive_maximum.clone());
            }
            quality_rules.push(rule);
        }

        // MinLength (for strings)
        if let Some(min_length) = prop_obj.get("minLength").and_then(|v| v.as_u64()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("minLength"));
            rule.insert("value".to_string(), json!(min_length));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // MaxLength (for strings)
        if let Some(max_length) = prop_obj.get("maxLength").and_then(|v| v.as_u64()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("maxLength"));
            rule.insert("value".to_string(), json!(max_length));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // MultipleOf (for numbers)
        if let Some(multiple_of) = prop_obj.get("multipleOf") {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("multipleOf"));
            rule.insert("value".to_string(), multiple_of.clone());
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // Const (constant value)
        if let Some(const_val) = prop_obj.get("const") {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("const"));
            rule.insert("value".to_string(), const_val.clone());
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // MinItems (for arrays)
        if let Some(min_items) = prop_obj.get("minItems").and_then(|v| v.as_u64()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("minItems"));
            rule.insert("value".to_string(), json!(min_items));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // MaxItems (for arrays)
        if let Some(max_items) = prop_obj.get("maxItems").and_then(|v| v.as_u64()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("maxItems"));
            rule.insert("value".to_string(), json!(max_items));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // UniqueItems (for arrays)
        if let Some(unique_items) = prop_obj.get("uniqueItems").and_then(|v| v.as_bool())
            && unique_items
        {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("uniqueItems"));
            rule.insert("value".to_string(), json!(true));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // MinProperties (for objects)
        if let Some(min_props) = prop_obj.get("minProperties").and_then(|v| v.as_u64()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("minProperties"));
            rule.insert("value".to_string(), json!(min_props));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // MaxProperties (for objects)
        if let Some(max_props) = prop_obj.get("maxProperties").and_then(|v| v.as_u64()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("maxProperties"));
            rule.insert("value".to_string(), json!(max_props));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // AdditionalProperties (for objects)
        if let Some(additional_props) = prop_obj.get("additionalProperties") {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("additionalProperties"));
            rule.insert("value".to_string(), additional_props.clone());
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // Format (already handled separately, but preserve as quality rule for completeness)
        if let Some(format_val) = prop_obj.get("format").and_then(|v| v.as_str()) {
            let mut rule = HashMap::new();
            rule.insert("type".to_string(), json!("format"));
            rule.insert("value".to_string(), json!(format_val));
            rule.insert("source".to_string(), json!("json_schema"));
            quality_rules.push(rule);
        }

        // AllOf, AnyOf, OneOf, Not (complex validation)
        for keyword in &["allOf", "anyOf", "oneOf", "not"] {
            if let Some(value) = prop_obj.get(*keyword) {
                let mut rule = HashMap::new();
                rule.insert("type".to_string(), json!(*keyword));
                rule.insert("value".to_string(), value.clone());
                rule.insert("source".to_string(), json!("json_schema"));
                quality_rules.push(rule);
            }
        }

        quality_rules
    }

    /// Extract enum values from JSON Schema property.
    fn extract_enum_values(&self, prop_obj: &serde_json::Map<String, Value>) -> Vec<String> {
        prop_obj
            .get("enum")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| {
                        // Convert enum values to strings
                        match v {
                            Value::String(s) => Some(s.clone()),
                            Value::Number(n) => Some(n.to_string()),
                            Value::Bool(b) => Some(b.to_string()),
                            Value::Null => Some("null".to_string()),
                            _ => serde_json::to_string(v).ok(),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Parser error structure (matches ODCL parser format).
#[derive(Debug, Clone)]
pub struct ParserError {
    pub error_type: String,
    pub field: Option<String>,
    pub message: String,
}
