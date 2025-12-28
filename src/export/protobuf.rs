//! Protobuf exporter for generating .proto files from data models.

use crate::models::{DataModel, Table};
use super::{ExportError, ExportResult};

/// Exporter for Protobuf format.
pub struct ProtobufExporter;

impl ProtobufExporter {
    /// Export tables to Protobuf format (SDK interface).
    pub fn export(&self, tables: &[Table]) -> Result<ExportResult, ExportError> {
        let proto = Self::export_model_from_tables(tables);
        Ok(ExportResult {
            content: proto,
            format: "protobuf".to_string(),
        })
    }

    fn export_model_from_tables(tables: &[Table]) -> String {
        let mut proto = String::new();
        proto.push_str("syntax = \"proto3\";\n\n");
        proto.push_str("package com.datamodel;\n\n");
        let mut field_number = 0u32;
        for table in tables {
            proto.push_str(&Self::export_table(table, &mut field_number));
            proto.push('\n');
        }
        proto
    }

    /// Export a table to Protobuf message format.
    pub fn export_table(table: &Table, field_number: &mut u32) -> String {
        let mut proto = String::new();

        proto.push_str(&format!("message {} {{\n", table.name));

        for column in &table.columns {
            *field_number += 1;

            let proto_type = Self::map_data_type_to_protobuf(&column.data_type);
            let repeated = if column.data_type.to_lowercase().contains("array") {
                "repeated "
            } else {
                ""
            };

            proto.push_str(&format!(
                "  {} {} {} = {};",
                if column.nullable { "optional" } else { "" },
                repeated,
                proto_type,
                field_number
            ));

            if !column.description.is_empty() {
                proto.push_str(&format!(" // {}", column.description));
            }

            proto.push('\n');
        }

        proto.push_str("}\n");
        proto
    }

    /// Export a data model to Protobuf format (legacy method for compatibility).
    pub fn export_model(model: &DataModel, table_ids: Option<&[uuid::Uuid]>) -> String {
        let mut proto = String::new();

        proto.push_str("syntax = \"proto3\";\n\n");
        proto.push_str("package com.datamodel;\n\n");

        let tables_to_export: Vec<&Table> = if let Some(ids) = table_ids {
            model
                .tables
                .iter()
                .filter(|t| ids.contains(&t.id))
                .collect()
        } else {
            model.tables.iter().collect()
        };

        let mut field_number = 0u32;
        for table in tables_to_export {
            proto.push_str(&Self::export_table(table, &mut field_number));
            proto.push('\n');
        }

        proto
    }

    /// Map SQL/ODCL data types to Protobuf types.
    fn map_data_type_to_protobuf(data_type: &str) -> String {
        let dt_lower = data_type.to_lowercase();

        match dt_lower.as_str() {
            "int" | "integer" | "smallint" | "tinyint" => "int32".to_string(),
            "bigint" => "int64".to_string(),
            "float" | "real" => "float".to_string(),
            "double" | "decimal" | "numeric" => "double".to_string(),
            "boolean" | "bool" => "bool".to_string(),
            "bytes" | "binary" | "varbinary" => "bytes".to_string(),
            "date" | "time" | "timestamp" | "datetime" => "string".to_string(), // Use string for dates
            "uuid" => "string".to_string(),
            _ => {
                // Default to string for VARCHAR, TEXT, CHAR, etc.
                // Default to string for VARCHAR, TEXT, CHAR, etc.
                "string".to_string()
            }
        }
    }
}
