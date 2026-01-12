//! ODCS parser service for parsing Open Data Contract Standard YAML files.
//!
//! This service parses ODCS (Open Data Contract Standard) v3.1.0 and legacy ODCL (Data Contract Specification) YAML files
//! and converts them to Table models. ODCL files are automatically converted to ODCS v3.1.0 format.
//! Supports multiple formats:
//! - ODCS v3.1.0 / v3.0.x format (apiVersion, kind, schema) - PRIMARY FORMAT
//! - ODCL (Data Contract Specification) format (dataContractSpecification, models, definitions) - LEGACY, converted to ODCS
//! - Simple ODCL format (name, columns) - LEGACY, converted to ODCS
//! - Liquibase format

use super::odcs_shared::{
    ParserError, column_to_column_data, expand_nested_column, json_value_to_serde_value,
    normalize_data_type, parse_data_vault_classification, parse_medallion_layer, parse_scd_pattern,
    resolve_ref, yaml_to_json_value,
};
use super::{ImportError, ImportResult, TableData};
use crate::models::column::ForeignKey;
use crate::models::enums::{DataVaultClassification, DatabaseType, MedallionLayer, SCDPattern};
use crate::models::{Column, PropertyRelationship, Table, Tag};
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::info;

// Re-export ParserError for backward compatibility
pub use super::odcs_shared::ParserError as OdcsParserError;

/// Convert a $ref path to a PropertyRelationship.
/// E.g., "#/definitions/order_id" -> PropertyRelationship { type: "foreignKey", to: "definitions/order_id" }
fn ref_to_relationships(ref_path: &Option<String>) -> Vec<PropertyRelationship> {
    match ref_path {
        Some(ref_str) => {
            let to = if ref_str.starts_with("#/definitions/") {
                let def_path = ref_str.strip_prefix("#/definitions/").unwrap_or(ref_str);
                format!("definitions/{}", def_path)
            } else if ref_str.starts_with("#/") {
                ref_str.strip_prefix("#/").unwrap_or(ref_str).to_string()
            } else {
                ref_str.clone()
            };
            vec![PropertyRelationship {
                relationship_type: "foreignKey".to_string(),
                to,
            }]
        }
        None => Vec::new(),
    }
}

/// ODCS parser service for parsing Open Data Contract Standard YAML files.
/// Handles ODCS v3.1.0 (primary format) and legacy ODCL formats (converted to ODCS).
pub struct ODCSImporter {
    /// Current YAML data for $ref resolution
    current_yaml_data: Option<serde_yaml::Value>,
}

impl ODCSImporter {
    /// Create a new ODCS parser instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::import::odcs::ODCSImporter;
    ///
    /// let mut importer = ODCSImporter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            current_yaml_data: None,
        }
    }

    /// Import ODCS/ODCL YAML content and create Table (SDK interface).
    ///
    /// Supports ODCS v3.1.0 (primary), legacy ODCL formats (converted to ODCS), and Liquibase formats.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - ODCS/ODCL YAML content as a string
    ///
    /// # Returns
    ///
    /// An `ImportResult` containing the extracted table and any parse errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_core::import::odcs::ODCSImporter;
    ///
    /// let mut importer = ODCSImporter::new();
    /// let yaml = r#"
    /// apiVersion: v3.1.0
    /// kind: DataContract
    /// id: 550e8400-e29b-41d4-a716-446655440000
    /// version: 1.0.0
    /// name: users
    /// schema:
    ///   fields:
    ///     - name: id
    ///       type: bigint
    /// "#;
    /// let result = importer.import(yaml).unwrap();
    /// assert_eq!(result.tables.len(), 1);
    /// ```
    pub fn import(&mut self, yaml_content: &str) -> Result<ImportResult, ImportError> {
        // First parse YAML to get raw data for ODCS field extraction
        let yaml_data: serde_yaml::Value = serde_yaml::from_str(yaml_content)
            .map_err(|e| ImportError::ParseError(format!("Failed to parse YAML: {}", e)))?;

        let json_data = yaml_to_json_value(&yaml_data).map_err(|e| {
            ImportError::ParseError(format!("Failed to convert YAML to JSON: {}", e))
        })?;

        match self.parse(yaml_content) {
            Ok((table, errors)) => {
                // Extract all ODCS contract-level fields from the raw JSON data
                let sdk_tables = vec![TableData {
                    table_index: 0,
                    id: Some(table.id.to_string()),
                    name: Some(table.name.clone()),
                    api_version: json_data
                        .get("apiVersion")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    version: json_data
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    status: json_data
                        .get("status")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    kind: json_data
                        .get("kind")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    domain: json_data
                        .get("domain")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    data_product: json_data
                        .get("dataProduct")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    tenant: json_data
                        .get("tenant")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    description: json_data.get("description").cloned(),
                    columns: table.columns.iter().map(column_to_column_data).collect(),
                    servers: json_data
                        .get("servers")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    team: json_data.get("team").cloned(),
                    support: json_data.get("support").cloned(),
                    roles: json_data
                        .get("roles")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    sla_properties: json_data
                        .get("slaProperties")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    quality: table.quality.clone(),
                    price: json_data.get("price").cloned(),
                    tags: table.tags.iter().map(|t| t.to_string()).collect(),
                    custom_properties: json_data
                        .get("customProperties")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    authoritative_definitions: json_data
                        .get("authoritativeDefinitions")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default(),
                    contract_created_ts: json_data
                        .get("contractCreatedTs")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    odcs_metadata: table.odcl_metadata.clone(),
                }];
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

    /// Parse ODCS/ODCL YAML content and create Table (public method for native app use).
    ///
    /// This method returns the full Table object with all metadata, suitable for use in
    /// native applications that need direct access to the parsed table structure.
    /// For API use, prefer the `import()` method which returns ImportResult.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (Table, list of errors/warnings).
    /// Errors list is empty if parsing is successful.
    pub fn parse_table(&mut self, yaml_content: &str) -> Result<(Table, Vec<ParserError>)> {
        self.parse(yaml_content)
    }

    /// Parse ODCS/ODCL YAML content and create Table (internal method).
    ///
    /// Supports ODCS v3.1.0/v3.0.x (primary), ODCL Data Contract spec (legacy, converted to ODCS),
    /// simple ODCL (legacy, converted to ODCS), and Liquibase formats.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (Table, list of errors/warnings).
    /// Errors list is empty if parsing is successful.
    fn parse(&mut self, yaml_content: &str) -> Result<(Table, Vec<ParserError>)> {
        // Errors are collected in helper functions, not here
        let _errors: Vec<ParserError> = Vec::new();

        // Parse YAML
        let data: serde_yaml::Value =
            serde_yaml::from_str(yaml_content).context("Failed to parse YAML")?;

        if data.is_null() {
            return Err(anyhow::anyhow!("Empty YAML content"));
        }

        // Store current YAML data for $ref resolution
        self.current_yaml_data = Some(data.clone());

        // Convert to JSON Value for easier manipulation
        let json_data = yaml_to_json_value(&data)?;

        // Check format and parse accordingly
        if self.is_liquibase_format(&json_data) {
            return self.parse_liquibase(&json_data);
        }

        if self.is_odcl_v3_format(&json_data) {
            return self.parse_odcl_v3(&json_data);
        }

        if self.is_data_contract_format(&json_data) {
            return self.parse_data_contract(&json_data);
        }

        // Fall back to simple ODCL format
        self.parse_simple_odcl(&json_data)
    }

    /// Check if YAML is in Liquibase format.
    fn is_liquibase_format(&self, data: &JsonValue) -> bool {
        if data.get("databaseChangeLog").is_some() {
            return true;
        }
        // Check for Liquibase-specific keys
        if let Some(obj) = data.as_object() {
            let obj_str = format!("{:?}", obj);
            if obj_str.contains("changeSet") {
                return true;
            }
        }
        false
    }

    /// Check if YAML is in ODCS v3.0.x format.
    fn is_odcl_v3_format(&self, data: &JsonValue) -> bool {
        if let Some(obj) = data.as_object() {
            let has_api_version = obj.contains_key("apiVersion");
            let has_kind = obj
                .get("kind")
                .and_then(|v| v.as_str())
                .map(|s| s == "DataContract")
                .unwrap_or(false);
            let has_id = obj.contains_key("id");
            let has_version = obj.contains_key("version");
            return has_api_version && has_kind && has_id && has_version;
        }
        false
    }

    /// Check if YAML is in Data Contract specification format.
    fn is_data_contract_format(&self, data: &JsonValue) -> bool {
        if let Some(obj) = data.as_object() {
            let has_spec = obj.contains_key("dataContractSpecification");
            let has_models = obj.get("models").and_then(|v| v.as_object()).is_some();
            return has_spec && has_models;
        }
        false
    }

    /// Parse simple ODCL format.
    fn parse_simple_odcl(&self, data: &JsonValue) -> Result<(Table, Vec<ParserError>)> {
        let mut errors = Vec::new();

        // Extract table name
        let name = data
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("ODCL YAML missing required 'name' field"))?
            .to_string();

        // Extract columns
        let columns_data = data
            .get("columns")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("ODCL YAML missing required 'columns' field"))?;

        let mut columns = Vec::new();
        for (idx, col_data) in columns_data.iter().enumerate() {
            match self.parse_column(col_data) {
                Ok(col) => columns.push(col),
                Err(e) => {
                    errors.push(ParserError {
                        error_type: "column_parse_error".to_string(),
                        field: format!("columns[{}]", idx),
                        message: e.to_string(),
                    });
                }
            }
        }

        // Extract metadata
        let database_type = self.extract_database_type(data);
        let medallion_layers = self.extract_medallion_layers(data);
        let scd_pattern = self.extract_scd_pattern(data);
        let data_vault_classification = self.extract_data_vault_classification(data);
        let quality_rules = self.extract_quality_rules(data);

        // Validate pattern exclusivity
        if scd_pattern.is_some() && data_vault_classification.is_some() {
            errors.push(ParserError {
                error_type: "validation_error".to_string(),
                field: "patterns".to_string(),
                message: "SCD pattern and Data Vault classification are mutually exclusive"
                    .to_string(),
            });
        }

        // Extract odcl_metadata
        let mut odcl_metadata = HashMap::new();
        if let Some(metadata) = data.get("odcl_metadata")
            && let Some(obj) = metadata.as_object()
        {
            for (key, value) in obj {
                odcl_metadata.insert(key.clone(), json_value_to_serde_value(value));
            }
        }

        let table_uuid = self.extract_table_uuid(data);

        let table = Table {
            id: table_uuid,
            name,
            columns,
            database_type,
            catalog_name: None,
            schema_name: None,
            medallion_layers,
            scd_pattern,
            data_vault_classification,
            modeling_level: None,
            tags: Vec::<Tag>::new(),
            odcl_metadata,
            owner: None,
            sla: None,
            contact_details: None,
            infrastructure_type: None,
            notes: None,
            position: None,
            yaml_file_path: None,
            drawio_cell_id: None,
            quality: quality_rules,
            errors: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        info!("Parsed ODCL table: {}", table.name);
        Ok((table, errors))
    }

    /// Parse a single column definition.
    fn parse_column(&self, col_data: &JsonValue) -> Result<Column> {
        let name = col_data
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Column missing 'name' field"))?
            .to_string();

        let data_type = col_data
            .get("data_type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Column missing 'data_type' field"))?
            .to_string();

        // Normalize data_type to uppercase (preserve STRUCT<...> format)
        let data_type = normalize_data_type(&data_type);

        let nullable = col_data
            .get("nullable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let primary_key = col_data
            .get("primary_key")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let foreign_key = col_data
            .get("foreign_key")
            .and_then(|v| self.parse_foreign_key(v));

        let constraints = col_data
            .get("constraints")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let description = col_data
            .get("description")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();

        // Extract column-level quality rules
        let mut column_quality_rules = Vec::new();
        if let Some(quality_val) = col_data.get("quality") {
            if let Some(arr) = quality_val.as_array() {
                // Array of quality rules
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let mut rule = HashMap::new();
                        for (key, value) in obj {
                            rule.insert(key.clone(), json_value_to_serde_value(value));
                        }
                        column_quality_rules.push(rule);
                    }
                }
            } else if let Some(obj) = quality_val.as_object() {
                // Single quality rule object
                let mut rule = HashMap::new();
                for (key, value) in obj {
                    rule.insert(key.clone(), json_value_to_serde_value(value));
                }
                column_quality_rules.push(rule);
            }
        }

        // Note: ODCS v3.1.0 uses the `required` field at the property level to indicate non-nullable columns,
        // not a quality rule. We should NOT add a "not_null" quality rule as it's not a valid ODCS quality type.
        // The `nullable` field will be converted to `required` during export.

        Ok(Column {
            name,
            data_type,
            nullable,
            primary_key,
            foreign_key,
            constraints,
            description,
            quality: column_quality_rules,
            ..Default::default()
        })
    }

    /// Parse foreign key from JSON value.
    fn parse_foreign_key(&self, fk_data: &JsonValue) -> Option<ForeignKey> {
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

    /// Extract database type from data.
    fn extract_database_type(&self, data: &JsonValue) -> Option<DatabaseType> {
        data.get("database_type")
            .and_then(|v| v.as_str())
            .and_then(|s| match s.to_uppercase().as_str() {
                "POSTGRES" | "POSTGRESQL" => Some(DatabaseType::Postgres),
                "MYSQL" => Some(DatabaseType::Mysql),
                "SQL_SERVER" | "SQLSERVER" => Some(DatabaseType::SqlServer),
                "DATABRICKS" | "DATABRICKS_DELTA" => Some(DatabaseType::DatabricksDelta),
                "AWS_GLUE" | "GLUE" => Some(DatabaseType::AwsGlue),
                _ => None,
            })
    }

    /// Extract medallion layers from data.
    fn extract_medallion_layers(&self, data: &JsonValue) -> Vec<MedallionLayer> {
        let mut layers = Vec::new();

        // Check plural form first
        if let Some(arr) = data.get("medallion_layers").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(s) = item.as_str()
                    && let Ok(layer) = parse_medallion_layer(s)
                {
                    layers.push(layer);
                }
            }
        }
        // Check singular form (backward compatibility)
        else if let Some(s) = data.get("medallion_layer").and_then(|v| v.as_str())
            && let Ok(layer) = parse_medallion_layer(s)
        {
            layers.push(layer);
        }

        layers
    }

    /// Extract SCD pattern from data.
    fn extract_scd_pattern(&self, data: &JsonValue) -> Option<SCDPattern> {
        data.get("scd_pattern")
            .and_then(|v| v.as_str())
            .and_then(|s| parse_scd_pattern(s).ok())
    }

    /// Extract Data Vault classification from data.
    fn extract_data_vault_classification(
        &self,
        data: &JsonValue,
    ) -> Option<DataVaultClassification> {
        data.get("data_vault_classification")
            .and_then(|v| v.as_str())
            .and_then(|s| parse_data_vault_classification(s).ok())
    }

    /// Extract quality rules from data.
    fn extract_quality_rules(&self, data: &JsonValue) -> Vec<HashMap<String, serde_json::Value>> {
        use serde_json::Value;
        let mut quality_rules = Vec::new();

        // Check for quality field at root level (array of objects or single object)
        if let Some(quality_val) = data.get("quality") {
            if let Some(arr) = quality_val.as_array() {
                // Array of quality rules
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let mut rule = HashMap::new();
                        for (key, value) in obj {
                            rule.insert(key.clone(), json_value_to_serde_value(value));
                        }
                        quality_rules.push(rule);
                    }
                }
            } else if let Some(obj) = quality_val.as_object() {
                // Single quality rule object
                let mut rule = HashMap::new();
                for (key, value) in obj {
                    rule.insert(key.clone(), json_value_to_serde_value(value));
                }
                quality_rules.push(rule);
            } else if let Some(s) = quality_val.as_str() {
                // Simple string quality value
                let mut rule = HashMap::new();
                rule.insert("value".to_string(), Value::String(s.to_string()));
                quality_rules.push(rule);
            }
        }

        // Check for quality in metadata (ODCL v3 format)
        if let Some(metadata) = data.get("metadata")
            && let Some(metadata_obj) = metadata.as_object()
            && let Some(quality_val) = metadata_obj.get("quality")
        {
            if let Some(arr) = quality_val.as_array() {
                // Array of quality rules
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let mut rule = HashMap::new();
                        for (key, value) in obj {
                            rule.insert(key.clone(), json_value_to_serde_value(value));
                        }
                        quality_rules.push(rule);
                    }
                }
            } else if let Some(obj) = quality_val.as_object() {
                // Single quality rule object
                let mut rule = HashMap::new();
                for (key, value) in obj {
                    rule.insert(key.clone(), json_value_to_serde_value(value));
                }
                quality_rules.push(rule);
            } else if let Some(s) = quality_val.as_str() {
                // Simple string quality value
                let mut rule = HashMap::new();
                rule.insert("value".to_string(), Value::String(s.to_string()));
                quality_rules.push(rule);
            }
        }

        // Check for tblproperties field (similar to SQL TBLPROPERTIES)
        if let Some(tblprops) = data.get("tblproperties")
            && let Some(obj) = tblprops.as_object()
        {
            for (key, value) in obj {
                let mut rule = HashMap::new();
                rule.insert("property".to_string(), Value::String(key.clone()));
                rule.insert("value".to_string(), json_value_to_serde_value(value));
                quality_rules.push(rule);
            }
        }

        quality_rules
    }

    /// Parse Liquibase YAML changelog format.
    ///
    /// Extracts the first `createTable` change from a Liquibase changelog.
    /// Supports both direct column definitions and nested column structures.
    ///
    /// # Supported Format
    ///
    fn parse_liquibase(&self, data: &JsonValue) -> Result<(Table, Vec<ParserError>)> {
        // Supported Liquibase YAML changelog format:
        // databaseChangeLog:
        //   - changeSet:
        //       - createTable:
        //           tableName: my_table
        //           columns:
        //             - column:
        //                 name: id
        //                 type: int
        //                 constraints:
        //                   primaryKey: true
        //                   nullable: false

        let mut errors = Vec::new();

        let changelog = data
            .get("databaseChangeLog")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow::anyhow!("Liquibase YAML missing databaseChangeLog array"))?;

        // Find first createTable change.
        let mut table_name: Option<String> = None;
        let mut columns: Vec<crate::models::column::Column> = Vec::new();

        for entry in changelog {
            // Entries are typically maps like { changeSet: [...] } or { changeSet: { changes: [...] } }
            if let Some(change_set) = entry.get("changeSet") {
                // Liquibase YAML can represent changeSet as map or list.
                let changes = if let Some(obj) = change_set.as_object() {
                    obj.get("changes")
                        .and_then(|v| v.as_array())
                        .cloned()
                        .unwrap_or_default()
                } else if let Some(arr) = change_set.as_array() {
                    // Some variants encode changes directly as list entries inside changeSet.
                    arr.clone()
                } else {
                    Vec::new()
                };

                for ch in changes {
                    let create = ch.get("createTable").or_else(|| ch.get("create_table"));
                    if let Some(create) = create {
                        table_name = create
                            .get("tableName")
                            .or_else(|| create.get("table_name"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        // columns: [ { column: { ... } }, ... ]
                        if let Some(cols) = create.get("columns").and_then(|v| v.as_array()) {
                            for col_entry in cols {
                                let col = col_entry.get("column").unwrap_or(col_entry);
                                let name = col
                                    .get("name")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let data_type = col
                                    .get("type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();

                                if name.is_empty() {
                                    errors.push(ParserError {
                                        error_type: "validation_error".to_string(),
                                        field: "columns.name".to_string(),
                                        message: "Liquibase createTable column missing name"
                                            .to_string(),
                                    });
                                    continue;
                                }

                                let mut column =
                                    crate::models::column::Column::new(name, data_type);

                                if let Some(constraints) =
                                    col.get("constraints").and_then(|v| v.as_object())
                                {
                                    if let Some(pk) =
                                        constraints.get("primaryKey").and_then(|v| v.as_bool())
                                    {
                                        column.primary_key = pk;
                                    }
                                    if let Some(nullable) =
                                        constraints.get("nullable").and_then(|v| v.as_bool())
                                    {
                                        column.nullable = nullable;
                                    }
                                }

                                columns.push(column);
                            }
                        }

                        // parse_table() returns a single Table, so we parse the first createTable.
                        // If multiple tables are needed, call parse_table() multiple times or use import().
                        break;
                    }
                }
            }
            if table_name.is_some() {
                break;
            }
        }

        let table_name = table_name
            .ok_or_else(|| anyhow::anyhow!("Liquibase changelog did not contain a createTable"))?;
        let table = Table::new(table_name, columns);
        // Preserve any errors collected.
        Ok((table, errors))
    }

    /// Parse ODCS v3.0.x format.
    fn parse_odcl_v3(&self, data: &JsonValue) -> Result<(Table, Vec<ParserError>)> {
        let mut errors = Vec::new();

        // Extract table name
        let table_name = data
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // Try to get from schema array
                data.get("schema")
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|obj| obj.as_object())
                    .and_then(|obj| obj.get("name"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .ok_or_else(|| {
                anyhow::anyhow!("ODCS v3.0.x YAML missing 'name' field and no schema objects")
            })?;

        // Extract schema - ODCS v3.0.x uses array of SchemaObject
        let schema = data
            .get("schema")
            .and_then(|v| v.as_array())
            .ok_or_else(|| {
                errors.push(ParserError {
                    error_type: "validation_error".to_string(),
                    field: "schema".to_string(),
                    message: "ODCS v3.0.x YAML missing 'schema' field".to_string(),
                });
                anyhow::anyhow!("Missing schema")
            });

        let schema = match schema {
            Ok(s) if s.is_empty() => {
                errors.push(ParserError {
                    error_type: "validation_error".to_string(),
                    field: "schema".to_string(),
                    message: "ODCS v3.0.x schema array is empty".to_string(),
                });
                let quality_rules = self.extract_quality_rules(data);
                let table_uuid = self.extract_table_uuid(data);
                let table = Table {
                    id: table_uuid,
                    name: table_name,
                    columns: Vec::new(),
                    database_type: None,
                    catalog_name: None,
                    schema_name: None,
                    medallion_layers: Vec::new(),
                    scd_pattern: None,
                    data_vault_classification: None,
                    modeling_level: None,
                    tags: Vec::<Tag>::new(),
                    odcl_metadata: HashMap::new(),
                    owner: None,
                    sla: None,
                    contact_details: None,
                    infrastructure_type: None,
                    notes: None,
                    position: None,
                    yaml_file_path: None,
                    drawio_cell_id: None,
                    quality: quality_rules,
                    errors: Vec::new(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                return Ok((table, errors));
            }
            Ok(s) => s,
            Err(_) => {
                let quality_rules = self.extract_quality_rules(data);
                let table_uuid = self.extract_table_uuid(data);
                let table = Table {
                    id: table_uuid,
                    name: table_name,
                    columns: Vec::new(),
                    database_type: None,
                    catalog_name: None,
                    schema_name: None,
                    medallion_layers: Vec::new(),
                    scd_pattern: None,
                    data_vault_classification: None,
                    modeling_level: None,
                    tags: Vec::<Tag>::new(),
                    odcl_metadata: HashMap::new(),
                    owner: None,
                    sla: None,
                    contact_details: None,
                    infrastructure_type: None,
                    notes: None,
                    position: None,
                    yaml_file_path: None,
                    drawio_cell_id: None,
                    quality: quality_rules,
                    errors: Vec::new(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                return Ok((table, errors));
            }
        };

        // Get the first schema object (table)
        let schema_object = schema.first().and_then(|v| v.as_object()).ok_or_else(|| {
            errors.push(ParserError {
                error_type: "validation_error".to_string(),
                field: "schema[0]".to_string(),
                message: "First schema object must be a dictionary".to_string(),
            });
            anyhow::anyhow!("Invalid schema object")
        })?;

        let object_name = schema_object
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&table_name);

        // Handle both object format (v3.0.x) and array format (v3.1.0) for properties
        let mut columns = Vec::new();

        if let Some(properties_obj) = schema_object.get("properties").and_then(|v| v.as_object()) {
            // Object format (v3.0.x): properties is a map of name -> property object
            for (prop_name, prop_data) in properties_obj {
                if let Some(prop_obj) = prop_data.as_object() {
                    match self.parse_odcl_v3_property(prop_name, prop_obj, data) {
                        Ok(mut cols) => columns.append(&mut cols),
                        Err(e) => {
                            errors.push(ParserError {
                                error_type: "property_parse_error".to_string(),
                                field: format!("Property '{}'", prop_name),
                                message: e.to_string(),
                            });
                        }
                    }
                } else {
                    errors.push(ParserError {
                        error_type: "validation_error".to_string(),
                        field: format!("Property '{}'", prop_name),
                        message: format!("Property '{}' must be an object", prop_name),
                    });
                }
            }
        } else if let Some(properties_arr) =
            schema_object.get("properties").and_then(|v| v.as_array())
        {
            // Array format (v3.1.0): properties is an array of property objects with 'name' field
            for (idx, prop_data) in properties_arr.iter().enumerate() {
                if let Some(prop_obj) = prop_data.as_object() {
                    // Extract name from property object (required in v3.1.0)
                    // ODCS v3.1.0 requires 'name' field, but we'll also accept 'id' as fallback
                    let prop_name = match prop_obj.get("name").or_else(|| prop_obj.get("id")) {
                        Some(JsonValue::String(s)) => s.as_str(),
                        _ => {
                            // Skip properties without name or id (not valid ODCS v3.1.0)
                            errors.push(ParserError {
                                error_type: "validation_error".to_string(),
                                field: format!("Property[{}]", idx),
                                message: format!(
                                    "Property[{}] missing required 'name' or 'id' field",
                                    idx
                                ),
                            });
                            continue;
                        }
                    };

                    match self.parse_odcl_v3_property(prop_name, prop_obj, data) {
                        Ok(mut cols) => columns.append(&mut cols),
                        Err(e) => {
                            errors.push(ParserError {
                                error_type: "property_parse_error".to_string(),
                                field: format!("Property[{}] '{}'", idx, prop_name),
                                message: e.to_string(),
                            });
                        }
                    }
                } else {
                    errors.push(ParserError {
                        error_type: "validation_error".to_string(),
                        field: format!("Property[{}]", idx),
                        message: format!("Property[{}] must be an object", idx),
                    });
                }
            }
        } else {
            errors.push(ParserError {
                error_type: "validation_error".to_string(),
                field: format!("Object '{}'", object_name),
                message: format!(
                    "Object '{}' missing 'properties' field or properties is invalid",
                    object_name
                ),
            });
        }

        // Extract metadata from customProperties
        let (medallion_layers, scd_pattern, data_vault_classification, mut tags): (
            _,
            _,
            _,
            Vec<Tag>,
        ) = self.extract_metadata_from_custom_properties(data);

        // Extract sharedDomains from customProperties
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

        // Extract tags from top-level tags field (if not already extracted from customProperties)
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

        // Extract database type from servers
        let database_type = self.extract_database_type_from_odcl_v3_servers(data);

        // Extract quality rules
        let quality_rules = self.extract_quality_rules(data);

        // Build ODCL metadata
        let mut odcl_metadata = HashMap::new();
        odcl_metadata.insert(
            "apiVersion".to_string(),
            json_value_to_serde_value(data.get("apiVersion").unwrap_or(&JsonValue::Null)),
        );
        odcl_metadata.insert(
            "kind".to_string(),
            json_value_to_serde_value(data.get("kind").unwrap_or(&JsonValue::Null)),
        );
        odcl_metadata.insert(
            "id".to_string(),
            json_value_to_serde_value(data.get("id").unwrap_or(&JsonValue::Null)),
        );
        odcl_metadata.insert(
            "version".to_string(),
            json_value_to_serde_value(data.get("version").unwrap_or(&JsonValue::Null)),
        );
        odcl_metadata.insert(
            "status".to_string(),
            json_value_to_serde_value(data.get("status").unwrap_or(&JsonValue::Null)),
        );

        // Extract servicelevels if present
        if let Some(servicelevels_val) = data.get("servicelevels") {
            odcl_metadata.insert(
                "servicelevels".to_string(),
                json_value_to_serde_value(servicelevels_val),
            );
        }

        // Extract links if present
        if let Some(links_val) = data.get("links") {
            odcl_metadata.insert("links".to_string(), json_value_to_serde_value(links_val));
        }

        // Extract domain, dataProduct, tenant
        if let Some(domain_val) = data.get("domain").and_then(|v| v.as_str()) {
            odcl_metadata.insert(
                "domain".to_string(),
                json_value_to_serde_value(&JsonValue::String(domain_val.to_string())),
            );
        }
        if let Some(data_product_val) = data.get("dataProduct").and_then(|v| v.as_str()) {
            odcl_metadata.insert(
                "dataProduct".to_string(),
                json_value_to_serde_value(&JsonValue::String(data_product_val.to_string())),
            );
        }
        if let Some(tenant_val) = data.get("tenant").and_then(|v| v.as_str()) {
            odcl_metadata.insert(
                "tenant".to_string(),
                json_value_to_serde_value(&JsonValue::String(tenant_val.to_string())),
            );
        }

        // Extract top-level description (can be object or string)
        if let Some(desc_val) = data.get("description") {
            odcl_metadata.insert(
                "description".to_string(),
                json_value_to_serde_value(desc_val),
            );
        }

        // Extract pricing
        if let Some(pricing_val) = data.get("pricing") {
            odcl_metadata.insert(
                "pricing".to_string(),
                json_value_to_serde_value(pricing_val),
            );
        }

        // Extract team
        if let Some(team_val) = data.get("team") {
            odcl_metadata.insert("team".to_string(), json_value_to_serde_value(team_val));
        }

        // Extract roles
        if let Some(roles_val) = data.get("roles") {
            odcl_metadata.insert("roles".to_string(), json_value_to_serde_value(roles_val));
        }

        // Extract terms
        if let Some(terms_val) = data.get("terms") {
            odcl_metadata.insert("terms".to_string(), json_value_to_serde_value(terms_val));
        }

        // Extract full servers array (not just type)
        if let Some(servers_val) = data.get("servers") {
            odcl_metadata.insert(
                "servers".to_string(),
                json_value_to_serde_value(servers_val),
            );
        }

        // Extract infrastructure
        if let Some(infrastructure_val) = data.get("infrastructure") {
            odcl_metadata.insert(
                "infrastructure".to_string(),
                json_value_to_serde_value(infrastructure_val),
            );
        }

        // Store sharedDomains in metadata (extracted from customProperties above)
        if !shared_domains.is_empty() {
            let shared_domains_json: Vec<serde_json::Value> = shared_domains
                .iter()
                .map(|d| serde_json::Value::String(d.clone()))
                .collect();
            odcl_metadata.insert(
                "sharedDomains".to_string(),
                serde_json::Value::Array(shared_domains_json),
            );
        }

        let table_uuid = self.extract_table_uuid(data);

        let table = Table {
            id: table_uuid,
            name: table_name,
            columns,
            database_type,
            catalog_name: None,
            schema_name: None,
            medallion_layers,
            scd_pattern,
            data_vault_classification,
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
            quality: quality_rules,
            errors: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        info!(
            "Parsed ODCL v3.0.0 table: {} with {} warnings/errors",
            table.name,
            errors.len()
        );
        Ok((table, errors))
    }

    /// Parse a single property from ODCS v3.0.x format (similar to Data Contract field).
    fn parse_odcl_v3_property(
        &self,
        prop_name: &str,
        prop_data: &serde_json::Map<String, JsonValue>,
        data: &JsonValue,
    ) -> Result<Vec<Column>> {
        // Reuse Data Contract field parsing logic (they're similar)
        let mut errors = Vec::new();
        self.parse_data_contract_field(prop_name, prop_data, data, &mut errors)
    }

    /// Extract table UUID from ODCS `id` field (standard) or fallback to customProperties/odcl_metadata (backward compatibility).
    /// Returns the UUID if found, otherwise generates a new one.
    fn extract_table_uuid(&self, data: &JsonValue) -> uuid::Uuid {
        // First check the top-level `id` field (ODCS spec: "A unique identifier used to reduce the risk of dataset name collisions, such as a UUID.")
        if let Some(id_val) = data.get("id")
            && let Some(id_str) = id_val.as_str()
        {
            if let Ok(uuid) = uuid::Uuid::parse_str(id_str) {
                tracing::debug!(
                    "[ODCSImporter] Extracted UUID from top-level 'id' field: {}",
                    uuid
                );
                return uuid;
            } else {
                tracing::warn!(
                    "[ODCSImporter] Found 'id' field but failed to parse as UUID: {}",
                    id_str
                );
            }
        }

        // Backward compatibility: check customProperties for tableUuid (legacy format)
        if let Some(custom_props) = data.get("customProperties").and_then(|v| v.as_array()) {
            for prop in custom_props {
                if let Some(prop_obj) = prop.as_object() {
                    let prop_key = prop_obj
                        .get("property")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    if prop_key == "tableUuid"
                        && let Some(uuid_str) = prop_obj.get("value").and_then(|v| v.as_str())
                        && let Ok(uuid) = uuid::Uuid::parse_str(uuid_str)
                    {
                        tracing::debug!(
                            "[ODCSImporter] Extracted UUID from customProperties.tableUuid: {}",
                            uuid
                        );
                        return uuid;
                    }
                }
            }
        }

        // Fallback: check odcl_metadata if present (legacy format)
        if let Some(metadata) = data.get("odcl_metadata").and_then(|v| v.as_object())
            && let Some(uuid_val) = metadata.get("tableUuid")
            && let Some(uuid_str) = uuid_val.as_str()
            && let Ok(uuid) = uuid::Uuid::parse_str(uuid_str)
        {
            tracing::debug!(
                "[ODCSImporter] Extracted UUID from odcl_metadata.tableUuid: {}",
                uuid
            );
            return uuid;
        }

        // Generate deterministic UUID v5 if not found (based on table name)
        let table_name = data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let new_uuid = crate::models::table::Table::generate_id(
            table_name, None, // database_type not available here
            None, // catalog_name not available here
            None, // schema_name not available here
        );
        tracing::warn!(
            "[ODCSImporter] No UUID found for table '{}', generating deterministic UUID: {}. This may cause relationships to become orphaned!",
            table_name,
            new_uuid
        );
        new_uuid
    }

    /// Extract metadata from customProperties in ODCS v3.0.x format.
    fn extract_metadata_from_custom_properties(
        &self,
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

    /// Extract database type from servers in ODCS v3.0.x format.
    fn extract_database_type_from_odcl_v3_servers(&self, data: &JsonValue) -> Option<DatabaseType> {
        // ODCS v3.0.x: servers is an array of Server objects
        if let Some(servers_arr) = data.get("servers").and_then(|v| v.as_array())
            && let Some(server_obj) = servers_arr.first().and_then(|v| v.as_object())
        {
            return server_obj
                .get("type")
                .and_then(|v| v.as_str())
                .and_then(|s| self.parse_database_type(s));
        }
        None
    }

    /// Parse Data Contract format.
    fn parse_data_contract(&self, data: &JsonValue) -> Result<(Table, Vec<ParserError>)> {
        let mut errors = Vec::new();

        // Extract models
        let models = data
            .get("models")
            .and_then(|v| v.as_object())
            .ok_or_else(|| anyhow::anyhow!("Data Contract YAML missing 'models' field"))?;

        // parse_table() returns a single Table, so we parse the first model.
        // If multiple models are needed, call parse_table() multiple times or use import().
        let (model_name, model_data) = models
            .iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("Data Contract 'models' object is empty"))?;

        let model_data = model_data
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Model '{}' must be an object", model_name))?;

        // Extract fields (columns)
        let fields = model_data
            .get("fields")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                errors.push(ParserError {
                    error_type: "validation_error".to_string(),
                    field: format!("Model '{}'", model_name),
                    message: format!("Model '{}' missing 'fields' field", model_name),
                });
                anyhow::anyhow!("Missing fields")
            });

        let fields = match fields {
            Ok(f) => f,
            Err(_) => {
                // Return empty table with errors
                let quality_rules = self.extract_quality_rules(data);
                let table_uuid = self.extract_table_uuid(data);
                let table = Table {
                    id: table_uuid,
                    name: model_name.clone(),
                    columns: Vec::new(),
                    database_type: None,
                    catalog_name: None,
                    schema_name: None,
                    medallion_layers: Vec::new(),
                    scd_pattern: None,
                    data_vault_classification: None,
                    modeling_level: None,
                    tags: Vec::<Tag>::new(),
                    odcl_metadata: HashMap::new(),
                    owner: None,
                    sla: None,
                    contact_details: None,
                    infrastructure_type: None,
                    notes: None,
                    position: None,
                    yaml_file_path: None,
                    drawio_cell_id: None,
                    quality: quality_rules,
                    errors: Vec::new(),
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                };
                return Ok((table, errors));
            }
        };

        // Parse fields as columns
        let mut columns = Vec::new();
        for (field_name, field_data) in fields {
            if let Some(field_obj) = field_data.as_object() {
                match self.parse_data_contract_field(field_name, field_obj, data, &mut errors) {
                    Ok(mut cols) => columns.append(&mut cols),
                    Err(e) => {
                        errors.push(ParserError {
                            error_type: "field_parse_error".to_string(),
                            field: format!("Field '{}'", field_name),
                            message: e.to_string(),
                        });
                    }
                }
            } else {
                errors.push(ParserError {
                    error_type: "validation_error".to_string(),
                    field: format!("Field '{}'", field_name),
                    message: format!("Field '{}' must be an object", field_name),
                });
            }
        }

        // Extract metadata from info section
        // The frontend expects info fields to be nested under "info" key
        let mut odcl_metadata = HashMap::new();

        // Extract info section and nest it properly
        // Convert JsonValue object to serde_json::Value::Object (Map)
        if let Some(info_val) = data.get("info") {
            // Convert JsonValue to serde_json::Value
            let info_json_value = json_value_to_serde_value(info_val);
            odcl_metadata.insert("info".to_string(), info_json_value);
        }

        odcl_metadata.insert(
            "dataContractSpecification".to_string(),
            json_value_to_serde_value(
                data.get("dataContractSpecification")
                    .unwrap_or(&JsonValue::Null),
            ),
        );
        odcl_metadata.insert(
            "id".to_string(),
            json_value_to_serde_value(data.get("id").unwrap_or(&JsonValue::Null)),
        );
        // Note: model_name is not added to metadata as it's redundant (it's the table name)

        // Extract servicelevels if present
        if let Some(servicelevels_val) = data.get("servicelevels") {
            odcl_metadata.insert(
                "servicelevels".to_string(),
                json_value_to_serde_value(servicelevels_val),
            );
        }

        // Extract links if present
        if let Some(links_val) = data.get("links") {
            odcl_metadata.insert("links".to_string(), json_value_to_serde_value(links_val));
        }

        // Extract domain, dataProduct, tenant
        if let Some(domain_val) = data.get("domain").and_then(|v| v.as_str()) {
            odcl_metadata.insert(
                "domain".to_string(),
                json_value_to_serde_value(&JsonValue::String(domain_val.to_string())),
            );
        }
        if let Some(data_product_val) = data.get("dataProduct").and_then(|v| v.as_str()) {
            odcl_metadata.insert(
                "dataProduct".to_string(),
                json_value_to_serde_value(&JsonValue::String(data_product_val.to_string())),
            );
        }
        if let Some(tenant_val) = data.get("tenant").and_then(|v| v.as_str()) {
            odcl_metadata.insert(
                "tenant".to_string(),
                json_value_to_serde_value(&JsonValue::String(tenant_val.to_string())),
            );
        }

        // Extract top-level description (can be object or string)
        if let Some(desc_val) = data.get("description") {
            odcl_metadata.insert(
                "description".to_string(),
                json_value_to_serde_value(desc_val),
            );
        }

        // Extract pricing
        if let Some(pricing_val) = data.get("pricing") {
            odcl_metadata.insert(
                "pricing".to_string(),
                json_value_to_serde_value(pricing_val),
            );
        }

        // Extract team
        if let Some(team_val) = data.get("team") {
            odcl_metadata.insert("team".to_string(), json_value_to_serde_value(team_val));
        }

        // Extract roles
        if let Some(roles_val) = data.get("roles") {
            odcl_metadata.insert("roles".to_string(), json_value_to_serde_value(roles_val));
        }

        // Extract terms
        if let Some(terms_val) = data.get("terms") {
            odcl_metadata.insert("terms".to_string(), json_value_to_serde_value(terms_val));
        }

        // Extract full servers array (not just type)
        if let Some(servers_val) = data.get("servers") {
            odcl_metadata.insert(
                "servers".to_string(),
                json_value_to_serde_value(servers_val),
            );
        }

        // Extract infrastructure
        if let Some(infrastructure_val) = data.get("infrastructure") {
            odcl_metadata.insert(
                "infrastructure".to_string(),
                json_value_to_serde_value(infrastructure_val),
            );
        }

        // Extract database type from servers if available
        let database_type = self.extract_database_type_from_servers(data);

        // Extract catalog and schema from customProperties
        let (catalog_name, schema_name) = self.extract_catalog_schema(data);

        // Extract sharedDomains from customProperties
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

        // Extract tags from top-level tags field (Data Contract format)
        let mut tags: Vec<Tag> = Vec::new();
        if let Some(tags_arr) = data.get("tags").and_then(|v| v.as_array()) {
            for item in tags_arr {
                if let Some(s) = item.as_str() {
                    // Parse tag string to Tag enum (supports Simple, Pair, List formats)
                    if let Ok(tag) = Tag::from_str(s) {
                        tags.push(tag);
                    } else {
                        // Fallback: create Simple tag if parsing fails
                        tags.push(crate::models::Tag::Simple(s.to_string()));
                    }
                }
            }
        }

        // Extract quality rules
        let quality_rules = self.extract_quality_rules(data);

        // Store sharedDomains in metadata
        if !shared_domains.is_empty() {
            let shared_domains_json: Vec<serde_json::Value> = shared_domains
                .iter()
                .map(|d| serde_json::Value::String(d.clone()))
                .collect();
            odcl_metadata.insert(
                "sharedDomains".to_string(),
                serde_json::Value::Array(shared_domains_json),
            );
        }

        let table_uuid = self.extract_table_uuid(data);

        let table = Table {
            id: table_uuid,
            name: model_name.clone(),
            columns,
            database_type,
            catalog_name,
            schema_name,
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
            quality: quality_rules,
            errors: Vec::new(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        info!(
            "Parsed Data Contract table: {} with {} warnings/errors",
            model_name,
            errors.len()
        );
        Ok((table, errors))
    }

    /// Parse a single field from Data Contract format.
    fn parse_data_contract_field(
        &self,
        field_name: &str,
        field_data: &serde_json::Map<String, JsonValue>,
        data: &JsonValue,
        errors: &mut Vec<ParserError>,
    ) -> Result<Vec<Column>> {
        let mut columns = Vec::new();

        // Helper function to extract quality rules from a JSON object
        let extract_quality_from_obj =
            |obj: &serde_json::Map<String, JsonValue>| -> Vec<HashMap<String, serde_json::Value>> {
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
            };

        // Extract description from field_data (preserve empty strings)
        let description = field_data
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Extract quality rules from field_data
        let mut quality_rules = extract_quality_from_obj(field_data);

        // Check for $ref
        if let Some(ref_str) = field_data.get("$ref").and_then(|v| v.as_str()) {
            // Store ref_path (preserve even if definition doesn't exist)
            let ref_path = Some(ref_str.to_string());

            if let Some(definition) = resolve_ref(ref_str, data) {
                // Merge quality rules from definition if field doesn't have any

                // Also extract quality rules from definition and merge (if field doesn't have any)
                if quality_rules.is_empty() {
                    if let Some(def_obj) = definition.as_object() {
                        quality_rules = extract_quality_from_obj(def_obj);
                    }
                } else {
                    // Merge definition quality rules if field has some
                    if let Some(def_obj) = definition.as_object() {
                        let def_quality = extract_quality_from_obj(def_obj);
                        // Append definition quality rules (field-level takes precedence)
                        quality_rules.extend(def_quality);
                    }
                }

                let required = field_data
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Note: ODCS v3.1.0 uses the `required` field at the property level to indicate non-nullable columns,
                // not a quality rule. We should NOT add a "not_null" quality rule as it's not a valid ODCS quality type.
                // The `required` field will be set based on the `required` parameter during column creation.

                // Check if definition is an object/struct with nested structure
                let has_nested = definition
                    .get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| s == "object")
                    .unwrap_or(false)
                    || definition.get("properties").is_some()
                    || definition.get("fields").is_some();

                if has_nested {
                    // Expand STRUCT from definition into nested columns with dot notation
                    if let Some(properties) =
                        definition.get("properties").and_then(|v| v.as_object())
                    {
                        // Recursively expand nested properties
                        let nested_required: Vec<String> = definition
                            .get("required")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        for (nested_name, nested_schema) in properties {
                            let nested_required_field = nested_required.contains(nested_name);
                            expand_nested_column(
                                &format!("{}.{}", field_name, nested_name),
                                nested_schema,
                                !nested_required_field,
                                &mut columns,
                                errors,
                            );
                        }
                    } else if let Some(fields) =
                        definition.get("fields").and_then(|v| v.as_object())
                    {
                        // Handle fields format (ODCL style)
                        for (nested_name, nested_schema) in fields {
                            expand_nested_column(
                                &format!("{}.{}", field_name, nested_name),
                                nested_schema,
                                true, // Assume nullable if not specified
                                &mut columns,
                                errors,
                            );
                        }
                    } else {
                        // Fallback: create parent column as OBJECT if we can't expand
                        let def_physical_type = definition
                            .get("physicalType")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: "OBJECT".to_string(),
                            physical_type: def_physical_type,
                            nullable: !required,
                            description: if description.is_empty() {
                                definition
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string()
                            } else {
                                description.clone()
                            },
                            quality: quality_rules.clone(),
                            relationships: ref_to_relationships(&ref_path),
                            ..Default::default()
                        });
                    }
                } else {
                    // Simple type from definition
                    let def_type = definition
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("STRING")
                        .to_uppercase();

                    let enum_values = definition
                        .get("enum")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                .collect()
                        })
                        .unwrap_or_default();

                    let def_physical_type = definition
                        .get("physicalType")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: def_type,
                        physical_type: def_physical_type,
                        nullable: !required,
                        description: if description.is_empty() {
                            definition
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string()
                        } else {
                            description
                        },
                        quality: quality_rules,
                        relationships: ref_to_relationships(&ref_path),
                        enum_values,
                        ..Default::default()
                    });
                }
                return Ok(columns);
            } else {
                // Undefined reference - create column with error
                let mut col_errors: Vec<HashMap<String, serde_json::Value>> = Vec::new();
                let mut error_map = HashMap::new();
                error_map.insert("type".to_string(), serde_json::json!("validation_error"));
                error_map.insert("field".to_string(), serde_json::json!("data_type"));
                error_map.insert(
                    "message".to_string(),
                    serde_json::json!(format!(
                        "Field '{}' references undefined definition: {}",
                        field_name, ref_str
                    )),
                );
                col_errors.push(error_map);
                let field_physical_type = field_data
                    .get("physicalType")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                columns.push(Column {
                    name: field_name.to_string(),
                    data_type: "OBJECT".to_string(),
                    physical_type: field_physical_type,
                    description,
                    errors: col_errors,
                    relationships: ref_to_relationships(&Some(ref_str.to_string())),
                    ..Default::default()
                });
                return Ok(columns);
            }
        }

        // Extract field type - check both "logicalType" (ODCS v3.1.0) and "type" (legacy)
        // Default to STRING if missing
        let field_type_str = field_data
            .get("logicalType")
            .and_then(|v| v.as_str())
            .or_else(|| field_data.get("type").and_then(|v| v.as_str()))
            .unwrap_or("STRING");

        // Check if type contains STRUCT definition (multiline STRUCT type)
        if field_type_str.contains("STRUCT<") || field_type_str.contains("ARRAY<STRUCT<") {
            match self.parse_struct_type_from_string(field_name, field_type_str, field_data) {
                Ok(nested_cols) if !nested_cols.is_empty() => {
                    // We have nested columns - add parent column with full type, then nested columns
                    let parent_data_type = if field_type_str.to_uppercase().starts_with("ARRAY<") {
                        "ARRAY<STRUCT<...>>".to_string()
                    } else {
                        "STRUCT<...>".to_string()
                    };

                    let struct_physical_type = field_data
                        .get("physicalType")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());

                    // Add parent column
                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: parent_data_type,
                        physical_type: struct_physical_type,
                        nullable: !field_data
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        description: description.clone(),
                        quality: quality_rules.clone(),
                        relationships: ref_to_relationships(
                            &field_data
                                .get("$ref")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        ),
                        ..Default::default()
                    });

                    // Add nested columns
                    columns.extend(nested_cols);
                    return Ok(columns);
                }
                Ok(_) | Err(_) => {
                    // If parsing fails or returns empty, fall back to using the type as-is
                }
            }
        }

        let field_type = normalize_data_type(field_type_str);

        // Handle ARRAY type
        if field_type == "ARRAY" {
            let items = field_data.get("items");
            if let Some(items_val) = items {
                if let Some(items_obj) = items_val.as_object() {
                    // Check if items is an object with fields (nested structure)
                    // Check both "logicalType" (ODCS v3.1.0) and "type" (legacy) for backward compatibility
                    let items_type = items_obj
                        .get("logicalType")
                        .and_then(|v| v.as_str())
                        .or_else(|| items_obj.get("type").and_then(|v| v.as_str()));

                    // Normalize legacy "type" values to "logicalType" equivalents
                    let normalized_items_type = match items_type {
                        Some("object") | Some("struct") => Some("object"),
                        Some("array") => Some("array"),
                        Some("string") | Some("varchar") | Some("char") | Some("text") => {
                            Some("string")
                        }
                        Some("integer") | Some("int") | Some("bigint") | Some("smallint")
                        | Some("tinyint") => Some("integer"),
                        Some("number") | Some("decimal") | Some("double") | Some("float")
                        | Some("numeric") => Some("number"),
                        Some("boolean") | Some("bool") => Some("boolean"),
                        Some("date") => Some("date"),
                        Some("timestamp") | Some("datetime") => Some("timestamp"),
                        Some("time") => Some("time"),
                        other => other,
                    };

                    if items_obj.get("fields").is_some()
                        || items_obj.get("properties").is_some()
                        || normalized_items_type == Some("object")
                    {
                        // Array of objects - create parent column as ARRAY<OBJECT>
                        let array_physical_type = field_data
                            .get("physicalType")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: "ARRAY<OBJECT>".to_string(),
                            physical_type: array_physical_type,
                            nullable: !field_data
                                .get("required")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            description: field_data
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            ..Default::default()
                        });

                        // Extract nested fields from items.properties or items.fields if present
                        // Handle both object format (legacy) and array format (ODCS v3.1.0)
                        let properties_obj =
                            items_obj.get("properties").and_then(|v| v.as_object());
                        let properties_arr = items_obj.get("properties").and_then(|v| v.as_array());
                        let fields_obj = items_obj.get("fields").and_then(|v| v.as_object());

                        if let Some(fields_map) = properties_obj.or(fields_obj) {
                            // Object format (legacy): properties is a map
                            for (nested_field_name, nested_field_data) in fields_map {
                                if let Some(nested_field_obj) = nested_field_data.as_object() {
                                    // Check both "logicalType" (ODCS v3.1.0) and "type" (legacy)
                                    let nested_field_type = nested_field_obj
                                        .get("logicalType")
                                        .and_then(|v| v.as_str())
                                        .or_else(|| {
                                            nested_field_obj.get("type").and_then(|v| v.as_str())
                                        })
                                        .unwrap_or("STRING");

                                    // Recursively parse nested fields with array prefix
                                    let nested_col_name =
                                        format!("{}.[].{}", field_name, nested_field_name);
                                    let mut local_errors = Vec::new();
                                    match self.parse_data_contract_field(
                                        &nested_col_name,
                                        nested_field_obj,
                                        data,
                                        &mut local_errors,
                                    ) {
                                        Ok(mut nested_cols) => {
                                            // If nested field is itself an OBJECT/STRUCT, it will return multiple columns
                                            // Otherwise, it returns a single column
                                            columns.append(&mut nested_cols);
                                        }
                                        Err(_) => {
                                            // Fallback: create simple nested column
                                            let nested_physical_type = nested_field_obj
                                                .get("physicalType")
                                                .and_then(|v| v.as_str())
                                                .map(|s| s.to_string());
                                            columns.push(Column {
                                                name: nested_col_name,
                                                data_type: nested_field_type.to_uppercase(),
                                                physical_type: nested_physical_type,
                                                nullable: !nested_field_obj
                                                    .get("required")
                                                    .and_then(|v| v.as_bool())
                                                    .unwrap_or(false),
                                                description: nested_field_obj
                                                    .get("description")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("")
                                                    .to_string(),
                                                ..Default::default()
                                            });
                                        }
                                    }
                                }
                            }
                        } else if let Some(properties_list) = properties_arr {
                            // Array format (ODCS v3.1.0): properties is an array with 'name' field
                            let mut local_errors = Vec::new();
                            for prop_data in properties_list {
                                if let Some(prop_obj) = prop_data.as_object() {
                                    // Extract name from property object (required in v3.1.0)
                                    let nested_field_name = prop_obj
                                        .get("name")
                                        .or_else(|| prop_obj.get("id"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("");

                                    if !nested_field_name.is_empty() {
                                        // Recursively parse nested fields with array prefix
                                        let nested_col_name =
                                            format!("{}.[].{}", field_name, nested_field_name);
                                        match self.parse_data_contract_field(
                                            &nested_col_name,
                                            prop_obj,
                                            data,
                                            &mut local_errors,
                                        ) {
                                            Ok(mut nested_cols) => {
                                                // If nested field is itself an OBJECT/STRUCT, it will return multiple columns
                                                // Otherwise, it returns a single column
                                                columns.append(&mut nested_cols);
                                            }
                                            Err(_) => {
                                                // Fallback: create simple nested column
                                                // Check both "logicalType" (ODCS v3.1.0) and "type" (legacy)
                                                let nested_field_type = prop_obj
                                                    .get("logicalType")
                                                    .and_then(|v| v.as_str())
                                                    .or_else(|| {
                                                        prop_obj
                                                            .get("type")
                                                            .and_then(|v| v.as_str())
                                                    })
                                                    .unwrap_or("STRING");
                                                let nested_physical_type = prop_obj
                                                    .get("physicalType")
                                                    .and_then(|v| v.as_str())
                                                    .map(|s| s.to_string());
                                                columns.push(Column {
                                                    name: nested_col_name,
                                                    data_type: nested_field_type.to_uppercase(),
                                                    physical_type: nested_physical_type,
                                                    nullable: !prop_obj
                                                        .get("required")
                                                        .and_then(|v| v.as_bool())
                                                        .unwrap_or(false),
                                                    description: prop_obj
                                                        .get("description")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("")
                                                        .to_string(),
                                                    ..Default::default()
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        return Ok(columns);
                    } else if let Some(item_type) = items_obj.get("type").and_then(|v| v.as_str()) {
                        // Array of simple type
                        let array_physical_type = field_data
                            .get("physicalType")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: format!("ARRAY<{}>", normalize_data_type(item_type)),
                            physical_type: array_physical_type,
                            nullable: !field_data
                                .get("required")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            description: description.clone(),
                            quality: quality_rules.clone(),
                            relationships: ref_to_relationships(
                                &field_data
                                    .get("$ref")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string()),
                            ),
                            ..Default::default()
                        });
                        return Ok(columns);
                    }
                } else if let Some(item_type_str) = items_val.as_str() {
                    // Array of simple type (string)
                    let array_physical_type = field_data
                        .get("physicalType")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: format!("ARRAY<{}>", normalize_data_type(item_type_str)),
                        physical_type: array_physical_type,
                        nullable: !field_data
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        description: description.clone(),
                        quality: quality_rules.clone(),
                        relationships: ref_to_relationships(
                            &field_data
                                .get("$ref")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        ),
                        ..Default::default()
                    });
                    return Ok(columns);
                }
            }
            // Array without items - default to ARRAY<STRING>
            let array_physical_type = field_data
                .get("physicalType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            columns.push(Column {
                name: field_name.to_string(),
                data_type: "ARRAY<STRING>".to_string(),
                physical_type: array_physical_type,
                nullable: !field_data
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                description: description.clone(),
                quality: quality_rules.clone(),
                relationships: ref_to_relationships(
                    &field_data
                        .get("$ref")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                ),
                ..Default::default()
            });
            return Ok(columns);
        }

        // Check if this is a nested object with fields or properties
        // Note: Export saves nested columns in properties.properties, but some formats use fields
        // Handle both object format (legacy/ODCL) and array format (ODCS v3.1.0)
        let nested_fields_obj = field_data
            .get("properties")
            .and_then(|v| v.as_object())
            .or_else(|| field_data.get("fields").and_then(|v| v.as_object()));
        let nested_fields_arr = field_data.get("properties").and_then(|v| v.as_array());

        if field_type == "OBJECT" && (nested_fields_obj.is_some() || nested_fields_arr.is_some()) {
            // Inline nested object - create parent column as OBJECT and extract nested fields

            // Extract physicalType for the parent OBJECT column
            let parent_physical_type = field_data
                .get("physicalType")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Create parent column
            columns.push(Column {
                name: field_name.to_string(),
                data_type: "OBJECT".to_string(),
                physical_type: parent_physical_type,
                nullable: !field_data
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                description: description.clone(),
                quality: quality_rules.clone(),
                relationships: ref_to_relationships(
                    &field_data
                        .get("$ref")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                ),
                ..Default::default()
            });

            // Extract nested fields recursively - handle both object and array formats
            if let Some(fields_obj) = nested_fields_obj {
                // Object format (legacy/ODCL): properties is a map of name -> schema
                for (nested_field_name, nested_field_data) in fields_obj {
                    if let Some(nested_field_obj) = nested_field_data.as_object() {
                        let nested_field_type = nested_field_obj
                            .get("logicalType")
                            .and_then(|v| v.as_str())
                            .or_else(|| nested_field_obj.get("type").and_then(|v| v.as_str()))
                            .unwrap_or("STRING");

                        // Recursively parse nested fields
                        let nested_col_name = format!("{}.{}", field_name, nested_field_name);
                        match self.parse_odcl_v3_property(&nested_col_name, nested_field_obj, data)
                        {
                            Ok(mut nested_cols) => {
                                // If nested field is itself an OBJECT/STRUCT, it will return multiple columns
                                // Otherwise, it returns a single column
                                columns.append(&mut nested_cols);
                            }
                            Err(_) => {
                                // Fallback: create simple nested column
                                let nested_physical_type = nested_field_obj
                                    .get("physicalType")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string());
                                columns.push(Column {
                                    name: nested_col_name,
                                    data_type: nested_field_type.to_uppercase(),
                                    physical_type: nested_physical_type,
                                    nullable: !nested_field_obj
                                        .get("required")
                                        .and_then(|v| v.as_bool())
                                        .unwrap_or(false),
                                    description: nested_field_obj
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            } else if let Some(fields_arr) = nested_fields_arr {
                // Array format (ODCS v3.1.0): properties is an array with 'name' field
                for prop_data in fields_arr {
                    if let Some(prop_obj) = prop_data.as_object() {
                        // Extract name from property object (required in v3.1.0)
                        let nested_field_name = prop_obj
                            .get("name")
                            .or_else(|| prop_obj.get("id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if !nested_field_name.is_empty() {
                            let nested_field_type = prop_obj
                                .get("logicalType")
                                .and_then(|v| v.as_str())
                                .or_else(|| prop_obj.get("type").and_then(|v| v.as_str()))
                                .unwrap_or("STRING");

                            // Recursively parse nested fields
                            let nested_col_name = format!("{}.{}", field_name, nested_field_name);
                            match self.parse_odcl_v3_property(&nested_col_name, prop_obj, data) {
                                Ok(mut nested_cols) => {
                                    // If nested field is itself an OBJECT/STRUCT, it will return multiple columns
                                    // Otherwise, it returns a single column
                                    columns.append(&mut nested_cols);
                                }
                                Err(_) => {
                                    // Fallback: create simple nested column
                                    let nested_physical_type = prop_obj
                                        .get("physicalType")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string());
                                    columns.push(Column {
                                        name: nested_col_name,
                                        data_type: nested_field_type.to_uppercase(),
                                        physical_type: nested_physical_type,
                                        nullable: !prop_obj
                                            .get("required")
                                            .and_then(|v| v.as_bool())
                                            .unwrap_or(false),
                                        description: prop_obj
                                            .get("description")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string(),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }

            return Ok(columns);
        }

        // Regular field (no $ref or $ref not found)
        let required = field_data
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Use description extracted at function start, or extract if not yet extracted
        let field_description = if description.is_empty() {
            field_data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            description
        };

        // Use quality_rules extracted at function start, or extract if not yet extracted
        let mut column_quality_rules = quality_rules;

        // Extract column-level quality rules if not already extracted
        if column_quality_rules.is_empty()
            && let Some(quality_val) = field_data.get("quality")
        {
            if let Some(arr) = quality_val.as_array() {
                // Array of quality rules
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        let mut rule = HashMap::new();
                        for (key, value) in obj {
                            rule.insert(key.clone(), json_value_to_serde_value(value));
                        }
                        column_quality_rules.push(rule);
                    }
                }
            } else if let Some(obj) = quality_val.as_object() {
                // Single quality rule object
                let mut rule = HashMap::new();
                for (key, value) in obj {
                    rule.insert(key.clone(), json_value_to_serde_value(value));
                }
                column_quality_rules.push(rule);
            }
        }

        // Note: ODCS v3.1.0 uses the `required` field at the property level to indicate non-nullable columns,
        // not a quality rule. We should NOT add a "not_null" quality rule as it's not a valid ODCS quality type.
        // The `required` field will be set based on the `required` parameter during column creation.

        // Extract physicalType (ODCS v3.1.0) - the actual database type
        let physical_type = field_data
            .get("physicalType")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Parse the column with all ODCS v3.1.0 metadata fields
        let column = self.parse_column_metadata_from_field(
            field_name,
            &field_type,
            physical_type,
            !required,
            field_description,
            column_quality_rules,
            field_data,
        );

        columns.push(column);

        Ok(columns)
    }

    /// Parse all column metadata fields from ODCS v3.1.0 field data.
    /// This extracts all supported column-level metadata including:
    /// - businessName, physicalName, logicalTypeOptions
    /// - primaryKey, primaryKeyPosition, unique
    /// - partitioned, partitionKeyPosition, clustered
    /// - classification, criticalDataElement, encryptedName
    /// - transformSourceObjects, transformLogic, transformDescription
    /// - examples, authoritativeDefinitions, tags, customProperties
    #[allow(clippy::too_many_arguments)]
    fn parse_column_metadata_from_field(
        &self,
        name: &str,
        data_type: &str,
        physical_type: Option<String>,
        nullable: bool,
        description: String,
        quality: Vec<HashMap<String, serde_json::Value>>,
        field_data: &serde_json::Map<String, JsonValue>,
    ) -> Column {
        use crate::models::{AuthoritativeDefinition, LogicalTypeOptions};

        // businessName
        let business_name = field_data
            .get("businessName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // physicalName
        let physical_name = field_data
            .get("physicalName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // logicalTypeOptions
        let logical_type_options = field_data.get("logicalTypeOptions").and_then(|v| {
            v.as_object().map(|opts| LogicalTypeOptions {
                min_length: opts.get("minLength").and_then(|v| v.as_i64()),
                max_length: opts.get("maxLength").and_then(|v| v.as_i64()),
                pattern: opts
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                format: opts
                    .get("format")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                minimum: opts.get("minimum").cloned(),
                maximum: opts.get("maximum").cloned(),
                exclusive_minimum: opts.get("exclusiveMinimum").cloned(),
                exclusive_maximum: opts.get("exclusiveMaximum").cloned(),
                precision: opts
                    .get("precision")
                    .and_then(|v| v.as_i64())
                    .map(|n| n as i32),
                scale: opts.get("scale").and_then(|v| v.as_i64()).map(|n| n as i32),
            })
        });

        // primaryKey
        let primary_key = field_data
            .get("primaryKey")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // primaryKeyPosition
        let primary_key_position = field_data
            .get("primaryKeyPosition")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32);

        // unique
        let unique = field_data
            .get("unique")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // partitioned
        let partitioned = field_data
            .get("partitioned")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // partitionKeyPosition
        let partition_key_position = field_data
            .get("partitionKeyPosition")
            .and_then(|v| v.as_i64())
            .map(|n| n as i32);

        // clustered
        let clustered = field_data
            .get("clustered")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // classification
        let classification = field_data
            .get("classification")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // criticalDataElement
        let critical_data_element = field_data
            .get("criticalDataElement")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // encryptedName
        let encrypted_name = field_data
            .get("encryptedName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // transformSourceObjects
        let transform_source_objects = field_data
            .get("transformSourceObjects")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // transformLogic
        let transform_logic = field_data
            .get("transformLogic")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // transformDescription
        let transform_description = field_data
            .get("transformDescription")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // examples
        let examples = field_data
            .get("examples")
            .and_then(|v| v.as_array())
            .map(|arr| arr.to_vec())
            .unwrap_or_default();

        // authoritativeDefinitions
        let authoritative_definitions = field_data
            .get("authoritativeDefinitions")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.as_object().map(|obj| AuthoritativeDefinition {
                            definition_type: obj
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            url: obj
                                .get("url")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // tags
        let tags = field_data
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // customProperties
        let custom_properties = field_data
            .get("customProperties")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| {
                        item.as_object().and_then(|obj| {
                            let key = obj.get("property").and_then(|v| v.as_str())?;
                            let value = obj.get("value").cloned()?;
                            Some((key.to_string(), value))
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        // businessKey (secondary_key)
        let secondary_key = field_data
            .get("businessKey")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // enum values
        let enum_values = field_data
            .get("enum")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        // constraints
        let constraints = field_data
            .get("constraints")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Column {
            name: name.to_string(),
            data_type: data_type.to_string(),
            physical_type,
            physical_name,
            nullable,
            description,
            quality,
            business_name,
            logical_type_options,
            primary_key,
            primary_key_position,
            unique,
            partitioned,
            partition_key_position,
            clustered,
            classification,
            critical_data_element,
            encrypted_name,
            transform_source_objects,
            transform_logic,
            transform_description,
            examples,
            authoritative_definitions,
            tags,
            custom_properties,
            secondary_key,
            enum_values,
            constraints,
            foreign_key: self.parse_foreign_key_from_data_contract(field_data),
            relationships: self.parse_relationships_from_field(field_data),
            ..Default::default()
        }
    }

    /// Parse foreign key from Data Contract field data.
    fn parse_foreign_key_from_data_contract(
        &self,
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

    /// Parse relationships from Data Contract field data.
    /// Also converts $ref to a relationship entry if present.
    fn parse_relationships_from_field(
        &self,
        field_data: &serde_json::Map<String, JsonValue>,
    ) -> Vec<PropertyRelationship> {
        let mut relationships = Vec::new();

        // Parse relationships array from ODCS v3.1.0 format
        if let Some(rels_array) = field_data.get("relationships").and_then(|v| v.as_array()) {
            for rel in rels_array {
                if let Some(rel_obj) = rel.as_object() {
                    let rel_type = rel_obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("foreignKey")
                        .to_string();
                    let to = rel_obj
                        .get("to")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    if !to.is_empty() {
                        relationships.push(PropertyRelationship {
                            relationship_type: rel_type,
                            to,
                        });
                    }
                }
            }
        }

        // Convert $ref to relationship if present and no relationships were parsed
        if relationships.is_empty()
            && let Some(ref_str) = field_data.get("$ref").and_then(|v| v.as_str())
        {
            // Convert $ref path to ODCS relationship format
            let to = if ref_str.starts_with("#/definitions/") {
                let def_path = ref_str.strip_prefix("#/definitions/").unwrap_or(ref_str);
                format!("definitions/{}", def_path)
            } else if ref_str.starts_with("#/") {
                ref_str.strip_prefix("#/").unwrap_or(ref_str).to_string()
            } else {
                ref_str.to_string()
            };

            relationships.push(PropertyRelationship {
                relationship_type: "foreignKey".to_string(),
                to,
            });
        }

        relationships
    }

    /// Extract database type from servers in Data Contract format.
    fn extract_database_type_from_servers(&self, data: &JsonValue) -> Option<DatabaseType> {
        // Data Contract format: servers can be object or array
        if let Some(servers_obj) = data.get("servers").and_then(|v| v.as_object()) {
            // Object format: { "server_name": { "type": "..." } }
            if let Some((_, server_data)) = servers_obj.iter().next()
                && let Some(server_obj) = server_data.as_object()
            {
                return server_obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .and_then(|s| self.parse_database_type(s));
            }
        } else if let Some(servers_arr) = data.get("servers").and_then(|v| v.as_array()) {
            // Array format: [ { "server": "...", "type": "..." } ]
            if let Some(server_obj) = servers_arr.first().and_then(|v| v.as_object()) {
                return server_obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .and_then(|s| self.parse_database_type(s));
            }
        }
        None
    }

    /// Parse database type string to enum.
    fn parse_database_type(&self, s: &str) -> Option<DatabaseType> {
        match s.to_lowercase().as_str() {
            "databricks" | "databricks_delta" => Some(DatabaseType::DatabricksDelta),
            "postgres" | "postgresql" => Some(DatabaseType::Postgres),
            "mysql" => Some(DatabaseType::Mysql),
            "sql_server" | "sqlserver" => Some(DatabaseType::SqlServer),
            "aws_glue" | "glue" => Some(DatabaseType::AwsGlue),
            _ => None,
        }
    }

    /// Parse STRUCT type definition from string (e.g., "ARRAY<STRUCT<ID: STRING, NAME: STRING>>").
    /// Parse STRUCT type from string and create nested columns
    /// This is public so it can be used by SQL importer to parse STRUCT types
    pub fn parse_struct_type_from_string(
        &self,
        field_name: &str,
        type_str: &str,
        field_data: &serde_json::Map<String, JsonValue>,
    ) -> Result<Vec<Column>> {
        let mut columns = Vec::new();

        // Normalize whitespace - replace newlines and multiple spaces with single space
        let normalized_type = type_str
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        let type_str_upper = normalized_type.to_uppercase();

        // Check if it's ARRAY<STRUCT<...>>
        let is_array = type_str_upper.starts_with("ARRAY<");
        let struct_start = type_str_upper.find("STRUCT<");

        if let Some(start_pos) = struct_start {
            // Extract STRUCT content - start from STRUCT< and find its closing >
            // For ARRAY<STRUCT<...>>, we still need to find the STRUCT<...> closing bracket
            let struct_content_start = start_pos + 7; // Skip "STRUCT<"
            let struct_content = &normalized_type[struct_content_start..];

            // Find matching closing bracket for STRUCT<, handling nested STRUCTs
            // We need to find the > that closes STRUCT<, not ARRAY<
            let mut depth = 1;
            let mut end_pos = None;
            for (i, ch) in struct_content.char_indices() {
                match ch {
                    '<' => depth += 1,
                    '>' => {
                        depth -= 1;
                        if depth == 0 {
                            end_pos = Some(i);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            // If no closing bracket found, try to infer it (handle malformed YAML)
            let struct_fields_str = if let Some(end) = end_pos {
                &struct_content[..end]
            } else {
                // Missing closing bracket - use everything up to the end
                struct_content.trim_end_matches('>').trim()
            };

            // Parse fields: "ID: STRING, NAME: STRING, ..."
            let fields = self.parse_struct_fields_from_string(struct_fields_str)?;

            // Create nested columns, handling nested STRUCTs recursively
            // For ARRAY<STRUCT<...>>, use .[] notation for array elements: parent.[].field
            // For STRUCT<...>, use dot notation: parent.field
            for (nested_name, nested_type) in fields {
                let nested_type_upper = nested_type.to_uppercase();
                let nested_col_name = if is_array {
                    format!("{}.[].{}", field_name, nested_name)
                } else {
                    format!("{}.{}", field_name, nested_name)
                };

                // Check if this field is itself a STRUCT or ARRAY<STRUCT>
                // Handle multiple levels of nesting: STRUCT<...>, ARRAY<STRUCT<...>>, ARRAY<STRUCT<...>> within STRUCT, etc.
                let is_nested_struct = nested_type_upper.starts_with("STRUCT<");
                let is_nested_array_struct = nested_type_upper.starts_with("ARRAY<STRUCT<");

                if is_nested_struct || is_nested_array_struct {
                    // Recursively parse nested STRUCT or ARRAY<STRUCT>
                    // The nested_col_name already includes the parent path:
                    // - For STRUCT within ARRAY: "items.[].details"
                    // - For STRUCT within STRUCT: "orderInfo.metadata"
                    // - For ARRAY<STRUCT> within STRUCT: "parent.items"
                    // - For ARRAY<STRUCT> within ARRAY<STRUCT>: "parent.[].items"
                    // Recursive calls will correctly build the full path with proper notation
                    match self.parse_struct_type_from_string(
                        &nested_col_name,
                        &nested_type,
                        field_data,
                    ) {
                        Ok(nested_cols) => {
                            // Add all recursively parsed columns
                            // These will have names like:
                            // - items.[].details.name (STRUCT within ARRAY<STRUCT>)
                            // - orderInfo.metadata.field (STRUCT within STRUCT)
                            // - parent.items.[].field (ARRAY<STRUCT> within STRUCT)
                            // - parent.[].items.[].field (ARRAY<STRUCT> within ARRAY<STRUCT>)
                            columns.extend(nested_cols);
                        }
                        Err(_) => {
                            // If recursive parsing fails, add the parent column with STRUCT/ARRAY type
                            let fallback_data_type = if is_nested_array_struct {
                                "ARRAY<STRUCT<...>>".to_string()
                            } else {
                                "STRUCT<...>".to_string()
                            };
                            let nested_physical_type = field_data
                                .get("physicalType")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string());
                            columns.push(Column {
                                name: nested_col_name,
                                data_type: fallback_data_type,
                                physical_type: nested_physical_type,
                                nullable: !field_data
                                    .get("required")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false),
                                description: field_data
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                ..Default::default()
                            });
                        }
                    }
                } else if nested_type_upper.starts_with("ARRAY<") {
                    // Handle ARRAY of simple types (not STRUCT) - these don't need nested columns
                    // But if it's ARRAY<STRUCT<...>>, it would have been caught above
                    let nested_physical_type = field_data
                        .get("physicalType")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    columns.push(Column {
                        name: nested_col_name,
                        data_type: normalize_data_type(&nested_type),
                        physical_type: nested_physical_type,
                        nullable: !field_data
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        description: field_data
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        ..Default::default()
                    });
                } else {
                    // Simple nested field (not a STRUCT)
                    let nested_physical_type = field_data
                        .get("physicalType")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    columns.push(Column {
                        name: nested_col_name,
                        data_type: normalize_data_type(&nested_type),
                        physical_type: nested_physical_type,
                        nullable: !field_data
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        description: field_data
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        ..Default::default()
                    });
                }
            }

            return Ok(columns);
        }

        // If no STRUCT found, return empty (fallback to regular parsing)
        Ok(Vec::new())
    }

    /// Parse STRUCT fields from string (e.g., "ID: STRING, NAME: STRING").
    fn parse_struct_fields_from_string(&self, fields_str: &str) -> Result<Vec<(String, String)>> {
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
                        && let Some((name, type_part)) = self.parse_field_definition(trimmed)
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
            && let Some((name, type_part)) = self.parse_field_definition(trimmed)
        {
            fields.push((name, type_part));
        }

        Ok(fields)
    }

    /// Parse a single field definition (e.g., "ID: STRING" or "DETAILS: STRUCT<...>").
    fn parse_field_definition(&self, field_def: &str) -> Option<(String, String)> {
        // Split by colon, but handle nested STRUCTs
        let colon_pos = field_def.find(':')?;
        let name = field_def[..colon_pos].trim().to_string();
        let type_part = field_def[colon_pos + 1..].trim().to_string();

        if name.is_empty() || type_part.is_empty() {
            return None;
        }

        Some((name, type_part))
    }

    /// Extract catalog and schema from customProperties.
    fn extract_catalog_schema(&self, data: &JsonValue) -> (Option<String>, Option<String>) {
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
}

impl Default for ODCSImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_odcl_table() {
        let mut parser = ODCSImporter::new();
        let odcl_yaml = r#"
name: users
columns:
  - name: id
    data_type: INT
    nullable: false
    primary_key: true
  - name: name
    data_type: VARCHAR(255)
    nullable: false
database_type: Postgres
"#;

        let (table, errors) = parser.parse(odcl_yaml).unwrap();
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[0].name, "id");
        assert_eq!(table.database_type, Some(DatabaseType::Postgres));
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_parse_odcl_with_metadata() {
        let mut parser = ODCSImporter::new();
        let odcl_yaml = r#"
name: users
columns:
  - name: id
    data_type: INT
medallion_layer: gold
scd_pattern: TYPE_2
odcl_metadata:
  description: "User table"
  owner: "data-team"
"#;

        let (table, errors) = parser.parse(odcl_yaml).unwrap();
        assert_eq!(table.medallion_layers.len(), 1);
        assert_eq!(table.medallion_layers[0], MedallionLayer::Gold);
        assert_eq!(table.scd_pattern, Some(SCDPattern::Type2));
        if let Some(serde_json::Value::String(desc)) = table.odcl_metadata.get("description") {
            assert_eq!(desc, "User table");
        }
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_parse_odcl_with_data_vault() {
        let mut parser = ODCSImporter::new();
        let odcl_yaml = r#"
name: hub_customer
columns:
  - name: customer_key
    data_type: VARCHAR(50)
data_vault_classification: Hub
"#;

        let (table, errors) = parser.parse(odcl_yaml).unwrap();
        assert_eq!(
            table.data_vault_classification,
            Some(DataVaultClassification::Hub)
        );
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_parse_invalid_odcl() {
        let mut parser = ODCSImporter::new();
        let invalid_yaml = "not: valid: yaml: structure:";

        // Should fail to parse YAML
        assert!(parser.parse(invalid_yaml).is_err());
    }

    #[test]
    fn test_parse_odcl_missing_required_fields() {
        let mut parser = ODCSImporter::new();
        let non_conformant = r#"
name: users
# Missing required columns field
"#;

        // Should fail with missing columns
        assert!(parser.parse(non_conformant).is_err());
    }

    #[test]
    fn test_parse_odcl_with_foreign_key() {
        let mut parser = ODCSImporter::new();
        let odcl_yaml = r#"
name: orders
columns:
  - name: id
    data_type: INT
    primary_key: true
  - name: user_id
    data_type: INT
    foreign_key:
      table_id: users
      column_name: id
"#;

        let (table, errors) = parser.parse(odcl_yaml).unwrap();
        assert_eq!(table.columns.len(), 2);
        let user_id_col = table.columns.iter().find(|c| c.name == "user_id").unwrap();
        assert!(user_id_col.foreign_key.is_some());
        assert_eq!(user_id_col.foreign_key.as_ref().unwrap().table_id, "users");
        assert_eq!(errors.len(), 0);
    }

    #[test]
    fn test_parse_odcl_with_constraints() {
        let mut parser = ODCSImporter::new();
        let odcl_yaml = r#"
name: products
columns:
  - name: id
    data_type: INT
    primary_key: true
  - name: name
    data_type: VARCHAR(255)
    nullable: false
    constraints:
      - UNIQUE
      - NOT NULL
"#;

        let (table, errors) = parser.parse(odcl_yaml).unwrap();
        assert_eq!(table.columns.len(), 2);
        let name_col = table.columns.iter().find(|c| c.name == "name").unwrap();
        assert!(!name_col.nullable);
        assert!(name_col.constraints.contains(&"UNIQUE".to_string()));
        assert_eq!(errors.len(), 0);
    }
}
