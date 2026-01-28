//! Converters between ODCS native types and legacy Table/Column types
//!
//! These converters enable backwards compatibility with existing APIs while
//! allowing the new ODCS-native types to be used internally.

use super::contract::ODCSContract;
use super::property::Property;
use super::schema::SchemaObject;
use super::supporting::{
    AuthoritativeDefinition as OdcsAuthDef, CustomProperty,
    LogicalTypeOptions as OdcsLogicalTypeOptions, PropertyRelationship as OdcsPropertyRelationship,
    QualityRule,
};
use crate::import::{ColumnData, TableData};
use crate::models::column::{
    AuthoritativeDefinition as ColumnAuthDef, Column,
    LogicalTypeOptions as ColumnLogicalTypeOptions,
    PropertyRelationship as ColumnPropertyRelationship,
};
use crate::models::table::Table;

// ============================================================================
// Data Type Mapping
// ============================================================================

/// Map a SQL/physical data type to an ODCS logical type.
///
/// This maps database-specific types to the ODCS v3.1.0 logical types:
/// - "string", "integer", "number", "boolean", "date", "timestamp", "time", "object", "array"
///
/// Returns (logical_type, is_array)
pub fn map_data_type_to_logical_type(data_type: &str) -> (String, bool) {
    let upper = data_type.to_uppercase();

    // Check for complex types first (before checking for subtypes like INT in STRUCT<...>)
    if upper.starts_with("ARRAY<") {
        return ("array".to_string(), true);
    }
    if upper == "STRUCT" || upper == "OBJECT" || upper.starts_with("STRUCT<") {
        return ("object".to_string(), false);
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
    } else {
        // Default to string for VARCHAR, CHAR, STRING, TEXT, etc.
        ("string".to_string(), false)
    }
}

/// Convert enum values to an ODCS quality rule.
///
/// ODCS v3.1.0 doesn't support an 'enum' field in properties, so we convert
/// enum values to SQL-based quality rules.
fn enum_values_to_quality_rule(enum_values: &[String]) -> QualityRule {
    let enum_list: String = enum_values
        .iter()
        .map(|e| format!("'{}'", e.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(", ");

    let query = format!(
        "SELECT COUNT(*) FROM ${{table}} WHERE ${{column}} NOT IN ({})",
        enum_list
    );

    QualityRule {
        rule_type: Some("sql".to_string()),
        query: Some(query),
        must_be: Some(serde_json::json!(0)),
        description: Some(format!(
            "Value must be one of: {}",
            enum_values.join(", ")
        )),
        ..Default::default()
    }
}

// ============================================================================
// Property <-> Column Converters
// ============================================================================

impl From<&Property> for Column {
    /// Convert a Property to a Column
    ///
    /// This flattens nested properties to dot-notation names for backwards compatibility.
    /// For example, a nested property `address.street` becomes a column named "address.street".
    fn from(prop: &Property) -> Self {
        Column {
            id: prop.id.clone(),
            name: prop.name.clone(),
            business_name: prop.business_name.clone(),
            description: prop.description.clone().unwrap_or_default(),
            data_type: prop.logical_type.clone(),
            physical_type: prop.physical_type.clone(),
            physical_name: prop.physical_name.clone(),
            logical_type_options: prop.logical_type_options.as_ref().map(|opts| {
                ColumnLogicalTypeOptions {
                    min_length: opts.min_length,
                    max_length: opts.max_length,
                    pattern: opts.pattern.clone(),
                    format: opts.format.clone(),
                    minimum: opts.minimum.clone(),
                    maximum: opts.maximum.clone(),
                    exclusive_minimum: opts.exclusive_minimum.clone(),
                    exclusive_maximum: opts.exclusive_maximum.clone(),
                    precision: opts.precision,
                    scale: opts.scale,
                }
            }),
            primary_key: prop.primary_key,
            primary_key_position: prop.primary_key_position,
            unique: prop.unique,
            nullable: !prop.required, // ODCS uses required, Column uses nullable (inverse)
            partitioned: prop.partitioned,
            partition_key_position: prop.partition_key_position,
            clustered: prop.clustered,
            classification: prop.classification.clone(),
            critical_data_element: prop.critical_data_element,
            encrypted_name: prop.encrypted_name.clone(),
            transform_source_objects: prop.transform_source_objects.clone(),
            transform_logic: prop.transform_logic.clone(),
            transform_description: prop.transform_description.clone(),
            examples: prop.examples.clone(),
            default_value: prop.default_value.clone(),
            relationships: prop
                .relationships
                .iter()
                .map(|r| ColumnPropertyRelationship {
                    relationship_type: r.relationship_type.clone(),
                    to: r.to.clone(),
                })
                .collect(),
            authoritative_definitions: prop
                .authoritative_definitions
                .iter()
                .map(|d| ColumnAuthDef {
                    definition_type: d.definition_type.clone(),
                    url: d.url.clone(),
                })
                .collect(),
            quality: prop
                .quality
                .iter()
                .map(|q| serde_json::to_value(q).ok())
                .filter_map(|v| v.and_then(|v| v.as_object().cloned()))
                .map(|m| m.into_iter().collect())
                .collect(),
            enum_values: prop.enum_values.clone(),
            tags: prop.tags.clone(),
            custom_properties: prop
                .custom_properties
                .iter()
                .map(|cp| (cp.property.clone(), cp.value.clone()))
                .collect(),
            // Legacy fields - default values
            secondary_key: false,
            composite_key: None,
            foreign_key: None,
            constraints: Vec::new(),
            errors: Vec::new(),
            column_order: 0,
            nested_data: None,
        }
    }
}

impl From<&Column> for Property {
    /// Convert a Column to a Property
    ///
    /// Note: This creates a flat property. To reconstruct nested structure from
    /// dot-notation column names, use `Property::from_flat_paths()`.
    ///
    /// This converter:
    /// - Maps SQL data types to ODCS logical types (e.g., BIGINT → "integer")
    /// - Sets physical_type from data_type if not already set
    /// - Converts enum_values to quality rules (ODCS spec requirement)
    fn from(col: &Column) -> Self {
        // Map data type to ODCS logical type
        let (logical_type, _is_array) = map_data_type_to_logical_type(&col.data_type);

        // Use physical_type if set, otherwise use the original data_type
        let physical_type = col
            .physical_type
            .clone()
            .or_else(|| Some(col.data_type.clone()));

        // Convert existing quality rules
        let mut quality: Vec<QualityRule> = col
            .quality
            .iter()
            .filter_map(|q| serde_json::to_value(q).ok())
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();

        // Convert enum_values to a quality rule (ODCS v3.1.0 doesn't support enum field)
        if !col.enum_values.is_empty() {
            quality.push(enum_values_to_quality_rule(&col.enum_values));
        }

        Property {
            id: col.id.clone(),
            name: col.name.clone(),
            business_name: col.business_name.clone(),
            description: if col.description.is_empty() {
                None
            } else {
                Some(col.description.clone())
            },
            logical_type,
            physical_type,
            physical_name: col.physical_name.clone(),
            logical_type_options: col.logical_type_options.as_ref().map(|opts| {
                OdcsLogicalTypeOptions {
                    min_length: opts.min_length,
                    max_length: opts.max_length,
                    pattern: opts.pattern.clone(),
                    format: opts.format.clone(),
                    minimum: opts.minimum.clone(),
                    maximum: opts.maximum.clone(),
                    exclusive_minimum: opts.exclusive_minimum.clone(),
                    exclusive_maximum: opts.exclusive_maximum.clone(),
                    precision: opts.precision,
                    scale: opts.scale,
                }
            }),
            required: !col.nullable, // Column uses nullable, ODCS uses required (inverse)
            primary_key: col.primary_key,
            primary_key_position: col.primary_key_position,
            unique: col.unique,
            partitioned: col.partitioned,
            partition_key_position: col.partition_key_position,
            clustered: col.clustered,
            classification: col.classification.clone(),
            critical_data_element: col.critical_data_element,
            encrypted_name: col.encrypted_name.clone(),
            transform_source_objects: col.transform_source_objects.clone(),
            transform_logic: col.transform_logic.clone(),
            transform_description: col.transform_description.clone(),
            examples: col.examples.clone(),
            default_value: col.default_value.clone(),
            relationships: col
                .relationships
                .iter()
                .map(|r| OdcsPropertyRelationship {
                    relationship_type: r.relationship_type.clone(),
                    to: r.to.clone(),
                })
                .collect(),
            authoritative_definitions: col
                .authoritative_definitions
                .iter()
                .map(|d| OdcsAuthDef {
                    definition_type: d.definition_type.clone(),
                    url: d.url.clone(),
                })
                .collect(),
            quality,
            enum_values: col.enum_values.clone(),
            tags: col.tags.clone(),
            custom_properties: col
                .custom_properties
                .iter()
                .map(|(k, v)| CustomProperty::new(k.clone(), v.clone()))
                .collect(),
            items: None,
            properties: Vec::new(),
        }
    }
}

// ============================================================================
// SchemaObject <-> Table Converters
// ============================================================================

impl From<&SchemaObject> for Table {
    /// Convert a SchemaObject to a Table
    ///
    /// This flattens nested properties to dot-notation column names.
    fn from(schema: &SchemaObject) -> Self {
        // Flatten all properties to columns with dot-notation names
        let columns = flatten_properties_to_columns(&schema.properties, "");

        let mut table = Table::new(schema.name.clone(), columns);

        // Set schema-level fields
        table.schema_name = schema.physical_name.clone();

        // Store schema-level metadata in odcl_metadata
        if let Some(ref id) = schema.id {
            table
                .odcl_metadata
                .insert("schemaId".to_string(), serde_json::json!(id));
        }
        if let Some(ref physical_name) = schema.physical_name {
            table
                .odcl_metadata
                .insert("physicalName".to_string(), serde_json::json!(physical_name));
        }
        if let Some(ref physical_type) = schema.physical_type {
            table
                .odcl_metadata
                .insert("physicalType".to_string(), serde_json::json!(physical_type));
        }
        if let Some(ref business_name) = schema.business_name {
            table
                .odcl_metadata
                .insert("businessName".to_string(), serde_json::json!(business_name));
        }
        if let Some(ref description) = schema.description {
            table.odcl_metadata.insert(
                "schemaDescription".to_string(),
                serde_json::json!(description),
            );
        }
        if let Some(ref granularity) = schema.data_granularity_description {
            table.odcl_metadata.insert(
                "dataGranularityDescription".to_string(),
                serde_json::json!(granularity),
            );
        }
        if !schema.tags.is_empty() {
            table
                .odcl_metadata
                .insert("schemaTags".to_string(), serde_json::json!(schema.tags));
        }
        if !schema.relationships.is_empty() {
            table.odcl_metadata.insert(
                "schemaRelationships".to_string(),
                serde_json::to_value(&schema.relationships).unwrap_or_default(),
            );
        }
        if !schema.quality.is_empty() {
            table.quality = schema
                .quality
                .iter()
                .filter_map(|q| serde_json::to_value(q).ok())
                .filter_map(|v| v.as_object().cloned())
                .map(|m| m.into_iter().collect())
                .collect();
        }
        if !schema.authoritative_definitions.is_empty() {
            table.odcl_metadata.insert(
                "authoritativeDefinitions".to_string(),
                serde_json::to_value(&schema.authoritative_definitions).unwrap_or_default(),
            );
        }
        if !schema.custom_properties.is_empty() {
            table.odcl_metadata.insert(
                "customProperties".to_string(),
                serde_json::to_value(&schema.custom_properties).unwrap_or_default(),
            );
        }

        table
    }
}

/// Helper function to flatten nested properties to columns with dot-notation names
fn flatten_properties_to_columns(properties: &[Property], prefix: &str) -> Vec<Column> {
    let mut columns = Vec::new();

    for prop in properties {
        let full_name = if prefix.is_empty() {
            prop.name.clone()
        } else {
            format!("{}.{}", prefix, prop.name)
        };

        // Create column for this property
        let mut col = Column::from(prop);
        col.name = full_name.clone();

        columns.push(col);

        // Recursively flatten nested object properties
        if !prop.properties.is_empty() {
            let nested = flatten_properties_to_columns(&prop.properties, &full_name);
            columns.extend(nested);
        }

        // Handle array items
        if let Some(ref items) = prop.items {
            let items_prefix = format!("{}.[]", full_name);
            let mut items_col = Column::from(items.as_ref());
            items_col.name = items_prefix.clone();
            columns.push(items_col);

            // Recursively flatten array item properties
            if !items.properties.is_empty() {
                let nested = flatten_properties_to_columns(&items.properties, &items_prefix);
                columns.extend(nested);
            }
        }
    }

    columns
}

/// Parse STRUCT fields from a data_type string like "STRUCT<name STRING, status STRING>"
/// Returns a vector of Property objects for the nested fields.
fn parse_struct_fields_from_data_type(data_type: &str) -> Vec<Property> {
    use crate::import::odcs::ODCSImporter;

    let importer = ODCSImporter::new();
    let field_data = serde_json::Map::new();

    // Extract STRUCT definition - handle both STRUCT<...> and ARRAY<STRUCT<...>>
    let struct_type = if data_type.to_uppercase().starts_with("ARRAY<STRUCT<") {
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
        importer.parse_struct_type_from_string("", struct_type, &field_data)
    {
        nested_cols
            .iter()
            .filter_map(|col| {
                // Extract just the field name (remove any prefix like ".[].field" -> "field")
                let field_name = col
                    .name
                    .strip_prefix(".[].")
                    .or_else(|| col.name.strip_prefix("."))
                    .unwrap_or(&col.name);

                if field_name.is_empty() {
                    return None;
                }

                let (logical_type, _) = map_data_type_to_logical_type(&col.data_type);
                Some(Property {
                    name: field_name.to_string(),
                    logical_type,
                    physical_type: Some(col.data_type.clone()),
                    ..Default::default()
                })
            })
            .collect()
    } else {
        Vec::new()
    }
}

/// Fix logical types for parent properties based on their original column data types.
///
/// When columns with ARRAY<STRUCT<...>> or STRUCT<...> types are flattened to dot-notation,
/// the parent property may lose its original type information. This function restores it
/// by looking up the original column's data_type.
///
/// It also expands STRUCT definitions from data_type strings when there are no existing
/// nested properties (i.e., when the STRUCT definition is embedded in the data_type).
fn fix_parent_logical_types(properties: &mut [Property], table: &Table) {
    for prop in properties.iter_mut() {
        // Find the original column for this property
        if let Some(col) = table.columns.iter().find(|c| c.name == prop.name) {
            let data_type_upper = col.data_type.to_uppercase();

            // Check if this is an ARRAY type
            if data_type_upper.starts_with("ARRAY<") {
                prop.logical_type = "array".to_string();

                // If it has nested properties but no items, wrap them in items
                if !prop.properties.is_empty() && prop.items.is_none() {
                    let items_prop = Property {
                        name: String::new(),
                        logical_type: "object".to_string(),
                        properties: std::mem::take(&mut prop.properties),
                        ..Default::default()
                    };
                    prop.items = Some(Box::new(items_prop));
                }
                // If no nested properties but ARRAY<STRUCT<...>>, parse from data_type
                else if prop.properties.is_empty()
                    && prop.items.is_none()
                    && data_type_upper.contains("STRUCT<")
                {
                    let nested_props = parse_struct_fields_from_data_type(&col.data_type);
                    if !nested_props.is_empty() {
                        let items_prop = Property {
                            name: String::new(),
                            logical_type: "object".to_string(),
                            properties: nested_props,
                            ..Default::default()
                        };
                        prop.items = Some(Box::new(items_prop));
                    }
                }
            }
            // Check if this is a STRUCT/OBJECT type
            else if data_type_upper.starts_with("STRUCT<")
                || data_type_upper == "STRUCT"
                || data_type_upper == "OBJECT"
            {
                prop.logical_type = "object".to_string();

                // If no nested properties, parse from data_type string
                if prop.properties.is_empty() && data_type_upper.starts_with("STRUCT<") {
                    prop.properties = parse_struct_fields_from_data_type(&col.data_type);
                }
            }
        }

        // Recursively fix nested properties
        if !prop.properties.is_empty() {
            fix_parent_logical_types(&mut prop.properties, table);
        }

        // Also fix items if present
        if let Some(ref mut items) = prop.items {
            if !items.properties.is_empty() {
                // For array items, we need to check columns with the .[] suffix
                let items_prefix = format!("{}.[]", prop.name);
                if let Some(items_col) = table.columns.iter().find(|c| c.name == items_prefix) {
                    let items_type_upper = items_col.data_type.to_uppercase();
                    if items_type_upper.starts_with("STRUCT<")
                        || items_type_upper == "STRUCT"
                        || items_type_upper == "OBJECT"
                    {
                        items.logical_type = "object".to_string();
                    }
                }
                fix_parent_logical_types(&mut items.properties, table);
            }
        }
    }
}

impl From<&Table> for SchemaObject {
    /// Convert a Table to a SchemaObject
    ///
    /// This reconstructs nested property structure from dot-notation column names.
    /// It also properly handles ARRAY and STRUCT types based on the column's data_type.
    fn from(table: &Table) -> Self {
        // Build flat property list first
        let flat_props: Vec<(String, Property)> = table
            .columns
            .iter()
            .map(|col| (col.name.clone(), Property::from(col)))
            .collect();

        // Reconstruct nested structure
        let mut properties = Property::from_flat_paths(&flat_props);

        // Post-process to fix logical types for parent properties that have nested children
        fix_parent_logical_types(&mut properties, table);

        let mut schema = SchemaObject::new(table.name.clone()).with_properties(properties);

        // Extract schema-level metadata from odcl_metadata
        if let Some(id) = table.odcl_metadata.get("schemaId").and_then(|v| v.as_str()) {
            schema.id = Some(id.to_string());
        }
        if let Some(physical_name) = table
            .odcl_metadata
            .get("physicalName")
            .and_then(|v| v.as_str())
        {
            schema.physical_name = Some(physical_name.to_string());
        } else if let Some(ref sn) = table.schema_name {
            schema.physical_name = Some(sn.clone());
        }
        if let Some(physical_type) = table
            .odcl_metadata
            .get("physicalType")
            .and_then(|v| v.as_str())
        {
            schema.physical_type = Some(physical_type.to_string());
        }
        if let Some(business_name) = table
            .odcl_metadata
            .get("businessName")
            .and_then(|v| v.as_str())
        {
            schema.business_name = Some(business_name.to_string());
        }
        if let Some(description) = table
            .odcl_metadata
            .get("schemaDescription")
            .and_then(|v| v.as_str())
        {
            schema.description = Some(description.to_string());
        }
        if let Some(granularity) = table
            .odcl_metadata
            .get("dataGranularityDescription")
            .and_then(|v| v.as_str())
        {
            schema.data_granularity_description = Some(granularity.to_string());
        }
        if let Some(tags) = table.odcl_metadata.get("schemaTags")
            && let Ok(parsed_tags) = serde_json::from_value::<Vec<String>>(tags.clone())
        {
            schema.tags = parsed_tags;
        }
        if let Some(rels) = table.odcl_metadata.get("schemaRelationships")
            && let Ok(parsed_rels) = serde_json::from_value(rels.clone())
        {
            schema.relationships = parsed_rels;
        }
        if !table.quality.is_empty() {
            schema.quality = table
                .quality
                .iter()
                .filter_map(|q| serde_json::to_value(q).ok())
                .filter_map(|v| serde_json::from_value(v).ok())
                .collect();
        }
        if let Some(auth_defs) = table.odcl_metadata.get("authoritativeDefinitions")
            && let Ok(parsed) = serde_json::from_value(auth_defs.clone())
        {
            schema.authoritative_definitions = parsed;
        }
        if let Some(custom) = table.odcl_metadata.get("customProperties")
            && let Ok(parsed) = serde_json::from_value(custom.clone())
        {
            schema.custom_properties = parsed;
        }

        schema
    }
}

// ============================================================================
// ODCSContract <-> Vec<Table> Converters
// ============================================================================

impl ODCSContract {
    /// Convert the contract to a vector of Tables
    ///
    /// Each SchemaObject becomes a Table, with contract-level metadata
    /// stored in each table's odcl_metadata.
    pub fn to_tables(&self) -> Vec<Table> {
        self.schema
            .iter()
            .map(|schema| {
                let mut table = Table::from(schema);

                // Store contract-level metadata
                table.odcl_metadata.insert(
                    "apiVersion".to_string(),
                    serde_json::json!(self.api_version),
                );
                table
                    .odcl_metadata
                    .insert("kind".to_string(), serde_json::json!(self.kind));
                table
                    .odcl_metadata
                    .insert("contractId".to_string(), serde_json::json!(self.id));
                table
                    .odcl_metadata
                    .insert("version".to_string(), serde_json::json!(self.version));
                table
                    .odcl_metadata
                    .insert("contractName".to_string(), serde_json::json!(self.name));

                if let Some(ref status) = self.status {
                    table
                        .odcl_metadata
                        .insert("status".to_string(), serde_json::json!(status));
                }
                if let Some(ref domain) = self.domain {
                    table
                        .odcl_metadata
                        .insert("domain".to_string(), serde_json::json!(domain));
                }
                if let Some(ref data_product) = self.data_product {
                    table
                        .odcl_metadata
                        .insert("dataProduct".to_string(), serde_json::json!(data_product));
                }
                if let Some(ref tenant) = self.tenant {
                    table
                        .odcl_metadata
                        .insert("tenant".to_string(), serde_json::json!(tenant));
                }
                if let Some(ref description) = self.description {
                    table.odcl_metadata.insert(
                        "description".to_string(),
                        serde_json::to_value(description).unwrap_or_default(),
                    );
                }
                if !self.servers.is_empty() {
                    table.odcl_metadata.insert(
                        "servers".to_string(),
                        serde_json::to_value(&self.servers).unwrap_or_default(),
                    );
                }
                if let Some(ref team) = self.team {
                    table.odcl_metadata.insert(
                        "team".to_string(),
                        serde_json::to_value(team).unwrap_or_default(),
                    );
                }
                if let Some(ref support) = self.support {
                    table.odcl_metadata.insert(
                        "support".to_string(),
                        serde_json::to_value(support).unwrap_or_default(),
                    );
                }
                if !self.roles.is_empty() {
                    table.odcl_metadata.insert(
                        "roles".to_string(),
                        serde_json::to_value(&self.roles).unwrap_or_default(),
                    );
                }
                if !self.service_levels.is_empty() {
                    table.odcl_metadata.insert(
                        "serviceLevels".to_string(),
                        serde_json::to_value(&self.service_levels).unwrap_or_default(),
                    );
                }
                if !self.quality.is_empty() {
                    table.odcl_metadata.insert(
                        "contractQuality".to_string(),
                        serde_json::to_value(&self.quality).unwrap_or_default(),
                    );
                }
                if let Some(ref price) = self.price {
                    table.odcl_metadata.insert(
                        "price".to_string(),
                        serde_json::to_value(price).unwrap_or_default(),
                    );
                }
                if let Some(ref terms) = self.terms {
                    table.odcl_metadata.insert(
                        "terms".to_string(),
                        serde_json::to_value(terms).unwrap_or_default(),
                    );
                }
                if !self.links.is_empty() {
                    table.odcl_metadata.insert(
                        "links".to_string(),
                        serde_json::to_value(&self.links).unwrap_or_default(),
                    );
                }
                if !self.authoritative_definitions.is_empty() {
                    table.odcl_metadata.insert(
                        "contractAuthoritativeDefinitions".to_string(),
                        serde_json::to_value(&self.authoritative_definitions).unwrap_or_default(),
                    );
                }
                if !self.tags.is_empty() {
                    table
                        .odcl_metadata
                        .insert("contractTags".to_string(), serde_json::json!(self.tags));
                }
                if !self.custom_properties.is_empty() {
                    table.odcl_metadata.insert(
                        "contractCustomProperties".to_string(),
                        serde_json::to_value(&self.custom_properties).unwrap_or_default(),
                    );
                }
                if let Some(ref ts) = self.contract_created_ts {
                    table
                        .odcl_metadata
                        .insert("contractCreatedTs".to_string(), serde_json::json!(ts));
                }

                table
            })
            .collect()
    }

    /// Create a contract from a vector of Tables
    ///
    /// Contract-level metadata is extracted from the first table's odcl_metadata.
    /// Each table becomes a SchemaObject.
    pub fn from_tables(tables: &[Table]) -> Self {
        if tables.is_empty() {
            return ODCSContract::default();
        }

        let first_table = &tables[0];
        let mut contract = ODCSContract::default();

        // Extract contract-level metadata from first table
        if let Some(api_version) = first_table
            .odcl_metadata
            .get("apiVersion")
            .and_then(|v| v.as_str())
        {
            contract.api_version = api_version.to_string();
        }
        if let Some(kind) = first_table
            .odcl_metadata
            .get("kind")
            .and_then(|v| v.as_str())
        {
            contract.kind = kind.to_string();
        }
        if let Some(id) = first_table
            .odcl_metadata
            .get("contractId")
            .and_then(|v| v.as_str())
        {
            contract.id = id.to_string();
        }
        if let Some(version) = first_table
            .odcl_metadata
            .get("version")
            .and_then(|v| v.as_str())
        {
            contract.version = version.to_string();
        }
        if let Some(name) = first_table
            .odcl_metadata
            .get("contractName")
            .and_then(|v| v.as_str())
        {
            contract.name = name.to_string();
        }
        if let Some(status) = first_table
            .odcl_metadata
            .get("status")
            .and_then(|v| v.as_str())
        {
            contract.status = Some(status.to_string());
        }
        if let Some(domain) = first_table
            .odcl_metadata
            .get("domain")
            .and_then(|v| v.as_str())
        {
            contract.domain = Some(domain.to_string());
        }
        if let Some(data_product) = first_table
            .odcl_metadata
            .get("dataProduct")
            .and_then(|v| v.as_str())
        {
            contract.data_product = Some(data_product.to_string());
        }
        if let Some(tenant) = first_table
            .odcl_metadata
            .get("tenant")
            .and_then(|v| v.as_str())
        {
            contract.tenant = Some(tenant.to_string());
        }
        if let Some(description) = first_table.odcl_metadata.get("description") {
            contract.description = serde_json::from_value(description.clone()).ok();
        }
        if let Some(servers) = first_table.odcl_metadata.get("servers") {
            contract.servers = serde_json::from_value(servers.clone()).unwrap_or_default();
        }
        if let Some(team) = first_table.odcl_metadata.get("team") {
            contract.team = serde_json::from_value(team.clone()).ok();
        }
        if let Some(support) = first_table.odcl_metadata.get("support") {
            contract.support = serde_json::from_value(support.clone()).ok();
        }
        if let Some(roles) = first_table.odcl_metadata.get("roles") {
            contract.roles = serde_json::from_value(roles.clone()).unwrap_or_default();
        }
        if let Some(service_levels) = first_table.odcl_metadata.get("serviceLevels") {
            contract.service_levels =
                serde_json::from_value(service_levels.clone()).unwrap_or_default();
        }
        if let Some(quality) = first_table.odcl_metadata.get("contractQuality") {
            contract.quality = serde_json::from_value(quality.clone()).unwrap_or_default();
        }
        if let Some(price) = first_table.odcl_metadata.get("price") {
            contract.price = serde_json::from_value(price.clone()).ok();
        }
        if let Some(terms) = first_table.odcl_metadata.get("terms") {
            contract.terms = serde_json::from_value(terms.clone()).ok();
        }
        if let Some(links) = first_table.odcl_metadata.get("links") {
            contract.links = serde_json::from_value(links.clone()).unwrap_or_default();
        }
        if let Some(auth_defs) = first_table
            .odcl_metadata
            .get("contractAuthoritativeDefinitions")
        {
            contract.authoritative_definitions =
                serde_json::from_value(auth_defs.clone()).unwrap_or_default();
        }
        if let Some(tags) = first_table.odcl_metadata.get("contractTags") {
            contract.tags = serde_json::from_value(tags.clone()).unwrap_or_default();
        }
        if let Some(custom) = first_table.odcl_metadata.get("contractCustomProperties") {
            contract.custom_properties = serde_json::from_value(custom.clone()).unwrap_or_default();
        }
        if let Some(ts) = first_table
            .odcl_metadata
            .get("contractCreatedTs")
            .and_then(|v| v.as_str())
        {
            contract.contract_created_ts = Some(ts.to_string());
        }

        // Convert each table to a schema object
        contract.schema = tables.iter().map(SchemaObject::from).collect();

        contract
    }

    /// Create a contract from a single Table with full metadata preservation.
    ///
    /// This is the preferred method for exporting a Table to ODCS format.
    /// It extracts all contract-level metadata from the table's odcl_metadata
    /// and creates a properly structured ODCSContract.
    ///
    /// Key behaviors:
    /// - Uses table.id as the contract id
    /// - Uses table.name as the contract name (if not in metadata)
    /// - Extracts version, status, domain, etc. from odcl_metadata
    /// - Handles additional fields like infrastructure, servicelevels, pricing
    /// - Preserves table tags at the contract level
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::models::table::Table;
    /// use data_modelling_core::models::column::Column;
    /// use data_modelling_core::models::odcs::ODCSContract;
    ///
    /// let mut table = Table::new(
    ///     "users".to_string(),
    ///     vec![Column::new("id".to_string(), "BIGINT".to_string())],
    /// );
    /// table.odcl_metadata.insert("version".to_string(), serde_json::json!("1.0.0"));
    /// table.odcl_metadata.insert("status".to_string(), serde_json::json!("active"));
    ///
    /// let contract = ODCSContract::from_table(&table);
    /// assert_eq!(contract.name, "users");
    /// assert_eq!(contract.version, "1.0.0");
    /// ```
    pub fn from_table(table: &Table) -> Self {
        // Start with from_tables which handles most metadata extraction
        let mut contract = Self::from_tables(&[table.clone()]);

        // Use table.id as contract id (matches existing export behavior)
        contract.id = table.id.to_string();

        // Use table.name as contract name if not set from metadata
        if contract.name.is_empty() {
            contract.name = table.name.clone();
        }

        // Set default version if not present
        if contract.version.is_empty() {
            contract.version = "1.0.0".to_string();
        }

        // Set default status if not present (required in ODCS v3.1.0)
        if contract.status.is_none() {
            contract.status = Some("draft".to_string());
        }

        // Extract additional fields that from_tables might not handle
        // Infrastructure
        if let Some(infrastructure) = table.odcl_metadata.get("infrastructure") {
            // Store as custom property since ODCSContract doesn't have infrastructure field
            if contract.custom_properties.iter().all(|cp| cp.property != "infrastructure") {
                contract.custom_properties.push(CustomProperty::new(
                    "infrastructure".to_string(),
                    infrastructure.clone(),
                ));
            }
        }

        // Servicelevels (note: ODCS uses "servicelevels" key in YAML but "service_levels" in struct)
        if let Some(servicelevels) = table.odcl_metadata.get("servicelevels") {
            if contract.service_levels.is_empty() {
                contract.service_levels =
                    serde_json::from_value(servicelevels.clone()).unwrap_or_default();
            }
        }

        // Pricing -> price (ODCS uses "price" field)
        if let Some(pricing) = table.odcl_metadata.get("pricing") {
            if contract.price.is_none() {
                contract.price = serde_json::from_value(pricing.clone()).ok();
            }
        }

        // Use table tags if contract tags are empty
        if contract.tags.is_empty() && !table.tags.is_empty() {
            contract.tags = table.tags.iter().map(|t| t.to_string()).collect();
        }

        // Set contract created timestamp if not present
        if contract.contract_created_ts.is_none() {
            contract.contract_created_ts = Some(table.created_at.to_rfc3339());
        }

        contract
    }

    /// Convert contract to TableData for API responses
    pub fn to_table_data(&self) -> Vec<TableData> {
        self.schema
            .iter()
            .enumerate()
            .map(|(idx, schema)| {
                let description_value = self
                    .description
                    .as_ref()
                    .map(|d| serde_json::to_value(d).unwrap_or(serde_json::Value::Null));

                TableData {
                    table_index: idx,
                    // Contract identity
                    id: Some(self.id.clone()),
                    name: Some(schema.name.clone()),
                    api_version: Some(self.api_version.clone()),
                    version: Some(self.version.clone()),
                    status: self.status.clone(),
                    kind: Some(self.kind.clone()),
                    // Domain & organization
                    domain: self.domain.clone(),
                    data_product: self.data_product.clone(),
                    tenant: self.tenant.clone(),
                    // Description
                    description: description_value,
                    // Schema-level fields
                    physical_name: schema.physical_name.clone(),
                    physical_type: schema.physical_type.clone(),
                    business_name: schema.business_name.clone(),
                    data_granularity_description: schema.data_granularity_description.clone(),
                    // Columns
                    columns: schema
                        .properties
                        .iter()
                        .map(property_to_column_data)
                        .collect(),
                    // Server configuration
                    servers: self
                        .servers
                        .iter()
                        .filter_map(|s| serde_json::to_value(s).ok())
                        .collect(),
                    // Team & support
                    team: self
                        .team
                        .as_ref()
                        .and_then(|t| serde_json::to_value(t).ok()),
                    support: self
                        .support
                        .as_ref()
                        .and_then(|s| serde_json::to_value(s).ok()),
                    // Roles
                    roles: self
                        .roles
                        .iter()
                        .filter_map(|r| serde_json::to_value(r).ok())
                        .collect(),
                    // SLA & quality
                    sla_properties: self
                        .service_levels
                        .iter()
                        .filter_map(|s| serde_json::to_value(s).ok())
                        .collect(),
                    quality: self
                        .quality
                        .iter()
                        .filter_map(|q| serde_json::to_value(q).ok())
                        .filter_map(|v| v.as_object().cloned())
                        .map(|m| m.into_iter().collect())
                        .collect(),
                    // Pricing
                    price: self
                        .price
                        .as_ref()
                        .and_then(|p| serde_json::to_value(p).ok()),
                    // Tags & custom properties
                    tags: self.tags.clone(),
                    custom_properties: self
                        .custom_properties
                        .iter()
                        .filter_map(|cp| serde_json::to_value(cp).ok())
                        .collect(),
                    authoritative_definitions: self
                        .authoritative_definitions
                        .iter()
                        .filter_map(|ad| serde_json::to_value(ad).ok())
                        .collect(),
                    // Timestamps
                    contract_created_ts: self.contract_created_ts.clone(),
                    // Metadata
                    odcs_metadata: std::collections::HashMap::new(),
                }
            })
            .collect()
    }
}

/// Helper function to convert Property to ColumnData
fn property_to_column_data(prop: &Property) -> ColumnData {
    ColumnData {
        id: prop.id.clone(),
        name: prop.name.clone(),
        business_name: prop.business_name.clone(),
        description: prop.description.clone(),
        data_type: prop.logical_type.clone(),
        physical_type: prop.physical_type.clone(),
        physical_name: prop.physical_name.clone(),
        logical_type_options: prop.logical_type_options.as_ref().map(|opts| {
            ColumnLogicalTypeOptions {
                min_length: opts.min_length,
                max_length: opts.max_length,
                pattern: opts.pattern.clone(),
                format: opts.format.clone(),
                minimum: opts.minimum.clone(),
                maximum: opts.maximum.clone(),
                exclusive_minimum: opts.exclusive_minimum.clone(),
                exclusive_maximum: opts.exclusive_maximum.clone(),
                precision: opts.precision,
                scale: opts.scale,
            }
        }),
        primary_key: prop.primary_key,
        primary_key_position: prop.primary_key_position,
        unique: prop.unique,
        nullable: !prop.required,
        partitioned: prop.partitioned,
        partition_key_position: prop.partition_key_position,
        clustered: prop.clustered,
        classification: prop.classification.clone(),
        critical_data_element: prop.critical_data_element,
        encrypted_name: prop.encrypted_name.clone(),
        transform_source_objects: prop.transform_source_objects.clone(),
        transform_logic: prop.transform_logic.clone(),
        transform_description: prop.transform_description.clone(),
        examples: prop.examples.clone(),
        default_value: prop.default_value.clone(),
        relationships: prop
            .relationships
            .iter()
            .map(|r| ColumnPropertyRelationship {
                relationship_type: r.relationship_type.clone(),
                to: r.to.clone(),
            })
            .collect(),
        authoritative_definitions: prop
            .authoritative_definitions
            .iter()
            .map(|d| ColumnAuthDef {
                definition_type: d.definition_type.clone(),
                url: d.url.clone(),
            })
            .collect(),
        quality: if prop.quality.is_empty() {
            None
        } else {
            Some(
                prop.quality
                    .iter()
                    .filter_map(|q| serde_json::to_value(q).ok())
                    .filter_map(|v| v.as_object().cloned())
                    .map(|m| m.into_iter().collect())
                    .collect(),
            )
        },
        enum_values: if prop.enum_values.is_empty() {
            None
        } else {
            Some(prop.enum_values.clone())
        },
        tags: prop.tags.clone(),
        custom_properties: prop
            .custom_properties
            .iter()
            .map(|cp| (cp.property.clone(), cp.value.clone()))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_to_column_roundtrip() {
        let prop = Property::new("email", "string")
            .with_required(true)
            .with_description("User email address")
            .with_classification("pii");

        let col = Column::from(&prop);
        assert_eq!(col.name, "email");
        assert_eq!(col.data_type, "string");
        assert!(!col.nullable); // required -> not nullable
        assert_eq!(col.description, "User email address");
        assert_eq!(col.classification, Some("pii".to_string()));

        let prop2 = Property::from(&col);
        assert_eq!(prop2.name, "email");
        assert_eq!(prop2.logical_type, "string");
        assert!(prop2.required);
        assert_eq!(prop2.description, Some("User email address".to_string()));
    }

    #[test]
    fn test_data_type_mapping() {
        // Test integer types
        assert_eq!(
            map_data_type_to_logical_type("BIGINT"),
            ("integer".to_string(), false)
        );
        assert_eq!(
            map_data_type_to_logical_type("INT"),
            ("integer".to_string(), false)
        );

        // Test number types
        assert_eq!(
            map_data_type_to_logical_type("DECIMAL(10,2)"),
            ("number".to_string(), false)
        );
        assert_eq!(
            map_data_type_to_logical_type("DOUBLE"),
            ("number".to_string(), false)
        );

        // Test boolean
        assert_eq!(
            map_data_type_to_logical_type("BOOLEAN"),
            ("boolean".to_string(), false)
        );

        // Test date/time types
        assert_eq!(
            map_data_type_to_logical_type("DATE"),
            ("date".to_string(), false)
        );
        assert_eq!(
            map_data_type_to_logical_type("TIMESTAMP"),
            ("timestamp".to_string(), false)
        );

        // Test array types
        assert_eq!(
            map_data_type_to_logical_type("ARRAY<STRING>"),
            ("array".to_string(), true)
        );

        // Test object types
        assert_eq!(
            map_data_type_to_logical_type("STRUCT<name STRING, age INT>"),
            ("object".to_string(), false)
        );

        // Test string default
        assert_eq!(
            map_data_type_to_logical_type("VARCHAR(255)"),
            ("string".to_string(), false)
        );
    }

    #[test]
    fn test_column_to_property_with_data_type_mapping() {
        let mut col = Column::new("age".to_string(), "BIGINT".to_string());
        col.nullable = false;

        let prop = Property::from(&col);
        assert_eq!(prop.name, "age");
        assert_eq!(prop.logical_type, "integer"); // Mapped from BIGINT
        assert_eq!(prop.physical_type, Some("BIGINT".to_string())); // Original type preserved
        assert!(prop.required);
    }

    #[test]
    fn test_enum_values_to_quality_rule() {
        let mut col = Column::new("status".to_string(), "VARCHAR(20)".to_string());
        col.enum_values = vec![
            "active".to_string(),
            "inactive".to_string(),
            "pending".to_string(),
        ];

        let prop = Property::from(&col);
        assert_eq!(prop.logical_type, "string");

        // Should have a quality rule for enum validation
        assert!(!prop.quality.is_empty());
        let enum_rule = prop.quality.iter().find(|q| q.rule_type == Some("sql".to_string()));
        assert!(enum_rule.is_some());
        let rule = enum_rule.unwrap();
        assert!(rule.query.as_ref().unwrap().contains("NOT IN"));
        assert!(rule.query.as_ref().unwrap().contains("'active'"));
    }

    #[test]
    fn test_schema_to_table_roundtrip() {
        let schema = SchemaObject::new("users")
            .with_physical_name("tbl_users")
            .with_physical_type("table")
            .with_business_name("User Accounts")
            .with_description("User data")
            .with_properties(vec![
                Property::new("id", "integer").with_primary_key(true),
                Property::new("email", "string").with_required(true),
            ]);

        let table = Table::from(&schema);
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(
            table
                .odcl_metadata
                .get("physicalName")
                .and_then(|v| v.as_str()),
            Some("tbl_users")
        );

        let schema2 = SchemaObject::from(&table);
        assert_eq!(schema2.name, "users");
        assert_eq!(schema2.physical_name, Some("tbl_users".to_string()));
        assert_eq!(schema2.physical_type, Some("table".to_string()));
        assert_eq!(schema2.properties.len(), 2);
    }

    #[test]
    fn test_contract_to_tables_roundtrip() {
        let contract = ODCSContract::new("test-contract", "1.0.0")
            .with_domain("test")
            .with_status("active")
            .with_schema(
                SchemaObject::new("orders")
                    .with_physical_type("table")
                    .with_properties(vec![
                        Property::new("id", "integer").with_primary_key(true),
                        Property::new("total", "number"),
                    ]),
            )
            .with_schema(
                SchemaObject::new("items")
                    .with_physical_type("table")
                    .with_properties(vec![Property::new("id", "integer").with_primary_key(true)]),
            );

        let tables = contract.to_tables();
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].name, "orders");
        assert_eq!(tables[1].name, "items");

        // Check contract metadata is in tables
        assert_eq!(
            tables[0]
                .odcl_metadata
                .get("domain")
                .and_then(|v| v.as_str()),
            Some("test")
        );

        let contract2 = ODCSContract::from_tables(&tables);
        assert_eq!(contract2.name, "test-contract");
        assert_eq!(contract2.version, "1.0.0");
        assert_eq!(contract2.domain, Some("test".to_string()));
        assert_eq!(contract2.schema_count(), 2);
    }

    #[test]
    fn test_nested_property_flattening() {
        let schema = SchemaObject::new("events").with_properties(vec![
            Property::new("id", "string"),
            Property::new("address", "object").with_nested_properties(vec![
                Property::new("street", "string"),
                Property::new("city", "string"),
            ]),
        ]);

        let table = Table::from(&schema);

        // Should have flattened columns: id, address, address.street, address.city
        let column_names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"id"));
        assert!(column_names.contains(&"address"));
        assert!(column_names.contains(&"address.street"));
        assert!(column_names.contains(&"address.city"));
    }

    #[test]
    fn test_to_table_data() {
        let contract = ODCSContract::new("test", "1.0.0")
            .with_domain("test-domain")
            .with_schema(
                SchemaObject::new("users")
                    .with_description("User data")
                    .with_properties(vec![
                        Property::new("id", "integer").with_primary_key(true),
                        Property::new("name", "string"),
                    ]),
            );

        let table_data = contract.to_table_data();
        assert_eq!(table_data.len(), 1);
        assert_eq!(table_data[0].name, Some("users".to_string()));
        // Schema-level description is stored in the schema, not propagated to TableData description
        // TableData.description is contract-level description
        assert_eq!(table_data[0].domain, Some("test-domain".to_string()));
        assert_eq!(table_data[0].columns.len(), 2);
    }

    #[test]
    fn test_array_struct_type_handling() {
        // Create a table with ARRAY<STRUCT<...>> column and nested children
        let mut table = Table::new(
            "orders".to_string(),
            vec![
                Column::new("id".to_string(), "BIGINT".to_string()),
                Column::new("items".to_string(), "ARRAY<STRUCT<name STRING, qty INT>>".to_string()),
                Column::new("items.[].name".to_string(), "STRING".to_string()),
                Column::new("items.[].qty".to_string(), "INT".to_string()),
            ],
        );
        table.columns[0].nullable = false;

        let schema = SchemaObject::from(&table);
        assert_eq!(schema.properties.len(), 2); // id and items

        // Check the id property
        let id_prop = schema.get_property("id").unwrap();
        assert_eq!(id_prop.logical_type, "integer");
        assert!(id_prop.required);

        // Check the items property is correctly typed as array
        let items_prop = schema.get_property("items").unwrap();
        assert_eq!(items_prop.logical_type, "array");
        assert!(items_prop.items.is_some());

        // Check the items contain nested properties
        let items_inner = items_prop.items.as_ref().unwrap();
        assert_eq!(items_inner.logical_type, "object");
        assert_eq!(items_inner.properties.len(), 2);
    }

    #[test]
    fn test_struct_type_handling() {
        // Create a table with STRUCT column and nested children
        let table = Table::new(
            "users".to_string(),
            vec![
                Column::new("id".to_string(), "BIGINT".to_string()),
                Column::new("address".to_string(), "STRUCT<street STRING, city STRING>".to_string()),
                Column::new("address.street".to_string(), "STRING".to_string()),
                Column::new("address.city".to_string(), "STRING".to_string()),
            ],
        );

        let schema = SchemaObject::from(&table);
        assert_eq!(schema.properties.len(), 2); // id and address

        // Check the address property is correctly typed as object
        let address_prop = schema.get_property("address").unwrap();
        assert_eq!(address_prop.logical_type, "object");
        assert_eq!(address_prop.properties.len(), 2);

        // Check nested properties
        let street = address_prop.properties.iter().find(|p| p.name == "street");
        assert!(street.is_some());
        assert_eq!(street.unwrap().logical_type, "string");
    }

    #[test]
    fn test_from_table() {
        use crate::models::tag::Tag;

        let mut table = Table::new(
            "orders".to_string(),
            vec![
                Column::new("id".to_string(), "BIGINT".to_string()),
                Column::new("total".to_string(), "DECIMAL(10,2)".to_string()),
            ],
        );

        // Set metadata
        table
            .odcl_metadata
            .insert("version".to_string(), serde_json::json!("2.0.0"));
        table
            .odcl_metadata
            .insert("status".to_string(), serde_json::json!("active"));
        table
            .odcl_metadata
            .insert("domain".to_string(), serde_json::json!("sales"));
        table.tags = vec![Tag::Simple("important".to_string())];

        let contract = ODCSContract::from_table(&table);

        // Check contract fields
        assert_eq!(contract.id, table.id.to_string());
        assert_eq!(contract.name, "orders");
        assert_eq!(contract.version, "2.0.0");
        assert_eq!(contract.status, Some("active".to_string()));
        assert_eq!(contract.domain, Some("sales".to_string()));
        assert_eq!(contract.tags, vec!["important".to_string()]);

        // Check schema
        assert_eq!(contract.schema.len(), 1);
        let schema = &contract.schema[0];
        assert_eq!(schema.name, "orders");
        assert_eq!(schema.properties.len(), 2);

        // Check properties have correct logical types
        let id_prop = schema.get_property("id").unwrap();
        assert_eq!(id_prop.logical_type, "integer");

        let total_prop = schema.get_property("total").unwrap();
        assert_eq!(total_prop.logical_type, "number");
    }

    #[test]
    fn test_from_table_defaults() {
        // Test that from_table sets sensible defaults
        let table = Table::new(
            "simple".to_string(),
            vec![Column::new("id".to_string(), "INT".to_string())],
        );

        let contract = ODCSContract::from_table(&table);

        assert_eq!(contract.name, "simple");
        assert_eq!(contract.version, "1.0.0"); // Default version
        assert_eq!(contract.status, Some("draft".to_string())); // Default status
        assert!(contract.contract_created_ts.is_some()); // Should have timestamp
    }
}
