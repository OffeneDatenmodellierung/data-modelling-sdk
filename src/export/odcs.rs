//! ODCS exporter for generating ODCS v3.1.0 YAML from data models.
//!
//! This module exports data models to ODCS (Open Data Contract Standard) v3.1.0 format only.
//! Legacy ODCL formats are no longer supported for export.

use super::{ExportError, ExportResult};
use crate::models::{DataModel, Table};
use serde_yaml;
use std::collections::HashMap;

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

        // Status - from metadata or default
        if let Some(status) = table.odcl_metadata.get("status")
            && !status.is_null()
        {
            yaml.insert(
                serde_yaml::Value::String("status".to_string()),
                Self::json_to_yaml_value(status),
            );
        }

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
                .map(|t| serde_yaml::Value::String(t.clone()))
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

        // Build properties from columns
        let mut properties = serde_yaml::Mapping::new();

        // Helper function to build nested properties structure
        fn build_nested_properties(
            parent_name: &str,
            all_columns: &[crate::models::Column],
            json_to_yaml_fn: &dyn Fn(&serde_json::Value) -> serde_yaml::Value,
        ) -> Option<serde_yaml::Mapping> {
            let parent_prefix = format!("{}.", parent_name);
            let nested_columns: Vec<&crate::models::Column> = all_columns
                .iter()
                .filter(|col| col.name.starts_with(&parent_prefix))
                .collect();

            if nested_columns.is_empty() {
                return None;
            }

            let mut nested_props = serde_yaml::Mapping::new();

            // Group nested columns by their immediate child name (first level only)
            let mut child_map: std::collections::HashMap<String, Vec<&crate::models::Column>> =
                std::collections::HashMap::new();

            for nested_col in &nested_columns {
                // Safety: skip columns that don't start with the expected prefix
                let Some(relative_name) = nested_col.name.strip_prefix(&parent_prefix) else {
                    continue;
                };
                if let Some(dot_pos) = relative_name.find('.') {
                    let child_name = &relative_name[..dot_pos];
                    child_map
                        .entry(child_name.to_string())
                        .or_default()
                        .push(nested_col);
                } else {
                    // Direct child - add to map
                    child_map
                        .entry(relative_name.to_string())
                        .or_default()
                        .push(nested_col);
                }
            }

            // Build properties for each child
            for (child_name, child_cols) in child_map {
                // Find the direct child column (no dots in relative name)
                let direct_child = child_cols.iter().find(|col| {
                    col.name
                        .strip_prefix(&parent_prefix)
                        .map(|rel_name| !rel_name.contains('.'))
                        .unwrap_or(false)
                });

                if let Some(child_col) = direct_child {
                    let mut child_prop = serde_yaml::Mapping::new();

                    // Check if this child has nested children
                    let child_has_nested = child_cols.iter().any(|col| {
                        col.name
                            .strip_prefix(&parent_prefix)
                            .map(|rel| {
                                rel.starts_with(&format!("{}.", child_name)) && rel != child_name
                            })
                            .unwrap_or(false)
                    });

                    // Handle ARRAY<OBJECT> or ARRAY<STRUCT> types
                    let data_type_upper = child_col.data_type.to_uppercase();
                    let is_array_object = data_type_upper.starts_with("ARRAY<")
                        && (data_type_upper.contains("OBJECT")
                            || data_type_upper.contains("STRUCT"));
                    let is_struct_or_object = data_type_upper == "STRUCT"
                        || data_type_upper == "OBJECT"
                        || data_type_upper.starts_with("STRUCT<");

                    // Try to build nested properties first (regardless of type)
                    let nested_props_map =
                        build_nested_properties(&child_col.name, all_columns, json_to_yaml_fn);

                    if is_array_object && (child_has_nested || nested_props_map.is_some()) {
                        // ARRAY<OBJECT> with nested fields
                        child_prop.insert(
                            serde_yaml::Value::String("type".to_string()),
                            serde_yaml::Value::String("array".to_string()),
                        );

                        let mut items = serde_yaml::Mapping::new();
                        items.insert(
                            serde_yaml::Value::String("type".to_string()),
                            serde_yaml::Value::String("object".to_string()),
                        );

                        // Add nested properties if they exist
                        if let Some(nested_props) = nested_props_map {
                            items.insert(
                                serde_yaml::Value::String("properties".to_string()),
                                serde_yaml::Value::Mapping(nested_props),
                            );
                        }

                        child_prop.insert(
                            serde_yaml::Value::String("items".to_string()),
                            serde_yaml::Value::Mapping(items),
                        );
                    } else if is_struct_or_object || child_has_nested || nested_props_map.is_some()
                    {
                        // OBJECT/STRUCT with nested properties, or any column with nested children
                        child_prop.insert(
                            serde_yaml::Value::String("type".to_string()),
                            serde_yaml::Value::String("object".to_string()),
                        );

                        // Add nested properties if they exist
                        if let Some(nested_props) = nested_props_map {
                            child_prop.insert(
                                serde_yaml::Value::String("properties".to_string()),
                                serde_yaml::Value::Mapping(nested_props),
                            );
                        }
                    } else {
                        // Simple field
                        child_prop.insert(
                            serde_yaml::Value::String("type".to_string()),
                            serde_yaml::Value::String(child_col.data_type.clone().to_lowercase()),
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

                    // Export enum values for nested columns
                    if !child_col.enum_values.is_empty() {
                        let enum_yaml: Vec<serde_yaml::Value> = child_col
                            .enum_values
                            .iter()
                            .map(|e| serde_yaml::Value::String(e.clone()))
                            .collect();
                        child_prop.insert(
                            serde_yaml::Value::String("enum".to_string()),
                            serde_yaml::Value::Sequence(enum_yaml),
                        );
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

                    nested_props.insert(
                        serde_yaml::Value::String(child_name),
                        serde_yaml::Value::Mapping(child_prop),
                    );
                }
            }

            if nested_props.is_empty() {
                None
            } else {
                Some(nested_props)
            }
        }

        for column in &table.columns {
            // Skip nested columns (they're handled as part of parent columns)
            if column.name.contains('.') {
                continue;
            }

            let mut prop = serde_yaml::Mapping::new();

            // Check if this column has nested columns
            let has_nested = table.columns.iter().any(|col| {
                col.name.starts_with(&format!("{}.", column.name)) && col.name != column.name
            });

            // Determine the type - handle ARRAY<OBJECT>, STRUCT, OBJECT, etc.
            let data_type_upper = column.data_type.to_uppercase();
            let is_array_object = data_type_upper.starts_with("ARRAY<")
                && (data_type_upper.contains("OBJECT") || data_type_upper.contains("STRUCT"));
            let is_struct_or_object = data_type_upper == "STRUCT"
                || data_type_upper == "OBJECT"
                || data_type_upper.starts_with("STRUCT<");

            // Always check for nested properties if nested columns exist
            if has_nested {
                // Try to build nested properties first
                let nested_props = build_nested_properties(
                    &column.name,
                    &table.columns,
                    &Self::json_to_yaml_value,
                );

                if is_array_object {
                    // ARRAY<OBJECT> with nested fields
                    prop.insert(
                        serde_yaml::Value::String("type".to_string()),
                        serde_yaml::Value::String("array".to_string()),
                    );

                    let mut items = serde_yaml::Mapping::new();
                    items.insert(
                        serde_yaml::Value::String("type".to_string()),
                        serde_yaml::Value::String("object".to_string()),
                    );

                    // Add nested properties if they exist
                    if let Some(nested_props_map) = nested_props {
                        items.insert(
                            serde_yaml::Value::String("properties".to_string()),
                            serde_yaml::Value::Mapping(nested_props_map),
                        );
                    }

                    prop.insert(
                        serde_yaml::Value::String("items".to_string()),
                        serde_yaml::Value::Mapping(items),
                    );
                } else if is_struct_or_object || nested_props.is_some() {
                    // OBJECT/STRUCT with nested fields, or any column with nested columns
                    prop.insert(
                        serde_yaml::Value::String("type".to_string()),
                        serde_yaml::Value::String("object".to_string()),
                    );

                    // Add nested properties if they exist
                    if let Some(nested_props_map) = nested_props {
                        prop.insert(
                            serde_yaml::Value::String("properties".to_string()),
                            serde_yaml::Value::Mapping(nested_props_map),
                        );
                    }
                } else {
                    // Has nested columns but couldn't build structure - use simple type
                    prop.insert(
                        serde_yaml::Value::String("type".to_string()),
                        serde_yaml::Value::String(column.data_type.clone().to_lowercase()),
                    );
                }
            } else {
                // No nested columns - use simple type
                prop.insert(
                    serde_yaml::Value::String("type".to_string()),
                    serde_yaml::Value::String(column.data_type.clone().to_lowercase()),
                );
            }

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

            // Export enum values
            if !column.enum_values.is_empty() {
                let enum_yaml: Vec<serde_yaml::Value> = column
                    .enum_values
                    .iter()
                    .map(|e| serde_yaml::Value::String(e.clone()))
                    .collect();
                prop.insert(
                    serde_yaml::Value::String("enum".to_string()),
                    serde_yaml::Value::Sequence(enum_yaml),
                );
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

            properties.insert(
                serde_yaml::Value::String(column.name.clone()),
                serde_yaml::Value::Mapping(prop),
            );
        }

        schema_obj.insert(
            serde_yaml::Value::String("properties".to_string()),
            serde_yaml::Value::Mapping(properties),
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
    use crate::models::Column;

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
                secondary_key: false,
                composite_key: None,
                foreign_key: None,
                constraints: Vec::new(),
                description: "Primary key".to_string(),
                errors: Vec::new(),
                quality: Vec::new(),
                enum_values: Vec::new(),
                column_order: 0,
            }],
            database_type: None,
            catalog_name: None,
            schema_name: None,
            medallion_layers: Vec::new(),
            scd_pattern: None,
            data_vault_classification: None,
            modeling_level: None,
            tags: vec!["test".to_string()],
            odcl_metadata: HashMap::new(),
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
