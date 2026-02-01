//! WASM bindings for data modelling SDK
//!
//! This crate provides WebAssembly bindings for the data modelling SDK,
//! exposing import/export functions for browser applications.

use data_modelling_core::export::ExportError;
use data_modelling_core::import::{ImportError, ImportResult};
use data_modelling_core::models::DataModel;
use js_sys;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_yaml;
use uuid;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures;

/// Structured error type for WASM bindings.
/// Provides detailed error information that can be parsed by JavaScript consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmError {
    /// Error category (e.g., "ImportError", "ExportError", "ValidationError")
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
    /// Optional error code for programmatic handling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl WasmError {
    /// Create a new WasmError with the given type and message
    fn new(error_type: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error_type: error_type.into(),
            message: message.into(),
            code: None,
            details: None,
        }
    }

    /// Create a WasmError with a specific error code
    fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Convert to JsValue for returning to JavaScript
    fn to_js_value(&self) -> JsValue {
        // Serialize to JSON string for structured error handling in JS
        match serde_json::to_string(self) {
            Ok(json) => JsValue::from_str(&json),
            // Fallback to simple message if serialization fails
            Err(_) => JsValue::from_str(&self.message),
        }
    }
}

/// Convert ImportError to structured JsValue for JavaScript error handling
fn import_error_to_js(err: ImportError) -> JsValue {
    WasmError::new("ImportError", err.to_string())
        .with_code("IMPORT_FAILED")
        .to_js_value()
}

/// Convert ExportError to structured JsValue for JavaScript error handling
fn export_error_to_js(err: ExportError) -> JsValue {
    WasmError::new("ExportError", err.to_string())
        .with_code("EXPORT_FAILED")
        .to_js_value()
}

/// Create a serialization error
fn serialization_error(err: impl std::fmt::Display) -> JsValue {
    WasmError::new(
        "SerializationError",
        format!("Serialization error: {}", err),
    )
    .with_code("SERIALIZATION_FAILED")
    .to_js_value()
}

/// Create a deserialization error
fn deserialization_error(err: impl std::fmt::Display) -> JsValue {
    WasmError::new(
        "DeserializationError",
        format!("Deserialization error: {}", err),
    )
    .with_code("DESERIALIZATION_FAILED")
    .to_js_value()
}

/// Create a parse error
fn parse_error(err: impl std::fmt::Display) -> JsValue {
    WasmError::new("ParseError", format!("Parse error: {}", err))
        .with_code("PARSE_FAILED")
        .to_js_value()
}

/// Create a validation error
fn validation_error(err: impl std::fmt::Display) -> JsValue {
    WasmError::new("ValidationError", err.to_string())
        .with_code("VALIDATION_FAILED")
        .to_js_value()
}

/// Create an invalid input error
fn invalid_input_error(field: &str, err: impl std::fmt::Display) -> JsValue {
    WasmError::new("InvalidInputError", format!("Invalid {}: {}", field, err))
        .with_code("INVALID_INPUT")
        .to_js_value()
}

/// Create a conversion error
fn conversion_error(err: impl std::fmt::Display) -> JsValue {
    WasmError::new("ConversionError", format!("Conversion error: {}", err))
        .with_code("CONVERSION_FAILED")
        .to_js_value()
}

/// Create a storage error
fn storage_error(err: impl std::fmt::Display) -> JsValue {
    WasmError::new("StorageError", format!("Storage error: {}", err))
        .with_code("STORAGE_FAILED")
        .to_js_value()
}

/// Serialize ImportResult to JSON string
fn serialize_import_result(result: &ImportResult) -> Result<String, JsValue> {
    serde_json::to_string(result).map_err(serialization_error)
}

/// Flatten STRUCT columns in ImportResult into nested columns with dot notation
///
/// This processes each table's columns and expands STRUCT types into individual
/// columns with parent.child naming:
/// - STRUCT<field1: TYPE1, field2: TYPE2> → parent.field1, parent.field2
/// - ARRAY<STRUCT<...>> → parent.[].field1, parent.[].field2
/// - MAP types are kept as-is (keys are dynamic)
fn flatten_struct_columns(result: ImportResult) -> ImportResult {
    use data_modelling_core::import::{ColumnData, ODCSImporter, TableData};

    let importer = ODCSImporter::new();

    let tables = result
        .tables
        .into_iter()
        .map(|table_data| {
            let mut all_columns = Vec::new();

            for col_data in table_data.columns {
                let data_type_upper = col_data.data_type.to_uppercase();
                let is_map = data_type_upper.starts_with("MAP<");

                // Skip parsing for MAP types - keys are dynamic
                if is_map {
                    all_columns.push(col_data);
                    continue;
                }

                // For STRUCT or ARRAY<STRUCT> types, try to parse and create nested columns
                let is_struct = data_type_upper.contains("STRUCT<");
                if is_struct {
                    let field_data = serde_json::Map::new();
                    if let Ok(nested_cols) = importer.parse_struct_type_from_string(
                        &col_data.name,
                        &col_data.data_type,
                        &field_data,
                    ) {
                        if !nested_cols.is_empty() {
                            // Add parent column with simplified type
                            let parent_data_type =
                                if col_data.data_type.to_uppercase().starts_with("ARRAY<") {
                                    "ARRAY<STRUCT<...>>".to_string()
                                } else {
                                    "STRUCT<...>".to_string()
                                };

                            all_columns.push(ColumnData {
                                name: col_data.name.clone(),
                                data_type: parent_data_type,
                                physical_type: col_data.physical_type.clone(),
                                nullable: col_data.nullable,
                                primary_key: col_data.primary_key,
                                description: col_data.description.clone(),
                                quality: col_data.quality.clone(),
                                relationships: col_data.relationships.clone(),
                                enum_values: col_data.enum_values.clone(),
                                ..Default::default()
                            });

                            // Add nested columns converted from Column to ColumnData
                            for nested_col in nested_cols {
                                all_columns.push(
                                    data_modelling_core::import::odcs_shared::column_to_column_data(
                                        &nested_col,
                                    ),
                                );
                            }
                            continue;
                        }
                    }
                }

                // Regular column or STRUCT parsing failed - add as-is
                all_columns.push(col_data);
            }

            TableData {
                table_index: table_data.table_index,
                id: table_data.id.clone(),
                name: table_data.name.clone(),
                columns: all_columns,
                api_version: table_data.api_version.clone(),
                version: table_data.version.clone(),
                status: table_data.status.clone(),
                kind: table_data.kind.clone(),
                domain: table_data.domain.clone(),
                data_product: table_data.data_product.clone(),
                tenant: table_data.tenant.clone(),
                description: table_data.description.clone(),
                servers: table_data.servers.clone(),
                team: table_data.team.clone(),
                support: table_data.support.clone(),
                roles: table_data.roles.clone(),
                sla_properties: table_data.sla_properties.clone(),
                quality: table_data.quality.clone(),
                price: table_data.price.clone(),
                tags: table_data.tags.clone(),
                custom_properties: table_data.custom_properties.clone(),
                authoritative_definitions: table_data.authoritative_definitions.clone(),
                contract_created_ts: table_data.contract_created_ts.clone(),
                odcs_metadata: table_data.odcs_metadata.clone(),
                physical_name: table_data.physical_name.clone(),
                physical_type: table_data.physical_type.clone(),
                business_name: table_data.business_name.clone(),
                data_granularity_description: table_data.data_granularity_description.clone(),
            }
        })
        .collect();

    ImportResult {
        tables,
        tables_requiring_name: result.tables_requiring_name,
        errors: result.errors,
        ai_suggestions: result.ai_suggestions,
    }
}

/// Deserialize workspace structure from JSON string
fn deserialize_workspace(json: &str) -> Result<DataModel, JsValue> {
    serde_json::from_str(json).map_err(deserialization_error)
}

/// Parse ODCS YAML content and return a structured workspace representation.
///
/// # Arguments
///
/// * `yaml_content` - ODCS YAML content as a string
///
/// # Returns
///
/// JSON string containing ImportResult object, or JsValue error
#[wasm_bindgen]
pub fn parse_odcs_yaml(yaml_content: &str) -> Result<String, JsValue> {
    let mut importer = data_modelling_core::import::ODCSImporter::new();
    match importer.import(yaml_content) {
        Ok(result) => {
            let flattened = flatten_struct_columns(result);
            serialize_import_result(&flattened)
        }
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Import data model from legacy ODCL (Open Data Contract Language) YAML format.
///
/// This function parses legacy ODCL formats including:
/// - Data Contract Specification format (dataContractSpecification, models, definitions)
/// - Simple ODCL format (name, columns)
///
/// For ODCS v3.1.0/v3.0.x format, use `parse_odcs_yaml` instead.
///
/// # Arguments
///
/// * `yaml_content` - ODCL YAML content as a string
///
/// # Returns
///
/// JSON string containing ImportResult object, or JsValue error
#[wasm_bindgen]
pub fn parse_odcl_yaml(yaml_content: &str) -> Result<String, JsValue> {
    let mut importer = data_modelling_core::import::ODCLImporter::new();
    match importer.import(yaml_content) {
        Ok(result) => {
            let flattened = flatten_struct_columns(result);
            serialize_import_result(&flattened)
        }
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Check if the given YAML content is in legacy ODCL format.
///
/// Returns true if the content is in ODCL format (Data Contract Specification
/// or simple ODCL format), false if it's in ODCS v3.x format or invalid.
///
/// # Arguments
///
/// * `yaml_content` - YAML content to check
///
/// # Returns
///
/// Boolean indicating if the content is ODCL format
#[wasm_bindgen]
pub fn is_odcl_format(yaml_content: &str) -> bool {
    let importer = data_modelling_core::import::ODCLImporter::new();
    importer.can_handle(yaml_content)
}

/// Export a workspace structure to ODCS YAML format.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
///
/// # Returns
///
/// ODCS YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_to_odcs_yaml(workspace_json: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;

    // Export all tables as separate YAML documents, joined with ---\n
    let exports =
        data_modelling_core::export::ODCSExporter::export_model(&model, None, "odcs_v3_1_0");

    // Combine all YAML documents into a single multi-document string
    let yaml_docs: Vec<String> = exports.values().cloned().collect();
    Ok(yaml_docs.join("\n---\n"))
}

/// Import data model from SQL CREATE TABLE statements.
///
/// # Arguments
///
/// * `sql_content` - SQL CREATE TABLE statements
/// * `dialect` - SQL dialect ("postgresql", "mysql", "sqlserver", "databricks")
///
/// # Returns
///
/// JSON string containing ImportResult object, or JsValue error
#[wasm_bindgen]
pub fn import_from_sql(sql_content: &str, dialect: &str) -> Result<String, JsValue> {
    let importer = data_modelling_core::import::SQLImporter::new(dialect);
    match importer.parse(sql_content) {
        Ok(result) => {
            // Flatten STRUCT columns into nested columns with dot notation
            let flattened = flatten_struct_columns(result);
            serialize_import_result(&flattened)
        }
        Err(err) => Err(parse_error(err)),
    }
}

/// Import data model from AVRO schema.
///
/// # Arguments
///
/// * `avro_content` - AVRO schema JSON as a string
///
/// # Returns
///
/// JSON string containing ImportResult object, or JsValue error
#[wasm_bindgen]
pub fn import_from_avro(avro_content: &str) -> Result<String, JsValue> {
    let importer = data_modelling_core::import::AvroImporter::new();
    match importer.import(avro_content) {
        Ok(result) => {
            let flattened = flatten_struct_columns(result);
            serialize_import_result(&flattened)
        }
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Import data model from JSON Schema definition.
///
/// # Arguments
///
/// * `json_schema_content` - JSON Schema definition as a string
///
/// # Returns
///
/// JSON string containing ImportResult object, or JsValue error
#[wasm_bindgen]
pub fn import_from_json_schema(json_schema_content: &str) -> Result<String, JsValue> {
    let importer = data_modelling_core::import::JSONSchemaImporter::new();
    match importer.import(json_schema_content) {
        Ok(result) => {
            let flattened = flatten_struct_columns(result);
            serialize_import_result(&flattened)
        }
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Import data model from Protobuf schema.
///
/// # Arguments
///
/// * `protobuf_content` - Protobuf schema text
///
/// # Returns
///
/// JSON string containing ImportResult object, or JsValue error
#[wasm_bindgen]
pub fn import_from_protobuf(protobuf_content: &str) -> Result<String, JsValue> {
    let importer = data_modelling_core::import::ProtobufImporter::new();
    match importer.import(protobuf_content) {
        Ok(result) => {
            let flattened = flatten_struct_columns(result);
            serialize_import_result(&flattened)
        }
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Export a data model to SQL CREATE TABLE statements.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `dialect` - SQL dialect ("postgresql", "mysql", "sqlserver", "databricks")
///
/// # Returns
///
/// SQL CREATE TABLE statements, or JsValue error
#[wasm_bindgen]
pub fn export_to_sql(workspace_json: &str, dialect: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let exporter = data_modelling_core::export::SQLExporter;
    match exporter.export(&model.tables, Some(dialect)) {
        Ok(result) => Ok(result.content),
        Err(err) => Err(export_error_to_js(err)),
    }
}

/// Export a data model to AVRO schema.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
///
/// # Returns
///
/// AVRO schema JSON string, or JsValue error
#[wasm_bindgen]
pub fn export_to_avro(workspace_json: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let exporter = data_modelling_core::export::AvroExporter;
    match exporter.export(&model.tables) {
        Ok(result) => Ok(result.content),
        Err(err) => Err(export_error_to_js(err)),
    }
}

/// Export a data model to JSON Schema definition.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
///
/// # Returns
///
/// JSON Schema definition string, or JsValue error
#[wasm_bindgen]
pub fn export_to_json_schema(workspace_json: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let exporter = data_modelling_core::export::JSONSchemaExporter;
    match exporter.export(&model.tables) {
        Ok(result) => Ok(result.content),
        Err(err) => Err(export_error_to_js(err)),
    }
}

/// Export a data model to Protobuf schema.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
///
/// # Returns
///
/// Protobuf schema text, or JsValue error
#[wasm_bindgen]
pub fn export_to_protobuf(workspace_json: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let exporter = data_modelling_core::export::ProtobufExporter;
    match exporter.export(&model.tables) {
        Ok(result) => Ok(result.content),
        Err(err) => Err(export_error_to_js(err)),
    }
}

/// Import CADS YAML content and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - CADS YAML content as a string
///
/// # Returns
///
/// JSON string containing CADS asset, or JsValue error
#[wasm_bindgen]
pub fn import_from_cads(yaml_content: &str) -> Result<String, JsValue> {
    let importer = data_modelling_core::import::CADSImporter::new();
    match importer.import(yaml_content) {
        Ok(asset) => serde_json::to_string(&asset).map_err(serialization_error),
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Export a CADS asset to YAML format.
///
/// # Arguments
///
/// * `asset_json` - JSON string containing CADS asset
///
/// # Returns
///
/// CADS YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_to_cads(asset_json: &str) -> Result<String, JsValue> {
    let asset: data_modelling_core::models::cads::CADSAsset =
        serde_json::from_str(asset_json).map_err(deserialization_error)?;
    let exporter = data_modelling_core::export::CADSExporter;
    match exporter.export(&asset) {
        Ok(yaml) => Ok(yaml),
        Err(err) => Err(export_error_to_js(err)),
    }
}

/// Import ODPS YAML content and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - ODPS YAML content as a string
///
/// # Returns
///
/// JSON string containing ODPS data product, or JsValue error
#[wasm_bindgen]
pub fn import_from_odps(yaml_content: &str) -> Result<String, JsValue> {
    let importer = data_modelling_core::import::ODPSImporter::new();
    match importer.import(yaml_content) {
        Ok(product) => serde_json::to_string(&product).map_err(serialization_error),
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Export an ODPS data product to YAML format.
///
/// # Arguments
///
/// * `product_json` - JSON string containing ODPS data product
///
/// # Returns
///
/// ODPS YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_to_odps(product_json: &str) -> Result<String, JsValue> {
    let product: data_modelling_core::models::odps::ODPSDataProduct =
        serde_json::from_str(product_json).map_err(deserialization_error)?;
    let exporter = data_modelling_core::export::ODPSExporter;
    match exporter.export(&product) {
        Ok(yaml) => Ok(yaml),
        Err(err) => Err(export_error_to_js(err)),
    }
}

/// Validate ODPS YAML content against the ODPS JSON Schema.
///
/// # Arguments
///
/// * `yaml_content` - ODPS YAML content as a string
///
/// # Returns
///
/// Empty string on success, or error message string
#[cfg(feature = "odps-validation")]
#[wasm_bindgen]
pub fn validate_odps(yaml_content: &str) -> Result<(), JsValue> {
    use data_modelling_core::validation::schema::validate_odps_internal;
    validate_odps_internal(yaml_content).map_err(validation_error)
}

#[cfg(not(feature = "odps-validation"))]
#[wasm_bindgen]
pub fn validate_odps(_yaml_content: &str) -> Result<(), JsValue> {
    // Validation disabled - feature not enabled
    // Return success to maintain backward compatibility
    Ok(())
}

/// Create a new business domain.
///
/// # Arguments
///
/// * `name` - Domain name
///
/// # Returns
///
/// JSON string containing Domain, or JsValue error
#[wasm_bindgen]
pub fn create_domain(name: &str) -> Result<String, JsValue> {
    let domain = data_modelling_core::models::domain::Domain::new(name.to_string());
    serde_json::to_string(&domain).map_err(serialization_error)
}

/// Import Domain YAML content and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Domain YAML content as a string
///
/// # Returns
///
/// JSON string containing Domain, or JsValue error
#[wasm_bindgen]
pub fn import_from_domain(yaml_content: &str) -> Result<String, JsValue> {
    match data_modelling_core::models::domain::Domain::from_yaml(yaml_content) {
        Ok(domain) => serde_json::to_string(&domain).map_err(serialization_error),
        Err(e) => Err(parse_error(e)),
    }
}

/// Export a Domain to YAML format.
///
/// # Arguments
///
/// * `domain_json` - JSON string containing Domain
///
/// # Returns
///
/// Domain YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_to_domain(domain_json: &str) -> Result<String, JsValue> {
    let domain: data_modelling_core::models::domain::Domain =
        serde_json::from_str(domain_json).map_err(deserialization_error)?;
    domain.to_yaml().map_err(serialization_error)
}

/// Migrate DataFlow YAML to Domain schema format.
///
/// # Arguments
///
/// * `dataflow_yaml` - DataFlow YAML content as a string
/// * `domain_name` - Optional domain name (defaults to "MigratedDomain")
///
/// # Returns
///
/// JSON string containing Domain, or JsValue error
#[wasm_bindgen]
pub fn migrate_dataflow_to_domain(
    dataflow_yaml: &str,
    domain_name: Option<String>,
) -> Result<String, JsValue> {
    match data_modelling_core::convert::migrate_dataflow::migrate_dataflow_to_domain(
        dataflow_yaml,
        domain_name.as_deref(),
    ) {
        Ok(domain) => serde_json::to_string(&domain).map_err(serialization_error),
        Err(e) => Err(conversion_error(e)),
    }
}

/// Parse a tag string into a Tag enum.
///
/// # Arguments
///
/// * `tag_str` - Tag string (Simple, Pair, or List format)
///
/// # Returns
///
/// JSON string containing Tag, or JsValue error
#[wasm_bindgen]
pub fn parse_tag(tag_str: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::Tag;
    use std::str::FromStr;
    match Tag::from_str(tag_str) {
        Ok(tag) => serde_json::to_string(&tag).map_err(serialization_error),
        Err(_) => Err(parse_error("Invalid tag format")),
    }
}

/// Serialize a Tag enum to string format.
///
/// # Arguments
///
/// * `tag_json` - JSON string containing Tag
///
/// # Returns
///
/// Tag string (Simple, Pair, or List format), or JsValue error
#[wasm_bindgen]
pub fn serialize_tag(tag_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::Tag;
    let tag: Tag = serde_json::from_str(tag_json).map_err(deserialization_error)?;
    Ok(tag.to_string())
}

/// Convert any format to ODCS v3.1.0 YAML format.
///
/// # Arguments
///
/// * `input` - Format-specific content as a string
/// * `format` - Optional format identifier. If None, attempts auto-detection.
///   Supported formats: "sql", "json_schema", "avro", "protobuf", "odcl", "odcs", "cads", "odps", "domain"
///
/// # Returns
///
/// ODCS v3.1.0 YAML string, or JsValue error
#[wasm_bindgen]
pub fn convert_to_odcs(input: &str, format: Option<String>) -> Result<String, JsValue> {
    match data_modelling_core::convert::convert_to_odcs(input, format.as_deref()) {
        Ok(yaml) => Ok(yaml),
        Err(e) => Err(conversion_error(e)),
    }
}

/// Filter Data Flow nodes (tables) by owner.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `owner` - Owner name to filter by (case-sensitive exact match)
///
/// # Returns
///
/// JSON string containing array of matching tables, or JsValue error
#[wasm_bindgen]
pub fn filter_nodes_by_owner(workspace_json: &str, owner: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let filtered = model.filter_nodes_by_owner(owner);
    serde_json::to_string(&filtered).map_err(serialization_error)
}

/// Filter Data Flow relationships by owner.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `owner` - Owner name to filter by (case-sensitive exact match)
///
/// # Returns
///
/// JSON string containing array of matching relationships, or JsValue error
#[wasm_bindgen]
pub fn filter_relationships_by_owner(workspace_json: &str, owner: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let filtered = model.filter_relationships_by_owner(owner);
    serde_json::to_string(&filtered).map_err(serialization_error)
}

/// Filter Data Flow nodes (tables) by infrastructure type.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `infrastructure_type` - Infrastructure type string (e.g., "Kafka", "PostgreSQL")
///
/// # Returns
///
/// JSON string containing array of matching tables, or JsValue error
#[wasm_bindgen]
pub fn filter_nodes_by_infrastructure_type(
    workspace_json: &str,
    infrastructure_type: &str,
) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let infra_type: data_modelling_core::models::enums::InfrastructureType =
        serde_json::from_str(&format!("\"{}\"", infrastructure_type))
            .map_err(|e| invalid_input_error("infrastructure type", e))?;
    let filtered = model.filter_nodes_by_infrastructure_type(infra_type);
    serde_json::to_string(&filtered).map_err(serialization_error)
}

/// Filter Data Flow relationships by infrastructure type.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `infrastructure_type` - Infrastructure type string (e.g., "Kafka", "PostgreSQL")
///
/// # Returns
///
/// JSON string containing array of matching relationships, or JsValue error
#[wasm_bindgen]
pub fn filter_relationships_by_infrastructure_type(
    workspace_json: &str,
    infrastructure_type: &str,
) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let infra_type: data_modelling_core::models::enums::InfrastructureType =
        serde_json::from_str(&format!("\"{}\"", infrastructure_type))
            .map_err(|e| invalid_input_error("infrastructure type", e))?;
    let filtered = model.filter_relationships_by_infrastructure_type(infra_type);
    serde_json::to_string(&filtered).map_err(serialization_error)
}

/// Filter Data Flow nodes and relationships by tag.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `tag` - Tag to filter by
///
/// # Returns
///
/// JSON string containing object with `nodes` and `relationships` arrays, or JsValue error
#[wasm_bindgen]
pub fn filter_by_tags(workspace_json: &str, tag: &str) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let (nodes, relationships) = model.filter_by_tags(tag);
    let result = serde_json::json!({
        "nodes": nodes,
        "relationships": relationships
    });
    serde_json::to_string(&result).map_err(serialization_error)
}

// ============================================================================
// Domain Operations
// ============================================================================

/// Add a system to a domain in a DataModel.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `domain_id` - Domain UUID as string
/// * `system_json` - JSON string containing System
///
/// # Returns
///
/// JSON string containing updated DataModel, or JsValue error
#[wasm_bindgen]
pub fn add_system_to_domain(
    workspace_json: &str,
    domain_id: &str,
    system_json: &str,
) -> Result<String, JsValue> {
    let mut model = deserialize_workspace(workspace_json)?;
    let domain_uuid =
        uuid::Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;
    let system: data_modelling_core::models::domain::System =
        serde_json::from_str(system_json).map_err(deserialization_error)?;
    model
        .add_system_to_domain(domain_uuid, system)
        .map_err(|e| WasmError::new("OperationError", e).to_js_value())?;
    serde_json::to_string(&model).map_err(serialization_error)
}

/// Add a CADS node to a domain in a DataModel.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `domain_id` - Domain UUID as string
/// * `node_json` - JSON string containing CADSNode
///
/// # Returns
///
/// JSON string containing updated DataModel, or JsValue error
#[wasm_bindgen]
pub fn add_cads_node_to_domain(
    workspace_json: &str,
    domain_id: &str,
    node_json: &str,
) -> Result<String, JsValue> {
    let mut model = deserialize_workspace(workspace_json)?;
    let domain_uuid =
        uuid::Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;
    let node: data_modelling_core::models::domain::CADSNode =
        serde_json::from_str(node_json).map_err(deserialization_error)?;
    model
        .add_cads_node_to_domain(domain_uuid, node)
        .map_err(|e| WasmError::new("OperationError", e).to_js_value())?;
    serde_json::to_string(&model).map_err(serialization_error)
}

/// Add an ODCS node to a domain in a DataModel.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `domain_id` - Domain UUID as string
/// * `node_json` - JSON string containing ODCSNode
///
/// # Returns
///
/// JSON string containing updated DataModel, or JsValue error
#[wasm_bindgen]
pub fn add_odcs_node_to_domain(
    workspace_json: &str,
    domain_id: &str,
    node_json: &str,
) -> Result<String, JsValue> {
    let mut model = deserialize_workspace(workspace_json)?;
    let domain_uuid =
        uuid::Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;
    let node: data_modelling_core::models::domain::ODCSNode =
        serde_json::from_str(node_json).map_err(deserialization_error)?;
    model
        .add_odcs_node_to_domain(domain_uuid, node)
        .map_err(|e| WasmError::new("OperationError", e).to_js_value())?;
    serde_json::to_string(&model).map_err(serialization_error)
}

// ============================================================================
// Validation Functions
// ============================================================================

/// Validate a table name.
///
/// # Arguments
///
/// * `name` - Table name to validate
///
/// # Returns
///
/// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
#[wasm_bindgen]
pub fn validate_table_name(name: &str) -> Result<String, JsValue> {
    match data_modelling_core::validation::input::validate_table_name(name) {
        Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
        Err(err) => Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string()),
    }
}

/// Validate a column name.
///
/// # Arguments
///
/// * `name` - Column name to validate
///
/// # Returns
///
/// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
#[wasm_bindgen]
pub fn validate_column_name(name: &str) -> Result<String, JsValue> {
    match data_modelling_core::validation::input::validate_column_name(name) {
        Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
        Err(err) => Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string()),
    }
}

/// Validate a UUID string.
///
/// # Arguments
///
/// * `id` - UUID string to validate
///
/// # Returns
///
/// JSON string with validation result: `{"valid": true, "uuid": "..."}` or `{"valid": false, "error": "error message"}`
#[wasm_bindgen]
pub fn validate_uuid(id: &str) -> Result<String, JsValue> {
    match data_modelling_core::validation::input::validate_uuid(id) {
        Ok(uuid) => Ok(serde_json::json!({"valid": true, "uuid": uuid.to_string()}).to_string()),
        Err(err) => Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string()),
    }
}

/// Validate a data type string.
///
/// # Arguments
///
/// * `data_type` - Data type string to validate
///
/// # Returns
///
/// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
#[wasm_bindgen]
pub fn validate_data_type(data_type: &str) -> Result<String, JsValue> {
    match data_modelling_core::validation::input::validate_data_type(data_type) {
        Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
        Err(err) => Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string()),
    }
}

/// Validate a description string.
///
/// # Arguments
///
/// * `desc` - Description string to validate
///
/// # Returns
///
/// JSON string with validation result: `{"valid": true}` or `{"valid": false, "error": "error message"}`
#[wasm_bindgen]
pub fn validate_description(desc: &str) -> Result<String, JsValue> {
    match data_modelling_core::validation::input::validate_description(desc) {
        Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
        Err(err) => Ok(serde_json::json!({"valid": false, "error": err.to_string()}).to_string()),
    }
}

/// Sanitize a SQL identifier by quoting it.
///
/// # Arguments
///
/// * `name` - SQL identifier to sanitize
/// * `dialect` - SQL dialect ("postgresql", "mysql", "sqlserver", etc.)
///
/// # Returns
///
/// Sanitized SQL identifier string
#[wasm_bindgen]
pub fn sanitize_sql_identifier(name: &str, dialect: &str) -> String {
    data_modelling_core::validation::input::sanitize_sql_identifier(name, dialect)
}

/// Sanitize a description string.
///
/// # Arguments
///
/// * `desc` - Description string to sanitize
///
/// # Returns
///
/// Sanitized description string
#[wasm_bindgen]
pub fn sanitize_description(desc: &str) -> String {
    data_modelling_core::validation::input::sanitize_description(desc)
}

/// Detect naming conflicts between existing and new tables.
///
/// # Arguments
///
/// * `existing_tables_json` - JSON string containing array of existing tables
/// * `new_tables_json` - JSON string containing array of new tables
///
/// # Returns
///
/// JSON string containing array of naming conflicts
#[wasm_bindgen]
pub fn detect_naming_conflicts(
    existing_tables_json: &str,
    new_tables_json: &str,
) -> Result<String, JsValue> {
    let existing_tables: Vec<data_modelling_core::models::Table> =
        serde_json::from_str(existing_tables_json).map_err(deserialization_error)?;
    let new_tables: Vec<data_modelling_core::models::Table> =
        serde_json::from_str(new_tables_json).map_err(deserialization_error)?;

    let validator = data_modelling_core::validation::tables::TableValidator::new();
    let conflicts = validator.detect_naming_conflicts(&existing_tables, &new_tables);

    serde_json::to_string(&conflicts).map_err(serialization_error)
}

/// Validate pattern exclusivity for a table (SCD pattern and Data Vault classification are mutually exclusive).
///
/// # Arguments
///
/// * `table_json` - JSON string containing table to validate
///
/// # Returns
///
/// JSON string with validation result: `{"valid": true}` or `{"valid": false, "violation": {...}}`
#[wasm_bindgen]
pub fn validate_pattern_exclusivity(table_json: &str) -> Result<String, JsValue> {
    let table: data_modelling_core::models::Table =
        serde_json::from_str(table_json).map_err(deserialization_error)?;

    let validator = data_modelling_core::validation::tables::TableValidator::new();
    match validator.validate_pattern_exclusivity(&table) {
        Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
        Err(violation) => {
            Ok(serde_json::json!({"valid": false, "violation": violation}).to_string())
        }
    }
}

/// Check for circular dependencies in relationships.
///
/// # Arguments
///
/// * `relationships_json` - JSON string containing array of existing relationships
/// * `source_table_id` - Source table ID (UUID string) of the new relationship
/// * `target_table_id` - Target table ID (UUID string) of the new relationship
///
/// # Returns
///
/// JSON string with result: `{"has_cycle": true/false, "cycle_path": [...]}` or error
#[wasm_bindgen]
pub fn check_circular_dependency(
    relationships_json: &str,
    source_table_id: &str,
    target_table_id: &str,
) -> Result<String, JsValue> {
    let relationships: Vec<data_modelling_core::models::Relationship> =
        serde_json::from_str(relationships_json).map_err(deserialization_error)?;

    let source_id = uuid::Uuid::parse_str(source_table_id)
        .map_err(|e| invalid_input_error("source_table_id", e))?;
    let target_id = uuid::Uuid::parse_str(target_table_id)
        .map_err(|e| invalid_input_error("target_table_id", e))?;

    let validator = data_modelling_core::validation::relationships::RelationshipValidator::new();
    match validator.check_circular_dependency(&relationships, source_id, target_id) {
        Ok((has_cycle, cycle_path)) => {
            let cycle_path_strs: Vec<String> = cycle_path
                .map(|path| path.iter().map(|id| id.to_string()).collect())
                .unwrap_or_default();
            Ok(serde_json::json!({
                "has_cycle": has_cycle,
                "cycle_path": cycle_path_strs
            })
            .to_string())
        }
        Err(err) => Err(validation_error(err)),
    }
}

/// Validate that source and target tables are different (no self-reference).
///
/// # Arguments
///
/// * `source_table_id` - Source table ID (UUID string)
/// * `target_table_id` - Target table ID (UUID string)
///
/// # Returns
///
/// JSON string with validation result: `{"valid": true}` or `{"valid": false, "self_reference": {...}}`
#[wasm_bindgen]
pub fn validate_no_self_reference(
    source_table_id: &str,
    target_table_id: &str,
) -> Result<String, JsValue> {
    let source_id = uuid::Uuid::parse_str(source_table_id)
        .map_err(|e| invalid_input_error("source_table_id", e))?;
    let target_id = uuid::Uuid::parse_str(target_table_id)
        .map_err(|e| invalid_input_error("target_table_id", e))?;

    let validator = data_modelling_core::validation::relationships::RelationshipValidator::new();
    match validator.validate_no_self_reference(source_id, target_id) {
        Ok(()) => Ok(serde_json::json!({"valid": true}).to_string()),
        Err(self_ref) => {
            Ok(serde_json::json!({"valid": false, "self_reference": self_ref}).to_string())
        }
    }
}

// ============================================================================
// PNG Export
// ============================================================================

/// Export a data model to PNG image format.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing workspace/data model structure
/// * `width` - Image width in pixels
/// * `height` - Image height in pixels
///
/// # Returns
///
/// Base64-encoded PNG image string, or JsValue error
#[cfg(feature = "png-export")]
#[wasm_bindgen]
pub fn export_to_png(workspace_json: &str, width: u32, height: u32) -> Result<String, JsValue> {
    let model = deserialize_workspace(workspace_json)?;
    let exporter = data_modelling_core::export::PNGExporter::new();
    match exporter.export(&model.tables, width, height) {
        Ok(result) => Ok(result.content), // Already base64-encoded
        Err(err) => Err(export_error_to_js(err)),
    }
}

// ============================================================================
// Model Loading/Saving (Async)
// ============================================================================

/// Load a model from browser storage (IndexedDB/localStorage).
///
/// # Arguments
///
/// * `db_name` - IndexedDB database name
/// * `store_name` - Object store name
/// * `workspace_path` - Workspace path to load from
///
/// # Returns
///
/// Promise that resolves to JSON string containing ModelLoadResult, or rejects with error
#[wasm_bindgen]
pub fn load_model(db_name: &str, store_name: &str, workspace_path: &str) -> js_sys::Promise {
    let db_name = db_name.to_string();
    let store_name = store_name.to_string();
    let workspace_path = workspace_path.to_string();

    wasm_bindgen_futures::future_to_promise(async move {
        let storage =
            data_modelling_core::storage::browser::BrowserStorageBackend::new(db_name, store_name);
        let loader = data_modelling_core::model::ModelLoader::new(storage);
        match loader.load_model(&workspace_path).await {
            Ok(result) => serde_json::to_string(&result)
                .map(|s| JsValue::from_str(&s))
                .map_err(serialization_error),
            Err(err) => Err(storage_error(err)),
        }
    })
}

/// Save a model to browser storage (IndexedDB/localStorage).
///
/// # Arguments
///
/// * `db_name` - IndexedDB database name
/// * `store_name` - Object store name
/// * `workspace_path` - Workspace path to save to
/// * `model_json` - JSON string containing DataModel to save
///
/// # Returns
///
/// Promise that resolves to success message, or rejects with error
#[wasm_bindgen]
pub fn save_model(
    db_name: &str,
    store_name: &str,
    workspace_path: &str,
    model_json: &str,
) -> js_sys::Promise {
    let db_name = db_name.to_string();
    let store_name = store_name.to_string();
    let workspace_path = workspace_path.to_string();
    let model_json = model_json.to_string();

    wasm_bindgen_futures::future_to_promise(async move {
        let model: data_modelling_core::models::DataModel =
            serde_json::from_str(&model_json).map_err(deserialization_error)?;

        let storage =
            data_modelling_core::storage::browser::BrowserStorageBackend::new(db_name, store_name);
        let saver = data_modelling_core::model::ModelSaver::new(storage);

        // Convert DataModel to table/relationship data for saving
        // For each table, save as YAML
        for table in &model.tables {
            // Export table to ODCS YAML
            let yaml =
                data_modelling_core::export::ODCSExporter::export_table(table, "odcs_v3_1_0");
            let table_data = data_modelling_core::model::saver::TableData {
                id: table.id,
                name: table.name.clone(),
                yaml_file_path: Some(format!("tables/{}.yaml", table.name)),
                yaml_value: serde_yaml::from_str(&yaml).map_err(parse_error)?,
            };
            saver
                .save_table(&workspace_path, &table_data)
                .await
                .map_err(storage_error)?;
        }

        // Save relationships
        if !model.relationships.is_empty() {
            let rel_data: Vec<data_modelling_core::model::saver::RelationshipData> = model
                .relationships
                .iter()
                .map(|rel| {
                    let yaml_value = serde_json::json!({
                        "id": rel.id.to_string(),
                        "source_table_id": rel.source_table_id.to_string(),
                        "target_table_id": rel.target_table_id.to_string(),
                    });
                    // Convert JSON value to YAML value
                    let yaml_str = serde_json::to_string(&yaml_value)
                        .map_err(|e| format!("Failed to serialize relationship: {}", e))?;
                    let yaml_value = serde_yaml::from_str(&yaml_str)
                        .map_err(|e| format!("Failed to convert to YAML: {}", e))?;
                    Ok(data_modelling_core::model::saver::RelationshipData {
                        id: rel.id,
                        source_table_id: rel.source_table_id,
                        target_table_id: rel.target_table_id,
                        yaml_value,
                    })
                })
                .collect::<Result<Vec<_>, String>>()
                .map_err(|e| WasmError::new("OperationError", e).to_js_value())?;

            saver
                .save_relationships(&workspace_path, &rel_data)
                .await
                .map_err(|e| storage_error(e))?;
        }

        Ok(JsValue::from_str("Model saved successfully"))
    })
}

// BPMN WASM Bindings
/// Import a BPMN model from XML content.
///
/// # Arguments
///
/// * `domain_id` - Domain UUID as string
/// * `xml_content` - BPMN XML content as a string
/// * `model_name` - Optional model name (extracted from XML if not provided)
///
/// # Returns
///
/// JSON string containing BPMNModel, or JsValue error
#[cfg(feature = "bpmn")]
#[wasm_bindgen]
pub fn import_bpmn_model(
    domain_id: &str,
    xml_content: &str,
    model_name: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::import::bpmn::BPMNImporter;
    use uuid::Uuid;

    let domain_uuid =
        Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;

    let mut importer = BPMNImporter::new();
    match importer.import(xml_content, domain_uuid, model_name.as_deref()) {
        Ok(model) => serde_json::to_string(&model).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(ImportError::ParseError(e.to_string()))),
    }
}

/// Export a BPMN model to XML content.
///
/// # Arguments
///
/// * `xml_content` - BPMN XML content as a string
///
/// # Returns
///
/// BPMN XML content as string, or JsValue error
#[cfg(feature = "bpmn")]
#[wasm_bindgen]
pub fn export_bpmn_model(xml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::bpmn::BPMNExporter;
    let exporter = BPMNExporter::new();
    exporter
        .export(xml_content)
        .map_err(|e| export_error_to_js(ExportError::SerializationError(e.to_string())))
}

// DMN WASM Bindings
/// Import a DMN model from XML content.
///
/// # Arguments
///
/// * `domain_id` - Domain UUID as string
/// * `xml_content` - DMN XML content as a string
/// * `model_name` - Optional model name (extracted from XML if not provided)
///
/// # Returns
///
/// JSON string containing DMNModel, or JsValue error
#[cfg(feature = "dmn")]
#[wasm_bindgen]
pub fn import_dmn_model(
    domain_id: &str,
    xml_content: &str,
    model_name: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::import::dmn::DMNImporter;
    use uuid::Uuid;

    let domain_uuid =
        Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;

    let mut importer = DMNImporter::new();
    match importer.import(xml_content, domain_uuid, model_name.as_deref()) {
        Ok(model) => serde_json::to_string(&model).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(ImportError::ParseError(e.to_string()))),
    }
}

/// Export a DMN model to XML content.
///
/// # Arguments
///
/// * `xml_content` - DMN XML content as a string
///
/// # Returns
///
/// DMN XML content as string, or JsValue error
#[cfg(feature = "dmn")]
#[wasm_bindgen]
pub fn export_dmn_model(xml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::dmn::DMNExporter;
    let exporter = DMNExporter::new();
    exporter
        .export(xml_content)
        .map_err(|e| export_error_to_js(ExportError::SerializationError(e.to_string())))
}

// OpenAPI WASM Bindings
/// Import an OpenAPI specification from YAML or JSON content.
///
/// # Arguments
///
/// * `domain_id` - Domain UUID as string
/// * `content` - OpenAPI YAML or JSON content as a string
/// * `api_name` - Optional API name (extracted from info.title if not provided)
///
/// # Returns
///
/// JSON string containing OpenAPIModel, or JsValue error
#[cfg(feature = "openapi")]
#[wasm_bindgen]
pub fn import_openapi_spec(
    domain_id: &str,
    content: &str,
    api_name: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::import::openapi::OpenAPIImporter;
    use uuid::Uuid;

    let domain_uuid =
        Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;

    let mut importer = OpenAPIImporter::new();
    match importer.import(content, domain_uuid, api_name.as_deref()) {
        Ok(model) => serde_json::to_string(&model).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(ImportError::ParseError(e.to_string()))),
    }
}

/// Export an OpenAPI specification to YAML or JSON content.
///
/// # Arguments
///
/// * `content` - OpenAPI content as a string
/// * `source_format` - Source format ("yaml" or "json")
/// * `target_format` - Optional target format for conversion (None to keep original)
///
/// # Returns
///
/// OpenAPI content in requested format, or JsValue error
#[cfg(feature = "openapi")]
#[wasm_bindgen]
pub fn export_openapi_spec(
    content: &str,
    source_format: &str,
    target_format: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::openapi::OpenAPIExporter;
    use data_modelling_core::models::openapi::OpenAPIFormat;

    let source_fmt = match source_format {
        "yaml" | "yml" => OpenAPIFormat::Yaml,
        "json" => OpenAPIFormat::Json,
        _ => {
            return Err(invalid_input_error("source format", "Use 'yaml' or 'json'"));
        }
    };

    let target_fmt = if let Some(tf) = target_format {
        match tf.as_str() {
            "yaml" | "yml" => Some(OpenAPIFormat::Yaml),
            "json" => Some(OpenAPIFormat::Json),
            _ => {
                return Err(invalid_input_error("target format", "Use 'yaml' or 'json'"));
            }
        }
    } else {
        None
    };

    let exporter = OpenAPIExporter::new();
    exporter
        .export(content, source_fmt, target_fmt)
        .map_err(|e| export_error_to_js(ExportError::SerializationError(e.to_string())))
}

/// Convert an OpenAPI schema component to an ODCS table.
///
/// # Arguments
///
/// * `openapi_content` - OpenAPI YAML or JSON content as a string
/// * `component_name` - Name of the schema component to convert
/// * `table_name` - Optional desired ODCS table name (uses component_name if None)
///
/// # Returns
///
/// JSON string containing ODCS Table, or JsValue error
#[cfg(feature = "openapi")]
#[wasm_bindgen]
pub fn convert_openapi_to_odcs(
    openapi_content: &str,
    component_name: &str,
    table_name: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::convert::openapi_to_odcs::OpenAPIToODCSConverter;

    let converter = OpenAPIToODCSConverter::new();
    match converter.convert_component(openapi_content, component_name, table_name.as_deref()) {
        Ok(table) => serde_json::to_string(&table).map_err(serialization_error),
        Err(e) => Err(conversion_error(e)),
    }
}

/// Analyze an OpenAPI component for conversion feasibility.
///
/// # Arguments
///
/// * `openapi_content` - OpenAPI YAML or JSON content as a string
/// * `component_name` - Name of the schema component to analyze
///
/// # Returns
///
/// JSON string containing ConversionReport, or JsValue error
#[cfg(feature = "openapi")]
#[wasm_bindgen]
pub fn analyze_openapi_conversion(
    openapi_content: &str,
    component_name: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::convert::openapi_to_odcs::OpenAPIToODCSConverter;

    let converter = OpenAPIToODCSConverter::new();
    match converter.analyze_conversion(openapi_content, component_name) {
        Ok(report) => serde_json::to_string(&report).map_err(serialization_error),
        Err(e) => Err(WasmError::new("AnalysisError", e.to_string())
            .with_code("ANALYSIS_FAILED")
            .to_js_value()),
    }
}

// ============================================================================
// Workspace and DomainConfig Operations
// ============================================================================

/// Create a new workspace.
///
/// # Arguments
///
/// * `name` - Workspace name
/// * `owner_id` - Owner UUID as string
///
/// # Returns
///
/// JSON string containing Workspace, or JsValue error
#[wasm_bindgen]
pub fn create_workspace(name: &str, owner_id: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::workspace::Workspace;
    use uuid::Uuid;

    let owner_uuid = Uuid::parse_str(owner_id).map_err(|e| invalid_input_error("owner ID", e))?;

    let workspace = Workspace::new(name.to_string(), owner_uuid);

    serde_json::to_string(&workspace).map_err(serialization_error)
}

/// Parse workspace YAML content and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Workspace YAML content as a string
///
/// # Returns
///
/// JSON string containing Workspace, or JsValue error
#[wasm_bindgen]
pub fn parse_workspace_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::workspace::Workspace;

    let workspace: Workspace = serde_yaml::from_str(yaml_content).map_err(parse_error)?;
    serde_json::to_string(&workspace).map_err(serialization_error)
}

/// Export a workspace to YAML format.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing Workspace
///
/// # Returns
///
/// Workspace YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_workspace_to_yaml(workspace_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::workspace::Workspace;

    let workspace: Workspace =
        serde_json::from_str(workspace_json).map_err(deserialization_error)?;
    serde_yaml::to_string(&workspace).map_err(serialization_error)
}

/// Add a domain reference to a workspace.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing Workspace
/// * `domain_id` - Domain UUID as string
/// * `domain_name` - Domain name
///
/// # Returns
///
/// JSON string containing updated Workspace, or JsValue error
#[wasm_bindgen]
pub fn add_domain_to_workspace(
    workspace_json: &str,
    domain_id: &str,
    domain_name: &str,
) -> Result<String, JsValue> {
    use chrono::Utc;
    use data_modelling_core::models::workspace::{DomainReference, Workspace};
    use uuid::Uuid;

    let mut workspace: Workspace =
        serde_json::from_str(workspace_json).map_err(deserialization_error)?;
    let domain_uuid =
        Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;

    // Check if domain already exists
    if workspace.domains.iter().any(|d| d.id == domain_uuid) {
        return Err(WasmError::new(
            "DuplicateError",
            format!("Domain {} already exists in workspace", domain_id),
        )
        .with_code("DUPLICATE_DOMAIN")
        .to_js_value());
    }

    workspace.domains.push(DomainReference {
        id: domain_uuid,
        name: domain_name.to_string(),
        description: None,
        systems: Vec::new(),
        view_positions: std::collections::HashMap::new(),
    });
    workspace.last_modified_at = Utc::now();

    serde_json::to_string(&workspace).map_err(serialization_error)
}

/// Remove a domain reference from a workspace.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing Workspace
/// * `domain_id` - Domain UUID as string to remove
///
/// # Returns
///
/// JSON string containing updated Workspace, or JsValue error
#[wasm_bindgen]
pub fn remove_domain_from_workspace(
    workspace_json: &str,
    domain_id: &str,
) -> Result<String, JsValue> {
    use chrono::Utc;
    use data_modelling_core::models::workspace::Workspace;
    use uuid::Uuid;

    let mut workspace: Workspace =
        serde_json::from_str(workspace_json).map_err(deserialization_error)?;
    let domain_uuid =
        Uuid::parse_str(domain_id).map_err(|e| invalid_input_error("domain ID", e))?;

    let original_len = workspace.domains.len();
    workspace.domains.retain(|d| d.id != domain_uuid);

    if workspace.domains.len() == original_len {
        return Err(WasmError::new(
            "NotFoundError",
            format!("Domain {} not found in workspace", domain_id),
        )
        .with_code("DOMAIN_NOT_FOUND")
        .to_js_value());
    }

    workspace.last_modified_at = Utc::now();
    serde_json::to_string(&workspace).map_err(serialization_error)
}

/// Add a relationship to a workspace.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing Workspace
/// * `relationship_json` - JSON string containing Relationship
///
/// # Returns
///
/// JSON string containing updated Workspace, or JsValue error
#[wasm_bindgen]
pub fn add_relationship_to_workspace(
    workspace_json: &str,
    relationship_json: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::Relationship;
    use data_modelling_core::models::workspace::Workspace;

    let mut workspace: Workspace =
        serde_json::from_str(workspace_json).map_err(deserialization_error)?;
    let relationship: Relationship =
        serde_json::from_str(relationship_json).map_err(deserialization_error)?;

    // Check if relationship already exists
    if workspace
        .relationships
        .iter()
        .any(|r| r.id == relationship.id)
    {
        return Err(WasmError::new(
            "DuplicateError",
            format!(
                "Relationship {} already exists in workspace",
                relationship.id
            ),
        )
        .with_code("DUPLICATE_RELATIONSHIP")
        .to_js_value());
    }

    workspace.add_relationship(relationship);
    serde_json::to_string(&workspace).map_err(serialization_error)
}

/// Remove a relationship from a workspace.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing Workspace
/// * `relationship_id` - Relationship UUID as string to remove
///
/// # Returns
///
/// JSON string containing updated Workspace, or JsValue error
#[wasm_bindgen]
pub fn remove_relationship_from_workspace(
    workspace_json: &str,
    relationship_id: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::workspace::Workspace;
    use uuid::Uuid;

    let mut workspace: Workspace =
        serde_json::from_str(workspace_json).map_err(deserialization_error)?;
    let relationship_uuid =
        Uuid::parse_str(relationship_id).map_err(|e| invalid_input_error("relationship ID", e))?;

    if !workspace.remove_relationship(relationship_uuid) {
        return Err(WasmError::new(
            "NotFoundError",
            format!("Relationship {} not found in workspace", relationship_id),
        )
        .with_code("RELATIONSHIP_NOT_FOUND")
        .to_js_value());
    }

    serde_json::to_string(&workspace).map_err(serialization_error)
}

/// Get relationships for a source table from a workspace.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing Workspace
/// * `source_table_id` - Source table UUID as string
///
/// # Returns
///
/// JSON string containing array of Relationships, or JsValue error
#[wasm_bindgen]
pub fn get_workspace_relationships_for_source(
    workspace_json: &str,
    source_table_id: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::workspace::Workspace;
    use uuid::Uuid;

    let workspace: Workspace =
        serde_json::from_str(workspace_json).map_err(deserialization_error)?;
    let source_uuid =
        Uuid::parse_str(source_table_id).map_err(|e| invalid_input_error("source table ID", e))?;

    let relationships: Vec<_> = workspace.get_relationships_for_source(source_uuid);
    serde_json::to_string(&relationships).map_err(serialization_error)
}

/// Get relationships for a target table from a workspace.
///
/// # Arguments
///
/// * `workspace_json` - JSON string containing Workspace
/// * `target_table_id` - Target table UUID as string
///
/// # Returns
///
/// JSON string containing array of Relationships, or JsValue error
#[wasm_bindgen]
pub fn get_workspace_relationships_for_target(
    workspace_json: &str,
    target_table_id: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::workspace::Workspace;
    use uuid::Uuid;

    let workspace: Workspace =
        serde_json::from_str(workspace_json).map_err(deserialization_error)?;
    let target_uuid =
        Uuid::parse_str(target_table_id).map_err(|e| invalid_input_error("target table ID", e))?;

    let relationships: Vec<_> = workspace.get_relationships_for_target(target_uuid);
    serde_json::to_string(&relationships).map_err(serialization_error)
}

/// Create a new domain configuration.
///
/// # Arguments
///
/// * `name` - Domain name
/// * `workspace_id` - Workspace UUID as string
///
/// # Returns
///
/// JSON string containing DomainConfig, or JsValue error
#[wasm_bindgen]
pub fn create_domain_config(name: &str, workspace_id: &str) -> Result<String, JsValue> {
    use chrono::Utc;
    use data_modelling_core::models::domain_config::DomainConfig;
    use std::collections::HashMap;
    use uuid::Uuid;

    let workspace_uuid =
        Uuid::parse_str(workspace_id).map_err(|e| invalid_input_error("workspace ID", e))?;

    let config = DomainConfig {
        id: Uuid::new_v4(),
        workspace_id: workspace_uuid,
        name: name.to_string(),
        description: None,
        created_at: Utc::now(),
        last_modified_at: Utc::now(),
        owner: None,
        systems: Vec::new(),
        tables: Vec::new(),
        products: Vec::new(),
        assets: Vec::new(),
        processes: Vec::new(),
        decisions: Vec::new(),
        view_positions: HashMap::new(),
        folder_path: None,
        workspace_path: None,
    };

    serde_json::to_string(&config).map_err(serialization_error)
}

/// Parse domain config YAML content and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Domain config YAML content as a string
///
/// # Returns
///
/// JSON string containing DomainConfig, or JsValue error
#[wasm_bindgen]
pub fn parse_domain_config_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::domain_config::DomainConfig;

    let config: DomainConfig = serde_yaml::from_str(yaml_content).map_err(parse_error)?;
    serde_json::to_string(&config).map_err(serialization_error)
}

/// Export a domain config to YAML format.
///
/// # Arguments
///
/// * `config_json` - JSON string containing DomainConfig
///
/// # Returns
///
/// DomainConfig YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_domain_config_to_yaml(config_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::domain_config::DomainConfig;

    let config: DomainConfig = serde_json::from_str(config_json).map_err(deserialization_error)?;
    serde_yaml::to_string(&config).map_err(serialization_error)
}

/// Get the domain ID from a domain config JSON.
///
/// # Arguments
///
/// * `config_json` - JSON string containing DomainConfig
///
/// # Returns
///
/// Domain UUID as string, or JsValue error
#[wasm_bindgen]
pub fn get_domain_config_id(config_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::domain_config::DomainConfig;

    let config: DomainConfig = serde_json::from_str(config_json).map_err(deserialization_error)?;
    Ok(config.id.to_string())
}

/// Update domain config with new view positions.
///
/// # Arguments
///
/// * `config_json` - JSON string containing DomainConfig
/// * `positions_json` - JSON string containing view positions map
///
/// # Returns
///
/// JSON string containing updated DomainConfig, or JsValue error
#[wasm_bindgen]
pub fn update_domain_view_positions(
    config_json: &str,
    positions_json: &str,
) -> Result<String, JsValue> {
    use chrono::Utc;
    use data_modelling_core::models::domain_config::{DomainConfig, ViewPosition};
    use std::collections::HashMap;

    let mut config: DomainConfig =
        serde_json::from_str(config_json).map_err(deserialization_error)?;
    let positions: HashMap<String, HashMap<String, ViewPosition>> =
        serde_json::from_str(positions_json).map_err(deserialization_error)?;

    config.view_positions = positions;
    config.last_modified_at = Utc::now();

    serde_json::to_string(&config).map_err(serialization_error)
}

/// Add an entity reference to a domain config.
///
/// # Arguments
///
/// * `config_json` - JSON string containing DomainConfig
/// * `entity_type` - Entity type: "system", "table", "product", "asset", "process", "decision"
/// * `entity_id` - Entity UUID as string
///
/// # Returns
///
/// JSON string containing updated DomainConfig, or JsValue error
#[wasm_bindgen]
pub fn add_entity_to_domain_config(
    config_json: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<String, JsValue> {
    use chrono::Utc;
    use data_modelling_core::models::domain_config::DomainConfig;
    use uuid::Uuid;

    let mut config: DomainConfig =
        serde_json::from_str(config_json).map_err(deserialization_error)?;
    let entity_uuid =
        Uuid::parse_str(entity_id).map_err(|e| invalid_input_error("entity ID", e))?;

    let entities = match entity_type {
        "system" => &mut config.systems,
        "table" => &mut config.tables,
        "product" => &mut config.products,
        "asset" => &mut config.assets,
        "process" => &mut config.processes,
        "decision" => &mut config.decisions,
        _ => {
            return Err(invalid_input_error(
                "entity type",
                "Use 'system', 'table', 'product', 'asset', 'process', or 'decision'",
            ));
        }
    };

    if entities.contains(&entity_uuid) {
        return Err(WasmError::new(
            "DuplicateError",
            format!(
                "{} {} already exists in domain config",
                entity_type, entity_id
            ),
        )
        .with_code("DUPLICATE_ENTITY")
        .to_js_value());
    }

    entities.push(entity_uuid);
    config.last_modified_at = Utc::now();

    serde_json::to_string(&config).map_err(serialization_error)
}

/// Remove an entity reference from a domain config.
///
/// # Arguments
///
/// * `config_json` - JSON string containing DomainConfig
/// * `entity_type` - Entity type: "system", "table", "product", "asset", "process", "decision"
/// * `entity_id` - Entity UUID as string to remove
///
/// # Returns
///
/// JSON string containing updated DomainConfig, or JsValue error
#[wasm_bindgen]
pub fn remove_entity_from_domain_config(
    config_json: &str,
    entity_type: &str,
    entity_id: &str,
) -> Result<String, JsValue> {
    use chrono::Utc;
    use data_modelling_core::models::domain_config::DomainConfig;
    use uuid::Uuid;

    let mut config: DomainConfig =
        serde_json::from_str(config_json).map_err(deserialization_error)?;
    let entity_uuid =
        Uuid::parse_str(entity_id).map_err(|e| invalid_input_error("entity ID", e))?;

    let entities = match entity_type {
        "system" => &mut config.systems,
        "table" => &mut config.tables,
        "product" => &mut config.products,
        "asset" => &mut config.assets,
        "process" => &mut config.processes,
        "decision" => &mut config.decisions,
        _ => {
            return Err(invalid_input_error(
                "entity type",
                "Use 'system', 'table', 'product', 'asset', 'process', or 'decision'",
            ));
        }
    };

    let original_len = entities.len();
    entities.retain(|id| *id != entity_uuid);

    if entities.len() == original_len {
        return Err(WasmError::new(
            "NotFoundError",
            format!("{} {} not found in domain config", entity_type, entity_id),
        )
        .with_code("ENTITY_NOT_FOUND")
        .to_js_value());
    }

    config.last_modified_at = Utc::now();
    serde_json::to_string(&config).map_err(serialization_error)
}

// ============================================================================
// Decision Log (DDL) Operations
// ============================================================================

/// Parse a decision YAML file and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Decision YAML content as a string (.madr.yaml)
///
/// # Returns
///
/// JSON string containing Decision, or JsValue error
#[wasm_bindgen]
pub fn parse_decision_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::import::decision::DecisionImporter;

    let importer = DecisionImporter::new();
    match importer.import(yaml_content) {
        Ok(decision) => serde_json::to_string(&decision).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(e)),
    }
}

/// Parse a decisions index YAML file and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Decisions index YAML content as a string (decisions.yaml)
///
/// # Returns
///
/// JSON string containing DecisionIndex, or JsValue error
#[wasm_bindgen]
pub fn parse_decision_index_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::import::decision::DecisionImporter;

    let importer = DecisionImporter::new();
    match importer.import_index(yaml_content) {
        Ok(index) => serde_json::to_string(&index).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(e)),
    }
}

/// Export a decision to YAML format.
///
/// # Arguments
///
/// * `decision_json` - JSON string containing Decision
///
/// # Returns
///
/// Decision YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_decision_to_yaml(decision_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::decision::DecisionExporter;
    use data_modelling_core::models::decision::Decision;

    let decision: Decision = serde_json::from_str(decision_json).map_err(deserialization_error)?;
    let exporter = DecisionExporter::new();
    exporter
        .export_without_validation(&decision)
        .map_err(export_error_to_js)
}

/// Export a decisions index to YAML format.
///
/// # Arguments
///
/// * `index_json` - JSON string containing DecisionIndex
///
/// # Returns
///
/// DecisionIndex YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_decision_index_to_yaml(index_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::decision::DecisionExporter;
    use data_modelling_core::models::decision::DecisionIndex;

    let index: DecisionIndex = serde_json::from_str(index_json).map_err(deserialization_error)?;
    let exporter = DecisionExporter::new();
    exporter.export_index(&index).map_err(export_error_to_js)
}

/// Export a decision to Markdown format (MADR template).
///
/// # Arguments
///
/// * `decision_json` - JSON string containing Decision
///
/// # Returns
///
/// Decision Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_decision_to_markdown(decision_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::markdown::MarkdownExporter;
    use data_modelling_core::models::decision::Decision;

    let decision: Decision = serde_json::from_str(decision_json).map_err(deserialization_error)?;
    let exporter = MarkdownExporter::new();
    exporter
        .export_decision(&decision)
        .map_err(export_error_to_js)
}

/// Create a new decision with required fields.
///
/// # Arguments
///
/// * `number` - Decision number (ADR-0001, ADR-0002, etc.)
/// * `title` - Short title describing the decision
/// * `context` - Problem statement and context
/// * `decision` - The decision that was made
///
/// # Returns
///
/// JSON string containing Decision, or JsValue error
#[wasm_bindgen]
pub fn create_decision(
    number: u32,
    title: &str,
    context: &str,
    decision: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::decision::Decision;

    let dec = Decision::new(number.into(), title, context, decision);
    serde_json::to_string(&dec).map_err(serialization_error)
}

/// Create a new empty decision index.
///
/// # Returns
///
/// JSON string containing DecisionIndex, or JsValue error
#[wasm_bindgen]
pub fn create_decision_index() -> Result<String, JsValue> {
    use data_modelling_core::models::decision::DecisionIndex;

    let index = DecisionIndex::new();
    serde_json::to_string(&index).map_err(serialization_error)
}

/// Add a decision to an index.
///
/// # Arguments
///
/// * `index_json` - JSON string containing DecisionIndex
/// * `decision_json` - JSON string containing Decision
/// * `filename` - Filename for the decision YAML file
///
/// # Returns
///
/// JSON string containing updated DecisionIndex, or JsValue error
#[wasm_bindgen]
pub fn add_decision_to_index(
    index_json: &str,
    decision_json: &str,
    filename: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::decision::{Decision, DecisionIndex};

    let mut index: DecisionIndex =
        serde_json::from_str(index_json).map_err(deserialization_error)?;
    let decision: Decision = serde_json::from_str(decision_json).map_err(deserialization_error)?;

    index.add_decision(&decision, filename.to_string());
    serde_json::to_string(&index).map_err(serialization_error)
}

// ============================================================================
// Knowledge Base (KB) Operations
// ============================================================================

/// Parse a knowledge article YAML file and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Knowledge article YAML content as a string (.kb.yaml)
///
/// # Returns
///
/// JSON string containing KnowledgeArticle, or JsValue error
#[wasm_bindgen]
pub fn parse_knowledge_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::import::knowledge::KnowledgeImporter;

    let importer = KnowledgeImporter::new();
    match importer.import(yaml_content) {
        Ok(article) => serde_json::to_string(&article).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(e)),
    }
}

/// Parse a knowledge index YAML file and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Knowledge index YAML content as a string (knowledge.yaml)
///
/// # Returns
///
/// JSON string containing KnowledgeIndex, or JsValue error
#[wasm_bindgen]
pub fn parse_knowledge_index_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::import::knowledge::KnowledgeImporter;

    let importer = KnowledgeImporter::new();
    match importer.import_index(yaml_content) {
        Ok(index) => serde_json::to_string(&index).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(e)),
    }
}

/// Export a knowledge article to YAML format.
///
/// # Arguments
///
/// * `article_json` - JSON string containing KnowledgeArticle
///
/// # Returns
///
/// KnowledgeArticle YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_knowledge_to_yaml(article_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::knowledge::KnowledgeExporter;
    use data_modelling_core::models::knowledge::KnowledgeArticle;

    let article: KnowledgeArticle =
        serde_json::from_str(article_json).map_err(deserialization_error)?;
    let exporter = KnowledgeExporter::new();
    exporter
        .export_without_validation(&article)
        .map_err(export_error_to_js)
}

/// Export a knowledge index to YAML format.
///
/// # Arguments
///
/// * `index_json` - JSON string containing KnowledgeIndex
///
/// # Returns
///
/// KnowledgeIndex YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_knowledge_index_to_yaml(index_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::knowledge::KnowledgeExporter;
    use data_modelling_core::models::knowledge::KnowledgeIndex;

    let index: KnowledgeIndex = serde_json::from_str(index_json).map_err(deserialization_error)?;
    let exporter = KnowledgeExporter::new();
    exporter.export_index(&index).map_err(export_error_to_js)
}

/// Export a knowledge article to Markdown format.
///
/// # Arguments
///
/// * `article_json` - JSON string containing KnowledgeArticle
///
/// # Returns
///
/// KnowledgeArticle Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_knowledge_to_markdown(article_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::markdown::MarkdownExporter;
    use data_modelling_core::models::knowledge::KnowledgeArticle;

    let article: KnowledgeArticle =
        serde_json::from_str(article_json).map_err(deserialization_error)?;
    let exporter = MarkdownExporter::new();
    exporter
        .export_knowledge(&article)
        .map_err(export_error_to_js)
}

/// Create a new knowledge article with required fields.
///
/// # Arguments
///
/// * `number` - Article number (1, 2, 3, etc. - will be formatted as KB-0001)
/// * `title` - Article title
/// * `summary` - Brief summary of the article
/// * `content` - Full article content in Markdown
/// * `author` - Article author (email or name)
///
/// # Returns
///
/// JSON string containing KnowledgeArticle, or JsValue error
#[wasm_bindgen]
pub fn create_knowledge_article(
    number: u32,
    title: &str,
    summary: &str,
    content: &str,
    author: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::knowledge::KnowledgeArticle;

    let article = KnowledgeArticle::new(number.into(), title, summary, content, author);
    serde_json::to_string(&article).map_err(serialization_error)
}

/// Create a new empty knowledge index.
///
/// # Returns
///
/// JSON string containing KnowledgeIndex, or JsValue error
#[wasm_bindgen]
pub fn create_knowledge_index() -> Result<String, JsValue> {
    use data_modelling_core::models::knowledge::KnowledgeIndex;

    let index = KnowledgeIndex::new();
    serde_json::to_string(&index).map_err(serialization_error)
}

/// Add an article to a knowledge index.
///
/// # Arguments
///
/// * `index_json` - JSON string containing KnowledgeIndex
/// * `article_json` - JSON string containing KnowledgeArticle
/// * `filename` - Filename for the article YAML file
///
/// # Returns
///
/// JSON string containing updated KnowledgeIndex, or JsValue error
#[wasm_bindgen]
pub fn add_article_to_knowledge_index(
    index_json: &str,
    article_json: &str,
    filename: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::knowledge::{KnowledgeArticle, KnowledgeIndex};

    let mut index: KnowledgeIndex =
        serde_json::from_str(index_json).map_err(deserialization_error)?;
    let article: KnowledgeArticle =
        serde_json::from_str(article_json).map_err(deserialization_error)?;

    index.add_article(&article, filename.to_string());
    serde_json::to_string(&index).map_err(serialization_error)
}

/// Search knowledge articles by title, summary, or content.
///
/// # Arguments
///
/// * `articles_json` - JSON string containing array of KnowledgeArticle
/// * `query` - Search query string (case-insensitive)
///
/// # Returns
///
/// JSON string containing array of matching KnowledgeArticle, or JsValue error
#[wasm_bindgen]
pub fn search_knowledge_articles(articles_json: &str, query: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::knowledge::KnowledgeArticle;

    let articles: Vec<KnowledgeArticle> =
        serde_json::from_str(articles_json).map_err(deserialization_error)?;

    let query_lower = query.to_lowercase();
    let matches: Vec<&KnowledgeArticle> = articles
        .iter()
        .filter(|article| {
            article.title.to_lowercase().contains(&query_lower)
                || article.summary.to_lowercase().contains(&query_lower)
                || article.content.to_lowercase().contains(&query_lower)
                || article
                    .tags
                    .iter()
                    .any(|tag| tag.to_string().to_lowercase().contains(&query_lower))
        })
        .collect();

    serde_json::to_string(&matches).map_err(serialization_error)
}

// ==================== PDF Export Bindings ====================

/// Export a decision to PDF format with optional branding.
///
/// # Arguments
///
/// * `decision_json` - JSON string containing Decision
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_decision_to_pdf(
    decision_json: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::models::decision::Decision;

    let decision: Decision = serde_json::from_str(decision_json).map_err(deserialization_error)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_decision(&decision)
        .map_err(export_error_to_js)?;

    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export a knowledge article to PDF format with optional branding.
///
/// # Arguments
///
/// * `article_json` - JSON string containing KnowledgeArticle
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_knowledge_to_pdf(
    article_json: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::models::knowledge::KnowledgeArticle;

    let article: KnowledgeArticle =
        serde_json::from_str(article_json).map_err(deserialization_error)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_knowledge(&article)
        .map_err(export_error_to_js)?;

    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export raw markdown content to PDF format with optional branding.
///
/// # Arguments
///
/// * `title` - Document title
/// * `content` - Markdown content
/// * `filename` - Output filename suggestion
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_markdown_to_pdf(
    title: &str,
    content: &str,
    filename: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_markdown(title, content, filename)
        .map_err(export_error_to_js)?;

    serde_json::to_string(&result).map_err(serialization_error)
}

// ==================== Branded Markdown Export Bindings ====================

/// Export a decision to branded Markdown format.
///
/// # Arguments
///
/// * `decision_json` - JSON string containing Decision
/// * `branding_json` - Optional JSON string containing MarkdownBrandingConfig
///
/// # Returns
///
/// Branded Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_decision_to_branded_markdown(
    decision_json: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::markdown::{BrandedMarkdownExporter, MarkdownBrandingConfig};
    use data_modelling_core::models::decision::Decision;

    let decision: Decision = serde_json::from_str(decision_json).map_err(deserialization_error)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: MarkdownBrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        BrandedMarkdownExporter::with_branding(branding)
    } else {
        BrandedMarkdownExporter::new()
    };

    exporter
        .export_decision(&decision)
        .map_err(export_error_to_js)
}

/// Export a knowledge article to branded Markdown format.
///
/// # Arguments
///
/// * `article_json` - JSON string containing KnowledgeArticle
/// * `branding_json` - Optional JSON string containing MarkdownBrandingConfig
///
/// # Returns
///
/// Branded Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_knowledge_to_branded_markdown(
    article_json: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::markdown::{BrandedMarkdownExporter, MarkdownBrandingConfig};
    use data_modelling_core::models::knowledge::KnowledgeArticle;

    let article: KnowledgeArticle =
        serde_json::from_str(article_json).map_err(deserialization_error)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: MarkdownBrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        BrandedMarkdownExporter::with_branding(branding)
    } else {
        BrandedMarkdownExporter::new()
    };

    exporter
        .export_knowledge(&article)
        .map_err(export_error_to_js)
}

/// Get default PDF branding configuration.
///
/// # Returns
///
/// JSON string containing default BrandingConfig
#[wasm_bindgen]
pub fn get_default_pdf_branding() -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::BrandingConfig;

    let config = BrandingConfig::default();
    serde_json::to_string(&config).map_err(serialization_error)
}

/// Get default Markdown branding configuration.
///
/// # Returns
///
/// JSON string containing default MarkdownBrandingConfig
#[wasm_bindgen]
pub fn get_default_markdown_branding() -> Result<String, JsValue> {
    use data_modelling_core::export::markdown::MarkdownBrandingConfig;

    let config = MarkdownBrandingConfig::default();
    serde_json::to_string(&config).map_err(serialization_error)
}

// ============================================================================
// ODCS/ODPS/CADS PDF Export Bindings
// ============================================================================

/// Export an ODCS Table (Data Contract) to PDF format with optional branding.
///
/// # Arguments
///
/// * `table_json` - JSON string containing Table
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_table_to_pdf(
    table_json: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::models::Table;

    let table: Table = serde_json::from_str(table_json).map_err(deserialization_error)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter.export_table(&table).map_err(export_error_to_js)?;

    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export an ODPS Data Product to PDF format with optional branding.
///
/// # Arguments
///
/// * `product_json` - JSON string containing ODPSDataProduct
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_odps_to_pdf(
    product_json: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::models::odps::ODPSDataProduct;

    let product: ODPSDataProduct =
        serde_json::from_str(product_json).map_err(deserialization_error)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_data_product(&product)
        .map_err(export_error_to_js)?;

    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export a CADS Asset to PDF format with optional branding.
///
/// # Arguments
///
/// * `asset_json` - JSON string containing CADSAsset
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_cads_to_pdf(
    asset_json: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::models::cads::CADSAsset;

    let asset: CADSAsset = serde_json::from_str(asset_json).map_err(deserialization_error)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_cads_asset(&asset)
        .map_err(export_error_to_js)?;

    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export an ODCS Table (Data Contract) to Markdown format.
///
/// # Arguments
///
/// * `table_json` - JSON string containing Table
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_table_to_markdown(table_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::PdfExporter;
    use data_modelling_core::models::Table;

    let table: Table = serde_json::from_str(table_json).map_err(deserialization_error)?;

    // Use PdfExporter's internal markdown generation
    let exporter = PdfExporter::new();
    Ok(exporter.table_to_markdown_public(&table))
}

/// Export an ODPS Data Product to Markdown format.
///
/// # Arguments
///
/// * `product_json` - JSON string containing ODPSDataProduct
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_odps_to_markdown(product_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::PdfExporter;
    use data_modelling_core::models::odps::ODPSDataProduct;

    let product: ODPSDataProduct =
        serde_json::from_str(product_json).map_err(deserialization_error)?;

    let exporter = PdfExporter::new();
    Ok(exporter.data_product_to_markdown_public(&product))
}

/// Export a CADS Asset to Markdown format.
///
/// # Arguments
///
/// * `asset_json` - JSON string containing CADSAsset
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_cads_to_markdown(asset_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::PdfExporter;
    use data_modelling_core::models::cads::CADSAsset;

    let asset: CADSAsset = serde_json::from_str(asset_json).map_err(deserialization_error)?;

    let exporter = PdfExporter::new();
    Ok(exporter.cads_asset_to_markdown_public(&asset))
}

// ============================================================================
// Native Format PDF/Markdown Export (accepts YAML content directly)
// ============================================================================

/// Export ODCS YAML content to PDF format with optional branding.
///
/// Accepts raw ODCS YAML content (as you would find in an .odcs.yaml file)
/// and exports it to PDF.
///
/// # Arguments
///
/// * `odcs_yaml` - ODCS YAML content as a string
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_odcs_yaml_to_pdf(
    odcs_yaml: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::import::ODCSImporter;

    // Parse ODCS YAML to get tables
    let mut importer = ODCSImporter::new();
    let import_result = importer.import(odcs_yaml).map_err(import_error_to_js)?;

    if import_result.tables.is_empty() {
        return Err(
            WasmError::new("ImportError", "ODCS content contains no tables")
                .with_code("NO_TABLES")
                .to_js_value(),
        );
    }

    // Convert first table from import result to Table
    let table_data = &import_result.tables[0];
    let table = data_modelling_core::models::Table::from_table_data(table_data);

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter.export_table(&table).map_err(export_error_to_js)?;
    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export ODCS YAML content to Markdown format.
///
/// Accepts raw ODCS YAML content and exports it to Markdown.
///
/// # Arguments
///
/// * `odcs_yaml` - ODCS YAML content as a string
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_odcs_yaml_to_markdown(odcs_yaml: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::PdfExporter;
    use data_modelling_core::import::ODCSImporter;

    let mut importer = ODCSImporter::new();
    let import_result = importer.import(odcs_yaml).map_err(import_error_to_js)?;

    if import_result.tables.is_empty() {
        return Err(
            WasmError::new("ImportError", "ODCS content contains no tables")
                .with_code("NO_TABLES")
                .to_js_value(),
        );
    }

    let table_data = &import_result.tables[0];
    let table = data_modelling_core::models::Table::from_table_data(table_data);

    let exporter = PdfExporter::new();
    Ok(exporter.table_to_markdown_public(&table))
}

/// Export ODPS YAML content to PDF format with optional branding.
///
/// Accepts raw ODPS YAML content (as you would find in an .odps.yaml file)
/// and exports it to PDF.
///
/// # Arguments
///
/// * `odps_yaml` - ODPS YAML content as a string
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_odps_yaml_to_pdf(
    odps_yaml: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::import::ODPSImporter;

    let importer = ODPSImporter::new();
    let product = importer.import(odps_yaml).map_err(import_error_to_js)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_data_product(&product)
        .map_err(export_error_to_js)?;
    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export ODPS YAML content to Markdown format.
///
/// Accepts raw ODPS YAML content and exports it to Markdown.
///
/// # Arguments
///
/// * `odps_yaml` - ODPS YAML content as a string
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_odps_yaml_to_markdown(odps_yaml: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::PdfExporter;
    use data_modelling_core::import::ODPSImporter;

    let importer = ODPSImporter::new();
    let product = importer.import(odps_yaml).map_err(import_error_to_js)?;

    let exporter = PdfExporter::new();
    Ok(exporter.data_product_to_markdown_public(&product))
}

/// Export CADS YAML content to PDF format with optional branding.
///
/// Accepts raw CADS YAML content (as you would find in a .cads.yaml file)
/// and exports it to PDF.
///
/// # Arguments
///
/// * `cads_yaml` - CADS YAML content as a string
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_cads_yaml_to_pdf(
    cads_yaml: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::import::CADSImporter;

    let importer = CADSImporter::new();
    let asset = importer.import(cads_yaml).map_err(import_error_to_js)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_cads_asset(&asset)
        .map_err(export_error_to_js)?;
    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export CADS YAML content to Markdown format.
///
/// Accepts raw CADS YAML content and exports it to Markdown.
///
/// # Arguments
///
/// * `cads_yaml` - CADS YAML content as a string
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_cads_yaml_to_markdown(cads_yaml: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::PdfExporter;
    use data_modelling_core::import::CADSImporter;

    let importer = CADSImporter::new();
    let asset = importer.import(cads_yaml).map_err(import_error_to_js)?;

    let exporter = PdfExporter::new();
    Ok(exporter.cads_asset_to_markdown_public(&asset))
}

/// Export Decision YAML content to PDF format with optional branding.
///
/// Accepts raw Decision YAML content (as you would find in a .madr.yaml file)
/// and exports it to PDF.
///
/// # Arguments
///
/// * `decision_yaml` - Decision YAML content as a string
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_decision_yaml_to_pdf(
    decision_yaml: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::import::decision::DecisionImporter;

    let importer = DecisionImporter::new();
    let decision = importer
        .import_without_validation(decision_yaml)
        .map_err(import_error_to_js)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_decision(&decision)
        .map_err(export_error_to_js)?;
    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export Decision YAML content to Markdown format.
///
/// Accepts raw Decision YAML content and exports it to Markdown.
///
/// # Arguments
///
/// * `decision_yaml` - Decision YAML content as a string
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_decision_yaml_to_markdown(decision_yaml: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::markdown::MarkdownExporter;
    use data_modelling_core::import::decision::DecisionImporter;

    let importer = DecisionImporter::new();
    let decision = importer
        .import_without_validation(decision_yaml)
        .map_err(import_error_to_js)?;

    let exporter = MarkdownExporter::new();
    exporter
        .export_decision(&decision)
        .map_err(export_error_to_js)
}

/// Export Knowledge Article YAML content to PDF format with optional branding.
///
/// Accepts raw Knowledge Article YAML content (as you would find in a .kb.yaml file)
/// and exports it to PDF.
///
/// # Arguments
///
/// * `knowledge_yaml` - Knowledge Article YAML content as a string
/// * `branding_json` - Optional JSON string containing BrandingConfig
///
/// # Returns
///
/// JSON string containing PdfExportResult (with base64-encoded PDF), or JsValue error
#[wasm_bindgen]
pub fn export_knowledge_yaml_to_pdf(
    knowledge_yaml: &str,
    branding_json: Option<String>,
) -> Result<String, JsValue> {
    use data_modelling_core::export::pdf::{BrandingConfig, PdfExporter};
    use data_modelling_core::import::knowledge::KnowledgeImporter;

    let importer = KnowledgeImporter::new();
    let article = importer
        .import_without_validation(knowledge_yaml)
        .map_err(import_error_to_js)?;

    let exporter = if let Some(branding_str) = branding_json {
        let branding: BrandingConfig =
            serde_json::from_str(&branding_str).map_err(deserialization_error)?;
        PdfExporter::with_branding(branding)
    } else {
        PdfExporter::new()
    };

    let result = exporter
        .export_knowledge(&article)
        .map_err(export_error_to_js)?;
    serde_json::to_string(&result).map_err(serialization_error)
}

/// Export Knowledge Article YAML content to Markdown format.
///
/// Accepts raw Knowledge Article YAML content and exports it to Markdown.
///
/// # Arguments
///
/// * `knowledge_yaml` - Knowledge Article YAML content as a string
///
/// # Returns
///
/// Markdown string, or JsValue error
#[wasm_bindgen]
pub fn export_knowledge_yaml_to_markdown(knowledge_yaml: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::markdown::MarkdownExporter;
    use data_modelling_core::import::knowledge::KnowledgeImporter;

    let importer = KnowledgeImporter::new();
    let article = importer
        .import_without_validation(knowledge_yaml)
        .map_err(import_error_to_js)?;

    let exporter = MarkdownExporter::new();
    exporter
        .export_knowledge(&article)
        .map_err(export_error_to_js)
}

// ============================================================================
// ODCS v2 API - Native Data Structures
// ============================================================================
// These functions work with ODCSContract directly, preserving all metadata
// and nested structures without flattening to Table/Column types.

/// Parse ODCS YAML content and return an ODCSContract JSON representation (v2 API).
///
/// This is the preferred v2 API that returns the native ODCSContract structure,
/// preserving all ODCS metadata, nested properties, and multi-table support.
///
/// Unlike `parse_odcs_yaml` which flattens to Table/Column types, this function
/// returns the full ODCSContract with:
/// - All contract-level fields (apiVersion, domain, team, etc.)
/// - Multiple schema objects (tables) preserved
/// - Nested properties (OBJECT/ARRAY types) preserved hierarchically
/// - All custom properties at each level
///
/// # Arguments
///
/// * `yaml_content` - ODCS YAML content as a string
///
/// # Returns
///
/// JSON string containing ODCSContract object, or JsValue error
///
/// # Example Response
///
/// ```json
/// {
///   "apiVersion": "v3.1.0",
///   "kind": "DataContract",
///   "name": "My Contract",
///   "schema": [
///     {
///       "name": "users",
///       "properties": [
///         { "name": "id", "logicalType": "integer", "primaryKey": true },
///         { "name": "address", "logicalType": "object", "properties": [...] }
///       ]
///     }
///   ]
/// }
/// ```
#[wasm_bindgen]
pub fn parse_odcs_yaml_v2(yaml_content: &str) -> Result<String, JsValue> {
    let mut importer = data_modelling_core::import::ODCSImporter::new();
    match importer.import_contract(yaml_content) {
        Ok(contract) => serde_json::to_string(&contract).map_err(serialization_error),
        Err(err) => Err(import_error_to_js(err)),
    }
}

/// Export an ODCSContract JSON to ODCS YAML format (v2 API).
///
/// This is the preferred v2 API that directly serializes an ODCSContract struct,
/// preserving all metadata and nested structures without reconstruction.
///
/// Unlike `export_to_odcs_yaml` which takes a workspace and reconstructs the ODCS
/// structure, this function directly serializes the provided contract.
///
/// # Arguments
///
/// * `contract_json` - JSON string containing ODCSContract object
///
/// # Returns
///
/// ODCS YAML format string, or JsValue error
///
/// # Example
///
/// ```javascript
/// const contract = {
///   apiVersion: "v3.1.0",
///   kind: "DataContract",
///   name: "My Contract",
///   schema: [{
///     name: "users",
///     properties: [
///       { name: "id", logicalType: "integer", primaryKey: true }
///     ]
///   }]
/// };
/// const yaml = export_odcs_yaml_v2(JSON.stringify(contract));
/// ```
#[wasm_bindgen]
pub fn export_odcs_yaml_v2(contract_json: &str) -> Result<String, JsValue> {
    let contract: data_modelling_core::models::odcs::ODCSContract =
        serde_json::from_str(contract_json).map_err(deserialization_error)?;
    Ok(data_modelling_core::export::ODCSExporter::export_contract(
        &contract,
    ))
}

/// Convert an ODCSContract to Table/Column format (v2 API).
///
/// This function converts an ODCSContract to the traditional Table/Column
/// format used by the v1 API. Useful for interoperability with existing code.
///
/// Note: This conversion may lose some metadata that doesn't fit in Table/Column.
/// For lossless round-trip, use `parse_odcs_yaml_v2` and `export_odcs_yaml_v2`.
///
/// # Arguments
///
/// * `contract_json` - JSON string containing ODCSContract object
///
/// # Returns
///
/// JSON string containing array of Table objects, or JsValue error
#[wasm_bindgen]
pub fn odcs_contract_to_tables(contract_json: &str) -> Result<String, JsValue> {
    let contract: data_modelling_core::models::odcs::ODCSContract =
        serde_json::from_str(contract_json).map_err(deserialization_error)?;
    let tables = contract.to_tables();
    serde_json::to_string(&tables).map_err(serialization_error)
}

/// Convert Tables to an ODCSContract (v2 API).
///
/// This function converts traditional Table/Column format to an ODCSContract.
/// Useful for migrating existing code to the v2 API.
///
/// Note: The resulting contract will have default values for fields that
/// don't exist in Table/Column format.
///
/// # Arguments
///
/// * `tables_json` - JSON string containing array of Table objects
///
/// # Returns
///
/// JSON string containing ODCSContract object, or JsValue error
#[wasm_bindgen]
pub fn tables_to_odcs_contract(tables_json: &str) -> Result<String, JsValue> {
    let tables: Vec<data_modelling_core::models::Table> =
        serde_json::from_str(tables_json).map_err(deserialization_error)?;
    let contract = data_modelling_core::models::odcs::ODCSContract::from_tables(&tables);
    serde_json::to_string(&contract).map_err(serialization_error)
}

/// Convert an ODCSContract to TableData format for UI rendering (v2 API).
///
/// This function converts an ODCSContract to TableData format, which includes
/// all the metadata needed for UI rendering (similar to ImportResult.tables).
///
/// # Arguments
///
/// * `contract_json` - JSON string containing ODCSContract object
///
/// # Returns
///
/// JSON string containing array of TableData objects, or JsValue error
#[wasm_bindgen]
pub fn odcs_contract_to_table_data(contract_json: &str) -> Result<String, JsValue> {
    let contract: data_modelling_core::models::odcs::ODCSContract =
        serde_json::from_str(contract_json).map_err(deserialization_error)?;
    let table_data = contract.to_table_data();
    serde_json::to_string(&table_data).map_err(serialization_error)
}

// ============================================================================
// Sketch (Excalidraw) Operations
// ============================================================================

/// Parse a sketch YAML file and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Sketch YAML content as a string (.sketch.yaml)
///
/// # Returns
///
/// JSON string containing Sketch, or JsValue error
#[wasm_bindgen]
pub fn parse_sketch_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::import::sketch::SketchImporter;

    let importer = SketchImporter::new();
    match importer.import(yaml_content) {
        Ok(sketch) => serde_json::to_string(&sketch).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(e)),
    }
}

/// Parse a sketches index YAML file and return a structured representation.
///
/// # Arguments
///
/// * `yaml_content` - Sketches index YAML content as a string (sketches.yaml)
///
/// # Returns
///
/// JSON string containing SketchIndex, or JsValue error
#[wasm_bindgen]
pub fn parse_sketch_index_yaml(yaml_content: &str) -> Result<String, JsValue> {
    use data_modelling_core::import::sketch::SketchImporter;

    let importer = SketchImporter::new();
    match importer.import_index(yaml_content) {
        Ok(index) => serde_json::to_string(&index).map_err(serialization_error),
        Err(e) => Err(import_error_to_js(e)),
    }
}

/// Export a sketch to YAML format.
///
/// # Arguments
///
/// * `sketch_json` - JSON string containing Sketch
///
/// # Returns
///
/// Sketch YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_sketch_to_yaml(sketch_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::sketch::SketchExporter;
    use data_modelling_core::models::sketch::Sketch;

    let sketch: Sketch = serde_json::from_str(sketch_json).map_err(deserialization_error)?;
    let exporter = SketchExporter::new();
    exporter
        .export_without_validation(&sketch)
        .map_err(export_error_to_js)
}

/// Export a sketches index to YAML format.
///
/// # Arguments
///
/// * `index_json` - JSON string containing SketchIndex
///
/// # Returns
///
/// SketchIndex YAML format string, or JsValue error
#[wasm_bindgen]
pub fn export_sketch_index_to_yaml(index_json: &str) -> Result<String, JsValue> {
    use data_modelling_core::export::sketch::SketchExporter;
    use data_modelling_core::models::sketch::SketchIndex;

    let index: SketchIndex = serde_json::from_str(index_json).map_err(deserialization_error)?;
    let exporter = SketchExporter::new();
    exporter.export_index(&index).map_err(export_error_to_js)
}

/// Create a new sketch with required fields.
///
/// # Arguments
///
/// * `number` - Sketch number (1, 2, 3, etc. - will be formatted as SKETCH-0001)
/// * `title` - Sketch title
/// * `sketch_type` - Sketch type (architecture, dataFlow, entityRelationship, sequence, flowchart, wireframe, concept, infrastructure, other)
/// * `excalidraw_data` - JSON string of Excalidraw scene data
///
/// # Returns
///
/// JSON string containing Sketch, or JsValue error
#[wasm_bindgen]
pub fn create_sketch(
    number: u64,
    title: &str,
    sketch_type: &str,
    excalidraw_data: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::sketch::{Sketch, SketchType};

    let stype = match sketch_type.to_lowercase().as_str() {
        "architecture" => SketchType::Architecture,
        "dataflow" | "data_flow" => SketchType::DataFlow,
        "entityrelationship" | "entity_relationship" | "er" => SketchType::EntityRelationship,
        "sequence" => SketchType::Sequence,
        "flowchart" => SketchType::Flowchart,
        "wireframe" => SketchType::Wireframe,
        "concept" => SketchType::Concept,
        "infrastructure" => SketchType::Infrastructure,
        _ => SketchType::Other,
    };

    let sketch = Sketch::new(number, title, excalidraw_data).with_type(stype);
    serde_json::to_string(&sketch).map_err(serialization_error)
}

/// Create a new empty sketch index.
///
/// # Returns
///
/// JSON string containing SketchIndex, or JsValue error
#[wasm_bindgen]
pub fn create_sketch_index() -> Result<String, JsValue> {
    use data_modelling_core::models::sketch::SketchIndex;

    let index = SketchIndex::new();
    serde_json::to_string(&index).map_err(serialization_error)
}

/// Add a sketch to an index.
///
/// # Arguments
///
/// * `index_json` - JSON string containing SketchIndex
/// * `sketch_json` - JSON string containing Sketch
/// * `filename` - Filename for the sketch YAML file
///
/// # Returns
///
/// JSON string containing updated SketchIndex, or JsValue error
#[wasm_bindgen]
pub fn add_sketch_to_index(
    index_json: &str,
    sketch_json: &str,
    filename: &str,
) -> Result<String, JsValue> {
    use data_modelling_core::models::sketch::{Sketch, SketchIndex};

    let mut index: SketchIndex = serde_json::from_str(index_json).map_err(deserialization_error)?;
    let sketch: Sketch = serde_json::from_str(sketch_json).map_err(deserialization_error)?;

    index.add_sketch(&sketch, filename.to_string());
    serde_json::to_string(&index).map_err(serialization_error)
}

/// Search sketches by title, description, or tags.
///
/// # Arguments
///
/// * `sketches_json` - JSON string containing array of Sketch
/// * `query` - Search query string (case-insensitive)
///
/// # Returns
///
/// JSON string containing array of matching Sketch, or JsValue error
#[wasm_bindgen]
pub fn search_sketches(sketches_json: &str, query: &str) -> Result<String, JsValue> {
    use data_modelling_core::models::sketch::Sketch;

    let sketches: Vec<Sketch> =
        serde_json::from_str(sketches_json).map_err(deserialization_error)?;

    let query_lower = query.to_lowercase();
    let matches: Vec<&Sketch> = sketches
        .iter()
        .filter(|sketch| {
            sketch.title.to_lowercase().contains(&query_lower)
                || sketch
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query_lower))
                    .unwrap_or(false)
                || sketch
                    .tags
                    .iter()
                    .any(|tag| tag.to_string().to_lowercase().contains(&query_lower))
        })
        .collect();

    serde_json::to_string(&matches).map_err(serialization_error)
}
