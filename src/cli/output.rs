//! Output formatting for CLI

use crate::import::ImportResult;

/// Format type mapping for display
pub struct TypeMapping {
    pub source_type: String,
    pub odcs_type: String,
    pub table_name: Option<String>,
    pub column_name: Option<String>,
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

        // Compact column list
        let col_names: Vec<String> = table
            .columns
            .iter()
            .map(|c| format!("{}:{}", c.name, c.data_type))
            .collect();
        output.push_str(&format!("  Columns: {}\n", col_names.join(", ")));
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
pub fn collect_type_mappings(_result: &ImportResult) -> Vec<TypeMapping> {
    // TODO: Extract actual type mappings from import result
    // This would require analyzing the source format types and their ODCS equivalents
    // For now, return empty vector - can be enhanced later
    Vec::new()
}
