//! ODCL (Open Data Contract Language) parser service for parsing legacy ODCL YAML files.
//!
//! This service parses legacy ODCL (Data Contract Specification) YAML files and converts
//! them to Table models. ODCL is the predecessor to ODCS (Open Data Contract Standard).
//!
//! Supports multiple legacy formats:
//! - Data Contract Specification format (dataContractSpecification, models, definitions)
//! - Simple ODCL format (name, columns)
//!
//! For ODCS v3.1.0/v3.0.x format, use the ODCSImporter instead.

use super::odcs_shared::{
    ParserError, column_to_column_data, expand_nested_column, extract_catalog_schema,
    extract_quality_from_obj, extract_shared_domains, json_value_to_serde_value,
    normalize_data_type, parse_data_vault_classification, parse_foreign_key,
    parse_foreign_key_from_data_contract, parse_medallion_layer, parse_scd_pattern,
    parse_struct_fields_from_string, resolve_ref, yaml_to_json_value,
};
use super::{ImportError, ImportResult, TableData};
use crate::models::enums::{DataVaultClassification, DatabaseType, MedallionLayer, SCDPattern};
use crate::models::{Column, PropertyRelationship, Table, Tag};
use anyhow::{Context, Result};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::str::FromStr;
use tracing::info;

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

/// ODCL parser service for parsing legacy Open Data Contract Language YAML files.
/// Handles Data Contract Specification format and simple ODCL format.
///
/// For ODCS v3.1.0 format, use ODCSImporter instead.
pub struct ODCLImporter {
    /// Current YAML data for $ref resolution
    current_yaml_data: Option<serde_yaml::Value>,
}

impl ODCLImporter {
    /// Create a new ODCL parser instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::odcl::ODCLImporter;
    ///
    /// let mut importer = ODCLImporter::new();
    /// ```
    pub fn new() -> Self {
        Self {
            current_yaml_data: None,
        }
    }

    /// Import ODCL YAML content and create Table (SDK interface).
    ///
    /// Supports Data Contract Specification format and simple ODCL format.
    ///
    /// # Arguments
    ///
    /// * `yaml_content` - ODCL YAML content as a string
    ///
    /// # Returns
    ///
    /// An `ImportResult` containing the extracted table and any parse errors.
    ///
    /// # Example
    ///
    /// ```rust
    /// use data_modelling_sdk::import::odcl::ODCLImporter;
    ///
    /// let mut importer = ODCLImporter::new();
    /// let yaml = r#"
    /// dataContractSpecification: 0.9.3
    /// id: urn:datacontract:example
    /// models:
    ///   users:
    ///     fields:
    ///       id:
    ///         type: bigint
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
                    columns: table.columns.iter().map(column_to_column_data).collect(),
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

    /// Parse ODCL YAML content and create Table (public method for native app use).
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

    /// Parse ODCL YAML content and create Table (internal method).
    ///
    /// Supports Data Contract Specification format and simple ODCL format.
    ///
    /// # Returns
    ///
    /// Returns a tuple of (Table, list of errors/warnings).
    /// Errors list is empty if parsing is successful.
    fn parse(&mut self, yaml_content: &str) -> Result<(Table, Vec<ParserError>)> {
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
        if self.is_data_contract_format(&json_data) {
            return self.parse_data_contract(&json_data);
        }

        // Fall back to simple ODCL format
        self.parse_simple_odcl(&json_data)
    }

    /// Check if this importer can handle the given YAML content.
    ///
    /// Returns true if the content is in ODCL format (Data Contract Specification
    /// or simple ODCL format), false if it's in ODCS v3.x format.
    pub fn can_handle(&self, yaml_content: &str) -> bool {
        let data: serde_yaml::Value = match serde_yaml::from_str(yaml_content) {
            Ok(d) => d,
            Err(_) => return false,
        };

        let json_data = match yaml_to_json_value(&data) {
            Ok(j) => j,
            Err(_) => return false,
        };

        // Check if it's ODCS v3.x format (should use ODCSImporter instead)
        if self.is_odcs_v3_format(&json_data) {
            return false;
        }

        // Check if it's Data Contract Specification format
        if self.is_data_contract_format(&json_data) {
            return true;
        }

        // Check if it's simple ODCL format (has name and columns)
        if let Some(obj) = json_data.as_object() {
            let has_name = obj.contains_key("name");
            let has_columns = obj.get("columns").and_then(|v| v.as_array()).is_some();
            return has_name && has_columns;
        }

        false
    }

    /// Check if YAML is in ODCS v3.x format.
    fn is_odcs_v3_format(&self, data: &JsonValue) -> bool {
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

        let foreign_key = col_data.get("foreign_key").and_then(parse_foreign_key);

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

        // Check for quality in metadata (ODCL format)
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
        let mut odcl_metadata = HashMap::new();

        // Extract info section and nest it properly
        if let Some(info_val) = data.get("info") {
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
        let (catalog_name, schema_name) = extract_catalog_schema(data);

        // Extract sharedDomains from customProperties
        let shared_domains = extract_shared_domains(data);

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
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: "OBJECT".to_string(),
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

                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: def_type,
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
                columns.push(Column {
                    name: field_name.to_string(),
                    data_type: "OBJECT".to_string(),
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

                    // Add parent column
                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: parent_data_type,
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
                        columns.push(Column {
                            name: field_name.to_string(),
                            data_type: "ARRAY<OBJECT>".to_string(),
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
                        let properties_obj =
                            items_obj.get("properties").and_then(|v| v.as_object());
                        let fields_obj = items_obj.get("fields").and_then(|v| v.as_object());

                        if let Some(fields_map) = properties_obj.or(fields_obj) {
                            for (nested_field_name, nested_field_data) in fields_map {
                                if let Some(nested_field_obj) = nested_field_data.as_object() {
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
                    columns.push(Column {
                        name: field_name.to_string(),
                        data_type: format!("ARRAY<{}>", normalize_data_type(item_type_str)),
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
            columns.push(Column {
                name: field_name.to_string(),
                data_type: "ARRAY<STRING>".to_string(),
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
        let nested_fields_obj = field_data
            .get("properties")
            .and_then(|v| v.as_object())
            .or_else(|| field_data.get("fields").and_then(|v| v.as_object()));

        if field_type == "OBJECT" && nested_fields_obj.is_some() {
            // Inline nested object - create parent column as OBJECT and extract nested fields
            columns.push(Column {
                name: field_name.to_string(),
                data_type: "OBJECT".to_string(),
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

            // Extract nested fields recursively
            if let Some(fields_obj) = nested_fields_obj {
                for (nested_field_name, nested_field_data) in fields_obj {
                    if let Some(nested_field_obj) = nested_field_data.as_object() {
                        let nested_field_type = nested_field_obj
                            .get("logicalType")
                            .and_then(|v| v.as_str())
                            .or_else(|| nested_field_obj.get("type").and_then(|v| v.as_str()))
                            .unwrap_or("STRING");

                        // Recursively parse nested fields
                        let nested_col_name = format!("{}.{}", field_name, nested_field_name);
                        match self.parse_data_contract_field(
                            &nested_col_name,
                            nested_field_obj,
                            data,
                            errors,
                        ) {
                            Ok(mut nested_cols) => {
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
            }

            return Ok(columns);
        }

        // Regular field (no $ref or $ref not found)
        let ref_path = field_data
            .get("$ref")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let required = field_data
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let field_description = if description.is_empty() {
            field_data
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        } else {
            description
        };

        // Extract column-level quality rules if not already extracted
        let mut column_quality_rules = quality_rules;
        if column_quality_rules.is_empty()
            && let Some(quality_val) = field_data.get("quality")
        {
            if let Some(arr) = quality_val.as_array() {
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
                let mut rule = HashMap::new();
                for (key, value) in obj {
                    rule.insert(key.clone(), json_value_to_serde_value(value));
                }
                column_quality_rules.push(rule);
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
            foreign_key: parse_foreign_key_from_data_contract(field_data),
            description: field_description,
            quality: column_quality_rules,
            relationships: ref_to_relationships(&ref_path),
            ..Default::default()
        });

        Ok(columns)
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

    /// Extract table UUID from ODCL `id` field or fallback.
    fn extract_table_uuid(&self, data: &JsonValue) -> uuid::Uuid {
        // First check the top-level `id` field
        if let Some(id_val) = data.get("id")
            && let Some(id_str) = id_val.as_str()
            && let Ok(uuid) = uuid::Uuid::parse_str(id_str)
        {
            tracing::debug!(
                "[ODCLImporter] Extracted UUID from top-level 'id' field: {}",
                uuid
            );
            return uuid;
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
                            "[ODCLImporter] Extracted UUID from customProperties.tableUuid: {}",
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
                "[ODCLImporter] Extracted UUID from odcl_metadata.tableUuid: {}",
                uuid
            );
            return uuid;
        }

        // Generate deterministic UUID v5 if not found (based on table name)
        let table_name = data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let new_uuid = crate::models::table::Table::generate_id(table_name, None, None, None);
        tracing::warn!(
            "[ODCLImporter] No UUID found for table '{}', generating deterministic UUID: {}",
            table_name,
            new_uuid
        );
        new_uuid
    }

    /// Parse STRUCT type from string and create nested columns.
    #[allow(clippy::only_used_in_recursion)]
    fn parse_struct_type_from_string(
        &self,
        field_name: &str,
        type_str: &str,
        field_data: &serde_json::Map<String, JsonValue>,
    ) -> Result<Vec<Column>> {
        let mut columns = Vec::new();

        // Normalize whitespace
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
            let struct_content_start = start_pos + 7; // Skip "STRUCT<"
            let struct_content = &normalized_type[struct_content_start..];

            // Find matching closing bracket for STRUCT<
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

            let struct_fields_str = if let Some(end) = end_pos {
                &struct_content[..end]
            } else {
                struct_content.trim_end_matches('>').trim()
            };

            // Parse fields: "ID: STRING, NAME: STRING, ..."
            let fields = parse_struct_fields_from_string(struct_fields_str)?;

            // Create nested columns
            for (nested_name, nested_type) in fields {
                let nested_type_upper = nested_type.to_uppercase();
                let nested_col_name = if is_array {
                    format!("{}.[].{}", field_name, nested_name)
                } else {
                    format!("{}.{}", field_name, nested_name)
                };

                let is_nested_struct = nested_type_upper.starts_with("STRUCT<");
                let is_nested_array_struct = nested_type_upper.starts_with("ARRAY<STRUCT<");

                if is_nested_struct || is_nested_array_struct {
                    // Recursively parse nested STRUCT or ARRAY<STRUCT>
                    match self.parse_struct_type_from_string(
                        &nested_col_name,
                        &nested_type,
                        field_data,
                    ) {
                        Ok(nested_cols) => {
                            columns.extend(nested_cols);
                        }
                        Err(_) => {
                            let fallback_data_type = if is_nested_array_struct {
                                "ARRAY<STRUCT<...>>".to_string()
                            } else {
                                "STRUCT<...>".to_string()
                            };
                            columns.push(Column {
                                name: nested_col_name,
                                data_type: fallback_data_type,
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
                    columns.push(Column {
                        name: nested_col_name,
                        data_type: normalize_data_type(&nested_type),
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
                    // Simple nested field
                    columns.push(Column {
                        name: nested_col_name,
                        data_type: normalize_data_type(&nested_type),
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
}

impl Default for ODCLImporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_odcl_table() {
        let mut parser = ODCLImporter::new();
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
        let mut parser = ODCLImporter::new();
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
    fn test_parse_data_contract_format() {
        let mut parser = ODCLImporter::new();
        let odcl_yaml = r#"
dataContractSpecification: 0.9.3
id: urn:datacontract:example
models:
  users:
    fields:
      id:
        type: bigint
        description: User ID
      name:
        type: string
        description: User name
"#;

        let (table, errors) = parser.parse(odcl_yaml).unwrap();
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_can_handle_odcl_format() {
        let parser = ODCLImporter::new();

        // Data Contract format should be handled
        let data_contract = r#"
dataContractSpecification: 0.9.3
id: test
models:
  users:
    fields:
      id:
        type: int
"#;
        assert!(parser.can_handle(data_contract));

        // Simple ODCL format should be handled
        let simple_odcl = r#"
name: users
columns:
  - name: id
    data_type: INT
"#;
        assert!(parser.can_handle(simple_odcl));

        // ODCS v3.x format should NOT be handled
        let odcs_v3 = r#"
apiVersion: v3.1.0
kind: DataContract
id: test-uuid
version: 1.0.0
name: users
schema:
  - name: users
    properties:
      - name: id
        logicalType: integer
"#;
        assert!(!parser.can_handle(odcs_v3));
    }
}
