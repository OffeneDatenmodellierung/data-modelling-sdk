//! Protobuf exporter for generating .proto files from data models.
//!
//! # Security
//!
//! All identifiers are sanitized to comply with Protobuf naming rules.
//! Reserved words are prefixed with an underscore to avoid conflicts.

use super::{ExportError, ExportResult};
use crate::models::{DataModel, Table};

/// Protobuf reserved words that cannot be used as field names.
const PROTOBUF_RESERVED: &[&str] = &[
    "syntax",
    "import",
    "weak",
    "public",
    "package",
    "option",
    "message",
    "enum",
    "service",
    "extend",
    "extensions",
    "reserved",
    "to",
    "max",
    "repeated",
    "optional",
    "required",
    "group",
    "oneof",
    "map",
    "returns",
    "rpc",
    "stream",
    "true",
    "false",
];

/// Exporter for Protobuf format.
pub struct ProtobufExporter;

impl ProtobufExporter {
    /// Export tables to Protobuf format (SDK interface).
    ///
    /// # Arguments
    ///
    /// * `tables` - Slice of tables to export
    ///
    /// # Returns
    ///
    /// An `ExportResult` containing Protobuf `.proto` file content.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::protobuf::ProtobufExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let tables = vec![
    ///     Table::new("User".to_string(), vec![Column::new("id".to_string(), "INT64".to_string())]),
    /// ];
    ///
    /// let exporter = ProtobufExporter;
    /// let result = exporter.export(&tables).unwrap();
    /// assert_eq!(result.format, "protobuf");
    /// assert!(result.content.contains("syntax = \"proto3\""));
    /// ```
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

    /// Export tags as Protobuf comments.
    fn export_tags_as_comments(tags: &[crate::models::Tag]) -> String {
        if tags.is_empty() {
            return String::new();
        }
        let tag_strings: Vec<String> = tags.iter().map(|t| t.to_string()).collect();
        format!("  // tags: {}\n", tag_strings.join(", "))
    }

    /// Export a table to Protobuf message format.
    ///
    /// # Arguments
    ///
    /// * `table` - The table to export
    /// * `field_number` - Mutable reference to field number counter (incremented for each field)
    ///
    /// # Returns
    ///
    /// A Protobuf message definition as a string.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::protobuf::ProtobufExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let table = Table::new(
    ///     "User".to_string(),
    ///     vec![Column::new("id".to_string(), "INT64".to_string())],
    /// );
    ///
    /// let mut field_number = 0u32;
    /// let proto = ProtobufExporter::export_table(&table, &mut field_number);
    /// assert!(proto.contains("message User"));
    /// ```
    pub fn export_table(table: &Table, field_number: &mut u32) -> String {
        let mut proto = String::new();

        let message_name = Self::sanitize_identifier(&table.name);
        proto.push_str(&format!("message {} {{\n", message_name));

        // Add tags as comments if present
        if !table.tags.is_empty() {
            proto.push_str(&Self::export_tags_as_comments(&table.tags));
        }

        for column in &table.columns {
            *field_number += 1;

            let proto_type = Self::map_data_type_to_protobuf(&column.data_type);
            let repeated = if column.data_type.to_lowercase().contains("array") {
                "repeated "
            } else {
                ""
            };

            let field_name = Self::sanitize_identifier(&column.name);

            proto.push_str(&format!(
                "  {}{}{} {} = {};",
                if column.nullable { "optional " } else { "" },
                repeated,
                proto_type,
                field_name,
                field_number
            ));

            // Sanitize description for comments (remove newlines)
            if !column.description.is_empty() {
                let desc = column.description.replace('\n', " ").replace('\r', "");
                proto.push_str(&format!(" // {}", desc));
            }

            proto.push('\n');
        }

        proto.push_str("}\n");
        proto
    }

    /// Sanitize an identifier for use in Protobuf.
    ///
    /// - Replaces invalid characters with underscores
    /// - Prefixes reserved words with underscore
    /// - Ensures identifier starts with a letter or underscore
    fn sanitize_identifier(name: &str) -> String {
        // Replace dots (nested columns) and other invalid chars with underscores
        let mut sanitized: String = name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        // Ensure starts with letter or underscore
        if let Some(first) = sanitized.chars().next()
            && first.is_numeric()
        {
            sanitized = format!("_{}", sanitized);
        }

        // Handle reserved words
        if PROTOBUF_RESERVED.contains(&sanitized.to_lowercase().as_str()) {
            sanitized = format!("_{}", sanitized);
        }

        sanitized
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
            "int" | "integer" | "smallint" | "tinyint" | "int32" => "int32".to_string(),
            "bigint" | "int64" | "long" => "int64".to_string(),
            "float" | "real" => "float".to_string(),
            "double" | "decimal" | "numeric" => "double".to_string(),
            "boolean" | "bool" => "bool".to_string(),
            "bytes" | "binary" | "varbinary" => "bytes".to_string(),
            "date" | "time" | "timestamp" | "datetime" => "string".to_string(), // Use string for dates
            "uuid" => "string".to_string(),
            _ => {
                // Default to string for VARCHAR, TEXT, CHAR, etc.
                "string".to_string()
            }
        }
    }
}
