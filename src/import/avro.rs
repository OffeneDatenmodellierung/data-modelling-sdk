//! AVRO schema parser for importing AVRO schemas into data models.
//!
//! # Validation
//!
//! All imported table and column names are validated for:
//! - Valid identifier format
//! - Maximum length limits

use crate::import::odcs_shared::column_to_column_data;
use crate::import::{ImportError, ImportResult, TableData};
use crate::models::{Column, Table, Tag};
use crate::validation::input::{validate_column_name, validate_table_name};
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::{info, warn};

/// Parser for AVRO schema format.
#[derive(Default)]
pub struct AvroImporter;

impl AvroImporter {
    /// Create a new AVRO parser instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::avro::AvroImporter;
    ///
    /// let importer = AvroImporter::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// Import AVRO schema content and create Table(s) (SDK interface).
    ///
    /// # Arguments
    ///
    /// * `avro_content` - AVRO schema as JSON string (can be a single record or array of records)
    ///
    /// # Returns
    ///
    /// An `ImportResult` containing extracted tables and any parse errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::avro::AvroImporter;
    ///
    /// let importer = AvroImporter::new();
    /// let schema = r#"
    /// {
    ///   "type": "record",
    ///   "name": "User",
    ///   "fields": [
    ///     {"name": "id", "type": "long"}
    ///   ]
    /// }
    /// "#;
    /// let result = importer.import(schema).unwrap();
    /// ```
    pub fn import(&self, avro_content: &str) -> Result<ImportResult, ImportError> {
        match self.parse(avro_content) {
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

    /// Parse AVRO schema content and create Table(s) (internal method).
    ///
    /// # Returns
    ///
    /// Returns a tuple of (Tables, list of errors/warnings).
    fn parse(&self, avro_content: &str) -> Result<(Vec<Table>, Vec<ParserError>)> {
        let mut errors = Vec::new();

        // Parse JSON
        let schema: Value =
            serde_json::from_str(avro_content).context("Failed to parse AVRO schema as JSON")?;

        let mut tables = Vec::new();

        // AVRO can be a single record or an array of records
        if let Some(schemas) = schema.as_array() {
            // Multiple schemas
            for (idx, schema_item) in schemas.iter().enumerate() {
                match self.parse_schema(schema_item, &mut errors) {
                    Ok(table) => tables.push(table),
                    Err(e) => {
                        errors.push(ParserError {
                            error_type: "parse_error".to_string(),
                            field: Some(format!("schema[{}]", idx)),
                            message: format!("Failed to parse schema: {}", e),
                        });
                    }
                }
            }
        } else {
            // Single schema
            match self.parse_schema(&schema, &mut errors) {
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

    /// Parse a single AVRO schema record.
    fn parse_schema(&self, schema: &Value, errors: &mut Vec<ParserError>) -> Result<Table> {
        let schema_obj = schema
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Schema must be an object"))?;

        // Extract record name
        let name = schema_obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: name"))?
            .to_string();

        // Validate table name
        if let Err(e) = validate_table_name(&name) {
            warn!("Table name validation warning for '{}': {}", name, e);
        }

        // Extract namespace (optional)
        let namespace = schema_obj
            .get("namespace")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Extract fields
        let fields = schema_obj
            .get("fields")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Missing required field: fields"))?;

        let mut columns = Vec::new();
        for (idx, field) in fields.iter().enumerate() {
            match self.parse_field(field, &name, errors) {
                Ok(mut cols) => columns.append(&mut cols),
                Err(e) => {
                    errors.push(ParserError {
                        error_type: "parse_error".to_string(),
                        field: Some(format!("fields[{}]", idx)),
                        message: format!("Failed to parse field: {}", e),
                    });
                }
            }
        }

        // Extract tags from AVRO schema (can be in root or in aliases/metadata)
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
        // Also check aliases/metadata for tags
        if let Some(aliases_arr) = schema_obj.get("aliases").and_then(|v| v.as_array()) {
            for item in aliases_arr {
                if let Some(s) = item.as_str() {
                    // AVRO aliases can be used as tags
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
        if let Some(ref ns) = namespace {
            odcl_metadata.insert("namespace".to_string(), json!(ns));
        }
        if let Some(doc) = schema_obj.get("doc").and_then(|v| v.as_str()) {
            odcl_metadata.insert("description".to_string(), json!(doc));
        }

        let table = Table {
            id: crate::models::table::Table::generate_id(&name, None, None, namespace.as_deref()),
            name: name.clone(),
            columns,
            database_type: None,
            catalog_name: None,
            schema_name: namespace.clone(),
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
            "Parsed AVRO schema: {} with {} columns",
            name,
            table.columns.len()
        );
        Ok(table)
    }

    /// Parse an AVRO field (which can be a simple field or nested record).
    fn parse_field(
        &self,
        field: &Value,
        _parent_name: &str,
        errors: &mut Vec<ParserError>,
    ) -> Result<Vec<Column>> {
        let field_obj = field
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Field must be an object"))?;

        let field_name = field_obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Field missing name"))?
            .to_string();

        // Validate column name
        if let Err(e) = validate_column_name(&field_name) {
            warn!("Column name validation warning for '{}': {}", field_name, e);
        }

        let field_type = field_obj
            .get("type")
            .ok_or_else(|| anyhow::anyhow!("Field missing type"))?;

        let description = field_obj
            .get("doc")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Handle union types (e.g., ["null", "string"] for nullable)
        let (avro_type, nullable) = if let Some(types) = field_type.as_array() {
            if types.len() == 2 && types.iter().any(|t| t.as_str() == Some("null")) {
                // Nullable type
                let non_null_type = types
                    .iter()
                    .find(|t| t.as_str() != Some("null"))
                    .ok_or_else(|| anyhow::anyhow!("Invalid union type"))?;
                (non_null_type, true)
            } else {
                // Complex union with multiple non-null types - use first non-null type
                // and mark as nullable since union implies optionality
                let first_non_null = types
                    .iter()
                    .find(|t| t.as_str() != Some("null"))
                    .unwrap_or(field_type);
                (first_non_null, true)
            }
        } else {
            (field_type, false)
        };

        // Parse the actual type
        let mut columns = Vec::new();
        if let Some(type_str) = avro_type.as_str() {
            // Simple type
            let data_type = self.map_avro_type_to_sql(type_str);
            columns.push(Column {
                name: field_name,
                data_type,
                nullable,
                description,
                ..Default::default()
            });
        } else if let Some(type_obj) = avro_type.as_object() {
            // Complex type (record, array, map)
            if type_obj.get("type").and_then(|v| v.as_str()) == Some("record") {
                // Nested record - create nested columns with dot notation
                let nested_name = type_obj
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(&field_name);
                let nested_fields = type_obj
                    .get("fields")
                    .and_then(|v| v.as_array())
                    .ok_or_else(|| anyhow::anyhow!("Nested record missing fields"))?;

                for nested_field in nested_fields {
                    match self.parse_field(nested_field, nested_name, errors) {
                        Ok(mut nested_cols) => {
                            // Prefix nested columns with parent field name
                            for col in nested_cols.iter_mut() {
                                col.name = format!("{}.{}", field_name, col.name);
                            }
                            columns.append(&mut nested_cols);
                        }
                        Err(e) => {
                            errors.push(ParserError {
                                error_type: "parse_error".to_string(),
                                field: Some(format!("{}.{}", field_name, nested_name)),
                                message: format!("Failed to parse nested field: {}", e),
                            });
                        }
                    }
                }
            } else if type_obj.get("type").and_then(|v| v.as_str()) == Some("array") {
                // Array type
                let items = type_obj
                    .get("items")
                    .ok_or_else(|| anyhow::anyhow!("Array type missing items"))?;

                let data_type = if let Some(items_str) = items.as_str() {
                    format!("ARRAY<{}>", self.map_avro_type_to_sql(items_str))
                } else if let Some(items_obj) = items.as_object() {
                    if items_obj.get("type").and_then(|v| v.as_str()) == Some("record") {
                        // Array of records - create nested columns
                        let nested_name = items_obj
                            .get("name")
                            .and_then(|v| v.as_str())
                            .unwrap_or(&field_name);
                        let nested_fields = items_obj
                            .get("fields")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| anyhow::anyhow!("Array record missing fields"))?;

                        for nested_field in nested_fields {
                            match self.parse_field(nested_field, nested_name, errors) {
                                Ok(mut nested_cols) => {
                                    for col in nested_cols.iter_mut() {
                                        col.name = format!("{}.{}", field_name, col.name);
                                    }
                                    columns.append(&mut nested_cols);
                                }
                                Err(e) => {
                                    errors.push(ParserError {
                                        error_type: "parse_error".to_string(),
                                        field: Some(format!("{}.{}", field_name, nested_name)),
                                        message: format!("Failed to parse array item field: {}", e),
                                    });
                                }
                            }
                        }
                        return Ok(columns);
                    } else {
                        format!("ARRAY<{}>", "STRUCT")
                    }
                } else {
                    "ARRAY<STRING>".to_string()
                };

                columns.push(Column {
                    name: field_name,
                    data_type,
                    nullable,
                    description,
                    ..Default::default()
                });
            } else {
                // Other complex types - default to STRUCT
                columns.push(Column {
                    name: field_name,
                    data_type: "STRUCT".to_string(),
                    nullable,
                    description,
                    ..Default::default()
                });
            }
        } else {
            return Err(anyhow::anyhow!("Unsupported field type format"));
        }

        Ok(columns)
    }

    /// Map AVRO type to SQL/ODCL data type.
    fn map_avro_type_to_sql(&self, avro_type: &str) -> String {
        match avro_type {
            "int" => "INTEGER".to_string(),
            "long" => "BIGINT".to_string(),
            "float" => "FLOAT".to_string(),
            "double" => "DOUBLE".to_string(),
            "boolean" => "BOOLEAN".to_string(),
            "bytes" => "BYTES".to_string(),
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
