//! Import command handlers

use crate::cli::error::CliError;
use crate::cli::output::{collect_type_mappings, format_compact_output, format_pretty_output};
use crate::cli::reference::resolve_reference;
#[cfg(feature = "openapi")]
use crate::cli::validation::validate_openapi;
use crate::cli::validation::{
    validate_avro, validate_json_schema, validate_odcs, validate_protobuf,
};
use crate::export::odcs::ODCSExporter;
use crate::import::{
    AvroImporter, ColumnData, ImportResult, JSONSchemaImporter, ODCSImporter, ODPSImporter,
    ProtobufImporter, SQLImporter, TableData,
};
use crate::models::{Column, Table};
use serde_json::Value as JsonValue;
use std::io::{self, Read};
use std::path::PathBuf;
use uuid::Uuid;

/// Input source for import operations
#[derive(Debug, Clone)]
pub enum InputSource {
    File(PathBuf),
    Stdin,
    String(String),
}

/// Arguments for import operations
#[derive(Debug, Clone)]
pub struct ImportArgs {
    pub format: ImportFormat,
    pub input: InputSource,
    pub dialect: Option<String>,
    pub uuid_override: Option<String>,
    pub resolve_references: bool,
    pub validate: bool,
    pub pretty: bool,
    pub jar_path: Option<PathBuf>,
    pub message_type: Option<String>,
    pub no_odcs: bool, // If true, don't write .odcs.yaml file
}

/// Import format enum
#[derive(Debug, Clone)]
pub enum ImportFormat {
    Sql,
    Avro,
    JsonSchema,
    Protobuf,
    OpenApi,
    Odcs,
    Odps,
}

/// Load input content from InputSource
pub fn load_input(input: &InputSource) -> Result<String, CliError> {
    match input {
        InputSource::File(path) => std::fs::read_to_string(path)
            .map_err(|e| CliError::FileReadError(path.clone(), e.to_string())),
        InputSource::Stdin => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .map_err(|e| CliError::InvalidArgument(format!("Failed to read stdin: {}", e)))?;
            Ok(buffer)
        }
        InputSource::String(content) => Ok(content.clone()),
    }
}

/// Convert ColumnData to Column
fn column_data_to_column(col_data: &ColumnData) -> Column {
    Column {
        name: col_data.name.clone(),
        data_type: col_data.data_type.clone(),
        nullable: col_data.nullable,
        primary_key: col_data.primary_key,
        secondary_key: false,
        composite_key: None,
        foreign_key: None,
        constraints: Vec::new(),
        description: col_data.description.clone().unwrap_or_default(),
        errors: Vec::new(),
        quality: col_data.quality.clone().unwrap_or_default(),
        ref_path: col_data.ref_path.clone(),
        enum_values: col_data.enum_values.clone().unwrap_or_default(),
        column_order: 0,
    }
}

/// Convert TableData to Table
fn table_data_to_table(table_data: &TableData, uuid: Option<Uuid>) -> Table {
    let table_name = table_data
        .name
        .clone()
        .unwrap_or_else(|| format!("table_{}", table_data.table_index));

    let columns: Vec<Column> = table_data
        .columns
        .iter()
        .map(column_data_to_column)
        .collect();

    let mut table = Table::new(table_name, columns);

    // Override UUID if provided
    if let Some(uuid_val) = uuid {
        table.id = uuid_val;
    }

    table
}

/// Write ODCS YAML files from import result
fn write_odcs_files(
    result: &ImportResult,
    base_path: Option<&std::path::Path>,
    uuid_override: Option<&str>,
) -> Result<(), CliError> {
    if result.tables.is_empty() {
        return Ok(());
    }

    for table_data in &result.tables {
        // Parse UUID override if provided (only for single table)
        let uuid = if let Some(uuid_str) = uuid_override {
            if result.tables.len() == 1 {
                Some(
                    Uuid::parse_str(uuid_str)
                        .map_err(|e| CliError::InvalidUuid(format!("{}: {}", uuid_str, e)))?,
                )
            } else {
                return Err(CliError::MultipleTablesWithUuid(result.tables.len()));
            }
        } else {
            None
        };

        let table = table_data_to_table(table_data, uuid);
        let odcs_yaml = ODCSExporter::export_table(&table, "odcs_v3_1_0");

        // Determine output file path
        let table_name = table_data.name.as_deref().unwrap_or("table");
        let output_path = if let Some(base) = base_path {
            base.join(format!("{}.odcs.yaml", table_name))
        } else {
            PathBuf::from(format!("{}.odcs.yaml", table_name))
        };

        // Write file
        std::fs::write(&output_path, odcs_yaml)
            .map_err(|e| CliError::FileWriteError(output_path.clone(), e.to_string()))?;

        println!("✅ Wrote ODCS file: {}", output_path.display());
    }

    Ok(())
}

/// Apply UUID override to import result (only for single-table imports)
pub fn apply_uuid_override(result: &mut ImportResult, uuid_str: &str) -> Result<(), CliError> {
    // Validate UUID format
    let _uuid = Uuid::parse_str(uuid_str)
        .map_err(|e| CliError::InvalidUuid(format!("{}: {}", uuid_str, e)))?;

    // Check that only one table was imported
    if result.tables.len() != 1 {
        return Err(CliError::MultipleTablesWithUuid(result.tables.len()));
    }

    // UUID override is handled in write_odcs_files
    Ok(())
}

/// Handle SQL import command
pub fn handle_import_sql(args: &ImportArgs) -> Result<(), CliError> {
    let dialect = args.dialect.as_ref().ok_or_else(|| {
        CliError::InvalidArgument("--dialect is required for SQL import".to_string())
    })?;

    // Load SQL input
    let sql_content = load_input(&args.input)?;

    if sql_content.trim().is_empty() {
        return Err(CliError::InvalidArgument(
            "No SQL content provided".to_string(),
        ));
    }

    // Parse SQL
    let importer = SQLImporter::new(dialect);
    let mut result = importer.parse(&sql_content).map_err(|e| {
        CliError::ImportError(crate::import::ImportError::ParseError(e.to_string()))
    })?;

    // Apply UUID override if provided
    if let Some(ref uuid) = args.uuid_override {
        apply_uuid_override(&mut result, uuid)?;
    }

    // Display results
    let mappings = collect_type_mappings(&result);
    let output = if args.pretty {
        format_pretty_output(&result, &mappings)
    } else {
        format_compact_output(&result)
    };
    print!("{}", output);

    // Write ODCS files unless --no-odcs is specified
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}

/// Handle AVRO import command
pub fn handle_import_avro(args: &ImportArgs) -> Result<(), CliError> {
    // Load AVRO input
    let avro_content = load_input(&args.input)?;

    // Validate if enabled
    if args.validate {
        validate_avro(&avro_content)?;
    }

    // TODO: Resolve external references if enabled
    // For AVRO, references are typically embedded, but could be in $ref fields

    // Import AVRO
    let importer = AvroImporter::new();
    let mut result = importer
        .import(&avro_content)
        .map_err(CliError::ImportError)?;

    // Apply UUID override if provided
    if let Some(ref uuid) = args.uuid_override {
        apply_uuid_override(&mut result, uuid)?;
    }

    // Display results
    let mappings = collect_type_mappings(&result);
    let output = if args.pretty {
        format_pretty_output(&result, &mappings)
    } else {
        format_compact_output(&result)
    };
    print!("{}", output);

    // Write ODCS files unless --no-odcs is specified
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}

/// Filter proto files by message type
fn filter_proto_by_message_type(
    proto_contents: Vec<(String, String)>,
    message_type: &str,
) -> Result<Vec<(String, String)>, CliError> {
    let mut filtered = Vec::new();

    for (file_name, content) in proto_contents {
        // Check if the proto file contains the specified message type
        // Look for "message <message_type>" pattern
        let message_pattern = format!("message {}", message_type);
        if content.contains(&message_pattern) {
            filtered.push((file_name, content));
        }
    }

    if filtered.is_empty() {
        Err(CliError::InvalidArgument(format!(
            "No proto files found containing message type '{}'",
            message_type
        )))
    } else {
        Ok(filtered)
    }
}

/// Resolve all external references in JSON Schema content
fn resolve_json_schema_references(
    content: &str,
    source_file: Option<&PathBuf>,
    resolve_refs: bool,
) -> Result<String, CliError> {
    if !resolve_refs {
        return Ok(content.to_string());
    }

    let mut schema: JsonValue = serde_json::from_str(content)
        .map_err(|e| CliError::InvalidArgument(format!("Invalid JSON Schema: {}", e)))?;

    // Find and resolve external $ref references
    let source_path = source_file.map(|p| p.as_path());
    resolve_json_refs_recursive(&mut schema, source_path)?;

    serde_json::to_string_pretty(&schema).map_err(|e| {
        CliError::InvalidArgument(format!("Failed to serialize resolved schema: {}", e))
    })
}

/// Recursively resolve $ref references in JSON Schema
fn resolve_json_refs_recursive(
    value: &mut JsonValue,
    source_file: Option<&std::path::Path>,
) -> Result<(), CliError> {
    match value {
        JsonValue::Object(obj) => {
            // Check for $ref field
            if let Some(ref_val) = obj.get("$ref").and_then(|v| v.as_str()) {
                // Only resolve external references (not JSON pointers like #/definitions/...)
                if !ref_val.starts_with('#')
                    && (ref_val.starts_with("http://")
                        || ref_val.starts_with("https://")
                        || !ref_val.starts_with('/'))
                {
                    let resolved_content = resolve_reference(ref_val, source_file)?;

                    // Parse resolved content and merge into current object
                    let resolved: JsonValue =
                        serde_json::from_str(&resolved_content).map_err(|e| {
                            CliError::ReferenceResolutionError(format!(
                                "Failed to parse resolved reference '{}': {}",
                                ref_val, e
                            ))
                        })?;

                    // Merge resolved content into current object (resolved takes precedence)
                    if let JsonValue::Object(resolved_obj) = resolved {
                        for (k, v) in resolved_obj {
                            if k != "$ref" {
                                obj.insert(k, v);
                            }
                        }
                    }
                }
            }

            // Recursively process all values
            for v in obj.values_mut() {
                resolve_json_refs_recursive(v, source_file)?;
            }
        }
        JsonValue::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_json_refs_recursive(item, source_file)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Handle JSON Schema import command
pub fn handle_import_json_schema(args: &ImportArgs) -> Result<(), CliError> {
    // Load JSON Schema input
    let mut json_content = load_input(&args.input)?;

    // Resolve external references if enabled
    if args.resolve_references {
        let source_file = match &args.input {
            InputSource::File(path) => Some(path),
            _ => None,
        };
        json_content =
            resolve_json_schema_references(&json_content, source_file, args.resolve_references)?;
    }

    // Validate if enabled
    if args.validate {
        validate_json_schema(&json_content)?;
    }

    // Import JSON Schema
    let importer = JSONSchemaImporter::new();
    let mut result = importer
        .import(&json_content)
        .map_err(CliError::ImportError)?;

    // Apply UUID override if provided
    if let Some(ref uuid) = args.uuid_override {
        apply_uuid_override(&mut result, uuid)?;
    }

    // Display results
    let mappings = collect_type_mappings(&result);
    let output = if args.pretty {
        format_pretty_output(&result, &mappings)
    } else {
        format_compact_output(&result)
    };
    print!("{}", output);

    // Write ODCS files unless --no-odcs is specified
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}

/// Handle Protobuf import command
pub fn handle_import_protobuf(args: &ImportArgs) -> Result<(), CliError> {
    // Handle JAR import if jar_path is provided
    if let Some(ref jar_path) = args.jar_path {
        return handle_import_protobuf_from_jar(args, jar_path);
    }

    // Load Protobuf input
    let proto_content = load_input(&args.input)?;

    // Validate if enabled
    if args.validate {
        validate_protobuf(&proto_content)?;
    }

    // Import Protobuf
    let importer = ProtobufImporter::new();
    let mut result = importer
        .import(&proto_content)
        .map_err(CliError::ImportError)?;

    // Apply UUID override if provided
    if let Some(ref uuid) = args.uuid_override {
        apply_uuid_override(&mut result, uuid)?;
    }

    // Display results
    let mappings = collect_type_mappings(&result);
    let output = if args.pretty {
        format_pretty_output(&result, &mappings)
    } else {
        format_compact_output(&result)
    };
    print!("{}", output);

    // Write ODCS files unless --no-odcs is specified
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}

/// Handle Protobuf import from JAR file
fn handle_import_protobuf_from_jar(args: &ImportArgs, jar_path: &PathBuf) -> Result<(), CliError> {
    use std::io::Read;
    use zip::ZipArchive;

    // Open JAR file as ZIP
    let file = std::fs::File::open(jar_path)
        .map_err(|e| CliError::FileReadError(jar_path.clone(), e.to_string()))?;

    let mut archive = ZipArchive::new(file)
        .map_err(|e| CliError::InvalidArgument(format!("Failed to open JAR file: {}", e)))?;

    // Extract all .proto files
    let mut proto_contents = Vec::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| {
            CliError::InvalidArgument(format!("Failed to read JAR entry {}: {}", i, e))
        })?;

        let file_name = file.name().to_string();
        if file_name.ends_with(".proto") {
            let mut content = String::new();
            file.read_to_string(&mut content).map_err(|e| {
                CliError::InvalidArgument(format!("Failed to read proto file {}: {}", file_name, e))
            })?;
            proto_contents.push((file_name, content));
        }
    }

    if proto_contents.is_empty() {
        return Err(CliError::InvalidArgument(
            "No .proto files found in JAR".to_string(),
        ));
    }

    // Filter by message type if specified
    let filtered_contents = if let Some(ref message_type) = args.message_type {
        filter_proto_by_message_type(proto_contents, message_type)?
    } else {
        proto_contents
    };

    // Merge proto files if multiple
    let merged_proto = if filtered_contents.len() == 1 {
        filtered_contents[0].1.clone()
    } else {
        // Simple merge - concatenate with newlines
        filtered_contents
            .iter()
            .map(|(_, content)| content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    };

    // Import merged proto
    let importer = ProtobufImporter::new();
    let mut result = importer
        .import(&merged_proto)
        .map_err(CliError::ImportError)?;

    // Apply UUID override if provided
    if let Some(ref uuid) = args.uuid_override {
        apply_uuid_override(&mut result, uuid)?;
    }

    // Display results
    let mappings = collect_type_mappings(&result);
    let output = if args.pretty {
        format_pretty_output(&result, &mappings)
    } else {
        format_compact_output(&result)
    };
    print!("{}", output);

    // Write ODCS files unless --no-odcs is specified
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}

/// Resolve all external references in OpenAPI content
#[cfg(feature = "openapi")]
fn resolve_openapi_references(
    content: &str,
    source_file: Option<&PathBuf>,
    resolve_refs: bool,
) -> Result<String, CliError> {
    if !resolve_refs {
        return Ok(content.to_string());
    }

    // Parse as YAML or JSON
    let mut spec: JsonValue = if content.trim_start().starts_with('{') {
        serde_json::from_str(content)
            .map_err(|e| CliError::InvalidArgument(format!("Invalid OpenAPI JSON: {}", e)))?
    } else {
        serde_yaml::from_str(content)
            .map_err(|e| CliError::InvalidArgument(format!("Invalid OpenAPI YAML: {}", e)))?
    };

    // Resolve external $ref references
    let source_path = source_file.map(|p| p.as_path());
    resolve_json_refs_recursive(&mut spec, source_path)?;

    // Output in same format as input
    if content.trim_start().starts_with('{') {
        serde_json::to_string_pretty(&spec).map_err(|e| {
            CliError::InvalidArgument(format!("Failed to serialize resolved spec: {}", e))
        })
    } else {
        serde_yaml::to_string(&spec).map_err(|e| {
            CliError::InvalidArgument(format!("Failed to serialize resolved spec: {}", e))
        })
    }
}

/// Handle OpenAPI import command
#[cfg(feature = "openapi")]
pub fn handle_import_openapi(args: &ImportArgs) -> Result<(), CliError> {
    use crate::convert::openapi_to_odcs::OpenAPIToODCSConverter;

    // Load OpenAPI input
    let mut openapi_content = load_input(&args.input)?;

    // Resolve external references if enabled
    if args.resolve_references {
        let source_file = match &args.input {
            InputSource::File(path) => Some(path),
            _ => None,
        };
        openapi_content =
            resolve_openapi_references(&openapi_content, source_file, args.resolve_references)?;
    }

    // Validate if enabled
    if args.validate {
        validate_openapi(&openapi_content)?;
    }

    // Convert OpenAPI to ODCS tables using converter
    let converter = OpenAPIToODCSConverter::new();

    // Parse OpenAPI spec to find schema components
    let spec: JsonValue = if openapi_content.trim_start().starts_with('{') {
        serde_json::from_str(&openapi_content).map_err(|e| {
            CliError::ImportError(crate::import::ImportError::OpenAPIParseError(e.to_string()))
        })?
    } else {
        serde_yaml::from_str(&openapi_content).map_err(|e| {
            CliError::ImportError(crate::import::ImportError::OpenAPIParseError(e.to_string()))
        })?
    };

    // Extract components/schemas section
    let components = spec.get("components").and_then(|c| c.get("schemas"));
    let mut tables = Vec::new();
    let mut errors = Vec::new();

    if let Some(schemas) = components.and_then(|s| s.as_object()) {
        for (schema_name, _schema_value) in schemas {
            match converter.convert_component(&openapi_content, schema_name, None) {
                Ok(table) => {
                    tables.push(crate::import::TableData {
                        table_index: tables.len(),
                        name: Some(table.name.clone()),
                        columns: table
                            .columns
                            .iter()
                            .map(|col| crate::import::ColumnData {
                                name: col.name.clone(),
                                data_type: col.data_type.clone(),
                                nullable: col.nullable,
                                primary_key: col.primary_key,
                                description: Some(col.description.clone()),
                                quality: None,
                                ref_path: col.ref_path.clone(),
                                enum_values: if col.enum_values.is_empty() {
                                    None
                                } else {
                                    Some(col.enum_values.clone())
                                },
                            })
                            .collect(),
                    });
                }
                Err(e) => {
                    errors.push(crate::import::ImportError::OpenAPIParseError(e.to_string()));
                }
            }
        }
    }

    let mut result = ImportResult {
        tables,
        tables_requiring_name: vec![],
        errors,
        ai_suggestions: None,
    };

    // Apply UUID override if provided
    if let Some(ref uuid) = args.uuid_override {
        apply_uuid_override(&mut result, uuid)?;
    }

    // Display results
    let mappings = collect_type_mappings(&result);
    let output = if args.pretty {
        format_pretty_output(&result, &mappings)
    } else {
        format_compact_output(&result)
    };
    print!("{}", output);

    // Write ODCS files unless --no-odcs is specified
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}

/// Handle ODPS import command
#[cfg(feature = "odps-validation")]
pub fn handle_import_odps(args: &ImportArgs) -> Result<(), CliError> {
    use crate::cli::validation::validate_odps;

    // Load ODPS input
    let odps_content = load_input(&args.input)?;

    // Validate if enabled
    if args.validate {
        validate_odps(&odps_content)?;
    }

    // Import ODPS
    let importer = ODPSImporter::new();
    let product = importer
        .import(&odps_content)
        .map_err(CliError::ImportError)?;

    // Display results
    if args.pretty {
        println!("ODPS Data Product");
        println!("=================");
        println!("ID:              {}", product.id);
        if let Some(name) = &product.name {
            println!("Name:            {}", name);
        }
        if let Some(version) = &product.version {
            println!("Version:         {}", version);
        }
        println!("Status:          {:?}", product.status);
        if let Some(domain) = &product.domain {
            println!("Domain:          {}", domain);
        }
        if let Some(tenant) = &product.tenant {
            println!("Tenant:          {}", tenant);
        }
        // Tags
        if !product.tags.is_empty() {
            println!("\nTags:");
            for tag in &product.tags {
                println!("  - {}", tag);
            }
        }

        // Description
        if let Some(description) = &product.description {
            println!("\nDescription:");
            if let Some(purpose) = &description.purpose {
                println!("  Purpose:       {}", purpose);
            }
            if let Some(usage) = &description.usage {
                println!("  Usage:         {}", usage);
            }
            if let Some(limitations) = &description.limitations {
                println!("  Limitations:   {}", limitations);
            }
        }

        if let Some(input_ports) = &product.input_ports {
            println!("\nInput Ports ({}):", input_ports.len());
            for port in input_ports {
                println!(
                    "  - {} v{} (contract: {})",
                    port.name, port.version, port.contract_id
                );
            }
        }
        if let Some(output_ports) = &product.output_ports {
            println!("\nOutput Ports ({}):", output_ports.len());
            for port in output_ports {
                println!("  - {} v{}", port.name, port.version);
            }
        }
        if let Some(management_ports) = &product.management_ports {
            println!("\nManagement Ports ({}):", management_ports.len());
            for port in management_ports {
                println!("  - {} ({})", port.name, port.content);
            }
        }
        if let Some(support) = &product.support {
            println!("\nSupport Channels ({}):", support.len());
            for s in support {
                println!("  - {}: {}", s.channel, s.url);
            }
        }
        if let Some(team) = &product.team {
            println!("\nTeam:");
            if let Some(name) = &team.name {
                println!("  Name:          {}", name);
            }
            if let Some(members) = &team.members {
                println!("  Members:       {}", members.len());
            }
        }
    } else {
        println!("Imported ODPS Data Product:");
        println!("  ID: {}", product.id);
        if let Some(name) = &product.name {
            println!("  Name: {}", name);
        }
        if let Some(version) = &product.version {
            println!("  Version: {}", version);
        }
        println!("  Status: {:?}", product.status);

        // Tags
        if !product.tags.is_empty() {
            let tags_str: Vec<String> = product.tags.iter().map(|t| t.to_string()).collect();
            println!("  Tags: {}", tags_str.join(", "));
        }

        // Description
        if let Some(description) = &product.description {
            let mut desc_parts = Vec::new();
            if description.purpose.is_some()
                || description.usage.is_some()
                || description.limitations.is_some()
            {
                desc_parts.push("present");
            }
            if !desc_parts.is_empty() {
                println!("  Description: {}", desc_parts.join(", "));
            }
        }

        // Input Ports
        if let Some(input_ports) = &product.input_ports {
            println!("  Input Ports: {}", input_ports.len());
        }

        // Output Ports
        if let Some(output_ports) = &product.output_ports {
            println!("  Output Ports: {}", output_ports.len());
        }

        // Management Ports
        if let Some(management_ports) = &product.management_ports {
            println!("  Management Ports: {}", management_ports.len());
        }

        // Support
        if let Some(support) = &product.support
            && !support.is_empty()
        {
            println!("  Support Channels: {}", support.len());
        }

        // Team
        if let Some(team) = &product.team {
            if let Some(name) = &team.name {
                println!("  Team: {}", name);
            } else if team.members.is_some() {
                println!("  Team: present");
            }
        }
    }

    Ok(())
}

#[cfg(not(feature = "odps-validation"))]
pub fn handle_import_odps(args: &ImportArgs) -> Result<(), CliError> {
    // Load ODPS input
    let odps_content = load_input(&args.input)?;

    // Import ODPS (without validation if feature not enabled)
    let importer = ODPSImporter::new();
    let product = importer
        .import(&odps_content)
        .map_err(CliError::ImportError)?;

    // Display results (same as above)
    if args.pretty {
        println!("ODPS Data Product");
        println!("=================");
        println!("ID:              {}", product.id);
        if let Some(name) = &product.name {
            println!("Name:            {}", name);
        }
        if let Some(version) = &product.version {
            println!("Version:         {}", version);
        }
        println!("Status:          {:?}", product.status);
    } else {
        println!("Imported ODPS Data Product:");
        println!("  ID: {}", product.id);
        if let Some(name) = &product.name {
            println!("  Name: {}", name);
        }
        println!("  Status: {:?}", product.status);
    }

    Ok(())
}

/// Handle ODCS import command
pub fn handle_import_odcs(args: &ImportArgs) -> Result<(), CliError> {
    // Load ODCS input
    let odcs_content = load_input(&args.input)?;

    // Validate if enabled
    if args.validate {
        validate_odcs(&odcs_content)?;
    }

    // Import ODCS
    let mut importer = ODCSImporter::new();
    let mut result = importer
        .import(&odcs_content)
        .map_err(CliError::ImportError)?;

    // Apply UUID override if provided
    if let Some(ref uuid) = args.uuid_override {
        apply_uuid_override(&mut result, uuid)?;
    }

    // Display results
    let mappings = collect_type_mappings(&result);
    let output = if args.pretty {
        format_pretty_output(&result, &mappings)
    } else {
        format_compact_output(&result)
    };
    print!("{}", output);

    // Write ODCS files unless --no-odcs is specified
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}
