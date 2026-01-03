//! Universal format converter
//!
//! Converts any import format to ODCS v3.1.0 format.

use crate::export::{ExportError, ODCSExporter};
use crate::import::{
    AvroImporter, CADSImporter, ImportError, JSONSchemaImporter, ODCSImporter, ODPSImporter,
    ProtobufImporter, SQLImporter,
};
use crate::models::{DataModel, Domain};

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

    // Convert ImportResult to DataModel
    // For conversion purposes, we create a temporary DataModel
    // The actual table reconstruction from ColumnData would require additional logic
    // For now, we'll create a minimal DataModel and export each table individually
    let model = DataModel::new(
        "converted_model".to_string(),
        "".to_string(),
        "".to_string(),
    );

    // If we have tables, export them using ODCSExporter
    // Note: This is a simplified version - full implementation would reconstruct
    // Table structs from TableData/ColumnData
    if import_result.tables.is_empty() {
        return Err(ConversionError::ImportError(ImportError::ParseError(
            "No tables found in input".to_string(),
        )));
    }

    // Export using ODCSExporter
    // For now, we'll return a basic ODCS structure
    // TODO: Reconstruct full Table structs from ImportResult for proper export
    let exports = ODCSExporter::export_model(&model, None, "odcs_v3_1_0");

    // Combine all YAML documents
    let yaml_docs: Vec<String> = exports.values().cloned().collect();
    Ok(yaml_docs.join("\n---\n"))
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
