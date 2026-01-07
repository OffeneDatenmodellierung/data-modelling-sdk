//! Export command handlers

use crate::cli::error::CliError;
use crate::export::{AvroExporter, JSONSchemaExporter, ODCSExporter, ProtobufExporter};
use std::path::PathBuf;
use std::process::Command;

/// Export format enum
#[derive(Debug, Clone)]
pub enum ExportFormat {
    Odcs,
    Avro,
    JsonSchema,
    Protobuf,
    ProtobufDescriptor,
    Odps,
}

/// Arguments for export operations
#[derive(Debug, Clone)]
pub struct ExportArgs {
    pub format: ExportFormat,
    pub input: PathBuf,
    pub output: PathBuf,
    pub force: bool,
    pub protoc_path: Option<PathBuf>,
    pub protobuf_version: Option<String>, // "proto2" or "proto3" (default: proto3)
}

/// Load tables from ODCS YAML file(s)
pub fn load_tables_from_odcs(input_path: &PathBuf) -> Result<Vec<crate::models::Table>, CliError> {
    use crate::import::ODCSImporter;

    let content = std::fs::read_to_string(input_path)
        .map_err(|e| CliError::FileReadError(input_path.clone(), e.to_string()))?;

    // Import ODCS file
    let mut importer = ODCSImporter::new();
    let import_result = importer
        .import(&content)
        .map_err(|e| CliError::InvalidArgument(format!("Failed to import ODCS file: {}", e)))?;

    // Convert ImportResult to Vec<Table>
    let mut tables = Vec::new();
    for table_data in import_result.tables {
        let table_name = table_data
            .name
            .unwrap_or_else(|| format!("table_{}", table_data.table_index));

        let columns: Vec<crate::models::Column> = table_data
            .columns
            .iter()
            .map(|col_data| crate::models::Column {
                id: col_data.id.clone(),
                name: col_data.name.clone(),
                business_name: col_data.business_name.clone(),
                description: col_data.description.clone().unwrap_or_default(),
                data_type: col_data.data_type.clone(),
                physical_type: col_data.physical_type.clone(),
                physical_name: col_data.physical_name.clone(),
                logical_type_options: col_data.logical_type_options.clone(),
                primary_key: col_data.primary_key,
                primary_key_position: col_data.primary_key_position,
                unique: col_data.unique,
                nullable: col_data.nullable,
                partitioned: col_data.partitioned,
                partition_key_position: col_data.partition_key_position,
                clustered: col_data.clustered,
                classification: col_data.classification.clone(),
                critical_data_element: col_data.critical_data_element,
                encrypted_name: col_data.encrypted_name.clone(),
                transform_source_objects: col_data.transform_source_objects.clone(),
                transform_logic: col_data.transform_logic.clone(),
                transform_description: col_data.transform_description.clone(),
                examples: col_data.examples.clone(),
                default_value: col_data.default_value.clone(),
                relationships: col_data.relationships.clone(),
                authoritative_definitions: col_data.authoritative_definitions.clone(),
                quality: col_data.quality.clone().unwrap_or_default(),
                enum_values: col_data.enum_values.clone().unwrap_or_default(),
                tags: col_data.tags.clone(),
                custom_properties: col_data.custom_properties.clone(),
                ..Default::default()
            })
            .collect();

        tables.push(crate::models::Table::new(table_name, columns));
    }

    Ok(tables)
}

/// Check if file exists and handle overwrite
pub fn check_file_overwrite(output_path: &std::path::Path, force: bool) -> Result<(), CliError> {
    if output_path.exists() && !force {
        return Err(CliError::InvalidArgument(format!(
            "Output file exists: {}. Use --force to overwrite.",
            output_path.display()
        )));
    }
    Ok(())
}

/// Write export output to file
pub fn write_export_output(output_path: &PathBuf, content: &str) -> Result<(), CliError> {
    // Create parent directories if needed
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            CliError::FileWriteError(
                output_path.clone(),
                format!("Failed to create directory: {}", e),
            )
        })?;
    }

    std::fs::write(output_path, content)
        .map_err(|e| CliError::FileWriteError(output_path.clone(), e.to_string()))
}

/// Handle ODCS export command
pub fn handle_export_odcs(args: &ExportArgs) -> Result<(), CliError> {
    // Check overwrite
    check_file_overwrite(&args.output, args.force)?;

    // Load tables from ODCS YAML file
    let tables = load_tables_from_odcs(&args.input)?;

    // Export to ODCS
    let exporter = ODCSExporter;
    let exports = exporter
        .export(&tables, "odcs_v3_1_0")
        .map_err(CliError::ExportError)?;

    // Write each table to separate file or combine
    let export_count = exports.len();
    if export_count == 1 {
        let (_, result) = exports.iter().next().unwrap();
        write_export_output(&args.output, &result.content)?;
        println!("✅ Exported 1 table to {}", args.output.display());
    } else {
        // Export each table to separate file
        let default_dir = PathBuf::from(".");
        for (_table_name, result) in exports {
            let mut output_path = args.output.clone();
            if let Some(stem) = output_path.file_stem() {
                let dir = output_path.parent().unwrap_or(&default_dir);
                output_path = dir.join(format!("{}.odcs.yaml", stem.to_string_lossy()));
            }
            write_export_output(&output_path, &result.content)?;
        }
        println!("✅ Exported {} tables to separate files", export_count);
    }

    Ok(())
}

/// Handle AVRO export command
pub fn handle_export_avro(args: &ExportArgs) -> Result<(), CliError> {
    check_file_overwrite(&args.output, args.force)?;

    let tables = load_tables_from_odcs(&args.input)?;

    let exporter = AvroExporter;
    let result = exporter.export(&tables).map_err(CliError::ExportError)?;

    write_export_output(&args.output, &result.content)?;
    println!("✅ Exported to AVRO format: {}", args.output.display());

    Ok(())
}

/// Handle JSON Schema export command
pub fn handle_export_json_schema(args: &ExportArgs) -> Result<(), CliError> {
    check_file_overwrite(&args.output, args.force)?;

    let tables = load_tables_from_odcs(&args.input)?;

    let exporter = JSONSchemaExporter;
    let result = exporter.export(&tables).map_err(CliError::ExportError)?;

    write_export_output(&args.output, &result.content)?;
    println!(
        "✅ Exported to JSON Schema format: {}",
        args.output.display()
    );

    Ok(())
}

/// Handle Protobuf export command
pub fn handle_export_protobuf(args: &ExportArgs) -> Result<(), CliError> {
    check_file_overwrite(&args.output, args.force)?;

    let tables = load_tables_from_odcs(&args.input)?;

    // Validate protobuf version
    let version = args.protobuf_version.as_deref().unwrap_or("proto3");
    if version != "proto2" && version != "proto3" {
        return Err(CliError::InvalidArgument(format!(
            "Invalid protobuf version: {}. Must be 'proto2' or 'proto3'",
            version
        )));
    }

    let exporter = ProtobufExporter;
    let result = exporter
        .export_with_version(&tables, version)
        .map_err(CliError::ExportError)?;

    write_export_output(&args.output, &result.content)?;
    println!("✅ Exported to Protobuf format: {}", args.output.display());

    Ok(())
}

/// Check if protoc is available
pub fn check_protoc_available(protoc_path: Option<&PathBuf>) -> Result<(), CliError> {
    let protoc = protoc_path
        .map(|p| p.as_os_str())
        .unwrap_or_else(|| std::ffi::OsStr::new("protoc"));

    let output = Command::new(protoc).arg("--version").output();

    match output {
        Ok(result) => {
            if result.status.success() {
                Ok(())
            } else {
                Err(CliError::ProtocNotFound)
            }
        }
        Err(_) => Err(CliError::ProtocNotFound),
    }
}

/// Generate Protobuf descriptor file using protoc
pub fn generate_protobuf_descriptor(
    proto_file: &std::path::Path,
    output_file: &std::path::Path,
    protoc_path: Option<&PathBuf>,
) -> Result<(), CliError> {
    let protoc = protoc_path
        .map(|p| p.as_os_str())
        .unwrap_or_else(|| std::ffi::OsStr::new("protoc"));

    let output = Command::new(protoc)
        .arg("--include_imports")
        .arg("--include_source_info")
        .arg(format!("--descriptor_set_out={}", output_file.display()))
        .arg(proto_file.as_os_str())
        .output()
        .map_err(|e| CliError::ProtocError(format!("Failed to execute protoc: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CliError::ProtocError(format!(
            "protoc compilation failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Handle Protobuf descriptor export command
pub fn handle_export_protobuf_descriptor(args: &ExportArgs) -> Result<(), CliError> {
    check_file_overwrite(&args.output, args.force)?;

    // Check protoc availability
    check_protoc_available(args.protoc_path.as_ref())?;

    // Load tables from ODCS and export to .proto first
    let tables = load_tables_from_odcs(&args.input)?;

    // Validate protobuf version
    let version = args.protobuf_version.as_deref().unwrap_or("proto3");
    if version != "proto2" && version != "proto3" {
        return Err(CliError::InvalidArgument(format!(
            "Invalid protobuf version: {}. Must be 'proto2' or 'proto3'",
            version
        )));
    }

    let exporter = ProtobufExporter;
    let proto_result = exporter
        .export_with_version(&tables, version)
        .map_err(CliError::ExportError)?;

    // Write temporary .proto file
    let temp_proto = args.output.with_extension("proto.tmp");
    write_export_output(&temp_proto, &proto_result.content)?;

    // Generate descriptor from .proto file
    generate_protobuf_descriptor(&temp_proto, &args.output, args.protoc_path.as_ref())?;

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_proto);

    println!(
        "✅ Generated Protobuf descriptor: {}",
        args.output.display()
    );

    Ok(())
}

/// Handle ODPS export command
///
/// ODPS is a native format - it only accepts ODPS input files.
/// ODCS cannot be converted to ODPS as they are different format types.
pub fn handle_export_odps(_args: &ExportArgs) -> Result<(), CliError> {
    #[cfg(not(feature = "odps-validation"))]
    {
        Err(CliError::InvalidArgument(
            "ODPS export requires 'odps-validation' feature".to_string(),
        ))
    }

    #[cfg(feature = "odps-validation")]
    {
        /// Detect if content is ODPS format
        fn is_odps_format(content: &str) -> bool {
            content.contains("apiVersion:") && content.contains("kind: DataProduct")
        }

        let args = _args;
        // Check overwrite
        check_file_overwrite(&args.output, args.force)?;

        // Read input file
        let content = std::fs::read_to_string(&args.input)
            .map_err(|e| CliError::FileReadError(args.input.clone(), e.to_string()))?;

        // Verify input is ODPS format
        if !is_odps_format(&content) {
            return Err(CliError::InvalidArgument(
                "Input file is not ODPS format. ODPS export only accepts ODPS input files.\n\
                ODCS and ODPS are separate native formats and cannot be converted between each other.\n\
                Use 'import odps' for ODPS files or 'export odcs' for ODCS files."
                    .to_string(),
            ));
        }

        // Import ODPS file
        use crate::export::ODPSExporter;
        use crate::import::ODPSImporter;

        let importer = ODPSImporter::new();
        let product = importer
            .import(&content)
            .map_err(|e| CliError::InvalidArgument(format!("Failed to import ODPS file: {e}")))?;

        // Export to ODPS YAML (validation happens inside exporter if feature enabled)
        let exporter = ODPSExporter;
        let yaml = exporter.export(&product).map_err(CliError::ExportError)?;

        // Write output
        write_export_output(&args.output, &yaml)?;
        println!(
            "✅ Exported ODPS data product to: {}",
            args.output.display()
        );

        Ok(())
    }
}
