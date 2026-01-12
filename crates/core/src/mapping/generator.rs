//! Transformation script generation for schema mappings

use super::config::TransformFormat;
use super::error::MappingResult;
use super::types::{SchemaMapping, TransformType};

/// Generate transformation script from a schema mapping
pub fn generate_transform(
    mapping: &SchemaMapping,
    format: TransformFormat,
    source_table: &str,
    target_table: &str,
) -> MappingResult<String> {
    match format {
        TransformFormat::Sql => generate_sql(mapping, source_table, target_table),
        TransformFormat::Jq => generate_jq(mapping),
        TransformFormat::Python => generate_python(mapping, source_table, target_table),
        TransformFormat::Spark => generate_spark(mapping, source_table, target_table),
    }
}

/// Generate SQL transformation (DuckDB compatible)
fn generate_sql(
    mapping: &SchemaMapping,
    source_table: &str,
    target_table: &str,
) -> MappingResult<String> {
    let mut lines = Vec::new();

    lines.push(format!("-- Schema mapping transformation"));
    lines.push(format!(
        "-- Source: {} -> Target: {}",
        source_table, target_table
    ));
    lines.push(format!(
        "-- Direct mappings: {}, Transformations: {}",
        mapping.direct_mappings.len(),
        mapping.transformations.len()
    ));
    lines.push(format!(
        "-- Coverage: {:.1}%",
        mapping.compatibility_score * 100.0
    ));
    lines.push(String::new());

    lines.push(format!("INSERT INTO {} (", target_table));

    // Collect all target columns
    let mut columns: Vec<String> = mapping
        .direct_mappings
        .iter()
        .map(|m| format!("    {}", escape_identifier(&m.target_path)))
        .collect();

    columns.extend(
        mapping
            .transformations
            .iter()
            .map(|t| format!("    {}", escape_identifier(&t.target_path))),
    );

    // Add columns for gaps with defaults
    for gap in &mapping.gaps {
        if gap.suggested_default.is_some() {
            columns.push(format!("    {}", escape_identifier(&gap.target_path)));
        }
    }

    lines.push(columns.join(",\n"));
    lines.push(")".to_string());
    lines.push("SELECT".to_string());

    // Generate SELECT expressions
    let mut select_exprs: Vec<String> = Vec::new();

    // Direct mappings
    for m in &mapping.direct_mappings {
        let comment = format!(
            "-- {} match, confidence: {:.0}%",
            m.match_method,
            m.confidence * 100.0
        );
        select_exprs.push(format!(
            "    {} AS {} {}",
            escape_identifier(&m.source_path),
            escape_identifier(&m.target_path),
            comment
        ));
    }

    // Transformations
    for t in &mapping.transformations {
        let expr = transform_to_sql(&t.transform_type, &t.source_paths);
        let comment = format!("-- {}", t.description);
        select_exprs.push(format!(
            "    {} AS {} {}",
            expr,
            escape_identifier(&t.target_path),
            comment
        ));
    }

    // Gap defaults
    for gap in &mapping.gaps {
        if let Some(ref default) = gap.suggested_default {
            let default_sql = value_to_sql(default);
            select_exprs.push(format!(
                "    {} AS {} -- default for missing field",
                default_sql,
                escape_identifier(&gap.target_path)
            ));
        }
    }

    lines.push(select_exprs.join(",\n"));
    lines.push(format!("FROM {};", source_table));

    // Add notes about unmapped extras
    if !mapping.extras.is_empty() {
        lines.push(String::new());
        lines.push("-- Note: The following source fields are not mapped:".to_string());
        for extra in &mapping.extras {
            lines.push(format!("--   {}", extra));
        }
    }

    // Add notes about required gaps
    let required_gaps: Vec<_> = mapping
        .gaps
        .iter()
        .filter(|g| g.required && g.suggested_default.is_none())
        .collect();
    if !required_gaps.is_empty() {
        lines.push(String::new());
        lines.push("-- WARNING: The following required target fields have no mapping:".to_string());
        for gap in required_gaps {
            lines.push(format!("--   {} ({})", gap.target_path, gap.target_type));
            if !gap.suggestions.is_empty() {
                lines.push(format!(
                    "--     Suggestions: {}",
                    gap.suggestions.join(", ")
                ));
            }
        }
    }

    Ok(lines.join("\n"))
}

/// Generate JQ transformation
fn generate_jq(mapping: &SchemaMapping) -> MappingResult<String> {
    let mut lines = Vec::new();

    lines.push("# Schema mapping transformation (jq)".to_string());
    lines.push(format!(
        "# Coverage: {:.1}%",
        mapping.compatibility_score * 100.0
    ));
    lines.push(String::new());
    lines.push("{".to_string());

    let mut assignments: Vec<String> = Vec::new();

    // Direct mappings
    for m in &mapping.direct_mappings {
        let source_jq = path_to_jq(&m.source_path);
        assignments.push(format!("  \"{}\": {}", m.target_path, source_jq));
    }

    // Transformations
    for t in &mapping.transformations {
        let expr = transform_to_jq(&t.transform_type, &t.source_paths);
        assignments.push(format!(
            "  \"{}\": {} # {}",
            t.target_path, expr, t.description
        ));
    }

    // Gap defaults
    for gap in &mapping.gaps {
        if let Some(ref default) = gap.suggested_default {
            let default_jq = serde_json::to_string(default).unwrap_or_else(|_| "null".to_string());
            assignments.push(format!(
                "  \"{}\": {} # default",
                gap.target_path, default_jq
            ));
        }
    }

    lines.push(assignments.join(",\n"));
    lines.push("}".to_string());

    Ok(lines.join("\n"))
}

/// Generate Python transformation
fn generate_python(
    mapping: &SchemaMapping,
    source_table: &str,
    target_table: &str,
) -> MappingResult<String> {
    let mut lines = Vec::new();

    lines.push("\"\"\"".to_string());
    lines.push("Schema mapping transformation".to_string());
    lines.push(format!(
        "Source: {} -> Target: {}",
        source_table, target_table
    ));
    lines.push(format!(
        "Coverage: {:.1}%",
        mapping.compatibility_score * 100.0
    ));
    lines.push("\"\"\"".to_string());
    lines.push(String::new());
    lines.push("import json".to_string());
    lines.push("from typing import Dict, Any, List".to_string());
    lines.push(String::new());
    lines.push(String::new());
    lines.push("def transform_record(source: Dict[str, Any]) -> Dict[str, Any]:".to_string());
    lines.push("    \"\"\"Transform a source record to target schema.\"\"\"".to_string());
    lines.push("    target = {}".to_string());
    lines.push(String::new());

    // Direct mappings
    if !mapping.direct_mappings.is_empty() {
        lines.push("    # Direct mappings".to_string());
        for m in &mapping.direct_mappings {
            let source_access = path_to_python_access(&m.source_path);
            lines.push(format!(
                "    target[\"{}\"] = {}  # {} match",
                m.target_path, source_access, m.match_method
            ));
        }
        lines.push(String::new());
    }

    // Transformations
    if !mapping.transformations.is_empty() {
        lines.push("    # Transformations".to_string());
        for t in &mapping.transformations {
            let expr = transform_to_python(&t.transform_type, &t.source_paths);
            lines.push(format!(
                "    target[\"{}\"] = {}  # {}",
                t.target_path, expr, t.description
            ));
        }
        lines.push(String::new());
    }

    // Gap defaults
    let gaps_with_defaults: Vec<_> = mapping
        .gaps
        .iter()
        .filter(|g| g.suggested_default.is_some())
        .collect();
    if !gaps_with_defaults.is_empty() {
        lines.push("    # Default values for missing fields".to_string());
        for gap in gaps_with_defaults {
            if let Some(ref default) = gap.suggested_default {
                let default_py = value_to_python(default);
                lines.push(format!(
                    "    target[\"{}\"] = {}",
                    gap.target_path, default_py
                ));
            }
        }
        lines.push(String::new());
    }

    lines.push("    return target".to_string());
    lines.push(String::new());
    lines.push(String::new());
    lines.push(
        "def transform_batch(records: List[Dict[str, Any]]) -> List[Dict[str, Any]]:".to_string(),
    );
    lines.push("    \"\"\"Transform a batch of records.\"\"\"".to_string());
    lines.push("    return [transform_record(r) for r in records]".to_string());

    Ok(lines.join("\n"))
}

/// Generate PySpark transformation
fn generate_spark(
    mapping: &SchemaMapping,
    source_table: &str,
    target_table: &str,
) -> MappingResult<String> {
    let mut lines = Vec::new();

    lines.push("\"\"\"".to_string());
    lines.push("Schema mapping transformation (PySpark)".to_string());
    lines.push(format!(
        "Source: {} -> Target: {}",
        source_table, target_table
    ));
    lines.push(format!(
        "Coverage: {:.1}%",
        mapping.compatibility_score * 100.0
    ));
    lines.push("\"\"\"".to_string());
    lines.push(String::new());
    lines.push("from pyspark.sql import DataFrame".to_string());
    lines.push("from pyspark.sql.functions import col, lit, concat, concat_ws, split".to_string());
    lines.push(
        "from pyspark.sql.types import StringType, IntegerType, DoubleType, BooleanType"
            .to_string(),
    );
    lines.push(String::new());
    lines.push(String::new());
    lines.push("def transform(df: DataFrame) -> DataFrame:".to_string());
    lines.push("    \"\"\"Transform source DataFrame to target schema.\"\"\"".to_string());
    lines.push("    return df.select(".to_string());

    let mut select_exprs: Vec<String> = Vec::new();

    // Direct mappings
    for m in &mapping.direct_mappings {
        let source_col = path_to_spark_col(&m.source_path);
        select_exprs.push(format!(
            "        {}.alias(\"{}\"),  # {}",
            source_col, m.target_path, m.match_method
        ));
    }

    // Transformations
    for t in &mapping.transformations {
        let expr = transform_to_spark(&t.transform_type, &t.source_paths);
        select_exprs.push(format!(
            "        {}.alias(\"{}\"),  # {}",
            expr, t.target_path, t.description
        ));
    }

    // Gap defaults
    for gap in &mapping.gaps {
        if let Some(ref default) = gap.suggested_default {
            let default_spark = value_to_spark_lit(default);
            select_exprs.push(format!(
                "        {}.alias(\"{}\"),  # default",
                default_spark, gap.target_path
            ));
        }
    }

    // Remove trailing comma from last expression
    if let Some(last) = select_exprs.last_mut() {
        if last.ends_with(',') {
            last.pop();
        }
    }

    lines.push(select_exprs.join("\n"));
    lines.push("    )".to_string());

    Ok(lines.join("\n"))
}

// Helper functions

fn escape_identifier(name: &str) -> String {
    if name.contains('.') {
        // For nested paths, use JSON extraction in SQL
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() == 1 {
            format!("\"{}\"", name)
        } else {
            format!("\"{}\"", parts.last().unwrap_or(&name))
        }
    } else {
        format!("\"{}\"", name)
    }
}

fn transform_to_sql(transform: &TransformType, sources: &[String]) -> String {
    match transform {
        TransformType::TypeCast { to_type, .. } => {
            let sql_type = json_type_to_sql(to_type);
            format!("CAST({} AS {})", escape_identifier(&sources[0]), sql_type)
        }
        TransformType::Rename => escape_identifier(&sources[0]),
        TransformType::Merge { separator } => {
            let sep = separator.as_deref().unwrap_or(" ");
            let cols: Vec<String> = sources.iter().map(|s| escape_identifier(s)).collect();
            format!("CONCAT_WS('{}', {})", sep, cols.join(", "))
        }
        TransformType::Split { delimiter, .. } => {
            format!(
                "STRING_SPLIT({}, '{}')",
                escape_identifier(&sources[0]),
                delimiter
            )
        }
        TransformType::FormatChange { to_format, .. } => {
            // Date format conversion
            format!(
                "STRFTIME({}, '{}')",
                escape_identifier(&sources[0]),
                to_format
            )
        }
        TransformType::Custom { expression } => expression.clone(),
        TransformType::Extract { json_path } => {
            format!(
                "JSON_EXTRACT({}, '{}')",
                escape_identifier(&sources[0]),
                json_path
            )
        }
        TransformType::Default { value } => value_to_sql(value),
    }
}

fn transform_to_jq(transform: &TransformType, sources: &[String]) -> String {
    match transform {
        TransformType::TypeCast { to_type, .. } => {
            let source = path_to_jq(&sources[0]);
            match to_type.as_str() {
                "integer" => format!("({} | tonumber)", source),
                "number" => format!("({} | tonumber)", source),
                "string" => format!("({} | tostring)", source),
                "boolean" => format!("({} | . == \"true\" or . == true)", source),
                _ => source,
            }
        }
        TransformType::Rename => path_to_jq(&sources[0]),
        TransformType::Merge { separator } => {
            let sep = separator.as_deref().unwrap_or(" ");
            let parts: Vec<String> = sources.iter().map(|s| path_to_jq(s)).collect();
            format!("([{}] | join(\"{}\"))", parts.join(", "), sep)
        }
        TransformType::Split { delimiter, .. } => {
            format!("({} | split(\"{}\"))", path_to_jq(&sources[0]), delimiter)
        }
        TransformType::FormatChange { .. } => path_to_jq(&sources[0]),
        TransformType::Custom { expression } => expression.clone(),
        TransformType::Extract { json_path } => {
            // Convert JSON path to jq path
            format!(".{}", json_path.trim_start_matches("$."))
        }
        TransformType::Default { value } => {
            serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
        }
    }
}

fn transform_to_python(transform: &TransformType, sources: &[String]) -> String {
    match transform {
        TransformType::TypeCast { to_type, .. } => {
            let source = path_to_python_access(&sources[0]);
            match to_type.as_str() {
                "integer" => format!("int({})", source),
                "number" => format!("float({})", source),
                "string" => format!("str({})", source),
                "boolean" => format!("bool({})", source),
                _ => source,
            }
        }
        TransformType::Rename => path_to_python_access(&sources[0]),
        TransformType::Merge { separator } => {
            let sep = separator.as_deref().unwrap_or(" ");
            let parts: Vec<String> = sources.iter().map(|s| path_to_python_access(s)).collect();
            format!("\"{}\".join([{}])", sep, parts.join(", "))
        }
        TransformType::Split { delimiter, .. } => {
            format!(
                "{}.split(\"{}\")",
                path_to_python_access(&sources[0]),
                delimiter
            )
        }
        TransformType::FormatChange { .. } => path_to_python_access(&sources[0]),
        TransformType::Custom { expression } => expression.clone(),
        TransformType::Extract { json_path } => {
            // Simple nested access
            let path_parts: Vec<&str> = json_path.trim_start_matches("$.").split('.').collect();
            let mut access = "source".to_string();
            for part in path_parts {
                access = format!("{}.get(\"{}\", {{}})", access, part);
            }
            access
        }
        TransformType::Default { value } => value_to_python(value),
    }
}

fn transform_to_spark(transform: &TransformType, sources: &[String]) -> String {
    match transform {
        TransformType::TypeCast { to_type, .. } => {
            let source = path_to_spark_col(&sources[0]);
            let spark_type = json_type_to_spark(to_type);
            format!("{}.cast({})", source, spark_type)
        }
        TransformType::Rename => path_to_spark_col(&sources[0]),
        TransformType::Merge { separator } => {
            let sep = separator.as_deref().unwrap_or(" ");
            let cols: Vec<String> = sources.iter().map(|s| path_to_spark_col(s)).collect();
            format!("concat_ws(\"{}\", {})", sep, cols.join(", "))
        }
        TransformType::Split { delimiter, .. } => {
            format!(
                "split({}, \"{}\")",
                path_to_spark_col(&sources[0]),
                delimiter
            )
        }
        TransformType::FormatChange { to_format, .. } => {
            format!(
                "date_format({}, \"{}\")",
                path_to_spark_col(&sources[0]),
                to_format
            )
        }
        TransformType::Custom { expression } => expression.clone(),
        TransformType::Extract { json_path } => {
            format!(
                "get_json_object({}, \"{}\")",
                path_to_spark_col(&sources[0]),
                json_path
            )
        }
        TransformType::Default { value } => value_to_spark_lit(value),
    }
}

fn path_to_jq(path: &str) -> String {
    if path.contains('.') {
        format!(".{}", path)
    } else {
        format!(".{}", path)
    }
}

fn path_to_python_access(path: &str) -> String {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.len() == 1 {
        format!("source.get(\"{}\")", path)
    } else {
        let mut access = "source".to_string();
        for part in parts {
            access = format!("{}.get(\"{}\", {{}})", access, part);
        }
        access
    }
}

fn path_to_spark_col(path: &str) -> String {
    if path.contains('.') {
        format!("col(\"{}\")", path)
    } else {
        format!("col(\"{}\")", path)
    }
}

fn value_to_sql(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        serde_json::Value::Array(_) => "ARRAY[]".to_string(),
        serde_json::Value::Object(_) => "'{}'::JSON".to_string(),
    }
}

fn value_to_python(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "None".to_string(),
        serde_json::Value::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("\"{}\"", s),
        serde_json::Value::Array(_) => "[]".to_string(),
        serde_json::Value::Object(_) => "{}".to_string(),
    }
}

fn value_to_spark_lit(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "lit(None)".to_string(),
        serde_json::Value::Bool(b) => format!("lit({})", if *b { "True" } else { "False" }),
        serde_json::Value::Number(n) => format!("lit({})", n),
        serde_json::Value::String(s) => format!("lit(\"{}\")", s),
        serde_json::Value::Array(_) => "lit([])".to_string(),
        serde_json::Value::Object(_) => "lit({})".to_string(),
    }
}

fn json_type_to_sql(json_type: &str) -> &'static str {
    match json_type {
        "string" => "VARCHAR",
        "integer" => "INTEGER",
        "number" => "DOUBLE",
        "boolean" => "BOOLEAN",
        "array" => "JSON",
        "object" => "JSON",
        _ => "VARCHAR",
    }
}

fn json_type_to_spark(json_type: &str) -> &'static str {
    match json_type {
        "string" => "StringType()",
        "integer" => "IntegerType()",
        "number" => "DoubleType()",
        "boolean" => "BooleanType()",
        _ => "StringType()",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapping::types::{FieldMapping, MatchMethod, TransformMapping};

    fn create_test_mapping() -> SchemaMapping {
        let mut mapping = SchemaMapping::empty();

        mapping.direct_mappings.push(
            FieldMapping::new("source_name", "target_name").with_match_method(MatchMethod::Exact),
        );
        mapping.direct_mappings.push(
            FieldMapping::new("source_email", "target_email")
                .with_match_method(MatchMethod::CaseInsensitive)
                .with_confidence(0.95),
        );

        mapping.transformations.push(TransformMapping::new(
            vec!["amount".to_string()],
            "amount_int",
            TransformType::TypeCast {
                from_type: "string".to_string(),
                to_type: "integer".to_string(),
            },
        ));

        mapping.compatibility_score = 0.85;
        mapping
    }

    #[test]
    fn test_generate_sql() {
        let mapping = create_test_mapping();
        let sql = generate_sql(&mapping, "source_table", "target_table").unwrap();

        assert!(sql.contains("INSERT INTO target_table"));
        assert!(sql.contains("SELECT"));
        assert!(sql.contains("CAST"));
        assert!(sql.contains("FROM source_table"));
    }

    #[test]
    fn test_generate_jq() {
        let mapping = create_test_mapping();
        let jq = generate_jq(&mapping).unwrap();

        assert!(jq.contains("{"));
        assert!(jq.contains("}"));
        assert!(jq.contains("\"target_name\""));
        assert!(jq.contains("tonumber"));
    }

    #[test]
    fn test_generate_python() {
        let mapping = create_test_mapping();
        let python = generate_python(&mapping, "source", "target").unwrap();

        assert!(python.contains("def transform_record"));
        assert!(python.contains("target = {}"));
        assert!(python.contains("int("));
        assert!(python.contains("return target"));
    }

    #[test]
    fn test_generate_spark() {
        let mapping = create_test_mapping();
        let spark = generate_spark(&mapping, "source", "target").unwrap();

        assert!(spark.contains("def transform"));
        assert!(spark.contains("DataFrame"));
        assert!(spark.contains("select"));
        assert!(spark.contains(".cast("));
    }

    #[test]
    fn test_transform_to_sql() {
        let cast = TransformType::TypeCast {
            from_type: "string".to_string(),
            to_type: "integer".to_string(),
        };
        let sql = transform_to_sql(&cast, &["amount".to_string()]);
        assert!(sql.contains("CAST"));
        assert!(sql.contains("INTEGER"));

        let merge = TransformType::Merge {
            separator: Some(", ".to_string()),
        };
        let sql = transform_to_sql(&merge, &["first".to_string(), "last".to_string()]);
        assert!(sql.contains("CONCAT_WS"));
    }

    #[test]
    fn test_value_to_sql() {
        assert_eq!(value_to_sql(&serde_json::json!(null)), "NULL");
        assert_eq!(value_to_sql(&serde_json::json!(true)), "TRUE");
        assert_eq!(value_to_sql(&serde_json::json!(42)), "42");
        assert_eq!(value_to_sql(&serde_json::json!("hello")), "'hello'");
    }
}
