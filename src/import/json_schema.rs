//! JSON Schema parser for importing JSON Schema into data models.
//!
//! # Validation
//!
//! All imported table and column names are validated for:
//! - Valid identifier format
//! - Maximum length limits

use super::{ImportError, ImportResult, TableData};
use crate::models::{Column, Table, Tag};
use crate::validation::input::{validate_column_name, validate_data_type, validate_table_name};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, warn};

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
                            .map(|c| super::ColumnData {
                                name: c.name.clone(),
                                data_type: c.data_type.clone(),
                                nullable: c.nullable,
                                primary_key: c.primary_key,
                                description: if c.description.is_empty() {
                                    None
                                } else {
                                    Some(c.description.clone())
                                },
                                quality: if c.quality.is_empty() {
                                    None
                                } else {
                                    Some(c.quality.clone())
                                },
                                ref_path: c.ref_path.clone(),
                            })
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
                } else {
                    // Object without properties - treat as STRUCT
                    columns.push(Column {
                        name: prop_name.to_string(),
                        data_type: "STRUCT".to_string(),
                        nullable,
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description,
                        quality: Vec::new(),
                        ref_path: None,
                        enum_values: Vec::new(),
                        errors: Vec::new(),
                        column_order: 0,
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

                columns.push(Column {
                    name: prop_name.to_string(),
                    data_type,
                    nullable,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description,
                    quality: Vec::new(),
                    ref_path: None,
                    enum_values: Vec::new(),
                    errors: Vec::new(),
                    column_order: 0,
                });
            }
            _ => {
                // Simple type
                let data_type = self.map_json_type_to_sql(prop_type);
                columns.push(Column {
                    name: prop_name.to_string(),
                    data_type,
                    nullable,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description,
                    quality: Vec::new(),
                    ref_path: None,
                    enum_values: Vec::new(),
                    errors: Vec::new(),
                    column_order: 0,
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
}

/// Parser error structure (matches ODCL parser format).
#[derive(Debug, Clone)]
pub struct ParserError {
    pub error_type: String,
    pub field: Option<String>,
    pub message: String,
}
