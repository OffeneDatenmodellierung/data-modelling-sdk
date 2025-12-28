//! Protobuf parser for importing .proto files into data models.

use crate::models::{Column, Table};
use crate::import::{ImportResult, ImportError, TableData};
use anyhow::Result;
use std::collections::HashMap;
use tracing::info;

/// Parser for Protobuf format.
pub struct ProtobufImporter;

impl Default for ProtobufImporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProtobufImporter {
    /// Create a new Protobuf parser instance.
    pub fn new() -> Self {
        Self
    }

    /// Import Protobuf content and create Table(s) (SDK interface).
    pub fn import(&self, proto_content: &str) -> Result<ImportResult, ImportError> {
        match self.parse(proto_content) {
            Ok((tables, errors)) => {
                let mut sdk_tables = Vec::new();
                for (idx, table) in tables.iter().enumerate() {
                    sdk_tables.push(TableData {
                        table_index: idx,
                        name: Some(table.name.clone()),
                        columns: table.columns.iter().map(|c| super::ColumnData {
                            name: c.name.clone(),
                            data_type: c.data_type.clone(),
                            nullable: c.nullable,
                            primary_key: c.primary_key,
                        }).collect(),
                    });
                }
                let sdk_errors: Vec<ImportError> = errors.iter().map(|e| ImportError::ParseError(e.message.clone())).collect();
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

    /// Parse Protobuf content and create Table(s) (internal method).
    ///
    /// # Returns
    ///
    /// Returns a tuple of (Tables, list of errors/warnings).
    fn parse(&self, proto_content: &str) -> Result<(Vec<Table>, Vec<ParserError>)> {
        let mut errors = Vec::new();
        let mut tables = Vec::new();

        // Simple parser for proto3 syntax
        // This is a basic implementation - for production, consider using prost or similar
        let lines: Vec<&str> = proto_content.lines().collect();
        let mut current_message: Option<Message> = None;
        let mut messages = Vec::new();

        for (_line_num, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Skip comments and empty lines
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
                continue;
            }

            // Check for message definition
            if trimmed.starts_with("message ") {
                // Save previous message if exists
                if let Some(msg) = current_message.take() {
                    messages.push(msg);
                }

                // Parse message name - handle both "message Name {" and "message Name{"
                let msg_name = trimmed
                    .strip_prefix("message ")
                    .and_then(|s| {
                        // Remove trailing "{"
                        let s = s.trim_end();
                        if let Some(stripped) = s.strip_suffix("{") {
                            Some(stripped)
                        } else if let Some(stripped) = s.strip_suffix(" {") {
                            Some(stripped)
                        } else {
                            s.split_whitespace().next()
                        }
                    })
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow::anyhow!("Invalid message syntax: {}", trimmed))?;

                current_message = Some(Message {
                    name: msg_name.to_string(),
                    fields: Vec::new(),
                    nested_messages: Vec::new(),
                });
            } else if trimmed == "}" || trimmed == "};" {
                // End of message
                if let Some(msg) = current_message.take() {
                    messages.push(msg);
                }
            } else if trimmed.starts_with("enum ") {
                // Skip enum definitions for now
                continue;
            } else if let Some(ref mut msg) = current_message {
                // Parse field - skip if it's an enum value
                if trimmed.contains("=") && !trimmed.contains(";") {
                    // Multi-line field definition, skip for now
                    continue;
                }
                if let Ok(field) = self.parse_field(trimmed, _line_num) {
                    msg.fields.push(field);
                } else {
                    // Don't add error for empty lines or comments that slipped through
                    if !trimmed.is_empty() && !trimmed.starts_with("//") {
                        errors.push(ParserError {
                            error_type: "parse_error".to_string(),
                            field: Some(format!("line {}", _line_num + 1)),
                            message: format!("Failed to parse field: {}", trimmed),
                        });
                    }
                }
            }
        }

        // Add last message if exists
        if let Some(msg) = current_message {
            messages.push(msg);
        }

        // Convert messages to tables
        for message in &messages {
            match self.message_to_table(message, &messages, &mut errors) {
                Ok(table) => tables.push(table),
                Err(e) => {
                    errors.push(ParserError {
                        error_type: "parse_error".to_string(),
                        field: Some(message.name.clone()),
                        message: format!("Failed to convert message to table: {}", e),
                    });
                }
            }
        }

        Ok((tables, errors))
    }

    /// Parse a Protobuf field line.
    fn parse_field(&self, line: &str, _line_num: usize) -> Result<ProtobufField> {
        // Remove comments
        let line = line.split("//").next().unwrap_or(line).trim();

        // Parse: [repeated] [optional] type name = number;
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(anyhow::anyhow!("Invalid field syntax"));
        }

        let mut idx = 0;
        let mut repeated = false;
        let mut optional = false;

        // Check for repeated/optional keywords
        while idx < parts.len() {
            match parts[idx] {
                "repeated" => {
                    repeated = true;
                    idx += 1;
                }
                "optional" => {
                    optional = true;
                    idx += 1;
                }
                _ => break,
            }
        }

        if idx >= parts.len() {
            return Err(anyhow::anyhow!("Missing field type"));
        }

        let field_type = parts[idx].to_string();
        idx += 1;

        if idx >= parts.len() {
            return Err(anyhow::anyhow!("Missing field name"));
        }

        let field_name = parts[idx]
            .strip_suffix(";")
            .unwrap_or(parts[idx])
            .to_string();
        idx += 1;

        // Field number (optional for parsing)
        let _field_number = if idx < parts.len() {
            parts[idx]
                .strip_prefix("=")
                .and_then(|s| s.strip_suffix(";"))
                .and_then(|s| s.parse::<u32>().ok())
        } else {
            None
        };

        Ok(ProtobufField {
            name: field_name,
            field_type,
            repeated,
            nullable: optional || repeated, // Repeated fields are nullable
        })
    }

    /// Convert a Protobuf message to a Table.
    fn message_to_table(
        &self,
        message: &Message,
        all_messages: &[Message],
        _errors: &mut Vec<ParserError>,
    ) -> Result<Table> {
        let mut columns = Vec::new();

        for field in &message.fields {
            // Check if field type is a nested message
            if let Some(nested_msg) = all_messages.iter().find(|m| m.name == field.field_type) {
                // Nested message - recursively extract nested columns with dot notation
                // Check if nested message itself contains nested messages
                for nested_field in &nested_msg.fields {
                    let nested_field_name = format!("{}.{}", field.name, nested_field.name);

                    // Check if this nested field is itself a nested message (deep nesting)
                    if let Some(deep_nested_msg) = all_messages
                        .iter()
                        .find(|m| m.name == nested_field.field_type)
                    {
                        // Deeply nested message - create columns for its fields
                        for deep_nested_field in &deep_nested_msg.fields {
                            let data_type = if deep_nested_field.repeated {
                                format!(
                                    "ARRAY<{}>",
                                    self.map_proto_type_to_sql(&deep_nested_field.field_type)
                                )
                            } else {
                                self.map_proto_type_to_sql(&deep_nested_field.field_type)
                            };

                            columns.push(Column {
                                name: format!("{}.{}", nested_field_name, deep_nested_field.name),
                                data_type,
                                nullable: nested_field.nullable || deep_nested_field.nullable,
                                primary_key: false,
                                secondary_key: false,
                                composite_key: None,
                                foreign_key: None,
                                constraints: Vec::new(),
                                description: String::new(),
                                quality: Vec::new(),
                                enum_values: Vec::new(),
                                errors: Vec::new(),
                                column_order: 0,
                            });
                        }
                    } else {
                        // Simple nested field
                        let data_type = if nested_field.repeated {
                            format!(
                                "ARRAY<{}>",
                                self.map_proto_type_to_sql(&nested_field.field_type)
                            )
                        } else {
                            self.map_proto_type_to_sql(&nested_field.field_type)
                        };

                        columns.push(Column {
                            name: nested_field_name,
                            data_type,
                            nullable: nested_field.nullable,
                            primary_key: false,
                            secondary_key: false,
                            composite_key: None,
                            foreign_key: None,
                            constraints: Vec::new(),
                            description: String::new(),
                            quality: Vec::new(),
                            enum_values: Vec::new(),
                            errors: Vec::new(),
                            column_order: 0,
                        });
                    }
                }
            } else {
                // Simple field
                let data_type = if field.repeated {
                    format!("ARRAY<{}>", self.map_proto_type_to_sql(&field.field_type))
                } else {
                    self.map_proto_type_to_sql(&field.field_type)
                };

                columns.push(Column {
                    name: field.name.clone(),
                    data_type,
                    nullable: field.nullable,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description: String::new(),
                    quality: Vec::new(),
                    enum_values: Vec::new(),
                    errors: Vec::new(),
                    column_order: 0,
                });
            }
        }

        let mut odcl_metadata = HashMap::new();
        odcl_metadata.insert(
            "syntax".to_string(),
            serde_json::Value::String("proto3".to_string()),
        );

        let table = Table {
            id: crate::models::table::Table::generate_id(&message.name, None, None, None),
            name: message.name.clone(),
            columns,
            database_type: None,
            catalog_name: None,
            schema_name: None,
            medallion_layers: Vec::new(),
            scd_pattern: None,
            data_vault_classification: None,
            modeling_level: None,
            tags: Vec::new(),
            odcl_metadata,
            position: None,
            yaml_file_path: None,
            drawio_cell_id: None,
            quality: Vec::new(),
            errors: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        info!(
            "Parsed Protobuf message: {} with {} columns",
            message.name,
            table.columns.len()
        );
        Ok(table)
    }

    /// Map Protobuf type to SQL/ODCL data type.
    fn map_proto_type_to_sql(&self, proto_type: &str) -> String {
        match proto_type {
            "int32" | "int" => "INTEGER".to_string(),
            "int64" | "long" => "BIGINT".to_string(),
            "float" => "FLOAT".to_string(),
            "double" => "DOUBLE".to_string(),
            "bool" | "boolean" => "BOOLEAN".to_string(),
            "bytes" => "BYTES".to_string(),
            "string" => "STRING".to_string(),
            _ => "STRING".to_string(), // Default fallback (including custom message types)
        }
    }
}

/// Protobuf message structure.
#[derive(Debug, Clone)]
struct Message {
    name: String,
    fields: Vec<ProtobufField>,
    #[allow(dead_code)]
    nested_messages: Vec<Message>,
}

/// Protobuf field structure.
#[derive(Debug, Clone)]
struct ProtobufField {
    name: String,
    field_type: String,
    repeated: bool,
    nullable: bool,
}

/// Parser error structure (matches ODCL parser format).
#[derive(Debug, Clone)]
pub struct ParserError {
    pub error_type: String,
    pub field: Option<String>,
    pub message: String,
}
