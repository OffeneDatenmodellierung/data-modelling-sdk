//! Import command handlers

use crate::cli::error::CliError;
use crate::cli::output::{collect_type_mappings, format_compact_output, format_pretty_output};
use crate::cli::reference::resolve_reference;
#[cfg(feature = "openapi")]
use crate::cli::validation::validate_openapi;
use crate::cli::validation::{
    validate_avro, validate_json_schema, validate_odcl, validate_odcs, validate_protobuf,
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
    pub no_odcs: bool,                // If true, don't write .odcs.yaml file
    pub root_message: Option<String>, // Root message for JAR imports (auto-detected if not provided)
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
    Odcl,
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
    let data_type_upper = col_data.data_type.to_uppercase();
    let is_array_struct =
        data_type_upper.starts_with("ARRAY<") && data_type_upper.contains("STRUCT<");
    let is_map = data_type_upper.starts_with("MAP<");

    // For ARRAY<STRUCT> or MAP types, append full type to description and use simplified data_type
    // Always use "ARRAY" as the data_type (not "ARRAY<STRUCT<...>>")
    let (data_type, description) = if is_array_struct || is_map {
        let simplified_type = if is_array_struct {
            "ARRAY".to_string()
        } else {
            "MAP".to_string()
        };
        let base_description = col_data.description.clone().unwrap_or_default();
        let full_description = if base_description.is_empty() {
            col_data.data_type.clone()
        } else {
            format!("{} || {}", base_description, col_data.data_type)
        };
        (simplified_type, full_description)
    } else {
        (
            col_data.data_type.clone(),
            col_data.description.clone().unwrap_or_default(),
        )
    };

    Column {
        name: col_data.name.clone(),
        data_type,
        physical_type: col_data.physical_type.clone(),
        nullable: col_data.nullable,
        primary_key: col_data.primary_key,
        secondary_key: false,
        composite_key: None,
        foreign_key: None,
        constraints: Vec::new(),
        description,
        errors: Vec::new(),
        quality: col_data.quality.clone().unwrap_or_default(),
        relationships: col_data.relationships.clone(),
        enum_values: col_data.enum_values.clone().unwrap_or_default(),
        column_order: 0,
        nested_data: None,
    }
}

/// Parse STRUCT type from data_type string and create nested columns
/// This ensures STRUCT columns from SQL imports are properly expanded into nested columns
fn parse_struct_columns(parent_name: &str, data_type: &str, col_data: &ColumnData) -> Vec<Column> {
    use crate::import::odcs::ODCSImporter;

    let importer = ODCSImporter::new();

    // Try to parse STRUCT type using ODCS importer's logic
    // Create a dummy field_data object for parsing
    let field_data = serde_json::Map::new();

    match importer.parse_struct_type_from_string(parent_name, data_type, &field_data) {
        Ok(nested_cols) if !nested_cols.is_empty() => {
            // parse_struct_type_from_string returns only nested columns, not the parent
            // So we need to add the parent column first
            let mut all_cols = Vec::new();

            // Add parent column
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
                secondary_key: false,
                composite_key: None,
                foreign_key: None,
                constraints: Vec::new(),
                description: col_data.description.clone().unwrap_or_default(),
                errors: Vec::new(),
                quality: col_data.quality.clone().unwrap_or_default(),
                relationships: col_data.relationships.clone(),
                enum_values: col_data.enum_values.clone().unwrap_or_default(),
                column_order: 0,
                nested_data: None,
            });

            // Add nested columns
            all_cols.extend(nested_cols);
            return all_cols;
        }
        Ok(_) => {
            // Parsing succeeded but returned empty - this shouldn't happen for valid STRUCT
            // Fall through to return empty
        }
        Err(_) => {
            // Parsing failed - silently fall through to return empty
            // This allows fallback to simple column
        }
    }

    // If parsing fails or returns empty, return empty (will fall back to simple column)
    Vec::new()
}

/// Convert TableData to Table
fn table_data_to_table(table_data: &TableData, uuid: Option<Uuid>) -> Table {
    let table_name = table_data
        .name
        .clone()
        .unwrap_or_else(|| format!("table_{}", table_data.table_index));

    let mut all_columns = Vec::new();

    // Convert columns - do NOT parse ARRAY<STRUCT> or MAP types (store in nestedData instead)
    for col_data in &table_data.columns {
        let data_type_upper = col_data.data_type.to_uppercase();
        let is_array_struct =
            data_type_upper.starts_with("ARRAY<") && data_type_upper.contains("STRUCT<");
        let is_map = data_type_upper.starts_with("MAP<");

        // Skip parsing for ARRAY<STRUCT> or MAP - they go in nestedData
        if is_array_struct || is_map {
            all_columns.push(column_data_to_column(col_data));
            continue;
        }

        // For regular STRUCT (not ARRAY<STRUCT>), try to parse and create nested columns
        let is_struct = data_type_upper.contains("STRUCT<") || data_type_upper == "STRUCT";
        if is_struct {
            // Try to parse STRUCT and create nested columns
            let struct_cols = parse_struct_columns(&col_data.name, &col_data.data_type, col_data);
            if !struct_cols.is_empty() {
                all_columns.extend(struct_cols);
                continue;
            }
        }

        // Regular column (non-STRUCT or STRUCT parsing failed)
        all_columns.push(column_data_to_column(col_data));
    }

    let mut table = Table::new(table_name, all_columns);

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

        // Validate exported ODCS YAML before writing (if validation enabled)
        #[cfg(feature = "schema-validation")]
        {
            validate_odcs(&odcs_yaml).map_err(|e| {
                CliError::ValidationError(format!("Exported ODCS file failed validation: {}", e))
            })?;
        }

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

/// Resolve external references in AVRO content
///
/// AVRO schemas can reference other schemas via:
/// - Named types (type references to other records in the same namespace)
/// - External file references (non-standard but common in some tooling)
///
/// This function resolves external file/URL references in the schema.
fn resolve_avro_references(
    content: &str,
    source_file: Option<&PathBuf>,
    resolve_refs: bool,
) -> Result<String, CliError> {
    if !resolve_refs {
        return Ok(content.to_string());
    }

    let mut schema: JsonValue = serde_json::from_str(content)
        .map_err(|e| CliError::InvalidArgument(format!("Invalid AVRO JSON: {}", e)))?;

    // Resolve external references in the schema
    let source_path = source_file.map(|p| p.as_path());
    resolve_avro_refs_recursive(&mut schema, source_path)?;

    serde_json::to_string_pretty(&schema).map_err(|e| {
        CliError::InvalidArgument(format!("Failed to serialize resolved AVRO schema: {}", e))
    })
}

/// Recursively resolve references in AVRO schema
///
/// Handles:
/// - "$ref" fields pointing to external schemas
/// - "type" fields that are URLs or file paths
fn resolve_avro_refs_recursive(
    value: &mut JsonValue,
    source_file: Option<&std::path::Path>,
) -> Result<(), CliError> {
    match value {
        JsonValue::Object(obj) => {
            // Check for $ref field (non-standard but used by some tools)
            if let Some(ref_val) = obj.get("$ref").and_then(|v| v.as_str()) {
                // Only resolve external references (file paths or URLs)
                if ref_val.starts_with("http://")
                    || ref_val.starts_with("https://")
                    || ref_val.ends_with(".avsc")
                    || ref_val.ends_with(".json")
                {
                    let resolved_content = resolve_reference(ref_val, source_file)?;
                    let resolved: JsonValue =
                        serde_json::from_str(&resolved_content).map_err(|e| {
                            CliError::ReferenceResolutionError(format!(
                                "Failed to parse resolved AVRO reference '{}': {}",
                                ref_val, e
                            ))
                        })?;

                    // Replace the entire object with resolved content
                    *value = resolved;
                    return Ok(());
                }
            }

            // Recursively process all values
            for v in obj.values_mut() {
                resolve_avro_refs_recursive(v, source_file)?;
            }
        }
        JsonValue::Array(arr) => {
            for item in arr.iter_mut() {
                resolve_avro_refs_recursive(item, source_file)?;
            }
        }
        _ => {}
    }

    Ok(())
}

/// Handle AVRO import command
pub fn handle_import_avro(args: &ImportArgs) -> Result<(), CliError> {
    // Load AVRO input
    let mut avro_content = load_input(&args.input)?;

    // Resolve external references if enabled
    if args.resolve_references {
        let source_file = match &args.input {
            InputSource::File(path) => Some(path),
            _ => None,
        };
        avro_content =
            resolve_avro_references(&avro_content, source_file, args.resolve_references)?;
    }

    // Validate if enabled
    if args.validate {
        validate_avro(&avro_content)?;
    }

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
#[allow(dead_code)]
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

/// Parsed protobuf message with its fields and dependencies
#[derive(Debug, Clone)]
struct ParsedProtoMessage {
    name: String,
    full_name: String, // Including package prefix
    fields: Vec<ParsedProtoField>,
    #[allow(dead_code)]
    source_file: String,
}

/// Parsed protobuf field
#[derive(Debug, Clone)]
struct ParsedProtoField {
    name: String,
    field_type: String,
    repeated: bool,
    optional: bool,
}

/// Extract all message definitions from proto content
fn extract_proto_messages(file_name: &str, content: &str) -> Vec<ParsedProtoMessage> {
    let mut messages = Vec::new();
    let mut current_package = String::new();
    let mut current_message: Option<(String, Vec<ParsedProtoField>)> = None;
    let mut brace_depth = 0;
    let mut in_message = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        // Extract package name
        if trimmed.starts_with("package ") {
            current_package = trimmed
                .strip_prefix("package ")
                .unwrap_or("")
                .trim_end_matches(';')
                .trim()
                .to_string();
            continue;
        }

        // Check for message start
        if trimmed.starts_with("message ") && !in_message {
            let msg_name = trimmed
                .strip_prefix("message ")
                .and_then(|s| {
                    let s = s.trim();
                    if let Some(idx) = s.find('{') {
                        Some(s[..idx].trim())
                    } else {
                        s.split_whitespace().next()
                    }
                })
                .unwrap_or("")
                .to_string();

            if !msg_name.is_empty() {
                current_message = Some((msg_name, Vec::new()));
                in_message = true;
                brace_depth = 1;
                if trimmed.matches('{').count() > trimmed.matches('}').count() {
                    // Opening brace on same line
                } else if !trimmed.contains('{') {
                    brace_depth = 0; // Brace on next line
                }
            }
            continue;
        }

        // Track brace depth
        if in_message {
            brace_depth += trimmed.matches('{').count();
            brace_depth = brace_depth.saturating_sub(trimmed.matches('}').count());

            // Parse fields (only at top level of message, brace_depth == 1)
            if brace_depth == 1
                && !trimmed.starts_with("message ")
                && !trimmed.starts_with("enum ")
                && !trimmed.starts_with("oneof ")
                && !trimmed.starts_with("reserved ")
                && !trimmed.starts_with("option ")
                && let Some((_, ref mut fields)) = current_message
                && let Some(field) = parse_proto_field_simple(trimmed)
            {
                fields.push(field);
            }

            // End of message
            if brace_depth == 0 {
                if let Some((msg_name, fields)) = current_message.take() {
                    let full_name = if current_package.is_empty() {
                        msg_name.clone()
                    } else {
                        format!("{}.{}", current_package, msg_name)
                    };
                    messages.push(ParsedProtoMessage {
                        name: msg_name,
                        full_name,
                        fields,
                        source_file: file_name.to_string(),
                    });
                }
                in_message = false;
            }
        }
    }

    messages
}

/// Parse a simple proto field line
fn parse_proto_field_simple(line: &str) -> Option<ParsedProtoField> {
    let line = line.split("//").next().unwrap_or(line).trim();
    if line.is_empty() || line == "}" || line == "{" {
        return None;
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }

    let mut idx = 0;
    let mut repeated = false;
    let mut optional = false;

    // Check for repeated/optional keywords
    while idx < parts.len() {
        match parts[idx] {
            "repeated" => {
                repeated = true;
                idx += 1;
            }
            "optional" => {
                optional = true;
                idx += 1;
            }
            _ => break,
        }
    }

    if idx + 2 > parts.len() {
        return None;
    }

    let field_type = parts[idx].to_string();
    let field_name = parts[idx + 1]
        .trim_end_matches(';')
        .trim_end_matches('=')
        .trim()
        .to_string();

    if field_name.is_empty() || field_name.starts_with('=') {
        return None;
    }

    Some(ParsedProtoField {
        name: field_name,
        field_type,
        repeated,
        optional,
    })
}

/// Check if a type is a scalar protobuf type
fn is_scalar_proto_type(type_name: &str) -> bool {
    matches!(
        type_name,
        "int32"
            | "int64"
            | "uint32"
            | "uint64"
            | "sint32"
            | "sint64"
            | "fixed32"
            | "fixed64"
            | "sfixed32"
            | "sfixed64"
            | "float"
            | "double"
            | "bool"
            | "string"
            | "bytes"
    )
}

/// Build a dependency graph from parsed messages
/// Returns: (message_name -> set of message names it references)
fn build_dependency_graph(
    messages: &[ParsedProtoMessage],
) -> std::collections::HashMap<String, std::collections::HashSet<String>> {
    use std::collections::{HashMap, HashSet};

    let message_names: HashSet<String> = messages.iter().map(|m| m.name.clone()).collect();
    let full_names: HashSet<String> = messages.iter().map(|m| m.full_name.clone()).collect();

    let mut graph: HashMap<String, HashSet<String>> = HashMap::new();

    for msg in messages {
        let mut deps = HashSet::new();
        for field in &msg.fields {
            let field_type = field.field_type.trim_start_matches('.');
            // Check if the field type references another message
            if !is_scalar_proto_type(field_type) {
                // Try exact match first
                if message_names.contains(field_type) {
                    deps.insert(field_type.to_string());
                } else if full_names.contains(field_type) {
                    // Extract just the message name from full name
                    if let Some(name) = field_type.rsplit('.').next()
                        && message_names.contains(name)
                    {
                        deps.insert(name.to_string());
                    }
                } else {
                    // Try matching just the last component
                    if let Some(name) = field_type.rsplit('.').next()
                        && message_names.contains(name)
                    {
                        deps.insert(name.to_string());
                    }
                }
            }
        }
        graph.insert(msg.name.clone(), deps);
    }

    graph
}

/// Find the root message (most outgoing references, no or fewest incoming references)
fn find_root_message(
    messages: &[ParsedProtoMessage],
    graph: &std::collections::HashMap<String, std::collections::HashSet<String>>,
) -> Option<String> {
    use std::collections::HashMap;

    if messages.is_empty() {
        return None;
    }

    // Count incoming references for each message
    let mut incoming_count: HashMap<String, usize> = HashMap::new();
    for msg in messages {
        incoming_count.insert(msg.name.clone(), 0);
    }

    for deps in graph.values() {
        for dep in deps {
            if let Some(count) = incoming_count.get_mut(dep) {
                *count += 1;
            }
        }
    }

    // Find messages with no incoming references (root candidates)
    let root_candidates: Vec<&String> = incoming_count
        .iter()
        .filter(|(_, count)| **count == 0)
        .map(|(name, _)| name)
        .collect();

    if root_candidates.len() == 1 {
        return Some(root_candidates[0].clone());
    }

    // If multiple candidates or none, pick the one with most outgoing references
    let mut best_candidate: Option<String> = None;
    let mut max_outgoing = 0;

    for msg in messages {
        let outgoing = graph.get(&msg.name).map(|deps| deps.len()).unwrap_or(0);
        let incoming = incoming_count.get(&msg.name).copied().unwrap_or(0);

        // Prefer messages with no incoming refs and most outgoing refs
        if incoming == 0 && outgoing > max_outgoing {
            max_outgoing = outgoing;
            best_candidate = Some(msg.name.clone());
        }
    }

    // If still no candidate, just pick the message with most outgoing refs
    if best_candidate.is_none() {
        for msg in messages {
            let outgoing = graph.get(&msg.name).map(|deps| deps.len()).unwrap_or(0);
            if outgoing > max_outgoing {
                max_outgoing = outgoing;
                best_candidate = Some(msg.name.clone());
            }
        }
    }

    // Last resort: return first message
    best_candidate.or_else(|| messages.first().map(|m| m.name.clone()))
}

/// Map proto type to SQL type
fn map_proto_to_sql_type(proto_type: &str) -> String {
    match proto_type {
        "int32" | "sint32" | "sfixed32" => "INTEGER".to_string(),
        "int64" | "sint64" | "sfixed64" => "BIGINT".to_string(),
        "uint32" | "fixed32" => "INTEGER".to_string(),
        "uint64" | "fixed64" => "BIGINT".to_string(),
        "float" => "FLOAT".to_string(),
        "double" => "DOUBLE".to_string(),
        "bool" => "BOOLEAN".to_string(),
        "string" => "STRING".to_string(),
        "bytes" => "BYTES".to_string(),
        _ => "STRING".to_string(), // Default for unknown/message types
    }
}

/// Flatten a message and all its dependencies into columns with dot notation
fn flatten_message_to_columns(
    root_message: &ParsedProtoMessage,
    all_messages: &[ParsedProtoMessage],
    prefix: &str,
    visited: &mut std::collections::HashSet<String>,
    max_depth: usize,
) -> Vec<ColumnData> {
    let mut columns = Vec::new();

    if max_depth == 0 {
        return columns;
    }

    // Prevent infinite recursion for circular references
    if visited.contains(&root_message.name) {
        return columns;
    }
    visited.insert(root_message.name.clone());

    for field in &root_message.fields {
        let column_name = if prefix.is_empty() {
            field.name.clone()
        } else {
            format!("{}.{}", prefix, field.name)
        };

        if is_scalar_proto_type(&field.field_type) {
            // Scalar type - add as column
            let data_type = if field.repeated {
                format!("ARRAY<{}>", map_proto_to_sql_type(&field.field_type))
            } else {
                map_proto_to_sql_type(&field.field_type)
            };

            columns.push(ColumnData {
                name: column_name,
                data_type,
                physical_type: None,
                nullable: field.optional || field.repeated,
                primary_key: false,
                description: None,
                quality: None,
                relationships: Vec::new(),
                enum_values: None,
            });
        } else {
            // Message type - find and flatten recursively
            let type_name = field
                .field_type
                .rsplit('.')
                .next()
                .unwrap_or(&field.field_type);
            if let Some(nested_msg) = all_messages.iter().find(|m| m.name == type_name) {
                let nested_columns = flatten_message_to_columns(
                    nested_msg,
                    all_messages,
                    &column_name,
                    visited,
                    max_depth - 1,
                );
                columns.extend(nested_columns);
            } else {
                // Unknown message type - add as STRING
                let data_type = if field.repeated {
                    "ARRAY<STRING>".to_string()
                } else {
                    "STRING".to_string()
                };
                columns.push(ColumnData {
                    name: column_name,
                    data_type,
                    physical_type: None,
                    nullable: field.optional || field.repeated,
                    primary_key: false,
                    description: Some(format!("Unknown message type: {}", field.field_type)),
                    quality: None,
                    relationships: Vec::new(),
                    enum_values: None,
                });
            }
        }
    }

    visited.remove(&root_message.name);
    columns
}

/// Handle Protobuf import from JAR file with dependency graph analysis
fn handle_import_protobuf_from_jar(args: &ImportArgs, jar_path: &PathBuf) -> Result<(), CliError> {
    use std::collections::HashSet;
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

    // Parse all proto files and extract messages
    let mut all_messages: Vec<ParsedProtoMessage> = Vec::new();
    for (file_name, content) in &proto_contents {
        let messages = extract_proto_messages(file_name, content);
        all_messages.extend(messages);
    }

    if all_messages.is_empty() {
        return Err(CliError::InvalidArgument(
            "No message definitions found in proto files".to_string(),
        ));
    }

    // Build dependency graph
    let graph = build_dependency_graph(&all_messages);

    // Determine root message
    let root_message_name = if let Some(ref specified_root) = args.root_message {
        // Use specified root message
        if !all_messages.iter().any(|m| &m.name == specified_root) {
            return Err(CliError::InvalidArgument(format!(
                "Specified root message '{}' not found. Available messages: {}",
                specified_root,
                all_messages
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        specified_root.clone()
    } else if let Some(ref filter_type) = args.message_type {
        // Use message_type as root if specified
        if !all_messages.iter().any(|m| &m.name == filter_type) {
            return Err(CliError::InvalidArgument(format!(
                "Specified message type '{}' not found. Available messages: {}",
                filter_type,
                all_messages
                    .iter()
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        filter_type.clone()
    } else {
        // Auto-detect root message
        find_root_message(&all_messages, &graph).ok_or_else(|| {
            CliError::InvalidArgument("Could not determine root message".to_string())
        })?
    };

    // Find the root message
    let root_message = all_messages
        .iter()
        .find(|m| m.name == root_message_name)
        .ok_or_else(|| {
            CliError::InvalidArgument(format!("Root message '{}' not found", root_message_name))
        })?;

    // Print dependency analysis in pretty mode
    if args.pretty {
        eprintln!("\n=== Protobuf JAR Analysis ===");
        eprintln!(
            "Found {} proto files with {} message definitions",
            proto_contents.len(),
            all_messages.len()
        );
        eprintln!("\nMessages found:");
        for msg in &all_messages {
            let deps = graph.get(&msg.name).map(|d| d.len()).unwrap_or(0);
            eprintln!("  - {} ({} dependencies)", msg.name, deps);
        }
        eprintln!("\nRoot message: {}", root_message_name);
        if let Some(deps) = graph.get(&root_message_name)
            && !deps.is_empty()
        {
            eprintln!(
                "Dependencies: {}",
                deps.iter().cloned().collect::<Vec<_>>().join(", ")
            );
        }
        eprintln!("=============================\n");
    }

    // Flatten root message and all dependencies into columns
    let mut visited = HashSet::new();
    let columns = flatten_message_to_columns(root_message, &all_messages, "", &mut visited, 10);

    // Create a single table from flattened columns
    let table_data = TableData {
        table_index: 0,
        name: Some(root_message_name.clone()),
        columns,
    };

    let result = ImportResult {
        tables: vec![table_data],
        tables_requiring_name: Vec::new(),
        errors: Vec::new(),
        ai_suggestions: None,
    };

    // Apply UUID override if provided
    let mut result = result;
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
        write_odcs_files(
            &result,
            Some(jar_path.parent().unwrap_or(std::path::Path::new("."))),
            args.uuid_override.as_deref(),
        )?;
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
                                physical_type: col.physical_type.clone(),
                                nullable: col.nullable,
                                primary_key: col.primary_key,
                                description: Some(col.description.clone()),
                                quality: None,
                                relationships: col.relationships.clone(),
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

/// Handle ODCL import command (legacy format, converted to ODCS)
pub fn handle_import_odcl(args: &ImportArgs) -> Result<(), CliError> {
    // Load ODCL input
    let odcl_content = load_input(&args.input)?;

    // Validate if enabled
    if args.validate {
        validate_odcl(&odcl_content)?;
    }

    // Import ODCL (ODCSImporter handles ODCL formats internally)
    // ODCL files are automatically converted to ODCS v3.1.0 format during import
    let mut importer = ODCSImporter::new();
    let mut result = importer
        .import(&odcl_content)
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
    // ODCL imports are converted to ODCS format
    if !args.no_odcs {
        let base_path = match &args.input {
            InputSource::File(path) => path.parent(),
            _ => None,
        };
        write_odcs_files(&result, base_path, args.uuid_override.as_deref())?;
    }

    Ok(())
}
