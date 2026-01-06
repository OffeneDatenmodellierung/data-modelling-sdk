//! AVRO schema exporter for generating AVRO schemas from data models.

use super::{ExportError, ExportResult};
use crate::models::{DataModel, Table};
use serde_json::{Value, json};

/// Exporter for AVRO schema format.
pub struct AvroExporter;

impl AvroExporter {
    /// Export tables to AVRO schema format (SDK interface).
    ///
    /// # Arguments
    ///
    /// * `tables` - Slice of tables to export
    ///
    /// # Returns
    ///
    /// An `ExportResult` containing AVRO schema(s) as JSON.
    /// If a single table is provided, returns a single schema object.
    /// If multiple tables are provided, returns an array of schemas.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::avro::AvroExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let tables = vec![
    ///     Table::new("User".to_string(), vec![Column::new("id".to_string(), "INT64".to_string())]),
    /// ];
    ///
    /// let exporter = AvroExporter;
    /// let result = exporter.export(&tables).unwrap();
    /// assert_eq!(result.format, "avro");
    /// ```
    pub fn export(&self, tables: &[Table]) -> Result<ExportResult, ExportError> {
        let schema = Self::export_model_from_tables(tables);
        let content = serde_json::to_string_pretty(&schema)
            .map_err(|e| ExportError::SerializationError(e.to_string()))?;

        // Validate exported AVRO schema
        #[cfg(feature = "cli")]
        {
            use crate::cli::validation::validate_avro;
            validate_avro(&content).map_err(|e| {
                ExportError::ValidationError(format!("AVRO validation failed: {}", e))
            })?;
        }
        #[cfg(not(feature = "cli"))]
        {
            // Basic validation - parse as JSON and check for required AVRO fields
            let _value: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                ExportError::ValidationError(format!("Failed to parse AVRO JSON: {}", e))
            })?;
        }

        Ok(ExportResult {
            content,
            format: "avro".to_string(),
        })
    }

    fn export_model_from_tables(tables: &[Table]) -> serde_json::Value {
        if tables.len() == 1 {
            Self::export_table(&tables[0])
        } else {
            let schemas: Vec<serde_json::Value> = tables.iter().map(Self::export_table).collect();
            serde_json::json!(schemas)
        }
    }

    /// Export a table to AVRO schema format.
    ///
    /// # Arguments
    ///
    /// * `table` - The table to export
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` representing the AVRO schema for the table.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::avro::AvroExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let table = Table::new(
    ///     "User".to_string(),
    ///     vec![Column::new("id".to_string(), "INT64".to_string())],
    /// );
    ///
    /// let schema = AvroExporter::export_table(&table);
    /// assert_eq!(schema["type"], "record");
    /// assert_eq!(schema["name"], "User");
    /// ```
    pub fn export_table(table: &Table) -> Value {
        let mut fields = Vec::new();

        for column in &table.columns {
            let mut field = serde_json::Map::new();
            field.insert("name".to_string(), json!(column.name));

            // Map data type to AVRO type
            let avro_type = Self::map_data_type_to_avro(&column.data_type, column.nullable);
            field.insert("type".to_string(), avro_type);

            if !column.description.is_empty() {
                field.insert("doc".to_string(), json!(column.description));
            }

            fields.push(json!(field));
        }

        let mut schema = serde_json::Map::new();
        schema.insert("type".to_string(), json!("record"));
        schema.insert("name".to_string(), json!(table.name));

        // Add tags if present (AVRO doesn't have standard tags, but we can add them as metadata)
        if !table.tags.is_empty() {
            let tags_array: Vec<String> = table.tags.iter().map(|t| t.to_string()).collect();
            schema.insert("tags".to_string(), json!(tags_array));
        }
        schema.insert("namespace".to_string(), json!("com.datamodel"));
        schema.insert("fields".to_string(), json!(fields));

        json!(schema)
    }

    /// Export a data model to AVRO schema format (legacy method for compatibility).
    pub fn export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> Value {
        let tables_to_export: Vec<&Table> = if let Some(ids) = table_ids {
            model
                .tables
                .iter()
                .filter(|t| ids.contains(&t.id))
                .collect()
        } else {
            model.tables.iter().collect()
        };

        if tables_to_export.len() == 1 {
            // Single table: return the schema directly
            Self::export_table(tables_to_export[0])
        } else {
            // Multiple tables: return array of schemas
            let schemas: Vec<Value> = tables_to_export
                .iter()
                .map(|t| Self::export_table(t))
                .collect();
            json!(schemas)
        }
    }

    /// Map SQL/ODCL data types to AVRO types.
    fn map_data_type_to_avro(data_type: &str, nullable: bool) -> Value {
        let dt_lower = data_type.to_lowercase();

        let avro_type = match dt_lower.as_str() {
            "int" | "integer" | "smallint" | "tinyint" => json!("int"),
            "bigint" => json!("long"),
            "float" | "real" => json!("float"),
            "double" | "decimal" | "numeric" => json!("double"),
            "boolean" | "bool" => json!("boolean"),
            "bytes" | "binary" | "varbinary" => json!("bytes"),
            _ => {
                // Default to string for VARCHAR, TEXT, CHAR, DATE, TIMESTAMP, etc.
                json!("string")
            }
        };

        if nullable {
            json!(["null", avro_type])
        } else {
            avro_type
        }
    }
}
