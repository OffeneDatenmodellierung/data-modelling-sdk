//! JSON Schema exporter for generating JSON Schema from data models.

use super::{ExportError, ExportResult};
use crate::models::{DataModel, Table};
use serde_json::{Value, json};

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
        Ok(ExportResult {
            content: serde_json::to_string_pretty(&schema)
                .map_err(|e| ExportError::SerializationError(e.to_string()))?,
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
                // Default to string for VARCHAR, TEXT, CHAR, etc.
                ("string".to_string(), None)
            }
        }
    }
}
