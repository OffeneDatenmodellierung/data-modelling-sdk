//! ODCS exporter for generating ODCS v3.1.0 YAML from data models.
//!
//! This module exports data models to ODCS (Open Data Contract Standard) v3.1.0 format only.
//! Legacy ODCL formats are no longer supported for export.

use super::{ExportError, ExportResult};
use crate::models::{Column, DataModel, Table};
use serde_yaml;
use std::collections::HashMap;

/// Get the physical type for a column.
/// Uses the dedicated physical_type field if available, otherwise falls back to data_type.
fn get_physical_type(column: &Column) -> String {
    column
        .physical_type
        .clone()
        .unwrap_or_else(|| column.data_type.clone())
}

/// Exporter for ODCS (Open Data Contract Standard) v3.1.0 YAML format.
pub struct ODCSExporter;

impl ODCSExporter {
    /// Export a table to ODCS v3.1.0 YAML format.
    ///
    /// Note: Only ODCS v3.1.0 format is supported. Legacy formats have been removed.
    ///
    /// # Arguments
    ///
    /// * `table` - The table to export
    /// * `_format` - Format parameter (ignored, always uses ODCS v3.1.0)
    ///
    /// # Returns
    ///
    /// A YAML string in ODCS v3.1.0 format.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::export::odcs::ODCSExporter;
    /// use data_modelling_sdk::models::{Table, Column};
    ///
    /// let table = Table::new(
    ///     "users".to_string(),
    ///     vec![Column::new("id".to_string(), "BIGINT".to_string())],
    /// );
    ///
    /// let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");
    /// assert!(yaml.contains("apiVersion: v3.1.0"));
    /// assert!(yaml.contains("kind: DataContract"));
    /// ```
    pub fn export_table(table: &Table, _format: &str) -> String {
        // All exports use ODCS v3.1.0 format
        Self::export_odcs_v3_1_0_format(table)
    }

    /// Parse STRUCT definition from data_type string and create nested properties
    /// This is used when SQL parser doesn't create nested columns but we have STRUCT types
    fn parse_struct_properties_from_data_type(
        parent_name: &str,
        data_type: &str,
        map_data_type_fn: &dyn Fn(&str) -> (String, bool),
    ) -> Option<Vec<serde_yaml::Value>> {
        use crate::import::odcs::ODCSImporter;

        let importer = ODCSImporter::new();
        let field_data = serde_json::Map::new();

        // Extract STRUCT definition from ARRAY<STRUCT<...>> if needed
        let struct_type = if data_type.to_uppercase().starts_with("ARRAY<STRUCT<") {
            // Extract the STRUCT<...> part from ARRAY<STRUCT<...>>
            if let Some(start) = data_type.find("STRUCT<") {
                &data_type[start..]
            } else {
                data_type
            }
        } else {
            data_type
        };

        // Try to parse STRUCT type to get nested columns
        if let Ok(nested_cols) =
            importer.parse_struct_type_from_string(parent_name, struct_type, &field_data)
            && !nested_cols.is_empty()
        {
            // Group nested columns by their immediate child name (handle nested STRUCTs)
            // For ARRAY<STRUCT>, columns have names like: "parent.[].field" or "parent.[].nested.field"
            // We need to group by immediate child: "field" vs "nested.field"
            use std::collections::HashMap;
            let mut props_map: HashMap<String, Vec<&crate::models::Column>> = HashMap::new();

            for nested_col in &nested_cols {
                // Extract the column name after removing parent prefix and array notation
                let name_after_prefix = nested_col
                    .name
                    .strip_prefix(&format!("{}.[]", parent_name))
                    .or_else(|| nested_col.name.strip_prefix(&format!("{}.", parent_name)))
                    .unwrap_or(&nested_col.name);

                // Get the immediate child name (first part before dot, if any)
                let immediate_child = name_after_prefix
                    .split('.')
                    .next()
                    .unwrap_or(name_after_prefix)
                    .to_string();

                props_map
                    .entry(immediate_child)
                    .or_default()
                    .push(nested_col);
            }

            // Convert grouped columns to properties array format
            let mut props_array = Vec::new();
            for (immediate_child, child_cols) in props_map {
                // Skip empty immediate_child names
                if immediate_child.is_empty() {
                    continue;
                }

                // Check if this is a nested STRUCT (has columns with dots after the immediate child)
                let has_nested_struct = child_cols.iter().any(|col| {
                    let name_after_prefix = col
                        .name
                        .strip_prefix(&format!("{}.[]", parent_name))
                        .or_else(|| col.name.strip_prefix(&format!("{}.", parent_name)))
                        .unwrap_or(&col.name);
                    name_after_prefix.contains('.')
                });

                if has_nested_struct {
                    // Nested STRUCT - create object property with nested properties
                    let mut prop_map = serde_yaml::Mapping::new();
                    let id = immediate_child
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() {
                                c.to_lowercase().to_string()
                            } else {
                                "_".to_string()
                            }
                        })
                        .collect::<String>()
                        .replace("__", "_");

                    prop_map.insert(
                        serde_yaml::Value::String("id".to_string()),
                        serde_yaml::Value::String(format!("{}_field", id)),
                    );
                    prop_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(immediate_child.clone()),
                    );
                    prop_map.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String("object".to_string()),
                    );

                    // Recursively build nested properties
                    let mut nested_props = Vec::new();
                    for nested_col in child_cols {
                        // Remove the immediate child prefix to get the nested field name
                        let name_after_prefix = nested_col
                            .name
                            .strip_prefix(&format!("{}.[]", parent_name))
                            .or_else(|| nested_col.name.strip_prefix(&format!("{}.", parent_name)))
                            .unwrap_or(&nested_col.name);

                        let nested_name = name_after_prefix
                            .strip_prefix(&format!("{}.", immediate_child))
                            .unwrap_or(name_after_prefix)
                            .to_string();

                        if !nested_name.is_empty() && nested_name != immediate_child {
                            let (logical_type, _) = map_data_type_fn(&nested_col.data_type);
                            let nested_id = nested_name
                                .chars()
                                .map(|c| {
                                    if c.is_alphanumeric() {
                                        c.to_lowercase().to_string()
                                    } else {
                                        "_".to_string()
                                    }
                                })
                                .collect::<String>()
                                .replace("__", "_");

                            let mut nested_prop = serde_yaml::Mapping::new();
                            nested_prop.insert(
                                serde_yaml::Value::String("id".to_string()),
                                serde_yaml::Value::String(format!("{}_field", nested_id)),
                            );
                            nested_prop.insert(
                                serde_yaml::Value::String("name".to_string()),
                                serde_yaml::Value::String(nested_name),
                            );
                            nested_prop.insert(
                                serde_yaml::Value::String("logicalType".to_string()),
                                serde_yaml::Value::String(logical_type),
                            );
                            nested_prop.insert(
                                serde_yaml::Value::String("physicalType".to_string()),
                                serde_yaml::Value::String(get_physical_type(nested_col)),
                            );

                            if !nested_col.nullable {
                                nested_prop.insert(
                                    serde_yaml::Value::String("required".to_string()),
                                    serde_yaml::Value::Bool(true),
                                );
                            }

                            nested_props.push(serde_yaml::Value::Mapping(nested_prop));
                        }
                    }

                    if !nested_props.is_empty() {
                        prop_map.insert(
                            serde_yaml::Value::String("properties".to_string()),
                            serde_yaml::Value::Sequence(nested_props),
                        );
                    }

                    props_array.push(serde_yaml::Value::Mapping(prop_map));
                } else {
                    // Simple field (no nested STRUCT)
                    let nested_col = child_cols[0];
                    let mut prop_map = serde_yaml::Mapping::new();
                    let (logical_type, _) = map_data_type_fn(&nested_col.data_type);

                    let id = immediate_child
                        .chars()
                        .map(|c| {
                            if c.is_alphanumeric() {
                                c.to_lowercase().to_string()
                            } else {
                                "_".to_string()
                            }
                        })
                        .collect::<String>()
                        .replace("__", "_");

                    prop_map.insert(
                        serde_yaml::Value::String("id".to_string()),
                        serde_yaml::Value::String(format!("{}_field", id)),
                    );
                    prop_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(immediate_child),
                    );
                    prop_map.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String(logical_type),
                    );
                    prop_map.insert(
                        serde_yaml::Value::String("physicalType".to_string()),
                        serde_yaml::Value::String(get_physical_type(nested_col)),
                    );

                    if !nested_col.nullable {
                        prop_map.insert(
                            serde_yaml::Value::String("required".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }

                    if !nested_col.description.is_empty() {
                        prop_map.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(nested_col.description.clone()),
                        );
                    }

                    props_array.push(serde_yaml::Value::Mapping(prop_map));
                }
            }
            return Some(props_array);
        }
        None
    }

    /// Map data type to ODCS logicalType
    /// Returns (logical_type, is_array)
    fn map_data_type_to_logical_type(data_type: &str) -> (String, bool) {
        let upper = data_type.to_uppercase();

        // Check for array types first
        if upper.starts_with("ARRAY<") {
            return ("array".to_string(), true);
        }

        // Map to ODCS logical types
        if upper.contains("INT") || upper == "BIGINT" || upper == "SMALLINT" || upper == "TINYINT" {
            ("integer".to_string(), false)
        } else if upper.contains("DECIMAL")
            || upper.contains("DOUBLE")
            || upper.contains("FLOAT")
            || upper.contains("NUMERIC")
            || upper == "NUMBER"
        {
            ("number".to_string(), false)
        } else if upper == "BOOLEAN" || upper == "BOOL" {
            ("boolean".to_string(), false)
        } else if upper == "DATE" {
            ("date".to_string(), false)
        } else if upper.contains("TIMESTAMP") {
            ("timestamp".to_string(), false)
        } else if upper == "TIME" {
            ("time".to_string(), false)
        } else if upper == "STRUCT" || upper == "OBJECT" || upper.starts_with("STRUCT<") {
            ("object".to_string(), false)
        } else {
            // Default to string for VARCHAR, CHAR, STRING, TEXT, etc.
            ("string".to_string(), false)
        }
    }

    /// Helper to convert serde_json::Value to serde_yaml::Value
    fn json_to_yaml_value(json: &serde_json::Value) -> serde_yaml::Value {
        match json {
            serde_json::Value::Null => serde_yaml::Value::Null,
            serde_json::Value::Bool(b) => serde_yaml::Value::Bool(*b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    serde_yaml::Value::Number(serde_yaml::Number::from(i))
                } else if let Some(f) = n.as_f64() {
                    serde_yaml::Value::Number(serde_yaml::Number::from(f))
                } else {
                    serde_yaml::Value::String(n.to_string())
                }
            }
            serde_json::Value::String(s) => serde_yaml::Value::String(s.clone()),
            serde_json::Value::Array(arr) => {
                let yaml_arr: Vec<serde_yaml::Value> =
                    arr.iter().map(Self::json_to_yaml_value).collect();
                serde_yaml::Value::Sequence(yaml_arr)
            }
            serde_json::Value::Object(obj) => {
                let mut yaml_map = serde_yaml::Mapping::new();
                for (k, v) in obj {
                    yaml_map.insert(
                        serde_yaml::Value::String(k.clone()),
                        Self::json_to_yaml_value(v),
                    );
                }
                serde_yaml::Value::Mapping(yaml_map)
            }
        }
    }

    /// Export in ODCS v3.1.0 format (the only supported export format).
    fn export_odcs_v3_1_0_format(table: &Table) -> String {
        let mut yaml = serde_yaml::Mapping::new();

        // Required ODCS v3.1.0 fields
        yaml.insert(
            serde_yaml::Value::String("apiVersion".to_string()),
            serde_yaml::Value::String("v3.1.0".to_string()),
        );
        yaml.insert(
            serde_yaml::Value::String("kind".to_string()),
            serde_yaml::Value::String("DataContract".to_string()),
        );

        // ID - use table UUID (ODCS spec: "A unique identifier used to reduce the risk of dataset name collisions, such as a UUID.")
        yaml.insert(
            serde_yaml::Value::String("id".to_string()),
            serde_yaml::Value::String(table.id.to_string()),
        );

        // Name
        yaml.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(table.name.clone()),
        );

        // Version - from metadata or default
        let version = table
            .odcl_metadata
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "1.0.0".to_string());
        yaml.insert(
            serde_yaml::Value::String("version".to_string()),
            serde_yaml::Value::String(version),
        );

        // Status - from metadata or default to "draft" (required field in ODCS v3.1.0)
        let status_value = table
            .odcl_metadata
            .get("status")
            .and_then(|v| {
                if v.is_null() {
                    None
                } else {
                    Some(Self::json_to_yaml_value(v))
                }
            })
            .unwrap_or_else(|| serde_yaml::Value::String("draft".to_string()));
        yaml.insert(
            serde_yaml::Value::String("status".to_string()),
            status_value,
        );

        // Domain - from metadata
        if let Some(domain) = table.odcl_metadata.get("domain")
            && !domain.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("domain".to_string()),
                Self::json_to_yaml_value(domain),
            );
        }

        // Data Product - from metadata
        if let Some(data_product) = table.odcl_metadata.get("dataProduct")
            && !data_product.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("dataProduct".to_string()),
                Self::json_to_yaml_value(data_product),
            );
        }

        // Tenant - from metadata
        if let Some(tenant) = table.odcl_metadata.get("tenant")
            && !tenant.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("tenant".to_string()),
                Self::json_to_yaml_value(tenant),
            );
        }

        // Description - from metadata (can be object or string)
        if let Some(description) = table.odcl_metadata.get("description")
            && !description.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("description".to_string()),
                Self::json_to_yaml_value(description),
            );
        }

        // Tags
        if !table.tags.is_empty() {
            let tags_yaml: Vec<serde_yaml::Value> = table
                .tags
                .iter()
                .map(|t| serde_yaml::Value::String(t.to_string()))
                .collect();
            yaml.insert(
                serde_yaml::Value::String("tags".to_string()),
                serde_yaml::Value::Sequence(tags_yaml),
            );
        }

        // Team - from metadata
        if let Some(team) = table.odcl_metadata.get("team")
            && !team.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("team".to_string()),
                Self::json_to_yaml_value(team),
            );
        }

        // Roles - from metadata
        if let Some(roles) = table.odcl_metadata.get("roles")
            && !roles.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("roles".to_string()),
                Self::json_to_yaml_value(roles),
            );
        }

        // Pricing - from metadata (ODCS uses "price")
        if let Some(pricing) = table.odcl_metadata.get("pricing")
            && !pricing.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("price".to_string()),
                Self::json_to_yaml_value(pricing),
            );
        }

        // Terms - from metadata
        if let Some(terms) = table.odcl_metadata.get("terms")
            && !terms.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("terms".to_string()),
                Self::json_to_yaml_value(terms),
            );
        }

        // Servers - from metadata
        if let Some(servers) = table.odcl_metadata.get("servers")
            && !servers.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("servers".to_string()),
                Self::json_to_yaml_value(servers),
            );
        }

        // Service Levels - from metadata
        if let Some(servicelevels) = table.odcl_metadata.get("servicelevels")
            && !servicelevels.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("servicelevels".to_string()),
                Self::json_to_yaml_value(servicelevels),
            );
        }

        // Links - from metadata
        if let Some(links) = table.odcl_metadata.get("links")
            && !links.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("links".to_string()),
                Self::json_to_yaml_value(links),
            );
        }

        // Infrastructure - from metadata
        if let Some(infrastructure) = table.odcl_metadata.get("infrastructure")
            && !infrastructure.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("infrastructure".to_string()),
                Self::json_to_yaml_value(infrastructure),
            );
        }

        // Schema array (ODCS v3.1.0 uses array of SchemaObject)
        let mut schema_array = Vec::new();
        let mut schema_obj = serde_yaml::Mapping::new();

        schema_obj.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(table.name.clone()),
        );

        // Build properties from columns (ODCS v3.1.0 uses array format)
        let mut properties = Vec::new();

        // Helper function to convert Mapping properties to Array format (ODCS v3.1.0)
        fn mapping_to_properties_array(props_map: serde_yaml::Mapping) -> Vec<serde_yaml::Value> {
            let mut props_array = Vec::new();
            for (key, value) in props_map {
                if let serde_yaml::Value::String(name) = key
                    && let serde_yaml::Value::Mapping(mut prop_map) = value
                {
                    // Add 'name' field to property object (required in ODCS v3.1.0)
                    prop_map.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(name.clone()),
                    );
                    props_array.push(serde_yaml::Value::Mapping(prop_map));
                }
            }
            props_array
        }

        // Helper function to build nested properties structure (returns array format for ODCS v3.1.0)
        fn build_nested_properties(
            parent_name: &str,
            _table_name: &str,
            all_columns: &[crate::models::Column],
            json_to_yaml_fn: &dyn Fn(&serde_json::Value) -> serde_yaml::Value,
            map_data_type_fn: &dyn Fn(&str) -> (String, bool),
        ) -> Option<Vec<serde_yaml::Value>> {
            // Handle both dot notation (parent.field) and array notation (parent.[].field)
            let parent_prefix_dot = format!("{}.", parent_name);
            let parent_prefix_array = format!("{}.[].", parent_name); // Include trailing dot for proper stripping
            let parent_prefix_array_no_dot = format!("{}.[]", parent_name); // For filtering
            let nested_columns: Vec<&crate::models::Column> = all_columns
                .iter()
                .filter(|col| {
                    col.name.starts_with(&parent_prefix_dot)
                        || col.name.starts_with(&parent_prefix_array_no_dot)
                })
                .collect();

            if nested_columns.is_empty() {
                return None;
            }

            let mut nested_props_map = serde_yaml::Mapping::new();

            // Group nested columns by their immediate child name (first level only)
            let mut child_map: std::collections::HashMap<String, Vec<&crate::models::Column>> =
                std::collections::HashMap::new();

            for nested_col in &nested_columns {
                // Handle both dot notation (parent.field) and array notation (parent.[].field)
                // Also handle deeply nested structures like parent.[].nested.[].field
                let relative_name = if nested_col.name.starts_with(&parent_prefix_array) {
                    // For array notation: parent.[].field -> field
                    // Or parent.[].nested.[].field -> nested.[].field (for nested arrays)
                    nested_col
                        .name
                        .strip_prefix(&parent_prefix_array)
                        .unwrap_or("")
                } else if nested_col.name.starts_with(&parent_prefix_dot) {
                    // For dot notation: parent.field -> field
                    // Or parent.nested.field -> nested.field (for nested structs)
                    nested_col
                        .name
                        .strip_prefix(&parent_prefix_dot)
                        .unwrap_or("")
                } else {
                    continue;
                };

                if relative_name.is_empty() {
                    continue;
                }

                // Find the immediate child name (first level only)
                // For "nested.[].field" -> "nested"
                // For "nested.field" -> "nested"
                // For "field" -> "field"
                let child_name = if let Some(dot_pos) = relative_name.find('.') {
                    // Check if it's array notation (.[]) or regular dot notation
                    if relative_name.starts_with(".[]") {
                        // This shouldn't happen - array notation should be at parent level
                        // But handle it just in case: ".[].field" -> skip
                        continue;
                    } else {
                        // Regular dot: "nested.field" -> "nested"
                        &relative_name[..dot_pos]
                    }
                } else {
                    // Direct child: "field" -> "field"
                    relative_name
                };

                child_map
                    .entry(child_name.to_string())
                    .or_default()
                    .push(nested_col);
            }

            // Build properties for each child
            for (child_name, child_cols) in child_map {
                // Skip empty child names
                if child_name.is_empty() {
                    continue;
                }
                // Find the direct child column (first level only, may have deeper nesting)
                // For ARRAY<STRUCT<...>>, we don't have a parent column - only nested columns exist
                // So we need to find a column that matches the child_name exactly or starts with child_name.
                let direct_child = child_cols.iter().find(|col| {
                    let rel_name = if col.name.starts_with(&parent_prefix_array) {
                        col.name.strip_prefix(&parent_prefix_array)
                    } else if col.name.starts_with(&parent_prefix_dot) {
                        col.name.strip_prefix(&parent_prefix_dot)
                    } else {
                        None
                    };
                    // Match if relative name equals child_name or starts with child_name followed by . or .[]
                    rel_name
                        .map(|rel| {
                            rel == child_name
                                || rel.starts_with(&format!("{}.", child_name))
                                || rel.starts_with(&format!("{}.[]", child_name))
                        })
                        .unwrap_or(false)
                });

                // Check if this child has nested children
                let child_has_nested = child_cols.iter().any(|col| {
                    let rel_name = if col.name.starts_with(&parent_prefix_array) {
                        col.name.strip_prefix(&parent_prefix_array)
                    } else if col.name.starts_with(&parent_prefix_dot) {
                        col.name.strip_prefix(&parent_prefix_dot)
                    } else {
                        None
                    };
                    rel_name
                        .map(|rel| {
                            rel.starts_with(&format!("{}.", child_name)) && rel != child_name
                        })
                        .unwrap_or(false)
                });

                // Find the direct child column
                // For ARRAY<STRUCT<...>>, we might not have a parent column, only nested columns
                // direct_child should already find columns where rel == child_name or starts with child_name.
                // If it's None, find a column where the relative name equals child_name exactly
                let child_col = if let Some(col) = direct_child {
                    Some(*col)
                } else {
                    // Find a column where the relative name equals child_name exactly
                    child_cols
                        .iter()
                        .find(|col| {
                            let rel_name = if col.name.starts_with(&parent_prefix_array) {
                                col.name.strip_prefix(&parent_prefix_array)
                            } else if col.name.starts_with(&parent_prefix_dot) {
                                col.name.strip_prefix(&parent_prefix_dot)
                            } else {
                                None
                            };
                            rel_name.map(|rel| rel == child_name).unwrap_or(false)
                        })
                        .copied()
                };

                if let Some(child_col) = child_col {
                    let mut child_prop = serde_yaml::Mapping::new();

                    // Add the name field (required in ODCS v3.1.0) - add it first
                    child_prop.insert(
                        serde_yaml::Value::String("name".to_string()),
                        serde_yaml::Value::String(child_name.clone()),
                    );

                    // Handle ARRAY<OBJECT> or ARRAY<STRUCT> types
                    let data_type_upper = child_col.data_type.to_uppercase();
                    let is_array_object = data_type_upper.starts_with("ARRAY<")
                        && (data_type_upper.contains("OBJECT")
                            || data_type_upper.contains("STRUCT"));
                    let is_struct_or_object = data_type_upper == "STRUCT"
                        || data_type_upper == "OBJECT"
                        || data_type_upper.starts_with("STRUCT<");

                    // Try to build nested properties first (regardless of type)
                    let nested_props_array = build_nested_properties(
                        &child_col.name,
                        _table_name,
                        all_columns,
                        json_to_yaml_fn,
                        map_data_type_fn,
                    );

                    if is_array_object && (child_has_nested || nested_props_array.is_some()) {
                        // ARRAY<OBJECT> with nested fields
                        child_prop.insert(
                            serde_yaml::Value::String("logicalType".to_string()),
                            serde_yaml::Value::String("array".to_string()),
                        );
                        child_prop.insert(
                            serde_yaml::Value::String("physicalType".to_string()),
                            serde_yaml::Value::String(get_physical_type(child_col)),
                        );

                        let mut items = serde_yaml::Mapping::new();
                        items.insert(
                            serde_yaml::Value::String("logicalType".to_string()),
                            serde_yaml::Value::String("object".to_string()),
                        );

                        // Add nested properties if they exist (already in array format)
                        if let Some(nested_props) = nested_props_array {
                            items.insert(
                                serde_yaml::Value::String("properties".to_string()),
                                serde_yaml::Value::Sequence(nested_props),
                            );
                        }

                        child_prop.insert(
                            serde_yaml::Value::String("items".to_string()),
                            serde_yaml::Value::Mapping(items),
                        );
                    } else if is_struct_or_object
                        || child_has_nested
                        || nested_props_array.is_some()
                    {
                        // OBJECT/STRUCT with nested properties, or any column with nested children
                        child_prop.insert(
                            serde_yaml::Value::String("logicalType".to_string()),
                            serde_yaml::Value::String("object".to_string()),
                        );
                        child_prop.insert(
                            serde_yaml::Value::String("physicalType".to_string()),
                            serde_yaml::Value::String(get_physical_type(child_col)),
                        );

                        // Add nested properties if they exist (already in array format)
                        if let Some(nested_props) = nested_props_array {
                            child_prop.insert(
                                serde_yaml::Value::String("properties".to_string()),
                                serde_yaml::Value::Sequence(nested_props),
                            );
                        }
                    } else {
                        // Simple field
                        let (logical_type, _) = map_data_type_fn(&child_col.data_type);
                        child_prop.insert(
                            serde_yaml::Value::String("logicalType".to_string()),
                            serde_yaml::Value::String(logical_type),
                        );
                        child_prop.insert(
                            serde_yaml::Value::String("physicalType".to_string()),
                            serde_yaml::Value::String(get_physical_type(child_col)),
                        );
                    }

                    if !child_col.nullable {
                        child_prop.insert(
                            serde_yaml::Value::String("required".to_string()),
                            serde_yaml::Value::Bool(true),
                        );
                    }

                    if !child_col.description.is_empty() {
                        child_prop.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(child_col.description.clone()),
                        );
                    }

                    // Export column-level quality rules for nested columns
                    if !child_col.quality.is_empty() {
                        let quality_array: Vec<serde_yaml::Value> = child_col
                            .quality
                            .iter()
                            .map(|rule| {
                                let mut rule_map = serde_yaml::Mapping::new();
                                for (k, v) in rule {
                                    rule_map.insert(
                                        serde_yaml::Value::String(k.clone()),
                                        json_to_yaml_fn(v),
                                    );
                                }
                                serde_yaml::Value::Mapping(rule_map)
                            })
                            .collect();
                        child_prop.insert(
                            serde_yaml::Value::String("quality".to_string()),
                            serde_yaml::Value::Sequence(quality_array),
                        );
                    }

                    // Export relationships array for nested columns (ODCS v3.1.0 format)
                    if !child_col.relationships.is_empty() {
                        let rels_yaml: Vec<serde_yaml::Value> = child_col
                            .relationships
                            .iter()
                            .map(|rel| {
                                let mut rel_map = serde_yaml::Mapping::new();
                                rel_map.insert(
                                    serde_yaml::Value::String("type".to_string()),
                                    serde_yaml::Value::String(rel.relationship_type.clone()),
                                );
                                rel_map.insert(
                                    serde_yaml::Value::String("to".to_string()),
                                    serde_yaml::Value::String(rel.to.clone()),
                                );
                                serde_yaml::Value::Mapping(rel_map)
                            })
                            .collect();
                        child_prop.insert(
                            serde_yaml::Value::String("relationships".to_string()),
                            serde_yaml::Value::Sequence(rels_yaml),
                        );
                    }

                    // Convert enum values to ODCS quality rules for nested columns
                    // ODCS v3.1.0 doesn't support 'enum' field in properties - use quality rules instead
                    if !child_col.enum_values.is_empty() {
                        let quality = child_prop
                            .entry(serde_yaml::Value::String("quality".to_string()))
                            .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));

                        if let serde_yaml::Value::Sequence(quality_rules) = quality {
                            let mut enum_rule = serde_yaml::Mapping::new();
                            enum_rule.insert(
                                serde_yaml::Value::String("type".to_string()),
                                serde_yaml::Value::String("sql".to_string()),
                            );

                            let enum_list: String = child_col
                                .enum_values
                                .iter()
                                .map(|e| format!("'{}'", e.replace('\'', "''")))
                                .collect::<Vec<_>>()
                                .join(", ");
                            let query = format!(
                                "SELECT COUNT(*) FROM ${{table}} WHERE ${{column}} NOT IN ({})",
                                enum_list
                            );

                            enum_rule.insert(
                                serde_yaml::Value::String("query".to_string()),
                                serde_yaml::Value::String(query),
                            );

                            enum_rule.insert(
                                serde_yaml::Value::String("mustBe".to_string()),
                                serde_yaml::Value::Number(serde_yaml::Number::from(0)),
                            );

                            enum_rule.insert(
                                serde_yaml::Value::String("description".to_string()),
                                serde_yaml::Value::String(format!(
                                    "Value must be one of: {}",
                                    child_col.enum_values.join(", ")
                                )),
                            );

                            quality_rules.push(serde_yaml::Value::Mapping(enum_rule));
                        }
                    }

                    // Export constraints for nested columns
                    if !child_col.constraints.is_empty() {
                        let constraints_yaml: Vec<serde_yaml::Value> = child_col
                            .constraints
                            .iter()
                            .map(|c| serde_yaml::Value::String(c.clone()))
                            .collect();
                        child_prop.insert(
                            serde_yaml::Value::String("constraints".to_string()),
                            serde_yaml::Value::Sequence(constraints_yaml),
                        );
                    }

                    // Export foreign key for nested columns
                    if let Some(ref fk) = child_col.foreign_key {
                        let mut fk_map = serde_yaml::Mapping::new();
                        fk_map.insert(
                            serde_yaml::Value::String("table".to_string()),
                            serde_yaml::Value::String(fk.table_id.clone()),
                        );
                        fk_map.insert(
                            serde_yaml::Value::String("column".to_string()),
                            serde_yaml::Value::String(fk.column_name.clone()),
                        );
                        child_prop.insert(
                            serde_yaml::Value::String("foreignKey".to_string()),
                            serde_yaml::Value::Mapping(fk_map),
                        );
                    }

                    nested_props_map.insert(
                        serde_yaml::Value::String(child_name.clone()),
                        serde_yaml::Value::Mapping(child_prop),
                    );
                } else if !child_cols.is_empty() {
                    // No exact match found, but we have columns for this child_name
                    // Use the first column that matches child_name exactly (should be the direct child)
                    if let Some(first_col) = child_cols.iter().find(|col| {
                        let rel_name = if col.name.starts_with(&parent_prefix_array) {
                            col.name.strip_prefix(&parent_prefix_array)
                        } else if col.name.starts_with(&parent_prefix_dot) {
                            col.name.strip_prefix(&parent_prefix_dot)
                        } else {
                            None
                        };
                        rel_name.map(|rel| rel == child_name).unwrap_or(false)
                    }) {
                        let mut child_prop = serde_yaml::Mapping::new();

                        // Add the name field (required in ODCS v3.1.0)
                        child_prop.insert(
                            serde_yaml::Value::String("name".to_string()),
                            serde_yaml::Value::String(child_name.clone()),
                        );

                        // Check if this child has nested children
                        let child_has_nested = child_cols.iter().any(|col| {
                            let rel_name = if col.name.starts_with(&parent_prefix_array) {
                                col.name.strip_prefix(&parent_prefix_array)
                            } else if col.name.starts_with(&parent_prefix_dot) {
                                col.name.strip_prefix(&parent_prefix_dot)
                            } else {
                                None
                            };
                            rel_name
                                .map(|rel| {
                                    rel.starts_with(&format!("{}.", child_name))
                                        && rel != child_name
                                })
                                .unwrap_or(false)
                        });

                        // Try to build nested properties if there are nested children
                        let nested_props_array = if child_has_nested {
                            build_nested_properties(
                                &first_col.name,
                                _table_name,
                                all_columns,
                                json_to_yaml_fn,
                                map_data_type_fn,
                            )
                        } else {
                            None
                        };

                        let data_type_upper = first_col.data_type.to_uppercase();
                        let is_array_object = data_type_upper.starts_with("ARRAY<")
                            && (data_type_upper.contains("OBJECT")
                                || data_type_upper.contains("STRUCT"));
                        let is_struct_or_object = data_type_upper == "STRUCT"
                            || data_type_upper == "OBJECT"
                            || data_type_upper.starts_with("STRUCT<");

                        if is_array_object && (child_has_nested || nested_props_array.is_some()) {
                            child_prop.insert(
                                serde_yaml::Value::String("logicalType".to_string()),
                                serde_yaml::Value::String("array".to_string()),
                            );
                            child_prop.insert(
                                serde_yaml::Value::String("physicalType".to_string()),
                                serde_yaml::Value::String(get_physical_type(first_col)),
                            );

                            let mut items = serde_yaml::Mapping::new();
                            items.insert(
                                serde_yaml::Value::String("logicalType".to_string()),
                                serde_yaml::Value::String("object".to_string()),
                            );

                            if let Some(nested_props) = nested_props_array {
                                items.insert(
                                    serde_yaml::Value::String("properties".to_string()),
                                    serde_yaml::Value::Sequence(nested_props),
                                );
                            }

                            child_prop.insert(
                                serde_yaml::Value::String("items".to_string()),
                                serde_yaml::Value::Mapping(items),
                            );
                        } else if is_struct_or_object
                            || child_has_nested
                            || nested_props_array.is_some()
                        {
                            child_prop.insert(
                                serde_yaml::Value::String("logicalType".to_string()),
                                serde_yaml::Value::String("object".to_string()),
                            );
                            child_prop.insert(
                                serde_yaml::Value::String("physicalType".to_string()),
                                serde_yaml::Value::String(get_physical_type(first_col)),
                            );

                            if let Some(nested_props) = nested_props_array {
                                child_prop.insert(
                                    serde_yaml::Value::String("properties".to_string()),
                                    serde_yaml::Value::Sequence(nested_props),
                                );
                            }
                        } else {
                            let (logical_type, _) = map_data_type_fn(&first_col.data_type);
                            child_prop.insert(
                                serde_yaml::Value::String("logicalType".to_string()),
                                serde_yaml::Value::String(logical_type),
                            );
                            child_prop.insert(
                                serde_yaml::Value::String("physicalType".to_string()),
                                serde_yaml::Value::String(get_physical_type(first_col)),
                            );
                        }

                        if !first_col.nullable {
                            child_prop.insert(
                                serde_yaml::Value::String("required".to_string()),
                                serde_yaml::Value::Bool(true),
                            );
                        }

                        if !first_col.description.is_empty() {
                            child_prop.insert(
                                serde_yaml::Value::String("description".to_string()),
                                serde_yaml::Value::String(first_col.description.clone()),
                            );
                        }

                        nested_props_map.insert(
                            serde_yaml::Value::String(child_name.clone()),
                            serde_yaml::Value::Mapping(child_prop),
                        );
                    }
                } else {
                    // No columns found for this child - skip
                    continue;
                }
            }

            if nested_props_map.is_empty() {
                None
            } else {
                // Convert Mapping to Array format (ODCS v3.1.0)
                Some(mapping_to_properties_array(nested_props_map))
            }
        }

        for column in &table.columns {
            // Skip nested columns (they're handled as part of parent columns)
            if column.name.contains('.') {
                continue;
            }

            let mut prop = serde_yaml::Mapping::new();

            // Check if this column has nested columns
            // Handle both dot notation (parent.field) and array notation (parent.[].field)
            // Also handle deeply nested structures like parent.[].nested.[].field
            let column_prefix_dot = format!("{}.", column.name);
            let column_prefix_array = format!("{}.[]", column.name);
            let has_nested = table.columns.iter().any(|col| {
                (col.name.starts_with(&column_prefix_dot)
                    || col.name.starts_with(&column_prefix_array))
                    && col.name != column.name
            });

            // Determine the type - handle ARRAY, STRUCT, OBJECT, etc.
            let data_type_upper = column.data_type.to_uppercase();

            // Check if this is ARRAY<STRUCT> - can be detected in two ways:
            // 1. data_type == "ARRAY" with full type in description after ||
            // 2. data_type == "ARRAY" or "ARRAY<OBJECT>" with nested columns using .[] notation
            let is_array_struct_from_desc =
                data_type_upper == "ARRAY" && column.description.contains("|| ARRAY<STRUCT<");
            let is_array_struct_from_nested = (data_type_upper == "ARRAY"
                || data_type_upper == "ARRAY<OBJECT>")
                && has_nested
                && table
                    .columns
                    .iter()
                    .any(|col| col.name.starts_with(&format!("{}.[]", column.name)));

            // Also check if data_type is ARRAY<STRING> but description contains ARRAY<STRUCT< (from SQL export)
            let is_array_struct_from_string_type = data_type_upper == "ARRAY<STRING>"
                && column.description.contains("|| ARRAY<STRUCT<");

            let is_array_struct = is_array_struct_from_desc
                || is_array_struct_from_nested
                || is_array_struct_from_string_type;

            // Extract the full ARRAY<STRUCT<...>> type from description if present
            let full_array_struct_type =
                if is_array_struct_from_desc || is_array_struct_from_string_type {
                    column
                        .description
                        .split("|| ")
                        .nth(1)
                        .map(|s| s.trim().to_string())
                } else if is_array_struct_from_nested {
                    // Reconstruct the STRUCT type from nested columns
                    // We'll use build_nested_properties to get the structure, then reconstruct the type string
                    None // Will be handled by parsing nested columns
                } else {
                    None
                };

            let is_array_object = data_type_upper.starts_with("ARRAY<")
                && (data_type_upper.contains("OBJECT") || data_type_upper.contains("STRUCT"));
            let is_struct_or_object = data_type_upper == "STRUCT"
                || data_type_upper == "OBJECT"
                || data_type_upper.starts_with("STRUCT<");

            // Handle ARRAY<STRUCT> types first - parse from description field or nested columns
            if is_array_struct {
                let struct_props = if let Some(ref full_type) = full_array_struct_type {
                    // Parse the STRUCT definition from the full type string (from SQL import)
                    Self::parse_struct_properties_from_data_type(
                        &column.name,
                        full_type,
                        &Self::map_data_type_to_logical_type,
                    )
                } else if is_array_struct_from_nested {
                    // Build properties from nested columns (from ODCS import)
                    build_nested_properties(
                        &column.name,
                        &table.name,
                        &table.columns,
                        &Self::json_to_yaml_value,
                        &Self::map_data_type_to_logical_type,
                    )
                } else {
                    None
                };

                if let Some(props) = struct_props {
                    // Create array of objects structure
                    prop.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String("array".to_string()),
                    );
                    prop.insert(
                        serde_yaml::Value::String("physicalType".to_string()),
                        serde_yaml::Value::String("ARRAY".to_string()),
                    );

                    let mut items = serde_yaml::Mapping::new();
                    items.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String("object".to_string()),
                    );
                    items.insert(
                        serde_yaml::Value::String("properties".to_string()),
                        serde_yaml::Value::Sequence(props),
                    );

                    prop.insert(
                        serde_yaml::Value::String("items".to_string()),
                        serde_yaml::Value::Mapping(items),
                    );

                    // Extract base description (before ||) if present
                    if is_array_struct_from_desc {
                        let base_description =
                            column.description.split("||").next().unwrap_or("").trim();
                        if !base_description.is_empty() {
                            prop.insert(
                                serde_yaml::Value::String("description".to_string()),
                                serde_yaml::Value::String(base_description.to_string()),
                            );
                        }
                    } else if !column.description.is_empty() {
                        // Use full description if not from SQL import
                        prop.insert(
                            serde_yaml::Value::String("description".to_string()),
                            serde_yaml::Value::String(column.description.clone()),
                        );
                    }
                } else {
                    // Fallback: if parsing fails, just use simple array type
                    prop.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String("array".to_string()),
                    );
                    prop.insert(
                        serde_yaml::Value::String("physicalType".to_string()),
                        serde_yaml::Value::String("ARRAY".to_string()),
                    );
                }
            }
            // Always check for nested properties if nested columns exist
            // Also handle STRUCT columns even if nested columns weren't created by SQL parser
            else if has_nested {
                // Try to build nested properties first
                let nested_props = build_nested_properties(
                    &column.name,
                    &table.name,
                    &table.columns,
                    &Self::json_to_yaml_value,
                    &Self::map_data_type_to_logical_type,
                );

                if is_array_object {
                    // ARRAY<OBJECT> with nested fields
                    prop.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String("array".to_string()),
                    );
                    prop.insert(
                        serde_yaml::Value::String("physicalType".to_string()),
                        serde_yaml::Value::String(get_physical_type(column)),
                    );

                    let mut items = serde_yaml::Mapping::new();
                    items.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String("object".to_string()),
                    );

                    // Add nested properties if they exist (already in array format)
                    if let Some(nested_props_array) = nested_props {
                        items.insert(
                            serde_yaml::Value::String("properties".to_string()),
                            serde_yaml::Value::Sequence(nested_props_array),
                        );
                    }

                    prop.insert(
                        serde_yaml::Value::String("items".to_string()),
                        serde_yaml::Value::Mapping(items),
                    );
                } else if is_struct_or_object || nested_props.is_some() {
                    // OBJECT/STRUCT with nested fields, or any column with nested columns
                    prop.insert(
                        serde_yaml::Value::String("logicalType".to_string()),
                        serde_yaml::Value::String("object".to_string()),
                    );
                    prop.insert(
                        serde_yaml::Value::String("physicalType".to_string()),
                        serde_yaml::Value::String(get_physical_type(column)),
                    );

                    // Add nested properties if they exist (already in array format)
                    if let Some(nested_props_array) = nested_props {
                        prop.insert(
                            serde_yaml::Value::String("properties".to_string()),
                            serde_yaml::Value::Sequence(nested_props_array),
                        );
                    }
                } else {
                    // Has nested columns but couldn't build structure - use simple type
                    // But if it's ARRAY<OBJECT> with nested columns, still export as ARRAY with items
                    if is_array_object && has_nested {
                        // Even if build_nested_properties failed, try to create a simple array structure
                        prop.insert(
                            serde_yaml::Value::String("logicalType".to_string()),
                            serde_yaml::Value::String("array".to_string()),
                        );
                        prop.insert(
                            serde_yaml::Value::String("physicalType".to_string()),
                            serde_yaml::Value::String("ARRAY".to_string()),
                        );

                        let mut items = serde_yaml::Mapping::new();
                        items.insert(
                            serde_yaml::Value::String("logicalType".to_string()),
                            serde_yaml::Value::String("object".to_string()),
                        );

                        // Try build_nested_properties one more time - it might work now
                        if let Some(nested_props_array) = build_nested_properties(
                            &column.name,
                            &table.name,
                            &table.columns,
                            &Self::json_to_yaml_value,
                            &Self::map_data_type_to_logical_type,
                        ) {
                            items.insert(
                                serde_yaml::Value::String("properties".to_string()),
                                serde_yaml::Value::Sequence(nested_props_array),
                            );
                        }

                        prop.insert(
                            serde_yaml::Value::String("items".to_string()),
                            serde_yaml::Value::Mapping(items),
                        );
                    } else {
                        let (logical_type, _) =
                            Self::map_data_type_to_logical_type(&column.data_type);
                        prop.insert(
                            serde_yaml::Value::String("logicalType".to_string()),
                            serde_yaml::Value::String(logical_type),
                        );
                        prop.insert(
                            serde_yaml::Value::String("physicalType".to_string()),
                            serde_yaml::Value::String(get_physical_type(column)),
                        );
                    }
                }
            } else if is_struct_or_object {
                // STRUCT/OBJECT type but no nested columns (e.g., from SQL parser that didn't create nested columns)
                // Try to parse STRUCT definition from data_type to create nested properties
                let parsed_props = Self::parse_struct_properties_from_data_type(
                    &column.name,
                    &column.data_type,
                    &Self::map_data_type_to_logical_type,
                );

                prop.insert(
                    serde_yaml::Value::String("logicalType".to_string()),
                    serde_yaml::Value::String("object".to_string()),
                );
                prop.insert(
                    serde_yaml::Value::String("physicalType".to_string()),
                    serde_yaml::Value::String(get_physical_type(column)),
                );

                // Add parsed nested properties if available
                if let Some(nested_props_array) = parsed_props {
                    prop.insert(
                        serde_yaml::Value::String("properties".to_string()),
                        serde_yaml::Value::Sequence(nested_props_array),
                    );
                }
            } else if prop.is_empty() {
                // No nested columns and prop is empty - use simple type
                let (logical_type, _) = Self::map_data_type_to_logical_type(&column.data_type);
                prop.insert(
                    serde_yaml::Value::String("logicalType".to_string()),
                    serde_yaml::Value::String(logical_type),
                );
                prop.insert(
                    serde_yaml::Value::String("physicalType".to_string()),
                    serde_yaml::Value::String(get_physical_type(column)),
                );
            }
            // If prop is not empty (was set by is_array_struct or has_nested blocks), use it as-is

            if !column.nullable {
                prop.insert(
                    serde_yaml::Value::String("required".to_string()),
                    serde_yaml::Value::Bool(true),
                );
            }

            if column.primary_key {
                prop.insert(
                    serde_yaml::Value::String("primaryKey".to_string()),
                    serde_yaml::Value::Bool(true),
                );
            }

            if column.secondary_key {
                prop.insert(
                    serde_yaml::Value::String("businessKey".to_string()),
                    serde_yaml::Value::Bool(true),
                );
            }

            if !column.description.is_empty() {
                prop.insert(
                    serde_yaml::Value::String("description".to_string()),
                    serde_yaml::Value::String(column.description.clone()),
                );
            }

            // Export column-level quality rules
            if !column.quality.is_empty() {
                let quality_array: Vec<serde_yaml::Value> = column
                    .quality
                    .iter()
                    .map(|rule| {
                        let mut rule_map = serde_yaml::Mapping::new();
                        for (k, v) in rule {
                            rule_map.insert(
                                serde_yaml::Value::String(k.clone()),
                                Self::json_to_yaml_value(v),
                            );
                        }
                        serde_yaml::Value::Mapping(rule_map)
                    })
                    .collect();
                prop.insert(
                    serde_yaml::Value::String("quality".to_string()),
                    serde_yaml::Value::Sequence(quality_array),
                );
            }

            // Export relationships array (ODCS v3.1.0 format)
            if !column.relationships.is_empty() {
                let rels_yaml: Vec<serde_yaml::Value> = column
                    .relationships
                    .iter()
                    .map(|rel| {
                        let mut rel_map = serde_yaml::Mapping::new();
                        rel_map.insert(
                            serde_yaml::Value::String("type".to_string()),
                            serde_yaml::Value::String(rel.relationship_type.clone()),
                        );
                        rel_map.insert(
                            serde_yaml::Value::String("to".to_string()),
                            serde_yaml::Value::String(rel.to.clone()),
                        );
                        serde_yaml::Value::Mapping(rel_map)
                    })
                    .collect();
                prop.insert(
                    serde_yaml::Value::String("relationships".to_string()),
                    serde_yaml::Value::Sequence(rels_yaml),
                );
            }

            // Convert enum values to ODCS quality rules
            // ODCS v3.1.0 doesn't support 'enum' field in properties - use quality rules instead
            // Use SQL type with IN clause to validate enum values
            if !column.enum_values.is_empty() {
                // Check if there's already a quality array, if not create one
                let quality = prop
                    .entry(serde_yaml::Value::String("quality".to_string()))
                    .or_insert_with(|| serde_yaml::Value::Sequence(Vec::new()));

                if let serde_yaml::Value::Sequence(quality_rules) = quality {
                    // Create a SQL quality rule for enum values
                    // Use SQL type with IN clause to validate enum values
                    let mut enum_rule = serde_yaml::Mapping::new();
                    enum_rule.insert(
                        serde_yaml::Value::String("type".to_string()),
                        serde_yaml::Value::String("sql".to_string()),
                    );

                    // Build SQL query with IN clause for enum values
                    let enum_list: String = column
                        .enum_values
                        .iter()
                        .map(|e| format!("'{}'", e.replace('\'', "''"))) // Escape single quotes
                        .collect::<Vec<_>>()
                        .join(", ");
                    let query = format!(
                        "SELECT COUNT(*) FROM ${{table}} WHERE ${{column}} NOT IN ({})",
                        enum_list
                    );

                    enum_rule.insert(
                        serde_yaml::Value::String("query".to_string()),
                        serde_yaml::Value::String(query),
                    );

                    enum_rule.insert(
                        serde_yaml::Value::String("mustBe".to_string()),
                        serde_yaml::Value::Number(serde_yaml::Number::from(0)),
                    );

                    enum_rule.insert(
                        serde_yaml::Value::String("description".to_string()),
                        serde_yaml::Value::String(format!(
                            "Value must be one of: {}",
                            column.enum_values.join(", ")
                        )),
                    );

                    quality_rules.push(serde_yaml::Value::Mapping(enum_rule));
                }
            }

            // Export constraints
            if !column.constraints.is_empty() {
                let constraints_yaml: Vec<serde_yaml::Value> = column
                    .constraints
                    .iter()
                    .map(|c| serde_yaml::Value::String(c.clone()))
                    .collect();
                prop.insert(
                    serde_yaml::Value::String("constraints".to_string()),
                    serde_yaml::Value::Sequence(constraints_yaml),
                );
            }

            // Export foreign key
            if let Some(ref fk) = column.foreign_key {
                let mut fk_map = serde_yaml::Mapping::new();
                fk_map.insert(
                    serde_yaml::Value::String("table".to_string()),
                    serde_yaml::Value::String(fk.table_id.clone()),
                );
                fk_map.insert(
                    serde_yaml::Value::String("column".to_string()),
                    serde_yaml::Value::String(fk.column_name.clone()),
                );
                prop.insert(
                    serde_yaml::Value::String("foreignKey".to_string()),
                    serde_yaml::Value::Mapping(fk_map),
                );
            }

            // Add 'id' and 'name' fields to property object (required in ODCS v3.1.0)
            // Generate ID from name (convert to snake_case)
            let id = column
                .name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() {
                        c.to_lowercase().to_string()
                    } else {
                        "_".to_string()
                    }
                })
                .collect::<String>()
                .replace("__", "_");

            prop.insert(
                serde_yaml::Value::String("id".to_string()),
                serde_yaml::Value::String(format!("{}_obj", id)),
            );
            prop.insert(
                serde_yaml::Value::String("name".to_string()),
                serde_yaml::Value::String(column.name.clone()),
            );

            properties.push(serde_yaml::Value::Mapping(prop));
        }

        schema_obj.insert(
            serde_yaml::Value::String("properties".to_string()),
            serde_yaml::Value::Sequence(properties),
        );

        schema_array.push(serde_yaml::Value::Mapping(schema_obj));
        yaml.insert(
            serde_yaml::Value::String("schema".to_string()),
            serde_yaml::Value::Sequence(schema_array),
        );

        // Table-level quality rules
        if !table.quality.is_empty() {
            let quality_array: Vec<serde_yaml::Value> = table
                .quality
                .iter()
                .map(|rule| {
                    let mut rule_map = serde_yaml::Mapping::new();
                    for (k, v) in rule {
                        rule_map.insert(
                            serde_yaml::Value::String(k.clone()),
                            Self::json_to_yaml_value(v),
                        );
                    }
                    serde_yaml::Value::Mapping(rule_map)
                })
                .collect();
            yaml.insert(
                serde_yaml::Value::String("quality".to_string()),
                serde_yaml::Value::Sequence(quality_array),
            );
        }

        // Custom Properties from metadata (excluding already exported fields)
        let excluded_keys = [
            "id",
            "version",
            "status",
            "domain",
            "dataProduct",
            "tenant",
            "description",
            "team",
            "roles",
            "pricing",
            "terms",
            "servers",
            "servicelevels",
            "links",
            "apiVersion",
            "kind",
            "info",
            "dataContractSpecification",
        ];

        let mut custom_props = Vec::new();
        for (key, value) in &table.odcl_metadata {
            if !excluded_keys.contains(&key.as_str()) && !value.is_null() {
                let mut prop = serde_yaml::Mapping::new();
                prop.insert(
                    serde_yaml::Value::String("property".to_string()),
                    serde_yaml::Value::String(key.clone()),
                );
                prop.insert(
                    serde_yaml::Value::String("value".to_string()),
                    Self::json_to_yaml_value(value),
                );
                custom_props.push(serde_yaml::Value::Mapping(prop));
            }
        }

        // Add database type as custom property if present
        if let Some(ref db_type) = table.database_type {
            let mut prop = serde_yaml::Mapping::new();
            prop.insert(
                serde_yaml::Value::String("property".to_string()),
                serde_yaml::Value::String("databaseType".to_string()),
            );
            prop.insert(
                serde_yaml::Value::String("value".to_string()),
                serde_yaml::Value::String(format!("{:?}", db_type)),
            );
            custom_props.push(serde_yaml::Value::Mapping(prop));
        }

        // Add medallion layers as custom property if present
        if !table.medallion_layers.is_empty() {
            let layers: Vec<serde_yaml::Value> = table
                .medallion_layers
                .iter()
                .map(|l| serde_yaml::Value::String(format!("{:?}", l)))
                .collect();
            let mut prop = serde_yaml::Mapping::new();
            prop.insert(
                serde_yaml::Value::String("property".to_string()),
                serde_yaml::Value::String("medallionLayers".to_string()),
            );
            prop.insert(
                serde_yaml::Value::String("value".to_string()),
                serde_yaml::Value::Sequence(layers),
            );
            custom_props.push(serde_yaml::Value::Mapping(prop));
        }

        // Add SCD pattern as custom property if present
        if let Some(ref scd_pattern) = table.scd_pattern {
            let mut prop = serde_yaml::Mapping::new();
            prop.insert(
                serde_yaml::Value::String("property".to_string()),
                serde_yaml::Value::String("scdPattern".to_string()),
            );
            prop.insert(
                serde_yaml::Value::String("value".to_string()),
                serde_yaml::Value::String(format!("{:?}", scd_pattern)),
            );
            custom_props.push(serde_yaml::Value::Mapping(prop));
        }

        // Add Data Vault classification as custom property if present
        if let Some(ref dv_class) = table.data_vault_classification {
            let mut prop = serde_yaml::Mapping::new();
            prop.insert(
                serde_yaml::Value::String("property".to_string()),
                serde_yaml::Value::String("dataVaultClassification".to_string()),
            );
            prop.insert(
                serde_yaml::Value::String("value".to_string()),
                serde_yaml::Value::String(format!("{:?}", dv_class)),
            );
            custom_props.push(serde_yaml::Value::Mapping(prop));
        }

        // Add catalog/schema names as custom properties if present
        if let Some(ref catalog) = table.catalog_name {
            let mut prop = serde_yaml::Mapping::new();
            prop.insert(
                serde_yaml::Value::String("property".to_string()),
                serde_yaml::Value::String("catalogName".to_string()),
            );
            prop.insert(
                serde_yaml::Value::String("value".to_string()),
                serde_yaml::Value::String(catalog.clone()),
            );
            custom_props.push(serde_yaml::Value::Mapping(prop));
        }

        if let Some(ref schema) = table.schema_name {
            let mut prop = serde_yaml::Mapping::new();
            prop.insert(
                serde_yaml::Value::String("property".to_string()),
                serde_yaml::Value::String("schemaName".to_string()),
            );
            prop.insert(
                serde_yaml::Value::String("value".to_string()),
                serde_yaml::Value::String(schema.clone()),
            );
            custom_props.push(serde_yaml::Value::Mapping(prop));
        }

        if !custom_props.is_empty() {
            yaml.insert(
                serde_yaml::Value::String("customProperties".to_string()),
                serde_yaml::Value::Sequence(custom_props),
            );
        }

        // Contract created timestamp
        yaml.insert(
            serde_yaml::Value::String("contractCreatedTs".to_string()),
            serde_yaml::Value::String(table.created_at.to_rfc3339()),
        );

        serde_yaml::to_string(&yaml).unwrap_or_default()
    }

    /// Export tables to ODCS v3.1.0 YAML format (SDK interface).
    pub fn export(
        &self,
        tables: &[Table],
        _format: &str,
    ) -> Result<HashMap<String, ExportResult>, ExportError> {
        let mut exports = HashMap::new();
        for table in tables {
            // All exports use ODCS v3.1.0 format
            let yaml = Self::export_odcs_v3_1_0_format(table);

            // Validate exported YAML against ODCS schema (if feature enabled)
            #[cfg(feature = "schema-validation")]
            {
                #[cfg(feature = "cli")]
                {
                    use crate::cli::validation::validate_odcs;
                    validate_odcs(&yaml).map_err(|e| {
                        ExportError::ValidationError(format!("ODCS validation failed: {}", e))
                    })?;
                }
                #[cfg(not(feature = "cli"))]
                {
                    // Inline validation when CLI feature is not enabled
                    use jsonschema::Validator;
                    use serde_json::Value;

                    let schema_content = include_str!("../../schemas/odcs-json-schema-v3.1.0.json");
                    let schema: Value = serde_json::from_str(schema_content).map_err(|e| {
                        ExportError::ValidationError(format!("Failed to load ODCS schema: {}", e))
                    })?;

                    let validator = Validator::new(&schema).map_err(|e| {
                        ExportError::ValidationError(format!(
                            "Failed to compile ODCS schema: {}",
                            e
                        ))
                    })?;

                    let data: Value = serde_yaml::from_str(&yaml).map_err(|e| {
                        ExportError::ValidationError(format!("Failed to parse YAML: {}", e))
                    })?;

                    if let Err(error) = validator.validate(&data) {
                        return Err(ExportError::ValidationError(format!(
                            "ODCS validation failed: {}",
                            error
                        )));
                    }
                }
            }

            exports.insert(
                table.name.clone(),
                ExportResult {
                    content: yaml,
                    format: "odcs_v3_1_0".to_string(),
                },
            );
        }
        Ok(exports)
    }

    /// Export a data model to ODCS v3.1.0 YAML format (legacy method for compatibility).
    pub fn export_model(
        model: &DataModel,
        table_ids: Option<&[uuid::Uuid]>,
        _format: &str,
    ) -> HashMap<String, String> {
        let tables_to_export: Vec<&Table> = if let Some(ids) = table_ids {
            model
                .tables
                .iter()
                .filter(|t| ids.contains(&t.id))
                .collect()
        } else {
            model.tables.iter().collect()
        };

        let mut exports = HashMap::new();
        for table in tables_to_export {
            // All exports use ODCS v3.1.0 format
            let yaml = Self::export_odcs_v3_1_0_format(table);
            exports.insert(table.name.clone(), yaml);
        }

        exports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Column, Tag};

    #[test]
    fn test_export_odcs_v3_1_0_basic() {
        let table = Table {
            id: Table::generate_id("test_table", None, None, None),
            name: "test_table".to_string(),
            columns: vec![Column {
                name: "id".to_string(),
                data_type: "BIGINT".to_string(),
                nullable: false,
                primary_key: true,
                description: "Primary key".to_string(),
                ..Default::default()
            }],
            database_type: None,
            catalog_name: None,
            schema_name: None,
            medallion_layers: Vec::new(),
            scd_pattern: None,
            data_vault_classification: None,
            modeling_level: None,
            tags: vec![Tag::Simple("test".to_string())],
            odcl_metadata: HashMap::new(),
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

        let yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        assert!(yaml.contains("apiVersion: v3.1.0"));
        assert!(yaml.contains("kind: DataContract"));
        assert!(yaml.contains("name: test_table"));
        assert!(yaml.contains("tags:"));
        assert!(yaml.contains("- test"));
    }
}
