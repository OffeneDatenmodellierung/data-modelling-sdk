//! Universal format converter
//!
//! Converts any import format to ODCS v3.1.0 format.

use crate::export::{ExportError, ODCSExporter};
use crate::import::{
    AvroImporter, CADSImporter, ColumnData, ImportError, ImportResult, JSONSchemaImporter,
    ODCSImporter, ODPSImporter, ProtobufImporter, SQLImporter, TableData,
};
use crate::models::{Column, DataModel, Domain, Table};

/// Error during format conversion
#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error("Import error: {0}")]
    ImportError(#[from] ImportError),
    #[error("Export error: {0}")]
    ExportError(#[from] ExportError),
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),
    #[error("Auto-detection failed: {0}")]
    AutoDetectionFailed(String),
    #[error("OpenAPI to ODCS conversion error: {0}")]
    OpenAPIToODCSError(String),
    #[error("OpenAPI component not found: {0}")]
    OpenAPIComponentNotFound(String),
    #[error("OpenAPI schema invalid: {0}")]
    OpenAPISchemaInvalid(String),
    #[error("Nested object conversion failed: {0}")]
    NestedObjectConversionFailed(String),
}

/// Parse STRUCT type columns into nested columns with dot notation
fn parse_struct_columns(parent_name: &str, data_type: &str, col_data: &ColumnData) -> Vec<Column> {
    let importer = ODCSImporter::new();

    // Try to parse STRUCT type using ODCS importer's logic
    let field_data = serde_json::Map::new();

    match importer.parse_struct_type_from_string(parent_name, data_type, &field_data) {
        Ok(nested_cols) if !nested_cols.is_empty() => {
            let mut all_cols = Vec::new();

            // Add parent column with simplified type
            let parent_data_type = if data_type.to_uppercase().starts_with("ARRAY<") {
                "ARRAY<STRUCT<...>>".to_string()
            } else {
                "STRUCT<...>".to_string()
            };

            all_cols.push(Column {
                name: parent_name.to_string(),
                data_type: parent_data_type,
                physical_type: col_data.physical_type.clone(),
                nullable: col_data.nullable,
                primary_key: col_data.primary_key,
                description: col_data.description.clone().unwrap_or_default(),
                quality: col_data.quality.clone().unwrap_or_default(),
                relationships: col_data.relationships.clone(),
                enum_values: col_data.enum_values.clone().unwrap_or_default(),
                ..Default::default()
            });

            // Add nested columns
            all_cols.extend(nested_cols);
            all_cols
        }
        _ => Vec::new(),
    }
}

/// Reconstruct a Table from TableData
///
/// Converts import-format TableData/ColumnData into full Table/Column structs
/// suitable for export operations. Handles STRUCT types by flattening them
/// into nested columns with dot notation:
/// - STRUCT<...> → parent.field
/// - ARRAY<STRUCT<...>> → parent.[].field
/// - MAP types are kept as-is (keys are dynamic)
fn table_data_to_table(table_data: &TableData) -> Table {
    let table_name = table_data
        .name
        .clone()
        .unwrap_or_else(|| format!("table_{}", table_data.table_index));

    let mut all_columns = Vec::new();

    for col_data in &table_data.columns {
        let data_type_upper = col_data.data_type.to_uppercase();
        let is_map = data_type_upper.starts_with("MAP<");

        // Skip parsing for MAP types - keys are dynamic
        if is_map {
            all_columns.push(column_data_to_column(col_data));
            continue;
        }

        // For STRUCT or ARRAY<STRUCT> types, try to parse and create nested columns
        let is_struct = data_type_upper.contains("STRUCT<");
        if is_struct {
            let struct_cols = parse_struct_columns(&col_data.name, &col_data.data_type, col_data);
            if !struct_cols.is_empty() {
                all_columns.extend(struct_cols);
                continue;
            }
        }

        // Regular column or STRUCT parsing failed - add as-is
        all_columns.push(column_data_to_column(col_data));
    }

    Table::new(table_name, all_columns)
}

/// Convert ColumnData to Column, preserving ALL ODCS v3.1.0 fields
fn column_data_to_column(col_data: &ColumnData) -> Column {
    Column {
        // Core Identity
        id: col_data.id.clone(),
        name: col_data.name.clone(),
        business_name: col_data.business_name.clone(),
        description: col_data.description.clone().unwrap_or_default(),
        // Type Information
        data_type: col_data.data_type.clone(),
        physical_type: col_data.physical_type.clone(),
        physical_name: col_data.physical_name.clone(),
        logical_type_options: col_data.logical_type_options.clone(),
        // Key Constraints
        primary_key: col_data.primary_key,
        primary_key_position: col_data.primary_key_position,
        unique: col_data.unique,
        nullable: col_data.nullable,
        // Partitioning & Clustering
        partitioned: col_data.partitioned,
        partition_key_position: col_data.partition_key_position,
        clustered: col_data.clustered,
        // Data Classification & Security
        classification: col_data.classification.clone(),
        critical_data_element: col_data.critical_data_element,
        encrypted_name: col_data.encrypted_name.clone(),
        // Transformation Metadata
        transform_source_objects: col_data.transform_source_objects.clone(),
        transform_logic: col_data.transform_logic.clone(),
        transform_description: col_data.transform_description.clone(),
        // Examples & Documentation
        examples: col_data.examples.clone(),
        default_value: col_data.default_value.clone(),
        // Relationships & References
        relationships: col_data.relationships.clone(),
        authoritative_definitions: col_data.authoritative_definitions.clone(),
        // Quality & Validation
        quality: col_data.quality.clone().unwrap_or_default(),
        enum_values: col_data.enum_values.clone().unwrap_or_default(),
        // Tags & Custom Properties
        tags: col_data.tags.clone(),
        custom_properties: col_data.custom_properties.clone(),
        // Legacy/Internal Fields - use defaults
        ..Default::default()
    }
}

/// Reconstruct full Table structs from ImportResult
///
/// This function converts the flat TableData/ColumnData structures from imports
/// into complete Table/Column model structs that can be used for export.
pub fn reconstruct_tables(import_result: &ImportResult) -> Vec<Table> {
    import_result
        .tables
        .iter()
        .map(table_data_to_table)
        .collect()
}

/// Convert any import format to ODCS v3.1.0 YAML format.
///
/// # Arguments
///
/// * `input` - Format-specific content as a string
/// * `format` - Optional format identifier. If None, attempts auto-detection.
///   Supported formats: "sql", "json_schema", "avro", "protobuf", "odcl", "odcs", "cads", "odps", "domain"
///
/// # Returns
///
/// ODCS v3.1.0 YAML string, or ConversionError
pub fn convert_to_odcs(input: &str, format: Option<&str>) -> Result<String, ConversionError> {
    // Determine format (auto-detect if not specified)
    let detected_format = if let Some(fmt) = format {
        fmt
    } else {
        auto_detect_format(input)?
    };

    // Import using appropriate importer
    let import_result = match detected_format {
        "odcs" => {
            let mut importer = ODCSImporter::new();
            importer
                .import(input)
                .map_err(ConversionError::ImportError)?
        }
        "odcl" => {
            let mut importer = ODCSImporter::new();
            importer
                .import(input)
                .map_err(ConversionError::ImportError)?
        }
        "sql" => {
            let importer = SQLImporter::new("postgresql");
            importer
                .parse(input)
                .map_err(|e| ConversionError::ImportError(ImportError::ParseError(e.to_string())))?
        }
        "json_schema" => {
            let importer = JSONSchemaImporter::new();
            importer
                .import(input)
                .map_err(ConversionError::ImportError)?
        }
        "avro" => {
            let importer = AvroImporter::new();
            importer
                .import(input)
                .map_err(ConversionError::ImportError)?
        }
        "protobuf" => {
            let importer = ProtobufImporter::new();
            importer
                .import(input)
                .map_err(ConversionError::ImportError)?
        }
        "cads" => {
            // CADS assets are compute assets, not data contracts
            // For CADS → ODCS conversion, we create a minimal ODCS representation
            // that captures metadata but doesn't represent a true data contract
            // This is a placeholder - full conversion would require understanding
            // the data schema produced by the CADS asset
            let importer = CADSImporter::new();
            let _asset = importer
                .import(input)
                .map_err(ConversionError::ImportError)?;

            // For now, return an error indicating CADS → ODCS conversion
            // requires additional context about the data schema
            return Err(ConversionError::UnsupportedFormat(
                "CADS → ODCS conversion requires data schema information. CADS assets represent compute resources, not data contracts.".to_string()
            ));
        }
        "odps" => {
            // ODPS Data Products link to ODCS Tables via contractId
            // For ODPS → ODCS conversion, we extract the referenced ODCS Tables
            // from the input/output ports and export them
            let importer = ODPSImporter::new();
            let product = importer
                .import(input)
                .map_err(ConversionError::ImportError)?;

            // Extract contractIds from input and output ports
            let mut contract_ids = Vec::new();
            if let Some(input_ports) = &product.input_ports {
                for port in input_ports {
                    contract_ids.push(port.contract_id.clone());
                }
            }
            if let Some(output_ports) = &product.output_ports {
                for port in output_ports {
                    if let Some(contract_id) = &port.contract_id {
                        contract_ids.push(contract_id.clone());
                    }
                }
            }

            if contract_ids.is_empty() {
                return Err(ConversionError::UnsupportedFormat(
                    "ODPS → ODCS conversion requires contractId references. No contractIds found in input/output ports.".to_string()
                ));
            }

            // For now, return an error indicating that ODPS → ODCS conversion
            // requires the actual ODCS Table definitions to be provided
            // In a full implementation, we would look up the ODCS Tables by contractId
            return Err(ConversionError::UnsupportedFormat(format!(
                "ODPS → ODCS conversion requires ODCS Table definitions for contractIds: {}. Please provide the referenced ODCS Tables.",
                contract_ids.join(", ")
            )));
        }
        "domain" => {
            // Domain schema stores references to ODCS Tables (ODCSNode with table_id)
            // but doesn't contain the full Table definitions
            // For Domain → ODCS conversion, we need the actual Table definitions
            let domain: Domain = serde_yaml::from_str(input).map_err(|e| {
                ConversionError::ImportError(ImportError::ParseError(format!(
                    "Failed to parse Domain YAML: {}",
                    e
                )))
            })?;

            // Extract ODCS node references
            let odcs_node_count = domain.odcs_nodes.len();
            if odcs_node_count == 0 {
                return Err(ConversionError::UnsupportedFormat(
                    "Domain → ODCS conversion: Domain contains no ODCS nodes.".to_string(),
                ));
            }

            // Domain schema only stores references, not full Table definitions
            // To convert Domain → ODCS, we need the actual Table definitions
            // This would require looking up Tables by table_id from a DataModel or similar
            return Err(ConversionError::UnsupportedFormat(format!(
                "Domain → ODCS conversion requires Table definitions. Domain contains {} ODCS node references, but full Table definitions must be provided separately (e.g., from a DataModel).",
                odcs_node_count
            )));
        }
        _ => {
            return Err(ConversionError::UnsupportedFormat(
                detected_format.to_string(),
            ));
        }
    };

    // Check for empty input
    if import_result.tables.is_empty() {
        return Err(ConversionError::ImportError(ImportError::ParseError(
            "No tables found in input".to_string(),
        )));
    }

    // Reconstruct full Table structs from ImportResult
    let tables = reconstruct_tables(&import_result);

    // Export each table to ODCS format
    let yaml_docs: Vec<String> = tables
        .iter()
        .map(|table| ODCSExporter::export_table(table, "odcs_v3_1_0"))
        .collect();

    Ok(yaml_docs.join("\n---\n"))
}

/// Convert ImportResult to a DataModel with fully reconstructed Tables
///
/// This is useful when you need the full DataModel structure after import,
/// rather than just the YAML output.
pub fn import_result_to_data_model(
    import_result: &ImportResult,
    model_name: &str,
) -> Result<DataModel, ConversionError> {
    if import_result.tables.is_empty() {
        return Err(ConversionError::ImportError(ImportError::ParseError(
            "No tables found in import result".to_string(),
        )));
    }

    let tables = reconstruct_tables(import_result);

    let mut model = DataModel::new(model_name.to_string(), String::new(), String::new());

    for table in tables {
        model.tables.push(table);
    }

    Ok(model)
}

/// Auto-detect format from input content
fn auto_detect_format(input: &str) -> Result<&str, ConversionError> {
    // Check for ODCS format
    if input.contains("apiVersion:") && input.contains("kind: DataContract") {
        return Ok("odcs");
    }

    // Check for ODCL format
    if input.contains("dataContractSpecification:") {
        return Ok("odcl");
    }

    // Check for SQL format
    if input.to_uppercase().contains("CREATE TABLE") {
        return Ok("sql");
    }

    // Check for JSON Schema format
    if input.trim_start().starts_with('{')
        && (input.contains("\"$schema\"") || input.contains("\"type\""))
    {
        return Ok("json_schema");
    }

    // Check for AVRO format
    if input.contains("\"type\"") && input.contains("\"fields\"") && input.contains("\"name\"") {
        return Ok("avro");
    }

    // Check for Protobuf format
    if input.contains("syntax") || input.contains("message") || input.contains("service") {
        return Ok("protobuf");
    }

    // Check for CADS format
    if input.contains("apiVersion:")
        && (input.contains("kind: AIModel")
            || input.contains("kind: MLPipeline")
            || input.contains("kind: Application")
            || input.contains("kind: ETLPipeline")
            || input.contains("kind: SourceSystem")
            || input.contains("kind: DestinationSystem"))
    {
        return Ok("cads");
    }

    // Check for ODPS format
    if input.contains("apiVersion:") && input.contains("kind: DataProduct") {
        return Ok("odps");
    }

    // Check for Domain format (Business Domain schema)
    if input.contains("systems:")
        && (input.contains("cads_nodes:") || input.contains("odcs_nodes:"))
    {
        return Ok("domain");
    }

    Err(ConversionError::AutoDetectionFailed(
        "Could not auto-detect format. Please specify format explicitly.".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reconstruct_tables_from_import_result() {
        let import_result = ImportResult {
            tables: vec![TableData {
                table_index: 0,
                name: Some("users".to_string()),
                columns: vec![
                    ColumnData {
                        name: "id".to_string(),
                        data_type: "INTEGER".to_string(),
                        nullable: false,
                        primary_key: true,
                        description: Some("User ID".to_string()),
                        ..Default::default()
                    },
                    ColumnData {
                        name: "name".to_string(),
                        data_type: "VARCHAR(100)".to_string(),
                        nullable: true,
                        ..Default::default()
                    },
                ],
            }],
            tables_requiring_name: vec![],
            errors: vec![],
            ai_suggestions: None,
        };

        let tables = reconstruct_tables(&import_result);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "users");
        assert_eq!(tables[0].columns.len(), 2);
        assert_eq!(tables[0].columns[0].name, "id");
        assert!(tables[0].columns[0].primary_key);
        assert_eq!(tables[0].columns[0].description, "User ID");
    }

    #[test]
    fn test_convert_sql_to_odcs() {
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name VARCHAR(100));";
        let result = convert_to_odcs(sql, Some("sql"));
        assert!(result.is_ok());
        let yaml = result.unwrap();
        assert!(yaml.contains("kind: DataContract"));
        assert!(yaml.contains("users"));
    }

    #[test]
    fn test_auto_detect_sql() {
        let sql = "CREATE TABLE test (id INT);";
        let format = auto_detect_format(sql);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), "sql");
    }

    #[test]
    fn test_auto_detect_odcs() {
        let odcs = "apiVersion: v3.1.0\nkind: DataContract\n";
        let format = auto_detect_format(odcs);
        assert!(format.is_ok());
        assert_eq!(format.unwrap(), "odcs");
    }

    #[test]
    fn test_import_result_to_data_model() {
        let import_result = ImportResult {
            tables: vec![TableData {
                table_index: 0,
                name: Some("orders".to_string()),
                columns: vec![ColumnData {
                    name: "order_id".to_string(),
                    data_type: "UUID".to_string(),
                    nullable: false,
                    primary_key: true,
                    ..Default::default()
                }],
            }],
            tables_requiring_name: vec![],
            errors: vec![],
            ai_suggestions: None,
        };

        let model = import_result_to_data_model(&import_result, "test_model");
        assert!(model.is_ok());
        let model = model.unwrap();
        assert_eq!(model.name, "test_model");
        assert_eq!(model.tables.len(), 1);
        assert_eq!(model.tables[0].name, "orders");
    }
}
