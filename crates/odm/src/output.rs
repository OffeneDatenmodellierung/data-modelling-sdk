//! Output formatting for CLI

use data_modelling_core::import::ImportResult;

/// Format type mapping for display
#[allow(dead_code)]
pub struct TypeMapping {
    pub source_type: String,
    pub odcs_type: String,
    pub table_name: Option<String>,
    pub column_name: Option<String>,
}

/// Represents a flattened column with its full path
struct FlattenedColumn {
    path: String,
    data_type: String,
}

/// Parse a STRUCT type string and extract field definitions
/// Returns a vector of (field_name, field_type) tuples
fn parse_struct_fields(struct_type: &str) -> Vec<(String, String)> {
    let mut fields = Vec::new();

    // Remove outer STRUCT< ... > wrapper
    let inner = if let Some(start) = struct_type.find('<') {
        let end = struct_type.rfind('>').unwrap_or(struct_type.len());
        &struct_type[start + 1..end]
    } else {
        return fields;
    };

    // Parse fields, handling nested types
    let mut current_field = String::new();
    let mut depth = 0;

    for ch in inner.chars() {
        match ch {
            '<' => {
                depth += 1;
                current_field.push(ch);
            }
            '>' => {
                depth -= 1;
                current_field.push(ch);
            }
            ',' if depth == 0 => {
                // End of a field
                if let Some((name, typ)) = parse_field_definition(current_field.trim()) {
                    fields.push((name, typ));
                }
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    // Handle last field
    if !current_field.trim().is_empty()
        && let Some((name, typ)) = parse_field_definition(current_field.trim())
    {
        fields.push((name, typ));
    }

    fields
}

/// Parse a single field definition like "name: TYPE" or "name TYPE"
fn parse_field_definition(field_str: &str) -> Option<(String, String)> {
    let field_str = field_str.trim();
    if field_str.is_empty() {
        return None;
    }

    // Try "name: TYPE" format first
    if let Some(colon_pos) = field_str.find(':') {
        let name = field_str[..colon_pos].trim().to_string();
        let mut typ = field_str[colon_pos + 1..].trim().to_string();
        // Remove any trailing " >" artifacts from nested struct parsing
        while typ.ends_with(" >") {
            typ = typ[..typ.len() - 2].trim().to_string();
        }
        if !name.is_empty() && !typ.is_empty() {
            return Some((name, typ));
        }
    }

    // Try "name TYPE" format (space-separated)
    let parts: Vec<&str> = field_str.splitn(2, ' ').collect();
    if parts.len() == 2 {
        let name = parts[0].trim().to_string();
        let mut typ = parts[1].trim().to_string();
        // Remove any trailing " >" artifacts from nested struct parsing
        while typ.ends_with(" >") {
            typ = typ[..typ.len() - 2].trim().to_string();
        }
        if !name.is_empty() && !typ.is_empty() {
            return Some((name, typ));
        }
    }

    None
}

/// Recursively flatten a column with nested types into dot-notation paths
fn flatten_column(prefix: &str, data_type: &str, result: &mut Vec<FlattenedColumn>) {
    let data_type_upper = data_type.to_uppercase();

    // Check if it's an ARRAY<STRUCT<...>> type
    if data_type_upper.starts_with("ARRAY<STRUCT<") {
        // Add the array column itself
        result.push(FlattenedColumn {
            path: prefix.to_string(),
            data_type: "ARRAY<STRUCT>".to_string(),
        });

        // Extract inner STRUCT definition
        let struct_start = data_type.find("STRUCT<").unwrap_or(0);
        let inner_struct = &data_type[struct_start..];

        // Parse and flatten the STRUCT fields with [] suffix
        let fields = parse_struct_fields(inner_struct);
        for (field_name, field_type) in fields {
            let child_path = format!("{}[].{}", prefix, field_name);
            flatten_column(&child_path, &field_type, result);
        }
    }
    // Check if it's a STRUCT<...> type
    else if data_type_upper.starts_with("STRUCT<") {
        // Add the struct column itself
        result.push(FlattenedColumn {
            path: prefix.to_string(),
            data_type: "STRUCT".to_string(),
        });

        // Parse and flatten the STRUCT fields
        let fields = parse_struct_fields(data_type);
        for (field_name, field_type) in fields {
            let child_path = format!("{}.{}", prefix, field_name);
            flatten_column(&child_path, &field_type, result);
        }
    }
    // MAP<...>, ARRAY<primitive>, or primitive type - all handled the same way
    else {
        result.push(FlattenedColumn {
            path: prefix.to_string(),
            data_type: data_type.to_string(),
        });
    }
}

/// Format import result in compact mode
pub fn format_compact_output(result: &ImportResult) -> String {
    let mut output = String::new();

    if !result.errors.is_empty() {
        output.push_str("\n⚠️  Parse Errors:\n");
        for error in &result.errors {
            output.push_str(&format!("  - {:?}\n", error));
        }
    }

    if !result.tables_requiring_name.is_empty() {
        output.push_str("\n⚠️  Tables Requiring Name Resolution:\n");
        for table in &result.tables_requiring_name {
            output.push_str(&format!("  - Table index: {}\n", table.table_index));
            if let Some(name) = &table.suggested_name {
                output.push_str(&format!("    Suggested name: {}\n", name));
            }
        }
    }

    output.push_str(&format!("\n✅ Parsed {} table(s):\n", result.tables.len()));
    for (idx, table) in result.tables.iter().enumerate() {
        output.push_str(&format!("\nTable {}:\n", idx + 1));
        output.push_str(&format!("  Name: {:?}\n", table.name));
        output.push_str(&format!("  Columns: {}\n", table.columns.len()));

        // Stage 1: Compact column list - one column per row with full type
        for col in &table.columns {
            output.push_str(&format!("  Columns: {}:{}\n", col.name, col.data_type));
        }

        // Stage 2: Expanded nested columns with jq-style dot notation
        let mut flattened_columns: Vec<FlattenedColumn> = Vec::new();
        for col in &table.columns {
            flatten_column(&col.name, &col.data_type, &mut flattened_columns);
        }

        // Only show expanded view if there are nested columns
        let has_nested = table.columns.iter().any(|col| {
            let dt = col.data_type.to_uppercase();
            dt.contains("STRUCT<") || dt.starts_with("ARRAY<STRUCT")
        });

        if has_nested {
            output.push_str(&format!(
                "\n  Expanded Columns (total: {}):\n",
                flattened_columns.len()
            ));
            for flat_col in &flattened_columns {
                output.push_str(&format!("    .{}: {}\n", flat_col.path, flat_col.data_type));
            }
        }
    }

    if result.errors.is_empty() && result.tables_requiring_name.is_empty() {
        output.push_str("\n✅ All checks passed!\n");
    }

    output
}

/// Format import result in pretty mode
pub fn format_pretty_output(result: &ImportResult, mappings: &[TypeMapping]) -> String {
    let mut output = String::new();

    if !result.errors.is_empty() {
        output.push_str("\n⚠️  Parse Errors:\n");
        for error in &result.errors {
            output.push_str(&format!("  - {:?}\n", error));
        }
    }

    if !result.tables_requiring_name.is_empty() {
        output.push_str("\n⚠️  Tables Requiring Name Resolution:\n");
        for table in &result.tables_requiring_name {
            output.push_str(&format!("  - Table index: {}\n", table.table_index));
            if let Some(name) = &table.suggested_name {
                output.push_str(&format!("    Suggested name: {}\n", name));
            }
        }
    }

    output.push_str(&format!("\n✅ Parsed {} table(s):\n", result.tables.len()));
    for (idx, table) in result.tables.iter().enumerate() {
        output.push_str(&format!("\nTable {}: {:?}\n", idx + 1, table.name));
        output.push_str(&format!("  Columns: {}\n", table.columns.len()));
        output.push_str("  Column Details:\n");

        for col in &table.columns {
            output.push_str(&format!("    - {} ({})\n", col.name, col.data_type));
            if let Some(desc) = &col.description {
                output.push_str(&format!("      Comment: {}\n", desc));
            }
            if col.primary_key {
                output.push_str("      Primary Key: true\n");
            }
            if !col.nullable {
                output.push_str("      Nullable: false\n");
            }
        }
    }

    // Display type mappings
    if !mappings.is_empty() {
        output.push_str("\nType Mappings:\n");
        for mapping in mappings {
            output.push_str(&format!(
                "  - {} → {}\n",
                mapping.source_type, mapping.odcs_type
            ));
        }
    }

    if result.errors.is_empty() && result.tables_requiring_name.is_empty() {
        output.push_str("\n✅ All checks passed!\n");
    }

    output
}

/// Collect type mappings from import result
///
/// Extracts unique type mappings showing how source format types
/// are mapped to ODCS/normalized data types.
pub fn collect_type_mappings(result: &ImportResult) -> Vec<TypeMapping> {
    use std::collections::HashSet;

    let mut mappings = Vec::new();
    let mut seen_types: HashSet<String> = HashSet::new();

    for table in &result.tables {
        let table_name = table.name.clone();

        for col in &table.columns {
            // Skip if we've already seen this source type
            let source_type = col.data_type.clone();
            if seen_types.contains(&source_type) {
                continue;
            }
            seen_types.insert(source_type.clone());

            // Map source type to ODCS type
            let odcs_type = map_to_odcs_type(&source_type);

            // Only add mapping if it's a meaningful conversion (types differ)
            if source_type.to_uppercase() != odcs_type.to_uppercase() {
                mappings.push(TypeMapping {
                    source_type,
                    odcs_type,
                    table_name: table_name.clone(),
                    column_name: Some(col.name.clone()),
                });
            }
        }
    }

    mappings
}

/// Map a source data type to its ODCS equivalent
///
/// This function normalizes various SQL dialect types to ODCS standard types.
fn map_to_odcs_type(source_type: &str) -> String {
    let upper = source_type.to_uppercase();

    // Handle parameterized types
    if upper.starts_with("VARCHAR") || upper.starts_with("CHAR") || upper.starts_with("NVARCHAR") {
        return "STRING".to_string();
    }
    if upper.starts_with("DECIMAL") || upper.starts_with("NUMERIC") {
        return "DECIMAL".to_string();
    }
    if upper.starts_with("ARRAY<") {
        return source_type.to_string(); // Keep ARRAY types as-is
    }
    if upper.starts_with("MAP<") {
        return source_type.to_string(); // Keep MAP types as-is
    }
    if upper.starts_with("STRUCT<") {
        return "STRUCT".to_string();
    }

    // Map common types
    match upper.as_str() {
        // Integer types
        "INT" | "INTEGER" | "INT4" | "INT32" => "INTEGER".to_string(),
        "BIGINT" | "INT8" | "INT64" | "LONG" => "LONG".to_string(),
        "SMALLINT" | "INT2" | "INT16" => "SHORT".to_string(),
        "TINYINT" | "INT1" => "BYTE".to_string(),

        // Floating point types
        "FLOAT" | "FLOAT4" | "REAL" => "FLOAT".to_string(),
        "DOUBLE" | "FLOAT8" | "DOUBLE PRECISION" => "DOUBLE".to_string(),

        // String types
        "TEXT" | "CLOB" | "LONGTEXT" | "MEDIUMTEXT" => "STRING".to_string(),
        "STRING" => "STRING".to_string(),

        // Boolean types
        "BOOL" | "BOOLEAN" => "BOOLEAN".to_string(),

        // Date/Time types
        "DATE" => "DATE".to_string(),
        "TIME" => "TIME".to_string(),
        "DATETIME" | "TIMESTAMP" | "TIMESTAMPTZ" | "TIMESTAMP WITH TIME ZONE" => {
            "TIMESTAMP".to_string()
        }

        // Binary types
        "BINARY" | "VARBINARY" | "BLOB" | "BYTEA" | "BYTES" => "BINARY".to_string(),

        // UUID types
        "UUID" | "GUID" => "UUID".to_string(),

        // JSON types
        "JSON" | "JSONB" => "JSON".to_string(),

        // Default: return as-is (already normalized or unknown)
        _ => source_type.to_string(),
    }
}
