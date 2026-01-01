//! ODCS parser service for parsing Open Data Contract Standard YAML files.
//!
//! This service parses ODCS (Open Data Contract Standard) v3.1.0 and legacy ODCL (Data Contract Specification) YAML files
//! and converts them to Table models. ODCL files are automatically converted to ODCS v3.1.0 format.
//! Supports multiple formats:
//! - ODCS v3.1.0 / v3.0.x format (apiVersion, kind, schema) - PRIMARY FORMAT
//! - ODCL (Data Contract Specification) format (dataContractSpecification, models, definitions) - LEGACY, converted to ODCS
//! - Simple ODCL format (name, columns) - LEGACY, converted to ODCS
//! - Liquibase format

use super::{ImportError, ImportResult, TableData};
use crate::models::column::ForeignKey;
use crate::models::enums::{DataVaultClassification, DatabaseType, MedallionLayer, SCDPattern};
use crate::models::{Column, Table};
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tracing::info;

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
    /// use data_modelling_sdk::import::odcs::ODCSImporter;
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
    /// use data_modelling_sdk::import::odcs::ODCSImporter;
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
        match self.parse(yaml_content) {
            Ok((table, errors)) => {
                let sdk_tables = vec![TableData {
                    table_index: 0,
                    name: Some(table.name.clone()),
                    columns: table
                        .columns
                        .iter()
                        .map(|c| super::ColumnData {
                            name: c.name.clone(),
                            data_type: c.data_type.clone(),
                            nullable: c.nullable,
                            primary_key: c.primary_key,
                        })
                        .collect(),
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

    /// Resolve a $ref reference like '#/definitions/betAction'.
    fn resolve_ref<'a>(&self, ref_str: &str, data: &'a JsonValue) -> Option<&'a JsonValue> {
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
            tags: Vec::new(),
            odcl_metadata,
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

        // If nullable=false (required=true), add a "not_null" quality rule if not already present
        if !nullable {
            let has_not_null = column_quality_rules.iter().any(|rule| {
                rule.get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        s.to_lowercase().contains("not_null")
                            || s.to_lowercase().contains("notnull")
                    })
                    .unwrap_or(false)
            });
            if !has_not_null {
                let mut not_null_rule = HashMap::new();
                not_null_rule.insert("type".to_string(), serde_json::json!("not_null"));
                not_null_rule.insert(
                    "description".to_string(),
                    serde_json::json!("Column must not be null"),
                );
                column_quality_rules.push(not_null_rule);
            }
        }

        Ok(Column {
            name,
            data_type,
            nullable,
            primary_key,
            secondary_key: false,
            composite_key: None,
            foreign_key,
            constraints,
            description,
            errors: Vec::new(),
            quality: column_quality_rules,
            enum_values: Vec::new(),
            column_order: 0,
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
                    tags: Vec::new(),
                    odcl_metadata: HashMap::new(),
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
                    tags: Vec::new(),
                    odcl_metadata: HashMap::new(),
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

        let properties = schema_object
            .get("properties")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                errors.push(ParserError {
                    error_type: "validation_error".to_string(),
                    field: format!("Object '{}'", object_name),
                    message: format!("Object '{}' missing 'properties' field", object_name),
                });
                anyhow::anyhow!("Missing properties")
            })?;

        // Parse properties as columns (similar to Data Contract)
        let mut columns = Vec::new();
        for (prop_name, prop_data) in properties {
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

        // Extract metadata from customProperties
        let (medallion_layers, scd_pattern, data_vault_classification, mut tags) =
            self.extract_metadata_from_custom_properties(data);

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
                if let Some(s) = item.as_str()
                    && !tags.contains(&s.to_string())
                {
                    tags.push(s.to_string());
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
        Vec<String>,
    ) {
        let mut medallion_layers = Vec::new();
        let mut scd_pattern = None;
        let mut data_vault_classification = None;
        let mut tags = Vec::new();

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
                                        tags.push(s.to_string());
                                    }
                                }
                            } else if let Some(s) = prop_value.and_then(|v| v.as_str()) {
                                // Comma-separated string
                                for part in s.split(',') {
                                    tags.push(part.trim().to_string());
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
                if let Some(s) = item.as_str()
                    && !tags.contains(&s.to_string())
                {
                    tags.push(s.to_string());
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
                    tags: Vec::new(),
                    odcl_metadata: HashMap::new(),
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
        let mut tags = Vec::new();
        if let Some(tags_arr) = data.get("tags").and_then(|v| v.as_array()) {
            for item in tags_arr {
                if let Some(s) = item.as_str() {
                    tags.push(s.to_string());
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

    /// Expand a nested column from a schema definition, creating columns with dot notation.
    ///
    /// This helper function recursively expands nested structures (OBJECT/STRUCT types)
    /// into flat columns with dot notation (e.g., "address.street", "address.city").
    #[allow(clippy::only_used_in_recursion)]
    fn expand_nested_column(
        &self,
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

        let schema_type = schema_obj
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("object");

        match schema_type {
            "object" | "struct" => {
                // Check if it has nested properties
                if let Some(properties) = schema_obj.get("properties").and_then(|v| v.as_object()) {
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
                        self.expand_nested_column(
                            &format!("{}.{}", column_name, nested_name),
                            nested_schema,
                            nullable || nested_nullable,
                            columns,
                            errors,
                        );
                    }
                } else {
                    // Object without properties - create as OBJECT type
                    let description = schema_obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    columns.push(Column {
                        name: column_name.to_string(),
                        data_type: "OBJECT".to_string(),
                        nullable,
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description,
                        quality: Vec::new(),
                        enum_values: Vec::new(),
                        errors: Vec::new(),
                        column_order: 0,
                    });
                }
            }
            "array" => {
                // Handle array types
                let items = schema_obj.get("items").unwrap_or(schema);
                let items_type = items
                    .as_object()
                    .and_then(|obj| obj.get("type").and_then(|v| v.as_str()))
                    .unwrap_or("string");

                if items_type == "object" || items_type == "struct" {
                    // Array of objects - expand nested structure
                    let description = schema_obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    columns.push(Column {
                        name: column_name.to_string(),
                        data_type: "ARRAY<OBJECT>".to_string(),
                        nullable,
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description,
                        quality: Vec::new(),
                        enum_values: Vec::new(),
                        errors: Vec::new(),
                        column_order: 0,
                    });
                    // Also expand nested properties with array prefix
                    if let Some(properties) = items
                        .as_object()
                        .and_then(|obj| obj.get("properties").and_then(|v| v.as_object()))
                    {
                        let nested_required: Vec<String> = items
                            .as_object()
                            .and_then(|obj| obj.get("required").and_then(|v| v.as_array()))
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default();

                        for (nested_name, nested_schema) in properties {
                            let nested_nullable = !nested_required.contains(nested_name);
                            self.expand_nested_column(
                                &format!("{}.{}", column_name, nested_name),
                                nested_schema,
                                nullable || nested_nullable,
                                columns,
                                errors,
                            );
                        }
                    }
                } else {
                    // Array of primitives
                    let data_type = format!("ARRAY<{}>", items_type.to_uppercase());
                    let description = schema_obj
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    columns.push(Column {
                        name: column_name.to_string(),
                        data_type,
                        nullable,
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description,
                        quality: Vec::new(),
                        enum_values: Vec::new(),
                        errors: Vec::new(),
                        column_order: 0,
                    });
                }
            }
            _ => {
                // Simple type
                let data_type = schema_type.to_uppercase();
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
                    nullable,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description,
                    quality: Vec::new(),
                    enum_values,
                    errors: Vec::new(),
                    column_order: 0,
                });
            }
        }
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

        // Check for $ref
        if let Some(ref_str) = field_data.get("$ref").and_then(|v| v.as_str()) {
            if let Some(definition) = self.resolve_ref(ref_str, data) {
                // Extract quality rules from field-level first (takes precedence)
                let mut quality_rules = extract_quality_from_obj(field_data);

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

                // If required=true, add not_null quality rule if not present
                if required {
                    let has_not_null = quality_rules.iter().any(|rule| {
                        rule.get("type")
                            .and_then(|v| v.as_str())
                            .map(|s| {
                                s.to_lowercase().contains("not_null")
                                    || s.to_lowercase().contains("notnull")
                            })
                            .unwrap_or(false)
                    });
                    if !has_not_null {
                        let mut not_null_rule = HashMap::new();
                        not_null_rule.insert("type".to_string(), serde_json::json!("not_null"));
                        not_null_rule.insert(
                            "description".to_string(),
                            serde_json::json!("Column must not be null"),
                        );
                        quality_rules.push(not_null_rule);
                    }
                }

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
                            self.expand_nested_column(
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
                            self.expand_nested_column(
                                &format!("{}.{}", field_name, nested_name),
                                nested_schema,
                                true, // Assume nullable if not specified
                                &mut columns,
                                errors,
                            );
                        }
                    } else {
                        // Fallback: create parent column as OBJECT if we can't expand
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: "OBJECT".to_string(),
                            nullable: !required,
                            primary_key: false,
                            secondary_key: false,
                            composite_key: None,
                            foreign_key: None,
                            constraints: Vec::new(),
                            description: field_data
                                .get("description")
                                .or_else(|| definition.get("description"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            errors: Vec::new(),
                            quality: quality_rules.clone(),
                            enum_values: Vec::new(),
                            column_order: 0,
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

                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: def_type,
                        nullable: !required,
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description: field_data
                            .get("description")
                            .or_else(|| definition.get("description"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        errors: Vec::new(),
                        quality: quality_rules,
                        enum_values,
                        column_order: 0,
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
                columns.push(Column {
                    name: field_name.to_string(),
                    data_type: "OBJECT".to_string(),
                    nullable: true,
                    primary_key: false,
                    secondary_key: false,
                    composite_key: None,
                    foreign_key: None,
                    constraints: Vec::new(),
                    description: field_data
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    errors: col_errors,
                    quality: Vec::new(),
                    enum_values: Vec::new(),
                    column_order: 0,
                });
                return Ok(columns);
            }
        }

        // Extract field type - default to STRING if missing
        let field_type_str = field_data
            .get("type")
            .and_then(|v| v.as_str())
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

                    // Add parent column
                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: parent_data_type,
                        nullable: !field_data
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description: field_data
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        errors: Vec::new(),
                        quality: Vec::new(),
                        enum_values: Vec::new(),
                        column_order: 0,
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
                    if items_obj.get("fields").is_some()
                        || items_obj.get("type").and_then(|v| v.as_str()) == Some("object")
                    {
                        // Array of objects - create parent column as ARRAY<OBJECT>
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: "ARRAY<OBJECT>".to_string(),
                            nullable: !field_data
                                .get("required")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            primary_key: false,
                            secondary_key: false,
                            composite_key: None,
                            foreign_key: None,
                            constraints: Vec::new(),
                            description: field_data
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            errors: Vec::new(),
                            quality: Vec::new(),
                            enum_values: Vec::new(),
                            column_order: 0,
                        });

                        // Extract nested fields from items.properties or items.fields if present
                        // Note: Export saves nested columns in properties.properties, but some formats use fields
                        let nested_fields_obj = items_obj
                            .get("properties")
                            .and_then(|v| v.as_object())
                            .or_else(|| items_obj.get("fields").and_then(|v| v.as_object()));

                        if let Some(fields_obj) = nested_fields_obj {
                            for (nested_field_name, nested_field_data) in fields_obj {
                                if let Some(nested_field_obj) = nested_field_data.as_object() {
                                    let nested_field_type = nested_field_obj
                                        .get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("STRING");

                                    // Recursively parse nested fields with array prefix
                                    let nested_col_name =
                                        format!("{}.{}", field_name, nested_field_name);
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
                                            columns.push(Column {
                                                name: nested_col_name,
                                                data_type: nested_field_type.to_uppercase(),
                                                nullable: !nested_field_obj
                                                    .get("required")
                                                    .and_then(|v| v.as_bool())
                                                    .unwrap_or(false),
                                                primary_key: false,
                                                secondary_key: false,
                                                composite_key: None,
                                                foreign_key: None,
                                                constraints: Vec::new(),
                                                description: nested_field_obj
                                                    .get("description")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("")
                                                    .to_string(),
                                                errors: Vec::new(),
                                                quality: Vec::new(),
                                                enum_values: Vec::new(),
                                                column_order: 0,
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        return Ok(columns);
                    } else if let Some(item_type) = items_obj.get("type").and_then(|v| v.as_str()) {
                        // Array of simple type
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: format!("ARRAY<{}>", normalize_data_type(item_type)),
                            nullable: !field_data
                                .get("required")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            primary_key: false,
                            secondary_key: false,
                            composite_key: None,
                            foreign_key: None,
                            constraints: Vec::new(),
                            description: field_data
                                .get("description")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            errors: Vec::new(),
                            quality: Vec::new(),
                            enum_values: Vec::new(),
                            column_order: 0,
                        });
                        return Ok(columns);
                    }
                } else if let Some(item_type_str) = items_val.as_str() {
                    // Array of simple type (string)
                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: format!("ARRAY<{}>", normalize_data_type(item_type_str)),
                        nullable: !field_data
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description: field_data
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        errors: Vec::new(),
                        quality: Vec::new(),
                        enum_values: Vec::new(),
                        column_order: 0,
                    });
                    return Ok(columns);
                }
            }
            // Array without items - default to ARRAY<STRING>
            columns.push(Column {
                name: field_name.to_string(),
                data_type: "ARRAY<STRING>".to_string(),
                nullable: !field_data
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                primary_key: false,
                secondary_key: false,
                composite_key: None,
                foreign_key: None,
                constraints: Vec::new(),
                description: field_data
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                errors: Vec::new(),
                quality: Vec::new(),
                enum_values: Vec::new(),
                column_order: 0,
            });
            return Ok(columns);
        }

        // Check if this is a nested object with fields or properties
        // Note: Export saves nested columns in properties.properties, but some formats use fields
        let nested_fields_obj = field_data
            .get("properties")
            .and_then(|v| v.as_object())
            .or_else(|| field_data.get("fields").and_then(|v| v.as_object()));

        if field_type == "OBJECT"
            && let Some(fields_obj) = nested_fields_obj
        {
            // Inline nested object - create parent column as OBJECT and extract nested fields

            // Create parent column
            columns.push(Column {
                name: field_name.to_string(),
                data_type: "OBJECT".to_string(),
                nullable: !field_data
                    .get("required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                primary_key: false,
                secondary_key: false,
                composite_key: None,
                foreign_key: None,
                constraints: Vec::new(),
                description: field_data
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                errors: Vec::new(),
                quality: Vec::new(),
                enum_values: Vec::new(),
                column_order: 0,
            });

            // Extract nested fields recursively
            for (nested_field_name, nested_field_data) in fields_obj {
                if let Some(nested_field_obj) = nested_field_data.as_object() {
                    let nested_field_type = nested_field_obj
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("STRING");

                    // Recursively parse nested fields
                    let nested_col_name = format!("{}.{}", field_name, nested_field_name);
                    match self.parse_odcl_v3_property(&nested_col_name, nested_field_obj, data) {
                        Ok(mut nested_cols) => {
                            // If nested field is itself an OBJECT/STRUCT, it will return multiple columns
                            // Otherwise, it returns a single column
                            columns.append(&mut nested_cols);
                        }
                        Err(_) => {
                            // Fallback: create simple nested column
                            columns.push(Column {
                                name: nested_col_name,
                                data_type: nested_field_type.to_uppercase(),
                                nullable: !nested_field_obj
                                    .get("required")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false),
                                primary_key: false,
                                secondary_key: false,
                                composite_key: None,
                                foreign_key: None,
                                constraints: Vec::new(),
                                description: nested_field_obj
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                errors: Vec::new(),
                                quality: Vec::new(),
                                enum_values: Vec::new(),
                                column_order: 0,
                            });
                        }
                    }
                }
            }

            return Ok(columns);
        }

        // Regular field
        let required = field_data
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let description = field_data
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Extract column-level quality rules
        let mut column_quality_rules = Vec::new();
        if let Some(quality_val) = field_data.get("quality") {
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

        // If required=true (nullable=false), add a "not_null" quality rule if not already present
        if required {
            let has_not_null = column_quality_rules.iter().any(|rule| {
                rule.get("type")
                    .and_then(|v| v.as_str())
                    .map(|s| {
                        s.to_lowercase().contains("not_null")
                            || s.to_lowercase().contains("notnull")
                    })
                    .unwrap_or(false)
            });
            if !has_not_null {
                let mut not_null_rule = HashMap::new();
                not_null_rule.insert("type".to_string(), serde_json::json!("not_null"));
                not_null_rule.insert(
                    "description".to_string(),
                    serde_json::json!("Column must not be null"),
                );
                column_quality_rules.push(not_null_rule);
            }
        }

        columns.push(Column {
            name: field_name.to_string(),
            data_type: field_type,
            nullable: !required,
            primary_key: field_data
                .get("primaryKey")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            secondary_key: false,
            composite_key: None,
            foreign_key: self.parse_foreign_key_from_data_contract(field_data),
            constraints: Vec::new(),
            description,
            errors: Vec::new(),
            quality: column_quality_rules,
            enum_values: Vec::new(),
            column_order: 0,
        });

        Ok(columns)
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
    fn parse_struct_type_from_string(
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
        let _is_array = type_str_upper.starts_with("ARRAY<");
        let struct_start = type_str_upper.find("STRUCT<");

        if let Some(start_pos) = struct_start {
            // Extract STRUCT content
            let struct_content = &normalized_type[start_pos + 7..]; // Skip "STRUCT<"

            // Find matching closing bracket, handling nested STRUCTs
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
            for (nested_name, nested_type) in fields {
                let nested_type_upper = nested_type.to_uppercase();
                let nested_col_name = format!("{}.{}", field_name, nested_name);

                // Check if this field is itself a STRUCT or ARRAY<STRUCT>
                if nested_type_upper.starts_with("STRUCT<") {
                    // Recursively parse nested STRUCT by creating a synthetic field_data
                    // and calling parse_struct_type_from_string recursively
                    let nested_struct_type_str = if nested_type_upper.starts_with("ARRAY<STRUCT<") {
                        // Handle ARRAY<STRUCT<...>>
                        nested_type.clone()
                    } else {
                        // Handle STRUCT<...>
                        nested_type.clone()
                    };

                    // Recursively parse the nested STRUCT
                    match self.parse_struct_type_from_string(
                        &nested_col_name,
                        &nested_struct_type_str,
                        field_data,
                    ) {
                        Ok(nested_cols) => {
                            // Add all recursively parsed columns
                            columns.extend(nested_cols);
                        }
                        Err(_) => {
                            // If recursive parsing fails, add the parent column with STRUCT type
                            columns.push(Column {
                                name: nested_col_name,
                                data_type: normalize_data_type(&nested_type),
                                nullable: !field_data
                                    .get("required")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false),
                                primary_key: false,
                                secondary_key: false,
                                composite_key: None,
                                foreign_key: None,
                                constraints: Vec::new(),
                                description: field_data
                                    .get("description")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string(),
                                errors: Vec::new(),
                                quality: Vec::new(),
                                enum_values: Vec::new(),
                                column_order: 0,
                            });
                        }
                    }
                } else {
                    // Simple nested field (not a STRUCT)
                    columns.push(Column {
                        name: nested_col_name,
                        data_type: normalize_data_type(&nested_type),
                        nullable: !field_data
                            .get("required")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false),
                        primary_key: false,
                        secondary_key: false,
                        composite_key: None,
                        foreign_key: None,
                        constraints: Vec::new(),
                        description: field_data
                            .get("description")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        errors: Vec::new(),
                        quality: Vec::new(),
                        enum_values: Vec::new(),
                        column_order: 0,
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

    /// Parse a single field definition (e.g., "ID: STRING" or "ALERTOPERATION: STRUCT<...>").
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

/// Parser error structure for detailed error reporting.
#[derive(Debug, Clone)]
pub struct ParserError {
    pub error_type: String,
    pub field: String,
    pub message: String,
}

/// Convert YAML Value to JSON Value for easier manipulation.
fn yaml_to_json_value(yaml: &serde_yaml::Value) -> Result<JsonValue> {
    // Convert YAML to JSON via serialization
    let json_str = serde_json::to_string(yaml).context("Failed to convert YAML to JSON")?;
    serde_json::from_str(&json_str).context("Failed to parse JSON")
}

/// Convert JSON Value to serde_json::Value for storage in HashMap.
fn json_value_to_serde_value(value: &JsonValue) -> serde_json::Value {
    value.clone()
}

/// Normalize data type to uppercase, preserving STRUCT<...>, ARRAY<...>, MAP<...> format.
fn normalize_data_type(data_type: &str) -> String {
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

// Helper functions for enum parsing
fn parse_medallion_layer(s: &str) -> Result<MedallionLayer> {
    match s.to_uppercase().as_str() {
        "BRONZE" => Ok(MedallionLayer::Bronze),
        "SILVER" => Ok(MedallionLayer::Silver),
        "GOLD" => Ok(MedallionLayer::Gold),
        "OPERATIONAL" => Ok(MedallionLayer::Operational),
        _ => Err(anyhow::anyhow!("Unknown medallion layer: {}", s)),
    }
}

fn parse_scd_pattern(s: &str) -> Result<SCDPattern> {
    match s.to_uppercase().as_str() {
        "TYPE_1" | "TYPE1" => Ok(SCDPattern::Type1),
        "TYPE_2" | "TYPE2" => Ok(SCDPattern::Type2),
        _ => Err(anyhow::anyhow!("Unknown SCD pattern: {}", s)),
    }
}

fn parse_data_vault_classification(s: &str) -> Result<DataVaultClassification> {
    match s.to_uppercase().as_str() {
        "HUB" => Ok(DataVaultClassification::Hub),
        "LINK" => Ok(DataVaultClassification::Link),
        "SATELLITE" | "SAT" => Ok(DataVaultClassification::Satellite),
        _ => Err(anyhow::anyhow!("Unknown Data Vault classification: {}", s)),
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
