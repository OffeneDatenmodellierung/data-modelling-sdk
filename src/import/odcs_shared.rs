//! Shared utilities for ODCS and ODCL parsing.
//!
//! This module contains common types, utility functions, and parsing helpers
//! used by both the ODCS (Open Data Contract Standard) and ODCL (legacy Data Contract)
//! importers. Separating these shared components allows for cleaner code organization
//! and easier testing.

use crate::models::column::ForeignKey;
use crate::models::enums::{DataVaultClassification, MedallionLayer, SCDPattern};
use crate::models::{Column, Tag};
use anyhow::Result;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::str::FromStr;

/// Parser error structure for detailed error reporting.
#[derive(Debug, Clone)]
pub struct ParserError {
    pub error_type: String,
    pub field: String,
    pub message: String,
}

/// Convert YAML Value to JSON Value for easier manipulation.
pub fn yaml_to_json_value(yaml: &serde_yaml::Value) -> Result<JsonValue> {
    use anyhow::Context;
    // Convert YAML to JSON via serialization
    let json_str = serde_json::to_string(yaml).context("Failed to convert YAML to JSON")?;
    serde_json::from_str(&json_str).context("Failed to parse JSON")
}

/// Convert JSON Value to serde_json::Value for storage in HashMap.
pub fn json_value_to_serde_value(value: &JsonValue) -> serde_json::Value {
    value.clone()
}

/// Normalize data type to uppercase, preserving STRUCT<...>, ARRAY<...>, MAP<...> format.
pub fn normalize_data_type(data_type: &str) -> String {
    if data_type.is_empty() {
        return data_type.to_string();
    }

    let upper = data_type.to_uppercase();

    // Handle STRUCT<...>, ARRAY<...>, MAP<...> preserving inner content
    if upper.starts_with("STRUCT") {
        if let Some(start) = data_type.find('<')
            && let Some(end) = data_type.rfind('>')
        {
            let inner = &data_type[start + 1..end];
            return format!("STRUCT<{}>", inner);
        }
        return format!("STRUCT{}", &data_type[6..]);
    } else if upper.starts_with("ARRAY") {
        if let Some(start) = data_type.find('<')
            && let Some(end) = data_type.rfind('>')
        {
            let inner = &data_type[start + 1..end];
            return format!("ARRAY<{}>", inner);
        }
        return format!("ARRAY{}", &data_type[5..]);
    } else if upper.starts_with("MAP") {
        if let Some(start) = data_type.find('<')
            && let Some(end) = data_type.rfind('>')
        {
            let inner = &data_type[start + 1..end];
            return format!("MAP<{}>", inner);
        }
        return format!("MAP{}", &data_type[3..]);
    }

    upper
}

/// Parse medallion layer from string.
pub fn parse_medallion_layer(s: &str) -> Result<MedallionLayer> {
    match s.to_uppercase().as_str() {
        "BRONZE" => Ok(MedallionLayer::Bronze),
        "SILVER" => Ok(MedallionLayer::Silver),
        "GOLD" => Ok(MedallionLayer::Gold),
        "OPERATIONAL" => Ok(MedallionLayer::Operational),
        _ => Err(anyhow::anyhow!("Unknown medallion layer: {}", s)),
    }
}

/// Parse SCD pattern from string.
pub fn parse_scd_pattern(s: &str) -> Result<SCDPattern> {
    match s.to_uppercase().as_str() {
        "TYPE_1" | "TYPE1" => Ok(SCDPattern::Type1),
        "TYPE_2" | "TYPE2" => Ok(SCDPattern::Type2),
        _ => Err(anyhow::anyhow!("Unknown SCD pattern: {}", s)),
    }
}

/// Parse Data Vault classification from string.
pub fn parse_data_vault_classification(s: &str) -> Result<DataVaultClassification> {
    match s.to_uppercase().as_str() {
        "HUB" => Ok(DataVaultClassification::Hub),
        "LINK" => Ok(DataVaultClassification::Link),
        "SATELLITE" | "SAT" => Ok(DataVaultClassification::Satellite),
        _ => Err(anyhow::anyhow!("Unknown Data Vault classification: {}", s)),
    }
}

/// Extract quality rules from a JSON object.
pub fn extract_quality_from_obj(
    obj: &serde_json::Map<String, JsonValue>,
) -> Vec<HashMap<String, serde_json::Value>> {
    let mut quality_rules = Vec::new();
    if let Some(quality_val) = obj.get("quality") {
        if let Some(arr) = quality_val.as_array() {
            // Array of quality rules
            for item in arr {
                if let Some(rule_obj) = item.as_object() {
                    let mut rule = HashMap::new();
                    for (key, value) in rule_obj {
                        rule.insert(key.clone(), json_value_to_serde_value(value));
                    }
                    quality_rules.push(rule);
                }
            }
        } else if let Some(rule_obj) = quality_val.as_object() {
            // Single quality rule object
            let mut rule = HashMap::new();
            for (key, value) in rule_obj {
                rule.insert(key.clone(), json_value_to_serde_value(value));
            }
            quality_rules.push(rule);
        }
    }
    quality_rules
}

/// Parse foreign key from JSON value.
pub fn parse_foreign_key(fk_data: &JsonValue) -> Option<ForeignKey> {
    let obj = fk_data.as_object()?;
    Some(ForeignKey {
        table_id: obj
            .get("table_id")
            .or_else(|| obj.get("table"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        column_name: obj
            .get("column_name")
            .or_else(|| obj.get("column"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

/// Parse foreign key from Data Contract field data.
pub fn parse_foreign_key_from_data_contract(
    field_data: &serde_json::Map<String, JsonValue>,
) -> Option<ForeignKey> {
    field_data
        .get("foreignKey")
        .and_then(|v| v.as_object())
        .map(|fk_obj| ForeignKey {
            table_id: fk_obj
                .get("table")
                .or_else(|| fk_obj.get("table_id"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            column_name: fk_obj
                .get("column")
                .or_else(|| fk_obj.get("column_name"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
}

/// Extract metadata from customProperties in ODCS/ODCL format.
pub fn extract_metadata_from_custom_properties(
    data: &JsonValue,
) -> (
    Vec<MedallionLayer>,
    Option<SCDPattern>,
    Option<DataVaultClassification>,
    Vec<Tag>,
) {
    let mut medallion_layers = Vec::new();
    let mut scd_pattern = None;
    let mut data_vault_classification = None;
    let mut tags: Vec<Tag> = Vec::new();

    if let Some(custom_props) = data.get("customProperties").and_then(|v| v.as_array()) {
        for prop in custom_props {
            if let Some(prop_obj) = prop.as_object() {
                let prop_key = prop_obj
                    .get("property")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let prop_value = prop_obj.get("value");

                match prop_key {
                    "medallionLayers" | "medallion_layers" => {
                        if let Some(arr) = prop_value.and_then(|v| v.as_array()) {
                            for item in arr {
                                if let Some(s) = item.as_str()
                                    && let Ok(layer) = parse_medallion_layer(s)
                                {
                                    medallion_layers.push(layer);
                                }
                            }
                        } else if let Some(s) = prop_value.and_then(|v| v.as_str()) {
                            // Comma-separated string
                            for part in s.split(',') {
                                if let Ok(layer) = parse_medallion_layer(part.trim()) {
                                    medallion_layers.push(layer);
                                }
                            }
                        }
                    }
                    "scdPattern" | "scd_pattern" => {
                        if let Some(s) = prop_value.and_then(|v| v.as_str()) {
                            scd_pattern = parse_scd_pattern(s).ok();
                        }
                    }
                    "dataVaultClassification" | "data_vault_classification" => {
                        if let Some(s) = prop_value.and_then(|v| v.as_str()) {
                            data_vault_classification = parse_data_vault_classification(s).ok();
                        }
                    }
                    "tags" => {
                        if let Some(arr) = prop_value.and_then(|v| v.as_array()) {
                            for item in arr {
                                if let Some(s) = item.as_str() {
                                    // Parse tag string to Tag enum
                                    if let Ok(tag) = Tag::from_str(s) {
                                        tags.push(tag);
                                    } else {
                                        tags.push(Tag::Simple(s.to_string()));
                                    }
                                }
                            }
                        } else if let Some(s) = prop_value.and_then(|v| v.as_str()) {
                            // Comma-separated string
                            for part in s.split(',') {
                                let part = part.trim();
                                if let Ok(tag) = Tag::from_str(part) {
                                    tags.push(tag);
                                } else {
                                    tags.push(Tag::Simple(part.to_string()));
                                }
                            }
                        }
                    }
                    "sharedDomains" | "shared_domains" => {
                        // sharedDomains will be stored in metadata by the caller
                        // This match is here for completeness but sharedDomains is handled separately
                    }
                    _ => {}
                }
            }
        }
    }

    // Also extract tags from top-level tags field
    if let Some(tags_arr) = data.get("tags").and_then(|v| v.as_array()) {
        for item in tags_arr {
            if let Some(s) = item.as_str() {
                // Parse tag string to Tag enum
                let tag = Tag::from_str(s).unwrap_or_else(|_| Tag::Simple(s.to_string()));
                if !tags.contains(&tag) {
                    tags.push(tag);
                }
            }
        }
    }

    (
        medallion_layers,
        scd_pattern,
        data_vault_classification,
        tags,
    )
}

/// Extract catalog and schema from customProperties.
pub fn extract_catalog_schema(data: &JsonValue) -> (Option<String>, Option<String>) {
    let mut catalog_name = None;
    let mut schema_name = None;

    if let Some(custom_props) = data.get("customProperties").and_then(|v| v.as_array()) {
        for prop in custom_props {
            if let Some(prop_obj) = prop.as_object() {
                let prop_key = prop_obj
                    .get("property")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let prop_value = prop_obj.get("value").and_then(|v| v.as_str());

                match prop_key {
                    "catalogName" | "catalog_name" => {
                        catalog_name = prop_value.map(|s| s.to_string());
                    }
                    "schemaName" | "schema_name" => {
                        schema_name = prop_value.map(|s| s.to_string());
                    }
                    _ => {}
                }
            }
        }
    }

    // Also check direct fields
    if catalog_name.is_none() {
        catalog_name = data
            .get("catalog_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }
    if schema_name.is_none() {
        schema_name = data
            .get("schema_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
    }

    (catalog_name, schema_name)
}

/// Extract sharedDomains from customProperties.
pub fn extract_shared_domains(data: &JsonValue) -> Vec<String> {
    let mut shared_domains: Vec<String> = Vec::new();
    if let Some(custom_props) = data.get("customProperties").and_then(|v| v.as_array()) {
        for prop in custom_props {
            if let Some(prop_obj) = prop.as_object() {
                let prop_key = prop_obj
                    .get("property")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if (prop_key == "sharedDomains" || prop_key == "shared_domains")
                    && let Some(arr) = prop_obj.get("value").and_then(|v| v.as_array())
                {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            shared_domains.push(s.to_string());
                        }
                    }
                }
            }
        }
    }
    shared_domains
}

/// Resolve a $ref reference like '#/definitions/orderAction'.
pub fn resolve_ref<'a>(ref_str: &str, data: &'a JsonValue) -> Option<&'a JsonValue> {
    if !ref_str.starts_with("#/") {
        return None;
    }

    // Remove the leading '#/'
    let path = &ref_str[2..];
    let parts: Vec<&str> = path.split('/').collect();

    // Navigate through the data structure
    let mut current = data;
    for part in parts {
        current = current.get(part)?;
    }

    if current.is_object() {
        Some(current)
    } else {
        None
    }
}

/// Expand a nested column from a schema definition, creating columns with dot notation.
///
/// This helper function recursively expands nested structures (OBJECT/STRUCT types)
/// into flat columns with dot notation (e.g., "address.street", "address.city").
#[allow(clippy::only_used_in_recursion)]
pub fn expand_nested_column(
    column_name: &str,
    schema: &JsonValue,
    nullable: bool,
    columns: &mut Vec<Column>,
    errors: &mut Vec<ParserError>,
) {
    let schema_obj = match schema.as_object() {
        Some(obj) => obj,
        None => {
            errors.push(ParserError {
                error_type: "parse_error".to_string(),
                field: column_name.to_string(),
                message: "Nested schema must be an object".to_string(),
            });
            return;
        }
    };

    // Check both "logicalType" (ODCS v3.1.0) and "type" (legacy/ODCL) for backward compatibility
    let schema_type_raw = schema_obj
        .get("logicalType")
        .and_then(|v| v.as_str())
        .or_else(|| schema_obj.get("type").and_then(|v| v.as_str()))
        .unwrap_or("object");

    // Normalize legacy "type" values to "logicalType" equivalents
    let schema_type = match schema_type_raw {
        "object" | "struct" => "object",
        "array" => "array",
        "string" | "varchar" | "char" | "text" => "string",
        "integer" | "int" | "bigint" | "smallint" | "tinyint" => "integer",
        "number" | "decimal" | "double" | "float" | "numeric" => "number",
        "boolean" | "bool" => "boolean",
        "date" => "date",
        "timestamp" | "datetime" => "timestamp",
        "time" => "time",
        _ => schema_type_raw,
    };

    match schema_type {
        "object" | "struct" => {
            // Check if it has nested properties - handle both object format (legacy/ODCL)
            // and array format (ODCS v3.1.0)
            let properties_obj = schema_obj.get("properties").and_then(|v| v.as_object());
            let properties_arr = schema_obj.get("properties").and_then(|v| v.as_array());

            if let Some(properties) = properties_obj {
                // Object format (legacy/ODCL): properties is a map of name -> schema
                let nested_required: Vec<String> = schema_obj
                    .get("required")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    })
                    .unwrap_or_default();

                for (nested_name, nested_schema) in properties {
                    let nested_nullable = !nested_required.contains(nested_name);
                    expand_nested_column(
                        &format!("{}.{}", column_name, nested_name),
                        nested_schema,
                        nullable || nested_nullable,
                        columns,
                        errors,
                    );
                }
            } else if let Some(properties_list) = properties_arr {
                // Array format (ODCS v3.1.0): properties is an array with 'name' field
                for prop_data in properties_list {
                    if let Some(prop_obj) = prop_data.as_object() {
                        // Extract name from property object (required in v3.1.0)
                        let nested_name = prop_obj
                            .get("name")
                            .or_else(|| prop_obj.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if !nested_name.is_empty() {
                            let nested_nullable = !prop_obj
                                .get("required")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            expand_nested_column(
                                &format!("{}.{}", column_name, nested_name),
                                prop_data,
                                nullable || nested_nullable,
                                columns,
                                errors,
                            );
                        }
                    }
                }
            } else {
                // Object without properties - create as OBJECT type
                let physical_type = schema_obj
                    .get("physicalType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = schema_obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                columns.push(Column {
                    name: column_name.to_string(),
                    data_type: "OBJECT".to_string(),
                    physical_type,
                    nullable,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description,
                    quality: Vec::new(),
                    relationships: Vec::new(),
                    enum_values: Vec::new(),
                    errors: Vec::new(),
                    column_order: 0,
                    nested_data: None,
                });
            }
        }
        "array" => {
            // Handle array types
            let items = schema_obj.get("items").unwrap_or(schema);
            // Check both "logicalType" (ODCS v3.1.0) and "type" (legacy) for backward compatibility
            let items_obj = items.as_object();
            let items_type_raw = items_obj
                .and_then(|obj| {
                    obj.get("logicalType")
                        .and_then(|v| v.as_str())
                        .or_else(|| obj.get("type").and_then(|v| v.as_str()))
                })
                .unwrap_or("string");

            // Normalize legacy "type" values to "logicalType" equivalents for backward compatibility
            let items_type = match items_type_raw {
                "object" | "struct" => "object",
                "array" => "array",
                "string" | "varchar" | "char" | "text" => "string",
                "integer" | "int" | "bigint" | "smallint" | "tinyint" => "integer",
                "number" | "decimal" | "double" | "float" | "numeric" => "number",
                "boolean" | "bool" => "boolean",
                "date" => "date",
                "timestamp" | "datetime" => "timestamp",
                "time" => "time",
                _ => items_type_raw,
            };

            if items_type == "object" {
                // Array of objects - expand nested structure
                let physical_type = schema_obj
                    .get("physicalType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = schema_obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                columns.push(Column {
                    name: column_name.to_string(),
                    data_type: "ARRAY<OBJECT>".to_string(),
                    physical_type,
                    nullable,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description,
                    quality: Vec::new(),
                    relationships: Vec::new(),
                    enum_values: Vec::new(),
                    errors: Vec::new(),
                    column_order: 0,
                    nested_data: None,
                });
                // Also expand nested properties with array prefix
                // Handle both object format (legacy) and array format (ODCS v3.1.0)
                let properties_obj = items
                    .as_object()
                    .and_then(|obj| obj.get("properties"))
                    .and_then(|v| v.as_object());
                let properties_arr = items
                    .as_object()
                    .and_then(|obj| obj.get("properties"))
                    .and_then(|v| v.as_array());

                if let Some(properties_map) = properties_obj {
                    // Object format (legacy): properties is a map
                    let nested_required: Vec<String> = items
                        .as_object()
                        .and_then(|obj| obj.get("required").and_then(|v| v.as_array()))
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    for (nested_name, nested_schema) in properties_map {
                        let nested_nullable = !nested_required.contains(nested_name);
                        expand_nested_column(
                            &format!("{}.[].{}", column_name, nested_name),
                            nested_schema,
                            nullable || nested_nullable,
                            columns,
                            errors,
                        );
                    }
                } else if let Some(properties_list) = properties_arr {
                    // Array format (ODCS v3.1.0): properties is an array with 'name' field
                    for prop_data in properties_list {
                        if let Some(prop_obj) = prop_data.as_object() {
                            // Extract name from property object (required in v3.1.0)
                            let nested_name = prop_obj
                                .get("name")
                                .or_else(|| prop_obj.get("id"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("");

                            if !nested_name.is_empty() {
                                let nested_nullable = !prop_obj
                                    .get("required")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);

                                expand_nested_column(
                                    &format!("{}.[].{}", column_name, nested_name),
                                    prop_data,
                                    nullable || nested_nullable,
                                    columns,
                                    errors,
                                );
                            }
                        }
                    }
                }
            } else {
                // Array of primitives
                let data_type = format!("ARRAY<{}>", items_type.to_uppercase());
                // Extract physicalType (ODCS v3.1.0) - the actual database type
                let physical_type = schema_obj
                    .get("physicalType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let description = schema_obj
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                columns.push(Column {
                    name: column_name.to_string(),
                    data_type,
                    physical_type,
                    nullable,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description,
                    quality: Vec::new(),
                    relationships: Vec::new(),
                    enum_values: Vec::new(),
                    errors: Vec::new(),
                    column_order: 0,
                    nested_data: None,
                });
            }
        }
        _ => {
            // Simple type
            let data_type = schema_type.to_uppercase();
            // Extract physicalType (ODCS v3.1.0) - the actual database type
            let physical_type = schema_obj
                .get("physicalType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let description = schema_obj
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let enum_values = schema_obj
                .get("enum")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            columns.push(Column {
                name: column_name.to_string(),
                data_type,
                physical_type,
                nullable,
                primary_key: false,
                secondary_key: false,
                composite_key: None,
                foreign_key: None,
                constraints: Vec::new(),
                description,
                quality: Vec::new(),
                relationships: Vec::new(),
                enum_values,
                errors: Vec::new(),
                column_order: 0,
                nested_data: None,
            });
        }
    }
}

/// Parse STRUCT fields from string (e.g., "ID: STRING, NAME: STRING").
pub fn parse_struct_fields_from_string(fields_str: &str) -> Result<Vec<(String, String)>> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut depth = 0;
    let mut in_string = false;
    let mut string_char = None;

    for ch in fields_str.chars() {
        match ch {
            '\'' | '"' if !in_string || Some(ch) == string_char => {
                if in_string {
                    in_string = false;
                    string_char = None;
                } else {
                    in_string = true;
                    string_char = Some(ch);
                }
                current_field.push(ch);
            }
            '<' if !in_string => {
                depth += 1;
                current_field.push(ch);
            }
            '>' if !in_string => {
                depth -= 1;
                current_field.push(ch);
            }
            ',' if !in_string && depth == 0 => {
                // End of current field
                let trimmed = current_field.trim();
                if !trimmed.is_empty()
                    && let Some((name, type_part)) = parse_field_definition(trimmed)
                {
                    fields.push((name, type_part));
                }
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    // Handle last field
    let trimmed = current_field.trim();
    if !trimmed.is_empty()
        && let Some((name, type_part)) = parse_field_definition(trimmed)
    {
        fields.push((name, type_part));
    }

    Ok(fields)
}

/// Parse a single field definition (e.g., "ID: STRING" or "DETAILS: STRUCT<...>").
pub fn parse_field_definition(field_def: &str) -> Option<(String, String)> {
    // Split by colon, but handle nested STRUCTs
    let colon_pos = field_def.find(':')?;
    let name = field_def[..colon_pos].trim().to_string();
    let type_part = field_def[colon_pos + 1..].trim().to_string();

    if name.is_empty() || type_part.is_empty() {
        return None;
    }

    Some((name, type_part))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_data_type() {
        assert_eq!(normalize_data_type("string"), "STRING");
        assert_eq!(normalize_data_type("int"), "INT");
        assert_eq!(normalize_data_type("STRUCT<a: INT>"), "STRUCT<a: INT>");
        assert_eq!(normalize_data_type("array<string>"), "ARRAY<string>");
        assert_eq!(normalize_data_type("MAP<string, int>"), "MAP<string, int>");
    }

    #[test]
    fn test_parse_medallion_layer() {
        assert!(matches!(
            parse_medallion_layer("bronze").unwrap(),
            MedallionLayer::Bronze
        ));
        assert!(matches!(
            parse_medallion_layer("SILVER").unwrap(),
            MedallionLayer::Silver
        ));
        assert!(matches!(
            parse_medallion_layer("Gold").unwrap(),
            MedallionLayer::Gold
        ));
        assert!(parse_medallion_layer("invalid").is_err());
    }

    #[test]
    fn test_parse_scd_pattern() {
        assert!(matches!(
            parse_scd_pattern("TYPE_1").unwrap(),
            SCDPattern::Type1
        ));
        assert!(matches!(
            parse_scd_pattern("type2").unwrap(),
            SCDPattern::Type2
        ));
        assert!(parse_scd_pattern("invalid").is_err());
    }

    #[test]
    fn test_parse_data_vault_classification() {
        assert!(matches!(
            parse_data_vault_classification("hub").unwrap(),
            DataVaultClassification::Hub
        ));
        assert!(matches!(
            parse_data_vault_classification("LINK").unwrap(),
            DataVaultClassification::Link
        ));
        assert!(matches!(
            parse_data_vault_classification("sat").unwrap(),
            DataVaultClassification::Satellite
        ));
        assert!(parse_data_vault_classification("invalid").is_err());
    }

    #[test]
    fn test_parse_field_definition() {
        let result = parse_field_definition("name: STRING");
        assert!(result.is_some());
        let (name, type_part) = result.unwrap();
        assert_eq!(name, "name");
        assert_eq!(type_part, "STRING");

        let result = parse_field_definition("nested: STRUCT<a: INT, b: STRING>");
        assert!(result.is_some());
        let (name, type_part) = result.unwrap();
        assert_eq!(name, "nested");
        assert_eq!(type_part, "STRUCT<a: INT, b: STRING>");
    }

    #[test]
    fn test_parse_struct_fields_from_string() {
        let fields = parse_struct_fields_from_string("id: INT, name: STRING").unwrap();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0], ("id".to_string(), "INT".to_string()));
        assert_eq!(fields[1], ("name".to_string(), "STRING".to_string()));

        let fields = parse_struct_fields_from_string(
            "id: INT, nested: STRUCT<a: INT, b: STRING>, name: STRING",
        )
        .unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], ("id".to_string(), "INT".to_string()));
        assert_eq!(
            fields[1],
            (
                "nested".to_string(),
                "STRUCT<a: INT, b: STRING>".to_string()
            )
        );
        assert_eq!(fields[2], ("name".to_string(), "STRING".to_string()));
    }
}
